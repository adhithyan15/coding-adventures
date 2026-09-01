//! Top-level Zilog Z80 simulator combining all components.
//!
//! The checked lifecycle uses the Z80's architectural 64 KiB address space.
//! Invalid images, ports, states, halted steps, and undefined instructions
//! return typed errors without changing state. Bounded runs are transactional
//! and successful instructions return complete before/after traces.

use cpu_simulator::Memory;

use crate::decode;
use crate::execute;
use crate::execute::{ExecuteResult, Flags, Registers};

const MEMORY_SIZE: usize = 65_536;
const NUM_PORTS: usize = 256;

/// Complete Zilog Z80 simulator.
pub struct Z80Simulator {
    /// Main bank (A/B/C/D/E/H/L) + alternate bank (A'/F'/B'/C'/D'/E'/H'/L')
    /// + IX/IY/SP/I/R.
    pub regs: Registers,
    /// Main-bank condition flags (S, Z, H, P/V, N, C).
    pub flags: Flags,
    /// Flat 64 KiB byte-addressable memory.
    pub mem: Memory,
    /// Program counter.
    pub pc: u16,
    /// True once `HALT` has executed.
    pub halted: bool,
    /// IFF1 — maskable-interrupt enable flip-flop.  Set by `EI`, cleared
    /// by `DI`.
    pub iff1: bool,
    /// IFF2 — shadow of IFF1, preserved by NMI.
    pub iff2: bool,
    /// Interrupt mode (0, 1, or 2).
    pub im: u8,
    /// 256 input ports, read by `IN A,(n)`.
    pub input_ports: [u8; 256],
    /// 256 output ports, written by `OUT (n),A`.
    pub output_ports: [u8; 256],
}

/// A checked Z80 simulator failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Z80Error {
    ProgramTooLarge { length: usize, capacity: usize },
    InvalidStateMemory { length: usize },
    UnknownOpcode { address: u16, raw: Vec<u8> },
    InvalidPort { port: usize },
    Halted,
}

impl std::fmt::Display for Z80Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProgramTooLarge { length, capacity } => {
                write!(
                    formatter,
                    "program of {length} bytes exceeds {capacity}-byte memory"
                )
            }
            Self::InvalidStateMemory { length } => {
                write!(
                    formatter,
                    "state has {length} memory bytes; expected {MEMORY_SIZE}"
                )
            }
            Self::UnknownOpcode { address, raw } => {
                write!(
                    formatter,
                    "unknown opcode bytes {raw:02X?} at {address:#06X}"
                )
            }
            Self::InvalidPort { port } => write!(formatter, "port {port} is outside 0..255"),
            Self::Halted => formatter.write_str("CPU is halted"),
        }
    }
}

impl std::error::Error for Z80Error {}

/// Complete owned architectural state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Z80State {
    pub regs: Registers,
    pub flags: Flags,
    pub memory: Box<[u8]>,
    pub pc: u16,
    pub halted: bool,
    pub iff1: bool,
    pub iff2: bool,
    pub im: u8,
    pub input_ports: [u8; NUM_PORTS],
    pub output_ports: [u8; NUM_PORTS],
}

/// Complete record of one successfully executed instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub address: u16,
    pub raw: Vec<u8>,
    pub mnemonic: String,
    pub state_before: Z80State,
    pub state_after: Z80State,
}

/// Observable outcome of a bounded simulator run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub pc: u16,
    pub traces: Vec<StepTrace>,
    pub final_state: Z80State,
}

impl Z80Simulator {
    /// Create a new simulator. The size argument is retained for source
    /// compatibility; the architectural address space is always 64 KiB.
    pub fn new(_memory_size: usize) -> Self {
        Self {
            regs: Registers::default(),
            flags: Flags::default(),
            mem: Memory::new(MEMORY_SIZE),
            pc: 0,
            halted: false,
            iff1: false,
            iff2: false,
            im: 0,
            input_ports: [0u8; NUM_PORTS],
            output_ports: [0u8; NUM_PORTS],
        }
    }

    /// Reset all architectural state.
    pub fn reset(&mut self) {
        *self = Self::new(MEMORY_SIZE);
    }

    /// Load a program at address zero without changing other state.
    pub fn load_program(&mut self, program: &[u8]) -> Result<(), Z80Error> {
        self.load_program_at(program, 0)
    }

