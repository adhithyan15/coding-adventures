//! Top-level Motorola 68000 simulator combining decode + execute.
//!
//! Public API shape mirrors every other Rust ISA simulator in this repo
//! (`mos6502_simulator::Mos6502Simulator`, `arm1_simulator::ARM1`):
//! `new(memory_size)`, public register/memory/flag fields,
//! `load_program(&[u8])`, `run(&[u8])`, `run_loaded_with_limit(max_steps)`,
//! `step() -> String`.  Like `mos6502-simulator`, this crate also exposes
//! `load_program_at`/`run_at` for the Python original's own convention
//! (programs load at `0x001000`, stack starts at `0x00F000`) as a
//! secondary convenience API — the zero-origin variants are primary,
//! matching every other Rust simulator in this repo (and what
//! `m68k-backend`'s emitted bytes expect to be loaded at).

use cpu_simulator::Memory;

use crate::execute;

/// Complete Motorola 68000 simulator: 8 data registers (`D0`-`D7`), 8
/// address registers (`A0`-`A7`, with `A7` doubling as the supervisor
/// stack pointer), a 32-bit program counter (only the low 24 bits are
/// significant -- see [`crate::opcodes::ADDR_MASK`]), a 16-bit status
/// register (CCR in the low 5 bits), and flat byte-addressable memory.
///
/// # Why `d`/`a` are arrays, not 16 named fields
///
/// Unlike the 6502's three small, irregular, individually-named
/// registers (`a`/`x`/`y`), the 68000 has 16 **uniform** 32-bit
/// general-purpose registers split into two orthogonal banks, and every
/// opword selects one by a 3-bit index (`dn`/`an`/`reg` fields
/// throughout `execute.rs`).  Arrays indexed by that register number are
/// the natural fit here — `sim.d[dn]`, not sixteen `match` arms over
/// `Dn0`..`Dn7` fields.
pub struct M68kSimulator {
    /// Data registers `D0`-`D7`.
    pub d: [u32; 8],
    /// Address registers `A0`-`A7`.  `a[7]` is always the supervisor
    /// stack pointer in this simulator (it runs permanently in
    /// supervisor mode, matching the Python original).
    pub a: [u32; 8],
    /// Program counter.  Only the low 24 bits are architecturally
    /// significant.
    pub pc: u32,
    /// 16-bit status register.  Bits 4-0 are the CCR (`X N Z V C`);
    /// bits 15-8 are the system byte (trace/supervisor/interrupt-mask).
    pub sr: u16,
    /// `true` once `TRAP #15` (this simulator's HALT convention — see
    /// the crate-level doc) or `STOP #imm` has executed.
    pub halted: bool,
    /// Flat byte-addressable memory.
    pub mem: Memory,
}

/// Observable outcome of a bounded simulator run.  Mirrors every other
/// Rust ISA simulator's `ExecutionResult { halted, steps, pc }` shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub pc: u32,
}

/// Power-on status register: supervisor mode set, interrupt-priority
/// mask at 7 (block all), matching the Python original's `_sr = 0x2700`.
const RESET_SR: u16 = 0x2700;

impl M68kSimulator {
    /// Create a new simulator with `memory_size` bytes of backing store
    /// (pass `0x100_0000` — 16 MiB — for the full 68000 address space;
    /// tests routinely use far less).  The stack pointer (`A7`) starts
    /// at the top of that memory (growing downward), `PC` at `0`
    /// (zero-origin convention — see the module doc for the alternative
    /// `load_program_at`/Python-matching `0x001000` convention).
    pub fn new(memory_size: usize) -> Self {
        Self {
            d: [0; 8],
            a: [0, 0, 0, 0, 0, 0, 0, memory_size as u32],
            pc: 0,
            sr: RESET_SR,
            halted: false,
            mem: Memory::new(memory_size),
        }
    }

    /// Load a program into memory at the given origin and set `pc`
    /// there — mirrors the Python original's `load(program)` (which
    /// always uses `0x001000`; this Rust port generalises the origin).
    pub fn load_program_at(&mut self, program: &[u8], origin: u32) {
        self.mem.load_bytes(origin as usize, program);
        self.pc = origin;
        self.halted = false;
    }

    /// Load a program at address `0` — the zero-origin convention every
    /// other Rust ISA simulator in this repo uses for `load_program`,
    /// and what `m68k-backend`'s emitted bytes are written to expect.
    pub fn load_program(&mut self, program: &[u8]) {
        self.load_program_at(program, 0);
    }

    /// Run a freshly-loaded program (at address 0) until halted or
    /// 10,000 steps (safety limit).
    pub fn run(&mut self, program: &[u8]) {
        self.load_program(program);
        self.run_loaded_with_limit(10_000);
    }

    /// Run instructions from an already-loaded program.
    pub fn run_loaded(&mut self) -> ExecutionResult {
        self.run_loaded_with_limit(10_000)
    }

