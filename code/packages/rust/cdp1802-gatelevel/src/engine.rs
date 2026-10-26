//! Independent checked candidate-transition model for the gate machine.
//!
//! The model owns the complete 16-register matrix and 64 KiB memory. Program
//! and data addresses wrap exactly as sixteen-bit architectural registers do;
//! installed-image bounds constrain instruction bytes only. Ports, DMA, and
//! interrupt acknowledgement are explicit transactions so later SoC models
//! can provide arbitration and timing without hidden callbacks.
//!
//! ```
//! use cdp1802_gatelevel::Cdp1802GateSimulator;
//!
//! let mut cpu = Cdp1802GateSimulator::new();
//! cpu.load(&[0xf8, 0x42, 0xb1, 0x00], 0x0100)?; // LDI; PHI R1; IDL
//! let result = cpu.run(3)?;
//! assert!(result.stopped);
//! assert_eq!(result.final_state.registers[1], 0x4200);
//! # Ok::<(), cdp1802_gatelevel::Cdp1802Error>(())
//! ```

use crate::decode::decode;
use crate::gates::{
    bit_and, bit_or, bit_xor, condition, decode4, decrement16, increment16, is_zero, select16,
    shift_left, shift_right,
};

use core::cell::Cell;
use core::fmt;

/// Bytes in the CDP1802 address space.
pub const MEMORY_SIZE: usize = 65_536;
/// Maximum complete-state traces retained by one run.
pub const MAX_RETAINED_STEPS: usize = 64;

/// Complete restorable CPU, environment, memory, and image state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cdp1802State {
    pub registers: [u16; 16],
    pub p: u8,
    pub x: u8,
    pub d: u8,
    pub df: bool,
    pub ie: bool,
    pub q: bool,
    pub t: u8,
    /// `true` means the corresponding active-low EF input is asserted.
    pub ef: [bool; 4],
    pub input_ports: [u8; 7],
    pub memory: Vec<u8>,
    pub origin: Option<u16>,
    /// Installed length uses `u32` so a full 65,536-byte image is representable.
    pub length: u32,
    pub first_instruction_pending: bool,
    pub idle: bool,
}

impl Default for Cdp1802State {
    fn default() -> Self {
        Self {
            registers: [0; 16],
            p: 0,
            x: 0,
            d: 0,
            df: false,
            ie: true,
            q: false,
            t: 0,
            ef: [false; 4],
            input_ports: [0; 7],
            memory: vec![0; MEMORY_SIZE],
            origin: None,
            length: 0,
            first_instruction_pending: true,
            idle: false,
        }
    }
}

/// Direction of an architectural data-memory access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryAccessKind {
    Read,
    Write,
}

/// One architectural data-memory access (instruction-byte traffic excluded).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryAccess {
    pub kind: MemoryAccessKind,
    pub address: u16,
    pub value: u8,
}

/// Explicit I/O transaction produced by one instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortTransaction {
    Write { port: u8, value: u8 },
    Read { port: u8, value: u8 },
}

/// One acknowledged DMA cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DmaTransaction {
    In { address: u16, value: u8 },
    Out { address: u16, value: u8 },
}

/// One accepted interrupt acknowledgement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterruptTransaction {
    pub saved_selectors: u8,
}

/// Exact committed instruction transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    pub p_before: u8,
    pub p_after: u8,
    pub x_before: u8,
    pub x_after: u8,
    pub raw: [u8; 3],
    pub width: u8,
    pub mnemonic: &'static str,
    pub d_before: u8,
    pub d_after: u8,
    pub df_before: bool,
    pub df_after: bool,
    pub q_before: bool,
    pub q_after: bool,
    pub ie_before: bool,
    pub ie_after: bool,
    pub memory_access: Option<MemoryAccess>,
    pub port_transaction: Option<PortTransaction>,
    pub branch_taken: Option<bool>,
    pub cycles: u8,
    pub idle_before: bool,
    pub idle_after: bool,
    pub state_before: Cdp1802State,
    pub state_after: Cdp1802State,
}

/// Successful bounded execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub stopped: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: Cdp1802State,
}

