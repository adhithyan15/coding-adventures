//! Checked ATmega328P/`avr5` classic AVR functional simulator.
//!
//! AVR's pleasant-looking byte registers hide three important boundaries:
//! Flash is Harvard and word-addressed by the PC, registers and status are
//! memory mapped, and updating pointer instructions have explicitly undefined
//! overlap cases.  This implementation makes all three visible.  A step runs
//! on a disposable complete state and commits only after every check succeeds.

use core::fmt;

pub const PROGRAM_SIZE: usize = 32 * 1024;
pub const DATA_SIZE: usize = 0x0900;
pub const RAM_END: u16 = 0x08ff;
pub const MAX_RETAINED_STEPS: usize = 64;
pub const SREG_ADDR: usize = 0x005f;
pub const SPL_ADDR: usize = 0x005d;
pub const SPH_ADDR: usize = 0x005e;

pub const FLAG_C: u8 = 1 << 0;
pub const FLAG_Z: u8 = 1 << 1;
pub const FLAG_N: u8 = 1 << 2;
pub const FLAG_V: u8 = 1 << 3;
pub const FLAG_S: u8 = 1 << 4;
pub const FLAG_H: u8 = 1 << 5;
pub const FLAG_T: u8 = 1 << 6;
pub const FLAG_I: u8 = 1 << 7;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AvrState {
    pub pc: u16,
    pub program: Vec<u8>,
    pub data: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: usize,
    pub loaded_len: usize,
}

impl Default for AvrState {
    fn default() -> Self {
        let mut data = vec![0; DATA_SIZE];
        data[SPL_ADDR] = RAM_END as u8;
        data[SPH_ADDR] = (RAM_END >> 8) as u8;
        Self {
            pc: 0,
            program: vec![0; PROGRAM_SIZE],
            data,
            halted: false,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }
}

impl AvrState {
    #[must_use]
    pub fn register(&self, index: usize) -> Option<u8> {
        self.data.get(index).copied().filter(|_| index < 32)
    }

    #[must_use]
    pub fn sreg(&self) -> u8 {
        self.data[SREG_ADDR]
    }

    #[must_use]
    pub fn sp(&self) -> u16 {
        u16::from_le_bytes([self.data[SPL_ADDR], self.data[SPH_ADDR]])
    }

    fn set_sp(&mut self, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.data[SPL_ADDR] = lo;
        self.data[SPH_ADDR] = hi;
    }

    fn pointer(&self, low: usize) -> u16 {
        u16::from_le_bytes([self.data[low], self.data[low + 1]])
    }

    fn set_pointer(&mut self, low: usize, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.data[low] = lo;
        self.data[low + 1] = hi;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    pub fetched: Vec<u8>,
    pub raw_words: Vec<u16>,
    pub width_words: u8,
    pub mnemonic: &'static str,
    pub branch_taken: bool,
    pub skipped: bool,
    pub state_after: AvrState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: AvrState,
}

/// Deterministic trace-allocation policy used by lifecycle tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TraceAllocation {
    #[default]
    Normal,
    Fail,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AvrError {
    EmptyProgram,
    MisalignedProgram { origin: usize, len: usize },
    ProgramOutOfRange { origin: usize, len: usize },
    FetchOutsideProgram { pc: u16 },
    TruncatedInstruction { pc: u16 },
    DataOutOfRange { address: u16 },
    ProgramReadOutOfRange { address: u16 },
    StackOutOfRange { sp: u16, width: usize },
    InvalidStackPointer { sp: u16 },
    InvalidState { reason: &'static str },
    UnknownOpcode { opcode: u16, pc: u16 },
    UnsupportedProfileInstruction { opcode: u16, pc: u16 },
    UnsupportedSystemInstruction { opcode: u16, pc: u16 },
    UnsupportedOperandCombination { opcode: u16, pc: u16 },
    Halted,
    RetentionLimitExceeded { requested: usize, limit: usize },
    AllocationFailed,
    StepLimitExceeded { limit: usize },
}

impl fmt::Display for AvrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => write!(f, "program image is empty"),
            Self::MisalignedProgram { origin, len } => write!(
                f,
                "program origin {origin:#x} and length {len} must be even"
            ),
            Self::ProgramOutOfRange { origin, len } => {
                write!(f, "program of {len} bytes does not fit at {origin:#x}")
            }
            Self::FetchOutsideProgram { pc } => write!(
                f,
                "fetch at word PC {pc:#06x} is outside the installed image"
            ),
            Self::TruncatedInstruction { pc } => {
                write!(f, "two-word instruction at word PC {pc:#06x} is truncated")
            }
            Self::DataOutOfRange { address } => {
                write!(f, "data address {address:#06x} is out of range")
            }
            Self::ProgramReadOutOfRange { address } => {
                write!(f, "program byte address {address:#06x} is out of range")
            }
            Self::StackOutOfRange { sp, width } => write!(
                f,
                "{width}-byte stack transfer from SP {sp:#06x} is out of range"
            ),
            Self::InvalidStackPointer { sp } => {
                write!(f, "stack pointer {sp:#06x} is outside SRAM")
            }
            Self::InvalidState { reason } => write!(f, "invalid AVR state: {reason}"),
            Self::UnknownOpcode { opcode, pc } => {
                write!(f, "unknown opcode {opcode:#06x} at word PC {pc:#06x}")
            }
            Self::UnsupportedProfileInstruction { opcode, pc } => write!(
                f,
                "opcode {opcode:#06x} at word PC {pc:#06x} is unavailable on avr5"
            ),
            Self::UnsupportedSystemInstruction { opcode, pc } => write!(
                f,
                "system opcode {opcode:#06x} at word PC {pc:#06x} is unsupported"
            ),
            Self::UnsupportedOperandCombination { opcode, pc } => write!(
                f,
                "opcode {opcode:#06x} at word PC {pc:#06x} has a manual-undefined operand overlap"
            ),
            Self::Halted => write!(f, "processor is halted"),
            Self::RetentionLimitExceeded { requested, limit } => write!(
                f,
                "requested {requested} retained traces exceeds limit {limit}"
            ),
            Self::AllocationFailed => write!(f, "trace allocation failed"),
            Self::StepLimitExceeded { limit } => {
                write!(f, "step limit {limit} exhausted before BREAK")
            }
        }
    }
}

