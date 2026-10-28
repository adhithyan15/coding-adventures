//! Checked functional model of the LatticeMico32 base integer processor.
//!
//! This crate models the fixed `Lm32BaseFunctionalV1` profile from Layer 07bd.
//! It owns all architectural state and returns explicit instruction/data bus,
//! interrupt, and exception transactions suitable for later SoC composition.

mod decode;

use core::cell::Cell;
use core::fmt;

pub use decode::{decode, opcode_class, DecodeError, DecodedInstruction, Operation};

/// Bytes in the owned address space.
pub const MEMORY_SIZE: usize = 65_536;
/// Maximum complete-state traces retained by one bounded run.
pub const MAX_RETAINED_STEPS: usize = 64;
/// Fixed configuration register for `Lm32BaseFunctionalV1`.
pub const CFG: u32 = 0x0802_010f;
/// Fixed extended configuration register.
pub const CFG2: u32 = 0;

pub const EXCEPTION_BREAKPOINT: u8 = 1;
pub const EXCEPTION_INSTRUCTION_BUS: u8 = 2;
pub const EXCEPTION_DATA_BUS: u8 = 4;
pub const EXCEPTION_DIVIDE_BY_ZERO: u8 = 5;
pub const EXCEPTION_INTERRUPT: u8 = 6;
pub const EXCEPTION_SYSTEM_CALL: u8 = 7;

/// Complete, owned, restorable architecture and environment state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lm32State {
    pub gpr: [u32; 32],
    pub pc: u32,
    pub ie: bool,
    pub eie: bool,
    pub bie: bool,
    pub dc_re: bool,
    pub im: u32,
    pub ip: u32,
    pub interrupt_lines: u32,
    pub eba: u32,
    pub deba: u32,
    pub memory: Vec<u8>,
    pub origin: Option<u32>,
    pub length: u32,
}

impl Default for Lm32State {
    fn default() -> Self {
        Self {
            gpr: [0; 32],
            pc: 0,
            ie: false,
            eie: false,
            bie: false,
            dc_re: false,
            im: 0,
            ip: 0,
            interrupt_lines: 0,
            eba: 0,
            deba: 0,
            memory: vec![0; MEMORY_SIZE],
            origin: None,
            length: 0,
        }
    }
}

/// Bus transaction category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusTransactionKind {
    InstructionRead,
    DataRead,
    DataWrite,
}

/// One ordered instruction or data movement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BusTransaction {
    pub kind: BusTransactionKind,
    pub address: u32,
    pub width: u8,
    pub value: u32,
    pub byte_enable: u8,
    pub success: bool,
}

/// Interrupt sampling performed at an instruction boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterruptSample {
    pub asserted: u32,
    pub newly_latched: u32,
    pub masked_pending: u32,
    pub accepted: u32,
}

/// One architectural exception transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExceptionTransition {
    pub id: u8,
    pub saved_pc_register: u8,
    pub saved_pc: u32,
    pub dc_re: bool,
    pub base: u32,
    pub vector: u32,
    pub ie_before: bool,
    pub saved_ie: bool,
}

/// One committed instruction-boundary transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lm32StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: Option<[u8; 4]>,
    pub raw_word: Option<u32>,
    pub mnemonic: &'static str,
    pub source_indices: [Option<u8>; 2],
    pub source_values: [Option<u32>; 2],
    pub destination: Option<(u8, u32)>,
    pub branch: Option<(bool, u32)>,
    pub transactions: Vec<BusTransaction>,
    pub interrupt: Option<InterruptSample>,
    pub exception: Option<ExceptionTransition>,
    pub state_after: Lm32State,
}

/// Why a successful bounded run stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Exception(u8),
}

/// Successful bounded execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub reason: StopReason,
    pub steps: usize,
    pub traces: Vec<Lm32StepTrace>,
    pub final_state: Lm32State,
}

/// Stable checked transport, lifecycle, and execution failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lm32Error {
    EmptyProgram,
    UnalignedProgram { origin: u32, length: usize },
    ProgramTooLarge { length: usize },
    ProgramOutOfRange { origin: u32, length: usize },
    NoProgramLoaded,
    MisalignedFetch { pc: u32 },
    FetchOutsideProgram { pc: u32 },
    MisalignedData { address: u32, width: u8 },
    MisalignedControlFlow { target: u32 },
    UnknownInstruction { word: u32, pc: u32 },
    UnsupportedInstruction { word: u32, pc: u32 },
    UnsupportedCsr { index: u8, write: bool },
    InvalidRegister { register: u8 },
    InvalidState { reason: &'static str },
    MemoryOutOfRange { address: u32, width: u8 },
    RetentionLimitExceeded { requested: usize, limit: usize },
    ZeroStepBudget,
    StepLimitExceeded { limit: usize },
    AllocationFailed,
}

