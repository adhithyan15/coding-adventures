//! Checked, deterministic functional model of one UltraSPARC T1 strand.
//!
//! The real T1 composes 32 of these architectural contexts around shared
//! execution, cache, translation, crossbar, and memory-controller structures.
//! This crate intentionally owns only the scalar integer contract that those
//! later gate and SoC models can use as their instruction-boundary oracle.

use core::fmt;

/// Size of the flat teaching memory.
pub const MEMORY_SIZE: usize = 65_536;
/// Four global sets (32 registers) plus eight 16-register window bodies.
pub const NUM_PHYSICAL_REGISTERS: usize = 160;
/// Maximum complete-state traces retained by one run.
pub const MAX_RETAINED_STEPS: usize = 1_024;
const GLOBAL_LEVELS: u8 = 4;
const WINDOWS: u8 = 8;

/// N/Z/V/C flags for either the low 32-bit or full 64-bit result.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConditionCodes {
    pub n: bool,
    pub z: bool,
    pub v: bool,
    pub c: bool,
}

/// UltraSPARC's two integer condition-code banks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConditionCodeRegister {
    pub icc: ConditionCodes,
    pub xcc: ConditionCodes,
}

/// Processor-state fields relevant to this single-strand integer boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessorState {
    pub privileged: bool,
    pub cle: bool,
    pub address_mask: bool,
}

impl Default for ProcessorState {
    fn default() -> Self {
        Self {
            privileged: true,
            cle: false,
            address_mask: false,
        }
    }
}

/// Complete owned state at an instruction boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenSparcT1State {
    pub registers: [u64; NUM_PHYSICAL_REGISTERS],
    pub pc: u64,
    pub npc: u64,
    pub ccr: ConditionCodeRegister,
    pub y: u32,
    pub asi: u8,
    pub pstate: ProcessorState,
    pub gl: u8,
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

impl Default for OpenSparcT1State {
    fn default() -> Self {
        Self {
            registers: [0; NUM_PHYSICAL_REGISTERS],
            pc: 0,
            npc: 4,
            ccr: ConditionCodeRegister::default(),
            y: 0,
            asi: 0x80,
            pstate: ProcessorState::default(),
            gl: 0,
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
    ControlTransfer,
    Window,
    Memory,
    Trap,
}

/// Checked lifecycle, transport, decode, and execution failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenSparcT1Error {
    EmptyProgram,
    MisalignedProgram { origin: u64, len: usize },
    ProgramOutOfRange { origin: u64, len: usize },
    MisalignedFetch { pc: u64 },
    FetchOutsideProgram { pc: u64 },
    MisalignedData { address: u64, width: usize },
    DataOutOfRange { address: u64, width: usize },
    MisalignedTarget { target: u64 },
    InvalidRegister { index: u8 },
    InvalidState { reason: &'static str },
    UnknownInstruction { word: u32, pc: u64 },
    UnsupportedInstruction { word: u32, pc: u64 },
    PrivilegedInstruction { word: u32, pc: u64 },
    DivideByZero,
    WindowSpill,
    WindowFill,
    CleanWindow,
    UnsupportedTrap { number: u8, pc: u64 },
    ControlTransferInDelaySlot { pc: u64 },
    Halted,
    RetentionLimitExceeded { requested: usize, limit: usize },
    AllocationFailed,
    StepLimitExceeded { limit: usize },
}

impl fmt::Display for OpenSparcT1Error {
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
            Self::InvalidState { reason } => write!(f, "invalid OpenSPARC T1 state: {reason}"),
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
            Self::ControlTransferInDelaySlot { pc } => {
                write!(f, "control transfer in delay slot at {pc:#018x}")
            }
            Self::Halted => f.write_str("strand is halted"),
            Self::RetentionLimitExceeded { requested, limit } => {
                write!(f, "requested {requested} traces exceeds limit {limit}")
            }
            Self::AllocationFailed => f.write_str("trace allocation failed"),
            Self::StepLimitExceeded { limit } => write!(f, "step limit {limit} exceeded"),
        }
    }
}

impl std::error::Error for OpenSparcT1Error {}

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
    pub state_after: OpenSparcT1State,
}

