//! Checked instruction-boundary model of the Zylin ZPU4 small core.
//!
//! The selected core is a 32-bit zero-operand stack machine. Internal RAM and
//! a deterministic external-word-bus adapter are explicit state so later SoC
//! models can replace the adapter without hidden callbacks.

mod decode;

use core::cell::Cell;
use core::fmt::{self, Write};

pub use decode::{decode, offset5, DecodedInstruction, Operation};

pub const INTERNAL_MEMORY_BYTES: usize = 32_768;
pub const EXTERNAL_WORDS: usize = 8_192;
pub const MAX_RETAINED_STEPS: usize = 64;
pub const RESET_SP: u16 = 0x7ff8;
pub const INTERRUPT_VECTOR: u16 = 0x0020;

/// Complete CPU, environment, memory, and installed-image state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZpuState {
    pub pc: u16,
    pub sp: u16,
    pub idim: bool,
    pub in_interrupt: bool,
    pub interrupt_level: bool,
    pub break_asserted: bool,
    pub internal_memory: Vec<u8>,
    pub external_bus_words: Vec<u32>,
    pub origin: Option<u16>,
    pub length: u32,
}

impl Default for ZpuState {
    fn default() -> Self {
        Self {
            pc: 0,
            sp: RESET_SP,
            idim: false,
            in_interrupt: false,
            interrupt_level: false,
            break_asserted: false,
            internal_memory: vec![0; INTERNAL_MEMORY_BYTES],
            external_bus_words: vec![0; EXTERNAL_WORDS],
            origin: None,
            length: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    Write,
}

/// One explicit internal data-memory access. Implicit stack-port traffic is
/// summarized by the before/after top words instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InternalAccess {
    pub kind: AccessKind,
    pub address: u16,
    pub value: u32,
}

/// One deterministic external adapter transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusTransaction {
    Read { address: u16, value: u32 },
    Write { address: u16, value: u32 },
}

/// One committed instruction or interrupt boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    pub sp_before: u16,
    pub sp_after: u16,
    pub raw: Option<u8>,
    pub mnemonic: String,
    pub idim_before: bool,
    pub idim_after: bool,
    pub in_interrupt_before: bool,
    pub in_interrupt_after: bool,
    pub interrupt_level: bool,
    pub break_before: bool,
    pub break_after: bool,
    pub top_before: u32,
    pub next_before: u32,
    pub top_after: u32,
    pub next_after: u32,
    pub internal_access: Option<InternalAccess>,
    pub bus_transaction: Option<BusTransaction>,
    pub state_after: ZpuState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub break_asserted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: ZpuState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZpuError {
    EmptyProgram,
    ProgramOutOfRange { origin: u16, length: usize },
    NoProgramLoaded,
    FetchOutsideProgram { pc: u16 },
    InvalidState { reason: &'static str },
    MisalignedAddress { address: u32 },
    AddressOutOfRange { address: u32 },
    StepLimit { limit: usize },
    TraceLimit { requested: usize, limit: usize },
    AllocationFailed,
}

impl fmt::Display for ZpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => f.write_str("program image is empty"),
            Self::ProgramOutOfRange { origin, length } => write!(
                f,
                "program of {length} bytes at {origin:#06x} exceeds internal RAM"
            ),
            Self::NoProgramLoaded => f.write_str("no program image is installed"),
            Self::FetchOutsideProgram { pc } => {
                write!(
                    f,
                    "opcode fetch at {pc:#06x} is outside the installed image"
                )
            }
            Self::InvalidState { reason } => write!(f, "invalid ZPU state: {reason}"),
            Self::MisalignedAddress { address } => {
                write!(f, "helper address {address:#010x} is not word aligned")
            }
            Self::AddressOutOfRange { address } => {
                write!(
                    f,
                    "helper address {address:#010x} is outside its memory region"
                )
            }
            Self::StepLimit { limit } => {
                write!(f, "step limit {limit} exhausted before BREAK")
            }
            Self::TraceLimit { requested, limit } => write!(
                f,
                "requested {requested} traces exceeds retention limit {limit}"
            ),
            Self::AllocationFailed => f.write_str("checked allocation failed"),
        }
    }
}

