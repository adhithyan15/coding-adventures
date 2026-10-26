//! Independent Armv7-M gate transition engine.

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

const APSR_N: u32 = 1 << 31;
const APSR_Z: u32 = 1 << 30;
const APSR_C: u32 = 1 << 29;
const APSR_V: u32 = 1 << 28;
const APSR_Q: u32 = 1 << 27;
const APSR_MASK: u32 = APSR_N | APSR_Z | APSR_C | APSR_V | APSR_Q;

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

/// Broad Armv7-M decode family.
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
    Thumb2Data,
    Thumb2Memory,
    Exclusive,
    TableBranch,
}

/// Complete restorable state at an instruction boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CortexM3State {
    /// R0-R12.
    pub registers: [u32; 13],
    pub msp: u32,
    pub psp: u32,
    pub lr: u32,
    pub pc: u32,
    /// N/Z/C/V/Q flags.
    pub apsr: u32,
    pub ipsr: u8,
    /// Architectural ITSTATE encoding, or zero outside an IT block.
    pub itstate: u8,
    pub primask: bool,
    pub basepri: u8,
    pub faultmask: bool,
    /// nPRIV is bit 0 and SPSEL is bit 1.
    pub control: u8,
    pub handler_mode: bool,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub sleeping: bool,
    /// True only while a sleeping core is blocked in WFE rather than WFI.
    pub waiting_for_event: bool,
    pub event_registered: bool,
    /// Address held by the local exclusive monitor.
    pub exclusive_address: Option<u32>,
    pub loaded_origin: u32,
    pub loaded_len: usize,
}

impl Default for CortexM3State {
    fn default() -> Self {
        Self {
            registers: [0; 13],
            msp: MEMORY_SIZE as u32,
            psp: MEMORY_SIZE as u32,
            lr: 0,
            pc: 0,
            apsr: 0,
            ipsr: 0,
            itstate: 0,
            primask: false,
            basepri: 0,
            faultmask: false,
            control: 0,
            handler_mode: false,
            memory: vec![0; MEMORY_SIZE],
            halted: false,
            sleeping: false,
            waiting_for_event: false,
            event_registered: false,
            exclusive_address: None,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }
}

/// Typed lifecycle, decode, and execution failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CortexM3Error {
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
    UnsupportedExceptionReturn { target: u32 },
    UnknownInstruction16 { raw: u16, pc: u32 },
    UnknownInstruction32 { first: u16, second: u16, pc: u32 },
    UnsupportedBreakpoint { immediate: u8, pc: u32 },
    SupervisorCall { immediate: u8, pc: u32 },
    RetentionLimitExceeded { requested: usize, limit: usize },
    StepLimitExceeded { limit: usize },
    AllocationFailed,
}

impl fmt::Display for CortexM3Error {
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
            Self::UnsupportedExceptionReturn { target } => write!(
                formatter,
                "exception-return target {target:#010x} is outside this core boundary"
            ),
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

impl std::error::Error for CortexM3Error {}

/// One committed architectural transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: [u8; 4],
    pub width: usize,
    pub mnemonic: &'static str,
    pub state_before: CortexM3State,
    pub state_after: CortexM3State,
}

/// Successful bounded execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub stopped: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: CortexM3State,
}

pub(crate) fn step_state(state: CortexM3State) -> Result<StepTrace, CortexM3Error> {
    if state.halted {
        return Err(CortexM3Error::Halted);
    }
    if state.sleeping {
        return Err(CortexM3Error::Sleeping);
    }
    let before_memory = try_memory_buffer(state.memory.len())?;
    let mut validation_memory = try_memory_buffer(state.memory.len())?;
    step_state_preallocated(state, before_memory, &mut validation_memory)
}

pub(crate) fn step_state_preallocated(
    mut state: CortexM3State,
    before_memory: Vec<u8>,
    validation_memory: &mut Vec<u8>,
) -> Result<StepTrace, CortexM3Error> {
    if state.halted {
        return Err(CortexM3Error::Halted);
    }
    if state.sleeping {
        return Err(CortexM3Error::Sleeping);
    }
    let before = clone_state_into(&state, before_memory);
    let pc = state.pc;
    let first = fetch16(&state, pc)?;
    let width = instruction_width(first);
    let second = if width == 4 {
        Some(
            fetch16(&state, crate::gates::add_32(pc, 2, 0).0).map_err(|error| match error {
                CortexM3Error::FetchOutsideProgram { .. } => {
                    CortexM3Error::FetchOutsideProgram { pc, width: 4 }
                }
                other => other,
            })?,
        )
    } else {
        None
    };
    let class = classify_at(first, second, pc)?;
    state.pc = crate::gates::add_32(pc, width as u32, 0).0;
    let sequential_pc = state.pc;
    let mnemonic = if before.itstate != 0 {
        let condition = before.itstate >> 4;
        validate_it_member(first, class, before.itstate, pc)?;
        let executes_unconditionally = line16(first, 8, 8, 0xbe) == 1;
        let mnemonic = if executes_unconditionally || condition_holds(&before, condition) {
            let mnemonic = execute_decoded(&mut state, first, second, class, pc)?;
            if suppresses_flags_in_it(first, class) {
                state.apsr = before.apsr;
            }
            if gates::equal_32(state.pc, sequential_pc) == 0 && advance_itstate(before.itstate) != 0
            {
                return Err(CortexM3Error::UnknownInstruction16 { raw: first, pc });
            }
            mnemonic
        } else {
            let mut validation = clone_state_into(&state, core::mem::take(validation_memory));
            let validation_result = execute_decoded(&mut validation, first, second, class, pc);
            *validation_memory = core::mem::take(&mut validation.memory);
            if let Err(error) = validation_result {
                let conditional_undefined =
                    class == InstructionClass::ConditionalBranch && field16(first, 8, 4) == 14;
                if !conditional_undefined
                    && matches!(
                        error,
                        CortexM3Error::UnknownInstruction16 { .. }
                            | CortexM3Error::UnknownInstruction32 { .. }
                    )
                {
                    return Err(error);
                }
            }
            "it-skip"
        };
        state.itstate = if gates::equal_32(state.pc, sequential_pc) == 1 {
            advance_itstate(before.itstate)
        } else {
            0
        };
        mnemonic
    } else {
        execute_decoded(&mut state, first, second, class, pc)?
    };
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
        state_before: before,
        state_after: state,
    })
}

