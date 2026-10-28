//! Checked, transactional simulator for the bounded ZipCPU revision-3 profile.
//!
//! The machine is deliberately an instruction-boundary oracle: it owns two
//! register banks, a small deterministic bus responder, exact transactions,
//! and the checked lifecycle needed by later educational gate and SoC cells.
//! Pipeline clocks and the upstream implementation are not part of production
//! code.
//!
//! ```
//! use zipcpu_simulator::{encoding, StopReason, ZipCpuSimulator};
//!
//! let program = encoding::assemble(&[
//!     encoding::ldi(1, 42),
//!     encoding::mov_to_cc(zipcpu_simulator::CC_SLEEP),
//! ])?;
//! let mut cpu = ZipCpuSimulator::new()?;
//! let result = cpu.execute(&program, 0x100, 2)?;
//! assert_eq!(result.final_state.supervisor_gpr[1], 42);
//! assert_eq!(result.stop_reason, StopReason::Halted);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::cell::Cell;
use std::error::Error;
use std::fmt;

/// Size of the deterministic teaching bus responder.
pub const MEMORY_SIZE: usize = 65_536;
/// Maximum complete-state traces retained by one bounded run.
pub const MAX_RETAINED_STEPS: usize = 64;

/// Immutable CPU-information projection in CC bits 31:23.
pub const CC_CPU_INFO: u32 = 0xeb00_0000;
/// Software-preserved per-bank CC storage.
pub const CC_RESERVED_MASK: u32 = 0x007f_0000;
pub const CC_FPU_ERROR: u32 = 1 << 12;
pub const CC_DIV_ERROR: u32 = 1 << 11;
pub const CC_BUS_ERROR: u32 = 1 << 10;
pub const CC_TRAP: u32 = 1 << 9;
pub const CC_ILLEGAL: u32 = 1 << 8;
pub const CC_BREAK: u32 = 1 << 7;
pub const CC_STEP: u32 = 1 << 6;
pub const CC_GIE: u32 = 1 << 5;
pub const CC_SLEEP: u32 = 1 << 4;
pub const CC_V: u32 = 1 << 3;
pub const CC_N: u32 = 1 << 2;
pub const CC_C: u32 = 1 << 1;
pub const CC_Z: u32 = 1;

const CC_FLAGS: u32 = CC_Z | CC_C | CC_N | CC_V;
const CC_USER_STATUS: u32 = CC_BREAK | CC_ILLEGAL | CC_BUS_ERROR | CC_DIV_ERROR;

/// Architectural register bank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bank {
    Supervisor,
    User,
}

/// Current privilege/register-bank mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Supervisor,
    User,
}

impl Mode {
    fn bank(self) -> Bank {
        match self {
            Self::Supervisor => Bank::Supervisor,
            Self::User => Bank::User,
        }
    }
}

/// Complete, owned, restorable state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZipCpuState {
    pub supervisor_gpr: [u32; 14],
    pub user_gpr: [u32; 14],
    pub supervisor_cc: u32,
    pub user_cc: u32,
    pub supervisor_pc: u32,
    pub user_pc: u32,
    pub mode: Mode,
    pub interrupt_asserted: bool,
    pub memory: Vec<u8>,
    pub image_origin: Option<u32>,
    pub image_length: u32,
}

/// One flag snapshot in Z/C/N/V order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flags {
    pub zero: bool,
    pub carry: bool,
    pub negative: bool,
    pub overflow: bool,
}

impl Flags {
    fn from_cc(cc: u32) -> Self {
        Self {
            zero: cc & CC_Z != 0,
            carry: cc & CC_C != 0,
            negative: cc & CC_N != 0,
            overflow: cc & CC_V != 0,
        }
    }

    fn bits(self) -> u32 {
        (u32::from(self.zero) * CC_Z)
            | (u32::from(self.carry) * CC_C)
            | (u32::from(self.negative) * CC_N)
            | (u32::from(self.overflow) * CC_V)
    }
}

/// A decoded operand-B form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperandB {
    Immediate(i32),
    Register { register: u8, immediate: i32 },
}

/// One of the admitted revision-3 instruction classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum InstructionClass {
    Sub = 0x00,
    And = 0x01,
    Add = 0x02,
    Or = 0x03,
    Xor = 0x04,
    Lsr = 0x05,
    Lsl = 0x06,
    Asr = 0x07,
    Brev = 0x08,
    Ldilo = 0x09,
    Mpyuhi = 0x0a,
    Mpyshi = 0x0b,
    Mpy = 0x0c,
    Mov = 0x0d,
    Divu = 0x0e,
    Divs = 0x0f,
    Cmp = 0x10,
    Tst = 0x11,
    Lw = 0x12,
    Sw = 0x13,
    Lh = 0x14,
    Sh = 0x15,
    Lb = 0x16,
    Sb = 0x17,
    Ldi = 0x18,
    Break = 0x1c,
    Noop = 0x1f,
}

impl InstructionClass {
    /// Stable uppercase trace token.
    #[must_use]
    pub const fn mnemonic(self) -> &'static str {
        match self {
            Self::Sub => "SUB",
            Self::And => "AND",
            Self::Add => "ADD",
            Self::Or => "OR",
            Self::Xor => "XOR",
            Self::Lsr => "LSR",
            Self::Lsl => "LSL",
            Self::Asr => "ASR",
            Self::Brev => "BREV",
            Self::Ldilo => "LDILO",
            Self::Mpyuhi => "MPYUHI",
            Self::Mpyshi => "MPYSHI",
            Self::Mpy => "MPY",
            Self::Mov => "MOV",
            Self::Divu => "DIVU",
            Self::Divs => "DIVS",
            Self::Cmp => "CMP",
            Self::Tst => "TST",
            Self::Lw => "LW",
            Self::Sw => "SW",
            Self::Lh => "LH",
            Self::Sh => "SH",
            Self::Lb => "LB",
            Self::Sb => "SB",
            Self::Ldi => "LDI",
            Self::Break => "BREAK",
            Self::Noop => "NOOP",
        }
    }
}

/// Conditional-execution selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Condition {
    Always = 0,
    Zero = 1,
    LessThan = 2,
    Carry = 3,
    Overflow = 4,
    NotZero = 5,
    GreaterEqual = 6,
    NotCarry = 7,
}

impl Condition {
    fn from_bits(bits: u8) -> Self {
        match bits & 7 {
            0 => Self::Always,
            1 => Self::Zero,
            2 => Self::LessThan,
            3 => Self::Carry,
            4 => Self::Overflow,
            5 => Self::NotZero,
            6 => Self::GreaterEqual,
            _ => Self::NotCarry,
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Self::Always => "",
            Self::Zero => ".Z",
            Self::LessThan => ".LT",
            Self::Carry => ".C",
            Self::Overflow => ".V",
            Self::NotZero => ".NZ",
            Self::GreaterEqual => ".GE",
            Self::NotCarry => ".NC",
        }
    }

    fn holds(self, flags: Flags) -> bool {
        match self {
            Self::Always => true,
            Self::Zero => flags.zero,
            Self::LessThan => flags.negative,
            Self::Carry => flags.carry,
            Self::Overflow => flags.overflow,
            Self::NotZero => !flags.zero,
            Self::GreaterEqual => !flags.negative,
            Self::NotCarry => !flags.carry,
        }
    }
}

/// Fully classified instruction, independent of execution state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub class: InstructionClass,
    pub destination: u8,
    pub condition: Condition,
    pub operand_b: OperandB,
    pub destination_user: bool,
    pub source_user: bool,
    pub raw: u32,
}

/// Stable decode classification for encodings outside the bounded profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    UnknownInstruction { word: u32 },
    UnsupportedInstruction { word: u32, feature: &'static str },
    ReservedInstruction { word: u32 },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownInstruction { word } => {
                write!(formatter, "unknown instruction {word:#010x}")
            }
            Self::UnsupportedInstruction { word, feature } => {
                write!(formatter, "instruction {word:#010x} requires {feature}")
            }
            Self::ReservedInstruction { word } => {
                write!(formatter, "reserved instruction {word:#010x}")
            }
        }
    }
}