/// Successful bounded execution result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: OpenSparcT1State,
}

/// Functional simulator for one UltraSPARC T1 strand.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenSparcT1Simulator {
    state: OpenSparcT1State,
}

impl OpenSparcT1Simulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.state = OpenSparcT1State::default();
    }

    #[must_use]
    pub fn get_state(&self) -> OpenSparcT1State {
        self.state.clone()
    }

    pub fn restore(&mut self, state: OpenSparcT1State) -> Result<(), OpenSparcT1Error> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    pub fn load(&mut self, program: &[u8], origin: u64) -> Result<(), OpenSparcT1Error> {
        validate_program(program, origin)?;
        let start = usize::try_from(origin).map_err(|_| OpenSparcT1Error::ProgramOutOfRange {
            origin,
            len: program.len(),
        })?;
        let end = start + program.len();
        let mut state = OpenSparcT1State {
            pc: origin,
            npc: origin + 4,
            loaded_origin: origin,
            loaded_len: program.len(),
            ..OpenSparcT1State::default()
        };
        state.memory[start..end].copy_from_slice(program);
        self.state = state;
        Ok(())
    }

    pub fn read_register(&self, index: u8) -> Result<u64, OpenSparcT1Error> {
        read_register(&self.state, index)
    }

    pub fn write_register(&mut self, index: u8, value: u64) -> Result<(), OpenSparcT1Error> {
        write_register(&mut self.state, index, value)
    }

    pub fn read_memory(&self, address: u64, width: usize) -> Result<u64, OpenSparcT1Error> {
        read_be(&self.state, address, width)
    }

    pub fn write_memory(
        &mut self,
        address: u64,
        width: usize,
        value: u64,
    ) -> Result<(), OpenSparcT1Error> {
        write_be(&mut self.state, address, width, value)
    }

    pub fn step(&mut self) -> Result<StepTrace, OpenSparcT1Error> {
        if self.state.halted {
            return Err(OpenSparcT1Error::Halted);
        }
        let pc = self.state.pc;
        let npc = self.state.npc;
        let word = fetch(&self.state)?;
        let class = classify_at(word, pc)?;
        if self.state.delay_slot && is_delayed_control_transfer(class) {
            return Err(OpenSparcT1Error::ControlTransferInDelaySlot { pc });
        }
        let mut next = self.state.clone();
        let mnemonic = execute_word(&mut next, word, class)?;
        let trace = StepTrace {
            pc_before: pc,
            npc_before: npc,
            pc_after: next.pc,
            npc_after: next.npc,
            raw: word.to_be_bytes(),
            raw_word: word,
            mnemonic,
            state_after: next.clone(),
        };
        self.state = next;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, OpenSparcT1Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(OpenSparcT1Error::RetentionLimitExceeded {
                requested: max_steps,
                limit: MAX_RETAINED_STEPS,
            });
        }
        let before = self.state.clone();
        let mut traces = Vec::new();
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| OpenSparcT1Error::AllocationFailed)?;
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
        Err(OpenSparcT1Error::StepLimitExceeded { limit: max_steps })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u64,
        max_steps: usize,
    ) -> Result<ExecutionResult, OpenSparcT1Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(OpenSparcT1Error::RetentionLimitExceeded {
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

fn validate_program(program: &[u8], origin: u64) -> Result<(), OpenSparcT1Error> {
    if program.is_empty() {
        return Err(OpenSparcT1Error::EmptyProgram);
    }
    if origin & 3 != 0 || program.len() & 3 != 0 {
        return Err(OpenSparcT1Error::MisalignedProgram {
            origin,
            len: program.len(),
        });
    }
    let start = usize::try_from(origin).map_err(|_| OpenSparcT1Error::ProgramOutOfRange {
        origin,
        len: program.len(),
    })?;
    if start
        .checked_add(program.len())
        .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(OpenSparcT1Error::ProgramOutOfRange {
            origin,
            len: program.len(),
        });
    }
    Ok(())
}

fn validate_state(state: &OpenSparcT1State) -> Result<(), OpenSparcT1Error> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(OpenSparcT1Error::InvalidState {
            reason: "memory size must be exactly 64 KiB",
        });
    }
    if [0_usize, 8, 16, 24]
        .into_iter()
        .any(|index| state.registers[index] != 0)
    {
        return Err(OpenSparcT1Error::InvalidState {
            reason: "every global level's g0 must be zero",
        });
    }
    if state.gl >= GLOBAL_LEVELS || state.cwp >= WINDOWS {
        return Err(OpenSparcT1Error::InvalidState {
            reason: "GL or CWP is out of range",
        });
    }
    if state.cansave > WINDOWS - 2
        || state.canrestore > WINDOWS - 2
        || state.otherwin > WINDOWS - 2
        || state.cleanwin > WINDOWS - 2
        || state.cleanwin < state.canrestore
        || state.cansave + state.canrestore + state.otherwin != WINDOWS - 2
    {
        return Err(OpenSparcT1Error::InvalidState {
            reason: "register-window counters are inconsistent",
        });
    }
    if state.wstate & !0x3f != 0 {
        return Err(OpenSparcT1Error::InvalidState {
            reason: "WSTATE contains reserved bits",
        });
    }
    if state.pc & 3 != 0 || state.npc & 3 != 0 {
        return Err(OpenSparcT1Error::InvalidState {
            reason: "PC and nPC must be word aligned",
        });
    }
    if state.pstate.cle {
        return Err(OpenSparcT1Error::InvalidState {
            reason: "little-endian implicit data accesses are outside this cell",
        });
    }
    if state.loaded_len == 0 {
        if state.loaded_origin != 0 {
            return Err(OpenSparcT1Error::InvalidState {
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
        return Err(OpenSparcT1Error::InvalidState {
            reason: "installed program range is invalid",
        });
    }
    Ok(())
}

fn register_physical_index(state: &OpenSparcT1State, index: u8) -> Result<usize, OpenSparcT1Error> {
    if index >= 32 {
        return Err(OpenSparcT1Error::InvalidRegister { index });
    }
    let physical = match index {
        0..=7 => usize::from(state.gl) * 8 + usize::from(index),
        8..=23 => 32 + usize::from(state.cwp) * 16 + usize::from(index - 8),
        _ => {
            let caller = (state.cwp + WINDOWS - 1) % WINDOWS;
            32 + usize::from(caller) * 16 + usize::from(index - 24)
        }
    };
    Ok(physical)
}

fn read_register(state: &OpenSparcT1State, index: u8) -> Result<u64, OpenSparcT1Error> {
    if index == 0 {
        return Ok(0);
    }
    Ok(state.registers[register_physical_index(state, index)?])
}

fn write_register(
    state: &mut OpenSparcT1State,
    index: u8,
    value: u64,
) -> Result<(), OpenSparcT1Error> {
    if index == 0 {
        return Ok(());
    }
    let physical = register_physical_index(state, index)?;
    state.registers[physical] = value;
    Ok(())
}

fn checked_range(address: u64, width: usize) -> Result<std::ops::Range<usize>, OpenSparcT1Error> {
    if !matches!(width, 1 | 2 | 4 | 8) {
        return Err(OpenSparcT1Error::DataOutOfRange { address, width });
    }
    if !address.is_multiple_of(width as u64) {
        return Err(OpenSparcT1Error::MisalignedData { address, width });
    }
    let start = usize::try_from(address)
        .map_err(|_| OpenSparcT1Error::DataOutOfRange { address, width })?;
    let end = start
        .checked_add(width)
        .filter(|end| *end <= MEMORY_SIZE)
        .ok_or(OpenSparcT1Error::DataOutOfRange { address, width })?;
    Ok(start..end)
}

fn read_be(state: &OpenSparcT1State, address: u64, width: usize) -> Result<u64, OpenSparcT1Error> {
    let range = checked_range(address, width)?;
    Ok(state.memory[range]
        .iter()
        .fold(0_u64, |value, byte| (value << 8) | u64::from(*byte)))
}

fn write_be(
    state: &mut OpenSparcT1State,
    address: u64,
    width: usize,
    value: u64,
) -> Result<(), OpenSparcT1Error> {
    let range = checked_range(address, width)?;
    for (offset, byte) in state.memory[range].iter_mut().enumerate() {
        let shift = (width - offset - 1) * 8;
        *byte = (value >> shift) as u8;
    }
    Ok(())
}

fn fetch(state: &OpenSparcT1State) -> Result<u32, OpenSparcT1Error> {
    if state.pc & 3 != 0 {
        return Err(OpenSparcT1Error::MisalignedFetch { pc: state.pc });
    }
    if state.loaded_len < 4 {
        return Err(OpenSparcT1Error::FetchOutsideProgram { pc: state.pc });
    }
    let relative = state.pc.checked_sub(state.loaded_origin);
    if relative.is_none_or(|offset| offset > state.loaded_len.saturating_sub(4) as u64) {
        return Err(OpenSparcT1Error::FetchOutsideProgram { pc: state.pc });
    }
    let start = usize::try_from(state.pc)
        .map_err(|_| OpenSparcT1Error::FetchOutsideProgram { pc: state.pc })?;
    let bytes: [u8; 4] = state.memory[start..start + 4]
        .try_into()
        .expect("validated fetch range is four bytes");
    Ok(u32::from_be_bytes(bytes))
}

fn sign_extend(value: u32, bits: u32) -> i64 {
    let shift = 64 - bits;
    ((u64::from(value) << shift) as i64) >> shift
}

fn operand2(state: &OpenSparcT1State, word: u32) -> Result<u64, OpenSparcT1Error> {
    if word & (1 << 13) != 0 {
        Ok(sign_extend(word & 0x1fff, 13) as u64)
    } else {
        if word & 0x1fe0 != 0 {
            return Err(OpenSparcT1Error::UnknownInstruction { word, pc: state.pc });
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
pub fn classify_word(word: u32) -> Result<InstructionClass, OpenSparcT1Error> {
    classify_at(word, 0)
}

fn classify_at(word: u32, pc: u64) -> Result<InstructionClass, OpenSparcT1Error> {
    match word >> 30 {
        1 => Ok(InstructionClass::Call),
        0 => match (word >> 22) & 7 {
            1 => Ok(InstructionClass::PredictedBranch),
            2 => Ok(InstructionClass::Branch),
            3 => Ok(InstructionClass::RegisterBranch),
            4 => Ok(InstructionClass::Sethi),
            5..=7 => Err(OpenSparcT1Error::UnsupportedInstruction { word, pc }),
            _ => Err(OpenSparcT1Error::UnknownInstruction { word, pc }),
        },
        2 => match op3(word) {
            0x00 | 0x04 | 0x08 | 0x0c | 0x10 | 0x14 | 0x18 | 0x1c => {
                Ok(InstructionClass::Arithmetic)
            }
            0x01..=0x03 | 0x05..=0x07 | 0x11..=0x13 | 0x15..=0x17 => Ok(InstructionClass::Logical),
            0x25..=0x27 => Ok(InstructionClass::Shift),
            0x09 | 0x0d | 0x2d => Ok(InstructionClass::MultiplyDivide),
            0x38 => Ok(InstructionClass::ControlTransfer),
            0x3c | 0x3d => Ok(InstructionClass::Window),
            0x3a => Ok(InstructionClass::Trap),
            0x28..=0x37 | 0x39 | 0x3b | 0x3e | 0x3f => {
                Err(OpenSparcT1Error::UnsupportedInstruction { word, pc })
            }
            _ => Err(OpenSparcT1Error::UnknownInstruction { word, pc }),
        },
        3 => match op3(word) {
            0x00..=0x02 | 0x04..=0x06 | 0x08..=0x0b | 0x0e => Ok(InstructionClass::Memory),
            0x10..=0x3f => Err(OpenSparcT1Error::UnsupportedInstruction { word, pc }),
            _ => Err(OpenSparcT1Error::UnknownInstruction { word, pc }),
        },
        _ => unreachable!(),
    }
}

fn is_delayed_control_transfer(class: InstructionClass) -> bool {
    matches!(
        class,
        InstructionClass::Call
            | InstructionClass::Branch
            | InstructionClass::PredictedBranch
            | InstructionClass::RegisterBranch
            | InstructionClass::ControlTransfer
    )
}

fn advance(state: &mut OpenSparcT1State) {
    state.pc = state.npc;
    state.npc = state.npc.wrapping_add(4);
    state.delay_slot = false;
}

fn delayed_transfer(state: &mut OpenSparcT1State, target: u64) -> Result<(), OpenSparcT1Error> {
    if target & 3 != 0 {
        return Err(OpenSparcT1Error::MisalignedTarget { target });
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

fn set_add_flags(state: &mut OpenSparcT1State, a: u64, b: u64, carry: bool) {
    state.ccr.icc = add_flags_32(a as u32, b as u32, carry);
    state.ccr.xcc = add_flags_64(a, b, carry);
}

fn set_sub_flags(state: &mut OpenSparcT1State, a: u64, b: u64, borrow: bool) {
    state.ccr.icc = sub_flags_32(a as u32, b as u32, borrow);
    state.ccr.xcc = sub_flags_64(a, b, borrow);
}

#[allow(clippy::too_many_lines)]
fn execute_word(
    state: &mut OpenSparcT1State,
    word: u32,
    class: InstructionClass,
) -> Result<&'static str, OpenSparcT1Error> {
    let old_pc = state.pc;
    match class {
        InstructionClass::Call => {
            let displacement = sign_extend(word & 0x3fff_ffff, 30) << 2;
            let target = old_pc.wrapping_add_signed(displacement);
            write_register(state, 15, old_pc)?;
            delayed_transfer(state, target)?;
            Ok("call")
        }
        InstructionClass::Sethi => {
            let destination = rd(word);
            let value = u64::from((word & 0x3f_ffff) << 10);
            write_register(state, destination, value)?;
            advance(state);
            Ok(if destination == 0 && value == 0 {
                "nop"
            } else {
                "sethi"
            })
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
                        return Err(OpenSparcT1Error::UnknownInstruction { word, pc: old_pc });
                    }
                };
                (cc, sign_extend(word & 0x7_ffff, 19) << 2)
            } else {
                (state.ccr.icc, sign_extend(word & 0x3f_ffff, 22) << 2)
            };
            let taken = branch_condition(condition, cc);
            let target = old_pc.wrapping_add_signed(displacement);
            if target & 3 != 0 {
                return Err(OpenSparcT1Error::MisalignedTarget { target });
            }
            if annul && (condition == 8 || !taken) {
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
            Ok(if class == InstructionClass::PredictedBranch {
                predicted_branch_mnemonic(condition)
            } else {
                integer_branch_mnemonic(condition)
            })
        }
        InstructionClass::RegisterBranch => execute_register_branch(state, word),
        InstructionClass::Arithmetic => execute_arithmetic(state, word),
        InstructionClass::Logical => execute_logical(state, word),
        InstructionClass::Shift => execute_shift(state, word),
        InstructionClass::MultiplyDivide => execute_multiply_divide(state, word),
        InstructionClass::ControlTransfer => {
            let a = read_register(state, rs1(word))?;
            let b = operand2(state, word)?;
            let target = a.wrapping_add(b);
            if target & 3 != 0 {
                return Err(OpenSparcT1Error::MisalignedTarget { target });
            }
            write_register(state, rd(word), old_pc)?;
            delayed_transfer(state, target)?;
            Ok("jmpl")
        }
        InstructionClass::Window => execute_window(state, word),
        InstructionClass::Memory => execute_memory(state, word),
        InstructionClass::Trap => {
            if word & (1 << 29) != 0 {
                return Err(OpenSparcT1Error::UnknownInstruction { word, pc: old_pc });
            }
            let cc = match (word >> 11) & 3 {
                0 => state.ccr.icc,
                2 => state.ccr.xcc,
                _ => {
                    return Err(OpenSparcT1Error::UnknownInstruction { word, pc: old_pc });
                }
            };
            let immediate = word & (1 << 13) != 0;
            if (immediate && word & 0x700 != 0) || (!immediate && word & 0x7e0 != 0) {
                return Err(OpenSparcT1Error::UnknownInstruction { word, pc: old_pc });
            }
            let condition = rd(word) & 0xf;
            let taken = branch_condition(condition, cc);
            if !taken {
                advance(state);
                return Ok(trap_mnemonic(condition));
            }
            let operand = if immediate {
                u64::from((word & 0xff) as u8)
            } else {
                read_register(state, (word & 0x1f) as u8)?
            };
            let mask = if state.pstate.privileged { 0xff } else { 0x7f };
            let number = (read_register(state, rs1(word))?.wrapping_add(operand) & mask) as u8;
            if condition == 8 && number == 0 {
                advance(state);
                state.halted = true;
                Ok("ta 0")
            } else {
                Err(OpenSparcT1Error::UnsupportedTrap { number, pc: old_pc })
            }
        }
    }
}

fn execute_arithmetic(
    state: &mut OpenSparcT1State,
    word: u32,
) -> Result<&'static str, OpenSparcT1Error> {
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
        0x00 => "add",
        0x04 => "sub",
        0x08 => "addc",
        0x0c => "subc",
        0x10 => "addcc",
        0x14 => "subcc",
        0x18 => "addccc",
        _ => "subccc",
    })
}

fn execute_register_branch(
    state: &mut OpenSparcT1State,
    word: u32,
) -> Result<&'static str, OpenSparcT1Error> {
    if word & (1 << 28) != 0 {
        return Err(OpenSparcT1Error::UnknownInstruction { word, pc: state.pc });
    }
    let condition = ((word >> 25) & 7) as u8;
    if condition == 0 || condition == 4 {
        return Err(OpenSparcT1Error::UnknownInstruction { word, pc: state.pc });
    }
    let value = read_register(state, rs1(word))? as i64;
    let taken = match condition {
        1 => value == 0,
        2 => value <= 0,
        3 => value < 0,
        5 => value != 0,
        6 => value > 0,
        _ => value >= 0,
    };
    let displacement = (((word >> 20) & 3) << 14) | (word & 0x3fff);
    let target = state
        .pc
        .wrapping_add_signed(sign_extend(displacement, 16) << 2);
    if target & 3 != 0 {
        return Err(OpenSparcT1Error::MisalignedTarget { target });
    }
    if !taken && word & (1 << 29) != 0 {
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
    Ok(register_branch_mnemonic(condition))
}

fn integer_branch_mnemonic(condition: u8) -> &'static str {
    [
        "bn", "be", "ble", "bl", "bleu", "bcs", "bneg", "bvs", "ba", "bne", "bg", "bge", "bgu",
        "bcc", "bpos", "bvc",
    ][usize::from(condition & 0xf)]
}

