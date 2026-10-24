//! A source-backed functional model of the IAS stored-program computer.
//!
//! The machine's most important idea is visible in its memory layout: a
//! 40-bit word may be data, or it may be two 20-bit orders.  The processor
//! fetches both orders together, executes the left one, and retains the right
//! one in control storage.  Consequently a left order that rewrites its own
//! right half does not change the order which has already been fetched.  This
//! simulator models that boundary explicitly instead of treating the two
//! halves as unrelated array elements.

use std::error::Error;
use std::fmt;

/// The 12-bit address space proposed by the IAS logical-design report.
pub const MEMORY_WORDS: usize = 1 << 12;
/// The largest architectural 40-bit word.
pub const WORD_MASK: u64 = (1_u64 << 40) - 1;
const SIGN_BIT: u64 = 1_u64 << 39;
const INSTRUCTION_MASK: u32 = (1_u32 << 20) - 1;
/// A bounded trace keeps complete-state tracing from becoming an allocation
/// primitive when a caller supplies an untrusted step count.
pub const MAX_STEPS: usize = 4_096;
/// The stop order anticipated by section 6.8.5 of the IAS report.
pub const HALT: u32 = 0;

/// Which half of a 40-bit memory word supplied an order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Half {
    Left,
    Right,
}

/// The 21 numbered IAS operations plus the stop order anticipated by the
/// report's control discussion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Opcode {
    Halt = 0,
    Load = 1,
    LoadNegative = 2,
    LoadAbsolute = 3,
    LoadNegativeAbsolute = 4,
    Add = 5,
    Subtract = 6,
    AddAbsolute = 7,
    SubtractAbsolute = 8,
    LoadArithmetic = 9,
    LoadArithmeticIntoAccumulator = 10,
    Multiply = 11,
    Divide = 12,
    JumpLeft = 13,
    JumpRight = 14,
    JumpNonNegativeLeft = 15,
    JumpNonNegativeRight = 16,
    Store = 17,
    StoreLeftAddress = 18,
    StoreRightAddress = 19,
    ShiftLeft = 20,
    ShiftRight = 21,
}

impl Opcode {
    fn decode(value: u8) -> Result<Self, IasError> {
        let opcode = match value {
            0 => Self::Halt,
            1 => Self::Load,
            2 => Self::LoadNegative,
            3 => Self::LoadAbsolute,
            4 => Self::LoadNegativeAbsolute,
            5 => Self::Add,
            6 => Self::Subtract,
            7 => Self::AddAbsolute,
            8 => Self::SubtractAbsolute,
            9 => Self::LoadArithmetic,
            10 => Self::LoadArithmeticIntoAccumulator,
            11 => Self::Multiply,
            12 => Self::Divide,
            13 => Self::JumpLeft,
            14 => Self::JumpRight,
            15 => Self::JumpNonNegativeLeft,
            16 => Self::JumpNonNegativeRight,
            17 => Self::Store,
            18 => Self::StoreLeftAddress,
            19 => Self::StoreRightAddress,
            20 => Self::ShiftLeft,
            21 => Self::ShiftRight,
            _ => return Err(IasError::InvalidOpcode { opcode: value }),
        };
        Ok(opcode)
    }

    /// Conventional descriptive spelling used in traces.
    pub const fn mnemonic(self) -> &'static str {
        match self {
            Self::Halt => "HALT",
            Self::Load => "LOAD M(X)",
            Self::LoadNegative => "LOAD -M(X)",
            Self::LoadAbsolute => "LOAD |M(X)|",
            Self::LoadNegativeAbsolute => "LOAD -|M(X)|",
            Self::Add => "ADD M(X)",
            Self::Subtract => "SUB M(X)",
            Self::AddAbsolute => "ADD |M(X)|",
            Self::SubtractAbsolute => "SUB |M(X)|",
            Self::LoadArithmetic => "LOAD MQ,M(X)",
            Self::LoadArithmeticIntoAccumulator => "LOAD MQ",
            Self::Multiply => "MUL M(X)",
            Self::Divide => "DIV M(X)",
            Self::JumpLeft => "JUMP M(X,0:19)",
            Self::JumpRight => "JUMP M(X,20:39)",
            Self::JumpNonNegativeLeft => "JUMP+ M(X,0:19)",
            Self::JumpNonNegativeRight => "JUMP+ M(X,20:39)",
            Self::Store => "STOR M(X)",
            Self::StoreLeftAddress => "STOR M(X,8:19)",
            Self::StoreRightAddress => "STOR M(X,28:39)",
            Self::ShiftLeft => "LSH",
            Self::ShiftRight => "RSH",
        }
    }
}

