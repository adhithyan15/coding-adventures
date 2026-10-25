//! Checked Armv6-M functional model of one Cortex-M0 core.

use core::fmt;

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
const APSR_MASK: u32 = APSR_N | APSR_Z | APSR_C | APSR_V;

/// Broad Armv6-M decode family.
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
pub struct CortexM0State {
    /// R0-R12.
    pub registers: [u32; 13],
    pub msp: u32,
    pub psp: u32,
    pub lr: u32,
    pub pc: u32,
    /// Only N/Z/C/V are stored here.
    pub apsr: u32,
    pub ipsr: u8,
    pub primask: bool,
    /// nPRIV is bit 0 and SPSEL is bit 1.
    pub control: u8,
    pub handler_mode: bool,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub sleeping: bool,
    pub event_registered: bool,
    pub loaded_origin: u32,
    pub loaded_len: usize,
}

impl Default for CortexM0State {
    fn default() -> Self {
        Self {
            registers: [0; 13],
            msp: MEMORY_SIZE as u32,
            psp: MEMORY_SIZE as u32,
            lr: 0,
            pc: 0,
            apsr: 0,
            ipsr: 0,
            primask: false,
            control: 0,
            handler_mode: false,
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
pub enum CortexM0Error {
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

impl fmt::Display for CortexM0Error {
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

impl std::error::Error for CortexM0Error {}

/// One committed architectural transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: [u8; 4],
    pub width: usize,
    pub mnemonic: &'static str,
    pub state_before: CortexM0State,
    pub state_after: CortexM0State,
}

/// Successful bounded execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub stopped: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: CortexM0State,
}

/// Exact single-core functional machine.
#[derive(Clone, Debug, Default)]
pub struct CortexM0Simulator {
    state: CortexM0State,
}

impl CortexM0Simulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.state = CortexM0State::default();
    }

    #[must_use]
    pub fn get_state(&self) -> CortexM0State {
        self.state.clone()
    }

    pub fn restore(&mut self, state: CortexM0State) -> Result<(), CortexM0Error> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), CortexM0Error> {
        validate_program(program, origin)?;
        let start = origin as usize;
        let mut state = CortexM0State {
            pc: origin,
            loaded_origin: origin,
            loaded_len: program.len(),
            ..CortexM0State::default()
        };
        state.memory[start..start + program.len()].copy_from_slice(program);
        self.state = state;
        Ok(())
    }

    pub fn read_register(&self, index: u8) -> Result<u32, CortexM0Error> {
        read_register(&self.state, index)
    }

    pub fn write_register(&mut self, index: u8, value: u32) -> Result<(), CortexM0Error> {
        write_register(&mut self.state, index, value)
    }

    pub fn read_memory(&self, address: u32, width: usize) -> Result<u32, CortexM0Error> {
        read_memory(&self.state, address, width)
    }

    pub fn write_memory(
        &mut self,
        address: u32,
        width: usize,
        value: u32,
    ) -> Result<(), CortexM0Error> {
        write_memory(&mut self.state, address, width, value)
    }

    pub fn signal_event(&mut self) {
        self.state.event_registered = true;
        self.state.sleeping = false;
    }

    pub fn step(&mut self) -> Result<StepTrace, CortexM0Error> {
        if self.state.halted {
            return Err(CortexM0Error::Halted);
        }
        if self.state.sleeping {
            return Err(CortexM0Error::Sleeping);
        }
        let before = try_clone_state(&self.state)?;
        let pc = before.pc;
        let first = fetch16(&before, pc)?;
        let width = instruction_width(first);
        let second = if width == 4 {
            Some(
                fetch16(&before, pc.wrapping_add(2)).map_err(|error| match error {
                    CortexM0Error::FetchOutsideProgram { .. } => {
                        CortexM0Error::FetchOutsideProgram { pc, width: 4 }
                    }
                    other => other,
                })?,
            )
        } else {
            None
        };
        let class = classify_at(first, second, pc)?;
        let mut next = try_clone_state(&before)?;
        next.pc = pc.wrapping_add(width as u32);
        let mnemonic = execute_decoded(&mut next, first, second, class, pc)?;
        let mut raw = [0; 4];
        raw[..2].copy_from_slice(&first.to_le_bytes());
        if let Some(second) = second {
            raw[2..].copy_from_slice(&second.to_le_bytes());
        }
        let trace = StepTrace {
            pc_before: pc,
            pc_after: next.pc,
            raw,
            width,
            mnemonic,
            state_before: before,
            state_after: try_clone_state(&next)?,
        };
        self.state = next;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, CortexM0Error> {
        check_retention(max_steps)?;
        let before = try_clone_state(&self.state)?;
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| CortexM0Error::AllocationFailed)?;
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.state = before;
                    return Err(error);
                }
            }
            if self.state.halted || self.state.sleeping {
                let final_state = match try_clone_state(&self.state) {
                    Ok(state) => state,
                    Err(error) => {
                        self.state = before;
                        return Err(error);
                    }
                };
                return Ok(ExecutionResult {
                    stopped: true,
                    steps: traces.len(),
                    traces,
                    final_state,
                });
            }
        }
        self.state = before;
        Err(CortexM0Error::StepLimitExceeded { limit: max_steps })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, CortexM0Error> {
        check_retention(max_steps)?;
        let before = try_clone_state(&self.state)?;
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = before;
                Err(error)
            }
        }
    }
}

