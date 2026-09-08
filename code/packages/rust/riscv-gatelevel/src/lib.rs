//! Gate-level RISC-V RV32I simulator for Spec 07a2.
//!
//! Persistent architectural storage is clocked through the repository's D
//! flip-flop primitive. Decode, arithmetic, comparison, Boolean logic, shifts,
//! addresses, branches, CSR operations, traps, and returns execute through the
//! independent gate engine in this crate.

use riscv_simulator::{
    Rv32IError, Rv32IExecutionResult, Rv32ISimulator, Rv32IState, Rv32IStepTrace, RV32I_MEMORY_SIZE,
};

mod engine;
mod state;

use engine::WorkingCpu;
use state::{clock_bit, clock_word, word32, DffMemory};

/// 524,288 memory + 1,024 GPR + 32 PC + 160 CSR + one HALT DFFs.
pub const FLIP_FLOP_COUNT: usize = RV32I_MEMORY_SIZE * 8 + 32 * 32 + 32 + 5 * 32 + 1;

/// Exact sequential architectural state for the RV32I machine.
#[derive(Debug, Clone)]
pub struct Rv32IGateLevel {
    memory: DffMemory,
    registers: [[u8; 32]; 32],
    pc: [u8; 32],
    csr_mstatus: [u8; 32],
    csr_mtvec: [u8; 32],
    csr_mscratch: [u8; 32],
    csr_mepc: [u8; 32],
    csr_mcause: [u8; 32],
    halted: u8,
    loaded_origin: u32,
    loaded_len: usize,
}