/// Typed checked-lifecycle failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cdp1802Error {
    EmptyProgram,
    ProgramTooLarge { length: usize },
    ProgramRangeOverflow { origin: u16, length: usize },
    NoProgramLoaded,
    FetchOutsideProgram { pc: u16 },
    TruncatedInstruction { pc: u16, needed: u8 },
    UnknownInstruction { raw: u8 },
    InvalidState { reason: &'static str },
    InvalidExternalFlag { flag: u8 },
    InvalidPort { port: u8 },
    Idling,
    InterruptInhibited,
    InterruptMasked,
    RetentionLimitExceeded { requested: usize, limit: usize },
    StepLimitExceeded { limit: usize },
    AllocationFailed,
}

impl fmt::Display for Cdp1802Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => formatter.write_str("program image is empty"),
            Self::ProgramTooLarge { length } => {
                write!(
                    formatter,
                    "program contains {length} bytes; maximum is 65536"
                )
            }
            Self::ProgramRangeOverflow { origin, length } => write!(
                formatter,
                "program of {length} bytes at {origin:#06x} wraps the address space"
            ),
            Self::NoProgramLoaded => formatter.write_str("no program image is installed"),
            Self::FetchOutsideProgram { pc } => {
                write!(
                    formatter,
                    "opcode fetch at {pc:#06x} is outside the installed image"
                )
            }
            Self::TruncatedInstruction { pc, needed } => write!(
                formatter,
                "instruction at {pc:#06x} requires {needed} installed bytes"
            ),
            Self::UnknownInstruction { raw } => {
                write!(formatter, "unknown CDP1802 instruction {raw:#04x}")
            }
            Self::InvalidState { reason } => write!(formatter, "invalid CDP1802 state: {reason}"),
            Self::InvalidExternalFlag { flag } => {
                write!(formatter, "external flag EF{flag} does not exist")
            }
            Self::InvalidPort { port } => write!(formatter, "port {port} does not exist"),
            Self::Idling => formatter.write_str("processor is in the IDL state"),
            Self::InterruptInhibited => {
                formatter.write_str("interrupt is inhibited until one instruction completes")
            }
            Self::InterruptMasked => formatter.write_str("interrupt enable is clear"),
            Self::RetentionLimitExceeded { requested, limit } => write!(
                formatter,
                "requested {requested} traces exceeds retention limit {limit}"
            ),
            Self::StepLimitExceeded { limit } => {
                write!(formatter, "step limit {limit} exhausted before IDL")
            }
            Self::AllocationFailed => formatter.write_str("checked allocation failed"),
        }
    }
}

impl std::error::Error for Cdp1802Error {}

/// Source-preserving failure returned by atomic load-and-run execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionError {
    pub source: Cdp1802Error,
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "CDP1802 execution failed: {}", self.source)
    }
}

impl std::error::Error for ExecutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Ephemeral independent candidate-transition machine.
#[derive(Debug)]
pub struct Cdp1802Simulator {
    state: Cdp1802State,
    allocation_failure_after: Cell<Option<usize>>,
}

impl Default for Cdp1802Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Cdp1802Simulator {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            allocation_failure_after: Cell::new(self.allocation_failure_after.get()),
        }
    }
}

#[derive(Default)]
struct TransitionMetadata {
    raw: [u8; 3],
    width: u8,
    mnemonic: &'static str,
    cycles: u8,
    memory_access: Option<MemoryAccess>,
    port_transaction: Option<PortTransaction>,
    branch_taken: Option<bool>,
}