    /// Load bytes at an arbitrary origin, wrapping through the 16-bit address
    /// space exactly once. Oversized images are rejected atomically.
    pub fn load_program_at(&mut self, program: &[u8], origin: u16) -> Result<(), Z80Error> {
        if program.len() > MEMORY_SIZE {
            return Err(Z80Error::ProgramTooLarge {
                length: program.len(),
                capacity: MEMORY_SIZE,
            });
        }
        for (offset, byte) in program.iter().copied().enumerate() {
            let address = origin.wrapping_add(offset as u16);
            self.mem.write_byte(address as usize, byte);
        }
        self.pc = origin;
        self.halted = false;
        Ok(())
    }

    /// Return a complete owned snapshot.
    pub fn snapshot(&self) -> Z80State {
        Z80State {
            regs: self.regs,
            flags: self.flags,
            memory: (0..MEMORY_SIZE)
                .map(|address| self.mem.read_byte(address))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            pc: self.pc,
            halted: self.halted,
            iff1: self.iff1,
            iff2: self.iff2,
            im: self.im,
            input_ports: self.input_ports,
            output_ports: self.output_ports,
        }
    }

    /// Restore a complete owned snapshot.
    pub fn restore(&mut self, state: &Z80State) -> Result<(), Z80Error> {
        if state.memory.len() != MEMORY_SIZE {
            return Err(Z80Error::InvalidStateMemory {
                length: state.memory.len(),
            });
        }
        self.regs = state.regs;
        self.flags = state.flags;
        self.mem.load_bytes(0, &state.memory);
        self.pc = state.pc;
        self.halted = state.halted;
        self.iff1 = state.iff1;
        self.iff2 = state.iff2;
        self.im = state.im;
        self.input_ports = state.input_ports;
        self.output_ports = state.output_ports;
        Ok(())
    }

    /// Set a checked input port latch.
    pub fn set_input_port(&mut self, port: usize, value: u8) -> Result<(), Z80Error> {
        let latch = self
            .input_ports
            .get_mut(port)
            .ok_or(Z80Error::InvalidPort { port })?;
        *latch = value;
        Ok(())
    }

    /// Read a checked output port latch.
    pub fn get_output_port(&self, port: usize) -> Result<u8, Z80Error> {
        self.output_ports
            .get(port)
            .copied()
            .ok_or(Z80Error::InvalidPort { port })
    }

    /// Load and run a fresh candidate transactionally.
    pub fn run(&mut self, program: &[u8], max_steps: usize) -> Result<ExecutionResult, Z80Error> {
        let mut candidate = Self::new(MEMORY_SIZE);
        candidate.input_ports = self.input_ports;
        candidate.load_program(program)?;
        let result = candidate.run_loaded_with_limit(max_steps)?;
        *self = candidate;
        Ok(result)
    }

    /// Run already-loaded code with a default safety bound.
    pub fn run_loaded(&mut self) -> Result<ExecutionResult, Z80Error> {
        self.run_loaded_with_limit(1_000_000)
    }

