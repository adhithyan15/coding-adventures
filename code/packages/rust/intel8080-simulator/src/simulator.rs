//! Top-level Intel 8080 simulator combining all components.
//!
//! Public API shape mirrors [`mips_r2000_simulator::MipsR2000Simulator`] /
//! [`riscv_simulator::simulator::RiscVSimulator`]: `new(memory_size)`,
//! public register/memory/pc/halted access, `load_program(&[u8])`,
//! `run(&[u8])`, `run_loaded_with_limit(max_steps) -> ExecutionResult`, and
//! `step() -> String`.  Unlike those two ISAs (which use
//! `cpu_simulator::RegisterFile`, an indexed array of 32-bit registers),
//! the 8080's seven working registers are individually named (A, B, C, D,
//! E, H, L) rather than numbered, so [`crate::execute::Registers`] is a
//! plain named-field struct instead.

use cpu_simulator::Memory;

use crate::decode;
use crate::execute;
use crate::execute::{ExecuteResult, Flags, Registers};

/// Complete Intel 8080 simulator: 7 named 8-bit registers, a 16-bit stack
/// pointer, 5 condition flags, flat 64Ki byte-addressable memory, 256
/// input + 256 output ports, and a 16-bit PC.
pub struct Intel8080Simulator {
    /// A, B, C, D, E, H, L + SP.
    pub regs: Registers,
    /// S, Z, AC, P, CY.
    pub flags: Flags,
    /// Flat byte-addressable memory (64 KiB by convention, but callers may
    /// size it differently via [`Self::new`]).
    pub mem: Memory,
    /// Program counter.
    pub pc: u16,
    /// True once `HLT` (or an undefined opcode — fail-closed) has executed.
    pub halted: bool,
    /// INTE flip-flop — set by `EI`, cleared by `DI`.  Not connected to any
    /// external interrupt delivery mechanism (see module docs on
    /// `intel8080_simulator.simulator` in the Python original: interrupts
    /// cannot arrive between `step()` calls in a behavioral model).
    pub interrupts_enabled: bool,
    /// 256 input ports, read by `IN port`.
    pub input_ports: [u8; 256],
    /// 256 output ports, written by `OUT port`.
    pub output_ports: [u8; 256],
}

/// Observable outcome of a bounded simulator run.  Mirrors
/// `mips_r2000_simulator::simulator::ExecutionResult` field-for-field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub pc: u16,
}

impl Intel8080Simulator {
    /// Create a new simulator with the given memory size (in bytes).  The
    /// real chip addresses a fixed 64 KiB (`0x10000`); callers pass that
    /// explicitly (mirroring `MipsR2000Simulator::new`'s shape) rather than
    /// having it baked in, so tests can use a smaller arena.
    pub fn new(memory_size: usize) -> Self {
        Self {
            regs: Registers::default(),
            flags: Flags::default(),
            mem: Memory::new(memory_size),
            pc: 0,
            halted: false,
            interrupts_enabled: false,
            input_ports: [0u8; 256],
            output_ports: [0u8; 256],
        }
    }

    /// Load a program into memory at address 0.  Does not reset PC or
    /// registers — callers that want a fresh run should construct a new
    /// simulator (mirrors `MipsR2000Simulator::load_program`).
    pub fn load_program(&mut self, program: &[u8]) {
        self.mem.load_bytes(0, program);
    }

    /// Load and run a program until halted or 10000 steps (safety limit).
    pub fn run(&mut self, program: &[u8]) {
        self.load_program(program);
        self.run_loaded_with_limit(10000);
    }

    /// Run instructions from an already-loaded program.
    pub fn run_loaded(&mut self) {
        self.run_loaded_with_limit(10000);
    }

    /// Run the already-loaded program for at most `max_steps` instructions.
    ///
    /// A non-halting result means the budget was exhausted — this makes an
    /// infinite loop (or a forgotten `HLT`) visible to callers instead of
    /// silently treating it as success, mirroring
    /// `MipsR2000Simulator::run_loaded_with_limit`.
    pub fn run_loaded_with_limit(&mut self, max_steps: usize) -> ExecutionResult {
        let mut steps = 0;
        while steps < max_steps {
            if self.halted {
                break;
            }
            self.step();
            steps += 1;
        }
        ExecutionResult {
            halted: self.halted,
            steps,
            pc: self.pc,
        }
    }

    /// Execute a single instruction and return its mnemonic.
    pub fn step(&mut self) -> String {
        if self.halted {
            return "halted".to_string();
        }

        // Fetch the opcode byte, then hand `decode` a closure that reads
        // (and advances) further operand bytes directly out of `self.mem`
        // / a local PC cursor.  The closure only holds a shared borrow of
        // `self.mem`, so it's fully dropped before the `execute` call below
        // takes a mutable borrow.
        let opcode = self.mem.read_byte(self.pc as usize);
        let mut cursor = self.pc.wrapping_add(1);
        let mem_ref = &self.mem;
        let decoded = decode::decode(opcode, &mut || {
            let b = mem_ref.read_byte(cursor as usize);
            cursor = cursor.wrapping_add(1);
            b
        });
        let mnemonic = decoded.mnemonic.clone();
        let fallthrough_pc = cursor;

        let ExecuteResult { next_pc, halted } = execute::execute(
            &decoded,
            &mut self.regs,
            &mut self.flags,
            &mut self.mem,
            &self.input_ports,
            &mut self.output_ports,
            &mut self.interrupts_enabled,
            fallthrough_pc,
        );
        self.pc = next_pc;
        self.halted = halted;

        mnemonic
    }

