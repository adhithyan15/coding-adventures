//! Gate-level IBM 704 simulator.
//!
//! Every persistent architectural bit is stored in a simulated master-slave
//! D flip-flop. Integer arithmetic flows through ripple-carry adders, multiply
//! uses a 35-stage AND/shift/add network, division uses restoring subtraction,
//! and instruction selection is driven by one-hot gate decoders. Host control
//! flow sequences clock edges and names trace records.

use arithmetic::adders::{ripple_carry_adder, ripple_carry_adder_with_carry};
use ibm704_encoder::{unpack_words, DecodeError, ADDR_MASK, BYTES_PER_WORD, SIGN_BIT, WORD_MASK};
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};
use logic_gates::sequential::{register, FlipFlopState};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

mod floating;

/// Maximum number of 36-bit words in an IBM 704 core-memory installation.
pub const MEMORY_WORDS: usize = 32_768;
/// Mask for the 35 magnitude bits of a sign-magnitude word.
pub const MAGNITUDE_MASK: u64 = SIGN_BIT - 1;

const ZERO_1: [u8; 1] = [0];
const ZERO_15: [u8; 15] = [0; 15];
const ZERO_35: [u8; 35] = [0; 35];
const ZERO_36: [u8; 36] = [0; 36];
const FP_FRAC_BITS: i32 = 27;
const FP_FRAC_MASK: u64 = (1 << FP_FRAC_BITS) - 1;
const FP_CHAR_SHIFT: u32 = 27;
const FP_EXCESS: i32 = 128;

/// Exact number of persistent D flip-flops in a full 32K-word machine.
pub const FLIP_FLOP_COUNT: usize = MEMORY_WORDS * 36 + 38 + 36 + 3 * 15 + 15 + 4;

/// Stable educational estimate of storage and combinational gate topology.
///
/// Storage is counted as six primitive gates per master-slave flip-flop. The
/// remaining term covers the opcode/prefix decoders, address and AC adders,
/// multiply/divide arrays, memory selection, and control fan-in. It is a
/// topology estimate, not a die-accurate reconstruction of the vacuum-tube 704.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 73_440;

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
struct BitRegister<const WIDTH: usize> {
    state: [FlipFlopState; WIDTH],
}

impl<const WIDTH: usize> BitRegister<WIDTH> {
    fn new(initial: &[u8; WIDTH]) -> Self {
        let mut value = Self {
            state: std::array::from_fn(|_| FlipFlopState::default()),
        };
        value.write(initial);
        value
    }

    fn read(&self) -> [u8; WIDTH] {
        let mut state = self.state.clone();
        register(&[0; WIDTH], 0, &mut state)
            .try_into()
            .expect("a fixed-width register preserves its width")
    }

    fn write(&mut self, data: &[u8; WIDTH]) {
        register(data, 0, &mut self.state);
        register(data, 1, &mut self.state);
    }
}

/// An owned snapshot of the complete architectural state.
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

/// One gate-level fetch/decode/execute clock cycle.
#[derive(Clone, Debug, PartialEq)]
pub struct StepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    pub instruction: u64,
    pub mnemonic: String,
    pub description: String,
}

/// Result of a bounded run that reached a halt instruction.
#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub final_state: IBM704State,
    pub traces: Vec<StepTrace>,
}

/// Fail-closed lifecycle and decode errors.
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operation {
    Htr,
    Hpr,
    Nop,
    Cla,
    Cal,
    Add,
    Sub,
    Adm,
    Sto,
    Stz,
    Stq,
    Ldq,
    Xca,
    Mpy,
    Dvp,
    Dvh,
    Tra,
    Tze,
    Tnz,
    Tpl,
    Tmi,
    Tov,
    Tno,
    Tqo,
    Tqp,
    Lxa,
    Lxd,
    Sxa,
    Sxd,
    Pax,
    Pdx,
    Pxa,
    Fad,
    Fsb,
    Fmp,
    Fdh,
    Fdp,
}

