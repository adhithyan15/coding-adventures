//! Independent Armv8-M Baseline gate transition engine.

use core::fmt;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

use crate::gates;

/// Owned teaching-memory size.
pub const MEMORY_SIZE: usize = 65_536;
/// Conservative budget for retained complete-state trace snapshots.
pub const MAX_RETAINED_BYTES: usize = 8 * 1_024 * 1_024;
/// Maximum complete-state traces retained by a bounded run.
pub const MAX_RETAINED_STEPS: usize = MAX_RETAINED_BYTES / (2 * MEMORY_SIZE);
/// Stack-pointer register number.
pub const SP: u8 = 13;
/// Link-register number.
pub const LR: u8 = 14;
/// Program-counter register number.
pub const PC: u8 = 15;
pub const SAU_REGION_COUNT: usize = 8;

const APSR_N: u32 = 1 << 31;
const APSR_Z: u32 = 1 << 30;
const APSR_C: u32 = 1 << 29;
const APSR_V: u32 = 1 << 28;
const APSR_MASK: u32 = APSR_N | APSR_Z | APSR_C | APSR_V;

/// This bounded gate cell implements only a processor without TrustZone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecurityExtension {
    Disabled,
    Enabled,
}

/// Current architectural security state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecurityState {
    Secure,
    NonSecure,
}

/// Address attribution in the fixed Security-disabled profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecurityAttribute {
    Secure,
    NonSecure,
    NonSecureCallable,
}

/// Canonical SAU descriptor. All fields are fixed clear in this cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SauRegion {
    pub enabled: bool,
    pub base: u32,
    pub limit: u32,
    pub non_secure_callable: bool,
}

fn field16(value: u16, offset: usize, width: usize) -> u16 {
    gates::field(u32::from(value), offset, width) as u16
}

fn line16(value: u16, offset: usize, width: usize, expected: u16) -> u8 {
    gates::selected(
        u32::from(field16(value, offset, width)),
        u32::from(expected),
        width,
    )
}

/// Broad Armv8-M Baseline decode family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstructionClass {
    ShiftAddSubtract,
    Immediate,
    DataProcessing,
    SpecialData,
    LiteralLoad,
    RegisterMemory,
    ImmediateMemory,
    StackMemory,
    AddressGeneration,
    Miscellaneous,
    MultipleMemory,
    ConditionalBranch,
    UnconditionalBranch,
    BranchLink,
    Barrier,
    SpecialRegister,
}

/// Complete restorable state at an instruction boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CortexM23State {
    /// R0-R12.
    pub registers: [u32; 13],
    pub msp_secure: u32,
    pub psp_secure: u32,
    pub msp_non_secure: u32,
    pub psp_non_secure: u32,
    pub lr: u32,
    pub pc: u32,
    /// Only N/Z/C/V are stored here.
    pub apsr: u32,
    pub ipsr: u8,
    pub primask_secure: bool,
    pub primask_non_secure: bool,
    /// nPRIV is bit 0 and SPSEL is bit 1.
    pub control_secure: u8,
    pub control_non_secure: u8,
    pub handler_mode: bool,
    pub security_extension: SecurityExtension,
    pub security_state: SecurityState,
    pub sau_enabled: bool,
    pub sau_all_non_secure: bool,
    pub sau_regions: [SauRegion; SAU_REGION_COUNT],
    pub memory: Vec<u8>,
    pub halted: bool,
    pub sleeping: bool,
    pub event_registered: bool,
    pub loaded_origin: u32,
    pub loaded_len: usize,
}

impl Default for CortexM23State {
    fn default() -> Self {
        Self {
            registers: [0; 13],
            msp_secure: MEMORY_SIZE as u32,
            psp_secure: MEMORY_SIZE as u32,
            msp_non_secure: MEMORY_SIZE as u32,
            psp_non_secure: MEMORY_SIZE as u32,
            lr: 0,
            pc: 0,
            apsr: 0,
            ipsr: 0,
            primask_secure: false,
            primask_non_secure: false,
            control_secure: 0,
            control_non_secure: 0,
            handler_mode: false,
            security_extension: SecurityExtension::Disabled,
            security_state: SecurityState::NonSecure,
            sau_enabled: false,
            sau_all_non_secure: true,
            sau_regions: [SauRegion::default(); SAU_REGION_COUNT],
            memory: vec![0; MEMORY_SIZE],
            halted: false,
            sleeping: false,
            event_registered: false,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }
}

/// Typed lifecycle, decode, and execution failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreError {
    EmptyProgram,
    MisalignedProgram { origin: u32 },
    ProgramOutOfRange { origin: u32, len: usize },
    InvalidState { reason: &'static str },
    InvalidRegister { index: u8 },
    Halted,
    Sleeping,
    MisalignedFetch { pc: u32 },
    FetchOutsideProgram { pc: u32, width: usize },
    MisalignedData { address: u32, width: usize },
    DataOutOfRange { address: u32, width: usize },
    InvalidThumbTarget { target: u32 },
    UnknownInstruction16 { raw: u16, pc: u32 },
    UnknownInstruction32 { first: u16, second: u16, pc: u32 },
    UnsupportedBreakpoint { immediate: u8, pc: u32 },
    SupervisorCall { immediate: u8, pc: u32 },
    RetentionLimitExceeded { requested: usize, limit: usize },
    StepLimitExceeded { limit: usize },
    AllocationFailed,
}

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => formatter.write_str("program is empty"),
            Self::MisalignedProgram { origin } => {
                write!(
                    formatter,
                    "program origin {origin:#010x} is not halfword aligned"
                )
            }
            Self::ProgramOutOfRange { origin, len } => write!(
                formatter,
                "program of {len} bytes at {origin:#010x} exceeds memory"
            ),
            Self::InvalidState { reason } => write!(formatter, "invalid state: {reason}"),
            Self::InvalidRegister { index } => write!(formatter, "invalid register r{index}"),
            Self::Halted => formatter.write_str("processor is halted"),
            Self::Sleeping => formatter.write_str("processor is sleeping"),
            Self::MisalignedFetch { pc } => write!(formatter, "misaligned fetch at {pc:#010x}"),
            Self::FetchOutsideProgram { pc, width } => {
                write!(
                    formatter,
                    "{width}-byte fetch at {pc:#010x} is outside the program"
                )
            }
            Self::MisalignedData { address, width } => {
                write!(
                    formatter,
                    "{width}-byte access at {address:#010x} is misaligned"
                )
            }
            Self::DataOutOfRange { address, width } => {
                write!(
                    formatter,
                    "{width}-byte access at {address:#010x} is out of range"
                )
            }
            Self::InvalidThumbTarget { target } => {
                write!(
                    formatter,
                    "branch target {target:#010x} does not select Thumb state"
                )
            }
            Self::UnknownInstruction16 { raw, pc } => {
                write!(formatter, "unknown instruction {raw:#06x} at {pc:#010x}")
            }
            Self::UnknownInstruction32 { first, second, pc } => write!(
                formatter,
                "unknown instruction {first:#06x} {second:#06x} at {pc:#010x}"
            ),
            Self::UnsupportedBreakpoint { immediate, pc } => write!(
                formatter,
                "BKPT #{immediate} at {pc:#010x} requires a debugger"
            ),
            Self::SupervisorCall { immediate, pc } => {
                write!(
                    formatter,
                    "SVC #{immediate} at {pc:#010x} requires exception entry"
                )
            }
            Self::RetentionLimitExceeded { requested, limit } => write!(
                formatter,
                "requested {requested} traces exceeds retention limit {limit}"
            ),
            Self::StepLimitExceeded { limit } => {
                write!(formatter, "step limit {limit} exhausted")
            }
            Self::AllocationFailed => formatter.write_str("trace allocation failed"),
        }
    }
}

