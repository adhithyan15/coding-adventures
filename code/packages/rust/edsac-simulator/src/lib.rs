//! Functional simulator for the 1949 EDSAC order code.
//!
//! The machine follows the University of Cambridge's surviving order-code
//! notes and reference simulator. Arithmetic uses the reference simulator's
//! 15-bit tank slices so serial shifts and multiplication remain exact.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Number of addressable short storage locations.
pub const MEMORY_CELLS: usize = 1_024;
/// Maximum raw value in an even cell, including its sandwich digit.
pub const EVEN_CELL_MASK: u32 = 0x3_ffff;
/// Maximum raw value in an odd 17-bit cell.
pub const ODD_CELL_MASK: u32 = 0x1_ffff;
/// Maximum long-word value (17 senior bits and 18 junior bits).
pub const LONG_WORD_MASK: u64 = 0x7_ffff_ffff;
const SLICE_MASK: u16 = 0x7fff;

/// Operand-length discriminator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Length {
    Short,
    Long,
}

/// EDSAC five-bit function codes, named by their teleprinter letters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Opcode {
    E = 3,
    R = 4,
    T = 5,
    Y = 6,
    U = 7,
    I = 8,
    O = 9,
    S = 12,
    Z = 13,
    F = 17,
    H = 21,
    N = 22,
    L = 25,
    X = 26,
    G = 27,
    A = 28,
    C = 30,
    V = 31,
}

impl Opcode {
    fn decode(code: u8) -> Option<Self> {
        Some(match code {
            3 => Self::E,
            4 => Self::R,
            5 => Self::T,
            6 => Self::Y,
            7 => Self::U,
            8 => Self::I,
            9 => Self::O,
            12 => Self::S,
            13 => Self::Z,
            17 => Self::F,
            21 => Self::H,
            22 => Self::N,
            25 => Self::L,
            26 => Self::X,
            27 => Self::G,
            28 => Self::A,
            30 => Self::C,
            31 => Self::V,
            _ => return None,
        })
    }

    const fn name(self) -> &'static str {
        match self {
            Self::E => "E",
            Self::R => "R",
            Self::T => "T",
            Self::Y => "Y",
            Self::U => "U",
            Self::I => "I",
            Self::O => "O",
            Self::S => "S",
            Self::Z => "Z",
            Self::F => "F",
            Self::H => "H",
            Self::N => "N",
            Self::L => "L",
            Self::X => "X",
            Self::G => "G",
            Self::A => "A",
            Self::C => "C",
            Self::V => "V",
        }
    }

    const fn requires_short(self) -> bool {
        matches!(
            self,
            Self::E | Self::Y | Self::I | Self::O | Self::F | Self::Z | Self::G
        )
    }
}

/// Encode one checked order. The address is masked to ten bits.
pub const fn encode_order(opcode: Opcode, address: usize, length: Length) -> u32 {
    encode_raw_order(opcode as u8, address, matches!(length, Length::Long))
}

/// Encode a raw five-bit function code for decode-boundary tests and tooling.
pub const fn encode_raw_order(opcode: u8, address: usize, long: bool) -> u32 {
    (((opcode as u32) & 0x1f) << 12) | (((address as u32) & 0x03ff) << 1) | (long as u32)
}

/// Complete owned architectural, peripheral, and load-lifecycle state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EdsacState {
    pub memory: Box<[u32; MEMORY_CELLS]>,
    pub scr: usize,
    pub order_tank: u32,
    pub accumulator: [u16; 5],
    pub multiplier: [u16; 3],
    pub multiplicand: [u16; 3],
    pub multiplicand_length: Length,
    pub input: Vec<u8>,
    pub input_cursor: usize,
    pub output: Vec<u8>,
    pub last_output: u8,
    pub halted: bool,
    pub loaded_origin: usize,
    pub loaded_words: usize,
}

