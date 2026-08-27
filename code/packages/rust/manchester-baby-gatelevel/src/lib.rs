//! Gate-level Manchester Baby (SSEM) simulator.
//!
//! The 1,062 persistent bits are held in D flip-flops. Instruction selection
//! uses NOT/AND/OR gates, while increments, negation, and subtraction flow
//! through ripple-carry adders. Host control flow sequences clock edges and
//! selects a store line, but never calculates an architectural data result.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use arithmetic::adders::{ripple_carry_adder, ripple_carry_adder_with_carry};
use logic_gates::gates::{and_gate, not_gate, or_gate};
use logic_gates::sequential::{register, FlipFlopState};

/// Number of 32-bit lines in the Williams-tube store.
pub const STORE_WORDS: usize = 32;

/// Exact number of simulated persistent state bits.
pub const FLIP_FLOP_COUNT: usize = STORE_WORDS * 32 + 32 + 5 + 1;

/// Educational gate-equivalent topology estimate.
///
/// This combines six primitive gates per flip-flop with the arithmetic,
/// opcode decoder, address selection, data mux, and control estimates from
/// Spec 07l2. The storage count is exact; this combinational count is not a
/// die-accurate reconstruction of the Williams-tube machine.
pub const ESTIMATED_GATE_COUNT: usize = 8_858;

const INITIAL_CI_BITS: [u8; 5] = [1; 5];
const ONE_5: [u8; 5] = [1, 0, 0, 0, 0];
const ZERO_32: [u8; 32] = [0; 32];

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
        let output = register(&[0; WIDTH], 0, &mut state);
        output
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

    /// Return the word at the current five-bit control-instruction line.
    pub fn present_instruction(&self) -> u32 {
        self.store[(self.ci & 0x1f) as usize]
    }
}

/// One gate-level fetch-decode-execute cycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub pc_before: u8,
    pub pc_after: u8,
    pub instruction: u32,
    pub mnemonic: String,
    pub description: String,
}

/// Result of a bounded run that reached STP.
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

/// Manchester Baby whose state and data paths are built from digital logic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManchesterBabyGateLevel {
    store: [BitRegister<32>; STORE_WORDS],
    accumulator: BitRegister<32>,
    ci: BitRegister<5>,
    halted: BitRegister<1>,
}

impl Default for ManchesterBabyGateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl ManchesterBabyGateLevel {
    /// Construct a machine in its documented power-on state.
    pub fn new() -> Self {
        Self {
            store: std::array::from_fn(|_| BitRegister::new(&ZERO_32)),
            accumulator: BitRegister::new(&ZERO_32),
            ci: BitRegister::new(&INITIAL_CI_BITS),
            halted: BitRegister::new(&[0]),
        }
    }

    /// Restore every state element through a flip-flop register write.
    pub fn reset(&mut self) {
        for word in &mut self.store {
            word.write(&ZERO_32);
        }
        self.accumulator.write(&ZERO_32);
        self.ci.write(&INITIAL_CI_BITS);
        self.halted.write(&[0]);
    }

    /// Load complete little-endian words at a word-addressed origin.
    ///
    /// An incomplete trailing word is ignored and input beyond line 31 is not
    /// written. The returned count is the number of words actually clocked in.
    pub fn load(&mut self, program: &[u8], origin: usize) -> Result<usize, BabyError> {
        if origin >= STORE_WORDS {
            return Err(BabyError::InvalidOrigin { origin });
        }

        let (complete_words, _trailing_bytes) = program.as_chunks::<4>();
        let mut loaded = 0;
        for (offset, bytes) in complete_words.iter().take(STORE_WORDS - origin).enumerate() {
            self.store[origin + offset].write(&u32_to_bits(u32::from_le_bytes(*bytes)));
            loaded += 1;
        }
        Ok(loaded)
    }

    /// Execute one pre-increment, fetch, one-hot decode, and clock cycle.
    pub fn step(&mut self) -> Result<StepTrace, BabyError> {
        if self.halted.read()[0] == 1 {
            return Err(BabyError::Halted);
        }

        let ci_before_bits = self.ci.read();
        let ci_before = bits_to_u32(&ci_before_bits) as u8;
        let incremented = add_bits(&ci_before_bits, &ONE_5);
        self.ci.write(&incremented);

        let fetched_line = bits_to_u32(&incremented) as usize;
        let instruction_bits = self.store[fetched_line].read();
        let instruction = bits_to_u32(&instruction_bits);
        let operand_bits: [u8; 5] = instruction_bits[..5]
            .try_into()
            .expect("the instruction always contains five operand bits");
        let operand = bits_to_u32(&operand_bits) as usize;
        let function_bits: [u8; 3] = instruction_bits[13..16]
            .try_into()
            .expect("the instruction always contains three function bits");
        let selects = decode_function(function_bits);

        let mnemonic = self.execute_selected(selects, operand);
        let pc_after = bits_to_u32(&self.ci.read()) as u8;
        Ok(StepTrace {
            pc_before: ci_before,
            pc_after,
            instruction,
            description: format!("{mnemonic} @ line {fetched_line}"),
            mnemonic,
        })
    }

