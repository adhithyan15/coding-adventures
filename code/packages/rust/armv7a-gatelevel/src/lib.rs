//! Gate-level ARMv7-A / Thumb-2 simulator for Spec 07x2.
//!
//! Persistent architectural storage is clocked through repository D flip-flops.
//! The mixed-width gate execution path is built alongside the lifecycle below.

use armv7a_simulator::{
    Armv7AError, Armv7ASimulator, Armv7AState, ExecutionResult, StepTrace, MEMORY_SIZE, PC, SP,
};

mod engine;
mod state;
use engine::WorkingCpu;
use state::{bits32, clock_bit, clock_word, word32, DffMemory};

/// 524,288 memory + 512 R0-R15 + 32 CPSR + one halt D flip-flops.
pub const FLIP_FLOP_COUNT: usize = MEMORY_SIZE * 8 + 16 * 32 + 32 + 1;

/// Exact sequential architectural state for the Thumb-2 machine.
#[derive(Debug, Clone)]
pub struct Armv7AGateLevel {
    memory: DffMemory,
    registers: [[u8; 32]; 16],
    cpsr: [u8; 32],
    halted: u8,
    loaded_origin: u32,
    loaded_len: usize,
}

impl Default for Armv7AGateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl Armv7AGateLevel {
    /// Construct the reset topology with SP=0xfff8 and CPSR.T set.
    #[must_use]
    pub fn new() -> Self {
        let mut registers = [[0; 32]; 16];
        registers[SP] = bits32(0xfff8);
        Self {
            memory: DffMemory::new(),
            registers,
            cpsr: bits32(1 << 5),
            halted: 0,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }

    /// Clock the complete reset state.
    pub fn reset(&mut self) {
        self.memory.clear();
        for register in &mut self.registers {
            clock_word(register, 0);
        }
        clock_word(&mut self.registers[SP], 0xfff8);
        clock_word(&mut self.cpsr, 1 << 5);
        clock_bit(&mut self.halted, false);
        self.loaded_origin = 0;
        self.loaded_len = 0;
    }

    /// Clone the complete architectural and installation state.
    #[must_use]
    pub fn get_state(&self) -> Armv7AState {
        let registers = std::array::from_fn(|index| word32(&self.registers[index]));
        Armv7AState {
            registers,
            pc: registers[PC],
            cpsr: word32(&self.cpsr),
            memory: self.memory.snapshot(),
            halted: self.halted != 0,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    /// Atomically restore a snapshot validated by the functional contract.
    pub fn restore(&mut self, snapshot: &Armv7AState) -> Result<(), Armv7AError> {
        let mut validator = Armv7ASimulator::new();
        validator.restore(snapshot)?;
        self.restore_validated(snapshot);
        Ok(())
    }

    fn restore_validated(&mut self, snapshot: &Armv7AState) {
        self.memory.restore_snapshot(&snapshot.memory);
        for (register, value) in self.registers.iter_mut().zip(snapshot.registers) {
            clock_word(register, value);
        }
        clock_word(&mut self.cpsr, snapshot.cpsr);
        clock_bit(&mut self.halted, snapshot.halted);
        self.loaded_origin = snapshot.loaded_origin;
        self.loaded_len = snapshot.loaded_len;
    }

    fn commit_transition(&mut self, next: &Armv7AState) {
        for (address, (&old, &value)) in self.memory.snapshot().iter().zip(&next.memory).enumerate()
        {
            if old != value {
                self.memory.write(address as u32, value);
            }
        }
        for (register, value) in self.registers.iter_mut().zip(next.registers) {
            clock_word(register, value);
        }
        clock_word(&mut self.cpsr, next.cpsr);
        clock_bit(&mut self.halted, next.halted);
        self.loaded_origin = next.loaded_origin;
        self.loaded_len = next.loaded_len;
    }

    /// Reset and install a program at address zero.
    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), Armv7AError> {
        self.load_at_checked(program, 0)
    }

    /// Reset and install a program at a checked halfword-aligned origin.
    pub fn load_at_checked(&mut self, program: &[u8], origin: u32) -> Result<(), Armv7AError> {
        let mut validator = Armv7ASimulator::new();
        validator.load_at_checked(program, origin)?;
        self.reset();
        for (offset, byte) in program.iter().copied().enumerate() {
            self.memory.write(origin + offset as u32, byte);
        }
        clock_word(&mut self.registers[PC], origin);
        self.loaded_origin = origin;
        self.loaded_len = program.len();
        Ok(())
    }

    /// Read one checked register.
    pub fn read_register(&self, index: usize) -> Result<u32, Armv7AError> {
        self.registers
            .get(index)
            .map(word32)
            .ok_or(Armv7AError::InvalidRegister { index })
    }

    /// Clock one checked register, keeping PC/R15 coherent.
    pub fn write_register(&mut self, index: usize, value: u32) -> Result<(), Armv7AError> {
        let register = self
            .registers
            .get_mut(index)
            .ok_or(Armv7AError::InvalidRegister { index })?;
        clock_word(register, if index == PC { value & !1 } else { value });
        Ok(())
    }

    /// Read one wrapping architectural memory byte.
    #[must_use]
    pub fn read_byte(&self, address: u32) -> u8 {
        self.memory.read(address)
    }

    /// Clock one wrapping architectural memory byte.
    pub fn write_byte(&mut self, address: u32, value: u8) {
        self.memory.write(address, value);
    }

    /// Execute one independent, atomic gate-machine transition.
    pub fn step_checked(&mut self) -> Result<StepTrace, Armv7AError> {
        if self.halted != 0 {
            return Err(Armv7AError::Halted);
        }
        let before = self.get_state();
        let mut working = WorkingCpu {
            state: before.clone(),
        };
        let (raw, width, mnemonic) = working.step_inner()?;
        self.commit_transition(&working.state);
        Ok(StepTrace {
            pc_before: before.pc,
            pc_after: working.state.pc,
            raw,
            width,
            mnemonic,
            state_before: before,
            state_after: self.get_state(),
        })
    }

    /// Run the installed program transactionally until halt.
    pub fn run_checked(&mut self, max_steps: usize) -> Result<ExecutionResult, Armv7AError> {
        let before = self.get_state();
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.halted != 0 {
                return Ok(ExecutionResult {
                    halted: true,
                    steps: traces.len(),
                    traces,
                    final_state: self.get_state(),
                });
            }
            match self.step_checked() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.restore_validated(&before);
                    return Err(error);
                }
            }
        }
        self.restore_validated(&before);
        Err(Armv7AError::StepLimitExceeded { max_steps })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_and_reset_are_exact() {
        assert_eq!(FLIP_FLOP_COUNT, 524_833);
        let cpu = Armv7AGateLevel::new();
        let state = cpu.get_state();
        assert_eq!(state.memory.len(), MEMORY_SIZE);
        assert_eq!(state.registers[SP], 0xfff8);
        assert_eq!(state.registers[PC], state.pc);
        assert_eq!(state.cpsr, 1 << 5);
    }