impl Error for DecodeError {}

/// Decode one raw big-endian instruction word.
pub fn decode(word: u32) -> Result<DecodedInstruction, DecodeError> {
    if word & 0x8000_0000 != 0 {
        return Err(DecodeError::UnsupportedInstruction {
            word,
            feature: "the disabled compressed instruction set",
        });
    }
    let destination = ((word >> 27) & 0x0f) as u8;
    let opcode = ((word >> 22) & 0x1f) as u8;
    let special_destination = destination & 0x0e == 0x0e;
    if opcode >= 0x1a {
        if !special_destination {
            return Err(DecodeError::UnsupportedInstruction {
                word,
                feature: "the absent floating-point unit",
            });
        }
        return match opcode {
            0x1a | 0x1b => Err(DecodeError::ReservedInstruction { word }),
            0x1c => Ok(DecodedInstruction::special(InstructionClass::Break, word)),
            0x1d => Err(DecodeError::UnsupportedInstruction {
                word,
                feature: "the disabled bus lock option",
            }),
            0x1e => Err(DecodeError::UnsupportedInstruction {
                word,
                feature: "the disabled simulator option",
            }),
            _ => Ok(DecodedInstruction::special(InstructionClass::Noop, word)),
        };
    }
    if matches!(opcode, 0x0e | 0x0f) && special_destination {
        return Err(DecodeError::ReservedInstruction { word });
    }
    let class = class_from_opcode(opcode).ok_or(DecodeError::UnknownInstruction { word })?;
    if class == InstructionClass::Ldi {
        return Ok(DecodedInstruction {
            class,
            destination,
            condition: Condition::Always,
            operand_b: OperandB::Immediate(sign_extend(word & 0x007f_ffff, 23)),
            destination_user: false,
            source_user: false,
            raw: word,
        });
    }
    let condition = Condition::from_bits(((word >> 19) & 7) as u8);
    if class == InstructionClass::Mov {
        return Ok(DecodedInstruction {
            class,
            destination,
            condition,
            operand_b: OperandB::Register {
                register: ((word >> 14) & 0x0f) as u8,
                immediate: sign_extend(word & 0x1fff, 13),
            },
            destination_user: word & (1 << 18) != 0,
            source_user: word & (1 << 13) != 0,
            raw: word,
        });
    }
    let operand_b = if word & (1 << 18) == 0 {
        OperandB::Immediate(sign_extend(word & 0x3_ffff, 18))
    } else {
        OperandB::Register {
            register: ((word >> 14) & 0x0f) as u8,
            immediate: sign_extend(word & 0x3fff, 14),
        }
    };
    Ok(DecodedInstruction {
        class,
        destination,
        condition,
        operand_b,
        destination_user: false,
        source_user: false,
        raw: word,
    })
}

impl DecodedInstruction {
    fn special(class: InstructionClass, raw: u32) -> Self {
        Self {
            class,
            destination: ((raw >> 27) & 0x0f) as u8,
            condition: Condition::Always,
            operand_b: OperandB::Immediate(0),
            destination_user: false,
            source_user: false,
            raw,
        }
    }
}

fn class_from_opcode(opcode: u8) -> Option<InstructionClass> {
    Some(match opcode {
        0x00 => InstructionClass::Sub,
        0x01 => InstructionClass::And,
        0x02 => InstructionClass::Add,
        0x03 => InstructionClass::Or,
        0x04 => InstructionClass::Xor,
        0x05 => InstructionClass::Lsr,
        0x06 => InstructionClass::Lsl,
        0x07 => InstructionClass::Asr,
        0x08 => InstructionClass::Brev,
        0x09 => InstructionClass::Ldilo,
        0x0a => InstructionClass::Mpyuhi,
        0x0b => InstructionClass::Mpyshi,
        0x0c => InstructionClass::Mpy,
        0x0d => InstructionClass::Mov,
        0x0e => InstructionClass::Divu,
        0x0f => InstructionClass::Divs,
        0x10 => InstructionClass::Cmp,
        0x11 => InstructionClass::Tst,
        0x12 => InstructionClass::Lw,
        0x13 => InstructionClass::Sw,
        0x14 => InstructionClass::Lh,
        0x15 => InstructionClass::Sh,
        0x16 => InstructionClass::Lb,
        0x17 => InstructionClass::Sb,
        0x18 | 0x19 => InstructionClass::Ldi,
        _ => return None,
    })
}

const fn sign_extend(value: u32, bits: u32) -> i32 {
    ((value << (32 - bits)) as i32) >> (32 - bits)
}

/// Direction of one explicit bus transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryDirection {
    Read,
    Write,
}

/// Purpose of one explicit bus transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryPurpose {
    Instruction,
    Data,
}

/// Exact ordered bus observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryTransaction {
    pub purpose: MemoryPurpose,
    pub direction: MemoryDirection,
    pub address: u32,
    pub width: u8,
    pub value: u32,
    pub byte_enable: u8,
    pub success: bool,
}

/// One captured architectural register input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisterAccess {
    pub bank: Bank,
    pub register: u8,
    pub value: u32,
}

/// One committed effect, retained in execution order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    RegisterWrite {
        bank: Bank,
        register: u8,
        value: u32,
    },
    FlagsWrite {
        bank: Bank,
        flags: Flags,
    },
    ModeWrite {
        mode: Mode,
    },
    MemoryWrite {
        address: u32,
        width: u8,
        value: u32,
    },
}

/// Architectural interrupt/fault cause.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultCause {
    IllegalInstruction,
    IllegalFetch,
    DataBusError,
    DivideByZero,
    Break,
    SoftwareTrap,
}

/// Successful terminal boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Running,
    Halted,
    WaitingForInterrupt,
    ExternalBreak,
    Interrupt,
    FaultToSupervisor(FaultCause),
    UserStep,
}

/// One exact committed instruction-boundary transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub sequential_pc: u32,
    pub selected_pc: u32,
    pub condition_met: bool,
    pub sources: Vec<RegisterAccess>,
    pub destination: Option<RegisterAccess>,
    pub flags_before: Flags,
    pub flags_after: Flags,
    pub supervisor_cc_before: u32,
    pub supervisor_cc_after: u32,
    pub user_cc_before: u32,
    pub user_cc_after: u32,
    pub mode_before: Mode,
    pub mode_after: Mode,
    pub raw_bytes: Vec<u8>,
    pub raw_word: u32,
    pub mnemonic: String,
    pub transactions: Vec<MemoryTransaction>,
    pub effects: Vec<Effect>,
    pub cause: Option<FaultCause>,
    pub stop_reason: StopReason,
    pub state_after: ZipCpuState,
}

/// Successful bounded run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub stop_reason: StopReason,
    pub final_state: ZipCpuState,
}

/// Stable checked-lifecycle failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZipCpuError {
    EmptyProgram,
    MisalignedProgram { origin: u32, length: usize },
    ProgramTooLarge { length: usize },
    ProgramRangeOverflow { origin: u32, length: usize },
    ProgramOutOfRange { origin: u32, length: usize },
    NoProgramLoaded,
    InvalidState { reason: &'static str },
    FetchOutsideProgram { pc: u32 },
    MisalignedData { address: u32, width: u8 },
    DataOutOfRange { address: u32, width: u8 },
    Halted,
    WaitingForInterrupt,
    TraceLimitExceeded { requested: usize, maximum: usize },
    ZeroStepBudget,
    StepLimitExceeded { maximum: usize },
    AllocationFailed,
}

/// Named production allocation boundary used by deterministic failure tests.
///
/// Each variant immediately precedes the real fallible allocation named by
/// the variant. The seam selects an occurrence of one boundary, so tests can
/// reach the same boundary after one or more successful candidate steps.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocationPoint {
    SnapshotState,
    RestoreStaging,
    LoadImage,
    StepEntryState,
    StepCandidateState,
    FetchedBytes,
    Mnemonic,
    Sources,
    Transactions,
    Effects,
    TraceState,
    RunEntryState,
    RunTraceRetention,
    RunFinalState,
    ExecuteEntryState,
    ExecuteWrapping,
}

