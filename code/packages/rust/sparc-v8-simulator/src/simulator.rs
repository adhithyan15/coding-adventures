//! Top-level SPARC V8 simulator combining all components.
//!
//! Public API shape mirrors [`mips_r2000_simulator::simulator::MipsR2000Simulator`]
//! / `riscv_simulator::simulator::RiscVSimulator`: `new(memory_size)`,
//! public `regs`/`mem`/`pc`/`halted` fields, `load_program(&[u8])`,
//! `run(&[u8])`, `run_loaded_with_limit(max_steps)`, and
//! `step() -> String`.  Struct is named `SparcV8Simulator` following the
//! `sparc-v8-gatelevel::SparcCpu` naming precedent for this architecture
//! (both crates share the `SPARC`/`Sparc` capitalisation the gate-level
//! port already established in-tree).

use cpu_simulator::Memory;

use crate::decode;
use crate::encoding::assemble;
use crate::execute::{self, Psr};
use crate::registers::RegisterWindowFile;

/// Complete SPARC V8 behavioral simulator: a windowed register file
/// (8 globals + `NWINDOWS` x 16 outs/locals, see [`crate::registers`]),
/// PSR condition-code flags, the `Y` multiply/divide auxiliary register,
/// flat byte-addressable memory (big-endian), and a 32-bit PC.
pub struct SparcV8Simulator {
    /// Windowed register file (56 physical registers + CWP).
    pub regs: RegisterWindowFile,
    /// Flat byte-addressable memory, read/written big-endian by this
    /// crate's `execute` module.
    pub mem: Memory,
    /// PSR condition-code flags (N/Z/V/C).
    pub psr: Psr,
    /// `Y` register — multiply/divide auxiliary.
    pub y: u32,
    /// Program counter.
    pub pc: i32,
    /// True once `ta 0` (the HALT sentinel) or a fault (divide-by-zero,
    /// register-window overflow, or a non-`TA` `Ticc` trap) has
    /// executed.
    pub halted: bool,
}

/// Observable outcome of a bounded simulator run.  Mirrors
/// `mips_r2000_simulator::simulator::ExecutionResult` field-for-field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub pc: i32,
}

impl SparcV8Simulator {
    /// Create a new simulator with the given memory size (in bytes).
    pub fn new(memory_size: usize) -> Self {
        Self {
            regs: RegisterWindowFile::new(),
            mem: Memory::new(memory_size),
            psr: Psr::default(),
            y: 0,
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
    /// A non-halting result means the budget was exhausted — this makes
    /// the execution limit visible to callers instead of silently
    /// treating an infinite loop as success.
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

        // Fetch — big-endian, per SPARC V8's default byte order.
        let raw = execute::fetch_word_be(&self.mem, self.pc as usize);

        // Decode
        let decoded = decode::decode(raw);
        let mnemonic = decoded.mnemonic.clone();

        // Execute
        let result = execute::execute(
            &decoded,
            &mut self.regs,
            &mut self.mem,
            &mut self.psr,
            &mut self.y,
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
    use crate::opcodes::*;
    use crate::registers::NWINDOWS;

    // Register-index constants for readability, in the virtual r0-r31
    // numbering (relative to the current window) -- mirror
    // `sparc_v8_simulator/state.py`'s `REG_*` aliases.
    const G0: u32 = 0;
    const G1: u32 = 1;
    const O0: u32 = 8;
    const O1: u32 = 9;
    const O2: u32 = 10;

    fn run_program(instructions: &[u32]) -> SparcV8Simulator {
        let mut sim = SparcV8Simulator::new(65536);
        sim.run_instructions(instructions);
        sim
    }

    #[test]
    fn bounded_run_reports_halt_and_instruction_count() {
        let mut sim = SparcV8Simulator::new(65536);
        sim.load_program(&assemble(&[encode_add_imm(O0, G0, 42), encode_ta(0)]));
        let result = sim.run_loaded_with_limit(10);
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(result.pc, 8);
    }

    // ── The trivial "load immediate into %o0 + halt" sequence the
    // sparc-v8-backend smoke test relies on: ADD %g0, 42, %o0; ta 0.
    #[test]
    fn load_immediate_then_halt() {
        let mut sim = SparcV8Simulator::new(65536);
        sim.load_program(&assemble(&[encode_add_imm(O0, G0, 42), encode_ta(0)]));
        let result = sim.run_loaded_with_limit(2);
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(sim.regs.read(O0), 42);
        // ta 0 still advances pc by 4 (see execute.rs module docs) --
        // unlike mips-r2000-simulator's SYSCALL, which leaves pc
        // unchanged at the halting instruction.
        assert_eq!(result.pc, 8);
    }

    #[test]
    fn g0_is_hardwired_zero() {
        let sim = run_program(&[encode_add_imm(G0, G0, 42), encode_ta(0)]);
        assert_eq!(sim.regs.read(G0), 0);
    }

    #[test]
    fn test_add_sub() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 10),
            encode_add_imm(O1, G0, 20),
            encode_add(O2, O0, O1),
            encode_sub(G1, O0, O1),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(O2), 30);
        assert_eq!(sim.regs.read(G1) as i32, -10);
    }

