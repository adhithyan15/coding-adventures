//! SPARC V8 CPU — instruction fetch, decode, and execution.
//!
//! # Halting convention
//!
//! `ta 0` encodes as `0x91D0_2000`.  When the CPU decodes this trap with
//! `cond=8` (always) and `trap_number=0`, it sets `halted = true` and stops.
//!
//! # Memory
//!
//! 64 KiB flat memory, addresses masked to 16 bits.  Instructions are
//! big-endian 32-bit words.
//!
//! # No branch delay slots
//!
//! This simulator omits the SPARC delay slot for simplicity; branches take
//! effect immediately (the instruction after the branch is *not* executed).

use crate::alu::{
    add32, addcc32, addx32, addxcc32, and32, andcc32, andn32, andncc32, mulscc, or32, orcc32,
    orn32, orncc32, sdiv64, sdiv64_with_overflow, sethi as alu_sethi, sll32, smul32, sra32, srl32,
    sub32, subcc32, subx32, subxcc32, udiv64, udiv64_with_overflow, umul32, xnor32, xnorcc32,
    xor32, xorcc32, Cc,
};
use crate::bits::{sext22, sext30, u32_to_bits};
use crate::decoder::{decode, Instruction, Src2};
use crate::register_file::{RegisterFile, MEM_SIZE};
use crate::state::{clock_bit, DffMemory};
use sparc_v8_simulator::execute::Psr;
use sparc_v8_simulator::{ExecutionResult, SparcError, SparcState, SparcV8Simulator, StepTrace};

const HALT_WORD: u32 = 0x91D0_2000;
const MEM_MASK: u32 = (MEM_SIZE - 1) as u32;

/// Exact persistent topology: 524,288 memory bits, 1,792 physical-register
/// bits, 64 PC/nPC bits, four CWP/depth bits, four PSR bits, 32 Y bits, and
/// one halt bit.
pub const FLIP_FLOP_COUNT: usize = MEM_SIZE * 8 + 56 * 32 + 2 * 32 + 2 * 2 + 4 + 32 + 1;

/// The SPARC V8 CPU.
pub struct SparcCpu {
    pub rf: RegisterFile,
    pub mem: DffMemory,
    pub halted: bool,
    pub steps: u64,
    halt_q: u8,
    loaded_origin: u32,
    loaded_len: usize,
}

impl SparcCpu {
    pub fn new() -> Self {
        Self {
            rf: RegisterFile::new(),
            mem: DffMemory::new(MEM_SIZE),
            halted: false,
            steps: 0,
            halt_q: 0,
            loaded_origin: 0,
            loaded_len: MEM_SIZE,
        }
    }

    /// Reset every persistent DFF and lifecycle field.
    pub fn reset(&mut self) {
        self.rf.reset();
        self.mem.clear();
        self.halted = false;
        clock_bit(&mut self.halt_q, false);
        self.steps = 0;
        self.loaded_origin = 0;
        self.loaded_len = MEM_SIZE;
    }

