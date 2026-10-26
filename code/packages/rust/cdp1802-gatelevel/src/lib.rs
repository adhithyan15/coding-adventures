//! DFF-backed educational gate model of the RCA CDP1802.
//!
//! Architectural state lives only in stable Q outputs.  The independent
//! transition engine computes a candidate, and a successful transaction clocks
//! that candidate into the authoritative storage bank.

mod decode;
#[allow(dead_code)]
mod engine;
mod gates;
mod state;

use core::cell::Cell;
use engine::Cdp1802Simulator as TransitionEngine;
use state::{clock_bit, DffMemory, StateRegister};

pub use decode::{decode, DecodedInstruction, InstructionClass};
pub use engine::{
    Cdp1802Error, Cdp1802State, DmaTransaction, ExecutionError, ExecutionResult,
    InterruptTransaction, MemoryAccess, MemoryAccessKind, PortTransaction, StepTrace,
    MAX_RETAINED_STEPS, MEMORY_SIZE,
};

/// Exact count of authoritative D flip-flops in the architectural model.
pub const FLIP_FLOP_COUNT: usize = 524_667;
/// Six primitive gates per repository D flip-flop.
pub const STORAGE_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6;

/// Gate-composed eight-bit addition, exposed for exhaustive verification.
#[doc(hidden)]
#[must_use]
pub fn add8(left: u8, right: u8, carry: bool) -> (u8, bool) {
    gates::add8(left, right, carry)
}

/// Gate-composed eight-bit subtraction with CDP1802 no-borrow DF.
#[doc(hidden)]
#[must_use]
pub fn subtract8(minuend: u8, subtrahend: u8, borrow: bool) -> (u8, bool) {
    gates::subtract8(minuend, subtrahend, borrow)
}

/// DFF-backed RCA CDP1802 gate simulator.
#[derive(Debug)]
pub struct Cdp1802GateSimulator {
    memory: DffMemory,
    registers: [StateRegister; 16],
    p: StateRegister,
    x: StateRegister,
    d: StateRegister,
    df: u8,
    ie: u8,
    q: u8,
    t: StateRegister,
    ef: [u8; 4],
    input_ports: [StateRegister; 7],
    origin_present: u8,
    origin: StateRegister,
    length: StateRegister,
    first_instruction_pending: u8,
    idle: u8,
    allocation_failure_after: Cell<Option<usize>>,
}

impl Default for Cdp1802GateSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Cdp1802GateSimulator {
    fn clone(&self) -> Self {
        Self {
            memory: self.memory.clone(),
            registers: self.registers.clone(),
            p: self.p.clone(),
            x: self.x.clone(),
            d: self.d.clone(),
            df: self.df,
            ie: self.ie,
            q: self.q,
            t: self.t.clone(),
            ef: self.ef,
            input_ports: self.input_ports.clone(),
            origin_present: self.origin_present,
            origin: self.origin.clone(),
            length: self.length.clone(),
            first_instruction_pending: self.first_instruction_pending,
            idle: self.idle,
            allocation_failure_after: Cell::new(self.allocation_failure_after.get()),
        }
    }
}

impl Cdp1802GateSimulator {
    /// Construct a deterministic reset machine.
    #[must_use]
    pub fn new() -> Self {
        let mut machine = Self {
            memory: DffMemory::new(MEMORY_SIZE),
            registers: std::array::from_fn(|_| StateRegister::new(16)),
            p: StateRegister::new(4),
            x: StateRegister::new(4),
            d: StateRegister::new(8),
            df: 0,
            ie: 0,
            q: 0,
            t: StateRegister::new(8),
            ef: [0; 4],
            input_ports: std::array::from_fn(|_| StateRegister::new(8)),
            origin_present: 0,
            origin: StateRegister::new(16),
            length: StateRegister::new(17),
            first_instruction_pending: 0,
            idle: 0,
            allocation_failure_after: Cell::new(None),
        };
        machine.reset();
        debug_assert_eq!(machine.flip_flop_count(), FLIP_FLOP_COUNT);
        machine
    }

    /// Return the topology-derived DFF inventory.
    #[must_use]
    pub fn flip_flop_count(&self) -> usize {
        self.memory.dff_count()
            + self
                .registers
                .iter()
                .map(StateRegister::dff_count)
                .sum::<usize>()
            + self.p.dff_count()
            + self.x.dff_count()
            + self.d.dff_count()
            + 3
            + self.t.dff_count()
            + self.ef.len()
            + self
                .input_ports
                .iter()
                .map(StateRegister::dff_count)
                .sum::<usize>()
            + 1
            + self.origin.dff_count()
            + self.length.dff_count()
            + 1
            + 1
    }

