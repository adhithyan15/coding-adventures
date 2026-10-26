//! Checked bounded Armv8-M Baseline functional model of one Cortex-M23 core.

use core::fmt;

pub use cortex_m0_simulator::{classify, encoding, InstructionClass};

/// Owned teaching-memory size.
pub const MEMORY_SIZE: usize = cortex_m0_simulator::MEMORY_SIZE;
/// Conservative budget for retained complete-state trace snapshots.
pub const MAX_RETAINED_BYTES: usize = cortex_m0_simulator::MAX_RETAINED_BYTES;
/// Maximum complete-state traces retained by a bounded run.
pub const MAX_RETAINED_STEPS: usize = cortex_m0_simulator::MAX_RETAINED_STEPS;
/// Number of modeled SAU regions.
pub const SAU_REGION_COUNT: usize = 8;
/// Stack-pointer register number.
pub const SP: u8 = cortex_m0_simulator::SP;
/// Link-register number.
pub const LR: u8 = cortex_m0_simulator::LR;
/// Program-counter register number.
pub const PC: u8 = cortex_m0_simulator::PC;

const APSR_MASK: u32 = 0xf000_0000;

/// Whether this configured processor implements the optional Security Extension.
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

/// Address attribution produced by the bounded SAU model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecurityAttribute {
    Secure,
    NonSecure,
    NonSecureCallable,
}

/// Kind of checked memory access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessKind {
    Instruction,
    DataRead,
    DataWrite,
}

/// One inclusive, 32-byte-granular SAU region descriptor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SauRegion {
    pub enabled: bool,
    pub base: u32,
    pub limit: u32,
    pub non_secure_callable: bool,
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
    /// N/Z/C/V flags.
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

impl CortexM23State {
    fn reset(configuration: SecurityExtension) -> Self {
        Self::with_memory(configuration, vec![0; MEMORY_SIZE])
    }

    fn with_memory(configuration: SecurityExtension, memory: Vec<u8>) -> Self {
        let enabled = configuration == SecurityExtension::Enabled;
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
            security_extension: configuration,
            security_state: if enabled {
                SecurityState::Secure
            } else {
                SecurityState::NonSecure
            },
            sau_enabled: false,
            sau_all_non_secure: !enabled,
            sau_regions: [SauRegion::default(); SAU_REGION_COUNT],
            memory,
            halted: false,
            sleeping: false,
            event_registered: false,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }
}

impl Default for CortexM23State {
    fn default() -> Self {
        Self::reset(SecurityExtension::Enabled)
    }
}

/// Typed lifecycle, security, decode, and execution failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CortexM23Error {
    Core(cortex_m0_simulator::CortexM0Error),
    SecurityExtensionUnavailable,
    SecureOnlyOperation,
    UnprivilegedOperation,
    InvalidSauRegion {
        index: usize,
        reason: &'static str,
    },
    SecurityAttributionFault {
        address: u32,
        width: usize,
        access: AccessKind,
        state: SecurityState,
        attribute: SecurityAttribute,
    },
    InvalidSecurityTransition {
        from: SecurityState,
        target: u32,
        required: SecurityAttribute,
    },
    RetentionLimitExceeded {
        requested: usize,
        limit: usize,
    },
    StepLimitExceeded {
        limit: usize,
    },
    AllocationFailed,
}

impl From<cortex_m0_simulator::CortexM0Error> for CortexM23Error {
    fn from(error: cortex_m0_simulator::CortexM0Error) -> Self {
        match error {
            cortex_m0_simulator::CortexM0Error::AllocationFailed => Self::AllocationFailed,
            other => Self::Core(other),
        }
    }
}

impl fmt::Display for CortexM23Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => error.fmt(formatter),
            Self::SecurityExtensionUnavailable => {
                formatter.write_str("Security Extension is not implemented")
            }
            Self::SecureOnlyOperation => formatter.write_str("operation requires Secure state"),
            Self::UnprivilegedOperation => formatter.write_str("operation requires privilege"),
            Self::InvalidSauRegion { index, reason } => {
                write!(formatter, "invalid SAU region {index}: {reason}")
            }
            Self::SecurityAttributionFault {
                address,
                width,
                access,
                state,
                attribute,
            } => write!(
                formatter,
                "{state:?} {access:?} of {width} bytes at {address:#010x} is attributed {attribute:?}"
            ),
            Self::InvalidSecurityTransition {
                from,
                target,
                required,
            } => write!(
                formatter,
                "invalid {from:?} transition to {target:#010x}; target must be {required:?}"
            ),
            Self::RetentionLimitExceeded { requested, limit } => write!(
                formatter,
                "requested {requested} traces exceeds retention limit {limit}"
            ),
            Self::StepLimitExceeded { limit } => write!(formatter, "step limit {limit} exhausted"),
            Self::AllocationFailed => formatter.write_str("state or trace allocation failed"),
        }
    }
}

impl std::error::Error for CortexM23Error {}

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

/// Exact single-core functional machine within the package's stated boundary.
#[derive(Debug)]
pub struct CortexM23Simulator {
    state: CortexM23State,
    /// Execution scratch. `state` is the sole architectural authority.
    core: cortex_m0_simulator::CortexM0Simulator,
}

impl Default for CortexM23Simulator {
    fn default() -> Self {
        Self::new(SecurityExtension::Enabled)
    }
}

impl CortexM23Simulator {
    #[must_use]
    pub fn new(configuration: SecurityExtension) -> Self {
        Self {
            state: CortexM23State::reset(configuration),
            core: cortex_m0_simulator::CortexM0Simulator::new(),
        }
    }

    pub fn reset(&mut self) {
        self.state = CortexM23State::reset(self.state.security_extension);
    }

    /// Fallible complete-state snapshot for checked callers.
    pub fn try_get_state(&self) -> Result<CortexM23State, CortexM23Error> {
        try_clone_state(&self.state)
    }

