//! Checked, deterministic model of the bounded SPARC V9 problem-state integer
//! contract in repository specification 07aw.

use core::fmt;
#[cfg(test)]
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
static FORCE_ALLOCATION_FAILURE: AtomicBool = AtomicBool::new(false);

/// Size of the flat teaching memory.
pub const MEMORY_SIZE: usize = 65_536;
/// Eight normal globals plus four 16-register window bodies.
pub const NUM_PHYSICAL_REGISTERS: usize = 72;
/// Maximum complete-state traces retained by one run.
pub const MAX_RETAINED_STEPS: usize = 64;
const WINDOWS: u8 = 4;

/// N/Z/V/C flags for either the low 32-bit or full 64-bit result.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConditionCodes {
    pub n: bool,
    pub z: bool,
    pub v: bool,
    pub c: bool,
}

/// SPARC V9's two integer condition-code banks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConditionCodeRegister {
    pub icc: ConditionCodes,
    pub xcc: ConditionCodes,
}

/// Complete owned state at an instruction boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SparcV9State {
    pub registers: [u64; NUM_PHYSICAL_REGISTERS],
    pub pc: u64,
    pub npc: u64,
    pub ccr: ConditionCodeRegister,
    /// Only the low 32 bits are architectural; restore rejects a nonzero high half.
    pub y: u64,
    pub cwp: u8,
    pub cansave: u8,
    pub canrestore: u8,
    pub cleanwin: u8,
    pub otherwin: u8,
    pub wstate: u8,
    pub memory: Vec<u8>,
    pub halted: bool,
    /// True when the instruction at `pc` is a delayed control transfer's slot.
    pub delay_slot: bool,
    pub loaded_origin: u64,
    pub loaded_len: usize,
}

impl Default for SparcV9State {
    fn default() -> Self {
        Self {
            registers: [0; NUM_PHYSICAL_REGISTERS],
            pc: 0,
            npc: 4,
            ccr: ConditionCodeRegister::default(),
            y: 0,
            cwp: 0,
            cansave: WINDOWS - 2,
            canrestore: 0,
            cleanwin: WINDOWS - 2,
            otherwin: 0,
            wstate: 0,
            memory: vec![0; MEMORY_SIZE],
            halted: false,
            delay_slot: false,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }
}

/// Broad decoded family used by boundary/classification tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstructionClass {
    Call,
    Sethi,
    Branch,
    PredictedBranch,
    RegisterBranch,
    Arithmetic,
    Logical,
    Shift,
    MultiplyDivide,
    ConditionalMove,
    RegisterMove,
    Barrier,
    ControlTransfer,
    Window,
    Memory,
    Trap,
}

/// Checked lifecycle, transport, decode, and execution failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SparcV9Error {
    EmptyProgram,
    MisalignedProgram { origin: u64, len: usize },
    ProgramOutOfRange { origin: u64, len: usize },
    MisalignedFetch { pc: u64 },
    FetchOutsideProgram { pc: u64 },
    MisalignedData { address: u64, width: usize },
    DataOutOfRange { address: u64, width: usize },
    MisalignedTarget { target: u64 },
    InvalidRegister { index: u8 },
    InvalidRegisterPair { index: u8 },
    InvalidState { reason: &'static str },
    UnknownInstruction { word: u32, pc: u64 },
    UnsupportedInstruction { word: u32, pc: u64 },
    PrivilegedInstruction { word: u32, pc: u64 },
    DivideByZero,
    WindowSpill,
    WindowFill,
    CleanWindow,
    UnsupportedTrap { number: u8, pc: u64 },
    Halted,
    RetentionLimitExceeded { requested: usize, limit: usize },
    AllocationFailed,
    StepLimitExceeded { limit: usize },
}

impl fmt::Display for SparcV9Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => f.write_str("program image is empty"),
            Self::MisalignedProgram { origin, len } => write!(
                f,
                "program at {origin:#018x} with {len} bytes must be word aligned"
            ),
            Self::ProgramOutOfRange { origin, len } => {
                write!(f, "program of {len} bytes does not fit at {origin:#018x}")
            }
            Self::MisalignedFetch { pc } => write!(f, "fetch at {pc:#018x} is misaligned"),
            Self::FetchOutsideProgram { pc } => {
                write!(f, "fetch at {pc:#018x} is outside the installed program")
            }
            Self::MisalignedData { address, width } => {
                write!(f, "{width}-byte access at {address:#018x} is misaligned")
            }
            Self::DataOutOfRange { address, width } => {
                write!(f, "{width}-byte access at {address:#018x} is out of range")
            }
            Self::MisalignedTarget { target } => {
                write!(f, "control-transfer target {target:#018x} is misaligned")
            }
            Self::InvalidRegister { index } => write!(f, "register r{index} does not exist"),
            Self::InvalidRegisterPair { index } => {
                write!(f, "register pair beginning at r{index} must be even")
            }
            Self::InvalidState { reason } => write!(f, "invalid SPARC V9 state: {reason}"),
            Self::UnknownInstruction { word, pc } => {
                write!(f, "unknown instruction {word:#010x} at {pc:#018x}")
            }
            Self::UnsupportedInstruction { word, pc } => {
                write!(f, "unsupported instruction {word:#010x} at {pc:#018x}")
            }
            Self::PrivilegedInstruction { word, pc } => {
                write!(f, "privileged instruction {word:#010x} at {pc:#018x}")
            }
            Self::DivideByZero => f.write_str("integer division by zero"),
            Self::WindowSpill => f.write_str("register-window spill is required"),
            Self::WindowFill => f.write_str("register-window fill is required"),
            Self::CleanWindow => f.write_str("clean-window trap is required"),
            Self::UnsupportedTrap { number, pc } => {
                write!(
                    f,
                    "software trap {number} at {pc:#018x} is outside this CPU cell"
                )
            }
            Self::Halted => f.write_str("processor is halted"),
            Self::RetentionLimitExceeded { requested, limit } => {
                write!(f, "requested {requested} traces exceeds limit {limit}")
            }
            Self::AllocationFailed => f.write_str("trace allocation failed"),
            Self::StepLimitExceeded { limit } => write!(f, "step limit {limit} exceeded"),
        }
    }
}

impl std::error::Error for SparcV9Error {}

/// One successful instruction transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u64,
    pub npc_before: u64,
    pub pc_after: u64,
    pub npc_after: u64,
    pub raw: [u8; 4],
    pub raw_word: u32,
    pub mnemonic: &'static str,
    /// The instruction itself occupied a previously scheduled delay slot.
    pub in_delay_slot: bool,
    /// This instruction scheduled a following delay slot.
    pub scheduled_delay: bool,
    /// This instruction annulled/skipped its architectural delay slot.
    pub annulled: bool,
    /// Address skipped by annulment, if any.
    pub skipped_pc: Option<u64>,
    pub state_after: SparcV9State,
}

/// Successful bounded execution result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: SparcV9State,
}

/// Functional simulator for bounded SPARC V9 core.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SparcV9Simulator {
    state: SparcV9State,
}

