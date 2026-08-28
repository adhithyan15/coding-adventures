//! Top-level MOS 6502 simulator combining decode + execute.
//!
//! The public lifecycle returns typed atomic errors, complete owned state,
//! before/after instruction traces, and transactional bounded-run results.
//! `load_program_at` supports the non-zero origins conventional for 6502
//! software while preserving the zero-origin convenience API.

use std::fmt;

use cpu_simulator::Memory;

use crate::decode;
use crate::execute;

/// Complete MOS 6502 (NMOS) simulator: accumulator + X/Y index registers,
/// an 8-bit stack pointer (offset into the fixed `$0100-$01FF` stack page),
/// a 16-bit program counter, the 7 active processor-status flags, and flat
/// byte-addressable memory and 240 memory-mapped I/O latches.
pub struct Mos6502Simulator {
    /// Accumulator — every arithmetic/logical operation targets this.
    pub a: u8,
    /// Index register X — addressing offsets, loop counters.
    pub x: u8,
    /// Index register Y — addressing offsets, loop counters.
    pub y: u8,
    /// Stack pointer — effective stack address is `0x0100 + s`.
    pub s: u8,
    /// Program counter.
    pub pc: u16,

    /// Negative flag — bit 7 of the last result.
    pub flag_n: bool,
    /// Overflow flag — signed arithmetic overflow.
    pub flag_v: bool,
    /// Break flag — set in the P byte pushed by `BRK`/`PHP`.
    pub flag_b: bool,
    /// Decimal flag — BCD mode for `ADC`/`SBC`.
    pub flag_d: bool,
    /// Interrupt-disable flag.
    pub flag_i: bool,
    /// Zero flag — last result was zero.
    pub flag_z: bool,
    /// Carry flag — carry out (or not-borrow for `SBC`).
    pub flag_c: bool,

    /// Flat byte-addressable memory.
    pub mem: Memory,
    /// True once `BRK` (the HALT sentinel — see `opcodes::BRK_OPCODE`) has
    /// executed.
    pub halted: bool,
    /// Values returned by reads from `$FF00-$FFEF`.
    pub input_ports: [u8; 240],
    /// Values captured by writes to `$FF00-$FFEF`.
    pub output_ports: [u8; 240],
}

/// Complete owned architectural state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mos6502State {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub s: u8,
    pub pc: u16,
    pub flag_n: bool,
    pub flag_v: bool,
    pub flag_b: bool,
    pub flag_d: bool,
    pub flag_i: bool,
    pub flag_z: bool,
    pub flag_c: bool,
    pub halted: bool,
    pub memory: Box<[u8]>,
    pub input_ports: [u8; 240],
    pub output_ports: [u8; 240],
}

/// One successfully completed instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub address: u16,
    pub raw: Vec<u8>,
    pub mnemonic: String,
    pub state_before: Mos6502State,
    pub state_after: Mos6502State,
}

/// Observable outcome of a bounded simulator run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub pc: u16,
    pub traces: Vec<StepTrace>,
    pub final_state: Mos6502State,
}

/// Typed lifecycle failures. Every failure preserves the complete prior state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mos6502Error {
    ProgramTooLarge { length: usize, capacity: usize },
    UnknownOpcode { address: u16, opcode: u8 },
    Halted,
    InvalidStateMemory { length: usize },
    InvalidPort { port: u8 },
}

impl fmt::Display for Mos6502Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProgramTooLarge { length, capacity } => {
                write!(f, "program has {length} bytes; capacity is {capacity}")
            }
            Self::UnknownOpcode { address, opcode } => {
                write!(f, "unknown opcode {opcode:#04X} at {address:#06X}")
            }
            Self::Halted => write!(f, "CPU is halted"),
            Self::InvalidStateMemory { length } => {
                write!(f, "state memory has {length} bytes; expected 65536")
            }
            Self::InvalidPort { port } => write!(f, "port {port} is outside 0..240"),
        }
    }
}

impl std::error::Error for Mos6502Error {}

/// Power-on stack pointer, matching the Python original's `_RESET_S`.
const RESET_S: u8 = 0xFD;