    pub fn restore(&mut self, state: CortexM23State) -> Result<(), CortexM23Error> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), CortexM23Error> {
        validate_program(program, origin)?;
        let mut next = try_reset_state(self.state.security_extension)?;
        let start = origin as usize;
        next.memory[start..start + program.len()].copy_from_slice(program);
        next.pc = origin;
        next.loaded_origin = origin;
        next.loaded_len = program.len();
        self.state = next;
        Ok(())
    }

    pub fn read_register(&self, index: u8) -> Result<u32, CortexM23Error> {
        match index {
            0..=12 => Ok(self.state.registers[index as usize]),
            SP => Ok(active_sp(&self.state)),
            LR => Ok(self.state.lr),
            PC => Ok(self.state.pc),
            _ => Err(cortex_m0_simulator::CortexM0Error::InvalidRegister { index }.into()),
        }
    }

    pub fn write_register(&mut self, index: u8, value: u32) -> Result<(), CortexM23Error> {
        match index {
            0..=12 => self.state.registers[index as usize] = value,
            SP => set_active_sp(&mut self.state, value & !3),
            LR => self.state.lr = value,
            PC => self.state.pc = value & !1,
            _ => return Err(cortex_m0_simulator::CortexM0Error::InvalidRegister { index }.into()),
        }
        Ok(())
    }

    pub fn read_memory(&self, address: u32, width: usize) -> Result<u32, CortexM23Error> {
        let range = checked_data_range(address, width)?;
        check_access(&self.state, address, width, AccessKind::DataRead)?;
        let mut bytes = [0; 4];
        bytes[..width].copy_from_slice(&self.state.memory[range]);
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn write_memory(
        &mut self,
        address: u32,
        width: usize,
        value: u32,
    ) -> Result<(), CortexM23Error> {
        let range = checked_data_range(address, width)?;
        check_access(&self.state, address, width, AccessKind::DataWrite)?;
        self.state.memory[range].copy_from_slice(&value.to_le_bytes()[..width]);
        Ok(())
    }

    #[must_use]
    pub fn attribute(&self, address: u32) -> SecurityAttribute {
        attribute(&self.state, address)
    }

    pub fn configure_sau(
        &mut self,
        enabled: bool,
        all_non_secure: bool,
    ) -> Result<(), CortexM23Error> {
        require_sau_write(&self.state)?;
        self.state.sau_enabled = enabled;
        self.state.sau_all_non_secure = all_non_secure;
        Ok(())
    }

    pub fn configure_sau_region(
        &mut self,
        index: usize,
        region: SauRegion,
    ) -> Result<(), CortexM23Error> {
        require_sau_write(&self.state)?;
        validate_region(index, &region)?;
        self.state.sau_regions[index] = region;
        Ok(())
    }

    /// Checked composition hook for a Secure-to-Non-secure branch.
    pub fn enter_non_secure(&mut self, target: u32) -> Result<(), CortexM23Error> {
        if self.state.security_extension == SecurityExtension::Disabled {
            return Err(CortexM23Error::SecurityExtensionUnavailable);
        }
        if self.state.security_state != SecurityState::Secure
            || target & 1 == 0
            || attribute(&self.state, target & !1) != SecurityAttribute::NonSecure
        {
            return Err(CortexM23Error::InvalidSecurityTransition {
                from: self.state.security_state,
                target,
                required: SecurityAttribute::NonSecure,
            });
        }
        self.state.security_state = SecurityState::NonSecure;
        self.state.pc = target & !1;
        Ok(())
    }

    /// Checked composition hook for entry through an NSC gateway.
    pub fn enter_secure_gateway(&mut self, target: u32) -> Result<(), CortexM23Error> {
        if self.state.security_extension == SecurityExtension::Disabled {
            return Err(CortexM23Error::SecurityExtensionUnavailable);
        }
        if self.state.security_state != SecurityState::NonSecure
            || target & 1 == 0
            || attribute(&self.state, target & !1) != SecurityAttribute::NonSecureCallable
        {
            return Err(CortexM23Error::InvalidSecurityTransition {
                from: self.state.security_state,
                target,
                required: SecurityAttribute::NonSecureCallable,
            });
        }
        self.state.security_state = SecurityState::Secure;
        self.state.pc = target & !1;
        Ok(())
    }

    pub fn signal_event(&mut self) {
        self.state.event_registered = true;
        self.state.sleeping = false;
    }

    pub fn step(&mut self) -> Result<StepTrace, CortexM23Error> {
        if self.state.halted {
            return Err(cortex_m0_simulator::CortexM0Error::Halted.into());
        }
        if self.state.sleeping {
            return Err(cortex_m0_simulator::CortexM0Error::Sleeping.into());
        }
        let before = try_clone_state(&self.state)?;
        let (first, _second, width) = fetch_instruction(&before)?;
        check_access(&before, before.pc, width, AccessKind::Instruction)?;

        // Execute only against the private scratch core first. This performs
        // the inherited decode and semantic checks without changing M23
        // architectural state. Data attribution is then checked before the
        // successful candidate is allowed to commit.
        self.core.restore(to_core_state(&before)?)?;
        let core_trace = self.core.step()?;
        if let Some((address, bytes, access)) = inherited_data_access(&before, first) {
            checked_instruction_data_range(address, bytes)?;
            check_access(&before, address, bytes, access)?;
        }

        let mut next = try_clone_state(&before)?;
        update_from_core(&mut next, core_trace.state_after);
        let state_after = try_clone_state(&next)?;
        let trace = StepTrace {
            pc_before: before.pc,
            pc_after: next.pc,
            raw: core_trace.raw,
            width,
            mnemonic: core_trace.mnemonic,
            security_before: before.security_state,
            security_after: next.security_state,
            state_before: before,
            state_after,
        };
        self.state = next;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, CortexM23Error> {
        check_retention(max_steps)?;
        let before = try_clone_state(&self.state)?;
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| CortexM23Error::AllocationFailed)?;
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
        Err(CortexM23Error::StepLimitExceeded { limit: max_steps })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, CortexM23Error> {
        check_retention(max_steps)?;
        let before = try_clone_state(&self.state)?;
        if let Err(error) = self.load(program, origin) {
            self.state = before;
            return Err(error);
        }
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = before;
                Err(error)
            }
        }
    }
}

