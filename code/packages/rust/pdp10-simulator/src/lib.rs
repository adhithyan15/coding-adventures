//! Checked functional simulator for the bounded DEC PDP-10 KA10 surface in
//! Spec 07aq.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Number of addressable KA10 words.
pub const MEMORY_WORDS: usize = 1 << 18;
/// Mask for one 36-bit architectural word.
pub const WORD_MASK: u64 = (1_u64 << 36) - 1;
/// Mask for one 18-bit address or half-word.
pub const ADDRESS_MASK: u32 = (1_u32 << 18) - 1;
/// Default load origin (octal 0200).
pub const DEFAULT_ORIGIN: u32 = 0o200;

const SIGN36: u64 = 1 << 35;
const SIGN35_MASK: u64 = SIGN36 - 1;

/// Arithmetic status flags exposed by the bounded KA10 cell.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ArithmeticFlags {
    pub overflow: bool,
    pub carry0: bool,
    pub carry1: bool,
    pub divide_check: bool,
}

/// Complete, owned architectural and lifecycle state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pdp10State {
    pub accumulators: [u64; 16],
    pub pc: u32,
    pub flags: ArithmeticFlags,
    pub halted: bool,
    pub memory: Vec<u64>,
    pub installed_origin: u32,
    pub installed_words: u32,
}

/// One deterministic successful instruction trace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub instruction: u64,
    pub effective_address: Option<u32>,
    pub mnemonic: String,
}

/// Result of a successful bounded run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub final_state: Pdp10State,
    pub traces: Vec<StepTrace>,
}

/// Typed failures for all checked simulator boundaries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Pdp10Error {
    WordOutOfRange { value: u64 },
    AddressOutOfRange { address: u32 },
    InvalidAccumulator { index: usize },
    InvalidTransportLength { bytes: usize },
    NonZeroTransportPadding { word_index: usize, byte: u8 },
    ProgramOutOfRange { origin: u32, words: usize },
    InvalidMemoryLength { words: usize },
    InvalidInstalledRange { origin: u32, words: u32 },
    FetchOutsideProgram { pc: u32 },
    IndirectAddressLimit { pc: u32 },
    UnsupportedOpcode { opcode: u16, pc: u32 },
    InvalidInstruction { instruction: u64, pc: u32 },
    DivideByZero { pc: u32 },
    QuotientOverflow { pc: u32 },
    Halted { pc: u32 },
    MaxStepsZero,
    MaxStepsExceeded { max_steps: usize },
}

impl Display for Pdp10Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WordOutOfRange { value } => {
                write!(f, "PDP-10 word {value:#x} exceeds 36 bits")
            }
            Self::AddressOutOfRange { address } => {
                write!(f, "PDP-10 address {address:#x} exceeds 18 bits")
            }
            Self::InvalidAccumulator { index } => {
                write!(f, "accumulator AC{index} is outside AC0..AC15")
            }
            Self::InvalidTransportLength { bytes } => write!(
                f,
                "PDP-10 transport has {bytes} bytes; length must be a multiple of five"
            ),
            Self::NonZeroTransportPadding { word_index, byte } => write!(
                f,
                "PDP-10 transport word {word_index} has nonzero high padding in {byte:#04x}"
            ),
            Self::ProgramOutOfRange { origin, words } => write!(
                f,
                "program of {words} words at {origin:#07o} does not fit KA10 memory"
            ),
            Self::InvalidMemoryLength { words } => {
                write!(
                    f,
                    "snapshot memory has {words} words, expected {MEMORY_WORDS}"
                )
            }
            Self::InvalidInstalledRange { origin, words } => write!(
                f,
                "installed range {origin:#07o}+{words:#o} is outside KA10 memory"
            ),
            Self::FetchOutsideProgram { pc } => {
                write!(
                    f,
                    "instruction fetch at {pc:#07o} is outside installed program"
                )
            }
            Self::IndirectAddressLimit { pc } => write!(
                f,
                "effective-address indirection at PC {pc:#07o} exceeded 64 links"
            ),
            Self::UnsupportedOpcode { opcode, pc } => {
                write!(f, "unsupported KA10 opcode {opcode:#05o} at PC {pc:#07o}")
            }
            Self::InvalidInstruction { instruction, pc } => write!(
                f,
                "invalid KA10 instruction {instruction:#014o} at PC {pc:#07o}"
            ),
            Self::DivideByZero { pc } => write!(f, "integer divide by zero at PC {pc:#07o}"),
            Self::QuotientOverflow { pc } => {
                write!(f, "integer quotient overflow at PC {pc:#07o}")
            }
            Self::Halted { pc } => write!(f, "processor is halted at PC {pc:#07o}"),
            Self::MaxStepsZero => write!(f, "maximum step count must be nonzero"),
            Self::MaxStepsExceeded { max_steps } => {
                write!(f, "maximum execution step count {max_steps} exceeded")
            }
        }
    }
}