fn validate_it_member(
    first: u16,
    class: InstructionClass,
    itstate: u8,
    pc: u32,
) -> Result<(), CortexM3Error> {
    let forbidden = first & 0xf500 == 0xb100
        || first & 0xffc0 == 0
        || matches!(first, 0xb661..=0xb663 | 0xb671..=0xb673)
        || first & 0xff00 == 0xbf00 && first & 0xf != 0
        || class == InstructionClass::ConditionalBranch
            && (first >> 8) & 0xf < 14
            && (first >> 8) & 0xf != u16::from(itstate >> 4);
    if forbidden || advance_itstate(itstate) != 0 && instruction_writes_pc(first, class) {
        return Err(CortexM3Error::UnknownInstruction16 { raw: first, pc });
    }
    Ok(())
}

fn instruction_writes_pc(first: u16, class: InstructionClass) -> bool {
    if class == InstructionClass::ConditionalBranch {
        return (first >> 8) & 0xf < 14;
    }
    if matches!(
        class,
        InstructionClass::UnconditionalBranch
            | InstructionClass::BranchLink
            | InstructionClass::TableBranch
    ) {
        return true;
    }
    if class == InstructionClass::SpecialData {
        let operation = (first >> 8) & 3;
        let rd = (((first >> 7) & 1) << 3) | (first & 7);
        return operation == 3 || matches!(operation, 0 | 2) && rd == u16::from(PC);
    }
    class == InstructionClass::Miscellaneous && first & 0xfe00 == 0xbc00 && first & 0x100 != 0
}

fn suppresses_flags_in_it(first: u16, class: InstructionClass) -> bool {
    match class {
        InstructionClass::ShiftAddSubtract => true,
        InstructionClass::Immediate => (first >> 11) & 3 != 1,
        InstructionClass::DataProcessing => !matches!((first >> 6) & 0xf, 8 | 10 | 11),
        _ => false,
    }
}

fn try_memory_buffer(len: usize) -> Result<Vec<u8>, CortexM3Error> {
    let mut memory = Vec::new();
    memory
        .try_reserve_exact(len)
        .map_err(|_| CortexM3Error::AllocationFailed)?;
    memory.resize(len, 0);
    Ok(memory)
}

pub(crate) fn clone_state_into(state: &CortexM3State, mut memory: Vec<u8>) -> CortexM3State {
    memory.clear();
    memory.extend_from_slice(&state.memory);
    CortexM3State {
        registers: state.registers,
        msp: state.msp,
        psp: state.psp,
        lr: state.lr,
        pc: state.pc,
        apsr: state.apsr,
        ipsr: state.ipsr,
        itstate: state.itstate,
        primask: state.primask,
        basepri: state.basepri,
        faultmask: state.faultmask,
        control: state.control,
        handler_mode: state.handler_mode,
        memory,
        halted: state.halted,
        sleeping: state.sleeping,
        waiting_for_event: state.waiting_for_event,
        event_registered: state.event_registered,
        exclusive_address: state.exclusive_address,
        loaded_origin: state.loaded_origin,
        loaded_len: state.loaded_len,
    }
}

pub(crate) fn validate_program(program: &[u8], origin: u32) -> Result<(), CortexM3Error> {
    if program.is_empty() {
        return Err(CortexM3Error::EmptyProgram);
    }
    if origin & 1 != 0 {
        return Err(CortexM3Error::MisalignedProgram { origin });
    }
    if usize::try_from(origin)
        .ok()
        .and_then(|start| start.checked_add(program.len()))
        .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(CortexM3Error::ProgramOutOfRange {
            origin,
            len: program.len(),
        });
    }
    Ok(())
}

pub(crate) fn validate_state(state: &CortexM3State) -> Result<(), CortexM3Error> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(CortexM3Error::InvalidState {
            reason: "memory must be exactly 64 KiB",
        });
    }
    if state.pc & 1 != 0 || state.msp & 3 != 0 || state.psp & 3 != 0 {
        return Err(CortexM3Error::InvalidState {
            reason: "PC or stack pointer alignment is invalid",
        });
    }
    if state.apsr & !APSR_MASK != 0
        || state.control & !3 != 0
        || (state.itstate != 0
            && (state.itstate & 0xf == 0
                || state.itstate >> 4 == 0xf
                || state.itstate >> 4 == 0xe && !(state.itstate & 0xf).is_power_of_two()))
    {
        return Err(CortexM3Error::InvalidState {
            reason: "reserved status bits are set",
        });
    }
    if !matches!(state.ipsr, 0 | 2..=6 | 11..=12 | 14..=15 | 16..=u8::MAX) {
        return Err(CortexM3Error::InvalidState {
            reason: "IPSR contains a reserved exception number",
        });
    }
    if state.handler_mode != (state.ipsr != 0) {
        return Err(CortexM3Error::InvalidState {
            reason: "mode and IPSR disagree",
        });
    }
    if state.handler_mode && state.control & 2 != 0 {
        return Err(CortexM3Error::InvalidState {
            reason: "handler mode cannot select the process stack",
        });
    }
    if state.waiting_for_event && (!state.sleeping || state.event_registered) {
        return Err(CortexM3Error::InvalidState {
            reason: "event-wait state is inconsistent",
        });
    }
    if state.halted && state.sleeping {
        return Err(CortexM3Error::InvalidState {
            reason: "a core cannot be halted and sleeping",
        });
    }
    if let Some(address) = state.exclusive_address {
        if address & 3 != 0
            || usize::try_from(address)
                .ok()
                .and_then(|start| start.checked_add(4))
                .is_none_or(|end| end > MEMORY_SIZE)
        {
            return Err(CortexM3Error::InvalidState {
                reason: "exclusive monitor address is invalid",
            });
        }
    }
    if state.loaded_len == 0 {
        if state.loaded_origin != 0 {
            return Err(CortexM3Error::InvalidState {
                reason: "empty image must use origin zero",
            });
        }
    } else if state.loaded_origin & 1 != 0
        || usize::try_from(state.loaded_origin)
            .ok()
            .and_then(|start| start.checked_add(state.loaded_len))
            .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(CortexM3Error::InvalidState {
            reason: "installed image range is invalid",
        });
    }
    Ok(())
}

fn active_sp(state: &CortexM3State) -> u32 {
    let select = and_gate(
        not_gate(u8::from(state.handler_mode)),
        gates::u8_to_bits(state.control)[1],
    );
    gates::select_32(state.msp, state.psp, select)
}