impl std::error::Error for ZpuError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionError {
    pub source: ZpuError,
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ZPU execution failed: {}", self.source)
    }
}

impl std::error::Error for ExecutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Exact checked machine for the `Zpu4Small32x15V1` profile.
#[derive(Debug)]
pub struct ZpuSimulator {
    state: ZpuState,
    allocation_failure_after: Cell<Option<usize>>,
    candidate_failure_after: Cell<Option<usize>>,
}

impl Default for ZpuSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl ZpuSimulator {
    /// Construct the deterministic reset machine through checked allocations.
    pub fn try_new() -> Result<Self, ZpuError> {
        Self::try_new_with_allocation_failure(None)
    }

    /// Deterministic constructor allocation seam used by contract tests.
    #[doc(hidden)]
    pub fn try_new_with_allocation_failure(checkpoints: Option<usize>) -> Result<Self, ZpuError> {
        let mut simulator = Self {
            state: ZpuState {
                pc: 0,
                sp: RESET_SP,
                idim: false,
                in_interrupt: false,
                interrupt_level: false,
                break_asserted: false,
                internal_memory: Vec::new(),
                external_bus_words: Vec::new(),
                origin: None,
                length: 0,
            },
            allocation_failure_after: Cell::new(checkpoints),
            candidate_failure_after: Cell::new(None),
        };
        simulator.state.internal_memory = simulator.zeroed_bytes(INTERNAL_MEMORY_BYTES)?;
        simulator.state.external_bus_words = simulator.zeroed_words(EXTERNAL_WORDS)?;
        simulator.allocation_failure_after.set(None);
        Ok(simulator)
    }

    #[must_use]
    pub fn new() -> Self {
        Self::try_new().expect("ZPU construction allocation failed")
    }

    pub fn reset(&mut self) {
        self.state.pc = 0;
        self.state.sp = RESET_SP;
        self.state.idim = false;
        self.state.in_interrupt = false;
        self.state.interrupt_level = false;
        self.state.break_asserted = false;
        self.state.internal_memory.fill(0);
        self.state.external_bus_words.fill(0);
        self.state.origin = None;
        self.state.length = 0;
    }

    pub fn load(&mut self, program: &[u8], origin: u16) -> Result<usize, ZpuError> {
        validate_program(program, origin)?;
        let mut internal_memory = self.zeroed_bytes(INTERNAL_MEMORY_BYTES)?;
        let external_bus_words = self.zeroed_words(EXTERNAL_WORDS)?;
        let start = usize::from(origin);
        internal_memory[start..start + program.len()].copy_from_slice(program);
        self.state = ZpuState {
            pc: origin,
            sp: RESET_SP,
            idim: false,
            in_interrupt: false,
            interrupt_level: false,
            break_asserted: false,
            internal_memory,
            external_bus_words,
            origin: Some(origin),
            length: program.len() as u32,
        };
        Ok(program.len())
    }

    pub fn snapshot(&self) -> Result<ZpuState, ZpuError> {
        self.try_clone_state(&self.state)
    }

    pub fn restore(&mut self, state: ZpuState) -> Result<(), ZpuError> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    pub fn set_interrupt_level(&mut self, asserted: bool) {
        self.state.interrupt_level = asserted;
    }

    pub fn read_internal_word(&self, address: u32) -> Result<u32, ZpuError> {
        let address = validate_helper(address, false)?;
        Ok(read_be(&self.state.internal_memory, usize::from(address)))
    }

    pub fn write_internal_word(&mut self, address: u32, value: u32) -> Result<(), ZpuError> {
        let address = validate_helper(address, false)?;
        write_be(&mut self.state.internal_memory, usize::from(address), value);
        Ok(())
    }

