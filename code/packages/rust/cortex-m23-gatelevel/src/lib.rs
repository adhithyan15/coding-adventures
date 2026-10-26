//! DFF-backed Armv8-M Baseline Cortex-M23 architectural gate simulator.
//!
//! A local transition engine reconstructs decode, arithmetic, address, flag,
//! and control_non_secure paths with repository gates. Only settled candidate values are
//! clocked into persistent repository D flip-flops.

mod engine;
pub mod gates;
mod state;

pub use engine::{
    classify, encoding, CoreError, CortexM23State, ExecutionResult, InstructionClass, SauRegion,
    SecurityAttribute, SecurityExtension, SecurityState, StepTrace, LR, MAX_RETAINED_BYTES,
    MAX_RETAINED_STEPS, MEMORY_SIZE, PC, SAU_REGION_COUNT, SP,
};

use state::{clock_bit, clock_bits, read_bits, DffMemory};
use std::fmt;
use std::sync::atomic::{AtomicIsize, Ordering};

// The counter is dormant in production. Unit tests arm it to make every
// fallible allocation boundary deterministic without adding a compatibility
// mirror to the architectural storage inventory.
static ALLOCATION_FAILURE_AFTER: AtomicIsize = AtomicIsize::new(-1);

pub(crate) fn allocation_checkpoint() -> Result<(), ()> {
    let remaining = ALLOCATION_FAILURE_AFTER.load(Ordering::Relaxed);
    if remaining < 0 {
        return Ok(());
    }
    if remaining == 0 {
        ALLOCATION_FAILURE_AFTER.store(-1, Ordering::Relaxed);
        return Err(());
    }
    ALLOCATION_FAILURE_AFTER.store(remaining - 1, Ordering::Relaxed);
    Ok(())
}

/// Typed lifecycle and bounded-profile failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CortexM23Error {
    Core(CoreError),
    SecurityExtensionUnavailable,
    RetentionLimitExceeded { requested: usize, limit: usize },
    StepLimitExceeded { limit: usize },
    AllocationFailed,
}

impl From<CoreError> for CortexM23Error {
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::AllocationFailed => Self::AllocationFailed,
            CoreError::RetentionLimitExceeded { requested, limit } => {
                Self::RetentionLimitExceeded { requested, limit }
            }
            CoreError::StepLimitExceeded { limit } => Self::StepLimitExceeded { limit },
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

/// Exact number of mutable persistent architectural D flip-flops.
pub const FLIP_FLOP_COUNT: usize =
    MEMORY_SIZE * 8 + 13 * 32 + 4 * 32 + 4 + 6 + 1 + 2 + 1 + 3 + 32 + 17;
/// Six primitive gates per master/slave DFF plus a finite combinational budget.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 98_304;

/// Complete Cortex-M23 state held in stable-Q simulated storage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CortexM23GateLevel {
    memory: DffMemory,
    registers: [[u8; 32]; 13],
    msp_non_secure: [u8; 32],
    psp_non_secure: [u8; 32],
    lr: [u8; 32],
    pc: [u8; 32],
    apsr: [u8; 4],
    ipsr: [u8; 6],
    primask_non_secure: u8,
    control_non_secure: [u8; 2],
    handler_mode: u8,
    halted: u8,
    sleeping: u8,
    event_registered: u8,
    loaded_origin: [u8; 32],
    loaded_len: [u8; 17],
}

impl Default for CortexM23GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl CortexM23GateLevel {
    /// Construct deterministic privileged Thread-mode reset state.
    #[must_use]
    pub fn new() -> Self {
        let mut cpu = Self {
            memory: DffMemory::new(MEMORY_SIZE),
            registers: [[0; 32]; 13],
            msp_non_secure: [0; 32],
            psp_non_secure: [0; 32],
            lr: [0; 32],
            pc: [0; 32],
            apsr: [0; 4],
            ipsr: [0; 6],
            primask_non_secure: 0,
            control_non_secure: [0; 2],
            handler_mode: 0,
            halted: 0,
            sleeping: 0,
            event_registered: 0,
            loaded_origin: [0; 32],
            loaded_len: [0; 17],
        };
        clock_bits(&mut cpu.msp_non_secure, MEMORY_SIZE as u64);
        clock_bits(&mut cpu.psp_non_secure, MEMORY_SIZE as u64);
        cpu
    }