#[derive(Clone, Copy, Debug)]
struct AllocationPlan {
    point: AllocationPoint,
    remaining_matches: usize,
}

impl fmt::Display for ZipCpuError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => formatter.write_str("program image is empty"),
            Self::MisalignedProgram { origin, length } => write!(
                formatter,
                "program at {origin:#010x} with length {length} is not word aligned"
            ),
            Self::ProgramTooLarge { length } => {
                write!(formatter, "program length {length} exceeds 65536 bytes")
            }
            Self::ProgramRangeOverflow { origin, length } => write!(
                formatter,
                "program at {origin:#010x} with length {length} wraps the address space"
            ),
            Self::ProgramOutOfRange { origin, length } => write!(
                formatter,
                "program at {origin:#010x} with length {length} exceeds owned memory"
            ),
            Self::NoProgramLoaded => formatter.write_str("no program image is installed"),
            Self::InvalidState { reason } => write!(formatter, "invalid ZipCPU state: {reason}"),
            Self::FetchOutsideProgram { pc } => write!(
                formatter,
                "instruction fetch at {pc:#010x} is outside the installed image"
            ),
            Self::MisalignedData { address, width } => write!(
                formatter,
                "misaligned {width}-byte data access at {address:#010x}"
            ),
            Self::DataOutOfRange { address, width } => write!(
                formatter,
                "{width}-byte data access at {address:#010x} is outside owned memory"
            ),
            Self::Halted => formatter.write_str("processor is halted"),
            Self::WaitingForInterrupt => formatter.write_str("processor is waiting for interrupt"),
            Self::TraceLimitExceeded { requested, maximum } => write!(
                formatter,
                "requested {requested} traces exceeds retention limit {maximum}"
            ),
            Self::ZeroStepBudget => formatter.write_str("step budget must be nonzero"),
            Self::StepLimitExceeded { maximum } => write!(
                formatter,
                "execution did not reach a terminal boundary within {maximum} steps"
            ),
            Self::AllocationFailed => formatter.write_str("checked allocation failed"),
        }
    }
}

impl Error for ZipCpuError {}

/// Source-preserving atomic execute failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionError {
    pub source: ZipCpuError,
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ZipCPU execution failed: {}", self.source)
    }
}

impl Error for ExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

/// Checked bounded functional machine.
#[derive(Debug)]
pub struct ZipCpuSimulator {
    state: ZipCpuState,
    allocation_failure: Cell<Option<AllocationPlan>>,
}

impl ZipCpuSimulator {
    /// Construct a deterministic reset machine using a fallible memory allocation.
    pub fn new() -> Result<Self, ZipCpuError> {
        Ok(Self {
            state: reset_state(allocate_memory()?),
            allocation_failure: Cell::new(None),
        })
    }

    /// Borrow the complete authoritative state without allocating.
    #[must_use]
    pub fn state(&self) -> &ZipCpuState {
        &self.state
    }

    /// Return a checked complete-state snapshot.
    pub fn snapshot(&self) -> Result<ZipCpuState, ZipCpuError> {
        self.checked_clone_state(&self.state, AllocationPoint::SnapshotState)
    }

    /// Atomically validate and restore a complete state.
    pub fn restore(&mut self, state: &ZipCpuState) -> Result<(), ZipCpuError> {
        validate_state(state)?;
        let candidate = self.checked_clone_state(state, AllocationPoint::RestoreStaging)?;
        self.state = candidate;
        Ok(())
    }

    /// Reset CPU-visible state while preserving memory and installed image metadata.
    pub fn reset(&mut self) {
        let origin = self.state.image_origin;
        let length = self.state.image_length;
        self.state.supervisor_gpr = [0; 14];
        self.state.user_gpr = [0; 14];
        self.state.supervisor_cc = CC_CPU_INFO;
        self.state.user_cc = CC_CPU_INFO | CC_GIE;
        self.state.supervisor_pc = 0;
        self.state.user_pc = 0;
        self.state.mode = Mode::Supervisor;
        self.state.interrupt_asserted = false;
        self.state.image_origin = origin;
        self.state.image_length = length;
    }