    #[test]
    fn restore_load_and_direct_access_round_trip() {
        let mut cpu = Armv7AGateLevel::new();
        cpu.load_at_checked(&[0, 0], 0x100).unwrap();
        cpu.write_register(2, 0x1234_5678).unwrap();
        cpu.write_byte(0x1_ffff, 0xa5);
        let snapshot = cpu.get_state();
        assert_eq!(snapshot.pc, 0x100);
        assert_eq!(cpu.read_register(2), Ok(0x1234_5678));
        assert_eq!(cpu.read_byte(0xffff), 0xa5);
        let mut restored = Armv7AGateLevel::new();
        restored.restore(&snapshot).unwrap();
        assert_eq!(restored.get_state(), snapshot);
    }

    #[test]
    fn independent_step_and_run_match_the_functional_state() {
        let program = armv7a_simulator::encoding::program(&[
            armv7a_simulator::encoding::mov_imm8(0, 40),
            armv7a_simulator::encoding::add_imm8(0, 2),
            armv7a_simulator::encoding::halt(),
        ]);
        let mut gate = Armv7AGateLevel::new();
        let mut functional = Armv7ASimulator::new();
        gate.load_checked(&program).unwrap();
        functional.load_checked(&program).unwrap();
        let gate_result = gate.run_checked(4).unwrap();
        let functional_result = functional.run_checked(4).unwrap();
        assert_eq!(gate_result.final_state, functional_result.final_state);
        assert_eq!(gate_result.traces, functional_result.traces);
    }
}