    /// Load a program into memory starting at `origin` and reset state.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), SparcError> {
        self.load_at_checked(program, origin)
    }

    /// Return every architectural and lifecycle field as an owned snapshot.
    pub fn get_state(&self) -> SparcState {
        SparcState {
            pc: self.rf.read_pc(),
            npc: self.rf.read_npc(),
            regs: self.rf.physical_registers(),
            cwp: self.rf.read_cwp(),
            save_depth: self.rf.read_save_depth(),
            psr: self.rf.read_psr(),
            y: self.rf.read_y(),
            memory: self.mem.snapshot(),
            halted: self.halted,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    /// Atomically restore a complete state after shared functional validation.
    pub fn restore(&mut self, state: &SparcState) -> Result<(), SparcError> {
        let mut validator = SparcV8Simulator::architectural();
        validator.restore(state)?;
        let mut rf = RegisterFile::new();
        rf.restore_physical(state.regs);
        rf.write_pc(state.pc);
        rf.write_npc(state.npc);
        rf.write_cwp(state.cwp);
        rf.write_save_depth(state.save_depth);
        rf.write_psr(state.psr);
        rf.write_y(state.y);
        self.rf = rf;
        self.mem.restore_snapshot(&state.memory);
        self.halted = state.halted;
        clock_bit(&mut self.halt_q, state.halted);
        self.loaded_origin = state.loaded_origin;
        self.loaded_len = state.loaded_len;
        self.steps = 0;
        Ok(())
    }

    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), SparcError> {
        self.load_at_checked(program, 0)
    }

    pub fn load_at_checked(&mut self, program: &[u8], origin: u32) -> Result<(), SparcError> {
        if origin & 3 != 0 {
            return Err(SparcError::MisalignedProgram { origin });
        }
        let start = origin as usize;
        if start
            .checked_add(program.len())
            .is_none_or(|end| end > MEM_SIZE)
        {
            return Err(SparcError::ProgramOutOfRange {
                origin,
                length: program.len(),
                memory_size: MEM_SIZE,
            });
        }
        self.reset();
        self.mem.copy_from_slice(start, program);
        self.rf.write_pc(origin);
        self.rf.write_npc(origin.wrapping_add(4) & MEM_MASK);
        self.loaded_origin = origin;
        self.loaded_len = program.len();
        Ok(())
    }

    pub fn read_register_checked(&self, index: u32) -> Result<u32, SparcError> {
        if index >= 32 {
            return Err(SparcError::InvalidRegister { index });
        }
        Ok(self.rf.read(index))
    }

    pub fn write_register_checked(&mut self, index: u32, value: u32) -> Result<(), SparcError> {
        if index >= 32 {
            return Err(SparcError::InvalidRegister { index });
        }
        self.rf.write(index, value);
        Ok(())
    }

    pub fn read_byte_checked(&self, address: u32) -> Result<u8, SparcError> {
        if address as usize >= MEM_SIZE {
            return Err(SparcError::MemoryOutOfRange { address, width: 1 });
        }
        Ok(self.mem[address as usize])
    }

    pub fn write_byte_checked(&mut self, address: u32, value: u8) -> Result<(), SparcError> {
        if address as usize >= MEM_SIZE {
            return Err(SparcError::MemoryOutOfRange { address, width: 1 });
        }
        self.mem.write(address as usize, value);
        Ok(())
    }

    pub fn read_word_checked(&self, address: u32) -> Result<u32, SparcError> {
        self.check_direct(address, 4)?;
        Ok(self.load_word(address))
    }

    pub fn write_word_checked(&mut self, address: u32, value: u32) -> Result<(), SparcError> {
        self.check_direct(address, 4)?;
        self.store_word(address, value);
        Ok(())
    }

    fn check_direct(&self, address: u32, width: usize) -> Result<(), SparcError> {
        if address as usize & (width - 1) != 0 {
            return Err(SparcError::MisalignedAccess { address, width });
        }
        if (address as usize)
            .checked_add(width)
            .is_none_or(|end| end > MEM_SIZE)
        {
            return Err(SparcError::MemoryOutOfRange { address, width });
        }
        Ok(())
    }

    /// Run the program until halted or `max_steps` are reached.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: u64,
    ) -> Result<(), SparcError> {
        self.load(program, origin)?;
        while !self.halted && self.steps < max_steps {
            self.step()?;
        }
        Ok(())
    }

    /// Execute a single instruction atomically.
    pub fn step(&mut self) -> Result<(), SparcError> {
        self.step_checked().map(|_| ())
    }

    /// Execute one instruction atomically with a complete shared trace.
    pub fn step_checked(&mut self) -> Result<StepTrace, SparcError> {
        if self.halted {
            return Err(SparcError::Halted);
        }
        let before = self.get_state();
        let mut oracle = SparcV8Simulator::architectural();
        oracle.restore(&before)?;
        let expected = oracle.step_checked()?;
        if let Err(error) = self.execute_one() {
            self.restore(&before)?;
            return Err(error);
        }
        let after = self.get_state();
        if after != expected.state_after {
            self.restore(&before)?;
            return Err(SparcError::InvalidState(format!(
                "gate transition for {:#010x} diverged from the functional contract",
                expected.raw
            )));
        }
        Ok(StepTrace {
            pc_before: before.pc,
            pc_after: after.pc,
            raw: expected.raw,
            mnemonic: expected.mnemonic,
            state_before: before,
            state_after: after,
        })
    }

    pub fn run_loaded_checked(&mut self, max_steps: usize) -> Result<ExecutionResult, SparcError> {
        let original = self.get_state();
        let mut traces = Vec::new();
        while traces.len() < max_steps && !self.halted {
            match self.step_checked() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.restore(&original)?;
                    return Err(error);
                }
            }
        }
        Ok(ExecutionResult {
            halted: self.halted,
            steps: traces.len(),
            pc: self.rf.read_pc() as i32,
            final_state: self.get_state(),
            traces,
        })
    }

    pub fn run_checked(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, SparcError> {
        let original = self.get_state();
        self.load_checked(program)?;
        match self.run_loaded_checked(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.restore(&original)?;
                Err(error)
            }
        }
    }

    fn execute_one(&mut self) -> Result<(), SparcError> {
        let word = self.fetch();
        if word == HALT_WORD {
            self.halted = true;
            clock_bit(&mut self.halt_q, true);
            self.rf
                .write_npc(self.rf.read_pc().wrapping_add(4) & MEM_MASK);
            self.steps += 1;
            return Ok(());
        }
        let instr = decode(word);
        self.dispatch(instr, word)?;
        self.rf
            .write_npc(self.rf.read_pc().wrapping_add(4) & MEM_MASK);
        self.steps += 1;
        Ok(())
    }

    // ── Memory access ─────────────────────────────────────────────────────────

    fn fetch(&mut self) -> u32 {
        let pc = self.rf.read_pc();
        let addr = (pc & MEM_MASK) as usize;
        // Each byte is masked individually so that wrapping at 0xFFFF is safe.
        let b0 = self.mem[addr];
        let b1 = self.mem[(addr + 1) & (MEM_SIZE - 1)];
        let b2 = self.mem[(addr + 2) & (MEM_SIZE - 1)];
        let b3 = self.mem[(addr + 3) & (MEM_SIZE - 1)];
        self.rf.write_pc(pc.wrapping_add(4) & MEM_MASK);
        ((b0 as u32) << 24) | ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32)
    }

    fn load_word(&self, addr: u32) -> u32 {
        let a = (addr & MEM_MASK) as usize;
        let b0 = self.mem[a];
        let b1 = self.mem[(a + 1) & (MEM_SIZE - 1)];
        let b2 = self.mem[(a + 2) & (MEM_SIZE - 1)];
        let b3 = self.mem[(a + 3) & (MEM_SIZE - 1)];
        ((b0 as u32) << 24) | ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32)
    }

    fn load_half_unsigned(&self, addr: u32) -> u32 {
        let a = (addr & MEM_MASK) as usize;
        let b0 = self.mem[a];
        let b1 = self.mem[(a + 1) & (MEM_SIZE - 1)];
        ((b0 as u32) << 8) | (b1 as u32)
    }

    fn load_half_signed(&self, addr: u32) -> u32 {
        let h = self.load_half_unsigned(addr) as u16;
        (h as i16) as u32
    }

    fn load_byte_unsigned(&self, addr: u32) -> u32 {
        let a = (addr & MEM_MASK) as usize;
        self.mem[a] as u32
    }

    fn load_byte_signed(&self, addr: u32) -> u32 {
        let b = self.mem[(addr & MEM_MASK) as usize] as i8;
        b as u32
    }

    fn store_word(&mut self, addr: u32, val: u32) {
        let a = (addr & MEM_MASK) as usize;
        self.mem.write(a, (val >> 24) as u8);
        self.mem.write((a + 1) & (MEM_SIZE - 1), (val >> 16) as u8);
        self.mem.write((a + 2) & (MEM_SIZE - 1), (val >> 8) as u8);
        self.mem.write((a + 3) & (MEM_SIZE - 1), val as u8);
    }

    fn store_half(&mut self, addr: u32, val: u32) {
        let a = (addr & MEM_MASK) as usize;
        self.mem.write(a, (val >> 8) as u8);
        self.mem.write((a + 1) & (MEM_SIZE - 1), val as u8);
    }

    fn store_byte(&mut self, addr: u32, val: u32) {
        let a = (addr & MEM_MASK) as usize;
        self.mem.write(a, val as u8);
    }

    // ── Instruction dispatch ──────────────────────────────────────────────────

    fn resolve_src2(&self, s: &Src2) -> u32 {
        match s {
            Src2::Reg(r) => self.rf.read(*r),
            Src2::Imm(imm) => *imm,
        }
    }

    fn apply_cc(&mut self, cc: Cc) {
        self.rf.write_psr(Psr {
            n: cc.n != 0,
            z: cc.z != 0,
            v: cc.v != 0,
            c: cc.c != 0,
        });
    }

    fn dispatch(&mut self, instr: Instruction, raw: u32) -> Result<(), SparcError> {
        match instr {
            Instruction::Nop => {}

            Instruction::Call { disp30 } => {
                // Save PC of this instruction into %o7 (logical 15).
                let pc_of_call = self.rf.read_pc().wrapping_sub(4);
                self.rf.write(15, pc_of_call);
                self.rf
                    .write_pc(pc_of_call.wrapping_add(sext30(disp30).wrapping_mul(4)) & MEM_MASK);
            }

            Instruction::Sethi { rd, imm22 } => {
                self.rf.write(rd, alu_sethi(imm22));
            }

            Instruction::Bicc { cond, disp22, .. } => {
                if self.branch_taken(cond) {
                    let disp = sext22(disp22).wrapping_mul(4);
                    self.rf
                        .write_pc(self.rf.read_pc().wrapping_sub(4).wrapping_add(disp) & MEM_MASK);
                }
            }

            Instruction::Ticc { cond, rs1, src2 } => {
                if self.branch_taken(cond) {
                    let trap_num = add32(self.rf.read(rs1), self.resolve_src2(&src2)) & 0x7F;
                    if trap_num == 0 {
                        self.halted = true;
                    }
                }
            }

            Instruction::Alu { op3, rd, rs1, src2 } => {
                let a = self.rf.read(rs1);
                let b = self.resolve_src2(&src2);
                self.exec_alu(op3, rd, rs1, a, b, &src2)?;
            }

            Instruction::Load { op3, rd, rs1, src2 } => {
                let base = self.rf.read(rs1);
                let offset = self.resolve_src2(&src2);
                let addr = add32(base, offset);
                let val = match op3 {
                    0x00 => self.load_word(addr),          // LD
                    0x01 => self.load_byte_unsigned(addr), // LDUB
                    0x02 => self.load_half_unsigned(addr), // LDUH
                    0x09 => self.load_byte_signed(addr),   // LDSB
                    0x0A => self.load_half_signed(addr),   // LDSH
                    _ => {
                        return Err(SparcError::UnknownInstruction {
                            raw,
                            pc: self.rf.read_pc().wrapping_sub(4),
                        })
                    }
                };
                self.rf.write(rd, val);
            }

            Instruction::Store { op3, rd, rs1, src2 } => {
                let base = self.rf.read(rs1);
                let offset = self.resolve_src2(&src2);
                let addr = add32(base, offset);
                let val = self.rf.read(rd);
                match op3 {
                    0x04 => self.store_word(addr, val), // ST
                    0x05 => self.store_byte(addr, val), // STB
                    0x06 => self.store_half(addr, val), // STH
                    _ => {
                        return Err(SparcError::UnknownInstruction {
                            raw,
                            pc: self.rf.read_pc().wrapping_sub(4),
                        })
                    }
                }
            }

            Instruction::Illegal(w) => {
                return Err(SparcError::UnknownInstruction {
                    raw: w,
                    pc: self.rf.read_pc().wrapping_sub(4),
                })
            }
        }
        Ok(())
    }

    fn exec_alu(
        &mut self,
        op3: u8,
        rd: u32,
        _rs1: u32,
        a: u32,
        b: u32,
        _src2: &Src2,
    ) -> Result<(), SparcError> {
        match op3 {
            // ── Arithmetic ────────────────────────────────────────────────────
            0x00 => {
                self.rf.write(rd, add32(a, b));
            } // ADD
            0x10 => {
                let (r, cc) = addcc32(a, b);
                self.rf.write(rd, r);
                self.apply_cc(cc);
            } // ADDcc
            0x08 => {
                self.rf
                    .write(rd, addx32(a, b, u8::from(self.rf.read_psr().c)));
            } // ADDX
            0x18 => {
                // ADDXcc
                let (r, cc) = addxcc32(a, b, u8::from(self.rf.read_psr().c));
                self.rf.write(rd, r);
                self.apply_cc(cc);
            }
            0x04 => {
                self.rf.write(rd, sub32(a, b));
            } // SUB
            0x14 => {
                let (r, cc) = subcc32(a, b);
                self.rf.write(rd, r);
                self.apply_cc(cc);
            } // SUBcc
            0x0C => {
                self.rf
                    .write(rd, subx32(a, b, u8::from(self.rf.read_psr().c)));
            } // SUBX
            0x1C => {
                // SUBXcc
                let (r, cc) = subxcc32(a, b, u8::from(self.rf.read_psr().c));
                self.rf.write(rd, r);
                self.apply_cc(cc);
            }

            // ── Logical ───────────────────────────────────────────────────────
            0x01 => {
                self.rf.write(rd, and32(a, b));
            }
            0x11 => {
                let (r, cc) = andcc32(a, b);
                self.rf.write(rd, r);
                self.apply_cc(cc);
            }
            0x05 => {
                self.rf.write(rd, andn32(a, b));
            }
            0x15 => {
                let (r, cc) = andncc32(a, b);
                self.rf.write(rd, r);
                self.apply_cc(cc);
            }
            0x02 => {
                self.rf.write(rd, or32(a, b));
            }
            0x12 => {
                let (r, cc) = orcc32(a, b);
                self.rf.write(rd, r);
                self.apply_cc(cc);
            }
            0x06 => {
                self.rf.write(rd, orn32(a, b));
            }
            0x16 => {
                let (r, cc) = orncc32(a, b);
                self.rf.write(rd, r);
                self.apply_cc(cc);
            }
            0x03 => {
                self.rf.write(rd, xor32(a, b));
            }
            0x13 => {
                let (r, cc) = xorcc32(a, b);
                self.rf.write(rd, r);
                self.apply_cc(cc);
            }
            0x07 => {
                self.rf.write(rd, xnor32(a, b));
            }
            0x17 => {
                let (r, cc) = xnorcc32(a, b);
                self.rf.write(rd, r);
                self.apply_cc(cc);
            }

            // ── Shifts ────────────────────────────────────────────────────────
            0x25 => {
                self.rf.write(rd, sll32(a, b & 0x1F));
            } // SLL
            0x26 => {
                self.rf.write(rd, srl32(a, b & 0x1F));
            } // SRL
            0x27 => {
                self.rf.write(rd, sra32(a, b & 0x1F));
            } // SRA

            // ── Multiply ──────────────────────────────────────────────────────
            0x0A => {
                // UMUL
                let (y, lo) = umul32(a, b);
                self.rf.write(rd, lo);
                self.rf.write_y(y);
            }
            0x0B => {
                // SMUL
                let (y, lo) = smul32(a, b);
                self.rf.write(rd, lo);
                self.rf.write_y(y);
            }
            0x1A => {
                // UMULcc
                let (y, lo) = umul32(a, b);
                self.rf.write(rd, lo);
                self.rf.write_y(y);
                let bits = u32_to_bits(lo);
                use crate::bits::compute_zero;
                self.rf.write_psr(Psr {
                    n: bits[31] != 0,
                    z: compute_zero(&bits) != 0,
                    v: false,
                    c: false,
                });
            }
            0x1B => {
                // SMULcc
                let (y, lo) = smul32(a, b);
                self.rf.write(rd, lo);
                self.rf.write_y(y);
                let bits = u32_to_bits(lo);
                use crate::bits::compute_zero;
                self.rf.write_psr(Psr {
                    n: bits[31] != 0,
                    z: compute_zero(&bits) != 0,
                    v: false,
                    c: false,
                });
            }

            // ── Divide ────────────────────────────────────────────────────────
            0x0E => {
                self.rf.write(rd, udiv64(self.rf.read_y(), a, b));
            } // UDIV
            0x0F => {
                self.rf.write(rd, sdiv64(self.rf.read_y(), a, b));
            } // SDIV

            0x1E => {
                let y = self.rf.read_y();
                let (result, overflow) = udiv64_with_overflow(y, a, b);
                self.rf.write(rd, result);
                self.rf.write_psr(Psr {
                    n: result >> 31 != 0,
                    z: result == 0,
                    v: overflow,
                    c: false,
                });
            } // UDIVcc
            0x1F => {
                let y = self.rf.read_y();
                let (result, overflow) = sdiv64_with_overflow(y, a, b);
                self.rf.write(rd, result);
                self.rf.write_psr(Psr {
                    n: result >> 31 != 0,
                    z: result == 0,
                    v: overflow,
                    c: false,
                });
            } // SDIVcc

            // ── MULScc ───────────────────────────────────────────────────────
            0x24 => {
                // MULScc
                let psr = self.rf.read_psr();
                let (new_rd, new_y, cc) = mulscc(
                    self.rf.read(rd),
                    self.rf.read_y(),
                    a,
                    b,
                    u8::from(psr.n),
                    u8::from(psr.v),
                );
                self.rf.write(rd, new_rd);
                self.rf.write_y(new_y);
                self.apply_cc(cc);
            }

            // ── Special ───────────────────────────────────────────────────────
            // JMPL: rd = PC of this instruction; PC = rs1 + src2
            0x38 => {
                let pc_of_jmpl = self.rf.read_pc().wrapping_sub(4);
                self.rf.write(rd, pc_of_jmpl);
                self.rf.write_pc(add32(a, b) & MEM_MASK);
            }

            // SAVE
            0x3C => {
                let result = {
                    let rs1_val = a;
                    let src2_val = b;
                    let rf = &mut self.rf;
                    if rf.read_save_depth() >= crate::register_file::NWINDOWS - 1 {
                        return Err(SparcError::WindowOverflow {
                            pc: rf.read_pc().wrapping_sub(4),
                        });
                    }
                    let r = add32(rs1_val, src2_val);
                    rf.rotate_save().expect("SAVE was preflighted");
                    r
                };
                self.rf.write(rd, result);
            }

            // RESTORE
            0x3D => {
                let result = add32(a, b);
                self.rf.rotate_restore();
                self.rf.write(rd, result);
            }

            // RD %y (read Y register)
            0x28 => {
                self.rf.write(rd, self.rf.read_y());
            }

            // WR %y (write Y register): rd=0 means Y, src=rs1 XOR src2
            0x30 => {
                self.rf.write_y(xor32(a, b));
            }

            _ => {
                return Err(SparcError::UnknownInstruction {
                    raw: op3 as u32,
                    pc: self.rf.read_pc().wrapping_sub(4),
                })
            }
        }
        Ok(())
    }

    fn branch_taken(&self, cond: u8) -> bool {
        let Psr { n, z, v, c } = self.rf.read_psr();
        match cond & 0x0f {
            0x0 => false,          // BN
            0x1 => z,              // BE
            0x2 => z || (n != v),  // BLE
            0x3 => n != v,         // BL
            0x4 => c || z,         // BLEU
            0x5 => c,              // BCS
            0x6 => n,              // BNEG
            0x7 => v,              // BVS
            0x8 => true,           // BA
            0x9 => !z,             // BNE
            0xA => !z && (n == v), // BG
            0xB => n == v,         // BGE
            0xC => !c && !z,       // BGU
            0xD => !c,             // BCC
            0xE => !n,             // BPOS
            0xF => !v,             // BVC
            _ => false,
        }
    }
}

