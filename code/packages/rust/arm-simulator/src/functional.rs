//! Exact, checked ARMv7 educational functional machine for Spec 07b.
//!
//! The crate's original [`crate::ARMSimulator`] remains available for legacy
//! consumers. This module supplies the completion contract: fixed 64 KiB
//! state, typed failures, atomic transitions, condition flags, and the full
//! instruction surface promised by Spec 07b.

/// Architectural memory size required by Spec 07b.
pub const MEMORY_SIZE: usize = 65_536;
/// Custom halt word shared with the legacy simulator.
pub const HALT_WORD: u32 = 0xffff_ffff;

/// ARM condition field values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Condition {
    Eq = 0x0,
    Ne = 0x1,
    Cs = 0x2,
    Cc = 0x3,
    Mi = 0x4,
    Pl = 0x5,
    Vs = 0x6,
    Vc = 0x7,
    Hi = 0x8,
    Ls = 0x9,
    Ge = 0xa,
    Lt = 0xb,
    Gt = 0xc,
    Le = 0xd,
    Al = 0xe,
    Nv = 0xf,
}

impl Condition {
    fn from_raw(raw: u32) -> Self {
        match raw & 0xf {
            0x0 => Self::Eq,
            0x1 => Self::Ne,
            0x2 => Self::Cs,
            0x3 => Self::Cc,
            0x4 => Self::Mi,
            0x5 => Self::Pl,
            0x6 => Self::Vs,
            0x7 => Self::Vc,
            0x8 => Self::Hi,
            0x9 => Self::Ls,
            0xa => Self::Ge,
            0xb => Self::Lt,
            0xc => Self::Gt,
            0xd => Self::Le,
            0xe => Self::Al,
            _ => Self::Nv,
        }
    }
}

/// Data-processing operations promised by Spec 07b.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DataOpcode {
    And = 0x0,
    Sub = 0x2,
    Add = 0x4,
    Cmp = 0xa,
    Orr = 0xc,
    Mov = 0xd,
}

/// The four condition flags exposed by the educational CPSR model.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    pub n: bool,
    pub z: bool,
    pub c: bool,
    pub v: bool,
}

impl Flags {
    /// Evaluate one of ARM's sixteen condition predicates.
    #[must_use]
    pub fn satisfies(self, condition: Condition) -> bool {
        match condition {
            Condition::Eq => self.z,
            Condition::Ne => !self.z,
            Condition::Cs => self.c,
            Condition::Cc => !self.c,
            Condition::Mi => self.n,
            Condition::Pl => !self.n,
            Condition::Vs => self.v,
            Condition::Vc => !self.v,
            Condition::Hi => self.c && !self.z,
            Condition::Ls => !self.c || self.z,
            Condition::Ge => self.n == self.v,
            Condition::Lt => self.n != self.v,
            Condition::Gt => !self.z && self.n == self.v,
            Condition::Le => self.z || self.n != self.v,
            Condition::Al => true,
            Condition::Nv => false,
        }
    }
}

/// Complete restorable state. Register 15 is always equal to `pc`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmState {
    pub registers: [u32; 16],
    pub pc: u32,
    pub flags: Flags,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u32,
    pub loaded_len: usize,
}

/// Typed failures for checked state, loading, execution, and direct access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArmError {
    Halted,
    InvalidRegister { index: usize },
    InvalidState(String),
    MisalignedProgram { origin: u32 },
    ProgramOutOfRange { origin: u32, length: usize },
    TruncatedInstruction { pc: u32 },
    MisalignedAccess { address: u32, width: usize },
    MemoryOutOfRange { address: u32, width: usize },
    UnsupportedAddressingMode { raw: u32, pc: u32 },
    UnsupportedOperand2 { raw: u32, pc: u32 },
    UnknownInstruction { raw: u32, pc: u32 },
    StepLimitExceeded { max_steps: usize },
}

