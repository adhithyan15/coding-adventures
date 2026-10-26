//! Checked functional simulator for the common Data General Nova CPU core.
//!
//! The model is intentionally bounded to the original 32-Kword Nova
//! architecture. Later Nova 3/4, Eclipse, 64-Kword, peripheral, and optional
//! arithmetic extensions remain explicit future work.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Number of 16-bit words in the common Nova address space.
pub const MEMORY_WORDS: usize = 32_768;
/// Mask for every 15-bit Nova address.
pub const ADDRESS_MASK: u16 = 0o77_777;
/// Conventional default load origin used by [`NovaSimulator::load`].
pub const DEFAULT_ORIGIN: u16 = 0o200;
/// Exact CPU-device halt instruction (`DOC 0,CPU`).
pub const HALT: u16 = 0o63_077;
const CARRY_BIT: u32 = 1 << 16;
const DATA_MASK: u32 = 0xffff;
const CARRY_DATA_MASK: u32 = CARRY_BIT | DATA_MASK;
const INDIRECT_LIMIT: usize = 65_536;

/// A semantic memory-reference operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryOperation {
    Jmp,
    Jsr,
    Isz,
    Dsz,
    Lda(u8),
    Sta(u8),
}

/// The four Nova displacement bases.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum AddressMode {
    PageZero = 0,
    PcRelative = 1,
    Ac2Relative = 2,
    Ac3Relative = 3,
}

/// Operate-field ALU selector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum AluOperation {
    Complement = 0,
    Negate = 1,
    Move = 2,
    Increment = 3,
    AddComplement = 4,
    Subtract = 5,
    Add = 6,
    And = 7,
}

/// Operate-field shift selector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum ShiftOperation {
    None = 0,
    Left = 1,
    Right = 2,
    ByteSwap = 3,
}

/// Operate-field carry-input selector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum CarryControl {
    Retain = 0,
    Clear = 1,
    Set = 2,
    Complement = 3,
}

/// Operate-field skip selector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum SkipCondition {
    None = 0,
    Always = 1,
    CarryZero = 2,
    CarryNonzero = 3,
    ResultZero = 4,
    ResultNonzero = 5,
    EitherZero = 6,
    BothNonzero = 7,
}

/// Encode one Nova memory-reference instruction.
pub fn encode_memory_reference(
    operation: MemoryOperation,
    indirect: bool,
    mode: AddressMode,
    displacement: u8,
) -> Result<u16, NovaError> {
    let opac = match operation {
        MemoryOperation::Jmp => 0,
        MemoryOperation::Jsr => 1,
        MemoryOperation::Isz => 2,
        MemoryOperation::Dsz => 3,
        MemoryOperation::Lda(ac) => 4 + checked_accumulator(ac)?,
        MemoryOperation::Sta(ac) => 8 + checked_accumulator(ac)?,
    };
    Ok((u16::from(opac) << 11)
        | (u16::from(indirect) << 10)
        | ((mode as u16) << 8)
        | u16::from(displacement))
}

/// Encode one Nova operate instruction.
#[allow(clippy::too_many_arguments)]
pub fn encode_operate(
    source: u8,
    destination: u8,
    alu: AluOperation,
    shift: ShiftOperation,
    carry: CarryControl,
    no_load: bool,
    skip: SkipCondition,
) -> Result<u16, NovaError> {
    let source = checked_accumulator(source)?;
    let destination = checked_accumulator(destination)?;
    Ok(0x8000
        | (u16::from(source) << 13)
        | (u16::from(destination) << 11)
        | ((alu as u16) << 8)
        | ((shift as u16) << 6)
        | ((carry as u16) << 4)
        | (u16::from(no_load) << 3)
        | skip as u16)
}

/// Convert a word to canonical two-byte big-endian transport.
pub const fn encode_word(word: u16) -> [u8; 2] {
    word.to_be_bytes()
}

/// Complete, deep-owned architectural and lifecycle state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NovaState {
    pub accumulators: [u16; 4],
    pub pc: u16,
    pub carry: bool,
    pub memory: Vec<u16>,
    pub halted: bool,
    pub installed_origin: u16,
    pub installed_words: usize,
}