fn active_control(state: &CortexM23State) -> u8 {
    match state.security_state {
        SecurityState::Secure => state.control_secure,
        SecurityState::NonSecure => state.control_non_secure,
    }
}

fn active_sp(state: &CortexM23State) -> u32 {
    let use_psp = !state.handler_mode && active_control(state) & 2 != 0;
    match (state.security_state, use_psp) {
        (SecurityState::Secure, false) => state.msp_secure,
        (SecurityState::Secure, true) => state.psp_secure,
        (SecurityState::NonSecure, false) => state.msp_non_secure,
        (SecurityState::NonSecure, true) => state.psp_non_secure,
    }
}

fn set_active_sp(state: &mut CortexM23State, value: u32) {
    let use_psp = !state.handler_mode && active_control(state) & 2 != 0;
    match (state.security_state, use_psp) {
        (SecurityState::Secure, false) => state.msp_secure = value,
        (SecurityState::Secure, true) => state.psp_secure = value,
        (SecurityState::NonSecure, false) => state.msp_non_secure = value,
        (SecurityState::NonSecure, true) => state.psp_non_secure = value,
    }
}

fn try_clone_memory(memory: &[u8]) -> Result<Vec<u8>, CortexM23Error> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(memory.len())
        .map_err(|_| CortexM23Error::AllocationFailed)?;
    copy.extend_from_slice(memory);
    Ok(copy)
}

fn try_reset_state(configuration: SecurityExtension) -> Result<CortexM23State, CortexM23Error> {
    let mut memory = Vec::new();
    memory
        .try_reserve_exact(MEMORY_SIZE)
        .map_err(|_| CortexM23Error::AllocationFailed)?;
    memory.resize(MEMORY_SIZE, 0);
    Ok(CortexM23State::with_memory(configuration, memory))
}

fn validate_program(program: &[u8], origin: u32) -> Result<(), CortexM23Error> {
    if program.is_empty() {
        return Err(cortex_m0_simulator::CortexM0Error::EmptyProgram.into());
    }
    if origin & 1 != 0 {
        return Err(cortex_m0_simulator::CortexM0Error::MisalignedProgram { origin }.into());
    }
    if usize::try_from(origin)
        .ok()
        .and_then(|start| start.checked_add(program.len()))
        .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(cortex_m0_simulator::CortexM0Error::ProgramOutOfRange {
            origin,
            len: program.len(),
        }
        .into());
    }
    Ok(())
}

fn try_clone_state(state: &CortexM23State) -> Result<CortexM23State, CortexM23Error> {
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
        memory: try_clone_memory(&state.memory)?,
        halted: state.halted,
        sleeping: state.sleeping,
        event_registered: state.event_registered,
        loaded_origin: state.loaded_origin,
        loaded_len: state.loaded_len,
    })
}

fn to_core_state(
    state: &CortexM23State,
) -> Result<cortex_m0_simulator::CortexM0State, CortexM23Error> {
    let (msp, psp, primask, control) = match state.security_state {
        SecurityState::Secure => (
            state.msp_secure,
            state.psp_secure,
            state.primask_secure,
            state.control_secure,
        ),
        SecurityState::NonSecure => (
            state.msp_non_secure,
            state.psp_non_secure,
            state.primask_non_secure,
            state.control_non_secure,
        ),
    };
    Ok(cortex_m0_simulator::CortexM0State {
        registers: state.registers,
        msp,
        psp,
        lr: state.lr,
        pc: state.pc,
        apsr: state.apsr,
        ipsr: state.ipsr,
        primask,
        control,
        handler_mode: state.handler_mode,
        memory: try_clone_memory(&state.memory)?,
        halted: state.halted,
        sleeping: state.sleeping,
        event_registered: state.event_registered,
        loaded_origin: state.loaded_origin,
        loaded_len: state.loaded_len,
    })
}

fn update_from_core(state: &mut CortexM23State, core: cortex_m0_simulator::CortexM0State) {
    state.registers = core.registers;
    state.lr = core.lr;
    state.pc = core.pc;
    state.apsr = core.apsr;
    state.ipsr = core.ipsr;
    state.handler_mode = core.handler_mode;
    match state.security_state {
        SecurityState::Secure => {
            state.msp_secure = core.msp;
            state.psp_secure = core.psp;
            state.primask_secure = core.primask;
            state.control_secure = core.control;
        }
        SecurityState::NonSecure => {
            state.msp_non_secure = core.msp;
            state.psp_non_secure = core.psp;
            state.primask_non_secure = core.primask;
            state.control_non_secure = core.control;
        }
    }
    state.memory = core.memory;
    state.halted = core.halted;
    state.sleeping = core.sleeping;
    state.event_registered = core.event_registered;
    state.loaded_origin = core.loaded_origin;
    state.loaded_len = core.loaded_len;
}