fn try_clone_state(state: &CortexM0State) -> Result<CortexM0State, CortexM0Error> {
    let mut memory = Vec::new();
    memory
        .try_reserve_exact(state.memory.len())
        .map_err(|_| CortexM0Error::AllocationFailed)?;
    memory.extend_from_slice(&state.memory);
    Ok(CortexM0State {
        registers: state.registers,
        msp: state.msp,
        psp: state.psp,
        lr: state.lr,
        pc: state.pc,
        apsr: state.apsr,
        ipsr: state.ipsr,
        primask: state.primask,
        control: state.control,
        handler_mode: state.handler_mode,
        memory,
        halted: state.halted,
        sleeping: state.sleeping,
        event_registered: state.event_registered,
        loaded_origin: state.loaded_origin,
        loaded_len: state.loaded_len,
    })
}

fn check_retention(max_steps: usize) -> Result<(), CortexM0Error> {
    if max_steps > MAX_RETAINED_STEPS {
        return Err(CortexM0Error::RetentionLimitExceeded {
            requested: max_steps,
            limit: MAX_RETAINED_STEPS,
        });
    }
    Ok(())
}

fn validate_program(program: &[u8], origin: u32) -> Result<(), CortexM0Error> {
    if program.is_empty() {
        return Err(CortexM0Error::EmptyProgram);
    }
    if origin & 1 != 0 {
        return Err(CortexM0Error::MisalignedProgram { origin });
    }
    if usize::try_from(origin)
        .ok()
        .and_then(|start| start.checked_add(program.len()))
        .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(CortexM0Error::ProgramOutOfRange {
            origin,
            len: program.len(),
        });
    }
    Ok(())
}

fn validate_state(state: &CortexM0State) -> Result<(), CortexM0Error> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(CortexM0Error::InvalidState {
            reason: "memory must be exactly 64 KiB",
        });
    }
    if state.pc & 1 != 0 || state.msp & 3 != 0 || state.psp & 3 != 0 {
        return Err(CortexM0Error::InvalidState {
            reason: "PC or stack pointer alignment is invalid",
        });
    }
    if state.apsr & !APSR_MASK != 0 || state.control & !3 != 0 || state.ipsr > 63 {
        return Err(CortexM0Error::InvalidState {
            reason: "reserved status bits are set",
        });
    }
    if state.handler_mode != (state.ipsr != 0) {
        return Err(CortexM0Error::InvalidState {
            reason: "mode and IPSR disagree",
        });
    }
    if state.loaded_len == 0 {
        if state.loaded_origin != 0 {
            return Err(CortexM0Error::InvalidState {
                reason: "empty image must use origin zero",
            });
        }
    } else if state.loaded_origin & 1 != 0
        || usize::try_from(state.loaded_origin)
            .ok()
            .and_then(|start| start.checked_add(state.loaded_len))
            .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(CortexM0Error::InvalidState {
            reason: "installed image range is invalid",
        });
    }
    Ok(())
}

fn active_sp(state: &CortexM0State) -> u32 {
    if !state.handler_mode && state.control & 2 != 0 {
        state.psp
    } else {
        state.msp
    }
}

fn set_active_sp(state: &mut CortexM0State, value: u32) {
    if !state.handler_mode && state.control & 2 != 0 {
        state.psp = value;
    } else {
        state.msp = value;
    }
}

