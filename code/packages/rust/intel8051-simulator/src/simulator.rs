//! # `intel8051-simulator::simulator` — top-level `Intel8051Simulator`.
//!
//! Ties `decode` + `execute` together into the public fetch-decode-
//! execute API, mirroring the Python reference's `I8051Simulator`
//! (`intel8051_simulator.simulator.I8051Simulator`, spec 07p) — same
//! reset/load/step/execute shape, same reset values, same HALT
//! convention — while keeping the public surface close to this
//! codebase's other Rust historical-arch simulators (`ARM1::new`,
//! `::load_program`, `::step`, `::run`, register/flag accessors).
//!
//! ## Why `new()` takes no size parameter
//!
//! `arm1_simulator::ARM1::new(memory_size)` and
//! `intel8008_simulator::Simulator::new()` both model architectures
//! with a single flat memory space, where "how much memory" is a
//! meaningful knob for a test harness to vary.  The 8051 has no such
//! knob: its three address spaces are architecturally fixed by the
//! instruction encoding itself (a 16-bit PC over 64 KiB code memory, an
//! 8-bit direct/indirect address over 256 B internal RAM, a 16-bit
//! DPTR over 64 KiB external data memory) — every real MCS-51 part
//! agrees on these sizes regardless of how much code/data a given
//! program actually uses.  `Intel8051Simulator::new()` therefore
//! allocates the full Harvard memory map unconditionally; there is
//! nothing useful a `memory_size` parameter could vary.

use crate::decode::decode;
use crate::execute::execute;
use crate::opcodes::{
    CODE_SIZE, IRAM_SIZE, PSW_AC, PSW_CY, PSW_OV, PSW_P, SFR_ACC, SFR_B, SFR_DPH, SFR_DPL,
    SFR_P0, SFR_P1, SFR_P2, SFR_P3, SFR_PSW, SFR_SP, XDATA_SIZE,
};

/// Summary of a bounded `run` — the counterpart to the Python
/// reference's `ExecutionResult` (from the shared `simulator_protocol`
/// package), trimmed to the fields this crate's callers (mainly
/// `intel8051-backend`'s tests) actually need.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionResult {
    /// `true` if the HALT sentinel (`0xA5`) was reached before
    /// `max_steps` ran out.
    pub halted: bool,
    /// Number of instructions actually executed.
    pub steps: usize,
    /// Program counter after the run stopped.
    pub pc: u16,
}

/// Behavioral simulator for the Intel 8051 (MCS-51) microcontroller.
///
/// Harvard architecture: `code`, `iram` (which aliases the SFR space
/// at 0x80-0xFF), and `xdata` are three independent byte arrays, exactly
/// matching `code/specs/07p-intel-8051-simulator.md`'s memory map.
pub struct Intel8051Simulator {
    /// 64 KiB code memory (program), fetched by `pc`, read by `MOVC`.
    pub(crate) code: Vec<u8>,
    /// 256 B internal RAM: 0x00-0x7F general/bit-addressable RAM,
    /// 0x80-0xFF Special Function Registers.
    pub(crate) iram: [u8; IRAM_SIZE],
    /// 64 KiB external data memory, reachable only via `MOVX`.
    pub(crate) xdata: Vec<u8>,
    /// 16-bit program counter.  Not an SFR — a dedicated field, same
    /// as every other simulator in this codebase.
    pub(crate) pc: u16,
    /// `true` once the HALT sentinel (`0xA5`) has been executed.
    pub(crate) halted: bool,
}

impl Default for Intel8051Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Intel8051Simulator {
    /// Construct a simulator in its power-on-reset state.  See the
    /// module doc comment for why there is no `memory_size` parameter.
    pub fn new() -> Self {
        let mut sim = Intel8051Simulator {
            code: vec![0u8; CODE_SIZE],
            iram: [0u8; IRAM_SIZE],
            xdata: vec![0u8; XDATA_SIZE],
            pc: 0,
            halted: false,
        };
        sim.reset();
        sim
    }