impl Cdp1802Simulator {
    /// Create a deterministic reset machine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Cdp1802State::default(),
            allocation_failure_after: Cell::new(None),
        }
    }

    /// Return to deterministic reset state without allocating.
    pub fn reset(&mut self) {
        self.state.registers = [0; 16];
        self.state.p = 0;
        self.state.x = 0;
        self.state.d = 0;
        self.state.df = false;
        self.state.ie = true;
        self.state.q = false;
        self.state.t = 0;
        self.state.ef = [false; 4];
        self.state.input_ports = [0; 7];
        self.state.memory.fill(0);
        self.state.origin = None;
        self.state.length = 0;
        self.state.first_instruction_pending = true;
        self.state.idle = false;
    }

    /// Infallibly clone complete state for ordinary inspection.
    #[must_use]
    pub fn get_state(&self) -> Cdp1802State {
        self.state.clone()
    }

    /// Alias for [`Self::get_state`] using lifecycle terminology.
    #[must_use]
    pub fn snapshot(&self) -> Cdp1802State {
        self.get_state()
    }

    /// Fallibly clone complete state for checked lifecycle composition.
    pub fn try_get_state(&self) -> Result<Cdp1802State, Cdp1802Error> {
        let memory = self.allocate_memory()?;
        Ok(copy_state_into(&self.state, memory))
    }

    /// Atomically validate and install a complete snapshot.
    pub fn restore(&mut self, state: Cdp1802State) -> Result<(), Cdp1802Error> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    /// Atomically replace memory and CPU state with a freshly loaded image.
    pub fn load(&mut self, program: &[u8], origin: u16) -> Result<(), Cdp1802Error> {
        validate_program(program, origin)?;
        let memory = self.allocate_memory()?;
        self.install_program(program, origin, memory);
        Ok(())
    }

    fn install_program(&mut self, program: &[u8], origin: u16, mut memory: Vec<u8>) {
        memory.fill(0);
        let start = usize::from(origin);
        memory[start..start + program.len()].copy_from_slice(program);
        self.state = Cdp1802State {
            memory,
            origin: Some(origin),
            length: program.len() as u32,
            registers: {
                let mut registers = [0; 16];
                registers[0] = origin;
                registers
            },
            ..state_without_memory()
        };
    }

    /// Read one byte from full architectural memory.
    #[must_use]
    pub fn read_memory(&self, address: u16) -> u8 {
        self.state.memory[usize::from(address)]
    }

    /// Write one byte to full architectural memory.
    pub fn write_memory(&mut self, address: u16, value: u8) {
        self.state.memory[usize::from(address)] = value;
    }

    /// Read a register-matrix entry.
    pub fn read_register(&self, register: u8) -> Result<u16, Cdp1802Error> {
        self.state
            .registers
            .get(usize::from(register))
            .copied()
            .ok_or(Cdp1802Error::InvalidState {
                reason: "register selector is outside 0..15",
            })
    }

    /// Write a register-matrix entry.
    pub fn write_register(&mut self, register: u8, value: u16) -> Result<(), Cdp1802Error> {
        let slot = self.state.registers.get_mut(usize::from(register)).ok_or(
            Cdp1802Error::InvalidState {
                reason: "register selector is outside 0..15",
            },
        )?;
        *slot = value;
        Ok(())
    }

    /// Configure an active-low sampled external flag (one-based EF number).
    pub fn set_external_flag(&mut self, flag: u8, asserted: bool) -> Result<(), Cdp1802Error> {
        let Some(slot) = flag
            .checked_sub(1)
            .and_then(|index| self.state.ef.get_mut(index as usize))
        else {
            return Err(Cdp1802Error::InvalidExternalFlag { flag });
        };
        *slot = asserted;
        Ok(())
    }

    /// Configure one deterministic input port (ports 1 through 7).
    pub fn set_input_port(&mut self, port: u8, value: u8) -> Result<(), Cdp1802Error> {
        let Some(slot) = port
            .checked_sub(1)
            .and_then(|index| self.state.input_ports.get_mut(index as usize))
        else {
            return Err(Cdp1802Error::InvalidPort { port });
        };
        *slot = value;
        Ok(())
    }

    /// Execute one checked, transactional instruction.
    pub fn step(&mut self) -> Result<StepTrace, Cdp1802Error> {
        let mut memories = self.prepare_memories(3)?;
        let after_memory = memories.pop().expect("three prepared memories");
        let candidate_memory = memories.pop().expect("three prepared memories");
        let before_memory = memories.pop().expect("three prepared memories");
        self.step_prepared(before_memory, candidate_memory, after_memory)
    }

    fn step_prepared(
        &mut self,
        before_memory: Vec<u8>,
        candidate_memory: Vec<u8>,
        after_memory: Vec<u8>,
    ) -> Result<StepTrace, Cdp1802Error> {
        validate_state(&self.state)?;
        if self.state.idle {
            return Err(Cdp1802Error::Idling);
        }
        let before = copy_state_into(&self.state, before_memory);
        let mut candidate = copy_state_into(&self.state, candidate_memory);
        let metadata = execute_one(&mut candidate)?;
        candidate.first_instruction_pending = false;
        let after = copy_state_into(&candidate, after_memory);
        let trace = StepTrace {
            pc_before: before.registers[usize::from(before.p)],
            pc_after: after.registers[usize::from(after.p)],
            p_before: before.p,
            p_after: after.p,
            x_before: before.x,
            x_after: after.x,
            raw: metadata.raw,
            width: metadata.width,
            mnemonic: metadata.mnemonic,
            d_before: before.d,
            d_after: after.d,
            df_before: before.df,
            df_after: after.df,
            q_before: before.q,
            q_after: after.q,
            ie_before: before.ie,
            ie_after: after.ie,
            memory_access: metadata.memory_access,
            port_transaction: metadata.port_transaction,
            branch_taken: metadata.branch_taken,
            cycles: metadata.cycles,
            idle_before: before.idle,
            idle_after: after.idle,
            state_before: before,
            state_after: after,
        };
        self.state = candidate;
        Ok(trace)
    }

    /// Execute up to `max_steps`, succeeding only when an IDL instruction commits.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Cdp1802Error> {
        let (traces, memories) = self.prepare_run(max_steps)?;
        self.run_prepared(max_steps, traces, memories)
    }

    fn prepare_run(
        &self,
        max_steps: usize,
    ) -> Result<(Vec<StepTrace>, Vec<Vec<u8>>), Cdp1802Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(Cdp1802Error::RetentionLimitExceeded {
                requested: max_steps,
                limit: MAX_RETAINED_STEPS,
            });
        }
        self.allocation_checkpoint()?;
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| Cdp1802Error::AllocationFailed)?;
        let memories = self.prepare_memories(max_steps.saturating_mul(3).saturating_add(2))?;
        Ok((traces, memories))
    }

    fn run_prepared(
        &mut self,
        max_steps: usize,
        mut traces: Vec<StepTrace>,
        memories: Vec<Vec<u8>>,
    ) -> Result<ExecutionResult, Cdp1802Error> {
        let mut memories = memories.into_iter();
        let rollback = copy_state_into(
            &self.state,
            memories.next().expect("run rollback memory prepared"),
        );
        for _ in 0..max_steps {
            let result = self.step_prepared(
                memories.next().expect("step before memory prepared"),
                memories.next().expect("step candidate memory prepared"),
                memories.next().expect("step after memory prepared"),
            );
            let trace = match result {
                Ok(trace) => trace,
                Err(error) => {
                    self.state = rollback;
                    return Err(error);
                }
            };
            let stopped = trace.idle_after;
            traces.push(trace);
            if stopped {
                let final_state = copy_state_into(
                    &self.state,
                    memories.next().expect("run final memory prepared"),
                );
                return Ok(ExecutionResult {
                    stopped: true,
                    steps: traces.len(),
                    traces,
                    final_state,
                });
            }
        }
        self.state = rollback;
        Err(Cdp1802Error::StepLimitExceeded { limit: max_steps })
    }

    /// Atomically load and run a program.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u16,
        max_steps: usize,
    ) -> Result<ExecutionResult, ExecutionError> {
        validate_program(program, origin).map_err(|source| ExecutionError { source })?;
        let (traces, memories) = self
            .prepare_run(max_steps)
            .map_err(|source| ExecutionError { source })?;
        let mut outer = self
            .prepare_memories(2)
            .map_err(|source| ExecutionError { source })?;
        let load_memory = outer.pop().expect("load memory prepared");
        let original = copy_state_into(
            &self.state,
            outer.pop().expect("execute rollback memory prepared"),
        );
        self.install_program(program, origin, load_memory);
        match self.run_prepared(max_steps, traces, memories) {
            Ok(result) => Ok(result),
            Err(source) => {
                self.state = original;
                Err(ExecutionError { source })
            }
        }
    }

    /// Acknowledge one DMA input cycle.
    pub fn dma_in(&mut self, value: u8) -> Result<DmaTransaction, Cdp1802Error> {
        validate_state(&self.state)?;
        let address = self.state.registers[0];
        self.state.memory[usize::from(address)] = value;
        self.state.registers[0] = increment16(address);
        self.state.idle = false;
        Ok(DmaTransaction::In { address, value })
    }

    /// Acknowledge one DMA output cycle.
    pub fn dma_out(&mut self) -> Result<DmaTransaction, Cdp1802Error> {
        validate_state(&self.state)?;
        let address = self.state.registers[0];
        let value = self.state.memory[usize::from(address)];
        self.state.registers[0] = increment16(address);
        self.state.idle = false;
        Ok(DmaTransaction::Out { address, value })
    }

    /// Acknowledge an eligible interrupt using the reset-inhibit-first rule.
    pub fn interrupt(&mut self) -> Result<InterruptTransaction, Cdp1802Error> {
        validate_state(&self.state)?;
        if self.state.first_instruction_pending {
            return Err(Cdp1802Error::InterruptInhibited);
        }
        if !self.state.ie {
            return Err(Cdp1802Error::InterruptMasked);
        }
        let saved_selectors = (self.state.x << 4) | self.state.p;
        self.state.t = saved_selectors;
        self.state.x = 2;
        self.state.p = 1;
        self.state.ie = false;
        self.state.idle = false;
        Ok(InterruptTransaction { saved_selectors })
    }

    /// Install a deterministic failure at a future checked allocation.
    ///
    /// `Some(0)` fails the next allocation checkpoint. This seam is public so
    /// downstream lifecycle tests can prove rollback without relying on global
    /// allocator behavior.
    #[doc(hidden)]
    pub fn fail_allocation_after(&self, checkpoints: Option<usize>) {
        self.allocation_failure_after.set(checkpoints);
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

    fn allocate_memory(&self) -> Result<Vec<u8>, Cdp1802Error> {
        self.allocation_checkpoint()?;
        let mut memory = Vec::new();
        memory
            .try_reserve_exact(MEMORY_SIZE)
            .map_err(|_| Cdp1802Error::AllocationFailed)?;
        memory.resize(MEMORY_SIZE, 0);
        Ok(memory)
    }

    fn prepare_memories(&self, count: usize) -> Result<Vec<Vec<u8>>, Cdp1802Error> {
        self.allocation_checkpoint()?;
        let mut memories = Vec::new();
        memories
            .try_reserve_exact(count)
            .map_err(|_| Cdp1802Error::AllocationFailed)?;
        for _ in 0..count {
            memories.push(self.allocate_memory()?);
        }
        Ok(memories)
    }
}

