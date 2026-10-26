//! DFF-backed educational HP PA-RISC 1.1 gate-level simulator.
//!
//! The bounded ISA surface is defined by Spec 07au2. Persistent architectural
//! state is held only in stable Q outputs; the cached public state is an
//! observer refreshed after successful commits.

mod decoder;
mod engine;
mod gates;
mod state;

use std::cell::Cell;

pub use pa_risc11_simulator::{
    PaRisc11Error, PaRisc11ExecutionResult, PaRisc11State, PaRisc11StepTrace, MAX_RETAINED_STEPS,
    MEMORY_SIZE,
};
pub use state::PhysicalStorageInventory;

use state::PhysicalState;

/// Exact persistent stable-Q inventory from Spec 07au2.
pub const FLIP_FLOP_COUNT: usize = 525_436;

/// Named deterministic allocation boundary used by lifecycle verification.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AllocationPoint {
    Snapshot,
    Restore,
    LoadStorage,
    Candidate,
    StateAfter,
    TraceRetention,
    FinalState,
}

/// Checked gate-level PA-RISC simulator.
#[derive(Debug)]
pub struct PaRisc11GateSimulator {
    physical: PhysicalState,
    observer: PaRisc11State,
    failure: Cell<Option<AllocationPoint>>,
}

impl Default for PaRisc11GateSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl PaRisc11GateSimulator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            physical: PhysicalState::new(),
            observer: PaRisc11State::default(),
            failure: Cell::new(None),
        }
    }

    #[must_use]
    pub fn state(&self) -> &PaRisc11State {
        &self.observer
    }

    #[must_use]
    pub fn physical_storage_inventory(&self) -> PhysicalStorageInventory {
        self.physical.inventory()
    }

    /// Arrange one deterministic typed failure. This testing seam is inert
    /// unless explicitly armed and never changes architectural state.
    #[doc(hidden)]
    pub fn inject_allocation_failure(&self, point: AllocationPoint) {
        self.failure.set(Some(point));
    }

    fn allocation(&self, point: AllocationPoint) -> Result<(), PaRisc11Error> {
        if self.failure.get() == Some(point) {
            self.failure.set(None);
            Err(PaRisc11Error::AllocationFailed)
        } else {
            Ok(())
        }
    }

    fn project(&self, point: AllocationPoint) -> Result<PaRisc11State, PaRisc11Error> {
        self.allocation(point)?;
        self.physical.project()
    }

    pub fn snapshot(&self) -> Result<PaRisc11State, PaRisc11Error> {
        self.project(AllocationPoint::Snapshot)
    }

    pub fn restore(&mut self, state: &PaRisc11State) -> Result<(), PaRisc11Error> {
        validate_state(state)?;
        self.allocation(AllocationPoint::Restore)?;
        let candidate = checked_clone(state)?;
        self.physical.apply(&candidate);
        self.observer = candidate;
        Ok(())
    }

    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), PaRisc11Error> {
        validate_program(program, origin)?;
        self.allocation(AllocationPoint::LoadStorage)?;
        let mut memory = Vec::new();
        memory
            .try_reserve_exact(MEMORY_SIZE)
            .map_err(|_| PaRisc11Error::AllocationFailed)?;
        memory.resize(MEMORY_SIZE, 0);
        let start = origin as usize;
        memory[start..start + program.len()].copy_from_slice(program);
        let candidate = PaRisc11State {
            gpr: [0; 32],
            carry_borrow: [false; 8],
            pc: origin,
            next_pc: origin + 4,
            nullify_next: false,
            in_delay_slot: false,
            memory,
            halted: false,
            loaded_origin: origin,
            loaded_len: program.len(),
        };
        self.physical.apply(&candidate);
        self.observer = candidate;
        Ok(())
    }

    pub fn step(&mut self) -> Result<PaRisc11StepTrace, PaRisc11Error> {
        let before = self.project(AllocationPoint::Snapshot)?;
        if before.halted {
            return Err(PaRisc11Error::Halted);
        }
        let (raw_bytes, raw_word) = fetch(&before)?;
        self.allocation(AllocationPoint::Candidate)?;
        let mut candidate = checked_clone(&before)?;
        let (mnemonic, nullified) = if before.nullify_next {
            candidate.pc = candidate.next_pc;
            candidate.next_pc = gates::add32(candidate.next_pc, 4, 0).0;
            candidate.nullify_next = false;
            candidate.in_delay_slot = false;
            ("NULLIFIED", true)
        } else {
            (engine::execute_word(&mut candidate, raw_word)?, false)
        };
        candidate.gpr[0] = 0;
        self.allocation(AllocationPoint::StateAfter)?;
        let state_after = checked_clone(&candidate)?;
        let trace = PaRisc11StepTrace {
            pc_before: before.pc,
            next_pc_before: before.next_pc,
            pc_after: candidate.pc,
            next_pc_after: candidate.next_pc,
            raw_bytes,
            raw_word,
            mnemonic,
            nullified,
            state_after,
        };
        self.physical.apply(&candidate);
        self.observer = candidate;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<PaRisc11ExecutionResult, PaRisc11Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(PaRisc11Error::TraceLimitExceeded {
                requested: max_steps,
                maximum: MAX_RETAINED_STEPS,
            });
        }
        let before = self.project(AllocationPoint::Snapshot)?;
        self.allocation(AllocationPoint::TraceRetention)?;
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| PaRisc11Error::AllocationFailed)?;
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => {
                    let halted = trace.state_after.halted;
                    traces.push(trace);
                    if halted {
                        if let Err(error) = self.allocation(AllocationPoint::FinalState) {
                            self.physical.apply(&before);
                            self.observer = before;
                            return Err(error);
                        }
                        let final_state = match self.physical.project() {
                            Ok(state) => state,
                            Err(error) => {
                                self.physical.apply(&before);
                                self.observer = before;
                                return Err(error);
                            }
                        };
                        return Ok(PaRisc11ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            traces,
                            final_state,
                        });
                    }
                }
                Err(error) => {
                    self.physical.apply(&before);
                    self.observer = before;
                    return Err(error);
                }
            }
        }
        self.physical.apply(&before);
        self.observer = before;
        Err(PaRisc11Error::StepLimitExceeded { max_steps })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<PaRisc11ExecutionResult, PaRisc11Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(PaRisc11Error::TraceLimitExceeded {
                requested: max_steps,
                maximum: MAX_RETAINED_STEPS,
            });
        }
        let before = self.project(AllocationPoint::Snapshot)?;
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.physical.apply(&before);
                self.observer = before;
                Err(error)
            }
        }
    }

    #[cfg(test)]
    fn corrupt_observer_halt(&mut self, halted: bool) {
        self.observer.halted = halted;
    }

    #[cfg(test)]
    fn corrupt_halt_q(&mut self, halted: bool) {
        self.physical.corrupt_halt_q(halted);
    }
}

