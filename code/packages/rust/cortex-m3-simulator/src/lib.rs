//! Checked Armv7-M functional model of one Cortex-M3 core.

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
const APSR_Q: u32 = 1 << 27;
const APSR_MASK: u32 = APSR_N | APSR_Z | APSR_C | APSR_V | APSR_Q;

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

/// Exact single-core functional machine.
#[derive(Clone, Debug, Default)]
pub struct CortexM3Simulator {
    state: CortexM3State,
}

impl CortexM3Simulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.state = CortexM3State::default();
    }

    #[must_use]
    pub fn get_state(&self) -> CortexM3State {
        self.state.clone()
    }

    /// Fallibly snapshot all architectural state and owned memory.
    ///
    /// Execution wrappers that promise typed allocation failures should use
    /// this method instead of the infallible compatibility accessor.
    pub fn try_get_state(&self) -> Result<CortexM3State, CortexM3Error> {
        try_clone_state(&self.state)
    }

    pub fn restore(&mut self, state: CortexM3State) -> Result<(), CortexM3Error> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), CortexM3Error> {
        validate_program(program, origin)?;
        let start = origin as usize;
        let mut state = CortexM3State {
            pc: origin,
            loaded_origin: origin,
            loaded_len: program.len(),
            ..CortexM3State::default()
        };
        state.memory[start..start + program.len()].copy_from_slice(program);
        self.state = state;
        Ok(())
    }

    pub fn read_register(&self, index: u8) -> Result<u32, CortexM3Error> {
        read_register(&self.state, index)
    }

    pub fn write_register(&mut self, index: u8, value: u32) -> Result<(), CortexM3Error> {
        write_register(&mut self.state, index, value)
    }

    pub fn read_memory(&self, address: u32, width: usize) -> Result<u32, CortexM3Error> {
        read_memory(&self.state, address, width)
    }

    pub fn write_memory(
        &mut self,
        address: u32,
        width: usize,
        value: u32,
    ) -> Result<(), CortexM3Error> {
        write_memory(&mut self.state, address, width, value)
    }

    pub fn signal_event(&mut self) {
        if self.state.sleeping && self.state.waiting_for_event {
            self.state.sleeping = false;
            self.state.waiting_for_event = false;
            self.state.event_registered = false;
        } else {
            self.state.event_registered = true;
        }
    }

    /// Wake a core blocked in WFI or WFE to model an accepted interrupt.
    pub fn signal_interrupt(&mut self) {
        self.state.sleeping = false;
        self.state.waiting_for_event = false;
    }

    pub fn step(&mut self) -> Result<StepTrace, CortexM3Error> {
        if self.state.halted {
            return Err(CortexM3Error::Halted);
        }
        if self.state.sleeping {
            return Err(CortexM3Error::Sleeping);
        }
        let before = try_clone_state(&self.state)?;
        let pc = before.pc;
        let first = fetch16(&before, pc)?;
        let width = instruction_width(first);
        let second = if width == 4 {
            Some(
                fetch16(&before, pc.wrapping_add(2)).map_err(|error| match error {
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
        let mut next = try_clone_state(&before)?;
        next.pc = pc.wrapping_add(width as u32);
        let sequential_pc = next.pc;
        let mnemonic = if before.itstate != 0 {
            let condition = before.itstate >> 4;
            validate_it_member(first, class, before.itstate, pc)?;
            let executes_unconditionally = first & 0xff00 == 0xbe00;
            let mnemonic = if executes_unconditionally || condition_holds(&before, condition) {
                let mnemonic = execute_decoded(&mut next, first, second, class, pc)?;
                if suppresses_flags_in_it(first, class) {
                    next.apsr = before.apsr;
                }
                if next.pc != sequential_pc && advance_itstate(before.itstate) != 0 {
                    return Err(CortexM3Error::UnknownInstruction16 { raw: first, pc });
                }
                mnemonic
            } else {
                let mut validation = try_clone_state(&next)?;
                if let Err(error) = execute_decoded(&mut validation, first, second, class, pc) {
                    let conditional_undefined =
                        class == InstructionClass::ConditionalBranch && (first >> 8) & 0xf == 14;
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
            next.itstate = if next.pc == sequential_pc {
                advance_itstate(before.itstate)
            } else {
                0
            };
            mnemonic
        } else {
            execute_decoded(&mut next, first, second, class, pc)?
        };
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

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, CortexM3Error> {
        check_retention(max_steps)?;
        let before = try_clone_state(&self.state)?;
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| CortexM3Error::AllocationFailed)?;
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
        Err(CortexM3Error::StepLimitExceeded { limit: max_steps })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, CortexM3Error> {
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

fn try_clone_state(state: &CortexM3State) -> Result<CortexM3State, CortexM3Error> {
    let mut memory = Vec::new();
    memory
        .try_reserve_exact(state.memory.len())
        .map_err(|_| CortexM3Error::AllocationFailed)?;
    memory.extend_from_slice(&state.memory);
    Ok(CortexM3State {
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
    })
}

fn check_retention(max_steps: usize) -> Result<(), CortexM3Error> {
    if max_steps > MAX_RETAINED_STEPS {
        return Err(CortexM3Error::RetentionLimitExceeded {
            requested: max_steps,
            limit: MAX_RETAINED_STEPS,
        });
    }
    Ok(())
}

fn validate_program(program: &[u8], origin: u32) -> Result<(), CortexM3Error> {
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

fn validate_state(state: &CortexM3State) -> Result<(), CortexM3Error> {
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
    if !state.handler_mode && state.control & 2 != 0 {
        state.psp
    } else {
        state.msp
    }
}

fn set_active_sp(state: &mut CortexM3State, value: u32) {
    if !state.handler_mode && state.control & 2 != 0 {
        state.psp = value;
    } else {
        state.msp = value;
    }
}

fn read_register(state: &CortexM3State, index: u8) -> Result<u32, CortexM3Error> {
    match index {
        0..=12 => Ok(state.registers[index as usize]),
        SP => Ok(active_sp(state)),
        LR => Ok(state.lr),
        PC => Ok(state.pc),
        _ => Err(CortexM3Error::InvalidRegister { index }),
    }
}

fn write_register(state: &mut CortexM3State, index: u8, value: u32) -> Result<(), CortexM3Error> {
    match index {
        0..=12 => state.registers[index as usize] = value,
        SP => set_active_sp(state, value & !3),
        LR => state.lr = value,
        PC => state.pc = value & !1,
        _ => return Err(CortexM3Error::InvalidRegister { index }),
    }
    Ok(())
}

fn checked_range(address: u32, width: usize) -> Result<std::ops::Range<usize>, CortexM3Error> {
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
    if width > 1 && address & (width as u32 - 1) != 0 {
        return Err(CortexM3Error::MisalignedData { address, width });
    }
    checked_range(address, width)
}

fn read_memory(state: &CortexM3State, address: u32, width: usize) -> Result<u32, CortexM3Error> {
    let range = checked_range(address, width)?;
    let mut bytes = [0; 4];
    bytes[..width].copy_from_slice(&state.memory[range]);
    Ok(u32::from_le_bytes(bytes))
}

fn write_memory(
    state: &mut CortexM3State,
    address: u32,
    width: usize,
    value: u32,
) -> Result<(), CortexM3Error> {
    let range = checked_range(address, width)?;
    state.memory[range].copy_from_slice(&value.to_le_bytes()[..width]);
    state.exclusive_address = None;
    Ok(())
}

fn fetch16(state: &CortexM3State, pc: u32) -> Result<u16, CortexM3Error> {
    if pc & 1 != 0 {
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
pub fn classify(first: u16, second: Option<u16>) -> Result<InstructionClass, CortexM3Error> {
    classify_at(first, second, 0)
}

fn classify_at(
    first: u16,
    second: Option<u16>,
    pc: u32,
) -> Result<InstructionClass, CortexM3Error> {
    if instruction_width(first) == 4 {
        let Some(second) = second else {
            return Err(CortexM3Error::FetchOutsideProgram { pc, width: 4 });
        };
        if first >> 11 == 0b11110 && second >> 14 == 0b11 && second & 0x1000 != 0 {
            return Ok(InstructionClass::BranchLink);
        }
        if first == 0xf3bf && matches!(second, 0x8f4f | 0x8f5f | 0x8f6f) {
            return Ok(InstructionClass::Barrier);
        }
        if first == 0xf3bf && second == 0x8f2f || matches!(first & 0xfff0, 0xe840 | 0xe850) {
            return Ok(InstructionClass::Exclusive);
        }
        if first & 0xfff0 == 0xe8d0 && second & 0xffe0 == 0xf000 {
            return Ok(InstructionClass::TableBranch);
        }
        if first == 0xf3ef && second & 0xf000 == 0x8000
            || first & 0xfff0 == 0xf380 && valid_msr_second(second)
        {
            return Ok(InstructionClass::SpecialRegister);
        }
        if matches!(
            first & 0xfff0,
            0xf880 | 0xf890 | 0xf8a0 | 0xf8b0 | 0xf8c0 | 0xf8d0 | 0xf990 | 0xf9b0
        ) {
            return Ok(InstructionClass::Thumb2Memory);
        }
        if matches!(first & 0xfbf0, 0xf240 | 0xf2c0)
            || matches!(first & 0xfff0, 0xfb90 | 0xfbb0 | 0xfa90 | 0xfab0)
        {
            return Ok(InstructionClass::Thumb2Data);
        }
        return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
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
    Err(CortexM3Error::UnknownInstruction16 { raw: first, pc })
}

fn valid_msr_second(second: u16) -> bool {
    if second & 0xf300 != 0x8000 {
        return false;
    }
    let mask = ((second >> 10) & 3) as u8;
    mask == 2
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
    state.apsr & mask != 0
}

fn set_flag(state: &mut CortexM3State, mask: u32, value: bool) {
    if value {
        state.apsr |= mask;
    } else {
        state.apsr &= !mask;
    }
}

fn set_nz(state: &mut CortexM3State, value: u32) {
    set_flag(state, APSR_N, value >> 31 != 0);
    set_flag(state, APSR_Z, value == 0);
}

fn add_with_carry(state: &mut CortexM3State, left: u32, right: u32, carry: bool) -> u32 {
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

fn subtract(state: &mut CortexM3State, left: u32, right: u32, borrow: bool) -> u32 {
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
    state: &mut CortexM3State,
    raw: u16,
) -> Result<&'static str, CortexM3Error> {
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

fn execute_immediate(state: &mut CortexM3State, raw: u16) -> Result<&'static str, CortexM3Error> {
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
    state: &mut CortexM3State,
    raw: u16,
) -> Result<&'static str, CortexM3Error> {
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

fn branch_exchange(state: &mut CortexM3State, target: u32) -> Result<(), CortexM3Error> {
    if target & 1 == 0 {
        return Err(CortexM3Error::InvalidThumbTarget { target });
    }
    state.pc = target & !1;
    Ok(())
}

fn bx_write_pc(state: &mut CortexM3State, target: u32) -> Result<(), CortexM3Error> {
    if state.handler_mode && target >> 28 == 0xf {
        return Err(CortexM3Error::UnsupportedExceptionReturn { target });
    }
    branch_exchange(state, target)
}

fn branch_write_pc(state: &mut CortexM3State, target: u32) {
    state.pc = target & !1;
}

fn execute_special(
    state: &mut CortexM3State,
    raw: u16,
    instruction_pc: u32,
) -> Result<&'static str, CortexM3Error> {
    let operation = (raw >> 8) & 3;
    let rm = ((raw >> 3) & 0xf) as u8;
    let rd = ((((raw >> 7) & 1) << 3) | (raw & 7)) as u8;
    if operation == 3 {
        let link = raw & 0x80 != 0;
        if raw & 7 != 0 || link && rm == PC {
            return Err(CortexM3Error::UnknownInstruction16 {
                raw,
                pc: instruction_pc,
            });
        }
        let target = if rm == PC {
            instruction_pc.wrapping_add(4)
        } else {
            read_register(state, rm)?
        };
        if link {
            state.lr = state.pc | 1;
        }
        if link {
            branch_exchange(state, target)?;
        } else {
            bx_write_pc(state, target)?;
        }
        return Ok(if link { "blx" } else { "bx" });
    }
    if operation == 1 && (rd < 8 && rm < 8 || matches!(rd, PC) || matches!(rm, PC))
        || operation == 0 && rd == PC && rm == PC
    {
        return Err(CortexM3Error::UnknownInstruction16 {
            raw,
            pc: instruction_pc,
        });
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
    state: &mut CortexM3State,
    raw: u16,
    instruction_pc: u32,
) -> Result<&'static str, CortexM3Error> {
    let rt = ((raw >> 8) & 7) as usize;
    let address = (instruction_pc.wrapping_add(4) & !3).wrapping_add(u32::from(raw & 0xff) * 4);
    state.registers[rt] = read_memory(state, address, 4)?;
    Ok("ldr")
}

fn execute_register_memory(
    state: &mut CortexM3State,
    raw: u16,
) -> Result<&'static str, CortexM3Error> {
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
    state: &mut CortexM3State,
    raw: u16,
) -> Result<&'static str, CortexM3Error> {
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
    state: &mut CortexM3State,
    raw: u16,
) -> Result<&'static str, CortexM3Error> {
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
    state: &mut CortexM3State,
    raw: u16,
    instruction_pc: u32,
) -> Result<&'static str, CortexM3Error> {
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
    state: &mut CortexM3State,
    raw: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
    if raw & 0xf500 == 0xb100 {
        let rn = (raw & 7) as usize;
        let nonzero = raw & 0x0800 != 0;
        let immediate = (u32::from((raw >> 9) & 1) << 6) | (u32::from((raw >> 3) & 0x1f) << 1);
        if (state.registers[rn] != 0) == nonzero {
            state.pc = pc.wrapping_add(4).wrapping_add(immediate);
            state.itstate = 0;
        }
        return Ok(if nonzero { "cbnz" } else { "cbz" });
    }
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
            return Err(CortexM3Error::UnknownInstruction16 { raw, pc });
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
            return Err(CortexM3Error::UnknownInstruction16 { raw, pc });
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
            bx_write_pc(state, target)?;
        }
        set_active_sp(state, address);
        return Ok("pop");
    }
    if raw & 0xff00 == 0xba00 {
        let operation = (raw >> 6) & 3;
        if operation == 2 {
            return Err(CortexM3Error::UnknownInstruction16 { raw, pc });
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
    if matches!(raw, 0xb661..=0xb663 | 0xb671..=0xb673) {
        if state.control & 1 == 0 || state.handler_mode {
            let disabled = raw & 0x10 != 0;
            if raw & 2 != 0 {
                state.primask = disabled;
            }
            if raw & 1 != 0 && (!disabled || can_raise_faultmask(state)) {
                state.faultmask = disabled;
            }
        }
        return Ok(if raw & 0x10 != 0 { "cpsid" } else { "cpsie" });
    }
    if raw & 0xff00 == 0xbe00 {
        let immediate = raw as u8;
        if immediate == 0 {
            state.halted = true;
            return Ok("bkpt 0");
        }
        return Err(CortexM3Error::UnsupportedBreakpoint { immediate, pc });
    }
    if raw & 0xff00 == 0xbf00 && raw & 0xf != 0 {
        let condition = ((raw >> 4) & 0xf) as u8;
        let mask = (raw & 0xf) as u8;
        if condition == 0xf || condition == 0xe && !mask.is_power_of_two() || state.itstate != 0 {
            return Err(CortexM3Error::UnknownInstruction16 { raw, pc });
        }
        state.itstate = (condition << 4) | mask;
        return Ok("it");
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
                    state.waiting_for_event = true;
                }
                Ok("wfe")
            }
            0x30 => {
                state.sleeping = true;
                state.waiting_for_event = false;
                Ok("wfi")
            }
            0x40 => {
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
    let load = raw & 0x0800 != 0;
    let rn = ((raw >> 8) & 7) as usize;
    let mask = raw & 0xff;
    if mask == 0 {
        return Err(CortexM3Error::UnknownInstruction16 {
            raw,
            pc: state.pc - 2,
        });
    }
    if !load && mask & (1 << rn) != 0 && mask.trailing_zeros() as usize != rn {
        return Err(CortexM3Error::UnknownInstruction16 {
            raw,
            pc: state.pc - 2,
        });
    }
    let mut address = state.registers[rn];
    checked_aligned_range(address, 4)?;
    let total_width = mask.count_ones() as usize * 4;
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

fn condition_holds(state: &CortexM3State, condition: u8) -> bool {
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
        13 => z || n != v,
        14 => true,
        _ => false,
    }
}

fn advance_itstate(itstate: u8) -> u8 {
    if itstate & 7 == 0 {
        0
    } else {
        (itstate & 0xe0) | ((itstate << 1) & 0x1f)
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
    state: &mut CortexM3State,
    raw: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
    let condition = ((raw >> 8) & 0xf) as u8;
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
        let offset = i32::from(raw as u8 as i8) * 2;
        state.pc = pc.wrapping_add(4).wrapping_add_signed(offset);
    }
    Ok(condition_mnemonic(condition))
}

fn execute_unconditional(
    state: &mut CortexM3State,
    raw: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
    let offset = sign_extend(u32::from(raw & 0x7ff), 11) * 2;
    state.pc = pc.wrapping_add(4).wrapping_add_signed(offset);
    Ok("b")
}

fn execute_bl(
    state: &mut CortexM3State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
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

fn execute_thumb2_data(
    state: &mut CortexM3State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
    if matches!(first & 0xfbf0, 0xf240 | 0xf2c0) {
        let rd = ((second >> 8) & 0xf) as usize;
        if rd > 12 {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
        }
        let immediate = (u32::from(first & 0xf) << 12)
            | (u32::from((first >> 10) & 1) << 11)
            | (u32::from((second >> 12) & 7) << 8)
            | u32::from(second & 0xff);
        if first & 0x0080 == 0 {
            state.registers[rd] = immediate;
            return Ok("movw");
        }
        state.registers[rd] = (state.registers[rd] & 0xffff) | (immediate << 16);
        return Ok("movt");
    }
    if matches!(first & 0xfff0, 0xfb90 | 0xfbb0) && second & 0xf0f0 == 0xf0f0 {
        let rn = (first & 0xf) as usize;
        let rd = ((second >> 8) & 0xf) as usize;
        let rm = (second & 0xf) as usize;
        if rn > 12 || rd > 12 || rm > 12 {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
        }
        if first & 0x20 == 0 {
            let divisor = state.registers[rm] as i32;
            state.registers[rd] = if divisor == 0 {
                0
            } else {
                (state.registers[rn] as i32).wrapping_div(divisor) as u32
            };
            return Ok("sdiv");
        }
        let divisor = state.registers[rm];
        state.registers[rd] = state.registers[rn].checked_div(divisor).unwrap_or(0);
        return Ok("udiv");
    }
    if matches!(first & 0xfff0, 0xfa90 | 0xfab0) {
        let rm = (first & 0xf) as usize;
        let rd = ((second >> 8) & 0xf) as usize;
        if rm > 12 || rd > 12 || second & 0xf != first & 0xf {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
        }
        if first & 0x20 == 0 && second & 0xf0f0 == 0xf0a0 {
            state.registers[rd] = state.registers[rm].reverse_bits();
            return Ok("rbit");
        }
        if first & 0x20 != 0 && second & 0xf0f0 == 0xf080 {
            state.registers[rd] = state.registers[rm].leading_zeros();
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
    let rn = (first & 0xf) as u8;
    let rt = ((second >> 12) & 0xf) as u8;
    let operation = first & 0xfff0;
    let word_transfer = matches!(operation, 0xf8c0 | 0xf8d0);
    if rn > SP || rt > 12 && !(word_transfer && rt == SP) {
        return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
    }
    let address = read_register(state, rn)?.wrapping_add(u32::from(second & 0xfff));
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
            let value = read_memory(state, address, 1)? as u8 as i8 as i32 as u32;
            write_register(state, rt, value)?;
            Ok("ldrsb.w")
        }
        0xf9b0 => {
            let value = read_memory(state, address, 2)? as u16 as i16 as i32 as u32;
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
    if first == 0xf3bf && second == 0x8f2f {
        state.exclusive_address = None;
        return Ok("clrex");
    }
    let rn = (first & 0xf) as u8;
    if rn == PC {
        return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
    }
    let address = read_register(state, rn)?.wrapping_add(u32::from(second as u8) * 4);
    checked_aligned_range(address, 4)?;
    if first & 0x10 != 0 {
        if second & 0x0f00 != 0x0f00 {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
        }
        let rt = ((second >> 12) & 0xf) as u8;
        if rt > 12 {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
        }
        let value = read_memory(state, address, 4)?;
        write_register(state, rt, value)?;
        state.exclusive_address = Some(address);
        return Ok("ldrex");
    }
    let rt = ((second >> 12) & 0xf) as u8;
    let rd = ((second >> 8) & 0xf) as u8;
    if rt > 12 || rd > 12 || rd == rt || rd == rn {
        return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
    }
    let success = state.exclusive_address == Some(address);
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
    let rn = (first & 0xf) as u8;
    let rm = (second & 0xf) as u8;
    if rn == SP || matches!(rm, SP | PC) || second & 0xffe0 != 0xf000 {
        return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
    }
    let halfword = second & 0x10 != 0;
    let index = read_register(state, rm)?;
    let base = if rn == PC {
        pc.wrapping_add(4)
    } else {
        read_register(state, rn)?
    };
    let address = base.wrapping_add(if halfword {
        index.wrapping_mul(2)
    } else {
        index
    });
    let offset = read_memory(state, address, if halfword { 2 } else { 1 })?;
    state.pc = pc.wrapping_add(4).wrapping_add(offset.wrapping_mul(2));
    state.itstate = 0;
    Ok(if halfword { "tbh" } else { "tbb" })
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
        1 => Some(state.apsr | u32::from(state.ipsr)),
        2 => Some(state.apsr),
        3 => Some(state.apsr | u32::from(state.ipsr)),
        5 => Some(u32::from(state.ipsr)),
        6 => Some(0),
        7 => Some(u32::from(state.ipsr)),
        8 => Some(state.msp),
        9 => Some(state.psp),
        16 => Some(u32::from(state.primask)),
        17 | 18 => Some(u32::from(state.basepri)),
        19 => Some(u32::from(state.faultmask)),
        20 => Some(u32::from(if state.handler_mode {
            state.control & !2
        } else {
            state.control
        })),
        _ => None,
    }
}

fn can_raise_faultmask(state: &CortexM3State) -> bool {
    !state.handler_mode || !matches!(state.ipsr, 2 | 3)
}

fn execute_special_register(
    state: &mut CortexM3State,
    first: u16,
    second: u16,
    pc: u32,
) -> Result<&'static str, CortexM3Error> {
    let sysm = second as u8;
    if first == 0xf3ef {
        let rd = ((second >> 8) & 0xf) as u8;
        if rd > 12 {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
        }
        let Some(mut value) = read_special(state, sysm) else {
            return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
        };
        if state.control & 1 != 0 && !state.handler_mode {
            value = match sysm {
                0..=3 => state.apsr,
                20 => value,
                _ => 0,
            };
        }
        state.registers[rd as usize] = value;
        return Ok("mrs");
    }
    let rn = (first & 0xf) as u8;
    if rn > 12 || !matches!(sysm, 0..=3 | 5..=9 | 16..=20) {
        return Err(CortexM3Error::UnknownInstruction32 { first, second, pc });
    }
    let value = state.registers[rn as usize];
    if matches!(sysm, 0..=3) {
        let mask = ((second >> 10) & 3) as u8;
        if mask & 2 != 0 {
            state.apsr = value & APSR_MASK;
        }
        return Ok("msr");
    }
    if matches!(sysm, 5..=7) || (state.control & 1 != 0 && !state.handler_mode) {
        return Ok("msr");
    }
    match sysm {
        8 => state.msp = value & !3,
        9 => state.psp = value & !3,
        16 => state.primask = value & 1 != 0,
        17 => state.basepri = value as u8,
        18 => {
            let requested = value as u8;
            if requested != 0 && (state.basepri == 0 || requested < state.basepri) {
                state.basepri = requested;
            }
        }
        19 => {
            let requested = value & 1 != 0;
            if !requested || can_raise_faultmask(state) {
                state.faultmask = requested;
            }
        }
        _ => {
            let requested = value as u8 & 3;
            state.control = if state.handler_mode {
                requested & 1
            } else {
                requested
            };
        }
    }
    Ok("msr")
}

/// Little-endian instruction encoding helpers for tests and downstream users.
pub mod encoding {
    /// Architecturally named APSR fields selected by an MSR mask.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    #[repr(u8)]
    pub enum ApsrMask {
        /// `GE[3:0]` only; requires the DSP extension.
        Ge = 1,
        /// N, Z, C, V, and Q.
        Nzcvq = 2,
        /// N, Z, C, V, Q, and `GE[3:0]`; requires the DSP extension.
        NzcvqGe = 3,
    }

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
        msr_masked(rn, sysm, ApsrMask::Nzcvq)
    }

    /// Encode MSR with an explicit typed architectural APSR mask.
    #[must_use]
    pub const fn msr_masked(rn: u16, sysm: u8, mask: ApsrMask) -> [u16; 2] {
        [
            0xf380 | (rn & 0xf),
            0x8000 | ((mask as u16) << 10) | sysm as u16,
        ]
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
