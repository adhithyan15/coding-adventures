//! Functional simulator for the DEC PDP-8/E base processor.
//!
//! The model owns one 4K field of 12-bit core memory, the accumulator, link,
//! program counter, multiplier quotient, switch register, interrupt controls,
//! and halt state. Optional memory extension, EAE, and peripheral hardware are
//! explicit boundaries rather than host-language approximations.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Number of 12-bit words in one PDP-8 memory field.
pub const MEMORY_WORDS: usize = 4_096;
/// Mask for every architectural word and address.
pub const WORD_MASK: u16 = 0o7777;
const PAGE_MASK: u16 = 0o7600;
const OFFSET_MASK: u16 = 0o0177;
const AUTO_INDEX_START: u16 = 0o0010;
const AUTO_INDEX_END: u16 = 0o0017;

/// Canonical Group 2 halt instruction.
pub const HLT: u16 = 0o7402;
/// Canonical no-operation instruction.
pub const NOP: u16 = 0o7000;

/// The six memory-reference opcode families.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum Opcode {
    And = 0,
    Tad = 1,
    Isz = 2,
    Dca = 3,
    Jms = 4,
    Jmp = 5,
}

impl Opcode {
    const fn mnemonic(self) -> &'static str {
        match self {
            Self::And => "AND",
            Self::Tad => "TAD",
            Self::Isz => "ISZ",
            Self::Dca => "DCA",
            Self::Jms => "JMS",
            Self::Jmp => "JMP",
        }
    }
}

/// Encode one PDP-8 memory-reference instruction.
pub const fn encode_memory_reference(
    opcode: Opcode,
    indirect: bool,
    current_page: bool,
    offset: u8,
) -> u16 {
    ((opcode as u16) << 9)
        | ((indirect as u16) << 8)
        | ((current_page as u16) << 7)
        | ((offset as u16) & OFFSET_MASK)
}

/// Encode an I/O-transfer instruction from its six-bit device and three pulses.
pub const fn encode_iot(device: u8, pulses: u8) -> u16 {
    0o6000 | (((device as u16) & 0o77) << 3) | ((pulses as u16) & 0o7)
}

/// An IOT asserted on the CPU's external device-select and I/O-pulse wires.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IotEvent {
    pub device: u8,
    pub pulses: u8,
}

/// Complete owned architectural and lifecycle state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pdp8State {
    pub memory: Box<[u16; MEMORY_WORDS]>,
    pub ac: u16,
    pub link: bool,
    pub pc: u16,
    pub mq: u16,
    pub switch_register: u16,
    pub interrupt_enable: bool,
    pub interrupt_pending: bool,
    /// One means that one more instruction must retire before interrupt entry.
    pub interrupt_delay: u8,
    pub halted: bool,
    pub loaded_origin: u16,
    pub loaded_words: usize,
}

/// One processor or interrupt cycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    /// `None` identifies an interrupt-entry cycle rather than an instruction.
    pub instruction: Option<u16>,
    pub effective_address: Option<u16>,
    pub mnemonic: String,
    pub description: String,
    pub iot: Option<IotEvent>,
}

/// Result of a bounded run that reached HLT.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub final_state: Pdp8State,
    pub traces: Vec<StepTrace>,
}

/// Fail-closed lifecycle and decode errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Pdp8Error {
    InvalidOrigin { origin: usize },
    ProgramTooLarge { words: usize, capacity: usize },
    OddProgramLength { bytes: usize },
    InvalidWord { word: u16, index: usize },
    InvalidAddress { address: usize },
    InvalidInterruptDelay { delay: u8 },
    Halted,
    InvalidOperate { instruction: u16 },
    UnsupportedEae { instruction: u16 },
    UnsupportedCpuIot { instruction: u16 },
    MaxStepsExceeded { max_steps: usize },
}