    /// Atomically install a raw big-endian image and deterministic load state.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), ZipCpuError> {
        validate_program(program, origin)?;
        self.allocation_checkpoint(AllocationPoint::LoadImage)?;
        let mut memory = allocate_memory()?;
        let start = origin as usize;
        memory[start..start + program.len()].copy_from_slice(program);
        let mut candidate = reset_state(memory);
        candidate.supervisor_pc = origin;
        candidate.image_origin = Some(origin);
        candidate.image_length = program.len() as u32;
        self.state = candidate;
        Ok(())
    }

    /// Atomically set the sampled external interrupt input.
    pub fn set_interrupt(&mut self, asserted: bool) -> Result<(), ZipCpuError> {
        validate_state(&self.state)?;
        self.state.interrupt_asserted = asserted;
        Ok(())
    }

    /// Read a retained register value. Active-PC pipeline adjustment is applied
    /// only during instruction operand capture, never by this state observer.
    pub fn read_register(&self, bank: Bank, register: u8) -> Result<u32, ZipCpuError> {
        validate_state(&self.state)?;
        read_register_stored(&self.state, bank, register).ok_or(ZipCpuError::InvalidState {
            reason: "register selector must be in 0..15",
        })
    }

    /// Read one checked data byte.
    pub fn read_byte(&self, address: u32) -> Result<u8, ZipCpuError> {
        validate_data_span(address, 1)?;
        Ok(self.state.memory[address as usize])
    }

    /// Read one checked big-endian halfword.
    pub fn read_halfword(&self, address: u32) -> Result<u16, ZipCpuError> {
        validate_data_span(address, 2)?;
        let start = address as usize;
        Ok(u16::from_be_bytes([
            self.state.memory[start],
            self.state.memory[start + 1],
        ]))
    }

    /// Read one checked big-endian word.
    pub fn read_word(&self, address: u32) -> Result<u32, ZipCpuError> {
        validate_data_span(address, 4)?;
        let start = address as usize;
        Ok(u32::from_be_bytes(
            self.state.memory[start..start + 4]
                .try_into()
                .expect("validated four-byte span"),
        ))
    }

    /// Atomically write one checked data byte.
    pub fn write_byte(&mut self, address: u32, value: u8) -> Result<(), ZipCpuError> {
        validate_data_span(address, 1)?;
        self.state.memory[address as usize] = value;
        Ok(())
    }

    /// Atomically write one checked big-endian halfword.
    pub fn write_halfword(&mut self, address: u32, value: u16) -> Result<(), ZipCpuError> {
        validate_data_span(address, 2)?;
        let start = address as usize;
        self.state.memory[start..start + 2].copy_from_slice(&value.to_be_bytes());
        Ok(())
    }

    /// Atomically write one checked big-endian word.
    pub fn write_word(&mut self, address: u32, value: u32) -> Result<(), ZipCpuError> {
        validate_data_span(address, 4)?;
        let start = address as usize;
        self.state.memory[start..start + 4].copy_from_slice(&value.to_be_bytes());
        Ok(())
    }

    /// Install a deterministic failure at one named real allocation boundary.
    ///
    /// `occurrence` is zero-based among visits to the selected point.
    #[doc(hidden)]
    pub fn fail_allocation_at(&self, point: AllocationPoint, occurrence: usize) {
        self.allocation_failure.set(Some(AllocationPlan {
            point,
            remaining_matches: occurrence,
        }));
    }

    /// Remove any installed deterministic allocation failure.
    #[doc(hidden)]
    pub fn clear_allocation_failure(&self) {
        self.allocation_failure.set(None);
    }

    fn allocation_checkpoint(&self, point: AllocationPoint) -> Result<(), ZipCpuError> {
        if let Some(mut plan) = self.allocation_failure.get() {
            if plan.point != point {
                return Ok(());
            }
            if plan.remaining_matches == 0 {
                self.allocation_failure.set(None);
                return Err(ZipCpuError::AllocationFailed);
            }
            plan.remaining_matches -= 1;
            self.allocation_failure.set(Some(plan));
        }
        Ok(())
    }

    fn checked_clone_state(
        &self,
        source: &ZipCpuState,
        point: AllocationPoint,
    ) -> Result<ZipCpuState, ZipCpuError> {
        self.allocation_checkpoint(point)?;
        let mut memory = allocate_memory()?;
        memory.copy_from_slice(&source.memory);
        Ok(copy_state_with_memory(source, memory))
    }

    /// Execute one atomic instruction-boundary transition.
    pub fn step(&mut self) -> Result<StepTrace, ZipCpuError> {
        validate_state(&self.state)?;
        let before = self.checked_clone_state(&self.state, AllocationPoint::StepEntryState)?;
        let mut candidate =
            self.checked_clone_state(&self.state, AllocationPoint::StepCandidateState)?;

        if before.mode == Mode::User && before.interrupt_asserted {
            set_shared(&mut candidate, CC_SLEEP, false);
            candidate.mode = Mode::Supervisor;
            let mut transition = Transition::environment("INTERRUPT", StopReason::Interrupt);
            transition.pc_before = active_pc(&before);
            transition.sequential_pc = active_pc(&before);
            return self.finish_transition(&before, candidate, transition);
        }
        if cc_stored(&before, Bank::Supervisor) & CC_SLEEP != 0 && before.mode == Mode::Supervisor {
            return Err(ZipCpuError::Halted);
        }
        if cc_stored(&before, Bank::User) & CC_SLEEP != 0 && before.mode == Mode::User {
            return Err(ZipCpuError::WaitingForInterrupt);
        }

        let pc = active_pc(&before);
        let Some(origin) = before.image_origin else {
            return Err(ZipCpuError::NoProgramLoaded);
        };
        let image_end = u64::from(origin) + u64::from(before.image_length);
        let fetch_end = u64::from(pc) + 4;
        if usize::try_from(pc).map_or(true, |start| start > MEMORY_SIZE.saturating_sub(4)) {
            let mut transition = Transition::environment(
                "ILLEGAL-FETCH",
                StopReason::FaultToSupervisor(FaultCause::IllegalFetch),
            );
            transition.pc_before = pc;
            transition.sequential_pc = pc.wrapping_add(4);
            transition.cause = Some(FaultCause::IllegalFetch);
            transition.transactions.push(MemoryTransaction {
                purpose: MemoryPurpose::Instruction,
                direction: MemoryDirection::Read,
                address: pc,
                width: 4,
                value: 0,
                byte_enable: 0x0f,
                success: false,
            });
            apply_fault(&mut candidate, FaultCause::IllegalFetch);
            transition.stop_reason = fault_stop(&before, FaultCause::IllegalFetch);
            return self.finish_transition(&before, candidate, transition);
        }
        if u64::from(pc) < u64::from(origin) || fetch_end > image_end {
            return Err(ZipCpuError::FetchOutsideProgram { pc });
        }
        let start = pc as usize;
        let bytes: [u8; 4] = before.memory[start..start + 4]
            .try_into()
            .expect("validated instruction span");
        let word = u32::from_be_bytes(bytes);
        let mut transition = Transition::instruction(pc, bytes, word);
        transition.transactions.push(MemoryTransaction {
            purpose: MemoryPurpose::Instruction,
            direction: MemoryDirection::Read,
            address: pc,
            width: 4,
            value: word,
            byte_enable: 0x0f,
            success: true,
        });

        let decoded = match decode(word) {
            Ok(decoded) => decoded,
            Err(_) => {
                transition.mnemonic = "ILLEGAL";
                transition.cause = Some(FaultCause::IllegalInstruction);
                transition.stop_reason = fault_stop(&before, FaultCause::IllegalInstruction);
                apply_fault(&mut candidate, FaultCause::IllegalInstruction);
                return self.finish_transition(&before, candidate, transition);
            }
        };
        transition.mnemonic = decoded.class.mnemonic();
        transition.condition = decoded.condition;
        transition.condition_met = decoded
            .condition
            .holds(flags_for_bank(&before, before.mode.bank()));
        if !transition.condition_met {
            set_active_pc(&mut candidate, pc.wrapping_add(4));
            return self.finish_transition(&before, candidate, transition);
        }

        if decoded.class == InstructionClass::Break {
            transition.cause = Some(FaultCause::Break);
            transition.stop_reason = fault_stop(&before, FaultCause::Break);
            apply_fault(&mut candidate, FaultCause::Break);
            return self.finish_transition(&before, candidate, transition);
        }
        set_active_pc(&mut candidate, pc.wrapping_add(4));
        if decoded.class != InstructionClass::Noop {
            execute_decoded(&before, &mut candidate, decoded, &mut transition)?;
        }

        if transition.cause.is_none() && before.mode == Mode::User {
            if candidate.mode == Mode::User && before.user_cc & CC_STEP != 0 {
                candidate.mode = Mode::Supervisor;
                transition.stop_reason = StopReason::UserStep;
            } else if candidate.mode == Mode::Supervisor {
                transition.cause = Some(FaultCause::SoftwareTrap);
                transition.stop_reason = StopReason::FaultToSupervisor(FaultCause::SoftwareTrap);
            }
        }
        if transition.stop_reason == StopReason::Running {
            if candidate.mode == Mode::Supervisor && candidate.supervisor_cc & CC_SLEEP != 0 {
                transition.stop_reason = StopReason::Halted;
            } else if candidate.mode == Mode::User && candidate.user_cc & CC_SLEEP != 0 {
                transition.stop_reason = StopReason::WaitingForInterrupt;
            }
        }
        self.finish_transition(&before, candidate, transition)
    }

    fn finish_transition(
        &mut self,
        before: &ZipCpuState,
        candidate: ZipCpuState,
        mut transition: Transition,
    ) -> Result<StepTrace, ZipCpuError> {
        validate_state(&candidate)?;
        if before.mode != candidate.mode {
            transition.effects.push(Effect::ModeWrite {
                mode: candidate.mode,
            });
        }
        self.allocation_checkpoint(AllocationPoint::FetchedBytes)?;
        let mut raw_bytes = Vec::new();
        raw_bytes
            .try_reserve_exact(transition.raw_len)
            .map_err(|_| ZipCpuError::AllocationFailed)?;
        raw_bytes.extend_from_slice(&transition.raw[..transition.raw_len]);

        self.allocation_checkpoint(AllocationPoint::Mnemonic)?;
        let mut mnemonic = String::new();
        mnemonic
            .try_reserve_exact(transition.mnemonic.len() + transition.condition.suffix().len())
            .map_err(|_| ZipCpuError::AllocationFailed)?;
        mnemonic.push_str(transition.mnemonic);
        if transition.condition_met || transition.condition != Condition::Always {
            mnemonic.push_str(transition.condition.suffix());
        }

        self.allocation_checkpoint(AllocationPoint::Sources)?;
        let mut sources = Vec::new();
        sources
            .try_reserve_exact(transition.sources.len())
            .map_err(|_| ZipCpuError::AllocationFailed)?;
        sources.extend(transition.sources);

        self.allocation_checkpoint(AllocationPoint::Transactions)?;
        let mut transactions = Vec::new();
        transactions
            .try_reserve_exact(transition.transactions.len())
            .map_err(|_| ZipCpuError::AllocationFailed)?;
        transactions.extend(transition.transactions);

        self.allocation_checkpoint(AllocationPoint::Effects)?;
        let mut effects = Vec::new();
        effects
            .try_reserve_exact(transition.effects.len())
            .map_err(|_| ZipCpuError::AllocationFailed)?;
        effects.extend(transition.effects);

        let state_after = self.checked_clone_state(&candidate, AllocationPoint::TraceState)?;
        let trace = StepTrace {
            pc_before: transition.pc_before,
            sequential_pc: transition.sequential_pc,
            selected_pc: active_pc(&candidate),
            condition_met: transition.condition_met,
            sources,
            destination: transition.destination,
            flags_before: flags_for_bank(before, before.mode.bank()),
            flags_after: flags_for_bank(&candidate, candidate.mode.bank()),
            supervisor_cc_before: before.supervisor_cc,
            supervisor_cc_after: candidate.supervisor_cc,
            user_cc_before: before.user_cc,
            user_cc_after: candidate.user_cc,
            mode_before: before.mode,
            mode_after: candidate.mode,
            raw_bytes,
            raw_word: transition.raw_word,
            mnemonic,
            transactions,
            effects,
            cause: transition.cause,
            stop_reason: transition.stop_reason,
            state_after,
        };
        self.state = candidate;
        Ok(trace)
    }

    /// Run atomically until a terminal architectural boundary.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, ZipCpuError> {
        if max_steps == 0 {
            return Err(ZipCpuError::ZeroStepBudget);
        }
        if max_steps > MAX_RETAINED_STEPS {
            return Err(ZipCpuError::TraceLimitExceeded {
                requested: max_steps,
                maximum: MAX_RETAINED_STEPS,
            });
        }
        let rollback = self.checked_clone_state(&self.state, AllocationPoint::RunEntryState)?;
        self.allocation_checkpoint(AllocationPoint::RunTraceRetention)?;
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| ZipCpuError::AllocationFailed)?;
        for _ in 0..max_steps {
            let trace = match self.step() {
                Ok(trace) => trace,
                Err(error) => {
                    self.state = rollback;
                    return Err(error);
                }
            };
            let stop_reason = trace.stop_reason;
            traces.push(trace);
            if stop_reason != StopReason::Running {
                let final_state =
                    match self.checked_clone_state(&self.state, AllocationPoint::RunFinalState) {
                        Ok(state) => state,
                        Err(error) => {
                            self.state = rollback;
                            return Err(error);
                        }
                    };
                return Ok(ExecutionResult {
                    steps: traces.len(),
                    traces,
                    stop_reason,
                    final_state,
                });
            }
        }
        self.state = rollback;
        Err(ZipCpuError::StepLimitExceeded { maximum: max_steps })
    }

    /// Atomically load and run one bounded image.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, ExecutionError> {
        validate_program(program, origin).map_err(|source| ExecutionError { source })?;
        let rollback = self
            .checked_clone_state(&self.state, AllocationPoint::ExecuteEntryState)
            .map_err(|source| ExecutionError { source })?;
        if let Err(source) = self.load(program, origin) {
            self.state = rollback;
            return Err(ExecutionError { source });
        }
        let result = match self.run(max_steps) {
            Ok(result) => result,
            Err(source) => {
                self.state = rollback;
                return Err(ExecutionError { source });
            }
        };

        // The wrapper owns its returned trace vector rather than smuggling a
        // successful run allocation through an untested infallible clone.
        if let Err(source) = self.allocation_checkpoint(AllocationPoint::ExecuteWrapping) {
            self.state = rollback;
            return Err(ExecutionError { source });
        }
        let mut traces = Vec::new();
        if traces.try_reserve_exact(result.traces.len()).is_err() {
            self.state = rollback;
            return Err(ExecutionError {
                source: ZipCpuError::AllocationFailed,
            });
        }
        traces.extend(result.traces);
        Ok(ExecutionResult { traces, ..result })
    }
}

