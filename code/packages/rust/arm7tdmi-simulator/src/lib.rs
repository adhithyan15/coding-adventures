//! Checked ARMv4T functional model of an ARM7TDMI core.
//!
//! ARM7TDMI has two unusually important programmer-visible ideas.  First, its
//! sixteen logical registers are views over thirty-one physical registers:
//! changing mode selects a bank, it does not copy values.  Second, reading R15
//! observes the pipeline (instruction address plus eight in ARM state and plus
//! four in Thumb state).  Both rules are represented directly below rather
//! than hidden behind a flat register array.

use core::fmt;
use core::ops::Range;

/// Size of the flat teaching memory owned by this CPU-only model.
pub const MEMORY_SIZE: usize = 65_536;
/// Maximum complete-state transitions retained by one bounded run.
pub const MAX_RETAINED_STEPS: usize = 64;

pub const CPSR_N: u32 = 1 << 31;
pub const CPSR_Z: u32 = 1 << 30;
pub const CPSR_C: u32 = 1 << 29;
pub const CPSR_V: u32 = 1 << 28;
pub const CPSR_I: u32 = 1 << 7;
pub const CPSR_F: u32 = 1 << 6;
pub const CPSR_T: u32 = 1 << 5;
const CPSR_MODE_MASK: u32 = 0x1f;
const CPSR_ADMITTED: u32 = 0xf000_00ff;

/// The seven ARMv4T processor modes admitted by the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    User = 0x10,
    Fiq = 0x11,
    Irq = 0x12,
    Supervisor = 0x13,
    Abort = 0x17,
    Undefined = 0x1b,
    System = 0x1f,
}

impl Mode {
    fn from_bits(bits: u32) -> Option<Self> {
        match bits & CPSR_MODE_MASK {
            0x10 => Some(Self::User),
            0x11 => Some(Self::Fiq),
            0x12 => Some(Self::Irq),
            0x13 => Some(Self::Supervisor),
            0x17 => Some(Self::Abort),
            0x1b => Some(Self::Undefined),
            0x1f => Some(Self::System),
            _ => None,
        }
    }
}

/// Which instruction encoding is selected by CPSR.T.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstructionSet {
    Arm,
    Thumb,
}

/// All thirty-one physical GPRs owned by ARM7TDMI.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PhysicalRegisters {
    /// R0-R7, shared by every mode.
    pub common_r0_r7: [u32; 8],
    /// R8-R12 outside FIQ mode.
    pub normal_r8_r12: [u32; 5],
    /// FIQ R8-R14.
    pub fiq_r8_r14: [u32; 7],
    /// User/System R13-R14.
    pub user_r13_r14: [u32; 2],
    pub irq_r13_r14: [u32; 2],
    pub supervisor_r13_r14: [u32; 2],
    pub abort_r13_r14: [u32; 2],
    pub undefined_r13_r14: [u32; 2],
    /// Authoritative next instruction address (physical R15).
    pub pc: u32,
}

/// Complete restorable CPU, memory, and installation state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arm7TdmiState {
    pub registers: PhysicalRegisters,
    pub cpsr: u32,
    pub spsr_fiq: u32,
    pub spsr_irq: u32,
    pub spsr_supervisor: u32,
    pub spsr_abort: u32,
    pub spsr_undefined: u32,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u32,
    pub loaded_len: usize,
}

impl Default for Arm7TdmiState {
    fn default() -> Self {
        Self {
            registers: PhysicalRegisters::default(),
            cpsr: Mode::Supervisor as u32 | CPSR_I | CPSR_F,
            spsr_fiq: 0,
            spsr_irq: 0,
            spsr_supervisor: 0,
            spsr_abort: 0,
            spsr_undefined: 0,
            memory: vec![0; MEMORY_SIZE],
            halted: false,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }
}

/// One exact committed instruction transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub instruction_set: InstructionSet,
    pub width: usize,
    pub fetched: [u8; 4],
    pub raw: u32,
    pub mnemonic: &'static str,
    pub condition_met: bool,
    pub state_before: Arm7TdmiState,
    pub state_after: Arm7TdmiState,
}

/// A successful bounded execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: Arm7TdmiState,
}

/// Typed transport, lifecycle, and instruction failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arm7TdmiError {
    EmptyProgram,
    MisalignedProgram {
        origin: u32,
        alignment: usize,
    },
    ProgramOutOfRange {
        origin: u32,
        length: usize,
    },
    InvalidState {
        reason: &'static str,
    },
    InvalidRegister {
        register: usize,
    },
    ReservedMode {
        mode: u8,
    },
    UnavailableSavedStatus {
        mode: Mode,
    },
    PrivilegedInstruction {
        raw: u32,
        pc: u32,
    },
    UnpredictableInstruction {
        raw: u32,
        pc: u32,
    },
    UnknownInstruction {
        raw: u32,
        width: usize,
        pc: u32,
    },
    UnsupportedInstruction {
        raw: u32,
        width: usize,
        pc: u32,
    },
    UnsupportedException {
        immediate: u32,
        pc: u32,
    },
    MisalignedFetch {
        pc: u32,
        alignment: usize,
    },
    FetchOutsideProgram {
        pc: u32,
        width: usize,
    },
    MisalignedData {
        address: u32,
        width: usize,
    },
    DataOutOfRange {
        address: u32,
        width: usize,
    },
    MisalignedTarget {
        target: u32,
        instruction_set: InstructionSet,
    },
    TargetOutOfRange {
        target: u32,
        instruction_set: InstructionSet,
    },
    Halted,
    RetentionLimitExceeded {
        requested: usize,
        limit: usize,
    },
    AllocationFailed,
    StepLimitExceeded {
        limit: usize,
    },
}

impl fmt::Display for Arm7TdmiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => f.write_str("program image is empty"),
            Self::MisalignedProgram { origin, alignment } => write!(
                f,
                "program at {origin:#010x} is not aligned to {alignment} bytes"
            ),
            Self::ProgramOutOfRange { origin, length } => write!(
                f,
                "program of {length} bytes at {origin:#010x} exceeds 64 KiB memory"
            ),
            Self::InvalidState { reason } => write!(f, "invalid ARM7TDMI state: {reason}"),
            Self::InvalidRegister { register } => {
                write!(f, "logical register R{register} does not exist")
            }
            Self::ReservedMode { mode } => write!(f, "processor mode {mode:#04x} is reserved"),
            Self::UnavailableSavedStatus { mode } => write!(f, "mode {mode:?} has no SPSR"),
            Self::PrivilegedInstruction { raw, pc } => {
                write!(f, "privileged instruction {raw:#010x} at {pc:#010x}")
            }
            Self::UnpredictableInstruction { raw, pc } => write!(
                f,
                "architecturally unpredictable instruction {raw:#010x} at {pc:#010x}"
            ),
            Self::UnknownInstruction { raw, width, pc } => write!(
                f,
                "unknown {width}-byte instruction {raw:#010x} at {pc:#010x}"
            ),
            Self::UnsupportedInstruction { raw, width, pc } => write!(
                f,
                "unsupported {width}-byte instruction {raw:#010x} at {pc:#010x}"
            ),
            Self::UnsupportedException { immediate, pc } => {
                write!(f, "unsupported SWI {immediate:#x} at {pc:#010x}")
            }
            Self::MisalignedFetch { pc, alignment } => {
                write!(f, "fetch at {pc:#010x} is not aligned to {alignment} bytes")
            }
            Self::FetchOutsideProgram { pc, width } => write!(
                f,
                "{width}-byte fetch at {pc:#010x} is outside the installed image"
            ),
            Self::MisalignedData { address, width } => write!(
                f,
                "{width}-byte data access at {address:#010x} is misaligned"
            ),
            Self::DataOutOfRange { address, width } => write!(
                f,
                "{width}-byte data access at {address:#010x} is out of range"
            ),
            Self::MisalignedTarget {
                target,
                instruction_set,
            } => write!(f, "{instruction_set:?} target {target:#010x} is misaligned"),
            Self::TargetOutOfRange {
                target,
                instruction_set,
            } => write!(
                f,
                "{instruction_set:?} target {target:#010x} is outside 64 KiB memory"
            ),
            Self::Halted => f.write_str("processor is halted"),
            Self::RetentionLimitExceeded { requested, limit } => write!(
                f,
                "requested {requested} traces exceeds retention limit {limit}"
            ),
            Self::AllocationFailed => f.write_str("trace allocation failed"),
            Self::StepLimitExceeded { limit } => {
                write!(f, "step limit {limit} exhausted before halt")
            }
        }
    }
}