impl Display for Pdp8Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOrigin { origin } => {
                write!(
                    formatter,
                    "word origin {origin:#o} is outside PDP-8 field 0"
                )
            }
            Self::ProgramTooLarge { words, capacity } => write!(
                formatter,
                "program has {words} words but the selected origin holds {capacity}"
            ),
            Self::OddProgramLength { bytes } => {
                write!(formatter, "program transport has odd byte length {bytes}")
            }
            Self::InvalidWord { word, index } => write!(
                formatter,
                "program word {index} is {word:#06o}, outside the 12-bit range"
            ),
            Self::InvalidAddress { address } => {
                write!(
                    formatter,
                    "word address {address:#o} is outside PDP-8 field 0"
                )
            }
            Self::InvalidInterruptDelay { delay } => {
                write!(formatter, "interrupt delay {delay} is outside 0..=1")
            }
            Self::Halted => write!(formatter, "the PDP-8 is halted"),
            Self::InvalidOperate { instruction } => {
                write!(formatter, "invalid OPR combination {instruction:#06o}")
            }
            Self::UnsupportedEae { instruction } => write!(
                formatter,
                "OPR Group 3 instruction {instruction:#06o} requires the optional KE8-E EAE"
            ),
            Self::UnsupportedCpuIot { instruction } => write!(
                formatter,
                "CPU IOT {instruction:#06o} requires unmodeled extended flags"
            ),
            Self::MaxStepsExceeded { max_steps } => {
                write!(
                    formatter,
                    "maximum execution step count {max_steps} exceeded"
                )
            }
        }
    }
}

impl Error for Pdp8Error {}

/// Instruction-level PDP-8/E base-processor simulator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pdp8Simulator {
    state: Pdp8State,
}

impl Default for Pdp8Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Pdp8Simulator {
    /// Construct the documented clear processor state with a zeroed 4K field.
    pub fn new() -> Self {
        Self {
            state: Pdp8State {
                memory: Box::new([0; MEMORY_WORDS]),
                ac: 0,
                link: false,
                pc: 0,
                mq: 0,
                switch_register: 0,
                interrupt_enable: false,
                interrupt_pending: false,
                interrupt_delay: 0,
                halted: false,
                loaded_origin: 0,
                loaded_words: 0,
            },
        }
    }

    /// Restore the complete clear processor state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Decode strict little-endian 16-bit containers carrying 12-bit words.
    pub fn load(&mut self, program: &[u8], origin: usize) -> Result<usize, Pdp8Error> {
        if !program.len().is_multiple_of(2) {
            return Err(Pdp8Error::OddProgramLength {
                bytes: program.len(),
            });
        }
        let words = program
            .as_chunks::<2>()
            .0
            .iter()
            .map(|bytes| u16::from_le_bytes(*bytes))
            .collect::<Vec<_>>();
        self.load_words(&words, origin)
    }

    /// Replace the machine with a clean state and install 12-bit words.
    pub fn load_words(&mut self, program: &[u16], origin: usize) -> Result<usize, Pdp8Error> {
        if origin >= MEMORY_WORDS {
            return Err(Pdp8Error::InvalidOrigin { origin });
        }
        let capacity = MEMORY_WORDS - origin;
        if program.len() > capacity {
            return Err(Pdp8Error::ProgramTooLarge {
                words: program.len(),
                capacity,
            });
        }
        if let Some((index, word)) = program
            .iter()
            .copied()
            .enumerate()
            .find(|(_, word)| *word > WORD_MASK)
        {
            return Err(Pdp8Error::InvalidWord { word, index });
        }

        let mut replacement = Self::new();
        replacement.state.memory[origin..origin + program.len()].copy_from_slice(program);
        replacement.state.pc = origin as u16;
        replacement.state.loaded_origin = origin as u16;
        replacement.state.loaded_words = program.len();
        *self = replacement;
        Ok(program.len())
    }

    /// Restore a validated complete snapshot.
    pub fn restore(&mut self, state: &Pdp8State) -> Result<(), Pdp8Error> {
        for (index, word) in state.memory.iter().copied().enumerate() {
            if word > WORD_MASK {
                return Err(Pdp8Error::InvalidWord { word, index });
            }
        }
        for (index, word) in [state.ac, state.pc, state.mq, state.switch_register]
            .into_iter()
            .enumerate()
        {
            if word > WORD_MASK {
                return Err(Pdp8Error::InvalidWord { word, index });
            }
        }
        if state.interrupt_delay > 1 {
            return Err(Pdp8Error::InvalidInterruptDelay {
                delay: state.interrupt_delay,
            });
        }
        let origin = usize::from(state.loaded_origin);
        if origin >= MEMORY_WORDS {
            return Err(Pdp8Error::InvalidOrigin { origin });
        }
        if state.loaded_words > MEMORY_WORDS - origin {
            return Err(Pdp8Error::ProgramTooLarge {
                words: state.loaded_words,
                capacity: MEMORY_WORDS - origin,
            });
        }
        self.state = state.clone();
        Ok(())
    }