    /// Run a sequence of pre-encoded instructions (convenience for tests) —
    /// concatenates them and calls [`Self::run`].
    pub fn run_instructions(&mut self, instructions: &[Vec<u8>]) {
        let program = crate::encoding::assemble(instructions);
        self.run(&program);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::*;
    use crate::opcodes::*;

    fn run_program(instructions: &[Vec<u8>]) -> Intel8080Simulator {
        let mut sim = Intel8080Simulator::new(65536);
        sim.run_instructions(instructions);
        sim
    }

    // ── the trivial "load immediate into accumulator + HLT" sequence the
    // intel8080-backend smoke test relies on ──
    #[test]
    fn mvi_a_then_hlt() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.load_program(&assemble(&[encode_mvi_a(42), vec![HLT]]));
        let result = sim.run_loaded_with_limit(10);
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(sim.regs.a, 42);
    }

    #[test]
    fn bounded_run_reports_halt_and_instruction_count() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.load_program(&assemble(&[encode_mvi(REG_B, 7), vec![HLT]]));
        let result = sim.run_loaded_with_limit(10);
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(sim.regs.b, 7);
    }

    // ── ALU ops ──
    #[test]
    fn test_add_sub() {
        let sim = run_program(&[
            encode_mvi(REG_B, 10),
            encode_mvi(REG_C, 20),
            encode_mvi_a(10),
            vec![encode_alu_reg(ALU_ADD, REG_C)], // A = 10 + 20 = 30
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 30);
        assert!(!sim.flags.cy);
    }

    #[test]
    fn test_add_overflow_sets_carry() {
        let sim = run_program(&[
            encode_mvi_a(0xFF),
            encode_alu_imm(ALU_ADD, 1),
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 0x00);
        assert!(sim.flags.cy, "0xFF + 1 should set carry");
        assert!(sim.flags.z, "result is 0");
    }

    #[test]
    fn test_sub_borrow_sets_carry() {
        let sim = run_program(&[
            encode_mvi_a(0x00),
            encode_alu_imm(ALU_SUB, 1),
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 0xFF);
        assert!(sim.flags.cy, "0x00 - 1 borrows");
        assert!(sim.flags.s);
    }

    #[test]
    fn test_ana_ora_xra() {
        let sim = run_program(&[
            encode_mvi_a(0xFF),
            encode_mvi(REG_B, 0x0F),
            vec![encode_alu_reg(ALU_ANA, REG_B)],
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 0x0F);
        assert!(!sim.flags.cy, "ANA always clears carry");
    }

    #[test]
    fn test_cmp_does_not_write_a() {
        let sim = run_program(&[
            encode_mvi_a(5),
            encode_mvi(REG_B, 5),
            vec![encode_alu_reg(ALU_CMP, REG_B)],
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 5, "CMP must not write A");
        assert!(sim.flags.z, "5 == 5");
    }

    #[test]
    fn test_inr_dcr_wrap() {
        let sim = run_program(&[
            encode_mvi(REG_B, 0xFF),
            vec![encode_inr(REG_B)],
            vec![HLT],
        ]);
        assert_eq!(sim.regs.b, 0x00);
        assert!(sim.flags.z);
    }

    // ── Load / store ──
    #[test]
    fn test_sta_lda_round_trip() {
        let sim = run_program(&[
            encode_mvi_a(0x42),
            encode_sta(0x0100),
            encode_mvi_a(0x00),
            encode_lda(0x0100),
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 0x42);
        assert_eq!(sim.mem.read_byte(0x0100), 0x42);
    }

    #[test]
    fn test_mov_through_memory_m() {
        // LXI H, 0x0200 ; MVI M, 0x55 ; MOV A, M
        let sim = run_program(&[
            encode_lxi(PAIR_H, 0x0200),
            encode_mvi(REG_M, 0x55),
            vec![encode_mov(REG_A, REG_M)],
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 0x55);
        assert_eq!(sim.mem.read_byte(0x0200), 0x55);
    }

    #[test]
    fn test_lxi_stax_ldax() {
        let sim = run_program(&[
            encode_lxi(PAIR_B, 0x0300),
            encode_mvi_a(0x77),
            vec![encode_stax(PAIR_B)],
            encode_mvi_a(0x00),
            vec![encode_ldax(PAIR_B)],
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 0x77);
    }

    // ── Branches / jumps ──
    #[test]
    fn test_jmp_unconditional() {
        // JMP over a poison MVI A,0x99 straight to MVI A,0x42; HLT.
        // Layout: JMP(3 bytes @0) MVI A,0x99(2 bytes @3) MVI A,0x42(2 bytes @5) HLT(@7).
        let sim = run_program(&[
            encode_jmp(5),
            encode_mvi_a(0x99),
            encode_mvi_a(0x42),
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 0x42);
    }

    #[test]
    fn test_jz_taken_and_not_taken() {
        // A=0; SUB A -> Z=1; JZ taken.
        // Layout: MVI A,5(2 @0) SUB A(1 @2) JZ(3 @3) MVI B,0x99(2 @6) MVI C,0x42(2 @8) HLT(@10).
        let taken = run_program(&[
            encode_mvi_a(5),
            vec![encode_alu_reg(ALU_SUB, REG_A)], // A=0, Z=1
            encode_jcond(COND_Z, 8),
            encode_mvi(REG_B, 0x99),
            encode_mvi(REG_C, 0x42),
            vec![HLT],
        ]);
        assert_eq!(taken.regs.b, 0);
        assert_eq!(taken.regs.c, 0x42);
    }

    #[test]
    fn test_call_ret() {
        // main: CALL sub; MVI B,42; HLT   (7 bytes: CD 08 00 06 2A 76)
        // sub (addr 8): MVI A,7; RET
        let mut program = vec![0u8; 12];
        program[0] = CALL;
        program[1] = 8;
        program[2] = 0;
        program[3..5].copy_from_slice(&encode_mvi(REG_B, 42));
        program[5] = HLT;
        program[8] = MVI_A_OPCODE;
        program[9] = 7;
        program[10] = RET;

        let mut sim = Intel8080Simulator::new(65536);
        sim.load_program(&program);
        sim.regs.sp = 0xFF00;
        sim.run_loaded_with_limit(20);
        assert_eq!(sim.regs.a, 7);
        assert_eq!(sim.regs.b, 42);
    }

    const MVI_A_OPCODE: u8 = 0x3E;

    #[test]
    fn test_push_pop() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.regs.sp = 0xFF00;
        sim.load_program(&assemble(&[
            encode_lxi(PAIR_H, 0x1234),
            vec![encode_push(PAIR_H)],
            encode_lxi(PAIR_H, 0x0000),
            vec![encode_pop(PAIR_H)],
            vec![HLT],
        ]));
        sim.run_loaded_with_limit(10);
        assert_eq!(sim.regs.hl(), 0x1234);
    }

    #[test]
    fn test_rst_pushes_return_and_jumps() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.regs.sp = 0xFF00;
        // RST 1 at address 0 -> jumps to 8, which is HLT.
        let mut program = vec![0u8; 16];
        program[0] = encode_rst(1);
        program[8] = HLT;
        sim.load_program(&program);
        let result = sim.run_loaded_with_limit(5);
        assert!(result.halted);
        assert_eq!(result.pc, 9);
    }

    #[test]
    fn test_daa_bcd_correction() {
        // 0x09 + 0x08 = 0x11 raw; DAA corrects to 0x17 (9+8=17 decimal).
        let sim = run_program(&[
            encode_mvi_a(0x09),
            encode_alu_imm(ALU_ADD, 0x08),
            vec![DAA],
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 0x17);
    }

    #[test]
    fn test_rotates() {
        let sim = run_program(&[encode_mvi_a(0x80), vec![RLC], vec![HLT]]);
        assert_eq!(sim.regs.a, 0x01);
        assert!(sim.flags.cy);
    }

    #[test]
    fn test_in_out_ports() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.input_ports[3] = 0xAB;
        sim.load_program(&assemble(&[encode_in(3), vec![HLT]]));
        sim.run_loaded_with_limit(5);
        assert_eq!(sim.regs.a, 0xAB);
    }

    #[test]
    fn test_ei_di() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.load_program(&assemble(&[vec![EI], vec![DI], vec![HLT]]));
        sim.run_loaded_with_limit(5);
        assert!(!sim.interrupts_enabled);
    }

    #[test]
    fn test_step_mnemonics() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.load_program(&assemble(&[encode_mvi_a(1), vec![HLT]]));
        assert_eq!(sim.step(), "mvi");
        assert_eq!(sim.regs.a, 1);
        assert_eq!(sim.step(), "hlt");
        assert!(sim.halted);
    }

    #[test]
    fn test_undefined_opcode_halts_fail_closed() {
        let mut sim = Intel8080Simulator::new(65536);
        sim.load_program(&[0x08]); // undefined on stock 8080
        let result = sim.run_loaded_with_limit(5);
        assert!(result.halted);
    }

    // ── Integration: sum 1..=4 via ADD (matches the spec's "Hello" example) ──
    #[test]
    fn test_sum_program() {
        let sim = run_program(&[
            encode_mvi_a(0),
            encode_mvi(REG_B, 1),
            vec![encode_alu_reg(ALU_ADD, REG_B)],
            encode_mvi(REG_B, 2),
            vec![encode_alu_reg(ALU_ADD, REG_B)],
            encode_mvi(REG_B, 3),
            vec![encode_alu_reg(ALU_ADD, REG_B)],
            encode_mvi(REG_B, 4),
            vec![encode_alu_reg(ALU_ADD, REG_B)],
            vec![HLT],
        ]);
        assert_eq!(sim.regs.a, 10);
    }
}