impl std::error::Error for Arm7TdmiError {}

/// Checked CPU-only ARM7TDMI functional simulator.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Arm7TdmiSimulator {
    state: Arm7TdmiState,
}

impl Arm7TdmiSimulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.state = Arm7TdmiState::default();
    }

    #[must_use]
    pub fn get_state(&self) -> Arm7TdmiState {
        self.state.clone()
    }

    #[must_use]
    pub fn state(&self) -> &Arm7TdmiState {
        &self.state
    }

    pub fn restore(&mut self, state: Arm7TdmiState) -> Result<(), Arm7TdmiError> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    pub fn load(
        &mut self,
        program: &[u8],
        origin: u32,
        instruction_set: InstructionSet,
    ) -> Result<(), Arm7TdmiError> {
        validate_image(program, origin, instruction_set)?;
        let start = origin as usize;
        let end = start + program.len();
        let mut next = Arm7TdmiState::default();
        next.memory[start..end].copy_from_slice(program);
        next.registers.pc = origin;
        next.loaded_origin = origin;
        next.loaded_len = program.len();
        if instruction_set == InstructionSet::Thumb {
            next.cpsr |= CPSR_T;
        }
        self.state = next;
        Ok(())
    }

    pub fn read_register(&self, register: usize) -> Result<u32, Arm7TdmiError> {
        read_register(&self.state, register)
    }

    pub fn write_register(&mut self, register: usize, value: u32) -> Result<(), Arm7TdmiError> {
        if register == 15 {
            write_pc_preserve(&mut self.state, value)
        } else {
            write_register(&mut self.state, register, value)
        }
    }

    pub fn read_memory(&self, address: u32, width: usize) -> Result<Vec<u8>, Arm7TdmiError> {
        Ok(self.state.memory[checked_range(address, width)?].to_vec())
    }

    pub fn write_memory(&mut self, address: u32, bytes: &[u8]) -> Result<(), Arm7TdmiError> {
        let range = checked_range(address, bytes.len())?;
        self.state.memory[range].copy_from_slice(bytes);
        Ok(())
    }

    pub fn step(&mut self) -> Result<StepTrace, Arm7TdmiError> {
        if self.state.halted {
            return Err(Arm7TdmiError::Halted);
        }
        let before = self.state.clone();
        let mut next = before.clone();
        let (set, width, fetched, raw) = fetch(&before)?;
        let pc = before.registers.pc;
        next.registers.pc = pc.wrapping_add(width as u32);
        let (mnemonic, condition_met) = match set {
            InstructionSet::Arm => execute_arm(&mut next, raw, pc)?,
            InstructionSet::Thumb => execute_thumb(&mut next, raw as u16, pc)?,
        };
        validate_state(&next)?;
        let trace = StepTrace {
            pc_before: pc,
            pc_after: next.registers.pc,
            instruction_set: set,
            width,
            fetched,
            raw,
            mnemonic,
            condition_met,
            state_before: before,
            state_after: next.clone(),
        };
        self.state = next;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Arm7TdmiError> {
        preflight_retention(max_steps)?;
        let original = self.state.clone();
        let mut traces = Vec::new();
        traces
            .try_reserve(max_steps)
            .map_err(|_| Arm7TdmiError::AllocationFailed)?;
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.state = original;
                    return Err(error);
                }
            }
            if self.state.halted {
                return Ok(ExecutionResult {
                    halted: true,
                    steps: traces.len(),
                    traces,
                    final_state: self.state.clone(),
                });
            }
        }
        self.state = original;
        Err(Arm7TdmiError::StepLimitExceeded { limit: max_steps })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        instruction_set: InstructionSet,
        max_steps: usize,
    ) -> Result<ExecutionResult, Arm7TdmiError> {
        preflight_retention(max_steps)?;
        validate_image(program, origin, instruction_set)?;
        let original = self.state.clone();
        self.load(program, origin, instruction_set)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = original;
                Err(error)
            }
        }
    }
}

fn mode(state: &Arm7TdmiState) -> Mode {
    Mode::from_bits(state.cpsr).expect("validated CPSR mode")
}

fn instruction_set(state: &Arm7TdmiState) -> InstructionSet {
    if state.cpsr & CPSR_T == 0 {
        InstructionSet::Arm
    } else {
        InstructionSet::Thumb
    }
}

fn bank_13_14(registers: &PhysicalRegisters, mode: Mode) -> &[u32; 2] {
    match mode {
        Mode::User | Mode::System => &registers.user_r13_r14,
        Mode::Fiq => (&registers.fiq_r8_r14[5..7])
            .try_into()
            .expect("two FIQ bank registers"),
        Mode::Irq => &registers.irq_r13_r14,
        Mode::Supervisor => &registers.supervisor_r13_r14,
        Mode::Abort => &registers.abort_r13_r14,
        Mode::Undefined => &registers.undefined_r13_r14,
    }
}

fn bank_13_14_mut(registers: &mut PhysicalRegisters, mode: Mode) -> &mut [u32; 2] {
    match mode {
        Mode::User | Mode::System => &mut registers.user_r13_r14,
        Mode::Fiq => (&mut registers.fiq_r8_r14[5..7])
            .try_into()
            .expect("two FIQ bank registers"),
        Mode::Irq => &mut registers.irq_r13_r14,
        Mode::Supervisor => &mut registers.supervisor_r13_r14,
        Mode::Abort => &mut registers.abort_r13_r14,
        Mode::Undefined => &mut registers.undefined_r13_r14,
    }
}

fn read_register(state: &Arm7TdmiState, register: usize) -> Result<u32, Arm7TdmiError> {
    let value = match register {
        0..=7 => state.registers.common_r0_r7[register],
        8..=12 if mode(state) == Mode::Fiq => state.registers.fiq_r8_r14[register - 8],
        8..=12 => state.registers.normal_r8_r12[register - 8],
        13..=14 => bank_13_14(&state.registers, mode(state))[register - 13],
        15 => state.registers.pc,
        _ => return Err(Arm7TdmiError::InvalidRegister { register }),
    };
    Ok(value)
}

fn write_register(
    state: &mut Arm7TdmiState,
    register: usize,
    value: u32,
) -> Result<(), Arm7TdmiError> {
    let current_mode = mode(state);
    match register {
        0..=7 => state.registers.common_r0_r7[register] = value,
        8..=12 if current_mode == Mode::Fiq => state.registers.fiq_r8_r14[register - 8] = value,
        8..=12 => state.registers.normal_r8_r12[register - 8] = value,
        13..=14 => bank_13_14_mut(&mut state.registers, current_mode)[register - 13] = value,
        15 => state.registers.pc = value,
        _ => return Err(Arm7TdmiError::InvalidRegister { register }),
    }
    Ok(())
}

fn spsr(state: &Arm7TdmiState) -> Result<u32, Arm7TdmiError> {
    match mode(state) {
        Mode::Fiq => Ok(state.spsr_fiq),
        Mode::Irq => Ok(state.spsr_irq),
        Mode::Supervisor => Ok(state.spsr_supervisor),
        Mode::Abort => Ok(state.spsr_abort),
        Mode::Undefined => Ok(state.spsr_undefined),
        mode @ (Mode::User | Mode::System) => Err(Arm7TdmiError::UnavailableSavedStatus { mode }),
    }
}

fn set_spsr(state: &mut Arm7TdmiState, value: u32) -> Result<(), Arm7TdmiError> {
    validate_status(value)?;
    match mode(state) {
        Mode::Fiq => state.spsr_fiq = value,
        Mode::Irq => state.spsr_irq = value,
        Mode::Supervisor => state.spsr_supervisor = value,
        Mode::Abort => state.spsr_abort = value,
        Mode::Undefined => state.spsr_undefined = value,
        mode @ (Mode::User | Mode::System) => {
            return Err(Arm7TdmiError::UnavailableSavedStatus { mode })
        }
    }
    Ok(())
}

fn validate_status(status: u32) -> Result<(), Arm7TdmiError> {
    if status & !CPSR_ADMITTED != 0 {
        return Err(Arm7TdmiError::InvalidState {
            reason: "status register contains reserved bits",
        });
    }
    Mode::from_bits(status).ok_or(Arm7TdmiError::ReservedMode {
        mode: (status & 0x1f) as u8,
    })?;
    Ok(())
}

