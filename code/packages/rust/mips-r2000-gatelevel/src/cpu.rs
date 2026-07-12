//! cpu.rs — MIPS R2000 gate-level CPU execution engine.
//!
//! # Architecture
//!
//! The MIPS R2000 (1985) is a 32-bit RISC processor using the MIPS I ISA.
//! It has 32 general-purpose registers, separate HI and LO registers for
//! multiplication/division results, and a flat 32-bit address space.
//!
//! # Pipeline model (simplified)
//!
//! ```text
//! FETCH   — read 4 bytes big-endian from mem[PC]; increment PC via gate-level adder
//! DECODE  — extract fields via bit-slice operations (decoder.rs)
//! EXECUTE — dispatch to per-instruction handler; all data ops go through alu.rs
//! WRITEBACK — store results into the register file
//! ```
//!
//! # Memory model
//!
//! 64 KB flat `Vec<u8>`, big-endian.  Words are 4-byte aligned.
//! Memory indexing (assembling bytes into words) uses Rust shifts/OR on
//! raw byte values — this is *memory decoding*, not data-path computation,
//! and is permitted by the gate-level constraint.
//!
//! # Halt sentinel
//!
//! Opcode `SYSCALL` (op=0, funct=0x0C) halts the simulator.  This matches
//! the Python reference which treats SYSCALL as a halt.
//!
//! # Branch/jump target arithmetic
//!
//! Branch targets use `pc_after_fetch + sext(imm16) * 4`.  Jump targets use
//! `(pc_after_fetch & 0xF000_0000) | (target26 << 2)`.  These address
//! computations use Rust integer arithmetic — memory-index arithmetic, not
//! data-path arithmetic — which is allowed.
//!
//! However, the PC is advanced and branch targets are stored through
//! `write_pc`, so all PC updates route through the register file's flip-flop
//! model.
//!
//! # Signed overflow
//!
//! ADD, ADDI, SUB raise `MipsError::SignedOverflow` on overflow.
//! ADDU, ADDIU, SUBU silently wrap.

use crate::alu::{
    add32, and32, div32, divu32, mult32, multu32, nor32, or32, sll32, slt32, sltu32, sra32, srl32,
    sub32, xor32,
};
use crate::bits::{bits_to_u32, int_to_bits32, shl_32};
use crate::decoder::{decode_instruction, InstrFormat};
use crate::register_file::RegisterFile32;

/// Memory size in bytes: 64 KB.
pub const MEM_SIZE: usize = 65536;

/// Register number for $ra (return address, used by JAL/JALR/BGEZAL/BLTZAL).
pub const REG_RA: usize = 31;

/// The halt opcode word: SYSCALL (op=0, funct=0x0C).
pub const HALT_OPCODE_WORD: u32 = 0x0000_000C;

/// Address mask for the 64 KB memory space.
const MEM_MASK: u32 = (MEM_SIZE as u32) - 1;

/// Errors that can occur during MIPS R2000 execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MipsError {
    /// ADD/ADDI/SUB raised a signed overflow exception.
    SignedOverflow(String),
    /// Misaligned memory access.
    Misalignment(String),
    /// BREAK instruction encountered.
    Break(u32),
    /// Unknown opcode or funct.
    UnknownOpcode(u32, u32),
    /// Program does not fit in memory at the given origin.
    InvalidLoad(String),
}

/// The MIPS R2000 gate-level CPU.
pub struct CpuMipsR2000 {
    /// Register file (32 GPRs + HI + LO + PC).
    pub rf: RegisterFile32,
    /// Flat 64 KB big-endian memory.
    pub mem: Vec<u8>,
    /// True after SYSCALL (halt sentinel).
    pub halted: bool,
}

impl CpuMipsR2000 {
    /// Create a new CPU with zeroed registers and memory.
    pub fn new() -> Self {
        Self {
            rf: RegisterFile32::new(),
            mem: vec![0u8; MEM_SIZE],
            halted: false,
        }
    }

    /// Reset CPU to power-on state: zero all registers, clear memory.
    pub fn reset(&mut self) {
        self.rf = RegisterFile32::new();
        self.mem.iter_mut().for_each(|b| *b = 0);
        self.halted = false;
    }

