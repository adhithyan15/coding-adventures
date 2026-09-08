//! Exact D-flip-flop storage and gate-backed execution for the Spec 07z2 Apple M1 machine.

use aarch64_gatelevel::AArch64GateLevel;
use aarch64_simulator::AArch64State;
pub use apple_m1_simulator::encoding;
use apple_m1_simulator::AppleM1Simulator;
pub use apple_m1_simulator::{
    AppleM1Error, AppleM1ExecutionResult, AppleM1State, AppleM1StepTrace, MEMORY_SIZE,
    REGISTER_COUNT, VECTOR_REGISTER_COUNT, XZR,
};

mod extended;
mod state;

use state::{clock_bit, clock_bits, clock_word128, clock_word64, word128, word64, DffMemory};

/// 524,288 memory + 2,048 GPR + 64 SP + 64 PC + four NZCV + 4,096 vector + one HALT DFFs.
pub const FLIP_FLOP_COUNT: usize =
    MEMORY_SIZE * 8 + REGISTER_COUNT * 64 + 64 + 64 + 4 + VECTOR_REGISTER_COUNT * 128 + 1;

/// Exact sequential Apple M1 architectural state.
#[derive(Debug, Clone)]
pub struct AppleM1GateLevel {
    memory: DffMemory,
    registers: [[u8; 64]; REGISTER_COUNT],
    sp: [u8; 64],
    pc: [u8; 64],
    nzcv: [u8; 4],
    vectors: [[u8; 128]; VECTOR_REGISTER_COUNT],
    halted: u8,
    loaded_origin: u64,
    loaded_len: usize,
}