impl std::error::Error for CoreError {}

/// One committed architectural transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: [u8; 4],
    pub width: usize,
    pub mnemonic: &'static str,
    pub security_before: SecurityState,
    pub security_after: SecurityState,
    pub state_before: CortexM23State,
    pub state_after: CortexM23State,
}

/// Successful bounded execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub stopped: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: CortexM23State,
}

pub(crate) fn step_state(mut state: CortexM23State) -> Result<StepTrace, CoreError> {
    if state.halted {
        return Err(CoreError::Halted);
    }
    if state.sleeping {
        return Err(CoreError::Sleeping);
    }
    let before = try_clone_state(&state)?;
    let pc = state.pc;
    let first = fetch16(&state, pc)?;
    let width = instruction_width(first);
    let second = if width == 4 {
        Some(
            fetch16(&state, crate::gates::add_32(pc, 2, 0).0).map_err(|error| match error {
                CoreError::FetchOutsideProgram { .. } => {
                    CoreError::FetchOutsideProgram { pc, width: 4 }
                }
                other => other,
            })?,
        )
    } else {
        None
    };
    let class = classify_at(first, second, pc)?;
    state.pc = crate::gates::add_32(pc, width as u32, 0).0;
    let mnemonic = execute_decoded(&mut state, first, second, class, pc)?;
    let mut raw = [0; 4];
    raw[..2].copy_from_slice(&gates::split_le_u16(first));
    if let Some(second) = second {
        raw[2..].copy_from_slice(&gates::split_le_u16(second));
    }
    Ok(StepTrace {
        pc_before: pc,
        pc_after: state.pc,
        raw,
        width,
        mnemonic,
        security_before: SecurityState::NonSecure,
        security_after: SecurityState::NonSecure,
        state_before: before,
        state_after: state,
    })
}

fn try_clone_state(state: &CortexM23State) -> Result<CortexM23State, CoreError> {
    crate::allocation_checkpoint().map_err(|()| CoreError::AllocationFailed)?;
    let mut memory = Vec::new();
    memory
        .try_reserve_exact(state.memory.len())
        .map_err(|_| CoreError::AllocationFailed)?;
    memory.extend_from_slice(&state.memory);
    Ok(CortexM23State {
        registers: state.registers,
        msp_secure: state.msp_secure,
        psp_secure: state.psp_secure,
        msp_non_secure: state.msp_non_secure,
        psp_non_secure: state.psp_non_secure,
        lr: state.lr,
        pc: state.pc,
        apsr: state.apsr,
        ipsr: state.ipsr,
        primask_secure: state.primask_secure,
        primask_non_secure: state.primask_non_secure,
        control_secure: state.control_secure,
        control_non_secure: state.control_non_secure,
        handler_mode: state.handler_mode,
        security_extension: state.security_extension,
        security_state: state.security_state,
        sau_enabled: state.sau_enabled,
        sau_all_non_secure: state.sau_all_non_secure,
        sau_regions: state.sau_regions,
        memory,
        halted: state.halted,
        sleeping: state.sleeping,
        event_registered: state.event_registered,
        loaded_origin: state.loaded_origin,
        loaded_len: state.loaded_len,
    })
}

pub(crate) fn validate_program(program: &[u8], origin: u32) -> Result<(), CoreError> {
    if program.is_empty() {
        return Err(CoreError::EmptyProgram);
    }
    if gates::aligned_32(origin, 2) == 0 {
        return Err(CoreError::MisalignedProgram { origin });
    }
    if program.len() > MEMORY_SIZE
        || gates::less_or_equal_unsigned_32(origin, (MEMORY_SIZE - program.len()) as u32) == 0
    {
        return Err(CoreError::ProgramOutOfRange {
            origin,
            len: program.len(),
        });
    }
    Ok(())
}

pub(crate) fn validate_state(state: &CortexM23State) -> Result<(), CoreError> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(CoreError::InvalidState {
            reason: "memory must be exactly 64 KiB",
        });
    }
    if gates::aligned_32(state.pc, 2) == 0
        || gates::aligned_32(state.msp_non_secure, 4) == 0
        || gates::aligned_32(state.psp_non_secure, 4) == 0
    {
        return Err(CoreError::InvalidState {
            reason: "PC or stack pointer alignment is invalid",
        });
    }
    if state.apsr & !APSR_MASK != 0 || state.control_non_secure & !3 != 0 || state.ipsr > 63 {
        return Err(CoreError::InvalidState {
            reason: "reserved status bits are set",
        });
    }
    if state.handler_mode != (state.ipsr != 0) {
        return Err(CoreError::InvalidState {
            reason: "mode and IPSR disagree",
        });
    }
    if state.security_extension != SecurityExtension::Disabled
        || state.security_state != SecurityState::NonSecure
        || state.msp_secure != MEMORY_SIZE as u32
        || state.psp_secure != MEMORY_SIZE as u32
        || state.primask_secure
        || state.control_secure != 0
        || state.sau_enabled
        || !state.sau_all_non_secure
        || state.sau_regions != [SauRegion::default(); SAU_REGION_COUNT]
    {
        return Err(CoreError::InvalidState {
            reason: "Security-disabled fields are not canonical",
        });
    }
    if state.loaded_len == 0 {
        if state.loaded_origin != 0 {
            return Err(CoreError::InvalidState {
                reason: "empty image must use origin zero",
            });
        }
    } else if gates::aligned_32(state.loaded_origin, 2) == 0
        || state.loaded_len > MEMORY_SIZE
        || gates::less_or_equal_unsigned_32(
            state.loaded_origin,
            (MEMORY_SIZE - state.loaded_len) as u32,
        ) == 0
    {
        return Err(CoreError::InvalidState {
            reason: "installed image range is invalid",
        });
    }
    Ok(())
}