impl SparcV9Simulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.state = SparcV9State::default();
    }

    #[must_use]
    pub fn get_state(&self) -> SparcV9State {
        self.state.clone()
    }

    pub fn restore(&mut self, state: SparcV9State) -> Result<(), SparcV9Error> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    pub fn load(&mut self, program: &[u8], origin: u64) -> Result<(), SparcV9Error> {
        validate_program(program, origin)?;
        let start = usize::try_from(origin).map_err(|_| SparcV9Error::ProgramOutOfRange {
            origin,
            len: program.len(),
        })?;
        let end = start + program.len();
        let mut state = SparcV9State {
            pc: origin,
            npc: origin + 4,
            loaded_origin: origin,
            loaded_len: program.len(),
            ..SparcV9State::default()
        };
        state.memory[start..end].copy_from_slice(program);
        self.state = state;
        Ok(())
    }

    pub fn read_register(&self, index: u8) -> Result<u64, SparcV9Error> {
        read_register(&self.state, index)
    }

    pub fn write_register(&mut self, index: u8, value: u64) -> Result<(), SparcV9Error> {
        write_register(&mut self.state, index, value)
    }

    pub fn read_memory(&self, address: u64, width: usize) -> Result<u64, SparcV9Error> {
        read_be(&self.state, address, width)
    }

    pub fn write_memory(
        &mut self,
        address: u64,
        width: usize,
        value: u64,
    ) -> Result<(), SparcV9Error> {
        write_be(&mut self.state, address, width, value)
    }

    pub fn step(&mut self) -> Result<StepTrace, SparcV9Error> {
        if self.state.halted {
            return Err(SparcV9Error::Halted);
        }
        let pc = self.state.pc;
        let npc = self.state.npc;
        let raw = fetch_bytes(&self.state)?;
        let word = u32::from_be_bytes(raw);
        let class = classify_at(word, pc)?;
        let in_delay_slot = self.state.delay_slot;
        let mut next = self.state.clone();
        let outcome = execute_word(&mut next, word, class)?;
        let trace = StepTrace {
            pc_before: pc,
            npc_before: npc,
            pc_after: next.pc,
            npc_after: next.npc,
            raw,
            raw_word: word,
            mnemonic: outcome.mnemonic,
            in_delay_slot,
            scheduled_delay: outcome.scheduled_delay,
            annulled: outcome.annulled,
            skipped_pc: outcome.skipped_pc,
            state_after: next.clone(),
        };
        self.state = next;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, SparcV9Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(SparcV9Error::RetentionLimitExceeded {
                requested: max_steps,
                limit: MAX_RETAINED_STEPS,
            });
        }
        let before = self.state.clone();
        #[cfg(test)]
        if FORCE_ALLOCATION_FAILURE.swap(false, Ordering::SeqCst) {
            return Err(SparcV9Error::AllocationFailed);
        }
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| SparcV9Error::AllocationFailed)?;
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.state = before;
                    return Err(error);
                }
            }
            if self.state.halted {
                return Ok(ExecutionResult {
                    halted: true,
                    steps: traces.len(),
                    traces,
                    final_state: self.state.clone(),
                });
            }
        }
        self.state = before;
        Err(SparcV9Error::StepLimitExceeded { limit: max_steps })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u64,
        max_steps: usize,
    ) -> Result<ExecutionResult, SparcV9Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(SparcV9Error::RetentionLimitExceeded {
                requested: max_steps,
                limit: MAX_RETAINED_STEPS,
            });
        }
        let before = self.state.clone();
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = before;
                Err(error)
            }
        }
    }
}

fn validate_program(program: &[u8], origin: u64) -> Result<(), SparcV9Error> {
    if program.is_empty() {
        return Err(SparcV9Error::EmptyProgram);
    }
    if origin & 3 != 0 || program.len() & 3 != 0 {
        return Err(SparcV9Error::MisalignedProgram {
            origin,
            len: program.len(),
        });
    }
    let start = usize::try_from(origin).map_err(|_| SparcV9Error::ProgramOutOfRange {
        origin,
        len: program.len(),
    })?;
    if start
        .checked_add(program.len())
        .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(SparcV9Error::ProgramOutOfRange {
            origin,
            len: program.len(),
        });
    }
    Ok(())
}

fn validate_state(state: &SparcV9State) -> Result<(), SparcV9Error> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(SparcV9Error::InvalidState {
            reason: "memory size must be exactly 64 KiB",
        });
    }
    if state.registers[0] != 0 {
        return Err(SparcV9Error::InvalidState {
            reason: "physical g0 must be zero",
        });
    }
    if state.cwp >= WINDOWS {
        return Err(SparcV9Error::InvalidState {
            reason: "CWP is out of range",
        });
    }
    if state.cansave >= WINDOWS
        || state.canrestore >= WINDOWS
        || state.otherwin >= WINDOWS
        || state.cleanwin >= WINDOWS
        || state.cleanwin < state.canrestore
        || state.cansave + state.canrestore + state.otherwin != WINDOWS - 2
    {
        return Err(SparcV9Error::InvalidState {
            reason: "register-window counters are inconsistent",
        });
    }
    if state.wstate & !0x3f != 0 {
        return Err(SparcV9Error::InvalidState {
            reason: "WSTATE contains reserved bits",
        });
    }
    if state.y >> 32 != 0 {
        return Err(SparcV9Error::InvalidState {
            reason: "Y contains nonzero reserved upper bits",
        });
    }
    if state.pc & 3 != 0 || state.npc & 3 != 0 {
        return Err(SparcV9Error::InvalidState {
            reason: "PC and nPC must be word aligned",
        });
    }
    if state.loaded_len == 0 {
        if state.loaded_origin != 0 {
            return Err(SparcV9Error::InvalidState {
                reason: "an empty installed image must have origin zero",
            });
        }
    } else if state.loaded_origin & 3 != 0
        || state.loaded_len & 3 != 0
        || usize::try_from(state.loaded_origin)
            .ok()
            .and_then(|origin| origin.checked_add(state.loaded_len))
            .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(SparcV9Error::InvalidState {
            reason: "installed program range is invalid",
        });
    }
    Ok(())
}

fn register_physical_index(state: &SparcV9State, index: u8) -> Result<usize, SparcV9Error> {
    if index >= 32 {
        return Err(SparcV9Error::InvalidRegister { index });
    }
    let physical = match index {
        0..=7 => usize::from(index),
        8..=23 => 8 + usize::from(state.cwp) * 16 + usize::from(index - 8),
        _ => {
            let caller = (state.cwp + WINDOWS - 1) % WINDOWS;
            8 + usize::from(caller) * 16 + usize::from(index - 24)
        }
    };
    Ok(physical)
}

fn read_register(state: &SparcV9State, index: u8) -> Result<u64, SparcV9Error> {
    if index == 0 {
        return Ok(0);
    }
    Ok(state.registers[register_physical_index(state, index)?])
}

fn write_register(state: &mut SparcV9State, index: u8, value: u64) -> Result<(), SparcV9Error> {
    if index == 0 {
        return Ok(());
    }
    let physical = register_physical_index(state, index)?;
    state.registers[physical] = value;
    Ok(())
}

fn checked_range(address: u64, width: usize) -> Result<std::ops::Range<usize>, SparcV9Error> {
    if !matches!(width, 1 | 2 | 4 | 8) {
        return Err(SparcV9Error::DataOutOfRange { address, width });
    }
    if !address.is_multiple_of(width as u64) {
        return Err(SparcV9Error::MisalignedData { address, width });
    }
    let start =
        usize::try_from(address).map_err(|_| SparcV9Error::DataOutOfRange { address, width })?;
    let end = start
        .checked_add(width)
        .filter(|end| *end <= MEMORY_SIZE)
        .ok_or(SparcV9Error::DataOutOfRange { address, width })?;
    Ok(start..end)
}