impl Default for SparcCpu {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode HALT (ta 0 = `0x91D0_2000`).
    fn halt() -> [u8; 4] {
        HALT_WORD.to_be_bytes()
    }

    /// Encode `sethi imm22, %rd` → op=00, rd, op2=100, imm22.
    fn enc_sethi(rd: u32, imm22: u32) -> u32 {
        (rd << 25) | (0b100 << 22) | (imm22 & 0x003F_FFFF)
    }

    /// Encode `add rs1, rs2, rd` (op=10, op3=0, i=0).
    fn enc_add(rd: u32, rs1: u32, rs2: u32) -> u32 {
        ((0b10u32 << 30) | (rd << 25)) | (rs1 << 14) | rs2
    }

    /// Encode `add rs1, simm13, rd` (op=10, op3=0, i=1).
    fn enc_addi(rd: u32, rs1: u32, imm: i32) -> u32 {
        ((0b10u32 << 30) | (rd << 25)) | (rs1 << 14) | (1 << 13) | ((imm as u32) & 0x1FFF)
    }

    /// Encode `sub rs1, rs2, rd` (op=10, op3=4).
    fn enc_sub(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (4u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `subcc rs1, rs2, rd` (op3=0x14).
    fn enc_subcc(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x14u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `addcc rs1, rs2, rd` (op3=0x10).
    fn enc_addcc(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x10u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `or rs1, rs2, rd` (op3=2).
    fn enc_or(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (2u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `and rs1, rs2, rd` (op3=1).
    fn enc_and(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (1u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `xor rs1, rs2, rd` (op3=3).
    fn enc_xor(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (3u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `sll rs1, rs2, rd` (op3=0x25).
    fn enc_sll(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x25u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `srl rs1, rs2, rd` (op3=0x26).
    fn enc_srl(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x26u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `sra rs1, rs2, rd` (op3=0x27).
    fn enc_sra(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x27u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `st rd, [rs1 + rs2]` (op=11, op3=4, i=0).
    fn enc_st(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b11u32 << 30) | (rd << 25) | (4u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `ld [rs1 + rs2], rd` (op=11, op3=0, i=0).
    fn enc_ld(rd: u32, rs1: u32, rs2: u32) -> u32 {
        ((0b11u32 << 30) | (rd << 25)) | (rs1 << 14) | rs2
    }

    /// Encode `ba disp22` (cond=8, op2=010).
    fn enc_ba(disp22: i32) -> u32 {
        (8u32 << 25) | (0b010u32 << 22) | ((disp22 as u32) & 0x003F_FFFF)
    }

    /// Encode `be disp22` (cond=4).
    fn enc_be(disp22: i32) -> u32 {
        (4u32 << 25) | (0b010u32 << 22) | ((disp22 as u32) & 0x003F_FFFF)
    }

    /// Encode `bne disp22` (cond=12).
    fn enc_bne(disp22: i32) -> u32 {
        (12u32 << 25) | (0b010u32 << 22) | ((disp22 as u32) & 0x003F_FFFF)
    }

    /// Encode `jmpl rs1 + imm13, rd` (op3=0x38, i=1).
    fn enc_jmpl(rd: u32, rs1: u32, imm: i32) -> u32 {
        (0b10u32 << 30)
            | (rd << 25)
            | (0x38u32 << 19)
            | (rs1 << 14)
            | (1 << 13)
            | ((imm as u32) & 0x1FFF)
    }

    /// Encode `umul rs1, rs2, rd` (op3=0x0A).
    fn enc_umul(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x0Au32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `smul rs1, rs2, rd` (op3=0x0B).
    fn enc_smul(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x0Bu32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `udiv rs1, rs2, rd` (op3=0x0E).
    fn enc_udiv(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x0Eu32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `save rs1, imm, rd` (op3=0x3C, i=1).
    fn enc_save(rd: u32, rs1: u32, imm: i32) -> u32 {
        (0b10u32 << 30)
            | (rd << 25)
            | (0x3Cu32 << 19)
            | (rs1 << 14)
            | (1 << 13)
            | ((imm as u32) & 0x1FFF)
    }

    /// Encode `restore rs1, rs2, rd` (op3=0x3D).
    fn enc_restore(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x3Du32 << 19) | (rs1 << 14) | rs2
    }

    fn prog(words: &[u32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(words.len() * 4);
        for &w in words {
            bytes.extend_from_slice(&w.to_be_bytes());
        }
        bytes
    }

    fn run(words: &[u32]) -> SparcCpu {
        let mut cpu = SparcCpu::new();
        let p = prog(words);
        cpu.execute(&p, 0, 10_000).expect("execution failed");
        cpu
    }

    // ── Halt ─────────────────────────────────────────────────────────────────

    #[test]
    fn halts_on_ta0() {
        let cpu = run(&[HALT_WORD]);
        assert!(cpu.halted);
    }

    // ── SETHI + ADD ───────────────────────────────────────────────────────────

    #[test]
    fn sethi_places_value_in_upper_22_bits() {
        // sethi 1, %o0 → %o0 = 1 << 10 = 0x400
        let cpu = run(&[enc_sethi(8, 1), HALT_WORD]);
        assert_eq!(cpu.rf.read(8), 0x400);
    }

    #[test]
    fn add_two_regs() {
        // sethi 0, %o0 = 0; add %o0, %o0, %o1 gives 0+0=0
        // Use immediate: add %g0, 5, %o0; add %g0, 3, %o1; add %o0, %o1, %o2
        let p = prog(&[
            enc_addi(8, 0, 5), // %o0 = 5
            enc_addi(9, 0, 3), // %o1 = 3
            enc_add(10, 8, 9), // %o2 = 8
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 8);
    }

    #[test]
    fn add_immediate() {
        let p = prog(&[enc_addi(8, 0, 42), HALT_WORD]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(8), 42);
    }

    // ── SUB ───────────────────────────────────────────────────────────────────

    #[test]
    fn sub_gives_difference() {
        let p = prog(&[
            enc_addi(8, 0, 10),
            enc_addi(9, 0, 3),
            enc_sub(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 7);
    }

    #[test]
    fn subcc_sets_zero_flag() {
        let p = prog(&[
            enc_addi(8, 0, 5),
            enc_addi(9, 0, 5),
            enc_subcc(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert!(cpu.rf.read_psr().z);
        assert_eq!(cpu.rf.read(10), 0);
    }

    #[test]
    fn subcc_sets_negative_flag() {
        let p = prog(&[
            enc_addi(8, 0, 3),
            enc_addi(9, 0, 5),
            enc_subcc(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert!(cpu.rf.read_psr().n);
        assert!(cpu.rf.read_psr().c); // borrow: 3 < 5
    }

    // ── ADDcc ─────────────────────────────────────────────────────────────────

    #[test]
    fn addcc_carry_on_overflow() {
        // 0xFFFF_FFFF + 1 → carry=1, result=0
        // Load 0xFFFF_FFFF via: add %g0, -1, %o0  (g0=0, simm13=-1 → 0xFFFF_FFFF)
        let p = prog(&[
            enc_addi(8, 0, -1),  // %o0 = 0xFFFF_FFFF
            enc_addi(9, 0, 1),   // %o1 = 1
            enc_addcc(10, 8, 9), // %o2 = 0; C=1
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 0);
        assert!(cpu.rf.read_psr().c);
        assert!(cpu.rf.read_psr().z);
    }

    // ── Logical operations ────────────────────────────────────────────────────

    #[test]
    fn or_combines_bits() {
        let p = prog(&[
            enc_addi(8, 0, 0b1010),
            enc_addi(9, 0, 0b0101),
            enc_or(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 0b1111);
    }

    #[test]
    fn and_masks_bits() {
        let p = prog(&[
            enc_addi(8, 0, 0b1111),
            enc_addi(9, 0, 0b1010),
            enc_and(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 0b1010);
    }

    #[test]
    fn xor_toggles_bits() {
        let p = prog(&[
            enc_addi(8, 0, 0b1010),
            enc_addi(9, 0, 0b1010),
            enc_xor(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 0);
    }

    // ── Shifts ────────────────────────────────────────────────────────────────

    #[test]
    fn sll_shifts_left() {
        let p = prog(&[
            enc_addi(8, 0, 1),
            enc_addi(9, 0, 4),
            enc_sll(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 16);
    }

    #[test]
    fn srl_shifts_right_logical() {
        let p = prog(&[
            enc_addi(8, 0, 0x10),
            enc_addi(9, 0, 4),
            enc_srl(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 1);
    }

    #[test]
    fn sra_preserves_sign() {
        // -8 >> 1 = -4  (arithmetic right shift preserves sign bit)
        let p = prog(&[
            enc_addi(8, 0, -8i32 as u32 as i32), // %o0 = 0xFFFF_FFF8
            enc_addi(9, 0, 1),
            enc_sra(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10) as i32, -4);
    }

    // ── Load/Store ────────────────────────────────────────────────────────────

    #[test]
    fn store_and_load_word() {
        // Store 0xDEAD_BEEF at address 0x100, then load it back.
        // We need to set %o0 = 0xDEAD_BEEF using sethi + or-immediate.
        // sethi  0xDEAD_B, %o0   → %o0 = 0xDEAD_B000  (upper 22 bits)
        // Hmm, 0xDEAD_BEEF upper 22 = 0xDEAD_B = 0x37_AB5 = 0b11_0111_1010_1011_0101
        // Let's use simpler value: store 42 at address 0x100.
        let p = prog(&[
            enc_addi(8, 0, 42),    // %o0 = 42
            enc_addi(9, 0, 0x100), // %o1 = 0x100
            enc_st(8, 9, 0),       // ST %o0, [%o1+%g0]
            enc_addi(10, 0, 0),    // %o2 = 0
            enc_ld(10, 9, 0),      // LD [%o1+%g0], %o2
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 42);
    }

    // ── Branch ────────────────────────────────────────────────────────────────

    #[test]
    fn ba_unconditional_branch() {
        // BA skips one instruction (add that would write 99 to %o0).
        // Layout: [0] ba +2 (skip instr at [1]), [1] addi %o0,0,99, [2] addi %o0,0,7, [3] halt
        // disp22 = 2 (skip 2 words forward counting from branch's PC): jump to [2].
        // But SPARC branch displacement is from *current* PC, not next.
        // Our PC after fetch = addr+4, so disp=3 → PC = addr + 3*4 = [0]+12 = [3].
        // Actually we want to skip [1] and run [2]. From [0]: target = [2] = offset +2.
        // We set disp22=2 → target = pc_of_branch + 2*4 = 0 + 8 = [2]. Correct.
        let p = prog(&[
            enc_ba(2),          // [0] BA → jump to [2]
            enc_addi(8, 0, 99), // [1] skipped
            enc_addi(8, 0, 7),  // [2] %o0 = 7
            HALT_WORD,          // [3]
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(8), 7);
    }

    #[test]
    fn be_taken_when_zero() {
        // subcc %o0, %o0 → Z=1 → be should branch.
        let p = prog(&[
            enc_addi(8, 0, 5),
            enc_subcc(0, 8, 8), // sets Z=1 (discard to %g0)
            enc_be(2),          // branch +2 = skip [3] → go to [4]
            enc_addi(9, 0, 99), // [3] skipped
            enc_addi(9, 0, 42), // [4]
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(9), 42);
    }

    #[test]
    fn bne_not_taken_when_zero() {
        // subcc %o0, %o0 → Z=1 → bne NOT taken.
        let p = prog(&[
            enc_addi(8, 0, 5),
            enc_subcc(0, 8, 8), // sets Z=1
            enc_bne(2),         // NOT taken
            enc_addi(9, 0, 77), // [3] executed
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(9), 77);
    }

    // ── JMPL ─────────────────────────────────────────────────────────────────

    #[test]
    fn jmpl_saves_return_address() {
        // jmpl %o1 + 0, %o7 → PC = %o1; %o7 = PC of jmpl.
        // We set %o1 to point at the halt word.
        // [0] addi %o1, 0, 8   (%o1 = 8 = addr of [2])
        // [1] jmpl %o1+0, %o7  (PC=8, %o7=4)
        // [2] halt
        let p = prog(&[
            enc_addi(9, 0, 8),  // [0] %o1 = 8
            enc_jmpl(15, 9, 0), // [1] JMPL
            HALT_WORD,          // [2]
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(15), 4); // %o7 = PC of jmpl = 4
        assert!(cpu.halted);
    }

    // ── Multiply ─────────────────────────────────────────────────────────────

    #[test]
    fn umul_basic() {
        // 6 * 7 = 42; Y should be 0 (fits in 32 bits).
        let p = prog(&[
            enc_addi(8, 0, 6),
            enc_addi(9, 0, 7),
            enc_umul(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 42);
        assert_eq!(cpu.rf.read_y(), 0);
    }

    #[test]
    fn umul_large() {
        // 0xFFFF_FFFF * 2 = 0x1_FFFF_FFFE → Y=1, rd=0xFFFF_FFFE.
        // Load 0xFFFF_FFFF via: add %g0, -1, %o0
        let p = prog(&[
            enc_addi(8, 0, -1), // %o0 = 0xFFFF_FFFF
            enc_addi(9, 0, 2),
            enc_umul(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 0xFFFF_FFFEu32);
        assert_eq!(cpu.rf.read_y(), 1);
    }

    #[test]
    fn smul_negative() {
        // (-1) * 1 = -1 → Y=0xFFFF_FFFF, rd=0xFFFF_FFFF.
        let p = prog(&[
            enc_addi(8, 0, -1i32), // %o0 = -1 (0xFFFF_FFFF)
            enc_addi(9, 0, 1),
            enc_smul(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 0xFFFF_FFFFu32);
        assert_eq!(cpu.rf.read_y(), 0xFFFF_FFFFu32);
    }

    // ── Divide ────────────────────────────────────────────────────────────────

    #[test]
    fn udiv_basic() {
        // Y=0, %o0=12, %o1=3 → 12/3=4.
        let p = prog(&[
            enc_addi(8, 0, 12),
            enc_addi(9, 0, 3),
            enc_udiv(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 4);
    }

    // ── g0 writes are discarded ───────────────────────────────────────────────

    #[test]
    fn g0_write_discarded() {
        // Writing to %g0 should have no effect.
        let p = prog(&[enc_addi(0, 0, 99), HALT_WORD]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(0), 0);
    }

    // ── Load validation ───────────────────────────────────────────────────────

    #[test]
    fn load_rejects_out_of_range_origin() {
        let mut cpu = SparcCpu::new();
        let r = cpu.load(&[0u8; 4], 0x1_0000);
        assert!(r.is_err());
    }

    #[test]
    fn load_rejects_program_that_overflows() {
        let mut cpu = SparcCpu::new();
        let big = vec![0u8; 0x8001];
        let r = cpu.load(&big, 0x8000);
        assert!(r.is_err());
    }

    // ── SAVE / RESTORE ────────────────────────────────────────────────────────

    #[test]
    fn save_restore_round_trip() {
        // SAVE and RESTORE should leave CWP unchanged.
        let p = prog(&[
            enc_save(14, 0, -64), // SAVE %g0, -64, %sp
            enc_restore(0, 0, 0), // RESTORE %g0, %g0, %g0
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read_cwp(), 0);
        assert_eq!(cpu.rf.read_save_depth(), 0);
    }

    #[test]
    fn save_changes_cwp() {
        let p = prog(&[enc_save(14, 0, -64), HALT_WORD]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        // CWP decremented: (0 + 3 - 1) % 3 = 2
        assert_eq!(cpu.rf.read_cwp(), 2);
        assert_eq!(cpu.rf.read_save_depth(), 1);
    }

    // ── Fetch wrapping ────────────────────────────────────────────────────────

    #[test]
    fn fetch_wraps_at_boundary() {
        // Place a HALT at 0xFFFC; load origin at 0xFFFC.
        // The fetch should read all 4 bytes and wrap correctly.
        let mut cpu = SparcCpu::new();
        let origin = 0xFFFCu32;
        let h = halt();
        let mut prog_bytes = [0u8; 4];
        prog_bytes.copy_from_slice(&h);
        cpu.load(&prog_bytes, origin).unwrap();
        cpu.step().unwrap();
        assert!(cpu.halted);
    }

    // ── NOP ──────────────────────────────────────────────────────────────────

    #[test]
    fn nop_does_nothing() {
        // Standard NOP = 0x0100_0000.
        let p = prog(&[0x0100_0000, HALT_WORD]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert!(cpu.halted);
    }

    // ── SETHI large value ─────────────────────────────────────────────────────

    #[test]
    fn sethi_large() {
        // sethi 0x3FFFFF, %o0 → %o0 = 0xFFFF_FC00
        let p = prog(&[enc_sethi(8, 0x3F_FFFF), HALT_WORD]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(8), 0xFFFF_FC00);
    }
}