    /// Clock every architectural bit to reset state.
    pub fn reset(&mut self) {
        self.memory.clear();
        for register in &mut self.registers {
            clock_bits(register, 0);
        }
        clock_bits(&mut self.msp_non_secure, MEMORY_SIZE as u64);
        clock_bits(&mut self.psp_non_secure, MEMORY_SIZE as u64);
        clock_bits(&mut self.lr, 0);
        clock_bits(&mut self.pc, 0);
        clock_bits(&mut self.apsr, 0);
        clock_bits(&mut self.ipsr, 0);
        clock_bit(&mut self.primask_non_secure, false);
        clock_bits(&mut self.control_non_secure, 0);
        clock_bit(&mut self.handler_mode, false);
        clock_bit(&mut self.halted, false);
        clock_bit(&mut self.sleeping, false);
        clock_bit(&mut self.event_registered, false);
        clock_bits(&mut self.loaded_origin, 0);
        clock_bits(&mut self.loaded_len, 0);
    }

    /// Snapshot every architectural bit into owned value storage.
    #[must_use]
    pub fn get_state(&self) -> CortexM23State {
        self.state_with_memory(self.memory.snapshot())
    }

    /// Fallibly snapshot every architectural bit into owned value storage.
    ///
    /// Checked execution uses this path so allocation exhaustion is typed and
    /// cannot expose a partially constructed snapshot.
    pub fn try_get_state(&self) -> Result<CortexM23State, CortexM23Error> {
        allocation_checkpoint().map_err(|()| CortexM23Error::AllocationFailed)?;
        let memory = self
            .memory
            .try_snapshot()
            .map_err(|()| CortexM23Error::AllocationFailed)?;
        Ok(self.state_with_memory(memory))
    }

    fn state_with_memory(&self, memory: Vec<u8>) -> CortexM23State {
        let apsr = (read_bits(&self.apsr) as u32) << 28;
        CortexM23State {
            registers: std::array::from_fn(|index| read_bits(&self.registers[index]) as u32),
            msp_secure: MEMORY_SIZE as u32,
            psp_secure: MEMORY_SIZE as u32,
            msp_non_secure: read_bits(&self.msp_non_secure) as u32,
            psp_non_secure: read_bits(&self.psp_non_secure) as u32,
            lr: read_bits(&self.lr) as u32,
            pc: read_bits(&self.pc) as u32,
            apsr,
            ipsr: read_bits(&self.ipsr) as u8,
            primask_secure: false,
            primask_non_secure: self.primask_non_secure == 1,
            control_secure: 0,
            control_non_secure: read_bits(&self.control_non_secure) as u8,
            handler_mode: self.handler_mode == 1,
            security_extension: SecurityExtension::Disabled,
            security_state: SecurityState::NonSecure,
            sau_enabled: false,
            sau_all_non_secure: true,
            sau_regions: [SauRegion::default(); SAU_REGION_COUNT],
            memory,
            halted: self.halted == 1,
            sleeping: self.sleeping == 1,
            event_registered: self.event_registered == 1,
            loaded_origin: read_bits(&self.loaded_origin) as u32,
            loaded_len: read_bits(&self.loaded_len) as usize,
        }
    }

    /// Validate and atomically clock a complete snapshot.
    pub fn restore(&mut self, snapshot: CortexM23State) -> Result<(), CortexM23Error> {
        engine::validate_state(&snapshot)?;
        self.restore_validated(&snapshot);
        Ok(())
    }

