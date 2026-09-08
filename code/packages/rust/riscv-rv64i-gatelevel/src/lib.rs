//! Exact gate-backed persistent state for the Spec 07y2 RV64I+M machine.

use riscv_rv64i_simulator::{
    Rv64IError, Rv64IExecutionResult, Rv64ISimulator, Rv64IState, Rv64IStepTrace, MEMORY_SIZE,
    RESET_STACK_POINTER,
};

mod engine;
mod state;

use engine::WorkingCpu;
use state::{clock_bit, clock_word, word64, DffMemory};

/// 524,288 memory + 2,048 GPR + 64 PC + one HALT DFFs.
pub const FLIP_FLOP_COUNT: usize = MEMORY_SIZE * 8 + 32 * 64 + 64 + 1;

/// Exact sequential architectural state for the RV64I+M machine.
#[derive(Debug, Clone)]
pub struct Rv64IGateLevel {
    memory: DffMemory,
    registers: [[u8; 64]; 32],
    pc: [u8; 64],
    halted: u8,
    loaded_origin: u64,
    loaded_len: usize,
}

impl Default for Rv64IGateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl Rv64IGateLevel {
    /// Construct and clock the reset topology.
    #[must_use]
    pub fn new() -> Self {
        let mut machine = Self {
            memory: DffMemory::new(),
            registers: [[0; 64]; 32],
            pc: [0; 64],
            halted: 0,
            loaded_origin: 0,
            loaded_len: 0,
        };
        machine.reset();
        machine
    }

    /// Clock the complete reset state.
    pub fn reset(&mut self) {
        self.memory.clear();
        for register in &mut self.registers {
            clock_word(register, 0);
        }
        clock_word(&mut self.registers[2], RESET_STACK_POINTER);
        clock_word(&mut self.pc, 0);
        clock_bit(&mut self.halted, false);
        self.loaded_origin = 0;
        self.loaded_len = 0;
    }

    /// Clone the complete architectural and installation state.
    #[must_use]
    pub fn get_state(&self) -> Rv64IState {
        Rv64IState {
            registers: std::array::from_fn(|index| word64(&self.registers[index])),
            pc: word64(&self.pc),
            memory: self.memory.snapshot(),
            halted: self.halted != 0,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    /// Atomically restore a snapshot validated by the functional contract.
    pub fn restore(&mut self, snapshot: &Rv64IState) -> Result<(), Rv64IError> {
        let mut validator = Rv64ISimulator::new();
        validator.restore(snapshot)?;
        self.restore_validated(snapshot);
        Ok(())
    }

    fn restore_validated(&mut self, snapshot: &Rv64IState) {
        self.memory.restore_snapshot(&snapshot.memory);
        for (register, value) in self.registers.iter_mut().zip(snapshot.registers) {
            clock_word(register, value);
        }
        clock_word(&mut self.pc, snapshot.pc);
        clock_bit(&mut self.halted, snapshot.halted);
        self.loaded_origin = snapshot.loaded_origin;
        self.loaded_len = snapshot.loaded_len;
    }

    fn commit_transition(&mut self, next: &Rv64IState) {
        for (address, (&old, &value)) in self.memory.snapshot().iter().zip(&next.memory).enumerate()
        {
            if old != value {
                let wrote = self.memory.write(address as u64, value);
                debug_assert!(wrote);
            }
        }
        for (register, value) in self.registers.iter_mut().zip(next.registers) {
            clock_word(register, value);
        }
        clock_word(&mut self.pc, next.pc);
        clock_bit(&mut self.halted, next.halted);
        self.loaded_origin = next.loaded_origin;
        self.loaded_len = next.loaded_len;
    }

    /// Reset and install a program at address zero.
    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), Rv64IError> {
        self.load_at_checked(program, 0)
    }

    /// Reset and install a program at a checked word-aligned origin.
    pub fn load_at_checked(&mut self, program: &[u8], origin: u64) -> Result<(), Rv64IError> {
        let mut validator = Rv64ISimulator::new();
        validator.load_at_checked(program, origin)?;
        self.reset();
        for (offset, byte) in program.iter().copied().enumerate() {
            let wrote = self.memory.write(origin + offset as u64, byte);
            debug_assert!(wrote);
        }
        clock_word(&mut self.pc, origin);
        self.loaded_origin = origin;
        self.loaded_len = program.len();
        Ok(())
    }

    /// Read one checked register.
    pub fn read_register(&self, index: usize) -> Result<u64, Rv64IError> {
        self.registers
            .get(index)
            .map(word64)
            .ok_or(Rv64IError::InvalidRegister { index })
    }

    /// Clock one checked register. Writes to x0 are discarded.
    pub fn write_register(&mut self, index: usize, value: u64) -> Result<(), Rv64IError> {
        let register = self
            .registers
            .get_mut(index)
            .ok_or(Rv64IError::InvalidRegister { index })?;
        if index != 0 {
            clock_word(register, value);
        }
        Ok(())
    }

    /// Read one checked architectural memory byte.
    pub fn read_byte(&self, address: u64) -> Result<u8, Rv64IError> {
        self.memory
            .read(address)
            .ok_or(Rv64IError::DataOutOfRange { address, width: 1 })
    }

    /// Clock one checked architectural memory byte.
    pub fn write_byte(&mut self, address: u64, value: u8) -> Result<(), Rv64IError> {
        if self.memory.write(address, value) {
            Ok(())
        } else {
            Err(Rv64IError::DataOutOfRange { address, width: 1 })
        }
    }

    /// Execute one independent, atomic gate-machine transition.
    pub fn step_checked(&mut self) -> Result<Rv64IStepTrace, Rv64IError> {
        if self.halted != 0 {
            return Err(Rv64IError::Halted);
        }
        let before = self.get_state();
        let mut working = WorkingCpu {
            state: before.clone(),
        };
        let (raw, mnemonic) = working.step_inner()?;
        self.commit_transition(&working.state);
        Ok(Rv64IStepTrace {
            pc_before: before.pc,
            pc_after: working.state.pc,
            raw,
            mnemonic,
            state_before: before,
            state_after: self.get_state(),
        })
    }

    /// Run the installed program transactionally until halt.
    pub fn run_checked(&mut self, max_steps: usize) -> Result<Rv64IExecutionResult, Rv64IError> {
        let before = self.get_state();
        let mut traces = Vec::new();
        if self.halted != 0 {
            return Ok(Rv64IExecutionResult {
                halted: true,
                steps: 0,
                traces,
                final_state: before,
            });
        }
        for _ in 0..max_steps {
            match self.step_checked() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.restore_validated(&before);
                    return Err(error);
                }
            }
            if self.halted != 0 {
                return Ok(Rv64IExecutionResult {
                    halted: true,
                    steps: traces.len(),
                    traces,
                    final_state: self.get_state(),
                });
            }
        }
        self.restore_validated(&before);
        Err(Rv64IError::StepLimitExceeded { max_steps })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_and_checked_storage_are_exact() {
        assert_eq!(FLIP_FLOP_COUNT, 526_401);
        let mut machine = Rv64IGateLevel::new();
        assert_eq!(machine.read_register(2), Ok(RESET_STACK_POINTER));
        machine.write_register(31, u64::MAX).unwrap();
        machine.write_register(0, u64::MAX).unwrap();
        assert_eq!(machine.read_register(31), Ok(u64::MAX));
        assert_eq!(machine.read_register(0), Ok(0));
    }
}