    /// Return a deep, owned snapshot.
    pub fn state(&self) -> Pdp8State {
        self.state.clone()
    }

    pub fn get_state(&self) -> Pdp8State {
        self.state()
    }

    pub fn read_memory(&self, address: usize) -> Result<u16, Pdp8Error> {
        self.state
            .memory
            .get(address)
            .copied()
            .ok_or(Pdp8Error::InvalidAddress { address })
    }

    pub fn write_memory(&mut self, address: usize, word: u16) -> Result<(), Pdp8Error> {
        if word > WORD_MASK {
            return Err(Pdp8Error::InvalidWord {
                word,
                index: address,
            });
        }
        let destination = self
            .state
            .memory
            .get_mut(address)
            .ok_or(Pdp8Error::InvalidAddress { address })?;
        *destination = word;
        Ok(())
    }

    pub fn set_accumulator(&mut self, value: u16) -> Result<(), Pdp8Error> {
        validate_word(value, 0)?;
        self.state.ac = value;
        Ok(())
    }

    pub fn set_link(&mut self, value: bool) {
        self.state.link = value;
    }

    pub fn set_program_counter(&mut self, value: u16) -> Result<(), Pdp8Error> {
        validate_word(value, 0)?;
        self.state.pc = value;
        Ok(())
    }

    pub fn set_multiplier_quotient(&mut self, value: u16) -> Result<(), Pdp8Error> {
        validate_word(value, 0)?;
        self.state.mq = value;
        Ok(())
    }

    pub fn set_switch_register(&mut self, value: u16) -> Result<(), Pdp8Error> {
        validate_word(value, 0)?;
        self.state.switch_register = value;
        Ok(())
    }

    /// Assert or release the aggregate peripheral interrupt-request wire.
    pub fn set_interrupt_request(&mut self, pending: bool) {
        self.state.interrupt_pending = pending;
    }

    /// Execute one transactional processor cycle.
    pub fn step(&mut self) -> Result<StepTrace, Pdp8Error> {
        if self.state.halted {
            return Err(Pdp8Error::Halted);
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

    /// Run until HLT, subject to a mandatory caller-provided cycle bound.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Pdp8Error> {
        let mut traces = Vec::new();
        while !self.state.halted {
            if traces.len() == max_steps {
                return Err(Pdp8Error::MaxStepsExceeded { max_steps });
            }
            traces.push(self.step()?);
        }
        Ok(ExecutionResult {
            halted: true,
            steps: traces.len(),
            final_state: self.state(),
            traces,
        })
    }

    /// Reset, load at `origin`, and execute until HLT or the cycle bound.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: usize,
        max_steps: usize,
    ) -> Result<ExecutionResult, Pdp8Error> {
        self.load(program, origin)?;
        self.run(max_steps)
    }

    fn step_inner(&mut self) -> Result<StepTrace, Pdp8Error> {
        if self.state.interrupt_enable
            && self.state.interrupt_pending
            && self.state.interrupt_delay == 0
        {
            let pc_before = self.state.pc;
            self.state.memory[0] = self.state.pc;
            self.state.pc = 1;
            self.state.interrupt_enable = false;
            self.state.interrupt_pending = false;
            return Ok(StepTrace {
                pc_before,
                pc_after: 1,
                instruction: None,
                effective_address: Some(0),
                mnemonic: "INTERRUPT".to_owned(),
                description: format!("INTERRUPT: M[0000] <- {pc_before:04o}; PC <- 0001"),
                iot: None,
            });
        }

        let pc_before = self.state.pc;
        let instruction = self.state.memory[usize::from(pc_before)];
        self.state.pc = mask_word(self.state.pc.wrapping_add(1));
        let opcode = instruction >> 9;
        let (mnemonic, effective_address, iot) = match opcode {
            0..=5 => {
                let opcode = match opcode {
                    0 => Opcode::And,
                    1 => Opcode::Tad,
                    2 => Opcode::Isz,
                    3 => Opcode::Dca,
                    4 => Opcode::Jms,
                    5 => Opcode::Jmp,
                    _ => unreachable!(),
                };
                let address = self.resolve_effective_address(instruction);
                let mnemonic = self.execute_memory_reference(opcode, address);
                (mnemonic, Some(address), None)
            }
            6 => {
                let (mnemonic, event) = self.execute_iot(instruction)?;
                (mnemonic, None, event)
            }
            7 => (self.execute_operate(instruction)?, None, None),
            _ => unreachable!("a three-bit opcode always decodes"),
        };

        if self.state.interrupt_delay > 0 {
            self.state.interrupt_delay -= 1;
        }
        let pc_after = self.state.pc;
        Ok(StepTrace {
            pc_before,
            pc_after,
            instruction: Some(instruction),
            effective_address,
            description: format!("{mnemonic} @ {pc_before:04o}"),
            mnemonic,
            iot,
        })
    }