impl fmt::Display for Lm32Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => f.write_str("program image is empty"),
            Self::UnalignedProgram { origin, length } => write!(
                f,
                "program at {origin:#010x} with {length} bytes is not word aligned"
            ),
            Self::ProgramTooLarge { length } => write!(
                f,
                "program contains {length} bytes; maximum is {MEMORY_SIZE}"
            ),
            Self::ProgramOutOfRange { origin, length } => write!(
                f,
                "program of {length} bytes does not fit at {origin:#010x}"
            ),
            Self::NoProgramLoaded => f.write_str("no program image is installed"),
            Self::MisalignedFetch { pc } => write!(f, "fetch at {pc:#010x} is not word aligned"),
            Self::FetchOutsideProgram { pc } => {
                write!(f, "fetch at {pc:#010x} is outside the installed image")
            }
            Self::MisalignedData { address, width } => {
                write!(f, "{width}-byte access at {address:#010x} is misaligned")
            }
            Self::MisalignedControlFlow { target } => {
                write!(f, "control-flow target {target:#010x} is not word aligned")
            }
            Self::UnknownInstruction { word, pc } => {
                write!(f, "unknown instruction {word:#010x} at {pc:#010x}")
            }
            Self::UnsupportedInstruction { word, pc } => {
                write!(f, "unsupported instruction {word:#010x} at {pc:#010x}")
            }
            Self::UnsupportedCsr { index, write } => write!(
                f,
                "unsupported CSR {} at index {index:#04x}",
                if *write { "write" } else { "read" }
            ),
            Self::InvalidRegister { register } => write!(f, "register r{register} does not exist"),
            Self::InvalidState { reason } => write!(f, "invalid LM32 state: {reason}"),
            Self::MemoryOutOfRange { address, width } => write!(
                f,
                "{width}-byte memory helper access at {address:#010x} is out of range"
            ),
            Self::RetentionLimitExceeded { requested, limit } => {
                write!(f, "requested {requested} traces exceeds limit {limit}")
            }
            Self::ZeroStepBudget => f.write_str("step budget must be nonzero"),
            Self::StepLimitExceeded { limit } => {
                write!(f, "step limit {limit} exhausted before an exception")
            }
            Self::AllocationFailed => f.write_str("checked allocation failed"),
        }
    }
}

impl std::error::Error for Lm32Error {}

/// Source-preserving `execute` failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionError {
    pub source: Lm32Error,
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LM32 execution failed: {}", self.source)
    }
}
impl std::error::Error for ExecutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Checked functional simulator for the fixed profile.
#[derive(Debug)]
pub struct Lm32Simulator {
    state: Lm32State,
    allocation_failure_after: Cell<Option<usize>>,
    candidate_failure_after: Cell<Option<usize>>,
}