    fn restore_validated(&mut self, snapshot: &CortexM23State) {
        self.memory.restore_snapshot(&snapshot.memory);
        for (target, value) in self.registers.iter_mut().zip(snapshot.registers) {
            clock_bits(target, u64::from(value));
        }
        clock_bits(&mut self.msp_non_secure, u64::from(snapshot.msp_non_secure));
        clock_bits(&mut self.psp_non_secure, u64::from(snapshot.psp_non_secure));
        clock_bits(&mut self.lr, u64::from(snapshot.lr));
        clock_bits(&mut self.pc, u64::from(snapshot.pc));
        clock_bits(&mut self.apsr, u64::from(snapshot.apsr >> 28));
        clock_bits(&mut self.ipsr, u64::from(snapshot.ipsr));
        clock_bit(&mut self.primask_non_secure, snapshot.primask_non_secure);
        clock_bits(
            &mut self.control_non_secure,
            u64::from(snapshot.control_non_secure),
        );
        clock_bit(&mut self.handler_mode, snapshot.handler_mode);
        clock_bit(&mut self.halted, snapshot.halted);
        clock_bit(&mut self.sleeping, snapshot.sleeping);
        clock_bit(&mut self.event_registered, snapshot.event_registered);
        clock_bits(&mut self.loaded_origin, u64::from(snapshot.loaded_origin));
        clock_bits(&mut self.loaded_len, snapshot.loaded_len as u64);
    }

    fn commit_transition(&mut self, next: &CortexM23State) {
        for (address, &value) in next.memory.iter().enumerate() {
            let current = self.memory.read(address);
            if current != value && gates::byte_changed(current, value) == 1 {
                self.memory.write(address, value);
            }
        }
        self.restore_registers_and_control(next);
    }

    fn restore_registers_and_control(&mut self, next: &CortexM23State) {
        for (target, value) in self.registers.iter_mut().zip(next.registers) {
            clock_bits(target, u64::from(value));
        }
        clock_bits(&mut self.msp_non_secure, u64::from(next.msp_non_secure));
        clock_bits(&mut self.psp_non_secure, u64::from(next.psp_non_secure));
        clock_bits(&mut self.lr, u64::from(next.lr));
        clock_bits(&mut self.pc, u64::from(next.pc));
        clock_bits(&mut self.apsr, u64::from(next.apsr >> 28));
        clock_bits(&mut self.ipsr, u64::from(next.ipsr));
        clock_bit(&mut self.primask_non_secure, next.primask_non_secure);
        clock_bits(
            &mut self.control_non_secure,
            u64::from(next.control_non_secure),
        );
        clock_bit(&mut self.handler_mode, next.handler_mode);
        clock_bit(&mut self.halted, next.halted);
        clock_bit(&mut self.sleeping, next.sleeping);
        clock_bit(&mut self.event_registered, next.event_registered);
        clock_bits(&mut self.loaded_origin, u64::from(next.loaded_origin));
        clock_bits(&mut self.loaded_len, next.loaded_len as u64);
    }

