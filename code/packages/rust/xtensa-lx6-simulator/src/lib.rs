//! Checked ESP32-class Xtensa LX6 integer functional simulator.
//!
//! Xtensa's encoding is a tree rather than a flat opcode list.  The low
//! nibble chooses both instruction width and a major family, then `op1`,
//! `op2`, and sometimes an operand field refine that family.  This module
//! deliberately keeps one decoder and one decoded representation: exact
//! reserved-field validation, lazy register-window checks, and execution all
//! consume the same value.

use core::fmt;

#[cfg(test)]
thread_local! {
    static ALLOCATION_FAILURE_AFTER: core::cell::Cell<Option<usize>> = const {
        core::cell::Cell::new(None)
    };
}

/// Owned teaching-memory size.
pub const MEMORY_SIZE: usize = 65_536;
/// Maximum number of complete-state traces retained by one run.
pub const MAX_RETAINED_STEPS: usize = 64;

const SR_LBEG: u8 = 0;
const SR_LEND: u8 = 1;
const SR_LCOUNT: u8 = 2;
const SR_SAR: u8 = 3;

/// The immutable public configuration represented by this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XtensaProfile {
    Esp32Lx6IntegerV1,
}

/// Stable reasons for architecturally documented undefined operations.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndefinedReason {
    InvalidEntrySource,
    ShiftAmountOutOfRange,
    InvalidReturnWindow,
}

/// One committed data-memory access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataAccess {
    Read { address: u32, width: u8, value: u32 },
    Write { address: u32, width: u8, value: u32 },
}

/// Complete owned architectural state, including hidden register windows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XtensaLx6State {
    pub ar: [u32; 64],
    pub pc: u32,
    pub sar: u8,
    pub window_base: u8,
    pub window_start: u16,
    pub call_inc: u8,
    pub lbeg: u32,
    pub lend: u32,
    pub lcount: u32,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub origin: Option<u32>,
    pub length: u32,
}

impl Default for XtensaLx6State {
    fn default() -> Self {
        Self {
            ar: [0; 64],
            pc: 0,
            sar: 0,
            window_base: 0,
            window_start: 1,
            call_inc: 0,
            lbeg: 0,
            lend: 0,
            lcount: 0,
            memory: vec![0; MEMORY_SIZE],
            halted: false,
            origin: None,
            length: 0,
        }
    }
}

/// One successful instruction transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw_bytes: [u8; 3],
    pub width: u8,
    pub raw: u32,
    pub mnemonic: &'static str,
    pub data_access: Option<DataAccess>,
    pub state_after: XtensaLx6State,
}

/// A successful bounded execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: XtensaLx6State,
}

/// Typed transport, decode, architectural, and lifecycle failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum XtensaLx6Error {
    EmptyProgram,
    ProgramOutOfRange {
        origin: u32,
        length: usize,
    },
    ProgramAddressOverflow {
        origin: u32,
        length: usize,
    },
    NoProgramLoaded,
    FetchOutsideProgram {
        pc: u32,
    },
    TruncatedInstruction {
        pc: u32,
        needed: u8,
        available: u8,
    },
    FetchOutOfRange {
        pc: u32,
        width: u8,
    },
    UnknownInstruction {
        raw: u32,
        width: u8,
    },
    UnsupportedOptionInstruction {
        raw: u32,
        width: u8,
    },
    UndefinedOperation {
        raw: u32,
        width: u8,
        reason: UndefinedReason,
    },
    UnsupportedSpecialRegister {
        number: u8,
    },
    PrivilegedInstruction {
        raw: u32,
        width: u8,
    },
    Breakpoint {
        code: u8,
    },
    DataOutOfRange {
        address: u32,
        width: u8,
    },
    InvalidLoop,
    WindowOverflow {
        increment: u8,
    },
    WindowUnderflow {
        increment: u8,
    },
    AllocaCause,
    InvalidState {
        reason: &'static str,
    },
    Halted,
    TraceLimitExceeded {
        requested: usize,
        limit: usize,
    },
    StepLimitExceeded {
        limit: usize,
    },
    AllocationFailed,
}

impl fmt::Display for XtensaLx6Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => f.write_str("program image is empty"),
            Self::ProgramOutOfRange { origin, length } => write!(
                f,
                "program of {length} bytes does not fit at {origin:#010x}"
            ),
            Self::ProgramAddressOverflow { origin, length } => write!(
                f,
                "program address {origin:#010x} plus {length} bytes overflows"
            ),
            Self::NoProgramLoaded => f.write_str("no program is installed"),
            Self::FetchOutsideProgram { pc } => {
                write!(f, "fetch at {pc:#010x} is outside the installed program")
            }
            Self::TruncatedInstruction {
                pc,
                needed,
                available,
            } => write!(
                f,
                "instruction at {pc:#010x} needs {needed} bytes but only {available} remain"
            ),
            Self::FetchOutOfRange { pc, width } => write!(
                f,
                "{width}-byte fetch at {pc:#010x} is outside teaching memory"
            ),
            Self::UnknownInstruction { raw, width } => {
                write!(f, "unknown {width}-byte instruction {raw:#08x}")
            }
            Self::UnsupportedOptionInstruction { raw, width } => {
                write!(f, "unsupported-option {width}-byte instruction {raw:#08x}")
            }
            Self::UndefinedOperation { raw, width, reason } => {
                write!(f, "undefined {width}-byte operation {raw:#08x}: {reason:?}")
            }
            Self::UnsupportedSpecialRegister { number } => {
                write!(f, "special register {number} is outside this profile")
            }
            Self::PrivilegedInstruction { raw, width } => {
                write!(f, "privileged {width}-byte instruction {raw:#08x}")
            }
            Self::Breakpoint { code } => write!(f, "breakpoint code {code:#04x}"),
            Self::DataOutOfRange { address, width } => write!(
                f,
                "{width}-byte data access at {address:#010x} is out of range"
            ),
            Self::InvalidLoop => {
                f.write_str("invalid zero-overhead loop extent or first instruction")
            }
            Self::WindowOverflow { increment } => {
                write!(f, "register-window overflow for increment {increment}")
            }
            Self::WindowUnderflow { increment } => {
                write!(f, "register-window underflow for increment {increment}")
            }
            Self::AllocaCause => f.write_str("MOVSP requires caller-window recovery"),
            Self::InvalidState { reason } => write!(f, "invalid Xtensa state: {reason}"),
            Self::Halted => f.write_str("processor is halted"),
            Self::TraceLimitExceeded { requested, limit } => {
                write!(f, "requested {requested} traces exceeds limit {limit}")
            }
            Self::StepLimitExceeded { limit } => {
                write!(f, "step limit {limit} exhausted before halt")
            }
            Self::AllocationFailed => f.write_str("state or trace allocation failed"),
        }
    }
}

impl std::error::Error for XtensaLx6Error {}

/// Checked simulator for the `Esp32Lx6IntegerV1` profile.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct XtensaLx6Simulator {
    state: XtensaLx6State,
}

impl XtensaLx6Simulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn profile(&self) -> XtensaProfile {
        XtensaProfile::Esp32Lx6IntegerV1
    }

    pub fn reset(&mut self) {
        self.state = XtensaLx6State::default();
    }

    pub fn snapshot(&self) -> Result<XtensaLx6State, XtensaLx6Error> {
        try_clone_state(&self.state)
    }

    pub fn restore(&mut self, state: XtensaLx6State) -> Result<(), XtensaLx6Error> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), XtensaLx6Error> {
        validate_program(program, origin)?;
        let mut next = XtensaLx6State {
            pc: origin,
            origin: Some(origin),
            length: program.len() as u32,
            ..XtensaLx6State::default()
        };
        let start = origin as usize;
        next.memory[start..start + program.len()].copy_from_slice(program);
        self.state = next;
        Ok(())
    }

    pub fn logical_register(&self, register: u8) -> Result<u32, XtensaLx6Error> {
        if register >= 16 {
            return Err(XtensaLx6Error::InvalidState {
                reason: "logical register index exceeds 15",
            });
        }
        Ok(read_ar(&self.state, register))
    }

    pub fn set_logical_register(&mut self, register: u8, value: u32) -> Result<(), XtensaLx6Error> {
        if register >= 16 {
            return Err(XtensaLx6Error::InvalidState {
                reason: "logical register index exceeds 15",
            });
        }
        write_ar(&mut self.state, register, value);
        Ok(())
    }

    pub fn read_memory(&self, address: u32, width: u8) -> Result<u32, XtensaLx6Error> {
        read_memory(&self.state, address, width)
    }

    pub fn write_memory(
        &mut self,
        address: u32,
        width: u8,
        value: u32,
    ) -> Result<(), XtensaLx6Error> {
        write_memory(&mut self.state, address, width, value)
    }

    pub fn step(&mut self) -> Result<StepTrace, XtensaLx6Error> {
        if self.state.halted {
            return Err(XtensaLx6Error::Halted);
        }
        let before = try_clone_state(&self.state)?;
        let fetched = fetch(&before)?;
        let decoded = decode(fetched.raw, fetched.width)?;
        window_check_for(&before, &decoded)?;
        let mut next = try_clone_state(&before)?;
        let sequential = before.pc.wrapping_add(u32::from(fetched.width));
        next.pc = sequential;
        let mut data_access = None;
        let control_taken = execute_decoded(
            &before,
            &mut next,
            &decoded,
            fetched.raw,
            fetched.width,
            &mut data_access,
        )?;
        if !control_taken
            && next.pc == sequential
            && sequential == before.lend
            && before.lcount != 0
        {
            next.pc = before.lbeg;
            next.lcount = before.lcount.wrapping_sub(1);
        }
        let trace = StepTrace {
            pc_before: before.pc,
            pc_after: next.pc,
            raw_bytes: fetched.bytes,
            width: fetched.width,
            raw: fetched.raw,
            mnemonic: decoded.mnemonic(),
            data_access,
            state_after: try_clone_state(&next)?,
        };
        self.state = next;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, XtensaLx6Error> {
        check_retention(max_steps)?;
        let entry = try_clone_state(&self.state)?;
        let mut traces = Vec::new();
        allocation_checkpoint()?;
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| XtensaLx6Error::AllocationFailed)?;
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.state = entry;
                    return Err(error);
                }
            }
            if self.state.halted {
                let final_state = match try_clone_state(&self.state) {
                    Ok(state) => state,
                    Err(error) => {
                        self.state = entry;
                        return Err(error);
                    }
                };
                return Ok(ExecutionResult {
                    halted: true,
                    steps: traces.len(),
                    traces,
                    final_state,
                });
            }
        }
        self.state = entry;
        Err(XtensaLx6Error::StepLimitExceeded { limit: max_steps })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, XtensaLx6Error> {
        check_retention(max_steps)?;
        validate_program(program, origin)?;
        let entry = try_clone_state(&self.state)?;
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = entry;
                Err(error)
            }
        }
    }
}

