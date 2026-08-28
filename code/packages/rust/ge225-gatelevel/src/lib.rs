//! Gate-level General Electric GE-225 simulator.
//!
//! Persistent words and registers are arrays of simulated D flip-flops.
//! Arithmetic and logic results use the repository's ripple-carry and primitive
//! gate networks. Host control flow sequences a clocked instruction cycle and
//! host integers identify memory addresses and trace fields.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};
use logic_gates::sequential::{register, FlipFlopState};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const MIN_MEMORY_WORDS: usize = 4_096;
pub const MAX_MEMORY_WORDS: usize = 16_384;
pub const WORD_MASK: i32 = (1 << 20) - 1;
pub const DATA_MASK: i32 = (1 << 19) - 1;
pub const SIGN_BIT: i32 = 1 << 19;
pub const ADDRESS_MASK: i32 = 0x1fff;
const X_MASK: i32 = 0x7fff;
const CLOCK_DAY_SIXTHS: i32 = 24 * 60 * 60 * 6;

/// Persistent non-memory bits in the P006B1 central binary/decimal/clock model.
pub const CENTRAL_FLIP_FLOPS: usize = 132;

#[derive(Clone, Debug, Eq, PartialEq)]
struct BitRegister<const WIDTH: usize> {
    state: [FlipFlopState; WIDTH],
}

type DecimalPairResult = ([u8; 20], [u8; 20], [u8; 2], u8);

impl<const WIDTH: usize> BitRegister<WIDTH> {
    fn zero() -> Self {
        Self::new(&[0; WIDTH])
    }

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
            .expect("a fixed-width gate register preserves its width")
    }

    fn write(&mut self, data: &[u8; WIDTH]) {
        register(data, 0, &mut self.state);
        register(data, 1, &mut self.state);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ge225GateState {
    pub a: i32,
    pub q: i32,
    pub m: i32,
    pub n: i32,
    pub pc: i32,
    pub ir: i32,
    pub overflow: bool,
    pub parity_error: bool,
    pub decimal_mode: bool,
    pub decimal_carry: i32,
    pub clock_sixths: i32,
    pub n_ready: bool,
    pub selected_x_group: usize,
    pub halted: bool,
    pub memory: Vec<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub pc_before: i32,
    pub pc_after: i32,
    pub instruction: i32,
    pub mnemonic: String,
    pub a_before: i32,
    pub a_after: i32,
    pub q_before: i32,
    pub q_after: i32,
    pub effective_address: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Ge225GateError {
    InvalidMemorySize { words: usize },
    InvalidOrigin { origin: usize },
    ProgramTooLarge { words: usize, capacity: usize },
    AddressOutOfRange { address: i32, capacity: usize },
    Halted,
    UnknownInstruction { word: i32, pc: i32 },
    InvalidAutomaticModification { word: i32 },
    ShiftCountOutOfRange { count: i32 },
    InvalidBcd { word: i32 },
    FlaggedDecimalOperand { double: bool },
    InvalidClock { value: i32 },
}

impl Display for Ge225GateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMemorySize { words } => write!(
                formatter,
                "memory must contain {MIN_MEMORY_WORDS}..={MAX_MEMORY_WORDS} words, got {words}"
            ),
            Self::InvalidOrigin { origin } => write!(formatter, "invalid load origin {origin}"),
            Self::ProgramTooLarge { words, capacity } => {
                write!(
                    formatter,
                    "program has {words} words but capacity is {capacity}"
                )
            }
            Self::AddressOutOfRange { address, capacity } => {
                write!(
                    formatter,
                    "address {address} is outside {capacity}-word memory"
                )
            }
            Self::Halted => write!(formatter, "the GE-225 gate-level simulator is halted"),
            Self::UnknownInstruction { word, pc } => {
                write!(formatter, "unknown GE-225 word {word:07o} at P={pc:05o}")
            }
            Self::InvalidAutomaticModification { word } => write!(
                formatter,
                "GE-225 automatic modification produced an invalid fixed word {word:07o}"
            ),
            Self::ShiftCountOutOfRange { count } => {
                write!(formatter, "modified GE-225 shift count exceeds 31: {count}")
            }
            Self::InvalidBcd { word } => {
                write!(
                    formatter,
                    "invalid GE-225 BCD digits in word {:07o}",
                    word & WORD_MASK
                )
            }
            Self::FlaggedDecimalOperand { double } => write!(
                formatter,
                "GE-225 {}decimal operand is flagged while A is unflagged",
                if *double { "double-" } else { "" }
            ),
            Self::InvalidClock { value } => {
                write!(
                    formatter,
                    "GE-225 clock must fit its 19-bit C register, got {value}"
                )
            }
        }
    }
}

impl Error for Ge225GateError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operation {
    Lda,
    Add,
    Sub,
    Sta,
    Bxl,
    Bxh,
    Ldx,
    Spb,
    Dld,
    Dad,
    Dsu,
    Dst,
    Inx,
    Mpy,
    Dvd,
    Stx,
    Ext,
    Cab,
    Dcb,
    Ory,
    Mov,
    Bru,
    Sto,
    Ldz,
    Ldo,
    Lmo,
    Cpl,
    Neg,
    Chs,
    Nop,
    Laq,
    Lqa,
    Xaq,
    Maq,
    Ado,
    Sbo,
    Lac,
    Lca,
    SetDecimalMode,
    SetBinaryMode,
    Bod,
    Bev,
    Bmi,
    Bpl,
    Bze,
    Bnz,
    Bov,
    Bno,
    Sra,
    Sna,
    Sca,
    San,
    Srd,
    Naq,
    Scd,
    Anq,
    Sla,
    Sld,
    Nor,
    Dno,
    Sxg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ge225GateLevel {
    memory: Vec<BitRegister<20>>,
    a: BitRegister<20>,
    q: BitRegister<20>,
    m: BitRegister<20>,
    n: BitRegister<6>,
    pc: BitRegister<15>,
    ir: BitRegister<20>,
    overflow: BitRegister<1>,
    parity_error: BitRegister<1>,
    decimal_mode: BitRegister<1>,
    decimal_carry: BitRegister<2>,
    clock_sixths: BitRegister<19>,
    n_ready: BitRegister<1>,
    selected_x_group: BitRegister<5>,
    halted: BitRegister<1>,
}

impl Ge225GateLevel {
    pub fn new(memory_words: usize) -> Result<Self, Ge225GateError> {
        if !(MIN_MEMORY_WORDS..=MAX_MEMORY_WORDS).contains(&memory_words) {
            return Err(Ge225GateError::InvalidMemorySize {
                words: memory_words,
            });
        }
        Ok(Self {
            memory: (0..memory_words).map(|_| BitRegister::zero()).collect(),
            a: BitRegister::zero(),
            q: BitRegister::zero(),
            m: BitRegister::zero(),
            n: BitRegister::zero(),
            pc: BitRegister::zero(),
            ir: BitRegister::zero(),
            overflow: BitRegister::zero(),
            parity_error: BitRegister::zero(),
            decimal_mode: BitRegister::zero(),
            decimal_carry: BitRegister::zero(),
            clock_sixths: BitRegister::zero(),
            n_ready: BitRegister::new(&[1]),
            selected_x_group: BitRegister::zero(),
            halted: BitRegister::zero(),
        })
    }

    pub fn reset(&mut self) {
        for word in &mut self.memory {
            word.write(&[0; 20]);
        }
        self.a.write(&[0; 20]);
        self.q.write(&[0; 20]);
        self.m.write(&[0; 20]);
        self.n.write(&[0; 6]);
        self.pc.write(&[0; 15]);
        self.ir.write(&[0; 20]);
        self.overflow.write(&[0]);
        self.parity_error.write(&[0]);
        self.decimal_mode.write(&[0]);
        self.decimal_carry.write(&[0; 2]);
        self.clock_sixths.write(&[0; 19]);
        self.n_ready.write(&[1]);
        self.selected_x_group.write(&[0; 5]);
        self.halted.write(&[0]);
    }

    pub fn load_words(&mut self, words: &[i32], origin: usize) -> Result<(), Ge225GateError> {
        if origin > self.memory.len() {
            return Err(Ge225GateError::InvalidOrigin { origin });
        }
        if words.len() > self.memory.len() - origin {
            return Err(Ge225GateError::ProgramTooLarge {
                words: words.len(),
                capacity: self.memory.len() - origin,
            });
        }
        for (destination, word) in self.memory[origin..].iter_mut().zip(words) {
            destination.write(&i32_to_bits::<20>(*word & WORD_MASK));
        }
        Ok(())
    }

