//! Checked functional Apple M1 simulator for Spec 07z.

pub mod encoding;
mod extended;

use std::fmt;

use aarch64_simulator::{AArch64Error, AArch64Simulator, AArch64State};
pub use aarch64_simulator::{MEMORY_SIZE, REGISTER_COUNT, XZR};

/// Number of 128-bit NEON/FP registers.
pub const VECTOR_REGISTER_COUNT: usize = 32;

/// Complete architectural and installed-program state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleM1State {
    pub registers: [u64; REGISTER_COUNT],
    pub sp: u64,
    pub pc: u64,
    pub nzcv: u8,
    pub vectors: [u128; VECTOR_REGISTER_COUNT],
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u64,
    pub loaded_len: usize,
}

/// Typed lifecycle, fetch, decode, and data failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppleM1Error {
    InvalidState(String),
    InvalidRegister { index: usize },
    InvalidVectorRegister { index: usize },
    MisalignedProgram { origin: u64 },
    ProgramOutOfRange { origin: u64, length: usize },
    MisalignedFetch { pc: u64 },
    FetchOutsideProgram { pc: u64 },
    MisalignedData { address: u64, width: usize },
    DataOutOfRange { address: u64, width: usize },
    UnknownInstruction { raw: u32, pc: u64 },
    Halted,
    StepLimitExceeded { max_steps: usize },
}

impl fmt::Display for AppleM1Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AppleM1Error {}

impl From<AArch64Error> for AppleM1Error {
    fn from(error: AArch64Error) -> Self {
        match error {
            AArch64Error::InvalidState(message) => Self::InvalidState(message),
            AArch64Error::InvalidRegister { index } => Self::InvalidRegister { index },
            AArch64Error::MisalignedProgram { origin } => Self::MisalignedProgram { origin },
            AArch64Error::ProgramOutOfRange { origin, length } => {
                Self::ProgramOutOfRange { origin, length }
            }
            AArch64Error::MisalignedFetch { pc } => Self::MisalignedFetch { pc },
            AArch64Error::FetchOutsideProgram { pc } => Self::FetchOutsideProgram { pc },
            AArch64Error::MisalignedData { address, width } => {
                Self::MisalignedData { address, width }
            }
            AArch64Error::DataOutOfRange { address, width } => {
                Self::DataOutOfRange { address, width }
            }
            AArch64Error::UnknownInstruction { raw, pc } => Self::UnknownInstruction { raw, pc },
            AArch64Error::Halted => Self::Halted,
            AArch64Error::StepLimitExceeded { max_steps } => Self::StepLimitExceeded { max_steps },
        }
    }
}

/// One complete instruction-boundary transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleM1StepTrace {
    pub pc_before: u64,
    pub pc_after: u64,
    pub raw: u32,
    pub mnemonic: &'static str,
    pub state_before: AppleM1State,
    pub state_after: AppleM1State,
}

/// Successful bounded execution result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleM1ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<AppleM1StepTrace>,
    pub final_state: AppleM1State,
}

/// Exact checked Apple M1 functional machine.
#[derive(Debug, Clone)]
pub struct AppleM1Simulator {
    state: AppleM1State,
}