fn state_without_memory() -> Cdp1802State {
    Cdp1802State {
        memory: Vec::new(),
        registers: [0; 16],
        p: 0,
        x: 0,
        d: 0,
        df: false,
        ie: true,
        q: false,
        t: 0,
        ef: [false; 4],
        input_ports: [0; 7],
        origin: None,
        length: 0,
        first_instruction_pending: true,
        idle: false,
    }
}

fn copy_state_into(source: &Cdp1802State, mut memory: Vec<u8>) -> Cdp1802State {
    memory.copy_from_slice(&source.memory);
    Cdp1802State {
        registers: source.registers,
        p: source.p,
        x: source.x,
        d: source.d,
        df: source.df,
        ie: source.ie,
        q: source.q,
        t: source.t,
        ef: source.ef,
        input_ports: source.input_ports,
        memory,
        origin: source.origin,
        length: source.length,
        first_instruction_pending: source.first_instruction_pending,
        idle: source.idle,
    }
}

fn validate_program(program: &[u8], origin: u16) -> Result<(), Cdp1802Error> {
    if program.is_empty() {
        return Err(Cdp1802Error::EmptyProgram);
    }
    if program.len() > MEMORY_SIZE {
        return Err(Cdp1802Error::ProgramTooLarge {
            length: program.len(),
        });
    }
    if usize::from(origin) + program.len() > MEMORY_SIZE {
        return Err(Cdp1802Error::ProgramRangeOverflow {
            origin,
            length: program.len(),
        });
    }
    Ok(())
}