fn active_sp(state: &CortexM23State) -> u32 {
    let select = and_gate(
        not_gate(u8::from(state.handler_mode)),
        gates::u8_to_bits(state.control_non_secure)[1],
    );
    gates::select_32(state.msp_non_secure, state.psp_non_secure, select)
}

fn set_active_sp(state: &mut CortexM23State, value: u32) {
    if and_gate(
        not_gate(u8::from(state.handler_mode)),
        gates::u8_to_bits(state.control_non_secure)[1],
    ) == 1
    {
        state.psp_non_secure = value;
    } else {
        state.msp_non_secure = value;
    }
}

pub(crate) fn read_register(state: &CortexM23State, index: u8) -> Result<u32, CoreError> {
    match index {
        0..=12 => Ok(state.registers[index as usize]),
        SP => Ok(active_sp(state)),
        LR => Ok(state.lr),
        PC => Ok(state.pc),
        _ => Err(CoreError::InvalidRegister { index }),
    }
}

pub(crate) fn write_register(
    state: &mut CortexM23State,
    index: u8,
    value: u32,
) -> Result<(), CoreError> {
    match index {
        0..=12 => state.registers[index as usize] = value,
        SP => set_active_sp(state, gates::bit_and_32(value, gates::bit_not_32(3))),
        LR => state.lr = value,
        PC => state.pc = gates::bit_and_32(value, gates::bit_not_32(1)),
        _ => return Err(CoreError::InvalidRegister { index }),
    }
    Ok(())
}

pub(crate) fn checked_range(
    address: u32,
    width: usize,
) -> Result<std::ops::Range<usize>, CoreError> {
    if !matches!(width, 1 | 2 | 4) {
        return Err(CoreError::DataOutOfRange { address, width });
    }
    if gates::aligned_32(address, width) == 0 {
        return Err(CoreError::MisalignedData { address, width });
    }
    let max_start = (MEMORY_SIZE - width) as u32;
    if gates::less_or_equal_unsigned_32(address, max_start) == 0 {
        return Err(CoreError::DataOutOfRange { address, width });
    }
    let start = address as usize;
    let end = start + width;
    Ok(start..end)
}

pub(crate) fn read_memory(
    state: &CortexM23State,
    address: u32,
    width: usize,
) -> Result<u32, CoreError> {
    let range = checked_range(address, width)?;
    let mut bytes = [0; 4];
    bytes[..width].copy_from_slice(&state.memory[range]);
    Ok(gates::assemble_le_u32(bytes))
}

pub(crate) fn write_memory(
    state: &mut CortexM23State,
    address: u32,
    width: usize,
    value: u32,
) -> Result<(), CoreError> {
    let range = checked_range(address, width)?;
    state.memory[range].copy_from_slice(&gates::split_le_u32(value)[..width]);
    Ok(())
}

fn fetch16(state: &CortexM23State, pc: u32) -> Result<u16, CoreError> {
    if gates::aligned_32(pc, 2) == 0 {
        return Err(CoreError::MisalignedFetch { pc });
    }
    if state.loaded_len < 2 {
        return Err(CoreError::FetchOutsideProgram { pc, width: 2 });
    }
    let (relative, borrow, _) = gates::sub_32(pc, state.loaded_origin, 0);
    let max_relative = gates::sub_32(state.loaded_len as u32, 2, 0).0;
    if borrow == 1 || gates::less_or_equal_unsigned_32(relative, max_relative) == 0 {
        return Err(CoreError::FetchOutsideProgram { pc, width: 2 });
    }
    let start = pc as usize;
    Ok(gates::assemble_le_u16([
        state.memory[start],
        state.memory[start + 1],
    ]))
}

pub(crate) fn instruction_width(first: u16) -> usize {
    let prefix = field16(first, 11, 5);
    if [0b11101, 0b11110, 0b11111]
        .into_iter()
        .fold(0, |signal, expected| {
            or_gate(signal, gates::selected(u32::from(prefix), expected, 5))
        })
        == 1
    {
        4
    } else {
        2
    }
}

/// Classify an encoded instruction without executing it.
pub fn classify(first: u16, second: Option<u16>) -> Result<InstructionClass, CoreError> {
    classify_at(first, second, 0)
}

pub(crate) fn classify_at(
    first: u16,
    second: Option<u16>,
    pc: u32,
) -> Result<InstructionClass, CoreError> {
    if instruction_width(first) == 4 {
        let Some(second) = second else {
            return Err(CoreError::FetchOutsideProgram { pc, width: 4 });
        };
        if and_gate(
            and_gate(line16(first, 11, 5, 0b11110), line16(second, 14, 2, 0b11)),
            line16(second, 12, 1, 1),
        ) == 1
        {
            return Ok(InstructionClass::BranchLink);
        }
        let barrier = [0x8f4f, 0x8f5f, 0x8f6f]
            .into_iter()
            .fold(0, |signal, word| {
                or_gate(signal, line16(second, 0, 16, word))
            });
        if and_gate(line16(first, 0, 16, 0xf3bf), barrier) == 1 {
            return Ok(InstructionClass::Barrier);
        }
        let mrs = and_gate(line16(first, 0, 16, 0xf3ef), line16(second, 12, 4, 8));
        let msr = and_gate(line16(first, 4, 12, 0xf38), line16(second, 8, 8, 0x88));
        if or_gate(mrs, msr) == 1 {
            return Ok(InstructionClass::SpecialRegister);
        }
        return Err(CoreError::UnknownInstruction32 { first, second, pc });
    }
    if line16(first, 13, 3, 0) == 1 {
        return Ok(InstructionClass::ShiftAddSubtract);
    }
    if line16(first, 13, 3, 1) == 1 {
        return Ok(InstructionClass::Immediate);
    }
    if line16(first, 10, 6, 0b01_0000) == 1 {
        return Ok(InstructionClass::DataProcessing);
    }
    if line16(first, 10, 6, 0b01_0001) == 1 {
        return Ok(InstructionClass::SpecialData);
    }
    if line16(first, 11, 5, 0b01001) == 1 {
        return Ok(InstructionClass::LiteralLoad);
    }
    if line16(first, 12, 4, 0b0101) == 1 {
        return Ok(InstructionClass::RegisterMemory);
    }
    if [0b0110, 0b0111, 0b1000]
        .into_iter()
        .fold(0, |signal, prefix| {
            or_gate(signal, line16(first, 12, 4, prefix))
        })
        == 1
    {
        return Ok(InstructionClass::ImmediateMemory);
    }
    if line16(first, 12, 4, 0b1001) == 1 {
        return Ok(InstructionClass::StackMemory);
    }
    if line16(first, 12, 4, 0b1010) == 1 {
        return Ok(InstructionClass::AddressGeneration);
    }
    if line16(first, 12, 4, 0b1011) == 1 {
        return Ok(InstructionClass::Miscellaneous);
    }
    if line16(first, 12, 4, 0b1100) == 1 {
        return Ok(InstructionClass::MultipleMemory);
    }
    if line16(first, 12, 4, 0b1101) == 1 {
        return Ok(InstructionClass::ConditionalBranch);
    }
    if line16(first, 11, 5, 0b11100) == 1 {
        return Ok(InstructionClass::UnconditionalBranch);
    }
    Err(CoreError::UnknownInstruction16 { raw: first, pc })
}