    /// Install a checked nonempty halfword-aligned little-endian image.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), CortexM23Error> {
        engine::validate_program(program, origin)?;
        self.reset();
        for (offset, value) in program.iter().copied().enumerate() {
            self.memory.write(origin as usize + offset, value);
        }
        clock_bits(&mut self.pc, u64::from(origin));
        clock_bits(&mut self.loaded_origin, u64::from(origin));
        clock_bits(&mut self.loaded_len, program.len() as u64);
        Ok(())
    }

    pub fn read_register(&self, index: u8) -> Result<u32, CortexM23Error> {
        match index {
            0..=12 => Ok(read_bits(&self.registers[index as usize]) as u32),
            SP => {
                let select = logic_gates::gates::and_gate(
                    logic_gates::gates::not_gate(self.handler_mode),
                    self.control_non_secure[1],
                );
                Ok(gates::select_32(
                    read_bits(&self.msp_non_secure) as u32,
                    read_bits(&self.psp_non_secure) as u32,
                    select,
                ))
            }
            LR => Ok(read_bits(&self.lr) as u32),
            PC => Ok(read_bits(&self.pc) as u32),
            _ => Err(CoreError::InvalidRegister { index }.into()),
        }
    }

    pub fn write_register(&mut self, index: u8, value: u32) -> Result<(), CortexM23Error> {
        match index {
            0..=12 => clock_bits(&mut self.registers[index as usize], u64::from(value)),
            SP => {
                let value = gates::bit_and_32(value, gates::bit_not_32(3));
                let select = logic_gates::gates::and_gate(
                    logic_gates::gates::not_gate(self.handler_mode),
                    self.control_non_secure[1],
                );
                if select == 1 {
                    clock_bits(&mut self.psp_non_secure, u64::from(value));
                } else {
                    clock_bits(&mut self.msp_non_secure, u64::from(value));
                }
            }
            LR => clock_bits(&mut self.lr, u64::from(value)),
            PC => clock_bits(
                &mut self.pc,
                u64::from(gates::bit_and_32(value, gates::bit_not_32(1))),
            ),
            _ => return Err(CoreError::InvalidRegister { index }.into()),
        }
        Ok(())
    }

    pub fn read_memory(&self, address: u32, width: usize) -> Result<u32, CortexM23Error> {
        let range = engine::checked_range(address, width)?;
        let mut bytes = [0; 4];
        for (target, index) in bytes.iter_mut().zip(range) {
            *target = self.memory.read(index);
        }
        Ok(gates::assemble_le_u32(bytes))
    }

    pub fn write_memory(
        &mut self,
        address: u32,
        width: usize,
        value: u32,
    ) -> Result<(), CortexM23Error> {
        let range = engine::checked_range(address, width)?;
        for (index, byte) in range.zip(gates::split_le_u32(value)) {
            self.memory.write(index, byte);
        }
        Ok(())
    }

    /// Every address is Non-secure when the optional Security Extension is absent.
    #[must_use]
    pub const fn attribute(&self, _address: u32) -> SecurityAttribute {
        SecurityAttribute::NonSecure
    }

    /// SAU configuration is unavailable in this fixed processor profile.
    pub fn configure_sau(
        &mut self,
        _enabled: bool,
        _all_non_secure: bool,
    ) -> Result<(), CortexM23Error> {
        Err(CortexM23Error::SecurityExtensionUnavailable)
    }

    /// SAU regions are unavailable in this fixed processor profile.
    pub fn configure_sau_region(
        &mut self,
        _index: usize,
        _region: SauRegion,
    ) -> Result<(), CortexM23Error> {
        Err(CortexM23Error::SecurityExtensionUnavailable)
    }

    /// Secure-to-Non-secure transition hooks require the omitted extension.
    pub fn enter_non_secure(&mut self, _target: u32) -> Result<(), CortexM23Error> {
        Err(CortexM23Error::SecurityExtensionUnavailable)
    }

    /// Non-secure-callable transition hooks require the omitted extension.
    pub fn enter_secure_gateway(&mut self, _target: u32) -> Result<(), CortexM23Error> {
        Err(CortexM23Error::SecurityExtensionUnavailable)
    }

    /// Register an event and wake a sleeping core through DFF inputs.
    pub fn signal_event(&mut self) {
        clock_bit(&mut self.event_registered, true);
        clock_bit(&mut self.sleeping, false);
    }

    /// Execute one independent gate-derived transition.
    pub fn step(&mut self) -> Result<StepTrace, CortexM23Error> {
        let trace = engine::step_state(self.try_get_state()?)?;
        self.commit_transition(&trace.state_after);
        Ok(trace)
    }

    /// Execute until halt/sleep or the supplied bound, rolling back on failure.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, CortexM23Error> {
        check_retention(max_steps)?;
        let before = self.try_get_state()?;
        let mut traces = Vec::new();
        allocation_checkpoint().map_err(|()| CortexM23Error::AllocationFailed)?;
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| CortexM23Error::AllocationFailed)?;
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.restore_validated(&before);
                    return Err(error);
                }
            }
            if self.halted == 1 || self.sleeping == 1 {
                allocation_checkpoint().map_err(|()| {
                    self.restore_validated(&before);
                    CortexM23Error::AllocationFailed
                })?;
                return Ok(ExecutionResult {
                    stopped: true,
                    steps: traces.len(),
                    traces,
                    final_state: self.try_get_state().inspect_err(|_| {
                        self.restore_validated(&before);
                    })?,
                });
            }
        }
        self.restore_validated(&before);
        Err(CortexM23Error::StepLimitExceeded { limit: max_steps })
    }

    /// Transactionally load and execute one image.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, CortexM23Error> {
        check_retention(max_steps)?;
        let before = self.try_get_state()?;
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.restore_validated(&before);
                Err(error)
            }
        }
    }

    #[must_use]
    pub const fn flip_flop_count(&self) -> usize {
        FLIP_FLOP_COUNT
    }

    #[must_use]
    pub const fn gate_count(&self) -> usize {
        ESTIMATED_GATE_COUNT
    }

    #[cfg(test)]
    fn fail_allocation_after(&self, successful_checkpoints: usize) {
        ALLOCATION_FAILURE_AFTER.store(successful_checkpoints as isize, Ordering::Relaxed);
    }
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
mod allocation_regressions {
    use super::{encoding, CortexM23Error, CortexM23GateLevel};
    use std::sync::Mutex;