fn validate_state(state: &Arm7TdmiState) -> Result<(), Arm7TdmiError> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(Arm7TdmiError::InvalidState {
            reason: "memory must contain exactly 65536 bytes",
        });
    }
    validate_status(state.cpsr)?;
    for status in [
        state.spsr_fiq,
        state.spsr_irq,
        state.spsr_supervisor,
        state.spsr_abort,
        state.spsr_undefined,
    ] {
        if status != 0 {
            validate_status(status)?;
        }
    }
    if state.loaded_len == 0 {
        return Err(Arm7TdmiError::InvalidState {
            reason: "installed image must be nonempty",
        });
    }
    if !(state.loaded_origin as usize).is_multiple_of(2) || !state.loaded_len.is_multiple_of(2) {
        return Err(Arm7TdmiError::InvalidState {
            reason: "installed image is not halfword aligned",
        });
    }
    if (state.loaded_origin as usize)
        .checked_add(state.loaded_len)
        .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(Arm7TdmiError::InvalidState {
            reason: "installed image range exceeds memory",
        });
    }
    let pc_alignment = if state.cpsr & CPSR_T == 0 { 4 } else { 2 };
    if !(state.registers.pc as usize).is_multiple_of(pc_alignment) {
        return Err(Arm7TdmiError::InvalidState {
            reason: "program counter is misaligned for current instruction state",
        });
    }
    Ok(())
}

fn validate_image(program: &[u8], origin: u32, set: InstructionSet) -> Result<(), Arm7TdmiError> {
    if program.is_empty() {
        return Err(Arm7TdmiError::EmptyProgram);
    }
    let alignment = if set == InstructionSet::Arm { 4 } else { 2 };
    if !(origin as usize).is_multiple_of(alignment) || !program.len().is_multiple_of(alignment) {
        return Err(Arm7TdmiError::MisalignedProgram { origin, alignment });
    }
    if (origin as usize)
        .checked_add(program.len())
        .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(Arm7TdmiError::ProgramOutOfRange {
            origin,
            length: program.len(),
        });
    }
    Ok(())
}

fn preflight_retention(max_steps: usize) -> Result<(), Arm7TdmiError> {
    if max_steps > MAX_RETAINED_STEPS {
        return Err(Arm7TdmiError::RetentionLimitExceeded {
            requested: max_steps,
            limit: MAX_RETAINED_STEPS,
        });
    }
    Ok(())
}

fn fetch(state: &Arm7TdmiState) -> Result<(InstructionSet, usize, [u8; 4], u32), Arm7TdmiError> {
    let set = instruction_set(state);
    let width = if set == InstructionSet::Arm { 4 } else { 2 };
    let pc = state.registers.pc;
    if !(pc as usize).is_multiple_of(width) {
        return Err(Arm7TdmiError::MisalignedFetch {
            pc,
            alignment: width,
        });
    }
    let start = pc as usize;
    let image_start = state.loaded_origin as usize;
    let image_end = image_start + state.loaded_len;
    let Some(end) = start.checked_add(width) else {
        return Err(Arm7TdmiError::FetchOutsideProgram { pc, width });
    };
    if start < image_start || end > image_end {
        return Err(Arm7TdmiError::FetchOutsideProgram { pc, width });
    }
    let mut bytes = [0; 4];
    bytes[..width].copy_from_slice(&state.memory[start..end]);
    let raw = if width == 4 {
        u32::from_le_bytes(bytes)
    } else {
        u16::from_le_bytes([bytes[0], bytes[1]]) as u32
    };
    Ok((set, width, bytes, raw))
}

fn checked_range(address: u32, width: usize) -> Result<Range<usize>, Arm7TdmiError> {
    let start = address as usize;
    let Some(end) = start.checked_add(width) else {
        return Err(Arm7TdmiError::DataOutOfRange { address, width });
    };
    if end > MEMORY_SIZE {
        return Err(Arm7TdmiError::DataOutOfRange { address, width });
    }
    Ok(start..end)
}

fn check_alignment(address: u32, width: usize) -> Result<(), Arm7TdmiError> {
    if width > 1 && !(address as usize).is_multiple_of(width) {
        return Err(Arm7TdmiError::MisalignedData { address, width });
    }
    Ok(())
}

fn read_mem(state: &Arm7TdmiState, address: u32, width: usize) -> Result<u32, Arm7TdmiError> {
    check_alignment(address, width)?;
    let range = checked_range(address, width)?;
    Ok(match width {
        1 => state.memory[range.start] as u32,
        2 => u16::from_le_bytes(state.memory[range].try_into().expect("halfword range")) as u32,
        4 => u32::from_le_bytes(state.memory[range].try_into().expect("word range")),
        _ => unreachable!("architectural access width"),
    })
}

fn write_mem(
    state: &mut Arm7TdmiState,
    address: u32,
    width: usize,
    value: u32,
) -> Result<(), Arm7TdmiError> {
    check_alignment(address, width)?;
    let range = checked_range(address, width)?;
    let bytes = value.to_le_bytes();
    state.memory[range].copy_from_slice(&bytes[..width]);
    Ok(())
}

fn condition_passed(cpsr: u32, condition: u8) -> bool {
    let n = cpsr & CPSR_N != 0;
    let z = cpsr & CPSR_Z != 0;
    let c = cpsr & CPSR_C != 0;
    let v = cpsr & CPSR_V != 0;
    match condition {
        0x0 => z,
        0x1 => !z,
        0x2 => c,
        0x3 => !c,
        0x4 => n,
        0x5 => !n,
        0x6 => v,
        0x7 => !v,
        0x8 => c && !z,
        0x9 => !c || z,
        0xa => n == v,
        0xb => n != v,
        0xc => !z && n == v,
        0xd => z || n != v,
        0xe => true,
        _ => false,
    }
}

fn set_nz(state: &mut Arm7TdmiState, value: u32) {
    state.cpsr = (state.cpsr & !(CPSR_N | CPSR_Z))
        | if value & 0x8000_0000 != 0 { CPSR_N } else { 0 }
        | if value == 0 { CPSR_Z } else { 0 };
}

fn set_nzc(state: &mut Arm7TdmiState, value: u32, carry: bool) {
    set_nz(state, value);
    if carry {
        state.cpsr |= CPSR_C;
    } else {
        state.cpsr &= !CPSR_C;
    }
}

fn set_nzcv(state: &mut Arm7TdmiState, value: u32, carry: bool, overflow: bool) {
    set_nzc(state, value, carry);
    if overflow {
        state.cpsr |= CPSR_V;
    } else {
        state.cpsr &= !CPSR_V;
    }
}

fn add_with_carry(left: u32, right: u32, carry: bool) -> (u32, bool, bool) {
    let wide = left as u64 + right as u64 + u64::from(carry);
    let result = wide as u32;
    let signed = left as i32 as i64 + right as i32 as i64 + i64::from(carry);
    (
        result,
        wide > u32::MAX as u64,
        signed < i32::MIN as i64 || signed > i32::MAX as i64,
    )
}

fn sign_extend(value: u32, bits: u32) -> u32 {
    (((value << (32 - bits)) as i32) >> (32 - bits)) as u32
}

fn arm_operand(state: &Arm7TdmiState, register: usize, pc: u32) -> u32 {
    if register == 15 {
        pc.wrapping_add(8)
    } else {
        read_register(state, register).expect("decoded register")
    }
}

fn arm_store_operand(state: &Arm7TdmiState, register: usize, pc: u32) -> u32 {
    if register == 15 {
        pc.wrapping_add(12)
    } else {
        read_register(state, register).expect("decoded register")
    }
}

fn thumb_operand(state: &Arm7TdmiState, register: usize, pc: u32) -> u32 {
    if register == 15 {
        pc.wrapping_add(4)
    } else {
        read_register(state, register).expect("decoded register")
    }
}

fn checked_target(target: u32, set: InstructionSet) -> Result<(), Arm7TdmiError> {
    let width = if set == InstructionSet::Arm { 4 } else { 2 };
    if !(target as usize).is_multiple_of(width) {
        return Err(Arm7TdmiError::MisalignedTarget {
            target,
            instruction_set: set,
        });
    }
    if (target as usize)
        .checked_add(width)
        .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(Arm7TdmiError::TargetOutOfRange {
            target,
            instruction_set: set,
        });
    }
    Ok(())
}

fn write_pc_preserve(state: &mut Arm7TdmiState, value: u32) -> Result<(), Arm7TdmiError> {
    let set = instruction_set(state);
    let target = if set == InstructionSet::Arm {
        value & !3
    } else {
        value & !1
    };
    checked_target(target, set)?;
    state.registers.pc = target;
    Ok(())
}