    pub fn read_external_word(&self, address: u32) -> Result<u32, ZpuError> {
        let address = validate_helper(address, true)?;
        Ok(self.state.external_bus_words[external_index(address)])
    }

    pub fn write_external_word(&mut self, address: u32, value: u32) -> Result<(), ZpuError> {
        let address = validate_helper(address, true)?;
        self.state.external_bus_words[external_index(address)] = value;
        Ok(())
    }

    /// Execute one atomic instruction or interrupt boundary.
    pub fn step(&mut self) -> Result<StepTrace, ZpuError> {
        validate_state(&self.state)?;
        let raw = fetch(&self.state)?;
        let top_before = stack_read(&self.state, self.state.sp);
        let next_before = stack_read(&self.state, stack_add(self.state.sp, 1));
        let mut candidate = self.try_clone_state(&self.state)?;
        let pc_before = candidate.pc;
        let sp_before = candidate.sp;
        let idim_before = candidate.idim;
        let in_interrupt_before = candidate.in_interrupt;
        let break_before = candidate.break_asserted;
        let mut internal_access = None;
        let mut bus_transaction = None;

        let accepted_interrupt =
            candidate.interrupt_level && !candidate.in_interrupt && !candidate.idim;
        let (trace_raw, mnemonic) = if accepted_interrupt {
            candidate.sp = stack_add(candidate.sp, -1);
            let return_pc = u32::from(candidate.pc);
            let destination = candidate.sp;
            stack_write(&mut candidate, destination, return_pc);
            candidate.pc = INTERRUPT_VECTOR;
            candidate.idim = false;
            candidate.in_interrupt = true;
            (None, self.owned_mnemonic("INTERRUPT", None)?)
        } else {
            if !candidate.interrupt_level {
                candidate.in_interrupt = false;
            }
            let decoded = decode(raw);
            let mnemonic = self.operation_mnemonic(decoded)?;
            execute_operation(
                &mut candidate,
                raw,
                decoded,
                &mut internal_access,
                &mut bus_transaction,
            );
            (Some(raw), mnemonic)
        };

        let top_after = stack_read(&candidate, candidate.sp);
        let next_after = stack_read(&candidate, stack_add(candidate.sp, 1));
        let state_after = self.try_clone_state(&candidate)?;
        let trace = StepTrace {
            pc_before,
            pc_after: candidate.pc,
            sp_before,
            sp_after: candidate.sp,
            raw: trace_raw,
            mnemonic,
            idim_before,
            idim_after: candidate.idim,
            in_interrupt_before,
            in_interrupt_after: candidate.in_interrupt,
            interrupt_level: candidate.interrupt_level,
            break_before,
            break_after: candidate.break_asserted,
            top_before,
            next_before,
            top_after,
            next_after,
            internal_access,
            bus_transaction,
            state_after,
        };
        self.state = candidate;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, ZpuError> {
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
            .map_err(|_| ZpuError::AllocationFailed)?;
        if candidate.state.break_asserted {
            return Ok(ExecutionResult {
                break_asserted: true,
                steps: 0,
                traces,
                final_state: candidate.snapshot()?,
            });
        }
        for _ in 0..max_steps {
            let trace = candidate.step()?;
            candidate.candidate_checkpoint()?;
            traces.push(trace);
            if candidate.state.break_asserted {
                let final_state = candidate.snapshot()?;
                self.state = candidate.state;
                return Ok(ExecutionResult {
                    break_asserted: true,
                    steps: traces.len(),
                    traces,
                    final_state,
                });
            }
        }
        Err(ZpuError::StepLimit { limit: max_steps })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u16,
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

    /// Deterministic test seam attached to real allocation checkpoints.
    pub fn fail_allocation_after(&self, checkpoints: Option<usize>) {
        self.allocation_failure_after.set(checkpoints);
    }

    /// Deterministic test seam after retained candidate instructions.
    pub fn fail_candidate_after(&self, checkpoints: Option<usize>) {
        self.candidate_failure_after.set(checkpoints);
    }

    fn allocation_checkpoint(&self) -> Result<(), ZpuError> {
        checkpoint(&self.allocation_failure_after)
    }

    fn candidate_checkpoint(&self) -> Result<(), ZpuError> {
        checkpoint(&self.candidate_failure_after)
    }

    fn zeroed_bytes(&self, count: usize) -> Result<Vec<u8>, ZpuError> {
        self.allocation_checkpoint()?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| ZpuError::AllocationFailed)?;
        values.resize(count, 0);
        Ok(values)
    }

    fn zeroed_words(&self, count: usize) -> Result<Vec<u32>, ZpuError> {
        self.allocation_checkpoint()?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| ZpuError::AllocationFailed)?;
        values.resize(count, 0);
        Ok(values)
    }

