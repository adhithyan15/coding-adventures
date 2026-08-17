//! Top-level MIPS R2000 simulator combining all components.
//!
//! Public API shape mirrors [`riscv_simulator::simulator::RiscVSimulator`]:
//! `new(memory_size)`, public `regs`/`mem`/`pc`/`halted` fields,
//! `load_program(&[u8])`, `run(&[u8])`, `run_loaded_with_limit(max_steps)`,
//! and `step() -> String`.

use cpu_simulator::{Memory, RegisterFile};

use crate::decode;
use crate::encoding::assemble;
use crate::execute;
use crate::execute::read_word_be;

/// Complete MIPS R2000 simulator: 32 GPRs (R0 hardwired zero), HI/LO,
/// flat byte-addressable memory (big-endian), and a 32-bit PC.
pub struct MipsR2000Simulator {
    /// 32 general-purpose registers.  `R0` reads as zero and discards
    /// writes (`RegisterFile::new(32, true)`).
    pub regs: RegisterFile,
    /// Flat byte-addressable memory, read/written big-endian by this
    /// crate's `execute` module.
    pub mem: Memory,
    /// High word of `MULT`/`MULTU` results and the remainder of
    /// `DIV`/`DIVU`.
    pub hi: u32,
    /// Low word of `MULT`/`MULTU` results and the quotient of
    /// `DIV`/`DIVU`.
    pub lo: u32,
    /// Program counter.
    pub pc: i32,
    /// True once `SYSCALL` (the HALT sentinel) or a fault (`BREAK`,
    /// signed-overflow `ADD`/`ADDI`/`SUB`, or divide-by-zero) has executed.
    pub halted: bool,
}

/// Observable outcome of a bounded simulator run.  Mirrors
/// `riscv_simulator::simulator::ExecutionResult` field-for-field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub pc: i32,
}

impl MipsR2000Simulator {
    /// Create a new simulator with the given memory size (in bytes).
    pub fn new(memory_size: usize) -> Self {
        Self {
            regs: RegisterFile::new(32, true),
            mem: Memory::new(memory_size),
            hi: 0,
            lo: 0,
            pc: 0,
            halted: false,
        }
    }

    /// Load a program (as raw big-endian bytes) into memory at address 0.
    pub fn load_program(&mut self, program: &[u8]) {
        self.mem.load_bytes(0, program);
    }

    /// Run until halted or 10000 steps (safety limit).
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
    /// A non-halting result means the budget was exhausted — this makes the
    /// execution limit visible to callers instead of silently treating an
    /// infinite loop (e.g. a `JR $ra` whose `$ra` was never set) as success.
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

        // Fetch — big-endian, per MIPS R2000's default byte order.
        let raw = read_word_be(&self.mem, self.pc as usize);

        // Decode
        let decoded = decode::decode(raw, self.pc);
        let mnemonic = decoded.mnemonic.clone();

        // Execute
        let result = execute::execute(
            &decoded,
            &mut self.regs,
            &mut self.mem,
            &mut self.hi,
            &mut self.lo,
            self.pc,
        );
        self.pc = result.next_pc;
        self.halted = result.halted;