    pub fn read_word(&self, address: i32) -> Result<i32, Ge225GateError> {
        let index = self.checked_address(address)?;
        Ok(bits_to_i32(&self.memory[index].read()))
    }

    pub fn write_word(&mut self, address: i32, word: i32) -> Result<(), Ge225GateError> {
        let index = self.checked_address(address)?;
        self.memory[index].write(&i32_to_bits::<20>(word & WORD_MASK));
        Ok(())
    }

    pub fn set_program_counter(&mut self, address: i32) -> Result<(), Ge225GateError> {
        self.checked_address(address)?;
        self.pc.write(&i32_to_bits::<15>(address));
        Ok(())
    }

    pub fn step(&mut self) -> Result<StepTrace, Ge225GateError> {
        if self.halted.read()[0] == 1 {
            return Err(Ge225GateError::Halted);
        }
        let pc_before = bits_to_i32(&self.pc.read());
        let instruction = self.read_word(pc_before)?;
        let (mut operation, mut modifier, mut address) =
            decode(instruction).ok_or(Ge225GateError::UnknownInstruction {
                word: instruction,
                pc: pc_before,
            })?;
        let mut ir_word = instruction;
        if is_fixed(operation) && modifier != 0 {
            if operation == Operation::Sxg {
                return Err(Ge225GateError::InvalidAutomaticModification { word: instruction });
            }
            let increment = self.x_word(modifier)? & ADDRESS_MASK;
            let modified_operand = (instruction + increment) & ADDRESS_MASK;
            ir_word = (instruction & !ADDRESS_MASK) | modified_operand;
            if is_shift(operation) {
                let modified_count = address + increment;
                if modified_count > 31 {
                    return Err(Ge225GateError::ShiftCountOutOfRange {
                        count: modified_count,
                    });
                }
                address = modified_count;
                modifier = 0;
            } else {
                let modified_word = (0o25 << 15) | modified_operand;
                let (modified_operation, modified_modifier, modified_address) =
                    decode(modified_word)
                        .filter(|(candidate, _, _)| is_fixed(*candidate))
                        .ok_or(Ge225GateError::InvalidAutomaticModification {
                            word: modified_word,
                        })?;
                operation = modified_operation;
                modifier = modified_modifier;
                address = modified_address;
            }
        }
        let sequential = pc_before + 1;
        if !matches!(operation, Operation::Bru | Operation::Spb) {
            self.checked_address(sequential)?;
        }

        let effective_address = if is_memory_reference(operation) {
            let effective = if operation == Operation::Bru && modifier == 0 {
                let target = (sequential & !ADDRESS_MASK) | address;
                self.checked_address(target)?;
                target
            } else {
                self.effective_address(address, modifier)?
            };
            Some(effective)
        } else {
            None
        };
        if matches!(
            operation,
            Operation::Dld | Operation::Dad | Operation::Dsu | Operation::Dst | Operation::Dcb
        ) {
            let pair_address = effective_address.ok_or(Ge225GateError::UnknownInstruction {
                word: instruction,
                pc: pc_before,
            })?;
            if pair_address & 1 == 0 {
                self.following_address(pair_address)?;
            }
        }
        if matches!(operation, Operation::Ldx | Operation::Stx) {
            self.checked_address(address)?;
        }
        if matches!(
            operation,
            Operation::Bxl
                | Operation::Bxh
                | Operation::Ldx
                | Operation::Spb
                | Operation::Inx
                | Operation::Stx
        ) {
            self.x_address(modifier)?;
        }
        if operation == Operation::Spb {
            self.checked_address((pc_before & !ADDRESS_MASK) | address)?;
        }
        if operation == Operation::Mov {
            let word_count = self.mov_word_count();
            self.checked_range(address, word_count)?;
            self.checked_range(bits_to_i32(&self.a.read()) & X_MASK, word_count)?;
            self.x_address(0)?;
        }
        let skip = self.skip_amount(operation, modifier, effective_address, address)?;
        if skip != 0 {
            self.checked_address(sequential + skip)?;
        }
        self.preflight_decimal(operation, effective_address)?;
        if ir_word == instruction && modifier != 0 {
            if let Some(modified) = effective_address
                .map(|effective| (instruction & !ADDRESS_MASK) | (effective & ADDRESS_MASK))
            {
                ir_word = modified;
            }
        }
        self.ir.write(&i32_to_bits::<20>(ir_word));
        self.pc.write(&i32_to_bits::<15>(sequential));
        let a_before = bits_to_i32(&self.a.read());
        let q_before = bits_to_i32(&self.q.read());
        let mnemonic = operation_name(operation).to_string();
        self.execute(operation, modifier, effective_address, address, pc_before)?;

        Ok(StepTrace {
            pc_before,
            pc_after: bits_to_i32(&self.pc.read()),
            instruction,
            mnemonic,
            a_before,
            a_after: bits_to_i32(&self.a.read()),
            q_before,
            q_after: bits_to_i32(&self.q.read()),
            effective_address,
        })
    }

    pub fn run(&mut self, max_steps: usize) -> Result<Vec<StepTrace>, Ge225GateError> {
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.halted.read()[0] == 1 {
                break;
            }
            traces.push(self.step()?);
        }
        Ok(traces)
    }

    pub fn get_state(&self) -> Ge225GateState {
        Ge225GateState {
            a: bits_to_i32(&self.a.read()),
            q: bits_to_i32(&self.q.read()),
            m: bits_to_i32(&self.m.read()),
            n: bits_to_i32(&self.n.read()),
            pc: bits_to_i32(&self.pc.read()),
            ir: bits_to_i32(&self.ir.read()),
            overflow: self.overflow.read()[0] == 1,
            parity_error: self.parity_error.read()[0] == 1,
            decimal_mode: self.decimal_mode.read()[0] == 1,
            decimal_carry: decode_decimal_carry(self.decimal_carry.read()),
            clock_sixths: bits_to_i32(&self.clock_sixths.read()),
            n_ready: self.n_ready.read()[0] == 1,
            selected_x_group: bits_to_i32(&self.selected_x_group.read()) as usize,
            halted: self.halted.read()[0] == 1,
            memory: self
                .memory
                .iter()
                .map(|word| bits_to_i32(&word.read()))
                .collect(),
        }
    }

    pub fn flip_flop_count(&self) -> usize {
        self.memory.len() * 20 + CENTRAL_FLIP_FLOPS
    }

    pub fn set_clock_sixths(&mut self, value: i32) -> Result<(), Ge225GateError> {
        if !(0..=DATA_MASK).contains(&value) {
            return Err(Ge225GateError::InvalidClock { value });
        }
        self.clock_sixths.write(&i32_to_bits::<19>(value));
        Ok(())
    }

    pub fn advance_clock_sixths(&mut self, ticks: u64) {
        let current = zero_extend::<19, 65>(self.clock_sixths.read());
        let ticks = u64_to_bits::<65>(ticks);
        let day = zero_extend::<20, 65>(i32_to_bits::<20>(CLOCK_DAY_SIXTHS));
        let word_modulus = zero_extend::<20, 65>(i32_to_bits::<20>(1 << 19));

        let normal_sum = gate_add(current, gate_divide_constant(ticks, CLOCK_DAY_SIXTHS).1).0;
        let normal = mux_bits(
            greater_or_equal(&normal_sum, &day),
            normal_sum,
            gate_subtract(normal_sum, day).0,
        );

        let until_word_wrap = gate_subtract(word_modulus, current).0;
        let before_word_wrap = gate_add(current, ticks).0;
        let after_word_wrap =
            gate_divide_constant(gate_subtract(ticks, until_word_wrap).0, CLOCK_DAY_SIXTHS).1;
        let exceptional = mux_bits(
            not_gate(greater_or_equal(&ticks, &until_word_wrap)),
            after_word_wrap,
            before_word_wrap,
        );
        let next = mux_bits(
            not_gate(greater_or_equal(&current, &day)),
            exceptional,
            normal,
        );
        let clock: [u8; 19] = next[..19]
            .try_into()
            .expect("the reduced GE-225 clock fits nineteen bits");
        self.clock_sixths.write(&clock);
    }

    pub fn clear_decimal_carry(&mut self) {
        self.decimal_carry.write(&[0; 2]);
    }