fn predicted_branch_mnemonic(condition: u8) -> &'static str {
    [
        "bpn", "bpe", "bple", "bpl", "bpleu", "bpcs", "bpneg", "bpvs", "bpa", "bpne", "bpg",
        "bpge", "bpgu", "bpcc", "bppos", "bpvc",
    ][usize::from(condition & 0xf)]
}

fn register_branch_mnemonic(condition: u8) -> &'static str {
    match condition {
        1 => "brz",
        2 => "brlez",
        3 => "brlz",
        5 => "brnz",
        6 => "brgz",
        _ => "brgez",
    }
}

fn trap_mnemonic(condition: u8) -> &'static str {
    [
        "tn", "te", "tle", "tl", "tleu", "tcs", "tneg", "tvs", "ta", "tne", "tg", "tge", "tgu",
        "tcc", "tpos", "tvc",
    ][usize::from(condition & 0xf)]
}

fn execute_logical(
    state: &mut OpenSparcT1State,
    word: u32,
) -> Result<&'static str, OpenSparcT1Error> {
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
        0x01 => "and",
        0x02 => "or",
        0x03 => "xor",
        0x05 => "andn",
        0x06 => "orn",
        0x07 => "xnor",
        0x11 => "andcc",
        0x12 => "orcc",
        0x13 => "xorcc",
        0x15 => "andncc",
        0x16 => "orncc",
        _ => "xnorcc",
    })
}

