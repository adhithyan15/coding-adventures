//! Top-level Intel 8086 simulator: segmented memory + registers +
//! fetch-decode-execute.
//!
//! # Segmented memory — the 8086's defining, structural feature
//!
//! Every physical memory access on the 8086 — instruction fetch, and (in
//! a full port) every data read/write — is computed from a **segment
//! register** and a 16-bit **offset**:
//!
//! ```text
//! physical_address = (segment_register << 4) + offset    (mod 2^20)
//! ```
//!
//! This gives a 1 MiB address space (`0x00000..=0xFFFFF`) built from
//! 16-bit segment×offset pairs, rather than one flat 20-bit program
//! counter. The four segment registers (`CS`/`DS`/`SS`/`ES`) each pick a
//! different 64 KiB *window* into that 1 MiB space; instruction fetch
//! always uses `CS:IP`.
//!
//! **This cannot be deferred**, unlike almost every other scoping
//! decision this lane makes (see `opcodes.rs`'s module doc for what
//! *was* deferred). Even the trivial two-instruction program this
//! crate's `intel8086-backend` smoke test compiles —
//! `MOV AX,imm16; HLT` — has its very first opcode byte fetched via
//! [`phys_addr`]`(self.cs, self.ip)`, not via `self.ip` alone. A flat-
//! memory shape (the shape every *other* Rust ISA simulator in this repo
//! uses — MIPS R2000, ARM1, MOS 6502, RISC-V) would not be able to
//! represent this at all: it has no separate "segment" concept for
//! `phys_addr` to combine with an offset. Structurally, `Intel8086Simulator`
//! must carry `cs`/`ds`/`ss`/`es` as first-class fields and route every
//! fetch through the segment-aware helper, which is exactly what this
//! module does — mirroring `code/packages/python/intel-8086-simulator`'s
//! `X86Simulator._phys`/`_fetch8`/`_fetch16` faithfully (see that
//! package's `simulator.py` module doc, "MEMORY MODEL" section).
//!
//! # Public API shape
//!
//! Mirrors [`mos6502_simulator::simulator::Mos6502Simulator`]'s shape:
//! `new(memory_size)`, public register/flag/`mem`/`halted` fields,
//! `load_program`/`load_program_at`, `run`, `run_loaded_with_limit`
//! returning an `ExecutionResult`, `step() -> String`. The one deviation:
//! `ExecutionResult.pc` becomes `ExecutionResult.ip` (the 8086 term for
//! its program counter — used throughout the Python reference and its
//! own `StepTrace`).

use cpu_simulator::Memory;

use crate::decode;
use crate::execute;
use crate::flags;

/// 20-bit physical address mask — a real 8086 has exactly `0xFFFFF`
/// (1,048,575) addressable bytes; `(seg << 4) + offset` can exceed that
/// (e.g. `CS=0xFFFF, IP=0xFFFF` gives `0x10FFEF`), and real silicon wraps
/// it — see `code/packages/python/intel-8086-simulator`'s `_PHYS_MASK`.
const PHYS_MASK: u32 = 0xFFFFF;

/// Compute the 20-bit physical address `(segment << 4) + offset`, masked
/// to the 8086's 1 MiB address space.
///
/// A free function (not a method) so `decode.rs` can call it without
/// needing a `&Intel8086Simulator` — decode only ever has `cs` and a
/// local `ip`, not the whole simulator (mirrors why `mos6502_simulator::
/// decode::resolve_address` takes `mem`/`pc`/`x`/`y` as plain parameters
/// rather than `&Mos6502Simulator`).
///
/// # Examples
///
/// ```
/// use intel8086_simulator::simulator::phys_addr;
///
/// assert_eq!(phys_addr(0, 0), 0);
/// assert_eq!(phys_addr(0x1000, 0x0010), 0x10010);
/// // CS=0x1000 -> base 0x10000; + offset 0x0010 -> 0x10010.
/// ```
pub fn phys_addr(segment: u16, offset: u16) -> usize {
    ((((segment as u32) << 4) + offset as u32) & PHYS_MASK) as usize
}