fn validate_state(state: &CortexM23State) -> Result<(), CortexM23Error> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(cortex_m0_simulator::CortexM0Error::InvalidState {
            reason: "memory must be exactly 64 KiB",
        }
        .into());
    }
    if state.pc & 1 != 0
        || [
            state.msp_secure,
            state.psp_secure,
            state.msp_non_secure,
            state.psp_non_secure,
        ]
        .iter()
        .any(|pointer| pointer & 3 != 0)
    {
        return Err(cortex_m0_simulator::CortexM0Error::InvalidState {
            reason: "PC or stack pointer alignment is invalid",
        }
        .into());
    }
    if state.apsr & !APSR_MASK != 0
        || state.control_secure & !3 != 0
        || state.control_non_secure & !3 != 0
        || state.ipsr > 63
        || state.handler_mode != (state.ipsr != 0)
    {
        return Err(cortex_m0_simulator::CortexM0Error::InvalidState {
            reason: "reserved status bits or mode consistency are invalid",
        }
        .into());
    }
    if state.security_extension == SecurityExtension::Disabled
        && (state.security_state != SecurityState::NonSecure
            || state.sau_enabled
            || !state.sau_all_non_secure
            || state.sau_regions.iter().any(|region| region.enabled))
    {
        return Err(cortex_m0_simulator::CortexM0Error::InvalidState {
            reason: "Security-disabled configuration must remain wholly Non-secure",
        }
        .into());
    }
    for (index, region) in state.sau_regions.iter().enumerate() {
        validate_region(index, region)?;
    }
    if state.loaded_len == 0 {
        if state.loaded_origin != 0 {
            return Err(cortex_m0_simulator::CortexM0Error::InvalidState {
                reason: "empty image must use origin zero",
            }
            .into());
        }
    } else if state.loaded_origin & 1 != 0
        || usize::try_from(state.loaded_origin)
            .ok()
            .and_then(|start| start.checked_add(state.loaded_len))
            .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(cortex_m0_simulator::CortexM0Error::InvalidState {
            reason: "installed image range is invalid",
        }
        .into());
    }
    Ok(())
}

fn validate_region(index: usize, region: &SauRegion) -> Result<(), CortexM23Error> {
    if index >= SAU_REGION_COUNT {
        return Err(CortexM23Error::InvalidSauRegion {
            index,
            reason: "index is out of range",
        });
    }
    if region.enabled
        && (region.base & 0x1f != 0
            || region.limit & 0x1f != 0x1f
            || region.base > region.limit
            || region.limit as usize >= MEMORY_SIZE)
    {
        return Err(CortexM23Error::InvalidSauRegion {
            index,
            reason: "enabled range must be 32-byte granular, ordered, and owned",
        });
    }
    Ok(())
}

fn require_sau_write(state: &CortexM23State) -> Result<(), CortexM23Error> {
    if state.security_extension == SecurityExtension::Disabled {
        return Err(CortexM23Error::SecurityExtensionUnavailable);
    }
    if state.security_state != SecurityState::Secure {
        return Err(CortexM23Error::SecureOnlyOperation);
    }
    if !state.handler_mode && state.control_secure & 1 != 0 {
        return Err(CortexM23Error::UnprivilegedOperation);
    }
    Ok(())
}

fn attribute(state: &CortexM23State, address: u32) -> SecurityAttribute {
    if state.security_extension == SecurityExtension::Disabled {
        return SecurityAttribute::NonSecure;
    }
    let mut matches = state.sau_regions.iter().filter(|region| {
        state.sau_enabled && region.enabled && address >= region.base && address <= region.limit
    });
    let Some(first) = matches.next() else {
        return if state.sau_all_non_secure {
            SecurityAttribute::NonSecure
        } else {
            SecurityAttribute::Secure
        };
    };
    if matches.next().is_some() {
        SecurityAttribute::Secure
    } else if first.non_secure_callable {
        SecurityAttribute::NonSecureCallable
    } else {
        SecurityAttribute::NonSecure
    }
}

fn check_access(
    state: &CortexM23State,
    address: u32,
    width: usize,
    access: AccessKind,
) -> Result<(), CortexM23Error> {
    if state.security_state == SecurityState::Secure {
        return Ok(());
    }
    let end = address.saturating_add(u32::try_from(width.saturating_sub(1)).unwrap_or(u32::MAX));
    let offending =
        (address..=end).find(|&byte| attribute(state, byte) != SecurityAttribute::NonSecure);
    if let Some(byte) = offending {
        return Err(CortexM23Error::SecurityAttributionFault {
            address,
            width,
            access,
            state: state.security_state,
            attribute: attribute(state, byte),
        });
    }
    Ok(())
}

fn checked_data_range(
    address: u32,
    width: usize,
) -> Result<std::ops::Range<usize>, CortexM23Error> {
    if !matches!(width, 1 | 2 | 4) {
        return Err(cortex_m0_simulator::CortexM0Error::DataOutOfRange { address, width }.into());
    }
    if width > 1 && address & (width as u32 - 1) != 0 {
        return Err(cortex_m0_simulator::CortexM0Error::MisalignedData { address, width }.into());
    }
    checked_transfer_range(address, width)
}

fn checked_transfer_range(
    address: u32,
    width: usize,
) -> Result<std::ops::Range<usize>, CortexM23Error> {
    let start = address as usize;
    let Some(end) = start.checked_add(width) else {
        return Err(cortex_m0_simulator::CortexM0Error::DataOutOfRange { address, width }.into());
    };
    if end > MEMORY_SIZE {
        return Err(cortex_m0_simulator::CortexM0Error::DataOutOfRange { address, width }.into());
    }
    Ok(start..end)
}

fn checked_instruction_data_range(
    address: u32,
    width: usize,
) -> Result<std::ops::Range<usize>, CortexM23Error> {
    if width <= 4 {
        return checked_data_range(address, width);
    }
    for offset in (0..width).step_by(4) {
        let word_address = address.wrapping_add(offset as u32);
        checked_data_range(word_address, 4)?;
    }
    checked_transfer_range(address, width)
}

fn fetch_instruction(state: &CortexM23State) -> Result<(u16, Option<u16>, usize), CortexM23Error> {
    let pc = state.pc;
    if pc & 1 != 0 {
        return Err(cortex_m0_simulator::CortexM0Error::MisalignedFetch { pc }.into());
    }
    if state.loaded_len < 2
        || pc.checked_sub(state.loaded_origin).is_none_or(|offset| {
            usize::try_from(offset)
                .ok()
                .is_none_or(|relative| relative > state.loaded_len.saturating_sub(2))
        })
    {
        return Err(
            cortex_m0_simulator::CortexM0Error::FetchOutsideProgram { pc, width: 2 }.into(),
        );
    }
    let start = pc as usize;
    let first = u16::from_le_bytes([state.memory[start], state.memory[start + 1]]);
    let width = if matches!(first >> 11, 0b11101..=0b11111) {
        4
    } else {
        2
    };
    let second = if width == 4 {
        let relative = pc.saturating_sub(state.loaded_origin) as usize;
        if relative > state.loaded_len.saturating_sub(4) {
            return Err(
                cortex_m0_simulator::CortexM0Error::FetchOutsideProgram { pc, width: 4 }.into(),
            );
        }
        Some(u16::from_le_bytes([
            state.memory[start + 2],
            state.memory[start + 3],
        ]))
    } else {
        None
    };
    Ok((first, second, width))
}

