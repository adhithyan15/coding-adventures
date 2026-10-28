//! Narrow gate-to-gate composition boundary.
//!
//! This module deliberately exposes settled value candidates rather than the
//! mutable DFF containers.  A composing gate model can therefore preallocate
//! every 64 KiB snapshot, derive a transition, and commit it only after its own
//! companion state has also settled.

use core::cell::Cell;

use crate::{engine, CortexM3Error, CortexM3GateLevel, CortexM3State, StepTrace, MEMORY_SIZE};

/// Deterministic controller shared by the production fallible allocator and
/// allocation-failure tests.
#[derive(Debug, Default)]
pub struct AllocationController {
    fail_after: Cell<Option<usize>>,
}

impl AllocationController {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fail_after: Cell::new(None),
        }
    }

    /// Fail the allocation checkpoint after `successful` further checkpoints.
    pub fn fail_after(&self, successful: usize) {
        self.fail_after.set(Some(successful));
    }

    /// Disable deterministic failure injection.
    pub fn clear_failure(&self) {
        self.fail_after.set(None);
    }

    fn checkpoint(&self) -> Result<(), CortexM3Error> {
        if let Some(remaining) = self.fail_after.get() {
            if remaining == 0 {
                self.fail_after.set(None);
                return Err(CortexM3Error::AllocationFailed);
            }
            self.fail_after.set(Some(remaining - 1));
        }
        Ok(())
    }

    /// Allocate one exactly sized snapshot buffer.
    pub fn memory_buffer(&self) -> Result<Vec<u8>, CortexM3Error> {
        self.checkpoint()?;
        let mut memory = Vec::new();
        memory
            .try_reserve_exact(MEMORY_SIZE)
            .map_err(|_| CortexM3Error::AllocationFailed)?;
        memory.resize(MEMORY_SIZE, 0);
        Ok(memory)
    }

    /// Reserve an outer trace vector before any transition commits.
    pub fn trace_buffer<T>(&self, len: usize) -> Result<Vec<T>, CortexM3Error> {
        self.checkpoint()?;
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(len)
            .map_err(|_| CortexM3Error::AllocationFailed)?;
        Ok(traces)
    }
}

/// Counts derived directly from the production DFF container widths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageInventory {
    pub memory: usize,
    pub general_registers: usize,
    pub stack_link_pc: usize,
    pub status: usize,
    pub power_event: usize,
    pub exclusive_monitor: usize,
    pub installed_image: usize,
}

impl StorageInventory {
    #[must_use]
    pub const fn total(self) -> usize {
        self.memory
            + self.general_registers
            + self.stack_link_pc
            + self.status
            + self.power_event
            + self.exclusive_monitor
            + self.installed_image
    }
}

/// Derive physical storage from the live production containers.
#[must_use]
pub fn storage_inventory(core: &CortexM3GateLevel) -> StorageInventory {
    let status_bits = [&core.primask, &core.faultmask, &core.handler_mode];
    let power_event_bits = [
        &core.halted,
        &core.sleeping,
        &core.waiting_for_event,
        &core.event_registered,
    ];
    let exclusive_valid_bits = [&core.exclusive_valid];
    StorageInventory {
        memory: core.memory.bit_len(),
        general_registers: core.registers.iter().map(|bank| bank.len()).sum(),
        stack_link_pc: core.msp.len() + core.psp.len() + core.lr.len() + core.pc.len(),
        status: core.apsr.len()
            + core.ipsr.len()
            + core.itstate.len()
            + core.basepri.len()
            + core.control.len()
            + status_bits.len(),
        power_event: power_event_bits.len(),
        exclusive_monitor: core.exclusive_address.len() + exclusive_valid_bits.len(),
        installed_image: core.loaded_origin.len() + core.loaded_len.len(),
    }
}

/// Settled state awaiting an explicit DFF commit.
///
/// `state_mut` remains available for repository gate companions that append
/// their own mux results after the initial settle pass.  Consequently the
/// final commit boundary revalidates the complete value before clocking Q.
#[derive(Debug)]
pub struct Candidate(CortexM3State);

impl Candidate {
    #[must_use]
    pub fn state(&self) -> &CortexM3State {
        &self.0
    }

    /// Candidate values are not authoritative storage.  Composed gate engines
    /// may settle their additional mux results here before final validation and
    /// commit.
    pub fn state_mut(&mut self) -> &mut CortexM3State {
        &mut self.0
    }

    #[must_use]
    pub fn into_state(self) -> CortexM3State {
        self.0
    }
}

/// Snapshot Q into caller-preallocated storage without an intermediate clone.
pub fn snapshot_into(
    core: &CortexM3GateLevel,
    memory: Vec<u8>,
    controller: &AllocationController,
) -> Result<CortexM3State, CortexM3Error> {
    controller.checkpoint()?;
    if memory.capacity() < MEMORY_SIZE {
        return Err(CortexM3Error::AllocationFailed);
    }
    Ok(core.state_in_preallocated(memory))
}

/// Clone a value snapshot into a second caller-preallocated memory buffer.
pub fn clone_into(
    state: &CortexM3State,
    memory: Vec<u8>,
    controller: &AllocationController,
) -> Result<CortexM3State, CortexM3Error> {
    controller.checkpoint()?;
    if memory.capacity() < MEMORY_SIZE {
        return Err(CortexM3Error::AllocationFailed);
    }
    Ok(engine::clone_state_into(state, memory))
}

/// Validate a complete candidate without touching a DFF.
pub fn settle(state: CortexM3State) -> Result<Candidate, CortexM3Error> {
    engine::validate_state(&state)?;
    Ok(Candidate(state))
}

/// Derive one inherited transition and exact trace without committing it.
pub fn delegated_step(
    core: &CortexM3GateLevel,
    state_memory: Vec<u8>,
    before_memory: Vec<u8>,
    validation_memory: &mut Vec<u8>,
    controller: &AllocationController,
) -> Result<StepTrace, CortexM3Error> {
    controller.checkpoint()?;
    if state_memory.capacity() < MEMORY_SIZE {
        return Err(CortexM3Error::AllocationFailed);
    }
    let state = core.state_in_preallocated(state_memory);
    delegated_step_from(state, before_memory, validation_memory, controller)
}

/// Derive a transition from an already preallocated Q snapshot.
pub fn delegated_step_from(
    state: CortexM3State,
    before_memory: Vec<u8>,
    validation_memory: &mut Vec<u8>,
    controller: &AllocationController,
) -> Result<StepTrace, CortexM3Error> {
    controller.checkpoint()?;
    if before_memory.capacity() < MEMORY_SIZE || validation_memory.capacity() < MEMORY_SIZE {
        return Err(CortexM3Error::AllocationFailed);
    }
    engine::step_state_preallocated(state, before_memory, validation_memory)
}

/// Revalidate and atomically clock a candidate into authoritative DFF storage.
///
/// Validation deliberately occurs here even though [`settle`] already checked
/// the original value.  Public candidate mutation is a compatibility seam for
/// composed gate engines, and no stale validation may authorize DFF clocks.
pub fn commit(core: &mut CortexM3GateLevel, candidate: &Candidate) -> Result<(), CortexM3Error> {
    engine::validate_state(&candidate.0)?;
    core.commit_transition(&candidate.0);
    Ok(())
}