fn execute_decoded(
    state: &mut CortexM23State,
    first: u16,
    second: Option<u16>,
    class: InstructionClass,
    pc: u32,
) -> Result<&'static str, CoreError> {
    match class {
        InstructionClass::ShiftAddSubtract => execute_shift_add_sub(state, first),
        InstructionClass::Immediate => execute_immediate(state, first),
        InstructionClass::DataProcessing => execute_data_processing(state, first),
        InstructionClass::SpecialData => execute_special(state, first, pc),
        InstructionClass::LiteralLoad => execute_literal_load(state, first, pc),
        InstructionClass::RegisterMemory => execute_register_memory(state, first),
        InstructionClass::ImmediateMemory => execute_immediate_memory(state, first),
        InstructionClass::StackMemory => execute_stack_memory(state, first),
        InstructionClass::AddressGeneration => execute_address_generation(state, first, pc),
        InstructionClass::Miscellaneous => execute_miscellaneous(state, first, pc),
        InstructionClass::MultipleMemory => execute_multiple_memory(state, first),
        InstructionClass::ConditionalBranch => execute_conditional(state, first, pc),
        InstructionClass::UnconditionalBranch => execute_unconditional(state, first, pc),
        InstructionClass::BranchLink => execute_bl(state, first, second.expect("classified"), pc),
        InstructionClass::Barrier => execute_barrier(second.expect("classified")),
        InstructionClass::SpecialRegister => {
            execute_special_register(state, first, second.expect("classified"), pc)
        }
    }
}

fn flag(state: &CortexM23State, mask: u32) -> bool {
    gates::bit_and_32(state.apsr, mask) != 0
}

fn set_flag(state: &mut CortexM23State, mask: u32, value: bool) {
    let cleared = gates::bit_and_32(state.apsr, gates::bit_not_32(mask));
    let asserted = gates::bit_or_32(cleared, mask);
    state.apsr = gates::select_32(cleared, asserted, u8::from(value));
}

fn set_nz(state: &mut CortexM23State, value: u32) {
    set_flag(state, APSR_N, gates::u32_to_bits(value)[31] == 1);
    set_flag(state, APSR_Z, gates::is_zero_32(value) == 1);
}

fn add_with_carry(state: &mut CortexM23State, left: u32, right: u32, carry: bool) -> u32 {
    let (result, carry_out, overflow) = gates::add_32(left, right, u8::from(carry));
    set_nz(state, result);
    set_flag(state, APSR_C, carry_out == 1);
    set_flag(state, APSR_V, overflow == 1);
    result
}

fn subtract(state: &mut CortexM23State, left: u32, right: u32, borrow: bool) -> u32 {
    let (result, borrowed, overflow) = gates::sub_32(left, right, u8::from(borrow));
    set_nz(state, result);
    set_flag(state, APSR_C, borrowed == 0);
    set_flag(state, APSR_V, overflow == 1);
    result
}

fn shift_immediate(value: u32, kind: u8, encoded: u32, carry: bool) -> (u32, bool) {
    let (result, carry_out) = gates::shift_immediate_network(value, kind, encoded, u8::from(carry));
    (result, carry_out == 1)
}

fn shift_register(value: u32, kind: u8, amount: u32, carry: bool) -> (u32, bool) {
    let (result, carry_out) = gates::shift_register_network(value, kind, amount, u8::from(carry));
    (result, carry_out == 1)
}

fn add_or_subtract(state: &mut CortexM23State, left: u32, right: u32, subtract_line: u8) -> u32 {
    let added = gates::add_32(left, right, 0);
    let subtracted = gates::sub_32(left, right, 0);
    let result = gates::select_32(added.0, subtracted.0, subtract_line);
    let carry = gates::select_one_hot_bit(
        [added.1, not_gate(subtracted.1)],
        [not_gate(subtract_line), subtract_line],
    );
    let overflow = gates::select_one_hot_bit(
        [added.2, subtracted.2],
        [not_gate(subtract_line), subtract_line],
    );
    set_nz(state, result);
    set_flag(state, APSR_C, carry == 1);
    set_flag(state, APSR_V, overflow == 1);
    result
}

fn execute_shift_add_sub(state: &mut CortexM23State, raw: u16) -> Result<&'static str, CoreError> {
    let operation = field16(raw, 11, 2);
    let shift_lines = [
        line16(operation, 0, 2, 0),
        line16(operation, 0, 2, 1),
        line16(operation, 0, 2, 2),
    ];
    let shift_family = gates::any_line(shift_lines);
    if shift_family == 1 {
        let amount = u32::from(field16(raw, 6, 5));
        let rm = field16(raw, 3, 3) as usize;
        let rd = field16(raw, 0, 3) as usize;
        let (result, carry) = shift_immediate(
            state.registers[rm],
            operation as u8,
            amount,
            flag(state, APSR_C),
        );
        state.registers[rd] = result;
        set_nz(state, result);
        set_flag(state, APSR_C, carry);
        if shift_lines[0] == 1 {
            return Ok("lsls");
        }
        if shift_lines[1] == 1 {
            return Ok("lsrs");
        }
        return Ok("asrs");
    }
    let op = field16(raw, 9, 2);
    let operand = field16(raw, 6, 3) as usize;
    let rn = field16(raw, 3, 3) as usize;
    let rd = field16(raw, 0, 3) as usize;
    let op_lines = [
        line16(op, 0, 2, 0),
        line16(op, 0, 2, 1),
        line16(op, 0, 2, 2),
        line16(op, 0, 2, 3),
    ];
    let immediate_line = or_gate(op_lines[2], op_lines[3]);
    let subtract_line = or_gate(op_lines[1], op_lines[3]);
    let right = gates::select_32(state.registers[operand], operand as u32, immediate_line);
    let result = add_or_subtract(state, state.registers[rn], right, subtract_line);
    state.registers[rd] = result;
    if subtract_line == 1 {
        Ok("subs")
    } else {
        Ok("adds")
    }
}