    /// Return the storage-only primitive-gate inventory.
    #[must_use]
    pub fn storage_gate_count(&self) -> usize {
        self.flip_flop_count() * 6
    }

    /// Clock deterministic reset values into every architectural Q output.
    pub fn reset(&mut self) {
        let reset = Cdp1802State::default();
        self.commit_state(&reset);
    }

    fn project_state(&self) -> Cdp1802State {
        Cdp1802State {
            registers: std::array::from_fn(|index| self.registers[index].read() as u16),
            p: self.p.read() as u8,
            x: self.x.read() as u8,
            d: self.d.read() as u8,
            df: self.df != 0,
            ie: self.ie != 0,
            q: self.q != 0,
            t: self.t.read() as u8,
            ef: std::array::from_fn(|index| self.ef[index] != 0),
            input_ports: std::array::from_fn(|index| self.input_ports[index].read() as u8),
            memory: self.memory.snapshot(),
            origin: (self.origin_present != 0).then(|| self.origin.read() as u16),
            length: self.length.read() as u32,
            first_instruction_pending: self.first_instruction_pending != 0,
            idle: self.idle != 0,
        }
    }

    fn commit_state(&mut self, candidate: &Cdp1802State) {
        for (storage, value) in self.registers.iter_mut().zip(candidate.registers) {
            storage.clock(u64::from(value));
        }
        self.p.clock(u64::from(candidate.p));
        self.x.clock(u64::from(candidate.x));
        self.d.clock(u64::from(candidate.d));
        clock_bit(&mut self.df, candidate.df);
        clock_bit(&mut self.ie, candidate.ie);
        clock_bit(&mut self.q, candidate.q);
        self.t.clock(u64::from(candidate.t));
        for (storage, value) in self.ef.iter_mut().zip(candidate.ef) {
            clock_bit(storage, value);
        }
        for (storage, value) in self.input_ports.iter_mut().zip(candidate.input_ports) {
            storage.clock(u64::from(value));
        }
        clock_bit(&mut self.origin_present, candidate.origin.is_some());
        self.origin.clock(u64::from(candidate.origin.unwrap_or(0)));
        self.length.clock(u64::from(candidate.length));
        clock_bit(
            &mut self.first_instruction_pending,
            candidate.first_instruction_pending,
        );
        clock_bit(&mut self.idle, candidate.idle);
        self.memory.restore(&candidate.memory);
    }

    fn allocation_checkpoint(&self) -> Result<(), Cdp1802Error> {
        if let Some(remaining) = self.allocation_failure_after.get() {
            if remaining == 0 {
                self.allocation_failure_after.set(None);
                return Err(Cdp1802Error::AllocationFailed);
            }
            self.allocation_failure_after.set(Some(remaining - 1));
        }
        Ok(())
    }

    fn engine(&self) -> Result<TransitionEngine, Cdp1802Error> {
        let mut engine = TransitionEngine::new();
        engine.restore(self.project_state())?;
        engine.fail_allocation_after(self.allocation_failure_after.get());
        Ok(engine)
    }

    fn finish_engine(&self) {
        self.allocation_failure_after.set(None);
    }

    /// Infallibly clone all stable Q outputs.
    #[must_use]
    pub fn get_state(&self) -> Cdp1802State {
        self.project_state()
    }

    /// Alias for [`Self::get_state`].
    #[must_use]
    pub fn snapshot(&self) -> Cdp1802State {
        self.get_state()
    }

    /// Fallibly clone all stable Q outputs.
    pub fn try_get_state(&self) -> Result<Cdp1802State, Cdp1802Error> {
        self.allocation_checkpoint()?;
        let memory = self
            .memory
            .try_snapshot()
            .map_err(|()| Cdp1802Error::AllocationFailed)?;
        let mut state = self.project_state();
        state.memory = memory;
        Ok(state)
    }

    /// Atomically validate and clock a complete snapshot into storage.
    pub fn restore(&mut self, state: Cdp1802State) -> Result<(), Cdp1802Error> {
        let mut validator = TransitionEngine::new();
        validator.restore(state.clone())?;
        self.commit_state(&state);
        Ok(())
    }