    /// Run the already-loaded program for at most `max_steps`
    /// instructions.
    ///
    /// A non-halting result means the budget was exhausted — this makes
    /// the execution limit visible to callers instead of silently
    /// treating an infinite loop (or a `Backend`-emitted program that
    /// never actually terminated — see `m68k-backend`'s `terminated`
    /// tracking) as success.
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

    /// Execute a single instruction and return its mnemonic (or an
    /// error description, or `"halted"` if already halted).
    ///
    /// A decode/execute failure (illegal opword, a deferred addressing
    /// mode or instruction family, a misaligned access) is a
    /// **fail-closed halt** — mirrors how `mos6502-simulator` handles
    /// illegal opcodes: no exception channel exists through
    /// `step() -> String`, so the simulator stops rather than silently
    /// corrupting state or panicking.  The Python original raises a
    /// Python exception for the same conditions instead.
    pub fn step(&mut self) -> String {
        if self.halted {
            return "halted".to_string();
        }
        match execute::decode_and_execute(self) {
            Ok(mnemonic) => mnemonic,
            Err(e) => {
                self.halted = true;
                e
            }
        }
    }

    // -- CCR flag accessors (mirrors the Python original's `M68KState`
    // properties) --------------------------------------------------------

    /// Extend flag (bit 4 of SR).
    pub fn flag_x(&self) -> bool {
        self.sr & (1 << 4) != 0
    }
    /// Negative flag (bit 3 of SR).
    pub fn flag_n(&self) -> bool {
        self.sr & (1 << 3) != 0
    }
    /// Zero flag (bit 2 of SR).
    pub fn flag_z(&self) -> bool {
        self.sr & (1 << 2) != 0
    }
    /// Overflow flag (bit 1 of SR).
    pub fn flag_v(&self) -> bool {
        self.sr & (1 << 1) != 0
    }
    /// Carry flag (bit 0 of SR).
    pub fn flag_c(&self) -> bool {
        self.sr & 1 != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{assemble, encode_move_l_imm_to_dn, encode_trap15};

    #[test]
    fn power_on_state() {
        let sim = M68kSimulator::new(65536);
        assert_eq!(sim.d, [0; 8]);
        assert_eq!(sim.a[7], 65536, "SP starts at top of memory");
        assert_eq!(sim.pc, 0);
        assert_eq!(sim.sr, 0x2700);
        assert!(!sim.halted);
    }

    /// The trivial "load immediate into D0 + halt-convention" sequence
    /// the `m68k-backend` smoke test relies on: `MOVE.L #42,D0; TRAP #15`.
    #[test]
    fn move_l_42_then_trap15_halts_with_d0_equals_42() {
        let mut sim = M68kSimulator::new(65536);
        let bytes = assemble(&[encode_move_l_imm_to_dn(0, 42), encode_trap15()]);
        sim.run(&bytes);
        assert!(sim.halted);
        assert_eq!(sim.d[0], 42);
    }

    #[test]
    fn bounded_run_reports_halt_and_instruction_count() {
        let mut sim = M68kSimulator::new(65536);
        let bytes = assemble(&[encode_move_l_imm_to_dn(0, 42), encode_trap15()]);
        sim.load_program(&bytes);
        let result = sim.run_loaded_with_limit(10);
        assert!(result.halted);
        assert_eq!(result.steps, 2);
    }

    #[test]
    fn step_returns_mnemonic() {
        let mut sim = M68kSimulator::new(65536);
        let bytes = assemble(&[encode_move_l_imm_to_dn(0, 42), encode_trap15()]);
        sim.load_program(&bytes);
        assert_eq!(sim.step(), "MOVE");
        assert_eq!(sim.d[0], 42);
        assert_eq!(sim.step(), "TRAP #15");
        assert!(sim.halted);
    }

    #[test]
    fn step_after_halt_returns_halted() {
        let mut sim = M68kSimulator::new(65536);
        sim.load_program(&encode_trap15());
        sim.step();
        assert!(sim.halted);
        assert_eq!(sim.step(), "halted");
    }

    #[test]
    fn illegal_line0_opcode_halts_fail_closed() {
        let mut sim = M68kSimulator::new(65536);
        sim.run(&[0x00, 0x00, 0x00, 0x01]); // line-0 immediate group, deferred
        assert!(sim.halted);
    }

    #[test]
    fn load_program_at_nonzero_origin() {
        let mut sim = M68kSimulator::new(65536);
        let bytes = assemble(&[encode_move_l_imm_to_dn(0, 42), encode_trap15()]);
        sim.load_program_at(&bytes, 0x0200);
        assert_eq!(sim.pc, 0x0200);
        sim.run_loaded_with_limit(10);
        assert_eq!(sim.d[0], 42);
    }

    #[test]
    fn ccr_accessors_read_sr_bits() {
        let mut sim = M68kSimulator::new(64);
        sim.sr = 0b0001_1111; // X N Z V C all set
        assert!(sim.flag_x() && sim.flag_n() && sim.flag_z() && sim.flag_v() && sim.flag_c());
    }
}