fn read_be(state: &SparcV9State, address: u64, width: usize) -> Result<u64, SparcV9Error> {
    let range = checked_range(address, width)?;
    Ok(state.memory[range]
        .iter()
        .fold(0_u64, |value, byte| (value << 8) | u64::from(*byte)))
}

fn write_be(
    state: &mut SparcV9State,
    address: u64,
    width: usize,
    value: u64,
) -> Result<(), SparcV9Error> {
    let range = checked_range(address, width)?;
    for (offset, byte) in state.memory[range].iter_mut().enumerate() {
        let shift = (width - offset - 1) * 8;
        *byte = (value >> shift) as u8;
    }
    Ok(())
}

fn fetch_bytes(state: &SparcV9State) -> Result<[u8; 4], SparcV9Error> {
    if state.pc & 3 != 0 {
        return Err(SparcV9Error::MisalignedFetch { pc: state.pc });
    }
    if state.loaded_len < 4 {
        return Err(SparcV9Error::FetchOutsideProgram { pc: state.pc });
    }
    let relative = state.pc.checked_sub(state.loaded_origin);
    if relative.is_none_or(|offset| offset > state.loaded_len.saturating_sub(4) as u64) {
        return Err(SparcV9Error::FetchOutsideProgram { pc: state.pc });
    }
    let start = usize::try_from(state.pc)
        .map_err(|_| SparcV9Error::FetchOutsideProgram { pc: state.pc })?;
    let bytes: [u8; 4] = state.memory[start..start + 4]
        .try_into()
        .expect("validated fetch range is four bytes");
    Ok(bytes)
}

fn sign_extend(value: u32, bits: u32) -> i64 {
    let shift = 64 - bits;
    ((u64::from(value) << shift) as i64) >> shift
}

fn operand2(state: &SparcV9State, word: u32) -> Result<u64, SparcV9Error> {
    if word & (1 << 13) != 0 {
        Ok(sign_extend(word & 0x1fff, 13) as u64)
    } else {
        if word & 0x1fe0 != 0 {
            return Err(SparcV9Error::UnknownInstruction { word, pc: state.pc });
        }
        read_register(state, (word & 0x1f) as u8)
    }
}

fn rs1(word: u32) -> u8 {
    ((word >> 14) & 0x1f) as u8
}

fn rd(word: u32) -> u8 {
    ((word >> 25) & 0x1f) as u8
}

fn op3(word: u32) -> u8 {
    ((word >> 19) & 0x3f) as u8
}

/// Classify one raw word without executing it.
pub fn classify_word(word: u32) -> Result<InstructionClass, SparcV9Error> {
    classify_at(word, 0)
}

fn classify_at(word: u32, pc: u64) -> Result<InstructionClass, SparcV9Error> {
    match word >> 30 {
        1 => Ok(InstructionClass::Call),
        0 => match (word >> 22) & 7 {
            1 => Ok(InstructionClass::PredictedBranch),
            2 => Ok(InstructionClass::Branch),
            3 => Ok(InstructionClass::RegisterBranch),
            4 => Ok(InstructionClass::Sethi),
            5..=7 => Err(SparcV9Error::UnsupportedInstruction { word, pc }),
            _ => Err(SparcV9Error::UnknownInstruction { word, pc }),
        },
        2 => match op3(word) {
            0x00 | 0x04 | 0x08 | 0x0c | 0x10 | 0x14 | 0x18 | 0x1c => {
                Ok(InstructionClass::Arithmetic)
            }
            0x01..=0x03 | 0x05..=0x07 | 0x11..=0x13 | 0x15..=0x17 => Ok(InstructionClass::Logical),
            0x25..=0x27 => Ok(InstructionClass::Shift),
            0x09..=0x0b | 0x0d..=0x0f | 0x1a | 0x1b | 0x1e | 0x1f | 0x2d | 0x2e => {
                Ok(InstructionClass::MultiplyDivide)
            }
            0x2c => Ok(InstructionClass::ConditionalMove),
            0x2f => Ok(InstructionClass::RegisterMove),
            0x28 => Ok(InstructionClass::Barrier),
            0x38 | 0x39 => Ok(InstructionClass::ControlTransfer),
            0x3c | 0x3d => Ok(InstructionClass::Window),
            0x3a => Ok(InstructionClass::Trap),
            0x29..=0x2b | 0x30..=0x37 | 0x3b | 0x3e | 0x3f => {
                Err(SparcV9Error::UnsupportedInstruction { word, pc })
            }
            _ => Err(SparcV9Error::UnknownInstruction { word, pc }),
        },
        3 => match op3(word) {
            0x00..=0x0b | 0x0e => Ok(InstructionClass::Memory),
            0x10..=0x3f => Err(SparcV9Error::UnsupportedInstruction { word, pc }),
            _ => Err(SparcV9Error::UnknownInstruction { word, pc }),
        },
        _ => unreachable!(),
    }
}