#[derive(Debug)]
struct FixedList<T, const N: usize> {
    entries: [Option<T>; N],
    len: usize,
}

impl<T, const N: usize> FixedList<T, N> {
    fn new() -> Self {
        Self {
            entries: std::array::from_fn(|_| None),
            len: 0,
        }
    }

    fn push(&mut self, value: T) {
        assert!(self.len < N, "fixed transition metadata capacity");
        self.entries[self.len] = Some(value);
        self.len += 1;
    }

    const fn len(&self) -> usize {
        self.len
    }
}

impl<T, const N: usize> IntoIterator for FixedList<T, N> {
    type Item = T;
    type IntoIter = std::iter::Flatten<std::array::IntoIter<Option<T>, N>>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter().flatten()
    }
}

struct Transition {
    pc_before: u32,
    sequential_pc: u32,
    condition: Condition,
    condition_met: bool,
    raw: [u8; 4],
    raw_len: usize,
    raw_word: u32,
    mnemonic: &'static str,
    sources: FixedList<RegisterAccess, 2>,
    destination: Option<RegisterAccess>,
    transactions: FixedList<MemoryTransaction, 2>,
    effects: FixedList<Effect, 5>,
    cause: Option<FaultCause>,
    stop_reason: StopReason,
}

impl Transition {
    fn instruction(pc: u32, raw: [u8; 4], raw_word: u32) -> Self {
        Self {
            pc_before: pc,
            sequential_pc: pc.wrapping_add(4),
            condition: Condition::Always,
            condition_met: true,
            raw,
            raw_len: 4,
            raw_word,
            mnemonic: "",
            sources: FixedList::new(),
            destination: None,
            transactions: FixedList::new(),
            effects: FixedList::new(),
            cause: None,
            stop_reason: StopReason::Running,
        }
    }

    fn environment(mnemonic: &'static str, stop_reason: StopReason) -> Self {
        Self {
            pc_before: 0,
            sequential_pc: 0,
            condition: Condition::Always,
            condition_met: true,
            raw: [0; 4],
            raw_len: 0,
            raw_word: 0,
            mnemonic,
            sources: FixedList::new(),
            destination: None,
            transactions: FixedList::new(),
            effects: FixedList::new(),
            cause: None,
            stop_reason,
        }
    }
}