impl Error for Pdp10Error {}

/// Decode canonical five-byte big-endian 36-bit word transport.
pub fn decode_transport(program: &[u8]) -> Result<Vec<u64>, Pdp10Error> {
    if !program.len().is_multiple_of(5) {
        return Err(Pdp10Error::InvalidTransportLength {
            bytes: program.len(),
        });
    }
    let mut words = Vec::new();
    for (word_index, chunk) in program.chunks_exact(5).enumerate() {
        if chunk[0] & 0xf0 != 0 {
            return Err(Pdp10Error::NonZeroTransportPadding {
                word_index,
                byte: chunk[0],
            });
        }
        let value = (u64::from(chunk[0]) << 32)
            | (u64::from(chunk[1]) << 24)
            | (u64::from(chunk[2]) << 16)
            | (u64::from(chunk[3]) << 8)
            | u64::from(chunk[4]);
        words.push(value);
    }
    Ok(words)
}

/// Encode one checked 36-bit word in canonical transport form.
pub fn encode_word(word: u64) -> Result<[u8; 5], Pdp10Error> {
    validate_word(word)?;
    Ok([
        ((word >> 32) & 0x0f) as u8,
        (word >> 24) as u8,
        (word >> 16) as u8,
        (word >> 8) as u8,
        word as u8,
    ])
}

/// Encode one checked KA10 instruction in canonical transport form.
pub fn encode_instruction(
    opcode: u16,
    accumulator: u8,
    indirect: bool,
    index: u8,
    address: u32,
) -> Result<[u8; 5], Pdp10Error> {
    if opcode > 0o777 || accumulator > 0o17 || index > 0o17 {
        return Err(Pdp10Error::InvalidInstruction {
            instruction: 0,
            pc: 0,
        });
    }
    validate_address(address)?;
    let word = (u64::from(opcode) << 27)
        | (u64::from(accumulator) << 23)
        | (u64::from(indirect) << 22)
        | (u64::from(index) << 18)
        | u64::from(address);
    encode_word(word)
}

/// Bounded KA10 functional machine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pdp10Simulator {
    accumulators: [u64; 16],
    pc: u32,
    flags: ArithmeticFlags,
    halted: bool,
    memory: Vec<u64>,
    installed_origin: u32,
    installed_words: u32,
}

impl Default for Pdp10Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Pdp10Simulator {
    /// Construct a powered-on processor with cleared storage.
    #[must_use]
    pub fn new() -> Self {
        Self {
            accumulators: [0; 16],
            pc: DEFAULT_ORIGIN,
            flags: ArithmeticFlags::default(),
            halted: false,
            memory: vec![0; MEMORY_WORDS],
            installed_origin: DEFAULT_ORIGIN,
            installed_words: 0,
        }
    }

    /// Restore power-on state, including cleared memory.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Load canonical word transport at an explicit 18-bit origin.
    pub fn load_at(&mut self, program: &[u8], origin: u32) -> Result<usize, Pdp10Error> {
        validate_address(origin)?;
        let words = decode_transport(program)?;
        let end = usize::try_from(origin)
            .expect("18-bit origin fits usize")
            .checked_add(words.len())
            .filter(|end| *end <= MEMORY_WORDS)
            .ok_or(Pdp10Error::ProgramOutOfRange {
                origin,
                words: words.len(),
            })?;
        let mut replacement = Self::new();
        replacement.pc = origin;
        replacement.installed_origin = origin;
        replacement.installed_words =
            u32::try_from(words.len()).map_err(|_| Pdp10Error::ProgramOutOfRange {
                origin,
                words: words.len(),
            })?;
        let start = usize::try_from(origin).expect("18-bit origin fits usize");
        replacement.memory[start..end].copy_from_slice(&words);
        *self = replacement;
        Ok(words.len())
    }

    /// Load canonical transport at octal 0200.
    pub fn load(&mut self, program: &[u8]) -> Result<usize, Pdp10Error> {
        self.load_at(program, DEFAULT_ORIGIN)
    }

    /// Return a complete owned snapshot.
    #[must_use]
    pub fn state(&self) -> Pdp10State {
        Pdp10State {
            accumulators: self.accumulators,
            pc: self.pc,
            flags: self.flags,
            halted: self.halted,
            memory: self.memory.clone(),
            installed_origin: self.installed_origin,
            installed_words: self.installed_words,
        }
    }