fn advance(state: &mut SparcV9State) {
    state.pc = state.npc;
    state.npc = state.npc.wrapping_add(4);
    state.delay_slot = false;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Outcome {
    mnemonic: &'static str,
    scheduled_delay: bool,
    annulled: bool,
    skipped_pc: Option<u64>,
}

impl Outcome {
    const fn ordinary(mnemonic: &'static str) -> Self {
        Self {
            mnemonic,
            scheduled_delay: false,
            annulled: false,
            skipped_pc: None,
        }
    }

    const fn delayed(mnemonic: &'static str) -> Self {
        Self {
            mnemonic,
            scheduled_delay: true,
            annulled: false,
            skipped_pc: None,
        }
    }

    const fn annulled(mnemonic: &'static str, skipped_pc: u64) -> Self {
        Self {
            mnemonic,
            scheduled_delay: false,
            annulled: true,
            skipped_pc: Some(skipped_pc),
        }
    }
}

fn delayed_transfer(state: &mut SparcV9State, target: u64) -> Result<(), SparcV9Error> {
    if target & 3 != 0 {
        return Err(SparcV9Error::MisalignedTarget { target });
    }
    state.pc = state.npc;
    state.npc = target;
    state.delay_slot = true;
    Ok(())
}

fn branch_condition(condition: u8, cc: ConditionCodes) -> bool {
    match condition & 0xf {
        0 => false,
        1 => cc.z,
        2 => cc.z || cc.n != cc.v,
        3 => cc.n != cc.v,
        4 => cc.c || cc.z,
        5 => cc.c,
        6 => cc.n,
        7 => cc.v,
        8 => true,
        9 => !cc.z,
        10 => !cc.z && cc.n == cc.v,
        11 => cc.n == cc.v,
        12 => !cc.c && !cc.z,
        13 => !cc.c,
        14 => !cc.n,
        _ => !cc.v,
    }
}

fn register_condition(condition: u8, value: i64) -> Option<bool> {
    Some(match condition {
        1 => value == 0,
        2 => value <= 0,
        3 => value < 0,
        5 => value != 0,
        6 => value > 0,
        7 => value >= 0,
        _ => return None,
    })
}

fn add_flags_32(a: u32, b: u32, carry: bool) -> ConditionCodes {
    let (partial, c1) = a.overflowing_add(b);
    let (result, c2) = partial.overflowing_add(u32::from(carry));
    ConditionCodes {
        n: (result as i32) < 0,
        z: result == 0,
        v: {
            let exact = i128::from(a as i32) + i128::from(b as i32) + i128::from(carry);
            exact < i128::from(i32::MIN) || exact > i128::from(i32::MAX)
        },
        c: c1 || c2,
    }
}

fn add_flags_64(a: u64, b: u64, carry: bool) -> ConditionCodes {
    let (partial, c1) = a.overflowing_add(b);
    let (result, c2) = partial.overflowing_add(u64::from(carry));
    ConditionCodes {
        n: (result as i64) < 0,
        z: result == 0,
        v: {
            let exact = i128::from(a as i64) + i128::from(b as i64) + i128::from(carry);
            exact < i128::from(i64::MIN) || exact > i128::from(i64::MAX)
        },
        c: c1 || c2,
    }
}

fn sub_flags_32(a: u32, b: u32, borrow: bool) -> ConditionCodes {
    let (partial, b1) = a.overflowing_sub(b);
    let (result, b2) = partial.overflowing_sub(u32::from(borrow));
    ConditionCodes {
        n: (result as i32) < 0,
        z: result == 0,
        v: {
            let exact = i128::from(a as i32) - i128::from(b as i32) - i128::from(borrow);
            exact < i128::from(i32::MIN) || exact > i128::from(i32::MAX)
        },
        c: b1 || b2,
    }
}

fn sub_flags_64(a: u64, b: u64, borrow: bool) -> ConditionCodes {
    let (partial, b1) = a.overflowing_sub(b);
    let (result, b2) = partial.overflowing_sub(u64::from(borrow));
    ConditionCodes {
        n: (result as i64) < 0,
        z: result == 0,
        v: {
            let exact = i128::from(a as i64) - i128::from(b as i64) - i128::from(borrow);
            exact < i128::from(i64::MIN) || exact > i128::from(i64::MAX)
        },
        c: b1 || b2,
    }
}

fn logic_flags(result: u64, bits: u32) -> ConditionCodes {
    let value = if bits == 32 {
        result & 0xffff_ffff
    } else {
        result
    };
    ConditionCodes {
        n: if bits == 32 {
            value & (1 << 31) != 0
        } else {
            value & (1 << 63) != 0
        },
        z: value == 0,
        v: false,
        c: false,
    }
}

fn set_add_flags(state: &mut SparcV9State, a: u64, b: u64, carry: bool) {
    state.ccr.icc = add_flags_32(a as u32, b as u32, carry);
    state.ccr.xcc = add_flags_64(a, b, carry);
}

fn set_sub_flags(state: &mut SparcV9State, a: u64, b: u64, borrow: bool) {
    state.ccr.icc = sub_flags_32(a as u32, b as u32, borrow);
    state.ccr.xcc = sub_flags_64(a, b, borrow);
}

#[allow(clippy::too_many_lines)]
fn execute_word(
    state: &mut SparcV9State,
    word: u32,
    class: InstructionClass,
) -> Result<Outcome, SparcV9Error> {
    let old_pc = state.pc;
    match class {
        InstructionClass::Call => {
            let displacement = sign_extend(word & 0x3fff_ffff, 30) << 2;
            let target = old_pc.wrapping_add_signed(displacement);
            write_register(state, 15, old_pc)?;
            delayed_transfer(state, target)?;
            Ok(Outcome::delayed("CALL"))
        }
        InstructionClass::Sethi => {
            let destination = rd(word);
            let value = u64::from((word & 0x3f_ffff) << 10);
            write_register(state, destination, value)?;
            advance(state);
            Ok(Outcome::ordinary(if destination == 0 && value == 0 {
                "NOP"
            } else {
                "SETHI"
            }))
        }
        InstructionClass::Branch | InstructionClass::PredictedBranch => {
            let condition = ((word >> 25) & 0xf) as u8;
            let annul = word & (1 << 29) != 0;
            let (cc, displacement) = if class == InstructionClass::PredictedBranch {
                let selector = (word >> 20) & 3;
                let cc = match selector {
                    0 => state.ccr.icc,
                    2 => state.ccr.xcc,
                    _ => {
                        return Err(SparcV9Error::UnknownInstruction { word, pc: old_pc });
                    }
                };
                (cc, sign_extend(word & 0x7_ffff, 19) << 2)
            } else {
                (state.ccr.icc, sign_extend(word & 0x3f_ffff, 22) << 2)
            };
            let taken = branch_condition(condition, cc);
            let target = old_pc.wrapping_add_signed(displacement);
            if target & 3 != 0 {
                return Err(SparcV9Error::MisalignedTarget { target });
            }
            let skipped = state.npc;
            let annulled = annul && (condition == 8 || !taken);
            if annulled {
                if condition == 8 {
                    state.pc = target;
                    state.npc = target.wrapping_add(4);
                } else {
                    state.pc = state.npc.wrapping_add(4);
                    state.npc = state.npc.wrapping_add(8);
                }
                state.delay_slot = false;
            } else {
                let next = if taken {
                    target
                } else {
                    state.npc.wrapping_add(4)
                };
                delayed_transfer(state, next)?;
            }
            let mnemonic = if class == InstructionClass::PredictedBranch {
                predicted_branch_mnemonic(condition)
            } else {
                integer_branch_mnemonic(condition)
            };
            Ok(if annulled {
                Outcome::annulled(mnemonic, skipped)
            } else {
                Outcome::delayed(mnemonic)
            })
        }
        InstructionClass::RegisterBranch => execute_register_branch(state, word),
        InstructionClass::Arithmetic => execute_arithmetic(state, word).map(Outcome::ordinary),
        InstructionClass::Logical => execute_logical(state, word).map(Outcome::ordinary),
        InstructionClass::Shift => execute_shift(state, word).map(Outcome::ordinary),
        InstructionClass::MultiplyDivide => {
            execute_multiply_divide(state, word).map(Outcome::ordinary)
        }
        InstructionClass::ConditionalMove => execute_movcc(state, word).map(Outcome::ordinary),
        InstructionClass::RegisterMove => execute_movr(state, word).map(Outcome::ordinary),
        InstructionClass::Barrier => execute_membar(state, word).map(Outcome::ordinary),
        InstructionClass::ControlTransfer => {
            let a = read_register(state, rs1(word))?;
            let b = operand2(state, word)?;
            let target = a.wrapping_add(b);
            if target & 3 != 0 {
                return Err(SparcV9Error::MisalignedTarget { target });
            }
            if op3(word) == 0x38 {
                write_register(state, rd(word), old_pc)?;
                delayed_transfer(state, target)?;
                Ok(Outcome::delayed("JMPL"))
            } else {
                if rd(word) != 0 {
                    return Err(SparcV9Error::UnknownInstruction { word, pc: old_pc });
                }
                if state.canrestore == 0 {
                    return Err(SparcV9Error::WindowFill);
                }
                state.cwp = (state.cwp + WINDOWS - 1) % WINDOWS;
                state.canrestore -= 1;
                state.cansave += 1;
                delayed_transfer(state, target)?;
                Ok(Outcome::delayed("RETURN"))
            }
        }
        InstructionClass::Window => execute_window(state, word).map(Outcome::ordinary),
        InstructionClass::Memory => execute_memory(state, word).map(Outcome::ordinary),
        InstructionClass::Trap => {
            if word == 0x91d0_2000 {
                advance(state);
                state.halted = true;
                Ok(Outcome::ordinary("TA 0"))
            } else {
                let immediate = word & (1 << 13) != 0;
                let operand = if immediate {
                    u64::from((word & 0xff) as u8)
                } else {
                    read_register(state, (word & 0x1f) as u8)?
                };
                let number = (read_register(state, rs1(word))?.wrapping_add(operand) & 0x7f) as u8;
                Err(SparcV9Error::UnsupportedTrap { number, pc: old_pc })
            }
        }
    }
}

fn execute_arithmetic(state: &mut SparcV9State, word: u32) -> Result<&'static str, SparcV9Error> {
    let operation = op3(word);
    let a = read_register(state, rs1(word))?;
    let b = operand2(state, word)?;
    let carry = matches!(operation, 0x08 | 0x18) && state.ccr.icc.c;
    let borrow = matches!(operation, 0x0c | 0x1c) && state.ccr.icc.c;
    let result = if matches!(operation, 0x00 | 0x08 | 0x10 | 0x18) {
        a.wrapping_add(b).wrapping_add(u64::from(carry))
    } else {
        a.wrapping_sub(b).wrapping_sub(u64::from(borrow))
    };
    write_register(state, rd(word), result)?;
    let set_cc = operation & 0x10 != 0;
    if set_cc {
        if operation & 0x04 == 0 {
            set_add_flags(state, a, b, carry);
        } else {
            set_sub_flags(state, a, b, borrow);
        }
    }
    advance(state);
    Ok(match operation {
        0x00 => "ADD",
        0x04 => "SUB",
        0x08 => "ADDC",
        0x0c => "SUBC",
        0x10 => "ADDCC",
        0x14 => "SUBCC",
        0x18 => "ADDCCC",
        _ => "SUBCCC",
    })
}