    fn try_clone_state(&self, state: &ZpuState) -> Result<ZpuState, ZpuError> {
        self.allocation_checkpoint()?;
        let mut internal_memory = Vec::new();
        internal_memory
            .try_reserve_exact(state.internal_memory.len())
            .map_err(|_| ZpuError::AllocationFailed)?;
        internal_memory.extend_from_slice(&state.internal_memory);
        self.allocation_checkpoint()?;
        let mut external_bus_words = Vec::new();
        external_bus_words
            .try_reserve_exact(state.external_bus_words.len())
            .map_err(|_| ZpuError::AllocationFailed)?;
        external_bus_words.extend_from_slice(&state.external_bus_words);
        Ok(ZpuState {
            pc: state.pc,
            sp: state.sp,
            idim: state.idim,
            in_interrupt: state.in_interrupt,
            interrupt_level: state.interrupt_level,
            break_asserted: state.break_asserted,
            internal_memory,
            external_bus_words,
            origin: state.origin,
            length: state.length,
        })
    }

    fn owned_mnemonic(&self, base: &str, parameter: Option<u8>) -> Result<String, ZpuError> {
        self.allocation_checkpoint()?;
        let mut value = String::new();
        value
            .try_reserve_exact(base.len() + 4)
            .map_err(|_| ZpuError::AllocationFailed)?;
        value.push_str(base);
        if let Some(parameter) = parameter {
            write!(&mut value, " {parameter}").map_err(|_| ZpuError::AllocationFailed)?;
        }
        Ok(value)
    }

    fn operation_mnemonic(&self, decoded: DecodedInstruction) -> Result<String, ZpuError> {
        use Operation::*;
        let (base, parameter) = match decoded.operation {
            Im => ("IM", None),
            LoadSp => ("LOADSP", Some(decoded.parameter)),
            StoreSp => ("STORESP", Some(decoded.parameter)),
            Emulate => ("EMULATE", Some(decoded.parameter)),
            AddSp => ("ADDSP", Some(decoded.parameter)),
            Break => ("BREAK", None),
            PushSp => ("PUSHSP", None),
            PopPc => ("POPPC", None),
            Add => ("ADD", None),
            And => ("AND", None),
            Or => ("OR", None),
            Load => ("LOAD", None),
            Not => ("NOT", None),
            Flip => ("FLIP", None),
            Nop => ("NOP", None),
            Store => ("STORE", None),
            PopSp => ("POPSP", None),
        };
        self.owned_mnemonic(base, parameter)
    }
}