/// One retired EDSAC order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub pc_before: usize,
    pub pc_after: usize,
    pub instruction: u32,
    pub opcode: Opcode,
    pub address: usize,
    pub length: Length,
    pub mnemonic: String,
    pub description: String,
    pub io_code: Option<u8>,
}

/// Successful bounded execution result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub final_state: EdsacState,
    pub traces: Vec<StepTrace>,
}

/// Fail-closed transport, state, decode, and execution errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EdsacError {
    InvalidTransportLength {
        bytes: usize,
    },
    InvalidOrigin {
        origin: usize,
    },
    ProgramTooLarge {
        words: usize,
        capacity: usize,
    },
    InvalidMemoryWord {
        address: usize,
        word: u32,
        maximum: u32,
    },
    InvalidAddress {
        address: usize,
    },
    MisalignedLongAddress {
        address: usize,
    },
    InvalidLongWord {
        word: u64,
    },
    InvalidRegisterSlice {
        register: &'static str,
        index: usize,
        value: u16,
    },
    InvalidRegisterPadding {
        register: &'static str,
        value: u16,
        required_zero_mask: u16,
    },
    InvalidSequenceControl {
        scr: usize,
    },
    InvalidOrderTank {
        order: u32,
    },
    InvalidInputCode {
        index: usize,
        code: u8,
    },
    InvalidInputCursor {
        cursor: usize,
        length: usize,
    },
    InvalidOutputCode {
        index: usize,
        code: u8,
    },
    InvalidLastOutput {
        code: u8,
    },
    InvalidLoadedRange {
        origin: usize,
        words: usize,
    },
    Halted,
    FetchOutsideProgram {
        scr: usize,
        origin: usize,
        words: usize,
    },
    UnknownOpcode {
        opcode: u8,
        instruction: u32,
    },
    InvalidLength {
        opcode: Opcode,
        instruction: u32,
    },
    InputExhausted,
    SequenceControlOverflow,
    MaxStepsExceeded {
        max_steps: usize,
    },
}

impl Display for EdsacError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransportLength { bytes } => {
                write!(
                    formatter,
                    "program transport has {bytes} bytes, not a multiple of four"
                )
            }
            Self::InvalidOrigin { origin } => {
                write!(
                    formatter,
                    "origin {origin} is outside the 1024-location store"
                )
            }
            Self::ProgramTooLarge { words, capacity } => write!(
                formatter,
                "program has {words} cells but the selected origin holds {capacity}"
            ),
            Self::InvalidMemoryWord {
                address,
                word,
                maximum,
            } => write!(
                formatter,
                "cell {address} contains {word:#x}, exceeding its {maximum:#x} maximum"
            ),
            Self::InvalidAddress { address } => {
                write!(formatter, "storage address {address} is outside 0..1024")
            }
            Self::MisalignedLongAddress { address } => {
                write!(formatter, "long-word address {address} is not even")
            }
            Self::InvalidLongWord { word } => {
                write!(formatter, "long word {word:#x} exceeds 35 bits")
            }
            Self::InvalidRegisterSlice {
                register,
                index,
                value,
            } => write!(
                formatter,
                "{register} slice {index} contains {value:#x}, exceeding 15 bits"
            ),
            Self::InvalidRegisterPadding {
                register,
                value,
                required_zero_mask,
            } => write!(
                formatter,
                "{register} tail {value:#x} sets padding bits {required_zero_mask:#x}"
            ),
            Self::InvalidSequenceControl { scr } => {
                write!(formatter, "sequence control value {scr} exceeds ten bits")
            }
            Self::InvalidOrderTank { order } => {
                write!(formatter, "order tank value {order:#x} exceeds 17 bits")
            }
            Self::InvalidInputCode { index, code } => {
                write!(
                    formatter,
                    "input code {index} is {code}, exceeding five bits"
                )
            }
            Self::InvalidInputCursor { cursor, length } => write!(
                formatter,
                "input cursor {cursor} exceeds input length {length}"
            ),
            Self::InvalidOutputCode { index, code } => {
                write!(
                    formatter,
                    "output code {index} is {code}, exceeding five bits"
                )
            }
            Self::InvalidLastOutput { code } => {
                write!(formatter, "last output code {code} exceeds five bits")
            }
            Self::InvalidLoadedRange { origin, words } => write!(
                formatter,
                "installed range {origin}..{} exceeds storage",
                origin.saturating_add(*words)
            ),
            Self::Halted => write!(formatter, "the EDSAC is halted"),
            Self::FetchOutsideProgram { scr, origin, words } => write!(
                formatter,
                "sequence control {scr} is outside installed range {origin}..{}",
                origin.saturating_add(*words)
            ),
            Self::UnknownOpcode {
                opcode,
                instruction,
            } => write!(
                formatter,
                "unknown function code {opcode} in order {instruction:#07x}"
            ),
            Self::InvalidLength {
                opcode,
                instruction,
            } => write!(
                formatter,
                "{} does not accept the long discriminator in order {instruction:#07x}",
                opcode.name()
            ),
            Self::InputExhausted => write!(formatter, "paper-tape input is exhausted"),
            Self::SequenceControlOverflow => {
                write!(
                    formatter,
                    "sequential execution would wrap sequence control"
                )
            }
            Self::MaxStepsExceeded { max_steps } => {
                write!(
                    formatter,
                    "maximum execution step count {max_steps} exceeded"
                )
            }
        }
    }
}