fn execute_shift(
    state: &mut OpenSparcT1State,
    word: u32,
) -> Result<&'static str, OpenSparcT1Error> {
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
        return Err(OpenSparcT1Error::UnknownInstruction { word, pc: state.pc });
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
        (0x25, false) => "sll",
        (0x25, true) => "sllx",
        (0x26, false) => "srl",
        (0x26, true) => "srlx",
        (0x27, false) => "sra",
        _ => "srax",
    })
}

fn execute_multiply_divide(
    state: &mut OpenSparcT1State,
    word: u32,
) -> Result<&'static str, OpenSparcT1Error> {
    let a = read_register(state, rs1(word))?;
    let b = operand2(state, word)?;
    let (result, mnemonic) = match op3(word) {
        0x09 => (a.wrapping_mul(b), "mulx"),
        0x0d => {
            if b == 0 {
                return Err(OpenSparcT1Error::DivideByZero);
            }
            (a / b, "udivx")
        }
        _ => {
            if b == 0 {
                return Err(OpenSparcT1Error::DivideByZero);
            }
            let result = if a == i64::MIN as u64 && b == u64::MAX {
                i64::MIN as u64
            } else {
                ((a as i64) / (b as i64)) as u64
            };
            (result, "sdivx")
        }
    };
    write_register(state, rd(word), result)?;
    advance(state);
    Ok(mnemonic)
}