fn bx(state: &mut Arm7TdmiState, target: u32, raw: u32, pc: u32) -> Result<(), Arm7TdmiError> {
    if target & 3 == 2 {
        return Err(Arm7TdmiError::UnpredictableInstruction { raw, pc });
    }
    if target & 1 != 0 {
        let exchanged = target & !1;
        checked_target(exchanged, InstructionSet::Thumb)?;
        state.cpsr |= CPSR_T;
        state.registers.pc = exchanged;
    } else {
        checked_target(target, InstructionSet::Arm)?;
        state.cpsr &= !CPSR_T;
        state.registers.pc = target;
    }
    Ok(())
}

// Execution is split by encoding below.  Each function receives a private
// cloned state, so every early typed error is automatically transactional.

fn execute_arm(
    state: &mut Arm7TdmiState,
    raw: u32,
    pc: u32,
) -> Result<(&'static str, bool), Arm7TdmiError> {
    let condition = (raw >> 28) as u8;
    if condition == 0xf {
        return Err(Arm7TdmiError::UnknownInstruction { raw, width: 4, pc });
    }
    if !condition_passed(state.cpsr, condition) {
        return Ok(("CONDITION_FAILED", false));
    }

    let mnemonic = if raw == 0xef00_0000 {
        state.halted = true;
        "SWI 0"
    } else if raw >> 24 & 0xf == 0xf {
        return Err(Arm7TdmiError::UnsupportedException {
            immediate: raw & 0x00ff_ffff,
            pc,
        });
    } else if raw & 0x0fff_fff0 == 0x012f_ff10 {
        let rm = (raw & 0xf) as usize;
        let target = arm_operand(state, rm, pc);
        bx(state, target, raw, pc)?;
        "BX"
    } else if raw & 0x0fff_fff0 == 0x012f_ff30 || raw & 0x0ff0_00f0 == 0x0120_0070 {
        return Err(Arm7TdmiError::UnsupportedInstruction { raw, width: 4, pc });
    } else if raw & 0x0fb0_0ff0 == 0x0100_0090 {
        execute_swap(state, raw, pc)?
    } else if raw & 0x0f80_00f0 == 0x0080_0090 {
        execute_long_multiply(state, raw, pc)?
    } else if raw & 0x0fc0_00f0 == 0x0000_0090 {
        execute_multiply(state, raw, pc)?
    } else if raw & 0x0e00_0090 == 0x0000_0090 {
        execute_halfword_transfer(state, raw, pc)?
    } else if raw & 0x0fbf_0fff == 0x010f_0000 {
        execute_mrs(state, raw)?
    } else if raw & 0x0fb0_fff0 == 0x0120_f000 {
        execute_msr(state, raw, pc, false)?
    } else if raw & 0x0fb0_f000 == 0x0320_f000 {
        execute_msr(state, raw, pc, true)?
    } else {
        match (raw >> 25) & 7 {
            0b000 | 0b001 => execute_data_processing(state, raw, pc)?,
            0b010 | 0b011 => execute_single_transfer(state, raw, pc)?,
            0b100 => execute_block_transfer(state, raw, pc)?,
            0b101 => execute_branch(state, raw, pc)?,
            0b110 | 0b111 => {
                return Err(Arm7TdmiError::UnsupportedInstruction { raw, width: 4, pc })
            }
            _ => unreachable!(),
        }
    };
    Ok((mnemonic, mnemonic != "CONDITION_FAILED"))
}

fn shifted(value: u32, kind: u32, amount: u32, immediate: bool, old_carry: bool) -> (u32, bool) {
    match kind {
        0 => {
            if amount == 0 {
                (value, old_carry)
            } else if amount < 32 {
                (value << amount, value >> (32 - amount) & 1 != 0)
            } else if amount == 32 {
                (0, value & 1 != 0)
            } else {
                (0, false)
            }
        }
        1 => {
            let amount = if immediate && amount == 0 { 32 } else { amount };
            if amount == 0 {
                (value, old_carry)
            } else if amount < 32 {
                (value >> amount, value >> (amount - 1) & 1 != 0)
            } else if amount == 32 {
                (0, value >> 31 != 0)
            } else {
                (0, false)
            }
        }
        2 => {
            let amount = if immediate && amount == 0 { 32 } else { amount };
            if amount == 0 {
                (value, old_carry)
            } else if amount < 32 {
                (
                    ((value as i32) >> amount) as u32,
                    value >> (amount - 1) & 1 != 0,
                )
            } else {
                let sign = value >> 31 != 0;
                (if sign { u32::MAX } else { 0 }, sign)
            }
        }
        3 => {
            if immediate && amount == 0 {
                ((u32::from(old_carry) << 31) | (value >> 1), value & 1 != 0)
            } else if amount == 0 {
                (value, old_carry)
            } else {
                let rotate = amount % 32;
                if rotate == 0 {
                    (value, value >> 31 != 0)
                } else {
                    (value.rotate_right(rotate), value >> (rotate - 1) & 1 != 0)
                }
            }
        }
        _ => unreachable!(),
    }
}

fn decode_operand2(state: &Arm7TdmiState, raw: u32, pc: u32) -> (u32, bool) {
    let carry = state.cpsr & CPSR_C != 0;
    if raw & (1 << 25) != 0 {
        let immediate = raw & 0xff;
        let rotate = ((raw >> 8) & 0xf) * 2;
        if rotate == 0 {
            (immediate, carry)
        } else {
            let value = immediate.rotate_right(rotate);
            (value, value >> 31 != 0)
        }
    } else {
        let rm = (raw & 0xf) as usize;
        let kind = (raw >> 5) & 3;
        if raw & (1 << 4) == 0 {
            shifted(
                arm_operand(state, rm, pc),
                kind,
                (raw >> 7) & 0x1f,
                true,
                carry,
            )
        } else {
            let rs = ((raw >> 8) & 0xf) as usize;
            let amount = arm_operand(state, rs, pc) & 0xff;
            let rm_value = if rm == 15 {
                pc.wrapping_add(12)
            } else {
                arm_operand(state, rm, pc)
            };
            shifted(rm_value, kind, amount, false, carry)
        }
    }
}

fn execute_data_processing(
    state: &mut Arm7TdmiState,
    raw: u32,
    pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    if raw & 0x0f90_00f0 == 0x0100_0090 {
        return Err(Arm7TdmiError::UnknownInstruction { raw, width: 4, pc });
    }
    let opcode = ((raw >> 21) & 0xf) as u8;
    let set_flags = raw & (1 << 20) != 0;
    let rn = ((raw >> 16) & 0xf) as usize;
    let rd = ((raw >> 12) & 0xf) as usize;
    let left = arm_operand(state, rn, pc);
    let (right, shifter_carry) = decode_operand2(state, raw, pc);
    let carry = state.cpsr & CPSR_C != 0;
    let mut arithmetic = None;
    let (result, mnemonic, writes) = match opcode {
        0x0 => (left & right, "AND", true),
        0x1 => (left ^ right, "EOR", true),
        0x2 => {
            let x = add_with_carry(left, !right, true);
            arithmetic = Some((x.1, x.2));
            (x.0, "SUB", true)
        }
        0x3 => {
            let x = add_with_carry(right, !left, true);
            arithmetic = Some((x.1, x.2));
            (x.0, "RSB", true)
        }
        0x4 => {
            let x = add_with_carry(left, right, false);
            arithmetic = Some((x.1, x.2));
            (x.0, "ADD", true)
        }
        0x5 => {
            let x = add_with_carry(left, right, carry);
            arithmetic = Some((x.1, x.2));
            (x.0, "ADC", true)
        }
        0x6 => {
            let x = add_with_carry(left, !right, carry);
            arithmetic = Some((x.1, x.2));
            (x.0, "SBC", true)
        }
        0x7 => {
            let x = add_with_carry(right, !left, carry);
            arithmetic = Some((x.1, x.2));
            (x.0, "RSC", true)
        }
        0x8 => (left & right, "TST", false),
        0x9 => (left ^ right, "TEQ", false),
        0xa => {
            let x = add_with_carry(left, !right, true);
            arithmetic = Some((x.1, x.2));
            (x.0, "CMP", false)
        }
        0xb => {
            let x = add_with_carry(left, right, false);
            arithmetic = Some((x.1, x.2));
            (x.0, "CMN", false)
        }
        0xc => (left | right, "ORR", true),
        0xd => (right, "MOV", true),
        0xe => (left & !right, "BIC", true),
        0xf => (!right, "MVN", true),
        _ => unreachable!(),
    };
    let updates = set_flags || !writes;
    if writes && rd == 15 {
        if set_flags {
            let restored = spsr(state)?;
            validate_status(restored)?;
            state.cpsr = restored;
        }
        write_pc_preserve(state, result)?;
    } else if writes {
        write_register(state, rd, result)?;
    }
    if updates && !(writes && rd == 15) {
        if let Some((c, v)) = arithmetic {
            set_nzcv(state, result, c, v);
        } else {
            set_nzc(state, result, shifter_carry);
        }
    }
    Ok(mnemonic)
}