    /// Atomically install a program image.
    pub fn load(&mut self, program: &[u8], origin: u16) -> Result<(), Cdp1802Error> {
        let mut engine = self.engine()?;
        let result = engine.load(program, origin);
        self.finish_engine();
        result?;
        self.commit_state(&engine.get_state());
        Ok(())
    }

    /// Read a byte directly from memory Q.
    #[must_use]
    pub fn read_memory(&self, address: u16) -> u8 {
        self.memory.read(usize::from(address))
    }

    /// Clock a byte directly into memory Q.
    pub fn write_memory(&mut self, address: u16, value: u8) {
        self.memory.write(usize::from(address), value);
    }

    /// Read one register-matrix Q bank.
    pub fn read_register(&self, register: u8) -> Result<u16, Cdp1802Error> {
        self.registers
            .get(usize::from(register))
            .map(|value| value.read() as u16)
            .ok_or(Cdp1802Error::InvalidState {
                reason: "register selector is outside 0..15",
            })
    }

    /// Clock one register-matrix Q bank.
    pub fn write_register(&mut self, register: u8, value: u16) -> Result<(), Cdp1802Error> {
        let storage =
            self.registers
                .get_mut(usize::from(register))
                .ok_or(Cdp1802Error::InvalidState {
                    reason: "register selector is outside 0..15",
                })?;
        storage.clock(u64::from(value));
        Ok(())
    }

    /// Set one active-low EF input latch.
    pub fn set_external_flag(&mut self, flag: u8, asserted: bool) -> Result<(), Cdp1802Error> {
        let storage = flag
            .checked_sub(1)
            .and_then(|index| self.ef.get_mut(index as usize))
            .ok_or(Cdp1802Error::InvalidExternalFlag { flag })?;
        clock_bit(storage, asserted);
        Ok(())
    }

    /// Clock a deterministic input-port latch.
    pub fn set_input_port(&mut self, port: u8, value: u8) -> Result<(), Cdp1802Error> {
        let storage = port
            .checked_sub(1)
            .and_then(|index| self.input_ports.get_mut(index as usize))
            .ok_or(Cdp1802Error::InvalidPort { port })?;
        storage.clock(u64::from(value));
        Ok(())
    }

    /// Execute one independent transition and clock it atomically.
    pub fn step(&mut self) -> Result<StepTrace, Cdp1802Error> {
        let mut engine = self.engine()?;
        let result = engine.step();
        self.finish_engine();
        let trace = result?;
        self.commit_state(&trace.state_after);
        Ok(trace)
    }

    /// Execute a bounded transaction, rolling back on every failure.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Cdp1802Error> {
        let mut engine = self.engine()?;
        let result = engine.run(max_steps);
        self.finish_engine();
        let result = result?;
        self.commit_state(&result.final_state);
        Ok(result)
    }

    /// Atomically load and execute a program.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u16,
        max_steps: usize,
    ) -> Result<ExecutionResult, ExecutionError> {
        let mut engine = self.engine().map_err(|source| ExecutionError { source })?;
        let result = engine.execute(program, origin, max_steps);
        self.finish_engine();
        let result = result?;
        self.commit_state(&result.final_state);
        Ok(result)
    }

    /// Acknowledge DMA input and clock the candidate atomically.
    pub fn dma_in(&mut self, value: u8) -> Result<DmaTransaction, Cdp1802Error> {
        let mut engine = self.engine()?;
        let transaction = engine.dma_in(value)?;
        self.commit_state(&engine.get_state());
        Ok(transaction)
    }

    /// Acknowledge DMA output and clock the candidate atomically.
    pub fn dma_out(&mut self) -> Result<DmaTransaction, Cdp1802Error> {
        let mut engine = self.engine()?;
        let transaction = engine.dma_out()?;
        self.commit_state(&engine.get_state());
        Ok(transaction)
    }

    /// Acknowledge an eligible interrupt and clock the candidate atomically.
    pub fn interrupt(&mut self) -> Result<InterruptTransaction, Cdp1802Error> {
        let mut engine = self.engine()?;
        let transaction = engine.interrupt()?;
        self.commit_state(&engine.get_state());
        Ok(transaction)
    }

    /// Install a deterministic checked-allocation failure seam.
    #[doc(hidden)]
    pub fn fail_allocation_after(&self, checkpoints: Option<usize>) {
        self.allocation_failure_after.set(checkpoints);
    }
}

/// Compatibility alias matching the functional package's type name.
pub type Cdp1802Simulator = Cdp1802GateSimulator;

#[cfg(test)]
mod tests;
