//! Exact gate-backed persistent state for the Spec 07v2 AArch64 machine.

use aarch64_simulator::{
    AArch64Error, AArch64ExecutionResult, AArch64Simulator, AArch64State, AArch64StepTrace,
    MEMORY_SIZE, REGISTER_COUNT, XZR,
};

mod engine;
mod state;

use state::{clock_bit, clock_bits, clock_word, word64, DffMemory};

/// 524,288 memory + 2,048 GPR + 64 SP + 64 PC + four NZCV + one HALT DFFs.
pub const FLIP_FLOP_COUNT: usize = MEMORY_SIZE * 8 + REGISTER_COUNT * 64 + 64 + 64 + 4 + 1;

/// Exact sequential architectural state for the checked AArch64 machine.
#[derive(Debug, Clone)]
pub struct AArch64GateLevel {
    memory: DffMemory,
    registers: [[u8; 64]; REGISTER_COUNT],
    sp: [u8; 64],
    pc: [u8; 64],
    nzcv: [u8; 4],
    halted: u8,
    loaded_origin: u64,
    loaded_len: usize,
}

impl Default for AArch64GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl AArch64GateLevel {
    #[must_use]
    pub fn new() -> Self {
        let mut machine = Self {
            memory: DffMemory::new(),
            registers: [[0; 64]; REGISTER_COUNT],
            sp: [0; 64],
            pc: [0; 64],
            nzcv: [0; 4],
            halted: 0,
            loaded_origin: 0,
            loaded_len: 0,
        };
        machine.reset();
        machine
    }

    pub fn reset(&mut self) {
        self.memory.clear();
        for register in &mut self.registers {
            clock_word(register, 0);
        }
        clock_word(&mut self.sp, 0);
        clock_word(&mut self.pc, 0);
        clock_bits(&mut self.nzcv, 0);
        clock_bit(&mut self.halted, false);
        self.loaded_origin = 0;
        self.loaded_len = 0;
    }