impl Mos6502Simulator {
    /// Create a simulator with the architecturally fixed 64 KiB address space.
    ///
    /// The argument is retained for source compatibility and is ignored.
    pub fn new(_memory_size: usize) -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            s: RESET_S,
            pc: 0,
            // P = 0x24 at power-on: bit 5 (unused) implicitly 1, I=1,
            // everything else clear -- matches the Python original's
            // `_RESET_P = 0x24`.
            flag_n: false,
            flag_v: false,
            flag_b: false,
            flag_d: false,
            flag_i: true,
            flag_z: false,
            flag_c: false,
            mem: Memory::new(65_536),
            halted: false,
            input_ports: [0; 240],
            output_ports: [0; 240],
        }
    }

    /// Load a program into memory at the given origin and set `pc` there
    /// — mirrors the Python original's `load(program, origin)`.
    pub fn load_program_at(&mut self, program: &[u8], origin: u16) -> Result<(), Mos6502Error> {
        if program.len() > 65_536 {
            return Err(Mos6502Error::ProgramTooLarge {
                length: program.len(),
                capacity: 65_536,
            });
        }
        for (offset, byte) in program.iter().copied().enumerate() {
            self.mem
                .write_byte(origin.wrapping_add(offset as u16) as usize, byte);
        }
        self.pc = origin;
        self.halted = false;
        Ok(())
    }

    /// Load a program at address 0 (the convention every other Rust ISA
    /// simulator in this repo uses for its `load_program`).
    pub fn load_program(&mut self, program: &[u8]) -> Result<(), Mos6502Error> {
        self.load_program_at(program, 0)
    }

    /// Run a freshly-loaded program (at address 0) until halted or 10000
    /// steps (safety limit).
    pub fn run(&mut self, program: &[u8]) -> Result<ExecutionResult, Mos6502Error> {
        let before = self.snapshot();
        self.load_program(program)?;
        match self.run_loaded_with_limit(10_000) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.restore(&before)?;
                Err(error)
            }
        }
    }

    /// Run instructions from an already-loaded program.
    pub fn run_loaded(&mut self) -> Result<ExecutionResult, Mos6502Error> {
        self.run_loaded_with_limit(10_000)
    }

    /// Run the already-loaded program for at most `max_steps`
    /// instructions.
    ///
    /// A non-halting result means the budget was exhausted — this makes
    /// the execution limit visible to callers instead of silently
    /// treating an infinite loop as success.
    pub fn run_loaded_with_limit(
        &mut self,
        max_steps: usize,
    ) -> Result<ExecutionResult, Mos6502Error> {
        let before = self.snapshot();
        let mut steps = 0;
        let mut traces = Vec::new();
        while steps < max_steps {
            if self.halted {
                break;
            }
            let trace = match self.step() {
                Ok(trace) => trace,
                Err(error) => {
                    self.restore(&before)?;
                    return Err(error);
                }
            };
            traces.push(trace);
            steps += 1;
        }
        Ok(ExecutionResult {
            halted: self.halted,
            steps,
            pc: self.pc,
            traces,
            final_state: self.snapshot(),
        })
    }

    /// Execute a single instruction and return its mnemonic.
    ///
    /// Fetch-decodes at the current `pc` (which `decode::fetch_decode`
    /// advances past the whole instruction as a side effect), then
    /// dispatches to `execute::execute`.  An illegal/undocumented opcode
    /// byte is a **fail-closed halt** (mirrors how `mips-r2000-simulator`
    /// handles signed-overflow/divide-by-zero: no exception channel exists
    /// through `step() -> String`, so the simulator stops rather than
    /// silently corrupting state or panicking) — the Python original
    /// raises `ValueError` for the same condition instead.
    pub fn step(&mut self) -> Result<StepTrace, Mos6502Error> {
        if self.halted {
            return Err(Mos6502Error::Halted);
        }

        let address = self.pc;
        let opcode = self.read_byte(address);
        let (_, mode) = crate::opcodes::lookup(opcode)
            .ok_or(Mos6502Error::UnknownOpcode { address, opcode })?;
        let length = mode.instruction_len();
        let raw = (0..length)
            .map(|offset| self.read_byte(address.wrapping_add(offset as u16)))
            .collect();
        let state_before = self.snapshot();
        let mut pc = address;
        let decoded =
            decode::fetch_decode_with_io(&self.mem, &self.input_ports, &mut pc, self.x, self.y)
                .expect("opcode was preflighted");
        self.pc = pc;

        let mnemonic = execute::execute(self, &decoded).to_string();
        Ok(StepTrace {
            address,
            raw,
            mnemonic,
            state_before,
            state_after: self.snapshot(),
        })
    }

    /// Run a list of raw instruction bytes (convenience for tests).
    pub fn run_bytes(&mut self, bytes: &[u8]) -> Result<ExecutionResult, Mos6502Error> {
        self.run(bytes)
    }

    pub fn snapshot(&self) -> Mos6502State {
        Mos6502State {
            a: self.a,
            x: self.x,
            y: self.y,
            s: self.s,
            pc: self.pc,
            flag_n: self.flag_n,
            flag_v: self.flag_v,
            flag_b: self.flag_b,
            flag_d: self.flag_d,
            flag_i: self.flag_i,
            flag_z: self.flag_z,
            flag_c: self.flag_c,
            halted: self.halted,
            memory: (0..65_536)
                .map(|address| self.mem.read_byte(address))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            input_ports: self.input_ports,
            output_ports: self.output_ports,
        }
    }

    pub fn state(&self) -> Mos6502State {
        self.snapshot()
    }

    pub fn restore(&mut self, state: &Mos6502State) -> Result<(), Mos6502Error> {
        if state.memory.len() != 65_536 {
            return Err(Mos6502Error::InvalidStateMemory {
                length: state.memory.len(),
            });
        }
        self.a = state.a;
        self.x = state.x;
        self.y = state.y;
        self.s = state.s;
        self.pc = state.pc;
        self.flag_n = state.flag_n;
        self.flag_v = state.flag_v;
        self.flag_b = state.flag_b;
        self.flag_d = state.flag_d;
        self.flag_i = state.flag_i;
        self.flag_z = state.flag_z;
        self.flag_c = state.flag_c;
        self.halted = state.halted;
        self.input_ports = state.input_ports;
        self.output_ports = state.output_ports;
        self.mem.load_bytes(0, &state.memory);
        Ok(())
    }

    pub fn reset(&mut self) {
        let input_ports = self.input_ports;
        let output_ports = self.output_ports;
        *self = Self::new(65_536);
        self.input_ports = input_ports;
        self.output_ports = output_ports;
    }

    pub fn set_input_port(&mut self, port: u8, value: u8) -> Result<(), Mos6502Error> {
        let slot = self
            .input_ports
            .get_mut(port as usize)
            .ok_or(Mos6502Error::InvalidPort { port })?;
        *slot = value;
        Ok(())
    }

    pub fn get_output_port(&self, port: u8) -> Result<u8, Mos6502Error> {
        self.output_ports
            .get(port as usize)
            .copied()
            .ok_or(Mos6502Error::InvalidPort { port })
    }

    pub(crate) fn read_byte(&self, address: u16) -> u8 {
        if (0xFF00..=0xFFEF).contains(&address) {
            self.input_ports[(address - 0xFF00) as usize]
        } else {
            self.mem.read_byte(address as usize)
        }
    }

    pub(crate) fn write_byte(&mut self, address: u16, value: u8) {
        if (0xFF00..=0xFFEF).contains(&address) {
            self.output_ports[(address - 0xFF00) as usize] = value;
        } else {
            self.mem.write_byte(address as usize, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcodes::{BRK_OPCODE, LDA_IMM_OPCODE};

    #[test]
    fn power_on_state_matches_python_reset() {
        let sim = Mos6502Simulator::new(65536);
        assert_eq!(sim.a, 0);
        assert_eq!(sim.x, 0);
        assert_eq!(sim.y, 0);
        assert_eq!(sim.s, 0xFD);
        assert_eq!(sim.pc, 0);
        assert!(sim.flag_i, "I=1 at power-on");
        assert!(
            !sim.flag_n && !sim.flag_v && !sim.flag_b && !sim.flag_d && !sim.flag_z && !sim.flag_c
        );
        assert!(!sim.halted);
    }

    // ── The trivial "load immediate into accumulator + halt-convention"
    // sequence the mos6502-backend smoke test relies on: LDA #42; BRK.
    #[test]
    fn lda_immediate_then_brk_halts_with_a_equals_42() {
        let mut sim = Mos6502Simulator::new(65536);
        sim.run(&[LDA_IMM_OPCODE, 42, BRK_OPCODE]).unwrap();
        assert!(sim.halted);
        assert_eq!(sim.a, 42);
    }

    #[test]
    fn bounded_run_reports_halt_and_instruction_count() {
        let mut sim = Mos6502Simulator::new(65536);
        sim.load_program(&[LDA_IMM_OPCODE, 42, BRK_OPCODE]).unwrap();
        let result = sim.run_loaded_with_limit(10).unwrap();
        assert!(result.halted);
        assert_eq!(result.steps, 2); // LDA #42, then BRK
    }

    #[test]
    fn lda_sets_zero_flag() {
        let mut sim = Mos6502Simulator::new(65536);
        sim.run(&[LDA_IMM_OPCODE, 0x00, BRK_OPCODE]).unwrap();
        assert!(sim.flag_z);
        assert!(!sim.flag_n);
    }

    #[test]
    fn lda_sets_negative_flag() {
        let mut sim = Mos6502Simulator::new(65536);
        sim.run(&[LDA_IMM_OPCODE, 0x80, BRK_OPCODE]).unwrap();
        assert!(sim.flag_n);
        assert!(!sim.flag_z);
    }

    #[test]
    fn ldx_ldy_immediate() {
        let mut sim = Mos6502Simulator::new(65536);
        // LDX #10; LDY #20; BRK
        sim.run(&[0xA2, 10, 0xA0, 20, BRK_OPCODE]).unwrap();
        assert_eq!(sim.x, 10);
        assert_eq!(sim.y, 20);
    }

    #[test]
    fn sta_zero_page_then_lda_round_trips() {
        let mut sim = Mos6502Simulator::new(65536);
        // LDA #0x55; STA $10; LDA #0; LDA $10; BRK
        sim.run(&[
            LDA_IMM_OPCODE,
            0x55,
            0x85,
            0x10, // STA $10
            LDA_IMM_OPCODE,
            0x00,
            0xA5,
            0x10, // LDA $10
            BRK_OPCODE,
        ])
        .unwrap();
        assert_eq!(sim.a, 0x55);
        assert_eq!(sim.mem.read_byte(0x10), 0x55);
    }

    #[test]
    fn adc_basic_binary() {
        let mut sim = Mos6502Simulator::new(65536);
        // LDA #10; CLC; ADC #5; BRK -> A = 15
        sim.run(&[LDA_IMM_OPCODE, 10, 0x18, 0x69, 5, BRK_OPCODE])
            .unwrap();
        assert_eq!(sim.a, 15);
        assert!(!sim.flag_c);
    }

    #[test]
    fn adc_sets_carry_on_overflow() {
        let mut sim = Mos6502Simulator::new(65536);
        // LDA #0xFF; CLC; ADC #1; BRK -> A = 0, C = 1
        sim.run(&[LDA_IMM_OPCODE, 0xFF, 0x18, 0x69, 1, BRK_OPCODE])
            .unwrap();
        assert_eq!(sim.a, 0);
        assert!(sim.flag_c);
        assert!(sim.flag_z);
    }

    #[test]
    fn adc_decimal_mode_bcd() {
        let mut sim = Mos6502Simulator::new(65536);
        // SED; LDA #0x09; CLC; ADC #0x01; BRK -> BCD 9+1=10 -> A=0x10
        sim.run(&[0xF8, LDA_IMM_OPCODE, 0x09, 0x18, 0x69, 0x01, BRK_OPCODE])
            .unwrap();
        assert_eq!(sim.a, 0x10);
        assert!(!sim.flag_c);
    }

    #[test]
    fn sbc_basic_binary() {
        let mut sim = Mos6502Simulator::new(65536);
        // LDA #10; SEC; SBC #3; BRK -> A = 7 (SEC clears the borrow)
        sim.run(&[LDA_IMM_OPCODE, 10, 0x38, 0xE9, 3, BRK_OPCODE])
            .unwrap();
        assert_eq!(sim.a, 7);
        assert!(sim.flag_c); // no borrow
    }

    #[test]
    fn and_ora_eor() {
        let mut sim = Mos6502Simulator::new(65536);
        // LDA #0xF0; AND #0x0F; BRK -> 0
        sim.run(&[LDA_IMM_OPCODE, 0xF0, 0x29, 0x0F, BRK_OPCODE])
            .unwrap();
        assert_eq!(sim.a, 0);
    }

    #[test]
    fn inx_iny_dex_dey() {
        let mut sim = Mos6502Simulator::new(65536);
        // LDX #5; INX; LDY #5; DEY; BRK
        sim.run(&[0xA2, 5, 0xE8, 0xA0, 5, 0x88, BRK_OPCODE])
            .unwrap();
        assert_eq!(sim.x, 6);
        assert_eq!(sim.y, 4);
    }

    #[test]
    fn cmp_sets_carry_when_a_gte_operand() {
        let mut sim = Mos6502Simulator::new(65536);
        // LDA #10; CMP #5; BRK -> C=1 (10>=5), Z=0
        sim.run(&[LDA_IMM_OPCODE, 10, 0xC9, 5, BRK_OPCODE]).unwrap();
        assert!(sim.flag_c);
        assert!(!sim.flag_z);
    }

    #[test]
    fn cmp_sets_zero_when_equal() {
        let mut sim = Mos6502Simulator::new(65536);
        sim.run(&[LDA_IMM_OPCODE, 5, 0xC9, 5, BRK_OPCODE]).unwrap();
        assert!(sim.flag_z);
        assert!(sim.flag_c);
    }

    #[test]
    fn branch_beq_taken() {
        let mut sim = Mos6502Simulator::new(65536);
        // LDA #0; BEQ +2 (skip next LDA); LDA #99; LDA #42; BRK
        sim.run(&[
            LDA_IMM_OPCODE,
            0,
            0xF0,
            2,
            LDA_IMM_OPCODE,
            99,
            LDA_IMM_OPCODE,
            42,
            BRK_OPCODE,
        ])
        .unwrap();
        assert_eq!(sim.a, 42);
    }

    #[test]
    fn branch_bne_not_taken() {
        let mut sim = Mos6502Simulator::new(65536);
        // LDA #0; BNE +2; LDA #42; BRK  (Z is set, so BNE not taken)
        sim.run(&[LDA_IMM_OPCODE, 0, 0xD0, 2, LDA_IMM_OPCODE, 42, BRK_OPCODE])
            .unwrap();
        assert_eq!(sim.a, 42);
    }

    #[test]
    fn backward_branch_loop_sum_1_to_5() {
        // LDX #5; LDA #0
        // loop: CLC; ADC #1(via X? simpler: just decrement X and add 1 each time)
        // Use a simple counting loop that decrements X to 0, incrementing A each time.
        let mut sim = Mos6502Simulator::new(65536);
        let prog = [
            0xA2,
            5, // LDX #5
            LDA_IMM_OPCODE,
            0, // LDA #0
            // loop (offset 4):
            0x18, // CLC
            0x69,
            1,    // ADC #1
            0xCA, // DEX
            0xD0,
            (-6i8) as u8, // BNE loop (back to CLC at offset 4; pc after this instr = 10, target = 10-6=4)
            BRK_OPCODE,
        ];
        sim.run(&prog).unwrap();
        assert_eq!(sim.a, 5);
        assert_eq!(sim.x, 0);
    }

    #[test]
    fn jmp_absolute() {
        let mut sim = Mos6502Simulator::new(65536);
        // JMP $0006; LDA #99 (skipped); LDA #99 (skipped); LDA #42; BRK
        sim.run(&[
            0x4C,
            0x06,
            0x00,
            LDA_IMM_OPCODE,
            99,
            0x00, /*pad*/
            LDA_IMM_OPCODE,
            42,
            BRK_OPCODE,
        ])
        .unwrap();
        assert_eq!(sim.a, 42);
    }

    #[test]
    fn jsr_rts_call_return() {
        let mut sim = Mos6502Simulator::new(65536);
        // main: JSR sub($0006); LDA #42; BRK
        // sub (addr 6): LDA #7; RTS
        let prog = [
            0x20,
            0x06,
            0x00, // JSR $0006
            LDA_IMM_OPCODE,
            99,         // would be A=99 if sub didn't run first -- overwritten after return
            BRK_OPCODE, // 6 total bytes for main, but sub starts at 6 -- adjust layout below
        ];
        let _ = prog;
        // Lay out explicitly: main at 0..3 is JSR $0008; then main continues
        // at 3 with LDA #42; BRK. Sub at 8: LDA #7; RTS.
        let mut mem_prog = vec![0x20, 0x08, 0x00]; // JSR $0008
        mem_prog.push(LDA_IMM_OPCODE);
        mem_prog.push(42); // LDA #42 (after return)
        mem_prog.push(BRK_OPCODE);
        mem_prog.push(0x00); // pad byte at 6 (unused)
        mem_prog.push(0x00); // pad byte at 7 (unused)
        mem_prog.push(LDA_IMM_OPCODE);
        mem_prog.push(7); // sub: LDA #7  (addr 8,9)
        mem_prog.push(0x60); // RTS (addr 10)
        sim.run(&mem_prog).unwrap();
        assert_eq!(sim.a, 42, "caller's LDA #42 must run after RTS returns");
    }

    #[test]
    fn php_plp_round_trip_flags() {
        let mut sim = Mos6502Simulator::new(65536);
        // SEC; PHP; CLC; PLP; BRK -> C restored to 1
        sim.run(&[0x38, 0x08, 0x18, 0x28, BRK_OPCODE]).unwrap();
        assert!(sim.flag_c);
    }

    #[test]
    fn pha_pla_round_trip_accumulator() {
        let mut sim = Mos6502Simulator::new(65536);
        // LDA #77; PHA; LDA #0; PLA; BRK -> A=77
        sim.run(&[
            LDA_IMM_OPCODE,
            77,
            0x48,
            LDA_IMM_OPCODE,
            0,
            0x68,
            BRK_OPCODE,
        ])
        .unwrap();
        assert_eq!(sim.a, 77);
    }

    #[test]
    fn illegal_opcode_is_typed_and_atomic() {
        let mut sim = Mos6502Simulator::new(65536);
        assert_eq!(
            sim.run(&[0x02]),
            Err(Mos6502Error::UnknownOpcode {
                address: 0,
                opcode: 0x02,
            })
        );
        assert!(!sim.halted);
    }

    #[test]
    fn step_returns_mnemonic() {
        let mut sim = Mos6502Simulator::new(65536);
        sim.load_program(&[LDA_IMM_OPCODE, 42, BRK_OPCODE]).unwrap();
        assert_eq!(sim.step().unwrap().mnemonic, "LDA");
        assert_eq!(sim.a, 42);
        assert_eq!(sim.step().unwrap().mnemonic, "BRK");
        assert!(sim.halted);
    }

    #[test]
    fn step_after_halt_returns_halted() {
        let mut sim = Mos6502Simulator::new(65536);
        sim.load_program(&[BRK_OPCODE]).unwrap();
        sim.step().unwrap();
        assert!(sim.halted);
        assert_eq!(sim.step(), Err(Mos6502Error::Halted));
    }

    #[test]
    fn load_program_at_nonzero_origin() {
        let mut sim = Mos6502Simulator::new(65536);
        sim.load_program_at(&[LDA_IMM_OPCODE, 42, BRK_OPCODE], 0x0200)
            .unwrap();
        assert_eq!(sim.pc, 0x0200);
        sim.run_loaded_with_limit(10).unwrap();
        assert_eq!(sim.a, 42);
    }
}