    fn preflight_decimal(
        &self,
        operation: Operation,
        effective_address: Option<i32>,
    ) -> Result<(), Ge225GateError> {
        if self.decimal_mode.read()[0] == 0 {
            return Ok(());
        }
        let carry = self.decimal_carry.read();
        match operation {
            Operation::Add | Operation::Sub => {
                let operand = self.read_word(effective_address.expect("memory operation"))?;
                gate_decimal_word(
                    self.a.read(),
                    i32_to_bits(operand),
                    operation == Operation::Sub,
                    carry,
                )?;
            }
            Operation::Dad | Operation::Dsu => {
                let address = effective_address.expect("memory operation");
                let first = self.read_word(address)?;
                let second = if address & 1 == 0 {
                    self.read_word(self.following_address(address)?)?
                } else {
                    first
                };
                gate_decimal_pair(
                    self.a.read(),
                    self.q.read(),
                    i32_to_bits(first),
                    i32_to_bits(second),
                    operation == Operation::Dsu,
                    carry,
                )?;
            }
            Operation::Ado | Operation::Sbo => {
                gate_decimal_word(
                    self.a.read(),
                    decimal_one_bits(),
                    operation == Operation::Sbo,
                    carry,
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    fn checked_address(&self, address: i32) -> Result<usize, Ge225GateError> {
        if address < 0 || address as usize >= self.memory.len() {
            return Err(Ge225GateError::AddressOutOfRange {
                address,
                capacity: self.memory.len(),
            });
        }
        Ok(address as usize)
    }

    fn effective_address(&self, address: i32, modifier: i32) -> Result<i32, Ge225GateError> {
        let effective = if modifier == 0 {
            address
        } else {
            let group = bits_to_i32(&self.selected_x_group.read()) as usize;
            let x_address = (group * 4 + modifier as usize) as i32;
            (address + (self.read_word(x_address)? & X_MASK)) & X_MASK
        };
        self.checked_address(effective)?;
        Ok(effective)
    }

    fn x_address(&self, modifier: i32) -> Result<i32, Ge225GateError> {
        let group = bits_to_i32(&self.selected_x_group.read());
        let address = group * 4 + modifier;
        self.checked_address(address)?;
        Ok(address)
    }

    fn x_word(&self, modifier: i32) -> Result<i32, Ge225GateError> {
        self.read_word(self.x_address(modifier)?)
    }

    fn set_x_word(&mut self, modifier: i32, value: i32) -> Result<(), Ge225GateError> {
        self.write_word(self.x_address(modifier)?, value)
    }

    fn following_address(&self, address: i32) -> Result<i32, Ge225GateError> {
        let following = address + 1;
        self.checked_address(following)?;
        Ok(following)
    }

    fn checked_range(
        &self,
        start: i32,
        word_count: usize,
    ) -> Result<std::ops::Range<usize>, Ge225GateError> {
        if start < 0 {
            return Err(Ge225GateError::AddressOutOfRange {
                address: start,
                capacity: self.memory.len(),
            });
        }
        let start = start as usize;
        let end = start
            .checked_add(word_count)
            .ok_or(Ge225GateError::AddressOutOfRange {
                address: i32::MAX,
                capacity: self.memory.len(),
            })?;
        if end > self.memory.len() {
            return Err(Ge225GateError::AddressOutOfRange {
                address: i32::try_from(end).unwrap_or(i32::MAX),
                capacity: self.memory.len(),
            });
        }
        Ok(start..end)
    }

    fn mov_word_count(&self) -> usize {
        let q = self.q.read();
        if q[19] == 0 {
            0
        } else {
            bits_to_i32(&gate_absolute(q)) as usize
        }
    }

    fn skip_amount(
        &self,
        operation: Operation,
        modifier: i32,
        effective_address: Option<i32>,
        raw_address: i32,
    ) -> Result<i32, Ge225GateError> {
        let a = self.a.read();
        Ok(match operation {
            Operation::Bxl => i32::from((self.x_word(modifier)? & ADDRESS_MASK) >= raw_address),
            Operation::Bxh => i32::from((self.x_word(modifier)? & ADDRESS_MASK) < raw_address),
            Operation::Cab => {
                let Some(address) = effective_address else {
                    return Ok(0);
                };
                let operand = i32_to_bits::<20>(self.read_word(address)?);
                match signed_compare(operand, a) {
                    0 => 1,
                    ordering if ordering < 0 => 2,
                    _ => 0,
                }
            }
            Operation::Dcb => {
                let Some(address) = effective_address else {
                    return Ok(0);
                };
                let first = self.read_word(address)?;
                let second = if address & 1 == 0 {
                    self.read_word(self.following_address(address)?)?
                } else {
                    first
                };
                let operand = join_double(i32_to_bits::<20>(first), i32_to_bits::<20>(second));
                match signed_compare(operand, join_double(a, self.q.read())) {
                    0 => 1,
                    ordering if ordering < 0 => 2,
                    _ => 0,
                }
            }
            Operation::Bod
            | Operation::Bev
            | Operation::Bmi
            | Operation::Bpl
            | Operation::Bze
            | Operation::Bnz
            | Operation::Bov
            | Operation::Bno => {
                let zero = is_zero(&a);
                let condition = match operation {
                    Operation::Bod => a[0],
                    Operation::Bev => not_gate(a[0]),
                    Operation::Bmi => a[19],
                    Operation::Bpl => not_gate(a[19]),
                    Operation::Bze => zero,
                    Operation::Bnz => not_gate(zero),
                    Operation::Bov => self.overflow.read()[0],
                    Operation::Bno => not_gate(self.overflow.read()[0]),
                    _ => unreachable!("the match only contains fixed branch tests"),
                };
                i32::from(condition == 0)
            }
            _ => 0,
        })
    }

    fn execute(
        &mut self,
        operation: Operation,
        modifier: i32,
        effective_address: Option<i32>,
        raw_address: i32,
        pc_before: i32,
    ) -> Result<(), Ge225GateError> {
        let address = effective_address.unwrap_or(raw_address);
        match operation {
            Operation::Lda => {
                let operand = self.read_word(address)?;
                self.m.write(&i32_to_bits::<20>(operand));
                self.a.write(&i32_to_bits::<20>(operand));
            }
            Operation::Add | Operation::Sub => {
                let operand = self.read_word(address)?;
                let left = self.a.read();
                let right = i32_to_bits::<20>(operand);
                let (result, carry, overflow) = if self.decimal_mode.read()[0] == 1 {
                    let (result, carry, overflow) = gate_decimal_word(
                        left,
                        right,
                        operation == Operation::Sub,
                        self.decimal_carry.read(),
                    )?;
                    (result, Some(carry), overflow)
                } else if operation == Operation::Add {
                    let (result, overflow) = gate_add(left, right);
                    (result, None, overflow)
                } else {
                    let (result, overflow) = gate_subtract(left, right);
                    (result, None, overflow)
                };
                self.m.write(&right);
                self.a.write(&result);
                if let Some(carry) = carry {
                    self.decimal_carry.write(&carry);
                }
                if overflow == 1 {
                    self.overflow.write(&[1]);
                }
            }
            Operation::Sta => {
                let value = self.a.read();
                let index = self.checked_address(address)?;
                self.memory[index].write(&value);
            }
            Operation::Bxl => {
                if (self.x_word(modifier)? & ADDRESS_MASK) >= raw_address {
                    self.advance_pc(1)?;
                }
            }
            Operation::Bxh => {
                if (self.x_word(modifier)? & ADDRESS_MASK) < raw_address {
                    self.advance_pc(1)?;
                }
            }
            Operation::Ldx => {
                let word = self.read_word(raw_address)?;
                self.set_x_word(modifier, word)?;
            }
            Operation::Spb => {
                let target = (pc_before & !ADDRESS_MASK) | raw_address;
                self.checked_address(target)?;
                self.set_x_word(modifier, pc_before)?;
                self.pc.write(&i32_to_bits::<15>(target));
            }
            Operation::Dld => {
                let first = self.read_word(address)?;
                let second = if address & 1 == 0 {
                    self.read_word(self.following_address(address)?)?
                } else {
                    first
                };
                self.a.write(&i32_to_bits::<20>(first));
                self.q.write(&i32_to_bits::<20>(second));
            }
            Operation::Dad | Operation::Dsu => {
                let first = self.read_word(address)?;
                let second = if address & 1 == 0 {
                    self.read_word(self.following_address(address)?)?
                } else {
                    first
                };
                if self.decimal_mode.read()[0] == 1 {
                    let (a, q, carry, overflow) = gate_decimal_pair(
                        self.a.read(),
                        self.q.read(),
                        i32_to_bits(first),
                        i32_to_bits(second),
                        operation == Operation::Dsu,
                        self.decimal_carry.read(),
                    )?;
                    self.a.write(&a);
                    self.q.write(&q);
                    self.decimal_carry.write(&carry);
                    if overflow == 1 {
                        self.overflow.write(&[1]);
                    }
                } else {
                    let left = join_double(self.a.read(), self.q.read());
                    let right = join_double(i32_to_bits::<20>(first), i32_to_bits::<20>(second));
                    let (result, overflow) = if operation == Operation::Dad {
                        gate_add(left, right)
                    } else {
                        gate_subtract(left, right)
                    };
                    let (a, q) = split_double(result);
                    self.a.write(&a);
                    self.q.write(&q);
                    if overflow == 1 {
                        self.overflow.write(&[1]);
                    }
                }
            }
            Operation::Dst => {
                if address & 1 == 0 {
                    let following = self.following_address(address)?;
                    self.write_word(address, bits_to_i32(&self.a.read()))?;
                    self.write_word(following, bits_to_i32(&self.q.read()))?;
                } else {
                    self.write_word(address, bits_to_i32(&self.q.read()))?;
                }
            }
            Operation::Inx => {
                let current = i32_to_bits::<15>(self.x_word(modifier)? & X_MASK);
                let increment = i32_to_bits::<15>(raw_address);
                let (updated, _) = gate_add(current, increment);
                let mut word = i32_to_bits::<20>(self.x_word(modifier)?);
                word[..15].copy_from_slice(&updated);
                self.set_x_word(modifier, bits_to_i32(&word))?;
            }
            Operation::Mpy => {
                let operand = self.read_word(address)?;
                self.m.write(&i32_to_bits::<20>(operand));
                let (product, overflow) =
                    gate_multiply_add(self.q.read(), i32_to_bits::<20>(operand), self.a.read());
                let (a, q) = split_double(product);
                self.a.write(&a);
                self.q.write(&q);
                self.overflow.write(&[overflow]);
            }
            Operation::Dvd => {
                let operand = self.read_word(address)?;
                let divisor = i32_to_bits::<20>(operand);
                self.m.write(&divisor);
                self.overflow.write(&[0]);
                let dividend = join_double(self.a.read(), self.q.read());
                if let Some((quotient, remainder)) = gate_divide(dividend, divisor) {
                    self.a.write(&quotient);
                    self.q.write(&remainder);
                } else {
                    self.overflow.write(&[1]);
                }
            }
            Operation::Stx => {
                self.write_word(raw_address, self.x_word(modifier)?)?;
            }
            Operation::Ext => {
                let operand = i32_to_bits::<20>(self.read_word(address)?);
                let result =
                    std::array::from_fn(|bit| and_gate(self.a.read()[bit], not_gate(operand[bit])));
                self.m.write(&operand);
                self.a.write(&result);
            }
            Operation::Cab => {
                let operand = i32_to_bits::<20>(self.read_word(address)?);
                self.m.write(&operand);
                let ordering = signed_compare(operand, self.a.read());
                if ordering == 0 {
                    self.advance_pc(1)?;
                } else if ordering < 0 {
                    self.advance_pc(2)?;
                }
            }
            Operation::Dcb => {
                let first = self.read_word(address)?;
                let second = if address & 1 == 0 {
                    self.read_word(self.following_address(address)?)?
                } else {
                    first
                };
                let operand = join_double(i32_to_bits::<20>(first), i32_to_bits::<20>(second));
                let accumulator = join_double(self.a.read(), self.q.read());
                let ordering = signed_compare(operand, accumulator);
                if ordering == 0 {
                    self.advance_pc(1)?;
                } else if ordering < 0 {
                    self.advance_pc(2)?;
                }
            }
            Operation::Ory => {
                let index = self.checked_address(address)?;
                let existing = self.memory[index].read();
                let a = self.a.read();
                self.memory[index]
                    .write(&std::array::from_fn(|bit| or_gate(existing[bit], a[bit])));
            }
            Operation::Mov => {
                let word_count = self.mov_word_count();
                let source = self.checked_range(raw_address, word_count)?;
                let destination = bits_to_i32(&self.a.read()) & X_MASK;
                let destination = self.checked_range(destination, word_count)?;
                let moved: Vec<[u8; 20]> = source.map(|index| self.memory[index].read()).collect();
                for (index, word) in destination.zip(moved) {
                    self.memory[index].write(&word);
                }
                self.set_x_word(0, bits_to_i32(&self.pc.read()))?;
                self.a.write(&[0; 20]);
            }
            Operation::Bru => self.pc.write(&i32_to_bits::<15>(address)),
            Operation::Sto => {
                let index = self.checked_address(address)?;
                let existing = self.memory[index].read();
                let a = self.a.read();
                self.memory[index].write(&std::array::from_fn(|bit| {
                    if bit < 13 {
                        a[bit]
                    } else {
                        existing[bit]
                    }
                }));
            }
            Operation::Ldz => self.a.write(&[0; 20]),
            Operation::Ldo => self.a.write(&i32_to_bits::<20>(1)),
            Operation::Lmo => self.a.write(&[1; 20]),
            Operation::Cpl => self
                .a
                .write(&std::array::from_fn(|bit| not_gate(self.a.read()[bit]))),
            Operation::Neg => {
                let before = self.a.read();
                let (result, overflow) = gate_subtract([0; 20], before);
                self.a.write(&result);
                if overflow == 1 {
                    self.overflow.write(&[1]);
                }
            }
            Operation::Chs => {
                let mut result = self.a.read();
                result[19] = not_gate(result[19]);
                self.a.write(&result);
            }
            Operation::Nop => {}
            Operation::Laq => self.a.write(&self.q.read()),
            Operation::Lqa => self.q.write(&self.a.read()),
            Operation::Xaq => {
                let a = self.a.read();
                let q = self.q.read();
                self.a.write(&q);
                self.q.write(&a);
            }
            Operation::Maq => {
                self.q.write(&self.a.read());
                self.a.write(&[0; 20]);
            }
            Operation::Ado | Operation::Sbo => {
                let before = self.a.read();
                let (result, carry, overflow) = if self.decimal_mode.read()[0] == 1 {
                    let (result, carry, overflow) = gate_decimal_word(
                        before,
                        decimal_one_bits(),
                        operation == Operation::Sbo,
                        self.decimal_carry.read(),
                    )?;
                    (result, Some(carry), overflow)
                } else if operation == Operation::Ado {
                    let (result, overflow) = gate_add(before, i32_to_bits::<20>(1));
                    (result, None, overflow)
                } else {
                    let (result, overflow) = gate_subtract(before, i32_to_bits::<20>(1));
                    (result, None, overflow)
                };
                self.a.write(&result);
                if let Some(carry) = carry {
                    self.decimal_carry.write(&carry);
                }
                if overflow == 1 {
                    self.overflow.write(&[1]);
                }
            }
            Operation::Lac => self.a.write(&with_sign_bits(self.clock_sixths.read(), 0)),
            Operation::Lca => {
                let clock: [u8; 19] = self.a.read()[..19]
                    .try_into()
                    .expect("the GE-225 clock receives A's nineteen data bits");
                self.clock_sixths.write(&clock);
            }
            Operation::SetDecimalMode => self.decimal_mode.write(&[1]),
            Operation::SetBinaryMode => self.decimal_mode.write(&[0]),
            Operation::Sra
            | Operation::Sna
            | Operation::Sca
            | Operation::San
            | Operation::Srd
            | Operation::Naq
            | Operation::Scd
            | Operation::Anq
            | Operation::Sla
            | Operation::Sld
            | Operation::Nor
            | Operation::Dno => self.execute_shift(operation, raw_address)?,
            Operation::Sxg => self.selected_x_group.write(&i32_to_bits::<5>(raw_address)),
            Operation::Bod
            | Operation::Bev
            | Operation::Bmi
            | Operation::Bpl
            | Operation::Bze
            | Operation::Bnz
            | Operation::Bov
            | Operation::Bno => {
                let a = self.a.read();
                let zero = is_zero(&a);
                let condition = match operation {
                    Operation::Bod => a[0],
                    Operation::Bev => not_gate(a[0]),
                    Operation::Bmi => a[19],
                    Operation::Bpl => not_gate(a[19]),
                    Operation::Bze => zero,
                    Operation::Bnz => not_gate(zero),
                    Operation::Bov => self.overflow.read()[0],
                    Operation::Bno => not_gate(self.overflow.read()[0]),
                    _ => 0,
                };
                if condition == 0 {
                    self.advance_pc(1)?;
                }
                if matches!(operation, Operation::Bov | Operation::Bno) {
                    self.overflow.write(&[0]);
                }
            }
        }
        Ok(())
    }

    fn execute_shift(&mut self, operation: Operation, count: i32) -> Result<(), Ge225GateError> {
        let count = count as usize;
        let a_before = self.a.read();
        let q_before = self.q.read();
        let a_sign = a_before[19];
        let q_sign = q_before[19];
        let mut a_data: [u8; 19] = a_before[..19]
            .try_into()
            .expect("the GE-225 A data field is nineteen bits");
        let mut q_data: [u8; 19] = q_before[..19]
            .try_into()
            .expect("the GE-225 Q data field is nineteen bits");
        match operation {
            Operation::Sra => {
                let mut shifted = a_before;
                for _ in 0..count.min(19) {
                    shifted = std::array::from_fn(|bit| {
                        if bit == 19 {
                            shifted[19]
                        } else {
                            shifted[bit + 1]
                        }
                    });
                }
                self.a.write(&shifted);
            }
            Operation::Sla => {
                for _ in 0..count {
                    if a_data[18] == 1 {
                        self.overflow.write(&[1]);
                    }
                    a_data = std::array::from_fn(|bit| if bit == 0 { 0 } else { a_data[bit - 1] });
                }
                self.a.write(&with_sign_bits(a_data, a_sign));
            }
            Operation::Sca => {
                for _ in 0..(count % 19) {
                    let low = a_data[0];
                    a_data =
                        std::array::from_fn(|bit| if bit == 18 { low } else { a_data[bit + 1] });
                }
                self.a.write(&with_sign_bits(a_data, a_sign));
            }
            Operation::San => {
                let mut n = self.n.read();
                for _ in 0..count {
                    let a_low = a_data[0];
                    a_data =
                        std::array::from_fn(|bit| if bit == 18 { a_sign } else { a_data[bit + 1] });
                    n = std::array::from_fn(|bit| if bit == 5 { a_low } else { n[bit + 1] });
                }
                self.a.write(&with_sign_bits(a_data, a_sign));
                self.n.write(&n);
            }
            Operation::Sna => {
                let mut n = self.n.read();
                for _ in 0..count {
                    let n_low = n[0];
                    a_data =
                        std::array::from_fn(|bit| if bit == 18 { n_low } else { a_data[bit + 1] });
                    n = std::array::from_fn(|bit| if bit == 5 { 0 } else { n[bit + 1] });
                }
                self.a.write(&with_sign_bits(a_data, a_sign));
                self.n.write(&n);
                self.n_ready.write(&[0]);
            }
            Operation::Srd => {
                for _ in 0..count {
                    let a_low = a_data[0];
                    a_data =
                        std::array::from_fn(|bit| if bit == 18 { a_sign } else { a_data[bit + 1] });
                    q_data =
                        std::array::from_fn(|bit| if bit == 18 { a_low } else { q_data[bit + 1] });
                }
                self.a.write(&with_sign_bits(a_data, a_sign));
                self.q.write(&with_sign_bits(q_data, a_sign));
            }
            Operation::Naq => {
                let mut n = self.n.read();
                for _ in 0..count {
                    let n_low = n[0];
                    let a_low = a_data[0];
                    a_data =
                        std::array::from_fn(|bit| if bit == 18 { n_low } else { a_data[bit + 1] });
                    q_data =
                        std::array::from_fn(|bit| if bit == 18 { a_low } else { q_data[bit + 1] });
                    n = std::array::from_fn(|bit| if bit == 5 { 0 } else { n[bit + 1] });
                }
                self.n.write(&n);
                self.a.write(&with_sign_bits(a_data, a_sign));
                self.q.write(&with_sign_bits(q_data, a_sign));
                self.n_ready.write(&[0]);
            }
            Operation::Scd => {
                for _ in 0..(count % 38) {
                    let low = q_data[0];
                    let a_low = a_data[0];
                    a_data =
                        std::array::from_fn(|bit| if bit == 18 { low } else { a_data[bit + 1] });
                    q_data =
                        std::array::from_fn(|bit| if bit == 18 { a_low } else { q_data[bit + 1] });
                }
                self.a.write(&with_sign_bits(a_data, a_sign));
                self.q.write(&with_sign_bits(q_data, a_sign));
            }
            Operation::Anq => {
                let mut a = a_before;
                let mut n = self.n.read();
                for _ in 0..count {
                    let bit = a[0];
                    a = std::array::from_fn(|position| {
                        if position == 19 {
                            a[19]
                        } else {
                            a[position + 1]
                        }
                    });
                    q_data = std::array::from_fn(|position| {
                        if position == 18 {
                            bit
                        } else {
                            q_data[position + 1]
                        }
                    });
                    n = std::array::from_fn(
                        |position| {
                            if position == 5 {
                                bit
                            } else {
                                n[position + 1]
                            }
                        },
                    );
                }
                self.a.write(&a);
                self.q.write(&with_sign_bits(q_data, a_sign));
                self.n.write(&n);
                self.n_ready.write(&[0]);
            }
            Operation::Sld => {
                for _ in 0..count {
                    if a_data[18] == 1 {
                        self.overflow.write(&[1]);
                    }
                    let q_high = q_data[18];
                    a_data =
                        std::array::from_fn(|bit| if bit == 0 { q_high } else { a_data[bit - 1] });
                    q_data = std::array::from_fn(|bit| if bit == 0 { 0 } else { q_data[bit - 1] });
                }
                self.a.write(&with_sign_bits(a_data, q_sign));
                self.q.write(&with_sign_bits(q_data, q_sign));
            }
            Operation::Nor => {
                let mut shifts = 0;
                while shifts < count && a_data[18] == a_sign {
                    if a_data[18] == 1 {
                        self.overflow.write(&[1]);
                    }
                    a_data = std::array::from_fn(|bit| if bit == 0 { 0 } else { a_data[bit - 1] });
                    shifts += 1;
                }
                self.a.write(&with_sign_bits(a_data, a_sign));
                self.write_word(0, (count - shifts) as i32)?;
            }
            Operation::Dno => {
                let mut shifts = 0;
                while shifts < count && a_data[18] == a_sign {
                    if a_data[18] == 1 {
                        self.overflow.write(&[1]);
                    }
                    let q_high = q_data[18];
                    a_data =
                        std::array::from_fn(|bit| if bit == 0 { q_high } else { a_data[bit - 1] });
                    q_data = std::array::from_fn(|bit| if bit == 0 { 0 } else { q_data[bit - 1] });
                    shifts += 1;
                }
                self.a.write(&with_sign_bits(a_data, q_sign));
                self.q.write(&with_sign_bits(q_data, q_sign));
                self.write_word(0, (count - shifts) as i32)?;
            }
            _ => unreachable!("only shift operations enter the gate shift network"),
        }
        Ok(())
    }

    fn advance_pc(&mut self, amount: i32) -> Result<(), Ge225GateError> {
        let next = bits_to_i32(&self.pc.read()) + amount;
        self.checked_address(next)?;
        self.pc.write(&i32_to_bits::<15>(next));
        Ok(())
    }
}

fn decode(word: i32) -> Option<(Operation, i32, i32)> {
    let normalized = word & WORD_MASK;
    let modifier = (normalized >> 13) & 0x03;
    let canonical = normalized & !(0x03 << 13);
    let canonical_bits = i32_to_bits::<20>(canonical);
    let fixed = FIXED_OPERATIONS.iter().find_map(|(code, operation)| {
        (equal_bits(&canonical_bits, &i32_to_bits::<20>(*code)) == 1).then_some(*operation)
    });
    if let Some(operation) = fixed {
        return Some((operation, modifier, 0));
    }
    let sxg_masked = canonical & !(0x1f << 3);
    if equal_bits(
        &i32_to_bits::<20>(sxg_masked),
        &i32_to_bits::<20>(0o2506003),
    ) == 1
    {
        return Some((Operation::Sxg, modifier, (canonical >> 3) & 0x1f));
    }
    let shift = SHIFT_OPERATIONS.iter().find_map(|(base, operation)| {
        let masked = canonical & !0o37;
        (equal_bits(&i32_to_bits::<20>(masked), &i32_to_bits::<20>(*base)) == 1)
            .then_some(*operation)
    });
    if let Some(operation) = shift {
        return Some((operation, modifier, canonical & 0o37));
    }
    let opcode_bits: [u8; 5] = canonical_bits[15..20]
        .try_into()
        .expect("the GE-225 opcode field is five bits");
    let selectors = decode_bits(opcode_bits);
    let operation = MEMORY_OPERATIONS
        .iter()
        .find_map(|(opcode, operation)| (selectors[*opcode] == 1).then_some(*operation))?;
    Some((operation, modifier, normalized & ADDRESS_MASK))
}

const FIXED_OPERATIONS: &[(i32, Operation)] = &[
    (0o2504002, Operation::Ldz),
    (0o2504022, Operation::Ldo),
    (0o2504102, Operation::Lmo),
    (0o2504502, Operation::Cpl),
    (0o2504522, Operation::Neg),
    (0o2504040, Operation::Chs),
    (0o2504012, Operation::Nop),
    (0o2504001, Operation::Laq),
    (0o2504004, Operation::Lqa),
    (0o2504005, Operation::Xaq),
    (0o2504006, Operation::Maq),
    (0o2504032, Operation::Ado),
    (0o2504112, Operation::Sbo),
    (0o2504202, Operation::Lac),
    (0o2504210, Operation::Lca),
    (0o2506011, Operation::SetDecimalMode),
    (0o2506012, Operation::SetBinaryMode),
    (0o2514000, Operation::Bod),
    (0o2516000, Operation::Bev),
    (0o2514001, Operation::Bmi),
    (0o2516001, Operation::Bpl),
    (0o2514002, Operation::Bze),
    (0o2516002, Operation::Bnz),
    (0o2514003, Operation::Bov),
    (0o2516003, Operation::Bno),
];

const MEMORY_OPERATIONS: &[(usize, Operation)] = &[
    (0o00, Operation::Lda),
    (0o01, Operation::Add),
    (0o02, Operation::Sub),
    (0o03, Operation::Sta),
    (0o04, Operation::Bxl),
    (0o05, Operation::Bxh),
    (0o06, Operation::Ldx),
    (0o07, Operation::Spb),
    (0o10, Operation::Dld),
    (0o11, Operation::Dad),
    (0o12, Operation::Dsu),
    (0o13, Operation::Dst),
    (0o14, Operation::Inx),
    (0o15, Operation::Mpy),
    (0o16, Operation::Dvd),
    (0o17, Operation::Stx),
    (0o20, Operation::Ext),
    (0o21, Operation::Cab),
    (0o22, Operation::Dcb),
    (0o23, Operation::Ory),
    (0o24, Operation::Mov),
    (0o26, Operation::Bru),
    (0o27, Operation::Sto),
];

const SHIFT_OPERATIONS: &[(i32, Operation)] = &[
    (0o2510000, Operation::Sra),
    (0o2510100, Operation::Sna),
    (0o2510040, Operation::Sca),
    (0o2510400, Operation::San),
    (0o2511000, Operation::Srd),
    (0o2511100, Operation::Naq),
    (0o2511200, Operation::Scd),
    (0o2511400, Operation::Anq),
    (0o2512000, Operation::Sla),
    (0o2512200, Operation::Sld),
    (0o2513000, Operation::Nor),
    (0o2513200, Operation::Dno),
];

fn is_fixed(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::Ldz
            | Operation::Ldo
            | Operation::Lmo
            | Operation::Cpl
            | Operation::Neg
            | Operation::Chs
            | Operation::Nop
            | Operation::Laq
            | Operation::Lqa
            | Operation::Xaq
            | Operation::Maq
            | Operation::Ado
            | Operation::Sbo
            | Operation::Lac
            | Operation::Lca
            | Operation::SetDecimalMode
            | Operation::SetBinaryMode
            | Operation::Bod
            | Operation::Bev
            | Operation::Bmi
            | Operation::Bpl
            | Operation::Bze
            | Operation::Bnz
            | Operation::Bov
            | Operation::Bno
            | Operation::Sra
            | Operation::Sna
            | Operation::Sca
            | Operation::San
            | Operation::Srd
            | Operation::Naq
            | Operation::Scd
            | Operation::Anq
            | Operation::Sla
            | Operation::Sld
            | Operation::Nor
            | Operation::Dno
            | Operation::Sxg
    )
}

fn is_shift(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::Sra
            | Operation::Sna
            | Operation::Sca
            | Operation::San
            | Operation::Srd
            | Operation::Naq
            | Operation::Scd
            | Operation::Anq
            | Operation::Sla
            | Operation::Sld
            | Operation::Nor
            | Operation::Dno
    )
}

fn is_memory_reference(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::Lda
            | Operation::Add
            | Operation::Sub
            | Operation::Sta
            | Operation::Dld
            | Operation::Dad
            | Operation::Dsu
            | Operation::Dst
            | Operation::Mpy
            | Operation::Dvd
            | Operation::Ext
            | Operation::Cab
            | Operation::Dcb
            | Operation::Ory
            | Operation::Bru
            | Operation::Sto
    )
}

fn operation_name(operation: Operation) -> &'static str {
    match operation {
        Operation::Lda => "LDA",
        Operation::Add => "ADD",
        Operation::Sub => "SUB",
        Operation::Sta => "STA",
        Operation::Bxl => "BXL",
        Operation::Bxh => "BXH",
        Operation::Ldx => "LDX",
        Operation::Spb => "SPB",
        Operation::Dld => "DLD",
        Operation::Dad => "DAD",
        Operation::Dsu => "DSU",
        Operation::Dst => "DST",
        Operation::Inx => "INX",
        Operation::Mpy => "MPY",
        Operation::Dvd => "DVD",
        Operation::Stx => "STX",
        Operation::Ext => "EXT",
        Operation::Cab => "CAB",
        Operation::Dcb => "DCB",
        Operation::Ory => "ORY",
        Operation::Mov => "MOV",
        Operation::Bru => "BRU",
        Operation::Sto => "STO",
        Operation::Ldz => "LDZ",
        Operation::Ldo => "LDO",
        Operation::Lmo => "LMO",
        Operation::Cpl => "CPL",
        Operation::Neg => "NEG",
        Operation::Chs => "CHS",
        Operation::Nop => "NOP",
        Operation::Laq => "LAQ",
        Operation::Lqa => "LQA",
        Operation::Xaq => "XAQ",
        Operation::Maq => "MAQ",
        Operation::Ado => "ADO",
        Operation::Sbo => "SBO",
        Operation::Lac => "LAC",
        Operation::Lca => "LCA",
        Operation::SetDecimalMode => "SET_DECMODE",
        Operation::SetBinaryMode => "SET_BINMODE",
        Operation::Bod => "BOD",
        Operation::Bev => "BEV",
        Operation::Bmi => "BMI",
        Operation::Bpl => "BPL",
        Operation::Bze => "BZE",
        Operation::Bnz => "BNZ",
        Operation::Bov => "BOV",
        Operation::Bno => "BNO",
        Operation::Sra => "SRA",
        Operation::Sna => "SNA",
        Operation::Sca => "SCA",
        Operation::San => "SAN",
        Operation::Srd => "SRD",
        Operation::Naq => "NAQ",
        Operation::Scd => "SCD",
        Operation::Anq => "ANQ",
        Operation::Sla => "SLA",
        Operation::Sld => "SLD",
        Operation::Nor => "NOR",
        Operation::Dno => "DNO",
        Operation::Sxg => "SXG",
    }
}