impl std::error::Error for AvrError {}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AvrClassicSimulator {
    state: AvrState,
}

#[derive(Clone, Copy)]
enum Update {
    None,
    PreDec,
    PostInc,
}

struct Transition {
    mnemonic: &'static str,
    width: u8,
    branch_taken: bool,
    skipped: bool,
}

impl AvrClassicSimulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn state(&self) -> &AvrState {
        &self.state
    }

    pub fn reset(&mut self) {
        self.state = AvrState::default();
    }

    pub fn load(&mut self, program: &[u8], origin: usize) -> Result<(), AvrError> {
        if program.is_empty() {
            return Err(AvrError::EmptyProgram);
        }
        if origin & 1 != 0 || program.len() & 1 != 0 {
            return Err(AvrError::MisalignedProgram {
                origin,
                len: program.len(),
            });
        }
        let end = origin
            .checked_add(program.len())
            .ok_or(AvrError::ProgramOutOfRange {
                origin,
                len: program.len(),
            })?;
        if end > PROGRAM_SIZE {
            return Err(AvrError::ProgramOutOfRange {
                origin,
                len: program.len(),
            });
        }
        let mut next = AvrState::default();
        next.program[origin..end].copy_from_slice(program);
        next.pc = (origin / 2) as u16;
        next.loaded_origin = origin;
        next.loaded_len = program.len();
        self.state = next;
        Ok(())
    }

    pub fn restore(&mut self, state: AvrState) -> Result<(), AvrError> {
        validate_state(&state)?;
        self.state = state;
        Ok(())
    }

    pub fn set_register(&mut self, index: usize, value: u8) -> Result<(), AvrError> {
        if index >= 32 {
            return Err(AvrError::InvalidState {
                reason: "register index exceeds R31",
            });
        }
        self.state.data[index] = value;
        Ok(())
    }

    pub fn set_data(&mut self, address: u16, value: u8) -> Result<(), AvrError> {
        let slot = self
            .state
            .data
            .get_mut(address as usize)
            .ok_or(AvrError::DataOutOfRange { address })?;
        *slot = value;
        Ok(())
    }

    pub fn set_pc(&mut self, pc: u16) {
        self.state.pc = pc;
    }

    pub fn step(&mut self) -> Result<StepTrace, AvrError> {
        if self.state.halted {
            return Err(AvrError::Halted);
        }
        let before = self.state.clone();
        let pc_before = before.pc;
        let first = fetch_word(&before, pc_before)?;
        let width = instruction_width(first);
        let second = if width == 2 {
            Some(fetch_second(&before, pc_before)?)
        } else {
            None
        };
        let byte = pc_before as usize * 2;
        let fetched = before.program[byte..byte + width as usize * 2].to_vec();
        let mut next = before.clone();
        let transition = execute_one(&mut next, first, second)?;
        let raw_words = second.map_or_else(|| vec![first], |word| vec![first, word]);
        let trace = StepTrace {
            pc_before,
            pc_after: next.pc,
            fetched,
            raw_words,
            width_words: transition.width,
            mnemonic: transition.mnemonic,
            branch_taken: transition.branch_taken,
            skipped: transition.skipped,
            state_after: next.clone(),
        };
        self.state = next;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, AvrError> {
        self.run_with_allocation(max_steps, TraceAllocation::Normal)
    }

    pub fn run_with_allocation(
        &mut self,
        max_steps: usize,
        allocation: TraceAllocation,
    ) -> Result<ExecutionResult, AvrError> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(AvrError::RetentionLimitExceeded {
                requested: max_steps,
                limit: MAX_RETAINED_STEPS,
            });
        }
        let original = self.state.clone();
        let mut traces = Vec::new();
        if allocation == TraceAllocation::Fail {
            return Err(AvrError::AllocationFailed);
        }
        traces
            .try_reserve_exact(max_steps)
            .map_err(|_| AvrError::AllocationFailed)?;
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => {
                    let halted = trace.state_after.halted;
                    traces.push(trace);
                    if halted {
                        return Ok(ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            traces,
                            final_state: self.state.clone(),
                        });
                    }
                }
                Err(error) => {
                    self.state = original;
                    return Err(error);
                }
            }
        }
        self.state = original;
        Err(AvrError::StepLimitExceeded { limit: max_steps })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: usize,
        max_steps: usize,
    ) -> Result<ExecutionResult, AvrError> {
        let original = self.state.clone();
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = original;
                Err(error)
            }
        }
    }
}