impl std::fmt::Display for ArmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Halted => f.write_str("CPU is halted"),
            Self::InvalidRegister { index } => write!(f, "register {index} is outside r0-r15"),
            Self::InvalidState(message) => f.write_str(message),
            Self::MisalignedProgram { origin } => {
                write!(f, "program origin {origin:#010x} is not word-aligned")
            }
            Self::ProgramOutOfRange { origin, length } => write!(
                f,
                "program of {length} bytes at {origin:#010x} exceeds 64 KiB memory"
            ),
            Self::TruncatedInstruction { pc } => {
                write!(
                    f,
                    "instruction at {pc:#010x} crosses installed program bounds"
                )
            }
            Self::MisalignedAccess { address, width } => {
                write!(f, "misaligned {width}-byte access at {address:#010x}")
            }
            Self::MemoryOutOfRange { address, width } => {
                write!(f, "{width}-byte access at {address:#010x} exceeds memory")
            }
            Self::UnsupportedAddressingMode { raw, pc } => write!(
                f,
                "unsupported addressing mode in {raw:#010x} at {pc:#010x}"
            ),
            Self::UnsupportedOperand2 { raw, pc } => {
                write!(f, "unsupported operand2 in {raw:#010x} at {pc:#010x}")
            }
            Self::UnknownInstruction { raw, pc } => {
                write!(f, "unknown instruction {raw:#010x} at {pc:#010x}")
            }
            Self::StepLimitExceeded { max_steps } => {
                write!(f, "step limit {max_steps} exceeded")
            }
        }
    }
}

impl std::error::Error for ArmError {}

/// One committed architectural transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: u32,
    pub mnemonic: String,
    pub condition_passed: bool,
    pub state_before: ArmState,
    pub state_after: ArmState,
}

/// Successful bounded execution result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: ArmState,
}

/// Exact ARMv7 educational functional simulator.
#[derive(Debug, Clone)]
pub struct Armv7Simulator {
    state: ArmState,
}

