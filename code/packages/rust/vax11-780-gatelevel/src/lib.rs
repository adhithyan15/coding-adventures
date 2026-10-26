//! DFF-backed educational DEC VAX-11/780 gate-level simulator.
//!
//! Persistent architectural state is clocked through repository D flip-flops.
//! Successful instructions are evaluated by an independent gate transition
//! engine and committed only after exact agreement with the checked functional
//! lifecycle oracle.

mod executor;
pub mod gates;
mod state;

pub use vax11_780_simulator::{
    encoding, ExecutionResult, Psl, ScalarState, StepTrace, VaxError, VaxSimulator, VaxState,
    MAX_RETAINED_TRACES, MEMORY_SIZE, REGISTER_PC, REGISTER_SP,
};

use state::{clock_bit, clock_bits, read_bits, DffMemory};

/// Exact number of persistent architectural D flip-flops.
pub const FLIP_FLOP_COUNT: usize = MEMORY_SIZE * 8 + 16 * 32 + 4 + 1 + 32 + 17;
/// Stable storage plus the package's bounded combinational gate estimate.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 131_072;

/// Gate-backed DEC VAX-11/780 integer machine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaxGateLevel {
    memory: DffMemory,
    regs: [[u8; 32]; 16],
    psl: [u8; 4],
    halted: u8,
    loaded_origin: [u8; 32],
    loaded_len: [u8; 17],
}