fn execute_multiply(
    state: &mut Arm7TdmiState,
    raw: u32,
    pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    let accumulate = raw & (1 << 21) != 0;
    let set_flags = raw & (1 << 20) != 0;
    let rd = ((raw >> 16) & 0xf) as usize;
    let rn = ((raw >> 12) & 0xf) as usize;
    let rs = ((raw >> 8) & 0xf) as usize;
    let rm = (raw & 0xf) as usize;
    if rd == 15 || rn == 15 || rs == 15 || rm == 15 || rd == rm {
        return Err(Arm7TdmiError::UnpredictableInstruction { raw, pc });
    }
    let mut result = read_register(state, rm)?.wrapping_mul(read_register(state, rs)?);
    if accumulate {
        result = result.wrapping_add(read_register(state, rn)?);
    }
    write_register(state, rd, result)?;
    if set_flags {
        set_nz(state, result);
    }
    Ok(if accumulate { "MLA" } else { "MUL" })
}

fn execute_long_multiply(
    state: &mut Arm7TdmiState,
    raw: u32,
    pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    let signed = raw & (1 << 22) != 0;
    let accumulate = raw & (1 << 21) != 0;
    let set_flags = raw & (1 << 20) != 0;
    let hi = ((raw >> 16) & 0xf) as usize;
    let lo = ((raw >> 12) & 0xf) as usize;
    let rs = ((raw >> 8) & 0xf) as usize;
    let rm = (raw & 0xf) as usize;
    if [hi, lo, rs, rm].contains(&15) || hi == lo || hi == rm || lo == rm {
        return Err(Arm7TdmiError::UnpredictableInstruction { raw, pc });
    }
    let product = if signed {
        (read_register(state, rm)? as i32 as i64)
            .wrapping_mul(read_register(state, rs)? as i32 as i64) as u64
    } else {
        (read_register(state, rm)? as u64).wrapping_mul(read_register(state, rs)? as u64)
    };
    let prior = ((read_register(state, hi)? as u64) << 32) | read_register(state, lo)? as u64;
    let result = if accumulate {
        product.wrapping_add(prior)
    } else {
        product
    };
    write_register(state, lo, result as u32)?;
    write_register(state, hi, (result >> 32) as u32)?;
    if set_flags {
        state.cpsr = (state.cpsr & !(CPSR_N | CPSR_Z))
            | if result >> 63 != 0 { CPSR_N } else { 0 }
            | if result == 0 { CPSR_Z } else { 0 };
    }
    Ok(match (signed, accumulate) {
        (false, false) => "UMULL",
        (false, true) => "UMLAL",
        (true, false) => "SMULL",
        (true, true) => "SMLAL",
    })
}

fn address_offset(state: &Arm7TdmiState, raw: u32, pc: u32) -> Result<u32, Arm7TdmiError> {
    if raw & (1 << 25) == 0 {
        return Ok(raw & 0xfff);
    }
    if raw & (1 << 4) != 0 {
        return Err(Arm7TdmiError::UnknownInstruction { raw, width: 4, pc });
    }
    let rm = (raw & 0xf) as usize;
    if rm == 15 {
        return Err(Arm7TdmiError::UnpredictableInstruction { raw, pc });
    }
    Ok(shifted(
        arm_operand(state, rm, pc),
        (raw >> 5) & 3,
        (raw >> 7) & 0x1f,
        true,
        state.cpsr & CPSR_C != 0,
    )
    .0)
}

fn execute_single_transfer(
    state: &mut Arm7TdmiState,
    raw: u32,
    pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    let pre = raw & (1 << 24) != 0;
    let up = raw & (1 << 23) != 0;
    let byte = raw & (1 << 22) != 0;
    let writeback = raw & (1 << 21) != 0 || !pre;
    let load = raw & (1 << 20) != 0;
    let rn = ((raw >> 16) & 0xf) as usize;
    let rd = ((raw >> 12) & 0xf) as usize;
    if rn == 15 && writeback || load && writeback && rn == rd {
        return Err(Arm7TdmiError::UnpredictableInstruction { raw, pc });
    }
    let base = arm_operand(state, rn, pc);
    let offset = address_offset(state, raw, pc)?;
    let indexed = if up {
        base.wrapping_add(offset)
    } else {
        base.wrapping_sub(offset)
    };
    let address = if pre { indexed } else { base };
    let width = if byte { 1 } else { 4 };
    check_alignment(address, width)?;
    checked_range(address, width)?;
    if load {
        let value = read_mem(state, address, width)?;
        if rd == 15 {
            write_pc_preserve(state, value)?;
        } else {
            write_register(state, rd, value)?;
        }
    } else {
        let value = arm_store_operand(state, rd, pc);
        write_mem(state, address, width, value)?;
    }
    if writeback {
        write_register(state, rn, indexed)?;
    }
    Ok(match (load, byte) {
        (true, true) => "LDRB",
        (true, false) => "LDR",
        (false, true) => "STRB",
        (false, false) => "STR",
    })
}

fn execute_halfword_transfer(
    state: &mut Arm7TdmiState,
    raw: u32,
    pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    let pre = raw & (1 << 24) != 0;
    let up = raw & (1 << 23) != 0;
    let immediate = raw & (1 << 22) != 0;
    let writeback = raw & (1 << 21) != 0 || !pre;
    let load = raw & (1 << 20) != 0;
    let rn = ((raw >> 16) & 0xf) as usize;
    let rd = ((raw >> 12) & 0xf) as usize;
    let kind = (raw >> 5) & 3;
    if kind == 0
        || rd == 15
        || rn == 15 && writeback
        || load && writeback && rn == rd
        || !load && kind != 1
    {
        return Err(Arm7TdmiError::UnpredictableInstruction { raw, pc });
    }
    if !immediate && raw & 0xf == 15 {
        return Err(Arm7TdmiError::UnpredictableInstruction { raw, pc });
    }
    let offset = if immediate {
        ((raw >> 4) & 0xf0) | (raw & 0xf)
    } else {
        arm_operand(state, (raw & 0xf) as usize, pc)
    };
    let base = arm_operand(state, rn, pc);
    let indexed = if up {
        base.wrapping_add(offset)
    } else {
        base.wrapping_sub(offset)
    };
    let address = if pre { indexed } else { base };
    let width = if kind == 2 { 1 } else { 2 };
    check_alignment(address, width)?;
    checked_range(address, width)?;
    if load {
        let mut value = read_mem(state, address, width)?;
        if kind == 2 {
            value = sign_extend(value, 8);
        } else if kind == 3 {
            value = sign_extend(value, 16);
        }
        write_register(state, rd, value)?;
    } else {
        write_mem(state, address, 2, arm_store_operand(state, rd, pc))?;
    }
    if writeback {
        write_register(state, rn, indexed)?;
    }
    Ok(match (load, kind) {
        (false, _) => "STRH",
        (true, 1) => "LDRH",
        (true, 2) => "LDRSB",
        (true, 3) => "LDRSH",
        _ => unreachable!(),
    })
}

fn execute_block_transfer(
    state: &mut Arm7TdmiState,
    raw: u32,
    pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    let pre = raw & (1 << 24) != 0;
    let up = raw & (1 << 23) != 0;
    let user = raw & (1 << 22) != 0;
    let writeback = raw & (1 << 21) != 0;
    let load = raw & (1 << 20) != 0;
    let rn = ((raw >> 16) & 0xf) as usize;
    let list = raw & 0xffff;
    if user {
        return Err(Arm7TdmiError::UnsupportedInstruction { raw, width: 4, pc });
    }
    if list == 0 || rn == 15 || load && writeback && list & (1 << rn) != 0 {
        return Err(Arm7TdmiError::UnpredictableInstruction { raw, pc });
    }
    let count = list.count_ones();
    let base = read_register(state, rn)?;
    let bytes = count * 4;
    let start = match (up, pre) {
        (true, false) => base,
        (true, true) => base.wrapping_add(4),
        (false, false) => base.wrapping_sub(bytes - 4),
        (false, true) => base.wrapping_sub(bytes),
    };
    let final_base = if up {
        base.wrapping_add(bytes)
    } else {
        base.wrapping_sub(bytes)
    };
    check_alignment(start, 4)?;
    checked_range(start, bytes as usize)?;
    let base_position = (list & ((1_u32 << rn) - 1)).count_ones();
    let mut loaded = [(0_usize, 0_u32); 16];
    let mut loaded_count = 0;
    for register in 0..16 {
        if list & (1 << register) == 0 {
            continue;
        }
        let address = start.wrapping_add(loaded_count as u32 * 4);
        if load {
            loaded[loaded_count] = (register, read_mem(state, address, 4)?);
        } else {
            let value = if register == rn && writeback && base_position != 0 {
                final_base
            } else {
                arm_store_operand(state, register, pc)
            };
            write_mem(state, address, 4, value)?;
        }
        loaded_count += 1;
    }
    if load {
        for &(register, value) in &loaded[..loaded_count] {
            if register == 15 {
                write_pc_preserve(state, value)?;
            } else {
                write_register(state, register, value)?;
            }
        }
    }
    if writeback {
        write_register(state, rn, final_base)?;
    }
    Ok(if load { "LDM" } else { "STM" })
}