fn execute_register_branch(state: &mut SparcV9State, word: u32) -> Result<Outcome, SparcV9Error> {
    if word & (1 << 28) != 0 {
        return Err(SparcV9Error::UnknownInstruction { word, pc: state.pc });
    }
    let condition = ((word >> 25) & 7) as u8;
    let value = read_register(state, rs1(word))? as i64;
    let taken = register_condition(condition, value)
        .ok_or(SparcV9Error::UnknownInstruction { word, pc: state.pc })?;
    let displacement = (((word >> 20) & 3) << 14) | (word & 0x3fff);
    let target = state
        .pc
        .wrapping_add_signed(sign_extend(displacement, 16) << 2);
    if target & 3 != 0 {
        return Err(SparcV9Error::MisalignedTarget { target });
    }
    let skipped = state.npc;
    let annulled = !taken && word & (1 << 29) != 0;
    if annulled {
        state.pc = state.npc.wrapping_add(4);
        state.npc = state.npc.wrapping_add(8);
        state.delay_slot = false;
    } else {
        let next = if taken {
            target
        } else {
            state.npc.wrapping_add(4)
        };
        delayed_transfer(state, next)?;
    }
    Ok(if annulled {
        Outcome::annulled(register_branch_mnemonic(condition), skipped)
    } else {
        Outcome::delayed(register_branch_mnemonic(condition))
    })
}

fn integer_branch_mnemonic(condition: u8) -> &'static str {
    [
        "BN", "BE", "BLE", "BL", "BLEU", "BCS", "BNEG", "BVS", "BA", "BNE", "BG", "BGE", "BGU",
        "BCC", "BPOS", "BVC",
    ][usize::from(condition & 0xf)]
}

fn predicted_branch_mnemonic(condition: u8) -> &'static str {
    [
        "BPN", "BPE", "BPLE", "BPL", "BPLEU", "BPCS", "BPNEG", "BPVS", "BPA", "BPNE", "BPG",
        "BPGE", "BPGU", "BPCC", "BPPOS", "BPVC",
    ][usize::from(condition & 0xf)]
}

fn register_branch_mnemonic(condition: u8) -> &'static str {
    match condition {
        1 => "BRZ",
        2 => "BRLEZ",
        3 => "BRLZ",
        5 => "BRNZ",
        6 => "BRGZ",
        _ => "BRGEZ",
    }
}

fn execute_logical(state: &mut SparcV9State, word: u32) -> Result<&'static str, SparcV9Error> {
    let operation = op3(word);
    let a = read_register(state, rs1(word))?;
    let b = operand2(state, word)?;
    let result = match operation & 7 {
        1 => a & b,
        2 => a | b,
        3 => a ^ b,
        5 => a & !b,
        6 => a | !b,
        _ => !(a ^ b),
    };
    write_register(state, rd(word), result)?;
    if operation & 0x10 != 0 {
        state.ccr.icc = logic_flags(result, 32);
        state.ccr.xcc = logic_flags(result, 64);
    }
    advance(state);
    Ok(match operation {
        0x01 => "AND",
        0x02 => "OR",
        0x03 => "XOR",
        0x05 => "ANDN",
        0x06 => "ORN",
        0x07 => "XNOR",
        0x11 => "ANDCC",
        0x12 => "ORCC",
        0x13 => "XORCC",
        0x15 => "ANDNCC",
        0x16 => "ORNCC",
        _ => "XNORCC",
    })
}

fn execute_shift(state: &mut SparcV9State, word: u32) -> Result<&'static str, SparcV9Error> {
    let a = read_register(state, rs1(word))?;
    let extended = word & (1 << 12) != 0;
    let immediate = word & (1 << 13) != 0;
    let reserved_mask = if immediate {
        if extended {
            0x0fc0
        } else {
            0x0fe0
        }
    } else {
        0x0fe0
    };
    if word & reserved_mask != 0 {
        return Err(SparcV9Error::UnknownInstruction { word, pc: state.pc });
    }
    let count = if immediate {
        if extended {
            word & 0x3f
        } else {
            word & 0x1f
        }
    } else {
        let raw = read_register(state, (word & 0x1f) as u8)?;
        (raw & if extended { 63 } else { 31 }) as u32
    };
    let result = match (op3(word), extended) {
        (0x25, true) => a.wrapping_shl(count),
        (0x26, true) => a.wrapping_shr(count),
        (0x27, true) => ((a as i64) >> count) as u64,
        (0x25, false) => a.wrapping_shl(count),
        (0x26, false) => u64::from((a as u32).wrapping_shr(count)),
        _ => ((a as u32 as i32) >> count) as i64 as u64,
    };
    write_register(state, rd(word), result)?;
    advance(state);
    Ok(match (op3(word), extended) {
        (0x25, false) => "SLL",
        (0x25, true) => "SLLX",
        (0x26, false) => "SRL",
        (0x26, true) => "SRLX",
        (0x27, false) => "SRA",
        _ => "SRAX",
    })
}