impl Default for Lm32Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Lm32Simulator {
    /// Construct the deterministic reset state.
    ///
    /// This convenience constructor performs the initial fixed-size memory
    /// allocation through Rust's ordinary infallible allocation API. The
    /// typed allocation contract begins once an instance exists; use the
    /// checked lifecycle and execution methods for fallible, atomic work.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Lm32State::default(),
            allocation_failure_after: Cell::new(None),
            candidate_failure_after: Cell::new(None),
        }
    }

    /// Reset processor/CSR state while preserving memory and image metadata.
    pub fn reset(&mut self) {
        self.state.gpr = [0; 32];
        self.state.pc = 0;
        self.state.ie = false;
        self.state.eie = false;
        self.state.bie = false;
        self.state.dc_re = false;
        self.state.im = 0;
        self.state.ip = 0;
        self.state.interrupt_lines = 0;
        self.state.eba = 0;
        self.state.deba = 0;
    }

    /// Atomically install a validated big-endian image.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), Lm32Error> {
        validate_program(program, origin)?;
        let mut memory = self.allocate_memory()?;
        let start = origin as usize;
        memory[start..start + program.len()].copy_from_slice(program);
        self.state = Lm32State {
            pc: origin,
            memory,
            origin: Some(origin),
            length: program.len() as u32,
            ..state_without_memory()
        };
        Ok(())
    }

    /// Fallibly copy complete state.
    pub fn snapshot(&self) -> Result<Lm32State, Lm32Error> {
        self.try_clone_state(&self.state)
    }

    /// Atomically validate and install complete state.
    pub fn restore(&mut self, state: Lm32State) -> Result<(), Lm32Error> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    pub fn read_register(&self, register: u8) -> Result<u32, Lm32Error> {
        self.state
            .gpr
            .get(register as usize)
            .copied()
            .ok_or(Lm32Error::InvalidRegister { register })
    }

    pub fn write_register(&mut self, register: u8, value: u32) -> Result<(), Lm32Error> {
        let slot = self
            .state
            .gpr
            .get_mut(register as usize)
            .ok_or(Lm32Error::InvalidRegister { register })?;
        *slot = value;
        Ok(())
    }

    pub fn read_memory(&self, address: u32, width: u8) -> Result<u32, Lm32Error> {
        validate_helper_range(address, width)?;
        Ok(read_be(&self.state.memory, address as usize, width))
    }

    pub fn write_memory(&mut self, address: u32, width: u8, value: u32) -> Result<(), Lm32Error> {
        validate_helper_range(address, width)?;
        write_be(&mut self.state.memory, address as usize, width, value);
        Ok(())
    }

    /// Set the logical asserted interrupt-line sample.
    pub fn set_interrupt_lines(&mut self, asserted_mask: u32) {
        self.state.interrupt_lines = asserted_mask;
    }

    /// Set the next instruction-fetch address without validating it.
    ///
    /// This diagnostic/control seam models an externally supplied fetch
    /// address.  It deliberately permits a misaligned value so the next
    /// checked execution boundary can report [`Lm32Error::MisalignedFetch`].
    /// Complete-state restore remains strict and rejects the same value as an
    /// invalid snapshot.
    pub fn set_fetch_address(&mut self, pc: u32) {
        self.state.pc = pc;
    }

    /// Execute one atomic boundary transition.
    pub fn step(&mut self) -> Result<Lm32StepTrace, Lm32Error> {
        validate_execution_state(&self.state)?;
        let mut candidate = self.try_clone_state(&self.state)?;
        let mut transactions = Vec::new();
        self.allocation_checkpoint()?;
        transactions
            .try_reserve_exact(2)
            .map_err(|_| Lm32Error::AllocationFailed)?;
        let partial = execute_boundary(&mut candidate, &mut transactions)?;
        self.allocation_checkpoint()?;
        let state_after = self.try_clone_state(&candidate)?;
        let trace = partial.finish(transactions, state_after);
        self.state = candidate;
        Ok(trace)
    }

    /// Execute until the first architectural exception, atomically.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Lm32Error> {
        validate_budget(max_steps)?;
        let mut candidate = Self {
            state: self.try_clone_state(&self.state)?,
            allocation_failure_after: Cell::new(self.allocation_failure_after.get()),
            candidate_failure_after: Cell::new(self.candidate_failure_after.get()),
        };
        let mut traces = Vec::new();
        self.allocation_checkpoint()?;
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| Lm32Error::AllocationFailed)?;
        for _ in 0..max_steps {
            let trace = candidate.step()?;
            candidate.candidate_checkpoint()?;
            let stopped = trace.exception.map(|value| value.id);
            traces.push(trace);
            if let Some(id) = stopped {
                let final_state = candidate.snapshot()?;
                self.state = candidate.state;
                return Ok(ExecutionResult {
                    reason: StopReason::Exception(id),
                    steps: traces.len(),
                    traces,
                    final_state,
                });
            }
        }
        Err(Lm32Error::StepLimitExceeded { limit: max_steps })
    }

    /// Atomically load and run one program.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, ExecutionError> {
        validate_program(program, origin).map_err(|source| ExecutionError { source })?;
        validate_budget(max_steps).map_err(|source| ExecutionError { source })?;
        let mut candidate = Self {
            state: self
                .try_clone_state(&self.state)
                .map_err(|source| ExecutionError { source })?,
            allocation_failure_after: Cell::new(self.allocation_failure_after.get()),
            candidate_failure_after: Cell::new(self.candidate_failure_after.get()),
        };
        candidate
            .load(program, origin)
            .map_err(|source| ExecutionError { source })?;
        let result = candidate
            .run(max_steps)
            .map_err(|source| ExecutionError { source })?;
        self.state = candidate.state;
        Ok(result)
    }

    /// Fail a future checked allocation (`Some(0)` fails the next checkpoint).
    #[doc(hidden)]
    pub fn fail_allocation_after(&self, checkpoints: Option<usize>) {
        self.allocation_failure_after.set(checkpoints);
    }

    /// Fail after a requested number of successfully evaluated candidate steps.
    #[doc(hidden)]
    pub fn fail_after_candidate_steps(&self, steps: Option<usize>) {
        self.candidate_failure_after.set(steps);
    }

    fn allocation_checkpoint(&self) -> Result<(), Lm32Error> {
        checkpoint(&self.allocation_failure_after)
    }
    fn candidate_checkpoint(&self) -> Result<(), Lm32Error> {
        checkpoint(&self.candidate_failure_after)
    }
    fn allocate_memory(&self) -> Result<Vec<u8>, Lm32Error> {
        self.allocation_checkpoint()?;
        let mut memory = Vec::new();
        memory
            .try_reserve_exact(MEMORY_SIZE)
            .map_err(|_| Lm32Error::AllocationFailed)?;
        memory.resize(MEMORY_SIZE, 0);
        Ok(memory)
    }
    fn try_clone_state(&self, source: &Lm32State) -> Result<Lm32State, Lm32Error> {
        let mut memory = self.allocate_memory()?;
        memory.copy_from_slice(&source.memory);
        Ok(Lm32State {
            gpr: source.gpr,
            pc: source.pc,
            ie: source.ie,
            eie: source.eie,
            bie: source.bie,
            dc_re: source.dc_re,
            im: source.im,
            ip: source.ip,
            interrupt_lines: source.interrupt_lines,
            eba: source.eba,
            deba: source.deba,
            memory,
            origin: source.origin,
            length: source.length,
        })
    }
}