fn validate_state(state: &Cdp1802State) -> Result<(), Cdp1802Error> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(Cdp1802Error::InvalidState {
            reason: "memory must contain exactly 65536 bytes",
        });
    }
    if state.p > 0x0f || state.x > 0x0f {
        return Err(Cdp1802Error::InvalidState {
            reason: "P and X must be four-bit selectors",
        });
    }
    match (state.origin, state.length) {
        (None, 0) => Ok(()),
        (None, _) => Err(Cdp1802Error::InvalidState {
            reason: "an installed length requires an origin",
        }),
        (Some(_), 0) => Err(Cdp1802Error::InvalidState {
            reason: "an installed origin requires a nonempty image",
        }),
        (Some(origin), length)
            if length <= MEMORY_SIZE as u32
                && u32::from(origin).saturating_add(length) <= MEMORY_SIZE as u32 =>
        {
            Ok(())
        }
        (Some(_), _) => Err(Cdp1802Error::InvalidState {
            reason: "installed image wraps or exceeds memory",
        }),
    }
}

fn image_contains(state: &Cdp1802State, address: u16) -> bool {
    let Some(origin) = state.origin else {
        return false;
    };
    let address = u32::from(address);
    let start = u32::from(origin);
    address >= start && address < start + state.length
}

fn execute_one(state: &mut Cdp1802State) -> Result<TransitionMetadata, Cdp1802Error> {
    if state.origin.is_none() {
        return Err(Cdp1802Error::NoProgramLoaded);
    }
    let pc = state.registers[usize::from(state.p)];
    if !image_contains(state, pc) {
        return Err(Cdp1802Error::FetchOutsideProgram { pc });
    }
    let opcode = state.memory[usize::from(pc)];
    let decoded = decode(opcode)?;
    state.registers[usize::from(state.p)] = increment16(pc);
    let mut metadata = TransitionMetadata {
        raw: [opcode, 0, 0],
        width: decoded.width,
        mnemonic: decoded.mnemonic,
        cycles: decoded.cycles,
        ..TransitionMetadata::default()
    };
    let high = decode4(opcode >> 4)
        .into_iter()
        .position(|wire| wire == 1)
        .expect("four-bit decoder is one-hot") as u8;
    let low = decode4(opcode & 0x0f)
        .into_iter()
        .position(|wire| wire == 1)
        .expect("four-bit decoder is one-hot") as u8;
    match high {
        0x0 => execute_zero(state, low, &mut metadata),
        0x1 => state.registers[usize::from(low)] = increment16(state.registers[usize::from(low)]),
        0x2 => state.registers[usize::from(low)] = decrement16(state.registers[usize::from(low)]),
        0x3 => execute_short_control(state, pc, low, &mut metadata)?,
        0x4 => {
            let address = state.registers[usize::from(low)];
            state.d = state.memory[usize::from(address)];
            state.registers[usize::from(low)] = increment16(address);
            metadata.memory_access = Some(read_access(address, state.d));
        }
        0x5 => {
            let address = state.registers[usize::from(low)];
            state.memory[usize::from(address)] = state.d;
            metadata.memory_access = Some(write_access(address, state.d));
        }
        0x6 => execute_io(state, low, &mut metadata),
        0x7 => execute_control(state, pc, low, &mut metadata)?,
        0x8 => state.d = state.registers[usize::from(low)] as u8,
        0x9 => state.d = (state.registers[usize::from(low)] >> 8) as u8,
        0xa => {
            let current = state.registers[usize::from(low)];
            state.registers[usize::from(low)] = (current & 0xff00) | u16::from(state.d);
        }
        0xb => {
            let current = state.registers[usize::from(low)];
            state.registers[usize::from(low)] = (current & 0x00ff) | (u16::from(state.d) << 8);
        }
        0xc => execute_long_control(state, pc, low, &mut metadata)?,
        0xd => state.p = low,
        0xe => state.x = low,
        0xf => execute_alu(state, pc, low, &mut metadata)?,
        _ => unreachable!("four-bit high nibble"),
    }
    Ok(metadata)
}

