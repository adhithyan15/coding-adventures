//! Functional simulator for the Manchester Baby (SSEM), the machine that ran
//! the first stored program on 21 June 1948.
//!
//! The complete architectural state is just a 32-word Williams-tube store, a
//! 32-bit accumulator, and a five-bit control-instruction counter. The counter
//! is unusual: hardware increments it *before* fetching an instruction. It is
//! therefore reset to 31 so the first fetch comes from line 0.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Number of 32-bit lines in the Williams-tube store.
pub const STORE_WORDS: usize = 32;

const CI_MASK: u8 = 0x1f;
const INITIAL_CI: u8 = CI_MASK;

/// The three-bit function field in bits 13 through 15 of an instruction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Function {
    Jump = 0b000,
    JumpRelative = 0b001,
    LoadNegative = 0b010,
    Store = 0b011,
    Subtract = 0b100,
    AlternateSubtract = 0b101,
    Compare = 0b110,
    Stop = 0b111,
}

impl Function {
    fn decode(instruction: u32) -> Self {
        match (instruction >> 13) & 0b111 {
            0b000 => Self::Jump,
            0b001 => Self::JumpRelative,
            0b010 => Self::LoadNegative,
            0b011 => Self::Store,
            0b100 => Self::Subtract,
            0b101 => Self::AlternateSubtract,
            0b110 => Self::Compare,
            0b111 => Self::Stop,
            _ => unreachable!("a three-bit field always decodes"),
        }
    }
}

/// Encode the architecturally meaningful fields of a Baby instruction.
///
/// The operand is masked to the five-bit store-address range. Compare and stop
/// ignore the operand in hardware, but retaining it here accurately reflects
/// the instruction word's layout.
pub const fn encode_instruction(function: Function, operand: u8) -> u32 {
    ((function as u32) << 13) | ((operand as u32) & 0x1f)
}

/// An owned snapshot of the Baby's complete architectural state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BabyState {
    pub store: [u32; STORE_WORDS],
    pub accumulator: u32,
    pub ci: u8,
    pub halted: bool,
}

impl BabyState {
    /// Interpret the accumulator as a signed two's-complement value.
    pub fn accumulator_signed(&self) -> i32 {
        self.accumulator as i32
    }

    /// Return the word at the current control-instruction line.
    pub fn present_instruction(&self) -> u32 {
        self.store[self.ci as usize]
    }
}

/// One fetch-decode-execute cycle, suitable for a debugger or teaching UI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    /// CI before the machine's mandatory pre-increment.
    pub pc_before: u8,
    /// CI after the instruction's effects.
    pub pc_after: u8,
    pub instruction: u32,
    pub mnemonic: String,
    pub description: String,
}

/// Result of a clean, bounded run that reached STP.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub final_state: BabyState,
    pub traces: Vec<StepTrace>,
}

/// Fail-closed errors for lifecycle operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BabyError {
    Halted,
    InvalidOrigin { origin: usize },
    MaxStepsExceeded { max_steps: usize },
}

impl Display for BabyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Halted => write!(formatter, "the Manchester Baby is halted"),
            Self::InvalidOrigin { origin } => write!(
                formatter,
                "store origin {origin} is outside the 0..{STORE_WORDS} range"
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

impl Error for BabyError {}

/// Instruction-level Manchester Baby simulator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BabySimulator {
    store: [u32; STORE_WORDS],
    accumulator: u32,
    ci: u8,
    halted: bool,
}

impl Default for BabySimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl BabySimulator {
    /// Construct a machine in its documented power-on state.
    pub const fn new() -> Self {
        Self {
            store: [0; STORE_WORDS],
            accumulator: 0,
            ci: INITIAL_CI,
            halted: false,
        }
    }

    /// Clear the store and restore all registers to their power-on values.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Load complete little-endian words at a word-addressed origin.
    ///
    /// An incomplete trailing word is ignored, and input beyond line 31 is not
    /// written. The returned count is the number of words actually loaded.
    pub fn load(&mut self, program: &[u8], origin: usize) -> Result<usize, BabyError> {
        if origin >= STORE_WORDS {
            return Err(BabyError::InvalidOrigin { origin });
        }

        let mut loaded = 0;
        for (offset, bytes) in program
            .chunks_exact(4)
            .take(STORE_WORDS - origin)
            .enumerate()
        {
            self.store[origin + offset] =
                u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            loaded += 1;
        }
        Ok(loaded)
    }

    /// Execute one pre-increment, fetch, decode, and execute cycle.
    pub fn step(&mut self) -> Result<StepTrace, BabyError> {
        if self.halted {
            return Err(BabyError::Halted);
        }

        let ci_before = self.ci;
        self.ci = self.ci.wrapping_add(1) & CI_MASK;
        let fetched_line = self.ci;
        let instruction = self.store[fetched_line as usize];
        let operand = (instruction & 0x1f) as u8;
        let function = Function::decode(instruction);

        let mnemonic = self.execute_instruction(function, operand);
        Ok(StepTrace {
            pc_before: ci_before,
            pc_after: self.ci,
            instruction,
            description: format!("{mnemonic} @ line {fetched_line}"),
            mnemonic,
        })
    }

    /// Run an already-loaded machine until STP, subject to a mandatory bound.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, BabyError> {
        let mut traces = Vec::new();

        while !self.halted {
            if traces.len() == max_steps {
                return Err(BabyError::MaxStepsExceeded { max_steps });
            }
            traces.push(self.step()?);
        }

        Ok(ExecutionResult {
            halted: true,
            steps: traces.len(),
            final_state: self.get_state(),
            traces,
        })
    }

    /// Reset, load at line 0, and execute a program until STP or the step bound.
    pub fn execute(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, BabyError> {
        self.reset();
        self.load(program, 0)?;
        self.run(max_steps)
    }

    /// Return an owned snapshot that cannot alias later machine mutations.
    pub fn get_state(&self) -> BabyState {
        BabyState {
            store: self.store,
            accumulator: self.accumulator,
            ci: self.ci,
            halted: self.halted,
        }
    }

    fn execute_instruction(&mut self, function: Function, operand: u8) -> String {
        let address = operand as usize;
        match function {
            Function::Jump => {
                self.ci = (self.store[address] as u8) & CI_MASK;
                format!("JMP {operand}")
            }
            Function::JumpRelative => {
                self.ci = ((self.ci as u32).wrapping_add(self.store[address]) as u8) & CI_MASK;
                format!("JRP {operand}")
            }
            Function::LoadNegative => {
                self.accumulator = 0_u32.wrapping_sub(self.store[address]);
                format!("LDN {operand}")
            }
            Function::Store => {
                self.store[address] = self.accumulator;
                format!("STO {operand}")
            }
            Function::Subtract | Function::AlternateSubtract => {
                self.accumulator = self.accumulator.wrapping_sub(self.store[address]);
                format!("SUB {operand}")
            }
            Function::Compare => {
                if self.accumulator & 0x8000_0000 != 0 {
                    self.ci = self.ci.wrapping_add(1) & CI_MASK;
                }
                "CMP".to_owned()
            }
            Function::Stop => {
                self.halted = true;
                "STP".to_owned()
            }
        }
    }
}