impl Default for Armv7Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Armv7Simulator {
    /// Construct the reset state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: ArmState {
                registers: [0; 16],
                pc: 0,
                flags: Flags::default(),
                memory: vec![0; MEMORY_SIZE],
                halted: false,
                loaded_origin: 0,
                loaded_len: 0,
            },
        }
    }

    /// Restore the power-on state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Clone the complete architectural and installation state.
    #[must_use]
    pub fn get_state(&self) -> ArmState {
        self.state.clone()
    }

    /// Atomically restore a previously captured state.
    pub fn restore(&mut self, state: &ArmState) -> Result<(), ArmError> {
        validate_state(state)?;
        self.state = state.clone();
        Ok(())
    }

    /// Reset and install a program at address zero.
    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), ArmError> {
        self.load_at_checked(program, 0)
    }

    /// Reset and install a program at an aligned origin.
    pub fn load_at_checked(&mut self, program: &[u8], origin: u32) -> Result<(), ArmError> {
        if origin & 3 != 0 {
            return Err(ArmError::MisalignedProgram { origin });
        }
        let start = origin as usize;
        if start >= MEMORY_SIZE {
            return Err(ArmError::ProgramOutOfRange {
                origin,
                length: program.len(),
            });
        }
        let end = start
            .checked_add(program.len())
            .filter(|end| *end <= MEMORY_SIZE)
            .ok_or(ArmError::ProgramOutOfRange {
                origin,
                length: program.len(),
            })?;
        self.reset();
        self.state.memory[start..end].copy_from_slice(program);
        self.state.loaded_origin = origin;
        self.state.loaded_len = program.len();
        self.set_pc(origin);
        Ok(())
    }

    /// Compatibility spelling for checked load.
    pub fn load(&mut self, program: &[u8]) -> Result<(), ArmError> {
        self.load_checked(program)
    }

    pub fn read_register_checked(&self, index: usize) -> Result<u32, ArmError> {
        self.state
            .registers
            .get(index)
            .copied()
            .ok_or(ArmError::InvalidRegister { index })
    }

    pub fn write_register_checked(&mut self, index: usize, value: u32) -> Result<(), ArmError> {
        if index >= 16 {
            return Err(ArmError::InvalidRegister { index });
        }
        if index == 15 {
            if value & 3 != 0 || value as usize >= MEMORY_SIZE {
                return Err(ArmError::InvalidState(
                    "R15/PC must be aligned inside memory".into(),
                ));
            }
            self.set_pc(value);
        } else {
            self.state.registers[index] = value;
        }
        Ok(())
    }

    pub fn read_byte_checked(&self, address: u32) -> Result<u8, ArmError> {
        Ok(self.state.memory[direct_range(address, 1)?])
    }

    pub fn write_byte_checked(&mut self, address: u32, value: u8) -> Result<(), ArmError> {
        let index = direct_range(address, 1)?;
        self.state.memory[index] = value;
        Ok(())
    }

    pub fn read_word_checked(&self, address: u32) -> Result<u32, ArmError> {
        require_alignment(address, 4)?;
        let start = direct_range(address, 4)?;
        Ok(u32::from_le_bytes(
            self.state.memory[start..start + 4]
                .try_into()
                .expect("validated word range"),
        ))
    }

    pub fn write_word_checked(&mut self, address: u32, value: u32) -> Result<(), ArmError> {
        require_alignment(address, 4)?;
        let start = direct_range(address, 4)?;
        self.state.memory[start..start + 4].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    /// Execute one transition, rolling all state back on any failure.
    pub fn step_checked(&mut self) -> Result<StepTrace, ArmError> {
        if self.state.halted {
            return Err(ArmError::Halted);
        }
        self.validate_fetch()?;
        let before = self.get_state();
        let pc = before.pc;
        let raw = self.fetch_word();
        match self.execute_raw(raw, pc) {
            Ok((mnemonic, condition_passed)) => Ok(StepTrace {
                pc_before: pc,
                pc_after: self.state.pc,
                raw,
                mnemonic: mnemonic.to_string(),
                condition_passed,
                state_before: before,
                state_after: self.get_state(),
            }),
            Err(error) => {
                self.state = before;
                Err(error)
            }
        }
    }

    pub fn step(&mut self) -> Result<StepTrace, ArmError> {
        self.step_checked()
    }

    /// Run the installed image transactionally until HLT.
    pub fn run_loaded_checked(&mut self, max_steps: usize) -> Result<ExecutionResult, ArmError> {
        let before = self.get_state();
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            match self.step_checked() {
                Ok(trace) => {
                    let halted = trace.state_after.halted;
                    traces.push(trace);
                    if halted {
                        return Ok(ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            traces,
                            final_state: self.get_state(),
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
        Err(ArmError::StepLimitExceeded { max_steps })
    }

    /// Reset, load, and run atomically.
    pub fn run_checked(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, ArmError> {
        let before = self.get_state();
        self.load_checked(program)?;
        match self.run_loaded_checked(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = before;
                Err(error)
            }
        }
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, ArmError> {
        self.run_checked(program, max_steps)
    }

    fn set_pc(&mut self, pc: u32) {
        self.state.pc = pc;
        self.state.registers[15] = pc;
    }

    fn validate_fetch(&self) -> Result<(), ArmError> {
        let pc = self.state.pc;
        require_alignment(pc, 4)?;
        let start = self.state.loaded_origin;
        let end = start.saturating_add(self.state.loaded_len as u32);
        if pc < start || pc.checked_add(4).is_none_or(|next| next > end) {
            return Err(ArmError::TruncatedInstruction { pc });
        }
        Ok(())
    }

    fn fetch_word(&self) -> u32 {
        let start = self.state.pc as usize;
        u32::from_le_bytes(
            self.state.memory[start..start + 4]
                .try_into()
                .expect("validated instruction fetch"),
        )
    }

    fn execute_raw(&mut self, raw: u32, pc: u32) -> Result<(&'static str, bool), ArmError> {
        if raw == HALT_WORD {
            self.state.halted = true;
            return Ok(("HLT", true));
        }
        let condition = Condition::from_raw(raw >> 28);
        if !self.state.flags.satisfies(condition) {
            self.set_pc(sequential_pc(pc)?);
            return Ok(("SKIP", false));
        }
        match (raw >> 25) & 7 {
            0 | 1 => self.execute_data_processing(raw, pc),
            2 | 3 => self.execute_transfer(raw, pc),
            5 => self.execute_branch(raw, pc),
            _ => Err(ArmError::UnknownInstruction { raw, pc }),
        }
        .map(|mnemonic| (mnemonic, true))
    }

    fn execute_data_processing(&mut self, raw: u32, pc: u32) -> Result<&'static str, ArmError> {
        if (raw >> 26) & 3 != 0 {
            return Err(ArmError::UnknownInstruction { raw, pc });
        }
        let immediate = raw & (1 << 25) != 0;
        let opcode = (raw >> 21) & 0xf;
        let set_flags = raw & (1 << 20) != 0 || opcode == 0xa;
        let rn = ((raw >> 16) & 0xf) as usize;
        let rd = ((raw >> 12) & 0xf) as usize;
        let a = self.state.registers[rn];
        let (operand, shifter_carry) = self.decode_operand2(raw, pc, immediate)?;
        let (mnemonic, result, carry, overflow, write_result) = match opcode {
            0x0 => ("AND", a & operand, shifter_carry, self.state.flags.v, true),
            0x2 => {
                let result = a.wrapping_sub(operand);
                (
                    "SUB",
                    result,
                    a >= operand,
                    sub_overflow(a, operand, result),
                    true,
                )
            }
            0x4 => {
                let (result, carry) = a.overflowing_add(operand);
                ("ADD", result, carry, add_overflow(a, operand, result), true)
            }
            0xa => {
                let result = a.wrapping_sub(operand);
                (
                    "CMP",
                    result,
                    a >= operand,
                    sub_overflow(a, operand, result),
                    false,
                )
            }
            0xc => ("ORR", a | operand, shifter_carry, self.state.flags.v, true),
            0xd => ("MOV", operand, shifter_carry, self.state.flags.v, true),
            _ => return Err(ArmError::UnknownInstruction { raw, pc }),
        };
        let mut next = sequential_pc(pc)?;
        if write_result {
            if rd == 15 {
                if result & 3 != 0 {
                    return Err(ArmError::MisalignedAccess {
                        address: result,
                        width: 4,
                    });
                }
                direct_range(result, 4)?;
                next = result;
            } else {
                self.state.registers[rd] = result;
            }
        }
        if set_flags {
            self.state.flags = Flags {
                n: result & 0x8000_0000 != 0,
                z: result == 0,
                c: carry,
                v: overflow,
            };
        }
        self.set_pc(next);
        Ok(mnemonic)
    }

    fn decode_operand2(&self, raw: u32, pc: u32, immediate: bool) -> Result<(u32, bool), ArmError> {
        let operand2 = raw & 0xfff;
        if immediate {
            let rotation = ((operand2 >> 8) & 0xf) * 2;
            let value = (operand2 & 0xff).rotate_right(rotation);
            let carry = if rotation == 0 {
                self.state.flags.c
            } else {
                value & 0x8000_0000 != 0
            };
            Ok((value, carry))
        } else {
            if operand2 & 0xff0 != 0 {
                return Err(ArmError::UnsupportedOperand2 { raw, pc });
            }
            Ok((
                self.state.registers[(operand2 & 0xf) as usize],
                self.state.flags.c,
            ))
        }
    }

    fn execute_transfer(&mut self, raw: u32, pc: u32) -> Result<&'static str, ArmError> {
        let immediate_offset = raw & (1 << 25) == 0;
        let preindexed = raw & (1 << 24) != 0;
        let byte = raw & (1 << 22) != 0;
        let writeback = raw & (1 << 21) != 0;
        if !immediate_offset || !preindexed || byte || writeback {
            return Err(ArmError::UnsupportedAddressingMode { raw, pc });
        }
        let rn = ((raw >> 16) & 0xf) as usize;
        let rd = ((raw >> 12) & 0xf) as usize;
        let offset = raw & 0xfff;
        let base = self.state.registers[rn];
        let address = if raw & (1 << 23) != 0 {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        if raw & (1 << 20) != 0 {
            let value = self.read_word_checked(address)?;
            if rd == 15 {
                if value & 3 != 0 {
                    return Err(ArmError::MisalignedAccess {
                        address: value,
                        width: 4,
                    });
                }
                direct_range(value, 4)?;
                self.set_pc(value);
            } else {
                self.state.registers[rd] = value;
                self.set_pc(sequential_pc(pc)?);
            }
            Ok("LDR")
        } else {
            self.write_word_checked(address, self.state.registers[rd])?;
            self.set_pc(sequential_pc(pc)?);
            Ok("STR")
        }
    }

    fn execute_branch(&mut self, raw: u32, pc: u32) -> Result<&'static str, ArmError> {
        let offset = (((raw & 0x00ff_ffff) << 8) as i32 >> 6) as u32;
        let target = pc.wrapping_add(8).wrapping_add(offset);
        if target & 3 != 0 || target as usize >= MEMORY_SIZE {
            return Err(ArmError::MemoryOutOfRange {
                address: target,
                width: 4,
            });
        }
        if raw & (1 << 24) != 0 {
            self.state.registers[14] = pc.wrapping_add(4);
        }
        self.set_pc(target);
        Ok(if raw & (1 << 24) != 0 { "BL" } else { "B" })
    }
}

fn add_overflow(a: u32, b: u32, result: u32) -> bool {
    (!(a ^ b) & (a ^ result) & 0x8000_0000) != 0
}

fn sub_overflow(a: u32, b: u32, result: u32) -> bool {
    ((a ^ b) & (a ^ result) & 0x8000_0000) != 0
}

fn validate_state(state: &ArmState) -> Result<(), ArmError> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(ArmError::InvalidState(format!(
            "memory must contain exactly {MEMORY_SIZE} bytes"
        )));
    }
    if state.pc & 3 != 0 || state.pc as usize >= MEMORY_SIZE || state.registers[15] != state.pc {
        return Err(ArmError::InvalidState(
            "R15 and PC must match at an aligned address inside memory".into(),
        ));
    }
    let start = state.loaded_origin as usize;
    if state.loaded_origin & 3 != 0
        || start >= MEMORY_SIZE
        || start
            .checked_add(state.loaded_len)
            .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(ArmError::InvalidState(
            "installed program range is invalid".into(),
        ));
    }
    Ok(())
}

