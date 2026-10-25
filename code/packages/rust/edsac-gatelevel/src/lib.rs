//! Exact DFF-backed digital-logic model of the 1949 EDSAC.
//!
//! A step has two phases. `engine` first settles a transient combinational
//! network built from repository gates. Only a successful transition reaches
//! `commit_state`, which presents the next values to the master/slave DFF banks
//! and raises their clock. That split is what makes a typed fault electrically
//! atomic: no rising edge occurs.

pub use edsac_simulator::{
    encode_order, encode_raw_order, EdsacError, EdsacState, ExecutionResult, Length, Opcode,
    StepTrace, EVEN_CELL_MASK, LONG_WORD_MASK, MEMORY_CELLS, ODD_CELL_MASK,
};

mod engine;
mod state;

use engine::WorkingMachine;
use state::{
    accumulator_value, bits_value, clock_accumulator, clock_bit, clock_bits, clock_operand,
    operand_value, DffStore, MEMORY_DFFS,
};

/// 17,920 store + 10 SCR + 17 order + 71 accumulator + two 35-bit operands
/// + one length + five last-output + one halt DFF.
pub const FLIP_FLOP_COUNT: usize = MEMORY_DFFS + 10 + 17 + 71 + 35 + 35 + 1 + 5 + 1;
/// Stable educational estimate; this is not a valve or transistor count.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 36_000;
/// Maximum number of retained instruction traces and per-step output events.
pub const MAX_RETAINED_STEPS: usize = 65_536;

fn validate_step_budget(max_steps: usize) -> Result<(), EdsacError> {
    if max_steps > MAX_RETAINED_STEPS {
        Err(EdsacError::MaxStepsExceeded { max_steps })
    } else {
        Ok(())
    }
}

/// Complete gate-level EDSAC machine at an instruction boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EdsacGateLevel {
    memory: DffStore,
    scr: [u8; 10],
    order_tank: [u8; 17],
    accumulator: [u8; 71],
    multiplier: [u8; 35],
    multiplicand: [u8; 35],
    multiplicand_long: u8,
    last_output: [u8; 5],
    halted: u8,
    input: Vec<u8>,
    input_cursor: usize,
    output: Vec<u8>,
    loaded_origin: usize,
    loaded_words: usize,
}

impl Default for EdsacGateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl EdsacGateLevel {
    /// Construct a clear EDSAC with no Initial Orders installed.
    #[must_use]
    pub fn new() -> Self {
        Self {
            memory: DffStore::new(),
            scr: [0; 10],
            order_tank: [0; 17],
            accumulator: [0; 71],
            multiplier: [0; 35],
            multiplicand: [0; 35],
            multiplicand_long: 0,
            last_output: [0; 5],
            halted: 0,
            input: Vec::new(),
            input_cursor: 0,
            output: Vec::new(),
            loaded_origin: 0,
            loaded_words: 0,
        }
    }

    /// Restore power-on state and clear all peripheral/lifecycle metadata.
    pub fn reset(&mut self) {
        self.memory.clear();
        clock_bits(&mut self.scr, 0);
        clock_bits(&mut self.order_tank, 0);
        clock_accumulator(&mut self.accumulator, [0; 5]);
        clock_operand(&mut self.multiplier, [0; 3]);
        clock_operand(&mut self.multiplicand, [0; 3]);
        clock_bit(&mut self.multiplicand_long, false);
        clock_bits(&mut self.last_output, 0);
        clock_bit(&mut self.halted, false);
        self.input.clear();
        self.input_cursor = 0;
        self.output.clear();
        self.loaded_origin = 0;
        self.loaded_words = 0;
    }

    /// Clone the complete fixed-width, peripheral, and lifecycle state.
    #[must_use]
    pub fn state(&self) -> EdsacState {
        EdsacState {
            memory: self.memory.snapshot(),
            scr: bits_value(&self.scr) as usize,
            order_tank: bits_value(&self.order_tank) as u32,
            accumulator: accumulator_value(&self.accumulator),
            multiplier: operand_value(&self.multiplier),
            multiplicand: operand_value(&self.multiplicand),
            multiplicand_length: if self.multiplicand_long == 0 {
                Length::Short
            } else {
                Length::Long
            },
            input: self.input.clone(),
            input_cursor: self.input_cursor,
            output: self.output.clone(),
            last_output: bits_value(&self.last_output) as u8,
            halted: self.halted != 0,
            loaded_origin: self.loaded_origin,
            loaded_words: self.loaded_words,
        }
    }

    /// Restore a functionally validated complete snapshot through DFF clocks.
    pub fn restore(&mut self, snapshot: &EdsacState) -> Result<(), EdsacError> {
        let mut validator = edsac_simulator::EdsacSimulator::new();
        validator.restore(snapshot)?;
        self.commit_state(snapshot);
        Ok(())
    }