fn execute_multiply_divide(
    state: &mut SparcV9State,
    word: u32,
) -> Result<&'static str, SparcV9Error> {
    let operation = op3(word);
    if operation == 0x2e && rs1(word) != 0 {
        return Err(SparcV9Error::UnknownInstruction { word, pc: state.pc });
    }
    let a = read_register(state, rs1(word))?;
    let b = operand2(state, word)?;
    let (result, mnemonic) = match operation {
        0x09 => (a.wrapping_mul(b), "MULX"),
        0x0d => {
            if b == 0 {
                return Err(SparcV9Error::DivideByZero);
            }
            (a / b, "UDIVX")
        }
        0x2d => {
            if b == 0 {
                return Err(SparcV9Error::DivideByZero);
            }
            let result = if a == i64::MIN as u64 && b == u64::MAX {
                i64::MIN as u64
            } else {
                ((a as i64) / (b as i64)) as u64
            };
            (result, "SDIVX")
        }
        0x2e => (u64::from(b.count_ones()), "POPC"),
        0x0a | 0x1a => {
            let product = u64::from(a as u32) * u64::from(b as u32);
            state.y = product >> 32;
            if operation == 0x1a {
                set_legacy_multiply_flags(state, product);
            }
            (product, if operation == 0x1a { "UMULCC" } else { "UMUL" })
        }
        0x0b | 0x1b => {
            let product =
                i64::from(a as u32 as i32).wrapping_mul(i64::from(b as u32 as i32)) as u64;
            state.y = product >> 32;
            if operation == 0x1b {
                set_legacy_multiply_flags(state, product);
            }
            (product, if operation == 0x1b { "SMULCC" } else { "SMUL" })
        }
        0x0e | 0x1e => {
            let divisor = u64::from(b as u32);
            if divisor == 0 {
                return Err(SparcV9Error::DivideByZero);
            }
            let dividend = (state.y << 32) | u64::from(a as u32);
            let quotient = dividend / divisor;
            let saturated = quotient > u64::from(u32::MAX);
            let selected = if saturated { u32::MAX } else { quotient as u32 };
            let value = u64::from(selected);
            if operation == 0x1e {
                set_legacy_divide_flags(state, value, selected, saturated);
            }
            (value, if operation == 0x1e { "UDIVCC" } else { "UDIV" })
        }
        _ => {
            let divisor = i128::from(b as u32 as i32);
            if divisor == 0 {
                return Err(SparcV9Error::DivideByZero);
            }
            let dividend = i128::from(((state.y << 32) | u64::from(a as u32)) as i64);
            let quotient = dividend / divisor;
            let saturated = quotient < i128::from(i32::MIN) || quotient > i128::from(i32::MAX);
            let selected = if quotient < i128::from(i32::MIN) {
                i32::MIN
            } else if quotient > i128::from(i32::MAX) {
                i32::MAX
            } else {
                quotient as i32
            };
            let value = i64::from(selected) as u64;
            if operation == 0x1f {
                set_legacy_divide_flags(state, value, selected as u32, saturated);
            }
            (value, if operation == 0x1f { "SDIVCC" } else { "SDIV" })
        }
    };
    write_register(state, rd(word), result)?;
    advance(state);
    Ok(mnemonic)
}

fn set_legacy_multiply_flags(state: &mut SparcV9State, product: u64) {
    state.ccr.icc = ConditionCodes {
        n: product & (1 << 31) != 0,
        z: product as u32 == 0,
        v: false,
        c: false,
    };
    state.ccr.xcc = ConditionCodes {
        n: product & (1 << 63) != 0,
        z: product == 0,
        v: false,
        c: false,
    };
}

fn set_legacy_divide_flags(
    state: &mut SparcV9State,
    extended: u64,
    selected: u32,
    saturated: bool,
) {
    state.ccr.icc = ConditionCodes {
        n: selected & (1 << 31) != 0,
        z: selected == 0,
        v: saturated,
        c: false,
    };
    state.ccr.xcc = ConditionCodes {
        n: extended & (1 << 63) != 0,
        z: extended == 0,
        v: false,
        c: false,
    };
}

fn execute_movcc(state: &mut SparcV9State, word: u32) -> Result<&'static str, SparcV9Error> {
    let selector = (((word >> 18) & 1) << 2) | ((word >> 11) & 3);
    let cc = match selector {
        4 => state.ccr.icc,
        6 => state.ccr.xcc,
        0..=3 => return Err(SparcV9Error::UnsupportedInstruction { word, pc: state.pc }),
        _ => return Err(SparcV9Error::UnknownInstruction { word, pc: state.pc }),
    };
    let immediate = word & (1 << 13) != 0;
    let value = if immediate {
        sign_extend(word & 0x7ff, 11) as u64
    } else {
        if word & 0x7e0 != 0 {
            return Err(SparcV9Error::UnknownInstruction { word, pc: state.pc });
        }
        read_register(state, (word & 0x1f) as u8)?
    };
    let condition = ((word >> 14) & 0xf) as u8;
    if branch_condition(condition, cc) {
        write_register(state, rd(word), value)?;
    }
    advance(state);
    Ok(movcc_mnemonic(condition))
}

fn execute_movr(state: &mut SparcV9State, word: u32) -> Result<&'static str, SparcV9Error> {
    let condition = ((word >> 10) & 7) as u8;
    let immediate = word & (1 << 13) != 0;
    let value = if immediate {
        sign_extend(word & 0x3ff, 10) as u64
    } else {
        if word & 0x3e0 != 0 {
            return Err(SparcV9Error::UnknownInstruction { word, pc: state.pc });
        }
        read_register(state, (word & 0x1f) as u8)?
    };
    let source = read_register(state, rs1(word))? as i64;
    let take = register_condition(condition, source)
        .ok_or(SparcV9Error::UnknownInstruction { word, pc: state.pc })?;
    if take {
        write_register(state, rd(word), value)?;
    }
    advance(state);
    Ok(match condition {
        1 => "MOVRZ",
        2 => "MOVRLEZ",
        3 => "MOVRLZ",
        5 => "MOVRNZ",
        6 => "MOVRGZ",
        _ => "MOVRGEZ",
    })
}

fn movcc_mnemonic(condition: u8) -> &'static str {
    [
        "MOVN", "MOVE", "MOVLE", "MOVL", "MOVLEU", "MOVCS", "MOVNEG", "MOVVS", "MOVA", "MOVNE",
        "MOVG", "MOVGE", "MOVGU", "MOVCC", "MOVPOS", "MOVVC",
    ][usize::from(condition & 0xf)]
}

fn execute_membar(state: &mut SparcV9State, word: u32) -> Result<&'static str, SparcV9Error> {
    if rd(word) != 0 || rs1(word) != 15 || word & (1 << 13) == 0 || word & 0x1f80 != 0 {
        return Err(SparcV9Error::UnknownInstruction { word, pc: state.pc });
    }
    advance(state);
    Ok("MEMBAR")
}

fn execute_window(state: &mut SparcV9State, word: u32) -> Result<&'static str, SparcV9Error> {
    let a = read_register(state, rs1(word))?;
    let b = operand2(state, word)?;
    let result = a.wrapping_add(b);
    if op3(word) == 0x3c {
        if state.cansave == 0 {
            return Err(SparcV9Error::WindowSpill);
        }
        if state.cleanwin == state.canrestore {
            return Err(SparcV9Error::CleanWindow);
        }
        state.cwp = (state.cwp + 1) % WINDOWS;
        state.cansave -= 1;
        state.canrestore += 1;
        write_register(state, rd(word), result)?;
        advance(state);
        Ok("SAVE")
    } else {
        if state.canrestore == 0 {
            return Err(SparcV9Error::WindowFill);
        }
        state.cwp = (state.cwp + WINDOWS - 1) % WINDOWS;
        state.canrestore -= 1;
        state.cansave += 1;
        write_register(state, rd(word), result)?;
        advance(state);
        Ok("RESTORE")
    }
}