fn read_register(state: &CortexM0State, index: u8) -> Result<u32, CortexM0Error> {
    match index {
        0..=12 => Ok(state.registers[index as usize]),
        SP => Ok(active_sp(state)),
        LR => Ok(state.lr),
        PC => Ok(state.pc),
        _ => Err(CortexM0Error::InvalidRegister { index }),
    }
}

fn write_register(state: &mut CortexM0State, index: u8, value: u32) -> Result<(), CortexM0Error> {
    match index {
        0..=12 => state.registers[index as usize] = value,
        SP => set_active_sp(state, value & !3),
        LR => state.lr = value,
        PC => state.pc = value & !1,
        _ => return Err(CortexM0Error::InvalidRegister { index }),
    }
    Ok(())
}

fn checked_range(address: u32, width: usize) -> Result<std::ops::Range<usize>, CortexM0Error> {
    if !matches!(width, 1 | 2 | 4) {
        return Err(CortexM0Error::DataOutOfRange { address, width });
    }
    if width > 1 && address & (width as u32 - 1) != 0 {
        return Err(CortexM0Error::MisalignedData { address, width });
    }
    let start = address as usize;
    let Some(end) = start.checked_add(width) else {
        return Err(CortexM0Error::DataOutOfRange { address, width });
    };
    if end > MEMORY_SIZE {
        return Err(CortexM0Error::DataOutOfRange { address, width });
    }
    Ok(start..end)
}

fn read_memory(state: &CortexM0State, address: u32, width: usize) -> Result<u32, CortexM0Error> {
    let range = checked_range(address, width)?;
    let mut bytes = [0; 4];
    bytes[..width].copy_from_slice(&state.memory[range]);
    Ok(u32::from_le_bytes(bytes))
}

fn write_memory(
    state: &mut CortexM0State,
    address: u32,
    width: usize,
    value: u32,
) -> Result<(), CortexM0Error> {
    let range = checked_range(address, width)?;
    state.memory[range].copy_from_slice(&value.to_le_bytes()[..width]);
    Ok(())
}

fn fetch16(state: &CortexM0State, pc: u32) -> Result<u16, CortexM0Error> {
    if pc & 1 != 0 {
        return Err(CortexM0Error::MisalignedFetch { pc });
    }
    if state.loaded_len < 2 {
        return Err(CortexM0Error::FetchOutsideProgram { pc, width: 2 });
    }
    let relative = pc.checked_sub(state.loaded_origin);
    if relative.is_none_or(|offset| offset > state.loaded_len.saturating_sub(2) as u32) {
        return Err(CortexM0Error::FetchOutsideProgram { pc, width: 2 });
    }
    let start = pc as usize;
    Ok(u16::from_le_bytes([
        state.memory[start],
        state.memory[start + 1],
    ]))
}

fn instruction_width(first: u16) -> usize {
    if matches!(first >> 11, 0b11101..=0b11111) {
        4
    } else {
        2
    }
}

/// Classify an encoded instruction without executing it.
pub fn classify(first: u16, second: Option<u16>) -> Result<InstructionClass, CortexM0Error> {
    classify_at(first, second, 0)
}

