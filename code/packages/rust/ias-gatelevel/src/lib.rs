//! DFF-backed educational IAS gate-level simulator.
//!
//! Persistent architectural state is clocked through repository D flip-flops.
//! Successful orders are evaluated by an independent gate transition engine
//! and committed only after exact agreement with the checked functional oracle.

mod executor;
pub mod gates;
mod state;

pub use ias_simulator::{
    encode_instruction, pack_instructions, Half, IasError, IasState, Opcode, RunResult, StepTrace,
    HALT, MAX_STEPS, MEMORY_WORDS, WORD_MASK,
};

use state::{clock_bit, clock_bits, read_bits, DffMemory40};

/// Exact number of persistent architectural D flip-flops.
pub const FLIP_FLOP_COUNT: usize =
    MEMORY_WORDS * 40 + 40 + 40 + 13 + 1 + 20 + 20 + 8 + 12 + 40 + 1 + 12 + 13;
/// Stable storage plus a bounded estimate for the reusable combinational networks.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 98_304;

/// Gate-backed IAS stored-program machine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IasGateLevel {
    memory: DffMemory40,
    accumulator: [u8; 40],
    arithmetic_register: [u8; 40],
    control_counter: [u8; 13],
    buffered_valid: u8,
    buffered_right: [u8; 20],
    current_instruction: [u8; 20],
    function_table: [u8; 8],
    memory_address: [u8; 12],
    selectron_register: [u8; 40],
    halted: u8,
    loaded_origin: [u8; 12],
    loaded_words: [u8; 13],
}