impl Default for Rv32IGateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl Rv32IGateLevel {
    /// Construct the reset topology.
    #[must_use]
    pub fn new() -> Self {
        Self {
            memory: DffMemory::new(),
            registers: [[0; 32]; 32],
            pc: [0; 32],
            csr_mstatus: [0; 32],
            csr_mtvec: [0; 32],
            csr_mscratch: [0; 32],
            csr_mepc: [0; 32],
            csr_mcause: [0; 32],
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
        clock_word(&mut self.pc, 0);
        clock_word(&mut self.csr_mstatus, 0);
        clock_word(&mut self.csr_mtvec, 0);
        clock_word(&mut self.csr_mscratch, 0);
        clock_word(&mut self.csr_mepc, 0);
        clock_word(&mut self.csr_mcause, 0);
        clock_bit(&mut self.halted, false);
        self.loaded_origin = 0;
        self.loaded_len = 0;
    }

    /// Clone the complete architectural and installation state.
    #[must_use]
    pub fn get_state(&self) -> Rv32IState {
        Rv32IState {
            registers: std::array::from_fn(|index| word32(&self.registers[index])),
            pc: word32(&self.pc),
            csr_mstatus: word32(&self.csr_mstatus),
            csr_mtvec: word32(&self.csr_mtvec),
            csr_mscratch: word32(&self.csr_mscratch),
            csr_mepc: word32(&self.csr_mepc),
            csr_mcause: word32(&self.csr_mcause),
            memory: self.memory.snapshot(),
            halted: self.halted != 0,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    /// Atomically restore a snapshot validated by the functional contract.
    pub fn restore(&mut self, snapshot: &Rv32IState) -> Result<(), Rv32IError> {
        let mut validator = Rv32ISimulator::new();
        validator.restore(snapshot)?;
        self.restore_validated(snapshot);
        Ok(())
    }

    fn restore_validated(&mut self, snapshot: &Rv32IState) {
        self.memory.restore_snapshot(&snapshot.memory);
        for (register, value) in self.registers.iter_mut().zip(snapshot.registers) {
            clock_word(register, value);
        }
        clock_word(&mut self.pc, snapshot.pc);
        clock_word(&mut self.csr_mstatus, snapshot.csr_mstatus);
        clock_word(&mut self.csr_mtvec, snapshot.csr_mtvec);
        clock_word(&mut self.csr_mscratch, snapshot.csr_mscratch);
        clock_word(&mut self.csr_mepc, snapshot.csr_mepc);
        clock_word(&mut self.csr_mcause, snapshot.csr_mcause);
        clock_bit(&mut self.halted, snapshot.halted);
        self.loaded_origin = snapshot.loaded_origin;
        self.loaded_len = snapshot.loaded_len;
    }

    fn commit_transition(&mut self, next: &Rv32IState) {
        for (address, (&old, &value)) in self.memory.snapshot().iter().zip(&next.memory).enumerate()
        {
            if old != value {
                let wrote = self.memory.write(address as u32, value);
                debug_assert!(wrote);
            }
        }
        for (register, value) in self.registers.iter_mut().zip(next.registers) {
            clock_word(register, value);
        }
        clock_word(&mut self.pc, next.pc);
        clock_word(&mut self.csr_mstatus, next.csr_mstatus);
        clock_word(&mut self.csr_mtvec, next.csr_mtvec);
        clock_word(&mut self.csr_mscratch, next.csr_mscratch);
        clock_word(&mut self.csr_mepc, next.csr_mepc);
        clock_word(&mut self.csr_mcause, next.csr_mcause);
        clock_bit(&mut self.halted, next.halted);
        self.loaded_origin = next.loaded_origin;
        self.loaded_len = next.loaded_len;
    }

    /// Reset and install a program at address zero.
    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), Rv32IError> {
        self.load_at_checked(program, 0)
    }

    /// Reset and install a program at a checked word-aligned origin.
    pub fn load_at_checked(&mut self, program: &[u8], origin: u32) -> Result<(), Rv32IError> {
        let mut validator = Rv32ISimulator::new();
        validator.load_at_checked(program, origin)?;
        self.reset();
        for (offset, byte) in program.iter().copied().enumerate() {
            let wrote = self.memory.write(origin + offset as u32, byte);
            debug_assert!(wrote);
        }
        clock_word(&mut self.pc, origin);
        self.loaded_origin = origin;
        self.loaded_len = program.len();
        Ok(())
    }

    /// Read one checked register.
    pub fn read_register(&self, index: usize) -> Result<u32, Rv32IError> {
        self.registers
            .get(index)
            .map(word32)
            .ok_or(Rv32IError::InvalidRegister { index })
    }

    /// Clock one checked register. Writes to x0 are discarded.
    pub fn write_register(&mut self, index: usize, value: u32) -> Result<(), Rv32IError> {
        let register = self
            .registers
            .get_mut(index)
            .ok_or(Rv32IError::InvalidRegister { index })?;
        if index != 0 {
            clock_word(register, value);
        }
        Ok(())
    }

    /// Read one checked architectural memory byte.
    pub fn read_byte(&self, address: u32) -> Result<u8, Rv32IError> {
        self.memory
            .read(address)
            .ok_or(Rv32IError::DataOutOfRange { address, width: 1 })
    }

    /// Clock one checked architectural memory byte.
    pub fn write_byte(&mut self, address: u32, value: u8) -> Result<(), Rv32IError> {
        if self.memory.write(address, value) {
            Ok(())
        } else {
            Err(Rv32IError::DataOutOfRange { address, width: 1 })
        }
    }

    /// Execute one independent, atomic gate-machine transition.
    pub fn step_checked(&mut self) -> Result<Rv32IStepTrace, Rv32IError> {
        if self.halted != 0 {
            return Err(Rv32IError::Halted);
        }
        let before = self.get_state();
        let mut working = WorkingCpu {
            state: before.clone(),
        };
        let (raw, mnemonic) = working.step_inner()?;
        self.commit_transition(&working.state);
        Ok(Rv32IStepTrace {
            pc_before: before.pc,
            pc_after: working.state.pc,
            raw,
            mnemonic,
            state_before: before,
            state_after: self.get_state(),
        })
    }

    /// Run the installed program transactionally until halt.
    pub fn run_checked(&mut self, max_steps: usize) -> Result<Rv32IExecutionResult, Rv32IError> {
        let before = self.get_state();
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.halted != 0 {
                return Ok(Rv32IExecutionResult {
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
        Err(Rv32IError::StepLimitExceeded { max_steps })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_and_reset_are_exact() {
        assert_eq!(FLIP_FLOP_COUNT, 525_505);
        let state = Rv32IGateLevel::new().get_state();
        assert_eq!(state.memory.len(), RV32I_MEMORY_SIZE);
        assert_eq!(state.registers, [0; 32]);
        assert_eq!(state.pc, 0);
        assert!(!state.halted);
    }

    #[test]
    fn restore_load_and_direct_access_round_trip() {
        let mut cpu = Rv32IGateLevel::new();
        cpu.load_at_checked(&[0x73, 0, 0, 0], 0x100).unwrap();
        cpu.write_register(2, 0x1234_5678).unwrap();
        cpu.write_byte(0xffff, 0xa5).unwrap();
        let snapshot = cpu.get_state();
        assert_eq!(snapshot.pc, 0x100);
        assert_eq!(cpu.read_register(2), Ok(0x1234_5678));
        assert_eq!(cpu.read_byte(0xffff), Ok(0xa5));
        let mut restored = Rv32IGateLevel::new();
        restored.restore(&snapshot).unwrap();
        assert_eq!(restored.get_state(), snapshot);
    }

    #[test]
    fn independent_step_and_run_match_functional_state_and_trace() {
        let program = riscv_simulator::encoding::assemble(&[
            riscv_simulator::encoding::encode_addi(1, 0, 40),
            riscv_simulator::encoding::encode_addi(1, 1, 2),
            riscv_simulator::encoding::encode_ecall(),
        ]);
        let mut gate = Rv32IGateLevel::new();
        let mut functional = Rv32ISimulator::new();
        gate.load_checked(&program).unwrap();
        functional.load_checked(&program).unwrap();
        let gate_result = gate.run_checked(4).unwrap();
        let functional_result = functional.run_checked(4).unwrap();
        assert_eq!(gate_result.final_state, functional_result.final_state);
        assert_eq!(gate_result.traces, functional_result.traces);
    }
}