    /// Return the CPU to power-on-reset state.  Per the 8051
    /// datasheet (and `code/specs/07p-intel-8051-simulator.md`'s
    /// "Reset state" table): `PC=0`, `SP=0x07`, ports `P0`-`P3`
    /// latched high (`0xFF`), every other SFR and all of IRAM zeroed.
    /// `code` and `xdata` are left untouched — only `load_program`
    /// changes code memory.
    pub fn reset(&mut self) {
        self.iram = [0u8; IRAM_SIZE];
        self.pc = 0x0000;
        self.halted = false;
        self.iram[SFR_SP as usize] = 0x07;
        for port_sfr in [SFR_P0, SFR_P1, SFR_P2, SFR_P3] {
            self.iram[port_sfr as usize] = 0xFF;
        }
    }

    /// Reset, then copy `code` into code memory starting at
    /// `start_addr`.  Mirrors `arm1_simulator::ARM1::load_program`'s
    /// signature (`code`, `start_addr`) rather than the Python
    /// reference's fixed-at-0x0000 `load`, since a `start_addr` lets
    /// callers place a ROM anywhere in the 64 KiB code space (the
    /// `intel8051-backend` trivial-ROM smoke tests always pass 0, but
    /// the flexibility costs nothing and matches the sibling
    /// simulators' shape).
    ///
    /// # Panics
    ///
    /// Panics if `code.len() + start_addr as usize` would overflow the
    /// 64 KiB code-memory space.
    pub fn load_program(&mut self, code: &[u8], start_addr: u16) {
        self.reset();
        let start = start_addr as usize;
        assert!(
            start + code.len() <= CODE_SIZE,
            "intel8051-simulator: program of {} bytes at 0x{start_addr:04X} \
             overflows the 64 KiB code-memory space",
            code.len()
        );
        self.code[start..start + code.len()].copy_from_slice(code);
        self.pc = start_addr;
    }

    /// Execute one instruction and return its mnemonic.  A no-op
    /// (`"HALT"`, PC unchanged) once `halted()` is already `true`,
    /// matching the Python reference's `step()`.
    pub fn step(&mut self) -> String {
        if self.halted {
            return "HALT".to_string();
        }
        let d = decode(&self.code, self.pc);
        // `execute` computes every branch/call target relative to the
        // *post-fetch* PC, so update `self.pc` before dispatching —
        // see `execute::execute`'s doc comment for why.
        self.pc = d.next_pc;
        execute(self, &d).to_string()
    }

    /// Run until `halted()` or `max_steps` instructions have executed,
    /// collecting each step's mnemonic.  Useful for tracing/debugging;
    /// [`Self::run_loaded_with_limit`] is the lighter-weight choice
    /// when callers only need the final outcome.
    pub fn run(&mut self, max_steps: usize) -> Vec<String> {
        let mut trace = Vec::new();
        while !self.halted && trace.len() < max_steps {
            trace.push(self.step());
        }
        trace
    }

    /// Run until `halted()` or `max_steps` instructions have executed,
    /// returning just the summary — no per-step allocation.  This is
    /// the guarded-execution entry point `intel8051-backend`'s tests
    /// use to prove a `SJMP $`-free HALT-terminated ROM actually stops
    /// within a bounded number of steps (rather than relying on a test
    /// timeout to catch an infinite loop).
    pub fn run_loaded_with_limit(&mut self, max_steps: usize) -> ExecutionResult {
        let mut steps = 0;
        while !self.halted && steps < max_steps {
            self.step();
            steps += 1;
        }
        ExecutionResult {
            halted: self.halted,
            steps,
            pc: self.pc,
        }
    }

    // ── Accessors ────────────────────────────────────────────────────

    pub fn pc(&self) -> u16 {
        self.pc
    }

    pub fn halted(&self) -> bool {
        self.halted
    }

    /// Accumulator (SFR 0xE0).
    pub fn acc(&self) -> u8 {
        self.iram[SFR_ACC as usize]
    }

    /// B register (SFR 0xF0) — the `MUL AB`/`DIV AB` second operand.
    pub fn b_register(&self) -> u8 {
        self.iram[SFR_B as usize]
    }

    /// Stack pointer (SFR 0x81).
    pub fn sp(&self) -> u8 {
        self.iram[SFR_SP as usize]
    }

    /// 16-bit data pointer, `DPH:DPL` (SFRs 0x83:0x82).
    pub fn dptr(&self) -> u16 {
        ((self.iram[SFR_DPH as usize] as u16) << 8) | self.iram[SFR_DPL as usize] as u16
    }