/// Complete (curated-subset) Intel 8086 simulator: four 16-bit
/// general-purpose registers (with byte-addressable halves), four
/// pointer/index registers, four segment registers, a 16-bit instruction
/// pointer, the six arithmetic/logical flags (CF/PF/AF/ZF/SF/OF, plus
/// TF/IF/DF carried for state fidelity though this crate's curated
/// opcode subset never sets them), and flat byte-addressable memory
/// accessed exclusively through segmented `CS:IP`/`(seg,offset)`
/// addressing.
pub struct Intel8086Simulator {
    // ── General-purpose (16-bit; byte halves via get_reg8/set_reg8) ────
    pub ax: u16,
    pub bx: u16,
    pub cx: u16,
    pub dx: u16,

    // ── Index / pointer ──────────────────────────────────────────────
    pub si: u16,
    pub di: u16,
    pub sp: u16,
    pub bp: u16,

    // ── Segment registers ────────────────────────────────────────────
    /// Code segment — `physical instruction fetch = CS<<4 + IP`.
    pub cs: u16,
    pub ds: u16,
    pub ss: u16,
    pub es: u16,

    /// Instruction pointer — 16-bit offset within `CS`.
    pub ip: u16,

    // ── Flags ────────────────────────────────────────────────────────
    pub flag_cf: bool,
    pub flag_pf: bool,
    pub flag_af: bool,
    pub flag_zf: bool,
    pub flag_sf: bool,
    pub flag_tf: bool,
    pub flag_if: bool,
    pub flag_df: bool,
    pub flag_of: bool,

    /// Flat byte-addressable memory, indexed by 20-bit physical address
    /// (see [`phys_addr`]) — NOT by `ip` directly.
    pub mem: Memory,

    /// `true` once `HLT` (`opcodes::HLT_OPCODE`) has executed.
    pub halted: bool,
}

/// Observable outcome of a bounded simulator run. Mirrors
/// `mos6502_simulator::simulator::ExecutionResult` field-for-field,
/// except `pc` is named `ip` (the 8086 term).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub ip: u16,
}

impl Intel8086Simulator {
    /// Create a new simulator with the given memory size (in bytes).
    ///
    /// A real 8086 has exactly `1_048_576` (1 MiB) bytes of address
    /// space; pass that for full fidelity. Tests in this crate mostly
    /// pass `65536` (64 KiB) since the trivial programs they exercise
    /// never address past the first segment window — `Memory` (from
    /// `cpu-simulator`) panics on any out-of-range access (see its
    /// `check_address`), so a smaller size is safe only as long as no
    /// test actually reaches past it.
    pub fn new(memory_size: usize) -> Self {
        Self {
            ax: 0,
            bx: 0,
            cx: 0,
            dx: 0,
            si: 0,
            di: 0,
            sp: 0,
            bp: 0,
            cs: 0,
            ds: 0,
            ss: 0,
            es: 0,
            ip: 0,
            flag_cf: false,
            flag_pf: false,
            flag_af: false,
            flag_zf: false,
            flag_sf: false,
            flag_tf: false,
            flag_if: false,
            flag_df: false,
            flag_of: false,
            mem: Memory::new(memory_size),
            halted: false,
        }
    }

    // ------------------------------------------------------------------
    // Register read/write by index (mirrors opcodes::REG_* encoding)
    // ------------------------------------------------------------------

    pub fn get_reg16(&self, reg: u8) -> u16 {
        match reg {
            0 => self.ax,
            1 => self.cx,
            2 => self.dx,
            3 => self.bx,
            4 => self.sp,
            5 => self.bp,
            6 => self.si,
            _ => self.di, // 7
        }
    }

    pub fn set_reg16(&mut self, reg: u8, val: u16) {
        match reg {
            0 => self.ax = val,
            1 => self.cx = val,
            2 => self.dx = val,
            3 => self.bx = val,
            4 => self.sp = val,
            5 => self.bp = val,
            6 => self.si = val,
            _ => self.di = val, // 7
        }
    }

    pub fn get_reg8(&self, reg: u8) -> u8 {
        match reg {
            0 => (self.ax & 0xFF) as u8,        // AL
            1 => (self.cx & 0xFF) as u8,        // CL
            2 => (self.dx & 0xFF) as u8,        // DL
            3 => (self.bx & 0xFF) as u8,        // BL
            4 => (self.ax >> 8) as u8,          // AH
            5 => (self.cx >> 8) as u8,          // CH
            6 => (self.dx >> 8) as u8,          // DH
            _ => (self.bx >> 8) as u8,          // BH (7)
        }
    }