fn checkpoint(cell: &Cell<Option<usize>>) -> Result<(), Lm32Error> {
    if let Some(remaining) = cell.get() {
        if remaining == 0 {
            cell.set(None);
            return Err(Lm32Error::AllocationFailed);
        }
        cell.set(Some(remaining - 1));
    }
    Ok(())
}

struct PartialTrace {
    pc_before: u32,
    raw: Option<[u8; 4]>,
    raw_word: Option<u32>,
    mnemonic: &'static str,
    source_indices: [Option<u8>; 2],
    source_values: [Option<u32>; 2],
    destination: Option<(u8, u32)>,
    branch: Option<(bool, u32)>,
    interrupt: Option<InterruptSample>,
    exception: Option<ExceptionTransition>,
}

impl PartialTrace {
    fn finish(self, transactions: Vec<BusTransaction>, state_after: Lm32State) -> Lm32StepTrace {
        Lm32StepTrace {
            pc_before: self.pc_before,
            pc_after: state_after.pc,
            raw: self.raw,
            raw_word: self.raw_word,
            mnemonic: self.mnemonic,
            source_indices: self.source_indices,
            source_values: self.source_values,
            destination: self.destination,
            branch: self.branch,
            transactions,
            interrupt: self.interrupt,
            exception: self.exception,
            state_after,
        }
    }
}