fn try_clone_state(state: &XtensaLx6State) -> Result<XtensaLx6State, XtensaLx6Error> {
    let mut memory = Vec::new();
    allocation_checkpoint()?;
    memory
        .try_reserve_exact(state.memory.len())
        .map_err(|_| XtensaLx6Error::AllocationFailed)?;
    memory.extend_from_slice(&state.memory);
    Ok(XtensaLx6State {
        ar: state.ar,
        pc: state.pc,
        sar: state.sar,
        window_base: state.window_base,
        window_start: state.window_start,
        call_inc: state.call_inc,
        lbeg: state.lbeg,
        lend: state.lend,
        lcount: state.lcount,
        memory,
        halted: state.halted,
        origin: state.origin,
        length: state.length,
    })
}

fn allocation_checkpoint() -> Result<(), XtensaLx6Error> {
    #[cfg(test)]
    ALLOCATION_FAILURE_AFTER.with(|remaining| {
        if let Some(count) = remaining.get() {
            if count == 0 {
                remaining.set(None);
                return Err(XtensaLx6Error::AllocationFailed);
            }
            remaining.set(Some(count - 1));
        }
        Ok(())
    })?;
    Ok(())
}

#[cfg(test)]
fn fail_allocation_after(successful_allocations: usize) {
    ALLOCATION_FAILURE_AFTER.with(|remaining| remaining.set(Some(successful_allocations)));
}

fn validate_program(program: &[u8], origin: u32) -> Result<(), XtensaLx6Error> {
    if program.is_empty() {
        return Err(XtensaLx6Error::EmptyProgram);
    }
    let length = u32::try_from(program.len()).map_err(|_| XtensaLx6Error::ProgramOutOfRange {
        origin,
        length: program.len(),
    })?;
    let end = origin
        .checked_add(length)
        .ok_or(XtensaLx6Error::ProgramAddressOverflow {
            origin,
            length: program.len(),
        })?;
    if end as usize > MEMORY_SIZE {
        return Err(XtensaLx6Error::ProgramOutOfRange {
            origin,
            length: program.len(),
        });
    }
    Ok(())
}

fn validate_state(state: &XtensaLx6State) -> Result<(), XtensaLx6Error> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(XtensaLx6Error::InvalidState {
            reason: "memory length must be exactly 65536 bytes",
        });
    }
    if state.sar > 63 {
        return Err(XtensaLx6Error::InvalidState {
            reason: "SAR exceeds six bits",
        });
    }
    if state.window_base > 15 {
        return Err(XtensaLx6Error::InvalidState {
            reason: "WindowBase exceeds four bits",
        });
    }
    if state.call_inc > 3 {
        return Err(XtensaLx6Error::InvalidState {
            reason: "CALLINC exceeds two bits",
        });
    }
    if state.window_start & (1u16 << state.window_base) == 0 {
        return Err(XtensaLx6Error::InvalidState {
            reason: "current window is not active",
        });
    }
    match state.origin {
        None if state.length == 0 && !state.halted => Ok(()),
        None => Err(XtensaLx6Error::InvalidState {
            reason: "halt or length requires an installed image",
        }),
        Some(_) if state.length == 0 => Err(XtensaLx6Error::InvalidState {
            reason: "installed image cannot be empty",
        }),
        Some(origin) => {
            let end = origin
                .checked_add(state.length)
                .ok_or(XtensaLx6Error::InvalidState {
                    reason: "installed range overflows",
                })?;
            if end as usize > MEMORY_SIZE {
                Err(XtensaLx6Error::InvalidState {
                    reason: "installed range exceeds memory",
                })
            } else {
                Ok(())
            }
        }
    }
}

fn check_retention(max_steps: usize) -> Result<(), XtensaLx6Error> {
    if max_steps > MAX_RETAINED_STEPS {
        Err(XtensaLx6Error::TraceLimitExceeded {
            requested: max_steps,
            limit: MAX_RETAINED_STEPS,
        })
    } else {
        Ok(())
    }
}

fn physical_index(state: &XtensaLx6State, logical: u8) -> usize {
    ((usize::from(state.window_base) * 4) + usize::from(logical)) & 63
}

fn read_ar(state: &XtensaLx6State, logical: u8) -> u32 {
    state.ar[physical_index(state, logical)]
}
fn write_ar(state: &mut XtensaLx6State, logical: u8, value: u32) {
    let index = physical_index(state, logical);
    state.ar[index] = value;
}

fn checked_range(address: u32, width: u8) -> Result<core::ops::Range<usize>, XtensaLx6Error> {
    if !matches!(width, 1 | 2 | 4) {
        return Err(XtensaLx6Error::InvalidState {
            reason: "memory width must be 1, 2, or 4",
        });
    }
    let start = address as usize;
    let end = start
        .checked_add(width as usize)
        .filter(|end| *end <= MEMORY_SIZE)
        .ok_or(XtensaLx6Error::DataOutOfRange { address, width })?;
    Ok(start..end)
}

fn read_memory(state: &XtensaLx6State, address: u32, width: u8) -> Result<u32, XtensaLx6Error> {
    let range = checked_range(address, width)?;
    let mut value = 0u32;
    for (shift, byte) in state.memory[range].iter().enumerate() {
        value |= u32::from(*byte) << (shift * 8);
    }
    Ok(value)
}

fn write_memory(
    state: &mut XtensaLx6State,
    address: u32,
    width: u8,
    value: u32,
) -> Result<(), XtensaLx6Error> {
    let range = checked_range(address, width)?;
    for (shift, byte) in state.memory[range].iter_mut().enumerate() {
        *byte = (value >> (shift * 8)) as u8;
    }
    Ok(())
}

struct Fetched {
    raw: u32,
    width: u8,
    bytes: [u8; 3],
}