fn decimal_one_bits() -> [u8; 20] {
    let mut word = [0; 20];
    word[0] = 1;
    word
}

fn gate_decimal_digits<const WIDTH: usize>(word: [u8; 20]) -> Result<[u8; WIDTH], Ge225GateError> {
    let ones: [u8; 4] = word[..4].try_into().expect("ones digit width");
    let tens: [u8; 4] = word[6..10].try_into().expect("tens digit width");
    let hundreds: [u8; 4] = word[12..16].try_into().expect("hundreds digit width");
    let ten = i32_to_bits::<4>(10);
    if [ones, tens, hundreds]
        .iter()
        .any(|digit| greater_or_equal(digit, &ten) == 1)
    {
        return Err(Ge225GateError::InvalidBcd {
            word: bits_to_i32(&word),
        });
    }
    let ones = zero_extend::<4, WIDTH>(ones);
    let tens = gate_multiply_constant(zero_extend::<4, WIDTH>(tens), 10);
    let hundreds = gate_multiply_constant(zero_extend::<4, WIDTH>(hundreds), 100);
    Ok(gate_add(gate_add(hundreds, tens).0, ones).0)
}

fn gate_decimal_word(
    accumulator: [u8; 20],
    operand: [u8; 20],
    subtract: bool,
    carry: [u8; 2],
) -> Result<([u8; 20], [u8; 2], u8), Ge225GateError> {
    let left_raw = gate_decimal_digits::<13>(accumulator)?;
    let right_raw = gate_decimal_digits::<13>(operand)?;
    let flagged = accumulator[18];
    if operand[18] == 1 && flagged == 0 {
        return Err(Ge225GateError::FlaggedDecimalOperand { double: false });
    }
    let total = gate_decimal_total(
        left_raw,
        and_gate(accumulator[19], flagged),
        right_raw,
        and_gate(operand[19], flagged),
        subtract,
        carry,
        1_000,
    );
    let (raw, negative, next_carry, overflow) = gate_normalize_decimal(total, flagged, 1_000);
    Ok((
        gate_encode_decimal(raw, negative, flagged),
        next_carry,
        overflow,
    ))
}

