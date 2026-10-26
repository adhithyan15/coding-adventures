//! Independent gate transition engine for one UltraSPARC T1 strand.
//!
//! The real T1 composes 32 of these architectural contexts around shared
//! execution, cache, translation, crossbar, and memory-controller structures.
//! This crate intentionally owns only the scalar integer contract that those
//! later gate and SoC models can use as their instruction-boundary oracle.

use core::fmt;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

use crate::gates;

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

pub(crate) fn step_state(mut state: OpenSparcT1State) -> Result<StepTrace, OpenSparcT1Error> {
    if state.halted {
        return Err(OpenSparcT1Error::Halted);
    }
    let pc = state.pc;
    let npc = state.npc;
    let word = fetch(&state)?;
    let class = classify_at(word, pc)?;
    if state.delay_slot && is_delayed_control_transfer(class) {
        return Err(OpenSparcT1Error::ControlTransferInDelaySlot { pc });
    }
    let mnemonic = execute_word(&mut state, word, class)?;
    Ok(StepTrace {
        pc_before: pc,
        npc_before: npc,
        pc_after: state.pc,
        npc_after: state.npc,
        raw: word.to_be_bytes(),
        raw_word: word,
        mnemonic,
        state_after: state,
    })
}

pub(crate) fn validate_program(program: &[u8], origin: u64) -> Result<(), OpenSparcT1Error> {
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

pub(crate) fn validate_state(state: &OpenSparcT1State) -> Result<(), OpenSparcT1Error> {
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
        0..=7 => {
            gates::add_64(
                gates::shift_left_64(u64::from(state.gl), 3),
                u64::from(index),
                0,
            )
            .0 as usize
        }
        8..=23 => {
            gates::add_64(
                gates::add_64(32, gates::shift_left_64(u64::from(state.cwp), 4), 0).0,
                u64::from(index - 8),
                0,
            )
            .0 as usize
        }
        _ => {
            let caller = gates::sub_64(u64::from(state.cwp), 1, 0).0 & 7;
            gates::add_64(
                gates::add_64(32, gates::shift_left_64(caller, 4), 0).0,
                u64::from(index - 24),
                0,
            )
            .0 as usize
        }
    };
    Ok(physical)
}

pub(crate) fn read_register(state: &OpenSparcT1State, index: u8) -> Result<u64, OpenSparcT1Error> {
    if index == 0 {
        return Ok(0);
    }
    Ok(state.registers[register_physical_index(state, index)?])
}

pub(crate) fn write_register(
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
    let address_bits = gates::u64_to_bits(address);
    let alignment_bits = width.trailing_zeros() as usize;
    if address_bits[..alignment_bits]
        .iter()
        .copied()
        .fold(0, or_gate)
        == 1
    {
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

pub(crate) fn read_be(
    state: &OpenSparcT1State,
    address: u64,
    width: usize,
) -> Result<u64, OpenSparcT1Error> {
    let range = checked_range(address, width)?;
    Ok(state.memory[range].iter().fold(0_u64, |value, byte| {
        gates::bit_or_64(gates::shift_left_64(value, 8), u64::from(*byte))
    }))
}

pub(crate) fn write_be(
    state: &mut OpenSparcT1State,
    address: u64,
    width: usize,
    value: u64,
) -> Result<(), OpenSparcT1Error> {
    let range = checked_range(address, width)?;
    for (offset, byte) in state.memory[range].iter_mut().enumerate() {
        let shift = (width - offset - 1) * 8;
        let bits = gates::u64_to_bits(value);
        *byte = gates::bits_to_u8(&std::array::from_fn::<_, 8, _>(|bit| bits[shift + bit]));
    }
    Ok(())
}

fn fetch(state: &OpenSparcT1State) -> Result<u32, OpenSparcT1Error> {
    if gates::u64_to_bits(state.pc)[..2]
        .iter()
        .copied()
        .fold(0, or_gate)
        == 1
    {
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
    Ok(bytes.iter().fold(0_u32, |value, byte| {
        gates::bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
            if bit < 8 {
                gates::u8_to_bits(*byte)[bit]
            } else {
                gates::u32_to_bits(value)[bit - 8]
            }
        }))
    }))
}

fn gate_field(word: u32, start: usize, width: usize) -> u32 {
    let bits = gates::u32_to_bits(word);
    gates::bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        if bit < width {
            bits[start + bit]
        } else {
            0
        }
    }))
}

fn sign_extend(value: u32, bits: usize) -> i64 {
    gates::sign_extend(u64::from(value), bits) as i64
}