fn set_active_sp(state: &mut CortexM3State, value: u32) {
    if and_gate(
        not_gate(u8::from(state.handler_mode)),
        gates::u8_to_bits(state.control)[1],
    ) == 1
    {
        state.psp = value;
    } else {
        state.msp = value;
    }
}

pub(crate) fn read_register(state: &CortexM3State, index: u8) -> Result<u32, CortexM3Error> {
    match index {
        0..=12 => Ok(state.registers[index as usize]),
        SP => Ok(active_sp(state)),
        LR => Ok(state.lr),
        PC => Ok(state.pc),
        _ => Err(CortexM3Error::InvalidRegister { index }),
    }
}

pub(crate) fn write_register(
    state: &mut CortexM3State,
    index: u8,
    value: u32,
) -> Result<(), CortexM3Error> {
    match index {
        0..=12 => state.registers[index as usize] = value,
        SP => set_active_sp(state, gates::bit_and_32(value, gates::bit_not_32(3))),
        LR => state.lr = value,
        PC => state.pc = gates::bit_and_32(value, gates::bit_not_32(1)),
        _ => return Err(CortexM3Error::InvalidRegister { index }),
    }
    Ok(())
}

pub(crate) fn checked_range(
    address: u32,
    width: usize,
) -> Result<std::ops::Range<usize>, CortexM3Error> {
    if !matches!(width, 1 | 2 | 4) {
        return Err(CortexM3Error::DataOutOfRange { address, width });
    }
    let start = address as usize;
    let Some(end) = start.checked_add(width) else {
        return Err(CortexM3Error::DataOutOfRange { address, width });
    };
    if end > MEMORY_SIZE {
        return Err(CortexM3Error::DataOutOfRange { address, width });
    }
    Ok(start..end)
}

fn checked_aligned_range(
    address: u32,
    width: usize,
) -> Result<std::ops::Range<usize>, CortexM3Error> {
    let bits = gates::u32_to_bits(address);
    let misaligned = match width {
        1 => 0,
        2 => bits[0],
        4 => or_gate(bits[0], bits[1]),
        _ => 1,
    };
    if misaligned == 1 {
        return Err(CortexM3Error::MisalignedData { address, width });
    }
    checked_range(address, width)
}

pub(crate) fn read_memory(
    state: &CortexM3State,
    address: u32,
    width: usize,
) -> Result<u32, CortexM3Error> {
    let range = checked_range(address, width)?;
    let mut bytes = [0; 4];
    bytes[..width].copy_from_slice(&state.memory[range]);
    Ok(gates::assemble_le_u32(bytes))
}

pub(crate) fn write_memory(
    state: &mut CortexM3State,
    address: u32,
    width: usize,
    value: u32,
) -> Result<(), CortexM3Error> {
    let range = checked_range(address, width)?;
    state.memory[range].copy_from_slice(&gates::split_le_u32(value)[..width]);
    state.exclusive_address = None;
    Ok(())
}

