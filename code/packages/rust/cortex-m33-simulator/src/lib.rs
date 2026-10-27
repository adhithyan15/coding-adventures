//! Checked bounded Armv8-M Mainline model of one Cortex-M33 core.
//!
//! The arithmetic and Thumb execution engine is the repository's checked
//! Cortex-M3 cell. This layer owns the state that Armv8-M adds around that
//! engine: security attribution, two banks of privileged control state, stack
//! limits, and atomic transitions between Secure and Non-secure execution.

use core::fmt;
use cortex_m3_simulator::{
    classify, CortexM3Error, CortexM3Simulator, CortexM3State, InstructionClass, MEMORY_SIZE, PC,
    SP,
};

/// Maximum complete-state traces retained by one bounded run.
pub const MAX_RETAINED_STEPS: usize = 64;
/// Number of programmable teaching-model SAU regions.
pub const SAU_REGION_COUNT: usize = 8;

/// Current Armv8-M security state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecurityState {
    Secure,
    NonSecure,
}

/// Result of the teaching SAU's address classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecurityAttribute {
    Secure,
    NonSecure,
    NonSecureCallable,
}

/// One 32-byte-granular SAU region.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SauRegion {
    pub base: u32,
    /// Inclusive limit; a valid enabled limit has low five bits set.
    pub limit: u32,
    pub enabled: bool,
    pub non_secure_callable: bool,
}

/// The registers banked by security state in this bounded Mainline model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SecurityBank {
    pub msp: u32,
    pub psp: u32,
    pub msplim: u32,
    pub psplim: u32,
    pub primask: bool,
    pub basepri: u8,
    pub faultmask: bool,
    pub control: u8,
}

impl Default for SecurityBank {
    fn default() -> Self {
        Self {
            msp: MEMORY_SIZE as u32,
            psp: MEMORY_SIZE as u32,
            msplim: 0,
            psplim: 0,
            primask: false,
            basepri: 0,
            faultmask: false,
            control: 0,
        }
    }
}

/// Complete restorable Cortex-M33 state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CortexM33State {
    /// Shared integer/status/memory state and the currently active bank.
    pub core: CortexM3State,
    pub secure_bank: SecurityBank,
    pub non_secure_bank: SecurityBank,
    pub security_state: SecurityState,
    pub security_extension: bool,
    pub sau_enabled: bool,
    pub sau_all_non_secure: bool,
    pub sau_regions: [SauRegion; SAU_REGION_COUNT],
}

impl CortexM33State {
    fn reset(security_extension: bool) -> Self {
        let security_state = if security_extension {
            SecurityState::Secure
        } else {
            SecurityState::NonSecure
        };
        let mut state = Self {
            core: CortexM3State::default(),
            secure_bank: SecurityBank::default(),
            non_secure_bank: SecurityBank::default(),
            security_state,
            security_extension,
            sau_enabled: false,
            sau_all_non_secure: false,
            sau_regions: [SauRegion::default(); SAU_REGION_COUNT],
        };
        load_active_bank(&mut state);
        state
    }
}

/// Typed lifecycle and security failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CortexM33Error {
    Core(CortexM3Error),
    SecurityExtensionUnavailable,
    PrivilegeViolation,
    InvalidSauRegion {
        reason: &'static str,
    },
    InvalidSecurityTransition {
        reason: &'static str,
    },
    SecurityAccessViolation {
        address: u32,
        width: usize,
        attribute: SecurityAttribute,
    },
    StackLimitViolation {
        stack_pointer: u32,
        limit: u32,
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

impl fmt::Display for CortexM33Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => error.fmt(formatter),
            Self::SecurityExtensionUnavailable => {
                formatter.write_str("Cortex-M33 Security Extension is unavailable")
            }
            Self::PrivilegeViolation => {
                formatter.write_str("operation requires privileged execution")
            }
            Self::InvalidSauRegion { reason } => write!(formatter, "invalid SAU region: {reason}"),
            Self::InvalidSecurityTransition { reason } => {
                write!(formatter, "invalid security transition: {reason}")
            }
            Self::SecurityAccessViolation {
                address,
                width,
                attribute,
            } => write!(
                formatter,
                "disallowed {width}-byte access at {address:#010x} targets {attribute:?} memory"
            ),
            Self::StackLimitViolation {
                stack_pointer,
                limit,
            } => write!(
                formatter,
                "stack pointer {stack_pointer:#010x} crossed limit {limit:#010x}"
            ),
            Self::RetentionLimitExceeded { requested, limit } => write!(
                formatter,
                "requested {requested} traces exceeds retention limit {limit}"
            ),
            Self::StepLimitExceeded { limit } => write!(formatter, "step limit {limit} exhausted"),
            Self::AllocationFailed => formatter.write_str("trace allocation failed"),
        }
    }
}