    /// Validate and atomically restore a complete snapshot.
    pub fn restore(&mut self, state: &Pdp10State) -> Result<(), Pdp10Error> {
        validate_state(state)?;
        self.accumulators = state.accumulators;
        self.pc = state.pc;
        self.flags = state.flags;
        self.halted = state.halted;
        self.memory.clone_from(&state.memory);
        self.installed_origin = state.installed_origin;
        self.installed_words = state.installed_words;
        Ok(())
    }

    /// Read one accumulator.
    pub fn accumulator(&self, index: usize) -> Result<u64, Pdp10Error> {
        self.accumulators
            .get(index)
            .copied()
            .ok_or(Pdp10Error::InvalidAccumulator { index })
    }

    /// Write one checked accumulator.
    pub fn set_accumulator(&mut self, index: usize, value: u64) -> Result<(), Pdp10Error> {
        validate_word(value)?;
        let accumulator = self
            .accumulators
            .get_mut(index)
            .ok_or(Pdp10Error::InvalidAccumulator { index })?;
        *accumulator = value;
        Ok(())
    }

    /// Read physical main memory without accumulator aliasing.
    pub fn memory_word(&self, address: u32) -> Result<u64, Pdp10Error> {
        validate_address(address)?;
        Ok(self.memory[usize::try_from(address).expect("18-bit address fits usize")])
    }

    /// Write physical main memory without accumulator aliasing.
    pub fn set_memory_word(&mut self, address: u32, value: u64) -> Result<(), Pdp10Error> {
        validate_address(address)?;
        validate_word(value)?;
        self.memory[usize::try_from(address).expect("18-bit address fits usize")] = value;
        Ok(())
    }

    /// Execute one checked instruction atomically.
    pub fn step(&mut self) -> Result<StepTrace, Pdp10Error> {
        if self.halted {
            return Err(Pdp10Error::Halted { pc: self.pc });
        }
        if !self.fetch_is_installed(self.pc) {
            return Err(Pdp10Error::FetchOutsideProgram { pc: self.pc });
        }
        let checkpoint = self.clone();
        let pc_before = self.pc;
        let instruction = self.memory[usize::try_from(self.pc).expect("PC fits usize")];
        self.pc = self.pc.wrapping_add(1) & ADDRESS_MASK;
        match self.execute_instruction(instruction, pc_before) {
            Ok((mnemonic, effective_address)) => Ok(StepTrace {
                pc_before,
                pc_after: self.pc,
                instruction,
                effective_address,
                mnemonic,
            }),
            Err(error) => {
                *self = checkpoint;
                Err(error)
            }
        }
    }