fn classify_at(
    first: u16,
    second: Option<u16>,
    pc: u32,
) -> Result<InstructionClass, CortexM0Error> {
    if instruction_width(first) == 4 {
        let Some(second) = second else {
            return Err(CortexM0Error::FetchOutsideProgram { pc, width: 4 });
        };
        if first >> 11 == 0b11110 && second >> 14 == 0b11 && second & 0x1000 != 0 {
            return Ok(InstructionClass::BranchLink);
        }
        if first == 0xf3bf && matches!(second, 0x8f4f | 0x8f5f | 0x8f6f) {
            return Ok(InstructionClass::Barrier);
        }
        if first == 0xf3ef && second & 0xf000 == 0x8000
            || first & 0xfff0 == 0xf380 && second & 0xff00 == 0x8800
        {
            return Ok(InstructionClass::SpecialRegister);
        }
        return Err(CortexM0Error::UnknownInstruction32 { first, second, pc });
    }
    if first >> 13 == 0 {
        return Ok(InstructionClass::ShiftAddSubtract);
    }
    if first >> 13 == 1 {
        return Ok(InstructionClass::Immediate);
    }
    if first >> 10 == 0b01_0000 {
        return Ok(InstructionClass::DataProcessing);
    }
    if first >> 10 == 0b01_0001 {
        return Ok(InstructionClass::SpecialData);
    }
    if first >> 11 == 0b01001 {
        return Ok(InstructionClass::LiteralLoad);
    }
    if first >> 12 == 0b0101 {
        return Ok(InstructionClass::RegisterMemory);
    }
    if matches!(first >> 12, 0b0110..=0b1000) {
        return Ok(InstructionClass::ImmediateMemory);
    }
    if first >> 12 == 0b1001 {
        return Ok(InstructionClass::StackMemory);
    }
    if first >> 12 == 0b1010 {
        return Ok(InstructionClass::AddressGeneration);
    }
    if first >> 12 == 0b1011 {
        return Ok(InstructionClass::Miscellaneous);
    }
    if first >> 12 == 0b1100 {
        return Ok(InstructionClass::MultipleMemory);
    }
    if first >> 12 == 0b1101 {
        return Ok(InstructionClass::ConditionalBranch);
    }
    if first >> 11 == 0b11100 {
        return Ok(InstructionClass::UnconditionalBranch);
    }
    Err(CortexM0Error::UnknownInstruction16 { raw: first, pc })
}

fn execute_decoded(
    state: &mut CortexM0State,
    first: u16,
    second: Option<u16>,
    class: InstructionClass,
    pc: u32,
) -> Result<&'static str, CortexM0Error> {
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

fn flag(state: &CortexM0State, mask: u32) -> bool {
    state.apsr & mask != 0
}

fn set_flag(state: &mut CortexM0State, mask: u32, value: bool) {
    if value {
        state.apsr |= mask;
    } else {
        state.apsr &= !mask;
    }
}

fn set_nz(state: &mut CortexM0State, value: u32) {
    set_flag(state, APSR_N, value >> 31 != 0);
    set_flag(state, APSR_Z, value == 0);
}

fn add_with_carry(state: &mut CortexM0State, left: u32, right: u32, carry: bool) -> u32 {
    let unsigned = u64::from(left) + u64::from(right) + u64::from(carry);
    let signed = i64::from(left as i32) + i64::from(right as i32) + i64::from(carry);
    let result = unsigned as u32;
    set_nz(state, result);
    set_flag(state, APSR_C, unsigned > u64::from(u32::MAX));
    set_flag(
        state,
        APSR_V,
        signed < i64::from(i32::MIN) || signed > i64::from(i32::MAX),
    );
    result
}

fn subtract(state: &mut CortexM0State, left: u32, right: u32, borrow: bool) -> u32 {
    add_with_carry(state, left, !right, !borrow)
}

fn shift_immediate(value: u32, kind: u8, encoded: u32, carry: bool) -> (u32, bool) {
    match kind {
        0 if encoded == 0 => (value, carry),
        0 => (value << encoded, value >> (32 - encoded) & 1 != 0),
        1 => {
            let amount = if encoded == 0 { 32 } else { encoded };
            (
                value.checked_shr(amount).unwrap_or(0),
                value >> (amount - 1) & 1 != 0,
            )
        }
        _ => {
            let amount = if encoded == 0 { 32 } else { encoded };
            let result = if amount >= 32 {
                if value >> 31 != 0 {
                    u32::MAX
                } else {
                    0
                }
            } else {
                ((value as i32) >> amount) as u32
            };
            (result, value >> (amount.min(32) - 1) & 1 != 0)
        }
    }
}

fn shift_register(value: u32, kind: u8, amount: u32, carry: bool) -> (u32, bool) {
    if amount == 0 {
        return (value, carry);
    }
    match kind {
        0 if amount < 32 => (value << amount, value >> (32 - amount) & 1 != 0),
        0 if amount == 32 => (0, value & 1 != 0),
        0 => (0, false),
        1 if amount < 32 => (value >> amount, value >> (amount - 1) & 1 != 0),
        1 if amount == 32 => (0, value >> 31 != 0),
        1 => (0, false),
        2 => {
            let result = if amount >= 32 {
                if value >> 31 != 0 {
                    u32::MAX
                } else {
                    0
                }
            } else {
                ((value as i32) >> amount) as u32
            };
            (result, value >> (amount.min(32) - 1) & 1 != 0)
        }
        _ => {
            let rotation = amount & 31;
            let result = value.rotate_right(rotation);
            (result, result >> 31 != 0)
        }
    }
}