fn execute_immediate(state: &mut CortexM23State, raw: u16) -> Result<&'static str, CoreError> {
    let operation = field16(raw, 11, 2);
    let register = field16(raw, 8, 3) as usize;
    let immediate = u32::from(field16(raw, 0, 8));
    match operation {
        0 => {
            state.registers[register] = immediate;
            set_nz(state, immediate);
            Ok("movs")
        }
        1 => {
            subtract(state, state.registers[register], immediate, false);
            Ok("cmp")
        }
        2 => {
            state.registers[register] =
                add_with_carry(state, state.registers[register], immediate, false);
            Ok("adds")
        }
        _ => {
            state.registers[register] =
                subtract(state, state.registers[register], immediate, false);
            Ok("subs")
        }
    }
}

fn execute_data_processing(
    state: &mut CortexM23State,
    raw: u16,
) -> Result<&'static str, CoreError> {
    let operation = field16(raw, 6, 4) as u8;
    let rm = field16(raw, 3, 3) as usize;
    let rdn = field16(raw, 0, 3) as usize;
    let left = state.registers[rdn];
    let right = state.registers[rm];
    Ok(match operation {
        0 => {
            state.registers[rdn] = gates::bit_and_32(left, right);
            set_nz(state, state.registers[rdn]);
            "ands"
        }
        1 => {
            state.registers[rdn] = gates::bit_xor_32(left, right);
            set_nz(state, state.registers[rdn]);
            "eors"
        }
        2..=4 | 7 => {
            let kind_lines = [
                gates::selected(u32::from(operation), 2, 4),
                gates::selected(u32::from(operation), 3, 4),
                gates::selected(u32::from(operation), 4, 4),
                gates::selected(u32::from(operation), 7, 4),
            ];
            let kind = gates::select_one_hot_32([0, 1, 2, 3], kind_lines) as u8;
            let (result, carry) = shift_register(
                left,
                kind,
                gates::bit_and_32(right, 0xff),
                flag(state, APSR_C),
            );
            state.registers[rdn] = result;
            set_nz(state, result);
            set_flag(state, APSR_C, carry);
            ["lsls", "lsrs", "asrs", "rors"][kind as usize]
        }
        5 => {
            state.registers[rdn] = add_with_carry(state, left, right, flag(state, APSR_C));
            "adcs"
        }
        6 => {
            state.registers[rdn] = subtract(state, left, right, !flag(state, APSR_C));
            "sbcs"
        }
        8 => {
            set_nz(state, gates::bit_and_32(left, right));
            "tst"
        }
        9 => {
            state.registers[rdn] = subtract(state, 0, right, false);
            "rsbs"
        }
        10 => {
            subtract(state, left, right, false);
            "cmp"
        }
        11 => {
            add_with_carry(state, left, right, false);
            "cmn"
        }
        12 => {
            state.registers[rdn] = gates::bit_or_32(left, right);
            set_nz(state, state.registers[rdn]);
            "orrs"
        }
        13 => {
            state.registers[rdn] = gates::multiply_32(left, right);
            set_nz(state, state.registers[rdn]);
            "muls"
        }
        14 => {
            state.registers[rdn] = gates::bit_and_32(left, gates::bit_not_32(right));
            set_nz(state, state.registers[rdn]);
            "bics"
        }
        _ => {
            state.registers[rdn] = gates::bit_not_32(right);
            set_nz(state, state.registers[rdn]);
            "mvns"
        }
    })
}

fn branch_exchange(state: &mut CortexM23State, target: u32) -> Result<(), CoreError> {
    if gates::bit_and_32(target, 1) == 0 {
        return Err(CoreError::InvalidThumbTarget { target });
    }
    state.pc = gates::bit_and_32(target, gates::bit_not_32(1));
    Ok(())
}

fn branch_write_pc(state: &mut CortexM23State, target: u32) {
    state.pc = gates::bit_and_32(target, gates::bit_not_32(1));
}

fn execute_special(
    state: &mut CortexM23State,
    raw: u16,
    instruction_pc: u32,
) -> Result<&'static str, CoreError> {
    let operation = field16(raw, 8, 2);
    let rm = field16(raw, 3, 4) as u8;
    let rd = gates::bit_or_32(
        gates::shift_left_32(u32::from(field16(raw, 7, 1)), 3),
        u32::from(field16(raw, 0, 3)),
    ) as u8;
    if operation == 3 {
        if field16(raw, 0, 3) != 0 || matches!(rm, SP | PC) {
            return Err(CoreError::UnknownInstruction16 {
                raw,
                pc: instruction_pc,
            });
        }
        let target = read_register(state, rm)?;
        if line16(raw, 7, 1, 1) == 1 {
            state.lr = gates::bit_or_32(state.pc, 1);
        }
        branch_exchange(state, target)?;
        return Ok(if line16(raw, 7, 1, 1) == 1 {
            "blx"
        } else {
            "bx"
        });
    }
    let left = if rd == PC {
        gates::add_32(instruction_pc, 4, 0).0
    } else {
        read_register(state, rd)?
    };
    let right = if rm == PC {
        gates::add_32(instruction_pc, 4, 0).0
    } else {
        read_register(state, rm)?
    };
    match operation {
        0 => {
            let result = gates::add_32(left, right, 0).0;
            if rd == PC {
                branch_write_pc(state, result);
            } else {
                write_register(state, rd, result)?;
            }
            Ok("add")
        }
        1 => {
            subtract(state, left, right, false);
            Ok("cmp")
        }
        _ => {
            if rd == PC {
                branch_write_pc(state, right);
            } else {
                write_register(state, rd, right)?;
            }
            Ok("mov")
        }
    }
}

fn execute_literal_load(
    state: &mut CortexM23State,
    raw: u16,
    instruction_pc: u32,
) -> Result<&'static str, CoreError> {
    let rt = field16(raw, 8, 3) as usize;
    let visible = gates::bit_and_32(gates::add_32(instruction_pc, 4, 0).0, gates::bit_not_32(3));
    let offset = gates::shift_left_32(u32::from(field16(raw, 0, 8)), 2);
    let address = gates::add_32(visible, offset, 0).0;
    state.registers[rt] = read_memory(state, address, 4)?;
    Ok("ldr")
}