fn checked_clone(state: &PaRisc11State) -> Result<PaRisc11State, PaRisc11Error> {
    let mut memory = Vec::new();
    memory
        .try_reserve_exact(state.memory.len())
        .map_err(|_| PaRisc11Error::AllocationFailed)?;
    memory.extend_from_slice(&state.memory);
    Ok(PaRisc11State {
        gpr: state.gpr,
        carry_borrow: state.carry_borrow,
        pc: state.pc,
        next_pc: state.next_pc,
        nullify_next: state.nullify_next,
        in_delay_slot: state.in_delay_slot,
        memory,
        halted: state.halted,
        loaded_origin: state.loaded_origin,
        loaded_len: state.loaded_len,
    })
}

fn validate_program(program: &[u8], origin: u32) -> Result<(), PaRisc11Error> {
    if program.is_empty() {
        return Err(PaRisc11Error::EmptyProgram);
    }
    if origin & 3 != 0 || program.len() & 3 != 0 {
        return Err(PaRisc11Error::MisalignedProgram {
            origin,
            len: program.len(),
        });
    }
    let end =
        (origin as usize)
            .checked_add(program.len())
            .ok_or(PaRisc11Error::ProgramOutOfRange {
                origin,
                len: program.len(),
            })?;
    if end > MEMORY_SIZE {
        return Err(PaRisc11Error::ProgramOutOfRange {
            origin,
            len: program.len(),
        });
    }
    Ok(())
}