    /// Run until halt, rolling the whole call back on any failure.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Pdp10Error> {
        if max_steps == 0 {
            return Err(Pdp10Error::MaxStepsZero);
        }
        let checkpoint = self.clone();
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.halted {
                return Ok(ExecutionResult {
                    halted: true,
                    steps: traces.len(),
                    final_state: self.state(),
                    traces,
                });
            }
            match self.step() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    *self = checkpoint;
                    return Err(error);
                }
            }
        }
        if self.halted {
            return Ok(ExecutionResult {
                halted: true,
                steps: traces.len(),
                final_state: self.state(),
                traces,
            });
        }
        *self = checkpoint;
        Err(Pdp10Error::MaxStepsExceeded { max_steps })
    }

    /// Load and run a fresh processor.
    pub fn execute(
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, Pdp10Error> {
        let mut simulator = Self::new();
        simulator.load_at(program, origin)?;
        simulator.run(max_steps)
    }

    fn fetch_is_installed(&self, pc: u32) -> bool {
        let start = u64::from(self.installed_origin);
        let end = start + u64::from(self.installed_words);
        let pc = u64::from(pc);
        pc >= start && pc < end
    }

    fn read_data(&self, address: u32) -> u64 {
        if address < 16 {
            self.accumulators[address as usize]
        } else {
            self.memory[usize::try_from(address).expect("18-bit address fits usize")]
        }
    }

    fn write_data(&mut self, address: u32, value: u64) {
        let value = value & WORD_MASK;
        if address < 16 {
            self.accumulators[address as usize] = value;
        } else {
            self.memory[usize::try_from(address).expect("18-bit address fits usize")] = value;
        }
    }

    fn effective_address(
        &self,
        mut indirect: bool,
        mut index: u8,
        mut address: u32,
        pc: u32,
    ) -> Result<u32, Pdp10Error> {
        for _ in 0..64 {
            if index != 0 {
                let index_value = self.accumulators[usize::from(index)] as u32 & ADDRESS_MASK;
                address = address.wrapping_add(index_value) & ADDRESS_MASK;
            }
            if !indirect {
                return Ok(address);
            }
            let word = self.read_data(address);
            indirect = ((word >> 22) & 1) != 0;
            index = ((word >> 18) & 0x0f) as u8;
            address = (word as u32) & ADDRESS_MASK;
        }
        Err(Pdp10Error::IndirectAddressLimit { pc })
    }

    fn execute_instruction(
        &mut self,
        instruction: u64,
        pc_before: u32,
    ) -> Result<(String, Option<u32>), Pdp10Error> {
        let opcode = ((instruction >> 27) & 0x1ff) as u16;
        let accumulator = ((instruction >> 23) & 0x0f) as usize;
        let indirect = ((instruction >> 22) & 1) != 0;
        let index = ((instruction >> 18) & 0x0f) as u8;
        let address = (instruction as u32) & ADDRESS_MASK;
        let ea = self.effective_address(indirect, index, address, pc_before)?;

        let mnemonic = match opcode {
            0o200..=0o203 => self.execute_move(opcode, accumulator, ea),
            0o210..=0o217 => self.execute_magnitude(opcode, accumulator, ea),
            0o220..=0o223 => self.execute_imul(opcode, accumulator, ea),
            0o230..=0o233 => self.execute_idiv(opcode, accumulator, ea, pc_before)?,
            0o252 | 0o253 => self.execute_aobj(opcode, accumulator, ea),
            0o254 => self.execute_jrst(accumulator, ea, instruction, pc_before)?,
            0o260..=0o263 => self.execute_stack(opcode, accumulator, ea),
            0o270..=0o277 => self.execute_add_sub(opcode, accumulator, ea),
            0o300..=0o307 => self.execute_cai(opcode, accumulator, ea),
            0o310..=0o317 => self.execute_cam(opcode, accumulator, ea),
            0o320..=0o327 => self.execute_jump(opcode, accumulator, ea),
            0o340..=0o347 => self.execute_aoj(opcode, accumulator, ea),
            0o360..=0o367 => self.execute_soj(opcode, accumulator, ea),
            0o400..=0o403 | 0o404..=0o407 | 0o430..=0o437 | 0o444..=0o447 | 0o474..=0o477 => {
                self.execute_boolean(opcode, accumulator, ea)
            }
            _ => {
                return Err(Pdp10Error::UnsupportedOpcode {
                    opcode,
                    pc: pc_before,
                });
            }
        };
        Ok((mnemonic, Some(ea)))
    }

    fn execute_move(&mut self, opcode: u16, ac: usize, ea: u32) -> String {
        match opcode & 3 {
            0 => self.accumulators[ac] = self.read_data(ea),
            1 => self.accumulators[ac] = u64::from(ea),
            2 => self.write_data(ea, self.accumulators[ac]),
            3 if ac != 0 => self.accumulators[ac] = self.read_data(ea),
            3 => {}
            _ => unreachable!(),
        }
        ["MOVE", "MOVEI", "MOVEM", "MOVES"][usize::from(opcode & 3)].to_owned()
    }

    fn execute_magnitude(&mut self, opcode: u16, ac: usize, ea: u32) -> String {
        let source = match opcode & 3 {
            1 => u64::from(ea),
            2 => self.accumulators[ac],
            0 | 3 => self.read_data(ea),
            _ => unreachable!(),
        };
        let negative = opcode < 0o214;
        let signed = signed36(source);
        let transformed = if negative {
            signed.wrapping_neg()
        } else {
            signed.saturating_abs()
        };
        let result = word36(transformed);
        let overflow = transformed > max_signed36();
        self.flags.overflow |= overflow;
        self.flags.carry0 |= negative && result == 0;
        self.flags.carry1 |= overflow || (negative && result == 0);
        self.store_form(opcode, ac, ea, result);
        if negative { "MOVN" } else { "MOVM" }.to_owned()
    }

    fn execute_imul(&mut self, opcode: u16, ac: usize, ea: u32) -> String {
        let rhs = self.form_operand(opcode, ea);
        let product = i128::from(signed36(self.accumulators[ac])) * i128::from(signed36(rhs));
        self.flags.overflow |=
            product < i128::from(min_signed36()) || product > i128::from(max_signed36());
        let result = (product as u128 as u64) & WORD_MASK;
        self.store_form(opcode, ac, ea, result);
        "IMUL".to_owned()
    }

    fn execute_idiv(
        &mut self,
        opcode: u16,
        ac: usize,
        ea: u32,
        pc: u32,
    ) -> Result<String, Pdp10Error> {
        let divisor = signed36(self.form_operand(opcode, ea));
        if divisor == 0 {
            self.flags.divide_check = true;
            return Err(Pdp10Error::DivideByZero { pc });
        }
        let next = (ac + 1) & 0x0f;
        let dividend = signed36(self.accumulators[ac]);
        let quotient = i128::from(dividend) / i128::from(divisor);
        let remainder = i128::from(dividend) % i128::from(divisor);
        if quotient < i128::from(min_signed36()) || quotient > i128::from(max_signed36()) {
            self.flags.divide_check = true;
            return Err(Pdp10Error::QuotientOverflow { pc });
        }
        let quotient_word = (quotient as u128 as u64) & WORD_MASK;
        let remainder_word = (remainder as u128 as u64) & WORD_MASK;
        match opcode & 3 {
            0 | 1 => {
                self.accumulators[ac] = quotient_word;
                self.accumulators[next] = remainder_word;
            }
            2 => self.write_data(ea, quotient_word),
            3 => {
                self.accumulators[ac] = quotient_word;
                self.accumulators[next] = remainder_word;
                self.write_data(ea, quotient_word);
            }
            _ => unreachable!(),
        }
        Ok("IDIV".to_owned())
    }

    fn execute_aobj(&mut self, opcode: u16, ac: usize, ea: u32) -> String {
        self.accumulators[ac] = self.accumulators[ac].wrapping_add((1_u64 << 18) | 1) & WORD_MASK;
        let negative = signed36(self.accumulators[ac]) < 0;
        if (opcode == 0o252 && !negative) || (opcode == 0o253 && negative) {
            self.pc = ea;
        }
        if opcode == 0o252 { "AOBJP" } else { "AOBJN" }.to_owned()
    }

    fn execute_jrst(
        &mut self,
        ac: usize,
        ea: u32,
        instruction: u64,
        pc: u32,
    ) -> Result<String, Pdp10Error> {
        match ac {
            0 => {
                self.pc = ea;
                Ok("JRST".to_owned())
            }
            4 => {
                self.pc = ea;
                self.halted = true;
                Ok("HALT".to_owned())
            }
            _ => Err(Pdp10Error::InvalidInstruction { instruction, pc }),
        }
    }

    fn execute_stack(&mut self, opcode: u16, ac: usize, ea: u32) -> String {
        match opcode {
            0o260 => {
                let address = self.push_address(ac);
                self.write_data(
                    address,
                    (u64::from(self.packed_flags()) << 18) | u64::from(self.pc),
                );
                self.pc = ea;
                "PUSHJ"
            }
            0o261 => {
                let value = self.read_data(ea);
                let address = self.push_address(ac);
                self.write_data(address, value);
                "PUSH"
            }
            0o262 => {
                let address = self.pop_address(ac);
                let value = self.read_data(address);
                self.write_data(ea, value);
                self.decrement_stack(ac);
                "POP"
            }
            0o263 => {
                let address = self.pop_address(ac);
                self.pc = self.read_data(address) as u32 & ADDRESS_MASK;
                self.decrement_stack(ac);
                "POPJ"
            }
            _ => unreachable!(),
        }
        .to_owned()
    }

    fn execute_add_sub(&mut self, opcode: u16, ac: usize, ea: u32) -> String {
        let rhs = self.form_operand(opcode, ea);
        let lhs = self.accumulators[ac];
        let subtract = opcode >= 0o274;
        let result = if subtract {
            self.sub36(lhs, rhs)
        } else {
            self.add36(lhs, rhs)
        };
        self.store_form(opcode, ac, ea, result);
        if subtract { "SUB" } else { "ADD" }.to_owned()
    }

    fn execute_cai(&mut self, opcode: u16, ac: usize, ea: u32) -> String {
        if condition(
            opcode & 7,
            signed36(self.accumulators[ac]).cmp(&i64::from(ea)),
        ) {
            self.pc = self.pc.wrapping_add(1) & ADDRESS_MASK;
        }
        format!("CAI{}", condition_suffix(opcode & 7))
    }

    fn execute_cam(&mut self, opcode: u16, ac: usize, ea: u32) -> String {
        let comparison = signed36(self.accumulators[ac]).cmp(&signed36(self.read_data(ea)));
        if condition(opcode & 7, comparison) {
            self.pc = self.pc.wrapping_add(1) & ADDRESS_MASK;
        }
        format!("CAM{}", condition_suffix(opcode & 7))
    }

    fn execute_jump(&mut self, opcode: u16, ac: usize, ea: u32) -> String {
        if condition(opcode & 7, signed36(self.accumulators[ac]).cmp(&0)) {
            self.pc = ea;
        }
        format!("JUMP{}", condition_suffix(opcode & 7))
    }

    fn execute_aoj(&mut self, opcode: u16, ac: usize, ea: u32) -> String {
        let value = self.add36(self.accumulators[ac], 1);
        self.accumulators[ac] = value;
        if condition(opcode & 7, signed36(value).cmp(&0)) {
            self.pc = ea;
        }
        format!("AOJ{}", condition_suffix(opcode & 7))
    }

    fn execute_soj(&mut self, opcode: u16, ac: usize, ea: u32) -> String {
        let value = self.sub36(self.accumulators[ac], 1);
        self.accumulators[ac] = value;
        if condition(opcode & 7, signed36(value).cmp(&0)) {
            self.pc = ea;
        }
        format!("SOJ{}", condition_suffix(opcode & 7))
    }

    fn execute_boolean(&mut self, opcode: u16, ac: usize, ea: u32) -> String {
        let family = opcode & !3;
        let rhs = self.form_operand(opcode, ea);
        let result = match family {
            0o400 => 0,
            0o404 => self.accumulators[ac] & rhs,
            0o430 => self.accumulators[ac] ^ rhs,
            0o434 => self.accumulators[ac] | rhs,
            0o444 => (!(self.accumulators[ac] ^ rhs)) & WORD_MASK,
            0o474 => WORD_MASK,
            _ => unreachable!(),
        };
        self.store_form(opcode, ac, ea, result);
        match family {
            0o400 => "SETZ",
            0o404 => "AND",
            0o430 => "XOR",
            0o434 => "IOR",
            0o444 => "EQV",
            0o474 => "SETO",
            _ => unreachable!(),
        }
        .to_owned()
    }

    fn form_operand(&self, opcode: u16, ea: u32) -> u64 {
        if opcode & 3 == 1 {
            u64::from(ea)
        } else {
            self.read_data(ea)
        }
    }

    fn store_form(&mut self, opcode: u16, ac: usize, ea: u32, value: u64) {
        match opcode & 3 {
            0 | 1 => self.accumulators[ac] = value,
            2 => self.write_data(ea, value),
            3 => {
                self.accumulators[ac] = value;
                self.write_data(ea, value);
            }
            _ => unreachable!(),
        }
    }

    fn add36(&mut self, lhs: u64, rhs: u64) -> u64 {
        let full = u128::from(lhs) + u128::from(rhs);
        let low35 = (lhs & SIGN35_MASK) + (rhs & SIGN35_MASK);
        let carry0 = full > u128::from(WORD_MASK);
        let carry1 = low35 > SIGN35_MASK;
        self.flags.carry0 |= carry0;
        self.flags.carry1 |= carry1;
        self.flags.overflow |= carry0 != carry1;
        (full as u64) & WORD_MASK
    }

    fn sub36(&mut self, lhs: u64, rhs: u64) -> u64 {
        let complement = (!rhs) & WORD_MASK;
        let full = u128::from(lhs) + u128::from(complement) + 1;
        let low35 = (lhs & SIGN35_MASK) + (complement & SIGN35_MASK) + 1;
        let carry0 = full > u128::from(WORD_MASK);
        let carry1 = low35 > SIGN35_MASK;
        self.flags.carry0 |= carry0;
        self.flags.carry1 |= carry1;
        self.flags.overflow |= carry0 != carry1;
        (full as u64) & WORD_MASK
    }

    fn push_address(&mut self, ac: usize) -> u32 {
        let left = ((self.accumulators[ac] >> 18) as u32).wrapping_add(1) & ADDRESS_MASK;
        let right = (self.accumulators[ac] as u32).wrapping_add(1) & ADDRESS_MASK;
        self.accumulators[ac] = (u64::from(left) << 18) | u64::from(right);
        right
    }

    fn pop_address(&self, ac: usize) -> u32 {
        self.accumulators[ac] as u32 & ADDRESS_MASK
    }

    fn decrement_stack(&mut self, ac: usize) {
        let left = ((self.accumulators[ac] >> 18) as u32).wrapping_sub(1) & ADDRESS_MASK;
        let right = (self.accumulators[ac] as u32).wrapping_sub(1) & ADDRESS_MASK;
        self.accumulators[ac] = (u64::from(left) << 18) | u64::from(right);
    }

    fn packed_flags(&self) -> u32 {
        (u32::from(self.flags.overflow) << 17)
            | (u32::from(self.flags.carry0) << 16)
            | (u32::from(self.flags.carry1) << 15)
            | (u32::from(self.flags.divide_check) << 5)
    }
}

