//! Checked IBM System/370 functional simulation.
//!
//! System/370 kept the System/360 application architecture and added a small
//! set of especially useful character operations. This crate therefore owns a
//! complete System/370 state while using the checked System/360 core only as a
//! private executor for instructions whose architecture is unchanged.

use ibm_system360_simulator as system360;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Installed main-storage size for this bounded model.
pub const MEMORY_BYTES: usize = 1 << 20;
const ADDRESS_MASK: u32 = 0x00ff_ffff;

/// Complete, owned System/370 architectural state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct System370State {
    pub gprs: [u32; 16],
    /// Raw long hexadecimal-floating values for architectural FPRs 0, 2, 4, 6.
    pub fprs: [u64; 4],
    pub pc: u32,
    pub condition_code: u8,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u32,
    pub loaded_len: usize,
}

/// One successfully executed System/370 instruction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: Vec<u8>,
    pub width: u8,
    pub mnemonic: String,
    pub description: String,
}

/// Successful bounded execution and its complete final state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub final_state: System370State,
    pub traces: Vec<StepTrace>,
}

/// Typed checked failures at the System/370 boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum System370Error {
    EmptyProgram,
    MisalignedProgram { origin: u32 },
    ProgramOutOfRange { origin: u32, bytes: usize },
    FetchOutsideProgram { pc: u32 },
    TruncatedInstruction { pc: u32, needed: usize },
    UnknownOpcode { opcode: u8, pc: u32 },
    UnsupportedSupervisorCall { number: u8 },
    MisalignedAccess { address: u32, alignment: usize },
    MemoryOutOfRange { address: u32, bytes: usize },
    InvalidRegister { bank: &'static str, index: usize },
    InvalidState { reason: &'static str },
    Specification { reason: &'static str },
    FixedPointOverflow,
    FixedPointDivide,
    FloatingExponent,
    FloatingDivide,
    Halted,
    MaxStepsExceeded { max_steps: usize },
}

impl Display for System370Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => write!(f, "System/370 program is empty"),
            Self::MisalignedProgram { origin } => {
                write!(f, "program origin {origin:#08x} is not halfword aligned")
            }
            Self::ProgramOutOfRange { origin, bytes } => write!(
                f,
                "program at {origin:#08x} with {bytes} bytes exceeds installed storage"
            ),
            Self::FetchOutsideProgram { pc } => write!(
                f,
                "instruction fetch at {pc:#08x} is outside the installed program"
            ),
            Self::TruncatedInstruction { pc, needed } => write!(
                f,
                "truncated instruction at {pc:#08x}: {needed} bytes required"
            ),
            Self::UnknownOpcode { opcode, pc } => {
                write!(f, "unknown opcode {opcode:#04x} at {pc:#08x}")
            }
            Self::UnsupportedSupervisorCall { number } => {
                write!(f, "unsupported supervisor call {number}")
            }
            Self::MisalignedAccess { address, alignment } => write!(
                f,
                "storage address {address:#08x} is not aligned to {alignment} bytes"
            ),
            Self::MemoryOutOfRange { address, bytes } => write!(
                f,
                "storage range {address:#08x}..+{bytes} exceeds installed storage"
            ),
            Self::InvalidRegister { bank, index } => {
                write!(f, "{bank} register {index} is invalid")
            }
            Self::InvalidState { reason } => write!(f, "invalid System/370 state: {reason}"),
            Self::Specification { reason } => {
                write!(f, "System/370 specification exception: {reason}")
            }
            Self::FixedPointOverflow => write!(f, "System/370 fixed-point overflow"),
            Self::FixedPointDivide => write!(f, "System/370 fixed-point divide exception"),
            Self::FloatingExponent => write!(f, "System/370 floating-point exponent exception"),
            Self::FloatingDivide => write!(f, "System/370 floating-point divide exception"),
            Self::Halted => write!(f, "System/370 CPU is halted"),
            Self::MaxStepsExceeded { max_steps } => {
                write!(f, "execution exceeded {max_steps} steps")
            }
        }
    }
}

impl Error for System370Error {}

/// Checked functional System/370 CPU.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct System370Simulator {
    state: System370State,
}

impl Default for System370Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl System370Simulator {
    /// Construct a reset CPU with one MiB of zeroed storage.
    pub fn new() -> Self {
        Self {
            state: System370State {
                gprs: [0; 16],
                fprs: [0; 4],
                pc: 0,
                condition_code: 0,
                memory: vec![0; MEMORY_BYTES],
                halted: false,
                loaded_origin: 0,
                loaded_len: 0,
            },
        }
    }