fn execute_shift_add_sub(
    state: &mut CortexM0State,
    raw: u16,
) -> Result<&'static str, CortexM0Error> {
    let operation = (raw >> 11) & 3;
    if operation < 3 {
        let amount = u32::from((raw >> 6) & 0x1f);
        let rm = ((raw >> 3) & 7) as usize;
        let rd = (raw & 7) as usize;
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
    let op = (raw >> 9) & 3;
    let operand = ((raw >> 6) & 7) as usize;
    let rn = ((raw >> 3) & 7) as usize;
    let rd = (raw & 7) as usize;
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

fn execute_immediate(state: &mut CortexM0State, raw: u16) -> Result<&'static str, CortexM0Error> {
    let operation = (raw >> 11) & 3;
    let register = ((raw >> 8) & 7) as usize;
    let immediate = u32::from(raw & 0xff);
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
    state: &mut CortexM0State,
    raw: u16,
) -> Result<&'static str, CortexM0Error> {
    let operation = ((raw >> 6) & 0xf) as u8;
    let rm = ((raw >> 3) & 7) as usize;
    let rdn = (raw & 7) as usize;
    let left = state.registers[rdn];
    let right = state.registers[rm];
    Ok(match operation {
        0 => {
            state.registers[rdn] = left & right;
            set_nz(state, state.registers[rdn]);
            "ands"
        }
        1 => {
            state.registers[rdn] = left ^ right;
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
            let (result, carry) = shift_register(left, kind, right & 0xff, flag(state, APSR_C));
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
            set_nz(state, left & right);
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
            state.registers[rdn] = left | right;
            set_nz(state, state.registers[rdn]);
            "orrs"
        }
        13 => {
            state.registers[rdn] = left.wrapping_mul(right);
            set_nz(state, state.registers[rdn]);
            "muls"
        }
        14 => {
            state.registers[rdn] = left & !right;
            set_nz(state, state.registers[rdn]);
            "bics"
        }
        _ => {
            state.registers[rdn] = !right;
            set_nz(state, state.registers[rdn]);
            "mvns"
        }
    })
}

fn branch_exchange(state: &mut CortexM0State, target: u32) -> Result<(), CortexM0Error> {
    if target & 1 == 0 {
        return Err(CortexM0Error::InvalidThumbTarget { target });
    }
    state.pc = target & !1;
    Ok(())
}

fn branch_write_pc(state: &mut CortexM0State, target: u32) {
    state.pc = target & !1;
}

fn execute_special(
    state: &mut CortexM0State,
    raw: u16,
    instruction_pc: u32,
) -> Result<&'static str, CortexM0Error> {
    let operation = (raw >> 8) & 3;
    let rm = ((raw >> 3) & 0xf) as u8;
    let rd = ((((raw >> 7) & 1) << 3) | (raw & 7)) as u8;
    if operation == 3 {
        if raw & 7 != 0 || matches!(rm, SP | PC) {
            return Err(CortexM0Error::UnknownInstruction16 {
                raw,
                pc: instruction_pc,
            });
        }
        let target = read_register(state, rm)?;
        if raw & 0x80 != 0 {
            state.lr = state.pc | 1;
        }
        branch_exchange(state, target)?;
        return Ok(if raw & 0x80 != 0 { "blx" } else { "bx" });
    }
    let left = if rd == PC {
        instruction_pc.wrapping_add(4)
    } else {
        read_register(state, rd)?
    };
    let right = if rm == PC {
        instruction_pc.wrapping_add(4)
    } else {
        read_register(state, rm)?
    };
    match operation {
        0 => {
            let result = left.wrapping_add(right);
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
    state: &mut CortexM0State,
    raw: u16,
    instruction_pc: u32,
) -> Result<&'static str, CortexM0Error> {
    let rt = ((raw >> 8) & 7) as usize;
    let address = (instruction_pc.wrapping_add(4) & !3).wrapping_add(u32::from(raw & 0xff) * 4);
    state.registers[rt] = read_memory(state, address, 4)?;
    Ok("ldr")
}