fn execute_boundary(
    state: &mut Lm32State,
    tx: &mut Vec<BusTransaction>,
) -> Result<PartialTrace, Lm32Error> {
    if state.origin.is_none() {
        return Err(Lm32Error::NoProgramLoaded);
    }
    let pc = state.pc;
    let previous_ip = state.ip;
    state.ip |= state.interrupt_lines;
    let pending = state.im & state.ip;
    let accepted = if state.ie { pending } else { 0 };
    let interrupt = InterruptSample {
        asserted: state.interrupt_lines,
        newly_latched: state.ip & !previous_ip,
        masked_pending: pending,
        accepted,
    };
    if accepted != 0 {
        let exception = take_exception(state, EXCEPTION_INTERRUPT, pc, false);
        return Ok(PartialTrace {
            pc_before: pc,
            raw: None,
            raw_word: None,
            mnemonic: "interrupt",
            source_indices: [None; 2],
            source_values: [None; 2],
            destination: None,
            branch: None,
            interrupt: Some(interrupt),
            exception: Some(exception),
        });
    }
    if pc & 3 != 0 {
        return Err(Lm32Error::MisalignedFetch { pc });
    }
    if pc > (MEMORY_SIZE as u32 - 4) {
        tx.push(BusTransaction {
            kind: BusTransactionKind::InstructionRead,
            address: pc,
            width: 4,
            value: 0,
            byte_enable: 0xf,
            success: false,
        });
        let exception = take_exception(state, EXCEPTION_INSTRUCTION_BUS, pc, false);
        return Ok(PartialTrace {
            pc_before: pc,
            raw: None,
            raw_word: None,
            mnemonic: "instruction_bus_error",
            source_indices: [None; 2],
            source_values: [None; 2],
            destination: None,
            branch: None,
            interrupt: Some(interrupt),
            exception: Some(exception),
        });
    }
    if !image_contains_word(state, pc) {
        return Err(Lm32Error::FetchOutsideProgram { pc });
    }
    let start = pc as usize;
    let raw = [
        state.memory[start],
        state.memory[start + 1],
        state.memory[start + 2],
        state.memory[start + 3],
    ];
    let word = u32::from_be_bytes(raw);
    tx.push(BusTransaction {
        kind: BusTransactionKind::InstructionRead,
        address: pc,
        width: 4,
        value: word,
        byte_enable: 0xf,
        success: true,
    });
    let decoded = decode(word).map_err(|error| match error {
        DecodeError::Unknown => Lm32Error::UnknownInstruction { word, pc },
        DecodeError::Unsupported => Lm32Error::UnsupportedInstruction { word, pc },
    })?;
    execute_decoded(state, decoded, pc, raw, interrupt, tx)
}