fn fetch16(state: &CortexM3State, pc: u32) -> Result<u16, CortexM3Error> {
    if gates::u32_to_bits(pc)[0] == 1 {
        return Err(CortexM3Error::MisalignedFetch { pc });
    }
    if state.loaded_len < 2 {
        return Err(CortexM3Error::FetchOutsideProgram { pc, width: 2 });
    }
    let relative = pc.checked_sub(state.loaded_origin);
    if relative.is_none_or(|offset| offset > state.loaded_len.saturating_sub(2) as u32) {
        return Err(CortexM3Error::FetchOutsideProgram { pc, width: 2 });
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
pub fn classify(first: u16, second: Option<u16>) -> Result<InstructionClass, CortexM3Error> {
    classify_at(first, second, 0)
}

pub(crate) fn classify_at(
    first: u16,
    second: Option<u16>,
    pc: u32,
) -> Result<InstructionClass, CortexM3Error> {
    if instruction_width(first) == 4 {
        let Some(second) = second else {
            return Err(CortexM3Error::FetchOutsideProgram { pc, width: 4 });
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
        let clrex = and_gate(line16(first, 0, 16, 0xf3bf), line16(second, 0, 16, 0x8f2f));
        let exclusive_family = [0xe840_u16, 0xe850]
            .into_iter()
            .fold(0, |signal, expected| {
                or_gate(signal, line16(first, 4, 12, expected >> 4))
            });
        if or_gate(clrex, exclusive_family) == 1 {
            return Ok(InstructionClass::Exclusive);
        }
        if and_gate(line16(first, 4, 12, 0xe8d), line16(second, 5, 11, 0x780)) == 1 {
            return Ok(InstructionClass::TableBranch);
        }
        let mrs = and_gate(line16(first, 0, 16, 0xf3ef), line16(second, 12, 4, 8));
        let msr = and_gate(line16(first, 4, 12, 0xf38), line16(second, 8, 8, 0x88));
        if or_gate(mrs, msr) == 1 {
            return Ok(InstructionClass::SpecialRegister);
        }
        let thumb2_memory = [
            0xf880_u16, 0xf890, 0xf8a0, 0xf8b0, 0xf8c0, 0xf8d0, 0xf990, 0xf9b0,
        ]
        .into_iter()
        .fold(0, |signal, expected| {
            or_gate(signal, line16(first, 4, 12, expected >> 4))
        });
        if thumb2_memory == 1 {
            return Ok(InstructionClass::Thumb2Memory);
        }
        let mov_wide_key = gates::bit_and_32(u32::from(first), 0xfbf0);
        let mov_wide = or_gate(
            gates::selected(mov_wide_key, 0xf240, 16),
            gates::selected(mov_wide_key, 0xf2c0, 16),
        );
        let thumb2_register = [0xfb90_u16, 0xfbb0, 0xfa90, 0xfab0]
            .into_iter()
            .fold(0, |signal, expected| {
                or_gate(signal, line16(first, 4, 12, expected >> 4))
            });
        if or_gate(mov_wide, thumb2_register) == 1 {
            return Ok(InstructionClass::Thumb2Data);
        }
        return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
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
    Err(CortexM3Error::UnknownInstruction16 { raw: first, pc })
}

fn execute_decoded(
    state: &mut CortexM3State,
    first: u16,
    second: Option<u16>,
    class: InstructionClass,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
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
        InstructionClass::Thumb2Data => {
            execute_thumb2_data(state, first, second.expect("classified"), pc)
        }
        InstructionClass::Thumb2Memory => {
            execute_thumb2_memory(state, first, second.expect("classified"), pc)
        }
        InstructionClass::Exclusive => {
            execute_exclusive(state, first, second.expect("classified"), pc)
        }
        InstructionClass::TableBranch => {
            execute_table_branch(state, first, second.expect("classified"), pc)
        }
    }
}

fn flag(state: &CortexM3State, mask: u32) -> bool {
    gates::bit_and_32(state.apsr, mask) != 0
}

fn set_flag(state: &mut CortexM3State, mask: u32, value: bool) {
    let cleared = gates::bit_and_32(state.apsr, gates::bit_not_32(mask));
    let asserted = gates::bit_or_32(cleared, mask);
    state.apsr = gates::select_32(cleared, asserted, u8::from(value));
}

fn set_nz(state: &mut CortexM3State, value: u32) {
    set_flag(state, APSR_N, gates::u32_to_bits(value)[31] == 1);
    set_flag(state, APSR_Z, gates::is_zero_32(value) == 1);
}

fn add_with_carry(state: &mut CortexM3State, left: u32, right: u32, carry: bool) -> u32 {
    let (result, carry_out, overflow) = gates::add_32(left, right, u8::from(carry));
    set_nz(state, result);
    set_flag(state, APSR_C, carry_out == 1);
    set_flag(state, APSR_V, overflow == 1);
    result
}

fn subtract(state: &mut CortexM3State, left: u32, right: u32, borrow: bool) -> u32 {
    let (result, borrowed, overflow) = gates::sub_32(left, right, u8::from(borrow));
    set_nz(state, result);
    set_flag(state, APSR_C, borrowed == 0);
    set_flag(state, APSR_V, overflow == 1);
    result
}

fn shift_immediate(value: u32, kind: u8, encoded: u32, carry: bool) -> (u32, bool) {
    match kind {
        0 if encoded == 0 => (value, carry),
        0 => (
            gates::shift_left_32(value, encoded),
            gates::u32_to_bits(value)[(32 - encoded) as usize] == 1,
        ),
        1 => {
            let amount = if encoded == 0 { 32 } else { encoded };
            (
                if amount == 32 {
                    0
                } else {
                    gates::shift_right_32(value, amount)
                },
                gates::u32_to_bits(value)[(amount - 1) as usize] == 1,
            )
        }
        _ => {
            let amount = if encoded == 0 { 32 } else { encoded };
            let result = if amount >= 32 {
                gates::select_32(0, u32::MAX, gates::u32_to_bits(value)[31])
            } else {
                gates::shift_arithmetic_right_32(value, amount)
            };
            (
                result,
                gates::u32_to_bits(value)[(amount.min(32) - 1) as usize] == 1,
            )
        }
    }
}

fn shift_register(value: u32, kind: u8, amount: u32, carry: bool) -> (u32, bool) {
    if amount == 0 {
        return (value, carry);
    }
    match kind {
        0 if amount < 32 => (
            gates::shift_left_32(value, amount),
            gates::u32_to_bits(value)[(32 - amount) as usize] == 1,
        ),
        0 if amount == 32 => (0, gates::u32_to_bits(value)[0] == 1),
        0 => (0, false),
        1 if amount < 32 => (
            gates::shift_right_32(value, amount),
            gates::u32_to_bits(value)[(amount - 1) as usize] == 1,
        ),
        1 if amount == 32 => (0, gates::u32_to_bits(value)[31] == 1),
        1 => (0, false),
        2 => {
            let result = if amount >= 32 {
                gates::select_32(0, u32::MAX, gates::u32_to_bits(value)[31])
            } else {
                gates::shift_arithmetic_right_32(value, amount)
            };
            (
                result,
                gates::u32_to_bits(value)[(amount.min(32) - 1) as usize] == 1,
            )
        }
        _ => {
            let rotation = gates::bit_and_32(amount, 31);
            let result = gates::rotate_right_32(value, rotation);
            (result, gates::u32_to_bits(result)[31] == 1)
        }
    }
}

fn execute_shift_add_sub(
    state: &mut CortexM3State,
    raw: u16,
) -> Result<&'static str, CortexM3Error> {
    let operation = field16(raw, 11, 2);
    if operation < 3 {
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
        return Ok(["lsls", "lsrs", "asrs"][operation as usize]);
    }
    let op = field16(raw, 9, 2);
    let operand = field16(raw, 6, 3) as usize;
    let rn = field16(raw, 3, 3) as usize;
    let rd = field16(raw, 0, 3) as usize;
    let right = if op < 2 {
        state.registers[operand]
    } else {
        operand as u32
    };
    let result = if op & 1 == 0 {
        add_with_carry(state, state.registers[rn], right, false)
    } else {
        subtract(state, state.registers[rn], right, false)
    };
    state.registers[rd] = result;
    Ok(if op & 1 == 0 { "adds" } else { "subs" })
}

fn execute_immediate(state: &mut CortexM3State, raw: u16) -> Result<&'static str, CortexM3Error> {
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
    state: &mut CortexM3State,
    raw: u16,
) -> Result<&'static str, CortexM3Error> {
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
            let kind = match operation {
                2 => 0,
                3 => 1,
                4 => 2,
                _ => 3,
            };
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

fn branch_exchange(state: &mut CortexM3State, target: u32) -> Result<(), CortexM3Error> {
    if gates::bit_and_32(target, 1) == 0 {
        return Err(CortexM3Error::InvalidThumbTarget { target });
    }
    state.pc = gates::bit_and_32(target, gates::bit_not_32(1));
    Ok(())
}

fn bx_write_pc(state: &mut CortexM3State, target: u32) -> Result<(), CortexM3Error> {
    let exception_return = and_gate(
        u8::from(state.handler_mode),
        gates::selected(gates::field(target, 28, 4), 0xf, 4),
    );
    if exception_return == 1 {
        return Err(CortexM3Error::UnsupportedExceptionReturn { target });
    }
    branch_exchange(state, target)
}

fn branch_write_pc(state: &mut CortexM3State, target: u32) {
    state.pc = gates::bit_and_32(target, gates::bit_not_32(1));
}

fn execute_special(
    state: &mut CortexM3State,
    raw: u16,
    instruction_pc: u32,
) -> Result<&'static str, CortexM3Error> {
    let operation = field16(raw, 8, 2);
    let rm = field16(raw, 3, 4) as u8;
    let rd = gates::bit_or_32(
        gates::shift_left_32(u32::from(field16(raw, 7, 1)), 3),
        u32::from(field16(raw, 0, 3)),
    ) as u8;
    if operation == 3 {
        let link = line16(raw, 7, 1, 1);
        if field16(raw, 0, 3) != 0 || link == 1 && rm == PC {
            return Err(CortexM3Error::UnknownInstruction16 {
                raw,
                pc: instruction_pc,
            });
        }
        let target = if rm == PC {
            gates::add_32(instruction_pc, 4, 0).0
        } else {
            read_register(state, rm)?
        };
        if link == 1 {
            state.lr = gates::bit_or_32(state.pc, 1);
            branch_exchange(state, target)?;
        } else {
            bx_write_pc(state, target)?;
        }
        return Ok(if link == 1 { "blx" } else { "bx" });
    }
    if operation == 1 && (rd < 8 && rm < 8 || rd == PC || rm == PC)
        || operation == 0 && rd == PC && rm == PC
    {
        return Err(CortexM3Error::UnknownInstruction16 {
            raw,
            pc: instruction_pc,
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
    state: &mut CortexM3State,
    raw: u16,
    instruction_pc: u32,
) -> Result<&'static str, CortexM3Error> {
    let rt = field16(raw, 8, 3) as usize;
    let visible = gates::bit_and_32(gates::add_32(instruction_pc, 4, 0).0, gates::bit_not_32(3));
    let offset = gates::shift_left_32(u32::from(field16(raw, 0, 8)), 2);
    let address = gates::add_32(visible, offset, 0).0;
    state.registers[rt] = read_memory(state, address, 4)?;
    Ok("ldr")
}

fn execute_register_memory(
    state: &mut CortexM3State,
    raw: u16,
) -> Result<&'static str, CortexM3Error> {
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
            state.registers[rt] = gates::sign_extend_8(read_memory(state, address, 1)? as u8);
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
            state.registers[rt] = gates::sign_extend_16(read_memory(state, address, 2)? as u16);
            "ldrsh"
        }
    })
}