fn execute_register_memory(
    state: &mut CortexM23State,
    raw: u16,
) -> Result<&'static str, CoreError> {
    let operation = field16(raw, 9, 3);
    let rm = field16(raw, 6, 3) as usize;
    let rn = field16(raw, 3, 3) as usize;
    let rt = field16(raw, 0, 3) as usize;
    let address = gates::add_32(state.registers[rn], state.registers[rm], 0).0;
    Ok(match operation {
        0 => {
            write_memory(state, address, 4, state.registers[rt])?;
            "str"
        }
        1 => {
            write_memory(state, address, 2, state.registers[rt])?;
            "strh"
        }
        2 => {
            write_memory(state, address, 1, state.registers[rt])?;
            "strb"
        }
        3 => {
            state.registers[rt] = gates::sign_extend(read_memory(state, address, 1)?, 8);
            "ldrsb"
        }
        4 => {
            state.registers[rt] = read_memory(state, address, 4)?;
            "ldr"
        }
        5 => {
            state.registers[rt] = read_memory(state, address, 2)?;
            "ldrh"
        }
        6 => {
            state.registers[rt] = read_memory(state, address, 1)?;
            "ldrb"
        }
        _ => {
            state.registers[rt] = gates::sign_extend(read_memory(state, address, 2)?, 16);
            "ldrsh"
        }
    })
}

fn execute_immediate_memory(
    state: &mut CortexM23State,
    raw: u16,
) -> Result<&'static str, CoreError> {
    let operation = field16(raw, 11, 5);
    let immediate = u32::from(field16(raw, 6, 5));
    let rn = field16(raw, 3, 3) as usize;
    let rt = field16(raw, 0, 3) as usize;
    let (scale, load, width, mnemonic): (u32, bool, usize, &str) = match operation {
        0b01100 => (4, false, 4, "str"),
        0b01101 => (4, true, 4, "ldr"),
        0b01110 => (1, false, 1, "strb"),
        0b01111 => (1, true, 1, "ldrb"),
        0b10000 => (2, false, 2, "strh"),
        _ => (2, true, 2, "ldrh"),
    };
    let scaled = gates::shift_left_32(immediate, scale.trailing_zeros());
    let address = gates::add_32(state.registers[rn], scaled, 0).0;
    if load {
        state.registers[rt] = read_memory(state, address, width)?;
    } else {
        write_memory(state, address, width, state.registers[rt])?;
    }
    Ok(mnemonic)
}

fn execute_stack_memory(state: &mut CortexM23State, raw: u16) -> Result<&'static str, CoreError> {
    let load = line16(raw, 11, 1, 1) == 1;
    let rt = field16(raw, 8, 3) as usize;
    let address = gates::add_32(
        active_sp(state),
        gates::shift_left_32(u32::from(field16(raw, 0, 8)), 2),
        0,
    )
    .0;
    if load {
        state.registers[rt] = read_memory(state, address, 4)?;
        Ok("ldr")
    } else {
        write_memory(state, address, 4, state.registers[rt])?;
        Ok("str")
    }
}

fn execute_address_generation(
    state: &mut CortexM23State,
    raw: u16,
    instruction_pc: u32,
) -> Result<&'static str, CoreError> {
    let rd = field16(raw, 8, 3) as usize;
    let immediate = gates::shift_left_32(u32::from(field16(raw, 0, 8)), 2);
    if line16(raw, 11, 1, 0) == 1 {
        let visible =
            gates::bit_and_32(gates::add_32(instruction_pc, 4, 0).0, gates::bit_not_32(3));
        state.registers[rd] = gates::add_32(visible, immediate, 0).0;
        Ok("adr")
    } else {
        state.registers[rd] = gates::add_32(active_sp(state), immediate, 0).0;
        Ok("add")
    }
}

fn execute_miscellaneous(
    state: &mut CortexM23State,
    raw: u16,
    pc: u32,
) -> Result<&'static str, CoreError> {
    if line16(raw, 8, 8, 0xb0) == 1 {
        let offset = gates::shift_left_32(u32::from(field16(raw, 0, 7)), 2);
        let value = if line16(raw, 7, 1, 1) == 1 {
            gates::sub_32(active_sp(state), offset, 0).0
        } else {
            gates::add_32(active_sp(state), offset, 0).0
        };
        set_active_sp(state, value);
        return Ok(if line16(raw, 7, 1, 1) == 1 {
            "sub"
        } else {
            "add"
        });
    }
    if line16(raw, 8, 8, 0xb2) == 1 {
        let operation = field16(raw, 6, 2);
        let rm = field16(raw, 3, 3) as usize;
        let rd = field16(raw, 0, 3) as usize;
        let value = state.registers[rm];
        state.registers[rd] = match operation {
            0 => gates::sign_extend(gates::bit_and_32(value, 0xffff), 16),
            1 => gates::sign_extend(gates::bit_and_32(value, 0xff), 8),
            2 => gates::bit_and_32(value, 0xffff),
            _ => gates::bit_and_32(value, 0xff),
        };
        return Ok(["sxth", "sxtb", "uxth", "uxtb"][operation as usize]);
    }
    if line16(raw, 9, 7, 0x5a) == 1 {
        let mask = field16(raw, 0, 8);
        let include_lr = line16(raw, 8, 1, 1) == 1;
        if mask == 0 && !include_lr {
            return Err(CoreError::UnknownInstruction16 { raw, pc });
        }
        let count = gates::add_32(gates::popcount_8(mask as u8), u32::from(include_lr), 0).0;
        let start = gates::sub_32(active_sp(state), gates::shift_left_32(count, 2), 0).0;
        let mut address = start;
        for register in 0..8 {
            if line16(mask, register, 1, 1) == 1 {
                write_memory(state, address, 4, state.registers[register])?;
                address = gates::add_32(address, 4, 0).0;
            }
        }
        if include_lr {
            write_memory(state, address, 4, state.lr)?;
        }
        set_active_sp(state, start);
        return Ok("push");
    }
    if line16(raw, 9, 7, 0x5e) == 1 {
        let mask = field16(raw, 0, 8);
        let include_pc = line16(raw, 8, 1, 1) == 1;
        if mask == 0 && !include_pc {
            return Err(CoreError::UnknownInstruction16 { raw, pc });
        }
        let mut address = active_sp(state);
        for register in 0..8 {
            if line16(mask, register, 1, 1) == 1 {
                state.registers[register] = read_memory(state, address, 4)?;
                address = gates::add_32(address, 4, 0).0;
            }
        }
        if include_pc {
            let target = read_memory(state, address, 4)?;
            address = gates::add_32(address, 4, 0).0;
            branch_exchange(state, target)?;
        }
        set_active_sp(state, address);
        return Ok("pop");
    }
    if line16(raw, 8, 8, 0xba) == 1 {
        let operation = field16(raw, 6, 2);
        if operation == 2 {
            return Err(CoreError::UnknownInstruction16 { raw, pc });
        }
        let rm = field16(raw, 3, 3) as usize;
        let rd = field16(raw, 0, 3) as usize;
        let value = state.registers[rm];
        state.registers[rd] = match operation {
            0 => gates::reverse_bytes(value),
            1 => gates::reverse_halfwords(value),
            _ => gates::sign_extend(
                gates::bit_and_32(gates::reverse_halfwords(value), 0xffff),
                16,
            ),
        };
        return Ok(match operation {
            0 => "rev",
            1 => "rev16",
            _ => "revsh",
        });
    }
    if matches!(raw, 0xb662 | 0xb672) {
        if or_gate(
            not_gate(gates::u8_to_bits(state.control_non_secure)[0]),
            u8::from(state.handler_mode),
        ) == 1
        {
            state.primask_non_secure = line16(raw, 4, 1, 1) == 1;
        }
        return Ok(if raw == 0xb672 { "cpsid" } else { "cpsie" });
    }
    if line16(raw, 8, 8, 0xbe) == 1 {
        let immediate = field16(raw, 0, 8) as u8;
        if immediate == 0 {
            state.halted = true;
            return Ok("bkpt 0");
        }
        return Err(CoreError::UnsupportedBreakpoint { immediate, pc });
    }
    if line16(raw, 8, 8, 0xbf) == 1 && line16(raw, 0, 4, 0) == 1 {
        return match field16(raw, 4, 4) {
            0x0 => Ok("nop"),
            0x1 => Ok("yield"),
            0x2 => {
                let event = u8::from(state.event_registered);
                state.sleeping =
                    or_gate(and_gate(event, u8::from(state.sleeping)), not_gate(event)) == 1;
                state.event_registered = false;
                Ok("wfe")
            }
            0x3 => {
                state.sleeping = true;
                Ok("wfi")
            }
            0x4 => {
                state.event_registered = true;
                Ok("sev")
            }
            _ => Ok("nop"),
        };
    }
    Err(CoreError::UnknownInstruction16 { raw, pc })
}