    fn resolve_effective_address(&mut self, instruction: u16) -> u16 {
        let offset = instruction & OFFSET_MASK;
        let mut address = if instruction & 0o0200 != 0 {
            (self.state.pc & PAGE_MASK) | offset
        } else {
            offset
        };
        if instruction & 0o0400 != 0 {
            if (AUTO_INDEX_START..=AUTO_INDEX_END).contains(&address) {
                self.state.memory[usize::from(address)] =
                    mask_word(self.state.memory[usize::from(address)].wrapping_add(1));
            }
            address = self.state.memory[usize::from(address)];
        }
        address
    }

    fn execute_memory_reference(&mut self, opcode: Opcode, address: u16) -> String {
        let index = usize::from(address);
        match opcode {
            Opcode::And => self.state.ac &= self.state.memory[index],
            Opcode::Tad => {
                let sum = u32::from(self.state.ac) + u32::from(self.state.memory[index]);
                self.state.ac = (sum as u16) & WORD_MASK;
                if sum > u32::from(WORD_MASK) {
                    self.state.link = !self.state.link;
                }
            }
            Opcode::Isz => {
                self.state.memory[index] = mask_word(self.state.memory[index].wrapping_add(1));
                if self.state.memory[index] == 0 {
                    self.state.pc = mask_word(self.state.pc.wrapping_add(1));
                }
            }
            Opcode::Dca => {
                self.state.memory[index] = self.state.ac;
                self.state.ac = 0;
            }
            Opcode::Jms => {
                self.state.memory[index] = self.state.pc;
                self.state.pc = mask_word(address.wrapping_add(1));
            }
            Opcode::Jmp => self.state.pc = address,
        }
        format!("{} {address:04o}", opcode.mnemonic())
    }

    fn execute_iot(&mut self, instruction: u16) -> Result<(String, Option<IotEvent>), Pdp8Error> {
        let device = ((instruction >> 3) & 0o77) as u8;
        let pulses = (instruction & 0o7) as u8;
        if device != 0 {
            let event = IotEvent { device, pulses };
            return Ok((format!("IOT {device:02o},{pulses:o}"), Some(event)));
        }

        let mnemonic = match pulses {
            0 => {
                if self.state.interrupt_enable {
                    self.state.pc = mask_word(self.state.pc.wrapping_add(1));
                }
                self.state.interrupt_enable = false;
                self.state.interrupt_delay = 0;
                "SKON"
            }
            1 => {
                self.state.interrupt_enable = true;
                // The generic post-retirement decrement leaves one full
                // instruction of the documented ION inhibit interval.
                self.state.interrupt_delay = 2;
                "ION"
            }
            2 => {
                self.state.interrupt_enable = false;
                self.state.interrupt_delay = 0;
                "IOF"
            }
            3 => {
                if self.state.interrupt_pending {
                    self.state.pc = mask_word(self.state.pc.wrapping_add(1));
                }
                "SRQ"
            }
            7 => {
                self.state.interrupt_enable = false;
                self.state.interrupt_pending = false;
                self.state.interrupt_delay = 0;
                "CAF"
            }
            _ => return Err(Pdp8Error::UnsupportedCpuIot { instruction }),
        };
        Ok((mnemonic.to_owned(), None))
    }