impl Error for EdsacError {}

/// Instruction-level 1949 EDSAC simulator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EdsacSimulator {
    state: EdsacState,
}

impl Default for EdsacSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl EdsacSimulator {
    /// Construct a completely clear machine without Initial Orders installed.
    pub fn new() -> Self {
        Self {
            state: EdsacState {
                memory: Box::new([0; MEMORY_CELLS]),
                scr: 0,
                order_tank: 0,
                accumulator: [0; 5],
                multiplier: [0; 3],
                multiplicand: [0; 3],
                multiplicand_length: Length::Short,
                input: Vec::new(),
                input_cursor: 0,
                output: Vec::new(),
                last_output: 0,
                halted: false,
                loaded_origin: 0,
                loaded_words: 0,
            },
        }
    }

    /// Restore the completely clear state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Clone the complete machine state.
    pub fn state(&self) -> EdsacState {
        self.state.clone()
    }

    /// Restore a validated complete machine state atomically.
    pub fn restore(&mut self, state: &EdsacState) -> Result<(), EdsacError> {
        validate_state(state)?;
        self.state = state.clone();
        Ok(())
    }

    /// Decode strict little-endian 32-bit storage-cell containers.
    pub fn load(&mut self, program: &[u8], origin: usize) -> Result<usize, EdsacError> {
        if !program.len().is_multiple_of(4) {
            return Err(EdsacError::InvalidTransportLength {
                bytes: program.len(),
            });
        }
        let words = program.len() / 4;
        validate_program_extent(words, origin)?;
        let decoded = program
            .as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| u32::from_le_bytes(*bytes))
            .collect::<Vec<_>>();
        self.load_words(&decoded, origin)
    }

    /// Replace the machine with a clear state and install raw storage cells.
    pub fn load_words(&mut self, program: &[u32], origin: usize) -> Result<usize, EdsacError> {
        validate_program_extent(program.len(), origin)?;
        for (index, &word) in program.iter().enumerate() {
            validate_memory_word(origin + index, word)?;
        }

        let mut next = Self::new();
        next.state.memory[origin..origin + program.len()].copy_from_slice(program);
        next.state.scr = origin;
        next.state.loaded_origin = origin;
        next.state.loaded_words = program.len();
        *self = next;
        Ok(program.len())
    }

    /// Replace the five-bit paper-tape rows and rewind its cursor.
    pub fn set_input(&mut self, input: &[u8]) -> Result<(), EdsacError> {
        for (index, &code) in input.iter().enumerate() {
            if code > 0x1f {
                return Err(EdsacError::InvalidInputCode { index, code });
            }
        }
        self.state.input = input.to_vec();
        self.state.input_cursor = 0;
        Ok(())
    }

    /// Set the five exact accumulator tank slices.
    pub fn set_accumulator(&mut self, accumulator: [u16; 5]) -> Result<(), EdsacError> {
        validate_accumulator(&accumulator)?;
        self.state.accumulator = accumulator;
        Ok(())
    }

    /// Read one raw short storage location.
    pub fn read_memory(&self, address: usize) -> Result<u32, EdsacError> {
        self.state
            .memory
            .get(address)
            .copied()
            .ok_or(EdsacError::InvalidAddress { address })
    }

    /// Write one checked raw short storage location.
    pub fn write_memory(&mut self, address: usize, word: u32) -> Result<(), EdsacError> {
        validate_memory_word(address, word)?;
        self.state.memory[address] = word;
        Ok(())
    }

    /// Read one aligned 35-bit long word.
    pub fn read_long(&self, address: usize) -> Result<u64, EdsacError> {
        validate_long_address(address)?;
        Ok(((self.state.memory[address + 1] as u64) << 18) | self.state.memory[address] as u64)
    }

    /// Write one aligned 35-bit long word.
    pub fn write_long(&mut self, address: usize, word: u64) -> Result<(), EdsacError> {
        validate_long_address(address)?;
        if word > LONG_WORD_MASK {
            return Err(EdsacError::InvalidLongWord { word });
        }
        self.state.memory[address] = (word as u32) & EVEN_CELL_MASK;
        self.state.memory[address + 1] = (word >> 18) as u32;
        Ok(())
    }

    /// Execute one order atomically.
    pub fn step(&mut self) -> Result<StepTrace, EdsacError> {
        let mut candidate = self.clone();
        let trace = candidate.step_inner()?;
        *self = candidate;
        Ok(trace)
    }

    /// Run until Z retires, transactionally bounded by `max_steps`.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, EdsacError> {
        let mut candidate = self.clone();
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if candidate.state.halted {
                let result = candidate.result(traces);
                *self = candidate;
                return Ok(result);
            }
            traces.push(candidate.step_inner()?);
        }
        if candidate.state.halted {
            let result = candidate.result(traces);
            *self = candidate;
            return Ok(result);
        }
        Err(EdsacError::MaxStepsExceeded { max_steps })
    }

    /// Load and transactionally execute raw storage cells.
    pub fn execute_words(
        &mut self,
        program: &[u32],
        origin: usize,
        max_steps: usize,
    ) -> Result<ExecutionResult, EdsacError> {
        let mut candidate = self.clone();
        candidate.load_words(program, origin)?;
        let result = candidate.run(max_steps)?;
        *self = candidate;
        Ok(result)
    }

    /// Load and transactionally execute little-endian storage containers.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: usize,
        max_steps: usize,
    ) -> Result<ExecutionResult, EdsacError> {
        let mut candidate = self.clone();
        candidate.load(program, origin)?;
        let result = candidate.run(max_steps)?;
        *self = candidate;
        Ok(result)
    }

    fn result(&self, traces: Vec<StepTrace>) -> ExecutionResult {
        ExecutionResult {
            halted: self.state.halted,
            steps: traces.len(),
            final_state: self.state.clone(),
            traces,
        }
    }

    fn step_inner(&mut self) -> Result<StepTrace, EdsacError> {
        if self.state.halted {
            return Err(EdsacError::Halted);
        }
        let pc_before = self.state.scr;
        let end = self
            .state
            .loaded_origin
            .saturating_add(self.state.loaded_words);
        if pc_before < self.state.loaded_origin || pc_before >= end {
            return Err(EdsacError::FetchOutsideProgram {
                scr: pc_before,
                origin: self.state.loaded_origin,
                words: self.state.loaded_words,
            });
        }

        let instruction = self.state.memory[pc_before] & ODD_CELL_MASK;
        let code = ((instruction >> 12) & 0x1f) as u8;
        let opcode = Opcode::decode(code).ok_or(EdsacError::UnknownOpcode {
            opcode: code,
            instruction,
        })?;
        let length = if instruction & 1 == 0 {
            Length::Short
        } else {
            Length::Long
        };
        if length == Length::Long && opcode.requires_short() {
            return Err(EdsacError::InvalidLength {
                opcode,
                instruction,
            });
        }
        let address = ((instruction >> 1) & 0x03ff) as usize;
        let operand = self.load_operand(address, length);

        self.state.order_tank = instruction;
        self.state.multiplicand = operand;
        self.state.multiplicand_length = length;
        self.state.scr = if pc_before + 1 < MEMORY_CELLS {
            pc_before + 1
        } else {
            0
        };

        let mut description = format!("{} {address}{}", opcode.name(), length_suffix(length));
        let mut io_code = None;
        let mut control_transferred = false;
        match opcode {
            Opcode::A => self.add_operand(),
            Opcode::S => {
                self.negate_operand();
                self.add_operand();
            }
            Opcode::H => self.state.multiplier = self.state.multiplicand,
            Opcode::V => self.multiply_add(),
            Opcode::N => {
                self.negate_operand();
                self.multiply_add();
            }
            Opcode::T | Opcode::U => {
                self.store_accumulator(address, length);
                if opcode == Opcode::T {
                    self.state.accumulator = [0; 5];
                }
            }
            Opcode::C => self.collate(),
            Opcode::R => {
                let count = instruction.trailing_zeros() as usize + 1;
                self.shift_right(count);
                description = format!("R {count}");
            }
            Opcode::L => {
                let count = instruction.trailing_zeros() as usize + 1;
                self.shift_left(count);
                description = format!("L {count}");
            }
            Opcode::E => {
                if !self.accumulator_negative() {
                    self.state.scr = address;
                    control_transferred = true;
                }
            }
            Opcode::G => {
                if self.accumulator_negative() {
                    self.state.scr = address;
                    control_transferred = true;
                }
            }
            Opcode::I => {
                let code = *self
                    .state
                    .input
                    .get(self.state.input_cursor)
                    .ok_or(EdsacError::InputExhausted)?;
                self.state.input_cursor += 1;
                self.state.memory[address] = (self.state.memory[address] & !0x1f) | code as u32;
                io_code = Some(code);
            }
            Opcode::O => {
                let code = ((self.state.memory[address] >> 12) & 0x1f) as u8;
                self.state.output.push(code);
                self.state.last_output = code;
                io_code = Some(code);
            }
            Opcode::F => {
                self.state.memory[address] =
                    (self.state.memory[address] & 0x0fff) | ((self.state.last_output as u32) << 12);
                io_code = Some(self.state.last_output);
            }
            Opcode::X => {}
            Opcode::Y => self.round(),
            Opcode::Z => self.state.halted = true,
        }

        if pc_before == MEMORY_CELLS - 1 && !self.state.halted && !control_transferred {
            return Err(EdsacError::SequenceControlOverflow);
        }

        Ok(StepTrace {
            pc_before,
            pc_after: self.state.scr,
            instruction,
            opcode,
            address,
            length,
            mnemonic: opcode.name().to_owned(),
            description,
            io_code,
        })
    }

    fn load_operand(&self, address: usize, length: Length) -> [u16; 3] {
        match length {
            Length::Short => {
                // Even locations physically carry the extra sandwich digit
                // used by a long word's junior half. It is not part of a
                // short operand and must not leak into the 15-bit tank slice.
                let value = self.state.memory[address] & ODD_CELL_MASK;
                [(value >> 2) as u16, ((value << 13) & 0x6000) as u16, 0]
            }
            Length::Long => {
                let even = address & !1;
                let senior = self.state.memory[even | 1];
                let junior = self.state.memory[even];
                [
                    (senior >> 2) as u16,
                    ((senior << 13 | junior >> 5) & 0x7fff) as u16,
                    ((junior << 10) & 0x7c00) as u16,
                ]
            }
        }
    }

    fn store_accumulator(&mut self, address: usize, length: Length) {
        let [a0, a1, a2, ..] = self.state.accumulator;
        match length {
            Length::Short => {
                self.state.memory[address] = ((a0 as u32) << 2) | ((a1 as u32) >> 13);
            }
            Length::Long => {
                let even = address & !1;
                self.state.memory[even | 1] = ((a0 as u32) << 2) | ((a1 as u32) >> 13);
                self.state.memory[even] =
                    (((a1 as u32) << 5) | ((a2 as u32) >> 10)) & EVEN_CELL_MASK;
            }
        }
    }

    fn add_operand(&mut self) {
        let [r0, r1, r2] = self.state.multiplicand;
        let a = &mut self.state.accumulator;
        let t2 = a[2] as u32 + r2 as u32;
        let t1 = a[1] as u32 + r1 as u32 + (t2 >> 15);
        let t0 = a[0] as u32 + r0 as u32 + (t1 >> 15);
        a[2] = (t2 as u16) & SLICE_MASK;
        a[1] = (t1 as u16) & SLICE_MASK;
        a[0] = (t0 as u16) & SLICE_MASK;
    }

    fn round(&mut self) {
        let a = &mut self.state.accumulator;
        let t2 = a[2] as u32 + 0x0200;
        let t1 = a[1] as u32 + (t2 >> 15);
        let t0 = a[0] as u32 + (t1 >> 15);
        a[2] = (t2 as u16) & SLICE_MASK;
        a[1] = (t1 as u16) & SLICE_MASK;
        a[0] = (t0 as u16) & SLICE_MASK;
    }

    fn collate(&mut self) {
        let h = self.state.multiplier;
        let r = self.state.multiplicand;
        let a = &mut self.state.accumulator;
        let t2 = a[2] as u32 + (h[2] & r[2]) as u32;
        let t1 = a[1] as u32 + (h[1] & r[1]) as u32 + (t2 >> 15);
        let t0 = a[0] as u32 + (h[0] & r[0]) as u32 + (t1 >> 15);
        a[2] = (t2 as u16) & SLICE_MASK;
        a[1] = (t1 as u16) & SLICE_MASK;
        a[0] = (t0 as u16) & SLICE_MASK;
    }

    fn negate_operand(&mut self) {
        let r = &mut self.state.multiplicand;
        if r[2] != 0 {
            r[2] = r[2].wrapping_neg() & SLICE_MASK;
            r[1] = !r[1] & SLICE_MASK;
            r[0] = !r[0] & SLICE_MASK;
        } else if r[1] != 0 {
            r[1] = r[1].wrapping_neg() & SLICE_MASK;
            r[0] = !r[0] & SLICE_MASK;
        } else {
            r[0] = r[0].wrapping_neg() & SLICE_MASK;
        }
    }

    fn multiply_add(&mut self) {
        let saved = self.state.accumulator;
        let mut negative = false;
        if self.state.multiplicand[0] & 0x4000 != 0 {
            self.negate_operand();
            negative = true;
        }
        let b = self.state.multiplicand;
        self.state.multiplicand = self.state.multiplier;
        if self.state.multiplicand[0] & 0x4000 != 0 {
            self.negate_operand();
            negative = !negative;
        }
        let r = self.state.multiplicand;

        let mut product = [0_u64; 5];
        let mut x = b[2] as u64 * r[2] as u64;
        product[4] = x >> 14;
        x = b[1] as u64 * r[2] as u64 + b[2] as u64 * r[1] as u64;
        product[4] += (x << 1) & 0x7ffc;
        product[3] = (x >> 14) + (product[4] >> 15);
        x = b[0] as u64 * r[2] as u64 + b[1] as u64 * r[1] as u64 + b[2] as u64 * r[0] as u64;
        product[3] += (x << 1) & 0x7ffe;
        product[2] = (x >> 14) + (product[3] >> 15);
        x = b[0] as u64 * r[1] as u64 + b[1] as u64 * r[0] as u64;
        product[2] += (x << 1) & 0x7ffe;
        product[1] = (x >> 14) + (product[2] >> 15);
        x = b[0] as u64 * r[0] as u64;
        product[1] += (x << 1) & 0x7fff;
        product[0] = (x >> 14) + (product[1] >> 15);
        for value in &mut product {
            *value &= 0x7fff;
        }
        self.state.accumulator = product.map(|value| value as u16);

        if negative {
            self.negate_accumulator();
        }
        self.add_full_accumulator(saved);
    }

    fn negate_accumulator(&mut self) {
        let a = &mut self.state.accumulator;
        if a[4] != 0 {
            a[4] = a[4].wrapping_neg() & SLICE_MASK;
            for value in &mut a[..4] {
                *value = !*value & SLICE_MASK;
            }
        } else if a[3] != 0 {
            a[3] = a[3].wrapping_neg() & SLICE_MASK;
            for value in &mut a[..3] {
                *value = !*value & SLICE_MASK;
            }
        } else if a[2] != 0 {
            a[2] = a[2].wrapping_neg() & SLICE_MASK;
            for value in &mut a[..2] {
                *value = !*value & SLICE_MASK;
            }
        } else if a[1] != 0 {
            a[1] = a[1].wrapping_neg() & SLICE_MASK;
            a[0] = !a[0] & SLICE_MASK;
        } else {
            a[0] = a[0].wrapping_neg() & SLICE_MASK;
        }
    }

    fn add_full_accumulator(&mut self, addend: [u16; 5]) {
        let a = &mut self.state.accumulator;
        let t4 = a[4] as u32 + addend[4] as u32;
        let t3 = a[3] as u32 + addend[3] as u32 + (t4 >> 15);
        let t2 = a[2] as u32 + addend[2] as u32 + (t3 >> 15);
        let t1 = a[1] as u32 + addend[1] as u32 + (t2 >> 15);
        let t0 = a[0] as u32 + addend[0] as u32 + (t1 >> 15);
        a[4] = (t4 as u16) & SLICE_MASK;
        a[3] = (t3 as u16) & SLICE_MASK;
        a[2] = (t2 as u16) & SLICE_MASK;
        a[1] = (t1 as u16) & SLICE_MASK;
        a[0] = (t0 as u16) & SLICE_MASK;
    }

    fn shift_right(&mut self, count: usize) {
        for _ in 0..count {
            let old = self.state.accumulator;
            self.state.accumulator = [
                (old[0] >> 1) | (old[0] & 0x4000),
                (old[1] >> 1) | ((old[0] & 1) << 14),
                (old[2] >> 1) | ((old[1] & 1) << 14),
                (old[3] >> 1) | ((old[2] & 1) << 14),
                (old[4] >> 1) | ((old[3] & 1) << 14),
            ];
        }
        self.state.accumulator[4] &= 0x7ff0;
    }

    fn shift_left(&mut self, count: usize) {
        for _ in 0..count {
            let old = self.state.accumulator;
            self.state.accumulator = [
                ((old[0] << 1) | (old[1] >> 14)) & SLICE_MASK,
                ((old[1] << 1) | (old[2] >> 14)) & SLICE_MASK,
                ((old[2] << 1) | (old[3] >> 14)) & SLICE_MASK,
                ((old[3] << 1) | (old[4] >> 14)) & SLICE_MASK,
                (old[4] << 1) & 0x7ff0,
            ];
        }
    }

    fn accumulator_negative(&self) -> bool {
        self.state.accumulator[0] & 0x4000 != 0
    }
}