    /// Load `program` bytes into memory at `origin`, then reset.
    ///
    /// Validates that the program fits before touching any CPU state; on
    /// failure the CPU is left unchanged and `Err(MipsError::InvalidLoad)`
    /// is returned so the caller can handle the error without a process abort.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), MipsError> {
        let start = origin as usize;
        if start >= MEM_SIZE {
            return Err(MipsError::InvalidLoad(format!(
                "origin {:#010x} is outside the 64 KB memory range",
                origin
            )));
        }
        let available = MEM_SIZE - start;
        if program.len() > available {
            return Err(MipsError::InvalidLoad(format!(
                "program length {} exceeds available {} bytes at origin {:#06x}",
                program.len(),
                available,
                origin
            )));
        }
        // Validate first, mutate after: reset only on success so that a bad
        // origin/length call leaves the CPU in its previous state.
        self.reset();
        let end = start + program.len();
        self.mem[start..end].copy_from_slice(program);
        self.rf.write_pc(origin);
        Ok(())
    }

    /// Run the program for up to `max_steps` instructions.
    ///
    /// Returns `Ok(steps)` on halt, `Err(MipsError)` on exception.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: u32,
    ) -> Result<u32, MipsError> {
        self.load(program, origin)?;
        let mut steps = 0u32;
        while !self.halted && steps < max_steps {
            self.step()?;
            steps += 1;
        }
        Ok(steps)
    }

    /// Execute one instruction.
    pub fn step(&mut self) -> Result<(), MipsError> {
        if self.halted {
            return Ok(());
        }
        self.execute_one()
    }

    // =========================================================================
    // Internal: fetch
    // =========================================================================

    fn fetch_word(&mut self) -> u32 {
        let addr = (self.rf.read_pc() & MEM_MASK) as usize;
        // Big-endian word assembly — each byte access is individually masked
        // so that a PC near the top of memory wraps safely rather than panicking.
        let b0 = self.mem[addr];
        let b1 = self.mem[(addr + 1) & (MEM_SIZE - 1)];
        let b2 = self.mem[(addr + 2) & (MEM_SIZE - 1)];
        let b3 = self.mem[(addr + 3) & (MEM_SIZE - 1)];
        let iw = ((b0 as u32) << 24) | ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);
        // Gate-level PC increment via ripple-carry adder
        self.rf.increment_pc(4);
        iw
    }

    // =========================================================================
    // Internal: memory access
    // =========================================================================

    fn eff_addr(&self, base: u32, offset: i32) -> u32 {
        // Address arithmetic (memory indexing) is permitted.
        base.wrapping_add(offset as u32) & MEM_MASK
    }

    fn check_align(&self, addr: u32, size: u32) -> Result<(), MipsError> {
        if addr & (size - 1) != 0 {
            return Err(MipsError::Misalignment(format!(
                "Misaligned {} access at {:#06x}",
                if size == 4 { "word" } else { "halfword" },
                addr
            )));
        }
        Ok(())
    }

    fn load_byte(&self, addr: u32) -> u8 {
        self.mem[(addr & MEM_MASK) as usize]
    }

    fn load_half(&self, addr: u32) -> Result<u16, MipsError> {
        self.check_align(addr, 2)?;
        let a = (addr & MEM_MASK) as usize;
        Ok(((self.mem[a] as u16) << 8) | self.mem[a + 1] as u16)
    }

    fn load_word(&self, addr: u32) -> Result<u32, MipsError> {
        self.check_align(addr, 4)?;
        let a = (addr & MEM_MASK) as usize;
        Ok(((self.mem[a] as u32) << 24)
            | ((self.mem[a + 1] as u32) << 16)
            | ((self.mem[a + 2] as u32) << 8)
            | (self.mem[a + 3] as u32))
    }

    fn store_byte(&mut self, addr: u32, val: u8) {
        self.mem[(addr & MEM_MASK) as usize] = val;
    }

    fn store_half(&mut self, addr: u32, val: u16) -> Result<(), MipsError> {
        self.check_align(addr, 2)?;
        let a = (addr & MEM_MASK) as usize;
        self.mem[a] = (val >> 8) as u8;
        self.mem[a + 1] = val as u8;
        Ok(())
    }

    fn store_word(&mut self, addr: u32, val: u32) -> Result<(), MipsError> {
        self.check_align(addr, 4)?;
        let a = (addr & MEM_MASK) as usize;
        self.mem[a] = (val >> 24) as u8;
        self.mem[a + 1] = (val >> 16) as u8;
        self.mem[a + 2] = (val >> 8) as u8;
        self.mem[a + 3] = val as u8;
        Ok(())
    }

    // =========================================================================
    // Internal: instruction execution
    // =========================================================================

    fn execute_one(&mut self) -> Result<(), MipsError> {
        let iw = self.fetch_word();

        // HALT: SYSCALL (op=0, funct=0x0C) or the explicit HALT word
        if iw == HALT_OPCODE_WORD || (iw >> 26 == 0 && (iw & 0x3F) == 0x0C) {
            self.halted = true;
            return Ok(());
        }

        // NOP: canonical SLL $zero, $zero, 0
        if iw == 0x0000_0000 {
            return Ok(());
        }

        let d = decode_instruction(iw);

        match d.format {
            InstrFormat::R => self.exec_r_type(d),
            InstrFormat::J => {
                if d.op == 2 {
                    self.exec_j(d.target26);
                } else {
                    self.exec_jal(d.target26);
                }
                Ok(())
            }
            InstrFormat::I => self.exec_i_type(d),
        }
    }

    // =========================================================================
    // R-type dispatch
    // =========================================================================

    fn exec_r_type(&mut self, d: crate::decoder::DecodedInstruction) -> Result<(), MipsError> {
        let rs = d.rs as usize;
        let rt = d.rt as usize;
        let rd = d.rd as usize;
        let shamt = d.shamt as u32;
        let funct = d.funct;
        let pc_after = self.rf.read_pc(); // already past instruction

        let rs_val = self.rf.read_reg(rs);
        let rt_val = self.rf.read_reg(rt);

        match funct {
            // ── Shifts ───────────────────────────────────────────────────────
            0x00 => {
                // SLL rd, rt, shamt
                self.rf.write_reg(rd, sll32(rt_val, shamt).result);
            }
            0x02 => {
                // SRL rd, rt, shamt
                self.rf.write_reg(rd, srl32(rt_val, shamt).result);
            }
            0x03 => {
                // SRA rd, rt, shamt
                self.rf.write_reg(rd, sra32(rt_val, shamt).result);
            }
            0x04 => {
                // SLLV rd, rt, rs — shift amount from lower 5 bits of rs
                let sa_bits = int_to_bits32(rs_val);
                let mut sa_arr = [0u8; 32];
                sa_arr[..5].copy_from_slice(&sa_bits[..5]);
                let sa = bits_to_u32(sa_arr);
                self.rf.write_reg(rd, sll32(rt_val, sa).result);
            }
            0x06 => {
                // SRLV rd, rt, rs
                let sa_bits = int_to_bits32(rs_val);
                let mut sa_arr = [0u8; 32];
                sa_arr[..5].copy_from_slice(&sa_bits[..5]);
                let sa = bits_to_u32(sa_arr);
                self.rf.write_reg(rd, srl32(rt_val, sa).result);
            }
            0x07 => {
                // SRAV rd, rt, rs
                let sa_bits = int_to_bits32(rs_val);
                let mut sa_arr = [0u8; 32];
                sa_arr[..5].copy_from_slice(&sa_bits[..5]);
                let sa = bits_to_u32(sa_arr);
                self.rf.write_reg(rd, sra32(rt_val, sa).result);
            }

            // ── Jumps ─────────────────────────────────────────────────────────
            0x08 => {
                // JR rs
                self.rf.write_pc(rs_val & MEM_MASK);
            }
            0x09 => {
                // JALR rd, rs
                self.rf.write_reg(rd, pc_after);
                self.rf.write_pc(rs_val & MEM_MASK);
            }

            // ── BREAK ──────────────────────────────────────────────────────────
            0x0D => {
                let pc_instr = self.rf.read_pc().wrapping_sub(4) & MEM_MASK;
                return Err(MipsError::Break(pc_instr));
            }

            // ── HI/LO moves ────────────────────────────────────────────────────
            0x10 => {
                // MFHI rd
                self.rf.write_reg(rd, self.rf.read_hi());
            }
            0x11 => {
                // MTHI rs
                self.rf.write_hi(rs_val);
            }
            0x12 => {
                // MFLO rd
                self.rf.write_reg(rd, self.rf.read_lo());
            }
            0x13 => {
                // MTLO rs
                self.rf.write_lo(rs_val);
            }

            // ── Multiply ───────────────────────────────────────────────────────
            0x18 => {
                // MULT rs, rt (signed)
                let (hi, lo) = mult32(rs_val, rt_val);
                self.rf.write_hi(hi);
                self.rf.write_lo(lo);
            }
            0x19 => {
                // MULTU rs, rt (unsigned)
                let (hi, lo) = multu32(rs_val, rt_val);
                self.rf.write_hi(hi);
                self.rf.write_lo(lo);
            }

            // ── Divide ─────────────────────────────────────────────────────────
            0x1A => {
                // DIV rs, rt (signed)
                let (q, r) = div32(rs_val, rt_val);
                self.rf.write_lo(q);
                self.rf.write_hi(r);
            }
            0x1B => {
                // DIVU rs, rt (unsigned)
                let (q, r) = divu32(rs_val, rt_val);
                self.rf.write_lo(q);
                self.rf.write_hi(r);
            }

            // ── Arithmetic ─────────────────────────────────────────────────────
            0x20 => {
                // ADD rd, rs, rt (signed; trap on overflow)
                let r = add32(rs_val, rt_val, 0);
                if r.overflow != 0 {
                    return Err(MipsError::SignedOverflow(format!(
                        "ADD: {:#010x} + {:#010x}",
                        rs_val, rt_val
                    )));
                }
                self.rf.write_reg(rd, r.result);
            }
            0x21 => {
                // ADDU rd, rs, rt (wraps silently)
                self.rf.write_reg(rd, add32(rs_val, rt_val, 0).result);
            }
            0x22 => {
                // SUB rd, rs, rt (signed; trap on overflow)
                let r = sub32(rs_val, rt_val);
                if r.overflow != 0 {
                    return Err(MipsError::SignedOverflow(format!(
                        "SUB: {:#010x} - {:#010x}",
                        rs_val, rt_val
                    )));
                }
                self.rf.write_reg(rd, r.result);
            }
            0x23 => {
                // SUBU rd, rs, rt (wraps silently)
                self.rf.write_reg(rd, sub32(rs_val, rt_val).result);
            }
            0x24 => {
                self.rf.write_reg(rd, and32(rs_val, rt_val).result);
            }
            0x25 => {
                self.rf.write_reg(rd, or32(rs_val, rt_val).result);
            }
            0x26 => {
                self.rf.write_reg(rd, xor32(rs_val, rt_val).result);
            }
            0x27 => {
                self.rf.write_reg(rd, nor32(rs_val, rt_val).result);
            }
            0x2A => {
                // SLT rd, rs, rt (signed)
                self.rf.write_reg(rd, slt32(rs_val, rt_val).result);
            }
            0x2B => {
                // SLTU rd, rs, rt (unsigned)
                self.rf.write_reg(rd, sltu32(rs_val, rt_val).result);
            }

            _ => {
                let pc_instr = self.rf.read_pc().wrapping_sub(4) & MEM_MASK;
                return Err(MipsError::UnknownOpcode(funct as u32, pc_instr));
            }
        }

        Ok(())
    }

    // =========================================================================
    // J-type instructions
    // =========================================================================

    fn exec_j(&mut self, target26: u32) {
        let pc_now = self.rf.read_pc(); // already past instruction
        // Target = (pc_now & 0xF000_0000) | (target26 << 2)
        let shifted = shl_32(target26, 2);
        let new_pc = ((pc_now & 0xF000_0000) | (shifted & 0x0FFF_FFFF)) & MEM_MASK;
        self.rf.write_pc(new_pc);
    }

    fn exec_jal(&mut self, target26: u32) {
        let pc_now = self.rf.read_pc();
        self.rf.write_reg(REG_RA, pc_now);
        let shifted = shl_32(target26, 2);
        let new_pc = ((pc_now & 0xF000_0000) | (shifted & 0x0FFF_FFFF)) & MEM_MASK;
        self.rf.write_pc(new_pc);
    }

    // =========================================================================
    // I-type dispatch
    // =========================================================================

    fn exec_i_type(&mut self, d: crate::decoder::DecodedInstruction) -> Result<(), MipsError> {
        let rs = d.rs as usize;
        let rt = d.rt as usize;
        let imm = d.imm16;
        let op = d.op;

        let rs_val = self.rf.read_reg(rs);

        match op {
            // ── REGIMM (op=0x01): branch on rs sign ──────────────────────────
            0x01 => {
                self.exec_regimm(rs_val, d.rt, imm);
            }

            // ── Branches ─────────────────────────────────────────────────────
            0x04 => {
                // BEQ rs, rt, offset
                let rt_val = self.rf.read_reg(rt);
                let eq_r = xor32(rs_val, rt_val);
                if eq_r.zero != 0 {
                    let pc_now = self.rf.read_pc();
                    let target = pc_now.wrapping_add((imm as u32).wrapping_mul(4)) & MEM_MASK;
                    self.rf.write_pc(target);
                }
            }
            0x05 => {
                // BNE rs, rt, offset
                let rt_val = self.rf.read_reg(rt);
                let eq_r = xor32(rs_val, rt_val);
                if eq_r.zero == 0 {
                    let pc_now = self.rf.read_pc();
                    let target = pc_now.wrapping_add((imm as u32).wrapping_mul(4)) & MEM_MASK;
                    self.rf.write_pc(target);
                }
            }
            0x06 => {
                // BLEZ rs, offset — branch if signed(rs) <= 0
                let rs_bits = int_to_bits32(rs_val);
                let rs_neg = rs_bits[31];
                let rs_zero = crate::bits::compute_zero(rs_val);
                if rs_neg != 0 || rs_zero != 0 {
                    let pc_now = self.rf.read_pc();
                    let target = pc_now.wrapping_add((imm as u32).wrapping_mul(4)) & MEM_MASK;
                    self.rf.write_pc(target);
                }
            }
            0x07 => {
                // BGTZ rs, offset — branch if signed(rs) > 0
                let rs_bits = int_to_bits32(rs_val);
                let rs_neg = rs_bits[31];
                let rs_zero = crate::bits::compute_zero(rs_val);
                if rs_neg == 0 && rs_zero == 0 {
                    let pc_now = self.rf.read_pc();
                    let target = pc_now.wrapping_add((imm as u32).wrapping_mul(4)) & MEM_MASK;
                    self.rf.write_pc(target);
                }
            }

            // ── Arithmetic / logic immediates ─────────────────────────────────
            0x08 => {
                // ADDI rt, rs, imm — signed; trap on overflow
                let imm_u = imm as u32;
                let r = add32(rs_val, imm_u, 0);
                if r.overflow != 0 {
                    return Err(MipsError::SignedOverflow(format!(
                        "ADDI: {:#010x} + {}",
                        rs_val, imm
                    )));
                }
                self.rf.write_reg(rt, r.result);
            }
            0x09 => {
                // ADDIU rt, rs, imm — wraps silently
                let imm_u = imm as u32;
                self.rf.write_reg(rt, add32(rs_val, imm_u, 0).result);
            }
            0x0A => {
                // SLTI rt, rs, imm — signed comparison
                let imm_u = imm as u32;
                self.rf.write_reg(rt, slt32(rs_val, imm_u).result);
            }
            0x0B => {
                // SLTIU rt, rs, imm — unsigned comparison (imm still sign-extended)
                let imm_u = imm as u32;
                self.rf.write_reg(rt, sltu32(rs_val, imm_u).result);
            }
            0x0C => {
                // ANDI rt, rs, imm — zero-extend immediate (mask off sign-extension)
                let imm_u = (imm as u32) & 0xFFFF;
                self.rf.write_reg(rt, and32(rs_val, imm_u).result);
            }
            0x0D => {
                // ORI rt, rs, imm — zero-extend immediate
                let imm_u = (imm as u32) & 0xFFFF;
                self.rf.write_reg(rt, or32(rs_val, imm_u).result);
            }
            0x0E => {
                // XORI rt, rs, imm — zero-extend immediate
                let imm_u = (imm as u32) & 0xFFFF;
                self.rf.write_reg(rt, xor32(rs_val, imm_u).result);
            }
            0x0F => {
                // LUI rt, imm — load imm into upper 16 bits, lower 16 = 0
                let imm_u = (imm as u32) & 0xFFFF;
                let val = shl_32(imm_u, 16);
                self.rf.write_reg(rt, val);
            }

            // ── Loads ─────────────────────────────────────────────────────────
            0x20 => {
                // LB rt, off(rs) — load byte, sign-extend
                let ea = self.eff_addr(rs_val, imm);
                let byte = self.load_byte(ea);
                let byte_bits = int_to_bits32(byte as u32);
                let sign = byte_bits[7];
                let mut extended = [0u8; 32];
                extended[..8].copy_from_slice(&byte_bits[..8]);
                extended[8..].fill(sign);
                self.rf.write_reg(rt, bits_to_u32(extended));
            }
            0x21 => {
                // LH rt, off(rs) — load halfword, sign-extend
                let ea = self.eff_addr(rs_val, imm);
                let half = self.load_half(ea)?;
                let half_bits = int_to_bits32(half as u32);
                let sign = half_bits[15];
                let mut extended = [0u8; 32];
                extended[..16].copy_from_slice(&half_bits[..16]);
                extended[16..].fill(sign);
                self.rf.write_reg(rt, bits_to_u32(extended));
            }
            0x22 => {
                // LWL rt, off(rs) — unaligned load left
                let ea = self.eff_addr(rs_val, imm);
                let byte_offset = (ea & 3) as usize; // 0=MSB, 3=LSB in big-endian
                let word_addr = ea & !3;
                let mem_word = self.load_word(word_addr)?;
                let rt_val = self.rf.read_reg(rt);
                // LWL loads bytes from mem_word[0..byte_offset] into HIGH bytes of rt.
                // shift = (3 - byte_offset) * 8 = low bits preserved from rt.
                let shift = (3 - byte_offset) * 8;
                let result = if shift == 0 {
                    mem_word
                } else {
                    let mem_mask = 0xFFFF_FFFFu32 ^ ((1u32 << shift) - 1);
                    let rt_mask = (1u32 << shift) - 1;
                    (mem_word & mem_mask) | (rt_val & rt_mask)
                };
                self.rf.write_reg(rt, result);
            }
            0x23 => {
                // LW rt, off(rs) — load word (4-byte aligned)
                let ea = self.eff_addr(rs_val, imm);
                let word = self.load_word(ea)?;
                self.rf.write_reg(rt, word);
            }
            0x24 => {
                // LBU rt, off(rs) — load byte, zero-extend
                let ea = self.eff_addr(rs_val, imm);
                self.rf.write_reg(rt, self.load_byte(ea) as u32);
            }
            0x25 => {
                // LHU rt, off(rs) — load halfword, zero-extend
                let ea = self.eff_addr(rs_val, imm);
                self.rf.write_reg(rt, self.load_half(ea)? as u32);
            }
            0x26 => {
                // LWR rt, off(rs) — unaligned load right
                let ea = self.eff_addr(rs_val, imm);
                let byte_offset = (ea & 3) as usize;
                let word_addr = ea & !3;
                let mem_word = self.load_word(word_addr)?;
                let rt_val = self.rf.read_reg(rt);
                // LWR loads bytes from mem_word[byte_offset..3] into LOW bytes of rt.
                // shift = byte_offset * 8 = high bits preserved from rt.
                let shift = byte_offset * 8;
                let result = if shift == 0 {
                    mem_word
                } else {
                    // byte_offset in 1..=3 here (shift==0 branch handles 0),
                    // so shift in {8,16,24} and low_bits in {24,16,8} — never 0 or 32,
                    // so 1u32 << low_bits is always in-range (no shift-amount panic).
                    let low_bits = 32 - shift;
                    let rt_mask = 0xFFFF_FFFFu32 ^ ((1u32 << low_bits) - 1);
                    let mem_mask = (1u32 << low_bits) - 1;
                    (rt_val & rt_mask) | (mem_word & mem_mask)
                };
                self.rf.write_reg(rt, result);
            }

            // ── Stores ─────────────────────────────────────────────────────────
            0x28 => {
                // SB rt, off(rs) — store least-significant byte
                let rt_val = self.rf.read_reg(rt);
                let ea = self.eff_addr(rs_val, imm);
                let rt_bits = int_to_bits32(rt_val);
                let mut byte_bits = [0u8; 32];
                byte_bits[..8].copy_from_slice(&rt_bits[..8]);
                self.store_byte(ea, bits_to_u32(byte_bits) as u8);
            }
            0x29 => {
                // SH rt, off(rs) — store least-significant halfword
                let rt_val = self.rf.read_reg(rt);
                let ea = self.eff_addr(rs_val, imm);
                let rt_bits = int_to_bits32(rt_val);
                let mut half_bits = [0u8; 32];
                half_bits[..16].copy_from_slice(&rt_bits[..16]);
                self.store_half(ea, bits_to_u32(half_bits) as u16)?;
            }
            0x2A => {
                // SWL rt, off(rs) — unaligned store left
                let rt_val = self.rf.read_reg(rt);
                let ea = self.eff_addr(rs_val, imm);
                let byte_offset = (ea & 3) as usize;
                let word_addr = ea & !3;
                let mem_word = self.load_word(word_addr)?;
                // SWL stores top (byte_offset+1) bytes of rt into memory high bytes.
                let shift = (3 - byte_offset) * 8; // low bits preserved from mem
                let result = if shift == 0 {
                    rt_val
                } else {
                    let mem_mask = (1u32 << shift) - 1;
                    let rt_mask = 0xFFFF_FFFFu32 ^ mem_mask;
                    (rt_val & rt_mask) | (mem_word & mem_mask)
                };
                self.store_word(word_addr, result)?;
            }
            0x2B => {
                // SW rt, off(rs) — store word (4-byte aligned)
                let rt_val = self.rf.read_reg(rt);
                let ea = self.eff_addr(rs_val, imm);
                self.store_word(ea, rt_val)?;
            }
            0x2E => {
                // SWR rt, off(rs) — unaligned store right
                let rt_val = self.rf.read_reg(rt);
                let ea = self.eff_addr(rs_val, imm);
                let byte_offset = (ea & 3) as usize;
                let word_addr = ea & !3;
                let mem_word = self.load_word(word_addr)?;
                // SWR stores low (4-byte_offset) bytes of rt into memory low bytes.
                let shift = byte_offset * 8; // high bits preserved from mem
                let result = if shift == 0 {
                    rt_val
                } else {
                    // byte_offset in 1..=3 here, so low_bits in {24,16,8} — no shift-amount panic.
                    let low_bits = 32 - shift;
                    let rt_mask = (1u32 << low_bits) - 1;
                    let mem_mask = 0xFFFF_FFFFu32 ^ rt_mask;
                    (mem_word & mem_mask) | (rt_val & rt_mask)
                };
                self.store_word(word_addr, result)?;
            }

            _ => {
                let pc_instr = self.rf.read_pc().wrapping_sub(4) & MEM_MASK;
                return Err(MipsError::UnknownOpcode(op as u32, pc_instr));
            }
        }

        Ok(())
    }

    // =========================================================================
    // REGIMM (op=0x01)
    // =========================================================================

    fn exec_regimm(&mut self, rs_val: u32, rt: u8, offset: i32) {
        let rs_bits = int_to_bits32(rs_val);
        let rs_negative = rs_bits[31];
        let pc_now = self.rf.read_pc();

        match rt {
            0x00 => {
                // BLTZ: branch if rs < 0
                if rs_negative != 0 {
                    let target = pc_now.wrapping_add((offset as u32).wrapping_mul(4)) & MEM_MASK;
                    self.rf.write_pc(target);
                }
            }
            0x01 => {
                // BGEZ: branch if rs >= 0
                if rs_negative == 0 {
                    let target = pc_now.wrapping_add((offset as u32).wrapping_mul(4)) & MEM_MASK;
                    self.rf.write_pc(target);
                }
            }
            0x10 => {
                // BLTZAL: $ra = pc_now; branch if rs < 0
                self.rf.write_reg(REG_RA, pc_now);
                if rs_negative != 0 {
                    let target = pc_now.wrapping_add((offset as u32).wrapping_mul(4)) & MEM_MASK;
                    self.rf.write_pc(target);
                }
            }
            0x11 => {
                // BGEZAL: $ra = pc_now; branch if rs >= 0
                self.rf.write_reg(REG_RA, pc_now);
                if rs_negative == 0 {
                    let target = pc_now.wrapping_add((offset as u32).wrapping_mul(4)) & MEM_MASK;
                    self.rf.write_pc(target);
                }
            }
            _ => {
                // Unknown REGIMM variant — silently ignore (like NOP)
            }
        }
    }
}