fn execute_zero(state: &mut Cdp1802State, low: u8, metadata: &mut TransitionMetadata) {
    if low == 0 {
        state.idle = true;
        let address = state.registers[0];
        metadata.memory_access = Some(read_access(address, state.memory[usize::from(address)]));
    } else {
        let address = state.registers[usize::from(low)];
        state.d = state.memory[usize::from(address)];
        metadata.memory_access = Some(read_access(address, state.d));
    }
}

fn execute_short_control(
    state: &mut Cdp1802State,
    pc: u16,
    low: u8,
    metadata: &mut TransitionMetadata,
) -> Result<(), Cdp1802Error> {
    let address = state.registers[usize::from(state.p)];
    let operand = fetch_operand(state, pc, address, 2)?;
    metadata.raw[1] = operand;
    let take = match low {
        0x0 => true,
        0x1 => state.q,
        0x2 => is_zero(state.d) != 0,
        0x3 => state.df,
        0x4..=0x7 => state.ef[usize::from(low - 4)],
        0x8 => false,
        0x9 => !state.q,
        0xa => is_zero(state.d) == 0,
        0xb => !state.df,
        0xc..=0xf => !state.ef[usize::from(low - 0xc)],
        _ => unreachable!("four-bit low nibble"),
    };
    let selected = usize::from(state.p);
    state.registers[selected] = select16(
        increment16(address),
        (address & 0xff00) | u16::from(operand),
        condition(take),
    );
    metadata.branch_taken = Some(take);
    Ok(())
}