    fn commit_state(&mut self, next: &EdsacState) {
        let current = self.memory.snapshot();
        for (address, (&old, &word)) in current.iter().zip(next.memory.iter()).enumerate() {
            if old != word {
                let wrote = self.memory.write(address, word);
                debug_assert!(wrote);
            }
        }
        clock_bits(&mut self.scr, next.scr as u128);
        clock_bits(&mut self.order_tank, u128::from(next.order_tank));
        clock_accumulator(&mut self.accumulator, next.accumulator);
        clock_operand(&mut self.multiplier, next.multiplier);
        clock_operand(&mut self.multiplicand, next.multiplicand);
        clock_bit(
            &mut self.multiplicand_long,
            next.multiplicand_length == Length::Long,
        );
        clock_bits(&mut self.last_output, u128::from(next.last_output));
        clock_bit(&mut self.halted, next.halted);
        self.input.clone_from(&next.input);
        self.input_cursor = next.input_cursor;
        self.output.clone_from(&next.output);
        self.loaded_origin = next.loaded_origin;
        self.loaded_words = next.loaded_words;
    }

    /// Decode checked little-endian 32-bit short-store containers.
    pub fn load(&mut self, program: &[u8], origin: usize) -> Result<usize, EdsacError> {
        let mut validator = edsac_simulator::EdsacSimulator::new();
        let loaded = validator.load(program, origin)?;
        self.commit_state(&validator.state());
        Ok(loaded)
    }

    /// Clear the machine and clock checked raw short-store cells into it.
    pub fn load_words(&mut self, program: &[u32], origin: usize) -> Result<usize, EdsacError> {
        let mut validator = edsac_simulator::EdsacSimulator::new();
        let loaded = validator.load_words(program, origin)?;
        self.commit_state(&validator.state());
        Ok(loaded)
    }

    /// Replace the host paper tape and rewind its cursor.
    pub fn set_input(&mut self, input: &[u8]) -> Result<(), EdsacError> {
        let mut validator = edsac_simulator::EdsacSimulator::new();
        validator.restore(&self.state())?;
        validator.set_input(input)?;
        self.commit_state(&validator.state());
        Ok(())
    }

    /// Clock the exact 71-bit accumulator value into its tank DFFs.
    pub fn set_accumulator(&mut self, accumulator: [u16; 5]) -> Result<(), EdsacError> {
        let mut validator = edsac_simulator::EdsacSimulator::new();
        validator.restore(&self.state())?;
        validator.set_accumulator(accumulator)?;
        clock_accumulator(&mut self.accumulator, accumulator);
        Ok(())
    }

    /// Read one raw short-store cell.
    pub fn read_memory(&self, address: usize) -> Result<u32, EdsacError> {
        self.memory
            .read(address)
            .ok_or(EdsacError::InvalidAddress { address })
    }

    /// Clock one validated raw short-store cell.
    pub fn write_memory(&mut self, address: usize, word: u32) -> Result<(), EdsacError> {
        let mut validator = edsac_simulator::EdsacSimulator::new();
        validator.restore(&self.state())?;
        validator.write_memory(address, word)?;
        let wrote = self.memory.write(address, word);
        debug_assert!(wrote);
        Ok(())
    }

    /// Read one aligned 35-bit long word.
    pub fn read_long(&self, address: usize) -> Result<u64, EdsacError> {
        let mut validator = edsac_simulator::EdsacSimulator::new();
        validator.restore(&self.state())?;
        validator.read_long(address)
    }

    /// Clock one aligned 35-bit long word into its even/odd pair.
    pub fn write_long(&mut self, address: usize, word: u64) -> Result<(), EdsacError> {
        let mut validator = edsac_simulator::EdsacSimulator::new();
        validator.restore(&self.state())?;
        validator.write_long(address, word)?;
        self.commit_state(&validator.state());
        Ok(())
    }

    /// Execute one independent gate transition atomically.
    pub fn step(&mut self) -> Result<StepTrace, EdsacError> {
        let mut working = WorkingMachine {
            state: self.state(),
        };
        let trace = working.step_inner()?;
        self.commit_state(&working.state);
        Ok(trace)
    }

    /// Run until Z retires, restoring the exact entry state on any error.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, EdsacError> {
        validate_step_budget(max_steps)?;
        let before = self.state();
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| EdsacError::MaxStepsExceeded { max_steps })?;
        for _ in 0..max_steps {
            if self.halted != 0 {
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
                    self.commit_state(&before);
                    return Err(error);
                }
            }
        }
        if self.halted != 0 {
            return Ok(ExecutionResult {
                halted: true,
                steps: traces.len(),
                final_state: self.state(),
                traces,
            });
        }
        self.commit_state(&before);
        Err(EdsacError::MaxStepsExceeded { max_steps })
    }

    /// Transactionally load and execute raw short-store cells.
    pub fn execute_words(
        &mut self,
        program: &[u32],
        origin: usize,
        max_steps: usize,
    ) -> Result<ExecutionResult, EdsacError> {
        validate_step_budget(max_steps)?;
        let before = self.state();
        self.load_words(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.commit_state(&before);
                Err(error)
            }
        }
    }

    /// Transactionally load and execute little-endian cell containers.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: usize,
        max_steps: usize,
    ) -> Result<ExecutionResult, EdsacError> {
        validate_step_budget(max_steps)?;
        let before = self.state();
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.commit_state(&before);
                Err(error)
            }
        }
    }

    /// Exact fixed-width persistent storage inventory.
    #[must_use]
    pub const fn flip_flop_count(&self) -> usize {
        FLIP_FLOP_COUNT
    }

    /// Stable educational primitive-gate estimate.
    #[must_use]
    pub const fn gate_count(&self) -> usize {
        ESTIMATED_GATE_COUNT
    }
}