    /// Run already-loaded code transactionally for at most `max_steps`.
    pub fn run_loaded_with_limit(&mut self, max_steps: usize) -> Result<ExecutionResult, Z80Error> {
        let before = self.snapshot();
        let mut traces = Vec::new();
        while traces.len() < max_steps && !self.halted {
            match self.step() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.restore(&before)?;
                    return Err(error);
                }
            }
        }
        Ok(ExecutionResult {
            halted: self.halted,
            steps: traces.len(),
            pc: self.pc,
            traces,
            final_state: self.snapshot(),
        })
    }

    /// Execute one instruction and return a complete trace.
    pub fn step(&mut self) -> Result<StepTrace, Z80Error> {
        if self.halted {
            return Err(Z80Error::Halted);
        }

        let address = self.pc;
        let first_byte = self.mem.read_byte(address as usize);
        let mut cursor = address.wrapping_add(1);
        let mem_ref = &self.mem;
        let decoded = decode::decode(first_byte, &mut || {
            let b = mem_ref.read_byte(cursor as usize);
            cursor = cursor.wrapping_add(1);
            b
        });
        if decoded.mnemonic == "undefined" {
            return Err(Z80Error::UnknownOpcode {
                address,
                raw: decoded.raw,
            });
        }
        let state_before = self.snapshot();
        let mnemonic = decoded.mnemonic.clone();
        let raw = decoded.raw.clone();
        let fallthrough_pc = cursor;
        let refreshes = raw.len() as u8;
        self.regs.r = (self.regs.r & 0x80) | (self.regs.r.wrapping_add(refreshes) & 0x7F);

        let ExecuteResult { next_pc, halted } = execute::execute(
            &decoded,
            &mut self.regs,
            &mut self.flags,
            &mut self.mem,
            &self.input_ports,
            &mut self.output_ports,
            &mut self.iff1,
            &mut self.iff2,
            &mut self.im,
            fallthrough_pc,
        );
        self.pc = next_pc;
        self.halted = halted;

        Ok(StepTrace {
            address,
            raw,
            mnemonic,
            state_before,
            state_after: self.snapshot(),
        })
    }

    /// Run a sequence of pre-encoded instructions.
    pub fn run_instructions(
        &mut self,
        instructions: &[Vec<u8>],
        max_steps: usize,
    ) -> Result<ExecutionResult, Z80Error> {
        let program = crate::encoding::assemble(instructions);
        self.run(&program, max_steps)
    }

    /// Fire a maskable interrupt. Returns whether it was accepted.
    pub fn interrupt(&mut self, data: u8) -> bool {
        if !self.iff1 {
            return false;
        }
        self.iff1 = false;
        self.iff2 = false;
        self.halted = false;
        self.regs.sp = self.regs.sp.wrapping_sub(2);
        self.mem.write_byte(self.regs.sp as usize, self.pc as u8);
        self.mem
            .write_byte(self.regs.sp.wrapping_add(1) as usize, (self.pc >> 8) as u8);
        self.pc = match self.im {
            0 => u16::from(data & 0x38),
            1 => 0x0038,
            _ => {
                let vector = (u16::from(self.regs.i) << 8) | u16::from(data & 0xFE);
                let lo = self.mem.read_byte(vector as usize) as u16;
                let hi = self.mem.read_byte(vector.wrapping_add(1) as usize) as u16;
                (hi << 8) | lo
            }
        };
        true
    }

    /// Fire a non-maskable interrupt.
    pub fn nmi(&mut self) {
        self.iff2 = self.iff1;
        self.iff1 = false;
        self.halted = false;
        self.regs.sp = self.regs.sp.wrapping_sub(2);
        self.mem.write_byte(self.regs.sp as usize, self.pc as u8);
        self.mem
            .write_byte(self.regs.sp.wrapping_add(1) as usize, (self.pc >> 8) as u8);
        self.pc = 0x0066;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::*;
    use crate::opcodes::*;

    fn run_program(instructions: &[Vec<u8>]) -> Z80Simulator {
        let mut sim = Z80Simulator::new(65536);
        sim.run_instructions(instructions, 10_000).unwrap();
        sim
    }

    // ── the trivial "load immediate into accumulator + HALT" sequence
    // the z80-backend smoke test relies on — byte-identical to
    // intel8080-simulator's canonical MVI A,42; HLT ──
    #[test]
    fn ld_a_n_then_halt() {
        let mut sim = Z80Simulator::new(65536);
        sim.load_program(&assemble(&[encode_ld_a_n(42), vec![HALT]]))
            .unwrap();
        let result = sim.run_loaded_with_limit(10).unwrap();
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(sim.regs.a, 42);
    }

    #[test]
    fn bounded_run_reports_halt_and_instruction_count() {
        let mut sim = Z80Simulator::new(65536);
        sim.load_program(&assemble(&[encode_ld_r_n(REG_B, 7), vec![HALT]]))
            .unwrap();
        let result = sim.run_loaded_with_limit(10).unwrap();
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(sim.regs.b, 7);
    }

    // ── ALU ops ──
    #[test]
    fn test_add_sub() {
        let sim = run_program(&[
            encode_ld_r_n(REG_B, 10),
            encode_ld_r_n(REG_C, 20),
            encode_ld_a_n(10),
            vec![encode_alu_reg(ALU_ADD, REG_C)], // A = 10 + 20 = 30
            vec![HALT],
        ]);
        assert_eq!(sim.regs.a, 30);
        assert!(!sim.flags.c);
    }

    #[test]
    fn test_add_overflow_sets_carry() {
        let sim = run_program(&[encode_ld_a_n(0xFF), encode_alu_imm(ALU_ADD, 1), vec![HALT]]);
        assert_eq!(sim.regs.a, 0x00);
        assert!(sim.flags.c, "0xFF + 1 should set carry");
        assert!(sim.flags.z, "result is 0");
    }

    #[test]
    fn test_sub_borrow_sets_carry_and_n() {
        let sim = run_program(&[encode_ld_a_n(0x00), encode_alu_imm(ALU_SUB, 1), vec![HALT]]);
        assert_eq!(sim.regs.a, 0xFF);
        assert!(sim.flags.c, "0x00 - 1 borrows");
        assert!(sim.flags.s);
        assert!(sim.flags.n, "N is set after SUB");
    }

    #[test]
    fn test_and_or_xor_parity() {
        let sim = run_program(&[
            encode_ld_a_n(0xFF),
            encode_ld_r_n(REG_B, 0x0F),
            vec![encode_alu_reg(ALU_AND, REG_B)],
            vec![HALT],
        ]);
        assert_eq!(sim.regs.a, 0x0F);
        assert!(!sim.flags.c, "AND always clears carry");
        assert!(sim.flags.h, "AND always sets H on Z80");
        assert!(sim.flags.pv, "0x0F has even parity (4 ones)");
    }

    #[test]
    fn test_cp_does_not_write_a() {
        let sim = run_program(&[
            encode_ld_a_n(5),
            encode_ld_r_n(REG_B, 5),
            vec![encode_alu_reg(ALU_CP, REG_B)],
            vec![HALT],
        ]);
        assert_eq!(sim.regs.a, 5, "CP must not write A");
        assert!(sim.flags.z, "5 == 5");
    }

    #[test]
    fn test_inc_dec_wrap_and_overflow_flag() {
        let sim = run_program(&[
            encode_ld_r_n(REG_B, 0xFF),
            vec![encode_inc_r(REG_B)],
            vec![HALT],
        ]);
        assert_eq!(sim.regs.b, 0x00);
        assert!(sim.flags.z);

        let sim2 = run_program(&[
            encode_ld_r_n(REG_B, 0x7F),
            vec![encode_inc_r(REG_B)],
            vec![HALT],
        ]);
        assert_eq!(sim2.regs.b, 0x80);
        assert!(sim2.flags.pv, "INC 0x7F -> 0x80 is a signed overflow");
    }

    // ── Load / store ──
    #[test]
    fn test_ld_nn_a_and_ld_a_nn_round_trip() {
        let sim = run_program(&[
            encode_ld_a_n(0x42),
            encode_ld_nn_a(0x0100),
            encode_ld_a_n(0x00),
            encode_ld_a_nn(0x0100),
            vec![HALT],
        ]);
        assert_eq!(sim.regs.a, 0x42);
        assert_eq!(sim.mem.read_byte(0x0100), 0x42);
    }

    #[test]
    fn test_ld_through_memory_hl() {
        // LD HL,0x0200 ; LD (HL),0x55 ; LD A,(HL)
        let sim = run_program(&[
            encode_ld_rp_nn(PAIR_HL, 0x0200),
            encode_ld_r_n(REG_M, 0x55),
            vec![encode_ld_r_r(REG_A, REG_M)],
            vec![HALT],
        ]);
        assert_eq!(sim.regs.a, 0x55);
        assert_eq!(sim.mem.read_byte(0x0200), 0x55);
    }

    #[test]
    fn test_ld_rp_nn_and_ld_rp_a_ld_a_rp() {
        let sim = run_program(&[
            encode_ld_rp_nn(PAIR_BC, 0x0300),
            encode_ld_a_n(0x77),
            vec![encode_ld_rp_a(PAIR_BC)],
            encode_ld_a_n(0x00),
            vec![encode_ld_a_rp(PAIR_BC)],
            vec![HALT],
        ]);
        assert_eq!(sim.regs.a, 0x77);
    }

    // ── Branches / jumps ──
    #[test]
    fn test_jp_unconditional() {
        // JP over a poison LD A,0x99 straight to LD A,0x42; HALT.
        let sim = run_program(&[
            encode_jp(5),
            encode_ld_a_n(0x99),
            encode_ld_a_n(0x42),
            vec![HALT],
        ]);
        assert_eq!(sim.regs.a, 0x42);
    }

    #[test]
    fn test_jp_z_taken() {
        let taken = run_program(&[
            encode_ld_a_n(5),
            vec![encode_alu_reg(ALU_SUB, REG_A)], // A=0, Z=1
            encode_jp_cond(COND_Z, 8),
            encode_ld_r_n(REG_B, 0x99),
            encode_ld_r_n(REG_C, 0x42),
            vec![HALT],
        ]);
        assert_eq!(taken.regs.b, 0);
        assert_eq!(taken.regs.c, 0x42);
    }

    #[test]
    fn test_jr_forward_and_backward_loop() {
        // JR forward over poison.
        let sim = run_program(&[
            encode_jr(2), // skip the next 2-byte instruction
            encode_ld_a_n(0x99),
            encode_ld_a_n(0x42),
            vec![HALT],
        ]);
        assert_eq!(sim.regs.a, 0x42);

        // for (b = 0; b != 3; b++) {} via JR NZ backward branch.  CP only
        // compares against the accumulator, so the loop body copies B
        // into A before each compare.
        let sim2 = run_program(&[
            encode_ld_r_n(REG_B, 0),
            encode_ld_r_n(REG_C, 3),
            vec![encode_inc_r(REG_B)],           // loop: (addr 4)
            vec![encode_ld_r_r(REG_A, REG_B)],   // (addr 5)
            vec![encode_alu_reg(ALU_CP, REG_C)], // (addr 6)
            encode_jr_nz(-5),                    // (addr 7-8) -> back to addr 4
            vec![HALT],                          // (addr 9)
        ]);
        assert_eq!(sim2.regs.b, 3);
    }

    #[test]
    fn test_djnz_loop() {
        // B counts down 5 -> 0; loop body increments C each pass.
        let sim = run_program(&[
            encode_ld_r_n(REG_B, 5),
            encode_ld_r_n(REG_C, 0),
            vec![encode_inc_r(REG_C)], // loop:
            encode_djnz(-3),
            vec![HALT],
        ]);
        assert_eq!(sim.regs.b, 0);
        assert_eq!(sim.regs.c, 5);
    }

    #[test]
    fn test_call_ret() {
        // main: CALL sub; LD B,42; HALT   (7 bytes: CD 08 00 06 2A 76)
        // sub (addr 8): LD A,7; RET
        let mut program = vec![0u8; 12];
        program[0] = CALL;
        program[1] = 8;
        program[2] = 0;
        program[3..5].copy_from_slice(&encode_ld_r_n(REG_B, 42));
        program[5] = HALT;
        program[8] = 0x3E; // LD A,n opcode
        program[9] = 7;
        program[10] = RET;

        let mut sim = Z80Simulator::new(65536);
        sim.load_program(&program).unwrap();
        sim.regs.sp = 0xFF00;
        sim.run_loaded_with_limit(20).unwrap();
        assert_eq!(sim.regs.a, 7);
        assert_eq!(sim.regs.b, 42);
    }

    #[test]
    fn test_push_pop() {
        let mut sim = Z80Simulator::new(65536);
        sim.regs.sp = 0xFF00;
        sim.load_program(&assemble(&[
            encode_ld_rp_nn(PAIR_HL, 0x1234),
            vec![encode_push(PAIR_HL)],
            encode_ld_rp_nn(PAIR_HL, 0x0000),
            vec![encode_pop(PAIR_HL)],
            vec![HALT],
        ]))
        .unwrap();
        sim.run_loaded_with_limit(10).unwrap();
        assert_eq!(sim.regs.hl(), 0x1234);
    }

    #[test]
    fn test_push_pop_af_round_trips_flags() {
        let mut sim = Z80Simulator::new(65536);
        sim.regs.sp = 0xFF00;
        // Set carry (SCF) so the flags byte is non-zero, push AF, clear
        // it, then pop AF back and check the carry flag returned.
        sim.load_program(&assemble(&[
            encode_ld_a_n(0x12),
            vec![SCF],
            vec![encode_push(PAIR_AF)],
            vec![CCF], // flip carry off before popping
            vec![encode_pop(PAIR_AF)],
            vec![HALT],
        ]))
        .unwrap();
        sim.run_loaded_with_limit(10).unwrap();
        assert_eq!(sim.regs.a, 0x12);
        assert!(sim.flags.c, "POP AF restores the carry flag pushed earlier");
    }

    #[test]
    fn test_rst_pushes_return_and_jumps() {
        let mut sim = Z80Simulator::new(65536);
        sim.regs.sp = 0xFF00;
        // RST 1 at address 0 -> jumps to 8, which is HALT.
        let mut program = vec![0u8; 16];
        program[0] = encode_rst(1);
        program[8] = HALT;
        sim.load_program(&program).unwrap();
        let result = sim.run_loaded_with_limit(5).unwrap();
        assert!(result.halted);
        assert_eq!(result.pc, 9);
    }

    #[test]
    fn test_daa_bcd_correction() {
        // 0x09 + 0x08 = 0x11 raw; DAA corrects to 0x17 (9+8=17 decimal).
        let sim = run_program(&[
            encode_ld_a_n(0x09),
            encode_alu_imm(ALU_ADD, 0x08),
            vec![DAA],
            vec![HALT],
        ]);
        assert_eq!(sim.regs.a, 0x17);
    }

    #[test]
    fn test_rotates() {
        let sim = run_program(&[encode_ld_a_n(0x80), vec![RLCA], vec![HALT]]);
        assert_eq!(sim.regs.a, 0x01);
        assert!(sim.flags.c);
    }

    #[test]
    fn test_in_out_ports() {
        let mut sim = Z80Simulator::new(65536);
        sim.input_ports[3] = 0xAB;
        sim.load_program(&assemble(&[encode_in(3), vec![HALT]]))
            .unwrap();
        sim.run_loaded_with_limit(5).unwrap();
        assert_eq!(sim.regs.a, 0xAB);
    }

    #[test]
    fn test_ei_di() {
        let mut sim = Z80Simulator::new(65536);
        sim.load_program(&assemble(&[vec![EI], vec![DI], vec![HALT]]))
            .unwrap();
        sim.run_loaded_with_limit(5).unwrap();
        assert!(!sim.iff1);
        assert!(!sim.iff2);
    }

    #[test]
    fn test_undefined_opcode_is_typed_and_atomic() {
        // Unassigned ED bytes remain typed and atomic.
        let mut sim = Z80Simulator::new(65536);
        sim.load_program(&[0xED, 0x00]).unwrap();
        let before = sim.snapshot();
        assert_eq!(
            sim.run_loaded_with_limit(5),
            Err(Z80Error::UnknownOpcode {
                address: 0,
                raw: vec![0xED, 0x00],
            })
        );
        assert_eq!(sim.snapshot(), before);
    }

    // ── Z80-only: alternate register bank ──
    #[test]
    fn test_ex_af_af_swaps_a_and_flags() {
        let mut sim = Z80Simulator::new(65536);
        sim.load_program(&assemble(&[
            encode_ld_a_n(0x11),
            vec![SCF], // carry set in the main bank
            vec![encode_ex_af_af()],
            encode_ld_a_n(0x22),
            vec![HALT],
        ]))
        .unwrap();
        sim.run_loaded_with_limit(10).unwrap();
        // After the swap the live A is whatever A' held (0), then
        // overwritten to 0x22; both power-on flag banks begin set.
        assert_eq!(sim.regs.a, 0x22);
        assert_eq!(sim.regs.a2, 0x11);
        assert!(sim.flags.c, "live carry came from the power-on shadow bank");
    }

    #[test]
    fn test_exx_swaps_bc_de_hl() {
        let mut sim = Z80Simulator::new(65536);
        sim.load_program(&assemble(&[
            encode_ld_rp_nn(PAIR_BC, 0x1234),
            vec![EXX],
            encode_ld_rp_nn(PAIR_BC, 0x5678),
            vec![HALT],
        ]))
        .unwrap();
        sim.run_loaded_with_limit(10).unwrap();
        assert_eq!(sim.regs.bc(), 0x5678, "live BC is the post-EXX value");
        assert_eq!(
            ((sim.regs.b2 as u16) << 8) | sim.regs.c2 as u16,
            0x1234,
            "shadow BC holds the pre-EXX value"
        );
    }

    // ── Z80-only: CB-prefixed bit ops ──
    #[test]
    fn test_cb_rlc_and_bit() {
        let sim = run_program(&[encode_ld_r_n(REG_B, 0x80), encode_rlc_r(REG_B), vec![HALT]]);
        assert_eq!(sim.regs.b, 0x01);
        assert!(sim.flags.c);

        let sim2 = run_program(&[encode_ld_a_n(0x80), encode_bit(7, REG_A), vec![HALT]]);
        assert!(!sim2.flags.z, "bit 7 of 0x80 is set, so Z should be clear");
    }

    #[test]
    fn test_cb_set_res() {
        let sim = run_program(&[encode_ld_r_n(REG_B, 0x00), encode_set(3, REG_B), vec![HALT]]);
        assert_eq!(sim.regs.b, 0x08);

        let sim2 = run_program(&[encode_ld_r_n(REG_B, 0xFF), encode_res(3, REG_B), vec![HALT]]);
        assert_eq!(sim2.regs.b, 0xF7);
    }

    // ── Z80-only: IX/IY direct-operation smoke test ──
    #[test]
    fn test_ld_ix_iy_and_inc() {
        let sim = run_program(&[
            encode_ld_ix_nn(0x1000),
            encode_inc_ix(),
            encode_ld_iy_nn(0x2000),
            encode_inc_iy(),
            vec![HALT],
        ]);
        assert_eq!(sim.regs.ix, 0x1001);
        assert_eq!(sim.regs.iy, 0x2001);
    }

    #[test]
    fn test_step_mnemonics() {
        let mut sim = Z80Simulator::new(65536);
        sim.load_program(&assemble(&[encode_ld_a_n(1), vec![HALT]]))
            .unwrap();
        assert_eq!(sim.step().unwrap().mnemonic, "ld_r_n");
        assert_eq!(sim.regs.a, 1);
        assert_eq!(sim.step().unwrap().mnemonic, "halt");
        assert!(sim.halted);
    }

    // ── Integration: sum 1..=4 via ADD ──
    #[test]
    fn test_sum_program() {
        let sim = run_program(&[
            encode_ld_a_n(0),
            encode_ld_r_n(REG_B, 1),
            vec![encode_alu_reg(ALU_ADD, REG_B)],
            encode_ld_r_n(REG_B, 2),
            vec![encode_alu_reg(ALU_ADD, REG_B)],
            encode_ld_r_n(REG_B, 3),
            vec![encode_alu_reg(ALU_ADD, REG_B)],
            encode_ld_r_n(REG_B, 4),
            vec![encode_alu_reg(ALU_ADD, REG_B)],
            vec![HALT],
        ]);
        assert_eq!(sim.regs.a, 10);
    }

    // ── Encode-decode round trip ──
    #[test]
    fn test_round_trip() {
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("halt", vec![HALT]),
            ("ld_r_r", vec![encode_ld_r_r(REG_A, REG_B)]),
            ("ld_r_n", encode_ld_r_n(REG_B, 1)),
            ("ld_rp_nn", encode_ld_rp_nn(PAIR_HL, 0x1234)),
            ("alu_reg", vec![encode_alu_reg(ALU_ADD, REG_B)]),
            ("alu_imm", encode_alu_imm(ALU_ADD, 5)),
            ("jp", encode_jp(0x1234)),
            ("jp_cond", encode_jp_cond(COND_Z, 0x1234)),
            ("call", encode_call(0x1234)),
            ("call_cond", encode_call_cond(COND_NZ, 0x1234)),
            ("ret", vec![RET]),
            ("ret_cond", vec![encode_ret_cond(COND_C)]),
            ("rst", vec![encode_rst(3)]),
            ("push", vec![encode_push(PAIR_BC)]),
            ("pop", vec![encode_pop(PAIR_BC)]),
            ("in", encode_in(1)),
            ("out", encode_out(1)),
            ("ei", vec![EI]),
            ("di", vec![DI]),
            ("ex_af_af", vec![encode_ex_af_af()]),
            ("exx", vec![encode_exx()]),
            ("djnz", encode_djnz(2)),
            ("jr", encode_jr(2)),
            ("jr_cond", encode_jr_nz(2)),
            ("cb_rot", encode_rlc_r(REG_B)),
            ("bit", encode_bit(0, REG_A)),
            ("res", encode_res(0, REG_A)),
            ("set", encode_set(0, REG_A)),
            ("index_ld_nn", encode_ld_ix_nn(0x1234)),
            ("index_ld_nn", encode_ld_iy_nn(0x1234)),
            ("index_inc", encode_inc_ix()),
            ("index_inc", encode_inc_iy()),
        ];
        for (name, encoded) in &cases {
            let mut idx = 1;
            let result = decode::decode(encoded[0], &mut || {
                let b = encoded[idx];
                idx += 1;
                b
            });
            assert_eq!(result.mnemonic, *name, "decode({encoded:02x?}) failed");
        }
    }
}