fn execute_register_memory(
    state: &mut CortexM0State,
    raw: u16,
) -> Result<&'static str, CortexM0Error> {
    let operation = (raw >> 9) & 7;
    let rm = ((raw >> 6) & 7) as usize;
    let rn = ((raw >> 3) & 7) as usize;
    let rt = (raw & 7) as usize;
    let address = state.registers[rn].wrapping_add(state.registers[rm]);
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
            state.registers[rt] = read_memory(state, address, 1)? as u8 as i8 as i32 as u32;
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
            state.registers[rt] = read_memory(state, address, 2)? as u16 as i16 as i32 as u32;
            "ldrsh"
        }
    })
}

fn execute_immediate_memory(
    state: &mut CortexM0State,
    raw: u16,
) -> Result<&'static str, CortexM0Error> {
    let operation = raw >> 11;
    let immediate = u32::from((raw >> 6) & 0x1f);
    let rn = ((raw >> 3) & 7) as usize;
    let rt = (raw & 7) as usize;
    let (scale, load, width, mnemonic) = match operation {
        0b01100 => (4, false, 4, "str"),
        0b01101 => (4, true, 4, "ldr"),
        0b01110 => (1, false, 1, "strb"),
        0b01111 => (1, true, 1, "ldrb"),
        0b10000 => (2, false, 2, "strh"),
        _ => (2, true, 2, "ldrh"),
    };
    let address = state.registers[rn].wrapping_add(immediate * scale);
    if load {
        state.registers[rt] = read_memory(state, address, width)?;
    } else {
        write_memory(state, address, width, state.registers[rt])?;
    }
    Ok(mnemonic)
}

fn execute_stack_memory(
    state: &mut CortexM0State,
    raw: u16,
) -> Result<&'static str, CortexM0Error> {
    let load = raw & 0x0800 != 0;
    let rt = ((raw >> 8) & 7) as usize;
    let address = active_sp(state).wrapping_add(u32::from(raw & 0xff) * 4);
    if load {
        state.registers[rt] = read_memory(state, address, 4)?;
        Ok("ldr")
    } else {
        write_memory(state, address, 4, state.registers[rt])?;
        Ok("str")
    }
}

fn execute_address_generation(
    state: &mut CortexM0State,
    raw: u16,
    instruction_pc: u32,
) -> Result<&'static str, CortexM0Error> {
    let rd = ((raw >> 8) & 7) as usize;
    let immediate = u32::from(raw & 0xff) * 4;
    if raw & 0x0800 == 0 {
        state.registers[rd] = (instruction_pc.wrapping_add(4) & !3).wrapping_add(immediate);
        Ok("adr")
    } else {
        state.registers[rd] = active_sp(state).wrapping_add(immediate);
        Ok("add")
    }
}