impl Default for IasGateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl IasGateLevel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            memory: DffMemory40::new(MEMORY_WORDS),
            accumulator: [0; 40],
            arithmetic_register: [0; 40],
            control_counter: [0; 13],
            buffered_valid: 0,
            buffered_right: [0; 20],
            current_instruction: [0; 20],
            function_table: [0; 8],
            memory_address: [0; 12],
            selectron_register: [0; 40],
            halted: 0,
            loaded_origin: [0; 12],
            loaded_words: [0; 13],
        }
    }

    /// Reset every persistent element, including all 163,840 memory DFFs.
    pub fn reset(&mut self) {
        self.memory.clear();
        clock_bits(&mut self.accumulator, 0);
        clock_bits(&mut self.arithmetic_register, 0);
        clock_bits(&mut self.control_counter, 0);
        clock_bit(&mut self.buffered_valid, false);
        clock_bits(&mut self.buffered_right, 0);
        clock_bits(&mut self.current_instruction, 0);
        clock_bits(&mut self.function_table, 0);
        clock_bits(&mut self.memory_address, 0);
        clock_bits(&mut self.selectron_register, 0);
        clock_bit(&mut self.halted, false);
        clock_bits(&mut self.loaded_origin, 0);
        clock_bits(&mut self.loaded_words, 0);
    }

    #[must_use]
    pub fn snapshot(&self) -> IasState {
        let memory: Box<[u64; MEMORY_WORDS]> = self
            .memory
            .snapshot()
            .into_boxed_slice()
            .try_into()
            .expect("IAS DFF memory has a fixed architectural size");
        IasState {
            memory,
            accumulator: read_bits(&self.accumulator),
            arithmetic_register: read_bits(&self.arithmetic_register),
            control_counter: read_bits(&self.control_counter) as u16,
            buffered_right: (self.buffered_valid != 0)
                .then(|| read_bits(&self.buffered_right) as u32),
            current_instruction: read_bits(&self.current_instruction) as u32,
            function_table: read_bits(&self.function_table) as u8,
            memory_address: read_bits(&self.memory_address) as u16,
            selectron_register: read_bits(&self.selectron_register),
            halted: self.halted != 0,
            loaded_origin: read_bits(&self.loaded_origin) as u16,
            loaded_words: read_bits(&self.loaded_words) as usize,
        }
    }

    #[must_use]
    pub fn get_state(&self) -> IasState {
        self.snapshot()
    }

    pub fn restore(&mut self, state: &IasState) -> Result<(), IasError> {
        let mut validator = ias_simulator::IasSimulator::new();
        validator.restore(state)?;
        self.restore_validated(state);
        Ok(())
    }

    fn restore_validated(&mut self, state: &IasState) {
        self.memory.restore_snapshot(state.memory.as_slice());
        self.restore_scalars(state);
    }

    fn restore_scalars(&mut self, state: &IasState) {
        clock_bits(&mut self.accumulator, state.accumulator);
        clock_bits(&mut self.arithmetic_register, state.arithmetic_register);
        clock_bits(&mut self.control_counter, u64::from(state.control_counter));
        clock_bit(&mut self.buffered_valid, state.buffered_right.is_some());
        clock_bits(
            &mut self.buffered_right,
            u64::from(state.buffered_right.unwrap_or_default()),
        );
        clock_bits(
            &mut self.current_instruction,
            u64::from(state.current_instruction),
        );
        clock_bits(&mut self.function_table, u64::from(state.function_table));
        clock_bits(&mut self.memory_address, u64::from(state.memory_address));
        clock_bits(&mut self.selectron_register, state.selectron_register);
        clock_bit(&mut self.halted, state.halted);
        clock_bits(&mut self.loaded_origin, u64::from(state.loaded_origin));
        clock_bits(&mut self.loaded_words, state.loaded_words as u64);
    }

    fn commit_transition(&mut self, next: &IasState) {
        for (address, value) in next.memory.iter().copied().enumerate() {
            if gates::word_changed(self.memory.read(address), value) {
                self.memory.write(address, value);
            }
        }
        self.restore_scalars(next);
    }

    pub fn load(&mut self, bytes: &[u8], origin: u16) -> Result<usize, IasError> {
        if bytes.is_empty() {
            return Err(IasError::EmptyProgram);
        }
        if !bytes.len().is_multiple_of(5) {
            return Err(IasError::TransportAlignment { bytes: bytes.len() });
        }
        let word_count = bytes.len() / 5;
        let start = usize::from(origin);
        if start >= MEMORY_WORDS {
            return Err(IasError::OriginOutOfRange { origin });
        }
        if word_count > MEMORY_WORDS - start {
            return Err(IasError::ProgramOutOfRange {
                origin,
                words: word_count,
            });
        }
        let mut words = Vec::with_capacity(word_count);
        for chunk in bytes.as_chunks::<5>().0 {
            words.push(
                (u64::from(chunk[0]) << 32)
                    | (u64::from(chunk[1]) << 24)
                    | (u64::from(chunk[2]) << 16)
                    | (u64::from(chunk[3]) << 8)
                    | u64::from(chunk[4]),
            );
        }
        self.load_words(&words, origin)
    }

    pub fn load_words(&mut self, words: &[u64], origin: u16) -> Result<usize, IasError> {
        let mut validator = ias_simulator::IasSimulator::new();
        validator.load_words(words, origin)?;
        self.reset();
        for (offset, word) in words.iter().copied().enumerate() {
            self.memory.write(usize::from(origin) + offset, word);
        }
        clock_bits(&mut self.control_counter, u64::from(origin));
        clock_bits(&mut self.loaded_origin, u64::from(origin));
        clock_bits(&mut self.loaded_words, words.len() as u64);
        Ok(words.len())
    }

    pub fn read_memory(&self, address: usize) -> Result<u64, IasError> {
        if address >= MEMORY_WORDS {
            return Err(IasError::MemoryOutOfRange { address });
        }
        Ok(self.memory.read(address))
    }

    pub fn write_memory(&mut self, address: usize, value: u64) -> Result<(), IasError> {
        if value > WORD_MASK {
            return Err(IasError::InvalidWord { value });
        }
        if address >= MEMORY_WORDS {
            return Err(IasError::MemoryOutOfRange { address });
        }
        self.memory.write(address, value);
        Ok(())
    }

    pub fn set_accumulator(&mut self, value: u64) -> Result<(), IasError> {
        if value > WORD_MASK {
            return Err(IasError::InvalidWord { value });
        }
        clock_bits(&mut self.accumulator, value);
        Ok(())
    }

    pub fn set_arithmetic_register(&mut self, value: u64) -> Result<(), IasError> {
        if value > WORD_MASK {
            return Err(IasError::InvalidWord { value });
        }
        clock_bits(&mut self.arithmetic_register, value);
        Ok(())
    }

    /// Settle one independent gate transition and clock it only after oracle agreement.
    pub fn step(&mut self) -> Result<StepTrace, IasError> {
        let before = self.snapshot();
        let mut oracle = ias_simulator::IasSimulator::new();
        oracle.restore(&before)?;
        let oracle_result = oracle.step();
        let gate_result = executor::execute(&before);
        match (oracle_result, gate_result) {
            (Ok(oracle_trace), Ok((gate_next, gate_trace)))
                if oracle_trace == gate_trace && oracle.snapshot() == gate_next =>
            {
                self.commit_transition(&gate_next);
                Ok(gate_trace)
            }
            (Err(oracle_error), Err(gate_error)) if oracle_error == gate_error => Err(gate_error),
            _ => Err(IasError::InvalidControlRegister),
        }
    }

    /// Run until stop, rolling the complete run back on any failure.
    pub fn run(&mut self, limit: usize) -> Result<RunResult, IasError> {
        if limit > MAX_STEPS {
            return Err(IasError::StepLimitTooLarge {
                limit,
                maximum: MAX_STEPS,
            });
        }
        if self.halted != 0 {
            let final_state = self.snapshot();
            return Ok(RunResult {
                traces: Vec::new(),
                final_state,
                halted: true,
            });
        }
        let mut candidate = self.clone();
        let mut traces = Vec::with_capacity(limit.min(64));
        for _ in 0..limit {
            let trace = candidate.step()?;
            let halted = trace.state_after.halted;
            traces.push(trace);
            if halted {
                let final_state = candidate.snapshot();
                *self = candidate;
                return Ok(RunResult {
                    traces,
                    final_state,
                    halted: true,
                });
            }
        }
        Err(IasError::StepLimitExceeded { limit })
    }

    pub fn execute(
        &mut self,
        bytes: &[u8],
        origin: u16,
        limit: usize,
    ) -> Result<RunResult, IasError> {
        if limit > MAX_STEPS {
            return Err(IasError::StepLimitTooLarge {
                limit,
                maximum: MAX_STEPS,
            });
        }
        let mut candidate = self.clone();
        candidate.load(bytes, origin)?;
        let result = candidate.run(limit)?;
        *self = candidate;
        Ok(result)
    }

    pub fn execute_words(
        &mut self,
        words: &[u64],
        origin: u16,
        limit: usize,
    ) -> Result<RunResult, IasError> {
        if limit > MAX_STEPS {
            return Err(IasError::StepLimitTooLarge {
                limit,
                maximum: MAX_STEPS,
            });
        }
        let mut candidate = self.clone();
        candidate.load_words(words, origin)?;
        let result = candidate.run(limit)?;
        *self = candidate;
        Ok(result)
    }

    #[must_use]
    pub const fn flip_flop_count(&self) -> usize {
        FLIP_FLOP_COUNT
    }

    #[must_use]
    pub const fn gate_count(&self) -> usize {
        ESTIMATED_GATE_COUNT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_storage_topology_is_reported() {
        let machine = IasGateLevel::new();
        assert_eq!(FLIP_FLOP_COUNT, 164_060);
        assert_eq!(machine.flip_flop_count(), 164_060);
        assert!(machine.gate_count() > machine.flip_flop_count());
        assert_eq!(machine.snapshot(), IasState::default());
    }

    #[test]
    fn direct_access_is_checked_and_clocked() {
        let mut machine = IasGateLevel::new();
        machine.write_memory(7, 0x12345).unwrap();
        assert_eq!(machine.read_memory(7), Ok(0x12345));
        assert_eq!(
            machine.write_memory(MEMORY_WORDS, 1),
            Err(IasError::MemoryOutOfRange {
                address: MEMORY_WORDS
            })
        );
        assert_eq!(
            machine.set_accumulator(WORD_MASK + 1),
            Err(IasError::InvalidWord {
                value: WORD_MASK + 1
            })
        );
        machine.reset();
        assert_eq!(machine.snapshot(), IasState::default());
    }
}