const OPERATION_TABLE: [(u16, Operation); 37] = [
    (OP_HTR, Operation::Htr),
    (OP_HPR, Operation::Hpr),
    (OP_NOP, Operation::Nop),
    (OP_CLA, Operation::Cla),
    (OP_CAL, Operation::Cal),
    (OP_ADD, Operation::Add),
    (OP_SUB, Operation::Sub),
    (OP_ADM, Operation::Adm),
    (OP_STO, Operation::Sto),
    (OP_STZ, Operation::Stz),
    (OP_STQ, Operation::Stq),
    (OP_LDQ, Operation::Ldq),
    (OP_XCA, Operation::Xca),
    (OP_MPY, Operation::Mpy),
    (OP_DVP, Operation::Dvp),
    (OP_DVH, Operation::Dvh),
    (OP_TRA, Operation::Tra),
    (OP_TZE, Operation::Tze),
    (OP_TNZ, Operation::Tnz),
    (OP_TPL, Operation::Tpl),
    (OP_TMI, Operation::Tmi),
    (OP_TOV, Operation::Tov),
    (OP_TNO, Operation::Tno),
    (OP_TQO, Operation::Tqo),
    (OP_TQP, Operation::Tqp),
    (OP_LXA, Operation::Lxa),
    (OP_LXD, Operation::Lxd),
    (OP_SXA, Operation::Sxa),
    (OP_SXD, Operation::Sxd),
    (OP_PAX, Operation::Pax),
    (OP_PDX, Operation::Pdx),
    (OP_PXA, Operation::Pxa),
    (OP_FAD, Operation::Fad),
    (OP_FSB, Operation::Fsb),
    (OP_FMP, Operation::Fmp),
    (OP_FDH, Operation::Fdh),
    (OP_FDP, Operation::Fdp),
];

/// IBM 704 whose persistent state and integer data paths are digital circuits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IBM704GateLevel {
    memory: Vec<BitRegister<36>>,
    ac_sign: BitRegister<1>,
    ac_p: BitRegister<1>,
    ac_q: BitRegister<1>,
    ac_magnitude: BitRegister<35>,
    mq: BitRegister<36>,
    index_a: BitRegister<15>,
    index_b: BitRegister<15>,
    index_c: BitRegister<15>,
    pc: BitRegister<15>,
    halted: BitRegister<1>,
    overflow_trigger: BitRegister<1>,
    divide_check_trigger: BitRegister<1>,
    mq_overflow: BitRegister<1>,
}

impl Default for IBM704GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl IBM704GateLevel {
    /// Construct a full 32K-word machine in its power-on state.
    pub fn new() -> Self {
        Self::with_memory_words(MEMORY_WORDS).expect("architectural memory size is valid")
    }

    /// Construct a smaller gate store for focused programs and tests.
    pub fn with_memory_words(words: usize) -> Result<Self, IBM704Error> {
        if words == 0 || words > MEMORY_WORDS {
            return Err(IBM704Error::InvalidMemorySize { words });
        }
        Ok(Self {
            memory: vec![BitRegister::new(&ZERO_36); words],
            ac_sign: BitRegister::new(&ZERO_1),
            ac_p: BitRegister::new(&ZERO_1),
            ac_q: BitRegister::new(&ZERO_1),
            ac_magnitude: BitRegister::new(&ZERO_35),
            mq: BitRegister::new(&ZERO_36),
            index_a: BitRegister::new(&ZERO_15),
            index_b: BitRegister::new(&ZERO_15),
            index_c: BitRegister::new(&ZERO_15),
            pc: BitRegister::new(&ZERO_15),
            halted: BitRegister::new(&ZERO_1),
            overflow_trigger: BitRegister::new(&ZERO_1),
            divide_check_trigger: BitRegister::new(&ZERO_1),
            mq_overflow: BitRegister::new(&ZERO_1),
        })
    }

    /// Clock zeros into every persistent state element.
    pub fn reset(&mut self) {
        for word in &mut self.memory {
            word.write(&ZERO_36);
        }
        self.ac_sign.write(&ZERO_1);
        self.ac_p.write(&ZERO_1);
        self.ac_q.write(&ZERO_1);
        self.ac_magnitude.write(&ZERO_35);
        self.mq.write(&ZERO_36);
        self.index_a.write(&ZERO_15);
        self.index_b.write(&ZERO_15);
        self.index_c.write(&ZERO_15);
        self.pc.write(&ZERO_15);
        self.halted.write(&ZERO_1);
        self.overflow_trigger.write(&ZERO_1);
        self.divide_check_trigger.write(&ZERO_1);
        self.mq_overflow.write(&ZERO_1);
    }

    /// Decode canonical five-byte big-endian words and clock them into memory.
    pub fn load(&mut self, program: &[u8], origin: usize) -> Result<usize, IBM704Error> {
        if !program.len().is_multiple_of(BYTES_PER_WORD) {
            return Err(DecodeError::InvalidLength(program.len()).into());
        }
        self.validate_load_bounds(program.len() / BYTES_PER_WORD, origin, program.is_empty())?;
        let words = unpack_words(program)?;
        self.load_words(&words, origin)
    }