    static ALLOCATION_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn program(words: &[u16]) -> Vec<u8> {
        words.iter().flat_map(|word| word.to_le_bytes()).collect()
    }

    fn loaded() -> CortexM23GateLevel {
        let mut cpu = CortexM23GateLevel::new();
        cpu.load(&program(&[encoding::bkpt(0)]), 0x100).unwrap();
        cpu
    }

    #[test]
    fn snapshot_and_candidate_allocation_failures_are_atomic() {
        let _guard = ALLOCATION_TEST_LOCK.lock().unwrap();
        for checkpoint in 0..=1 {
            let mut cpu = loaded();
            let before = cpu.get_state();
            cpu.fail_allocation_after(checkpoint);
            assert_eq!(cpu.step(), Err(CortexM23Error::AllocationFailed));
            assert_eq!(cpu.get_state(), before);
        }
    }

    #[test]
    fn run_allocation_boundaries_restore_stable_q() {
        let _guard = ALLOCATION_TEST_LOCK.lock().unwrap();
        // before-run snapshot, trace reserve, step snapshot, candidate clone,
        // result construction, and final-state snapshot.
        for checkpoint in 0..=5 {
            let mut cpu = loaded();
            let before = cpu.get_state();
            cpu.fail_allocation_after(checkpoint);
            assert_eq!(cpu.run(1), Err(CortexM23Error::AllocationFailed));
            assert_eq!(cpu.get_state(), before);
        }
    }

    #[test]
    fn execute_allocation_boundaries_restore_the_preexisting_machine() {
        let _guard = ALLOCATION_TEST_LOCK.lock().unwrap();
        let replacement = program(&[encoding::bkpt(0)]);
        // execute snapshot, run snapshot, trace reserve, step snapshot,
        // candidate clone, result construction, and final-state snapshot.
        for checkpoint in 0..=6 {
            let mut cpu = loaded();
            cpu.write_register(0, 0xfeed_beef).unwrap();
            let before = cpu.get_state();
            cpu.fail_allocation_after(checkpoint);
            assert_eq!(
                cpu.execute(&replacement, 0x200, 1),
                Err(CortexM23Error::AllocationFailed)
            );
            assert_eq!(cpu.get_state(), before);
        }
    }
}