fn operand2(state: &OpenSparcT1State, word: u32) -> Result<u64, OpenSparcT1Error> {
    if gates::u32_to_bits(word)[13] == 1 {
        Ok(sign_extend(gate_field(word, 0, 13), 13) as u64)
    } else {
        if gates::u32_to_bits(word)[5..13]
            .iter()
            .copied()
            .fold(0, or_gate)
            == 1
        {
            return Err(OpenSparcT1Error::UnknownInstruction { word, pc: state.pc });
        }
        read_register(state, gate_field(word, 0, 5) as u8)
    }
}

fn rs1(word: u32) -> u8 {
    gate_field(word, 14, 5) as u8
}

fn rd(word: u32) -> u8 {
    gate_field(word, 25, 5) as u8
}

fn op3(word: u32) -> u8 {
    gates::decode_op3(word)
        .iter()
        .position(|line| *line == 1)
        .expect("the six-bit op3 decoder always selects one line") as u8
}

/// Classify one raw word without executing it.
pub fn classify_word(word: u32) -> Result<InstructionClass, OpenSparcT1Error> {
    classify_at(word, 0)
}

fn classify_at(word: u32, pc: u64) -> Result<InstructionClass, OpenSparcT1Error> {
    let formats = gates::decode_format(word);
    let format = formats
        .iter()
        .position(|line| *line == 1)
        .expect("the two-bit format decoder always selects one line");
    match format {
        1 => Ok(InstructionClass::Call),
        0 => match gate_field(word, 22, 3) {
            1 => Ok(InstructionClass::PredictedBranch),
            2 => Ok(InstructionClass::Branch),
            3 => Ok(InstructionClass::RegisterBranch),
            4 => Ok(InstructionClass::Sethi),
            5..=7 => Err(OpenSparcT1Error::UnsupportedInstruction { word, pc }),
            _ => Err(OpenSparcT1Error::UnknownInstruction { word, pc }),
        },
        2 => match gates::decode_op3(word)
            .iter()
            .position(|line| *line == 1)
            .expect("the six-bit op3 decoder always selects one line")
        {
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
        3 => match gates::decode_op3(word)
            .iter()
            .position(|line| *line == 1)
            .expect("the six-bit op3 decoder always selects one line")
        {
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
    state.npc = gates::add_64(state.npc, 4, 0).0;
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
    let n = u8::from(cc.n);
    let z = u8::from(cc.z);
    let v = u8::from(cc.v);
    let c = u8::from(cc.c);
    let nv = xor_gate(n, v);
    let lines = [
        0,
        z,
        or_gate(z, nv),
        nv,
        or_gate(c, z),
        c,
        n,
        v,
        1,
        not_gate(z),
        and_gate(not_gate(z), not_gate(nv)),
        not_gate(nv),
        and_gate(not_gate(c), not_gate(z)),
        not_gate(c),
        not_gate(n),
        not_gate(v),
    ];
    lines[usize::from(condition & 0xf)] == 1
}

fn add_flags_32(a: u32, b: u32, carry: bool) -> ConditionCodes {
    let (result, carry_out, overflow) = gates::add_32(a, b, u8::from(carry));
    ConditionCodes {
        n: gates::u32_to_bits(result)[31] == 1,
        z: gates::is_zero_32(result) == 1,
        v: overflow == 1,
        c: carry_out == 1,
    }
}

fn add_flags_64(a: u64, b: u64, carry: bool) -> ConditionCodes {
    let (result, carry_out, overflow) = gates::add_64(a, b, u8::from(carry));
    ConditionCodes {
        n: gates::u64_to_bits(result)[63] == 1,
        z: gates::is_zero_64(result) == 1,
        v: overflow == 1,
        c: carry_out == 1,
    }
}

fn sub_flags_32(a: u32, b: u32, borrow: bool) -> ConditionCodes {
    let (result, borrow_out, overflow) = gates::sub_32(a, b, u8::from(borrow));
    ConditionCodes {
        n: gates::u32_to_bits(result)[31] == 1,
        z: gates::is_zero_32(result) == 1,
        v: overflow == 1,
        c: borrow_out == 1,
    }
}

fn sub_flags_64(a: u64, b: u64, borrow: bool) -> ConditionCodes {
    let (result, borrow_out, overflow) = gates::sub_64(a, b, u8::from(borrow));
    ConditionCodes {
        n: gates::u64_to_bits(result)[63] == 1,
        z: gates::is_zero_64(result) == 1,
        v: overflow == 1,
        c: borrow_out == 1,
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
            gates::u64_to_bits(value)[31] == 1
        } else {
            gates::u64_to_bits(value)[63] == 1
        },
        z: gates::is_zero_64(value) == 1,
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
            let displacement =
                gates::shift_left_64(sign_extend(gate_field(word, 0, 30), 30) as u64, 2);
            let target = gates::add_64(old_pc, displacement, 0).0;
            write_register(state, 15, old_pc)?;
            delayed_transfer(state, target)?;
            Ok("call")
        }
        InstructionClass::Sethi => {
            let destination = rd(word);
            let value = gates::shift_left_64(u64::from(gate_field(word, 0, 22)), 10);
            write_register(state, destination, value)?;
            advance(state);
            Ok(if destination == 0 && value == 0 {
                "nop"
            } else {
                "sethi"
            })
        }
        InstructionClass::Branch | InstructionClass::PredictedBranch => {
            let condition = gate_field(word, 25, 4) as u8;
            let annul = gates::u32_to_bits(word)[29] == 1;
            let (cc, displacement) = if class == InstructionClass::PredictedBranch {
                let selector = gate_field(word, 20, 2);
                let cc = match selector {
                    0 => state.ccr.icc,
                    2 => state.ccr.xcc,
                    _ => {
                        return Err(OpenSparcT1Error::UnknownInstruction { word, pc: old_pc });
                    }
                };
                (
                    cc,
                    gates::shift_left_64(sign_extend(gate_field(word, 0, 19), 19) as u64, 2) as i64,
                )
            } else {
                (
                    state.ccr.icc,
                    gates::shift_left_64(sign_extend(gate_field(word, 0, 22), 22) as u64, 2) as i64,
                )
            };
            let taken = branch_condition(condition, cc);
            let target = gates::add_64(old_pc, displacement as u64, 0).0;
            if target & 3 != 0 {
                return Err(OpenSparcT1Error::MisalignedTarget { target });
            }
            if annul && (condition == 8 || !taken) {
                if condition == 8 {
                    state.pc = target;
                    state.npc = gates::add_64(target, 4, 0).0;
                } else {
                    state.pc = gates::add_64(state.npc, 4, 0).0;
                    state.npc = gates::add_64(state.npc, 8, 0).0;
                }
                state.delay_slot = false;
            } else {
                let next = if taken {
                    target
                } else {
                    gates::add_64(state.npc, 4, 0).0
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
            let target = gates::add_64(a, b, 0).0;
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
            if gates::u32_to_bits(word)[29] == 1 {
                return Err(OpenSparcT1Error::UnknownInstruction { word, pc: old_pc });
            }
            let cc = match gate_field(word, 11, 2) {
                0 => state.ccr.icc,
                2 => state.ccr.xcc,
                _ => {
                    return Err(OpenSparcT1Error::UnknownInstruction { word, pc: old_pc });
                }
            };
            let immediate = gates::u32_to_bits(word)[13] == 1;
            if (immediate && gate_field(word, 8, 3) != 0)
                || (!immediate && gate_field(word, 5, 6) != 0)
            {
                return Err(OpenSparcT1Error::UnknownInstruction { word, pc: old_pc });
            }
            let condition = rd(word) & 0xf;
            let taken = branch_condition(condition, cc);
            if !taken {
                advance(state);
                return Ok(trap_mnemonic(condition));
            }
            let operand = if immediate {
                u64::from(gate_field(word, 0, 8))
            } else {
                read_register(state, gate_field(word, 0, 5) as u8)?
            };
            let mask = if state.pstate.privileged { 0xff } else { 0x7f };
            let number =
                (gates::add_64(read_register(state, rs1(word))?, operand, 0).0 & mask) as u8;
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
        gates::add_64(a, b, u8::from(carry)).0
    } else {
        gates::sub_64(a, b, u8::from(borrow)).0
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
    if gates::u32_to_bits(word)[28] == 1 {
        return Err(OpenSparcT1Error::UnknownInstruction { word, pc: state.pc });
    }
    let condition = gate_field(word, 25, 3) as u8;
    if condition == 0 || condition == 4 {
        return Err(OpenSparcT1Error::UnknownInstruction { word, pc: state.pc });
    }
    let value = read_register(state, rs1(word))?;
    let zero = gates::is_zero_64(value);
    let negative = gates::u64_to_bits(value)[63];
    let taken = match condition {
        1 => zero,
        2 => or_gate(zero, negative),
        3 => negative,
        5 => not_gate(zero),
        6 => and_gate(not_gate(zero), not_gate(negative)),
        _ => not_gate(negative),
    } == 1;
    let displacement = gates::bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| match bit {
        0..=13 => gates::u32_to_bits(word)[bit],
        14..=15 => gates::u32_to_bits(word)[bit + 6],
        _ => 0,
    }));
    let target = gates::add_64(
        state.pc,
        gates::shift_left_64(sign_extend(displacement, 16) as u64, 2),
        0,
    )
    .0;
    if target & 3 != 0 {
        return Err(OpenSparcT1Error::MisalignedTarget { target });
    }
    if !taken && gates::u32_to_bits(word)[29] == 1 {
        state.pc = gates::add_64(state.npc, 4, 0).0;
        state.npc = gates::add_64(state.npc, 8, 0).0;
        state.delay_slot = false;
    } else {
        let next = if taken {
            target
        } else {
            gates::add_64(state.npc, 4, 0).0
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
        1 => gates::bit_and_64(a, b),
        2 => gates::bit_or_64(a, b),
        3 => gates::bit_xor_64(a, b),
        5 => gates::bit_and_64(a, gates::bit_not_64(b)),
        6 => gates::bit_or_64(a, gates::bit_not_64(b)),
        _ => gates::bit_not_64(gates::bit_xor_64(a, b)),
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
    let extended = gates::u32_to_bits(word)[12] == 1;
    let immediate = gates::u32_to_bits(word)[13] == 1;
    let reserved_mask: u32 = if immediate {
        if extended {
            0x0fc0
        } else {
            0x0fe0
        }
    } else {
        0x0fe0
    };
    if gates::bit_and_64(u64::from(word), u64::from(reserved_mask)) != 0 {
        return Err(OpenSparcT1Error::UnknownInstruction { word, pc: state.pc });
    }
    let count = if immediate {
        if extended {
            gate_field(word, 0, 6)
        } else {
            gate_field(word, 0, 5)
        }
    } else {
        let raw = read_register(state, gate_field(word, 0, 5) as u8)?;
        (raw & if extended { 63 } else { 31 }) as u32
    };
    let result = match (op3(word), extended) {
        (0x25, true) => gates::shift_left_64(a, count),
        (0x26, true) => gates::shift_right_64(a, count),
        (0x27, true) => gates::shift_arithmetic_right_64(a, count),
        (0x25, false) => gates::shift_left_64(a, count),
        (0x26, false) => gates::shift_right_64(a & 0xffff_ffff, count),
        _ => gates::sign_extend(gates::shift_arithmetic_right_64(a & 0xffff_ffff, count), 32),
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
        0x09 => (gates::multiply_unsigned_64(a, b), "mulx"),
        0x0d => {
            if b == 0 {
                return Err(OpenSparcT1Error::DivideByZero);
            }
            (gates::divide_unsigned_64(a, b), "udivx")
        }
        _ => {
            if b == 0 {
                return Err(OpenSparcT1Error::DivideByZero);
            }
            let result = if a == i64::MIN as u64 && b == u64::MAX {
                i64::MIN as u64
            } else {
                gates::divide_signed_64(a, b)
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
    let result = gates::add_64(a, b, 0).0;
    if op3(word) == 0x3c {
        if state.cansave == 0 {
            return Err(OpenSparcT1Error::WindowSpill);
        }
        if state.cleanwin == state.canrestore {
            return Err(OpenSparcT1Error::CleanWindow);
        }
        state.cwp = gates::add_64(u64::from(state.cwp), 1, 0).0 as u8 & 7;
        state.cansave = gates::sub_64(u64::from(state.cansave), 1, 0).0 as u8;
        state.canrestore = gates::add_64(u64::from(state.canrestore), 1, 0).0 as u8;
        write_register(state, rd(word), result)?;
        advance(state);
        Ok("save")
    } else {
        if state.canrestore == 0 {
            return Err(OpenSparcT1Error::WindowFill);
        }
        state.cwp = gates::sub_64(u64::from(state.cwp), 1, 0).0 as u8 & 7;
        state.canrestore = gates::sub_64(u64::from(state.canrestore), 1, 0).0 as u8;
        state.cansave = gates::add_64(u64::from(state.cansave), 1, 0).0 as u8;
        write_register(state, rd(word), result)?;
        advance(state);
        Ok("restore")
    }
}

fn execute_memory(
    state: &mut OpenSparcT1State,
    word: u32,
) -> Result<&'static str, OpenSparcT1Error> {
    let address = gates::add_64(read_register(state, rs1(word))?, operand2(state, word)?, 0).0;
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
            let value = gates::sign_extend(read_be(state, address, 4)?, 32);
            write_register(state, destination, value)?;
            "ldsw"
        }
        0x09 => {
            let value = gates::sign_extend(read_be(state, address, 1)?, 8);
            write_register(state, destination, value)?;
            "ldsb"
        }
        0x0a => {
            let value = gates::sign_extend(read_be(state, address, 2)?, 16);
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