    /// Program status word (SFR 0xD0).
    pub fn psw(&self) -> u8 {
        self.iram[SFR_PSW as usize]
    }

    pub fn cy(&self) -> bool {
        self.iram[SFR_PSW as usize] & PSW_CY != 0
    }

    pub fn ac(&self) -> bool {
        self.iram[SFR_PSW as usize] & PSW_AC != 0
    }

    pub fn ov(&self) -> bool {
        self.iram[SFR_PSW as usize] & PSW_OV != 0
    }

    pub fn parity_flag(&self) -> bool {
        self.iram[SFR_PSW as usize] & PSW_P != 0
    }

    /// Active register bank (0-3), from PSW.RS1:RS0.
    pub fn bank(&self) -> u8 {
        (self.iram[SFR_PSW as usize] >> 3) & 0x3
    }

    /// Read working register `Rn` (0-7) from the currently active
    /// bank — the same value `MOV A, Rn` etc. would read.
    pub fn read_register(&self, n: u8) -> u8 {
        self.iram[self.rn_addr(n)]
    }

    /// Read one byte of internal RAM / SFR space by its direct
    /// address (0x00-0xFF).
    pub fn read_iram(&self, addr: u8) -> u8 {
        self.iram[addr as usize]
    }

    /// Read one byte of code memory.
    pub fn read_code(&self, addr: u16) -> u8 {
        self.code[addr as usize]
    }

    /// Read one byte of external data memory.
    pub fn read_xdata(&self, addr: u16) -> u8 {
        self.xdata[addr as usize]
    }

    // ── Internal helpers used by `execute` ──────────────────────────
    //
    // Ported 1:1 from `simulator.py`'s `_rn`/`_set_rn`/`_dptr`/
    // `_set_dptr`/`_acc`/`_set_acc`/`_update_parity`/`_cy`/
    // `_set_flags`/`_direct_read`/`_direct_write`/`_indirect_read`/
    // `_indirect_write`/`_bit_addr`/`_read_bit`/`_write_bit`/`_push8`/
    // `_pop8`/`_push_pc`/`_pop_pc` private methods.

    pub(crate) fn rn_addr(&self, n: u8) -> usize {
        let bank = (self.iram[SFR_PSW as usize] >> 3) & 0x3;
        (bank * 8 + (n & 0x7)) as usize
    }

    pub(crate) fn rn(&self, n: u8) -> u8 {
        self.iram[self.rn_addr(n)]
    }

    pub(crate) fn set_rn(&mut self, n: u8, val: u8) {
        let addr = self.rn_addr(n);
        self.iram[addr] = val;
    }

    pub(crate) fn set_dptr(&mut self, val: u16) {
        self.iram[SFR_DPH as usize] = (val >> 8) as u8;
        self.iram[SFR_DPL as usize] = (val & 0xFF) as u8;
    }

    pub(crate) fn set_acc(&mut self, val: u8) {
        self.iram[SFR_ACC as usize] = val;
        self.update_parity();
    }

    /// Recompute PSW.P from ACC (even parity: `P=1` iff ACC has an odd
    /// popcount).
    pub(crate) fn update_parity(&mut self) {
        let a = self.iram[SFR_ACC as usize];
        let p = {
            let mut v = a;
            v ^= v >> 4;
            v ^= v >> 2;
            v ^= v >> 1;
            v & 1
        };
        if p != 0 {
            self.iram[SFR_PSW as usize] |= PSW_P;
        } else {
            self.iram[SFR_PSW as usize] &= !PSW_P;
        }
    }

    pub(crate) fn cy_bit(&self) -> u8 {
        u8::from(self.iram[SFR_PSW as usize] & PSW_CY != 0)
    }

    /// Update CY, AC, OV in PSW.  P is handled separately (via
    /// `set_acc`/`update_parity`), matching the Python reference.
    pub(crate) fn set_flags(&mut self, cy: u8, ac: u8, ov: u8) {
        let mut psw = self.iram[SFR_PSW as usize];
        psw &= !(PSW_CY | PSW_AC | PSW_OV);
        if cy != 0 {
            psw |= PSW_CY;
        }
        if ac != 0 {
            psw |= PSW_AC;
        }
        if ov != 0 {
            psw |= PSW_OV;
        }
        self.iram[SFR_PSW as usize] = psw;
    }