    fn execute_operate(&mut self, instruction: u16) -> Result<String, Pdp8Error> {
        if instruction & 0o0400 == 0 {
            self.execute_group_one(instruction)
        } else if instruction & 1 == 0 {
            self.execute_group_two(instruction)
        } else {
            Err(Pdp8Error::UnsupportedEae { instruction })
        }
    }

    fn execute_group_one(&mut self, instruction: u16) -> Result<String, Pdp8Error> {
        let rotate_right = instruction & 0o0010 != 0;
        let rotate_left = instruction & 0o0004 != 0;
        if rotate_right && rotate_left {
            return Err(Pdp8Error::InvalidOperate { instruction });
        }

        let mut operations = Vec::new();
        if instruction & 0o0200 != 0 {
            self.state.ac = 0;
            operations.push("CLA");
        }
        if instruction & 0o0100 != 0 {
            self.state.link = false;
            operations.push("CLL");
        }
        if instruction & 0o0040 != 0 {
            self.state.ac ^= WORD_MASK;
            operations.push("CMA");
        }
        if instruction & 0o0020 != 0 {
            self.state.link = !self.state.link;
            operations.push("CML");
        }
        if instruction & 0o0001 != 0 {
            let combined = self.link_ac().wrapping_add(1) & 0x1fff;
            self.set_link_ac(combined);
            operations.push("IAC");
        }

        let twice_or_bsw = instruction & 0o0002 != 0;
        if rotate_right {
            let count = if twice_or_bsw { 2 } else { 1 };
            for _ in 0..count {
                let combined = self.link_ac();
                self.set_link_ac((combined >> 1) | ((combined & 1) << 12));
            }
            operations.push(if count == 2 { "RTR" } else { "RAR" });
        } else if rotate_left {
            let count = if twice_or_bsw { 2 } else { 1 };
            for _ in 0..count {
                let combined = self.link_ac();
                self.set_link_ac(((combined << 1) & 0x1fff) | (combined >> 12));
            }
            operations.push(if count == 2 { "RTL" } else { "RAL" });
        } else if twice_or_bsw {
            self.state.ac = ((self.state.ac & 0o0077) << 6) | (self.state.ac >> 6);
            operations.push("BSW");
        }

        Ok(join_operations(&operations))
    }

    fn execute_group_two(&mut self, instruction: u16) -> Result<String, Pdp8Error> {
        let mut operations = Vec::new();
        let mut selected_condition = false;
        if instruction & 0o0100 != 0 {
            selected_condition |= self.state.ac & 0o4000 != 0;
            operations.push("SMA");
        }
        if instruction & 0o0040 != 0 {
            selected_condition |= self.state.ac == 0;
            operations.push("SZA");
        }
        if instruction & 0o0020 != 0 {
            selected_condition |= self.state.link;
            operations.push("SNL");
        }
        let reverse = instruction & 0o0010 != 0;
        if reverse {
            operations.push("RSS");
        }
        if if reverse {
            !selected_condition
        } else {
            selected_condition
        } {
            self.state.pc = mask_word(self.state.pc.wrapping_add(1));
        }
        if instruction & 0o0200 != 0 {
            self.state.ac = 0;
            operations.push("CLA");
        }
        if instruction & 0o0004 != 0 {
            self.state.ac |= self.state.switch_register;
            operations.push("OSR");
        }
        if instruction & 0o0002 != 0 {
            self.state.halted = true;
            operations.push("HLT");
        }
        Ok(join_operations(&operations))
    }

    fn link_ac(&self) -> u16 {
        (u16::from(self.state.link) << 12) | self.state.ac
    }

    fn set_link_ac(&mut self, combined: u16) {
        self.state.link = combined & 0x1000 != 0;
        self.state.ac = combined & WORD_MASK;
    }
}

fn join_operations(operations: &[&str]) -> String {
    if operations.is_empty() {
        "NOP".to_owned()
    } else {
        operations.join(" ")
    }
}

const fn mask_word(value: u16) -> u16 {
    value & WORD_MASK
}

fn validate_word(word: u16, index: usize) -> Result<(), Pdp8Error> {
    if word > WORD_MASK {
        Err(Pdp8Error::InvalidWord { word, index })
    } else {
        Ok(())
    }
}