fn validate_state(state: &AvrState) -> Result<(), AvrError> {
    if state.program.len() != PROGRAM_SIZE {
        return Err(AvrError::InvalidState {
            reason: "program storage length is not 32 KiB",
        });
    }
    if state.data.len() != DATA_SIZE {
        return Err(AvrError::InvalidState {
            reason: "data storage length is not 0x900 bytes",
        });
    }
    if state.loaded_origin & 1 != 0 || state.loaded_len & 1 != 0 {
        return Err(AvrError::InvalidState {
            reason: "installed image is not word aligned",
        });
    }
    let end = state
        .loaded_origin
        .checked_add(state.loaded_len)
        .ok_or(AvrError::InvalidState {
            reason: "installed image overflows",
        })?;
    if end > PROGRAM_SIZE {
        return Err(AvrError::InvalidState {
            reason: "installed image exceeds Flash",
        });
    }
    let sp = state.sp();
    if !(0x0100..=RAM_END).contains(&sp) {
        return Err(AvrError::InvalidStackPointer { sp });
    }
    if state.loaded_len != 0 {
        validate_target(state, state.pc)?;
    }
    Ok(())
}

fn image_bounds(state: &AvrState) -> (usize, usize) {
    (state.loaded_origin, state.loaded_origin + state.loaded_len)
}

fn fetch_word(state: &AvrState, pc: u16) -> Result<u16, AvrError> {
    let address = pc as usize * 2;
    let (start, end) = image_bounds(state);
    if address < start || address.checked_add(2).is_none_or(|last| last > end) {
        return Err(AvrError::FetchOutsideProgram { pc });
    }
    Ok(u16::from_le_bytes([
        state.program[address],
        state.program[address + 1],
    ]))
}

fn fetch_second(state: &AvrState, pc: u16) -> Result<u16, AvrError> {
    fetch_word(state, pc.wrapping_add(1)).map_err(|_| AvrError::TruncatedInstruction { pc })
}

fn validate_target(state: &AvrState, pc: u16) -> Result<(), AvrError> {
    let word = fetch_word(state, pc)?;
    if instruction_width(word) == 2 {
        fetch_second(state, pc)?;
    }
    Ok(())
}

fn instruction_width(word: u16) -> u8 {
    if word & 0xfe0f == 0x9000
        || word & 0xfe0f == 0x9200
        || word & 0xfe0e == 0x940c
        || word & 0xfe0e == 0x940e
    {
        2
    } else {
        1
    }
}

fn reg_pair(word: u16) -> (usize, usize) {
    let d = ((word >> 4) & 0x1f) as usize;
    let r = ((word & 0x0f) | ((word >> 5) & 0x10)) as usize;
    (d, r)
}

fn immediate(word: u16) -> (usize, u8) {
    (
        16 + ((word >> 4) & 0x0f) as usize,
        (((word >> 4) & 0xf0) | (word & 0x0f)) as u8,
    )
}

fn set_flags(state: &mut AvrState, mask: u8, values: u8) {
    state.data[SREG_ADDR] = (state.data[SREG_ADDR] & !mask) | (values & mask);
}

const fn flag(condition: bool, mask: u8) -> u8 {
    if condition {
        mask
    } else {
        0
    }
}

fn add_flags(a: u8, b: u8, result: u8) -> u8 {
    let h = ((a & b) | (b & !result) | (!result & a)) & 0x08 != 0;
    let v = ((a & b & !result) | (!a & !b & result)) & 0x80 != 0;
    let n = result & 0x80 != 0;
    let c = ((a & b) | (b & !result) | (!result & a)) & 0x80 != 0;
    flag(h, FLAG_H)
        | flag(v, FLAG_V)
        | flag(n, FLAG_N)
        | flag(n ^ v, FLAG_S)
        | flag(result == 0, FLAG_Z)
        | flag(c, FLAG_C)
}

fn sub_flags(a: u8, b: u8, result: u8) -> u8 {
    let h = ((!a & b) | (b & result) | (result & !a)) & 0x08 != 0;
    let v = ((a & !b & !result) | (!a & b & result)) & 0x80 != 0;
    let n = result & 0x80 != 0;
    let c = ((!a & b) | (b & result) | (result & !a)) & 0x80 != 0;
    flag(h, FLAG_H)
        | flag(v, FLAG_V)
        | flag(n, FLAG_N)
        | flag(n ^ v, FLAG_S)
        | flag(result == 0, FLAG_Z)
        | flag(c, FLAG_C)
}

fn logic_flags(result: u8) -> u8 {
    let n = result & 0x80 != 0;
    flag(n, FLAG_N) | flag(n, FLAG_S) | flag(result == 0, FLAG_Z)
}

fn data_read(state: &AvrState, address: u16) -> Result<u8, AvrError> {
    state
        .data
        .get(address as usize)
        .copied()
        .ok_or(AvrError::DataOutOfRange { address })
}