    #[test]
    fn test_addcc_sets_flags() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 5),
            encode_addcc_imm(O1, O0, -5), // 5 + (-5) = 0
            encode_ta(0),
        ]);
        assert!(sim.psr.z);
        assert_eq!(sim.regs.read(O1), 0);
    }

    #[test]
    fn test_logic_ops() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 0xFF),
            encode_add_imm(O1, G0, 0x0F),
            encode_and(2, O0, O1),
            encode_or(3, O0, O1),
            encode_xor(4, O0, O1),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 0x0F);
        assert_eq!(sim.regs.read(3), 0xFF);
        assert_eq!(sim.regs.read(4), 0xF0);
    }

    #[test]
    fn test_shifts() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 1),
            encode_sll(2, O0, 4),
            encode_add_imm(O1, G0, -1),
            encode_srl(3, O1, 28),
            encode_sra(4, O1, 2),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 16);
        assert_eq!(sim.regs.read(3), 0x0F);
        assert_eq!(sim.regs.read(4) as i32, -1);
    }

    #[test]
    fn test_sethi_upper_bits() {
        let sim = run_program(&[encode_sethi(O0, 0x1234), encode_ta(0)]);
        assert_eq!(sim.regs.read(O0), 0x1234 << 10);
    }

    #[test]
    fn test_umul_smul() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, -1), // 0xFFFFFFFF
            encode_add_imm(O1, G0, 2),
            encode_umul(2, O0, O1),
            encode_add_imm(3, G0, 0),
            encode_rdy(3),
            encode_smul(4, O0, O1),
            encode_ta(0),
        ]);
        // unsigned: 0xFFFFFFFF * 2 = 0x1_FFFFFFFE
        assert_eq!(sim.regs.read(2), 0xFFFF_FFFE);
        assert_eq!(sim.regs.read(3), 1);
        // signed: -1 * 2 = -2
        assert_eq!(sim.regs.read(4) as i32, -2);
    }

    #[test]
    fn test_udiv_sdiv() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 20),
            encode_add_imm(O1, G0, 6),
            encode_udiv(2, O0, O1),
            encode_add_imm(O0, G0, -20),
            // SDIV's dividend is Y:rs1 -- real SPARC code must sign-extend
            // the dividend into %y before a 32-bit signed divide.  SRA by
            // 31 broadcasts O0's sign bit; WRY (XOR with %g0) copies it
            // into Y.
            encode_sra(O2, O0, 31),
            encode_wry(O2, G0),
            encode_sdiv(3, O0, O1),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 3);
        assert_eq!(sim.regs.read(3) as i32, -3);
    }

    #[test]
    fn test_udiv_by_zero_halts() {
        let mut sim = SparcV8Simulator::new(65536);
        sim.run_instructions(&[encode_add_imm(O0, G0, 10), encode_udiv(2, O0, G0), encode_ta(0)]);
        assert!(sim.halted);
    }

    #[test]
    fn test_sw_lw() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 0x100),
            encode_add_imm(O1, G0, 0x42),
            encode_st(O1, O0, 0),
            encode_ld(2, O0, 0),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 0x42);
    }

    #[test]
    fn test_stb_ldub_ldsb_sign() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 0x200),
            encode_add_imm(O1, G0, 0xFF),
            encode_stb(O1, O0, 0),
            encode_ldub(2, O0, 0),
            encode_ldsb(3, O0, 0),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 0xFF);
        assert_eq!(sim.regs.read(3) as i32, -1);
    }

    #[test]
    fn test_sth_lduh_big_endian() {
        let mut sim = SparcV8Simulator::new(65536);
        sim.run_instructions(&[
            encode_add_imm(O0, G0, 0x200),
            encode_add_imm(O1, G0, 0x0678), // within the 13-bit signed imm range
            encode_sth(O1, O0, 0),
            encode_lduh(2, O0, 0),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 0x0678);
        // Big-endian: high byte at the lower address.
        assert_eq!(sim.mem.read_byte(0x200), 0x06);
        assert_eq!(sim.mem.read_byte(0x201), 0x78);
    }

    // ── Branches ──
    #[test]
    fn test_be_taken() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 5),
            encode_subcc_imm(O1, O0, 5), // Z=1
            encode_bicc(COND_BE, 2),     // taken: skip the next instruction
            encode_add_imm(2, G0, 999),
            encode_add_imm(3, G0, 42),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 0);
        assert_eq!(sim.regs.read(3), 42);
    }

    #[test]
    fn test_bne_not_taken() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 5),
            encode_subcc_imm(O1, O0, 5), // Z=1
            encode_bicc(COND_BNE, 3),    // not taken
            encode_add_imm(2, G0, 42),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 42);
    }

    #[test]
    fn test_branch_backward_loop() {
        // for (o0 = 0; o0 != 3; o0++) {}
        let sim = run_program(&[
            encode_add_imm(O0, G0, 0),
            encode_add_imm(O1, G0, 3),
            encode_add_imm(O0, O0, 1),   // loop:
            encode_subcc(O2, O0, O1),    // O0 - O1 -> Z when equal
            encode_bicc(COND_BNE, -2),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(O0), 3);
    }

    // ── CALL / JMPL ──
    #[test]
    fn test_call_sets_o7_and_jumps() {
        // main (word 0): CALL sub (word index 2, byte 8)
        //                ta 0                       (word 1, byte 4 -- skipped)
        // sub  (word 2, byte 8): add %g0,7,%o0 ; ta 0
        let sim = run_program(&[
            encode_call(2),
            encode_ta(0),
            encode_add_imm(O0, G0, 7),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(15), 0); // %o7 = call's own pc (word 0)
        assert_eq!(sim.regs.read(O0), 7);
    }

    #[test]
    fn test_jmpl_absolute_jump() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 12), // target byte address
            encode_jmpl(15, O0, 0),
            encode_add_imm(2, G0, 999),
            encode_add_imm(3, G0, 42),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(2), 0);
        assert_eq!(sim.regs.read(3), 42);
    }

    // ── SAVE / RESTORE ──
    #[test]
    fn test_save_restore_round_trip() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 99), // %o0 = 99 in window 0
            encode_save(14, O0, 0),     // %sp' = %o0 + 0 = 99; rotates CWP
            encode_restore(14, G0, 0),  // rotates CWP back
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.cwp, 0);
        assert_eq!(sim.regs.read(O0), 99);
    }

    #[test]
    fn test_save_aliases_outs_to_ins_of_new_window() {
        let sim = run_program(&[
            encode_add_imm(O0, G0, 99), // %o0 = 99 (window 0)
            encode_save(14, G0, 0),     // rotate; %sp' = 0
            encode_ta(0),
        ]);
        // %i0 (virt 24) in the post-SAVE window aliases %o0 of window 0.
        assert_eq!(sim.regs.read(24), 99);
    }

    #[test]
    fn test_save_overflow_halts() {
        let mut instrs = Vec::new();
        for _ in 0..NWINDOWS {
            instrs.push(encode_save(14, G0, 0));
        }
        instrs.push(encode_ta(0));
        let sim = run_program(&instrs);
        assert!(sim.halted);
    }

    #[test]
    fn test_step_mnemonics() {
        let mut sim = SparcV8Simulator::new(65536);
        let prog = assemble(&[encode_add_imm(O0, G0, 1), encode_add_imm(O1, G0, 2), encode_ta(0)]);
        sim.load_program(&prog);
        assert_eq!(sim.step(), "add");
        assert_eq!(sim.regs.read(O0), 1);
        assert_eq!(sim.step(), "add");
        assert_eq!(sim.regs.read(O1), 2);
    }

    // ── Integration: sum 1..=10 ──
    #[test]
    fn test_sum_loop() {
        // o0 = i = 1; o1 = sum = 0; o2 = 11
        // loop: o1 += o0; o0 += 1; subcc o3,o0,o2; bne loop
        let sim = run_program(&[
            encode_add_imm(O0, G0, 1),
            encode_add_imm(O1, G0, 0),
            encode_add_imm(O2, G0, 11),
            encode_add(O1, O1, O0),    // loop:
            encode_add_imm(O0, O0, 1),
            encode_subcc(3, O0, O2),
            encode_bicc(COND_BNE, -3),
            encode_ta(0),
        ]);
        assert_eq!(sim.regs.read(O1), 55);
    }

    #[test]
    fn test_assemble_is_big_endian() {
        let bytes = assemble(&[0x1234_5678]);
        assert_eq!(bytes, vec![0x12, 0x34, 0x56, 0x78]);
    }
}