fn execute_immediate_memory(
    state: &mut CortexM3State,
    raw: u16,
) -> Result<&'static str, CortexM3Error> {
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

fn execute_stack_memory(
    state: &mut CortexM3State,
    raw: u16,
) -> Result<&'static str, CortexM3Error> {
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
    state: &mut CortexM3State,
    raw: u16,
    instruction_pc: u32,
) -> Result<&'static str, CortexM3Error> {
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
    state: &mut CortexM3State,
    raw: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
    if gates::selected(gates::bit_and_32(u32::from(raw), 0xf500), 0xb100, 16) == 1 {
        let rn = field16(raw, 0, 3) as usize;
        let nonzero = line16(raw, 11, 1, 1);
        let immediate = gates::bit_or_32(
            gates::shift_left_32(u32::from(field16(raw, 9, 1)), 6),
            gates::shift_left_32(u32::from(field16(raw, 3, 5)), 1),
        );
        let register_nonzero = not_gate(gates::is_zero_32(state.registers[rn]));
        if xor_gate(register_nonzero, nonzero) == 0 {
            state.pc = gates::add_32(gates::add_32(pc, 4, 0).0, immediate, 0).0;
            state.itstate = 0;
        }
        return Ok(if nonzero == 1 { "cbnz" } else { "cbz" });
    }
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
            0 => gates::sign_extend_16(gates::bit_and_32(value, 0xffff) as u16),
            1 => gates::sign_extend_8(gates::bit_and_32(value, 0xff) as u8),
            2 => gates::bit_and_32(value, 0xffff),
            _ => gates::bit_and_32(value, 0xff),
        };
        return Ok(["sxth", "sxtb", "uxth", "uxtb"][operation as usize]);
    }
    if line16(raw, 9, 7, 0x5a) == 1 {
        let mask = field16(raw, 0, 8);
        let include_lr = line16(raw, 8, 1, 1) == 1;
        if mask == 0 && !include_lr {
            return Err(CortexM3Error::UnknownInstruction16 { raw, pc });
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
            return Err(CortexM3Error::UnknownInstruction16 { raw, pc });
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
            bx_write_pc(state, target)?;
        }
        set_active_sp(state, address);
        return Ok("pop");
    }
    if line16(raw, 8, 8, 0xba) == 1 {
        let operation = field16(raw, 6, 2);
        if operation == 2 {
            return Err(CortexM3Error::UnknownInstruction16 { raw, pc });
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
    if matches!(raw, 0xb661..=0xb663 | 0xb671..=0xb673) {
        if or_gate(
            not_gate(gates::u8_to_bits(state.control)[0]),
            u8::from(state.handler_mode),
        ) == 1
        {
            let disabled = line16(raw, 4, 1, 1);
            if line16(raw, 1, 1, 1) == 1 {
                state.primask = disabled == 1;
            }
            let faultmask_enable = and_gate(
                line16(raw, 0, 1, 1),
                or_gate(not_gate(disabled), can_raise_faultmask(state)),
            );
            if faultmask_enable == 1 {
                state.faultmask = disabled == 1;
            }
        }
        return Ok(if line16(raw, 4, 1, 1) == 1 {
            "cpsid"
        } else {
            "cpsie"
        });
    }
    if line16(raw, 8, 8, 0xbe) == 1 {
        let immediate = raw as u8;
        if immediate == 0 {
            state.halted = true;
            return Ok("bkpt 0");
        }
        return Err(CortexM3Error::UnsupportedBreakpoint { immediate, pc });
    }
    if line16(raw, 8, 8, 0xbf) == 1 && line16(raw, 0, 4, 0) == 0 {
        let condition = field16(raw, 4, 4) as u8;
        let mask = field16(raw, 0, 4) as u8;
        if condition == 15 || condition == 14 && !mask.is_power_of_two() || state.itstate != 0 {
            return Err(CortexM3Error::UnknownInstruction16 { raw, pc });
        }
        state.itstate = gates::bit_or_32(
            gates::shift_left_32(u32::from(condition), 4),
            u32::from(mask),
        ) as u8;
        return Ok("it");
    }
    if line16(raw, 8, 8, 0xbf) == 1 && line16(raw, 0, 4, 0) == 1 {
        return match field16(raw, 4, 4) {
            0x0 => Ok("nop"),
            0x1 => Ok("yield"),
            0x2 => {
                let event = u8::from(state.event_registered);
                let no_event = not_gate(event);
                state.sleeping = or_gate(and_gate(event, u8::from(state.sleeping)), no_event) == 1;
                state.waiting_for_event =
                    or_gate(and_gate(event, u8::from(state.waiting_for_event)), no_event) == 1;
                state.event_registered = false;
                Ok("wfe")
            }
            0x3 => {
                state.sleeping = true;
                state.waiting_for_event = false;
                Ok("wfi")
            }
            0x4 => {
                state.event_registered = true;
                Ok("sev")
            }
            _ => Ok("nop"),
        };
    }
    Err(CortexM3Error::UnknownInstruction16 { raw, pc })
}

fn execute_multiple_memory(
    state: &mut CortexM3State,
    raw: u16,
) -> Result<&'static str, CortexM3Error> {
    let load = line16(raw, 11, 1, 1) == 1;
    let rn = field16(raw, 8, 3) as usize;
    let mask = field16(raw, 0, 8);
    if mask == 0 {
        return Err(CortexM3Error::UnknownInstruction16 {
            raw,
            pc: gates::sub_32(state.pc, 2, 0).0,
        });
    }
    let base_selected = line16(mask, rn, 1, 1);
    let lower_selected = (0..rn).fold(0, |signal, register| {
        or_gate(signal, line16(mask, register, 1, 1))
    });
    if !load && and_gate(base_selected, lower_selected) == 1 {
        return Err(CortexM3Error::UnknownInstruction16 {
            raw,
            pc: gates::sub_32(state.pc, 2, 0).0,
        });
    }
    let mut address = state.registers[rn];
    checked_aligned_range(address, 4)?;
    let total_width = gates::shift_left_32(gates::popcount_8(mask as u8), 2) as usize;
    if usize::try_from(address)
        .ok()
        .and_then(|start| start.checked_add(total_width))
        .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(CortexM3Error::DataOutOfRange {
            address,
            width: total_width,
        });
    }
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