fn execute_branch(
    state: &mut Arm7TdmiState,
    raw: u32,
    pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    let link = raw & (1 << 24) != 0;
    let displacement = sign_extend(raw & 0x00ff_ffff, 24).wrapping_shl(2);
    let target = pc.wrapping_add(8).wrapping_add(displacement);
    checked_target(target, InstructionSet::Arm)?;
    if link {
        write_register(state, 14, pc.wrapping_add(4))?;
    }
    state.registers.pc = target;
    Ok(if link { "BL" } else { "B" })
}

fn execute_swap(
    state: &mut Arm7TdmiState,
    raw: u32,
    pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    let byte = raw & (1 << 22) != 0;
    let rn = ((raw >> 16) & 0xf) as usize;
    let rd = ((raw >> 12) & 0xf) as usize;
    let rm = (raw & 0xf) as usize;
    if [rn, rd, rm].contains(&15) || rn == rd || rn == rm {
        return Err(Arm7TdmiError::UnpredictableInstruction { raw, pc });
    }
    let address = read_register(state, rn)?;
    let width = if byte { 1 } else { 4 };
    let old = read_mem(state, address, width)?;
    let replacement = read_register(state, rm)?;
    write_mem(state, address, width, replacement)?;
    write_register(state, rd, old)?;
    Ok(if byte { "SWPB" } else { "SWP" })
}

fn execute_mrs(state: &mut Arm7TdmiState, raw: u32) -> Result<&'static str, Arm7TdmiError> {
    let saved = raw & (1 << 22) != 0;
    let rd = ((raw >> 12) & 0xf) as usize;
    if rd == 15 {
        return Err(Arm7TdmiError::UnpredictableInstruction {
            raw,
            pc: state.registers.pc.wrapping_sub(4),
        });
    }
    let value = if saved { spsr(state)? } else { state.cpsr };
    write_register(state, rd, value)?;
    Ok(if saved { "MRS SPSR" } else { "MRS CPSR" })
}

fn execute_msr(
    state: &mut Arm7TdmiState,
    raw: u32,
    pc: u32,
    immediate: bool,
) -> Result<&'static str, Arm7TdmiError> {
    let saved = raw & (1 << 22) != 0;
    let fields = (raw >> 16) & 0xf;
    let operand = if immediate {
        (raw & 0xff).rotate_right(((raw >> 8) & 0xf) * 2)
    } else {
        let rm = (raw & 0xf) as usize;
        if rm == 15 {
            return Err(Arm7TdmiError::UnpredictableInstruction { raw, pc });
        }
        read_register(state, rm)?
    };
    let mut mask = 0;
    if fields & 1 != 0 {
        mask |= 0x0000_00ff;
    }
    if fields & 2 != 0 {
        mask |= 0x0000_ff00;
    }
    if fields & 4 != 0 {
        mask |= 0x00ff_0000;
    }
    if fields & 8 != 0 {
        mask |= 0xff00_0000;
    }
    mask &= CPSR_ADMITTED;
    if saved {
        let current = spsr(state)?;
        set_spsr(state, (current & !mask) | (operand & mask))?;
        Ok("MSR SPSR")
    } else {
        if mode(state) == Mode::User {
            mask &= 0xff00_0000;
        }
        let value = (state.cpsr & !mask) | (operand & mask);
        if value & CPSR_T != state.cpsr & CPSR_T {
            return Err(Arm7TdmiError::UnpredictableInstruction { raw, pc });
        }
        validate_status(value)?;
        state.cpsr = value;
        Ok("MSR CPSR")
    }
}

fn execute_thumb(
    state: &mut Arm7TdmiState,
    raw: u16,
    pc: u32,
) -> Result<(&'static str, bool), Arm7TdmiError> {
    let word = raw as u32;
    if raw & 0xff00 == 0xbe00
        || raw & 0xfff7 == 0xb650
        || raw & 0xffe8 == 0xb660
        || raw & 0xff87 == 0x4780
        || raw & 0xf800 == 0xe800
    {
        return Err(Arm7TdmiError::UnsupportedInstruction {
            raw: word,
            width: 2,
            pc,
        });
    }
    let mnemonic = match raw >> 13 {
        0b000 => execute_thumb_shift_add(state, raw, pc)?,
        0b001 => execute_thumb_immediate(state, raw)?,
        0b010 => execute_thumb_data_or_register_transfer(state, raw, pc)?,
        0b011 => execute_thumb_immediate_transfer(state, raw, false)?,
        0b100 => {
            if raw & 0x1000 == 0 {
                execute_thumb_immediate_transfer(state, raw, true)?
            } else {
                execute_thumb_sp_transfer(state, raw)?
            }
        }
        0b101 => execute_thumb_misc(state, raw, pc)?,
        0b110 => execute_thumb_multiple_or_conditional(state, raw, pc)?,
        0b111 => execute_thumb_branch_or_link(state, raw, pc)?,
        _ => {
            return Err(Arm7TdmiError::UnknownInstruction {
                raw: word,
                width: 2,
                pc,
            })
        }
    };
    Ok((mnemonic, mnemonic != "CONDITION_FAILED"))
}

fn execute_thumb_shift_add(
    state: &mut Arm7TdmiState,
    raw: u16,
    _pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    let rd = (raw & 7) as usize;
    let rs = ((raw >> 3) & 7) as usize;
    if raw & 0x1800 != 0x1800 {
        let kind = ((raw >> 11) & 3) as u32;
        let amount = ((raw >> 6) & 0x1f) as u32;
        let (result, carry) = shifted(
            read_register(state, rs)?,
            kind,
            amount,
            true,
            state.cpsr & CPSR_C != 0,
        );
        write_register(state, rd, result)?;
        set_nzc(state, result, carry);
        return Ok(match kind {
            0 => "LSL",
            1 => "LSR",
            2 => "ASR",
            _ => unreachable!(),
        });
    }
    let immediate = raw & (1 << 10) != 0;
    let subtract = raw & (1 << 9) != 0;
    let right = if immediate {
        ((raw >> 6) & 7) as u32
    } else {
        read_register(state, ((raw >> 6) & 7) as usize)?
    };
    let left = read_register(state, rs)?;
    let (result, carry, overflow) = if subtract {
        add_with_carry(left, !right, true)
    } else {
        add_with_carry(left, right, false)
    };
    write_register(state, rd, result)?;
    set_nzcv(state, result, carry, overflow);
    Ok(if subtract { "SUB" } else { "ADD" })
}

fn execute_thumb_immediate(
    state: &mut Arm7TdmiState,
    raw: u16,
) -> Result<&'static str, Arm7TdmiError> {
    let operation = (raw >> 11) & 3;
    let rd = ((raw >> 8) & 7) as usize;
    let immediate = (raw & 0xff) as u32;
    let left = read_register(state, rd)?;
    match operation {
        0 => {
            write_register(state, rd, immediate)?;
            set_nz(state, immediate);
            Ok("MOV")
        }
        1 => {
            let (result, c, v) = add_with_carry(left, !immediate, true);
            set_nzcv(state, result, c, v);
            Ok("CMP")
        }
        2 => {
            let (result, c, v) = add_with_carry(left, immediate, false);
            write_register(state, rd, result)?;
            set_nzcv(state, result, c, v);
            Ok("ADD")
        }
        3 => {
            let (result, c, v) = add_with_carry(left, !immediate, true);
            write_register(state, rd, result)?;
            set_nzcv(state, result, c, v);
            Ok("SUB")
        }
        _ => unreachable!(),
    }
}