/// Typed failures are part of the simulator contract.  None of these leave a
/// partially fetched or partially executed state behind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IasError {
    TransportAlignment { bytes: usize },
    EmptyProgram,
    OriginOutOfRange { origin: u16 },
    ProgramOutOfRange { origin: u16, words: usize },
    MemoryOutOfRange { address: usize },
    InvalidAddress { address: u16 },
    InvalidWord { value: u64 },
    InvalidInstruction { value: u32 },
    InvalidOpcode { opcode: u8 },
    DivideByZero,
    QuotientOverflow,
    FetchOutsideProgram { word: u16 },
    Halted,
    StepLimitTooLarge { limit: usize, maximum: usize },
    StepLimitExceeded { limit: usize },
    InvalidStateWord { address: usize },
    InvalidAccumulator,
    InvalidArithmeticRegister,
    InvalidControlCounter,
    InvalidBufferedInstruction,
    InvalidControlRegister,
    InvalidFunctionTable,
    InvalidMemoryAddress,
    InvalidSelectronRegister,
    InvalidLoadedRange,
}

impl fmt::Display for IasError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TransportAlignment { bytes } => {
                write!(
                    formatter,
                    "IAS transport length {bytes} is not a multiple of five"
                )
            }
            Self::EmptyProgram => write!(formatter, "IAS program image is empty"),
            Self::OriginOutOfRange { origin } => {
                write!(formatter, "IAS origin {origin} is outside 12-bit memory")
            }
            Self::ProgramOutOfRange { origin, words } => write!(
                formatter,
                "IAS image of {words} words at origin {origin} exceeds memory"
            ),
            Self::MemoryOutOfRange { address } => {
                write!(formatter, "IAS memory address {address} is out of range")
            }
            Self::InvalidAddress { address } => {
                write!(
                    formatter,
                    "IAS encoded address {address} is wider than 12 bits"
                )
            }
            Self::InvalidWord { value } => {
                write!(formatter, "IAS word {value:#x} is wider than 40 bits")
            }
            Self::InvalidInstruction { value } => {
                write!(
                    formatter,
                    "IAS instruction {value:#x} is wider than 20 bits"
                )
            }
            Self::InvalidOpcode { opcode } => write!(formatter, "unknown IAS opcode {opcode:#04x}"),
            Self::DivideByZero => write!(formatter, "IAS division by zero"),
            Self::QuotientOverflow => write!(formatter, "IAS quotient does not fit 40 bits"),
            Self::FetchOutsideProgram { word } => {
                write!(
                    formatter,
                    "IAS fetch at word {word} is outside the installed image"
                )
            }
            Self::Halted => write!(formatter, "IAS processor is halted"),
            Self::StepLimitTooLarge { limit, maximum } => write!(
                formatter,
                "IAS step limit {limit} exceeds the safe maximum {maximum}"
            ),
            Self::StepLimitExceeded { limit } => {
                write!(formatter, "IAS execution did not halt within {limit} steps")
            }
            Self::InvalidStateWord { address } => {
                write!(
                    formatter,
                    "IAS snapshot word {address} is wider than 40 bits"
                )
            }
            Self::InvalidAccumulator => write!(formatter, "IAS snapshot accumulator is invalid"),
            Self::InvalidArithmeticRegister => {
                write!(formatter, "IAS snapshot arithmetic register is invalid")
            }
            Self::InvalidControlCounter => {
                write!(formatter, "IAS snapshot control counter is invalid")
            }
            Self::InvalidBufferedInstruction => {
                write!(formatter, "IAS snapshot buffered instruction is invalid")
            }
            Self::InvalidControlRegister => {
                write!(formatter, "IAS snapshot control register is invalid")
            }
            Self::InvalidFunctionTable => {
                write!(formatter, "IAS snapshot function table is invalid")
            }
            Self::InvalidMemoryAddress => {
                write!(formatter, "IAS snapshot memory address is invalid")
            }
            Self::InvalidSelectronRegister => {
                write!(formatter, "IAS snapshot Selectron register is invalid")
            }
            Self::InvalidLoadedRange => write!(formatter, "IAS snapshot loaded range is invalid"),
        }
    }
}

impl Error for IasError {}