fn gate_decimal_pair(
    a: [u8; 20],
    q: [u8; 20],
    high_operand: [u8; 20],
    low_operand: [u8; 20],
    subtract: bool,
    carry: [u8; 2],
) -> Result<DecimalPairResult, Ge225GateError> {
    let a_high = gate_decimal_digits::<23>(a)?;
    let a_low = gate_decimal_digits::<23>(q)?;
    let operand_high = gate_decimal_digits::<23>(high_operand)?;
    let operand_low = gate_decimal_digits::<23>(low_operand)?;
    let flagged = a[18];
    if high_operand[18] == 1 && flagged == 0 {
        return Err(Ge225GateError::FlaggedDecimalOperand { double: true });
    }
    let left_raw = gate_add(gate_multiply_constant(a_high, 1_000), a_low).0;
    let right_raw = gate_add(gate_multiply_constant(operand_high, 1_000), operand_low).0;
    let total = gate_decimal_total(
        left_raw,
        and_gate(a[19], flagged),
        right_raw,
        and_gate(high_operand[19], flagged),
        subtract,
        carry,
        1_000_000,
    );
    let (raw, negative, next_carry, overflow) = gate_normalize_decimal(total, flagged, 1_000_000);
    let (high, low) = gate_divide_constant(raw, 1_000);
    Ok((
        gate_encode_decimal(high, negative, flagged),
        gate_encode_decimal(low, 0, 0),
        next_carry,
        overflow,
    ))
}