fn data_write(state: &mut AvrState, address: u16, value: u8) -> Result<(), AvrError> {
    let slot = state
        .data
        .get_mut(address as usize)
        .ok_or(AvrError::DataOutOfRange { address })?;
    *slot = value;
    Ok(())
}

fn displacement(word: u16) -> u16 {
    ((word >> 8) & 0x20) | ((word >> 7) & 0x18) | (word & 0x07)
}

fn pointer_transfer(
    state: &mut AvrState,
    word: u16,
    pc: u16,
    load: bool,
    low: usize,
    update: Update,
    q: u16,
) -> Result<(), AvrError> {
    let (d, _) = reg_pair(word);
    let mut pointer = state.pointer(low);
    if matches!(update, Update::PreDec) {
        pointer = pointer.wrapping_sub(1);
    }
    if !matches!(update, Update::None) && (d == low || d == low + 1) {
        return Err(AvrError::UnsupportedOperandCombination { opcode: word, pc });
    }
    let address = pointer.wrapping_add(q);
    if load {
        let value = data_read(state, address)?;
        state.data[d] = value;
    } else {
        let value = state.data[d];
        data_write(state, address, value)?;
    }
    if matches!(update, Update::PostInc) {
        pointer = pointer.wrapping_add(1);
    }
    if !matches!(update, Update::None) {
        state.set_pointer(low, pointer);
    }
    Ok(())
}

fn relative_target(pc: u16, width: u8, encoded: u16, bits: u8) -> u16 {
    let shift = 16 - bits;
    let signed = ((encoded << shift) as i16) >> shift;
    ((pc as i32 + width as i32 + signed as i32) & 0x3fff) as u16
}

fn push_byte(state: &mut AvrState, value: u8) -> Result<(), AvrError> {
    let sp = state.sp();
    if !(0x0100..=RAM_END).contains(&sp) || sp == 0x0100 {
        return Err(AvrError::StackOutOfRange { sp, width: 1 });
    }
    state.data[sp as usize] = value;
    state.set_sp(sp - 1);
    Ok(())
}

fn pop_byte(state: &mut AvrState) -> Result<u8, AvrError> {
    let sp = state.sp();
    if !(0x0100..RAM_END).contains(&sp) {
        return Err(AvrError::StackOutOfRange { sp, width: 1 });
    }
    let next = sp + 1;
    let value = state.data[next as usize];
    state.set_sp(next);
    Ok(value)
}

fn push_return(state: &mut AvrState, address: u16) -> Result<(), AvrError> {
    let sp = state.sp();
    if !(0x0101..=RAM_END).contains(&sp) {
        return Err(AvrError::StackOutOfRange { sp, width: 2 });
    }
    let [lo, hi] = address.to_le_bytes();
    push_byte(state, lo)?;
    push_byte(state, hi)
}

fn pop_return(state: &mut AvrState) -> Result<u16, AvrError> {
    let sp = state.sp();
    if !(0x0100..=RAM_END - 2).contains(&sp) {
        return Err(AvrError::StackOutOfRange { sp, width: 2 });
    }
    let hi = pop_byte(state)?;
    let lo = pop_byte(state)?;
    Ok(u16::from_le_bytes([lo, hi]))
}

fn transition(mnemonic: &'static str, width: u8) -> Transition {
    Transition {
        mnemonic,
        width,
        branch_taken: false,
        skipped: false,
    }
}