    /// Direct addressing: 0x00-0x7F hits internal RAM, 0x80-0xFF hits
    /// SFRs — both live in the same `iram` array.
    pub(crate) fn direct_read(&self, addr: u8) -> u8 {
        self.iram[addr as usize]
    }

    pub(crate) fn direct_write(&mut self, addr: u8, val: u8) {
        self.iram[addr as usize] = val;
        if addr == SFR_ACC {
            self.update_parity();
        }
    }

    /// Register-indirect addressing (`@Ri`, `i` = 0 or 1).  On the
    /// base 8051, `@Ri` can only reach 0x00-0x7F; addresses 0x80-0xFF
    /// are undefined (the base model has no upper 128 B RAM bank the
    /// 8052 has) — the Python reference raises `ValueError`, so this
    /// port panics with an equivalent message.
    pub(crate) fn indirect_read(&self, ri: u8) -> u8 {
        let addr = self.iram[self.rn_addr(ri & 1)];
        assert!(
            addr <= 0x7F,
            "intel8051-simulator: indirect address 0x{addr:02X} >= 0x80 \
             is undefined on the base 8051 (no upper RAM bank)"
        );
        self.iram[addr as usize]
    }

    pub(crate) fn indirect_write(&mut self, ri: u8, val: u8) {
        let addr = self.iram[self.rn_addr(ri & 1)];
        assert!(
            addr <= 0x7F,
            "intel8051-simulator: indirect address 0x{addr:02X} >= 0x80 \
             is undefined on the base 8051 (no upper RAM bank)"
        );
        self.iram[addr as usize] = val;
    }

    /// Resolve a bit address (0x00-0xFF) to `(iram_byte_addr, bit_pos)`.
    /// Bits 0x00-0x7F map to RAM bytes 0x20-0x2F; bits 0x80-0xFF map
    /// directly onto SFR bits (`byte = bit & 0xF8`, `pos = bit & 7`).
    pub(crate) fn bit_addr(&self, bit: u8) -> (u8, u8) {
        if bit < 0x80 {
            (0x20 + (bit >> 3), bit & 0x7)
        } else {
            (bit & 0xF8, bit & 0x7)
        }
    }

    pub(crate) fn read_bit(&self, bit: u8) -> u8 {
        let (addr, pos) = self.bit_addr(bit);
        (self.iram[addr as usize] >> pos) & 1
    }

    pub(crate) fn write_bit(&mut self, bit: u8, val: u8) {
        let (addr, pos) = self.bit_addr(bit);
        if val != 0 {
            self.iram[addr as usize] |= 1 << pos;
        } else {
            self.iram[addr as usize] &= !(1 << pos);
        }
        if addr == SFR_ACC {
            self.update_parity();
        }
    }

    pub(crate) fn push8(&mut self, val: u8) {
        let sp = self.iram[SFR_SP as usize].wrapping_add(1);
        self.iram[SFR_SP as usize] = sp;
        self.iram[sp as usize] = val;
    }

    pub(crate) fn pop8(&mut self) -> u8 {
        let sp = self.iram[SFR_SP as usize];
        let val = self.iram[sp as usize];
        self.iram[SFR_SP as usize] = sp.wrapping_sub(1);
        val
    }

    /// Push 16-bit PC — low byte first, then high byte (so the high
    /// byte ends up on top of the stack, matching `LCALL`/`ACALL`'s
    /// wire format and `pop_pc`'s pop order).
    pub(crate) fn push_pc(&mut self) {
        self.push8((self.pc & 0xFF) as u8);
        self.push8((self.pc >> 8) as u8);
    }