fn gate_decimal_total<const WIDTH: usize>(
    left_raw: [u8; WIDTH],
    left_negative: u8,
    right_raw: [u8; WIDTH],
    right_negative: u8,
    subtract: bool,
    carry: [u8; 2],
    modulus: i32,
) -> [u8; WIDTH] {
    let modulus_bits = i32_to_bits::<WIDTH>(modulus);
    let left_signed = mux_bits(
        and_gate(left_negative, not_gate(is_zero(&left_raw))),
        left_raw,
        gate_subtract(left_raw, modulus_bits).0,
    );
    let right_signed = mux_bits(
        and_gate(right_negative, not_gate(is_zero(&right_raw))),
        right_raw,
        gate_subtract(right_raw, modulus_bits).0,
    );
    let combined = if subtract {
        gate_subtract(left_signed, right_signed).0
    } else {
        gate_add(left_signed, right_signed).0
    };
    gate_add(combined, sign_extend::<2, WIDTH>(carry)).0
}

fn gate_normalize_decimal<const WIDTH: usize>(
    total: [u8; WIDTH],
    flagged: u8,
    modulus: i32,
) -> ([u8; WIDTH], u8, [u8; 2], u8) {
    let modulus_bits = i32_to_bits::<WIDTH>(modulus);
    let twice_modulus = i32_to_bits::<WIDTH>(modulus * 2);
    let negative_modulus = gate_subtract([0; WIDTH], modulus_bits).0;
    let at_least_modulus = signed_greater_or_equal(&total, &modulus_bits);
    let at_most_negative_modulus = signed_greater_or_equal(&negative_modulus, &total);
    let negative = total[WIDTH - 1];
    let middle_negative = and_gate(negative, not_gate(at_most_negative_modulus));

    let plus_modulus = gate_add(total, modulus_bits).0;
    let plus_twice_modulus = gate_add(total, twice_modulus).0;
    let minus_modulus = gate_subtract(total, modulus_bits).0;
    let mut raw = mux_bits(middle_negative, total, plus_modulus);
    raw = mux_bits(at_most_negative_modulus, raw, plus_twice_modulus);
    raw = mux_bits(at_least_modulus, raw, minus_modulus);

    let flagged_negative = or_gate(
        at_least_modulus,
        and_gate(negative, not_gate(at_most_negative_modulus)),
    );
    let overflow = and_gate(flagged, or_gate(at_least_modulus, at_most_negative_modulus));
    let unflagged = not_gate(flagged);
    let positive_carry = and_gate(unflagged, at_least_modulus);
    let negative_carry = and_gate(unflagged, negative);
    let carry = [or_gate(positive_carry, negative_carry), negative_carry];
    (raw, and_gate(flagged, flagged_negative), carry, overflow)
}