/// One successfully retired instruction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    pub instruction: u16,
    pub effective_address: Option<u16>,
    pub mnemonic: String,
}

/// Result of a bounded execution that reached HALT.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub final_state: NovaState,
    pub traces: Vec<StepTrace>,
}

/// Fail-closed transport, architecture, and lifecycle faults.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NovaError {
    InvalidTransportLength { bytes: usize },
    AddressOutOfRange { address: u32 },
    InvalidAccumulator { index: u8 },
    ProgramOutOfRange { origin: u16, words: usize },
    InvalidMemoryLength { words: usize },
    InvalidInstalledRange { origin: u16, words: usize },
    FetchOutsideProgram { pc: u16 },
    IndirectAddressLimit { pc: u16 },
    UnsupportedIo { instruction: u16, pc: u16 },
    Halted,
    ZeroStepLimit,
    MaxStepsExceeded { max_steps: usize },
}

impl Display for NovaError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransportLength { bytes } => {
                write!(formatter, "program transport has odd byte length {bytes}")
            }
            Self::AddressOutOfRange { address } => {
                write!(formatter, "address {address:#07o} exceeds 15 bits")
            }
            Self::InvalidAccumulator { index } => {
                write!(formatter, "accumulator index {index} is outside 0..=3")
            }
            Self::ProgramOutOfRange { origin, words } => write!(
                formatter,
                "{words}-word program does not fit at origin {origin:#07o}"
            ),
            Self::InvalidMemoryLength { words } => {
                write!(
                    formatter,
                    "state contains {words} memory words, expected {MEMORY_WORDS}"
                )
            }
            Self::InvalidInstalledRange { origin, words } => write!(
                formatter,
                "installed range {origin:#07o}+{words} is outside memory"
            ),
            Self::FetchOutsideProgram { pc } => {
                write!(
                    formatter,
                    "fetch at {pc:#07o} is outside the installed program"
                )
            }
            Self::IndirectAddressLimit { pc } => {
                write!(
                    formatter,
                    "indirect-address chain at {pc:#07o} exceeded its bound"
                )
            }
            Self::UnsupportedIo { instruction, pc } => write!(
                formatter,
                "I/O instruction {instruction:#07o} at {pc:#07o} is outside this CPU cell"
            ),
            Self::Halted => write!(formatter, "the Nova CPU is halted"),
            Self::ZeroStepLimit => write!(formatter, "execution step bound must be nonzero"),
            Self::MaxStepsExceeded { max_steps } => {
                write!(
                    formatter,
                    "maximum execution step count {max_steps} exceeded"
                )
            }
        }
    }
}

impl Error for NovaError {}

/// Checked Data General Nova functional simulator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NovaSimulator {
    state: NovaState,
}