impl Default for AppleM1Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl AppleM1Simulator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: AppleM1State {
                registers: [0; REGISTER_COUNT],
                sp: 0,
                pc: 0,
                nzcv: 0,
                vectors: [0; VECTOR_REGISTER_COUNT],
                memory: vec![0; MEMORY_SIZE],
                halted: false,
                loaded_origin: 0,
                loaded_len: 0,
            },
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    #[must_use]
    pub fn get_state(&self) -> AppleM1State {
        self.state.clone()
    }

    pub fn restore(&mut self, snapshot: &AppleM1State) -> Result<(), AppleM1Error> {
        Self::validate_state(snapshot)?;
        self.state = snapshot.clone();
        Ok(())
    }

    fn validate_state(state: &AppleM1State) -> Result<(), AppleM1Error> {
        if state.memory.len() != MEMORY_SIZE {
            return Err(AppleM1Error::InvalidState(
                "memory must contain exactly 65,536 bytes".into(),
            ));
        }
        if state.registers[XZR] != 0 {
            return Err(AppleM1Error::InvalidState("XZR must be zero".into()));
        }
        if state.nzcv > 0x0f {
            return Err(AppleM1Error::InvalidState("NZCV must fit four bits".into()));
        }
        if state.pc & 3 != 0 {
            return Err(AppleM1Error::InvalidState("PC must be word aligned".into()));
        }
        let end = state
            .loaded_origin
            .checked_add(state.loaded_len as u64)
            .ok_or_else(|| AppleM1Error::InvalidState("installed range overflow".into()))?;
        if state.loaded_origin & 3 != 0 || end > MEMORY_SIZE as u64 {
            return Err(AppleM1Error::InvalidState(
                "installed range is invalid".into(),
            ));
        }
        Ok(())
    }

    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), AppleM1Error> {
        self.load_at_checked(program, 0)
    }

    pub fn load_at_checked(&mut self, program: &[u8], origin: u64) -> Result<(), AppleM1Error> {
        if origin & 3 != 0 {
            return Err(AppleM1Error::MisalignedProgram { origin });
        }
        let end =
            origin
                .checked_add(program.len() as u64)
                .ok_or(AppleM1Error::ProgramOutOfRange {
                    origin,
                    length: program.len(),
                })?;
        if end > MEMORY_SIZE as u64 {
            return Err(AppleM1Error::ProgramOutOfRange {
                origin,
                length: program.len(),
            });
        }
        self.reset();
        self.state.memory[origin as usize..end as usize].copy_from_slice(program);
        self.state.pc = origin;
        self.state.loaded_origin = origin;
        self.state.loaded_len = program.len();
        Ok(())
    }

    pub fn read_register(&self, index: usize) -> Result<u64, AppleM1Error> {
        self.state
            .registers
            .get(index)
            .copied()
            .ok_or(AppleM1Error::InvalidRegister { index })
    }

    pub fn write_register(&mut self, index: usize, value: u64) -> Result<(), AppleM1Error> {
        let register = self
            .state
            .registers
            .get_mut(index)
            .ok_or(AppleM1Error::InvalidRegister { index })?;
        if index != XZR {
            *register = value;
        }
        Ok(())
    }

    pub fn read_vector(&self, index: usize) -> Result<u128, AppleM1Error> {
        self.state
            .vectors
            .get(index)
            .copied()
            .ok_or(AppleM1Error::InvalidVectorRegister { index })
    }

    pub fn write_vector(&mut self, index: usize, value: u128) -> Result<(), AppleM1Error> {
        *self
            .state
            .vectors
            .get_mut(index)
            .ok_or(AppleM1Error::InvalidVectorRegister { index })? = value;
        Ok(())
    }

    #[must_use]
    pub fn stack_pointer(&self) -> u64 {
        self.state.sp
    }

    pub fn set_stack_pointer(&mut self, value: u64) {
        self.state.sp = value;
    }

    #[must_use]
    pub fn nzcv(&self) -> u8 {
        self.state.nzcv
    }

    pub fn set_nzcv(&mut self, value: u8) -> Result<(), AppleM1Error> {
        if value > 0x0f {
            return Err(AppleM1Error::InvalidState("NZCV must fit four bits".into()));
        }
        self.state.nzcv = value;
        Ok(())
    }

    pub fn read_byte(&self, address: u64) -> Result<u8, AppleM1Error> {
        self.state
            .memory
            .get(address as usize)
            .copied()
            .ok_or(AppleM1Error::DataOutOfRange { address, width: 1 })
    }

    pub fn write_byte(&mut self, address: u64, value: u8) -> Result<(), AppleM1Error> {
        *self
            .state
            .memory
            .get_mut(address as usize)
            .ok_or(AppleM1Error::DataOutOfRange { address, width: 1 })? = value;
        Ok(())
    }

    fn fetch(&self) -> Result<u32, AppleM1Error> {
        let pc = self.state.pc;
        if pc & 3 != 0 {
            return Err(AppleM1Error::MisalignedFetch { pc });
        }
        let installed_end = self.state.loaded_origin + self.state.loaded_len as u64;
        if pc < self.state.loaded_origin || pc.checked_add(4).is_none_or(|end| end > installed_end)
        {
            return Err(AppleM1Error::FetchOutsideProgram { pc });
        }
        let start = pc as usize;
        Ok(u32::from_be_bytes(
            self.state.memory[start..start + 4]
                .try_into()
                .expect("checked fetch"),
        ))
    }

    fn integer_state(&self) -> AArch64State {
        AArch64State {
            registers: self.state.registers,
            sp: self.state.sp,
            pc: self.state.pc,
            nzcv: self.state.nzcv,
            memory: self.state.memory.clone(),
            halted: self.state.halted,
            loaded_origin: self.state.loaded_origin,
            loaded_len: self.state.loaded_len,
        }
    }

    fn commit_integer_state(&mut self, state: AArch64State) {
        self.state.registers = state.registers;
        self.state.sp = state.sp;
        self.state.pc = state.pc;
        self.state.nzcv = state.nzcv;
        self.state.memory = state.memory;
        self.state.halted = state.halted;
        self.state.loaded_origin = state.loaded_origin;
        self.state.loaded_len = state.loaded_len;
    }

    /// Fetch, strictly decode, and atomically execute one instruction.
    pub fn step_checked(&mut self) -> Result<AppleM1StepTrace, AppleM1Error> {
        if self.state.halted {
            return Err(AppleM1Error::Halted);
        }
        let before = self.get_state();
        let raw = self.fetch()?;

        // Apple M1 teaching semantics retain SVC as a non-halting no-op.
        let mnemonic = if raw & 0xffe0_001f == 0xd400_0001 {
            self.state.pc = before.pc.wrapping_add(4);
            "svc"
        } else if extended::is_extended(raw) {
            let mut working = before.clone();
            let mnemonic = extended::execute(&mut working, raw)?;
            self.state = working;
            mnemonic
        } else {
            let mut integer = AArch64Simulator::new();
            integer.restore(&self.integer_state())?;
            let trace = integer.step_checked()?;
            self.commit_integer_state(trace.state_after);
            trace.mnemonic
        };

        Ok(AppleM1StepTrace {
            pc_before: before.pc,
            pc_after: self.state.pc,
            raw,
            mnemonic,
            state_before: before,
            state_after: self.get_state(),
        })
    }

    pub fn run_checked(
        &mut self,
        max_steps: usize,
    ) -> Result<AppleM1ExecutionResult, AppleM1Error> {
        let before = self.get_state();
        let mut traces = Vec::new();
        if before.halted {
            return Ok(AppleM1ExecutionResult {
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
                    self.state = before;
                    return Err(error);
                }
            }
            if self.state.halted {
                return Ok(AppleM1ExecutionResult {
                    halted: true,
                    steps: traces.len(),
                    traces,
                    final_state: self.get_state(),
                });
            }
        }
        self.state = before;
        Err(AppleM1Error::StepLimitExceeded { max_steps })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aarch64_simulator::encoding as a64;

    #[test]
    fn integer_base_vectors_and_svc_lifecycle_are_exact() {
        let words = [
            a64::move_wide(1, 2, 0, 7, 0),
            a64::add_sub_immediate(1, 0, 0, 5, 0, 0, 1),
            a64::svc(42),
            a64::halt(),
        ];
        let mut cpu = AppleM1Simulator::new();
        cpu.load_checked(&a64::program(&words)).unwrap();
        let result = cpu.run_checked(words.len()).unwrap();
        assert!(result.halted);
        assert_eq!(result.steps, words.len());
        assert_eq!(result.final_state.registers[1], 12);
        assert_eq!(result.traces[2].mnemonic, "svc");
    }

    #[test]
    fn vectors_and_restore_are_checked() {
        let mut cpu = AppleM1Simulator::new();
        cpu.write_vector(31, u128::MAX).unwrap();
        assert_eq!(cpu.read_vector(31), Ok(u128::MAX));
        assert_eq!(
            cpu.read_vector(VECTOR_REGISTER_COUNT),
            Err(AppleM1Error::InvalidVectorRegister {
                index: VECTOR_REGISTER_COUNT,
            })
        );
        let snapshot = cpu.get_state();
        cpu.reset();
        cpu.restore(&snapshot).unwrap();
        assert_eq!(cpu.get_state(), snapshot);
    }
}