fn execute_decoded(
    state: &mut Lm32State,
    d: DecodedInstruction,
    pc: u32,
    raw: [u8; 4],
    interrupt: InterruptSample,
    tx: &mut Vec<BusTransaction>,
) -> Result<PartialTrace, Lm32Error> {
    use Operation::*;
    let a = state.gpr[d.source_0 as usize];
    let b = state.gpr[d.source_1 as usize];
    let mut result = None;
    let mut branch = None;
    let mut exception = None;
    let register_form = d.raw >> 31 != 0;
    let mut sources = if register_form {
        [Some(d.source_0), Some(d.source_1)]
    } else {
        [Some(d.source_0), None]
    };
    let mut values = if register_form {
        [Some(a), Some(b)]
    } else {
        [Some(a), None]
    };
    state.pc = pc.wrapping_add(4);
    match d.operation {
        Add => result = Some(a.wrapping_add(if d.raw >> 31 == 0 { d.immediate } else { b })),
        Sub => result = Some(a.wrapping_sub(b)),
        Mul => result = Some(a.wrapping_mul(if d.raw >> 31 == 0 { d.immediate } else { b })),
        Divu | Modu => {
            let quotient = if d.operation == Divu {
                a.checked_div(b)
            } else {
                a.checked_rem(b)
            };
            if let Some(value) = quotient {
                result = Some(value);
            } else {
                exception = Some(take_exception(state, EXCEPTION_DIVIDE_BY_ZERO, pc, false));
            }
        }
        And => result = Some(a & if d.raw >> 31 == 0 { d.immediate } else { b }),
        AndHi => result = Some(a & d.immediate),
        Or => result = Some(a | if d.raw >> 31 == 0 { d.immediate } else { b }),
        OrHi => result = Some(a | d.immediate),
        Xor => result = Some(a ^ if d.raw >> 31 == 0 { d.immediate } else { b }),
        Xnor => result = Some(!(a ^ if d.raw >> 31 == 0 { d.immediate } else { b })),
        Nor => result = Some(!(a | if d.raw >> 31 == 0 { d.immediate } else { b })),
        Sl => result = Some(a.wrapping_shl((if d.raw >> 31 == 0 { d.immediate } else { b }) & 31)),
        Sr => {
            result =
                Some(((a as i32) >> ((if d.raw >> 31 == 0 { d.immediate } else { b }) & 31)) as u32)
        }
        Sru => result = Some(a >> ((if d.raw >> 31 == 0 { d.immediate } else { b }) & 31)),
        Sextb => result = Some((a as u8 as i8 as i32) as u32),
        Sexth => result = Some((a as u16 as i16 as i32) as u32),
        Cmpe | Cmpne | Cmpg | Cmpge | Cmpgu | Cmpgeu => {
            let rhs = if d.raw >> 31 == 0 { d.immediate } else { b };
            let yes = match d.operation {
                Cmpe => a == rhs,
                Cmpne => a != rhs,
                Cmpg => (a as i32) > (rhs as i32),
                Cmpge => (a as i32) >= (rhs as i32),
                Cmpgu => a > rhs,
                Cmpgeu => a >= rhs,
                _ => unreachable!(),
            };
            result = Some(u32::from(yes));
        }
        Lb | Lbu | Lh | Lhu | Lw => {
            let width = if d.operation == Lw {
                4
            } else if matches!(d.operation, Lh | Lhu) {
                2
            } else {
                1
            };
            let address = a.wrapping_add(d.immediate);
            match data_read(state, address, width, tx)? {
                Some(value) => {
                    result = Some(match d.operation {
                        Lb => (value as u8 as i8 as i32) as u32,
                        Lh => (value as u16 as i16 as i32) as u32,
                        _ => value,
                    })
                }
                None => exception = Some(take_exception(state, EXCEPTION_DATA_BUS, pc, false)),
            }
        }
        Sb | Sh | Sw => {
            let width = if d.operation == Sw {
                4
            } else if d.operation == Sh {
                2
            } else {
                1
            };
            let address = a.wrapping_add(d.immediate);
            if !data_write(state, address, width, b, tx)? {
                exception = Some(take_exception(state, EXCEPTION_DATA_BUS, pc, false));
            }
            sources = [Some(d.source_0), Some(d.source_1)];
            values = [Some(a), Some(b)];
        }
        Be | Bne | Bg | Bge | Bgu | Bgeu => {
            sources = [Some(d.source_0), Some(d.source_1)];
            values = [Some(a), Some(b)];
            let taken = match d.operation {
                Be => a == b,
                Bne => a != b,
                Bg => (a as i32) > (b as i32),
                Bge => (a as i32) >= (b as i32),
                Bgu => a > b,
                Bgeu => a >= b,
                _ => unreachable!(),
            };
            let target = pc.wrapping_add(d.immediate.wrapping_shl(2));
            if taken {
                set_control_target(state, target)?;
            }
            branch = Some((taken, target));
        }
        Bi | Calli => {
            let target = pc.wrapping_add(d.immediate.wrapping_shl(2));
            if d.operation == Calli {
                state.gpr[29] = pc.wrapping_add(4);
                result = Some(state.gpr[29]);
            }
            set_control_target(state, target)?;
            branch = Some((true, target));
            sources = [None; 2];
            values = [None; 2];
        }
        Branch | Call => {
            let target = a;
            if d.operation == Call {
                state.gpr[29] = pc.wrapping_add(4);
                result = Some(state.gpr[29]);
            }
            if d.operation == Branch && d.source_0 == 30 {
                state.ie = state.eie;
            }
            if d.operation == Branch && d.source_0 == 31 {
                state.ie = state.bie;
            }
            set_control_target(state, target)?;
            branch = Some((true, target));
            sources = [Some(d.source_0), None];
            values = [Some(a), None];
        }
        Rcsr => {
            result = Some(read_csr(state, d.source_0)?);
            sources = [None; 2];
            values = [None; 2];
        }
        Wcsr => {
            write_csr(state, d.source_0, b)?;
            result = None;
            sources = [Some(d.source_1), None];
            values = [Some(b), None];
        }
        Break => {
            exception = Some(take_exception(state, EXCEPTION_BREAKPOINT, pc, true));
            sources = [None; 2];
            values = [None; 2];
        }
        Scall => {
            exception = Some(take_exception(state, EXCEPTION_SYSTEM_CALL, pc, false));
            sources = [None; 2];
            values = [None; 2];
        }
    }
    let destination = if let Some(value) = result {
        let register = if matches!(d.operation, Call | Calli) {
            29
        } else {
            d.destination
        };
        state.gpr[register as usize] = value;
        Some((register, value))
    } else {
        None
    };
    Ok(PartialTrace {
        pc_before: pc,
        raw: Some(raw),
        raw_word: Some(d.raw),
        mnemonic: d.mnemonic,
        source_indices: sources,
        source_values: values,
        destination,
        branch,
        interrupt: Some(interrupt),
        exception,
    })
}