    #[must_use]
    pub fn get_state(&self) -> AArch64State {
        AArch64State {
            registers: std::array::from_fn(|index| word64(&self.registers[index])),
            sp: word64(&self.sp),
            pc: word64(&self.pc),
            nzcv: self
                .nzcv
                .iter()
                .enumerate()
                .fold(0, |value, (bit, q)| value | (q << bit)),
            memory: self.memory.snapshot(),
            halted: self.halted != 0,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    pub fn restore(&mut self, snapshot: &AArch64State) -> Result<(), AArch64Error> {
        let mut validator = AArch64Simulator::new();
        validator.restore(snapshot)?;
        self.restore_validated(snapshot);
        Ok(())
    }

    fn restore_validated(&mut self, snapshot: &AArch64State) {
        self.memory.restore_snapshot(&snapshot.memory);
        for (register, value) in self.registers.iter_mut().zip(snapshot.registers) {
            clock_word(register, value);
        }
        clock_word(&mut self.sp, snapshot.sp);
        clock_word(&mut self.pc, snapshot.pc);
        clock_bits(&mut self.nzcv, snapshot.nzcv);
        clock_bit(&mut self.halted, snapshot.halted);
        self.loaded_origin = snapshot.loaded_origin;
        self.loaded_len = snapshot.loaded_len;
    }

    fn commit_transition(&mut self, next: &AArch64State) {
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
        clock_word(&mut self.sp, next.sp);
        clock_word(&mut self.pc, next.pc);
        clock_bits(&mut self.nzcv, next.nzcv);
        clock_bit(&mut self.halted, next.halted);
        self.loaded_origin = next.loaded_origin;
        self.loaded_len = next.loaded_len;
    }

    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), AArch64Error> {
        self.load_at_checked(program, 0)
    }

    pub fn load_at_checked(&mut self, program: &[u8], origin: u64) -> Result<(), AArch64Error> {
        let mut validator = AArch64Simulator::new();
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

    pub fn read_register(&self, index: usize) -> Result<u64, AArch64Error> {
        self.registers
            .get(index)
            .map(word64)
            .ok_or(AArch64Error::InvalidRegister { index })
    }

    pub fn write_register(&mut self, index: usize, value: u64) -> Result<(), AArch64Error> {
        let register = self
            .registers
            .get_mut(index)
            .ok_or(AArch64Error::InvalidRegister { index })?;
        if index != XZR {
            clock_word(register, value);
        }
        Ok(())
    }

    #[must_use]
    pub fn stack_pointer(&self) -> u64 {
        word64(&self.sp)
    }

    pub fn set_stack_pointer(&mut self, value: u64) {
        clock_word(&mut self.sp, value);
    }

    #[must_use]
    pub fn nzcv(&self) -> u8 {
        self.get_state().nzcv
    }

    pub fn set_nzcv(&mut self, value: u8) -> Result<(), AArch64Error> {
        let mut validator = AArch64Simulator::new();
        validator.set_nzcv(value)?;
        clock_bits(&mut self.nzcv, value);
        Ok(())
    }

    pub fn read_byte(&self, address: u64) -> Result<u8, AArch64Error> {
        self.memory
            .read(address)
            .ok_or(AArch64Error::DataOutOfRange { address, width: 1 })
    }

    pub fn write_byte(&mut self, address: u64, value: u8) -> Result<(), AArch64Error> {
        if self.memory.write(address, value) {
            Ok(())
        } else {
            Err(AArch64Error::DataOutOfRange { address, width: 1 })
        }
    }

    /// Execute one transition through the independent gate decode engine.
    pub fn step_checked(&mut self) -> Result<AArch64StepTrace, AArch64Error> {
        let before = self.get_state();
        let mut working = engine::WorkingCpu {
            state: before.clone(),
        };
        let (raw, mnemonic) = working.step_inner()?;
        self.commit_transition(&working.state);
        Ok(AArch64StepTrace {
            pc_before: before.pc,
            pc_after: word64(&self.pc),
            raw,
            mnemonic,
            state_before: before,
            state_after: self.get_state(),
        })
    }

    pub fn run_checked(
        &mut self,
        max_steps: usize,
    ) -> Result<AArch64ExecutionResult, AArch64Error> {
        let before = self.get_state();
        let mut traces = Vec::new();
        if before.halted {
            return Ok(AArch64ExecutionResult {
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
                return Ok(AArch64ExecutionResult {
                    halted: true,
                    steps: traces.len(),
                    traces,
                    final_state: self.get_state(),
                });
            }
        }
        self.restore_validated(&before);
        Err(AArch64Error::StepLimitExceeded { max_steps })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aarch64_simulator::encoding as enc;

    #[test]
    fn topology_and_checked_storage_are_exact() {
        assert_eq!(FLIP_FLOP_COUNT, 526_469);
        let mut machine = AArch64GateLevel::new();
        machine.write_register(30, u64::MAX).unwrap();
        machine.write_register(XZR, u64::MAX).unwrap();
        machine.set_stack_pointer(0x1234);
        machine.set_nzcv(0b1010).unwrap();
        assert_eq!(machine.read_register(30), Ok(u64::MAX));
        assert_eq!(machine.read_register(XZR), Ok(0));
        assert_eq!(machine.stack_pointer(), 0x1234);
        assert_eq!(machine.nzcv(), 0b1010);
    }

    #[test]
    fn independent_integer_bootstrap_matches_functional_state() {
        let program = enc::program(&[
            enc::move_wide(1, 2, 0, 7, 0),
            enc::add_sub_immediate(1, 0, 1, 1, 0, 0, 1),
            enc::add_sub_immediate(1, 1, 0, 3, 0, 1, 2),
            enc::move_wide(1, 3, 1, 0xabcd, 2),
            enc::nop(),
            enc::halt(),
        ]);
        let mut functional = AArch64Simulator::new();
        let mut gate = AArch64GateLevel::new();
        functional.load_checked(&program).unwrap();
        gate.load_checked(&program).unwrap();
        while !functional.get_state().halted {
            let expected = functional.step_checked().unwrap();
            let actual = gate.step_checked().unwrap();
            assert_eq!(actual.raw, expected.raw);
            assert_eq!(actual.mnemonic, expected.mnemonic);
            assert_eq!(actual.state_after, expected.state_after);
        }
    }

    #[test]
    fn independent_branches_match_functional_state() {
        let programs = [
            enc::program(&[
                enc::branch(false, 2),
                enc::move_wide(1, 2, 0, 1, 0),
                enc::halt(),
            ]),
            enc::program(&[
                enc::compare_branch(1, false, 2, 0),
                enc::move_wide(1, 2, 0, 1, 0),
                enc::halt(),
            ]),
            enc::program(&[
                enc::test_branch(63, false, 2, 0),
                enc::move_wide(1, 2, 0, 1, 0),
                enc::halt(),
            ]),
        ];
        for program in programs {
            let mut functional = AArch64Simulator::new();
            let mut gate = AArch64GateLevel::new();
            functional.load_checked(&program).unwrap();
            gate.load_checked(&program).unwrap();
            assert_eq!(
                gate.step_checked().unwrap().state_after,
                functional.step_checked().unwrap().state_after
            );
        }
    }

    #[test]
    fn independent_logic_shift_and_memory_match_functional_state() {
        let words = [
            enc::move_wide(1, 2, 0, 0x100, 0),
            enc::move_wide(1, 2, 0, 0xff80, 1),
            enc::logical_register(1, 1, (0, 4), 0, 1, XZR as u32, 2),
            enc::add_sub_register(1, 0, 1, (1, 1), 2, 1, 3),
            enc::logical_immediate(1, 2, 1, 0, 7, 3, 4),
            enc::load_store_unsigned(1, 0, 0, 0, 0, 1),
            enc::load_store_unsigned(1, 0, 2, 0, 0, 5),
            enc::halt(),
        ];
        let program = enc::program(&words);
        let mut functional = AArch64Simulator::new();
        let mut gate = AArch64GateLevel::new();
        functional.load_checked(&program).unwrap();
        gate.load_checked(&program).unwrap();
        for _ in words {
            let expected = functional.step_checked().unwrap();
            let actual = gate.step_checked().unwrap();
            assert_eq!(actual.mnemonic, expected.mnemonic);
            assert_eq!(actual.state_after, expected.state_after);
        }
    }

    #[test]
    fn independent_complex_integer_families_match_functional_state() {
        let words = [
            enc::move_wide(1, 2, 0, 100, 0),
            enc::move_wide(1, 2, 0, 7, 1),
            enc::move_wide(1, 0, 0, 0, 2),
            enc::data_two_source(1, 1, 2, 0, 3),
            enc::data_two_source(1, 1, 3, 2, 4),
            enc::data_two_source(1, 1, 8, 0, 5),
            enc::data_two_source(1, 1, 9, 0, 6),
            enc::data_two_source(1, 1, 10, 2, 7),
            enc::data_two_source(1, 1, 11, 0, 8),
            enc::data_one_source(1, 0, 0, 9),
            enc::data_one_source(1, 1, 0, 10),
            enc::data_one_source(1, 2, 0, 11),
            enc::data_one_source(1, 3, 0, 12),
            enc::data_one_source(1, 4, 0, 13),
            enc::multiply_add(1, 0, 1, false, 1, 0, 14),
            enc::multiply_add(1, 0, 1, true, 1, 0, 15),
            enc::multiply_add(1, 1, 1, false, 0, 2, 16),
            enc::multiply_add(1, 2, 1, false, 0, 0, 17),
            enc::conditional_select(1, false, 1, 0, false, 0, 18),
            enc::conditional_select(1, false, 1, 1, true, 0, 19),
            enc::svc(42),
            enc::halt(),
        ];
        let program = enc::program(&words);
        let mut functional = AArch64Simulator::new();
        let mut gate = AArch64GateLevel::new();
        functional.load_checked(&program).unwrap();
        gate.load_checked(&program).unwrap();
        functional.set_nzcv(0b0100).unwrap();
        gate.set_nzcv(0b0100).unwrap();
        for _ in words {
            let expected = functional.step_checked().unwrap();
            let actual = gate.step_checked().unwrap();
            assert_eq!(actual.mnemonic, expected.mnemonic);
            assert_eq!(actual.state_after, expected.state_after);
        }
    }
}