fn require_alignment(address: u32, width: usize) -> Result<(), ArmError> {
    if !address.is_multiple_of(width as u32) {
        return Err(ArmError::MisalignedAccess { address, width });
    }
    Ok(())
}

fn sequential_pc(pc: u32) -> Result<u32, ArmError> {
    let next = pc.checked_add(4).ok_or(ArmError::MemoryOutOfRange {
        address: pc,
        width: 4,
    })?;
    direct_range(next, 4)?;
    Ok(next)
}

fn direct_range(address: u32, width: usize) -> Result<usize, ArmError> {
    let start = address as usize;
    start
        .checked_add(width)
        .filter(|end| *end <= MEMORY_SIZE)
        .map(|_| start)
        .ok_or(ArmError::MemoryOutOfRange { address, width })
}

/// Encode one data-processing immediate instruction.
#[must_use]
pub fn encode_data_immediate(
    condition: Condition,
    opcode: DataOpcode,
    set_flags: bool,
    rn: usize,
    rd: usize,
    rotate: u32,
    imm8: u32,
) -> u32 {
    ((condition as u32) << 28)
        | (1 << 25)
        | ((opcode as u32) << 21)
        | (u32::from(set_flags) << 20)
        | (((rn as u32) & 0xf) << 16)
        | (((rd as u32) & 0xf) << 12)
        | ((rotate & 0xf) << 8)
        | (imm8 & 0xff)
}