fn data_read(
    state: &Lm32State,
    address: u32,
    width: u8,
    tx: &mut Vec<BusTransaction>,
) -> Result<Option<u32>, Lm32Error> {
    validate_alignment(address, width)?;
    if !range_fits(address, width) {
        tx.push(BusTransaction {
            kind: BusTransactionKind::DataRead,
            address,
            width,
            value: 0,
            byte_enable: byte_enable(address, width),
            success: false,
        });
        return Ok(None);
    }
    let value = read_be(&state.memory, address as usize, width);
    tx.push(BusTransaction {
        kind: BusTransactionKind::DataRead,
        address,
        width,
        value,
        byte_enable: byte_enable(address, width),
        success: true,
    });
    Ok(Some(value))
}

fn data_write(
    state: &mut Lm32State,
    address: u32,
    width: u8,
    value: u32,
    tx: &mut Vec<BusTransaction>,
) -> Result<bool, Lm32Error> {
    validate_alignment(address, width)?;
    // The pinned RTL presents narrow stores as replicated values on d_dat_o;
    // byte-enable selects the addressed lane(s). Keep the trace at that bus
    // boundary rather than exposing the unmodified register source.
    let bus_value = match width {
        1 => (value & 0xff).wrapping_mul(0x0101_0101),
        2 => {
            let half = value & 0xffff;
            half | (half << 16)
        }
        _ => value,
    };
    if !range_fits(address, width) {
        tx.push(BusTransaction {
            kind: BusTransactionKind::DataWrite,
            address,
            width,
            value: bus_value,
            byte_enable: byte_enable(address, width),
            success: false,
        });
        return Ok(false);
    }
    write_be(&mut state.memory, address as usize, width, value);
    tx.push(BusTransaction {
        kind: BusTransactionKind::DataWrite,
        address,
        width,
        value: bus_value,
        byte_enable: byte_enable(address, width),
        success: true,
    });
    Ok(true)
}

fn take_exception(state: &mut Lm32State, id: u8, pc: u32, debug: bool) -> ExceptionTransition {
    let ie_before = state.ie;
    let (register, base, saved_ie) = if debug {
        state.gpr[31] = pc;
        state.bie = state.ie;
        (31, state.deba, state.bie)
    } else {
        state.gpr[30] = pc;
        state.eie = state.ie;
        let base = if state.dc_re { state.deba } else { state.eba };
        (30, base, state.eie)
    };
    state.ie = false;
    let vector = base.wrapping_add(u32::from(id) * 32);
    state.pc = vector;
    ExceptionTransition {
        id,
        saved_pc_register: register,
        saved_pc: pc,
        dc_re: state.dc_re,
        base,
        vector,
        ie_before,
        saved_ie,
    }
}

fn read_csr(state: &Lm32State, index: u8) -> Result<u32, Lm32Error> {
    match index {
        0 => Ok(u32::from(state.ie) | (u32::from(state.eie) << 1) | (u32::from(state.bie) << 2)),
        1 => Ok(state.im),
        2 => Ok(state.ip),
        6 => Ok(CFG),
        7 => Ok(state.eba),
        9 => Ok(state.deba),
        10 => Ok(CFG2),
        _ => Err(Lm32Error::UnsupportedCsr {
            index,
            write: false,
        }),
    }
}

fn write_csr(state: &mut Lm32State, index: u8, value: u32) -> Result<(), Lm32Error> {
    match index {
        0 => {
            state.ie = value & 1 != 0;
            state.eie = value & 2 != 0;
            state.bie = value & 4 != 0;
        }
        1 => state.im = value,
        2 => state.ip &= !value,
        7 => state.eba = if value & 0xff == 0 { value } else { 0 },
        8 => state.dc_re = value & 2 != 0,
        9 => state.deba = if value & 0xff == 0 { value } else { 0 },
        _ => return Err(Lm32Error::UnsupportedCsr { index, write: true }),
    }
    Ok(())
}

fn validate_program(program: &[u8], origin: u32) -> Result<(), Lm32Error> {
    if program.is_empty() {
        return Err(Lm32Error::EmptyProgram);
    }
    if origin & 3 != 0 || program.len() & 3 != 0 {
        return Err(Lm32Error::UnalignedProgram {
            origin,
            length: program.len(),
        });
    }
    if program.len() > MEMORY_SIZE {
        return Err(Lm32Error::ProgramTooLarge {
            length: program.len(),
        });
    }
    if origin > MEMORY_SIZE as u32
        || (origin as usize)
            .checked_add(program.len())
            .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(Lm32Error::ProgramOutOfRange {
            origin,
            length: program.len(),
        });
    }
    Ok(())
}