fn validate_word(value: u64) -> Result<(), Pdp10Error> {
    if value & !WORD_MASK != 0 {
        return Err(Pdp10Error::WordOutOfRange { value });
    }
    Ok(())
}

fn validate_address(address: u32) -> Result<(), Pdp10Error> {
    if address & !ADDRESS_MASK != 0 {
        return Err(Pdp10Error::AddressOutOfRange { address });
    }
    Ok(())
}

fn validate_state(state: &Pdp10State) -> Result<(), Pdp10Error> {
    if state.memory.len() != MEMORY_WORDS {
        return Err(Pdp10Error::InvalidMemoryLength {
            words: state.memory.len(),
        });
    }
    validate_address(state.pc)?;
    for &word in state.accumulators.iter().chain(state.memory.iter()) {
        validate_word(word)?;
    }
    let end = u64::from(state.installed_origin) + u64::from(state.installed_words);
    if state.installed_origin > ADDRESS_MASK || end > MEMORY_WORDS as u64 {
        return Err(Pdp10Error::InvalidInstalledRange {
            origin: state.installed_origin,
            words: state.installed_words,
        });
    }
    Ok(())
}

fn signed36(word: u64) -> i64 {
    let word = word & WORD_MASK;
    if word & SIGN36 != 0 {
        (word as i64) - (1_i64 << 36)
    } else {
        word as i64
    }
}