impl Default for CpuMipsR2000 {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Unit tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Encode instructions as big-endian u32 → 4 bytes
    fn encode(word: u32) -> [u8; 4] {
        [
            (word >> 24) as u8,
            (word >> 16) as u8,
            (word >> 8) as u8,
            word as u8,
        ]
    }

    fn halt() -> [u8; 4] {
        encode(HALT_OPCODE_WORD)
    }

    // R-type: op=0, rs, rt, rd, shamt, funct
    fn r_instr(rs: u8, rt: u8, rd: u8, shamt: u8, funct: u8) -> u32 {
        ((rs as u32) << 21) | ((rt as u32) << 16) | ((rd as u32) << 11) | ((shamt as u32) << 6) | (funct as u32)
    }

    // I-type: op, rs, rt, imm16
    fn i_instr(op: u8, rs: u8, rt: u8, imm16: i16) -> u32 {
        ((op as u32) << 26) | ((rs as u32) << 21) | ((rt as u32) << 16) | ((imm16 as u16) as u32)
    }

    // J-type: op, target26
    fn j_instr(op: u8, target26: u32) -> u32 {
        ((op as u32) << 26) | (target26 & 0x03FF_FFFF)
    }

    // ── ADDU / SUBU ──────────────────────────────────────────────────────────