fn execute_decoded(
    before: &ZipCpuState,
    candidate: &mut ZipCpuState,
    decoded: DecodedInstruction,
    transition: &mut Transition,
) -> Result<(), ZipCpuError> {
    let active_bank = before.mode.bank();
    let destination_bank = if decoded.class == InstructionClass::Mov
        && before.mode == Mode::Supervisor
        && decoded.destination_user
    {
        Bank::User
    } else {
        active_bank
    };
    let source_bank = if decoded.class == InstructionClass::Mov
        && before.mode == Mode::Supervisor
        && decoded.source_user
    {
        Bank::User
    } else {
        active_bank
    };

    let reads_a = !matches!(
        decoded.class,
        InstructionClass::Brev
            | InstructionClass::Mov
            | InstructionClass::Ldi
            | InstructionClass::Lw
            | InstructionClass::Lh
            | InstructionClass::Lb
    );
    let a = if reads_a {
        let value = read_operand_register(before, active_bank, decoded.destination);
        transition.sources.push(RegisterAccess {
            bank: active_bank,
            register: decoded.destination,
            value,
        });
        value
    } else {
        0
    };
    let b = match decoded.operand_b {
        OperandB::Immediate(value) => value as u32,
        OperandB::Register {
            register,
            immediate,
        } => {
            let value = read_operand_register(before, source_bank, register);
            transition.sources.push(RegisterAccess {
                bank: source_bank,
                register,
                value,
            });
            let offset = if register == 15 {
                immediate.wrapping_mul(4) as u32
            } else {
                immediate as u32
            };
            value.wrapping_add(offset)
        }
    };

    if matches!(
        decoded.class,
        InstructionClass::Lw
            | InstructionClass::Sw
            | InstructionClass::Lh
            | InstructionClass::Sh
            | InstructionClass::Lb
            | InstructionClass::Sb
    ) {
        return execute_memory(
            before,
            candidate,
            decoded,
            destination_bank,
            a,
            b,
            transition,
        );
    }

    let (result, flags) = match decoded.class {
        InstructionClass::Sub | InstructionClass::Cmp => arithmetic_sub(a, b),
        InstructionClass::And | InstructionClass::Tst => logic_result(a & b),
        InstructionClass::Add => arithmetic_add(a, b),
        InstructionClass::Or => logic_result(a | b),
        InstructionClass::Xor => logic_result(a ^ b),
        InstructionClass::Lsr => logical_shift_right(a, b),
        InstructionClass::Lsl => logical_shift_left(a, b),
        InstructionClass::Asr => arithmetic_shift_right(a, b),
        InstructionClass::Brev => (b.reverse_bits(), Flags::from_cc(0)),
        InstructionClass::Ldilo => ((a & 0xffff_0000) | (b & 0xffff), Flags::from_cc(0)),
        InstructionClass::Mpyuhi => {
            let result = ((u64::from(a) * u64::from(b)) >> 32) as u32;
            (result, logic_result(result).1)
        }
        InstructionClass::Mpyshi => {
            let product = i64::from(a as i32) * i64::from(b as i32);
            let result = (product >> 32) as u32;
            (result, logic_result(result).1)
        }
        InstructionClass::Mpy => {
            let result = a.wrapping_mul(b);
            (result, logic_result(result).1)
        }
        InstructionClass::Mov | InstructionClass::Ldi => (b, Flags::from_cc(0)),
        InstructionClass::Divu => {
            if b == 0 {
                return apply_execution_fault(
                    before,
                    candidate,
                    transition,
                    FaultCause::DivideByZero,
                );
            }
            let result = a / b;
            (
                result,
                Flags {
                    zero: result == 0,
                    carry: a % b == 0,
                    negative: result & 0x8000_0000 != 0,
                    overflow: false,
                },
            )
        }
        InstructionClass::Divs => {
            if b == 0 {
                return apply_execution_fault(
                    before,
                    candidate,
                    transition,
                    FaultCause::DivideByZero,
                );
            }
            let lhs = a as i32;
            let rhs = b as i32;
            let (quotient, overflowed) = lhs.overflowing_div(rhs);
            let remainder_zero = overflowed || lhs % rhs == 0;
            let result = quotient as u32;
            (
                result,
                Flags {
                    zero: result == 0,
                    carry: remainder_zero,
                    negative: result & 0x8000_0000 != 0,
                    overflow: false,
                },
            )
        }
        InstructionClass::Noop | InstructionClass::Break => return Ok(()),
        InstructionClass::Lw
        | InstructionClass::Sw
        | InstructionClass::Lh
        | InstructionClass::Sh
        | InstructionClass::Lb
        | InstructionClass::Sb => unreachable!("memory handled above"),
    };

    let writes_result = !matches!(decoded.class, InstructionClass::Cmp | InstructionClass::Tst);
    if writes_result {
        write_register(
            candidate,
            destination_bank,
            decoded.destination,
            result,
            before.mode,
        );
        let observed = read_register_stored(candidate, destination_bank, decoded.destination)
            .expect("decoded destination");
        let destination = RegisterAccess {
            bank: destination_bank,
            register: decoded.destination,
            value: observed,
        };
        transition.destination = Some(destination.clone());
        transition.effects.push(Effect::RegisterWrite {
            bank: destination_bank,
            register: decoded.destination,
            value: observed,
        });
    }
    let flag_class = !matches!(
        decoded.class,
        InstructionClass::Brev
            | InstructionClass::Ldilo
            | InstructionClass::Mov
            | InstructionClass::Ldi
    );
    let condition_allows_flags = decoded.condition == Condition::Always
        || matches!(decoded.class, InstructionClass::Cmp | InstructionClass::Tst);
    if flag_class && condition_allows_flags && !matches!(decoded.destination, 14 | 15) {
        write_flags(candidate, active_bank, flags);
        transition.effects.push(Effect::FlagsWrite {
            bank: active_bank,
            flags,
        });
    }
    Ok(())
}

fn execute_memory(
    before: &ZipCpuState,
    candidate: &mut ZipCpuState,
    decoded: DecodedInstruction,
    destination_bank: Bank,
    a: u32,
    address: u32,
    transition: &mut Transition,
) -> Result<(), ZipCpuError> {
    let (width, direction) = match decoded.class {
        InstructionClass::Lw => (4, MemoryDirection::Read),
        InstructionClass::Sw => (4, MemoryDirection::Write),
        InstructionClass::Lh => (2, MemoryDirection::Read),
        InstructionClass::Sh => (2, MemoryDirection::Write),
        InstructionClass::Lb => (1, MemoryDirection::Read),
        InstructionClass::Sb => (1, MemoryDirection::Write),
        _ => unreachable!("memory class"),
    };
    let request_value = match decoded.class {
        InstructionClass::Sw => a,
        InstructionClass::Sh => u32::from(a as u16),
        InstructionClass::Sb => u32::from(a as u8),
        _ => 0,
    };
    if validate_data_span(address, width).is_err() {
        transition.transactions.push(MemoryTransaction {
            purpose: MemoryPurpose::Data,
            direction,
            address,
            width,
            value: request_value,
            byte_enable: byte_enable(address, width),
            success: false,
        });
        return apply_execution_fault(before, candidate, transition, FaultCause::DataBusError);
    }
    let start = address as usize;
    let value = match decoded.class {
        InstructionClass::Lw => u32::from_be_bytes(
            before.memory[start..start + 4]
                .try_into()
                .expect("validated word"),
        ),
        InstructionClass::Lh => u32::from(u16::from_be_bytes([
            before.memory[start],
            before.memory[start + 1],
        ])),
        InstructionClass::Lb => u32::from(before.memory[start]),
        InstructionClass::Sw => {
            candidate.memory[start..start + 4].copy_from_slice(&a.to_be_bytes());
            a
        }
        InstructionClass::Sh => {
            let bytes = (a as u16).to_be_bytes();
            candidate.memory[start..start + 2].copy_from_slice(&bytes);
            u32::from(a as u16)
        }
        InstructionClass::Sb => {
            candidate.memory[start] = a as u8;
            u32::from(a as u8)
        }
        _ => unreachable!("memory class"),
    };
    transition.transactions.push(MemoryTransaction {
        purpose: MemoryPurpose::Data,
        direction,
        address,
        width,
        value,
        byte_enable: byte_enable(address, width),
        success: true,
    });
    if direction == MemoryDirection::Read {
        write_register(
            candidate,
            destination_bank,
            decoded.destination,
            value,
            before.mode,
        );
        let observed = read_register_stored(candidate, destination_bank, decoded.destination)
            .expect("decoded destination");
        transition.destination = Some(RegisterAccess {
            bank: destination_bank,
            register: decoded.destination,
            value: observed,
        });
        transition.effects.push(Effect::RegisterWrite {
            bank: destination_bank,
            register: decoded.destination,
            value: observed,
        });
    } else {
        transition.effects.push(Effect::MemoryWrite {
            address,
            width,
            value,
        });
    }
    Ok(())
}

fn apply_execution_fault(
    before: &ZipCpuState,
    candidate: &mut ZipCpuState,
    transition: &mut Transition,
    cause: FaultCause,
) -> Result<(), ZipCpuError> {
    *candidate = copy_state_with_memory(before, std::mem::take(&mut candidate.memory));
    candidate.memory.copy_from_slice(&before.memory);
    apply_fault(candidate, cause);
    transition.cause = Some(cause);
    transition.stop_reason = fault_stop(before, cause);
    transition.destination = None;
    transition.effects = FixedList::new();
    Ok(())
}

fn arithmetic_add(a: u32, b: u32) -> (u32, Flags) {
    let (result, carry) = a.overflowing_add(b);
    let overflow = (!(a ^ b) & (a ^ result) & 0x8000_0000) != 0;
    (
        result,
        Flags {
            zero: result == 0,
            carry,
            negative: (result & 0x8000_0000 != 0) ^ overflow,
            overflow,
        },
    )
}

fn arithmetic_sub(a: u32, b: u32) -> (u32, Flags) {
    let result = a.wrapping_sub(b);
    let overflow = ((a ^ b) & (a ^ result) & 0x8000_0000) != 0;
    (
        result,
        Flags {
            zero: result == 0,
            carry: a < b,
            negative: (result & 0x8000_0000 != 0) ^ overflow,
            overflow,
        },
    )
}

fn logic_result(result: u32) -> (u32, Flags) {
    (
        result,
        Flags {
            zero: result == 0,
            carry: false,
            negative: result & 0x8000_0000 != 0,
            overflow: false,
        },
    )
}

fn logical_shift_right(a: u32, count: u32) -> (u32, Flags) {
    let result = if count >= 32 { 0 } else { a >> count };
    let carry = match count {
        0 => false,
        1..=32 => (a >> (count - 1)) & 1 != 0,
        _ => false,
    };
    let overflow = (a ^ result) & 0x8000_0000 != 0;
    (
        result,
        Flags {
            zero: result == 0,
            carry,
            negative: result & 0x8000_0000 != 0,
            overflow,
        },
    )
}