fn validate_state(state: &Lm32State) -> Result<(), Lm32Error> {
    validate_execution_state(state)?;
    if state.pc & 3 != 0 {
        return Err(Lm32Error::InvalidState {
            reason: "PC must be word aligned",
        });
    }
    Ok(())
}

fn validate_execution_state(state: &Lm32State) -> Result<(), Lm32Error> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(Lm32Error::InvalidState {
            reason: "memory must contain exactly 65536 bytes",
        });
    }
    if state.eba & 0xff != 0 || state.deba & 0xff != 0 {
        return Err(Lm32Error::InvalidState {
            reason: "EBA and DEBA must be 256-byte aligned",
        });
    }
    match (state.origin, state.length) {
        (None, 0) => Ok(()),
        (Some(origin), length)
            if length != 0
                && origin & 3 == 0
                && length & 3 == 0
                && origin
                    .checked_add(length)
                    .is_some_and(|end| end <= MEMORY_SIZE as u32) =>
        {
            Ok(())
        }
        _ => Err(Lm32Error::InvalidState {
            reason: "image metadata must be canonical, aligned, nonempty, and in range",
        }),
    }
}

fn validate_budget(max_steps: usize) -> Result<(), Lm32Error> {
    if max_steps == 0 {
        return Err(Lm32Error::ZeroStepBudget);
    }
    if max_steps > MAX_RETAINED_STEPS {
        return Err(Lm32Error::RetentionLimitExceeded {
            requested: max_steps,
            limit: MAX_RETAINED_STEPS,
        });
    }
    Ok(())
}

fn state_without_memory() -> Lm32State {
    Lm32State {
        gpr: [0; 32],
        pc: 0,
        ie: false,
        eie: false,
        bie: false,
        dc_re: false,
        im: 0,
        ip: 0,
        interrupt_lines: 0,
        eba: 0,
        deba: 0,
        memory: Vec::new(),
        origin: None,
        length: 0,
    }
}

fn image_contains_word(state: &Lm32State, pc: u32) -> bool {
    state.origin.is_some_and(|origin| {
        pc >= origin
            && pc
                .checked_add(4)
                .is_some_and(|end| end <= origin + state.length)
    })
}

fn set_control_target(state: &mut Lm32State, target: u32) -> Result<(), Lm32Error> {
    if target & 3 != 0 {
        return Err(Lm32Error::MisalignedControlFlow { target });
    }
    state.pc = target;
    Ok(())
}

fn validate_alignment(address: u32, width: u8) -> Result<(), Lm32Error> {
    if !matches!(width, 1 | 2 | 4) || !address.is_multiple_of(u32::from(width)) {
        return Err(Lm32Error::MisalignedData { address, width });
    }
    Ok(())
}

fn validate_helper_range(address: u32, width: u8) -> Result<(), Lm32Error> {
    validate_alignment(address, width)?;
    if !range_fits(address, width) {
        return Err(Lm32Error::MemoryOutOfRange { address, width });
    }
    Ok(())
}

fn range_fits(address: u32, width: u8) -> bool {
    address
        .checked_add(u32::from(width))
        .is_some_and(|end| end <= MEMORY_SIZE as u32)
}
fn byte_enable(address: u32, width: u8) -> u8 {
    match width {
        1 => 1 << (3 - (address & 3)),
        2 => {
            if address & 2 == 0 {
                0xc
            } else {
                3
            }
        }
        4 => 0xf,
        _ => 0,
    }
}

fn read_be(memory: &[u8], start: usize, width: u8) -> u32 {
    match width {
        1 => u32::from(memory[start]),
        2 => u32::from(u16::from_be_bytes([memory[start], memory[start + 1]])),
        4 => u32::from_be_bytes([
            memory[start],
            memory[start + 1],
            memory[start + 2],
            memory[start + 3],
        ]),
        _ => unreachable!(),
    }
}

fn write_be(memory: &mut [u8], start: usize, width: u8, value: u32) {
    let bytes = value.to_be_bytes();
    match width {
        1 => memory[start] = bytes[3],
        2 => memory[start..start + 2].copy_from_slice(&bytes[2..]),
        4 => memory[start..start + 4].copy_from_slice(&bytes),
        _ => unreachable!(),
    }
}