    pub fn set_reg8(&mut self, reg: u8, val: u8) {
        let v = val as u16;
        match reg {
            0 => self.ax = (self.ax & 0xFF00) | v,
            1 => self.cx = (self.cx & 0xFF00) | v,
            2 => self.dx = (self.dx & 0xFF00) | v,
            3 => self.bx = (self.bx & 0xFF00) | v,
            4 => self.ax = (self.ax & 0x00FF) | (v << 8),
            5 => self.cx = (self.cx & 0x00FF) | (v << 8),
            6 => self.dx = (self.dx & 0x00FF) | (v << 8),
            _ => self.bx = (self.bx & 0x00FF) | (v << 8), // BH (7)
        }
    }

    // ------------------------------------------------------------------
    // ALU (word-width only in this crate's curated subset)
    // ------------------------------------------------------------------

    /// Compute `a OP b` for `mnemonic` in `{ADD, SUB, CMP, AND, OR, XOR}`,
    /// updating flags exactly as `execute::execute`'s Python-reference
    /// counterparts (`_add`/`_sub`/`_and`/`_or`/`_xor`) do, and returning
    /// the masked 16-bit result. `CMP` computes flags identically to
    /// `SUB` — the caller is responsible for discarding the result
    /// instead of writing it back (mirrors the Python original's
    /// `alu_op == 7` "CMP does not write" special case).
    pub(crate) fn alu16(&mut self, mnemonic: &str, a: u16, b: u16) -> u16 {
        match mnemonic {
            "ADD" => {
                let raw = a as u32 + b as u32;
                let result = raw as u16;
                self.flag_cf = flags::compute_cf_add(raw, true);
                self.flag_af = flags::compute_af_add(a, b, 0);
                self.flag_of = flags::compute_of_add(a, b, result, true);
                let (sf, zf, pf) = flags::compute_szp(result, true);
                self.flag_sf = sf;
                self.flag_zf = zf;
                self.flag_pf = pf;
                result
            }
            "SUB" | "CMP" => {
                let result = a.wrapping_sub(b);
                self.flag_cf = flags::compute_cf_sub(a as u32, b as u32, 0);
                self.flag_af = flags::compute_af_sub(a, b, 0);
                self.flag_of = flags::compute_of_sub(a, b, result, true);
                let (sf, zf, pf) = flags::compute_szp(result, true);
                self.flag_sf = sf;
                self.flag_zf = zf;
                self.flag_pf = pf;
                result
            }
            "AND" | "OR" | "XOR" => {
                let result = match mnemonic {
                    "AND" => a & b,
                    "OR" => a | b,
                    _ => a ^ b,
                };
                // AND/OR/XOR: CF=0, OF=0, AF undefined (cleared) -- mirrors
                // the Python original's `_set_flags_logic`.
                self.flag_cf = false;
                self.flag_of = false;
                self.flag_af = false;
                let (sf, zf, pf) = flags::compute_szp(result, true);
                self.flag_sf = sf;
                self.flag_zf = zf;
                self.flag_pf = pf;
                result
            }
            other => panic!("intel8086-simulator: alu16 called with non-ALU mnemonic {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // Load / run
    // ------------------------------------------------------------------

    /// Write `program` into memory at physical address `origin`, then
    /// reset `IP` to 0 (mirroring the Python original's `load(program,
    /// origin)`, which is always paired with `reset()` setting `CS=IP=0`
    /// in the one-shot `execute()` entry point). Does **not** reset `CS`
    /// or any other register — a caller that wants a genuinely fresh
    /// simulator should construct a new one via [`Self::new`].
    pub fn load_program_at(&mut self, program: &[u8], origin: usize) {
        self.mem.load_bytes(origin, program);
        self.ip = 0;
        self.halted = false;
    }

    /// Load a program at physical address 0 — the convention every other
    /// Rust ISA simulator in this repo uses for `load_program`. With
    /// `CS=0` (the power-on / [`Self::new`] default), `CS:IP = 0:0`
    /// fetches from physical address 0, so this matches
    /// `load_program_at(program, 0)` exactly when `CS` hasn't been
    /// changed since construction.
    pub fn load_program(&mut self, program: &[u8]) {
        self.load_program_at(program, 0);
    }

    /// Load and run a program from a fresh state, until halted or 10000
    /// steps (safety limit) — convenience wrapper mirroring
    /// `Mos6502Simulator::run`.
    pub fn run(&mut self, program: &[u8]) {
        self.load_program(program);
        self.run_loaded_with_limit(10000);
    }

    /// Run the already-loaded program for at most `max_steps`
    /// instructions.
    ///
    /// A non-halting result means the step budget was exhausted — surfaces
    /// runaway/looping programs to the caller instead of silently treating
    /// them as success (same contract as `Mos6502Simulator::
    /// run_loaded_with_limit`).
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
            ip: self.ip,
        }
    }

    /// Execute a single instruction and return its mnemonic.
    ///
    /// Fetch-decodes at `CS:IP` (which `decode::fetch_decode` advances
    /// past the whole instruction as a side effect via its local `ip`
    /// copy), then dispatches to `execute::execute`. An opcode outside
    /// this crate's curated subset is a **fail-closed halt** — mirrors
    /// `mos6502-simulator`'s illegal-opcode handling (no exception
    /// channel exists through `step() -> String`, so the simulator stops
    /// rather than silently corrupting state or panicking); the Python
    /// original instead falls through to treating any unrecognised
    /// opcode as `HLT` (see `simulator.py`'s final `# Unknown opcode --
    /// treat as HLT` fallback) -- functionally equivalent (`halted`
    /// becomes `true` either way), just surfaced through `Err` first.
    pub fn step(&mut self) -> String {
        if self.halted {
            return "halted".to_string();
        }

        let mut ip = self.ip;
        let decoded = match decode::fetch_decode(&self.mem, self.cs, &mut ip) {
            Ok(d) => d,
            Err(_) => {
                self.halted = true;
                return "illegal".to_string();
            }
        };
        self.ip = ip;

        execute::execute(self, &decoded).to_string()
    }

    /// Run a list of raw instruction bytes (convenience for tests).
    pub fn run_bytes(&mut self, bytes: &[u8]) {
        self.run(bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcodes;
    use crate::opcodes::{HLT_OPCODE, MOV_REG_IMM16_BASE, REG_AX};

    // ── Segmentation math itself ────────────────────────────────────

    #[test]
    fn phys_addr_zero_segment_zero_offset() {
        assert_eq!(phys_addr(0, 0), 0);
    }

    #[test]
    fn phys_addr_combines_segment_shifted_and_offset() {
        // CS=0x1000 -> base 0x10000; + offset 0x0010 -> 0x10010.
        assert_eq!(phys_addr(0x1000, 0x0010), 0x10010);
    }

    #[test]
    fn phys_addr_matches_python_reference_formula() {
        // physical = (segment * 16 + offset) & 0xFFFFF -- from
        // code/packages/python/intel-8086-simulator's `_phys`.
        for (seg, off, expected) in [
            (0u16, 0u16, 0usize),
            (0x07C0, 0x0000, 0x07C00), // classic boot-sector CS
            (0xFFFF, 0x0020, 0x00010), // beyond 1MB, masked (wraps)
        ] {
            assert_eq!(phys_addr(seg, off), expected);
        }
    }

    #[test]
    fn phys_addr_wraps_at_20_bits() {
        // CS=0xFFFF, IP=0xFFFF -> (0xFFFF<<4)+0xFFFF = 0x10FFEF,
        // masked to 20 bits -> 0x0FFEF.
        assert_eq!(phys_addr(0xFFFF, 0xFFFF), 0x0FFEF);
    }

    // ── Register read/write ─────────────────────────────────────────

    #[test]
    fn reg16_round_trip_all_eight() {
        let mut sim = Intel8086Simulator::new(65536);
        for reg in 0u8..8 {
            sim.set_reg16(reg, 0x1000 + reg as u16);
        }
        for reg in 0u8..8 {
            assert_eq!(sim.get_reg16(reg), 0x1000 + reg as u16);
        }
    }

    #[test]
    fn reg8_halves_share_storage_with_reg16() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.set_reg16(REG_AX, 0x1234);
        assert_eq!(sim.get_reg8(opcodes::REG_AL), 0x34);
        assert_eq!(sim.get_reg8(opcodes::REG_AH), 0x12);
        sim.set_reg8(opcodes::REG_AL, 0xFF);
        assert_eq!(sim.ax, 0x12FF);
    }

    // ── The trivial "load immediate + halt" sequence the
    // intel8086-backend smoke test relies on ──────────────────────────

    #[test]
    fn mov_ax_immediate_then_hlt_halts_with_ax_equals_42() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.run(&[MOV_REG_IMM16_BASE + REG_AX, 42, 0x00, HLT_OPCODE]);
        assert!(sim.halted);
        assert_eq!(sim.ax, 42);
    }

    #[test]
    fn bounded_run_reports_halt_and_instruction_count() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.load_program(&[0xB8, 42, 0x00, 0xF4]);
        let result = sim.run_loaded_with_limit(10);
        assert!(result.halted);
        assert_eq!(result.steps, 2); // MOV AX,42, then HLT
    }