impl std::error::Error for CortexM33Error {}

impl From<CortexM3Error> for CortexM33Error {
    fn from(value: CortexM3Error) -> Self {
        Self::Core(value)
    }
}

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
    pub state_before: CortexM33State,
    pub state_after: CortexM33State,
}

/// Successful bounded execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub stopped: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: CortexM33State,
}

/// Bounded Cortex-M33 functional machine.
#[derive(Clone, Debug)]
pub struct CortexM33Simulator {
    state: CortexM33State,
}

impl Default for CortexM33Simulator {
    fn default() -> Self {
        Self::new(true)
    }
}

impl CortexM33Simulator {
    /// Construct a reset core, optionally including the Security Extension.
    #[must_use]
    pub fn new(security_extension: bool) -> Self {
        Self {
            state: CortexM33State::reset(security_extension),
        }
    }

    pub fn reset(&mut self) {
        self.state = CortexM33State::reset(self.state.security_extension);
    }

    #[must_use]
    pub const fn security_state(&self) -> SecurityState {
        self.state.security_state
    }

    pub fn get_state(&self) -> Result<CortexM33State, CortexM33Error> {
        try_clone_state(&self.state)
    }

    pub fn restore(&mut self, state: CortexM33State) -> Result<(), CortexM33Error> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    /// Reset, validate, and install a program atomically.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), CortexM33Error> {
        let before = try_clone_state(&self.state)?;
        let mut core = CortexM3Simulator::new();
        if let Err(error) = core.load(program, origin) {
            return Err(error.into());
        }
        self.reset();
        self.state.core = core.get_state();
        load_active_bank(&mut self.state);
        if let Err(error) = self.authorize_fetch_range(origin, program.len()) {
            self.state = before;
            return Err(error);
        }
        Ok(())
    }

    /// Configure one SAU region. Only privileged Secure code can do so.
    pub fn configure_sau_region(
        &mut self,
        index: usize,
        region: SauRegion,
    ) -> Result<(), CortexM33Error> {
        self.require_privileged_secure()?;
        if index >= SAU_REGION_COUNT {
            return Err(CortexM33Error::InvalidSauRegion {
                reason: "region index is out of range",
            });
        }
        validate_region(region)?;
        self.state.sau_regions[index] = region;
        Ok(())
    }

    /// Set SAU ENABLE and ALLNS controls.
    pub fn set_sau_control(
        &mut self,
        enabled: bool,
        all_non_secure: bool,
    ) -> Result<(), CortexM33Error> {
        self.require_privileged_secure()?;
        self.state.sau_enabled = enabled;
        self.state.sau_all_non_secure = all_non_secure;
        Ok(())
    }

    #[must_use]
    pub fn attribute(&self, address: u32) -> SecurityAttribute {
        attribute(&self.state, address)
    }

    /// Set the active security bank's stack-limit registers.
    pub fn set_stack_limits(&mut self, msplim: u32, psplim: u32) -> Result<(), CortexM33Error> {
        if !self.state.core.handler_mode && self.state.core.control & 1 != 0 {
            return Err(CortexM33Error::PrivilegeViolation);
        }
        if msplim & 7 != 0
            || psplim & 7 != 0
            || msplim > MEMORY_SIZE as u32
            || psplim > MEMORY_SIZE as u32
        {
            return Err(CortexM33Error::Core(CortexM3Error::InvalidState {
                reason: "stack limits must be 8-byte aligned and inside memory",
            }));
        }
        let mut bank = active_bank(&self.state);
        bank.msplim = msplim;
        bank.psplim = psplim;
        validate_bank_stack_limits(bank)?;
        *active_bank_mut(&mut self.state) = bank;
        Ok(())
    }

    /// Checked Secure-to-Non-secure transition hook.
    pub fn branch_non_secure(&mut self, target: u32, link: bool) -> Result<(), CortexM33Error> {
        if !self.state.security_extension {
            return Err(CortexM33Error::SecurityExtensionUnavailable);
        }
        if self.state.security_state != SecurityState::Secure {
            return Err(CortexM33Error::InvalidSecurityTransition {
                reason: "only Secure code may initiate a Non-secure branch",
            });
        }
        // ALLNS classifies the address space when the SAU is disabled, but it
        // does not expand this simulator's owned memory.  Settle the minimum
        // Thumb fetch range before swapping banks or writing LR/PC so an
        // attributed but unowned target remains a typed, atomic fault.
        checked_owned_range(target, 2)?;
        if target & 1 != 0 || self.attribute(target) != SecurityAttribute::NonSecure {
            return Err(CortexM33Error::InvalidSecurityTransition {
                reason: "target must select attributed Non-secure Thumb code",
            });
        }
        save_active_bank(&mut self.state);
        if link {
            self.state.core.lr = self.state.core.pc | 1;
        }
        self.state.security_state = SecurityState::NonSecure;
        load_active_bank(&mut self.state);
        self.state.core.pc = target;
        Ok(())
    }

    /// Checked Non-secure-to-Secure gateway hook.
    pub fn enter_secure_gateway(&mut self, target: u32) -> Result<(), CortexM33Error> {
        if !self.state.security_extension {
            return Err(CortexM33Error::SecurityExtensionUnavailable);
        }
        if self.state.security_state != SecurityState::NonSecure {
            return Err(CortexM33Error::InvalidSecurityTransition {
                reason: "gateway entry requires Non-secure execution",
            });
        }
        if target & 1 != 0 || self.attribute(target) != SecurityAttribute::NonSecureCallable {
            return Err(CortexM33Error::InvalidSecurityTransition {
                reason: "gateway target must be an aligned NSC address",
            });
        }
        save_active_bank(&mut self.state);
        self.state.security_state = SecurityState::Secure;
        load_active_bank(&mut self.state);
        self.state.core.lr &= !1;
        self.state.core.pc = target;
        Ok(())
    }

    pub fn read_register(&self, index: u8) -> Result<u32, CortexM33Error> {
        let mut core = CortexM3Simulator::new();
        core.restore(self.state.core.clone())?;
        Ok(core.read_register(index)?)
    }

    pub fn write_register(&mut self, index: u8, value: u32) -> Result<(), CortexM33Error> {
        let mut core = CortexM3Simulator::new();
        core.restore(self.state.core.clone())?;
        core.write_register(index, value)?;
        let mut next = try_clone_state(&self.state)?;
        next.core = core.get_state();
        save_active_bank(&mut next);
        validate_active_stack_limit(&next)?;
        self.state = next;
        Ok(())
    }

    pub fn read_memory(&self, address: u32, width: usize) -> Result<u32, CortexM33Error> {
        self.authorize_data_range(address, width)?;
        let range = checked_range(address, width)?;
        let mut bytes = [0; 4];
        bytes[..width].copy_from_slice(&self.state.core.memory[range]);
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn write_memory(
        &mut self,
        address: u32,
        width: usize,
        value: u32,
    ) -> Result<(), CortexM33Error> {
        self.authorize_data_range(address, width)?;
        let range = checked_range(address, width)?;
        self.state.core.memory[range].copy_from_slice(&value.to_le_bytes()[..width]);
        self.state.core.exclusive_address = None;
        Ok(())
    }

    pub fn signal_event(&mut self) -> Result<(), CortexM33Error> {
        self.with_core(|core| {
            core.signal_event();
            Ok(())
        })
    }

    pub fn signal_interrupt(&mut self) -> Result<(), CortexM33Error> {
        self.with_core(|core| {
            core.signal_interrupt();
            Ok(())
        })
    }

    /// Execute one inherited instruction transactionally.
    pub fn step(&mut self) -> Result<StepTrace, CortexM33Error> {
        let before = try_clone_state(&self.state)?;
        let (first, second, width) = fetch_instruction(&before.core)?;
        self.authorize_fetch_range(before.core.pc, width)?;

        let mut core = CortexM3Simulator::new();
        core.restore(before.core.clone())?;
        let inherited = core.step()?;

        if inherited.mnemonic != "it-skip" {
            for (address, access_width) in memory_accesses(&before.core, first, second)? {
                self.authorize_data_range(address, access_width)?;
            }
        }

        let mut next = try_clone_state(&before)?;
        next.core = inherited.state_after;
        save_active_bank(&mut next);
        validate_active_stack_limit(&next)?;
        validate_bank_stack_limits(active_bank(&next))?;
        validate_bank(active_bank(&next))?;
        let after = try_clone_state(&next)?;
        let trace = StepTrace {
            pc_before: before.core.pc,
            pc_after: after.core.pc,
            raw: inherited.raw,
            width: inherited.width,
            mnemonic: inherited.mnemonic,
            security_before: before.security_state,
            security_after: after.security_state,
            state_before: before,
            state_after: after,
        };
        self.state = next;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, CortexM33Error> {
        self.run_with_final_clone(max_steps, try_clone_state)
    }

    fn run_with_final_clone(
        &mut self,
        max_steps: usize,
        mut final_clone: impl FnMut(&CortexM33State) -> Result<CortexM33State, CortexM33Error>,
    ) -> Result<ExecutionResult, CortexM33Error> {
        check_retention(max_steps)?;
        let before = try_clone_state(&self.state)?;
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| CortexM33Error::AllocationFailed)?;
        for _ in 0..max_steps {
            let trace = match self.step() {
                Ok(trace) => trace,
                Err(error) => {
                    self.state = before;
                    return Err(error);
                }
            };
            if self.state.core.halted || self.state.core.sleeping {
                let final_state = match final_clone(&trace.state_after) {
                    Ok(state) => state,
                    Err(error) => {
                        self.state = before;
                        return Err(error);
                    }
                };
                traces.push(trace);
                return Ok(ExecutionResult {
                    stopped: true,
                    steps: traces.len(),
                    traces,
                    final_state,
                });
            }
            traces.push(trace);
        }
        self.state = before;
        Err(CortexM33Error::StepLimitExceeded { limit: max_steps })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, CortexM33Error> {
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

    fn require_privileged_secure(&self) -> Result<(), CortexM33Error> {
        if !self.state.security_extension {
            return Err(CortexM33Error::SecurityExtensionUnavailable);
        }
        let privileged = self.state.core.handler_mode || self.state.core.control & 1 == 0;
        if self.state.security_state != SecurityState::Secure || !privileged {
            return Err(CortexM33Error::PrivilegeViolation);
        }
        Ok(())
    }

    fn authorize_data_range(&self, address: u32, width: usize) -> Result<(), CortexM33Error> {
        checked_owned_range(address, width)?;
        if self.state.security_state == SecurityState::Secure {
            return Ok(());
        }
        self.require_range_attribute(address, width, |attribute| {
            attribute == SecurityAttribute::NonSecure
        })
    }

    fn authorize_fetch_range(&self, address: u32, width: usize) -> Result<(), CortexM33Error> {
        checked_owned_range(address, width)?;
        self.require_range_attribute(address, width, |attribute| {
            match self.state.security_state {
                SecurityState::Secure => attribute != SecurityAttribute::NonSecure,
                SecurityState::NonSecure => attribute == SecurityAttribute::NonSecure,
            }
        })
    }

    fn require_range_attribute(
        &self,
        address: u32,
        width: usize,
        allowed: impl Fn(SecurityAttribute) -> bool,
    ) -> Result<(), CortexM33Error> {
        let end = address
            .checked_add(width as u32)
            .ok_or(CortexM33Error::Core(CortexM3Error::DataOutOfRange {
                address,
                width,
            }))?;
        for current in address..end {
            let attribute = self.attribute(current);
            if !allowed(attribute) {
                return Err(CortexM33Error::SecurityAccessViolation {
                    address,
                    width,
                    attribute,
                });
            }
        }
        Ok(())
    }

    fn with_core(
        &mut self,
        operation: impl FnOnce(&mut CortexM3Simulator) -> Result<(), CortexM33Error>,
    ) -> Result<(), CortexM33Error> {
        let mut core = CortexM3Simulator::new();
        core.restore(self.state.core.clone())?;
        operation(&mut core)?;
        self.state.core = core.get_state();
        save_active_bank(&mut self.state);
        Ok(())
    }
}

fn try_clone_state(state: &CortexM33State) -> Result<CortexM33State, CortexM33Error> {
    let mut memory = Vec::new();
    memory
        .try_reserve_exact(state.core.memory.len())
        .map_err(|_| CortexM33Error::AllocationFailed)?;
    memory.extend_from_slice(&state.core.memory);
    let mut cloned = state.clone();
    cloned.core.memory = memory;
    Ok(cloned)
}

fn validate_state(state: &CortexM33State) -> Result<(), CortexM33Error> {
    if !state.security_extension && state.security_state != SecurityState::NonSecure {
        return Err(CortexM33Error::Core(CortexM3Error::InvalidState {
            reason: "a core without the Security Extension must be Non-secure",
        }));
    }
    if !state.security_extension
        && (state.sau_enabled
            || state.sau_all_non_secure
            || state.sau_regions.iter().any(|region| region.enabled))
    {
        return Err(CortexM33Error::Core(CortexM3Error::InvalidState {
            reason: "SAU state exists without the Security Extension",
        }));
    }
    for region in state.sau_regions {
        validate_region(region)?;
    }
    for bank in [state.secure_bank, state.non_secure_bank] {
        validate_bank(bank)?;
    }
    let mut core = CortexM3Simulator::new();
    core.restore(state.core.clone())?;
    let mut expected = state.clone();
    load_active_bank(&mut expected);
    if expected.core.msp != state.core.msp
        || expected.core.psp != state.core.psp
        || expected.core.primask != state.core.primask
        || expected.core.basepri != state.core.basepri
        || expected.core.faultmask != state.core.faultmask
        || expected.core.control != state.core.control
    {
        return Err(CortexM33Error::Core(CortexM3Error::InvalidState {
            reason: "active security bank does not match core state",
        }));
    }
    validate_active_stack_limit(state)
}

fn validate_region(region: SauRegion) -> Result<(), CortexM33Error> {
    if !region.enabled {
        return Ok(());
    }
    if region.base & 0x1f != 0 {
        return Err(CortexM33Error::InvalidSauRegion {
            reason: "base is not 32-byte aligned",
        });
    }
    if region.limit & 0x1f != 0x1f {
        return Err(CortexM33Error::InvalidSauRegion {
            reason: "inclusive limit must end a 32-byte block",
        });
    }
    if region.base > region.limit || region.limit >= MEMORY_SIZE as u32 {
        return Err(CortexM33Error::InvalidSauRegion {
            reason: "range is empty or outside owned memory",
        });
    }
    Ok(())
}

fn validate_bank(bank: SecurityBank) -> Result<(), CortexM33Error> {
    for value in [bank.msp, bank.psp] {
        if value & 3 != 0 || value > MEMORY_SIZE as u32 {
            return Err(CortexM33Error::Core(CortexM3Error::InvalidState {
                reason: "banked stack pointer is invalid",
            }));
        }
    }
    for value in [bank.msplim, bank.psplim] {
        if value & 7 != 0 || value > MEMORY_SIZE as u32 {
            return Err(CortexM33Error::Core(CortexM3Error::InvalidState {
                reason: "banked stack limit is invalid",
            }));
        }
    }
    if bank.control & !3 != 0 {
        return Err(CortexM33Error::Core(CortexM3Error::InvalidState {
            reason: "banked CONTROL has reserved bits set",
        }));
    }
    if (bank.msplim != 0 && bank.msp < bank.msplim) || (bank.psplim != 0 && bank.psp < bank.psplim)
    {
        return Err(CortexM33Error::Core(CortexM3Error::InvalidState {
            reason: "banked stack pointer is below its limit",
        }));
    }
    Ok(())
}

fn validate_bank_stack_limits(bank: SecurityBank) -> Result<(), CortexM33Error> {
    for (stack_pointer, limit) in [(bank.msp, bank.msplim), (bank.psp, bank.psplim)] {
        if limit != 0 && stack_pointer < limit {
            return Err(CortexM33Error::StackLimitViolation {
                stack_pointer,
                limit,
            });
        }
    }
    Ok(())
}

fn attribute(state: &CortexM33State, address: u32) -> SecurityAttribute {
    if !state.security_extension {
        return SecurityAttribute::NonSecure;
    }
    if !state.sau_enabled {
        return if state.sau_all_non_secure {
            SecurityAttribute::NonSecure
        } else {
            SecurityAttribute::Secure
        };
    }
    let mut saw_non_secure = false;
    let mut saw_nsc = false;
    for region in state
        .sau_regions
        .iter()
        .filter(|region| region.enabled && address >= region.base && address <= region.limit)
    {
        if region.non_secure_callable {
            saw_nsc = true;
        } else {
            saw_non_secure = true;
        }
    }
    match (saw_non_secure, saw_nsc) {
        (false, false) => SecurityAttribute::Secure,
        (true, false) => SecurityAttribute::NonSecure,
        (false, true) => SecurityAttribute::NonSecureCallable,
        (true, true) => SecurityAttribute::Secure,
    }
}

fn active_bank(state: &CortexM33State) -> SecurityBank {
    match state.security_state {
        SecurityState::Secure => state.secure_bank,
        SecurityState::NonSecure => state.non_secure_bank,
    }
}

fn active_bank_mut(state: &mut CortexM33State) -> &mut SecurityBank {
    match state.security_state {
        SecurityState::Secure => &mut state.secure_bank,
        SecurityState::NonSecure => &mut state.non_secure_bank,
    }
}

fn save_active_bank(state: &mut CortexM33State) {
    let core = &state.core;
    let bank = SecurityBank {
        msp: core.msp,
        psp: core.psp,
        msplim: active_bank(state).msplim,
        psplim: active_bank(state).psplim,
        primask: core.primask,
        basepri: core.basepri,
        faultmask: core.faultmask,
        control: core.control,
    };
    *active_bank_mut(state) = bank;
}

fn load_active_bank(state: &mut CortexM33State) {
    let bank = active_bank(state);
    state.core.msp = bank.msp;
    state.core.psp = bank.psp;
    state.core.primask = bank.primask;
    state.core.basepri = bank.basepri;
    state.core.faultmask = bank.faultmask;
    state.core.control = bank.control;
}

fn active_stack(state: &CortexM33State) -> (u32, u32) {
    let bank = active_bank(state);
    if state.core.handler_mode || state.core.control & 2 == 0 {
        (state.core.msp, bank.msplim)
    } else {
        (state.core.psp, bank.psplim)
    }
}

fn validate_active_stack_limit(state: &CortexM33State) -> Result<(), CortexM33Error> {
    let (stack_pointer, limit) = active_stack(state);
    if limit != 0 && stack_pointer < limit {
        return Err(CortexM33Error::StackLimitViolation {
            stack_pointer,
            limit,
        });
    }
    Ok(())
}

fn checked_range(address: u32, width: usize) -> Result<std::ops::Range<usize>, CortexM33Error> {
    if !matches!(width, 1 | 2 | 4) {
        return Err(CortexM33Error::Core(CortexM3Error::DataOutOfRange {
            address,
            width,
        }));
    }
    checked_owned_range(address, width)
}

fn checked_owned_range(
    address: u32,
    width: usize,
) -> Result<std::ops::Range<usize>, CortexM33Error> {
    if width == 0 {
        return Err(CortexM33Error::Core(CortexM3Error::DataOutOfRange {
            address,
            width,
        }));
    }
    let start = usize::try_from(address)
        .map_err(|_| CortexM33Error::Core(CortexM3Error::DataOutOfRange { address, width }))?;
    let end = start
        .checked_add(width)
        .filter(|end| *end <= MEMORY_SIZE)
        .ok_or(CortexM33Error::Core(CortexM3Error::DataOutOfRange {
            address,
            width,
        }))?;
    Ok(start..end)
}

fn fetch_instruction(core: &CortexM3State) -> Result<(u16, Option<u16>, usize), CortexM33Error> {
    let pc = core.pc;
    if pc & 1 != 0 {
        return Err(CortexM3Error::MisalignedFetch { pc }.into());
    }
    let installed_end = core.loaded_origin as usize + core.loaded_len;
    let start = pc as usize;
    if start < core.loaded_origin as usize
        || start.checked_add(2).is_none_or(|end| end > installed_end)
    {
        return Err(CortexM3Error::FetchOutsideProgram { pc, width: 2 }.into());
    }
    let first = u16::from_le_bytes([core.memory[start], core.memory[start + 1]]);
    let width = if matches!(first >> 11, 0b11101..=0b11111) {
        4
    } else {
        2
    };
    let second = if width == 4 {
        if start.checked_add(4).is_none_or(|end| end > installed_end) {
            return Err(CortexM3Error::FetchOutsideProgram { pc, width: 4 }.into());
        }
        Some(u16::from_le_bytes([
            core.memory[start + 2],
            core.memory[start + 3],
        ]))
    } else {
        None
    };
    Ok((first, second, width))
}

fn memory_accesses(
    state: &CortexM3State,
    first: u16,
    second: Option<u16>,
) -> Result<Vec<(u32, usize)>, CortexM33Error> {
    let class = classify(first, second)?;
    let mut accesses = Vec::new();
    match class {
        InstructionClass::LiteralLoad => {
            accesses.push((
                (state.pc.wrapping_add(4) & !3).wrapping_add(u32::from(first & 0xff) * 4),
                4,
            ));
        }
        InstructionClass::RegisterMemory => {
            let operation = (first >> 9) & 7;
            let rm = ((first >> 6) & 7) as usize;
            let rn = ((first >> 3) & 7) as usize;
            let width = match operation {
                0 | 4 => 4,
                1 | 5 | 7 => 2,
                _ => 1,
            };
            accesses.push((state.registers[rn].wrapping_add(state.registers[rm]), width));
        }
        InstructionClass::ImmediateMemory => {
            let operation = first >> 11;
            let immediate = u32::from((first >> 6) & 0x1f);
            let rn = ((first >> 3) & 7) as usize;
            let (scale, width) = match operation {
                0b01100 | 0b01101 => (4, 4),
                0b01110 | 0b01111 => (1, 1),
                _ => (2, 2),
            };
            accesses.push((state.registers[rn].wrapping_add(immediate * scale), width));
        }
        InstructionClass::StackMemory => {
            accesses.push((
                core_active_sp(state).wrapping_add(u32::from(first & 0xff) * 4),
                4,
            ));
        }
        InstructionClass::Miscellaneous if first & 0xfe00 == 0xb400 => {
            let count = (first & 0xff).count_ones() + u32::from(first & 0x100 != 0);
            accesses.push((
                core_active_sp(state).wrapping_sub(count * 4),
                count as usize * 4,
            ));
        }
        InstructionClass::Miscellaneous if first & 0xfe00 == 0xbc00 => {
            let count = (first & 0xff).count_ones() + u32::from(first & 0x100 != 0);
            accesses.push((core_active_sp(state), count as usize * 4));
        }
        InstructionClass::MultipleMemory => {
            let rn = ((first >> 8) & 7) as usize;
            accesses.push((
                state.registers[rn],
                (first & 0xff).count_ones() as usize * 4,
            ));
        }
        InstructionClass::Thumb2Memory => {
            let second = second.expect("classified 32-bit instruction");
            let rn = (first & 0xf) as u8;
            let address = core_register(state, rn)?.wrapping_add(u32::from(second & 0xfff));
            let width = match first & 0xfff0 {
                0xf880 | 0xf890 | 0xf990 => 1,
                0xf8a0 | 0xf8b0 | 0xf9b0 => 2,
                _ => 4,
            };
            accesses.push((address, width));
        }
        InstructionClass::Exclusive if first != 0xf3bf => {
            let second = second.expect("classified 32-bit instruction");
            let rn = (first & 0xf) as u8;
            accesses.push((
                core_register(state, rn)?.wrapping_add(u32::from(second as u8) * 4),
                4,
            ));
        }
        InstructionClass::TableBranch => {
            let second = second.expect("classified 32-bit instruction");
            let rn = (first & 0xf) as u8;
            let rm = (second & 0xf) as u8;
            let base = if rn == PC {
                state.pc.wrapping_add(4)
            } else {
                core_register(state, rn)?
            };
            let halfword = second & 0x10 != 0;
            let index = core_register(state, rm)?;
            accesses.push((
                base.wrapping_add(if halfword {
                    index.wrapping_mul(2)
                } else {
                    index
                }),
                if halfword { 2 } else { 1 },
            ));
        }
        _ => {}
    }
    Ok(accesses)
}

fn core_active_sp(state: &CortexM3State) -> u32 {
    if state.handler_mode || state.control & 2 == 0 {
        state.msp
    } else {
        state.psp
    }
}

fn core_register(state: &CortexM3State, index: u8) -> Result<u32, CortexM33Error> {
    Ok(match index {
        0..=12 => state.registers[index as usize],
        SP => core_active_sp(state),
        14 => state.lr,
        PC => state.pc,
        _ => return Err(CortexM3Error::InvalidRegister { index }.into()),
    })
}

fn check_retention(requested: usize) -> Result<(), CortexM33Error> {
    if requested > MAX_RETAINED_STEPS {
        Err(CortexM33Error::RetentionLimitExceeded {
            requested,
            limit: MAX_RETAINED_STEPS,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core() -> CortexM3State {
        let mut state = CortexM3State {
            pc: 0x100,
            msp: 0x800,
            ..CortexM3State::default()
        };
        state.registers[0] = 0x20;
        state.registers[1] = 0x400;
        state.registers[2] = 4;
        state
    }

    #[test]
    fn access_classifier_covers_every_inherited_memory_family() {
        let state = core();
        assert_eq!(
            memory_accesses(&state, 0x4801, None).unwrap(),
            vec![(0x108, 4)]
        );
        assert_eq!(
            memory_accesses(&state, 0x5088, None).unwrap(),
            vec![(0x404, 4)]
        );
        assert_eq!(
            memory_accesses(&state, 0x7048, None).unwrap(),
            vec![(0x401, 1)]
        );
        assert_eq!(
            memory_accesses(&state, 0x9801, None).unwrap(),
            vec![(0x804, 4)]
        );
        assert_eq!(
            memory_accesses(&state, 0xb403, None).unwrap(),
            vec![(0x7f8, 8)]
        );
        assert_eq!(
            memory_accesses(&state, 0xbc03, None).unwrap(),
            vec![(0x800, 8)]
        );
        assert_eq!(
            memory_accesses(&state, 0xc103, None).unwrap(),
            vec![(0x400, 8)]
        );
        assert_eq!(
            memory_accesses(&state, 0xf8d1, Some(0x0004)).unwrap(),
            vec![(0x404, 4)]
        );
        assert_eq!(
            memory_accesses(&state, 0xe851, Some(0x0f00)).unwrap(),
            vec![(0x400, 4)]
        );
        assert_eq!(
            memory_accesses(&state, 0xe8d1, Some(0xf002)).unwrap(),
            vec![(0x404, 1)]
        );
        assert!(memory_accesses(&state, 0x2001, None).unwrap().is_empty());
    }

    #[test]
    fn allocation_clone_and_range_helpers_preserve_complete_state() {
        let state = CortexM33State::reset(true);
        assert_eq!(try_clone_state(&state).unwrap(), state);
        assert_eq!(checked_range(4, 4).unwrap(), 4..8);
        assert!(checked_range(4, 3).is_err());
        assert!(checked_owned_range(4, 0).is_err());
        assert!(checked_owned_range(u32::MAX, 8).is_err());
        assert_eq!(core_register(&state.core, 14).unwrap(), 0);
        assert!(core_register(&state.core, 16).is_err());
    }

    #[test]
    fn final_state_allocation_failure_rolls_back_the_run() {
        let mut simulator = CortexM33Simulator::new(false);
        simulator.load(&0xbe00_u16.to_le_bytes(), 0).unwrap();
        let before = simulator.get_state().unwrap();
        assert_eq!(
            simulator.run_with_final_clone(1, |_| Err(CortexM33Error::AllocationFailed)),
            Err(CortexM33Error::AllocationFailed)
        );
        assert_eq!(simulator.get_state().unwrap(), before);
    }
}