fn execute_operation(
    state: &mut ZpuState,
    raw: u8,
    decoded: DecodedInstruction,
    internal_access: &mut Option<InternalAccess>,
    bus_transaction: &mut Option<BusTransaction>,
) {
    use Operation::*;
    let pc = state.pc;
    let sp = state.sp;
    let top = stack_read(state, sp);
    let next = stack_read(state, stack_add(sp, 1));
    state.pc = pc.wrapping_add(1) & 0x7fff;
    if decoded.operation != Im {
        state.idim = false;
    }
    match decoded.operation {
        Im => {
            if state.idim {
                stack_write(state, sp, top.wrapping_shl(7) | u32::from(raw & 0x7f));
            } else {
                state.sp = stack_add(sp, -1);
                let value = ((i32::from(raw & 0x7f) << 25) >> 25) as u32;
                let destination = state.sp;
                stack_write(state, destination, value);
            }
            state.idim = true;
        }
        LoadSp => {
            let address = stack_add(sp, i16::from(decoded.parameter));
            let value = stack_read(state, address);
            state.sp = stack_add(sp, -1);
            let destination = state.sp;
            stack_write(state, destination, value);
            *internal_access = Some(InternalAccess {
                kind: AccessKind::Read,
                address,
                value,
            });
        }
        StoreSp => {
            let address = stack_add(sp, i16::from(decoded.parameter));
            stack_write(state, address, top);
            state.sp = stack_add(sp, 1);
            *internal_access = Some(InternalAccess {
                kind: AccessKind::Write,
                address,
                value: top,
            });
        }
        AddSp => {
            let address = stack_add(sp, i16::from(decoded.parameter));
            let operand = stack_read(state, address);
            let value = top.wrapping_add(operand);
            stack_write(state, sp, value);
            *internal_access = Some(InternalAccess {
                kind: AccessKind::Read,
                address,
                value: operand,
            });
        }
        Emulate => {
            state.sp = stack_add(sp, -1);
            let destination = state.sp;
            stack_write(state, destination, u32::from(pc.wrapping_add(1) & 0x7fff));
            state.pc = u16::from(decoded.parameter) << 5;
        }
        Break => state.break_asserted = true,
        PushSp => {
            state.sp = stack_add(sp, -1);
            let destination = state.sp;
            stack_write(state, destination, u32::from(sp));
        }
        PopPc => {
            state.pc = (top as u16) & 0x7fff;
            state.sp = stack_add(sp, 1);
        }
        Add | And | Or => {
            let value = match decoded.operation {
                Add => next.wrapping_add(top),
                And => next & top,
                Or => next | top,
                _ => unreachable!(),
            };
            state.sp = stack_add(sp, 1);
            let destination = state.sp;
            stack_write(state, destination, value);
        }
        Load => {
            let address = (top as u16) & 0xfffc;
            let value = if address & 0x8000 == 0 {
                let value = read_be(&state.internal_memory, usize::from(address));
                *internal_access = Some(InternalAccess {
                    kind: AccessKind::Read,
                    address,
                    value,
                });
                value
            } else {
                let value = state.external_bus_words[external_index(address)];
                *bus_transaction = Some(BusTransaction::Read { address, value });
                value
            };
            stack_write(state, sp, value);
        }
        Not => stack_write(state, sp, !top),
        Flip => stack_write(state, sp, top.reverse_bits()),
        Nop => {}
        Store => {
            let address = (top as u16) & 0xfffc;
            if address & 0x8000 == 0 {
                write_be(&mut state.internal_memory, usize::from(address), next);
                *internal_access = Some(InternalAccess {
                    kind: AccessKind::Write,
                    address,
                    value: next,
                });
            } else {
                state.external_bus_words[external_index(address)] = next;
                *bus_transaction = Some(BusTransaction::Write {
                    address,
                    value: next,
                });
            }
            state.sp = stack_add(sp, 2);
        }
        PopSp => state.sp = (top as u16) & 0x7ffc,
    }
}

fn validate_program(program: &[u8], origin: u16) -> Result<(), ZpuError> {
    if program.is_empty() {
        return Err(ZpuError::EmptyProgram);
    }
    let end =
        usize::from(origin)
            .checked_add(program.len())
            .ok_or(ZpuError::ProgramOutOfRange {
                origin,
                length: program.len(),
            })?;
    if origin > 0x7fff || end > INTERNAL_MEMORY_BYTES {
        return Err(ZpuError::ProgramOutOfRange {
            origin,
            length: program.len(),
        });
    }
    Ok(())
}