    #[test]
    fn power_on_state_is_all_zero() {
        let sim = Intel8086Simulator::new(65536);
        assert_eq!(sim.ax, 0);
        assert_eq!(sim.cs, 0);
        assert_eq!(sim.ip, 0);
        assert!(!sim.halted);
        assert!(!sim.flag_cf && !sim.flag_zf && !sim.flag_sf);
    }

    #[test]
    fn add_ax_immediate_sets_carry_on_overflow() {
        let mut sim = Intel8086Simulator::new(65536);
        // MOV AX,0xFFFF; ADD AX,1; HLT -> AX=0, CF=1, ZF=1
        sim.run(&[0xB8, 0xFF, 0xFF, 0x05, 0x01, 0x00, 0xF4]);
        assert_eq!(sim.ax, 0);
        assert!(sim.flag_cf);
        assert!(sim.flag_zf);
    }

    #[test]
    fn sub_ax_immediate_basic() {
        let mut sim = Intel8086Simulator::new(65536);
        // MOV AX,10; SUB AX,3; HLT -> AX=7
        sim.run(&[0xB8, 10, 0x00, 0x2D, 3, 0x00, 0xF4]);
        assert_eq!(sim.ax, 7);
        assert!(!sim.flag_cf);
    }

    #[test]
    fn cmp_ax_immediate_does_not_write_back() {
        let mut sim = Intel8086Simulator::new(65536);
        // MOV AX,5; CMP AX,5; HLT -> AX still 5, ZF=1
        sim.run(&[0xB8, 5, 0x00, 0x3D, 5, 0x00, 0xF4]);
        assert_eq!(sim.ax, 5);
        assert!(sim.flag_zf);
    }