fn execute_io(state: &mut Cdp1802State, low: u8, metadata: &mut TransitionMetadata) {
    if low == 0 {
        let selected = usize::from(state.x);
        state.registers[selected] = increment16(state.registers[selected]);
    } else if low <= 7 {
        let address = state.registers[usize::from(state.x)];
        let value = state.memory[usize::from(address)];
        state.registers[usize::from(state.x)] = increment16(address);
        metadata.memory_access = Some(read_access(address, value));
        metadata.port_transaction = Some(PortTransaction::Write { port: low, value });
    } else {
        let port = low - 8;
        let value = state.input_ports[usize::from(port - 1)];
        let address = state.registers[usize::from(state.x)];
        state.memory[usize::from(address)] = value;
        state.d = value;
        metadata.memory_access = Some(write_access(address, value));
        metadata.port_transaction = Some(PortTransaction::Read { port, value });
    }
}

fn execute_control(
    state: &mut Cdp1802State,
    pc: u16,
    low: u8,
    metadata: &mut TransitionMetadata,
) -> Result<(), Cdp1802Error> {
    match low {
        0x0 | 0x1 => {
            let old_x = state.x;
            let address = state.registers[usize::from(old_x)];
            let value = state.memory[usize::from(address)];
            state.x = value >> 4;
            state.p = value & 0x0f;
            state.registers[usize::from(old_x)] = increment16(address);
            state.ie = low == 0;
            metadata.memory_access = Some(read_access(address, value));
        }
        0x2 => {
            let address = state.registers[usize::from(state.x)];
            state.d = state.memory[usize::from(address)];
            state.registers[usize::from(state.x)] = increment16(address);
            metadata.memory_access = Some(read_access(address, state.d));
        }
        0x3 => {
            let address = state.registers[usize::from(state.x)];
            state.memory[usize::from(address)] = state.d;
            state.registers[usize::from(state.x)] = decrement16(address);
            metadata.memory_access = Some(write_access(address, state.d));
        }
        0x4 | 0x5 | 0x7 => {
            let address = state.registers[usize::from(state.x)];
            let value = state.memory[usize::from(address)];
            metadata.memory_access = Some(read_access(address, value));
            if low == 4 {
                (state.d, state.df) = add8(state.d, value, state.df);
            } else if low == 5 {
                (state.d, state.df) = subtract8(value, state.d, !state.df);
            } else {
                (state.d, state.df) = subtract8(state.d, value, !state.df);
            }
        }
        0x6 => {
            let old_df = state.df;
            (state.d, state.df) = shift_right(state.d, old_df);
        }
        0x8 => {
            let address = state.registers[usize::from(state.x)];
            state.memory[usize::from(address)] = state.t;
            metadata.memory_access = Some(write_access(address, state.t));
        }
        0x9 => {
            let old_p = state.p;
            state.t = (state.x << 4) | old_p;
            let address = state.registers[2];
            state.memory[usize::from(address)] = state.t;
            state.x = old_p;
            state.registers[2] = decrement16(address);
            metadata.memory_access = Some(write_access(address, state.t));
        }
        0xa => state.q = false,
        0xb => state.q = true,
        0xc | 0xd | 0xf => {
            let immediate = fetch_immediate(state, pc, metadata)?;
            if low == 0xc {
                (state.d, state.df) = add8(state.d, immediate, state.df);
            } else if low == 0xd {
                (state.d, state.df) = subtract8(immediate, state.d, !state.df);
            } else {
                (state.d, state.df) = subtract8(state.d, immediate, !state.df);
            }
        }
        0xe => {
            let old_df = state.df;
            (state.d, state.df) = shift_left(state.d, old_df);
        }
        _ => unreachable!("four-bit low nibble"),
    }
    Ok(())
}