fn validate_state(state: &PaRisc11State) -> Result<(), PaRisc11Error> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(PaRisc11Error::InvalidState("memory length"));
    }
    if state.gpr[0] != 0 {
        return Err(PaRisc11Error::InvalidState("GR0 is not zero"));
    }
    if state.pc & 3 != 0 || state.next_pc & 3 != 0 {
        return Err(PaRisc11Error::InvalidState("instruction queue alignment"));
    }
    if state.loaded_len == 0 {
        if state.loaded_origin != 0 || state.halted || state.nullify_next || state.in_delay_slot {
            return Err(PaRisc11Error::InvalidState("empty-image lifecycle"));
        }
        return Ok(());
    }
    if state.loaded_origin & 3 != 0 || state.loaded_len & 3 != 0 {
        return Err(PaRisc11Error::InvalidState("installed image alignment"));
    }
    let end = (state.loaded_origin as usize)
        .checked_add(state.loaded_len)
        .ok_or(PaRisc11Error::InvalidState("installed image overflow"))?;
    if end > MEMORY_SIZE {
        return Err(PaRisc11Error::InvalidState("installed image range"));
    }
    if state.pc as usize > MEMORY_SIZE || state.next_pc as usize > MEMORY_SIZE {
        return Err(PaRisc11Error::InvalidState("instruction queue range"));
    }
    if state.nullify_next && !state.in_delay_slot {
        return Err(PaRisc11Error::InvalidState(
            "nullification outside delay slot",
        ));
    }
    Ok(())
}

fn fetch(state: &PaRisc11State) -> Result<([u8; 4], u32), PaRisc11Error> {
    if state.pc & 3 != 0 {
        return Err(PaRisc11Error::MisalignedFetch { pc: state.pc });
    }
    let start = state.pc as usize;
    let installed_start = state.loaded_origin as usize;
    let installed_end = installed_start + state.loaded_len;
    if start < installed_start || start.checked_add(4).is_none_or(|end| end > installed_end) {
        return Err(PaRisc11Error::FetchOutsideProgram { pc: state.pc });
    }
    let raw_bytes: [u8; 4] = state.memory[start..start + 4]
        .try_into()
        .expect("validated four-byte fetch");
    Ok((raw_bytes, u32::from_be_bytes(raw_bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_is_derived_and_exact() {
        let cpu = PaRisc11GateSimulator::new();
        let inventory = cpu.physical_storage_inventory();
        assert_eq!(inventory.memory, MEMORY_SIZE * 8);
        assert_eq!(inventory.general_registers, 32 * 32);
        assert_eq!(inventory.total(), FLIP_FLOP_COUNT);
    }

    #[test]
    fn stable_q_is_authoritative_in_both_divergence_directions() {
        let mut cpu = PaRisc11GateSimulator::new();
        cpu.load(&[0; 4], 0).unwrap();
        cpu.corrupt_observer_halt(true);
        assert_eq!(cpu.step().unwrap().mnemonic, "BREAK");
        assert!(cpu.state().halted);

        let mut cpu = PaRisc11GateSimulator::new();
        cpu.load(&[0; 4], 0).unwrap();
        cpu.corrupt_halt_q(true);
        assert_eq!(cpu.step(), Err(PaRisc11Error::Halted));
        assert!(
            !cpu.state().halted,
            "observer remains a non-authoritative cache"
        );
        assert!(cpu.snapshot().unwrap().halted);
    }
}