fn execute_thumb_data_or_register_transfer(
    state: &mut Arm7TdmiState,
    raw: u16,
    pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    if raw & 0xfc00 == 0x4000 {
        return execute_thumb_alu(state, raw);
    }
    if raw & 0xfc00 == 0x4400 {
        return execute_thumb_high(state, raw, pc);
    }
    if raw & 0xf800 == 0x4800 {
        let rd = ((raw >> 8) & 7) as usize;
        let address = pc.wrapping_add(4) & !3;
        let address = address.wrapping_add(((raw & 0xff) as u32) << 2);
        write_register(state, rd, read_mem(state, address, 4)?)?;
        return Ok("LDR LITERAL");
    }
    if raw & 0xf000 == 0x5000 {
        return execute_thumb_register_transfer(state, raw);
    }
    Err(Arm7TdmiError::UnknownInstruction {
        raw: raw as u32,
        width: 2,
        pc,
    })
}

fn execute_thumb_alu(state: &mut Arm7TdmiState, raw: u16) -> Result<&'static str, Arm7TdmiError> {
    let opcode = (raw >> 6) & 0xf;
    let rs = ((raw >> 3) & 7) as usize;
    let rd = (raw & 7) as usize;
    let left = read_register(state, rd)?;
    let right = read_register(state, rs)?;
    let carry = state.cpsr & CPSR_C != 0;
    match opcode {
        0x0 => {
            let r = left & right;
            write_register(state, rd, r)?;
            set_nz(state, r);
            Ok("AND")
        }
        0x1 => {
            let r = left ^ right;
            write_register(state, rd, r)?;
            set_nz(state, r);
            Ok("EOR")
        }
        0x2..=0x4 | 0x7 => {
            let kind = match opcode {
                2 => 0,
                3 => 1,
                4 => 2,
                7 => 3,
                _ => unreachable!(),
            };
            let (r, c) = shifted(left, kind, right & 0xff, false, carry);
            write_register(state, rd, r)?;
            set_nzc(state, r, c);
            Ok(match opcode {
                2 => "LSL",
                3 => "LSR",
                4 => "ASR",
                7 => "ROR",
                _ => unreachable!(),
            })
        }
        0x5 => {
            let (r, c, v) = add_with_carry(left, right, carry);
            write_register(state, rd, r)?;
            set_nzcv(state, r, c, v);
            Ok("ADC")
        }
        0x6 => {
            let (r, c, v) = add_with_carry(left, !right, carry);
            write_register(state, rd, r)?;
            set_nzcv(state, r, c, v);
            Ok("SBC")
        }
        0x8 => {
            set_nz(state, left & right);
            Ok("TST")
        }
        0x9 => {
            let (r, c, v) = add_with_carry(0, !right, true);
            write_register(state, rd, r)?;
            set_nzcv(state, r, c, v);
            Ok("NEG")
        }
        0xa => {
            let (r, c, v) = add_with_carry(left, !right, true);
            set_nzcv(state, r, c, v);
            Ok("CMP")
        }
        0xb => {
            let (r, c, v) = add_with_carry(left, right, false);
            set_nzcv(state, r, c, v);
            Ok("CMN")
        }
        0xc => {
            let r = left | right;
            write_register(state, rd, r)?;
            set_nz(state, r);
            Ok("ORR")
        }
        0xd => {
            let r = left.wrapping_mul(right);
            write_register(state, rd, r)?;
            set_nz(state, r);
            Ok("MUL")
        }
        0xe => {
            let r = left & !right;
            write_register(state, rd, r)?;
            set_nz(state, r);
            Ok("BIC")
        }
        0xf => {
            let r = !right;
            write_register(state, rd, r)?;
            set_nz(state, r);
            Ok("MVN")
        }
        _ => unreachable!(),
    }
}

fn execute_thumb_high(
    state: &mut Arm7TdmiState,
    raw: u16,
    pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    let operation = (raw >> 8) & 3;
    if operation != 3 && raw & 0x00c0 == 0 {
        return Err(Arm7TdmiError::UnpredictableInstruction {
            raw: raw as u32,
            pc,
        });
    }
    let rd = ((raw & 7) | ((raw >> 4) & 8)) as usize;
    let rs = (((raw >> 3) & 7) | ((raw >> 3) & 8)) as usize;
    let left = thumb_operand(state, rd, pc);
    let right = thumb_operand(state, rs, pc);
    match operation {
        0 => {
            let result = left.wrapping_add(right);
            if rd == 15 {
                write_pc_preserve(state, result)?;
            } else {
                write_register(state, rd, result)?;
            }
            Ok("ADD HIGH")
        }
        1 => {
            let (r, c, v) = add_with_carry(left, !right, true);
            set_nzcv(state, r, c, v);
            Ok("CMP HIGH")
        }
        2 => {
            if rd == 15 {
                write_pc_preserve(state, right)?;
            } else {
                write_register(state, rd, right)?;
            }
            Ok("MOV HIGH")
        }
        3 => {
            if raw & 0x0087 != 0 {
                return Err(Arm7TdmiError::UnknownInstruction {
                    raw: raw as u32,
                    width: 2,
                    pc,
                });
            }
            bx(state, right, raw as u32, pc)?;
            Ok("BX")
        }
        _ => unreachable!(),
    }
}

fn execute_thumb_register_transfer(
    state: &mut Arm7TdmiState,
    raw: u16,
) -> Result<&'static str, Arm7TdmiError> {
    let operation = (raw >> 9) & 7;
    let rm = ((raw >> 6) & 7) as usize;
    let rn = ((raw >> 3) & 7) as usize;
    let rd = (raw & 7) as usize;
    let address = read_register(state, rn)?.wrapping_add(read_register(state, rm)?);
    match operation {
        0 => {
            write_mem(state, address, 4, read_register(state, rd)?)?;
            Ok("STR")
        }
        1 => {
            write_mem(state, address, 2, read_register(state, rd)?)?;
            Ok("STRH")
        }
        2 => {
            write_mem(state, address, 1, read_register(state, rd)?)?;
            Ok("STRB")
        }
        3 => {
            let v = sign_extend(read_mem(state, address, 1)?, 8);
            write_register(state, rd, v)?;
            Ok("LDRSB")
        }
        4 => {
            let v = read_mem(state, address, 4)?;
            write_register(state, rd, v)?;
            Ok("LDR")
        }
        5 => {
            let v = read_mem(state, address, 2)?;
            write_register(state, rd, v)?;
            Ok("LDRH")
        }
        6 => {
            let v = read_mem(state, address, 1)?;
            write_register(state, rd, v)?;
            Ok("LDRB")
        }
        7 => {
            let v = sign_extend(read_mem(state, address, 2)?, 16);
            write_register(state, rd, v)?;
            Ok("LDRSH")
        }
        _ => unreachable!(),
    }
}

fn execute_thumb_immediate_transfer(
    state: &mut Arm7TdmiState,
    raw: u16,
    halfword: bool,
) -> Result<&'static str, Arm7TdmiError> {
    let load = raw & (1 << 11) != 0;
    let byte = !halfword && raw & (1 << 12) != 0;
    let rn = ((raw >> 3) & 7) as usize;
    let rd = (raw & 7) as usize;
    let shift = if halfword {
        1
    } else if byte {
        0
    } else {
        2
    };
    let address = read_register(state, rn)?.wrapping_add((((raw >> 6) & 0x1f) as u32) << shift);
    let width = if byte {
        1
    } else if halfword {
        2
    } else {
        4
    };
    if load {
        let v = read_mem(state, address, width)?;
        write_register(state, rd, v)?;
    } else {
        write_mem(state, address, width, read_register(state, rd)?)?;
    }
    Ok(match (load, width) {
        (true, 1) => "LDRB",
        (false, 1) => "STRB",
        (true, 2) => "LDRH",
        (false, 2) => "STRH",
        (true, 4) => "LDR",
        (false, 4) => "STR",
        _ => unreachable!(),
    })
}

fn execute_thumb_sp_transfer(
    state: &mut Arm7TdmiState,
    raw: u16,
) -> Result<&'static str, Arm7TdmiError> {
    let load = raw & (1 << 11) != 0;
    let rd = ((raw >> 8) & 7) as usize;
    let address = read_register(state, 13)?.wrapping_add(((raw & 0xff) as u32) << 2);
    if load {
        let v = read_mem(state, address, 4)?;
        write_register(state, rd, v)?;
    } else {
        write_mem(state, address, 4, read_register(state, rd)?)?;
    }
    Ok(if load { "LDR SP" } else { "STR SP" })
}