fn condition_holds(state: &CortexM3State, condition: u8) -> bool {
    let n = u8::from(flag(state, APSR_N));
    let z = u8::from(flag(state, APSR_Z));
    let c = u8::from(flag(state, APSR_C));
    let v = u8::from(flag(state, APSR_V));
    let signal = match condition {
        0 => z,
        1 => not_gate(z),
        2 => c,
        3 => not_gate(c),
        4 => n,
        5 => not_gate(n),
        6 => v,
        7 => not_gate(v),
        8 => and_gate(c, not_gate(z)),
        9 => or_gate(not_gate(c), z),
        10 => not_gate(xor_gate(n, v)),
        11 => xor_gate(n, v),
        12 => and_gate(not_gate(z), not_gate(xor_gate(n, v))),
        13 => or_gate(z, xor_gate(n, v)),
        14 => 1,
        _ => 0,
    };
    signal == 1
}

fn advance_itstate(itstate: u8) -> u8 {
    if gates::selected(gates::bit_and_32(u32::from(itstate), 7), 0, 3) == 1 {
        0
    } else {
        gates::bit_or_32(
            gates::bit_and_32(u32::from(itstate), 0xe0),
            gates::bit_and_32(gates::shift_left_32(u32::from(itstate), 1), 0x1f),
        ) as u8
    }
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
    state: &mut CortexM3State,
    raw: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
    let condition = field16(raw, 8, 4) as u8;
    if condition == 15 {
        return Err(CortexM3Error::SupervisorCall {
            immediate: raw as u8,
            pc,
        });
    }
    if condition == 14 {
        return Err(CortexM3Error::UnknownInstruction16 { raw, pc });
    }
    if condition_holds(state, condition) {
        let offset = gates::shift_left_32(gates::sign_extend(u32::from(raw as u8), 8), 1);
        state.pc = gates::add_32(gates::add_32(pc, 4, 0).0, offset, 0).0;
    }
    Ok(condition_mnemonic(condition))
}

fn execute_unconditional(
    state: &mut CortexM3State,
    raw: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
    let offset = gates::shift_left_32(sign_extend(u32::from(field16(raw, 0, 11)), 11), 1);
    state.pc = gates::add_32(gates::add_32(pc, 4, 0).0, offset, 0).0;
    Ok("b")
}