fn execute_miscellaneous(
    state: &mut CortexM0State,
    raw: u16,
    pc: u32,
) -> Result<&'static str, CortexM0Error> {
    if raw & 0xff00 == 0xb000 {
        let offset = u32::from(raw & 0x7f) * 4;
        let value = if raw & 0x80 != 0 {
            active_sp(state).wrapping_sub(offset)
        } else {
            active_sp(state).wrapping_add(offset)
        };
        set_active_sp(state, value);
        return Ok(if raw & 0x80 != 0 { "sub" } else { "add" });
    }
    if raw & 0xff00 == 0xb200 {
        let operation = (raw >> 6) & 3;
        let rm = ((raw >> 3) & 7) as usize;
        let rd = (raw & 7) as usize;
        let value = state.registers[rm];
        state.registers[rd] = match operation {
            0 => value as u16 as i16 as i32 as u32,
            1 => value as u8 as i8 as i32 as u32,
            2 => u32::from(value as u16),
            _ => u32::from(value as u8),
        };
        return Ok(["sxth", "sxtb", "uxth", "uxtb"][operation as usize]);
    }
    if raw & 0xfe00 == 0xb400 {
        let mask = raw & 0xff;
        let include_lr = raw & 0x100 != 0;
        if mask == 0 && !include_lr {
            return Err(CortexM0Error::UnknownInstruction16 { raw, pc });
        }
        let count = mask.count_ones() + u32::from(include_lr);
        let start = active_sp(state).wrapping_sub(count * 4);
        let mut address = start;
        for register in 0..8 {
            if mask & (1 << register) != 0 {
                write_memory(state, address, 4, state.registers[register])?;
                address = address.wrapping_add(4);
            }
        }
        if include_lr {
            write_memory(state, address, 4, state.lr)?;
        }
        set_active_sp(state, start);
        return Ok("push");
    }
    if raw & 0xfe00 == 0xbc00 {
        let mask = raw & 0xff;
        let include_pc = raw & 0x100 != 0;
        if mask == 0 && !include_pc {
            return Err(CortexM0Error::UnknownInstruction16 { raw, pc });
        }
        let mut address = active_sp(state);
        for register in 0..8 {
            if mask & (1 << register) != 0 {
                state.registers[register] = read_memory(state, address, 4)?;
                address = address.wrapping_add(4);
            }
        }
        if include_pc {
            let target = read_memory(state, address, 4)?;
            address = address.wrapping_add(4);
            branch_exchange(state, target)?;
        }
        set_active_sp(state, address);
        return Ok("pop");
    }
    if raw & 0xff00 == 0xba00 {
        let operation = (raw >> 6) & 3;
        if operation == 2 {
            return Err(CortexM0Error::UnknownInstruction16 { raw, pc });
        }
        let rm = ((raw >> 3) & 7) as usize;
        let rd = (raw & 7) as usize;
        let value = state.registers[rm];
        state.registers[rd] = match operation {
            0 => value.swap_bytes(),
            1 => value.rotate_left(16).swap_bytes(),
            _ => (value as u16).swap_bytes() as i16 as i32 as u32,
        };
        return Ok(match operation {
            0 => "rev",
            1 => "rev16",
            _ => "revsh",
        });
    }
    if matches!(raw, 0xb662 | 0xb672) {
        if state.control & 1 == 0 || state.handler_mode {
            state.primask = raw == 0xb672;
        }
        return Ok(if raw == 0xb672 { "cpsid" } else { "cpsie" });
    }
    if raw & 0xff00 == 0xbe00 {
        let immediate = raw as u8;
        if immediate == 0 {
            state.halted = true;
            return Ok("bkpt 0");
        }
        return Err(CortexM0Error::UnsupportedBreakpoint { immediate, pc });
    }
    if raw & 0xff0f == 0xbf00 {
        return match raw & 0xf0 {
            0x00 => Ok("nop"),
            0x10 => Ok("yield"),
            0x20 => {
                if state.event_registered {
                    state.event_registered = false;
                } else {
                    state.sleeping = true;
                }
                Ok("wfe")
            }
            0x30 => {
                state.sleeping = true;
                Ok("wfi")
            }
            0x40 => {
                state.event_registered = true;
                Ok("sev")
            }
            _ => Ok("nop"),
        };
    }
    Err(CortexM0Error::UnknownInstruction16 { raw, pc })
}

fn execute_multiple_memory(
    state: &mut CortexM0State,
    raw: u16,
) -> Result<&'static str, CortexM0Error> {
    let load = raw & 0x0800 != 0;
    let rn = ((raw >> 8) & 7) as usize;
    let mask = raw & 0xff;
    if mask == 0 {
        return Err(CortexM0Error::UnknownInstruction16 {
            raw,
            pc: state.pc - 2,
        });
    }
    if !load && mask & (1 << rn) != 0 && mask.trailing_zeros() as usize != rn {
        return Err(CortexM0Error::UnknownInstruction16 {
            raw,
            pc: state.pc - 2,
        });
    }
    let mut address = state.registers[rn];
    for register in 0..8 {
        if mask & (1 << register) != 0 {
            if load {
                state.registers[register] = read_memory(state, address, 4)?;
            } else {
                write_memory(state, address, 4, state.registers[register])?;
            }
            address = address.wrapping_add(4);
        }
    }
    if !load || mask & (1 << rn) == 0 {
        state.registers[rn] = address;
    }
    Ok(if load { "ldm" } else { "stm" })
}

fn condition_holds(state: &CortexM0State, condition: u8) -> bool {
    let n = flag(state, APSR_N);
    let z = flag(state, APSR_Z);
    let c = flag(state, APSR_C);
    let v = flag(state, APSR_V);
    match condition {
        0 => z,
        1 => !z,
        2 => c,
        3 => !c,
        4 => n,
        5 => !n,
        6 => v,
        7 => !v,
        8 => c && !z,
        9 => !c || z,
        10 => n == v,
        11 => n != v,
        12 => !z && n == v,
        _ => z || n != v,
    }
}

fn condition_mnemonic(condition: u8) -> &'static str {
    [
        "beq", "bne", "bcs", "bcc", "bmi", "bpl", "bvs", "bvc", "bhi", "bls", "bge", "blt", "bgt",
        "ble",
    ][condition as usize]
}