fn execute_one(
    state: &mut AvrState,
    word: u16,
    second: Option<u16>,
) -> Result<Transition, AvrError> {
    let pc = state.pc;
    let width = instruction_width(word);
    state.pc = state.pc.wrapping_add(width as u16) & 0x3fff;

    if word == 0x0000 {
        return Ok(transition("NOP", 1));
    }
    if word == 0x9598 {
        state.halted = true;
        return Ok(transition("BREAK", 1));
    }
    if matches!(word, 0x9588 | 0x95a8 | 0x95e8 | 0x95f8) {
        return Err(AvrError::UnsupportedSystemInstruction { opcode: word, pc });
    }

    let binary = [
        (0xfc00, 0x0c00, "ADD", 0u8),
        (0xfc00, 0x1c00, "ADC", 1),
        (0xfc00, 0x1800, "SUB", 2),
        (0xfc00, 0x0800, "SBC", 3),
        (0xfc00, 0x1400, "CP", 4),
        (0xfc00, 0x0400, "CPC", 5),
        (0xfc00, 0x2000, "AND", 6),
        (0xfc00, 0x2400, "EOR", 7),
        (0xfc00, 0x2800, "OR", 8),
        (0xfc00, 0x2c00, "MOV", 9),
    ];
    for (mask, value, name, kind) in binary {
        if word & mask == value {
            let (d, r) = reg_pair(word);
            let a = state.data[d];
            let carry = u8::from(state.sreg() & FLAG_C != 0);
            match kind {
                0 | 1 => {
                    let b = state.data[r];
                    let result = a
                        .wrapping_add(b)
                        .wrapping_add(if kind == 1 { carry } else { 0 });
                    let mut flags = add_flags(a, b, result);
                    if kind == 1 && result == 0 && state.sreg() & FLAG_Z == 0 {
                        flags &= !FLAG_Z;
                    }
                    state.data[d] = result;
                    set_flags(
                        state,
                        FLAG_H | FLAG_V | FLAG_N | FLAG_S | FLAG_Z | FLAG_C,
                        flags,
                    );
                }
                2..=5 => {
                    let b = state.data[r];
                    let result = a.wrapping_sub(b).wrapping_sub(if matches!(kind, 3 | 5) {
                        carry
                    } else {
                        0
                    });
                    let mut flags = sub_flags(a, b, result);
                    if matches!(kind, 3 | 5) && result == 0 && state.sreg() & FLAG_Z == 0 {
                        flags &= !FLAG_Z;
                    }
                    if !matches!(kind, 4 | 5) {
                        state.data[d] = result;
                    }
                    set_flags(
                        state,
                        FLAG_H | FLAG_V | FLAG_N | FLAG_S | FLAG_Z | FLAG_C,
                        flags,
                    );
                }
                6..=8 => {
                    let result = match kind {
                        6 => a & state.data[r],
                        7 => a ^ state.data[r],
                        _ => a | state.data[r],
                    };
                    state.data[d] = result;
                    set_flags(
                        state,
                        FLAG_V | FLAG_N | FLAG_S | FLAG_Z,
                        logic_flags(result),
                    );
                }
                _ => state.data[d] = state.data[r],
            }
            return Ok(transition(name, 1));
        }
    }

    if word & 0xff00 == 0x0100 {
        let d = ((word >> 4) & 0x0f) as usize * 2;
        let r = (word & 0x0f) as usize * 2;
        state.data[d] = state.data[r];
        state.data[d + 1] = state.data[r + 1];
        return Ok(transition("MOVW", 1));
    }

    let imm_family = [
        (0xf000, 0x3000, "CPI", 0u8),
        (0xf000, 0x4000, "SBCI", 1),
        (0xf000, 0x5000, "SUBI", 2),
        (0xf000, 0x6000, "ORI", 3),
        (0xf000, 0x7000, "ANDI", 4),
        (0xf000, 0xe000, "LDI", 5),
    ];
    for (mask, value, name, kind) in imm_family {
        if word & mask == value {
            let (d, k) = immediate(word);
            let a = state.data[d];
            match kind {
                0..=2 => {
                    let carry = if kind == 1 && state.sreg() & FLAG_C != 0 {
                        1
                    } else {
                        0
                    };
                    let b = k;
                    let result = a.wrapping_sub(b).wrapping_sub(carry);
                    let mut flags = sub_flags(a, b, result);
                    if kind == 1 && result == 0 && state.sreg() & FLAG_Z == 0 {
                        flags &= !FLAG_Z;
                    }
                    if kind != 0 {
                        state.data[d] = result;
                    }
                    set_flags(
                        state,
                        FLAG_H | FLAG_V | FLAG_N | FLAG_S | FLAG_Z | FLAG_C,
                        flags,
                    );
                }
                3 | 4 => {
                    let result = if kind == 3 { a | k } else { a & k };
                    state.data[d] = result;
                    set_flags(
                        state,
                        FLAG_V | FLAG_N | FLAG_S | FLAG_Z,
                        logic_flags(result),
                    );
                }
                _ => state.data[d] = k,
            }
            return Ok(transition(name, 1));
        }
    }

    if word & 0xff00 == 0x9600 || word & 0xff00 == 0x9700 {
        let low = 24 + (((word >> 4) & 3) as usize * 2);
        let k = ((word >> 2) & 0x30) | (word & 0x0f);
        let a = state.pointer(low);
        let subtract = word & 0x0100 != 0;
        let result = if subtract {
            a.wrapping_sub(k)
        } else {
            a.wrapping_add(k)
        };
        state.set_pointer(low, result);
        let r15 = result & 0x8000 != 0;
        let a15 = a & 0x8000 != 0;
        let v = if subtract { a15 && !r15 } else { !a15 && r15 };
        let c = if subtract { !a15 && r15 } else { a15 && !r15 };
        let flags = flag(r15, FLAG_N)
            | flag(v, FLAG_V)
            | flag(r15 ^ v, FLAG_S)
            | flag(result == 0, FLAG_Z)
            | flag(c, FLAG_C);
        set_flags(state, FLAG_N | FLAG_V | FLAG_S | FLAG_Z | FLAG_C, flags);
        return Ok(transition(if subtract { "SBIW" } else { "ADIW" }, 1));
    }

    let unary = [
        (0x9400, "COM", 0u8),
        (0x9401, "NEG", 1),
        (0x9402, "SWAP", 2),
        (0x9403, "INC", 3),
        (0x9405, "ASR", 4),
        (0x9406, "LSR", 5),
        (0x9407, "ROR", 6),
        (0x940a, "DEC", 7),
    ];
    for (value, name, kind) in unary {
        if word & 0xfe0f == value {
            let d = ((word >> 4) & 0x1f) as usize;
            let a = state.data[d];
            let (result, mask, flags) = match kind {
                0 => {
                    let r = !a;
                    (
                        r,
                        FLAG_V | FLAG_N | FLAG_S | FLAG_Z | FLAG_C,
                        logic_flags(r) | FLAG_C,
                    )
                }
                1 => {
                    let r = 0u8.wrapping_sub(a);
                    let v = r == 0x80;
                    let n = r & 0x80 != 0;
                    (
                        r,
                        FLAG_H | FLAG_V | FLAG_N | FLAG_S | FLAG_Z | FLAG_C,
                        flag((r | a) & 0x08 != 0, FLAG_H)
                            | flag(v, FLAG_V)
                            | flag(n, FLAG_N)
                            | flag(n ^ v, FLAG_S)
                            | flag(r == 0, FLAG_Z)
                            | flag(r != 0, FLAG_C),
                    )
                }
                2 => (a.rotate_left(4), 0, 0),
                3 | 7 => {
                    let r = if kind == 3 {
                        a.wrapping_add(1)
                    } else {
                        a.wrapping_sub(1)
                    };
                    let v = if kind == 3 { r == 0x80 } else { r == 0x7f };
                    let n = r & 0x80 != 0;
                    (
                        r,
                        FLAG_V | FLAG_N | FLAG_S | FLAG_Z,
                        flag(v, FLAG_V)
                            | flag(n, FLAG_N)
                            | flag(n ^ v, FLAG_S)
                            | flag(r == 0, FLAG_Z),
                    )
                }
                4..=6 => {
                    let old_c = state.sreg() & FLAG_C != 0;
                    let c = a & 1 != 0;
                    let r = match kind {
                        4 => (a >> 1) | (a & 0x80),
                        5 => a >> 1,
                        _ => (a >> 1) | (u8::from(old_c) << 7),
                    };
                    let n = r & 0x80 != 0;
                    let v = n ^ c;
                    (
                        r,
                        FLAG_V | FLAG_N | FLAG_S | FLAG_Z | FLAG_C,
                        flag(v, FLAG_V)
                            | flag(n, FLAG_N)
                            | flag(n ^ v, FLAG_S)
                            | flag(r == 0, FLAG_Z)
                            | flag(c, FLAG_C),
                    )
                }
                _ => unreachable!(),
            };
            state.data[d] = result;
            set_flags(state, mask, flags);
            return Ok(transition(name, 1));
        }
    }

    if word & 0xfc00 == 0x9c00
        || word & 0xff00 == 0x0200
        || word & 0xff88 == 0x0300
        || word & 0xff88 == 0x0308
        || word & 0xff88 == 0x0380
        || word & 0xff88 == 0x0388
    {
        let (a, b, fractional, name) = if word & 0xfc00 == 0x9c00 {
            let (d, r) = reg_pair(word);
            (state.data[d] as i16, state.data[r] as i16, false, "MUL")
        } else if word & 0xff00 == 0x0200 {
            let d = 16 + ((word >> 4) & 0xf) as usize;
            let r = 16 + (word & 0xf) as usize;
            (
                state.data[d] as i8 as i16,
                state.data[r] as i8 as i16,
                false,
                "MULS",
            )
        } else {
            let d = 16 + ((word >> 4) & 7) as usize;
            let r = 16 + (word & 7) as usize;
            let variant = word & 0xff88;
            let signed_a = matches!(variant, 0x0300 | 0x0380 | 0x0388);
            let signed_b = variant == 0x0380;
            let aa = if signed_a {
                state.data[d] as i8 as i16
            } else {
                state.data[d] as i16
            };
            let bb = if signed_b {
                state.data[r] as i8 as i16
            } else {
                state.data[r] as i16
            };
            let frac = variant != 0x0300;
            let name = match variant {
                0x0300 => "MULSU",
                0x0308 => "FMUL",
                0x0380 => "FMULS",
                0x0388 => "FMULSU",
                _ => unreachable!(),
            };
            (aa, bb, frac, name)
        };
        let mut product = (a as i32 * b as i32) as u16;
        if fractional {
            product = product.wrapping_shl(1);
        }
        state.data[0] = product as u8;
        state.data[1] = (product >> 8) as u8;
        set_flags(
            state,
            FLAG_Z | FLAG_C,
            flag(product == 0, FLAG_Z) | flag(product & 0x8000 != 0, FLAG_C),
        );
        return Ok(transition(name, 1));
    }

    if word & 0xfe0f == 0x9000 || word & 0xfe0f == 0x9200 {
        let d = ((word >> 4) & 0x1f) as usize;
        let address = second.expect("width decoder supplied second word");
        if word & 0x0200 == 0 {
            state.data[d] = data_read(state, address)?;
            return Ok(transition("LDS", 2));
        }
        let value = state.data[d];
        data_write(state, address, value)?;
        return Ok(transition("STS", 2));
    }

    for (value, load, low, update, name) in [
        (0x900c, true, 26, Update::None, "LD"),
        (0x900d, true, 26, Update::PostInc, "LD"),
        (0x900e, true, 26, Update::PreDec, "LD"),
        (0x920c, false, 26, Update::None, "ST"),
        (0x920d, false, 26, Update::PostInc, "ST"),
        (0x920e, false, 26, Update::PreDec, "ST"),
        (0x9009, true, 28, Update::PostInc, "LD"),
        (0x900a, true, 28, Update::PreDec, "LD"),
        (0x9209, false, 28, Update::PostInc, "ST"),
        (0x920a, false, 28, Update::PreDec, "ST"),
        (0x9001, true, 30, Update::PostInc, "LD"),
        (0x9002, true, 30, Update::PreDec, "LD"),
        (0x9201, false, 30, Update::PostInc, "ST"),
        (0x9202, false, 30, Update::PreDec, "ST"),
    ] {
        if word & 0xfe0f == value {
            pointer_transfer(state, word, pc, load, low, update, 0)?;
            return Ok(transition(name, 1));
        }
    }
    if word & 0xd208 == 0x8000
        || word & 0xd208 == 0x8008
        || word & 0xd208 == 0x8200
        || word & 0xd208 == 0x8208
    {
        let load = word & 0x0200 == 0;
        let low = if word & 0x0008 == 0 { 30 } else { 28 };
        pointer_transfer(state, word, pc, load, low, Update::None, displacement(word))?;
        return Ok(transition(if load { "LD" } else { "ST" }, 1));
    }
    if word & 0xfe0f == 0x920f {
        let d = ((word >> 4) & 31) as usize;
        push_byte(state, state.data[d])?;
        return Ok(transition("PUSH", 1));
    }
    if word & 0xfe0f == 0x900f {
        let d = ((word >> 4) & 31) as usize;
        state.data[d] = pop_byte(state)?;
        return Ok(transition("POP", 1));
    }

    if word & 0xf800 == 0xb000 || word & 0xf800 == 0xb800 {
        let d = ((word >> 4) & 31) as usize;
        let a = ((word >> 5) & 0x30) | (word & 0xf) | 0x20;
        if word & 0x0800 == 0 {
            state.data[d] = data_read(state, a)?;
            return Ok(transition("IN", 1));
        }
        let value = state.data[d];
        data_write(state, a, value)?;
        return Ok(transition("OUT", 1));
    }

    if word == 0x95c8 {
        let z = state.pointer(30);
        state.data[0] = *state
            .program
            .get(z as usize)
            .ok_or(AvrError::ProgramReadOutOfRange { address: z })?;
        return Ok(transition("LPM", 1));
    }
    if word & 0xfe0f == 0x9004 || word & 0xfe0f == 0x9005 {
        let d = ((word >> 4) & 31) as usize;
        let z = state.pointer(30);
        if word & 0xf == 5 && (d == 30 || d == 31) {
            return Err(AvrError::UnsupportedOperandCombination { opcode: word, pc });
        }
        let value = *state
            .program
            .get(z as usize)
            .ok_or(AvrError::ProgramReadOutOfRange { address: z })?;
        state.data[d] = value;
        if word & 0xf == 5 {
            state.set_pointer(30, z.wrapping_add(1));
        }
        return Ok(transition("LPM", 1));
    }

    if word & 0xff8f == 0x9408 || word & 0xff8f == 0x9488 {
        let bit = ((word >> 4) & 7) as u8;
        let set = word & 0x0080 == 0;
        let mask = 1 << bit;
        set_flags(state, mask, if set { mask } else { 0 });
        return Ok(transition(if set { "BSET" } else { "BCLR" }, 1));
    }
    if word & 0xfe08 == 0xf800 || word & 0xfe08 == 0xfa00 {
        let d = ((word >> 4) & 31) as usize;
        let bit = (word & 7) as u8;
        let mask = 1 << bit;
        if word & 0x0200 == 0 {
            let value = if state.sreg() & FLAG_T != 0 { mask } else { 0 };
            state.data[d] = (state.data[d] & !mask) | value;
            return Ok(transition("BLD", 1));
        }
        set_flags(
            state,
            FLAG_T,
            if state.data[d] & mask != 0 { FLAG_T } else { 0 },
        );
        return Ok(transition("BST", 1));
    }
    if matches!(word & 0xff00, 0x9800 | 0x9900 | 0x9a00 | 0x9b00) {
        let a = ((word >> 3) & 0x1f) + 0x20;
        let bit = (word & 7) as u8;
        let mask = 1 << bit;
        match word & 0x0300 {
            0 => {
                let v = data_read(state, a)? & !mask;
                data_write(state, a, v)?;
                return Ok(transition("CBI", 1));
            }
            0x0200 => {
                let v = data_read(state, a)? | mask;
                data_write(state, a, v)?;
                return Ok(transition("SBI", 1));
            }
            0x0100 | 0x0300 => {
                let set = data_read(state, a)? & mask != 0;
                let skip = if word & 0x0200 == 0 { !set } else { set };
                return finish_skip(
                    state,
                    skip,
                    if word & 0x0200 == 0 { "SBIC" } else { "SBIS" },
                );
            }
            _ => unreachable!(),
        }
    }
    if word & 0xfe08 == 0xfc00 || word & 0xfe08 == 0xfe00 {
        let d = ((word >> 4) & 31) as usize;
        let set = state.data[d] & (1 << (word & 7)) != 0;
        let skip = if word & 0x0200 == 0 { !set } else { set };
        return finish_skip(
            state,
            skip,
            if word & 0x0200 == 0 { "SBRC" } else { "SBRS" },
        );
    }
    if word & 0xfc00 == 0x1000 {
        let (d, r) = reg_pair(word);
        return finish_skip(state, state.data[d] == state.data[r], "CPSE");
    }

    if word & 0xfc00 == 0xf000 || word & 0xfc00 == 0xf400 {
        let bit = (word & 7) as u8;
        let set = state.sreg() & (1 << bit) != 0;
        let take = if word & 0x0400 == 0 { set } else { !set };
        if take {
            let target = relative_target(pc, 1, (word >> 3) & 0x7f, 7);
            validate_target(state, target)?;
            state.pc = target;
        }
        let mut t = transition(if word & 0x0400 == 0 { "BRBS" } else { "BRBC" }, 1);
        t.branch_taken = take;
        return Ok(t);
    }
    if word & 0xf000 == 0xc000 || word & 0xf000 == 0xd000 {
        let call = word & 0x1000 != 0;
        let target = relative_target(pc, 1, word & 0x0fff, 12);
        validate_target(state, target)?;
        if call {
            push_return(state, state.pc)?;
        }
        state.pc = target;
        let mut t = transition(if call { "RCALL" } else { "RJMP" }, 1);
        t.branch_taken = true;
        return Ok(t);
    }
    if word & 0xfe0e == 0x940c || word & 0xfe0e == 0x940e {
        let call = word & 2 != 0;
        let upper = ((word & 0x01f0) >> 3) | (word & 1);
        let full_target =
            (upper as u32) << 16 | second.expect("width decoder supplied second word") as u32;
        if full_target > 0x3fff {
            return Err(AvrError::UnsupportedProfileInstruction { opcode: word, pc });
        }
        let target = full_target as u16;
        validate_target(state, target)?;
        if call {
            push_return(state, state.pc)?;
        }
        state.pc = target;
        let mut t = transition(if call { "CALL" } else { "JMP" }, 2);
        t.branch_taken = true;
        return Ok(t);
    }
    if word == 0x9409 || word == 0x9509 {
        let call = word == 0x9509;
        let target = state.pointer(30) & 0x3fff;
        validate_target(state, target)?;
        if call {
            push_return(state, state.pc)?;
        }
        state.pc = target;
        let mut t = transition(if call { "ICALL" } else { "IJMP" }, 1);
        t.branch_taken = true;
        return Ok(t);
    }
    if word == 0x9508 || word == 0x9518 {
        let target = pop_return(state)? & 0x3fff;
        validate_target(state, target)?;
        state.pc = target;
        if word == 0x9518 {
            set_flags(state, FLAG_I, FLAG_I);
        }
        let mut t = transition(if word == 0x9518 { "RETI" } else { "RET" }, 1);
        t.branch_taken = true;
        return Ok(t);
    }

    // Enhanced/extended-family instructions which have recognizable AVR
    // encodings but are not present in the pinned ATmega328P `avr5` profile.
    if matches!(word, 0x9419 | 0x9519 | 0x95d8)
        || word & 0xfe0f == 0x9006
        || word & 0xfe0f == 0x9007
        || matches!(word & 0xfe0f, 0x9204..=0x9207)
        || word & 0xff0f == 0x940b
    {
        return Err(AvrError::UnsupportedProfileInstruction { opcode: word, pc });
    }

    Err(AvrError::UnknownOpcode { opcode: word, pc })
}