fn inherited_data_access(state: &CortexM23State, raw: u16) -> Option<(u32, usize, AccessKind)> {
    if raw & 0xf800 == 0x4800 {
        let address = (state.pc.wrapping_add(4) & !3).wrapping_add(u32::from(raw & 0xff) * 4);
        return Some((address, 4, AccessKind::DataRead));
    }
    if raw & 0xf000 == 0x5000 {
        let operation = (raw >> 9) & 7;
        let rm = ((raw >> 6) & 7) as usize;
        let rn = ((raw >> 3) & 7) as usize;
        let address = state.registers[rn].wrapping_add(state.registers[rm]);
        let width = [4, 2, 1, 1, 4, 2, 1, 2][operation as usize];
        let access = if operation < 3 {
            AccessKind::DataWrite
        } else {
            AccessKind::DataRead
        };
        return Some((address, width, access));
    }
    if matches!(raw >> 12, 0b0110..=0b1000) {
        let operation = raw >> 11;
        let immediate = u32::from((raw >> 6) & 0x1f);
        let rn = ((raw >> 3) & 7) as usize;
        let (scale, load, width) = match operation {
            0b01100 => (4, false, 4),
            0b01101 => (4, true, 4),
            0b01110 => (1, false, 1),
            0b01111 => (1, true, 1),
            0b10000 => (2, false, 2),
            _ => (2, true, 2),
        };
        return Some((
            state.registers[rn].wrapping_add(immediate * scale),
            width,
            if load {
                AccessKind::DataRead
            } else {
                AccessKind::DataWrite
            },
        ));
    }
    if raw & 0xf000 == 0x9000 {
        return Some((
            active_sp(state).wrapping_add(u32::from(raw & 0xff) * 4),
            4,
            if raw & 0x0800 != 0 {
                AccessKind::DataRead
            } else {
                AccessKind::DataWrite
            },
        ));
    }
    if raw & 0xfe00 == 0xb400 {
        let count = (raw & 0xff).count_ones() + u32::from(raw & 0x100 != 0);
        if count != 0 {
            return Some((
                active_sp(state).wrapping_sub(count * 4),
                count as usize * 4,
                AccessKind::DataWrite,
            ));
        }
    }
    if raw & 0xfe00 == 0xbc00 {
        let count = (raw & 0xff).count_ones() + u32::from(raw & 0x100 != 0);
        if count != 0 {
            return Some((active_sp(state), count as usize * 4, AccessKind::DataRead));
        }
    }
    if raw & 0xf000 == 0xc000 {
        let count = (raw & 0xff).count_ones();
        if count != 0 {
            let rn = ((raw >> 8) & 7) as usize;
            return Some((
                state.registers[rn],
                count as usize * 4,
                if raw & 0x0800 != 0 {
                    AccessKind::DataRead
                } else {
                    AccessKind::DataWrite
                },
            ));
        }
    }
    None
}