fn gate_encode_decimal<const WIDTH: usize>(
    raw: [u8; WIDTH],
    negative: u8,
    flagged: u8,
) -> [u8; 20] {
    let (hundreds, remainder) = gate_divide_constant(raw, 100);
    let (tens, ones) = gate_divide_constant(remainder, 10);
    let mut word = [0; 20];
    word[..4].copy_from_slice(&ones[..4]);
    word[6..10].copy_from_slice(&tens[..4]);
    word[12..16].copy_from_slice(&hundreds[..4]);
    word[18] = flagged;
    word[19] = negative;
    word
}

fn gate_divide_constant<const WIDTH: usize>(
    dividend: [u8; WIDTH],
    divisor: i32,
) -> ([u8; WIDTH], [u8; WIDTH]) {
    let divisor = u64_to_bits::<WIDTH>(divisor as u64);
    let mut quotient = [0; WIDTH];
    let mut remainder = [0; WIDTH];
    for bit in (0..WIDTH).rev() {
        remainder = std::array::from_fn(|position| {
            if position == 0 {
                dividend[bit]
            } else {
                remainder[position - 1]
            }
        });
        let subtract = greater_or_equal(&remainder, &divisor);
        remainder = mux_bits(subtract, remainder, gate_subtract(remainder, divisor).0);
        quotient[bit] = subtract;
    }
    (quotient, remainder)
}

fn gate_multiply_constant<const WIDTH: usize>(value: [u8; WIDTH], multiplier: u32) -> [u8; WIDTH] {
    let mut product = [0; WIDTH];
    for bit in 0..32 {
        if (multiplier >> bit) & 1 == 1 {
            let partial = std::array::from_fn(|position| {
                if position >= bit {
                    value[position - bit]
                } else {
                    0
                }
            });
            product = gate_add(product, partial).0;
        }
    }
    product
}

fn mux_bits<const WIDTH: usize>(select: u8, zero: [u8; WIDTH], one: [u8; WIDTH]) -> [u8; WIDTH] {
    std::array::from_fn(|bit| {
        or_gate(
            and_gate(not_gate(select), zero[bit]),
            and_gate(select, one[bit]),
        )
    })
}

fn zero_extend<const FROM: usize, const TO: usize>(value: [u8; FROM]) -> [u8; TO] {
    std::array::from_fn(|bit| if bit < FROM { value[bit] } else { 0 })
}

fn sign_extend<const FROM: usize, const TO: usize>(value: [u8; FROM]) -> [u8; TO] {
    std::array::from_fn(|bit| {
        if bit < FROM {
            value[bit]
        } else {
            value[FROM - 1]
        }
    })
}

fn signed_greater_or_equal<const WIDTH: usize>(left: &[u8; WIDTH], right: &[u8; WIDTH]) -> u8 {
    let signs_differ = xor_gate(left[WIDTH - 1], right[WIDTH - 1]);
    let left_positive = and_gate(not_gate(left[WIDTH - 1]), right[WIDTH - 1]);
    or_gate(
        and_gate(signs_differ, left_positive),
        and_gate(not_gate(signs_differ), greater_or_equal(left, right)),
    )
}

fn decode_decimal_carry(bits: [u8; 2]) -> i32 {
    match bits {
        [1, 1] => -1,
        [1, 0] => 1,
        _ => 0,
    }
}