fn execute_long_control(
    state: &mut Cdp1802State,
    pc: u16,
    low: u8,
    metadata: &mut TransitionMetadata,
) -> Result<(), Cdp1802Error> {
    if matches!(low, 0x0..=0x3 | 0x9..=0xb) {
        let address = state.registers[usize::from(state.p)];
        let high = fetch_operand(state, pc, address, 3)?;
        let second_address = increment16(address);
        let low_byte = fetch_operand(state, pc, second_address, 3)?;
        metadata.raw[1] = high;
        metadata.raw[2] = low_byte;
        let take = match low {
            0x0 => true,
            0x1 => state.q,
            0x2 => is_zero(state.d) != 0,
            0x3 => state.df,
            0x9 => !state.q,
            0xa => is_zero(state.d) == 0,
            0xb => !state.df,
            _ => unreachable!("long branch opcode"),
        };
        state.registers[usize::from(state.p)] = select16(
            increment16(increment16(address)),
            (u16::from(high) << 8) | u16::from(low_byte),
            condition(take),
        );
        metadata.branch_taken = Some(take);
    } else if low != 4 {
        let take = match low {
            0x5 => !state.q,
            0x6 => is_zero(state.d) == 0,
            0x7 => !state.df,
            0x8 => true,
            0xc => state.ie,
            0xd => state.q,
            0xe => is_zero(state.d) != 0,
            0xf => state.df,
            _ => unreachable!("long skip opcode"),
        };
        let selected = usize::from(state.p);
        let first_address = state.registers[selected];
        let _first_bus_value = state.memory[usize::from(first_address)];
        let second_address = if take {
            increment16(first_address)
        } else {
            first_address
        };
        let _second_bus_value = state.memory[usize::from(second_address)];
        state.registers[selected] = select16(
            first_address,
            increment16(increment16(first_address)),
            condition(take),
        );
        metadata.branch_taken = Some(take);
    }
    Ok(())
}

fn execute_alu(
    state: &mut Cdp1802State,
    pc: u16,
    low: u8,
    metadata: &mut TransitionMetadata,
) -> Result<(), Cdp1802Error> {
    if low == 0x6 {
        (state.d, state.df) = shift_right(state.d, false);
        return Ok(());
    }
    if low == 0xe {
        (state.d, state.df) = shift_left(state.d, false);
        return Ok(());
    }
    let mut address = None;
    let operand = if low < 8 {
        let location = state.registers[usize::from(state.x)];
        address = Some(location);
        state.memory[usize::from(location)]
    } else {
        fetch_immediate(state, pc, metadata)?
    };
    if let Some(location) = address {
        metadata.memory_access = Some(read_access(location, operand));
    }
    match low {
        0x0 | 0x8 => state.d = operand,
        0x1 | 0x9 => state.d = bit_or(state.d, operand),
        0x2 | 0xa => state.d = bit_and(state.d, operand),
        0x3 | 0xb => state.d = bit_xor(state.d, operand),
        0x4 | 0xc => (state.d, state.df) = add8(state.d, operand, false),
        0x5 | 0xd => (state.d, state.df) = subtract8(operand, state.d, false),
        0x7 | 0xf => (state.d, state.df) = subtract8(state.d, operand, false),
        0x6 | 0xe => unreachable!("shift opcodes return before operand selection"),
        _ => unreachable!("four-bit low nibble"),
    }
    Ok(())
}

fn fetch_immediate(
    state: &mut Cdp1802State,
    pc: u16,
    metadata: &mut TransitionMetadata,
) -> Result<u8, Cdp1802Error> {
    let selected = usize::from(state.p);
    let address = state.registers[selected];
    let value = fetch_operand(state, pc, address, metadata.width)?;
    metadata.raw[1] = value;
    state.registers[selected] = increment16(address);
    Ok(value)
}

fn fetch_operand(
    state: &Cdp1802State,
    pc: u16,
    address: u16,
    needed: u8,
) -> Result<u8, Cdp1802Error> {
    if !image_contains(state, address) {
        return Err(Cdp1802Error::TruncatedInstruction { pc, needed });
    }
    Ok(state.memory[usize::from(address)])
}

fn read_access(address: u16, value: u8) -> MemoryAccess {
    MemoryAccess {
        kind: MemoryAccessKind::Read,
        address,
        value,
    }
}

fn write_access(address: u16, value: u8) -> MemoryAccess {
    MemoryAccess {
        kind: MemoryAccessKind::Write,
        address,
        value,
    }
}

/// Gate-composed eight-bit addition used by instruction families.
#[doc(hidden)]
#[must_use]
pub fn add8(left: u8, right: u8, carry: bool) -> (u8, bool) {
    crate::gates::add8(left, right, carry)
}

/// Eight-bit subtraction returning the result and CDP1802 no-borrow DF.
#[doc(hidden)]
#[must_use]
pub fn subtract8(minuend: u8, subtrahend: u8, borrow: bool) -> (u8, bool) {
    crate::gates::subtract8(minuend, subtrahend, borrow)
}