fn execute_bl(
    state: &mut CortexM3State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
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

fn execute_thumb2_data(
    state: &mut CortexM3State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
    let mov_wide_key = gates::bit_and_32(u32::from(first), 0xfbf0);
    if or_gate(
        gates::selected(mov_wide_key, 0xf240, 16),
        gates::selected(mov_wide_key, 0xf2c0, 16),
    ) == 1
    {
        let rd = field16(second, 8, 4) as usize;
        if rd > 12 {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
        }
        let immediate = [
            gates::shift_left_32(u32::from(field16(first, 0, 4)), 12),
            gates::shift_left_32(u32::from(field16(first, 10, 1)), 11),
            gates::shift_left_32(u32::from(field16(second, 12, 3)), 8),
            u32::from(field16(second, 0, 8)),
        ]
        .into_iter()
        .fold(0, gates::bit_or_32);
        if line16(first, 7, 1, 0) == 1 {
            state.registers[rd] = immediate;
            return Ok("movw");
        }
        state.registers[rd] = gates::bit_or_32(
            gates::bit_and_32(state.registers[rd], 0xffff),
            gates::shift_left_32(immediate, 16),
        );
        return Ok("movt");
    }
    let first_family = gates::bit_and_32(u32::from(first), 0xfff0);
    let division = or_gate(
        gates::selected(first_family, 0xfb90, 16),
        gates::selected(first_family, 0xfbb0, 16),
    );
    if and_gate(
        division,
        gates::selected(gates::bit_and_32(u32::from(second), 0xf0f0), 0xf0f0, 16),
    ) == 1
    {
        let rn = field16(first, 0, 4) as usize;
        let rd = field16(second, 8, 4) as usize;
        let rm = field16(second, 0, 4) as usize;
        if rn > 12 || rd > 12 || rm > 12 {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
        }
        let divisor = state.registers[rm];
        let divide_by_zero = gates::is_zero_32(divisor);
        if line16(first, 5, 1, 0) == 1 {
            state.registers[rd] = gates::select_32(
                gates::divide_signed_32(state.registers[rn], divisor),
                0,
                divide_by_zero,
            );
            return Ok("sdiv");
        }
        state.registers[rd] = gates::select_32(
            gates::divide_unsigned_32(state.registers[rn], divisor),
            0,
            divide_by_zero,
        );
        return Ok("udiv");
    }
    let bit_family = or_gate(
        gates::selected(first_family, 0xfa90, 16),
        gates::selected(first_family, 0xfab0, 16),
    );
    if bit_family == 1 {
        let rm = field16(first, 0, 4) as usize;
        let rd = field16(second, 8, 4) as usize;
        if rm > 12 || rd > 12 || field16(second, 0, 4) != field16(first, 0, 4) {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
        }
        let second_key = gates::bit_and_32(u32::from(second), 0xf0f0);
        if line16(first, 5, 1, 0) == 1 && gates::selected(second_key, 0xf0a0, 16) == 1 {
            state.registers[rd] = gates::reverse_bits_32(state.registers[rm]);
            return Ok("rbit");
        }
        if line16(first, 5, 1, 1) == 1 && gates::selected(second_key, 0xf080, 16) == 1 {
            state.registers[rd] = gates::leading_zeros_32(state.registers[rm]);
            return Ok("clz");
        }
    }
    Err(CortexM3Error::UnknownInstruction32 { first, second, pc })
}

fn execute_thumb2_memory(
    state: &mut CortexM3State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
    let rn = field16(first, 0, 4) as u8;
    let rt = field16(second, 12, 4) as u8;
    let operation = gates::bit_and_32(u32::from(first), 0xfff0) as u16;
    let word_transfer = matches!(operation, 0xf8c0 | 0xf8d0);
    if rn > SP || rt > 12 && !(word_transfer && rt == SP) {
        return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
    }
    let address = gates::add_32(
        read_register(state, rn)?,
        u32::from(field16(second, 0, 12)),
        0,
    )
    .0;
    match operation {
        0xf880 => {
            write_memory(state, address, 1, read_register(state, rt)?)?;
            Ok("strb.w")
        }
        0xf890 => {
            write_register(state, rt, read_memory(state, address, 1)?)?;
            Ok("ldrb.w")
        }
        0xf8a0 => {
            write_memory(state, address, 2, read_register(state, rt)?)?;
            Ok("strh.w")
        }
        0xf8b0 => {
            write_register(state, rt, read_memory(state, address, 2)?)?;
            Ok("ldrh.w")
        }
        0xf8c0 => {
            write_memory(state, address, 4, read_register(state, rt)?)?;
            Ok("str.w")
        }
        0xf8d0 => {
            write_register(state, rt, read_memory(state, address, 4)?)?;
            Ok("ldr.w")
        }
        0xf990 => {
            let value = gates::sign_extend_8(read_memory(state, address, 1)? as u8);
            write_register(state, rt, value)?;
            Ok("ldrsb.w")
        }
        0xf9b0 => {
            let value = gates::sign_extend_16(read_memory(state, address, 2)? as u16);
            write_register(state, rt, value)?;
            Ok("ldrsh.w")
        }
        _ => Err(CortexM3Error::UnknownInstruction32 { first, second, pc }),
    }
}

fn execute_exclusive(
    state: &mut CortexM3State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
    if and_gate(line16(first, 0, 16, 0xf3bf), line16(second, 0, 16, 0x8f2f)) == 1 {
        state.exclusive_address = None;
        return Ok("clrex");
    }
    let rn = field16(first, 0, 4) as u8;
    if rn == PC {
        return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
    }
    let address = gates::add_32(
        read_register(state, rn)?,
        gates::shift_left_32(u32::from(field16(second, 0, 8)), 2),
        0,
    )
    .0;
    checked_aligned_range(address, 4)?;
    if line16(first, 4, 1, 1) == 1 {
        if gates::selected(gates::bit_and_32(u32::from(second), 0x0f00), 0x0f00, 16) == 0 {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
        }
        let rt = field16(second, 12, 4) as u8;
        if rt > 12 {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
        }
        let value = read_memory(state, address, 4)?;
        write_register(state, rt, value)?;
        state.exclusive_address = Some(address);
        return Ok("ldrex");
    }
    let rt = field16(second, 12, 4) as u8;
    let rd = field16(second, 8, 4) as u8;
    if rt > 12 || rd > 12 || rd == rt || rd == rn {
        return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
    }
    let success = state
        .exclusive_address
        .is_some_and(|reserved| gates::equal_32(reserved, address) == 1);
    if success {
        write_memory(state, address, 4, read_register(state, rt)?)?;
    }
    state.exclusive_address = None;
    write_register(state, rd, u32::from(!success))?;
    Ok("strex")
}

fn execute_table_branch(
    state: &mut CortexM3State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
    let rn = field16(first, 0, 4) as u8;
    let rm = field16(second, 0, 4) as u8;
    if rn == SP
        || matches!(rm, SP | PC)
        || gates::selected(gates::bit_and_32(u32::from(second), 0xffe0), 0xf000, 16) == 0
    {
        return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
    }
    let halfword = line16(second, 4, 1, 1);
    let index = read_register(state, rm)?;
    let base = if rn == PC {
        gates::add_32(pc, 4, 0).0
    } else {
        read_register(state, rn)?
    };
    let address = gates::add_32(
        base,
        if halfword == 1 {
            gates::shift_left_32(index, 1)
        } else {
            index
        },
        0,
    )
    .0;
    let offset = read_memory(state, address, if halfword == 1 { 2 } else { 1 })?;
    state.pc = gates::add_32(
        gates::add_32(pc, 4, 0).0,
        gates::shift_left_32(offset, 1),
        0,
    )
    .0;
    state.itstate = 0;
    Ok(if halfword == 1 { "tbh" } else { "tbb" })
}

fn execute_barrier(second: u16) -> Result<&'static str, CortexM3Error> {
    Ok(match second {
        0x8f4f => "dsb",
        0x8f5f => "dmb",
        _ => "isb",
    })
}

fn read_special(state: &CortexM3State, sysm: u8) -> Option<u32> {
    match sysm {
        0 => Some(state.apsr),
        1 => Some(gates::bit_or_32(state.apsr, u32::from(state.ipsr))),
        2 => Some(state.apsr),
        3 => Some(gates::bit_or_32(state.apsr, u32::from(state.ipsr))),
        5 => Some(u32::from(state.ipsr)),
        6 => Some(0),
        7 => Some(u32::from(state.ipsr)),
        8 => Some(state.msp),
        9 => Some(state.psp),
        16 => Some(u32::from(state.primask)),
        17 | 18 => Some(u32::from(state.basepri)),
        19 => Some(u32::from(state.faultmask)),
        20 => Some(gates::select_32(
            u32::from(state.control),
            gates::bit_and_32(u32::from(state.control), gates::bit_not_32(2)),
            u8::from(state.handler_mode),
        )),
        _ => None,
    }
}