    /// Reset registers, storage, halt state, and installed-program metadata.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Return a complete owned snapshot.
    pub fn get_state(&self) -> System370State {
        self.state.clone()
    }

    /// Atomically restore a complete validated snapshot.
    pub fn restore(&mut self, state: System370State) -> Result<(), System370Error> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    /// Atomically reset and install a raw program image.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<usize, System370Error> {
        if program.is_empty() {
            return Err(System370Error::EmptyProgram);
        }
        if !origin.is_multiple_of(2) {
            return Err(System370Error::MisalignedProgram { origin });
        }
        let end = (origin as usize).checked_add(program.len()).ok_or(
            System370Error::ProgramOutOfRange {
                origin,
                bytes: program.len(),
            },
        )?;
        if origin > ADDRESS_MASK || end > MEMORY_BYTES {
            return Err(System370Error::ProgramOutOfRange {
                origin,
                bytes: program.len(),
            });
        }
        let mut next = Self::new();
        next.state.pc = origin;
        next.state.loaded_origin = origin;
        next.state.loaded_len = program.len();
        next.state.memory[origin as usize..end].copy_from_slice(program);
        *self = next;
        Ok(program.len())
    }

    pub fn read_gpr(&self, index: usize) -> Result<u32, System370Error> {
        self.state
            .gprs
            .get(index)
            .copied()
            .ok_or(System370Error::InvalidRegister {
                bank: "general",
                index,
            })
    }

    pub fn write_gpr(&mut self, index: usize, value: u32) -> Result<(), System370Error> {
        *self
            .state
            .gprs
            .get_mut(index)
            .ok_or(System370Error::InvalidRegister {
                bank: "general",
                index,
            })? = value;
        Ok(())
    }

    fn fpr_slot(index: usize) -> Result<usize, System370Error> {
        if index <= 6 && index.is_multiple_of(2) {
            Ok(index / 2)
        } else {
            Err(System370Error::InvalidRegister {
                bank: "floating-point",
                index,
            })
        }
    }

    pub fn read_fpr(&self, index: usize) -> Result<u64, System370Error> {
        Ok(self.state.fprs[Self::fpr_slot(index)?])
    }

    pub fn write_fpr(&mut self, index: usize, value: u64) -> Result<(), System370Error> {
        let slot = Self::fpr_slot(index)?;
        self.state.fprs[slot] = value;
        Ok(())
    }

    pub fn set_program_counter(&mut self, pc: u32) -> Result<(), System370Error> {
        if pc > ADDRESS_MASK || !pc.is_multiple_of(2) {
            return Err(System370Error::InvalidState {
                reason: "instruction address is invalid",
            });
        }
        self.state.pc = pc;
        Ok(())
    }

    pub fn read_memory(&self, address: u32, bytes: usize) -> Result<Vec<u8>, System370Error> {
        Ok(self.state.memory[Self::range(address, bytes)?].to_vec())
    }

    pub fn write_memory(&mut self, address: u32, data: &[u8]) -> Result<(), System370Error> {
        let range = Self::range(address, data.len())?;
        self.state.memory[range].copy_from_slice(data);
        Ok(())
    }

    /// Execute one instruction atomically.
    pub fn step(&mut self) -> Result<StepTrace, System370Error> {
        if self.state.halted {
            return Err(System370Error::Halted);
        }
        let before = self.state.clone();
        match self.step_inner() {
            Ok(trace) => Ok(trace),
            Err(error) => {
                self.state = before;
                Err(error)
            }
        }
    }

    /// Execute until `SVC 0` or atomically fail at the step bound.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, System370Error> {
        let before = self.state.clone();
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => {
                    traces.push(trace);
                    if self.state.halted {
                        return Ok(ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            final_state: self.get_state(),
                            traces,
                        });
                    }
                }
                Err(error) => {
                    self.state = before;
                    return Err(error);
                }
            }
        }
        self.state = before;
        Err(System370Error::MaxStepsExceeded { max_steps })
    }

    /// Load and run on an owned CPU value.
    pub fn execute(
        mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, System370Error> {
        self.load(program, origin)?;
        self.run(max_steps)
    }

    fn step_inner(&mut self) -> Result<StepTrace, System370Error> {
        let opcode = self.fetch_opcode()?;
        if !matches!(opcode, 0x0e | 0x0f | 0xbd..=0xbf) {
            return self.step_inherited();
        }
        let width = if opcode <= 0x0f { 2 } else { 4 };
        let raw = self.fetch_exact(width)?;
        let pc_before = self.state.pc;
        self.state.pc = self.state.pc.wrapping_add(width as u32) & ADDRESS_MASK;
        let (mnemonic, description) = match opcode {
            0x0e => self.exec_mvcl(&raw)?,
            0x0f => self.exec_clcl(&raw)?,
            0xbd => self.exec_clm(&raw)?,
            0xbe => self.exec_stcm(&raw)?,
            0xbf => self.exec_icm(&raw)?,
            _ => unreachable!(),
        };
        Ok(StepTrace {
            pc_before,
            pc_after: self.state.pc,
            raw,
            width: width as u8,
            mnemonic: mnemonic.to_owned(),
            description,
        })
    }

    fn step_inherited(&mut self) -> Result<StepTrace, System370Error> {
        let mut inherited = system360::System360Simulator::new();
        inherited
            .restore(to_system360_state(&self.state))
            .map_err(map_system360_error)?;
        let trace = inherited.step().map_err(map_system360_error)?;
        self.state = from_system360_state(inherited.get_state());
        Ok(StepTrace {
            pc_before: trace.pc_before,
            pc_after: trace.pc_after,
            raw: trace.raw,
            width: trace.width,
            mnemonic: trace.mnemonic,
            description: trace.description,
        })
    }

    fn exec_mvcl(&mut self, raw: &[u8]) -> Result<(&'static str, String), System370Error> {
        let r1 = usize::from(raw[1] >> 4);
        let r2 = usize::from(raw[1] & 0x0f);
        self.long_pairs(r1, r2)?;
        let destination = self.state.gprs[r1] & ADDRESS_MASK;
        let destination_len = self.state.gprs[r1 + 1] & ADDRESS_MASK;
        let source = self.state.gprs[r2] & ADDRESS_MASK;
        let source_len = self.state.gprs[r2 + 1] & ADDRESS_MASK;
        let pad = (self.state.gprs[r2 + 1] >> 24) as u8;
        let copied = destination_len.min(source_len);
        // Recognition of destructive overlap suppresses the operation: no
        // storage byte is referenced and neither operand pair progresses.
        if copied != 0 && destination > source && destination.wrapping_sub(source) < copied {
            self.state.condition_code = 3;
            return Ok(("MVCL", format!("MVCL R{r1},R{r2} (destructive overlap)")));
        }
        if destination_len != 0 {
            Self::range(destination, destination_len as usize)?;
        }
        if copied != 0 {
            Self::range(source, copied as usize)?;
        }
        for offset in 0..destination_len {
            let byte = if offset < source_len {
                self.state.memory[(source + offset) as usize]
            } else {
                pad
            };
            self.state.memory[(destination + offset) as usize] = byte;
        }
        self.state.gprs[r1] = destination.wrapping_add(destination_len) & ADDRESS_MASK;
        self.state.gprs[r1 + 1] &= 0xff00_0000;
        self.state.gprs[r2] = source.wrapping_add(copied) & ADDRESS_MASK;
        self.state.gprs[r2 + 1] = (self.state.gprs[r2 + 1] & 0xff00_0000) | (source_len - copied);
        self.state.condition_code = ordering_code(destination_len.cmp(&source_len));
        Ok(("MVCL", format!("MVCL R{r1},R{r2}")))
    }

    fn exec_clcl(&mut self, raw: &[u8]) -> Result<(&'static str, String), System370Error> {
        let r1 = usize::from(raw[1] >> 4);
        let r2 = usize::from(raw[1] & 0x0f);
        self.long_pairs(r1, r2)?;
        let mut first_address = self.state.gprs[r1] & ADDRESS_MASK;
        let mut first_len = self.state.gprs[r1 + 1] & ADDRESS_MASK;
        let mut second_address = self.state.gprs[r2] & ADDRESS_MASK;
        let mut second_len = self.state.gprs[r2 + 1] & ADDRESS_MASK;
        let pad = (self.state.gprs[r2 + 1] >> 24) as u8;
        let mut condition_code = 0;
        while first_len != 0 || second_len != 0 {
            let first = if first_len == 0 {
                pad
            } else {
                self.state.memory[Self::range(first_address, 1)?.start]
            };
            let second = if second_len == 0 {
                pad
            } else {
                self.state.memory[Self::range(second_address, 1)?.start]
            };
            if first != second {
                condition_code = ordering_code(first.cmp(&second));
                break;
            }
            if first_len != 0 {
                first_address = first_address.wrapping_add(1) & ADDRESS_MASK;
                first_len -= 1;
            }
            if second_len != 0 {
                second_address = second_address.wrapping_add(1) & ADDRESS_MASK;
                second_len -= 1;
            }
        }
        self.state.gprs[r1] = first_address;
        self.state.gprs[r1 + 1] = (self.state.gprs[r1 + 1] & 0xff00_0000) | first_len;
        self.state.gprs[r2] = second_address;
        self.state.gprs[r2 + 1] = (self.state.gprs[r2 + 1] & 0xff00_0000) | second_len;
        self.state.condition_code = condition_code;
        Ok(("CLCL", format!("CLCL R{r1},R{r2}")))
    }

    fn exec_clm(&mut self, raw: &[u8]) -> Result<(&'static str, String), System370Error> {
        let register = usize::from(raw[1] >> 4);
        let mask = raw[1] & 0x0f;
        let address = self.effective_address(raw);
        let mut consumed = 0_u32;
        let mut condition_code = 0;
        if mask == 0 {
            Self::range(address, 1)?;
        }
        for byte_index in 0..4 {
            if mask & (8 >> byte_index) == 0 {
                continue;
            }
            let register_byte = (self.state.gprs[register] >> (24 - byte_index * 8)) as u8;
            let storage_byte = self.state.memory[Self::range(address + consumed, 1)?.start];
            consumed += 1;
            if register_byte != storage_byte {
                condition_code = ordering_code(register_byte.cmp(&storage_byte));
                break;
            }
        }
        self.state.condition_code = condition_code;
        Ok(("CLM", format!("CLM R{register},{mask:#x},{address:#08x}")))
    }

    fn exec_stcm(&mut self, raw: &[u8]) -> Result<(&'static str, String), System370Error> {
        let register = usize::from(raw[1] >> 4);
        let mask = raw[1] & 0x0f;
        let address = self.effective_address(raw);
        let bytes: Vec<_> = (0..4)
            .filter(|byte_index| mask & (8 >> byte_index) != 0)
            .map(|byte_index| (self.state.gprs[register] >> (24 - byte_index * 8)) as u8)
            .collect();
        if bytes.is_empty() {
            Self::range(address, 1)?;
        } else {
            let range = Self::range(address, bytes.len())?;
            self.state.memory[range].copy_from_slice(&bytes);
        }
        Ok(("STCM", format!("STCM R{register},{mask:#x},{address:#08x}")))
    }

    fn exec_icm(&mut self, raw: &[u8]) -> Result<(&'static str, String), System370Error> {
        let register = usize::from(raw[1] >> 4);
        let mask = raw[1] & 0x0f;
        let address = self.effective_address(raw);
        if mask == 0 {
            Self::range(address, 1)?;
            self.state.condition_code = 0;
            return Ok(("ICM", format!("ICM R{register},0,{address:#08x}")));
        }
        let count = mask.count_ones() as usize;
        let bytes = self.state.memory[Self::range(address, count)?].to_vec();
        let mut value = self.state.gprs[register];
        let mut source = 0;
        for byte_index in 0..4 {
            if mask & (8 >> byte_index) == 0 {
                continue;
            }
            let shift = 24 - byte_index * 8;
            value = (value & !(0xff << shift)) | (u32::from(bytes[source]) << shift);
            source += 1;
        }
        self.state.gprs[register] = value;
        self.state.condition_code = if bytes.iter().all(|byte| *byte == 0) {
            0
        } else if bytes[0] & 0x80 != 0 {
            1
        } else {
            2
        };
        Ok(("ICM", format!("ICM R{register},{mask:#x},{address:#08x}")))
    }

    fn long_pairs(&self, r1: usize, r2: usize) -> Result<(), System370Error> {
        if r1 > 14 || r2 > 14 || !r1.is_multiple_of(2) || !r2.is_multiple_of(2) {
            Err(System370Error::Specification {
                reason: "long instruction register pairs must start at even registers",
            })
        } else {
            Ok(())
        }
    }

    fn effective_address(&self, raw: &[u8]) -> u32 {
        let base = usize::from(raw[2] >> 4);
        let displacement = (u32::from(raw[2] & 0x0f) << 8) | u32::from(raw[3]);
        (if base == 0 { 0 } else { self.state.gprs[base] }).wrapping_add(displacement)
            & ADDRESS_MASK
    }

    fn fetch_opcode(&self) -> Result<u8, System370Error> {
        if !self.state.pc.is_multiple_of(2) {
            return Err(System370Error::InvalidState {
                reason: "instruction address is not halfword aligned",
            });
        }
        let start = self.state.pc as usize;
        let low = self.state.loaded_origin as usize;
        let high = low + self.state.loaded_len;
        if start < low || start >= high {
            return Err(System370Error::FetchOutsideProgram { pc: self.state.pc });
        }
        Ok(self.state.memory[start])
    }

    fn fetch_exact(&self, width: usize) -> Result<Vec<u8>, System370Error> {
        let start = self.state.pc as usize;
        let high = self.state.loaded_origin as usize + self.state.loaded_len;
        if start + width > high {
            return Err(System370Error::TruncatedInstruction {
                pc: self.state.pc,
                needed: width,
            });
        }
        Ok(self.state.memory[start..start + width].to_vec())
    }

    fn range(address: u32, bytes: usize) -> Result<std::ops::Range<usize>, System370Error> {
        let end = (address as usize)
            .checked_add(bytes)
            .ok_or(System370Error::MemoryOutOfRange { address, bytes })?;
        if address > ADDRESS_MASK || end > MEMORY_BYTES {
            return Err(System370Error::MemoryOutOfRange { address, bytes });
        }
        Ok(address as usize..end)
    }
}