fn execute_memory(state: &mut SparcV9State, word: u32) -> Result<&'static str, SparcV9Error> {
    let address = read_register(state, rs1(word))?.wrapping_add(operand2(state, word)?);
    let destination = rd(word);
    let mnemonic = match op3(word) {
        0x00 => {
            let value = read_be(state, address, 4)?;
            write_register(state, destination, value)?;
            "LDUW"
        }
        0x01 => {
            let value = read_be(state, address, 1)?;
            write_register(state, destination, value)?;
            "LDUB"
        }
        0x02 => {
            let value = read_be(state, address, 2)?;
            write_register(state, destination, value)?;
            "LDUH"
        }
        0x03 => {
            if destination & 1 != 0 {
                return Err(SparcV9Error::InvalidRegisterPair { index: destination });
            }
            let value = read_be(state, address, 8)?;
            write_register(state, destination, value >> 32)?;
            write_register(state, destination + 1, value & 0xffff_ffff)?;
            "LDD"
        }
        0x04 => {
            write_be(state, address, 4, read_register(state, destination)?)?;
            "STW"
        }
        0x05 => {
            write_be(state, address, 1, read_register(state, destination)?)?;
            "STB"
        }
        0x06 => {
            write_be(state, address, 2, read_register(state, destination)?)?;
            "STH"
        }
        0x07 => {
            if destination & 1 != 0 {
                return Err(SparcV9Error::InvalidRegisterPair { index: destination });
            }
            checked_range(address, 8)?;
            let high = read_register(state, destination)? as u32;
            let low = read_register(state, destination + 1)? as u32;
            write_be(state, address, 8, (u64::from(high) << 32) | u64::from(low))?;
            "STD"
        }
        0x08 => {
            let value = read_be(state, address, 4)? as u32 as i32 as i64 as u64;
            write_register(state, destination, value)?;
            "LDSW"
        }
        0x09 => {
            let value = read_be(state, address, 1)? as u8 as i8 as i64 as u64;
            write_register(state, destination, value)?;
            "LDSB"
        }
        0x0a => {
            let value = read_be(state, address, 2)? as u16 as i16 as i64 as u64;
            write_register(state, destination, value)?;
            "LDSH"
        }
        0x0b => {
            let value = read_be(state, address, 8)?;
            write_register(state, destination, value)?;
            "LDX"
        }
        _ => {
            write_be(state, address, 8, read_register(state, destination)?)?;
            "STX"
        }
    };
    advance(state);
    Ok(mnemonic)
}

/// Instruction encoding helpers for tests and downstream assemblers.
pub mod encoding {
    /// General format-3 constructor.
    #[must_use]
    pub const fn encode_format3(
        op: u32,
        rd: u32,
        op3: u32,
        rs1: u32,
        immediate: bool,
        operand: i32,
    ) -> u32 {
        (op << 30)
            | ((rd & 0x1f) << 25)
            | ((op3 & 0x3f) << 19)
            | ((rs1 & 0x1f) << 14)
            | ((immediate as u32) << 13)
            | ((operand as u32) & if immediate { 0x1fff } else { 0x1f })
    }

    const fn reg(op3: u32, rd: u32, rs1: u32, rs2: u32) -> u32 {
        encode_format3(2, rd, op3, rs1, false, rs2 as i32)
    }

    const fn imm(op3: u32, rd: u32, rs1: u32, value: i32) -> u32 {
        encode_format3(2, rd, op3, rs1, true, value)
    }

    const fn mem(op3: u32, rd: u32, rs1: u32, offset: i32) -> u32 {
        encode_format3(3, rd, op3, rs1, true, offset)
    }

    #[must_use]
    pub const fn encode_call(displacement_words: i32) -> u32 {
        (1 << 30) | ((displacement_words as u32) & 0x3fff_ffff)
    }

    #[must_use]
    pub const fn encode_sethi(rd: u32, immediate: u32) -> u32 {
        (rd << 25) | (4 << 22) | (immediate & 0x3f_ffff)
    }

    #[must_use]
    pub const fn encode_nop() -> u32 {
        encode_sethi(0, 0)
    }

    #[must_use]
    pub const fn encode_bicc(condition: u32, annul: bool, displacement_words: i32) -> u32 {
        ((annul as u32) << 29)
            | ((condition & 0xf) << 25)
            | (2 << 22)
            | ((displacement_words as u32) & 0x3f_ffff)
    }

    #[must_use]
    pub const fn encode_bpcc(
        condition: u32,
        annul: bool,
        xcc: bool,
        displacement_words: i32,
    ) -> u32 {
        ((annul as u32) << 29)
            | ((condition & 0xf) << 25)
            | (1 << 22)
            | ((if xcc { 2 } else { 0 }) << 20)
            | (1 << 19)
            | ((displacement_words as u32) & 0x7_ffff)
    }

    #[must_use]
    pub const fn encode_bpr(
        condition: u32,
        annul: bool,
        predict_taken: bool,
        rs1: u32,
        displacement_words: i32,
    ) -> u32 {
        let displacement = displacement_words as u32;
        ((annul as u32) << 29)
            | ((condition & 7) << 25)
            | (3 << 22)
            | (((displacement >> 14) & 3) << 20)
            | ((predict_taken as u32) << 19)
            | ((rs1 & 0x1f) << 14)
            | (displacement & 0x3fff)
    }