fn validate_state(state: &ZpuState) -> Result<(), ZpuError> {
    if state.pc > 0x7fff {
        return Err(ZpuError::InvalidState {
            reason: "PC exceeds the 15-bit core address",
        });
    }
    if state.sp > 0x7ffc || !state.sp.is_multiple_of(4) {
        return Err(ZpuError::InvalidState {
            reason: "SP is not a 15-bit aligned word address",
        });
    }
    if state.internal_memory.len() != INTERNAL_MEMORY_BYTES {
        return Err(ZpuError::InvalidState {
            reason: "internal memory length is not 32768 bytes",
        });
    }
    if state.external_bus_words.len() != EXTERNAL_WORDS {
        return Err(ZpuError::InvalidState {
            reason: "external adapter length is not 8192 words",
        });
    }
    match state.origin {
        None if state.length == 0 => {}
        Some(origin) if state.length != 0 => {
            let end = usize::from(origin)
                .checked_add(state.length as usize)
                .ok_or(ZpuError::InvalidState {
                    reason: "installed image range overflows",
                })?;
            if origin > 0x7fff || end > INTERNAL_MEMORY_BYTES {
                return Err(ZpuError::InvalidState {
                    reason: "installed image lies outside internal memory",
                });
            }
        }
        _ => {
            return Err(ZpuError::InvalidState {
                reason: "installed image metadata is not canonical",
            });
        }
    }
    Ok(())
}

fn validate_budget(max_steps: usize) -> Result<(), ZpuError> {
    if max_steps > MAX_RETAINED_STEPS {
        return Err(ZpuError::TraceLimit {
            requested: max_steps,
            limit: MAX_RETAINED_STEPS,
        });
    }
    if max_steps == 0 {
        return Err(ZpuError::StepLimit { limit: 0 });
    }
    Ok(())
}

fn fetch(state: &ZpuState) -> Result<u8, ZpuError> {
    let origin = state.origin.ok_or(ZpuError::NoProgramLoaded)?;
    let end = u32::from(origin) + state.length;
    let pc = u32::from(state.pc);
    if pc < u32::from(origin) || pc >= end {
        return Err(ZpuError::FetchOutsideProgram { pc: state.pc });
    }
    Ok(state.internal_memory[usize::from(state.pc)])
}

fn validate_helper(address: u32, external: bool) -> Result<u16, ZpuError> {
    if !address.is_multiple_of(4) {
        return Err(ZpuError::MisalignedAddress { address });
    }
    let in_range = if external {
        (0x8000..=0xfffc).contains(&address)
    } else {
        address <= 0x7ffc
    };
    if !in_range {
        return Err(ZpuError::AddressOutOfRange { address });
    }
    Ok(address as u16)
}

fn stack_add(sp: u16, words: i16) -> u16 {
    let words = i32::from(sp >> 2) + i32::from(words);
    (((words & 0x1fff) as u16) << 2) & 0x7ffc
}

fn stack_read(state: &ZpuState, address: u16) -> u32 {
    read_be(&state.internal_memory, usize::from(address & 0x7ffc))
}

fn stack_write(state: &mut ZpuState, address: u16, value: u32) {
    write_be(
        &mut state.internal_memory,
        usize::from(address & 0x7ffc),
        value,
    );
}

fn read_be(memory: &[u8], address: usize) -> u32 {
    u32::from_be_bytes(memory[address..address + 4].try_into().expect("word range"))
}

fn write_be(memory: &mut [u8], address: usize, value: u32) {
    memory[address..address + 4].copy_from_slice(&value.to_be_bytes());
}

fn external_index(address: u16) -> usize {
    usize::from((address - 0x8000) >> 2)
}

fn checkpoint(cell: &Cell<Option<usize>>) -> Result<(), ZpuError> {
    if let Some(remaining) = cell.get() {
        if remaining == 0 {
            return Err(ZpuError::AllocationFailed);
        }
        cell.set(Some(remaining - 1));
    }
    Ok(())
}