fn execute_thumb_misc(
    state: &mut Arm7TdmiState,
    raw: u16,
    pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    if raw & 0xf000 == 0xa000 {
        let rd = ((raw >> 8) & 7) as usize;
        let base = if raw & (1 << 11) != 0 {
            read_register(state, 13)?
        } else {
            pc.wrapping_add(4) & !3
        };
        write_register(state, rd, base.wrapping_add(((raw & 0xff) as u32) << 2))?;
        return Ok(if raw & (1 << 11) != 0 {
            "ADD SP"
        } else {
            "ADD PC"
        });
    }
    if raw & 0xff00 == 0xb000 {
        let offset = ((raw & 0x7f) as u32) << 2;
        let sp = read_register(state, 13)?;
        write_register(
            state,
            13,
            if raw & 0x80 != 0 {
                sp.wrapping_sub(offset)
            } else {
                sp.wrapping_add(offset)
            },
        )?;
        return Ok(if raw & 0x80 != 0 { "SUB SP" } else { "ADD SP" });
    }
    if raw & 0xf600 == 0xb400 {
        return execute_thumb_push_pop(state, raw, pc);
    }
    Err(Arm7TdmiError::UnknownInstruction {
        raw: raw as u32,
        width: 2,
        pc,
    })
}

fn execute_thumb_push_pop(
    state: &mut Arm7TdmiState,
    raw: u16,
    pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    let pop = raw & (1 << 11) != 0;
    let extension = raw & (1 << 8) != 0;
    let list = (raw & 0xff) as u32;
    let count = list.count_ones() + u32::from(extension);
    if count == 0 {
        return Err(Arm7TdmiError::UnpredictableInstruction {
            raw: raw as u32,
            pc,
        });
    }
    let sp = read_register(state, 13)?;
    let start = if pop { sp } else { sp.wrapping_sub(count * 4) };
    checked_range(start, (count * 4) as usize)?;
    check_alignment(start, 4)?;
    let mut values = [0_u32; 9];
    let mut index = 0_usize;
    if pop {
        for register in 0..8 {
            if list & (1 << register) != 0 {
                values[index] = read_mem(state, start + index as u32 * 4, 4)?;
                index += 1;
            }
        }
        if extension {
            values[index] = read_mem(state, start + index as u32 * 4, 4)?;
        }
        index = 0;
        for register in 0..8 {
            if list & (1 << register) != 0 {
                write_register(state, register, values[index])?;
                index += 1;
            }
        }
        if extension {
            write_pc_preserve(state, values[index])?;
        }
        write_register(state, 13, sp.wrapping_add(count * 4))?;
        Ok("POP")
    } else {
        for register in 0..8 {
            if list & (1 << register) != 0 {
                write_mem(
                    state,
                    start + index as u32 * 4,
                    4,
                    read_register(state, register)?,
                )?;
                index += 1;
            }
        }
        if extension {
            write_mem(
                state,
                start + index as u32 * 4,
                4,
                read_register(state, 14)?,
            )?;
        }
        write_register(state, 13, start)?;
        Ok("PUSH")
    }
}

fn execute_thumb_multiple_or_conditional(
    state: &mut Arm7TdmiState,
    raw: u16,
    pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    if raw & 0xf000 == 0xc000 {
        let load = raw & (1 << 11) != 0;
        let rn = ((raw >> 8) & 7) as usize;
        let list = (raw & 0xff) as u32;
        if list == 0 || load && list & (1 << rn) != 0 {
            return Err(Arm7TdmiError::UnpredictableInstruction {
                raw: raw as u32,
                pc,
            });
        }
        let base = read_register(state, rn)?;
        let count = list.count_ones();
        checked_range(base, (count * 4) as usize)?;
        check_alignment(base, 4)?;
        let mut values = [0_u32; 8];
        let mut index = 0;
        if load {
            for register in 0..8 {
                if list & (1 << register) != 0 {
                    values[index] = read_mem(state, base + index as u32 * 4, 4)?;
                    index += 1;
                }
            }
            index = 0;
            for register in 0..8 {
                if list & (1 << register) != 0 {
                    write_register(state, register, values[index])?;
                    index += 1;
                }
            }
        } else {
            for register in 0..8 {
                if list & (1 << register) != 0 {
                    write_mem(
                        state,
                        base + index as u32 * 4,
                        4,
                        read_register(state, register)?,
                    )?;
                    index += 1;
                }
            }
        }
        write_register(state, rn, base.wrapping_add(count * 4))?;
        return Ok(if load { "LDMIA" } else { "STMIA" });
    }
    if raw & 0xf000 == 0xd000 {
        let condition = ((raw >> 8) & 0xf) as u8;
        if condition == 0xf {
            if raw == 0xdf00 {
                state.halted = true;
                return Ok("SWI 0");
            }
            return Err(Arm7TdmiError::UnsupportedException {
                immediate: (raw & 0xff) as u32,
                pc,
            });
        }
        if condition == 0xe {
            return Err(Arm7TdmiError::UnknownInstruction {
                raw: raw as u32,
                width: 2,
                pc,
            });
        }
        if condition_passed(state.cpsr, condition) {
            let target = pc
                .wrapping_add(4)
                .wrapping_add(sign_extend((raw & 0xff) as u32, 8) << 1);
            checked_target(target, InstructionSet::Thumb)?;
            state.registers.pc = target;
            Ok("B COND")
        } else {
            Ok("CONDITION_FAILED")
        }
    } else {
        Err(Arm7TdmiError::UnknownInstruction {
            raw: raw as u32,
            width: 2,
            pc,
        })
    }
}

fn execute_thumb_branch_or_link(
    state: &mut Arm7TdmiState,
    raw: u16,
    pc: u32,
) -> Result<&'static str, Arm7TdmiError> {
    if raw & 0xf800 == 0xe000 {
        let target = pc
            .wrapping_add(4)
            .wrapping_add(sign_extend((raw & 0x7ff) as u32, 11) << 1);
        checked_target(target, InstructionSet::Thumb)?;
        state.registers.pc = target;
        return Ok("B");
    }
    if raw & 0xf800 == 0xf000 {
        let intermediate = pc
            .wrapping_add(4)
            .wrapping_add(sign_extend((raw & 0x7ff) as u32, 11) << 12);
        write_register(state, 14, intermediate)?;
        return Ok("BL PREFIX");
    }
    if raw & 0xf800 == 0xf800 {
        let lr = read_register(state, 14)?;
        let target = lr.wrapping_add(((raw & 0x7ff) as u32) << 1);
        let target = target & !1;
        checked_target(target, InstructionSet::Thumb)?;
        write_register(state, 14, pc.wrapping_add(2) | 1)?;
        state.registers.pc = target;
        return Ok("BL SUFFIX");
    }
    Err(Arm7TdmiError::UnknownInstruction {
        raw: raw as u32,
        width: 2,
        pc,
    })
}

#[cfg(test)]
mod arithmetic_oracles {
    use super::*;

    #[test]
    fn add_with_carry_matches_independent_wide_oracle() {
        let values = [0, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_fffe, u32::MAX];
        for left in values {
            for right in values {
                for carry in [false, true] {
                    let (result, unsigned_carry, overflow) = add_with_carry(left, right, carry);
                    let wide = left as u64 + right as u64 + u64::from(carry);
                    let signed = left as i32 as i64 + right as i32 as i64 + i64::from(carry);
                    assert_eq!(result, wide as u32);
                    assert_eq!(unsigned_carry, wide > u32::MAX as u64);
                    assert_eq!(
                        overflow,
                        signed < i32::MIN as i64 || signed > i32::MAX as i64
                    );
                }
            }
        }
    }

    #[test]
    fn shifter_boundary_table_matches_armv4t_rules() {
        let value = 0x8000_0001;
        assert_eq!(shifted(value, 0, 0, true, true), (value, true));
        assert_eq!(shifted(value, 0, 32, false, false), (0, true));
        assert_eq!(shifted(value, 0, 33, false, true), (0, false));
        assert_eq!(shifted(value, 1, 0, true, false), (0, true));
        assert_eq!(shifted(value, 1, 32, false, false), (0, true));
        assert_eq!(shifted(value, 2, 0, true, false), (u32::MAX, true));
        assert_eq!(shifted(value, 2, 33, false, false), (u32::MAX, true));
        assert_eq!(shifted(value, 3, 0, true, true), (0xc000_0000, true));
        assert_eq!(shifted(value, 3, 32, false, false), (value, true));
        assert_eq!(shifted(value, 3, 33, false, false), (0xc000_0000, true));
    }
}