impl Default for NovaSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl NovaSimulator {
    /// Construct a reset machine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: NovaState {
                accumulators: [0; 4],
                pc: 0,
                carry: false,
                memory: vec![0; MEMORY_WORDS],
                halted: false,
                installed_origin: 0,
                installed_words: 0,
            },
        }
    }

    /// Return a complete deep-owned state snapshot.
    #[must_use]
    pub fn state(&self) -> NovaState {
        self.state.clone()
    }

    /// Restore reset state, including cleared memory and lifecycle metadata.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Restore a validated complete state atomically.
    pub fn restore(&mut self, state: &NovaState) -> Result<(), NovaError> {
        validate_state(state)?;
        self.state = state.clone();
        Ok(())
    }

    /// Load canonical transport at [`DEFAULT_ORIGIN`].
    pub fn load(&mut self, program: &[u8]) -> Result<(), NovaError> {
        self.load_at(program, DEFAULT_ORIGIN)
    }

    /// Validate and load canonical transport at an explicit word origin.
    pub fn load_at(&mut self, program: &[u8], origin: u16) -> Result<(), NovaError> {
        validate_address(u32::from(origin))?;
        let words = decode_transport(program)?;
        let end = usize::from(origin).saturating_add(words.len());
        if end > MEMORY_WORDS {
            return Err(NovaError::ProgramOutOfRange {
                origin,
                words: words.len(),
            });
        }
        let mut next = Self::new();
        next.state.memory[usize::from(origin)..end].copy_from_slice(&words);
        next.state.pc = origin;
        next.state.installed_origin = origin;
        next.state.installed_words = words.len();
        *self = next;
        Ok(())
    }

    /// Read one accumulator.
    pub fn accumulator(&self, index: u8) -> Result<u16, NovaError> {
        Ok(self.state.accumulators[usize::from(checked_accumulator(index)?)])
    }

    /// Set one accumulator.
    pub fn set_accumulator(&mut self, index: u8, value: u16) -> Result<(), NovaError> {
        let index = usize::from(checked_accumulator(index)?);
        self.state.accumulators[index] = value;
        Ok(())
    }

    /// Read one owned memory word.
    pub fn memory_word(&self, address: u16) -> Result<u16, NovaError> {
        validate_address(u32::from(address))?;
        Ok(self.state.memory[usize::from(address)])
    }

    /// Set one owned memory word.
    pub fn set_memory_word(&mut self, address: u16, value: u16) -> Result<(), NovaError> {
        validate_address(u32::from(address))?;
        self.state.memory[usize::from(address)] = value;
        Ok(())
    }

    /// Retire one instruction atomically.
    pub fn step(&mut self) -> Result<StepTrace, NovaError> {
        if self.state.halted {
            return Err(NovaError::Halted);
        }
        let mut next = self.clone();
        let trace = next.step_inner()?;
        validate_state(&next.state)?;
        *self = next;
        Ok(trace)
    }

    /// Run until HALT within a nonzero step bound, rolling back on failure.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, NovaError> {
        if max_steps == 0 {
            return Err(NovaError::ZeroStepLimit);
        }
        let before = self.clone();
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    *self = before;
                    return Err(error);
                }
            }
            if self.state.halted {
                return Ok(ExecutionResult {
                    halted: true,
                    steps: traces.len(),
                    final_state: self.state(),
                    traces,
                });
            }
        }
        *self = before;
        Err(NovaError::MaxStepsExceeded { max_steps })
    }

    /// Construct, load, and run one bounded program.
    pub fn execute(
        program: &[u8],
        origin: u16,
        max_steps: usize,
    ) -> Result<ExecutionResult, NovaError> {
        let mut cpu = Self::new();
        cpu.load_at(program, origin)?;
        cpu.run(max_steps)
    }

    fn step_inner(&mut self) -> Result<StepTrace, NovaError> {
        let pc_before = self.state.pc;
        self.validate_fetch(pc_before)?;
        let instruction = self.state.memory[usize::from(pc_before)];
        self.state.pc = self.state.pc.wrapping_add(1) & ADDRESS_MASK;

        let (mnemonic, effective_address) = if instruction & 0x8000 != 0 {
            (self.execute_operate(instruction), None)
        } else if instruction < 0o60_000 {
            let ea = self.effective_address(instruction, pc_before)?;
            (self.execute_memory_reference(instruction, ea), Some(ea))
        } else if instruction == HALT {
            self.state.halted = true;
            ("HALT".to_owned(), None)
        } else {
            return Err(NovaError::UnsupportedIo {
                instruction,
                pc: pc_before,
            });
        };

        Ok(StepTrace {
            pc_before,
            pc_after: self.state.pc,
            instruction,
            effective_address,
            mnemonic,
        })
    }

    fn validate_fetch(&self, pc: u16) -> Result<(), NovaError> {
        let start = usize::from(self.state.installed_origin);
        let end = start + self.state.installed_words;
        if usize::from(pc) < start || usize::from(pc) >= end {
            return Err(NovaError::FetchOutsideProgram { pc });
        }
        Ok(())
    }

    fn effective_address(&mut self, instruction: u16, fault_pc: u16) -> Result<u16, NovaError> {
        let displacement = (instruction & 0xff) as u8;
        let mode = (instruction >> 8) & 3;
        let mut address = match mode {
            0 => u16::from(displacement),
            1 => add_signed_displacement(self.state.pc, displacement),
            2 => add_signed_displacement(self.state.accumulators[2], displacement),
            3 => add_signed_displacement(self.state.accumulators[3], displacement),
            _ => unreachable!(),
        };
        if instruction & (1 << 10) == 0 {
            return Ok(address);
        }

        for _ in 0..INDIRECT_LIMIT {
            let word = self.state.memory[usize::from(address)];
            let another_level = word & 0x8000 != 0;
            if (0o20..=0o27).contains(&address) {
                self.state.memory[usize::from(address)] = word.wrapping_add(1);
            } else if (0o30..=0o37).contains(&address) {
                self.state.memory[usize::from(address)] = word.wrapping_sub(1);
            }
            address = self.state.memory[usize::from(address)] & ADDRESS_MASK;
            if !another_level {
                return Ok(address);
            }
        }
        Err(NovaError::IndirectAddressLimit { pc: fault_pc })
    }

    fn execute_memory_reference(&mut self, instruction: u16, ea: u16) -> String {
        let opac = (instruction >> 11) & 0x0f;
        match opac {
            0 => {
                self.state.pc = ea;
                "JMP".to_owned()
            }
            1 => {
                self.state.accumulators[3] = self.state.pc;
                self.state.pc = ea;
                "JSR".to_owned()
            }
            2 => {
                let value = self.state.memory[usize::from(ea)].wrapping_add(1);
                self.state.memory[usize::from(ea)] = value;
                if value == 0 {
                    self.state.pc = self.state.pc.wrapping_add(1) & ADDRESS_MASK;
                }
                "ISZ".to_owned()
            }
            3 => {
                let value = self.state.memory[usize::from(ea)].wrapping_sub(1);
                self.state.memory[usize::from(ea)] = value;
                if value == 0 {
                    self.state.pc = self.state.pc.wrapping_add(1) & ADDRESS_MASK;
                }
                "DSZ".to_owned()
            }
            4..=7 => {
                let ac = usize::from(opac - 4);
                self.state.accumulators[ac] = self.state.memory[usize::from(ea)];
                format!("LDA {ac}")
            }
            8..=11 => {
                let ac = usize::from(opac - 8);
                self.state.memory[usize::from(ea)] = self.state.accumulators[ac];
                format!("STA {ac}")
            }
            _ => unreachable!("I/O instructions are decoded before memory references"),
        }
    }

    fn execute_operate(&mut self, instruction: u16) -> String {
        let source = usize::from((instruction >> 13) & 3);
        let destination = usize::from((instruction >> 11) & 3);
        let alu = (instruction >> 8) & 7;
        let shift = (instruction >> 6) & 3;
        let carry_control = (instruction >> 4) & 3;
        let no_load = instruction & 8 != 0;
        let skip = instruction & 7;

        let mut value = u32::from(self.state.accumulators[source]);
        value |= match carry_control {
            0 => u32::from(self.state.carry) << 16,
            1 => 0,
            2 => CARRY_BIT,
            3 => u32::from(!self.state.carry) << 16,
            _ => unreachable!(),
        };
        let destination_value = u32::from(self.state.accumulators[destination]);
        value = match alu {
            0 => value ^ DATA_MASK,
            1 => ((value ^ DATA_MASK) + 1) & CARRY_DATA_MASK,
            2 => value,
            3 => (value + 1) & CARRY_DATA_MASK,
            4 => ((value ^ DATA_MASK) + destination_value) & CARRY_DATA_MASK,
            5 => ((value ^ DATA_MASK) + destination_value + 1) & CARRY_DATA_MASK,
            6 => (value + destination_value) & CARRY_DATA_MASK,
            7 => value & (destination_value | CARRY_BIT),
            _ => unreachable!(),
        };
        value = match shift {
            0 => value,
            1 => ((value << 1) | (value >> 16)) & CARRY_DATA_MASK,
            2 => ((value >> 1) | (value << 16)) & CARRY_DATA_MASK,
            3 => ((value & 0xff) << 8) | ((value >> 8) & 0xff) | (value & CARRY_BIT),
            _ => unreachable!(),
        };

        if should_skip(skip, value) {
            self.state.pc = self.state.pc.wrapping_add(1) & ADDRESS_MASK;
        }
        if !no_load {
            self.state.accumulators[destination] = value as u16;
            self.state.carry = value & CARRY_BIT != 0;
        }
        operate_mnemonic(
            alu,
            carry_control,
            shift,
            no_load,
            source,
            destination,
            skip,
        )
    }
}