fn execute_multiple_memory(
    state: &mut CortexM23State,
    raw: u16,
) -> Result<&'static str, CoreError> {
    let load = line16(raw, 11, 1, 1) == 1;
    let rn = field16(raw, 8, 3) as usize;
    let mask = field16(raw, 0, 8);
    if mask == 0 {
        return Err(CoreError::UnknownInstruction16 {
            raw,
            pc: gates::sub_32(state.pc, 2, 0).0,
        });
    }
    let base_selected = line16(mask, rn, 1, 1);
    let lower_selected = (0..rn).fold(0, |signal, register| {
        or_gate(signal, line16(mask, register, 1, 1))
    });
    if !load && and_gate(base_selected, lower_selected) == 1 {
        return Err(CoreError::UnknownInstruction16 {
            raw,
            pc: gates::sub_32(state.pc, 2, 0).0,
        });
    }
    let mut address = state.registers[rn];
    for register in 0..8 {
        if line16(mask, register, 1, 1) == 1 {
            if load {
                state.registers[register] = read_memory(state, address, 4)?;
            } else {
                write_memory(state, address, 4, state.registers[register])?;
            }
            address = gates::add_32(address, 4, 0).0;
        }
    }
    if !load || base_selected == 0 {
        state.registers[rn] = address;
    }
    Ok(if load { "ldm" } else { "stm" })
}

fn condition_holds(state: &CortexM23State, condition: u8) -> bool {
    let n = u8::from(flag(state, APSR_N));
    let z = u8::from(flag(state, APSR_Z));
    let c = u8::from(flag(state, APSR_C));
    let v = u8::from(flag(state, APSR_V));
    let n_xor_v = xor_gate(n, v);
    let predicates = [
        z,
        not_gate(z),
        c,
        not_gate(c),
        n,
        not_gate(n),
        v,
        not_gate(v),
        and_gate(c, not_gate(z)),
        or_gate(not_gate(c), z),
        not_gate(n_xor_v),
        n_xor_v,
        and_gate(not_gate(z), not_gate(n_xor_v)),
        or_gate(z, n_xor_v),
    ];
    let lines = std::array::from_fn(|index| gates::selected(u32::from(condition), index as u32, 4));
    let signal = gates::select_one_hot_bit(predicates, lines);
    signal == 1
}

fn condition_mnemonic(condition: u8) -> &'static str {
    [
        "beq", "bne", "bcs", "bcc", "bmi", "bpl", "bvs", "bvc", "bhi", "bls", "bge", "blt", "bgt",
        "ble",
    ][condition as usize]
}

fn sign_extend(value: u32, bits: u32) -> u32 {
    gates::sign_extend(value, bits as usize)
}

fn execute_conditional(
    state: &mut CortexM23State,
    raw: u16,
    pc: u32,
) -> Result<&'static str, CoreError> {
    let condition = field16(raw, 8, 4) as u8;
    if condition == 15 {
        return Err(CoreError::SupervisorCall {
            immediate: field16(raw, 0, 8) as u8,
            pc,
        });
    }
    if condition == 14 {
        return Err(CoreError::UnknownInstruction16 { raw, pc });
    }
    if condition_holds(state, condition) {
        let offset = gates::shift_left_32(gates::sign_extend(u32::from(field16(raw, 0, 8)), 8), 1);
        state.pc = gates::add_32(gates::add_32(pc, 4, 0).0, offset, 0).0;
    }
    Ok(condition_mnemonic(condition))
}

fn execute_unconditional(
    state: &mut CortexM23State,
    raw: u16,
    pc: u32,
) -> Result<&'static str, CoreError> {
    let offset = gates::shift_left_32(sign_extend(u32::from(field16(raw, 0, 11)), 11), 1);
    state.pc = gates::add_32(gates::add_32(pc, 4, 0).0, offset, 0).0;
    Ok("b")
}

fn execute_bl(
    state: &mut CortexM23State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CoreError> {
    let s = u32::from(field16(first, 10, 1));
    let j1 = u32::from(field16(second, 13, 1));
    let j2 = u32::from(field16(second, 11, 1));
    let i1 = u32::from(not_gate(xor_gate(j1 as u8, s as u8)));
    let i2 = u32::from(not_gate(xor_gate(j2 as u8, s as u8)));
    let immediate = [
        gates::shift_left_32(s, 24),
        gates::shift_left_32(i1, 23),
        gates::shift_left_32(i2, 22),
        gates::shift_left_32(u32::from(field16(first, 0, 10)), 12),
        gates::shift_left_32(u32::from(field16(second, 0, 11)), 1),
    ]
    .into_iter()
    .fold(0, gates::bit_or_32);
    state.lr = gates::bit_or_32(gates::add_32(pc, 4, 0).0, 1);
    state.pc = gates::add_32(gates::add_32(pc, 4, 0).0, sign_extend(immediate, 25), 0).0;
    Ok("bl")
}

fn execute_barrier(second: u16) -> Result<&'static str, CoreError> {
    let dsb = line16(second, 0, 16, 0x8f4f);
    let dmb = line16(second, 0, 16, 0x8f5f);
    if dsb == 1 {
        return Ok("dsb");
    }
    if dmb == 1 {
        return Ok("dmb");
    }
    Ok("isb")
}