/// Encode one unshifted-register data-processing instruction.
#[must_use]
pub fn encode_data_register(
    condition: Condition,
    opcode: DataOpcode,
    set_flags: bool,
    rn: usize,
    rd: usize,
    rm: usize,
) -> u32 {
    ((condition as u32) << 28)
        | ((opcode as u32) << 21)
        | (u32::from(set_flags) << 20)
        | (((rn as u32) & 0xf) << 16)
        | (((rd as u32) & 0xf) << 12)
        | ((rm as u32) & 0xf)
}

/// Encode pre-indexed word LDR/STR with an immediate offset.
#[must_use]
pub fn encode_transfer(
    condition: Condition,
    load: bool,
    add_offset: bool,
    rn: usize,
    rd: usize,
    offset: u32,
) -> u32 {
    ((condition as u32) << 28)
        | (1 << 26)
        | (1 << 24)
        | (u32::from(add_offset) << 23)
        | (u32::from(load) << 20)
        | (((rn as u32) & 0xf) << 16)
        | (((rd as u32) & 0xf) << 12)
        | (offset & 0xfff)
}

/// Encode B/BL. `byte_offset` is relative to architectural PC+8.
#[must_use]
pub fn encode_branch(condition: Condition, link: bool, byte_offset: i32) -> u32 {
    ((condition as u32) << 28)
        | (0b101 << 25)
        | (u32::from(link) << 24)
        | (((byte_offset >> 2) as u32) & 0x00ff_ffff)
}