fn check_retention(max_steps: usize) -> Result<(), CortexM23Error> {
    if max_steps > MAX_RETAINED_STEPS {
        return Err(CortexM23Error::RetentionLimitExceeded {
            requested: max_steps,
            limit: MAX_RETAINED_STEPS,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(words: &[u16]) -> Vec<u8> {
        encoding::program(words)
    }

    fn snapshot(simulator: &CortexM23Simulator) -> CortexM23State {
        simulator.try_get_state().expect("state snapshot")
    }

    fn non_secure_program(words: &[u16]) -> CortexM23Simulator {
        let mut simulator = CortexM23Simulator::default();
        simulator.load(&bytes(words), 0).expect("load");
        simulator
            .configure_sau_region(
                0,
                SauRegion {
                    enabled: true,
                    base: 0,
                    limit: 31,
                    non_secure_callable: false,
                },
            )
            .expect("region");
        simulator.configure_sau(true, false).expect("enable SAU");
        simulator.enter_non_secure(1).expect("enter NS");
        simulator
    }

    #[test]
    fn reset_configuration_and_program_validation_are_explicit() {
        let mut enabled = CortexM23Simulator::default();
        let state = snapshot(&enabled);
        assert_eq!(state.security_state, SecurityState::Secure);
        assert!(!state.sau_all_non_secure);
        assert_eq!(enabled.attribute(0), SecurityAttribute::Secure);
        enabled.write_register(0, 9).unwrap();
        enabled.reset();
        assert_eq!(enabled.read_register(0).unwrap(), 0);

        let mut disabled = CortexM23Simulator::new(SecurityExtension::Disabled);
        assert_eq!(disabled.attribute(0), SecurityAttribute::NonSecure);
        assert_eq!(snapshot(&disabled).security_state, SecurityState::NonSecure);
        assert_eq!(
            disabled.load(&[], 0),
            Err(CortexM23Error::Core(
                cortex_m0_simulator::CortexM0Error::EmptyProgram
            ))
        );
        assert_eq!(
            disabled.load(&[0, 0], 1),
            Err(CortexM23Error::Core(
                cortex_m0_simulator::CortexM0Error::MisalignedProgram { origin: 1 }
            ))
        );
        assert_eq!(
            disabled.load(&[0; 4], (MEMORY_SIZE - 2) as u32),
            Err(CortexM23Error::Core(
                cortex_m0_simulator::CortexM0Error::ProgramOutOfRange {
                    origin: (MEMORY_SIZE - 2) as u32,
                    len: 4
                }
            ))
        );
    }

    #[test]
    fn sau_default_single_overlap_nsc_and_disabled_rules_are_deterministic() {
        let mut simulator = CortexM23Simulator::default();
        simulator
            .configure_sau_region(
                0,
                SauRegion {
                    enabled: true,
                    base: 0x100,
                    limit: 0x17f,
                    non_secure_callable: false,
                },
            )
            .unwrap();
        simulator
            .configure_sau_region(
                1,
                SauRegion {
                    enabled: true,
                    base: 0x140,
                    limit: 0x15f,
                    non_secure_callable: true,
                },
            )
            .unwrap();
        simulator.configure_sau(true, false).unwrap();
        assert_eq!(simulator.attribute(0x100), SecurityAttribute::NonSecure);
        assert_eq!(simulator.attribute(0x150), SecurityAttribute::Secure);
        assert_eq!(simulator.attribute(0x180), SecurityAttribute::Secure);
        simulator
            .configure_sau_region(
                0,
                SauRegion {
                    enabled: false,
                    ..SauRegion::default()
                },
            )
            .unwrap();
        assert_eq!(
            simulator.attribute(0x150),
            SecurityAttribute::NonSecureCallable
        );
        simulator.configure_sau(false, true).unwrap();
        assert_eq!(simulator.attribute(0x150), SecurityAttribute::NonSecure);
        assert_eq!(simulator.attribute(0xf000), SecurityAttribute::NonSecure);
    }

    #[test]
    fn sau_writes_require_extension_secure_privilege_and_valid_regions() {
        let mut absent = CortexM23Simulator::new(SecurityExtension::Disabled);
        assert_eq!(
            absent.configure_sau(true, false),
            Err(CortexM23Error::SecurityExtensionUnavailable)
        );
        let mut simulator = non_secure_program(&[encoding::nop()]);
        assert_eq!(
            simulator.configure_sau(false, false),
            Err(CortexM23Error::SecureOnlyOperation)
        );

        let mut simulator = CortexM23Simulator::default();
        let mut state = snapshot(&simulator);
        state.control_secure = 1;
        simulator.restore(state).unwrap();
        assert_eq!(
            simulator.configure_sau(true, false),
            Err(CortexM23Error::UnprivilegedOperation)
        );
        let invalids = [
            (
                8,
                SauRegion {
                    enabled: false,
                    ..SauRegion::default()
                },
            ),
            (
                0,
                SauRegion {
                    enabled: true,
                    base: 1,
                    limit: 31,
                    non_secure_callable: false,
                },
            ),
            (
                0,
                SauRegion {
                    enabled: true,
                    base: 64,
                    limit: 31,
                    non_secure_callable: false,
                },
            ),
        ];
        let mut privileged = CortexM23Simulator::default();
        for (index, region) in invalids {
            assert!(matches!(
                privileged.configure_sau_region(index, region),
                Err(CortexM23Error::InvalidSauRegion { .. })
            ));
        }
    }

    #[test]
    fn transitions_are_checked_and_banked_state_is_independent() {
        let mut simulator = CortexM23Simulator::default();
        simulator.load(&bytes(&[encoding::nop(); 32]), 0).unwrap();
        simulator.write_register(SP, 0x800).unwrap();
        simulator
            .configure_sau_region(
                0,
                SauRegion {
                    enabled: true,
                    base: 0,
                    limit: 31,
                    non_secure_callable: false,
                },
            )
            .unwrap();
        simulator
            .configure_sau_region(
                1,
                SauRegion {
                    enabled: true,
                    base: 32,
                    limit: 63,
                    non_secure_callable: true,
                },
            )
            .unwrap();
        simulator.configure_sau(true, false).unwrap();
        assert!(matches!(
            simulator.enter_non_secure(0),
            Err(CortexM23Error::InvalidSecurityTransition { .. })
        ));
        simulator.enter_non_secure(1).unwrap();
        assert_eq!(simulator.read_register(SP).unwrap(), MEMORY_SIZE as u32);
        simulator.write_register(SP, 0x900).unwrap();
        assert!(matches!(
            simulator.enter_non_secure(1),
            Err(CortexM23Error::InvalidSecurityTransition { .. })
        ));
        simulator.enter_secure_gateway(33).unwrap();
        assert_eq!(simulator.read_register(SP).unwrap(), 0x800);
        assert_eq!(snapshot(&simulator).msp_non_secure, 0x900);

        let mut absent = CortexM23Simulator::new(SecurityExtension::Disabled);
        assert_eq!(
            absent.enter_non_secure(1),
            Err(CortexM23Error::SecurityExtensionUnavailable)
        );
        assert_eq!(
            absent.enter_secure_gateway(1),
            Err(CortexM23Error::SecurityExtensionUnavailable)
        );
    }

    #[test]
    fn direct_memory_and_register_apis_are_checked() {
        let mut simulator = CortexM23Simulator::new(SecurityExtension::Disabled);
        simulator.write_memory(0x100, 4, 0x1122_3344).unwrap();
        assert_eq!(simulator.read_memory(0x100, 4).unwrap(), 0x1122_3344);
        assert_eq!(simulator.read_memory(0x101, 1).unwrap(), 0x33);
        assert!(matches!(
            simulator.read_memory(1, 2),
            Err(CortexM23Error::Core(
                cortex_m0_simulator::CortexM0Error::MisalignedData { .. }
            ))
        ));
        assert!(matches!(
            simulator.write_memory(MEMORY_SIZE as u32, 1, 0),
            Err(CortexM23Error::Core(
                cortex_m0_simulator::CortexM0Error::DataOutOfRange { .. }
            ))
        ));
        assert!(simulator.read_memory(0, 3).is_err());
        assert!(simulator.read_register(16).is_err());
        assert!(simulator.write_register(16, 0).is_err());
        simulator.write_register(PC, 3).unwrap();
        simulator.write_register(LR, 7).unwrap();
        assert_eq!(simulator.read_register(PC).unwrap(), 2);
        assert_eq!(simulator.read_register(LR).unwrap(), 7);
    }

    #[test]
    fn non_secure_direct_access_rejects_secure_and_nsc_ranges_atomically() {
        let mut simulator = non_secure_program(&[encoding::nop()]);
        let before = snapshot(&simulator);
        let error = simulator.write_memory(0x100, 4, 1).unwrap_err();
        assert!(matches!(
            error,
            CortexM23Error::SecurityAttributionFault {
                access: AccessKind::DataWrite,
                attribute: SecurityAttribute::Secure,
                ..
            }
        ));
        assert_eq!(snapshot(&simulator), before);
    }

    #[test]
    fn fetch_attribution_fault_is_typed_and_atomic() {
        let mut simulator = non_secure_program(&[encoding::nop(); 17]);
        let mut state = snapshot(&simulator);
        state.pc = 32;
        simulator.restore(state).unwrap();
        let before = snapshot(&simulator);
        assert!(matches!(
            simulator.step(),
            Err(CortexM23Error::SecurityAttributionFault {
                access: AccessKind::Instruction,
                ..
            })
        ));
        assert_eq!(snapshot(&simulator), before);
    }

    #[test]
    fn wide_fetch_cannot_straddle_a_non_secure_boundary() {
        let mut simulator = CortexM23Simulator::default();
        simulator.load(&bytes(&[encoding::nop(); 17]), 0).unwrap();
        simulator
            .configure_sau_region(
                0,
                SauRegion {
                    enabled: true,
                    base: 0,
                    limit: 31,
                    non_secure_callable: false,
                },
            )
            .unwrap();
        simulator.configure_sau(true, false).unwrap();
        simulator.enter_non_secure(31).unwrap();
        let mut state = snapshot(&simulator);
        state.pc = 30;
        state.memory[30..34].copy_from_slice(&[0x00, 0xf0, 0x00, 0xf8]);
        simulator.restore(state).unwrap();
        assert!(matches!(
            simulator.step(),
            Err(CortexM23Error::SecurityAttributionFault {
                access: AccessKind::Instruction,
                ..
            })
        ));
    }

    #[test]
    fn every_inherited_memory_family_is_security_preflighted() {
        let cases = [
            (0x4807, 0x100, MEMORY_SIZE as u32), // literal LDR -> address 32
            (0x5808, 0x100, MEMORY_SIZE as u32), // register LDR
            (0x6808, 0x100, MEMORY_SIZE as u32), // immediate word LDR
            (0x8808, 0x100, MEMORY_SIZE as u32), // immediate halfword LDR
            (0x9800, 0, 0x100),                  // SP-relative LDR
            (0xb401, 0, 0x104),                  // PUSH
            (0xbc01, 0, 0x100),                  // POP
            (0xc101, 0x100, MEMORY_SIZE as u32), // STM
            (0xc901, 0x100, MEMORY_SIZE as u32), // LDM
        ];
        for (raw, r1, sp) in cases {
            let mut simulator = non_secure_program(&[raw]);
            let mut state = snapshot(&simulator);
            state.registers[1] = r1;
            state.msp_non_secure = sp;
            simulator.restore(state).unwrap();
            let before = snapshot(&simulator);
            assert!(
                matches!(
                    simulator.step(),
                    Err(CortexM23Error::SecurityAttributionFault { .. })
                ),
                "raw={raw:#06x}"
            );
            assert_eq!(snapshot(&simulator), before);
        }
    }

    #[test]
    fn register_memory_store_and_byte_paths_are_preflighted() {
        for raw in [0x5008, 0x5208, 0x5408, 0x5608, 0x5a08, 0x5c08, 0x5e08] {
            let mut simulator = non_secure_program(&[raw]);
            let mut state = snapshot(&simulator);
            state.registers[1] = 0x100;
            simulator.restore(state).unwrap();
            assert!(matches!(
                simulator.step(),
                Err(CortexM23Error::SecurityAttributionFault { .. })
            ));
        }
        for raw in [0x6008, 0x7008, 0x7808, 0x8008] {
            let mut simulator = non_secure_program(&[raw]);
            let mut state = snapshot(&simulator);
            state.registers[1] = 0x100;
            simulator.restore(state).unwrap();
            assert!(matches!(
                simulator.step(),
                Err(CortexM23Error::SecurityAttributionFault { .. })
            ));
        }
    }

    #[test]
    fn inherited_alignment_fault_precedes_security_attribution() {
        let mut simulator = non_secure_program(&[0x5808]);
        let mut state = snapshot(&simulator);
        state.registers[1] = 0x101;
        simulator.restore(state).unwrap();
        assert!(matches!(
            simulator.step(),
            Err(CortexM23Error::Core(
                cortex_m0_simulator::CortexM0Error::MisalignedData {
                    address: 0x101,
                    width: 4
                }
            ))
        ));
    }

    #[test]
    fn inherited_invalid_multiple_encoding_precedes_security_attribution() {
        // STMIA r1!, {r0, r1} is invalid because the writeback base is in the
        // list but is not the lowest-numbered register. Its apparent data
        // address is Secure, so this guards the decode-vs-SAU fault priority.
        let raw = 0xc103;
        let mut simulator = non_secure_program(&[raw]);
        let mut state = snapshot(&simulator);
        state.registers[1] = 0x100;
        simulator.restore(state).unwrap();
        let before = snapshot(&simulator);
        let error = simulator.step().unwrap_err();
        assert_eq!(
            error,
            CortexM23Error::Core(cortex_m0_simulator::CortexM0Error::UnknownInstruction16 {
                raw,
                pc: 0
            })
        );
        assert_eq!(snapshot(&simulator), before);

        let mut core = cortex_m0_simulator::CortexM0Simulator::new();
        core.load(&bytes(&[raw]), 0).unwrap();
        core.write_register(1, 0x100).unwrap();
        assert_eq!(
            core.step(),
            Err(cortex_m0_simulator::CortexM0Error::UnknownInstruction16 { raw, pc: 0 })
        );
    }

    #[test]
    fn inherited_instruction_families_match_cortex_m0_exactly() {
        let bl = encoding::bl(0);
        let cases: &[&[u16]] = &[
            &[0x0008],
            &[encoding::movs(0, 7)],
            &[0x4008],
            &[0x4608],
            &[0x4800, 0, 0],
            &[0x5808],
            &[0x6808],
            &[0x9800],
            &[0xa000],
            &[0xb200],
            &[0xc901],
            &[encoding::b_cond(1, 0)],
            &[encoding::branch(0)],
            &[bl[0], bl[1]],
            &[0xf3bf, 0x8f4f],
            &[0xf3ef, 0x8000],
        ];
        for words in cases {
            let program = bytes(words);
            let mut m0 = cortex_m0_simulator::CortexM0Simulator::new();
            m0.load(&program, 0).unwrap();
            m0.write_register(0, 0).unwrap();
            m0.write_register(1, 0).unwrap();
            m0.write_register(SP, 0).unwrap();
            let expected = m0.step();

            let mut m23 = CortexM23Simulator::new(SecurityExtension::Disabled);
            m23.load(&program, 0).unwrap();
            m23.write_register(0, 0).unwrap();
            m23.write_register(1, 0).unwrap();
            m23.write_register(SP, 0).unwrap();
            let actual = m23.step();
            match (expected, actual) {
                (Ok(expected), Ok(actual)) => {
                    assert_eq!(actual.raw, expected.raw);
                    assert_eq!(actual.width, expected.width);
                    assert_eq!(actual.mnemonic, expected.mnemonic);
                    let converted = to_core_state(&actual.state_after).unwrap();
                    assert_eq!(converted, expected.state_after);
                }
                (Err(expected), Err(CortexM23Error::Core(actual))) => {
                    assert_eq!(actual, expected);
                }
                pair => panic!("differential mismatch: {pair:?}"),
            }
        }
    }

    #[test]
    fn successful_trace_is_complete_and_halt_run_is_bounded() {
        let program = bytes(&[encoding::movs(0, 9), encoding::bkpt(0)]);
        let mut simulator = CortexM23Simulator::new(SecurityExtension::Disabled);
        simulator.load(&program, 0).unwrap();
        let trace = simulator.step().unwrap();
        assert_eq!(trace.pc_before, 0);
        assert_eq!(trace.pc_after, 2);
        assert_eq!(trace.raw[..2], encoding::movs(0, 9).to_le_bytes());
        assert_eq!(trace.state_after.registers[0], 9);
        let result = simulator.run(2).unwrap();
        assert!(result.stopped);
        assert_eq!(result.steps, 1);
        assert!(result.final_state.halted);
    }

    #[test]
    fn run_and_execute_roll_back_complete_state_on_failure() {
        let mut simulator = CortexM23Simulator::new(SecurityExtension::Disabled);
        simulator.load(&bytes(&[encoding::nop()]), 0).unwrap();
        let before = snapshot(&simulator);
        assert_eq!(
            simulator.run(1),
            Err(CortexM23Error::StepLimitExceeded { limit: 1 })
        );
        assert_eq!(snapshot(&simulator), before);

        let error = simulator
            .execute(&bytes(&[encoding::bkpt(1)]), 0, 2)
            .unwrap_err();
        assert!(matches!(
            error,
            CortexM23Error::Core(cortex_m0_simulator::CortexM0Error::UnsupportedBreakpoint { .. })
        ));
        assert_eq!(snapshot(&simulator), before);
        assert_eq!(
            simulator.run(MAX_RETAINED_STEPS + 1),
            Err(CortexM23Error::RetentionLimitExceeded {
                requested: MAX_RETAINED_STEPS + 1,
                limit: MAX_RETAINED_STEPS
            })
        );

        let result = simulator
            .execute(&bytes(&[encoding::bkpt(0)]), 0, 1)
            .unwrap();
        assert!(result.stopped);
    }

    #[test]
    fn sleeping_halted_and_event_lifecycle_is_checked() {
        let mut simulator = CortexM23Simulator::new(SecurityExtension::Disabled);
        simulator
            .load(&bytes(&[0xbf20, encoding::nop()]), 0)
            .unwrap();
        simulator.step().unwrap();
        assert!(matches!(
            simulator.step(),
            Err(CortexM23Error::Core(
                cortex_m0_simulator::CortexM0Error::Sleeping
            ))
        ));
        simulator.signal_event();
        simulator.step().unwrap();
        simulator.load(&bytes(&[encoding::bkpt(0)]), 0).unwrap();
        simulator.step().unwrap();
        assert!(matches!(
            simulator.step(),
            Err(CortexM23Error::Core(
                cortex_m0_simulator::CortexM0Error::Halted
            ))
        ));
    }

    #[test]
    fn restore_rejects_incomplete_or_inconsistent_state_atomically() {
        let mut simulator = CortexM23Simulator::default();
        let before = snapshot(&simulator);
        let mut invalid = before.clone();
        invalid.memory.pop();
        assert!(simulator.restore(invalid).is_err());
        assert_eq!(snapshot(&simulator), before);

        let mut invalid = before.clone();
        invalid.pc = 1;
        assert!(simulator.restore(invalid).is_err());
        let mut invalid = before.clone();
        invalid.control_secure = 4;
        assert!(simulator.restore(invalid).is_err());
        let mut invalid = before.clone();
        invalid.handler_mode = true;
        assert!(simulator.restore(invalid).is_err());

        let mut absent = CortexM23State::reset(SecurityExtension::Disabled);
        absent.security_state = SecurityState::Secure;
        assert!(simulator.restore(absent).is_err());
    }

    #[test]
    fn error_messages_are_actionable() {
        assert!(CortexM23Error::SecurityExtensionUnavailable
            .to_string()
            .contains("Security Extension"));
        assert!(CortexM23Error::UnprivilegedOperation
            .to_string()
            .contains("privilege"));
        assert!(CortexM23Error::InvalidSauRegion {
            index: 2,
            reason: "bad"
        }
        .to_string()
        .contains("region 2"));
    }
}