fn sign_extend(value: u32, bits: u32) -> i32 {
    let shift = 32 - bits;
    ((value << shift) as i32) >> shift
}

fn execute_conditional(
    state: &mut CortexM0State,
    raw: u16,
    pc: u32,
) -> Result<&'static str, CortexM0Error> {
    let condition = ((raw >> 8) & 0xf) as u8;
    if condition == 15 {
        return Err(CortexM0Error::SupervisorCall {
            immediate: raw as u8,
            pc,
        });
    }
    if condition == 14 {
        return Err(CortexM0Error::UnknownInstruction16 { raw, pc });
    }
    if condition_holds(state, condition) {
        let offset = i32::from(raw as u8 as i8) * 2;
        state.pc = pc.wrapping_add(4).wrapping_add_signed(offset);
    }
    Ok(condition_mnemonic(condition))
}

fn execute_unconditional(
    state: &mut CortexM0State,
    raw: u16,
    pc: u32,
) -> Result<&'static str, CortexM0Error> {
    let offset = sign_extend(u32::from(raw & 0x7ff), 11) * 2;
    state.pc = pc.wrapping_add(4).wrapping_add_signed(offset);
    Ok("b")
}

fn execute_bl(
    state: &mut CortexM0State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM0Error> {
    let s = u32::from((first >> 10) & 1);
    let j1 = u32::from((second >> 13) & 1);
    let j2 = u32::from((second >> 11) & 1);
    let i1 = u32::from((j1 ^ s) == 0);
    let i2 = u32::from((j2 ^ s) == 0);
    let immediate = (s << 24)
        | (i1 << 23)
        | (i2 << 22)
        | (u32::from(first & 0x03ff) << 12)
        | (u32::from(second & 0x07ff) << 1);
    state.lr = pc.wrapping_add(4) | 1;
    state.pc = pc
        .wrapping_add(4)
        .wrapping_add_signed(sign_extend(immediate, 25));
    Ok("bl")
}

fn execute_barrier(second: u16) -> Result<&'static str, CortexM0Error> {
    Ok(match second {
        0x8f4f => "dsb",
        0x8f5f => "dmb",
        _ => "isb",
    })
}

fn read_special(state: &CortexM0State, sysm: u8) -> Option<u32> {
    match sysm {
        0 => Some(state.apsr),
        1 => Some(state.apsr | u32::from(state.ipsr)),
        2 => Some(state.apsr),
        3 => Some(state.apsr | u32::from(state.ipsr)),
        5 => Some(u32::from(state.ipsr)),
        6 => Some(0),
        7 => Some(u32::from(state.ipsr)),
        8 => Some(state.msp),
        9 => Some(state.psp),
        16 => Some(u32::from(state.primask)),
        20 => Some(u32::from(state.control)),
        _ => None,
    }
}

fn execute_special_register(
    state: &mut CortexM0State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM0Error> {
    let sysm = second as u8;
    if first == 0xf3ef {
        let rd = ((second >> 8) & 0xf) as u8;
        if rd > 12 {
            return Err(CortexM0Error::UnknownInstruction32 { first, second, pc });
        }
        let Some(mut value) = read_special(state, sysm) else {
            return Err(CortexM0Error::UnknownInstruction32 { first, second, pc });
        };
        if state.control & 1 != 0 && !state.handler_mode && !matches!(sysm, 0 | 20) {
            value = 0;
        }
        state.registers[rd as usize] = value;
        return Ok("mrs");
    }
    let rn = (first & 0xf) as u8;
    if rn > 12 || !matches!(sysm, 0..=3 | 5..=9 | 16 | 20) {
        return Err(CortexM0Error::UnknownInstruction32 { first, second, pc });
    }
    let value = state.registers[rn as usize];
    if matches!(sysm, 0..=3) {
        state.apsr = value & APSR_MASK;
        return Ok("msr");
    }
    if matches!(sysm, 5..=7) || (state.control & 1 != 0 && !state.handler_mode) {
        return Ok("msr");
    }
    match sysm {
        8 => state.msp = value & !3,
        9 => state.psp = value & !3,
        16 => state.primask = value & 1 != 0,
        _ => {
            let requested = value as u8 & 3;
            state.control = if state.handler_mode {
                (state.control & 2) | (requested & 1)
            } else {
                requested
            };
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