fn logical_shift_left(a: u32, count: u32) -> (u32, Flags) {
    let result = if count >= 32 { 0 } else { a << count };
    let carry = match count {
        0 => false,
        1..=32 => (a >> (32 - count)) & 1 != 0,
        _ => false,
    };
    let overflow = (a ^ result) & 0x8000_0000 != 0;
    (
        result,
        Flags {
            zero: result == 0,
            carry,
            negative: result & 0x8000_0000 != 0,
            overflow,
        },
    )
}

fn arithmetic_shift_right(a: u32, count: u32) -> (u32, Flags) {
    let result = if count >= 32 {
        if a & 0x8000_0000 != 0 {
            u32::MAX
        } else {
            0
        }
    } else {
        ((a as i32) >> count) as u32
    };
    let carry = match count {
        0 => false,
        1..=31 => (a >> (count - 1)) & 1 != 0,
        _ => a & 0x8000_0000 != 0,
    };
    (
        result,
        Flags {
            zero: result == 0,
            carry,
            negative: result & 0x8000_0000 != 0,
            overflow: false,
        },
    )
}

fn allocate_memory() -> Result<Vec<u8>, ZipCpuError> {
    let mut memory = Vec::new();
    memory
        .try_reserve_exact(MEMORY_SIZE)
        .map_err(|_| ZipCpuError::AllocationFailed)?;
    memory.resize(MEMORY_SIZE, 0);
    Ok(memory)
}

fn reset_state(memory: Vec<u8>) -> ZipCpuState {
    ZipCpuState {
        supervisor_gpr: [0; 14],
        user_gpr: [0; 14],
        supervisor_cc: CC_CPU_INFO,
        user_cc: CC_CPU_INFO | CC_GIE,
        supervisor_pc: 0,
        user_pc: 0,
        mode: Mode::Supervisor,
        interrupt_asserted: false,
        memory,
        image_origin: None,
        image_length: 0,
    }
}

fn copy_state_with_memory(source: &ZipCpuState, memory: Vec<u8>) -> ZipCpuState {
    ZipCpuState {
        supervisor_gpr: source.supervisor_gpr,
        user_gpr: source.user_gpr,
        supervisor_cc: source.supervisor_cc,
        user_cc: source.user_cc,
        supervisor_pc: source.supervisor_pc,
        user_pc: source.user_pc,
        mode: source.mode,
        interrupt_asserted: source.interrupt_asserted,
        memory,
        image_origin: source.image_origin,
        image_length: source.image_length,
    }
}

fn validate_program(program: &[u8], origin: u32) -> Result<(), ZipCpuError> {
    if program.is_empty() {
        return Err(ZipCpuError::EmptyProgram);
    }
    if program.len() > MEMORY_SIZE {
        return Err(ZipCpuError::ProgramTooLarge {
            length: program.len(),
        });
    }
    if origin & 3 != 0 || program.len() & 3 != 0 {
        return Err(ZipCpuError::MisalignedProgram {
            origin,
            length: program.len(),
        });
    }
    let end = u64::from(origin) + program.len() as u64;
    if end > u64::from(u32::MAX) + 1 {
        return Err(ZipCpuError::ProgramRangeOverflow {
            origin,
            length: program.len(),
        });
    }
    if end > MEMORY_SIZE as u64 {
        return Err(ZipCpuError::ProgramOutOfRange {
            origin,
            length: program.len(),
        });
    }
    Ok(())
}

fn validate_state(state: &ZipCpuState) -> Result<(), ZipCpuError> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(ZipCpuError::InvalidState {
            reason: "memory must contain exactly 65536 bytes",
        });
    }
    if state.supervisor_pc & 3 != 0 || state.user_pc & 3 != 0 {
        return Err(ZipCpuError::InvalidState {
            reason: "both retained PCs must be word aligned",
        });
    }
    if state.supervisor_cc & 0xff80_0000 != CC_CPU_INFO
        || state.user_cc & 0xff80_0000 != CC_CPU_INFO
    {
        return Err(ZipCpuError::InvalidState {
            reason: "CC CPU-information bits do not match the fixed profile",
        });
    }
    if state.supervisor_cc & (CC_GIE | CC_STEP | CC_FPU_ERROR | 0xe000) != 0 {
        return Err(ZipCpuError::InvalidState {
            reason: "supervisor CC contains a noncanonical fixed/read-zero bit",
        });
    }
    if state.user_cc & CC_GIE == 0 || state.user_cc & (CC_FPU_ERROR | 0xe000) != 0 {
        return Err(ZipCpuError::InvalidState {
            reason: "user CC contains a noncanonical fixed/read-zero bit",
        });
    }
    if (state.supervisor_cc ^ state.user_cc) & (CC_SLEEP | CC_TRAP) != 0 {
        return Err(ZipCpuError::InvalidState {
            reason: "SLEEP and TRAP must be shared by both CC banks",
        });
    }
    match (state.image_origin, state.image_length) {
        (None, 0) => {}
        (None, _) => {
            return Err(ZipCpuError::InvalidState {
                reason: "an installed length requires an origin",
            });
        }
        (Some(_), 0) => {
            return Err(ZipCpuError::InvalidState {
                reason: "an installed origin requires a nonempty image",
            });
        }
        (Some(origin), length) => {
            if origin & 3 != 0 || length & 3 != 0 {
                return Err(ZipCpuError::InvalidState {
                    reason: "installed image origin and length must be word aligned",
                });
            }
            if u64::from(origin) + u64::from(length) > MEMORY_SIZE as u64 {
                return Err(ZipCpuError::InvalidState {
                    reason: "installed image must fit wholly inside owned memory",
                });
            }
        }
    }
    Ok(())
}

fn validate_data_span(address: u32, width: u8) -> Result<(), ZipCpuError> {
    let alignment = u32::from(width);
    if alignment > 1 && !address.is_multiple_of(alignment) {
        return Err(ZipCpuError::MisalignedData { address, width });
    }
    let end = u64::from(address) + u64::from(width);
    if end > MEMORY_SIZE as u64 {
        return Err(ZipCpuError::DataOutOfRange { address, width });
    }
    Ok(())
}

fn byte_enable(address: u32, width: u8) -> u8 {
    match width {
        4 => 0x0f,
        2 if address & 2 == 0 => 0x0c,
        2 => 0x03,
        1 => 1 << (3 - (address & 3)),
        _ => 0,
    }
}

fn cc_stored(state: &ZipCpuState, bank: Bank) -> u32 {
    match bank {
        Bank::Supervisor => state.supervisor_cc,
        Bank::User => state.user_cc,
    }
}

fn cc_stored_mut(state: &mut ZipCpuState, bank: Bank) -> &mut u32 {
    match bank {
        Bank::Supervisor => &mut state.supervisor_cc,
        Bank::User => &mut state.user_cc,
    }
}

fn cc_read(state: &ZipCpuState, bank: Bank) -> u32 {
    let mut value = cc_stored(state, bank);
    if bank == Bank::Supervisor {
        value &= !(CC_GIE | CC_STEP);
    } else {
        value |= CC_GIE;
        if state.mode == Mode::User {
            value &= !CC_STEP;
        }
    }
    value & !0xe000
}

fn flags_for_bank(state: &ZipCpuState, bank: Bank) -> Flags {
    Flags::from_cc(cc_stored(state, bank))
}

fn write_flags(state: &mut ZipCpuState, bank: Bank, flags: Flags) {
    let cc = cc_stored_mut(state, bank);
    *cc = (*cc & !CC_FLAGS) | flags.bits();
}

fn set_shared(state: &mut ZipCpuState, mask: u32, set: bool) {
    if set {
        state.supervisor_cc |= mask;
        state.user_cc |= mask;
    } else {
        state.supervisor_cc &= !mask;
        state.user_cc &= !mask;
    }
}

fn active_pc(state: &ZipCpuState) -> u32 {
    match state.mode {
        Mode::Supervisor => state.supervisor_pc,
        Mode::User => state.user_pc,
    }
}