fn validate_state(state: &System370State) -> Result<(), System370Error> {
    if state.memory.len() != MEMORY_BYTES {
        return Err(System370Error::InvalidState {
            reason: "memory has the wrong installed size",
        });
    }
    if state.condition_code > 3 {
        return Err(System370Error::InvalidState {
            reason: "condition code is outside 0..3",
        });
    }
    if state.pc > ADDRESS_MASK || !state.pc.is_multiple_of(2) {
        return Err(System370Error::InvalidState {
            reason: "instruction address is invalid",
        });
    }
    let end = (state.loaded_origin as usize).checked_add(state.loaded_len);
    if state.loaded_origin > ADDRESS_MASK
        || state.loaded_len != 0 && !state.loaded_origin.is_multiple_of(2)
        || end.is_none_or(|end| end > MEMORY_BYTES)
    {
        return Err(System370Error::InvalidState {
            reason: "installed program range is invalid",
        });
    }
    Ok(())
}

fn ordering_code(ordering: std::cmp::Ordering) -> u8 {
    match ordering {
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Less => 1,
        std::cmp::Ordering::Greater => 2,
    }
}

fn to_system360_state(state: &System370State) -> system360::System360State {
    system360::System360State {
        gprs: state.gprs,
        fprs: state.fprs,
        pc: state.pc,
        condition_code: state.condition_code,
        memory: state.memory.clone(),
        halted: state.halted,
        loaded_origin: state.loaded_origin,
        loaded_len: state.loaded_len,
    }
}