/// Encode the custom Spec 07b halt sentinel.
#[must_use]
pub const fn encode_halt() -> u32 {
    HALT_WORD
}

/// Assemble words using ARM's little-endian memory order.
#[must_use]
pub fn assemble_words(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_spec_surface_executes() {
        let words = [
            encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 0, 0, 7),
            encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 1, 0, 5),
            encode_data_register(Condition::Al, DataOpcode::Add, true, 0, 2, 1),
            encode_data_register(Condition::Al, DataOpcode::Cmp, true, 2, 0, 1),
            encode_data_register(Condition::Gt, DataOpcode::Sub, false, 2, 3, 1),
            encode_data_register(Condition::Al, DataOpcode::And, false, 2, 4, 0),
            encode_data_register(Condition::Al, DataOpcode::Orr, false, 4, 5, 1),
            encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 6, 0, 0x80),
            encode_transfer(Condition::Al, false, true, 6, 5, 0),
            encode_transfer(Condition::Al, true, true, 6, 7, 0),
            encode_halt(),
        ];
        let mut cpu = Armv7Simulator::new();
        let result = cpu.run_checked(&assemble_words(&words), 16).unwrap();
        assert!(result.halted);
        assert_eq!(result.final_state.registers[2], 12);
        assert_eq!(result.final_state.registers[3], 7);
        assert_eq!(result.final_state.registers[5], 5);
        assert_eq!(result.final_state.registers[7], 5);
        assert_eq!(&result.final_state.memory[0x80..0x84], &[5, 0, 0, 0]);
    }

    #[test]
    fn all_condition_predicates_are_defined() {
        let flags = Flags {
            n: true,
            z: false,
            c: true,
            v: true,
        };
        let expected = [
            false, true, true, false, true, false, true, false, true, false, true, false, true,
            false, true, false,
        ];
        for (raw, expected) in expected.into_iter().enumerate() {
            assert_eq!(flags.satisfies(Condition::from_raw(raw as u32)), expected);
        }
    }

    #[test]
    fn branch_and_faults_are_atomic() {
        let program = assemble_words(&[
            encode_branch(Condition::Al, false, 0),
            0x0123_4567,
            HALT_WORD,
        ]);
        let mut cpu = Armv7Simulator::new();
        let result = cpu.run_checked(&program, 3).unwrap();
        assert_eq!(result.steps, 2);
        assert_eq!(result.traces[0].pc_after, 8);

        cpu.load_checked(&assemble_words(&[0xee00_0000])).unwrap();
        let before = cpu.get_state();
        assert!(matches!(
            cpu.step_checked(),
            Err(ArmError::UnknownInstruction { .. })
        ));
        assert_eq!(cpu.get_state(), before);
    }
}