fn fetch(state: &XtensaLx6State) -> Result<Fetched, XtensaLx6Error> {
    let origin = state.origin.ok_or(XtensaLx6Error::NoProgramLoaded)?;
    let end = origin
        .checked_add(state.length)
        .ok_or(XtensaLx6Error::InvalidState {
            reason: "installed range overflows",
        })?;
    if state.pc < origin || state.pc >= end {
        return Err(XtensaLx6Error::FetchOutsideProgram { pc: state.pc });
    }
    if state.pc as usize >= MEMORY_SIZE {
        return Err(XtensaLx6Error::FetchOutOfRange {
            pc: state.pc,
            width: 2,
        });
    }
    let available = end - state.pc;
    if available < 2 {
        return Err(XtensaLx6Error::TruncatedInstruction {
            pc: state.pc,
            needed: 2,
            available: available as u8,
        });
    }
    let p = state.pc as usize;
    if p.checked_add(2).is_none_or(|end| end > MEMORY_SIZE) {
        return Err(XtensaLx6Error::FetchOutOfRange {
            pc: state.pc,
            width: 2,
        });
    }
    let mut bytes = [state.memory[p], state.memory[p + 1], 0];
    let width = if bytes[0] & 8 != 0 { 2 } else { 3 };
    if width == 3 {
        if available < 3 {
            return Err(XtensaLx6Error::TruncatedInstruction {
                pc: state.pc,
                needed: 3,
                available: available as u8,
            });
        }
        if p.checked_add(3).is_none_or(|end| end > MEMORY_SIZE) {
            return Err(XtensaLx6Error::FetchOutOfRange {
                pc: state.pc,
                width: 3,
            });
        }
        bytes[2] = state.memory[p + 2];
    }
    let raw = u32::from(bytes[0]) | (u32::from(bytes[1]) << 8) | (u32::from(bytes[2]) << 16);
    Ok(Fetched { raw, width, bytes })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Alu {
    Add,
    AddX2,
    AddX4,
    AddX8,
    Sub,
    SubX2,
    SubX4,
    SubX8,
    And,
    Or,
    Xor,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConditionalMove {
    Eqz,
    Nez,
    Ltz,
    Gez,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SarShift {
    Sll,
    Sra,
    Src,
    Srl,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MemoryOp {
    L8ui,
    L16ui,
    L16si,
    L32i,
    L32r,
    S8i,
    S16i,
    S32i,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BranchReg {
    None,
    All,
    Any,
    NotAll,
    Eq,
    Ne,
    Lt,
    Ge,
    Ltu,
    Geu,
    BitClear,
    BitSet,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BranchImm {
    Eq,
    Ne,
    Lt,
    Ge,
    Ltu,
    Geu,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BranchZero {
    Eq,
    Ne,
    Lt,
    Ge,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoopOp {
    Always,
    Gtz,
    Nez,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpecialOp {
    Read,
    Write,
    Exchange,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SyncOp {
    Isync,
    Rsync,
    Esync,
    Dsync,
    Memw,
    Extw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Alu(Alu),
    Neg,
    Abs,
    Addi,
    Addmi,
    Movi,
    Extui,
    ConditionalMove(ConditionalMove),
    ShiftImmediateLeft,
    ShiftImmediateArithmetic,
    ShiftImmediateLogical,
    SarShift(SarShift),
    Ssa8b,
    Ssa8l,
    Ssai,
    Ssl,
    Ssr,
    Memory(MemoryOp),
    BranchReg(BranchReg),
    BranchImm(BranchImm),
    BranchZero(BranchZero),
    BranchBitImmediate { set: bool },
    Jump,
    JumpRegister,
    Call { increment: u8, indirect: bool },
    Return,
    Entry,
    Retw,
    Movsp,
    Loop(LoopOp),
    Special(SpecialOp),
    Sync(SyncOp),
    Nop,
    Halt,
    AddN,
    AddiN,
    MovN,
    MoviN,
    L32iN,
    S32iN,
    BranchZeroN { ne: bool },
    NopN,
    ReturnN,
    RetwN,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Decoded {
    op: Op,
    r: u8,
    s: u8,
    t: u8,
    imm: u32,
}

/// Exact state-independent classification result for generator and tooling use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstructionInfo {
    pub mnemonic: &'static str,
    pub width: u8,
}

/// Classify one little-endian raw instruction using the production decoder.
///
/// `raw` contains only the low `width * 8` bits.  Width is selected by the
/// low opcode nibble in normal fetch; callers that provide an inconsistent
/// width receive `UnknownInstruction`.
pub fn classify_instruction(raw: u32, width: u8) -> Result<InstructionInfo, XtensaLx6Error> {
    if !matches!(width, 2 | 3) || ((raw & 8 != 0) != (width == 2)) {
        return Err(XtensaLx6Error::UnknownInstruction { raw, width });
    }
    let decoded = decode(raw, width)?;
    Ok(InstructionInfo {
        mnemonic: decoded.mnemonic(),
        width,
    })
}

impl Decoded {
    fn mnemonic(self) -> &'static str {
        match self.op {
            Op::Alu(Alu::Add) => "ADD",
            Op::Alu(Alu::AddX2) => "ADDX2",
            Op::Alu(Alu::AddX4) => "ADDX4",
            Op::Alu(Alu::AddX8) => "ADDX8",
            Op::Alu(Alu::Sub) => "SUB",
            Op::Alu(Alu::SubX2) => "SUBX2",
            Op::Alu(Alu::SubX4) => "SUBX4",
            Op::Alu(Alu::SubX8) => "SUBX8",
            Op::Alu(Alu::And) => "AND",
            Op::Alu(Alu::Or) => "OR",
            Op::Alu(Alu::Xor) => "XOR",
            Op::Neg => "NEG",
            Op::Abs => "ABS",
            Op::Addi => "ADDI",
            Op::Addmi => "ADDMI",
            Op::Movi => "MOVI",
            Op::Extui => "EXTUI",
            Op::ConditionalMove(ConditionalMove::Eqz) => "MOVEQZ",
            Op::ConditionalMove(ConditionalMove::Nez) => "MOVNEZ",
            Op::ConditionalMove(ConditionalMove::Ltz) => "MOVLTZ",
            Op::ConditionalMove(ConditionalMove::Gez) => "MOVGEZ",
            Op::ShiftImmediateLeft => "SLLI",
            Op::ShiftImmediateArithmetic => "SRAI",
            Op::ShiftImmediateLogical => "SRLI",
            Op::SarShift(SarShift::Sll) => "SLL",
            Op::SarShift(SarShift::Sra) => "SRA",
            Op::SarShift(SarShift::Src) => "SRC",
            Op::SarShift(SarShift::Srl) => "SRL",
            Op::Ssa8b => "SSA8B",
            Op::Ssa8l => "SSA8L",
            Op::Ssai => "SSAI",
            Op::Ssl => "SSL",
            Op::Ssr => "SSR",
            Op::Memory(MemoryOp::L8ui) => "L8UI",
            Op::Memory(MemoryOp::L16ui) => "L16UI",
            Op::Memory(MemoryOp::L16si) => "L16SI",
            Op::Memory(MemoryOp::L32i) => "L32I",
            Op::Memory(MemoryOp::L32r) => "L32R",
            Op::Memory(MemoryOp::S8i) => "S8I",
            Op::Memory(MemoryOp::S16i) => "S16I",
            Op::Memory(MemoryOp::S32i) => "S32I",
            Op::BranchReg(BranchReg::None) => "BNONE",
            Op::BranchReg(BranchReg::All) => "BALL",
            Op::BranchReg(BranchReg::Any) => "BANY",
            Op::BranchReg(BranchReg::NotAll) => "BNALL",
            Op::BranchReg(BranchReg::Eq) => "BEQ",
            Op::BranchReg(BranchReg::Ne) => "BNE",
            Op::BranchReg(BranchReg::Lt) => "BLT",
            Op::BranchReg(BranchReg::Ge) => "BGE",
            Op::BranchReg(BranchReg::Ltu) => "BLTU",
            Op::BranchReg(BranchReg::Geu) => "BGEU",
            Op::BranchReg(BranchReg::BitClear) => "BBC",
            Op::BranchReg(BranchReg::BitSet) => "BBS",
            Op::BranchImm(BranchImm::Eq) => "BEQI",
            Op::BranchImm(BranchImm::Ne) => "BNEI",
            Op::BranchImm(BranchImm::Lt) => "BLTI",
            Op::BranchImm(BranchImm::Ge) => "BGEI",
            Op::BranchImm(BranchImm::Ltu) => "BLTUI",
            Op::BranchImm(BranchImm::Geu) => "BGEUI",
            Op::BranchZero(BranchZero::Eq) => "BEQZ",
            Op::BranchZero(BranchZero::Ne) => "BNEZ",
            Op::BranchZero(BranchZero::Lt) => "BLTZ",
            Op::BranchZero(BranchZero::Ge) => "BGEZ",
            Op::BranchBitImmediate { set: false } => "BBCI",
            Op::BranchBitImmediate { set: true } => "BBSI",
            Op::Jump => "J",
            Op::JumpRegister => "JX",
            Op::Call {
                increment: 0,
                indirect: false,
            } => "CALL0",
            Op::Call {
                increment: 1,
                indirect: false,
            } => "CALL4",
            Op::Call {
                increment: 2,
                indirect: false,
            } => "CALL8",
            Op::Call {
                increment: 3,
                indirect: false,
            } => "CALL12",
            Op::Call {
                increment: 0,
                indirect: true,
            } => "CALLX0",
            Op::Call {
                increment: 1,
                indirect: true,
            } => "CALLX4",
            Op::Call {
                increment: 2,
                indirect: true,
            } => "CALLX8",
            Op::Call {
                increment: 3,
                indirect: true,
            } => "CALLX12",
            Op::Call { .. } => unreachable!(),
            Op::Return => "RET",
            Op::Entry => "ENTRY",
            Op::Retw => "RETW",
            Op::Movsp => "MOVSP",
            Op::Loop(LoopOp::Always) => "LOOP",
            Op::Loop(LoopOp::Gtz) => "LOOPGTZ",
            Op::Loop(LoopOp::Nez) => "LOOPNEZ",
            Op::Special(SpecialOp::Read) => "RSR",
            Op::Special(SpecialOp::Write) => "WSR",
            Op::Special(SpecialOp::Exchange) => "XSR",
            Op::Sync(SyncOp::Isync) => "ISYNC",
            Op::Sync(SyncOp::Rsync) => "RSYNC",
            Op::Sync(SyncOp::Esync) => "ESYNC",
            Op::Sync(SyncOp::Dsync) => "DSYNC",
            Op::Sync(SyncOp::Memw) => "MEMW",
            Op::Sync(SyncOp::Extw) => "EXTW",
            Op::Nop => "NOP",
            Op::Halt => "BREAK",
            Op::AddN => "ADD.N",
            Op::AddiN => "ADDI.N",
            Op::MovN => "MOV.N",
            Op::MoviN => "MOVI.N",
            Op::L32iN => "L32I.N",
            Op::S32iN => "S32I.N",
            Op::BranchZeroN { ne: false } => "BEQZ.N",
            Op::BranchZeroN { ne: true } => "BNEZ.N",
            Op::NopN => "NOP.N",
            Op::ReturnN => "RET.N",
            Op::RetwN => "RETW.N",
        }
    }
}

fn d(op: Op, r: u8, s: u8, t: u8, imm: u32) -> Decoded {
    Decoded { op, r, s, t, imm }
}

/// Decode one already-fetched instruction without inspecting machine state.
fn decode(raw: u32, width: u8) -> Result<Decoded, XtensaLx6Error> {
    if width == 2 {
        return decode_narrow(raw as u16);
    }
    let op0 = (raw & 0xf) as u8;
    let t = ((raw >> 4) & 0xf) as u8;
    let s = ((raw >> 8) & 0xf) as u8;
    let r = ((raw >> 12) & 0xf) as u8;
    let op1 = ((raw >> 16) & 0xf) as u8;
    let op2 = ((raw >> 20) & 0xf) as u8;
    let imm8 = (raw >> 16) & 0xff;
    let unknown = || XtensaLx6Error::UnknownInstruction { raw, width: 3 };
    let unsupported = || XtensaLx6Error::UnsupportedOptionInstruction { raw, width: 3 };
    match op0 {
        0 => match op1 {
            0 => match op2 {
                0 => decode_rst0(raw, r, s, t),
                1 => Ok(d(Op::Alu(Alu::And), r, s, t, 0)),
                2 => Ok(d(Op::Alu(Alu::Or), r, s, t, 0)),
                3 => Ok(d(Op::Alu(Alu::Xor), r, s, t, 0)),
                4 => decode_st1(raw, r, s, t),
                5 => Err(unsupported()),
                6 => match s {
                    0 => Ok(d(Op::Neg, r, 0, t, 0)),
                    1 => Ok(d(Op::Abs, r, 0, t, 0)),
                    _ => Err(unknown()),
                },
                8 => Ok(d(Op::Alu(Alu::Add), r, s, t, 0)),
                9 => Ok(d(Op::Alu(Alu::AddX2), r, s, t, 0)),
                10 => Ok(d(Op::Alu(Alu::AddX4), r, s, t, 0)),
                11 => Ok(d(Op::Alu(Alu::AddX8), r, s, t, 0)),
                12 => Ok(d(Op::Alu(Alu::Sub), r, s, t, 0)),
                13 => Ok(d(Op::Alu(Alu::SubX2), r, s, t, 0)),
                14 => Ok(d(Op::Alu(Alu::SubX4), r, s, t, 0)),
                15 => Ok(d(Op::Alu(Alu::SubX8), r, s, t, 0)),
                _ => Err(unknown()),
            },
            1 => match op2 {
                0 | 1 => {
                    let encoded = t | ((op2 & 1) << 4);
                    if encoded == 0 {
                        Err(unknown())
                    } else {
                        Ok(d(Op::ShiftImmediateLeft, r, s, 0, u32::from(32 - encoded)))
                    }
                }
                2 | 3 => Ok(d(
                    Op::ShiftImmediateArithmetic,
                    r,
                    0,
                    t,
                    u32::from(s | ((op2 & 1) << 4)),
                )),
                4 => Ok(d(Op::ShiftImmediateLogical, r, 0, t, u32::from(s))),
                6 => Ok(d(
                    Op::Special(SpecialOp::Exchange),
                    0,
                    0,
                    t,
                    u32::from((r << 4) | s),
                )),
                8 => Ok(d(Op::SarShift(SarShift::Src), r, s, t, 0)),
                9 if s == 0 => Ok(d(Op::SarShift(SarShift::Srl), r, 0, t, 0)),
                10 if t == 0 => Ok(d(Op::SarShift(SarShift::Sll), r, s, 0, 0)),
                11 if s == 0 => Ok(d(Op::SarShift(SarShift::Sra), r, 0, t, 0)),
                12 | 13 => Err(unsupported()),
                _ => Err(unknown()),
            },
            2 => {
                if matches!(op2, 0 | 1 | 2 | 4 | 8 | 10..=15) {
                    Err(unsupported())
                } else {
                    Err(unknown())
                }
            }
            3 => match op2 {
                0 => Ok(d(
                    Op::Special(SpecialOp::Read),
                    0,
                    0,
                    t,
                    u32::from((r << 4) | s),
                )),
                1 => Ok(d(
                    Op::Special(SpecialOp::Write),
                    0,
                    0,
                    t,
                    u32::from((r << 4) | s),
                )),
                8 => Ok(d(Op::ConditionalMove(ConditionalMove::Eqz), r, s, t, 0)),
                9 => Ok(d(Op::ConditionalMove(ConditionalMove::Nez), r, s, t, 0)),
                10 => Ok(d(Op::ConditionalMove(ConditionalMove::Ltz), r, s, t, 0)),
                11 => Ok(d(Op::ConditionalMove(ConditionalMove::Gez), r, s, t, 0)),
                _ => Err(unsupported()),
            },
            4 | 5 => {
                let sa = s | ((op1 & 1) << 4);
                if u16::from(sa) + u16::from(op2) > 31 {
                    Err(unknown())
                } else {
                    Ok(d(
                        Op::Extui,
                        r,
                        0,
                        t,
                        (u32::from(sa) << 8) | u32::from(op2 + 1),
                    ))
                }
            }
            9 if matches!(op2, 0 | 4) => {
                Err(XtensaLx6Error::PrivilegedInstruction { raw, width: 3 })
            }
            6 | 7 => Err(unknown()),
            8 => {
                if matches!(op2, 0 | 1 | 4 | 5) {
                    Err(unsupported())
                } else {
                    Err(unknown())
                }
            }
            9 => Err(unknown()),
            10 => {
                let valid = matches!(op2, 0 | 1 | 2 | 4 | 5 | 8..=14)
                    || op2 == 15 && matches!(t, 0 | 1 | 4 | 5 | 6);
                if valid {
                    Err(unsupported())
                } else {
                    Err(unknown())
                }
            }
            11 => {
                if matches!(op2, 1..=13) {
                    Err(unsupported())
                } else {
                    Err(unknown())
                }
            }
            _ => Err(unknown()),
        },
        1 => Ok(d(Op::Memory(MemoryOp::L32r), 0, 0, t, raw >> 8)),
        2 => match r {
            0 => Ok(d(Op::Memory(MemoryOp::L8ui), 0, s, t, imm8)),
            1 => Ok(d(Op::Memory(MemoryOp::L16ui), 0, s, t, imm8)),
            2 => Ok(d(Op::Memory(MemoryOp::L32i), 0, s, t, imm8)),
            4 => Ok(d(Op::Memory(MemoryOp::S8i), 0, s, t, imm8)),
            5 => Ok(d(Op::Memory(MemoryOp::S16i), 0, s, t, imm8)),
            6 => Ok(d(Op::Memory(MemoryOp::S32i), 0, s, t, imm8)),
            9 => Ok(d(Op::Memory(MemoryOp::L16si), 0, s, t, imm8)),
            10 => Ok(d(Op::Movi, 0, 0, t, (u32::from(s) << 8) | imm8)),
            12 => Ok(d(Op::Addi, 0, s, t, imm8)),
            13 => Ok(d(Op::Addmi, 0, s, t, imm8)),
            11 | 14 | 15 => Err(unsupported()),
            7 => decode_cache_option(raw, t, imm8),
            _ => Err(unknown()),
        },
        3 => {
            if matches!(r, 0 | 4 | 8 | 12) {
                Err(unsupported())
            } else {
                Err(unknown())
            }
        }
        4 => Err(unsupported()),
        5 => {
            let increment = t & 3;
            let offset = raw >> 6;
            Ok(d(
                Op::Call {
                    increment,
                    indirect: false,
                },
                0,
                0,
                0,
                offset,
            ))
        }
        6 => decode_si(raw, r, s, t),
        7 => decode_branch(raw, r, s, t),
        _ => Err(unknown()),
    }
}

fn decode_rst0(raw: u32, r: u8, s: u8, t: u8) -> Result<Decoded, XtensaLx6Error> {
    let unknown = || XtensaLx6Error::UnknownInstruction { raw, width: 3 };
    match r {
        0 => {
            let n = t & 3;
            let m = t >> 2;
            match m {
                2 => match n {
                    0 if s == 0 => Ok(d(Op::Return, 0, 0, 0, 0)),
                    1 if s == 0 => Ok(d(Op::Retw, 0, 0, 0, 0)),
                    2 => Ok(d(Op::JumpRegister, 0, s, 0, 0)),
                    _ => Err(unknown()),
                },
                3 => Ok(d(
                    Op::Call {
                        increment: n,
                        indirect: true,
                    },
                    0,
                    s,
                    0,
                    0,
                )),
                _ => Err(unknown()),
            }
        }
        1 => Ok(d(Op::Movsp, 0, s, t, 0)),
        2 if s == 0 && t == 15 => Ok(d(Op::Nop, 0, 0, 0, 0)),
        2 if s == 0 && matches!(t, 0 | 1 | 2 | 3 | 12 | 13) => Ok(d(
            Op::Sync(match t {
                0 => SyncOp::Isync,
                1 => SyncOp::Rsync,
                2 => SyncOp::Esync,
                3 => SyncOp::Dsync,
                12 => SyncOp::Memw,
                13 => SyncOp::Extw,
                _ => unreachable!(),
            }),
            0,
            0,
            0,
            0,
        )),
        2 if s == 0 && t == 8 => {
            Err(XtensaLx6Error::UnsupportedOptionInstruction { raw, width: 3 })
        }
        3 if (t == 0 && matches!(s, 0 | 2 | 4 | 5)) || t == 1 || (t == 2 && s == 0) => {
            Err(XtensaLx6Error::PrivilegedInstruction { raw, width: 3 })
        }
        4 if s == 0 && t == 0 => Ok(d(Op::Halt, 0, 0, 0, 0)),
        4 => Err(XtensaLx6Error::Breakpoint { code: (s << 4) | t }),
        5 if s == 0 && t == 0 => {
            Err(XtensaLx6Error::UnsupportedOptionInstruction { raw, width: 3 })
        }
        6 => Err(XtensaLx6Error::PrivilegedInstruction { raw, width: 3 }),
        7 if t == 0 => Err(XtensaLx6Error::PrivilegedInstruction { raw, width: 3 }),
        8..=11 => Err(XtensaLx6Error::UnsupportedOptionInstruction { raw, width: 3 }),
        _ => Err(unknown()),
    }
}

fn decode_st1(raw: u32, r: u8, s: u8, t: u8) -> Result<Decoded, XtensaLx6Error> {
    let unknown = || XtensaLx6Error::UnknownInstruction { raw, width: 3 };
    match r {
        0 if t == 0 => Ok(d(Op::Ssr, 0, s, 0, 0)),
        1 if t == 0 => Ok(d(Op::Ssl, 0, s, 0, 0)),
        2 if t == 0 => Ok(d(Op::Ssa8l, 0, s, 0, 0)),
        3 if t == 0 => Ok(d(Op::Ssa8b, 0, s, 0, 0)),
        4 if t <= 1 => Ok(d(Op::Ssai, 0, 0, 0, u32::from(s | (t << 4)))),
        8 if s == 0 => Err(XtensaLx6Error::PrivilegedInstruction { raw, width: 3 }),
        5 | 7 | 10 | 11 => Err(XtensaLx6Error::UnsupportedOptionInstruction { raw, width: 3 }),
        _ => Err(unknown()),
    }
}

fn decode_cache_option(raw: u32, t: u8, imm8: u32) -> Result<Decoded, XtensaLx6Error> {
    let op1 = (imm8 & 0xf) as u8;
    let valid = match t {
        0..=9 | 12 | 14 | 15 => true,
        10 => matches!(op1, 0 | 2 | 3 | 4 | 5),
        13 => matches!(op1, 0 | 2 | 3),
        _ => false,
    };
    if valid {
        Err(XtensaLx6Error::UnsupportedOptionInstruction { raw, width: 3 })
    } else {
        Err(XtensaLx6Error::UnknownInstruction { raw, width: 3 })
    }
}

fn reserved_branch_disp(raw: u32, disp: i32) -> Result<(), XtensaLx6Error> {
    if (-3..=0).contains(&disp) {
        Err(XtensaLx6Error::UnknownInstruction { raw, width: 3 })
    } else {
        Ok(())
    }
}

fn decode_si(raw: u32, r: u8, s: u8, t: u8) -> Result<Decoded, XtensaLx6Error> {
    let n = t & 3;
    let m = t >> 2;
    match n {
        0 => Ok(d(Op::Jump, 0, 0, 0, raw >> 6)),
        1 => {
            let disp = sign_extend(raw >> 12, 12);
            reserved_branch_disp(raw, disp)?;
            let relation = match m {
                0 => BranchZero::Eq,
                1 => BranchZero::Ne,
                2 => BranchZero::Lt,
                _ => BranchZero::Ge,
            };
            Ok(d(Op::BranchZero(relation), 0, s, 0, disp as u32))
        }
        2 => {
            let disp = sign_extend(raw >> 16, 8);
            reserved_branch_disp(raw, disp)?;
            let relation = match m {
                0 => BranchImm::Eq,
                1 => BranchImm::Ne,
                2 => BranchImm::Lt,
                _ => BranchImm::Ge,
            };
            Ok(d(Op::BranchImm(relation), r, s, 0, disp as u32))
        }
        3 => match m {
            0 => Ok(d(Op::Entry, 0, s, 0, raw >> 12)),
            1 => {
                let disp = (raw >> 16) & 0xff;
                match r {
                    0 | 1 => Err(XtensaLx6Error::UnsupportedOptionInstruction { raw, width: 3 }),
                    8 => Ok(d(Op::Loop(LoopOp::Always), 0, s, 0, disp)),
                    9 => Ok(d(Op::Loop(LoopOp::Nez), 0, s, 0, disp)),
                    10 => Ok(d(Op::Loop(LoopOp::Gtz), 0, s, 0, disp)),
                    _ => Err(XtensaLx6Error::UnknownInstruction { raw, width: 3 }),
                }
            }
            2 | 3 => {
                let disp = sign_extend(raw >> 16, 8);
                reserved_branch_disp(raw, disp)?;
                Ok(d(
                    Op::BranchImm(if m == 2 {
                        BranchImm::Ltu
                    } else {
                        BranchImm::Geu
                    }),
                    r,
                    s,
                    0,
                    disp as u32,
                ))
            }
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

fn decode_branch(raw: u32, r: u8, s: u8, t: u8) -> Result<Decoded, XtensaLx6Error> {
    let disp = sign_extend(raw >> 16, 8);
    reserved_branch_disp(raw, disp)?;
    let op = match r {
        0 => Op::BranchReg(BranchReg::None),
        1 => Op::BranchReg(BranchReg::Eq),
        2 => Op::BranchReg(BranchReg::Lt),
        3 => Op::BranchReg(BranchReg::Ltu),
        4 => Op::BranchReg(BranchReg::All),
        5 => Op::BranchReg(BranchReg::BitClear),
        6 | 7 => Op::BranchBitImmediate { set: false },
        8 => Op::BranchReg(BranchReg::Any),
        9 => Op::BranchReg(BranchReg::Ne),
        10 => Op::BranchReg(BranchReg::Ge),
        11 => Op::BranchReg(BranchReg::Geu),
        12 => Op::BranchReg(BranchReg::NotAll),
        13 => Op::BranchReg(BranchReg::BitSet),
        14 | 15 => Op::BranchBitImmediate { set: true },
        _ => unreachable!(),
    };
    Ok(d(op, r, s, t, disp as u32))
}

fn decode_narrow(raw: u16) -> Result<Decoded, XtensaLx6Error> {
    let value = u32::from(raw);
    let op0 = (raw & 0xf) as u8;
    let t = ((raw >> 4) & 0xf) as u8;
    let s = ((raw >> 8) & 0xf) as u8;
    let r = ((raw >> 12) & 0xf) as u8;
    let unknown = || XtensaLx6Error::UnknownInstruction {
        raw: value,
        width: 2,
    };
    match op0 {
        8 => Ok(d(Op::L32iN, r, s, t, u32::from(r))),
        9 => Ok(d(Op::S32iN, r, s, t, u32::from(r))),
        10 => Ok(d(Op::AddN, r, s, t, 0)),
        11 => Ok(d(Op::AddiN, r, s, t, 0)),
        12 if t < 8 => {
            let imm7 = u32::from(r | ((t & 7) << 4));
            Ok(d(Op::MoviN, 0, s, 0, imm7))
        }
        12 if t & 0xc == 8 => {
            let imm6 = u32::from(r | ((t & 1) << 4) | ((t & 2) << 4));
            Ok(d(Op::BranchZeroN { ne: false }, 0, s, 0, imm6))
        }
        12 if t & 0xc == 12 => {
            let imm6 = u32::from(r | ((t & 1) << 4) | ((t & 2) << 4));
            Ok(d(Op::BranchZeroN { ne: true }, 0, s, 0, imm6))
        }
        13 if r == 0 && s != t => Ok(d(Op::MovN, 0, s, t, 0)),
        13 if r == 15 && s == 0 => match t {
            0 => Ok(d(Op::ReturnN, 0, 0, 0, 0)),
            1 => Ok(d(Op::RetwN, 0, 0, 0, 0)),
            2 => Err(XtensaLx6Error::Breakpoint { code: 0 }),
            3 => Ok(d(Op::NopN, 0, 0, 0, 0)),
            6 => Err(unknown()),
            _ => Err(unknown()),
        },
        _ => Err(unknown()),
    }
}

fn sign_extend(value: u32, bits: u8) -> i32 {
    ((value << (32 - bits)) as i32) >> (32 - bits)
}

fn group(register: u8) -> u8 {
    register >> 2
}

fn window_groups(state: &XtensaLx6State, decoded: &Decoded) -> Option<[u8; 3]> {
    let groups = match decoded.op {
        Op::Call { increment, .. } => (0, 0, increment),
        Op::Entry => (0, state.call_inc, 0),
        Op::Retw | Op::RetwN => return None,
        Op::Alu(_) | Op::ConditionalMove(_) | Op::AddN => {
            (group(decoded.r), group(decoded.s), group(decoded.t))
        }
        Op::Neg | Op::Abs => (group(decoded.r), 0, group(decoded.t)),
        Op::Addi | Op::Addmi => (0, group(decoded.s), group(decoded.t)),
        Op::Movi => (0, 0, group(decoded.t)),
        Op::Extui => (group(decoded.r), 0, group(decoded.t)),
        Op::ShiftImmediateLeft => (group(decoded.r), group(decoded.s), 0),
        Op::ShiftImmediateArithmetic | Op::ShiftImmediateLogical => {
            (group(decoded.r), 0, group(decoded.t))
        }
        Op::SarShift(SarShift::Sll) => (group(decoded.r), group(decoded.s), 0),
        Op::SarShift(SarShift::Sra | SarShift::Srl) => (group(decoded.r), 0, group(decoded.t)),
        Op::SarShift(SarShift::Src) => (group(decoded.r), group(decoded.s), group(decoded.t)),
        Op::Ssa8b | Op::Ssa8l | Op::Ssl | Op::Ssr => (0, group(decoded.s), 0),
        Op::Ssai | Op::Sync(_) | Op::Nop | Op::Halt | Op::NopN | Op::Jump => (0, 0, 0),
        Op::Memory(MemoryOp::L32r) => (0, 0, group(decoded.t)),
        Op::Memory(_) | Op::L32iN | Op::S32iN => (0, group(decoded.s), group(decoded.t)),
        Op::BranchReg(_) => (0, group(decoded.s), group(decoded.t)),
        Op::BranchImm(_)
        | Op::BranchZero(_)
        | Op::BranchBitImmediate { .. }
        | Op::Loop(_)
        | Op::BranchZeroN { .. } => (0, group(decoded.s), 0),
        Op::JumpRegister => (0, group(decoded.s), 0),
        Op::Return | Op::ReturnN => (0, 0, 0),
        Op::Movsp | Op::MovN => (0, group(decoded.s), group(decoded.t)),
        Op::Special(_) => (0, 0, group(decoded.t)),
        Op::AddiN => (group(decoded.r), group(decoded.s), 0),
        Op::MoviN => (0, group(decoded.s), 0),
    };
    Some([groups.0, groups.1, groups.2])
}

fn window_check_for(state: &XtensaLx6State, decoded: &Decoded) -> Result<(), XtensaLx6Error> {
    let Some(groups) = window_groups(state, decoded) else {
        return Ok(());
    };
    let wb = state.window_base;
    let active = |increment: u8| state.window_start & (1u16 << ((wb + increment) & 15)) != 0;
    if groups.iter().any(|value| *value != 0) && active(1) {
        return Err(XtensaLx6Error::WindowOverflow { increment: 1 });
    }
    if groups.iter().any(|value| value & 2 != 0) && active(2) {
        return Err(XtensaLx6Error::WindowOverflow { increment: 2 });
    }
    if groups.contains(&3) && active(3) {
        return Err(XtensaLx6Error::WindowOverflow { increment: 3 });
    }
    Ok(())
}

fn execute_decoded(
    before: &XtensaLx6State,
    next: &mut XtensaLx6State,
    decoded: &Decoded,
    raw: u32,
    width: u8,
    access: &mut Option<DataAccess>,
) -> Result<bool, XtensaLx6Error> {
    let r = decoded.r;
    let s = decoded.s;
    let t = decoded.t;
    match decoded.op {
        Op::Alu(op) => {
            let left = read_ar(before, s);
            let right = read_ar(before, t);
            let value = match op {
                Alu::Add => left.wrapping_add(right),
                Alu::AddX2 => left.wrapping_shl(1).wrapping_add(right),
                Alu::AddX4 => left.wrapping_shl(2).wrapping_add(right),
                Alu::AddX8 => left.wrapping_shl(3).wrapping_add(right),
                Alu::Sub => left.wrapping_sub(right),
                Alu::SubX2 => left.wrapping_shl(1).wrapping_sub(right),
                Alu::SubX4 => left.wrapping_shl(2).wrapping_sub(right),
                Alu::SubX8 => left.wrapping_shl(3).wrapping_sub(right),
                Alu::And => left & right,
                Alu::Or => left | right,
                Alu::Xor => left ^ right,
            };
            write_ar(next, r, value);
        }
        Op::Neg => write_ar(next, r, 0u32.wrapping_sub(read_ar(before, t))),
        Op::Abs => {
            let value = read_ar(before, t);
            write_ar(
                next,
                r,
                if (value as i32) < 0 {
                    0u32.wrapping_sub(value)
                } else {
                    value
                },
            );
        }
        Op::Addi => write_ar(
            next,
            t,
            read_ar(before, s).wrapping_add(sign_extend(decoded.imm, 8) as u32),
        ),
        Op::Addmi => write_ar(
            next,
            t,
            read_ar(before, s).wrapping_add((sign_extend(decoded.imm, 8) << 8) as u32),
        ),
        Op::Movi => write_ar(next, t, sign_extend(decoded.imm, 12) as u32),
        Op::Extui => {
            let shift = decoded.imm >> 8;
            let bits = decoded.imm & 0xff;
            let mask = if bits == 32 {
                u32::MAX
            } else {
                (1u32 << bits) - 1
            };
            write_ar(next, r, (read_ar(before, t) >> shift) & mask);
        }
        Op::ConditionalMove(condition) => {
            let test = read_ar(before, t);
            let take = match condition {
                ConditionalMove::Eqz => test == 0,
                ConditionalMove::Nez => test != 0,
                ConditionalMove::Ltz => (test as i32) < 0,
                ConditionalMove::Gez => (test as i32) >= 0,
            };
            if take {
                write_ar(next, r, read_ar(before, s));
            }
        }
        Op::ShiftImmediateLeft => write_ar(next, r, read_ar(before, s).wrapping_shl(decoded.imm)),
        Op::ShiftImmediateArithmetic => {
            write_ar(next, r, ((read_ar(before, t) as i32) >> decoded.imm) as u32)
        }
        Op::ShiftImmediateLogical => write_ar(next, r, read_ar(before, t) >> decoded.imm),
        Op::SarShift(operation) => {
            let amount = u32::from(before.sar);
            if amount > 32 {
                return Err(XtensaLx6Error::UndefinedOperation {
                    raw,
                    width,
                    reason: UndefinedReason::ShiftAmountOutOfRange,
                });
            }
            let value = match operation {
                SarShift::Sll => {
                    let left = 32 - amount;
                    if left == 32 {
                        0
                    } else {
                        read_ar(before, s) << left
                    }
                }
                SarShift::Sra => {
                    if amount == 32 {
                        if (read_ar(before, t) as i32) < 0 {
                            u32::MAX
                        } else {
                            0
                        }
                    } else {
                        ((read_ar(before, t) as i32) >> amount) as u32
                    }
                }
                SarShift::Srl => {
                    if amount == 32 {
                        0
                    } else {
                        read_ar(before, t) >> amount
                    }
                }
                SarShift::Src => {
                    ((u64::from(read_ar(before, s)) << 32 | u64::from(read_ar(before, t)))
                        >> amount) as u32
                }
            };
            write_ar(next, r, value);
        }
        Op::Ssa8b => next.sar = 32 - ((read_ar(before, s) as u8 & 3) << 3),
        Op::Ssa8l => next.sar = (read_ar(before, s) as u8 & 3) << 3,
        Op::Ssai => next.sar = decoded.imm as u8,
        Op::Ssl => {
            let low = read_ar(before, s) as u8 & 31;
            next.sar = 32 - low;
        }
        Op::Ssr => next.sar = read_ar(before, s) as u8 & 31,
        Op::Memory(operation) => execute_memory(before, next, operation, decoded, access)?,
        Op::BranchReg(relation) => {
            let left = read_ar(before, s);
            let right = read_ar(before, t);
            let taken = match relation {
                BranchReg::None => left & right == 0,
                BranchReg::All => left & right == right,
                BranchReg::Any => left & right != 0,
                BranchReg::NotAll => left & right != right,
                BranchReg::Eq => left == right,
                BranchReg::Ne => left != right,
                BranchReg::Lt => (left as i32) < (right as i32),
                BranchReg::Ge => (left as i32) >= (right as i32),
                BranchReg::Ltu => left < right,
                BranchReg::Geu => left >= right,
                BranchReg::BitClear => left & (1 << (right & 31)) == 0,
                BranchReg::BitSet => left & (1 << (right & 31)) != 0,
            };
            if taken {
                next.pc = branch_target(before.pc, decoded.imm);
                return Ok(true);
            }
        }
        Op::BranchImm(relation) => {
            let left = read_ar(before, s);
            let right = if matches!(relation, BranchImm::Ltu | BranchImm::Geu) {
                b4constu(r)
            } else {
                b4const(r)
            };
            let taken = match relation {
                BranchImm::Eq => left == right,
                BranchImm::Ne => left != right,
                BranchImm::Lt => (left as i32) < (right as i32),
                BranchImm::Ge => (left as i32) >= (right as i32),
                BranchImm::Ltu => left < right,
                BranchImm::Geu => left >= right,
            };
            if taken {
                next.pc = branch_target(before.pc, decoded.imm);
                return Ok(true);
            }
        }
        Op::BranchZero(relation) => {
            let value = read_ar(before, s);
            let taken = match relation {
                BranchZero::Eq => value == 0,
                BranchZero::Ne => value != 0,
                BranchZero::Lt => (value as i32) < 0,
                BranchZero::Ge => (value as i32) >= 0,
            };
            if taken {
                next.pc = branch_target(before.pc, decoded.imm);
                return Ok(true);
            }
        }
        Op::BranchBitImmediate { set } => {
            let bit = u32::from(t | ((r & 1) << 4));
            let present = read_ar(before, s) & (1 << bit) != 0;
            if present == set {
                next.pc = branch_target(before.pc, decoded.imm);
                return Ok(true);
            }
        }
        Op::Jump => {
            next.pc = before
                .pc
                .wrapping_add(4)
                .wrapping_add((sign_extend(decoded.imm, 18)) as u32);
            return Ok(true);
        }
        Op::JumpRegister => {
            next.pc = read_ar(before, s);
            return Ok(true);
        }
        Op::Call {
            increment,
            indirect,
        } => {
            let target = if indirect {
                read_ar(before, s)
            } else {
                ((before.pc & !3).wrapping_add((sign_extend(decoded.imm, 18) << 2) as u32))
                    .wrapping_add(4)
            };
            let link = (u32::from(increment) << 30) | before.pc.wrapping_add(3) & 0x3fff_ffff;
            write_ar(next, increment * 4, link);
            if increment != 0 {
                next.call_inc = increment;
            }
            next.pc = target;
            return Ok(true);
        }
        Op::Return | Op::ReturnN => {
            next.pc = read_ar(before, 0);
            return Ok(true);
        }
        Op::Entry => {
            if s > 3 {
                return Err(XtensaLx6Error::UndefinedOperation {
                    raw,
                    width,
                    reason: UndefinedReason::InvalidEntrySource,
                });
            }
            let stack = read_ar(before, s).wrapping_sub(decoded.imm << 3);
            let new_wb = (before.window_base + before.call_inc) & 15;
            next.window_base = new_wb;
            next.window_start |= 1u16 << new_wb;
            write_ar(next, s, stack);
        }
        Op::Retw | Op::RetwN => {
            execute_retw(before, next, raw, width)?;
            return Ok(true);
        }
        Op::Movsp => {
            let available = (1..=3).any(|back| {
                before.window_start & (1u16 << (before.window_base.wrapping_sub(back) & 15)) != 0
            });
            if !available {
                return Err(XtensaLx6Error::AllocaCause);
            }
            write_ar(next, t, read_ar(before, s));
        }
        Op::Loop(operation) => {
            let count = read_ar(before, s);
            let lbeg = before.pc.wrapping_add(3);
            let lend = before.pc.wrapping_add(4).wrapping_add(decoded.imm);
            validate_loop(before, lbeg, lend)?;
            next.lcount = count.wrapping_sub(1);
            next.lbeg = lbeg;
            next.lend = lend;
            let skip = match operation {
                LoopOp::Always => false,
                LoopOp::Gtz => (count as i32) <= 0,
                LoopOp::Nez => count == 0,
            };
            if skip {
                next.pc = lend;
                return Ok(true);
            }
        }
        Op::Special(operation) => execute_special(before, next, operation, decoded.imm as u8, t)?,
        Op::Sync(_) | Op::Nop => {}
        Op::Halt => next.halted = true,
        Op::AddN => write_ar(next, r, read_ar(before, s).wrapping_add(read_ar(before, t))),
        Op::AddiN => {
            let immediate = if t == 0 { u32::MAX } else { u32::from(t) };
            write_ar(next, r, read_ar(before, s).wrapping_add(immediate));
        }
        Op::MovN => write_ar(next, s, read_ar(before, t)),
        Op::MoviN => {
            let value = decoded.imm;
            let signed = if value & 0x60 == 0x60 {
                value | !0x7f
            } else {
                value
            };
            write_ar(next, s, signed);
        }
        Op::L32iN | Op::S32iN => execute_narrow_memory(before, next, decoded, access)?,
        Op::BranchZeroN { ne } => {
            let taken = (read_ar(before, s) != 0) == ne;
            if taken {
                next.pc = before.pc.wrapping_add(4).wrapping_add(decoded.imm);
                return Ok(true);
            }
        }
        Op::NopN => {}
    }
    Ok(false)
}

fn branch_target(pc: u32, encoded: u32) -> u32 {
    pc.wrapping_add(4).wrapping_add(encoded)
}

fn b4const(index: u8) -> u32 {
    const VALUES: [i32; 16] = [-1, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 16, 32, 64, 128, 256];
    VALUES[index as usize] as u32
}
fn b4constu(index: u8) -> u32 {
    const VALUES: [u32; 16] = [
        32768, 65536, 2, 3, 4, 5, 6, 7, 8, 10, 12, 16, 32, 64, 128, 256,
    ];
    VALUES[index as usize]
}

fn execute_memory(
    before: &XtensaLx6State,
    next: &mut XtensaLx6State,
    operation: MemoryOp,
    decoded: &Decoded,
    access: &mut Option<DataAccess>,
) -> Result<(), XtensaLx6Error> {
    let (address, width, load, signed) = match operation {
        MemoryOp::L32r => {
            let base = before.pc.wrapping_add(3) & !3;
            let offset = (sign_extend(decoded.imm, 16) << 2) as u32;
            (base.wrapping_add(offset), 4, true, false)
        }
        MemoryOp::L8ui => (
            read_ar(before, decoded.s).wrapping_add(decoded.imm),
            1,
            true,
            false,
        ),
        MemoryOp::L16ui => (
            read_ar(before, decoded.s).wrapping_add(decoded.imm << 1),
            2,
            true,
            false,
        ),
        MemoryOp::L16si => (
            read_ar(before, decoded.s).wrapping_add(decoded.imm << 1),
            2,
            true,
            true,
        ),
        MemoryOp::L32i => (
            read_ar(before, decoded.s).wrapping_add(decoded.imm << 2),
            4,
            true,
            false,
        ),
        MemoryOp::S8i => (
            read_ar(before, decoded.s).wrapping_add(decoded.imm),
            1,
            false,
            false,
        ),
        MemoryOp::S16i => (
            read_ar(before, decoded.s).wrapping_add(decoded.imm << 1),
            2,
            false,
            false,
        ),
        MemoryOp::S32i => (
            read_ar(before, decoded.s).wrapping_add(decoded.imm << 2),
            4,
            false,
            false,
        ),
    };
    if load {
        let raw = read_memory(before, address, width)?;
        let value = if signed {
            sign_extend(raw, 16) as u32
        } else {
            raw
        };
        write_ar(next, decoded.t, value);
        *access = Some(DataAccess::Read {
            address,
            width,
            value: raw,
        });
    } else {
        let value = read_ar(before, decoded.t);
        write_memory(next, address, width, value)?;
        *access = Some(DataAccess::Write {
            address,
            width,
            value: value
                & if width == 1 {
                    0xff
                } else if width == 2 {
                    0xffff
                } else {
                    u32::MAX
                },
        });
    }
    Ok(())
}

fn execute_narrow_memory(
    before: &XtensaLx6State,
    next: &mut XtensaLx6State,
    decoded: &Decoded,
    access: &mut Option<DataAccess>,
) -> Result<(), XtensaLx6Error> {
    let address = read_ar(before, decoded.s).wrapping_add(decoded.imm << 2);
    match decoded.op {
        Op::L32iN => {
            let value = read_memory(before, address, 4)?;
            write_ar(next, decoded.t, value);
            *access = Some(DataAccess::Read {
                address,
                width: 4,
                value,
            });
        }
        Op::S32iN => {
            let value = read_ar(before, decoded.t);
            write_memory(next, address, 4, value)?;
            *access = Some(DataAccess::Write {
                address,
                width: 4,
                value,
            });
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn execute_special(
    before: &XtensaLx6State,
    next: &mut XtensaLx6State,
    operation: SpecialOp,
    number: u8,
    register: u8,
) -> Result<(), XtensaLx6Error> {
    let old = match number {
        SR_SAR => u32::from(before.sar),
        SR_LBEG => before.lbeg,
        SR_LEND => before.lend,
        SR_LCOUNT => before.lcount,
        _ => return Err(XtensaLx6Error::UnsupportedSpecialRegister { number }),
    };
    match operation {
        SpecialOp::Read => write_ar(next, register, old),
        SpecialOp::Write => write_special(next, number, read_ar(before, register)),
        SpecialOp::Exchange => {
            write_special(next, number, read_ar(before, register));
            write_ar(next, register, old);
        }
    }
    Ok(())
}

fn write_special(state: &mut XtensaLx6State, number: u8, value: u32) {
    match number {
        SR_SAR => state.sar = value as u8 & 63,
        SR_LBEG => state.lbeg = value,
        SR_LEND => state.lend = value,
        SR_LCOUNT => state.lcount = value,
        _ => unreachable!(),
    }
}

fn execute_retw(
    before: &XtensaLx6State,
    next: &mut XtensaLx6State,
    raw: u32,
    width: u8,
) -> Result<(), XtensaLx6Error> {
    let link = read_ar(before, 0);
    let increment = (link >> 30) as u8;
    let first = (1..=3)
        .find(|back| {
            before.window_start & (1u16 << (before.window_base.wrapping_sub(*back) & 15)) != 0
        })
        .unwrap_or(0);
    if increment == 0 || (first != 0 && first != increment) {
        return Err(XtensaLx6Error::UndefinedOperation {
            raw,
            width,
            reason: UndefinedReason::InvalidReturnWindow,
        });
    }
    let old_wb = before.window_base;
    let new_wb = old_wb.wrapping_sub(increment) & 15;
    if before.window_start & (1u16 << new_wb) == 0 {
        return Err(XtensaLx6Error::WindowUnderflow { increment });
    }
    next.window_base = new_wb;
    next.window_start &= !(1u16 << old_wb);
    next.pc = (before.pc & 0xc000_0000) | (link & 0x3fff_ffff);
    Ok(())
}

fn validate_loop(state: &XtensaLx6State, lbeg: u32, lend: u32) -> Result<(), XtensaLx6Error> {
    let origin = state.origin.ok_or(XtensaLx6Error::InvalidLoop)?;
    let end = origin
        .checked_add(state.length)
        .ok_or(XtensaLx6Error::InvalidLoop)?;
    if lbeg < origin || lbeg >= lend || lend > end {
        return Err(XtensaLx6Error::InvalidLoop);
    }
    let p = lbeg as usize;
    if p + 2 > state.memory.len() {
        return Err(XtensaLx6Error::InvalidLoop);
    }
    let width = if state.memory[p] & 8 != 0 { 2 } else { 3 };
    if lbeg + width > lend || (lbeg >> 2) != ((lbeg + width - 1) >> 2) {
        return Err(XtensaLx6Error::InvalidLoop);
    }
    Ok(())
}

#[cfg(test)]
mod window_group_regressions {
    use super::{
        d, window_groups, Alu, BranchImm, BranchReg, BranchZero, ConditionalMove, LoopOp, MemoryOp,
        Op, SarShift, SpecialOp, SyncOp, XtensaLx6State,
    };

    fn assert_groups(state: &XtensaLx6State, operations: &[Op], expected: Option<[u8; 3]>) {
        for operation in operations {
            let decoded = d(*operation, 15, 11, 7, 0);
            assert_eq!(window_groups(state, &decoded), expected, "{operation:?}");
        }
    }

    #[test]
    fn every_included_operation_declares_its_exact_ar_reference_groups() {
        let state = XtensaLx6State {
            call_inc: 3,
            ..XtensaLx6State::default()
        };
        assert_groups(
            &state,
            &[
                Op::Alu(Alu::Add),
                Op::Alu(Alu::AddX2),
                Op::Alu(Alu::AddX4),
                Op::Alu(Alu::AddX8),
                Op::Alu(Alu::Sub),
                Op::Alu(Alu::SubX2),
                Op::Alu(Alu::SubX4),
                Op::Alu(Alu::SubX8),
                Op::Alu(Alu::And),
                Op::Alu(Alu::Or),
                Op::Alu(Alu::Xor),
                Op::ConditionalMove(ConditionalMove::Eqz),
                Op::ConditionalMove(ConditionalMove::Nez),
                Op::ConditionalMove(ConditionalMove::Ltz),
                Op::ConditionalMove(ConditionalMove::Gez),
                Op::AddN,
            ],
            Some([3, 2, 1]),
        );
        assert_groups(&state, &[Op::Neg, Op::Abs], Some([3, 0, 1]));
        assert_groups(&state, &[Op::Addi, Op::Addmi], Some([0, 2, 1]));
        assert_groups(&state, &[Op::Movi], Some([0, 0, 1]));
        assert_groups(&state, &[Op::Extui], Some([3, 0, 1]));
        assert_groups(&state, &[Op::ShiftImmediateLeft], Some([3, 2, 0]));
        assert_groups(
            &state,
            &[Op::ShiftImmediateArithmetic, Op::ShiftImmediateLogical],
            Some([3, 0, 1]),
        );
        assert_groups(&state, &[Op::SarShift(SarShift::Sll)], Some([3, 2, 0]));
        assert_groups(
            &state,
            &[Op::SarShift(SarShift::Sra), Op::SarShift(SarShift::Srl)],
            Some([3, 0, 1]),
        );
        assert_groups(&state, &[Op::SarShift(SarShift::Src)], Some([3, 2, 1]));
        assert_groups(
            &state,
            &[Op::Ssa8b, Op::Ssa8l, Op::Ssl, Op::Ssr],
            Some([0, 2, 0]),
        );
        assert_groups(&state, &[Op::Memory(MemoryOp::L32r)], Some([0, 0, 1]));
        assert_groups(
            &state,
            &[
                Op::Memory(MemoryOp::L8ui),
                Op::Memory(MemoryOp::L16ui),
                Op::Memory(MemoryOp::L16si),
                Op::Memory(MemoryOp::L32i),
                Op::Memory(MemoryOp::S8i),
                Op::Memory(MemoryOp::S16i),
                Op::Memory(MemoryOp::S32i),
                Op::L32iN,
                Op::S32iN,
            ],
            Some([0, 2, 1]),
        );
        assert_groups(
            &state,
            &[
                Op::BranchReg(BranchReg::None),
                Op::BranchReg(BranchReg::All),
                Op::BranchReg(BranchReg::Any),
                Op::BranchReg(BranchReg::NotAll),
                Op::BranchReg(BranchReg::Eq),
                Op::BranchReg(BranchReg::Ne),
                Op::BranchReg(BranchReg::Lt),
                Op::BranchReg(BranchReg::Ge),
                Op::BranchReg(BranchReg::Ltu),
                Op::BranchReg(BranchReg::Geu),
                Op::BranchReg(BranchReg::BitClear),
                Op::BranchReg(BranchReg::BitSet),
            ],
            Some([0, 2, 1]),
        );
        assert_groups(
            &state,
            &[
                Op::BranchImm(BranchImm::Eq),
                Op::BranchImm(BranchImm::Ne),
                Op::BranchImm(BranchImm::Lt),
                Op::BranchImm(BranchImm::Ge),
                Op::BranchImm(BranchImm::Ltu),
                Op::BranchImm(BranchImm::Geu),
                Op::BranchZero(BranchZero::Eq),
                Op::BranchZero(BranchZero::Ne),
                Op::BranchZero(BranchZero::Lt),
                Op::BranchZero(BranchZero::Ge),
                Op::BranchBitImmediate { set: false },
                Op::BranchBitImmediate { set: true },
                Op::Loop(LoopOp::Always),
                Op::Loop(LoopOp::Gtz),
                Op::Loop(LoopOp::Nez),
                Op::BranchZeroN { ne: false },
                Op::BranchZeroN { ne: true },
            ],
            Some([0, 2, 0]),
        );
        assert_groups(&state, &[Op::JumpRegister], Some([0, 2, 0]));
        assert_groups(&state, &[Op::Movsp, Op::MovN], Some([0, 2, 1]));
        assert_groups(
            &state,
            &[
                Op::Special(SpecialOp::Read),
                Op::Special(SpecialOp::Write),
                Op::Special(SpecialOp::Exchange),
            ],
            Some([0, 0, 1]),
        );
        assert_groups(&state, &[Op::AddiN], Some([3, 2, 0]));
        assert_groups(&state, &[Op::MoviN], Some([0, 2, 0]));

        for increment in 0..=3 {
            assert_groups(
                &state,
                &[
                    Op::Call {
                        increment,
                        indirect: false,
                    },
                    Op::Call {
                        increment,
                        indirect: true,
                    },
                ],
                Some([0, 0, increment]),
            );
        }
        assert_groups(&state, &[Op::Entry], Some([0, 3, 0]));
        assert_groups(&state, &[Op::Retw, Op::RetwN], None);
        assert_groups(
            &state,
            &[
                Op::Ssai,
                Op::Sync(SyncOp::Isync),
                Op::Sync(SyncOp::Rsync),
                Op::Sync(SyncOp::Esync),
                Op::Sync(SyncOp::Dsync),
                Op::Sync(SyncOp::Memw),
                Op::Sync(SyncOp::Extw),
                Op::Nop,
                Op::Halt,
                Op::NopN,
                Op::Jump,
                Op::Return,
                Op::ReturnN,
            ],
            Some([0, 0, 0]),
        );
    }
}

#[cfg(test)]
mod allocation_regressions {
    use super::{fail_allocation_after, XtensaLx6Error, XtensaLx6Simulator};

    const HALT: [u8; 3] = [0x00, 0x40, 0x00];

    fn loaded_halt() -> XtensaLx6Simulator {
        let mut simulator = XtensaLx6Simulator::new();
        simulator.load(&HALT, 0x100).unwrap();
        simulator
    }

    #[test]
    fn snapshot_allocation_failure_is_typed_and_atomic() {
        let simulator = loaded_halt();
        let before = simulator.state.clone();
        fail_allocation_after(0);
        assert_eq!(simulator.snapshot(), Err(XtensaLx6Error::AllocationFailed));
        assert_eq!(simulator.state, before);
    }

    #[test]
    fn every_step_snapshot_allocation_boundary_is_atomic() {
        for successful_allocations in 0..=2 {
            let mut simulator = loaded_halt();
            let before = simulator.state.clone();
            fail_allocation_after(successful_allocations);
            assert_eq!(simulator.step(), Err(XtensaLx6Error::AllocationFailed));
            assert_eq!(simulator.state, before);
        }
    }

    #[test]
    fn trace_reservation_failure_restores_run_entry_state() {
        let mut simulator = loaded_halt();
        let before = simulator.state.clone();
        fail_allocation_after(1);
        assert_eq!(simulator.run(1), Err(XtensaLx6Error::AllocationFailed));
        assert_eq!(simulator.state, before);
    }

    #[test]
    fn final_state_allocation_failure_restores_whole_run() {
        let mut simulator = loaded_halt();
        let before = simulator.state.clone();
        fail_allocation_after(5);
        assert_eq!(simulator.run(1), Err(XtensaLx6Error::AllocationFailed));
        assert_eq!(simulator.state, before);
    }

    #[test]
    fn final_state_allocation_failure_restores_execute_entry_state() {
        let mut simulator = loaded_halt();
        let before = simulator.state.clone();
        fail_allocation_after(6);
        assert_eq!(
            simulator.execute(&HALT, 0x200, 1),
            Err(XtensaLx6Error::AllocationFailed)
        );
        assert_eq!(simulator.state, before);
    }
}