    /// Run an already-loaded machine until STP, subject to a mandatory bound.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, BabyError> {
        let mut traces = Vec::new();
        while self.halted.read()[0] == 0 {
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

    /// Reset, load at line zero, and execute until STP or the step bound.
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
            store: std::array::from_fn(|index| bits_to_u32(&self.store[index].read())),
            accumulator: bits_to_u32(&self.accumulator.read()),
            ci: bits_to_u32(&self.ci.read()) as u8,
            halted: self.halted.read()[0] == 1,
        }
    }

    /// Exact number of persistent D flip-flops in the model.
    pub const fn flip_flop_count(&self) -> usize {
        FLIP_FLOP_COUNT
    }

    /// Stable educational estimate of storage plus combinational gate topology.
    pub const fn gate_count(&self) -> usize {
        ESTIMATED_GATE_COUNT
    }

    fn execute_selected(&mut self, selects: [u8; 8], operand: usize) -> String {
        let selected_word = self.store[operand].read();
        if selects[0] == 1 {
            self.ci.write(
                &selected_word[..5]
                    .try_into()
                    .expect("a store word always has five low bits"),
            );
            format!("JMP {operand}")
        } else if selects[1] == 1 {
            let displacement: [u8; 5] = selected_word[..5]
                .try_into()
                .expect("a store word always has five low bits");
            self.ci.write(&add_bits(&self.ci.read(), &displacement));
            format!("JRP {operand}")
        } else if selects[2] == 1 {
            let inverted = invert_bits(&selected_word);
            self.accumulator
                .write(&add_bits_with_carry(&ZERO_32, &inverted, 1));
            format!("LDN {operand}")
        } else if selects[3] == 1 {
            self.store[operand].write(&self.accumulator.read());
            format!("STO {operand}")
        } else if or_gate(selects[4], selects[5]) == 1 {
            let inverted = invert_bits(&selected_word);
            self.accumulator
                .write(&add_bits_with_carry(&self.accumulator.read(), &inverted, 1));
            format!("SUB {operand}")
        } else if selects[6] == 1 {
            let negative = self.accumulator.read()[31];
            if and_gate(selects[6], negative) == 1 {
                self.ci.write(&add_bits(&self.ci.read(), &ONE_5));
            }
            "CMP".to_owned()
        } else {
            debug_assert_eq!(selects[7], 1, "one decoder output must be active");
            self.halted.write(&[1]);
            "STP".to_owned()
        }
    }
}

fn decode_function(bits: [u8; 3]) -> [u8; 8] {
    let inverted = bits.map(not_gate);
    std::array::from_fn(|function| {
        let b0 = if function & 1 == 0 {
            inverted[0]
        } else {
            bits[0]
        };
        let b1 = if function & 2 == 0 {
            inverted[1]
        } else {
            bits[1]
        };
        let b2 = if function & 4 == 0 {
            inverted[2]
        } else {
            bits[2]
        };
        and_gate(and_gate(b0, b1), b2)
    })
}

fn add_bits<const WIDTH: usize>(a: &[u8; WIDTH], b: &[u8; WIDTH]) -> [u8; WIDTH] {
    ripple_carry_adder(a, b)
        .sum
        .try_into()
        .expect("a ripple-carry adder preserves its input width")
}

fn add_bits_with_carry<const WIDTH: usize>(
    a: &[u8; WIDTH],
    b: &[u8; WIDTH],
    carry: u8,
) -> [u8; WIDTH] {
    ripple_carry_adder_with_carry(a, b, carry)
        .sum
        .try_into()
        .expect("a ripple-carry adder preserves its input width")
}

fn invert_bits<const WIDTH: usize>(bits: &[u8; WIDTH]) -> [u8; WIDTH] {
    bits.map(not_gate)
}

fn u32_to_bits(value: u32) -> [u8; 32] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn bits_to_u32(bits: &[u8]) -> u32 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, input)| value | u32::from(*input) << bit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_is_one_hot_for_every_function() {
        for function in 0..8 {
            let bits = [function & 1, (function >> 1) & 1, (function >> 2) & 1];
            let outputs = decode_function(bits);
            assert_eq!(outputs.iter().sum::<u8>(), 1);
            assert_eq!(outputs[function as usize], 1);
        }
    }

    #[test]
    fn bit_register_persists_rising_edge_data() {
        let mut register = BitRegister::<5>::new(&[0; 5]);
        register.write(&[1, 0, 1, 1, 0]);
        assert_eq!(register.read(), [1, 0, 1, 1, 0]);
    }

    #[test]
    fn gate_arithmetic_wraps_to_the_selected_width() {
        assert_eq!(add_bits(&[1; 5], &ONE_5), [0; 5]);
        assert_eq!(
            add_bits_with_carry(&ZERO_32, &invert_bits(&ZERO_32), 1),
            ZERO_32
        );
    }
}