fn finish_skip(
    state: &mut AvrState,
    skip: bool,
    name: &'static str,
) -> Result<Transition, AvrError> {
    if skip {
        let next = fetch_word(state, state.pc)?;
        let width = instruction_width(next);
        if width == 2 {
            fetch_second(state, state.pc)?;
        }
        state.pc = state.pc.wrapping_add(width as u16) & 0x3fff;
    }
    let mut t = transition(name, 1);
    t.skipped = skip;
    t.branch_taken = skip;
    Ok(t)
}

#[cfg(test)]
mod flag_formula_tests {
    use super::*;

    #[test]
    fn add_and_adc_flags_match_independent_integer_oracle_exhaustively() {
        for a in 0u16..=255 {
            for b in 0u16..=255 {
                for carry in 0u16..=1 {
                    let wide = a + b + carry;
                    let result = wide as u8;
                    let signed = a as i8 as i16 + b as i8 as i16 + carry as i16;
                    let expected = flag((a & 0xf) + (b & 0xf) + carry > 0xf, FLAG_H)
                        | flag(!(-128..=127).contains(&signed), FLAG_V)
                        | flag(result & 0x80 != 0, FLAG_N)
                        | flag(
                            (result & 0x80 != 0) ^ !(-128..=127).contains(&signed),
                            FLAG_S,
                        )
                        | flag(result == 0, FLAG_Z)
                        | flag(wide > 0xff, FLAG_C);
                    assert_eq!(add_flags(a as u8, b as u8, result), expected);
                }
            }
        }
    }

    #[test]
    fn subtract_compare_and_borrow_flags_match_oracle_exhaustively() {
        for a in 0u16..=255 {
            for b in 0u16..=255 {
                for carry in 0u16..=1 {
                    let result = (a as u8).wrapping_sub(b as u8).wrapping_sub(carry as u8);
                    let signed = a as i8 as i16 - b as i8 as i16 - carry as i16;
                    let overflow = !(-128..=127).contains(&signed);
                    let expected = flag((a & 0xf) < (b & 0xf) + carry, FLAG_H)
                        | flag(overflow, FLAG_V)
                        | flag(result & 0x80 != 0, FLAG_N)
                        | flag((result & 0x80 != 0) ^ overflow, FLAG_S)
                        | flag(result == 0, FLAG_Z)
                        | flag(a < b + carry, FLAG_C);
                    assert_eq!(sub_flags(a as u8, b as u8, result), expected);
                }
            }
        }
    }
}