    /// Pop 16-bit PC — high byte first (it was pushed last), then low
    /// byte.
    pub(crate) fn pop_pc(&mut self) {
        let hi = self.pop8();
        let lo = self.pop8();
        self.pc = ((hi as u16) << 8) | lo as u16;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_state_matches_datasheet() {
        let sim = Intel8051Simulator::new();
        assert_eq!(sim.pc(), 0);
        assert_eq!(sim.sp(), 0x07);
        assert_eq!(sim.acc(), 0);
        assert!(!sim.halted());
        // Port latches P0-P3 = 0xFF at reset.
        assert_eq!(sim.read_iram(SFR_P0), 0xFF);
        assert_eq!(sim.read_iram(SFR_P1), 0xFF);
        assert_eq!(sim.read_iram(SFR_P2), 0xFF);
        assert_eq!(sim.read_iram(SFR_P3), 0xFF);
    }

    #[test]
    fn load_program_places_code_at_start_addr_and_resets_pc() {
        let mut sim = Intel8051Simulator::new();
        sim.load_program(&[0x74, 42, 0xA5], 0);
        assert_eq!(sim.pc(), 0);
        assert_eq!(sim.read_code(0), 0x74);
        assert_eq!(sim.read_code(1), 42);
        assert_eq!(sim.read_code(2), 0xA5);
    }

    #[test]
    fn mov_a_imm_then_halt() {
        // MOV A, #42 ; HALT (0xA5)
        let mut sim = Intel8051Simulator::new();
        sim.load_program(&[0x74, 42, 0xA5], 0);
        let result = sim.run_loaded_with_limit(100);
        assert!(result.halted);
        assert_eq!(result.steps, 2);
        assert_eq!(sim.acc(), 42);
    }

    #[test]
    fn add_a_imm_updates_acc_and_flags() {
        // MOV A, #0xFF ; ADD A, #0x01 ; HALT -> ACC=0, CY=1
        let mut sim = Intel8051Simulator::new();
        sim.load_program(&[0x74, 0xFF, 0x24, 0x01, 0xA5], 0);
        let result = sim.run_loaded_with_limit(100);
        assert!(result.halted);
        assert_eq!(sim.acc(), 0x00);
        assert!(sim.cy());
    }

    #[test]
    fn mov_rn_imm_and_mov_a_rn_round_trip() {
        // MOV R0, #7 ; MOV A, R0 ; HALT
        let mut sim = Intel8051Simulator::new();
        sim.load_program(&[0x78, 7, 0xE8, 0xA5], 0);
        sim.run_loaded_with_limit(100);
        assert_eq!(sim.read_register(0), 7);
        assert_eq!(sim.acc(), 7);
    }

    #[test]
    fn run_loaded_with_limit_stops_at_max_steps_when_unhalted() {
        // SJMP $ (rel = -2, encodes "jump to self") — an infinite
        // loop that never executes HALT.  Confirms the step limit
        // actually bounds execution rather than spinning forever.
        let mut sim = Intel8051Simulator::new();
        sim.load_program(&[0x80, 0xFE], 0); // SJMP -2
        let result = sim.run_loaded_with_limit(50);
        assert!(!result.halted);
        assert_eq!(result.steps, 50);
    }

    #[test]
    fn step_after_halted_is_a_noop() {
        let mut sim = Intel8051Simulator::new();
        sim.load_program(&[0xA5], 0);
        assert_eq!(sim.step(), "HALT");
        let pc_after_first_halt = sim.pc();
        assert_eq!(sim.step(), "HALT");
        assert_eq!(sim.pc(), pc_after_first_halt, "PC must not advance once halted");
    }

    #[test]
    fn djnz_loop_sums_one_through_ten() {
        // R0 = 10 (counter), R1 = 0 (accumulator).
        // loop: MOV A, R1 ; ADD A, R0 ; MOV R1, A ; DJNZ R0, loop ; HALT
        // Sum 10+9+...+1 = 55.
        let mut sim = Intel8051Simulator::new();
        let code: &[u8] = &[
            0x78, 10, // MOV R0, #10
            0x79, 0, // MOV R1, #0
            // loop:
            0xE9, // MOV A, R1
            0x28, // ADD A, R0
            0xF9, // MOV R1, A
            0xD8, (-5i8) as u8, // DJNZ R0, loop (rel = loop_target(4) - next_pc(9) = -5)
            0xA5, // HALT
        ];
        sim.load_program(code, 0);
        let result = sim.run_loaded_with_limit(1000);
        assert!(result.halted);
        assert_eq!(sim.read_register(1), 55);
    }
}