    #[test]
    fn mov_reg_reg16_copies_between_registers() {
        let mut sim = Intel8086Simulator::new(65536);
        // MOV AX,99; MOV CX,AX; HLT
        sim.run(&[0xB8, 99, 0x00, 0x8B, 0xC8, 0xF4]);
        assert_eq!(sim.cx, 99);
        assert_eq!(sim.ax, 99);
    }

    #[test]
    fn inc_dec_reg16_preserve_carry_flag() {
        let mut sim = Intel8086Simulator::new(65536);
        // MOV AX,0xFFFF; ADD AX,1 (sets CF=1); INC CX (must NOT clear CF); HLT
        sim.run(&[0xB8, 0xFF, 0xFF, 0x05, 0x01, 0x00, 0x41, 0xF4]);
        assert!(sim.flag_cf, "INC must not affect CF");
        assert_eq!(sim.cx, 1);
    }

    #[test]
    fn illegal_opcode_halts_fail_closed() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.run(&[0xE8, 0x00, 0x00]); // CALL near -- not in this subset
        assert!(sim.halted);
    }

    #[test]
    fn step_after_halt_returns_halted() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.load_program(&[HLT_OPCODE]);
        sim.step();
        assert!(sim.halted);
        assert_eq!(sim.step(), "halted");
    }

    #[test]
    fn step_returns_mnemonic() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.load_program(&[0xB8, 42, 0x00, 0xF4]);
        assert_eq!(sim.step(), "MOV");
        assert_eq!(sim.ax, 42);
        assert_eq!(sim.step(), "HLT");
        assert!(sim.halted);
    }

    #[test]
    fn load_program_at_nonzero_origin_with_matching_cs() {
        // Loading at a non-zero physical origin only fetches correctly if
        // CS:0 maps to that same physical address -- set CS accordingly
        // first (mirrors real boot-sector-style loading at CS=0x07C0).
        let mut sim = Intel8086Simulator::new(1 << 20);
        let cs = 0x0010u16;
        let origin = phys_addr(cs, 0);
        sim.cs = cs;
        sim.load_program_at(&[0xB8, 42, 0x00, 0xF4], origin);
        let result = sim.run_loaded_with_limit(10);
        assert!(result.halted);
        assert_eq!(sim.ax, 42);
    }
}