fn decode_transport(program: &[u8]) -> Result<Vec<u16>, NovaError> {
    if !program.len().is_multiple_of(2) {
        return Err(NovaError::InvalidTransportLength {
            bytes: program.len(),
        });
    }
    Ok(program
        .chunks_exact(2)
        .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
        .collect())
}

fn validate_address(address: u32) -> Result<(), NovaError> {
    if address > u32::from(ADDRESS_MASK) {
        return Err(NovaError::AddressOutOfRange { address });
    }
    Ok(())
}

fn validate_state(state: &NovaState) -> Result<(), NovaError> {
    if state.memory.len() != MEMORY_WORDS {
        return Err(NovaError::InvalidMemoryLength {
            words: state.memory.len(),
        });
    }
    validate_address(u32::from(state.pc))?;
    let end = usize::from(state.installed_origin).saturating_add(state.installed_words);
    if state.installed_origin > ADDRESS_MASK || end > MEMORY_WORDS {
        return Err(NovaError::InvalidInstalledRange {
            origin: state.installed_origin,
            words: state.installed_words,
        });
    }
    Ok(())
}

fn checked_accumulator(index: u8) -> Result<u8, NovaError> {
    if index > 3 {
        return Err(NovaError::InvalidAccumulator { index });
    }
    Ok(index)
}