fn word36(value: i64) -> u64 {
    (value as u64) & WORD_MASK
}

const fn min_signed36() -> i64 {
    -(1_i64 << 35)
}

const fn max_signed36() -> i64 {
    (1_i64 << 35) - 1
}

fn condition(selector: u16, ordering: std::cmp::Ordering) -> bool {
    use std::cmp::Ordering::{Equal, Greater, Less};
    match selector {
        0 => false,
        1 => ordering == Less,
        2 => ordering == Equal,
        3 => ordering != Greater,
        4 => true,
        5 => ordering != Less,
        6 => ordering != Equal,
        7 => ordering == Greater,
        _ => unreachable!(),
    }
}

fn condition_suffix(selector: u16) -> &'static str {
    ["", "L", "E", "LE", "A", "GE", "N", "G"][usize::from(selector)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instruction(opcode: u16, ac: u8, indirect: bool, index: u8, address: u32) -> u64 {
        let bytes = encode_instruction(opcode, ac, indirect, index, address).unwrap();
        decode_transport(&bytes).unwrap()[0]
    }

    fn program(words: &[u64]) -> Vec<u8> {
        words
            .iter()
            .flat_map(|word| encode_word(*word).unwrap())
            .collect()
    }

    #[test]
    fn transport_is_strict_and_round_trips() {
        let word = instruction(0o201, 1, true, 2, 0o123456);
        assert_eq!(
            decode_transport(&encode_word(word).unwrap()).unwrap(),
            [word]
        );
        assert!(matches!(
            decode_transport(&[0; 4]),
            Err(Pdp10Error::InvalidTransportLength { bytes: 4 })
        ));
        assert!(matches!(
            decode_transport(&[0x10, 0, 0, 0, 0]),
            Err(Pdp10Error::NonZeroTransportPadding { .. })
        ));
        assert!(encode_word(1 << 36).is_err());
    }

    #[test]
    fn load_restore_and_aliasing_are_checked() {
        let mut cpu = Pdp10Simulator::new();
        cpu.load_at(&program(&[instruction(0o254, 4, false, 0, 0)]), 0o200)
            .unwrap();
        cpu.set_accumulator(3, 0o123).unwrap();
        assert_eq!(cpu.read_data(3), 0o123);
        cpu.write_data(3, 0o456);
        assert_eq!(cpu.accumulator(3).unwrap(), 0o456);
        assert_eq!(cpu.memory_word(3).unwrap(), 0);
        let state = cpu.state();
        cpu.reset();
        cpu.restore(&state).unwrap();
        assert_eq!(cpu.state(), state);
        let mut invalid = state;
        invalid.memory.pop();
        let before = cpu.state();
        assert!(cpu.restore(&invalid).is_err());
        assert_eq!(cpu.state(), before);
    }

    #[test]
    fn move_arithmetic_boolean_and_halt_execute() {
        let words = [
            instruction(0o201, 1, false, 0, 7),
            instruction(0o271, 1, false, 0, 5),
            instruction(0o431, 1, false, 0, 3),
            instruction(0o202, 1, false, 0, 0o400),
            instruction(0o254, 4, false, 0, 0),
        ];
        let result = Pdp10Simulator::execute(&program(&words), 0o200, 5).unwrap();
        assert!(result.halted);
        assert_eq!(result.steps, 5);
        assert_eq!(result.final_state.memory[0o400], (7 + 5) ^ 3);
        assert_eq!(result.traces[0].mnemonic, "MOVEI");
        assert_eq!(result.traces[4].mnemonic, "HALT");
    }

    #[test]
    fn indexed_indirect_addressing_and_limit_are_atomic() {
        let words = [
            instruction(0o200, 1, true, 2, 0o300),
            instruction(0o254, 4, false, 0, 0),
        ];
        let mut cpu = Pdp10Simulator::new();
        cpu.load_at(&program(&words), 0o200).unwrap();
        cpu.set_accumulator(2, 2).unwrap();
        cpu.set_memory_word(0o302, instruction(0, 0, false, 0, 0o500))
            .unwrap();
        cpu.set_memory_word(0o500, 0o7654321).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.accumulator(1).unwrap(), 0o7654321);

        cpu.restore(&{
            let mut state = cpu.state();
            state.pc = 0o200;
            state.halted = false;
            state.memory[0o302] = instruction(0, 0, true, 0, 0o302);
            state
        })
        .unwrap();
        let before = cpu.state();
        assert!(matches!(
            cpu.step(),
            Err(Pdp10Error::IndirectAddressLimit { .. })
        ));
        assert_eq!(cpu.state(), before);
    }

    #[test]
    fn compare_jump_and_count_conditions_cover_all_selectors() {
        for selector in 0..8_u16 {
            let words = [
                instruction(0o300 + selector, 1, false, 0, 5),
                instruction(0o254, 4, false, 0, 0),
                instruction(0o254, 4, false, 0, 0),
            ];
            let mut cpu = Pdp10Simulator::new();
            cpu.load_at(&program(&words), 0o200).unwrap();
            cpu.set_accumulator(1, 4).unwrap();
            let trace = cpu.step().unwrap();
            let expected = if condition(selector, 4_i64.cmp(&5)) {
                0o202
            } else {
                0o201
            };
            assert_eq!(trace.pc_after, expected);
        }
    }

    #[test]
    fn multiply_divide_and_faults_are_atomic() {
        let words = [
            instruction(0o221, 1, false, 0, 7),
            instruction(0o231, 1, false, 0, 3),
            instruction(0o254, 4, false, 0, 0),
        ];
        let mut cpu = Pdp10Simulator::new();
        cpu.load_at(&program(&words), 0o200).unwrap();
        cpu.set_accumulator(1, 6).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.accumulator(1).unwrap(), 42);
        cpu.set_accumulator(2, WORD_MASK).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.accumulator(1).unwrap(), 14);
        assert_eq!(cpu.accumulator(2).unwrap(), 0);

        let failing = program(&[instruction(0o231, 1, false, 0, 0)]);
        cpu.load_at(&failing, 0o200).unwrap();
        let before = cpu.state();
        assert!(matches!(cpu.step(), Err(Pdp10Error::DivideByZero { .. })));
        assert_eq!(cpu.state(), before);
    }

    #[test]
    fn stack_and_bounded_run_are_transactional() {
        let words = [
            instruction(0o261, 3, false, 0, 0o300),
            instruction(0o262, 3, false, 0, 0o301),
            instruction(0o254, 4, false, 0, 0),
        ];
        let mut cpu = Pdp10Simulator::new();
        cpu.load_at(&program(&words), 0o200).unwrap();
        cpu.set_accumulator(3, (0o777_u64 << 18) | 0o400).unwrap();
        cpu.set_memory_word(0o300, 0o123456).unwrap();
        let result = cpu.run(3).unwrap();
        assert_eq!(result.final_state.memory[0o301], 0o123456);
        assert_eq!(
            result.final_state.accumulators[3],
            (0o777_u64 << 18) | 0o400
        );

        let mut cpu = Pdp10Simulator::new();
        cpu.load_at(&program(&[instruction(0o201, 1, false, 0, 1)]), 0o200)
            .unwrap();
        let before = cpu.state();
        assert!(matches!(
            cpu.run(1),
            Err(Pdp10Error::MaxStepsExceeded { max_steps: 1 })
        ));
        assert_eq!(cpu.state(), before);
    }
}