fn can_raise_faultmask(state: &CortexM3State) -> u8 {
    let nonmaskable_handler = and_gate(
        u8::from(state.handler_mode),
        or_gate(
            gates::selected(u32::from(state.ipsr), 2, 8),
            gates::selected(u32::from(state.ipsr), 3, 8),
        ),
    );
    not_gate(nonmaskable_handler)
}

fn execute_special_register(
    state: &mut CortexM3State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
    let sysm = second as u8;
    if first == 0xf3ef {
        let rd = field16(second, 8, 4) as u8;
        if rd > 12 {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
        }
        let Some(mut value) = read_special(state, sysm) else {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
        };
        let unprivileged = and_gate(
            gates::u32_to_bits(u32::from(state.control))[0],
            not_gate(u8::from(state.handler_mode)),
        );
        let unprivileged_value = match sysm {
            0..=3 => state.apsr,
            20 => value,
            _ => 0,
        };
        value = gates::select_32(value, unprivileged_value, unprivileged);
        state.registers[rd as usize] = value;
        return Ok("mrs");
    }
    let rn = field16(first, 0, 4) as u8;
    if rn > 12 || !matches!(sysm, 0..=3 | 5..=9 | 16..=20) {
        return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
    }
    let value = state.registers[rn as usize];
    if matches!(sysm, 0..=3) {
        state.apsr = gates::bit_and_32(value, APSR_MASK);
        return Ok("msr");
    }
    let unprivileged = and_gate(
        gates::u32_to_bits(u32::from(state.control))[0],
        not_gate(u8::from(state.handler_mode)),
    );
    if matches!(sysm, 5..=7) || unprivileged == 1 {
        return Ok("msr");
    }
    match sysm {
        8 => state.msp = gates::bit_and_32(value, gates::bit_not_32(3)),
        9 => state.psp = gates::bit_and_32(value, gates::bit_not_32(3)),
        16 => state.primask = gates::bit_and_32(value, 1) != 0,
        17 => state.basepri = value as u8,
        18 => {
            let requested = value as u8;
            let update = and_gate(
                not_gate(gates::is_zero_8(requested)),
                or_gate(
                    gates::is_zero_8(state.basepri),
                    gates::unsigned_less_32(u32::from(requested), u32::from(state.basepri)),
                ),
            );
            state.basepri =
                gates::select_32(u32::from(state.basepri), u32::from(requested), update) as u8;
        }
        19 => {
            let requested = gates::u32_to_bits(value)[0];
            let update = or_gate(not_gate(requested), can_raise_faultmask(state));
            state.faultmask =
                gates::select_32(u32::from(state.faultmask), u32::from(requested), update) != 0;
        }
        _ => {
            let requested = gates::bit_and_32(value, 3) as u8;
            state.control = gates::select_32(
                u32::from(requested),
                gates::bit_and_32(u32::from(requested), 1),
                u8::from(state.handler_mode),
            ) as u8;
        }
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
    pub const fn it(condition: u8, mask: u8) -> u16 {
        0xbf00 | (((condition as u16) & 0xf) << 4) | ((mask as u16) & 0xf)
    }

    #[must_use]
    pub const fn cbz(rn: u8, bytes: u8) -> u16 {
        0xb100
            | ((((bytes as u16) >> 6) & 1) << 9)
            | ((((bytes as u16) >> 1) & 0x1f) << 3)
            | ((rn as u16) & 7)
    }

    #[must_use]
    pub const fn cbnz(rn: u8, bytes: u8) -> u16 {
        cbz(rn, bytes) | 0x0800
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

    #[must_use]
    pub const fn movw(rd: u8, immediate: u16) -> [u16; 2] {
        [
            0xf240 | (((immediate >> 11) & 1) << 10) | ((immediate >> 12) & 0xf),
            (((immediate >> 8) & 7) << 12) | (((rd as u16) & 0xf) << 8) | (immediate & 0xff),
        ]
    }

    #[must_use]
    pub const fn movt(rd: u8, immediate: u16) -> [u16; 2] {
        [
            0xf2c0 | (((immediate >> 11) & 1) << 10) | ((immediate >> 12) & 0xf),
            (((immediate >> 8) & 7) << 12) | (((rd as u16) & 0xf) << 8) | (immediate & 0xff),
        ]
    }

    #[must_use]
    pub const fn udiv(rd: u8, rn: u8, rm: u8) -> [u16; 2] {
        [
            0xfbb0 | ((rn as u16) & 0xf),
            0xf0f0 | (((rd as u16) & 0xf) << 8) | ((rm as u16) & 0xf),
        ]
    }

    #[must_use]
    pub const fn sdiv(rd: u8, rn: u8, rm: u8) -> [u16; 2] {
        [
            0xfb90 | ((rn as u16) & 0xf),
            0xf0f0 | (((rd as u16) & 0xf) << 8) | ((rm as u16) & 0xf),
        ]
    }

    #[must_use]
    pub const fn ldrex(rt: u8, rn: u8, words: u8) -> [u16; 2] {
        [
            0xe850 | ((rn as u16) & 0xf),
            (((rt as u16) & 0xf) << 12) | 0x0f00 | words as u16,
        ]
    }

    #[must_use]
    pub const fn strex(rd: u8, rt: u8, rn: u8, words: u8) -> [u16; 2] {
        [
            0xe840 | ((rn as u16) & 0xf),
            (((rt as u16) & 0xf) << 12) | (((rd as u16) & 0xf) << 8) | words as u16,
        ]
    }

    #[must_use]
    pub const fn clrex() -> [u16; 2] {
        [0xf3bf, 0x8f2f]
    }

    #[must_use]
    pub const fn tbb(rn: u8, rm: u8) -> [u16; 2] {
        [0xe8d0 | ((rn as u16) & 0xf), 0xf000 | ((rm as u16) & 0xf)]
    }

    #[must_use]
    pub const fn tbh(rn: u8, rm: u8) -> [u16; 2] {
        [0xe8d0 | ((rn as u16) & 0xf), 0xf010 | ((rm as u16) & 0xf)]
    }
}