    #[must_use]
    pub const fn encode_add(rd: u32, rs1: u32, rs2: u32) -> u32 {
        reg(0x00, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_add_imm(rd: u32, rs1: u32, value: i32) -> u32 {
        imm(0x00, rd, rs1, value)
    }
    #[must_use]
    pub const fn encode_addcc(rd: u32, rs1: u32, rs2: u32) -> u32 {
        reg(0x10, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_addc(rd: u32, rs1: u32, rs2: u32) -> u32 {
        reg(0x08, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_addccc(rd: u32, rs1: u32, rs2: u32) -> u32 {
        reg(0x18, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_subcc(rd: u32, rs1: u32, rs2: u32) -> u32 {
        reg(0x14, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_subc(rd: u32, rs1: u32, rs2: u32) -> u32 {
        reg(0x0c, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_subccc(rd: u32, rs1: u32, rs2: u32) -> u32 {
        reg(0x1c, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_xor(rd: u32, rs1: u32, rs2: u32) -> u32 {
        reg(0x03, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_xorcc(rd: u32, rs1: u32, rs2: u32) -> u32 {
        reg(0x13, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_and(rd: u32, rs1: u32, rs2: u32) -> u32 {
        reg(0x01, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_or(rd: u32, rs1: u32, rs2: u32) -> u32 {
        reg(0x02, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_sub(rd: u32, rs1: u32, rs2: u32) -> u32 {
        reg(0x04, rd, rs1, rs2)
    }

    const fn shift(op3: u32, rd: u32, rs1: u32, count: u32, extended: bool) -> u32 {
        (2 << 30)
            | ((rd & 0x1f) << 25)
            | (op3 << 19)
            | ((rs1 & 0x1f) << 14)
            | (1 << 13)
            | ((extended as u32) << 12)
            | (count & if extended { 0x3f } else { 0x1f })
    }

    #[must_use]
    pub const fn encode_sllx(rd: u32, rs1: u32, count: u32) -> u32 {
        shift(0x25, rd, rs1, count, true)
    }
    #[must_use]
    pub const fn encode_srl(rd: u32, rs1: u32, count: u32) -> u32 {
        shift(0x26, rd, rs1, count, false)
    }
    #[must_use]
    pub const fn encode_srax(rd: u32, rs1: u32, count: u32) -> u32 {
        shift(0x27, rd, rs1, count, true)
    }

    #[must_use]
    pub const fn encode_mulx(rd: u32, rs1: u32, rs2: u32) -> u32 {
        reg(0x09, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_udivx(rd: u32, rs1: u32, rs2: u32) -> u32 {
        reg(0x0d, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_sdivx_imm(rd: u32, rs1: u32, value: i32) -> u32 {
        imm(0x2d, rd, rs1, value)
    }
    #[must_use]
    pub const fn encode_umul(rd: u32, rs1: u32, rs2: u32, set_cc: bool) -> u32 {
        reg(if set_cc { 0x1a } else { 0x0a }, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_smul(rd: u32, rs1: u32, rs2: u32, set_cc: bool) -> u32 {
        reg(if set_cc { 0x1b } else { 0x0b }, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_udiv(rd: u32, rs1: u32, rs2: u32, set_cc: bool) -> u32 {
        reg(if set_cc { 0x1e } else { 0x0e }, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_sdiv(rd: u32, rs1: u32, rs2: u32, set_cc: bool) -> u32 {
        reg(if set_cc { 0x1f } else { 0x0f }, rd, rs1, rs2)
    }
    #[must_use]
    pub const fn encode_popc(rd: u32, rs2: u32) -> u32 {
        reg(0x2e, rd, 0, rs2)
    }
    #[must_use]
    pub const fn encode_movcc(
        rd: u32,
        condition: u32,
        xcc: bool,
        immediate: bool,
        operand: i32,
    ) -> u32 {
        (2 << 30)
            | ((rd & 0x1f) << 25)
            | (0x2c << 19)
            | (1 << 18)
            | ((condition & 0xf) << 14)
            | ((immediate as u32) << 13)
            | ((if xcc { 2 } else { 0 }) << 11)
            | ((operand as u32) & if immediate { 0x7ff } else { 0x1f })
    }
    #[must_use]
    pub const fn encode_movr(
        rd: u32,
        rs1: u32,
        condition: u32,
        immediate: bool,
        operand: i32,
    ) -> u32 {
        (2 << 30)
            | ((rd & 0x1f) << 25)
            | (0x2f << 19)
            | ((rs1 & 0x1f) << 14)
            | ((immediate as u32) << 13)
            | ((condition & 7) << 10)
            | ((operand as u32) & if immediate { 0x3ff } else { 0x1f })
    }
    #[must_use]
    pub const fn encode_membar(cmask: u32, mmask: u32) -> u32 {
        (2 << 30) | (0x28 << 19) | (15 << 14) | (1 << 13) | ((cmask & 7) << 4) | (mmask & 0xf)
    }
    #[must_use]
    pub const fn encode_jmpl(rd: u32, rs1: u32, value: i32) -> u32 {
        imm(0x38, rd, rs1, value)
    }
    #[must_use]
    pub const fn encode_save(rd: u32, rs1: u32, value: i32) -> u32 {
        imm(0x3c, rd, rs1, value)
    }
    #[must_use]
    pub const fn encode_restore(rd: u32, rs1: u32, value: i32) -> u32 {
        imm(0x3d, rd, rs1, value)
    }
    #[must_use]
    pub const fn encode_return(rs1: u32, value: i32) -> u32 {
        imm(0x39, 0, rs1, value)
    }
    #[must_use]
    pub const fn encode_ta(number: u32) -> u32 {
        encode_tcc(8, false, 0, number, true)
    }

    #[must_use]
    pub const fn encode_tcc(
        condition: u32,
        xcc: bool,
        rs1: u32,
        operand: u32,
        immediate: bool,
    ) -> u32 {
        (2 << 30)
            | ((condition & 0xf) << 25)
            | (0x3a << 19)
            | ((rs1 & 0x1f) << 14)
            | ((immediate as u32) << 13)
            | ((if xcc { 2 } else { 0 }) << 11)
            | (operand & if immediate { 0xff } else { 0x1f })
    }

    #[must_use]
    pub const fn encode_lduw(rd: u32, rs1: u32, offset: i32) -> u32 {
        mem(0x00, rd, rs1, offset)
    }
    #[must_use]
    pub const fn encode_ldub(rd: u32, rs1: u32, offset: i32) -> u32 {
        mem(0x01, rd, rs1, offset)
    }
    #[must_use]
    pub const fn encode_lduh(rd: u32, rs1: u32, offset: i32) -> u32 {
        mem(0x02, rd, rs1, offset)
    }
    #[must_use]
    pub const fn encode_ldd(rd: u32, rs1: u32, offset: i32) -> u32 {
        mem(0x03, rd, rs1, offset)
    }
    #[must_use]
    pub const fn encode_stw(rd: u32, rs1: u32, offset: i32) -> u32 {
        mem(0x04, rd, rs1, offset)
    }
    #[must_use]
    pub const fn encode_stb(rd: u32, rs1: u32, offset: i32) -> u32 {
        mem(0x05, rd, rs1, offset)
    }
    #[must_use]
    pub const fn encode_sth(rd: u32, rs1: u32, offset: i32) -> u32 {
        mem(0x06, rd, rs1, offset)
    }
    #[must_use]
    pub const fn encode_std(rd: u32, rs1: u32, offset: i32) -> u32 {
        mem(0x07, rd, rs1, offset)
    }
    #[must_use]
    pub const fn encode_ldsw(rd: u32, rs1: u32, offset: i32) -> u32 {
        mem(0x08, rd, rs1, offset)
    }
    #[must_use]
    pub const fn encode_ldsb(rd: u32, rs1: u32, offset: i32) -> u32 {
        mem(0x09, rd, rs1, offset)
    }
    #[must_use]
    pub const fn encode_ldsh(rd: u32, rs1: u32, offset: i32) -> u32 {
        mem(0x0a, rd, rs1, offset)
    }
    #[must_use]
    pub const fn encode_ldx(rd: u32, rs1: u32, offset: i32) -> u32 {
        mem(0x0b, rd, rs1, offset)
    }
    #[must_use]
    pub const fn encode_stx(rd: u32, rs1: u32, offset: i32) -> u32 {
        mem(0x0e, rd, rs1, offset)
    }
}

/// Convert instruction words to big-endian bytes.
#[must_use]
pub fn assemble(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_be_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_integer_condition_sense_is_centralized() {
        let cases = [
            (
                ConditionCodes::default(),
                [
                    false, false, false, false, false, false, false, false, true, true, true, true,
                    true, true, true, true,
                ],
            ),
            (
                ConditionCodes {
                    n: true,
                    z: false,
                    v: false,
                    c: false,
                },
                [
                    false, false, true, true, false, false, true, false, true, true, false, false,
                    true, true, false, true,
                ],
            ),
            (
                ConditionCodes {
                    n: false,
                    z: true,
                    v: true,
                    c: true,
                },
                [
                    false, true, true, true, true, true, false, true, true, false, false, false,
                    false, false, true, false,
                ],
            ),
        ];
        for (cc, expected) in cases {
            for (condition, expected) in expected.into_iter().enumerate() {
                assert_eq!(
                    branch_condition(condition as u8, cc),
                    expected,
                    "cond={condition} cc={cc:?}"
                );
            }
        }
    }

    #[test]
    fn forced_trace_allocation_failure_preserves_run_state() {
        let mut simulator = SparcV9Simulator::new();
        simulator
            .load(&assemble(&[encoding::encode_ta(0)]), 0x100)
            .unwrap();
        let before = simulator.get_state();
        FORCE_ALLOCATION_FAILURE.store(true, Ordering::SeqCst);
        assert_eq!(simulator.run(1), Err(SparcV9Error::AllocationFailed));
        assert_eq!(simulator.get_state(), before);
    }

    #[test]
    fn every_register_condition_sense_is_centralized() {
        for (value, expected) in [
            (-1, [false, false, true, true, false, true, false, false]),
            (0, [false, true, true, false, false, false, false, true]),
            (1, [false, false, false, false, false, true, true, true]),
        ] {
            for (condition, expected) in expected.into_iter().enumerate() {
                if condition == 0 || condition == 4 {
                    assert_eq!(register_condition(condition as u8, value), None);
                } else {
                    assert_eq!(register_condition(condition as u8, value), Some(expected));
                }
            }
        }
    }
}