fn execute_window(
    state: &mut OpenSparcT1State,
    word: u32,
) -> Result<&'static str, OpenSparcT1Error> {
    let a = read_register(state, rs1(word))?;
    let b = operand2(state, word)?;
    let result = a.wrapping_add(b);
    if op3(word) == 0x3c {
        if state.cansave == 0 {
            return Err(OpenSparcT1Error::WindowSpill);
        }
        if state.cleanwin == state.canrestore {
            return Err(OpenSparcT1Error::CleanWindow);
        }
        state.cwp = (state.cwp + 1) % WINDOWS;
        state.cansave -= 1;
        state.canrestore += 1;
        write_register(state, rd(word), result)?;
        advance(state);
        Ok("save")
    } else {
        if state.canrestore == 0 {
            return Err(OpenSparcT1Error::WindowFill);
        }
        state.cwp = (state.cwp + WINDOWS - 1) % WINDOWS;
        state.canrestore -= 1;
        state.cansave += 1;
        write_register(state, rd(word), result)?;
        advance(state);
        Ok("restore")
    }
}

fn execute_memory(
    state: &mut OpenSparcT1State,
    word: u32,
) -> Result<&'static str, OpenSparcT1Error> {
    let address = read_register(state, rs1(word))?.wrapping_add(operand2(state, word)?);
    let destination = rd(word);
    let mnemonic = match op3(word) {
        0x00 => {
            let value = read_be(state, address, 4)?;
            write_register(state, destination, value)?;
            "lduw"
        }
        0x01 => {
            let value = read_be(state, address, 1)?;
            write_register(state, destination, value)?;
            "ldub"
        }
        0x02 => {
            let value = read_be(state, address, 2)?;
            write_register(state, destination, value)?;
            "lduh"
        }
        0x04 => {
            write_be(state, address, 4, read_register(state, destination)?)?;
            "stw"
        }
        0x05 => {
            write_be(state, address, 1, read_register(state, destination)?)?;
            "stb"
        }
        0x06 => {
            write_be(state, address, 2, read_register(state, destination)?)?;
            "sth"
        }
        0x08 => {
            let value = read_be(state, address, 4)? as u32 as i32 as i64 as u64;
            write_register(state, destination, value)?;
            "ldsw"
        }
        0x09 => {
            let value = read_be(state, address, 1)? as u8 as i8 as i64 as u64;
            write_register(state, destination, value)?;
            "ldsb"
        }
        0x0a => {
            let value = read_be(state, address, 2)? as u16 as i16 as i64 as u64;
            write_register(state, destination, value)?;
            "ldsh"
        }
        0x0b => {
            let value = read_be(state, address, 8)?;
            write_register(state, destination, value)?;
            "ldx"
        }
        _ => {
            write_be(state, address, 8, read_register(state, destination)?)?;
            "stx"
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