impl Default for AppleM1GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl AppleM1GateLevel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            memory: DffMemory::new(),
            registers: [[0; 64]; REGISTER_COUNT],
            sp: [0; 64],
            pc: [0; 64],
            nzcv: [0; 4],
            vectors: [[0; 128]; VECTOR_REGISTER_COUNT],
            halted: 0,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    #[must_use]
    pub fn get_state(&self) -> AppleM1State {
        AppleM1State {
            registers: std::array::from_fn(|index| word64(&self.registers[index])),
            sp: word64(&self.sp),
            pc: word64(&self.pc),
            nzcv: self
                .nzcv
                .iter()
                .enumerate()
                .fold(0, |v, (bit, q)| v | (q << bit)),
            vectors: std::array::from_fn(|index| word128(&self.vectors[index])),
            memory: self.memory.snapshot(),
            halted: self.halted != 0,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    pub fn restore(&mut self, snapshot: &AppleM1State) -> Result<(), AppleM1Error> {
        let mut validator = AppleM1Simulator::new();
        validator.restore(snapshot)?;
        self.commit(snapshot);
        Ok(())
    }

    fn commit(&mut self, next: &AppleM1State) {
        self.memory.restore(&next.memory);
        for (register, value) in self.registers.iter_mut().zip(next.registers) {
            clock_word64(register, value);
        }
        clock_word64(&mut self.sp, next.sp);
        clock_word64(&mut self.pc, next.pc);
        clock_bits(&mut self.nzcv, next.nzcv);
        for (register, value) in self.vectors.iter_mut().zip(next.vectors) {
            clock_word128(register, value);
        }
        clock_bit(&mut self.halted, next.halted);
        self.loaded_origin = next.loaded_origin;
        self.loaded_len = next.loaded_len;
    }

    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), AppleM1Error> {
        self.load_at_checked(program, 0)
    }

    pub fn load_at_checked(&mut self, program: &[u8], origin: u64) -> Result<(), AppleM1Error> {
        let mut validator = AppleM1Simulator::new();
        validator.load_at_checked(program, origin)?;
        self.commit(&validator.get_state());
        Ok(())
    }

    pub fn read_register(&self, index: usize) -> Result<u64, AppleM1Error> {
        self.registers
            .get(index)
            .map(word64)
            .ok_or(AppleM1Error::InvalidRegister { index })
    }

    pub fn write_register(&mut self, index: usize, value: u64) -> Result<(), AppleM1Error> {
        let register = self
            .registers
            .get_mut(index)
            .ok_or(AppleM1Error::InvalidRegister { index })?;
        if index != XZR {
            clock_word64(register, value);
        }
        Ok(())
    }

    pub fn read_vector(&self, index: usize) -> Result<u128, AppleM1Error> {
        self.vectors
            .get(index)
            .map(word128)
            .ok_or(AppleM1Error::InvalidVectorRegister { index })
    }

    pub fn write_vector(&mut self, index: usize, value: u128) -> Result<(), AppleM1Error> {
        let register = self
            .vectors
            .get_mut(index)
            .ok_or(AppleM1Error::InvalidVectorRegister { index })?;
        clock_word128(register, value);
        Ok(())
    }

    #[must_use]
    pub fn stack_pointer(&self) -> u64 {
        word64(&self.sp)
    }

    pub fn set_stack_pointer(&mut self, value: u64) {
        clock_word64(&mut self.sp, value);
    }

    #[must_use]
    pub fn nzcv(&self) -> u8 {
        self.get_state().nzcv
    }

    pub fn set_nzcv(&mut self, value: u8) -> Result<(), AppleM1Error> {
        let mut validator = AppleM1Simulator::new();
        validator.set_nzcv(value)?;
        clock_bits(&mut self.nzcv, value);
        Ok(())
    }

    pub fn read_byte(&self, address: u64) -> Result<u8, AppleM1Error> {
        self.memory
            .read(address)
            .ok_or(AppleM1Error::DataOutOfRange { address, width: 1 })
    }

    pub fn write_byte(&mut self, address: u64, value: u8) -> Result<(), AppleM1Error> {
        if self.memory.write(address, value) {
            Ok(())
        } else {
            Err(AppleM1Error::DataOutOfRange { address, width: 1 })
        }
    }

    /// Execute one atomic transition. Base integer instructions use the independent
    /// AArch64 gate engine; Apple FP/NEON instructions use this package's independent decoder.
    pub fn step_checked(&mut self) -> Result<AppleM1StepTrace, AppleM1Error> {
        let before = self.get_state();
        if before.halted {
            return Err(AppleM1Error::Halted);
        }
        if before.pc & 3 != 0 {
            return Err(AppleM1Error::MisalignedFetch { pc: before.pc });
        }
        let end = before.loaded_origin + before.loaded_len as u64;
        if before.pc < before.loaded_origin || before.pc.checked_add(4).is_none_or(|pc| pc > end) {
            return Err(AppleM1Error::FetchOutsideProgram { pc: before.pc });
        }
        let start = before.pc as usize;
        let raw = u32::from_be_bytes(
            before.memory[start..start + 4]
                .try_into()
                .expect("checked fetch"),
        );

        let (mnemonic, after) = if extended::is_extended(raw) {
            let mut after = before.clone();
            let mnemonic = extended::execute(&mut after, raw)?;
            (mnemonic, after)
        } else {
            let mut gate = AArch64GateLevel::new();
            gate.restore(&AArch64State {
                registers: before.registers,
                sp: before.sp,
                pc: before.pc,
                nzcv: before.nzcv,
                memory: before.memory.clone(),
                halted: before.halted,
                loaded_origin: before.loaded_origin,
                loaded_len: before.loaded_len,
            })?;
            let trace = gate.step_checked()?;
            let mut after = before.clone();
            after.registers = trace.state_after.registers;
            after.sp = trace.state_after.sp;
            after.pc = trace.state_after.pc;
            after.nzcv = trace.state_after.nzcv;
            after.memory = trace.state_after.memory;
            after.halted = trace.state_after.halted;
            (trace.mnemonic, after)
        };
        self.commit(&after);
        Ok(AppleM1StepTrace {
            pc_before: before.pc,
            pc_after: after.pc,
            raw,
            mnemonic,
            state_before: before,
            state_after: after,
        })
    }

    pub fn run_checked(
        &mut self,
        max_steps: usize,
    ) -> Result<AppleM1ExecutionResult, AppleM1Error> {
        let before = self.get_state();
        if before.halted {
            return Ok(AppleM1ExecutionResult {
                halted: true,
                steps: 0,
                traces: Vec::new(),
                final_state: before,
            });
        }
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            match self.step_checked() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.commit(&before);
                    return Err(error);
                }
            }
            if self.halted != 0 {
                return Ok(AppleM1ExecutionResult {
                    halted: true,
                    steps: traces.len(),
                    traces,
                    final_state: self.get_state(),
                });
            }
        }
        self.commit(&before);
        Err(AppleM1Error::StepLimitExceeded { max_steps })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_and_dff_state_are_exact() {
        assert_eq!(FLIP_FLOP_COUNT, 530_565);
        let mut cpu = AppleM1GateLevel::new();
        cpu.write_register(2, 0x1234).unwrap();
        cpu.write_vector(31, u128::MAX).unwrap();
        cpu.set_nzcv(0b1010).unwrap();
        assert_eq!(cpu.read_register(2), Ok(0x1234));
        assert_eq!(cpu.read_vector(31), Ok(u128::MAX));
        assert_eq!(cpu.nzcv(), 0b1010);
    }

    #[test]
    fn integer_and_neon_paths_match_functional() {
        let words = [
            encoding::move_wide(1, 2, 0, 7, 0),
            encoding::neon_duplicate(true, 0b00001, 0, 1),
            encoding::halt(),
        ];
        let program = encoding::program(&words);
        let mut gate = AppleM1GateLevel::new();
        let mut functional = AppleM1Simulator::new();
        gate.load_checked(&program).unwrap();
        functional.load_checked(&program).unwrap();
        assert_eq!(gate.run_checked(3), functional.run_checked(3));
    }

    #[test]
    fn step_and_run_faults_are_transactional() {
        let mut cpu = AppleM1GateLevel::new();
        cpu.load_checked(&encoding::program(&[u32::MAX])).unwrap();
        let before = cpu.get_state();
        assert!(matches!(
            cpu.step_checked(),
            Err(AppleM1Error::UnknownInstruction { .. })
        ));
        assert_eq!(cpu.get_state(), before);
        assert!(matches!(
            cpu.run_checked(1),
            Err(AppleM1Error::UnknownInstruction { .. })
        ));
        assert_eq!(cpu.get_state(), before);
    }
}