fn length_suffix(length: Length) -> char {
    match length {
        Length::Short => 'F',
        Length::Long => 'D',
    }
}

fn validate_program_extent(words: usize, origin: usize) -> Result<(), EdsacError> {
    if origin >= MEMORY_CELLS {
        return Err(EdsacError::InvalidOrigin { origin });
    }
    let capacity = MEMORY_CELLS - origin;
    if words > capacity {
        return Err(EdsacError::ProgramTooLarge { words, capacity });
    }
    Ok(())
}

fn validate_memory_word(address: usize, word: u32) -> Result<(), EdsacError> {
    if address >= MEMORY_CELLS {
        return Err(EdsacError::InvalidAddress { address });
    }
    let maximum = if address.is_multiple_of(2) {
        EVEN_CELL_MASK
    } else {
        ODD_CELL_MASK
    };
    if word > maximum {
        return Err(EdsacError::InvalidMemoryWord {
            address,
            word,
            maximum,
        });
    }
    Ok(())
}

fn validate_long_address(address: usize) -> Result<(), EdsacError> {
    if address >= MEMORY_CELLS - 1 {
        return Err(EdsacError::InvalidAddress { address });
    }
    if !address.is_multiple_of(2) {
        return Err(EdsacError::MisalignedLongAddress { address });
    }
    Ok(())
}