        mnemonic
    }

    /// Run a list of instruction words (convenience for tests).
    pub fn run_instructions(&mut self, instructions: &[u32]) {
        let program = assemble(instructions);
        self.run(&program);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::*;

    // Register-index constants for readability — mirror the psABI names
    // documented in `code/specs/07q-mips-r2000-simulator.md` /
    // `mips_r2000_simulator/state.py`.
    const ZERO: u32 = 0;
    const V0: u32 = 2;
    const T0: u32 = 8;
    const T1: u32 = 9;
    const T2: u32 = 10;
    const RA: u32 = 31;

    fn run_program(instructions: &[u32]) -> MipsR2000Simulator {
        let mut sim = MipsR2000Simulator::new(65536);
        sim.run_instructions(instructions);
        sim
    }

    #[test]
    fn bounded_run_reports_halt_and_instruction_count() {
        let mut sim = MipsR2000Simulator::new(65536);
        sim.load_program(&assemble(&[encode_addiu(T0, ZERO, 42), encode_syscall()]));
        let result = sim.run_loaded_with_limit(10);
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(result.pc, 4);
    }

    // ── the trivial "load immediate + jump-register-return" sequence ──
    // the mips-r2000-backend smoke test relies on: ADDIU $v0, $zero, 42;
    // JR $ra.  $ra was never set (starts at 0), so JR loops back to
    // address 0 — we only assert the register state after exactly two
    // steps, not that the program halts.
    #[test]
    fn load_immediate_then_jump_register_return() {
        let mut sim = MipsR2000Simulator::new(65536);
        sim.load_program(&assemble(&[encode_addiu(V0, ZERO, 42), encode_jr(RA)]));
        let result = sim.run_loaded_with_limit(2);
        assert!(!result.halted, "JR is not a halt instruction");
        assert_eq!(result.steps, 2);
        assert_eq!(sim.regs.read(V0 as usize), 42);
        assert_eq!(result.pc, 0, "JR $ra jumps back to address 0 since $ra was never set");
    }

    // ── ALU ops ──
    #[test]
    fn test_add_sub() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 10),
            encode_addiu(T1, ZERO, 20),
            encode_add(T2, T0, T1),
            encode_sub(3, T0, T1),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(T2 as usize), 30);
        assert_eq!(sim.regs.read(3) as i32, -10);
    }

    #[test]
    fn test_add_overflow_halts_without_writing_rd() {
        let mut sim = MipsR2000Simulator::new(65536);
        sim.run_instructions(&[
            encode_lui(T0, 0x7FFF),
            encode_ori(T0, T0, 0xFFFF), // T0 = i32::MAX
            encode_addiu(T1, ZERO, 1),
            encode_addiu(3, ZERO, 999),
            encode_add(3, T0, T1), // overflow: i32::MAX + 1
            encode_syscall(),
        ]);
        assert!(sim.halted);
        // rd (register 3) must NOT have been overwritten by the faulting ADD.
        assert_eq!(sim.regs.read(3), 999);
    }

    #[test]
    fn test_addu_wraps_without_halting() {
        let sim = run_program(&[
            encode_lui(T0, 0xFFFF),
            encode_ori(T0, T0, 0xFFFF), // T0 = 0xFFFFFFFF (-1 as u32)
            encode_addiu(T1, ZERO, 1),
            encode_addu(T2, T0, T1), // wraps to 0
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(T2 as usize), 0);
    }

    #[test]
    fn test_and_or_xor_nor() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 0xFF),
            encode_addiu(T1, ZERO, 0x0F),
            encode_and(2, T0, T1),
            encode_or(3, T0, T1),
            encode_xor(4, T0, T1),
            encode_nor(5, ZERO, ZERO),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0x0F);
        assert_eq!(sim.regs.read(3), 0xFF);
        assert_eq!(sim.regs.read(4), 0xF0);
        assert_eq!(sim.regs.read(5), 0xFFFF_FFFF);
    }

    #[test]
    fn test_slt_sltu() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, (-5i32) as u16 as i32),
            encode_addiu(T1, ZERO, 3),
            encode_slt(2, T0, T1),
            encode_slt(3, T1, T0),
            encode_sltu(4, T0, T1), // unsigned(-5) is huge, not < 3
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 1);
        assert_eq!(sim.regs.read(3), 0);
        assert_eq!(sim.regs.read(4), 0);
    }

    #[test]
    fn test_shifts() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 1),
            encode_sll(2, T0, 4),
            encode_addiu(T1, ZERO, (-1i32) as u16 as i32),
            encode_srl(3, T1, 28),
            encode_addiu(T1, ZERO, (-16i32) as u16 as i32),
            encode_sra(4, T1, 2),
            encode_addiu(T0, ZERO, 2),
            encode_sllv(5, 2, T0), // (1<<4)<<2 = 64
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 16);
        assert_eq!(sim.regs.read(3), 0x0F);
        assert_eq!(sim.regs.read(4) as i32, -4);
        assert_eq!(sim.regs.read(5), 64);
    }

    #[test]
    fn test_mult_multu() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, (-1i32) as u16 as i32),
            encode_addiu(T1, ZERO, 2),
            encode_mult(T0, T1),
            encode_mflo(2),
            encode_mfhi(3),
            encode_multu(T0, T1),
            encode_mflo(4),
            encode_mfhi(5),
            encode_syscall(),
        ]);
        // signed: -1 * 2 = -2 -> LO=0xFFFFFFFE, HI=0xFFFFFFFF (sign-extended product)
        assert_eq!(sim.regs.read(2), 0xFFFF_FFFE);
        assert_eq!(sim.regs.read(3), 0xFFFF_FFFF);
        // unsigned: 0xFFFFFFFF * 2 = 0x1_FFFFFFFE
        assert_eq!(sim.regs.read(4), 0xFFFF_FFFE);
        assert_eq!(sim.regs.read(5), 1);
    }

    #[test]
    fn test_div() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, -20i32 & 0xFFFF),
            encode_addiu(T1, ZERO, 6),
            encode_div(T0, T1),
            encode_mflo(2), // quotient
            encode_mfhi(3), // remainder
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2) as i32, -3);
        assert_eq!(sim.regs.read(3) as i32, -2);
    }

    #[test]
    fn test_divu() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 20),
            encode_addiu(T1, ZERO, 6),
            encode_divu(T0, T1),
            encode_mflo(2), // quotient
            encode_mfhi(3), // remainder
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 3);
        assert_eq!(sim.regs.read(3), 2);
    }

    #[test]
    fn test_divu_by_zero_halts() {
        let mut sim = MipsR2000Simulator::new(65536);
        sim.run_instructions(&[
            encode_addiu(T0, ZERO, 10),
            encode_divu(T0, ZERO),
            encode_syscall(),
        ]);
        assert!(sim.halted);
    }

    // ── Loads & stores ──
    #[test]
    fn test_sw_lw() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 0x100),
            encode_addiu(T1, ZERO, 0x42),
            encode_sw(T1, T0, 0),
            encode_lw(2, T0, 0),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0x42);
    }

    #[test]
    fn test_sb_lbu_lb_sign() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 0x200),
            encode_addiu(T1, ZERO, 0xFF),
            encode_sb(T1, T0, 0),
            encode_lbu(2, T0, 0),
            encode_lb(3, T0, 0),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0xFF);
        assert_eq!(sim.regs.read(3) as i32, -1);
    }

    #[test]
    fn test_sh_lhu_big_endian() {
        let mut sim = MipsR2000Simulator::new(65536);
        sim.run_instructions(&[
            encode_addiu(T0, ZERO, 0x200),
            encode_addiu(T1, ZERO, 0x1234),
            encode_sh(T1, T0, 0),
            encode_lhu(2, T0, 0),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0x1234);
        // Big-endian: high byte at the lower address.
        assert_eq!(sim.mem.read_byte(0x200), 0x12);
        assert_eq!(sim.mem.read_byte(0x201), 0x34);
    }

    // ── Branches ──
    #[test]
    fn test_beq_taken() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 5),
            encode_addiu(T1, ZERO, 5),
            encode_beq(T0, T1, 1), // skip the next instruction
            encode_addiu(2, ZERO, 999),
            encode_addiu(3, ZERO, 42),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0);
        assert_eq!(sim.regs.read(3), 42);
    }

    #[test]
    fn test_bne_not_taken() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 5),
            encode_addiu(T1, ZERO, 5),
            encode_bne(T0, T1, 2),
            encode_addiu(2, ZERO, 42),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 42);
    }

    #[test]
    fn test_blez_bgtz() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 0),
            encode_blez(T0, 1),
            encode_addiu(2, ZERO, 999),
            encode_addiu(3, ZERO, 42),
            encode_addiu(T1, ZERO, 5),
            encode_bgtz(T1, 1),
            encode_addiu(4, ZERO, 999),
            encode_addiu(5, ZERO, 43),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0);
        assert_eq!(sim.regs.read(3), 42);
        assert_eq!(sim.regs.read(4), 0);
        assert_eq!(sim.regs.read(5), 43);
    }

    #[test]
    fn test_bltz_bgez() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, (-1i32) as u16 as i32),
            encode_bltz(T0, 1),
            encode_addiu(2, ZERO, 999),
            encode_addiu(3, ZERO, 42),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0);
        assert_eq!(sim.regs.read(3), 42);
    }

    #[test]
    fn test_branch_backward_loop() {
        // for (t0 = 0; t0 != 3; t0++) {}
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 0),
            encode_addiu(T1, ZERO, 3),
            encode_addiu(T0, T0, 1),
            encode_bne(T0, T1, -2),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(T0 as usize), 3);
    }

    // ── Jumps ──
    #[test]
    fn test_j() {
        let sim = run_program(&[
            encode_j(3), // jump to word index 3 == byte address 12
            encode_addiu(2, ZERO, 999),
            encode_addiu(2, ZERO, 999),
            encode_addiu(3, ZERO, 42),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0);
        assert_eq!(sim.regs.read(3), 42);
    }

    #[test]
    fn test_jal_jr_call_return() {
        // main: JAL sub; ADDIU $3,$zero,42; SYSCALL
        // sub (word index 3, byte 12): ADDIU $2,$zero,7; JR $ra
        let sim = run_program(&[
            encode_jal(3),
            encode_addiu(3, ZERO, 42),
            encode_syscall(),
            encode_addiu(2, ZERO, 7),
            encode_jr(RA),
        ]);
        assert_eq!(sim.regs.read(RA as usize), 4);
        assert_eq!(sim.regs.read(2), 7);
        assert_eq!(sim.regs.read(3), 42);
    }

    #[test]
    fn test_jalr() {
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 12),
            encode_jalr(RA, T0),
            encode_addiu(2, ZERO, 999),
            encode_addiu(3, ZERO, 42),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(RA as usize), 8);
        assert_eq!(sim.regs.read(2), 0);
        assert_eq!(sim.regs.read(3), 42);
    }

    // ── I-type arithmetic / logic ──
    #[test]
    fn test_addi() {
        let sim = run_program(&[
            encode_addi(T0, ZERO, 42),
            encode_addi(T1, T0, 10),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(T0 as usize), 42);
        assert_eq!(sim.regs.read(T1 as usize), 52);
    }

    #[test]
    fn test_andi_ori_xori_zero_extend() {
        let sim = run_program(&[
            encode_lui(T0, 0xFFFF),
            encode_ori(T0, T0, 0xFFFF), // T0 = 0xFFFFFFFF
            encode_andi(2, T0, 0x00FF),
            encode_ori(3, ZERO, 0x1234),
            encode_xori(4, T0, 0xFFFF),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(2), 0x00FF);
        assert_eq!(sim.regs.read(3), 0x1234);
        assert_eq!(sim.regs.read(4), 0xFFFF_0000);
    }

    #[test]
    fn test_lui() {
        let sim = run_program(&[encode_lui(T0, 0x1234), encode_syscall()]);
        assert_eq!(sim.regs.read(T0 as usize), 0x1234_0000);
    }

    // ── R0 hardwired ──
    #[test]
    fn test_r0_hardwired() {
        let sim = run_program(&[encode_addiu(ZERO, ZERO, 42), encode_syscall()]);
        assert_eq!(sim.regs.read(0), 0);
    }

    #[test]
    fn test_break_halts() {
        let sim = run_program(&[encode_break()]);
        assert!(sim.halted);
    }

    #[test]
    fn test_step_mnemonics() {
        let mut sim = MipsR2000Simulator::new(65536);
        let prog = assemble(&[encode_addiu(T0, ZERO, 1), encode_addiu(T1, ZERO, 2), encode_syscall()]);
        sim.load_program(&prog);
        assert_eq!(sim.step(), "addiu");
        assert_eq!(sim.regs.read(T0 as usize), 1);
        assert_eq!(sim.step(), "addiu");
        assert_eq!(sim.regs.read(T1 as usize), 2);
    }

    // ── Integration: sum 1..=10 ──
    #[test]
    fn test_sum_loop() {
        // t0 = i = 1; t1 = sum = 0; t2 = 11
        // loop: t1 += t0; t0 += 1; bne t0, t2, loop
        let sim = run_program(&[
            encode_addiu(T0, ZERO, 1),
            encode_addiu(T1, ZERO, 0),
            encode_addiu(T2, ZERO, 11),
            encode_addu(T1, T1, T0), // loop:
            encode_addiu(T0, T0, 1),
            encode_bne(T0, T2, -3),
            encode_syscall(),
        ]);
        assert_eq!(sim.regs.read(T1 as usize), 55);
    }

    // ── Encode-decode round trip ──
    #[test]
    fn test_round_trip() {
        let cases: Vec<(&str, u32)> = vec![
            ("sll", encode_sll(1, 2, 5)),
            ("srl", encode_srl(1, 2, 5)),
            ("sra", encode_sra(1, 2, 5)),
            ("sllv", encode_sllv(1, 2, 3)),
            ("srlv", encode_srlv(1, 2, 3)),
            ("srav", encode_srav(1, 2, 3)),
            ("jr", encode_jr(31)),
            ("jalr", encode_jalr(31, 2)),
            ("syscall", encode_syscall()),
            ("break", encode_break()),
            ("mfhi", encode_mfhi(1)),
            ("mthi", encode_mthi(1)),
            ("mflo", encode_mflo(1)),
            ("mtlo", encode_mtlo(1)),
            ("mult", encode_mult(1, 2)),
            ("multu", encode_multu(1, 2)),
            ("div", encode_div(1, 2)),
            ("divu", encode_divu(1, 2)),
            ("add", encode_add(1, 2, 3)),
            ("addu", encode_addu(1, 2, 3)),
            ("sub", encode_sub(1, 2, 3)),
            ("subu", encode_subu(1, 2, 3)),
            ("and", encode_and(1, 2, 3)),
            ("or", encode_or(1, 2, 3)),
            ("xor", encode_xor(1, 2, 3)),
            ("nor", encode_nor(1, 2, 3)),
            ("slt", encode_slt(1, 2, 3)),
            ("sltu", encode_sltu(1, 2, 3)),
            ("bltz", encode_bltz(1, 8)),
            ("bgez", encode_bgez(1, 8)),
            ("bltzal", encode_bltzal(1, 8)),
            ("bgezal", encode_bgezal(1, 8)),
            ("j", encode_j(100)),
            ("jal", encode_jal(100)),
            ("beq", encode_beq(1, 2, 8)),
            ("bne", encode_bne(1, 2, 8)),
            ("blez", encode_blez(1, 8)),
            ("bgtz", encode_bgtz(1, 8)),
            ("addi", encode_addi(1, 2, 42)),
            ("addiu", encode_addiu(1, 2, 42)),
            ("slti", encode_slti(1, 2, -5)),
            ("sltiu", encode_sltiu(1, 2, 5)),
            ("andi", encode_andi(1, 2, 0xFF)),
            ("ori", encode_ori(1, 2, 0xFF)),
            ("xori", encode_xori(1, 2, 0xFF)),
            ("lui", encode_lui(1, 0x1234)),
            ("lb", encode_lb(1, 2, 4)),
            ("lh", encode_lh(1, 2, 4)),
            ("lw", encode_lw(1, 2, 4)),
            ("lbu", encode_lbu(1, 2, 4)),
            ("lhu", encode_lhu(1, 2, 4)),
            ("sb", encode_sb(1, 2, 4)),
            ("sh", encode_sh(1, 2, 4)),
            ("sw", encode_sw(1, 2, 4)),
        ];
        for (name, encoded) in &cases {
            let result = decode::decode(*encoded, 0);
            assert_eq!(result.mnemonic, *name, "decode(0x{encoded:08x}) failed");
        }
    }

    #[test]
    fn test_assemble_is_big_endian() {
        let bytes = assemble(&[0x1234_5678]);
        assert_eq!(bytes, vec![0x12, 0x34, 0x56, 0x78]);
    }
}