fn gate_add<const WIDTH: usize>(left: [u8; WIDTH], right: [u8; WIDTH]) -> ([u8; WIDTH], u8) {
    let result = ripple_carry_adder_with_carry(&left, &right, 0);
    let sum: [u8; WIDTH] = result.sum.try_into().expect("ripple adder preserves width");
    let overflow = and_gate(
        not_gate(xor_gate(left[WIDTH - 1], right[WIDTH - 1])),
        xor_gate(left[WIDTH - 1], sum[WIDTH - 1]),
    );
    (sum, overflow)
}

fn gate_subtract<const WIDTH: usize>(left: [u8; WIDTH], right: [u8; WIDTH]) -> ([u8; WIDTH], u8) {
    let inverted: [u8; WIDTH] = std::array::from_fn(|bit| not_gate(right[bit]));
    let result = ripple_carry_adder_with_carry(&left, &inverted, 1);
    let difference: [u8; WIDTH] = result.sum.try_into().expect("ripple adder preserves width");
    let overflow = and_gate(
        xor_gate(left[WIDTH - 1], right[WIDTH - 1]),
        xor_gate(left[WIDTH - 1], difference[WIDTH - 1]),
    );
    (difference, overflow)
}

fn join_double(a: [u8; 20], q: [u8; 20]) -> [u8; 39] {
    std::array::from_fn(|bit| if bit < 19 { q[bit] } else { a[bit - 19] })
}

fn split_double(value: [u8; 39]) -> ([u8; 20], [u8; 20]) {
    let a = std::array::from_fn(|bit| value[bit + 19]);
    let mut q = [0; 20];
    q[..19].copy_from_slice(&value[..19]);
    q[19] = a[19];
    (a, q)
}

fn with_sign_bits(data: [u8; 19], sign: u8) -> [u8; 20] {
    std::array::from_fn(|bit| if bit == 19 { sign } else { data[bit] })
}

fn gate_twos_complement<const WIDTH: usize>(value: [u8; WIDTH]) -> [u8; WIDTH] {
    gate_subtract([0; WIDTH], value).0
}

fn gate_absolute<const WIDTH: usize>(value: [u8; WIDTH]) -> [u8; WIDTH] {
    let negative = value[WIDTH - 1];
    let complemented = gate_twos_complement(value);
    std::array::from_fn(|bit| {
        or_gate(
            and_gate(not_gate(negative), value[bit]),
            and_gate(negative, complemented[bit]),
        )
    })
}

fn gate_multiply_add(q: [u8; 20], operand: [u8; 20], a: [u8; 20]) -> ([u8; 39], u8) {
    let q_magnitude = gate_absolute(q);
    let operand_magnitude = gate_absolute(operand);
    let mut product = [0; 39];
    for multiplier_bit in 0..19 {
        let partial = std::array::from_fn(|bit| {
            if bit >= multiplier_bit && bit - multiplier_bit < 19 {
                and_gate(
                    q_magnitude[bit - multiplier_bit],
                    operand_magnitude[multiplier_bit],
                )
            } else {
                0
            }
        });
        product = gate_add(product, partial).0;
    }
    let product_sign = xor_gate(q[19], operand[19]);
    let negative_product = gate_twos_complement(product);
    product = std::array::from_fn(|bit| {
        or_gate(
            and_gate(not_gate(product_sign), product[bit]),
            and_gate(product_sign, negative_product[bit]),
        )
    });
    let a_extended = std::array::from_fn(|bit| if bit < 20 { a[bit] } else { a[19] });
    gate_add(product, a_extended)
}

fn gate_divide(dividend: [u8; 39], divisor: [u8; 20]) -> Option<([u8; 20], [u8; 20])> {
    let divisor_magnitude = gate_absolute(divisor);
    if is_zero(&divisor_magnitude) == 1 {
        return None;
    }
    let high: [u8; 20] = dividend[19..39]
        .try_into()
        .expect("the GE-225 double high half is 20 bits");
    if greater_or_equal(&gate_absolute(high), &divisor_magnitude) == 1 {
        return None;
    }

    let dividend_sign = dividend[38];
    let divisor_sign = divisor[19];
    let dividend_magnitude = gate_absolute(dividend);
    let divisor_wide: [u8; 21] =
        std::array::from_fn(|bit| if bit < 20 { divisor_magnitude[bit] } else { 0 });
    let mut remainder = [0; 21];
    let mut quotient_wide = [0; 39];
    for dividend_bit in (0..39).rev() {
        for bit in (1..21).rev() {
            remainder[bit] = remainder[bit - 1];
        }
        remainder[0] = dividend_magnitude[dividend_bit];
        let subtract = greater_or_equal(&remainder, &divisor_wide);
        let difference = gate_subtract(remainder, divisor_wide).0;
        remainder = std::array::from_fn(|bit| {
            or_gate(
                and_gate(not_gate(subtract), remainder[bit]),
                and_gate(subtract, difference[bit]),
            )
        });
        quotient_wide[dividend_bit] = subtract;
    }

    let quotient_magnitude: [u8; 20] = quotient_wide[..20]
        .try_into()
        .expect("the preflight-bounded GE-225 quotient is 20 bits");
    let remainder_magnitude: [u8; 20] = remainder[..20]
        .try_into()
        .expect("the GE-225 divide remainder is 20 bits");
    let result_sign = xor_gate(dividend_sign, divisor_sign);
    let negative_quotient = gate_twos_complement(quotient_magnitude);
    let negative_remainder = gate_twos_complement(remainder_magnitude);
    let quotient = std::array::from_fn(|bit| {
        or_gate(
            and_gate(not_gate(result_sign), quotient_magnitude[bit]),
            and_gate(result_sign, negative_quotient[bit]),
        )
    });
    let remainder = std::array::from_fn(|bit| {
        or_gate(
            and_gate(not_gate(result_sign), remainder_magnitude[bit]),
            and_gate(result_sign, negative_remainder[bit]),
        )
    });
    Some((quotient, remainder))
}

fn greater_or_equal<const WIDTH: usize>(left: &[u8; WIDTH], right: &[u8; WIDTH]) -> u8 {
    let mut greater = 0;
    let mut equal = 1;
    for bit in (0..WIDTH).rev() {
        greater = or_gate(
            greater,
            and_gate(equal, and_gate(left[bit], not_gate(right[bit]))),
        );
        equal = and_gate(equal, not_gate(xor_gate(left[bit], right[bit])));
    }
    or_gate(greater, equal)
}

fn signed_compare<const WIDTH: usize>(left: [u8; WIDTH], right: [u8; WIDTH]) -> i8 {
    if left == right {
        return 0;
    }
    let left_sign = left[WIDTH - 1];
    let right_sign = right[WIDTH - 1];
    if left_sign != right_sign {
        return if left_sign == 1 { -1 } else { 1 };
    }
    for bit in (0..WIDTH - 1).rev() {
        if left[bit] != right[bit] {
            let greater = if left[bit] == 1 { 1 } else { -1 };
            return if left_sign == 1 { -greater } else { greater };
        }
    }
    0
}

fn is_zero<const WIDTH: usize>(bits: &[u8; WIDTH]) -> u8 {
    bits.iter()
        .fold(1, |zero, bit| and_gate(zero, not_gate(*bit)))
}

fn equal_bits<const WIDTH: usize>(left: &[u8; WIDTH], right: &[u8; WIDTH]) -> u8 {
    left.iter().zip(right).fold(1, |equal, (a, b)| {
        and_gate(equal, not_gate(xor_gate(*a, *b)))
    })
}

fn decode_bits<const WIDTH: usize>(bits: [u8; WIDTH]) -> Vec<u8> {
    (0..(1usize << WIDTH))
        .map(|value| equal_bits(&bits, &i32_to_bits::<WIDTH>(value as i32)))
        .collect()
}

fn i32_to_bits<const WIDTH: usize>(value: i32) -> [u8; WIDTH] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn u64_to_bits<const WIDTH: usize>(value: u64) -> [u8; WIDTH] {
    std::array::from_fn(|bit| {
        if bit < 64 {
            ((value >> bit) & 1) as u8
        } else {
            0
        }
    })
}

fn bits_to_i32(bits: &[u8]) -> i32 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, input)| value | i32::from(*input) << bit)
}

pub fn encode_instruction(opcode: i32, modifier: i32, address: i32) -> Option<i32> {
    if !(0..=0o37).contains(&opcode)
        || !(0..=3).contains(&modifier)
        || !(0..=ADDRESS_MASK).contains(&address)
    {
        return None;
    }
    Some((opcode << 15) | (modifier << 13) | address)
}
