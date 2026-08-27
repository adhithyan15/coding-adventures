//! Functional IBM 704 simulator.
//!
//! The machine has 32K 36-bit words, a 38-bit sign-magnitude accumulator,
//! the 36-bit MQ register, three 15-bit index registers, and a 15-bit PC.
//! Programs use the canonical five-byte big-endian transport supplied by
//! [`ibm704_encoder`].

use ibm704_encoder::{unpack_words, DecodeError, ADDR_MASK, BYTES_PER_WORD, SIGN_BIT, WORD_MASK};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const MEMORY_WORDS: usize = 32_768;
pub const MAGNITUDE_MASK: u64 = SIGN_BIT - 1;
const TAG_MASK: u64 = 0b111;
const FP_FRAC_BITS: i32 = 27;
const FP_FRAC_MASK: u64 = (1 << FP_FRAC_BITS) - 1;
const FP_CHAR_SHIFT: u32 = 27;
const FP_EXCESS: i32 = 128;

pub const OP_HTR: u16 = 0x000;
pub const OP_HPR: u16 = 0x110;
pub const OP_NOP: u16 = 0x1f1;
pub const OP_CLA: u16 = 0x140;
pub const OP_CAL: u16 = 0x940;
pub const OP_ADD: u16 = 0x100;
pub const OP_SUB: u16 = 0x102;
pub const OP_ADM: u16 = 0x101;
pub const OP_STO: u16 = 0x181;
pub const OP_STZ: u16 = 0x180;
pub const OP_STQ: u16 = 0x980;
pub const OP_LDQ: u16 = 0x170;
pub const OP_XCA: u16 = 0x059;
pub const OP_MPY: u16 = 0x080;
pub const OP_DVP: u16 = 0x091;
pub const OP_DVH: u16 = 0x090;
pub const OP_TRA: u16 = 0x010;
pub const OP_TZE: u16 = 0x040;
pub const OP_TNZ: u16 = 0x840;
pub const OP_TPL: u16 = 0x050;
pub const OP_TMI: u16 = 0x850;
pub const OP_TOV: u16 = 0x060;
pub const OP_TNO: u16 = 0x860;
pub const OP_TQO: u16 = 0x071;
pub const OP_TQP: u16 = 0x072;
pub const OP_LXA: u16 = 0x15c;
pub const OP_LXD: u16 = 0x95c;
pub const OP_SXA: u16 = 0x19c;
pub const OP_SXD: u16 = 0x99c;
pub const OP_PAX: u16 = 0x1dc;
pub const OP_PDX: u16 = 0x9dc;
pub const OP_PXA: u16 = 0x1ec;
pub const OP_FAD: u16 = 0x0c0;
pub const OP_FSB: u16 = 0x0c2;
pub const OP_FMP: u16 = 0x0b0;
pub const OP_FDH: u16 = 0x0a0;
pub const OP_FDP: u16 = 0x0a1;

pub const PREFIX_TXI: u8 = 0b001;
pub const PREFIX_TIX: u8 = 0b010;
pub const PREFIX_TXH: u8 = 0b011;
pub const PREFIX_TXL: u8 = 0b101;
pub const PREFIX_TNX: u8 = 0b110;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IBM704State {
    pub accumulator_sign: bool,
    /// Q in bit 1 and P in bit 0.
    pub accumulator_qp: u8,
    pub accumulator_p: bool,
    pub accumulator_q: bool,
    pub accumulator_magnitude: u64,
    pub mq: u64,
    pub mq_sign: bool,
    pub mq_magnitude: u64,
    pub index_a: u16,
    pub index_b: u16,
    pub index_c: u16,
    pub pc: u16,
    pub halted: bool,
    pub overflow_trigger: bool,
    pub divide_check_trigger: bool,
    pub memory: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    pub instruction: u64,
    pub mnemonic: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub final_state: IBM704State,
    pub traces: Vec<StepTrace>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IBM704Error {
    InvalidMemorySize { words: usize },
    InvalidProgram(DecodeError),
    ProgramTooLarge { words: usize, capacity: usize },
    InvalidOrigin { origin: usize },
    AddressOutOfRange { address: usize, capacity: usize },
    Halted,
    UnknownOpcode { opcode: u16, pc: u16 },
    UnknownTypeAPrefix { prefix: u8, pc: u16 },
    MaxStepsExceeded { max_steps: usize },
}

impl Display for IBM704Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMemorySize { words } => {
                write!(f, "memory size must be in 1..={MEMORY_WORDS}, got {words}")
            }
            Self::InvalidProgram(error) => write!(f, "invalid IBM 704 program: {error}"),
            Self::ProgramTooLarge { words, capacity } => {
                write!(f, "program has {words} words but memory holds {capacity}")
            }
            Self::InvalidOrigin { origin } => write!(f, "load origin {origin} is outside memory"),
            Self::AddressOutOfRange { address, capacity } => {
                write!(f, "address {address} is outside {capacity}-word memory")
            }
            Self::Halted => write!(f, "the IBM 704 is halted"),
            Self::UnknownOpcode { opcode, pc } => {
                write!(f, "unknown opcode {opcode:#05x} at PC={pc:#06x}")
            }
            Self::UnknownTypeAPrefix { prefix, pc } => {
                write!(f, "unknown Type A prefix {prefix:#05b} at PC={pc:#06x}")
            }
            Self::MaxStepsExceeded { max_steps } => {
                write!(f, "maximum execution step count {max_steps} exceeded")
            }
        }
    }
}