    /// Mask and clock host-supplied words into gate memory.
    pub fn load_words(&mut self, words: &[u64], origin: usize) -> Result<usize, IBM704Error> {
        self.validate_load_bounds(words.len(), origin, words.is_empty())?;
        for (destination, word) in self.memory[origin..origin + words.len()]
            .iter_mut()
            .zip(words)
        {
            destination.write(&u64_to_bits::<36>(*word & WORD_MASK));
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
            .map(|word| bits_to_u64(&word.read()))
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
        destination.write(&u64_to_bits::<36>(word & WORD_MASK));
        Ok(())
    }

    /// Execute one gate-level fetch/decode/execute cycle.
    pub fn step(&mut self) -> Result<StepTrace, IBM704Error> {
        if self.halted.read()[0] == 1 {
            return Err(IBM704Error::Halted);
        }
        let pc_before = self.pc_value();
        let instruction = match self.read_word(pc_before as usize) {
            Ok(word) => word,
            Err(error) => {
                self.halted.write(&[1]);
                return Err(error);
            }
        };
        let instruction_bits = u64_to_bits::<36>(instruction);
        let prefix: [u8; 3] = instruction_bits[33..36]
            .try_into()
            .expect("three prefix bits");
        let is_type_a = or_gate(prefix[0], prefix[1]);
        let result = if is_type_a == 1 {
            self.execute_type_a(prefix, &instruction_bits)
        } else {
            self.execute_type_b(&instruction_bits)
        };
        match result {
            Ok((mnemonic, description)) => Ok(StepTrace {
                pc_before,
                pc_after: self.pc_value(),
                instruction,
                mnemonic,
                description,
            }),
            Err(error) => {
                self.halted.write(&[1]);
                Err(error)
            }
        }
    }

    /// Run until a halt, subject to a mandatory instruction bound.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, IBM704Error> {
        let mut traces = Vec::new();
        while self.halted.read()[0] == 0 {
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

    /// Reset, load at word zero, and execute until halt or the step bound.
    pub fn execute(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, IBM704Error> {
        self.reset();
        self.load(program, 0)?;
        self.run(max_steps)
    }

    /// Return an owned architectural snapshot.
    pub fn get_state(&self) -> IBM704State {
        let ac_sign = self.ac_sign.read()[0] == 1;
        let ac_p = self.ac_p.read()[0] == 1;
        let ac_q = self.ac_q.read()[0] == 1;
        let mq = bits_to_u64(&self.mq.read());
        IBM704State {
            accumulator_sign: ac_sign,
            accumulator_qp: ((ac_q as u8) << 1) | ac_p as u8,
            accumulator_p: ac_p,
            accumulator_q: ac_q,
            accumulator_magnitude: bits_to_u64(&self.ac_magnitude.read()),
            mq,
            mq_sign: mq & SIGN_BIT != 0,
            mq_magnitude: mq & MAGNITUDE_MASK,
            index_a: bits_to_u64(&self.index_a.read()) as u16,
            index_b: bits_to_u64(&self.index_b.read()) as u16,
            index_c: bits_to_u64(&self.index_c.read()) as u16,
            pc: self.pc_value(),
            halted: self.halted.read()[0] == 1,
            overflow_trigger: self.overflow_trigger.read()[0] == 1,
            divide_check_trigger: self.divide_check_trigger.read()[0] == 1,
            memory: self
                .memory
                .iter()
                .map(|word| bits_to_u64(&word.read()))
                .collect(),
        }
    }

    /// Exact persistent-bit count for this configured memory size.
    pub fn flip_flop_count(&self) -> usize {
        self.memory.len() * 36 + 38 + 36 + 3 * 15 + 15 + 4
    }

    /// Educational gate-equivalent count scaled for this configured memory.
    pub fn gate_count(&self) -> usize {
        self.flip_flop_count() * 6 + 73_440
    }

    fn pc_value(&self) -> u16 {
        bits_to_u64(&self.pc.read()) as u16
    }
    fn read(&self, address: u16) -> Result<u64, IBM704Error> {
        self.read_word(address as usize)
    }
    fn write(&mut self, address: u16, word: u64) -> Result<(), IBM704Error> {
        self.write_word(address as usize, word)
    }
    fn ac_word(&self) -> u64 {
        make_word(
            self.ac_sign.read()[0] == 1,
            bits_to_u64(&self.ac_magnitude.read()),
        )
    }
    fn set_ac_word(&mut self, word: u64) {
        self.ac_sign.write(&[((word & SIGN_BIT) != 0) as u8]);
        self.ac_magnitude
            .write(&u64_to_bits::<35>(word & MAGNITUDE_MASK));
        self.ac_q.write(&ZERO_1);
        self.ac_p.write(&ZERO_1);
    }
    fn advance(&mut self) {
        self.pc
            .write(&add_bits(&self.pc.read(), &u64_to_bits::<15>(1)));
    }
    fn index_bits(&self, tag: [u8; 3]) -> [u8; 15] {
        let a = self.index_a.read();
        let b = self.index_b.read();
        let c = self.index_c.read();
        std::array::from_fn(|bit| {
            or_gate(
                or_gate(and_gate(tag[0], a[bit]), and_gate(tag[1], b[bit])),
                and_gate(tag[2], c[bit]),
            )
        })
    }
    fn set_index(&mut self, tag: [u8; 3], value: [u8; 15]) {
        if tag[0] == 1 {
            self.index_a.write(&value);
        }
        if tag[1] == 1 {
            self.index_b.write(&value);
        }
        if tag[2] == 1 {
            self.index_c.write(&value);
        }
    }
    fn effective_bits(&self, address: [u8; 15], tag: [u8; 3]) -> [u8; 15] {
        subtract_bits(&address, &self.index_bits(tag))
    }
    fn branch(&mut self, condition: u8, address: [u8; 15], tag: [u8; 3]) {
        let target = self.effective_bits(address, tag);
        let sequential = add_bits(&self.pc.read(), &u64_to_bits::<15>(1));
        self.pc.write(&mux_bits(condition, &sequential, &target));
    }

    fn execute_type_a(
        &mut self,
        prefix: [u8; 3],
        word: &[u8; 36],
    ) -> Result<(String, String), IBM704Error> {
        let decrement: [u8; 15] = word[18..33].try_into().expect("15 decrement bits");
        let tag: [u8; 3] = word[15..18].try_into().expect("three tag bits");
        let address: [u8; 15] = word[..15].try_into().expect("15 address bits");
        let ir = self.index_bits(tag);
        let greater = greater_than_bits(&ir, &decrement);
        let less_or_equal = not_gate(greater);
        let selectors = decode_bits(prefix);
        let (mnemonic, set_ir, new_ir, branch_condition, branch_unconditional) =
            if selectors[PREFIX_TXI as usize] == 1 {
                ("TXI", 1, add_bits(&ir, &decrement), 1, true)
            } else if selectors[PREFIX_TIX as usize] == 1 {
                (
                    "TIX",
                    greater,
                    subtract_bits(&ir, &decrement),
                    greater,
                    false,
                )
            } else if selectors[PREFIX_TXH as usize] == 1 {
                ("TXH", 0, ir, greater, false)
            } else if selectors[PREFIX_TXL as usize] == 1 {
                ("TXL", 0, ir, less_or_equal, false)
            } else if selectors[PREFIX_TNX as usize] == 1 {
                (
                    "TNX",
                    greater,
                    subtract_bits(&ir, &decrement),
                    less_or_equal,
                    false,
                )
            } else {
                return Err(IBM704Error::UnknownTypeAPrefix {
                    prefix: bits_to_u64(&prefix) as u8,
                    pc: self.pc_value(),
                });
            };
        if set_ir == 1 {
            self.set_index(tag, new_ir);
        }
        if branch_unconditional || branch_condition == 1 {
            self.pc.write(&address);
        } else {
            self.advance();
        }
        let address_value = bits_to_u64(&address) as u16;
        let tag_value = bits_to_u64(&tag) as u8;
        let decrement_value = bits_to_u64(&decrement) as u16;
        let ir_value = bits_to_u64(&ir) as u16;
        Ok((
            format!("{mnemonic} {address_value},{tag_value},{decrement_value}"),
            format!("{mnemonic} with IR{tag_value}={ir_value}, decrement={decrement_value}"),
        ))
    }

    fn execute_type_b(&mut self, word: &[u8; 36]) -> Result<(String, String), IBM704Error> {
        let opcode_bits: [u8; 12] = word[24..36].try_into().expect("12 opcode bits");
        let opcode = bits_to_u64(&opcode_bits) as u16;
        let operation = decode_operation(opcode_bits).ok_or(IBM704Error::UnknownOpcode {
            opcode,
            pc: self.pc_value(),
        })?;
        let tag: [u8; 3] = word[15..18].try_into().expect("three tag bits");
        let address: [u8; 15] = word[..15].try_into().expect("15 address bits");
        let address_value = bits_to_u64(&address) as u16;
        let tag_value = bits_to_u64(&tag) as u8;
        let effective = self.effective_bits(address, tag);
        let eff = bits_to_u64(&effective) as u16;
        let operand = || {
            if tag_value == 0 {
                address_value.to_string()
            } else {
                format!("{address_value},{tag_value}")
            }
        };

        let mnemonic = match operation {
            Operation::Htr => {
                self.pc.write(&effective);
                self.halted.write(&[1]);
                "HTR"
            }
            Operation::Hpr => {
                self.advance();
                self.halted.write(&[1]);
                "HPR"
            }
            Operation::Nop => {
                self.advance();
                "NOP"
            }
            Operation::Cla => {
                let value = self.read(eff)?;
                self.set_ac_word(value);
                self.advance();
                "CLA"
            }
            Operation::Cal => {
                let value = self.read(eff)?;
                self.ac_sign.write(&ZERO_1);
                self.ac_q.write(&ZERO_1);
                self.ac_p.write(&[((value & SIGN_BIT) != 0) as u8]);
                self.ac_magnitude
                    .write(&u64_to_bits::<35>(value & MAGNITUDE_MASK));
                self.advance();
                "CAL"
            }
            Operation::Sto => {
                self.write(eff, self.ac_word())?;
                self.advance();
                "STO"
            }
            Operation::Stz => {
                self.write(eff, 0)?;
                self.advance();
                "STZ"
            }
            Operation::Stq => {
                self.write(eff, bits_to_u64(&self.mq.read()))?;
                self.advance();
                "STQ"
            }
            Operation::Ldq => {
                let value = self.read(eff)?;
                self.mq.write(&u64_to_bits::<36>(value));
                self.advance();
                "LDQ"
            }
            Operation::Xca => {
                let ac = self.ac_word();
                let mq = bits_to_u64(&self.mq.read());
                self.set_ac_word(mq);
                self.mq.write(&u64_to_bits::<36>(ac));
                self.advance();
                "XCA"
            }
            Operation::Add | Operation::Sub | Operation::Adm => {
                let value = self.read(eff)?;
                let rhs_sign = match operation {
                    Operation::Sub => (value & SIGN_BIT == 0) as u8,
                    Operation::Adm => 0,
                    _ => ((value & SIGN_BIT) != 0) as u8,
                };
                let (sign, magnitude, overflow) = add_sign_magnitude_bits(
                    self.ac_sign.read()[0],
                    self.ac_magnitude.read(),
                    rhs_sign,
                    u64_to_bits::<35>(value & MAGNITUDE_MASK),
                );
                self.ac_sign.write(&[sign]);
                self.ac_magnitude.write(&magnitude);
                if overflow == 1 {
                    self.ac_p.write(&[1]);
                    self.overflow_trigger.write(&[1]);
                }
                self.advance();
                match operation {
                    Operation::Add => "ADD",
                    Operation::Sub => "SUB",
                    _ => "ADM",
                }
            }
            Operation::Mpy => {
                let rhs = self.read(eff)?;
                let mq = bits_to_u64(&self.mq.read());
                let product = multiply_bits(
                    u64_to_bits::<35>(mq & MAGNITUDE_MASK),
                    u64_to_bits::<35>(rhs & MAGNITUDE_MASK),
                );
                let product_zero = is_zero_bits(&product);
                let sign = and_gate(
                    xor_gate(((mq & SIGN_BIT) != 0) as u8, ((rhs & SIGN_BIT) != 0) as u8),
                    not_gate(product_zero),
                );
                let low: [u8; 35] = product[..35].try_into().expect("low product");
                let high: [u8; 35] = product[35..].try_into().expect("high product");
                self.ac_sign.write(&[sign]);
                self.ac_magnitude.write(&high);
                self.ac_p.write(&ZERO_1);
                self.ac_q.write(&ZERO_1);
                self.mq.write(&join_sign_magnitude(sign, low));
                self.advance();
                "MPY"
            }
            Operation::Dvp | Operation::Dvh => {
                let rhs = self.read(eff)?;
                let divisor = u64_to_bits::<35>(rhs & MAGNITUDE_MASK);
                let ac = self.ac_magnitude.read();
                let divide_check = or_gate(
                    is_zero_bits(&divisor),
                    not_gate(greater_than_bits(&divisor, &ac)),
                );
                if divide_check == 1 {
                    self.divide_check_trigger.write(&[1]);
                    self.advance();
                    if operation == Operation::Dvh {
                        self.halted.write(&[1]);
                    }
                } else {
                    let mq = self.mq.read();
                    let mq_mag: [u8; 35] = mq[..35].try_into().expect("MQ magnitude");
                    let mut dividend = [0; 70];
                    dividend[..35].copy_from_slice(&mq_mag);
                    dividend[35..].copy_from_slice(&ac);
                    let (quotient, remainder) = divide_bits(dividend, divisor);
                    let q_nonzero = not_gate(is_zero_bits(&quotient));
                    let r_nonzero = not_gate(is_zero_bits(&remainder));
                    let q_sign = and_gate(
                        q_nonzero,
                        xor_gate(self.ac_sign.read()[0], ((rhs & SIGN_BIT) != 0) as u8),
                    );
                    let r_sign = and_gate(r_nonzero, self.ac_sign.read()[0]);
                    self.mq.write(&join_sign_magnitude(q_sign, quotient));
                    self.ac_sign.write(&[r_sign]);
                    self.ac_magnitude.write(&remainder);
                    self.ac_q.write(&ZERO_1);
                    self.ac_p.write(&ZERO_1);
                    self.advance();
                }
                if operation == Operation::Dvp {
                    "DVP"
                } else {
                    "DVH"
                }
            }
            Operation::Tra => {
                self.pc.write(&effective);
                "TRA"
            }
            Operation::Tze => {
                self.branch(is_zero_bits(&self.ac_magnitude.read()), address, tag);
                "TZE"
            }
            Operation::Tnz => {
                self.branch(
                    not_gate(is_zero_bits(&self.ac_magnitude.read())),
                    address,
                    tag,
                );
                "TNZ"
            }
            Operation::Tpl => {
                self.branch(not_gate(self.ac_sign.read()[0]), address, tag);
                "TPL"
            }
            Operation::Tmi => {
                self.branch(self.ac_sign.read()[0], address, tag);
                "TMI"
            }
            Operation::Tov => {
                let condition = self.overflow_trigger.read()[0];
                if condition == 1 {
                    self.overflow_trigger.write(&ZERO_1);
                }
                self.branch(condition, address, tag);
                "TOV"
            }
            Operation::Tno => {
                let condition = not_gate(self.overflow_trigger.read()[0]);
                self.overflow_trigger.write(&ZERO_1);
                self.branch(condition, address, tag);
                "TNO"
            }
            Operation::Tqo => {
                let condition = self.mq_overflow.read()[0];
                if condition == 1 {
                    self.mq_overflow.write(&ZERO_1);
                }
                self.branch(condition, address, tag);
                "TQO"
            }
            Operation::Tqp => {
                self.branch(not_gate(self.mq.read()[35]), address, tag);
                "TQP"
            }
            Operation::Lxa => {
                let value = self.read(address_value)?;
                self.set_index(tag, u64_to_bits::<15>(value & ADDR_MASK));
                self.advance();
                "LXA"
            }
            Operation::Lxd => {
                let value = self.read(address_value)?;
                self.set_index(tag, u64_to_bits::<15>((value >> 18) & ADDR_MASK));
                self.advance();
                "LXD"
            }
            Operation::Sxa => {
                let existing = self.read(address_value)?;
                let index = bits_to_u64(&self.index_bits(tag));
                self.write(address_value, (existing & !ADDR_MASK) | index)?;
                self.advance();
                "SXA"
            }
            Operation::Sxd => {
                let existing = self.read(address_value)?;
                let index = bits_to_u64(&self.index_bits(tag));
                let mask = ADDR_MASK << 18;
                self.write(address_value, (existing & !mask) | (index << 18))?;
                self.advance();
                "SXD"
            }
            Operation::Pax => {
                self.set_index(tag, u64_to_bits::<15>(self.ac_word() & ADDR_MASK));
                self.advance();
                "PAX"
            }
            Operation::Pdx => {
                self.set_index(tag, u64_to_bits::<15>((self.ac_word() >> 18) & ADDR_MASK));
                self.advance();
                "PDX"
            }
            Operation::Pxa => {
                self.ac_sign.write(&ZERO_1);
                self.ac_magnitude
                    .write(&widen_bits::<15, 35>(self.index_bits(tag)));
                self.ac_p.write(&ZERO_1);
                self.ac_q.write(&ZERO_1);
                self.advance();
                "PXA"
            }
            Operation::Fad | Operation::Fsb => {
                let result = floating::add_words(
                    self.ac_word(),
                    self.read(eff)?,
                    operation == Operation::Fsb,
                );
                self.store_fp_word(result);
                self.advance();
                if operation == Operation::Fad {
                    "FAD"
                } else {
                    "FSB"
                }
            }
            Operation::Fmp => {
                let result =
                    floating::multiply_words(bits_to_u64(&self.mq.read()), self.read(eff)?);
                self.store_fp_word(result);
                self.advance();
                "FMP"
            }
            Operation::Fdh | Operation::Fdp => {
                let divisor = self.read(eff)?;
                if let Some((quotient, remainder)) = floating::divide_words(self.ac_word(), divisor)
                {
                    self.mq.write(&u64_to_bits::<36>(quotient));
                    self.set_ac_word(remainder);
                    self.advance();
                } else {
                    self.divide_check_trigger.write(&[1]);
                    self.advance();
                    if operation == Operation::Fdh {
                        self.halted.write(&[1]);
                    }
                }
                if operation == Operation::Fdh {
                    "FDH"
                } else {
                    "FDP"
                }
            }
        };
        Ok((
            if matches!(operation, Operation::Hpr | Operation::Nop | Operation::Xca) {
                mnemonic.to_owned()
            } else {
                format!("{mnemonic} {}", operand())
            },
            format!("{mnemonic} completed; effective address {eff}"),
        ))
    }

    fn store_fp_word(&mut self, word: u64) {
        self.set_ac_word(word);
        self.mq.write(&u64_to_bits::<36>(word));
    }
}

fn decode_operation(bits: [u8; 12]) -> Option<Operation> {
    OPERATION_TABLE.iter().find_map(|(code, operation)| {
        (equal_bits(&bits, &u64_to_bits::<12>(*code as u64)) == 1).then_some(*operation)
    })
}

fn decode_bits<const WIDTH: usize>(bits: [u8; WIDTH]) -> Vec<u8> {
    (0..(1usize << WIDTH))
        .map(|value| equal_bits(&bits, &u64_to_bits::<WIDTH>(value as u64)))
        .collect()
}

fn equal_bits<const WIDTH: usize>(a: &[u8; WIDTH], b: &[u8; WIDTH]) -> u8 {
    a.iter().zip(b).fold(1, |equal, (left, right)| {
        and_gate(equal, not_gate(xor_gate(*left, *right)))
    })
}

fn is_zero_bits<const WIDTH: usize>(bits: &[u8; WIDTH]) -> u8 {
    not_gate(bits.iter().copied().fold(0, or_gate))
}

fn greater_than_bits<const WIDTH: usize>(a: &[u8; WIDTH], b: &[u8; WIDTH]) -> u8 {
    let mut greater = 0;
    let mut equal = 1;
    for bit in (0..WIDTH).rev() {
        greater = or_gate(greater, and_gate(equal, and_gate(a[bit], not_gate(b[bit]))));
        equal = and_gate(equal, not_gate(xor_gate(a[bit], b[bit])));
    }
    greater
}

fn add_bits<const WIDTH: usize>(a: &[u8; WIDTH], b: &[u8; WIDTH]) -> [u8; WIDTH] {
    ripple_carry_adder(a, b)
        .sum
        .try_into()
        .expect("adder preserves width")
}

fn subtract_bits<const WIDTH: usize>(a: &[u8; WIDTH], b: &[u8; WIDTH]) -> [u8; WIDTH] {
    ripple_carry_adder_with_carry(a, &b.map(not_gate), 1)
        .sum
        .try_into()
        .expect("subtractor preserves width")
}

fn mux_bits<const WIDTH: usize>(select_b: u8, a: &[u8; WIDTH], b: &[u8; WIDTH]) -> [u8; WIDTH] {
    std::array::from_fn(|bit| {
        or_gate(
            and_gate(not_gate(select_b), a[bit]),
            and_gate(select_b, b[bit]),
        )
    })
}

fn add_sign_magnitude_bits(
    sign_a: u8,
    mag_a: [u8; 35],
    sign_b: u8,
    mag_b: [u8; 35],
) -> (u8, [u8; 35], u8) {
    let same_sign = not_gate(xor_gate(sign_a, sign_b));
    if same_sign == 1 {
        let result = ripple_carry_adder(&mag_a, &mag_b);
        let magnitude: [u8; 35] = result.sum.try_into().expect("35-bit AC adder");
        (
            and_gate(sign_a, not_gate(is_zero_bits(&magnitude))),
            magnitude,
            result.carry_out,
        )
    } else {
        let a_greater_or_equal = not_gate(greater_than_bits(&mag_b, &mag_a));
        let magnitude = if a_greater_or_equal == 1 {
            subtract_bits(&mag_a, &mag_b)
        } else {
            subtract_bits(&mag_b, &mag_a)
        };
        let selected_sign = or_gate(
            and_gate(a_greater_or_equal, sign_a),
            and_gate(not_gate(a_greater_or_equal), sign_b),
        );
        (
            and_gate(selected_sign, not_gate(is_zero_bits(&magnitude))),
            magnitude,
            0,
        )
    }
}

fn multiply_bits(a: [u8; 35], b: [u8; 35]) -> [u8; 70] {
    let mut product = [0; 70];
    for multiplier_bit in 0..35 {
        let partial: [u8; 70] = std::array::from_fn(|bit| {
            if bit >= multiplier_bit && bit - multiplier_bit < 35 {
                and_gate(a[bit - multiplier_bit], b[multiplier_bit])
            } else {
                0
            }
        });
        product = add_bits(&product, &partial);
    }
    product
}

fn divide_bits(dividend: [u8; 70], divisor: [u8; 35]) -> ([u8; 35], [u8; 35]) {
    let divisor_wide = widen_bits::<35, 36>(divisor);
    let mut remainder = [0; 36];
    let mut quotient_wide = [0; 70];
    for dividend_bit in (0..70).rev() {
        for bit in (1..36).rev() {
            remainder[bit] = remainder[bit - 1];
        }
        remainder[0] = dividend[dividend_bit];
        let subtract = not_gate(greater_than_bits(&divisor_wide, &remainder));
        let difference = subtract_bits(&remainder, &divisor_wide);
        remainder = mux_bits(subtract, &remainder, &difference);
        quotient_wide[dividend_bit] = subtract;
    }
    let quotient: [u8; 35] = quotient_wide[..35].try_into().expect("bounded quotient");
    let remainder: [u8; 35] = remainder[..35].try_into().expect("bounded remainder");
    (quotient, remainder)
}

fn join_sign_magnitude(sign: u8, magnitude: [u8; 35]) -> [u8; 36] {
    let mut result = [0; 36];
    result[..35].copy_from_slice(&magnitude);
    result[35] = sign;
    result
}

fn widen_bits<const FROM: usize, const TO: usize>(bits: [u8; FROM]) -> [u8; TO] {
    assert!(FROM <= TO);
    std::array::from_fn(|bit| if bit < FROM { bits[bit] } else { 0 })
}

fn u64_to_bits<const WIDTH: usize>(value: u64) -> [u8; WIDTH] {
    std::array::from_fn(|bit| {
        if bit < u64::BITS as usize {
            ((value >> bit) & 1) as u8
        } else {
            0
        }
    })
}

fn bits_to_u64(bits: &[u8]) -> u64 {
    bits.iter()
        .take(u64::BITS as usize)
        .enumerate()
        .fold(0, |value, (bit, input)| value | u64::from(*input) << bit)
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

/// Add two masked sign-magnitude values through the 35-bit gate network.
pub fn add_sign_magnitude(sign_a: bool, mag_a: u64, sign_b: bool, mag_b: u64) -> (bool, u64, bool) {
    let (sign, magnitude, overflow) = add_sign_magnitude_bits(
        sign_a as u8,
        u64_to_bits::<35>(mag_a & MAGNITUDE_MASK),
        sign_b as u8,
        u64_to_bits::<35>(mag_b & MAGNITUDE_MASK),
    );
    (sign == 1, bits_to_u64(&magnitude), overflow == 1)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoders_are_one_hot() {
        for value in 0..8 {
            let outputs = decode_bits(u64_to_bits::<3>(value));
            assert_eq!(outputs.iter().sum::<u8>(), 1);
            assert_eq!(outputs[value as usize], 1);
        }
        for (opcode, operation) in OPERATION_TABLE {
            assert_eq!(
                decode_operation(u64_to_bits::<12>(opcode as u64)),
                Some(operation)
            );
        }
    }

    #[test]
    fn gate_arithmetic_networks_cover_carries_products_and_division() {
        assert_eq!(
            bits_to_u64(&add_bits(&u64_to_bits::<15>(32_767), &u64_to_bits::<15>(1))),
            0
        );
        assert_eq!(
            bits_to_u64(&subtract_bits(&u64_to_bits::<15>(3), &u64_to_bits::<15>(5))),
            32_766
        );
        assert_eq!(
            bits_to_u64(&multiply_bits(u64_to_bits::<35>(6), u64_to_bits::<35>(7))),
            42
        );
        let (quotient, remainder) = divide_bits(u64_to_bits::<70>(100), u64_to_bits::<35>(7));
        assert_eq!(bits_to_u64(&quotient), 14);
        assert_eq!(bits_to_u64(&remainder), 2);
    }

    #[test]
    fn bit_register_persists_only_clocked_data() {
        let mut value = BitRegister::<15>::new(&ZERO_15);
        value.write(&u64_to_bits::<15>(0x4567));
        assert_eq!(bits_to_u64(&value.read()), 0x4567);
    }
}
