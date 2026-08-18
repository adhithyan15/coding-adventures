//! Top-level Zilog Z80 simulator combining all components.
//!
//! Public API shape mirrors [`intel8080_simulator::Intel8080Simulator`] /
//! [`mips_r2000_simulator::MipsR2000Simulator`]: `new(memory_size)`,
//! public register/memory/pc/halted access, `load_program(&[u8])`,
//! `run(&[u8])`, `run_loaded_with_limit(max_steps) -> ExecutionResult`,
//! and `step() -> String`.

use cpu_simulator::Memory;

use crate::decode;
use crate::execute;
use crate::execute::{ExecuteResult, Flags, Registers};

/// Complete Zilog Z80 simulator: main + alternate register banks, IX/IY
/// index registers, a 16-bit stack pointer, I/R special registers, six
/// named condition flags, flat 64Ki byte-addressable memory, 256 input +
/// 256 output ports, and a 16-bit PC.
pub struct Z80Simulator {
    /// Main bank (A/B/C/D/E/H/L) + alternate bank (A'/F'/B'/C'/D'/E'/H'/L')
    /// + IX/IY/SP/I/R.
    pub regs: Registers,
    /// Main-bank condition flags (S, Z, H, P/V, N, C).
    pub flags: Flags,
    /// Flat byte-addressable memory (64 KiB by convention, but callers may
    /// size it differently via [`Self::new`]).
    pub mem: Memory,
    /// Program counter.
    pub pc: u16,
    /// True once `HALT` — or a fail-closed undefined/`ED`-prefixed opcode
    /// (see `execute.rs` module docs) — has executed.
    pub halted: bool,
    /// IFF1 — maskable-interrupt enable flip-flop.  Set by `EI`, cleared
    /// by `DI`.
    pub iff1: bool,
    /// IFF2 — shadow of IFF1 (real hardware preserves it across NMI; this
    /// behavioral model has no NMI delivery path, so it always tracks
    /// IFF1 exactly, same simplification the Python original's `EI`/`DI`
    /// handlers make).
    pub iff2: bool,
    /// Interrupt mode (0, 1, or 2).  Stored for completeness; no
    /// interrupt-delivery mechanism exists in this behavioral model (see
    /// `intel8080_simulator::Intel8080Simulator::interrupts_enabled`'s
    /// module docs for the same caveat on the 8080 side — interrupts
    /// cannot arrive between `step()` calls).
    pub im: u8,
    /// 256 input ports, read by `IN A,(n)`.
    pub input_ports: [u8; 256],
    /// 256 output ports, written by `OUT (n),A`.
    pub output_ports: [u8; 256],
}

/// Observable outcome of a bounded simulator run.  Mirrors
/// `intel8080_simulator::simulator::ExecutionResult` field-for-field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub pc: u16,
}

impl Z80Simulator {
    /// Create a new simulator with the given memory size (in bytes).  The
    /// real chip addresses a fixed 64 KiB (`0x10000`); callers pass that
    /// explicitly (mirrors `Intel8080Simulator::new`'s shape) rather than
    /// having it baked in, so tests can use a smaller arena.
    pub fn new(memory_size: usize) -> Self {
        Self {
            regs: Registers::default(),
            flags: Flags::default(),
            mem: Memory::new(memory_size),
            pc: 0,
            halted: false,
            iff1: false,
            iff2: false,
            im: 0,
            input_ports: [0u8; 256],
            output_ports: [0u8; 256],
        }
    }

    /// Load a program into memory at address 0.  Does not reset PC or
    /// registers — callers that want a fresh run should construct a new
    /// simulator (mirrors `Intel8080Simulator::load_program`).
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

    /// Run the already-loaded program for at most `max_steps`
    /// instructions.
    ///
    /// A non-halting result means the budget was exhausted — this makes
    /// an infinite loop (or a forgotten `HALT`) visible to callers
    /// instead of silently treating it as success, mirroring
    /// `Intel8080Simulator::run_loaded_with_limit`.
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

        // Fetch the first opcode byte, then hand `decode` a closure that
        // reads (and advances) further bytes directly out of `self.mem` /
        // a local PC cursor.  The closure only holds a shared borrow of
        // `self.mem`, so it's fully dropped before the `execute` call
        // below takes a mutable borrow — same pattern
        // `Intel8080Simulator::step` uses.
        let first_byte = self.mem.read_byte(self.pc as usize);
        let mut cursor = self.pc.wrapping_add(1);
        let mem_ref = &self.mem;
        let decoded = decode::decode(first_byte, &mut || {
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
            &mut self.iff1,
            &mut self.iff2,
            fallthrough_pc,
        );
        self.pc = next_pc;
        self.halted = halted;

        mnemonic
    }

    /// Run a sequence of pre-encoded instructions (convenience for tests)
    /// — concatenates them and calls [`Self::run`].
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

    fn run_program(instructions: &[Vec<u8>]) -> Z80Simulator {
        let mut sim = Z80Simulator::new(65536);
        sim.run_instructions(instructions);
        sim
    }