fn set_active_pc(state: &mut ZipCpuState, value: u32) {
    match state.mode {
        Mode::Supervisor => state.supervisor_pc = value & !3,
        Mode::User => state.user_pc = value & !3,
    }
}

fn read_register_stored(state: &ZipCpuState, bank: Bank, register: u8) -> Option<u32> {
    match (bank, register) {
        (Bank::Supervisor, 0..=13) => Some(state.supervisor_gpr[register as usize]),
        (Bank::User, 0..=13) => Some(state.user_gpr[register as usize]),
        (_, 14) => Some(cc_read(state, bank)),
        (Bank::Supervisor, 15) => Some(state.supervisor_pc),
        (Bank::User, 15) => Some(state.user_pc),
        _ => None,
    }
}

fn read_operand_register(state: &ZipCpuState, bank: Bank, register: u8) -> u32 {
    let value = read_register_stored(state, bank, register).expect("decoded register");
    if register == 15 && bank == state.mode.bank() {
        value.wrapping_add(4)
    } else {
        value
    }
}

fn write_register(
    state: &mut ZipCpuState,
    bank: Bank,
    register: u8,
    value: u32,
    writer_mode: Mode,
) {
    match (bank, register) {
        (Bank::Supervisor, 0..=13) => state.supervisor_gpr[register as usize] = value,
        (Bank::User, 0..=13) => state.user_gpr[register as usize] = value,
        (_, 14) => write_cc(state, bank, value, writer_mode),
        (Bank::Supervisor, 15) => state.supervisor_pc = value & !3,
        (Bank::User, 15) => state.user_pc = value & !3,
        _ => unreachable!("decoded register"),
    }
}

fn write_cc(state: &mut ZipCpuState, bank: Bank, value: u32, writer_mode: Mode) {
    let old = cc_stored(state, bank);
    let mut next = old;
    next = (next & !(CC_RESERVED_MASK | CC_FLAGS)) | (value & (CC_RESERVED_MASK | CC_FLAGS));
    match (writer_mode, bank) {
        (Mode::Supervisor, Bank::Supervisor) => {
            next = (next & !CC_BREAK) | (value & CC_BREAK);
        }
        (Mode::Supervisor, Bank::User) => {
            next = (next & !CC_STEP) | (value & CC_STEP);
            next = (next & !CC_USER_STATUS) | (old & value & CC_USER_STATUS);
            let retain_trap = old & value & CC_TRAP != 0;
            if !retain_trap {
                next &= !CC_TRAP;
            }
            set_shared(state, CC_TRAP, retain_trap);
        }
        (Mode::User, Bank::User) => {
            next = (next & !(CC_STEP | CC_USER_STATUS)) | (old & (CC_STEP | CC_USER_STATUS));
        }
        (Mode::User, Bank::Supervisor) => unreachable!("user cannot select supervisor CC"),
    }
    next = (next & 0x007f_1fff) | CC_CPU_INFO;
    if bank == Bank::Supervisor {
        next &= !(CC_GIE | CC_STEP | CC_FPU_ERROR);
    } else {
        next = (next | CC_GIE) & !CC_FPU_ERROR;
    }
    *cc_stored_mut(state, bank) = next;

    let request_gie = value & CC_GIE != 0;
    if writer_mode == Mode::Supervisor && bank == Bank::Supervisor && request_gie {
        if !state.interrupt_asserted {
            set_shared(state, CC_SLEEP | CC_TRAP, false);
            state.user_cc &= !(CC_USER_STATUS | CC_FPU_ERROR);
            state.mode = Mode::User;
        }
    } else if writer_mode == Mode::User && !request_gie {
        set_shared(state, CC_SLEEP, false);
        set_shared(state, CC_TRAP, true);
        state.mode = Mode::Supervisor;
    } else {
        set_shared(state, CC_SLEEP, value & CC_SLEEP != 0);
    }
}

fn apply_fault(state: &mut ZipCpuState, cause: FaultCause) {
    let origin_mode = state.mode;
    let status = match cause {
        FaultCause::IllegalInstruction | FaultCause::IllegalFetch => CC_ILLEGAL,
        FaultCause::DataBusError => CC_BUS_ERROR,
        FaultCause::DivideByZero => CC_DIV_ERROR,
        FaultCause::Break => CC_BREAK,
        FaultCause::SoftwareTrap => CC_TRAP,
    };
    match origin_mode {
        Mode::User => {
            state.user_cc |= status;
            set_shared(state, CC_SLEEP, false);
            state.mode = Mode::Supervisor;
        }
        Mode::Supervisor => {
            if !matches!(cause, FaultCause::Break | FaultCause::SoftwareTrap) {
                state.supervisor_cc |= status;
            }
        }
    }
}

fn fault_stop(before: &ZipCpuState, cause: FaultCause) -> StopReason {
    if before.mode == Mode::Supervisor
        || (cause == FaultCause::Break && before.supervisor_cc & CC_BREAK != 0)
    {
        StopReason::ExternalBreak
    } else {
        StopReason::FaultToSupervisor(cause)
    }
}

/// Independent encoder helpers used by tests, corpora, and examples.
pub mod encoding {
    use super::{Condition, InstructionClass, ZipCpuError, CC_GIE};

    /// Encode the ordinary immediate form.
    #[must_use]
    pub fn immediate(
        class: InstructionClass,
        destination: u8,
        condition: Condition,
        immediate: i32,
    ) -> u32 {
        (u32::from(destination & 0x0f) << 27)
            | (u32::from(class as u8) << 22)
            | (u32::from(condition as u8) << 19)
            | ((immediate as u32) & 0x3_ffff)
    }

    /// Encode the ordinary register-plus-immediate form.
    #[must_use]
    pub fn register(
        class: InstructionClass,
        destination: u8,
        condition: Condition,
        source: u8,
        immediate: i32,
    ) -> u32 {
        (u32::from(destination & 0x0f) << 27)
            | (u32::from(class as u8) << 22)
            | (u32::from(condition as u8) << 19)
            | (1 << 18)
            | (u32::from(source & 0x0f) << 14)
            | ((immediate as u32) & 0x3fff)
    }

    /// Encode the bank-selecting MOV form.
    #[must_use]
    pub fn mov(
        destination: u8,
        destination_user: bool,
        condition: Condition,
        source: u8,
        source_user: bool,
        immediate: i32,
    ) -> u32 {
        (u32::from(destination & 0x0f) << 27)
            | (u32::from(InstructionClass::Mov as u8) << 22)
            | (u32::from(condition as u8) << 19)
            | (u32::from(destination_user) << 18)
            | (u32::from(source & 0x0f) << 14)
            | (u32::from(source_user) << 13)
            | ((immediate as u32) & 0x1fff)
    }

    /// Encode LDI's signed 23-bit immediate.
    #[must_use]
    pub fn ldi(destination: u8, immediate: i32) -> u32 {
        (u32::from(destination & 0x0f) << 27)
            | (u32::from(InstructionClass::Ldi as u8) << 22)
            | ((immediate as u32) & 0x7f_ffff)
    }

    /// Encode the special BREAK or NOOP group.
    #[must_use]
    pub fn special(class: InstructionClass, payload: u32) -> u32 {
        (0x0f << 27) | (u32::from(class as u8) << 22) | (payload & 0x3f_ffff)
    }

    /// Encode a supervisor HALT (`OR CC_SLEEP,CC`).
    #[must_use]
    pub fn mov_to_cc(value: u32) -> u32 {
        immediate(InstructionClass::Or, 14, Condition::Always, value as i32)
    }

    /// Encode a return-to-user command (`OR CC_GIE,CC`).
    #[must_use]
    pub fn return_to_user() -> u32 {
        mov_to_cc(CC_GIE)
    }

    /// Serialize words using exact big-endian instruction transport.
    pub fn assemble(words: &[u32]) -> Result<Vec<u8>, ZipCpuError> {
        let length = words
            .len()
            .checked_mul(4)
            .ok_or(ZipCpuError::AllocationFailed)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| ZipCpuError::AllocationFailed)?;
        for word in words {
            bytes.extend_from_slice(&word.to_be_bytes());
        }
        Ok(bytes)
    }
}