/// A deep, complete machine snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IasState {
    pub memory: Box<[u64; MEMORY_WORDS]>,
    pub accumulator: u64,
    pub arithmetic_register: u64,
    pub control_counter: u16,
    pub buffered_right: Option<u32>,
    pub current_instruction: u32,
    pub function_table: u8,
    pub memory_address: u16,
    pub selectron_register: u64,
    pub halted: bool,
    pub loaded_origin: u16,
    pub loaded_words: usize,
}

impl Default for IasState {
    fn default() -> Self {
        Self {
            memory: Box::new([0; MEMORY_WORDS]),
            accumulator: 0,
            arithmetic_register: 0,
            control_counter: 0,
            buffered_right: None,
            current_instruction: 0,
            function_table: 0,
            memory_address: 0,
            selectron_register: 0,
            halted: false,
            loaded_origin: 0,
            loaded_words: 0,
        }
    }
}

/// One retired order plus its complete post-state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub word_address: u16,
    pub half: Half,
    pub raw: u32,
    pub opcode: Opcode,
    pub address: u16,
    pub mnemonic: &'static str,
    pub accumulator_before: u64,
    pub accumulator_after: u64,
    pub arithmetic_before: u64,
    pub arithmetic_after: u64,
    pub state_after: IasState,
}

/// A bounded execution result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResult {
    pub traces: Vec<StepTrace>,
    pub final_state: IasState,
    pub halted: bool,
}

/// Encode one 8-bit operation and 12-bit address.
pub fn encode_instruction(opcode: Opcode, address: u16) -> Result<u32, IasError> {
    if address as usize >= MEMORY_WORDS {
        return Err(IasError::InvalidAddress { address });
    }
    Ok(((opcode as u32) << 12) | u32::from(address))
}

/// Place a left and right 20-bit order in one 40-bit word.
pub fn pack_instructions(left: u32, right: u32) -> Result<u64, IasError> {
    validate_instruction(left)?;
    validate_instruction(right)?;
    Ok((u64::from(left) << 20) | u64::from(right))
}

fn validate_instruction(value: u32) -> Result<(), IasError> {
    if value > INSTRUCTION_MASK {
        Err(IasError::InvalidInstruction { value })
    } else {
        Ok(())
    }
}

fn validate_word(value: u64) -> Result<(), IasError> {
    if value > WORD_MASK {
        Err(IasError::InvalidWord { value })
    } else {
        Ok(())
    }
}

/// Deterministic IAS processor and electronic memory.
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct IasSimulator {
    state: IasState,
}