    // ── the trivial "load immediate into accumulator + HALT" sequence
    // the z80-backend smoke test relies on — byte-identical to
    // intel8080-simulator's canonical MVI A,42; HLT ──
    #[test]
    fn ld_a_n_then_halt() {
        let mut sim = Z80Simulator::new(65536);
        sim.load_program(&assemble(&[encode_ld_a_n(42), vec![HALT]]));
        let result = sim.run_loaded_with_limit(10);
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(sim.regs.a, 42);
    }

    #[test]
    fn bounded_run_reports_halt_and_instruction_count() {
        let mut sim = Z80Simulator::new(65536);
        sim.load_program(&assemble(&[encode_ld_r_n(REG_B, 7), vec![HALT]]));
        let result = sim.run_loaded_with_limit(10);
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
        let sim = run_program(&[
            encode_ld_a_n(0xFF),
            encode_alu_imm(ALU_ADD, 1),
            vec![HALT],
        ]);
        assert_eq!(sim.regs.a, 0x00);
        assert!(sim.flags.c, "0xFF + 1 should set carry");
        assert!(sim.flags.z, "result is 0");
    }

    #[test]
    fn test_sub_borrow_sets_carry_and_n() {
        let sim = run_program(&[
            encode_ld_a_n(0x00),
            encode_alu_imm(ALU_SUB, 1),
            vec![HALT],
        ]);
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
            vec![encode_inc_r(REG_B)],               // loop: (addr 4)
            vec![encode_ld_r_r(REG_A, REG_B)],        // (addr 5)
            vec![encode_alu_reg(ALU_CP, REG_C)],      // (addr 6)
            encode_jr_nz(-5),                         // (addr 7-8) -> back to addr 4
            vec![HALT],                                // (addr 9)
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
        sim.load_program(&program);
        sim.regs.sp = 0xFF00;
        sim.run_loaded_with_limit(20);
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
        ]));
        sim.run_loaded_with_limit(10);
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
        ]));
        sim.run_loaded_with_limit(10);
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
        sim.load_program(&program);
        let result = sim.run_loaded_with_limit(5);
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
        sim.load_program(&assemble(&[encode_in(3), vec![HALT]]));
        sim.run_loaded_with_limit(5);
        assert_eq!(sim.regs.a, 0xAB);
    }

    #[test]
    fn test_ei_di() {
        let mut sim = Z80Simulator::new(65536);
        sim.load_program(&assemble(&[vec![EI], vec![DI], vec![HALT]]));
        sim.run_loaded_with_limit(5);
        assert!(!sim.iff1);
        assert!(!sim.iff2);
    }

    #[test]
    fn test_undefined_opcode_halts_fail_closed() {
        // 0xED (ED-prefix) is not ported — every ED opcode halts.
        let mut sim = Z80Simulator::new(65536);
        sim.load_program(&[0xED, 0x57]); // LD A,I on real hardware
        let result = sim.run_loaded_with_limit(5);
        assert!(result.halted);
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
        ]));
        sim.run_loaded_with_limit(10);
        // After the swap the live A is whatever A' held (0), then
        // overwritten to 0x22; the shadow bank now holds A=0x11, carry set.
        assert_eq!(sim.regs.a, 0x22);
        assert_eq!(sim.regs.a2, 0x11);
        assert!(!sim.flags.c, "live carry came from the (unset) shadow bank");
    }

    #[test]
    fn test_exx_swaps_bc_de_hl() {
        let mut sim = Z80Simulator::new(65536);
        sim.load_program(&assemble(&[
            encode_ld_rp_nn(PAIR_BC, 0x1234),
            vec![EXX],
            encode_ld_rp_nn(PAIR_BC, 0x5678),
            vec![HALT],
        ]));
        sim.run_loaded_with_limit(10);
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
        let sim = run_program(&[
            encode_ld_r_n(REG_B, 0x80),
            encode_rlc_r(REG_B),
            vec![HALT],
        ]);
        assert_eq!(sim.regs.b, 0x01);
        assert!(sim.flags.c);

        let sim2 = run_program(&[
            encode_ld_a_n(0x80),
            encode_bit(7, REG_A),
            vec![HALT],
        ]);
        assert!(!sim2.flags.z, "bit 7 of 0x80 is set, so Z should be clear");
    }

    #[test]
    fn test_cb_set_res() {
        let sim = run_program(&[
            encode_ld_r_n(REG_B, 0x00),
            encode_set(3, REG_B),
            vec![HALT],
        ]);
        assert_eq!(sim.regs.b, 0x08);

        let sim2 = run_program(&[
            encode_ld_r_n(REG_B, 0xFF),
            encode_res(3, REG_B),
            vec![HALT],
        ]);
        assert_eq!(sim2.regs.b, 0xF7);
    }

    // ── Z80-only: IX/IY basics ──
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
        sim.load_program(&assemble(&[encode_ld_a_n(1), vec![HALT]]));
        assert_eq!(sim.step(), "ld_r_n");
        assert_eq!(sim.regs.a, 1);
        assert_eq!(sim.step(), "halt");
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
            ("ld_ix_nn", encode_ld_ix_nn(0x1234)),
            ("ld_iy_nn", encode_ld_iy_nn(0x1234)),
            ("inc_ix", encode_inc_ix()),
            ("inc_iy", encode_inc_iy()),
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