impl Error for IBM704Error {}

impl From<DecodeError> for IBM704Error {
    fn from(value: DecodeError) -> Self {
        Self::InvalidProgram(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IBM704Simulator {
    memory: Vec<u64>,
    ac_sign: bool,
    ac_p: bool,
    ac_q: bool,
    ac_magnitude: u64,
    mq: u64,
    index_a: u16,
    index_b: u16,
    index_c: u16,
    pc: u16,
    halted: bool,
    overflow_trigger: bool,
    divide_check_trigger: bool,
    mq_overflow: bool,
}

impl Default for IBM704Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl IBM704Simulator {
    pub fn new() -> Self {
        Self::with_memory_words(MEMORY_WORDS).expect("architectural memory size is valid")
    }

    pub fn with_memory_words(words: usize) -> Result<Self, IBM704Error> {
        if words == 0 || words > MEMORY_WORDS {
            return Err(IBM704Error::InvalidMemorySize { words });
        }
        Ok(Self {
            memory: vec![0; words],
            ac_sign: false,
            ac_p: false,
            ac_q: false,
            ac_magnitude: 0,
            mq: 0,
            index_a: 0,
            index_b: 0,
            index_c: 0,
            pc: 0,
            halted: false,
            overflow_trigger: false,
            divide_check_trigger: false,
            mq_overflow: false,
        })
    }

    pub fn reset(&mut self) {
        let words = self.memory.len();
        *self = Self::with_memory_words(words).expect("existing memory size is valid");
    }

    pub fn load(&mut self, program: &[u8], origin: usize) -> Result<usize, IBM704Error> {
        if !program.len().is_multiple_of(BYTES_PER_WORD) {
            return Err(DecodeError::InvalidLength(program.len()).into());
        }
        self.validate_load_bounds(program.len() / BYTES_PER_WORD, origin, program.is_empty())?;
        let words = unpack_words(program)?;
        self.memory[origin..origin + words.len()].copy_from_slice(&words);
        Ok(words.len())
    }

    pub fn load_words(&mut self, words: &[u64], origin: usize) -> Result<usize, IBM704Error> {
        self.validate_load_bounds(words.len(), origin, words.is_empty())?;
        for (destination, word) in self.memory[origin..origin + words.len()]
            .iter_mut()
            .zip(words)
        {
            *destination = *word & WORD_MASK;
        }
        Ok(words.len())
    }

    fn validate_load_bounds(
        &self,
        words: usize,
        origin: usize,
        empty: bool,
    ) -> Result<(), IBM704Error> {
        if origin > self.memory.len() || (origin == self.memory.len() && !empty) {
            return Err(IBM704Error::InvalidOrigin { origin });
        }
        let capacity = self.memory.len() - origin;
        if words > capacity {
            return Err(IBM704Error::ProgramTooLarge { words, capacity });
        }
        Ok(())
    }

    pub fn read_word(&self, address: usize) -> Result<u64, IBM704Error> {
        self.memory
            .get(address)
            .copied()
            .ok_or(IBM704Error::AddressOutOfRange {
                address,
                capacity: self.memory.len(),
            })
    }

    pub fn write_word(&mut self, address: usize, word: u64) -> Result<(), IBM704Error> {
        let capacity = self.memory.len();
        let destination = self
            .memory
            .get_mut(address)
            .ok_or(IBM704Error::AddressOutOfRange { address, capacity })?;
        *destination = word & WORD_MASK;
        Ok(())
    }

    pub fn step(&mut self) -> Result<StepTrace, IBM704Error> {
        if self.halted {
            return Err(IBM704Error::Halted);
        }
        let pc_before = self.pc;
        let instruction = match self.read_word(pc_before as usize) {
            Ok(word) => word,
            Err(error) => {
                self.halted = true;
                return Err(error);
            }
        };
        let prefix = ((instruction >> 33) & 7) as u8;
        let result = if prefix & 0b11 != 0 {
            self.execute_type_a(prefix, instruction)
        } else {
            self.execute_type_b(instruction)
        };
        match result {
            Ok((mnemonic, description)) => Ok(StepTrace {
                pc_before,
                pc_after: self.pc,
                instruction,
                mnemonic,
                description,
            }),
            Err(error) => {
                self.halted = true;
                Err(error)
            }
        }
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, IBM704Error> {
        let mut traces = Vec::new();
        while !self.halted {
            if traces.len() == max_steps {
                return Err(IBM704Error::MaxStepsExceeded { max_steps });
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

    pub fn execute(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, IBM704Error> {
        self.reset();
        self.load(program, 0)?;
        self.run(max_steps)
    }

    pub fn get_state(&self) -> IBM704State {
        IBM704State {
            accumulator_sign: self.ac_sign,
            accumulator_qp: ((self.ac_q as u8) << 1) | self.ac_p as u8,
            accumulator_p: self.ac_p,
            accumulator_q: self.ac_q,
            accumulator_magnitude: self.ac_magnitude,
            mq: self.mq,
            mq_sign: self.mq & SIGN_BIT != 0,
            mq_magnitude: self.mq & MAGNITUDE_MASK,
            index_a: self.index_a,
            index_b: self.index_b,
            index_c: self.index_c,
            pc: self.pc,
            halted: self.halted,
            overflow_trigger: self.overflow_trigger,
            divide_check_trigger: self.divide_check_trigger,
            memory: self.memory.clone(),
        }
    }

    fn read(&self, address: u16) -> Result<u64, IBM704Error> {
        self.read_word(address as usize)
    }
    fn write(&mut self, address: u16, word: u64) -> Result<(), IBM704Error> {
        self.write_word(address as usize, word)
    }
    fn ac_word(&self) -> u64 {
        make_word(self.ac_sign, self.ac_magnitude)
    }
    fn set_ac_word(&mut self, word: u64) {
        self.ac_sign = word & SIGN_BIT != 0;
        self.ac_magnitude = word & MAGNITUDE_MASK;
        self.ac_q = false;
        self.ac_p = false;
    }
    fn advance(&mut self) {
        self.pc = self.pc.wrapping_add(1) & ADDR_MASK as u16;
    }
    fn index(&self, tag: u8) -> u16 {
        let mut value = 0;
        if tag & 1 != 0 {
            value |= self.index_a;
        }
        if tag & 2 != 0 {
            value |= self.index_b;
        }
        if tag & 4 != 0 {
            value |= self.index_c;
        }
        value
    }
    fn set_index(&mut self, tag: u8, value: u16) {
        let value = value & ADDR_MASK as u16;
        if tag & 1 != 0 {
            self.index_a = value;
        }
        if tag & 2 != 0 {
            self.index_b = value;
        }
        if tag & 4 != 0 {
            self.index_c = value;
        }
    }
    fn effective(&self, address: u16, tag: u8) -> u16 {
        address.wrapping_sub(self.index(tag)) & ADDR_MASK as u16
    }
    fn branch(&mut self, condition: bool, address: u16, tag: u8) {
        if condition {
            self.pc = self.effective(address, tag);
        } else {
            self.advance();
        }
    }

    fn execute_type_a(&mut self, prefix: u8, word: u64) -> Result<(String, String), IBM704Error> {
        let decrement = ((word >> 18) & ADDR_MASK) as u16;
        let tag = ((word >> 15) & TAG_MASK) as u8;
        let address = (word & ADDR_MASK) as u16;
        let ir = self.index(tag);
        let mnemonic = match prefix {
            PREFIX_TXI => {
                let value = ir.wrapping_add(decrement) & ADDR_MASK as u16;
                self.set_index(tag, value);
                self.pc = address;
                "TXI"
            }
            PREFIX_TIX => {
                if ir > decrement {
                    self.set_index(tag, ir - decrement);
                    self.pc = address;
                } else {
                    self.advance();
                }
                "TIX"
            }
            PREFIX_TXH => {
                if ir > decrement {
                    self.pc = address;
                } else {
                    self.advance();
                }
                "TXH"
            }
            PREFIX_TXL => {
                if ir <= decrement {
                    self.pc = address;
                } else {
                    self.advance();
                }
                "TXL"
            }
            PREFIX_TNX => {
                if ir <= decrement {
                    self.pc = address;
                } else {
                    self.set_index(tag, ir - decrement);
                    self.advance();
                }
                "TNX"
            }
            _ => {
                return Err(IBM704Error::UnknownTypeAPrefix {
                    prefix,
                    pc: self.pc,
                })
            }
        };
        Ok((
            format!("{mnemonic} {address},{tag},{decrement}"),
            format!("{mnemonic} with IR{tag}={ir}, decrement={decrement}"),
        ))
    }

    fn execute_type_b(&mut self, word: u64) -> Result<(String, String), IBM704Error> {
        let opcode = ((word >> 24) & 0xfff) as u16;
        let tag = ((word >> 15) & TAG_MASK) as u8;
        let address = (word & ADDR_MASK) as u16;
        let eff = self.effective(address, tag);
        let operand = || {
            if tag == 0 {
                address.to_string()
            } else {
                format!("{address},{tag}")
            }
        };
        let mnemonic = match opcode {
            OP_HTR => {
                self.pc = eff;
                self.halted = true;
                "HTR"
            }
            OP_HPR => {
                self.advance();
                self.halted = true;
                "HPR"
            }
            OP_NOP => {
                self.advance();
                "NOP"
            }
            OP_CLA => {
                let value = self.read(eff)?;
                self.set_ac_word(value);
                self.advance();
                "CLA"
            }
            OP_CAL => {
                let value = self.read(eff)?;
                self.ac_sign = false;
                self.ac_q = false;
                self.ac_p = value & SIGN_BIT != 0;
                self.ac_magnitude = value & MAGNITUDE_MASK;
                self.advance();
                "CAL"
            }
            OP_STO => {
                self.write(eff, self.ac_word())?;
                self.advance();
                "STO"
            }
            OP_STZ => {
                self.write(eff, 0)?;
                self.advance();
                "STZ"
            }
            OP_STQ => {
                self.write(eff, self.mq)?;
                self.advance();
                "STQ"
            }
            OP_LDQ => {
                self.mq = self.read(eff)?;
                self.advance();
                "LDQ"
            }
            OP_XCA => {
                let ac = self.ac_word();
                self.set_ac_word(self.mq);
                self.mq = ac;
                self.advance();
                "XCA"
            }
            OP_ADD | OP_SUB | OP_ADM => {
                let value = self.read(eff)?;
                let sign = match opcode {
                    OP_SUB => !word_sign(value),
                    OP_ADM => false,
                    _ => word_sign(value),
                };
                let (result_sign, magnitude, overflow) = add_sign_magnitude(
                    self.ac_sign,
                    self.ac_magnitude,
                    sign,
                    word_magnitude(value),
                );
                self.ac_sign = result_sign;
                self.ac_magnitude = magnitude;
                if overflow {
                    self.ac_p = true;
                    self.overflow_trigger = true;
                }
                self.advance();
                if opcode == OP_ADD {
                    "ADD"
                } else if opcode == OP_SUB {
                    "SUB"
                } else {
                    "ADM"
                }
            }
            OP_MPY => {
                let value = self.read(eff)?;
                let product = (word_magnitude(self.mq) as u128) * (word_magnitude(value) as u128);
                let mut sign = word_sign(self.mq) ^ word_sign(value);
                if product == 0 {
                    sign = false;
                }
                self.ac_sign = sign;
                self.ac_magnitude = ((product >> 35) as u64) & MAGNITUDE_MASK;
                self.ac_p = false;
                self.ac_q = false;
                self.mq = make_word(sign, product as u64 & MAGNITUDE_MASK);
                self.advance();
                "MPY"
            }
            OP_DVP | OP_DVH => {
                let value = self.read(eff)?;
                let divisor = word_magnitude(value);
                if divisor == 0 || divisor <= self.ac_magnitude {
                    self.divide_check_trigger = true;
                    if opcode == OP_DVH {
                        self.advance();
                        self.halted = true;
                    } else {
                        self.advance();
                    }
                } else {
                    let dividend =
                        ((self.ac_magnitude as u128) << 35) | word_magnitude(self.mq) as u128;
                    let quotient = dividend / divisor as u128;
                    let remainder = dividend % divisor as u128;
                    let q_sign = quotient != 0 && (self.ac_sign ^ word_sign(value));
                    let r_sign = remainder != 0 && self.ac_sign;
                    self.mq = make_word(q_sign, quotient as u64 & MAGNITUDE_MASK);
                    self.ac_sign = r_sign;
                    self.ac_magnitude = remainder as u64 & MAGNITUDE_MASK;
                    self.ac_q = false;
                    self.ac_p = false;
                    self.advance();
                }
                if opcode == OP_DVP {
                    "DVP"
                } else {
                    "DVH"
                }
            }
            OP_TRA => {
                self.pc = eff;
                "TRA"
            }
            OP_TZE => {
                self.branch(self.ac_magnitude == 0, address, tag);
                "TZE"
            }
            OP_TNZ => {
                self.branch(self.ac_magnitude != 0, address, tag);
                "TNZ"
            }
            OP_TPL => {
                self.branch(!self.ac_sign, address, tag);
                "TPL"
            }
            OP_TMI => {
                self.branch(self.ac_sign, address, tag);
                "TMI"
            }
            OP_TOV => {
                let condition = self.overflow_trigger;
                if condition {
                    self.overflow_trigger = false;
                }
                self.branch(condition, address, tag);
                "TOV"
            }
            OP_TNO => {
                let condition = !self.overflow_trigger;
                self.overflow_trigger = false;
                self.branch(condition, address, tag);
                "TNO"
            }
            OP_TQO => {
                let condition = self.mq_overflow;
                if condition {
                    self.mq_overflow = false;
                }
                self.branch(condition, address, tag);
                "TQO"
            }
            OP_TQP => {
                self.branch(!word_sign(self.mq), address, tag);
                "TQP"
            }
            OP_LXA => {
                let value = (self.read(address)? & ADDR_MASK) as u16;
                self.set_index(tag, value);
                self.advance();
                "LXA"
            }
            OP_LXD => {
                let value = ((self.read(address)? >> 18) & ADDR_MASK) as u16;
                self.set_index(tag, value);
                self.advance();
                "LXD"
            }
            OP_SXA => {
                let existing = self.read(address)?;
                self.write(address, (existing & !ADDR_MASK) | self.index(tag) as u64)?;
                self.advance();
                "SXA"
            }
            OP_SXD => {
                let existing = self.read(address)?;
                let mask = ADDR_MASK << 18;
                self.write(
                    address,
                    (existing & !mask) | ((self.index(tag) as u64) << 18),
                )?;
                self.advance();
                "SXD"
            }
            OP_PAX => {
                self.set_index(tag, (self.ac_word() & ADDR_MASK) as u16);
                self.advance();
                "PAX"
            }
            OP_PDX => {
                self.set_index(tag, ((self.ac_word() >> 18) & ADDR_MASK) as u16);
                self.advance();
                "PDX"
            }
            OP_PXA => {
                self.ac_sign = false;
                self.ac_magnitude = self.index(tag) as u64;
                self.ac_p = false;
                self.ac_q = false;
                self.advance();
                "PXA"
            }
            OP_FAD | OP_FSB => {
                let rhs = fp_to_float(self.read(eff)?);
                let lhs = fp_to_float(self.ac_word());
                let result = if opcode == OP_FAD {
                    lhs + rhs
                } else {
                    lhs - rhs
                };
                self.store_fp_result(result);
                self.advance();
                if opcode == OP_FAD {
                    "FAD"
                } else {
                    "FSB"
                }
            }
            OP_FMP => {
                let result = fp_to_float(self.mq) * fp_to_float(self.read(eff)?);
                self.store_fp_result(result);
                self.advance();
                "FMP"
            }
            OP_FDH | OP_FDP => {
                let divisor = fp_to_float(self.read(eff)?);
                if divisor == 0.0 {
                    self.divide_check_trigger = true;
                    self.advance();
                    if opcode == OP_FDH {
                        self.halted = true;
                    }
                } else {
                    let dividend = fp_to_float(self.ac_word());
                    let quotient = dividend / divisor;
                    let remainder = dividend - quotient * divisor;
                    self.mq = float_to_fp(quotient);
                    self.set_ac_word(float_to_fp(remainder));
                    self.advance();
                }
                if opcode == OP_FDH {
                    "FDH"
                } else {
                    "FDP"
                }
            }
            _ => {
                return Err(IBM704Error::UnknownOpcode {
                    opcode,
                    pc: self.pc,
                })
            }
        };
        Ok((
            if matches!(opcode, OP_HPR | OP_NOP | OP_XCA) {
                mnemonic.to_owned()
            } else {
                format!("{mnemonic} {}", operand())
            },
            format!("{mnemonic} completed; effective address {eff}"),
        ))
    }

    fn store_fp_result(&mut self, value: f64) {
        let word = float_to_fp(value);
        self.set_ac_word(word);
        self.mq = word;
    }
}

pub const fn make_word(sign: bool, magnitude: u64) -> u64 {
    (if sign { SIGN_BIT } else { 0 }) | (magnitude & MAGNITUDE_MASK)
}
pub const fn word_sign(word: u64) -> bool {
    word & SIGN_BIT != 0
}
pub const fn word_magnitude(word: u64) -> u64 {
    word & MAGNITUDE_MASK
}

pub const fn add_sign_magnitude(
    sign_a: bool,
    mag_a: u64,
    sign_b: bool,
    mag_b: u64,
) -> (bool, u64, bool) {
    let mag_a = mag_a & MAGNITUDE_MASK;
    let mag_b = mag_b & MAGNITUDE_MASK;
    if sign_a == sign_b {
        let result = mag_a + mag_b;
        if result > MAGNITUDE_MASK {
            (sign_a, result & MAGNITUDE_MASK, true)
        } else if result == 0 {
            (false, 0, false)
        } else {
            (sign_a, result, false)
        }
    } else if mag_a >= mag_b {
        let result = mag_a - mag_b;
        (result != 0 && sign_a, result, false)
    } else {
        (sign_b, mag_b - mag_a, false)
    }
}

pub fn fp_to_float(word: u64) -> f64 {
    let sign = word_sign(word);
    let characteristic = ((word >> FP_CHAR_SHIFT) & 0xff) as i32;
    let fraction = word & FP_FRAC_MASK;
    if characteristic == 0 && fraction == 0 {
        return 0.0;
    }
    let value = fraction as f64 * 2f64.powi(characteristic - FP_EXCESS - FP_FRAC_BITS);
    if sign {
        -value
    } else {
        value
    }
}

pub fn float_to_fp(value: f64) -> u64 {
    if value == 0.0 || !value.is_finite() {
        return 0;
    }
    let sign = value.is_sign_negative();
    let magnitude = value.abs();
    let exponent = magnitude.log2().floor() as i32 + 1;
    let mantissa = magnitude / 2f64.powi(exponent);
    let mut characteristic = exponent + FP_EXCESS;
    let mut fraction = (mantissa * (1u64 << FP_FRAC_BITS) as f64).round_ties_even() as u64;
    if fraction >= 1 << FP_FRAC_BITS {
        fraction >>= 1;
        characteristic += 1;
    }
    if characteristic < 0 {
        return if sign { SIGN_BIT } else { 0 };
    }
    if characteristic > 0xff {
        return make_word(sign, (0xff << FP_CHAR_SHIFT) | FP_FRAC_MASK);
    }
    make_word(
        sign,
        ((characteristic as u64) << FP_CHAR_SHIFT) | (fraction & FP_FRAC_MASK),
    )
}