impl IasSimulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> &IasState {
        &self.state
    }

    pub fn snapshot(&self) -> IasState {
        self.state.clone()
    }

    /// Reset includes memory.  No previous program or lifecycle range survives.
    pub fn reset(&mut self) {
        self.state = IasState::default();
    }

    pub fn restore(&mut self, state: &IasState) -> Result<(), IasError> {
        validate_state(state)?;
        self.state = state.clone();
        Ok(())
    }

    /// Load five-byte big-endian words after complete preflight.
    pub fn load(&mut self, bytes: &[u8], origin: u16) -> Result<usize, IasError> {
        if bytes.is_empty() {
            return Err(IasError::EmptyProgram);
        }
        if !bytes.len().is_multiple_of(5) {
            return Err(IasError::TransportAlignment { bytes: bytes.len() });
        }
        let words = bytes.len() / 5;
        validate_load_range(origin, words)?;

        let mut decoded = Vec::with_capacity(words);
        for chunk in bytes.as_chunks::<5>().0 {
            let value = (u64::from(chunk[0]) << 32)
                | (u64::from(chunk[1]) << 24)
                | (u64::from(chunk[2]) << 16)
                | (u64::from(chunk[3]) << 8)
                | u64::from(chunk[4]);
            decoded.push(value);
        }
        self.load_words(&decoded, origin)
    }

    /// Install checked words into a freshly reset machine.
    pub fn load_words(&mut self, words: &[u64], origin: u16) -> Result<usize, IasError> {
        if words.is_empty() {
            return Err(IasError::EmptyProgram);
        }
        validate_load_range(origin, words.len())?;
        for &word in words {
            validate_word(word)?;
        }

        let mut next = IasState {
            control_counter: origin,
            loaded_origin: origin,
            loaded_words: words.len(),
            ..IasState::default()
        };
        let start = usize::from(origin);
        next.memory[start..start + words.len()].copy_from_slice(words);
        self.state = next;
        Ok(words.len())
    }

    pub fn read_memory(&self, address: usize) -> Result<u64, IasError> {
        self.state
            .memory
            .get(address)
            .copied()
            .ok_or(IasError::MemoryOutOfRange { address })
    }

    pub fn write_memory(&mut self, address: usize, value: u64) -> Result<(), IasError> {
        validate_word(value)?;
        let cell = self
            .state
            .memory
            .get_mut(address)
            .ok_or(IasError::MemoryOutOfRange { address })?;
        *cell = value;
        Ok(())
    }

    pub fn set_accumulator(&mut self, value: u64) -> Result<(), IasError> {
        validate_word(value)?;
        self.state.accumulator = value;
        Ok(())
    }

    pub fn set_arithmetic_register(&mut self, value: u64) -> Result<(), IasError> {
        validate_word(value)?;
        self.state.arithmetic_register = value;
        Ok(())
    }

    /// Retire one order atomically.
    pub fn step(&mut self) -> Result<StepTrace, IasError> {
        if self.state.halted {
            return Err(IasError::Halted);
        }
        let checkpoint = self.state.clone();
        match self.step_inner() {
            Ok(trace) => Ok(trace),
            Err(error) => {
                self.state = checkpoint;
                Err(error)
            }
        }
    }

    fn step_inner(&mut self) -> Result<StepTrace, IasError> {
        let (word_address, half, raw) = if let Some(right) = self.state.buffered_right.take() {
            let source = self
                .state
                .control_counter
                .checked_sub(1)
                .ok_or(IasError::InvalidControlCounter)?;
            (source, Half::Right, right)
        } else {
            let word_address = self.state.control_counter;
            self.ensure_fetch(word_address)?;
            let fetched = self.state.memory[usize::from(word_address)];
            self.state.memory_address = word_address;
            self.state.selectron_register = fetched;
            self.state.buffered_right = Some((fetched & u64::from(INSTRUCTION_MASK)) as u32);
            self.state.control_counter = word_address + 1;
            (word_address, Half::Left, (fetched >> 20) as u32)
        };

        validate_instruction(raw)?;
        let opcode_number = (raw >> 12) as u8;
        let address = (raw & 0x0fff) as u16;
        self.state.current_instruction = raw;
        self.state.function_table = opcode_number;
        self.state.memory_address = address;
        let accumulator_before = self.state.accumulator;
        let arithmetic_before = self.state.arithmetic_register;

        if opcode_number == Opcode::Halt as u8 {
            self.state.halted = true;
            self.state.buffered_right = None;
            validate_state(&self.state)?;
            return Ok(StepTrace {
                word_address,
                half,
                raw,
                opcode: Opcode::Halt,
                address,
                mnemonic: "HALT",
                accumulator_before,
                accumulator_after: self.state.accumulator,
                arithmetic_before,
                arithmetic_after: self.state.arithmetic_register,
                state_after: self.state.clone(),
            });
        }

        let opcode = Opcode::decode(opcode_number)?;
        self.execute_order(opcode, address)?;
        validate_state(&self.state)?;
        Ok(StepTrace {
            word_address,
            half,
            raw,
            opcode,
            address,
            mnemonic: opcode.mnemonic(),
            accumulator_before,
            accumulator_after: self.state.accumulator,
            arithmetic_before,
            arithmetic_after: self.state.arithmetic_register,
            state_after: self.state.clone(),
        })
    }

    fn execute_order(&mut self, opcode: Opcode, address: u16) -> Result<(), IasError> {
        match opcode {
            Opcode::Halt => {
                self.state.halted = true;
                self.state.buffered_right = None;
            }
            Opcode::Load => self.state.accumulator = self.read_operand(address),
            Opcode::LoadNegative => self.state.accumulator = negate(self.read_operand(address)),
            Opcode::LoadAbsolute => self.state.accumulator = absolute(self.read_operand(address)),
            Opcode::LoadNegativeAbsolute => {
                self.state.accumulator = negate(absolute(self.read_operand(address)));
            }
            Opcode::Add => {
                let operand = self.read_operand(address);
                self.add(operand);
            }
            Opcode::Subtract => {
                let operand = self.read_operand(address);
                self.add(negate(operand));
            }
            Opcode::AddAbsolute => {
                let operand = self.read_operand(address);
                self.add(absolute(operand));
            }
            Opcode::SubtractAbsolute => {
                let operand = self.read_operand(address);
                self.add(negate(absolute(operand)));
            }
            Opcode::LoadArithmetic => self.state.arithmetic_register = self.read_operand(address),
            Opcode::LoadArithmeticIntoAccumulator => {
                self.state.accumulator = self.state.arithmetic_register;
            }
            Opcode::Multiply => {
                let operand = self.read_operand(address);
                self.multiply(operand);
            }
            Opcode::Divide => {
                let operand = self.read_operand(address);
                self.divide(operand)?;
            }
            Opcode::JumpLeft => self.jump_left(address)?,
            Opcode::JumpRight => self.jump_right(address)?,
            Opcode::JumpNonNegativeLeft => {
                if !is_negative(self.state.accumulator) {
                    self.jump_left(address)?;
                }
            }
            Opcode::JumpNonNegativeRight => {
                if !is_negative(self.state.accumulator) {
                    self.jump_right(address)?;
                }
            }
            Opcode::Store => {
                self.state.memory[usize::from(address)] = self.state.accumulator;
                self.state.selectron_register = self.state.accumulator;
            }
            Opcode::StoreLeftAddress => self.store_address(address, Half::Left),
            Opcode::StoreRightAddress => self.store_address(address, Half::Right),
            Opcode::ShiftLeft => self.state.accumulator = (self.state.accumulator << 1) & WORD_MASK,
            Opcode::ShiftRight => {
                self.state.accumulator = from_signed(to_signed(self.state.accumulator) >> 1);
            }
        }
        Ok(())
    }

    fn read_operand(&mut self, address: u16) -> u64 {
        let value = self.state.memory[usize::from(address)];
        self.state.selectron_register = value;
        value
    }

    fn add(&mut self, value: u64) {
        self.state.accumulator = self.state.accumulator.wrapping_add(value) & WORD_MASK;
    }

    fn multiply(&mut self, operand: u64) {
        let left = i128::from(to_signed(self.state.arithmetic_register));
        let right = i128::from(to_signed(operand));
        let product = left * right;
        let product_mask = (1_i128 << 80) - 1;
        let encoded = (product & product_mask) as u128;
        self.state.accumulator = ((encoded >> 40) as u64) & WORD_MASK;
        self.state.arithmetic_register = (encoded as u64) & WORD_MASK;
    }

    fn divide(&mut self, operand: u64) -> Result<(), IasError> {
        let divisor = i128::from(to_signed(operand));
        if divisor == 0 {
            return Err(IasError::DivideByZero);
        }
        let dividend = i128::from(to_signed(self.state.accumulator));
        let quotient = dividend / divisor;
        let remainder = dividend % divisor;
        let minimum = -(1_i128 << 39);
        let maximum = (1_i128 << 39) - 1;
        if !(minimum..=maximum).contains(&quotient) {
            return Err(IasError::QuotientOverflow);
        }
        self.state.arithmetic_register = from_signed(quotient as i64);
        self.state.accumulator = from_signed(remainder as i64);
        Ok(())
    }

    fn jump_left(&mut self, address: u16) -> Result<(), IasError> {
        self.ensure_fetch(address)?;
        self.state.control_counter = address;
        self.state.buffered_right = None;
        Ok(())
    }

    fn jump_right(&mut self, address: u16) -> Result<(), IasError> {
        self.ensure_fetch(address)?;
        let fetched = self.state.memory[usize::from(address)];
        self.state.selectron_register = fetched;
        self.state.control_counter = address + 1;
        self.state.buffered_right = Some((fetched & u64::from(INSTRUCTION_MASK)) as u32);
        Ok(())
    }

    fn store_address(&mut self, address: u16, half: Half) {
        let old = self.state.memory[usize::from(address)];
        let replacement = self.state.accumulator & 0x0fff;
        let changed = match half {
            Half::Left => (old & !(0x0fff_u64 << 20)) | (replacement << 20),
            Half::Right => (old & !0x0fff_u64) | replacement,
        };
        self.state.memory[usize::from(address)] = changed;
        self.state.selectron_register = changed;
    }

    fn ensure_fetch(&self, word: u16) -> Result<(), IasError> {
        let start = usize::from(self.state.loaded_origin);
        let end = start + self.state.loaded_words;
        if self.state.loaded_words == 0 || !(start..end).contains(&usize::from(word)) {
            Err(IasError::FetchOutsideProgram { word })
        } else {
            Ok(())
        }
    }

    /// Run until stop, rolling the complete run back on any failure.
    pub fn run(&mut self, limit: usize) -> Result<RunResult, IasError> {
        validate_step_limit(limit)?;
        if self.state.halted {
            return Ok(RunResult {
                traces: Vec::new(),
                final_state: self.state.clone(),
                halted: true,
            });
        }

        let checkpoint = self.state.clone();
        let mut traces = Vec::with_capacity(limit.min(64));
        for _ in 0..limit {
            match self.step() {
                Ok(trace) => {
                    let halted = trace.state_after.halted;
                    traces.push(trace);
                    if halted {
                        return Ok(RunResult {
                            traces,
                            final_state: self.state.clone(),
                            halted: true,
                        });
                    }
                }
                Err(error) => {
                    self.state = checkpoint;
                    return Err(error);
                }
            }
        }
        self.state = checkpoint;
        Err(IasError::StepLimitExceeded { limit })
    }

    pub fn execute(
        &mut self,
        bytes: &[u8],
        origin: u16,
        limit: usize,
    ) -> Result<RunResult, IasError> {
        validate_step_limit(limit)?;
        let checkpoint = self.state.clone();
        if let Err(error) = self.load(bytes, origin) {
            self.state = checkpoint;
            return Err(error);
        }
        match self.run(limit) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = checkpoint;
                Err(error)
            }
        }
    }

    pub fn execute_words(
        &mut self,
        words: &[u64],
        origin: u16,
        limit: usize,
    ) -> Result<RunResult, IasError> {
        validate_step_limit(limit)?;
        let checkpoint = self.state.clone();
        if let Err(error) = self.load_words(words, origin) {
            self.state = checkpoint;
            return Err(error);
        }
        match self.run(limit) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = checkpoint;
                Err(error)
            }
        }
    }
}