fn add_signed_displacement(base: u16, displacement: u8) -> u16 {
    let signed = i32::from(displacement as i8);
    ((i32::from(base) + signed) as u16) & ADDRESS_MASK
}

fn should_skip(selector: u16, value: u32) -> bool {
    match selector {
        0 => false,
        1 => true,
        2 => value < CARRY_BIT,
        3 => value >= CARRY_BIT,
        4 => value & DATA_MASK == 0,
        5 => value & DATA_MASK != 0,
        6 => value <= CARRY_BIT,
        7 => value > CARRY_BIT,
        _ => unreachable!(),
    }
}

#[allow(clippy::too_many_arguments)]
fn operate_mnemonic(
    alu: u16,
    carry: u16,
    shift: u16,
    no_load: bool,
    source: usize,
    destination: usize,
    skip: u16,
) -> String {
    let alu = ["COM", "NEG", "MOV", "INC", "ADC", "SUB", "ADD", "AND"][usize::from(alu)];
    let carry = ["", "Z", "O", "C"][usize::from(carry)];
    let shift = ["", "L", "R", "S"][usize::from(shift)];
    let no_load = if no_load { "#" } else { "" };
    let skip = ["", "SKP", "SZC", "SNC", "SZR", "SNR", "SEZ", "SBN"][usize::from(skip)];
    if skip.is_empty() {
        format!("{alu}{carry}{shift}{no_load} {source},{destination}")
    } else {
        format!("{alu}{carry}{shift}{no_load} {source},{destination},{skip}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(words: &[u16]) -> Vec<u8> {
        words.iter().flat_map(|word| word.to_be_bytes()).collect()
    }

    fn mref(operation: MemoryOperation, indirect: bool, mode: AddressMode, disp: u8) -> u16 {
        encode_memory_reference(operation, indirect, mode, disp).unwrap()
    }

    #[test]
    fn transport_and_public_bounds_are_checked() {
        let mut cpu = NovaSimulator::new();
        assert!(matches!(
            cpu.load(&[0]),
            Err(NovaError::InvalidTransportLength { .. })
        ));
        assert!(matches!(
            cpu.load_at(&program(&[HALT, HALT]), ADDRESS_MASK),
            Err(NovaError::ProgramOutOfRange { .. })
        ));
        assert!(matches!(
            cpu.accumulator(4),
            Err(NovaError::InvalidAccumulator { .. })
        ));
        cpu.load(&program(&[HALT])).unwrap();
        let before = cpu.state();
        assert!(cpu.set_memory_word(0xffff, 0).is_err());
        assert_eq!(cpu.state(), before);
    }

    #[test]
    fn memory_reference_data_and_control_execute() {
        let words = [
            mref(MemoryOperation::Lda(0), false, AddressMode::PageZero, 0o20),
            mref(MemoryOperation::Sta(0), false, AddressMode::PageZero, 0o21),
            mref(MemoryOperation::Isz, false, AddressMode::PageZero, 0o22),
            HALT,
            HALT,
        ];
        let mut cpu = NovaSimulator::new();
        cpu.load(&program(&words)).unwrap();
        cpu.set_memory_word(0o20, 0o12345).unwrap();
        cpu.set_memory_word(0o22, 0xffff).unwrap();
        let result = cpu.run(4).unwrap();
        assert_eq!(result.final_state.accumulators[0], 0o12345);
        assert_eq!(result.final_state.memory[0o21], 0o12345);
        assert_eq!(result.final_state.memory[0o22], 0);
        assert_eq!(result.traces.last().unwrap().pc_before, DEFAULT_ORIGIN + 4);
    }

    #[test]
    fn addressing_modes_and_indirection_are_exact() {
        let instruction = mref(
            MemoryOperation::Lda(1),
            true,
            AddressMode::Ac2Relative,
            0xff,
        );
        let mut cpu = NovaSimulator::new();
        cpu.load(&program(&[instruction, HALT])).unwrap();
        cpu.set_accumulator(2, 0o31).unwrap();
        cpu.set_memory_word(0o30, 0x8000 | 0o21).unwrap();
        cpu.set_memory_word(0o20, 0o400).unwrap();
        cpu.set_memory_word(0o401, 0o7654).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.accumulator(1).unwrap(), 0o7654);
        assert_eq!(cpu.memory_word(0o30).unwrap(), 0x8000 | 0o20);
        assert_eq!(cpu.memory_word(0o20).unwrap(), 0o401);
    }

    #[test]
    fn operate_fields_update_or_preserve_state() {
        let add = encode_operate(
            0,
            1,
            AluOperation::Add,
            ShiftOperation::Left,
            CarryControl::Clear,
            false,
            SkipCondition::ResultNonzero,
        )
        .unwrap();
        let probe = encode_operate(
            0,
            1,
            AluOperation::Move,
            ShiftOperation::None,
            CarryControl::Set,
            true,
            SkipCondition::BothNonzero,
        )
        .unwrap();
        let mut cpu = NovaSimulator::new();
        cpu.load(&program(&[add, probe, HALT, HALT])).unwrap();
        cpu.set_accumulator(0, 1).unwrap();
        cpu.set_accumulator(1, 2).unwrap();
        let trace = cpu.step().unwrap();
        assert_eq!(cpu.accumulator(1).unwrap(), 6);
        assert_eq!(trace.pc_after, DEFAULT_ORIGIN + 2);
        let before = cpu.state();
        cpu.step().unwrap();
        assert_eq!(cpu.accumulator(1).unwrap(), before.accumulators[1]);
        assert_eq!(cpu.state().carry, before.carry);
    }

    #[test]
    fn jsr_and_halt_have_checked_lifecycle() {
        let jsr = mref(MemoryOperation::Jsr, false, AddressMode::PageZero, 0o205);
        let mut cpu = NovaSimulator::new();
        cpu.load(&program(&[jsr, HALT, 0, 0, 0, HALT])).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.accumulator(3).unwrap(), DEFAULT_ORIGIN + 1);
        assert_eq!(cpu.state().pc, 0o205);
        let halt = cpu.step().unwrap();
        assert_eq!(halt.mnemonic, "HALT");
        assert_eq!(halt.pc_after, 0o206);
        let before = cpu.state();
        assert_eq!(cpu.step(), Err(NovaError::Halted));
        assert_eq!(cpu.state(), before);
    }

    #[test]
    fn failed_run_rolls_back_complete_state() {
        let mut cpu = NovaSimulator::new();
        cpu.load(&program(&[mref(
            MemoryOperation::Jmp,
            false,
            AddressMode::PageZero,
            DEFAULT_ORIGIN as u8,
        )]))
        .unwrap();
        let before = cpu.state();
        assert_eq!(
            cpu.run(2),
            Err(NovaError::MaxStepsExceeded { max_steps: 2 })
        );
        assert_eq!(cpu.state(), before);
    }

    #[test]
    fn every_operate_field_combination_matches_the_field_reference() {
        for source in 0..4_u8 {
            for destination in 0..4_u8 {
                for alu in 0..8_u16 {
                    for shift in 0..4_u16 {
                        for carry_control in 0..4_u16 {
                            for no_load in [false, true] {
                                for skip in 0..8_u16 {
                                    let instruction = 0x8000
                                        | (u16::from(source) << 13)
                                        | (u16::from(destination) << 11)
                                        | (alu << 8)
                                        | (shift << 6)
                                        | (carry_control << 4)
                                        | (u16::from(no_load) << 3)
                                        | skip;
                                    let mut cpu = NovaSimulator::new();
                                    cpu.state.pc = 0o1234;
                                    cpu.state.carry = source & 1 != 0;
                                    cpu.state.accumulators = [0x0000, 0x0001, 0x8000, 0xffff];
                                    let before = cpu.state.clone();
                                    let (expected_value, expected_skip) = reference_operate(
                                        before.accumulators[usize::from(source)],
                                        before.accumulators[usize::from(destination)],
                                        before.carry,
                                        alu,
                                        shift,
                                        carry_control,
                                        skip,
                                    );
                                    cpu.execute_operate(instruction);
                                    let expected_pc = if expected_skip { 0o1235 } else { 0o1234 };
                                    assert_eq!(
                                        cpu.state.pc, expected_pc,
                                        "word {instruction:#07o}"
                                    );
                                    if no_load {
                                        assert_eq!(
                                            cpu.state.accumulators, before.accumulators,
                                            "word {instruction:#07o}"
                                        );
                                        assert_eq!(
                                            cpu.state.carry, before.carry,
                                            "word {instruction:#07o}"
                                        );
                                    } else {
                                        assert_eq!(
                                            cpu.state.accumulators[usize::from(destination)],
                                            expected_value as u16,
                                            "word {instruction:#07o}"
                                        );
                                        assert_eq!(
                                            cpu.state.carry,
                                            expected_value & CARRY_BIT != 0,
                                            "word {instruction:#07o}"
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn reference_operate(
        source: u16,
        destination: u16,
        old_carry: bool,
        alu: u16,
        shift: u16,
        carry_control: u16,
        skip: u16,
    ) -> (u32, bool) {
        let selected_carry = match carry_control {
            0 => old_carry,
            1 => false,
            2 => true,
            3 => !old_carry,
            _ => unreachable!(),
        };
        let mut value = u32::from(source) | (u32::from(selected_carry) << 16);
        let destination = u32::from(destination);
        value = match alu {
            0 => value ^ 0xffff,
            1 => (!value & 0xffff | (value & CARRY_BIT)).wrapping_add(1) & 0x1ffff,
            2 => value,
            3 => value.wrapping_add(1) & 0x1ffff,
            4 => ((!value & 0xffff) | (value & CARRY_BIT)).wrapping_add(destination) & 0x1ffff,
            5 => {
                ((!value & 0xffff) | (value & CARRY_BIT))
                    .wrapping_add(destination)
                    .wrapping_add(1)
                    & 0x1ffff
            }
            6 => value.wrapping_add(destination) & 0x1ffff,
            7 => value & (destination | CARRY_BIT),
            _ => unreachable!(),
        };
        value = match shift {
            0 => value,
            1 => ((value << 1) | ((value >> 16) & 1)) & 0x1ffff,
            2 => ((value >> 1) | ((value & 1) << 16)) & 0x1ffff,
            3 => (value & CARRY_BIT) | ((value & 0xff) << 8) | ((value >> 8) & 0xff),
            _ => unreachable!(),
        };
        let carry = value & CARRY_BIT != 0;
        let zero = value & 0xffff == 0;
        let should_skip = match skip {
            0 => false,
            1 => true,
            2 => !carry,
            3 => carry,
            4 => zero,
            5 => !zero,
            6 => !carry || zero,
            7 => carry && !zero,
            _ => unreachable!(),
        };
        (value, should_skip)
    }
}