impl Default for VaxGateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl VaxGateLevel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            memory: DffMemory::new(MEMORY_SIZE),
            regs: [[0; 32]; 16],
            psl: [0; 4],
            halted: 0,
            loaded_origin: [0; 32],
            loaded_len: [0; 17],
        }
    }

    pub fn reset(&mut self) {
        self.memory.clear();
        for register in &mut self.regs {
            clock_bits(register, 0);
        }
        clock_bits(&mut self.psl, 0);
        clock_bit(&mut self.halted, false);
        clock_bits(&mut self.loaded_origin, 0);
        clock_bits(&mut self.loaded_len, 0);
    }

    #[must_use]
    pub fn get_state(&self) -> VaxState {
        let psl = read_bits(&self.psl) as u8;
        VaxState {
            regs: std::array::from_fn(|index| read_bits(&self.regs[index]) as u32),
            psl: Psl {
                n: psl & 1 != 0,
                z: psl & 2 != 0,
                v: psl & 4 != 0,
                c: psl & 8 != 0,
            },
            memory: self.memory.snapshot(),
            halted: self.halted != 0,
            loaded_origin: read_bits(&self.loaded_origin) as u32,
            loaded_len: read_bits(&self.loaded_len) as usize,
        }
    }

    pub fn restore(&mut self, state: &VaxState) -> Result<(), VaxError> {
        let mut validator = VaxSimulator::new();
        validator.restore(state)?;
        self.restore_validated(state);
        Ok(())
    }

    fn restore_validated(&mut self, state: &VaxState) {
        self.memory.restore_snapshot(&state.memory);
        for (register, value) in self.regs.iter_mut().zip(state.regs) {
            clock_bits(register, u64::from(value));
        }
        let psl = u64::from(state.psl.n)
            | (u64::from(state.psl.z) << 1)
            | (u64::from(state.psl.v) << 2)
            | (u64::from(state.psl.c) << 3);
        clock_bits(&mut self.psl, psl);
        clock_bit(&mut self.halted, state.halted);
        clock_bits(&mut self.loaded_origin, u64::from(state.loaded_origin));
        clock_bits(&mut self.loaded_len, state.loaded_len as u64);
    }

    fn commit_transition(&mut self, next: &VaxState) {
        for (address, value) in next.memory.iter().copied().enumerate() {
            if gates::byte_changed(self.memory.read(address), value) {
                self.memory.write(address, value);
            }
        }
        self.restore_scalars(next);
    }

    fn restore_scalars(&mut self, state: &VaxState) {
        for (register, value) in self.regs.iter_mut().zip(state.regs) {
            clock_bits(register, u64::from(value));
        }
        let psl = u64::from(state.psl.n)
            | (u64::from(state.psl.z) << 1)
            | (u64::from(state.psl.v) << 2)
            | (u64::from(state.psl.c) << 3);
        clock_bits(&mut self.psl, psl);
        clock_bit(&mut self.halted, state.halted);
        clock_bits(&mut self.loaded_origin, u64::from(state.loaded_origin));
        clock_bits(&mut self.loaded_len, state.loaded_len as u64);
    }

    #[must_use]
    pub fn psl(&self) -> Psl {
        self.get_state().psl
    }

    pub fn set_psl(&mut self, psl: Psl) {
        let mut state = self.get_state();
        state.psl = psl;
        self.restore_scalars(&state);
    }

    pub fn read_register_checked(&self, index: usize) -> Result<u32, VaxError> {
        self.regs
            .get(index)
            .map(|bits| read_bits(bits) as u32)
            .ok_or(VaxError::InvalidRegister { index })
    }

    pub fn write_register_checked(&mut self, index: usize, value: u32) -> Result<(), VaxError> {
        let register = self
            .regs
            .get_mut(index)
            .ok_or(VaxError::InvalidRegister { index })?;
        clock_bits(register, u64::from(value));
        Ok(())
    }

    pub fn write_register_width_checked(
        &mut self,
        index: usize,
        width: usize,
        value: u32,
    ) -> Result<(), VaxError> {
        if !matches!(width, 1 | 2 | 4) {
            return Err(VaxError::InvalidState(format!(
                "invalid operand width {width}"
            )));
        }
        let previous = self.read_register_checked(index)?;
        let mask = gates::width_mask(width);
        self.write_register_checked(index, (previous & !mask) | (value & mask))
    }

    fn checked_range(address: u32, width: usize) -> Result<std::ops::Range<usize>, VaxError> {
        let start =
            usize::try_from(address).map_err(|_| VaxError::MemoryOutOfRange { address, width })?;
        let end = start
            .checked_add(width)
            .filter(|end| *end <= MEMORY_SIZE)
            .ok_or(VaxError::MemoryOutOfRange { address, width })?;
        Ok(start..end)
    }

    pub fn read_byte_checked(&self, address: u32) -> Result<u8, VaxError> {
        Ok(self.memory.read(Self::checked_range(address, 1)?.start))
    }

    pub fn read_word_checked(&self, address: u32) -> Result<u16, VaxError> {
        let range = Self::checked_range(address, 2)?;
        Ok(u16::from_le_bytes([
            self.memory.read(range.start),
            self.memory.read(range.start + 1),
        ]))
    }

    pub fn read_long_checked(&self, address: u32) -> Result<u32, VaxError> {
        let range = Self::checked_range(address, 4)?;
        Ok(u32::from_le_bytes(std::array::from_fn(|offset| {
            self.memory.read(range.start + offset)
        })))
    }

    pub fn write_byte_checked(&mut self, address: u32, value: u8) -> Result<(), VaxError> {
        let start = Self::checked_range(address, 1)?.start;
        self.memory.write(start, value);
        Ok(())
    }

    pub fn write_word_checked(&mut self, address: u32, value: u16) -> Result<(), VaxError> {
        let range = Self::checked_range(address, 2)?;
        for (address, byte) in range.zip(value.to_le_bytes()) {
            self.memory.write(address, byte);
        }
        Ok(())
    }

    pub fn write_long_checked(&mut self, address: u32, value: u32) -> Result<(), VaxError> {
        let range = Self::checked_range(address, 4)?;
        for (address, byte) in range.zip(value.to_le_bytes()) {
            self.memory.write(address, byte);
        }
        Ok(())
    }

    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), VaxError> {
        self.load_at_checked(program, 0)
    }

    pub fn load_at_checked(&mut self, program: &[u8], origin: u32) -> Result<(), VaxError> {
        let mut validator = VaxSimulator::new();
        validator.load_at_checked(program, origin)?;
        self.reset();
        for (offset, byte) in program.iter().copied().enumerate() {
            self.memory.write(origin as usize + offset, byte);
        }
        clock_bits(&mut self.regs[REGISTER_PC], u64::from(origin));
        clock_bits(&mut self.loaded_origin, u64::from(origin));
        clock_bits(&mut self.loaded_len, program.len() as u64);
        Ok(())
    }

    /// Settle one independent gate transition and clock it only after oracle agreement.
    pub fn step_checked(&mut self) -> Result<StepTrace, VaxError> {
        let before = self.get_state();
        let mut oracle = VaxSimulator::new();
        oracle.restore(&before)?;
        let oracle_trace = oracle.step_checked()?;
        let oracle_next = oracle.get_state();
        let (gate_next, gate_trace) = executor::execute(&before)?;
        if gate_trace != oracle_trace || gate_next != oracle_next {
            return Err(VaxError::InvalidState(
                "independent gate transition diverged from the functional oracle".to_owned(),
            ));
        }
        self.commit_transition(&gate_next);
        Ok(gate_trace)
    }

    pub fn run_loaded_checked(&mut self, max_steps: usize) -> Result<ExecutionResult, VaxError> {
        if max_steps > MAX_RETAINED_TRACES {
            return Err(VaxError::TraceLimitTooLarge {
                requested: max_steps,
                maximum: MAX_RETAINED_TRACES,
            });
        }
        let mut candidate = self.clone();
        let mut traces = Vec::with_capacity(max_steps.min(1024));
        while candidate.halted == 0 {
            if traces.len() == max_steps {
                return Err(VaxError::StepLimitExceeded { max_steps });
            }
            traces.push(candidate.step_checked()?);
        }
        let result = ExecutionResult {
            halted: true,
            steps: traces.len(),
            traces,
            final_state: candidate.get_state(),
        };
        *self = candidate;
        Ok(result)
    }

    pub fn run_checked(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, VaxError> {
        let mut candidate = self.clone();
        candidate.load_checked(program)?;
        let result = candidate.run_loaded_checked(max_steps)?;
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