fn validate_load_range(origin: u16, words: usize) -> Result<(), IasError> {
    let start = usize::from(origin);
    if start >= MEMORY_WORDS {
        return Err(IasError::OriginOutOfRange { origin });
    }
    if words > MEMORY_WORDS - start {
        return Err(IasError::ProgramOutOfRange { origin, words });
    }
    Ok(())
}

fn validate_step_limit(limit: usize) -> Result<(), IasError> {
    if limit > MAX_STEPS {
        Err(IasError::StepLimitTooLarge {
            limit,
            maximum: MAX_STEPS,
        })
    } else {
        Ok(())
    }
}

fn validate_state(state: &IasState) -> Result<(), IasError> {
    if let Some(address) = state.memory.iter().position(|word| *word > WORD_MASK) {
        return Err(IasError::InvalidStateWord { address });
    }
    if state.accumulator > WORD_MASK {
        return Err(IasError::InvalidAccumulator);
    }
    if state.arithmetic_register > WORD_MASK {
        return Err(IasError::InvalidArithmeticRegister);
    }
    if state.current_instruction > INSTRUCTION_MASK {
        return Err(IasError::InvalidControlRegister);
    }
    if state
        .buffered_right
        .is_some_and(|instruction| instruction > INSTRUCTION_MASK)
    {
        return Err(IasError::InvalidBufferedInstruction);
    }
    if state.function_table > Opcode::ShiftRight as u8 {
        return Err(IasError::InvalidFunctionTable);
    }
    if usize::from(state.memory_address) >= MEMORY_WORDS {
        return Err(IasError::InvalidMemoryAddress);
    }
    if state.selectron_register > WORD_MASK {
        return Err(IasError::InvalidSelectronRegister);
    }

    let start = usize::from(state.loaded_origin);
    if state.loaded_words == 0 {
        if state.loaded_origin != 0 || state.control_counter != 0 || state.buffered_right.is_some()
        {
            return Err(IasError::InvalidLoadedRange);
        }
        return Ok(());
    }
    if start >= MEMORY_WORDS || state.loaded_words > MEMORY_WORDS - start {
        return Err(IasError::InvalidLoadedRange);
    }
    let end = start + state.loaded_words;
    let counter = usize::from(state.control_counter);
    if counter < start || counter > end {
        return Err(IasError::InvalidControlCounter);
    }
    if state.buffered_right.is_some() && (counter == 0 || !(start..end).contains(&(counter - 1))) {
        return Err(IasError::InvalidControlCounter);
    }
    Ok(())
}

fn is_negative(value: u64) -> bool {
    value & SIGN_BIT != 0
}

fn negate(value: u64) -> u64 {
    (!value).wrapping_add(1) & WORD_MASK
}

fn absolute(value: u64) -> u64 {
    if is_negative(value) {
        negate(value)
    } else {
        value
    }
}

fn to_signed(value: u64) -> i64 {
    if is_negative(value) {
        (value | !WORD_MASK) as i64
    } else {
        value as i64
    }
}

fn from_signed(value: i64) -> u64 {
    (value as u64) & WORD_MASK
}