fn from_system360_state(state: system360::System360State) -> System370State {
    System370State {
        gprs: state.gprs,
        fprs: state.fprs,
        pc: state.pc,
        condition_code: state.condition_code,
        memory: state.memory,
        halted: state.halted,
        loaded_origin: state.loaded_origin,
        loaded_len: state.loaded_len,
    }
}

fn map_system360_error(error: system360::System360Error) -> System370Error {
    match error {
        system360::System360Error::EmptyProgram => System370Error::EmptyProgram,
        system360::System360Error::MisalignedProgram { origin } => {
            System370Error::MisalignedProgram { origin }
        }
        system360::System360Error::ProgramOutOfRange { origin, bytes } => {
            System370Error::ProgramOutOfRange { origin, bytes }
        }
        system360::System360Error::FetchOutsideProgram { pc } => {
            System370Error::FetchOutsideProgram { pc }
        }
        system360::System360Error::TruncatedInstruction { pc, needed } => {
            System370Error::TruncatedInstruction { pc, needed }
        }
        system360::System360Error::UnknownOpcode { opcode, pc } => {
            System370Error::UnknownOpcode { opcode, pc }
        }
        system360::System360Error::UnsupportedSupervisorCall { number } => {
            System370Error::UnsupportedSupervisorCall { number }
        }
        system360::System360Error::MisalignedAccess { address, alignment } => {
            System370Error::MisalignedAccess { address, alignment }
        }
        system360::System360Error::MemoryOutOfRange { address, bytes } => {
            System370Error::MemoryOutOfRange { address, bytes }
        }
        system360::System360Error::InvalidRegister { bank, index } => {
            System370Error::InvalidRegister { bank, index }
        }
        system360::System360Error::InvalidState { reason } => {
            System370Error::InvalidState { reason }
        }
        system360::System360Error::Specification { reason } => {
            System370Error::Specification { reason }
        }
        system360::System360Error::FixedPointOverflow => System370Error::FixedPointOverflow,
        system360::System360Error::FixedPointDivide => System370Error::FixedPointDivide,
        system360::System360Error::FloatingExponent => System370Error::FloatingExponent,
        system360::System360Error::FloatingDivide => System370Error::FloatingDivide,
        system360::System360Error::Halted => System370Error::Halted,
        system360::System360Error::MaxStepsExceeded { max_steps } => {
            System370Error::MaxStepsExceeded { max_steps }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_reset_and_error_text_are_stable() {
        let mut cpu = System370Simulator::default();
        cpu.write_gpr(1, 7).unwrap();
        cpu.reset();
        assert_eq!(cpu.get_state(), System370Simulator::new().get_state());
        let errors = [
            System370Error::EmptyProgram,
            System370Error::MisalignedProgram { origin: 1 },
            System370Error::ProgramOutOfRange {
                origin: 0,
                bytes: usize::MAX,
            },
            System370Error::FetchOutsideProgram { pc: 0 },
            System370Error::TruncatedInstruction { pc: 0, needed: 4 },
            System370Error::UnknownOpcode { opcode: 0, pc: 0 },
            System370Error::UnsupportedSupervisorCall { number: 1 },
            System370Error::MisalignedAccess {
                address: 1,
                alignment: 4,
            },
            System370Error::MemoryOutOfRange {
                address: 0,
                bytes: usize::MAX,
            },
            System370Error::InvalidRegister {
                bank: "general",
                index: 16,
            },
            System370Error::InvalidState { reason: "test" },
            System370Error::Specification { reason: "test" },
            System370Error::FixedPointOverflow,
            System370Error::FixedPointDivide,
            System370Error::FloatingExponent,
            System370Error::FloatingDivide,
            System370Error::Halted,
            System370Error::MaxStepsExceeded { max_steps: 1 },
        ];
        assert!(errors.iter().all(|error| !error.to_string().is_empty()));
    }

    #[test]
    fn inherited_errors_map_without_leaking_predecessor_types() {
        let errors = [
            system360::System360Error::EmptyProgram,
            system360::System360Error::MisalignedProgram { origin: 1 },
            system360::System360Error::ProgramOutOfRange {
                origin: 0,
                bytes: usize::MAX,
            },
            system360::System360Error::FetchOutsideProgram { pc: 2 },
            system360::System360Error::TruncatedInstruction { pc: 0, needed: 6 },
            system360::System360Error::UnknownOpcode { opcode: 0, pc: 0 },
            system360::System360Error::UnsupportedSupervisorCall { number: 1 },
            system360::System360Error::MisalignedAccess {
                address: 1,
                alignment: 4,
            },
            system360::System360Error::MemoryOutOfRange {
                address: MEMORY_BYTES as u32,
                bytes: 1,
            },
            system360::System360Error::InvalidRegister {
                bank: "general",
                index: 16,
            },
            system360::System360Error::InvalidState { reason: "test" },
            system360::System360Error::Specification { reason: "test" },
            system360::System360Error::FixedPointOverflow,
            system360::System360Error::FixedPointDivide,
            system360::System360Error::FloatingExponent,
            system360::System360Error::FloatingDivide,
            system360::System360Error::Halted,
            system360::System360Error::MaxStepsExceeded { max_steps: 1 },
        ];
        let mapped: Vec<_> = errors.into_iter().map(map_system360_error).collect();
        assert!(matches!(mapped[0], System370Error::EmptyProgram));
        assert!(matches!(
            mapped.last(),
            Some(System370Error::MaxStepsExceeded { max_steps: 1 })
        ));
        assert!(mapped.iter().all(|error| !error.to_string().is_empty()));
    }
}