fn special_lines(sysm: u8) -> [u8; 11] {
    [0_u32, 1, 2, 3, 5, 6, 7, 8, 9, 16, 20].map(|value| gates::selected(u32::from(sysm), value, 8))
}

fn read_special(state: &CortexM23State, sysm: u8) -> (u32, u8) {
    let lines = special_lines(sysm);
    let xpsr = gates::bit_or_32(state.apsr, u32::from(state.ipsr));
    let value = gates::select_one_hot_32(
        [
            state.apsr,
            xpsr,
            state.apsr,
            xpsr,
            u32::from(state.ipsr),
            0,
            u32::from(state.ipsr),
            state.msp_non_secure,
            state.psp_non_secure,
            u32::from(state.primask_non_secure),
            u32::from(state.control_non_secure),
        ],
        lines,
    );
    (value, gates::any_line(lines))
}

fn execute_special_register(
    state: &mut CortexM23State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CoreError> {
    let sysm = field16(second, 0, 8) as u8;
    let lines = special_lines(sysm);
    let mrs_line = line16(first, 0, 16, 0xf3ef);
    if mrs_line == 1 {
        let rd = field16(second, 8, 4) as u8;
        let rd_valid = gates::less_or_equal_unsigned_32(u32::from(rd), 12);
        if rd_valid == 0 {
            return Err(CoreError::UnknownInstruction32 { first, second, pc });
        }
        let (mut value, special_valid) = read_special(state, sysm);
        if special_valid == 0 {
            return Err(CoreError::UnknownInstruction32 { first, second, pc });
        }
        let unprivileged = and_gate(
            gates::u32_to_bits(u32::from(state.control_non_secure))[0],
            not_gate(u8::from(state.handler_mode)),
        );
        let unprivileged_visible = or_gate(lines[0], lines[10]);
        let unprivileged_value = gates::select_32(0, value, unprivileged_visible);
        value = gates::select_32(value, unprivileged_value, unprivileged);
        state.registers[rd as usize] = value;
        return Ok("mrs");
    }
    let rn = field16(first, 0, 4) as u8;
    let rn_valid = gates::less_or_equal_unsigned_32(u32::from(rn), 12);
    let special_valid = gates::any_line(lines);
    if and_gate(rn_valid, special_valid) == 0 {
        return Err(CoreError::UnknownInstruction32 { first, second, pc });
    }
    let value = state.registers[rn as usize];
    let apsr_write = gates::any_line([lines[0], lines[1], lines[2], lines[3]]);
    if apsr_write == 1 {
        state.apsr = gates::bit_and_32(value, APSR_MASK);
        return Ok("msr");
    }
    let unprivileged = and_gate(
        gates::u32_to_bits(u32::from(state.control_non_secure))[0],
        not_gate(u8::from(state.handler_mode)),
    );
    let read_only = gates::any_line([lines[4], lines[5], lines[6]]);
    if or_gate(read_only, unprivileged) == 1 {
        return Ok("msr");
    }
    let aligned = gates::bit_and_32(value, gates::bit_not_32(3));
    if lines[7] == 1 {
        state.msp_non_secure = aligned;
    }
    if lines[8] == 1 {
        state.psp_non_secure = aligned;
    }
    if lines[9] == 1 {
        state.primask_non_secure = gates::u32_to_bits(value)[0] == 1;
    }
    if lines[10] == 1 {
        let requested = gates::field(value, 0, 2) as u8;
        let handler_value = gates::bit_or_32(
            gates::bit_and_32(u32::from(state.control_non_secure), 2),
            gates::bit_and_32(u32::from(requested), 1),
        );
        state.control_non_secure = gates::select_32(
            u32::from(requested),
            handler_value,
            u8::from(state.handler_mode),
        ) as u8;
    }
    Ok("msr")
}

/// Little-endian instruction encoding helpers for tests and downstream users.
pub mod encoding {
    #[must_use]
    pub const fn halfword(value: u16) -> [u8; 2] {
        value.to_le_bytes()
    }

    #[must_use]
    pub fn program(halfwords: &[u16]) -> Vec<u8> {
        halfwords
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect()
    }

    #[must_use]
    pub const fn movs(rd: u16, immediate: u16) -> u16 {
        0x2000 | ((rd & 7) << 8) | (immediate & 0xff)
    }

    #[must_use]
    pub const fn adds_imm(rd: u16, immediate: u16) -> u16 {
        0x3000 | ((rd & 7) << 8) | (immediate & 0xff)
    }

    #[must_use]
    pub const fn subs_imm(rd: u16, immediate: u16) -> u16 {
        0x3800 | ((rd & 7) << 8) | (immediate & 0xff)
    }

    #[must_use]
    pub const fn str_imm(rt: u16, rn: u16, words: u16) -> u16 {
        0x6000 | ((words & 0x1f) << 6) | ((rn & 7) << 3) | (rt & 7)
    }

    #[must_use]
    pub const fn ldr_imm(rt: u16, rn: u16, words: u16) -> u16 {
        0x6800 | ((words & 0x1f) << 6) | ((rn & 7) << 3) | (rt & 7)
    }

    #[must_use]
    pub const fn b_cond(condition: u16, halfwords: i8) -> u16 {
        0xd000 | ((condition & 0xf) << 8) | halfwords as u8 as u16
    }

    #[must_use]
    pub const fn branch(halfwords: i16) -> u16 {
        0xe000 | ((halfwords as u16) & 0x7ff)
    }

    #[must_use]
    pub const fn bkpt(immediate: u8) -> u16 {
        0xbe00 | immediate as u16
    }

    #[must_use]
    pub const fn nop() -> u16 {
        0xbf00
    }

    #[must_use]
    pub fn bl(offset_bytes: i32) -> [u16; 2] {
        let immediate = offset_bytes as u32 & 0x01ff_fffe;
        let s = (immediate >> 24) & 1;
        let i1 = (immediate >> 23) & 1;
        let i2 = (immediate >> 22) & 1;
        let j1 = (!(i1 ^ s)) & 1;
        let j2 = (!(i2 ^ s)) & 1;
        [
            0xf000 | ((s as u16) << 10) | ((immediate >> 12) as u16 & 0x03ff),
            0xd000 | ((j1 as u16) << 13) | ((j2 as u16) << 11) | (immediate as u16 & 0x07ff),
        ]
    }

    #[must_use]
    pub const fn mrs(rd: u16, sysm: u8) -> [u16; 2] {
        [0xf3ef, 0x8000 | ((rd & 0xf) << 8) | sysm as u16]
    }

    #[must_use]
    pub const fn msr(rn: u16, sysm: u8) -> [u16; 2] {
        [0xf380 | (rn & 0xf), 0x8800 | sysm as u16]
    }
}