    #[test]
    fn test_addu_basic() {
        let mut cpu = CpuMipsR2000::new();
        // ADDIU $t0, $zero, 10   → $t0 = 10
        // ADDIU $t1, $zero, 20   → $t1 = 20
        // ADDU  $t2, $t0, $t1   → $t2 = 30
        // HALT
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 10)),
            encode(i_instr(0x09, 0, 9, 20)),
            encode(r_instr(8, 9, 10, 0, 0x21)),
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 30);
    }

    #[test]
    fn test_subu_basic() {
        let mut cpu = CpuMipsR2000::new();
        // $t0=50, $t1=20; SUBU $t2, $t0, $t1 → 30
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 50)),
            encode(i_instr(0x09, 0, 9, 20)),
            encode(r_instr(8, 9, 10, 0, 0x23)),
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 30);
    }

    #[test]
    fn test_add_overflow() {
        let mut cpu = CpuMipsR2000::new();
        // LUI $t0, 0x7FFF → 0x7FFF_0000
        // ORI $t0, $t0, 0xFFFF → 0x7FFF_FFFF
        // ADDIU $t1, $zero, 1
        // ADD $t2, $t0, $t1  → signed overflow
        let prog: Vec<u8> = [
            encode(i_instr(0x0F, 0, 8, 0x7FFF_u16 as i16)),
            encode(i_instr(0x0D, 8, 8, -1i16)), // ORI 0xFFFF zero-extends
            encode(i_instr(0x09, 0, 9, 1)),
            encode(r_instr(8, 9, 10, 0, 0x20)), // ADD
            halt(),
        ]
        .concat();
        let result = cpu.execute(&prog, 0, 100);
        assert!(matches!(result, Err(MipsError::SignedOverflow(_))));
    }

    #[test]
    fn test_sub_overflow() {
        let mut cpu = CpuMipsR2000::new();
        // LUI $t0, 0x8000 → $t0 = 0x8000_0000 (MIN_INT)
        // ADDIU $t1, $zero, 1
        // SUB $t2, $t0, $t1 → overflow
        let prog: Vec<u8> = [
            encode(i_instr(0x0F, 0, 8, -0x8000i16)), // LUI with 0x8000
            encode(i_instr(0x09, 0, 9, 1)),
            encode(r_instr(8, 9, 10, 0, 0x22)), // SUB
            halt(),
        ]
        .concat();
        let result = cpu.execute(&prog, 0, 100);
        assert!(matches!(result, Err(MipsError::SignedOverflow(_))));
    }

    // ── Shifts ───────────────────────────────────────────────────────────────

    #[test]
    fn test_sll_srl_sra() {
        let mut cpu = CpuMipsR2000::new();
        // $t0 = 8; SLL $t1,$t0,2 → 32; SRL $t2,$t0,1 → 4
        // $t3 = 0x8000_0000; SRA $t4, $t3, 1 → 0xC000_0000
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 8)),                   // $t0 = 8
            encode(r_instr(0, 8, 9, 2, 0x00)),                 // SLL $t1, $t0, 2
            encode(r_instr(0, 8, 10, 1, 0x02)),                // SRL $t2, $t0, 1
            encode(i_instr(0x0F, 0, 11, -0x8000i16)),          // LUI $t3, 0x8000
            encode(r_instr(0, 11, 12, 1, 0x03)),               // SRA $t4, $t3, 1
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(9), 32);
        assert_eq!(cpu.rf.read_reg(10), 4);
        assert_eq!(cpu.rf.read_reg(12), 0xC000_0000);
    }

    #[test]
    fn test_sllv_srlv_srav() {
        let mut cpu = CpuMipsR2000::new();
        // $t0=4 (shift), $t1=1; SLLV $t2,$t1,$t0 → 16
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 4)),      // $t0 = 4 (shift amount)
            encode(i_instr(0x09, 0, 9, 1)),       // $t1 = 1 (value)
            encode(r_instr(8, 9, 10, 0, 0x04)),   // SLLV $t2, $t1, $t0
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 16);
    }

    // ── AND / OR / XOR / NOR ─────────────────────────────────────────────────

    #[test]
    fn test_logical_ops() {
        let mut cpu = CpuMipsR2000::new();
        // $t0=0xFF, $t1=0x0F
        // AND $t2,$t0,$t1 → 0x0F
        // OR  $t3,$t0,$t1 → 0xFF
        // XOR $t4,$t0,$t1 → 0xF0
        // NOR $t5,$t0,$zero → 0xFFFF_FF00
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 0xFF)),
            encode(i_instr(0x09, 0, 9, 0x0F)),
            encode(r_instr(8, 9, 10, 0, 0x24)), // AND
            encode(r_instr(8, 9, 11, 0, 0x25)), // OR
            encode(r_instr(8, 9, 12, 0, 0x26)), // XOR
            encode(r_instr(8, 0, 13, 0, 0x27)), // NOR $t5,$t0,$zero
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 0x0F);
        assert_eq!(cpu.rf.read_reg(11), 0xFF);
        assert_eq!(cpu.rf.read_reg(12), 0xF0);
        assert_eq!(cpu.rf.read_reg(13), 0xFFFF_FF00);
    }

    // ── SLT / SLTU ───────────────────────────────────────────────────────────

    #[test]
    fn test_slt_sltu() {
        let mut cpu = CpuMipsR2000::new();
        // $t0 = 0x8000_0000 (MIN_INT signed, max unsigned)
        // $t1 = 1
        // SLT  $t2,$t0,$t1 → 1 (MIN_INT < 1 signed)
        // SLTU $t3,$t0,$t1 → 0 (0x8000_0000 > 1 unsigned)
        let prog: Vec<u8> = [
            encode(i_instr(0x0F, 0, 8, -0x8000i16)), // LUI 0x8000
            encode(i_instr(0x09, 0, 9, 1)),
            encode(r_instr(8, 9, 10, 0, 0x2A)),       // SLT
            encode(r_instr(8, 9, 11, 0, 0x2B)),       // SLTU
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 1); // SLT: MIN_INT < 1 signed
        assert_eq!(cpu.rf.read_reg(11), 0); // SLTU: 0x8000_0000 > 1 unsigned
    }

    // ── Multiply / divide ─────────────────────────────────────────────────────

    #[test]
    fn test_multu_mfhi_mflo() {
        let mut cpu = CpuMipsR2000::new();
        // $t0=6, $t1=7; MULTU; LO=42, HI=0
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 6)),
            encode(i_instr(0x09, 0, 9, 7)),
            encode(r_instr(8, 9, 0, 0, 0x19)), // MULTU
            encode(r_instr(0, 0, 10, 0, 0x12)), // MFLO $t2
            encode(r_instr(0, 0, 11, 0, 0x10)), // MFHI $t3
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 42);
        assert_eq!(cpu.rf.read_reg(11), 0);
    }

    #[test]
    fn test_divu_mflo_mfhi() {
        let mut cpu = CpuMipsR2000::new();
        // 10 / 3: q=3, r=1
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 10)),
            encode(i_instr(0x09, 0, 9, 3)),
            encode(r_instr(8, 9, 0, 0, 0x1B)),  // DIVU
            encode(r_instr(0, 0, 10, 0, 0x12)), // MFLO (quotient)
            encode(r_instr(0, 0, 11, 0, 0x10)), // MFHI (remainder)
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 3);
        assert_eq!(cpu.rf.read_reg(11), 1);
    }

    #[test]
    fn test_mult_signed() {
        let mut cpu = CpuMipsR2000::new();
        // -1 * -1 = 1 (via LUI+ADDIU for -1)
        // ADDIU $t0, $zero, -1 → 0xFFFF_FFFF
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, -1i16)), // $t0 = 0xFFFF_FFFF (-1)
            encode(i_instr(0x09, 0, 9, -1i16)), // $t1 = 0xFFFF_FFFF (-1)
            encode(r_instr(8, 9, 0, 0, 0x18)),  // MULT
            encode(r_instr(0, 0, 10, 0, 0x12)), // MFLO → 1
            encode(r_instr(0, 0, 11, 0, 0x10)), // MFHI → 0
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 1);
        assert_eq!(cpu.rf.read_reg(11), 0);
    }

    // ── HI/LO move instructions ───────────────────────────────────────────────

    #[test]
    fn test_mthi_mtlo() {
        let mut cpu = CpuMipsR2000::new();
        // $t0=0xDEAD; MTHI $t0; MFHI $t1 → 0xDEAD
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 0x1234)),
            encode(r_instr(8, 0, 0, 0, 0x11)),  // MTHI $t0
            encode(r_instr(0, 0, 9, 0, 0x10)),  // MFHI $t1
            encode(i_instr(0x09, 0, 10, 0x5678)),
            encode(r_instr(10, 0, 0, 0, 0x13)), // MTLO
            encode(r_instr(0, 0, 11, 0, 0x12)), // MFLO
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(9), 0x1234);
        assert_eq!(cpu.rf.read_reg(11), 0x5678);
    }

    // ── Branches ─────────────────────────────────────────────────────────────

    #[test]
    fn test_beq_taken() {
        let mut cpu = CpuMipsR2000::new();
        // $t0=$t1=5; BEQ $t0,$t1,2 → skip 2 instructions (jump to HALT)
        // Next: two NOPs then HALT
        // If BEQ taken: skip NOPs, land on HALT at offset +8+8=16 bytes
        // Layout: [ADDIU t0,5][ADDIU t1,5][BEQ t0,t1,+2][NOP][NOP][ADDIU t2,99][HALT]
        // Offset is in instructions past BEQ's PC+4 = 3*4=12, +2*4=8 → target=20
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 5)),            // 0: $t0=5
            encode(i_instr(0x09, 0, 9, 5)),            // 4: $t1=5
            encode(i_instr(0x04, 8, 9, 2)),             // 8: BEQ $t0,$t1,+2 → PC=12+8=20
            encode(0x0000_0000u32),                      // 12: NOP
            encode(0x0000_0000u32),                      // 16: NOP
            encode(i_instr(0x09, 0, 10, 99)),           // 20: $t2=99  ← skipped if taken?
            halt(),                                      // 24: HALT
        ]
        .concat();
        // BEQ taken: PC after BEQ fetch=12, target=12+2*4=20, so exec 20 (ADDIU t2,99), then HALT
        // Actually we jump to 20 which is ADDIU t2,99, then halt
        // Let me rethink: BEQ at addr 8, fetch advances PC to 12
        // target = 12 + 2*4 = 20
        // At addr 20: ADDIU $t2, $zero, 99 → $t2 = 99
        // At addr 24: HALT
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 99); // The ADDIU executes (BEQ target is the ADDIU)
        // BEQ skips two NOPs at 12,16 and lands on ADDIU at 20 then HALT at 24
    }

    #[test]
    fn test_bne_not_taken() {
        let mut cpu = CpuMipsR2000::new();
        // $t0=5, $t1=5; BNE $t0,$t1,5 → not taken; $t2=1
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 5)),
            encode(i_instr(0x09, 0, 9, 5)),
            encode(i_instr(0x05, 8, 9, 5)),   // BNE not taken
            encode(i_instr(0x09, 0, 10, 1)),  // $t2=1 (should execute)
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 1);
    }

    #[test]
    fn test_blez_bgtz() {
        let mut cpu = CpuMipsR2000::new();
        // $t0=-1; BLEZ $t0,1 → taken (skip NOP, hit ADDIU t1,1, then HALT)
        // Then $t1=1 (after the skip)
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, -1i16)),   // 0: $t0=-1
            encode(i_instr(0x06, 8, 0, 1)),        // 4: BLEZ $t0,+1 → target=8+4=12
            encode(0x0000_0000u32),                 // 8: NOP (skipped)
            encode(i_instr(0x09, 0, 9, 1)),        // 12: $t1=1 ← lands here
            halt(),                                 // 16: HALT
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(9), 1);
    }

    #[test]
    fn test_bltz_bgezal() {
        let mut cpu = CpuMipsR2000::new();
        // $t0=1 (positive); BGEZ $t0,1 → taken (rs>=0)
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 1)),        // $t0=1
            encode(i_instr(0x01, 8, 0x01, 1)),     // BGEZ $t0,+1 → target=8+4=12
            encode(0x0000_0000u32),                 // NOP (skipped)
            encode(i_instr(0x09, 0, 9, 42)),        // $t1=42 ← lands here
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(9), 42);
    }

    // ── JR / JALR ─────────────────────────────────────────────────────────────

    #[test]
    fn test_jr() {
        let mut cpu = CpuMipsR2000::new();
        // $t0 points to HALT word; JR $t0 → halt
        // Layout: [JR $t0][ADDIU $t1,99][HALT]
        // We want JR to jump to the HALT at byte 8.
        let halt_addr = 8u32;
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, halt_addr as i16)), // $t0=8
            encode(r_instr(8, 0, 0, 0, 0x08)),              // JR $t0
            encode(i_instr(0x09, 0, 9, 99)),                // ADDIU (skipped)
            halt(),                                          // addr 12
        ]
        .concat();
        // JR jumps to addr 8, but halt() is at addr 12... let me fix the layout.
        // The halt should be at the address $t0 points to.
        // $t0 = halt_addr = 8; JR $t0 → fetch from 8 → which is ADDIU $t1,99
        // That's wrong. Let me recalculate:
        // Byte 0: ADDIU $t0, $zero, 12 (set $t0 = address of HALT)
        // Byte 4: JR $t0
        // Byte 8: ADDIU $t1, $zero, 99 (skipped)
        // Byte 12: HALT
        drop(prog);
        let halt_addr = 12u32;
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, halt_addr as i16)), // 0: $t0=12
            encode(r_instr(8, 0, 0, 0, 0x08)),              // 4: JR $t0 → jump to 12
            encode(i_instr(0x09, 0, 9, 99)),                // 8: ADDIU (skipped)
            halt(),                                          // 12: HALT
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert!(cpu.halted);
        assert_eq!(cpu.rf.read_reg(9), 0); // ADDIU was skipped
    }

    #[test]
    fn test_jal_jr() {
        let mut cpu = CpuMipsR2000::new();
        // JAL to subroutine; subroutine sets $t0=42, JR $ra back; then HALT
        // Memory layout (all in the 64KB 16-bit word space):
        // 0x0000: JAL target=8>>2=2   → target26=2, new_pc = 0|8 = 8
        // 0x0004: HALT (if JAL falls through — shouldn't)
        // 0x0008: ADDIU $t0,$zero,42
        // 0x000C: JR $ra
        // 0x0010: HALT
        let prog: Vec<u8> = [
            encode(j_instr(0x03, 2)),                // 0: JAL 2 (target26=2 → PC = 0|(2<<2) = 8)
            halt(),                                  // 4: (shouldn't reach)
            encode(i_instr(0x09, 0, 8, 42)),         // 8: $t0=42
            encode(r_instr(31, 0, 0, 0, 0x08)),      // 12: JR $ra
            halt(),                                  // 16: HALT (return point)
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(8), 42);
        // $ra should be 4 (the instruction after JAL)
        assert_eq!(cpu.rf.read_reg(REG_RA), 4);
    }

    // ── Immediate operations ──────────────────────────────────────────────────

    #[test]
    fn test_lui_ori() {
        let mut cpu = CpuMipsR2000::new();
        // LUI $t0, 0xBEEF → 0xBEEF_0000
        // ORI $t0, $t0, 0xCAFE → 0xBEEF_CAFE
        let prog: Vec<u8> = [
            encode(i_instr(0x0F, 0, 8, -0x4111i16)), // LUI 0xBEEF (0xBEEF = -0x4111 signed)
            encode(i_instr(0x0D, 8, 8, -0x3502i16)), // ORI 0xCAFE (zero-extends)
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        // LUI 0xBEEF = 0xBEEF_0000; ORI 0xCAFE masks to low 16 bits → 0xBEEF_CAFE
        assert_eq!(cpu.rf.read_reg(8), 0xBEEF_0000 | 0xCAFE);
    }

    #[test]
    fn test_andi_xori() {
        let mut cpu = CpuMipsR2000::new();
        // $t0=0xFF; ANDI $t1,$t0,0x0F → 0x0F; XORI $t2,$t0,0x55 → 0xAA
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 0xFF)),
            encode(i_instr(0x0C, 8, 9, 0x0F)),   // ANDI
            encode(i_instr(0x0E, 8, 10, 0x55)),  // XORI
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(9), 0x0F);
        assert_eq!(cpu.rf.read_reg(10), 0xAA);
    }

    #[test]
    fn test_slti_sltiu() {
        let mut cpu = CpuMipsR2000::new();
        // $t0=-5 (0xFFFF_FFFB); SLTI $t1,$t0,0 → 1 (-5<0 signed)
        // SLTIU $t2,$t0,1 → 0 (0xFFFF_FFFB > 1 unsigned)
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, -5i16)),    // $t0=-5
            encode(i_instr(0x0A, 8, 9, 0)),         // SLTI $t1,$t0,0 → 1
            encode(i_instr(0x0B, 8, 10, 1)),        // SLTIU $t2,$t0,1 → 0
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(9), 1);
        assert_eq!(cpu.rf.read_reg(10), 0);
    }

    // ── Load / Store ─────────────────────────────────────────────────────────

    #[test]
    fn test_sw_lw() {
        let mut cpu = CpuMipsR2000::new();
        // Store 0xDEAD_BEEF to address 0x100; load it back
        // $t0 = 0x100 (base), $t1 = 0xDEAD_BEEF
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 0x100)),        // $t0=0x100
            encode(i_instr(0x0F, 0, 9, -0x2153i16)),   // LUI 0xDEAD
            encode(i_instr(0x0D, 9, 9, -0x4111i16)),   // ORI 0xBEEF (zero-ext)
            encode(i_instr(0x2B, 8, 9, 0)),             // SW $t1, 0($t0)
            encode(i_instr(0x23, 8, 10, 0)),            // LW $t2, 0($t0)
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 0xDEAD_BEEF);
    }

    #[test]
    fn test_sb_lbu_lb() {
        let mut cpu = CpuMipsR2000::new();
        // Store byte 0xFF to addr 0x200; LBU (zero-extend) and LB (sign-extend)
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 0x200)),       // $t0=0x200
            encode(i_instr(0x09, 0, 9, -1i16)),       // $t1=0xFF
            encode(i_instr(0x28, 8, 9, 0)),            // SB $t1, 0($t0)
            encode(i_instr(0x24, 8, 10, 0)),           // LBU → 0xFF (zero-ext)
            encode(i_instr(0x20, 8, 11, 0)),           // LB  → 0xFFFF_FFFF (sign-ext)
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 0xFF);
        assert_eq!(cpu.rf.read_reg(11), 0xFFFF_FFFF);
    }

    #[test]
    fn test_sh_lhu_lh() {
        let mut cpu = CpuMipsR2000::new();
        // Store 0x8001 as halfword; LHU and LH
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 0x300)),       // $t0=0x300
            encode(i_instr(0x0F, 0, 9, -0x7FFFi16)),  // LUI 0x8001... actually LUI 0x8001
            encode(i_instr(0x09, 0, 9, -0x7FFFi16)),  // $t1=0x8001 via ADDIU
            encode(i_instr(0x29, 8, 9, 0)),            // SH $t1, 0($t0)
            encode(i_instr(0x25, 8, 10, 0)),           // LHU → 0x8001 (zero-ext)
            encode(i_instr(0x21, 8, 11, 0)),           // LH  → 0xFFFF_8001 (sign-ext)
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 0x8001);
        assert_eq!(cpu.rf.read_reg(11), 0xFFFF_8001);
    }

    // ── R0 is always zero ─────────────────────────────────────────────────────

    #[test]
    fn test_r0_always_zero() {
        let mut cpu = CpuMipsR2000::new();
        // Try to write to R0 via ADDU $zero, $zero, $t0 (rd=0)
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 99)),           // $t0=99
            encode(r_instr(0, 8, 0, 0, 0x21)),         // ADDU $zero,$zero,$t0 (rd=0, discarded)
            encode(r_instr(0, 0, 9, 0, 0x21)),         // ADDU $t1,$zero,$zero → 0
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(0), 0);
        assert_eq!(cpu.rf.read_reg(9), 0);
    }

    // ── BLTZAL / BGEZAL ───────────────────────────────────────────────────────

    #[test]
    fn test_bgezal() {
        let mut cpu = CpuMipsR2000::new();
        // $t0=5 (>= 0); BGEZAL $t0, 1 → $ra = pc_after, branch taken
        // Layout: [ADDIU $t0,5][BGEZAL +1][NOP][ADDIU $t1,42][HALT]
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 5)),          // 0: $t0=5
            encode(i_instr(0x01, 8, 0x11, 1)),       // 4: BGEZAL $t0,+1 → target=8+4=12
            encode(0x0000_0000u32),                   // 8: NOP (skipped)
            encode(i_instr(0x09, 0, 9, 42)),          // 12: $t1=42
            halt(),                                   // 16: HALT
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(9), 42);
        assert_eq!(cpu.rf.read_reg(REG_RA), 8); // $ra = pc after BGEZAL
    }

    // ── Unaligned loads/stores ────────────────────────────────────────────────

    #[test]
    fn test_lwl_lwr() {
        let mut cpu = CpuMipsR2000::new();
        // Store 0x1234_5678 at address 0x400 (aligned)
        // LWL $t1, 1($t0) (ea=0x401, byte_offset=1, load bytes 0,1 → high 16 bits)
        // LWR $t1, 3($t0) (ea=0x403, byte_offset=3, load byte 3 → low byte)
        // Together LWL+LWR loads the word from an unaligned address
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 0x400)),         // $t0=0x400
            encode(i_instr(0x0F, 0, 9, 0x1234)),        // LUI $t1, 0x1234
            encode(i_instr(0x0D, 9, 9, 0x5678)),        // ORI $t1, $t1, 0x5678
            encode(i_instr(0x2B, 8, 9, 0)),              // SW $t1, 0($t0) → mem[0x400]=0x1234_5678
            // LWL byte_offset=1: loads mem bytes 0,1 (0x12,0x34) into high 2 bytes of $t2
            // $t2 initial=0; result = (0x1234_5678 & 0xFFFF_0000) | (0 & 0x0000_FFFF) = 0x1234_0000
            encode(i_instr(0x22, 8, 10, 1)),             // LWL $t2, 1($t0) ea=0x401
            // Now load LWR byte_offset=1: loads bytes 1,2,3 (0x34,0x56,0x78) into low 3 bytes
            // $t3 initial=0; result = (0 & 0xFF00_0000) | (0x1234_5678 & 0x00FF_FFFF) = 0x0034_5678
            encode(i_instr(0x26, 8, 11, 1)),             // LWR $t3, 1($t0) ea=0x401
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 0x1234_0000);
        assert_eq!(cpu.rf.read_reg(11), 0x0034_5678);
    }

    // ── Misalignment check ────────────────────────────────────────────────────

    #[test]
    fn test_misaligned_lw_error() {
        let mut cpu = CpuMipsR2000::new();
        // Try to LW from address 0x101 (not 4-byte aligned)
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 0x101)), // $t0=0x101
            encode(i_instr(0x23, 8, 9, 0)),      // LW $t1, 0($t0) — misaligned
            halt(),
        ]
        .concat();
        let result = cpu.execute(&prog, 0, 100);
        assert!(matches!(result, Err(MipsError::Misalignment(_))));
    }

    // ── NOP ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_nop() {
        let mut cpu = CpuMipsR2000::new();
        let prog: Vec<u8> = [
            encode(0x0000_0000u32), // NOP
            encode(0x0000_0000u32), // NOP
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert!(cpu.halted);
        // All registers still 0
        for i in 0..32 {
            assert_eq!(cpu.rf.read_reg(i), 0);
        }
    }

    // ── DIV signed ───────────────────────────────────────────────────────────

    #[test]
    fn test_div_signed() {
        let mut cpu = CpuMipsR2000::new();
        // -10 / 3 = -3 remainder -1
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, -10i16)),   // $t0 = -10
            encode(i_instr(0x09, 0, 9, 3)),         // $t1 = 3
            encode(r_instr(8, 9, 0, 0, 0x1A)),      // DIV
            encode(r_instr(0, 0, 10, 0, 0x12)),     // MFLO → quotient
            encode(r_instr(0, 0, 11, 0, 0x10)),     // MFHI → remainder
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10) as i32, -3);
        assert_eq!(cpu.rf.read_reg(11) as i32, -1);
    }

    // ── BREAK ────────────────────────────────────────────────────────────────

    #[test]
    fn test_break_raises_error() {
        let mut cpu = CpuMipsR2000::new();
        let prog: Vec<u8> = [
            encode(r_instr(0, 0, 0, 0, 0x0D)), // BREAK
            halt(),
        ]
        .concat();
        let result = cpu.execute(&prog, 0, 100);
        assert!(matches!(result, Err(MipsError::Break(_))));
    }

    // ── Iteration-count guard (multu large values) ────────────────────────────

    #[test]
    fn test_multu_large() {
        let mut cpu = CpuMipsR2000::new();
        // 0xFFFF_FFFF * 0xFFFF_FFFF = 0xFFFF_FFFE_0000_0001
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, -1i16)),     // $t0=0xFFFF_FFFF
            encode(r_instr(8, 8, 0, 0, 0x19)),       // MULTU $t0,$t0
            encode(r_instr(0, 0, 9, 0, 0x12)),       // MFLO
            encode(r_instr(0, 0, 10, 0, 0x10)),      // MFHI
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(9), 0x0000_0001); // lo
        assert_eq!(cpu.rf.read_reg(10), 0xFFFF_FFFE); // hi
    }

    // ── SWL / SWR ────────────────────────────────────────────────────────────

    #[test]
    fn test_swl_swr() {
        let mut cpu = CpuMipsR2000::new();
        // Write 0x1234_5678 to 0x500 (aligned).
        // SWL: store MSB bytes of $t1 into mem starting at unaligned addr.
        // SWR: store LSB bytes.
        // For SWL at ea=0x501 (byte_offset=1):
        //   stores top 2 bytes of $t1 (0x12,0x34) to mem[0x500,0x501]
        //   mem was 0x1234_5678; after SWL: mem[0x500]=0x12,0x501=0x34
        //   Actually SWL byte_offset=1: stores rt bytes 0..1 (MSB 0x12,0x34) → mem[0x500..0x501]
        //   preserved low bits: shift=(3-1)*8=16 → mem keeps lower 16 bits 0x5678
        //   result: (0xDEAD_BEEF & 0xFFFF_0000) | (0x1234_5678 & 0x0000_FFFF)
        //         = 0xDEAD_0000 | 0x5678 = 0xDEAD_5678
        let prog: Vec<u8> = [
            encode(i_instr(0x09, 0, 8, 0x500)),         // $t0=0x500
            // Set mem[0x500]=0x1234_5678
            encode(i_instr(0x0F, 0, 9, 0x1234)),        // LUI 0x1234
            encode(i_instr(0x0D, 9, 9, 0x5678)),        // ORI 0x5678
            encode(i_instr(0x2B, 8, 9, 0)),              // SW $t1, 0($t0)
            // Set $t1=0xDEAD_BEEF
            encode(i_instr(0x0F, 0, 9, -0x2153i16)),    // LUI 0xDEAD
            encode(i_instr(0x0D, 9, 9, -0x4111i16)),    // ORI 0xBEEF
            // SWL at ea=0x501, byte_offset=1: store high 2 bytes of $t1 into mem[0x500..0x501]
            // shift=(3-1)*8=16; mem keeps low 16 bits; rt provides high 16 bits
            // result = (0xDEAD_BEEF & 0xFFFF_0000) | (0x1234_5678 & 0x0000_FFFF)
            //        = 0xDEAD_0000 | 0x0000_5678 = 0xDEAD_5678
            encode(i_instr(0x2A, 8, 9, 1)),              // SWL $t1, 1($t0)
            encode(i_instr(0x23, 8, 10, 0)),             // LW $t2, 0($t0) → verify
            halt(),
        ]
        .concat();
        cpu.execute(&prog, 0, 100).unwrap();
        assert_eq!(cpu.rf.read_reg(10), 0xDEAD_5678);
    }
}