fn validate_slices(register: &'static str, values: &[u16]) -> Result<(), EdsacError> {
    for (index, &value) in values.iter().enumerate() {
        if value > SLICE_MASK {
            return Err(EdsacError::InvalidRegisterSlice {
                register,
                index,
                value,
            });
        }
    }
    Ok(())
}

fn validate_state(state: &EdsacState) -> Result<(), EdsacError> {
    for (address, &word) in state.memory.iter().enumerate() {
        validate_memory_word(address, word)?;
    }
    if state.scr >= MEMORY_CELLS {
        return Err(EdsacError::InvalidSequenceControl { scr: state.scr });
    }
    if state.order_tank > ODD_CELL_MASK {
        return Err(EdsacError::InvalidOrderTank {
            order: state.order_tank,
        });
    }
    validate_accumulator(&state.accumulator)?;
    validate_operand("multiplier", &state.multiplier)?;
    validate_operand("multiplicand", &state.multiplicand)?;
    for (index, &code) in state.input.iter().enumerate() {
        if code > 0x1f {
            return Err(EdsacError::InvalidInputCode { index, code });
        }
    }
    if state.input_cursor > state.input.len() {
        return Err(EdsacError::InvalidInputCursor {
            cursor: state.input_cursor,
            length: state.input.len(),
        });
    }
    for (index, &code) in state.output.iter().enumerate() {
        if code > 0x1f {
            return Err(EdsacError::InvalidOutputCode { index, code });
        }
    }
    if state.last_output > 0x1f {
        return Err(EdsacError::InvalidLastOutput {
            code: state.last_output,
        });
    }
    if state.loaded_origin >= MEMORY_CELLS
        || state.loaded_words > MEMORY_CELLS - state.loaded_origin
    {
        return Err(EdsacError::InvalidLoadedRange {
            origin: state.loaded_origin,
            words: state.loaded_words,
        });
    }
    Ok(())
}

fn validate_accumulator(accumulator: &[u16; 5]) -> Result<(), EdsacError> {
    validate_slices("accumulator", accumulator)?;
    if accumulator[4] & 0x000f != 0 {
        return Err(EdsacError::InvalidRegisterPadding {
            register: "accumulator",
            value: accumulator[4],
            required_zero_mask: 0x000f,
        });
    }
    Ok(())
}

fn validate_operand(register: &'static str, operand: &[u16; 3]) -> Result<(), EdsacError> {
    validate_slices(register, operand)?;
    if operand[2] & 0x03ff != 0 {
        return Err(EdsacError::InvalidRegisterPadding {
            register,
            value: operand[2],
            required_zero_mask: 0x03ff,
        });
    }
    Ok(())
}
