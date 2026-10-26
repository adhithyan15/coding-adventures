//! Checked, transactional HP PA-RISC 1.1 problem-state integer simulator.
//!
//! ```
//! use pa_risc11_simulator::PaRisc11Simulator;
//!
//! // LDO 7(0),r1 ; BREAK 0,0.
//! let program = [0x34, 0x01, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x00];
//! let mut cpu = PaRisc11Simulator::new();
//! let result = cpu.execute(&program, 0x100, 2)?;
//! assert_eq!(result.final_state.gpr[1], 7);
//! # Ok::<(), pa_risc11_simulator::PaRisc11Error>(())
//! ```

#[cfg(test)]
use std::cell::Cell;
use std::error::Error;
use std::fmt;

/// Size of the flat teaching memory.
pub const MEMORY_SIZE: usize = 65_536;
/// Maximum number of complete-state traces retained by one run.
pub const MAX_RETAINED_STEPS: usize = 64;

/// Complete restorable architectural and lifecycle state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaRisc11State {
    /// General registers. Element zero must remain zero.
    pub gpr: [u32; 32],
    /// PSW carry/borrow outputs, ordered from the most-significant nibble to
    /// the least-significant nibble as C/B{0} through C/B{7}.
    pub carry_borrow: [bool; 8],
    /// Front of the instruction address offset queue.
    pub pc: u32,
    /// Back of the instruction address offset queue.
    pub next_pc: u32,
    /// Whether the instruction at `pc` is nullified.
    pub nullify_next: bool,
    /// Whether the instruction at `pc` occupies a control-transfer delay slot.
    pub in_delay_slot: bool,
    /// Complete flat big-endian teaching memory.
    pub memory: Vec<u8>,
    /// Deterministic repository halt state.
    pub halted: bool,
    /// Installed program origin.
    pub loaded_origin: u32,
    /// Installed program length in bytes.
    pub loaded_len: usize,
}

impl Default for PaRisc11State {
    fn default() -> Self {
        Self {
            gpr: [0; 32],
            carry_borrow: [false; 8],
            pc: 0,
            next_pc: 4,
            nullify_next: false,
            in_delay_slot: false,
            memory: vec![0; MEMORY_SIZE],
            halted: false,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }
}

/// One exact committed transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaRisc11StepTrace {
    pub pc_before: u32,
    pub next_pc_before: u32,
    pub pc_after: u32,
    pub next_pc_after: u32,
    pub raw_bytes: [u8; 4],
    pub raw_word: u32,
    pub mnemonic: &'static str,
    pub nullified: bool,
    pub state_after: PaRisc11State,
}

/// Successful bounded execution result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaRisc11ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<PaRisc11StepTrace>,
    pub final_state: PaRisc11State,
}

/// Typed checked lifecycle and execution failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaRisc11Error {
    EmptyProgram,
    MisalignedProgram { origin: u32, len: usize },
    ProgramOutOfRange { origin: u32, len: usize },
    InvalidState(&'static str),
    MisalignedFetch { pc: u32 },
    FetchOutsideProgram { pc: u32 },
    MisalignedData { address: u32, width: usize },
    DataOutOfRange { address: u32, width: usize },
    UnknownInstruction { pc: u32, word: u32 },
    UnsupportedInstruction { pc: u32, word: u32 },
    UnsupportedDelayedControl { pc: u32 },
    Halted,
    TraceLimitExceeded { requested: usize, maximum: usize },
    AllocationFailed,
    StepLimitExceeded { max_steps: usize },
}

impl fmt::Display for PaRisc11Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => f.write_str("program is empty"),
            Self::MisalignedProgram { origin, len } => write!(
                f,
                "program origin {origin:#x} or length {len} is not word aligned"
            ),
            Self::ProgramOutOfRange { origin, len } => {
                write!(
                    f,
                    "program at {origin:#x} with length {len} is out of range"
                )
            }
            Self::InvalidState(reason) => write!(f, "invalid PA-RISC state: {reason}"),
            Self::MisalignedFetch { pc } => write!(f, "misaligned instruction fetch at {pc:#x}"),
            Self::FetchOutsideProgram { pc } => {
                write!(
                    f,
                    "instruction fetch at {pc:#x} is outside the installed program"
                )
            }
            Self::MisalignedData { address, width } => {
                write!(f, "misaligned {width}-byte data access at {address:#x}")
            }
            Self::DataOutOfRange { address, width } => {
                write!(
                    f,
                    "{width}-byte data access at {address:#x} is out of range"
                )
            }
            Self::UnknownInstruction { pc, word } => {
                write!(f, "unknown instruction {word:#010x} at {pc:#x}")
            }
            Self::UnsupportedInstruction { pc, word } => {
                write!(f, "unsupported instruction {word:#010x} at {pc:#x}")
            }
            Self::UnsupportedDelayedControl { pc } => {
                write!(f, "control transfer in a live delay slot at {pc:#x}")
            }
            Self::Halted => f.write_str("processor is halted"),
            Self::TraceLimitExceeded { requested, maximum } => write!(
                f,
                "requested {requested} traces exceeds retention limit {maximum}"
            ),
            Self::AllocationFailed => f.write_str("state or trace allocation failed"),
            Self::StepLimitExceeded { max_steps } => {
                write!(f, "execution did not halt within {max_steps} steps")
            }
        }
    }
}

impl Error for PaRisc11Error {}

/// Checked PA-RISC 1.1 simulator.
#[derive(Debug)]
pub struct PaRisc11Simulator {
    state: PaRisc11State,
    #[cfg(test)]
    snapshot_failure_after: Cell<Option<usize>>,
}

impl Default for PaRisc11Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl PaRisc11Simulator {
    /// Construct a reset processor with empty memory.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: PaRisc11State::default(),
            #[cfg(test)]
            snapshot_failure_after: Cell::new(None),
        }
    }

    /// Borrow the current complete state without allocating.
    #[must_use]
    pub fn state(&self) -> &PaRisc11State {
        &self.state
    }

    /// Copy the complete state with a typed allocation failure.
    pub fn snapshot(&self) -> Result<PaRisc11State, PaRisc11Error> {
        self.checked_clone(&self.state)
    }

    /// Validate and atomically restore complete state.
    pub fn restore(&mut self, state: &PaRisc11State) -> Result<(), PaRisc11Error> {
        validate_state(state)?;
        self.state = self.checked_clone(state)?;
        Ok(())
    }

    /// Replace the complete machine state and install a raw big-endian image.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), PaRisc11Error> {
        validate_program(program, origin)?;
        let mut memory = Vec::new();
        memory
            .try_reserve_exact(MEMORY_SIZE)
            .map_err(|_| PaRisc11Error::AllocationFailed)?;
        memory.resize(MEMORY_SIZE, 0);
        let start = origin as usize;
        memory[start..start + program.len()].copy_from_slice(program);
        self.state = PaRisc11State {
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
        Ok(())
    }

    /// Execute one atomic transition.
    pub fn step(&mut self) -> Result<PaRisc11StepTrace, PaRisc11Error> {
        if self.state.halted {
            return Err(PaRisc11Error::Halted);
        }
        let before = self.checked_clone(&self.state)?;
        let (bytes, word) = fetch(&before)?;
        let mut candidate = self.checked_clone(&before)?;
        let pc_before = before.pc;
        let next_pc_before = before.next_pc;

        let (mnemonic, nullified) = if before.nullify_next {
            advance_queue(&mut candidate);
            candidate.nullify_next = false;
            candidate.in_delay_slot = false;
            ("NULLIFIED", true)
        } else {
            let mnemonic = execute_word(&mut candidate, word)?;
            (mnemonic, false)
        };
        candidate.gpr[0] = 0;
        let state_after = self.checked_clone(&candidate)?;
        let trace = PaRisc11StepTrace {
            pc_before,
            next_pc_before,
            pc_after: candidate.pc,
            next_pc_after: candidate.next_pc,
            raw_bytes: bytes,
            raw_word: word,
            mnemonic,
            nullified,
            state_after,
        };
        self.state = candidate;
        Ok(trace)
    }

    /// Run the installed image transactionally until halt.
    pub fn run(&mut self, max_steps: usize) -> Result<PaRisc11ExecutionResult, PaRisc11Error> {
        if max_steps > MAX_RETAINED_STEPS {
            return Err(PaRisc11Error::TraceLimitExceeded {
                requested: max_steps,
                maximum: MAX_RETAINED_STEPS,
            });
        }
        let before = self.checked_clone(&self.state)?;
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
                        return Ok(PaRisc11ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            traces,
                            final_state: match self.checked_clone(&self.state) {
                                Ok(state) => state,
                                Err(error) => {
                                    self.state = before;
                                    return Err(error);
                                }
                            },
                        });
                    }
                }
                Err(error) => {
                    self.state = before;
                    return Err(error);
                }
            }
        }
        self.state = before;
        Err(PaRisc11Error::StepLimitExceeded { max_steps })
    }

    /// Atomically install and run an image.
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
        let before = self.checked_clone(&self.state)?;
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = before;
                Err(error)
            }
        }
    }

    fn checked_clone(&self, state: &PaRisc11State) -> Result<PaRisc11State, PaRisc11Error> {
        #[cfg(test)]
        if let Some(remaining) = self.snapshot_failure_after.get() {
            if remaining == 0 {
                self.snapshot_failure_after.set(None);
                return Err(PaRisc11Error::AllocationFailed);
            }
            self.snapshot_failure_after.set(Some(remaining - 1));
        }
        checked_clone_state(state)
    }

    #[cfg(test)]
    fn fail_snapshot_after(&self, successful_snapshots: usize) {
        self.snapshot_failure_after.set(Some(successful_snapshots));
    }
}

fn checked_clone_state(state: &PaRisc11State) -> Result<PaRisc11State, PaRisc11Error> {
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
    let bytes: [u8; 4] = state.memory[start..start + 4]
        .try_into()
        .expect("checked four-byte fetch");
    Ok((bytes, u32::from_be_bytes(bytes)))
}

fn advance_queue(state: &mut PaRisc11State) {
    state.pc = state.next_pc;
    state.next_pc = state.next_pc.wrapping_add(4);
    state.in_delay_slot = false;
}

fn read_gpr(state: &PaRisc11State, register: usize) -> u32 {
    if register == 0 {
        0
    } else {
        state.gpr[register]
    }
}

fn write_gpr(state: &mut PaRisc11State, register: usize, value: u32) {
    if register != 0 {
        state.gpr[register] = value;
    }
}

fn low_sign(field: u32, width: u32) -> i32 {
    let sign = field & 1;
    let magnitude = field >> 1;
    let assembled = magnitude | (sign << (width - 1));
    sign_extend(assembled, width)
}

fn sign_extend(value: u32, width: u32) -> i32 {
    ((value << (32 - width)) as i32) >> (32 - width)
}

fn assemble_21(field: u32) -> u32 {
    ((field & 1) << 20)
        | (((field >> 1) & 0x7ff) << 9)
        | (((field >> 14) & 0x3) << 7)
        | (((field >> 16) & 0x1f) << 2)
        | ((field >> 12) & 0x3)
}

fn branch_disp12(word: u32) -> i32 {
    let w1 = (word >> 2) & 0x7ff;
    let w = word & 1;
    let assembled = (w << 11) | ((w1 & 1) << 10) | (w1 >> 1);
    sign_extend(assembled, 12) << 2
}

fn branch_disp17(word: u32) -> i32 {
    let w1 = (word >> 16) & 0x1f;
    let w2 = (word >> 2) & 0x7ff;
    let w = word & 1;
    let assembled = (w << 16) | (w1 << 11) | ((w2 & 1) << 10) | (w2 >> 1);
    sign_extend(assembled, 17) << 2
}

fn data_address(base: u32, displacement: i32) -> u32 {
    base.wrapping_add(displacement as u32)
}

fn data_range(address: u32, width: usize) -> Result<std::ops::Range<usize>, PaRisc11Error> {
    if width > 1 && !(address as usize).is_multiple_of(width) {
        return Err(PaRisc11Error::MisalignedData { address, width });
    }
    let start = address as usize;
    let end = start
        .checked_add(width)
        .ok_or(PaRisc11Error::DataOutOfRange { address, width })?;
    if end > MEMORY_SIZE {
        return Err(PaRisc11Error::DataOutOfRange { address, width });
    }
    Ok(start..end)
}

fn load_data(state: &PaRisc11State, address: u32, width: usize) -> Result<u32, PaRisc11Error> {
    let range = data_range(address, width)?;
    Ok(state.memory[range]
        .iter()
        .fold(0_u32, |value, byte| (value << 8) | u32::from(*byte)))
}

fn store_data(
    state: &mut PaRisc11State,
    address: u32,
    width: usize,
    value: u32,
) -> Result<(), PaRisc11Error> {
    let range = data_range(address, width)?;
    for (offset, byte) in state.memory[range].iter_mut().enumerate() {
        let shift = (width - offset - 1) * 8;
        *byte = (value >> shift) as u8;
    }
    Ok(())
}

fn in_live_delay_slot(state: &PaRisc11State) -> bool {
    state.in_delay_slot
}

fn install_branch(state: &mut PaRisc11State, taken: bool, displacement: i32, n: bool) {
    let old_pc = state.pc;
    let old_next = state.next_pc;
    state.pc = old_next;
    state.next_pc = if taken {
        old_pc.wrapping_add(8).wrapping_add(displacement as u32)
    } else {
        old_next.wrapping_add(4)
    };
    state.nullify_next = n && if displacement >= 0 { taken } else { !taken };
    state.in_delay_slot = true;
}

fn compare_condition(condition: u32, left: u32, right: u32) -> bool {
    let result = left.wrapping_sub(right);
    match condition {
        0 => false,
        1 => left == right,
        2 => (left as i32) < (right as i32),
        3 => (left as i32) <= (right as i32),
        4 => left < right,
        5 => left <= right,
        6 => ((left ^ right) & (left ^ result) & 0x8000_0000) != 0,
        _ => result & 1 != 0,
    }
}

fn add_condition(condition: u32, left: u32, right: u32, result: u32) -> bool {
    let unsigned_carry = u64::from(left) + u64::from(right) > u64::from(u32::MAX);
    let overflow = ((left ^ result) & (right ^ result) & 0x8000_0000) != 0;
    match condition {
        0 => false,
        1 => result == 0,
        2 => i64::from(left as i32) < -i64::from(right as i32),
        3 => i64::from(left as i32) <= -i64::from(right as i32),
        4 => !unsigned_carry,
        5 => result == 0 || !unsigned_carry,
        6 => overflow,
        _ => result & 1 != 0,
    }
}

fn execute_word(state: &mut PaRisc11State, word: u32) -> Result<&'static str, PaRisc11Error> {
    let pc = state.pc;
    let primary = word >> 26;
    match primary {
        0x00 => {
            if word != 0 {
                return Err(PaRisc11Error::UnsupportedInstruction { pc, word });
            }
            advance_queue(state);
            state.halted = true;
            Ok("BREAK")
        }
        0x02 => execute_alu(state, word),
        0x08 => {
            let target = ((word >> 21) & 31) as usize;
            let value = assemble_21(word & 0x1f_ffff) << 11;
            write_gpr(state, target, value);
            advance_queue(state);
            Ok("LDIL")
        }
        0x0a => {
            let source = ((word >> 21) & 31) as usize;
            let value = read_gpr(state, source).wrapping_add(assemble_21(word & 0x1f_ffff) << 11);
            write_gpr(state, 1, value);
            advance_queue(state);
            Ok("ADDIL")
        }
        0x0d => {
            if (word >> 14) & 3 != 0 {
                return Err(PaRisc11Error::UnsupportedInstruction { pc, word });
            }
            let base = ((word >> 21) & 31) as usize;
            let target = ((word >> 16) & 31) as usize;
            let value = data_address(read_gpr(state, base), low_sign(word & 0x3fff, 14));
            write_gpr(state, target, value);
            advance_queue(state);
            Ok("LDO")
        }
        0x10..=0x12 | 0x18..=0x1a => execute_memory(state, word),
        0x20..=0x23 | 0x28..=0x2b => execute_conditional_branch(state, word),
        0x34 => execute_shd(state, word),
        0x3a => execute_branch(state, word),
        _ => Err(PaRisc11Error::UnknownInstruction { pc, word }),
    }
}

fn execute_alu(state: &mut PaRisc11State, word: u32) -> Result<&'static str, PaRisc11Error> {
    let pc = state.pc;
    if ((word >> 13) & 7) != 0 || ((word >> 12) & 1) != 0 || ((word >> 5) & 1) != 0 {
        return Err(PaRisc11Error::UnsupportedInstruction { pc, word });
    }
    let right = read_gpr(state, ((word >> 21) & 31) as usize);
    let left = read_gpr(state, ((word >> 16) & 31) as usize);
    let target = (word & 31) as usize;
    let extension = (word >> 6) & 0x3f;
    let (mnemonic, value, carry_borrow) = match extension {
        0x00 => ("ANDCM", left & !right, None),
        0x08 => ("AND", left & right, None),
        0x09 => ("OR", left | right, None),
        0x0a => ("XOR", left ^ right, None),
        0x10 => (
            "SUB",
            left.wrapping_sub(right),
            Some(nibble_carries(left, !right, true)),
        ),
        0x18 => (
            "ADD",
            left.wrapping_add(right),
            Some(nibble_carries(left, right, false)),
        ),
        _ => return Err(PaRisc11Error::UnsupportedInstruction { pc, word }),
    };
    write_gpr(state, target, value);
    if let Some(carry_borrow) = carry_borrow {
        state.carry_borrow = carry_borrow;
    }
    advance_queue(state);
    Ok(mnemonic)
}

fn nibble_carries(left: u32, right: u32, carry_in: bool) -> [bool; 8] {
    let mut carries = [false; 8];
    let mut carry = u16::from(carry_in);
    for nibble in 0..8 {
        let shift = nibble * 4;
        let sum = u16::from(((left >> shift) & 0xf) as u8)
            + u16::from(((right >> shift) & 0xf) as u8)
            + carry;
        carry = sum >> 4;
        carries[7 - nibble] = carry != 0;
    }
    carries
}

fn execute_memory(state: &mut PaRisc11State, word: u32) -> Result<&'static str, PaRisc11Error> {
    let pc = state.pc;
    if (word >> 14) & 3 != 0 {
        return Err(PaRisc11Error::UnsupportedInstruction { pc, word });
    }
    let primary = word >> 26;
    let base = ((word >> 21) & 31) as usize;
    let register = ((word >> 16) & 31) as usize;
    let address = data_address(read_gpr(state, base), low_sign(word & 0x3fff, 14));
    let (width, load, mnemonic) = match primary {
        0x10 => (1, true, "LDB"),
        0x11 => (2, true, "LDH"),
        0x12 => (4, true, "LDW"),
        0x18 => (1, false, "STB"),
        0x19 => (2, false, "STH"),
        0x1a => (4, false, "STW"),
        _ => unreachable!("caller classified memory opcode"),
    };
    if load {
        let value = load_data(state, address, width)?;
        write_gpr(state, register, value);
    } else {
        store_data(state, address, width, read_gpr(state, register))?;
    }
    advance_queue(state);
    Ok(mnemonic)
}

fn execute_shd(state: &mut PaRisc11State, word: u32) -> Result<&'static str, PaRisc11Error> {
    let pc = state.pc;
    if ((word >> 13) & 7) != 0 || ((word >> 10) & 7) != 2 {
        return Err(PaRisc11Error::UnsupportedInstruction { pc, word });
    }
    let right = read_gpr(state, ((word >> 21) & 31) as usize);
    let left = read_gpr(state, ((word >> 16) & 31) as usize);
    let count = 31 - ((word >> 5) & 31);
    let result = if count == 0 {
        right
    } else {
        left.wrapping_shl(32 - count) | right.wrapping_shr(count)
    };
    write_gpr(state, (word & 31) as usize, result);
    advance_queue(state);
    Ok("SHD")
}

fn execute_conditional_branch(
    state: &mut PaRisc11State,
    word: u32,
) -> Result<&'static str, PaRisc11Error> {
    let pc = state.pc;
    if in_live_delay_slot(state) {
        return Err(PaRisc11Error::UnsupportedDelayedControl { pc });
    }
    let primary = word >> 26;
    let immediate = primary & 1 != 0;
    let branch_false = primary & 2 != 0;
    let add = primary >= 0x28;
    let condition = (word >> 13) & 7;
    let register = ((word >> 21) & 31) as usize;
    let left = if immediate {
        low_sign((word >> 16) & 31, 5) as u32
    } else {
        read_gpr(state, ((word >> 16) & 31) as usize)
    };
    let right = read_gpr(state, register);
    let predicate = if add {
        let result = left.wrapping_add(right);
        write_gpr(state, register, result);
        add_condition(condition, left, right, result)
    } else {
        compare_condition(condition, left, right)
    };
    let taken = if branch_false { !predicate } else { predicate };
    let displacement = branch_disp12(word);
    install_branch(state, taken, displacement, (word >> 1) & 1 != 0);
    Ok(match (add, immediate, branch_false) {
        (false, false, false) => "COMBT",
        (false, false, true) => "COMBF",
        (false, true, false) => "COMIBT",
        (false, true, true) => "COMIBF",
        (true, false, false) => "ADDBT",
        (true, false, true) => "ADDBF",
        (true, true, false) => "ADDIBT",
        (true, true, true) => "ADDIBF",
    })
}

fn execute_branch(state: &mut PaRisc11State, word: u32) -> Result<&'static str, PaRisc11Error> {
    let pc = state.pc;
    if in_live_delay_slot(state) {
        return Err(PaRisc11Error::UnsupportedDelayedControl { pc });
    }
    let extension = (word >> 13) & 7;
    let target_register = ((word >> 21) & 31) as usize;
    let base_or_index = ((word >> 16) & 31) as usize;
    let n = (word >> 1) & 1 != 0;
    let old_next = state.next_pc;
    match extension {
        0 => {
            let displacement = branch_disp17(word);
            let target = pc.wrapping_add(8).wrapping_add(displacement as u32);
            write_gpr(state, target_register, old_next.wrapping_add(4));
            state.pc = old_next;
            state.next_pc = target;
            state.nullify_next = n;
            state.in_delay_slot = true;
            Ok("BL")
        }
        2 | 6 => {
            if word & 1 != 0 || (word >> 2) & 0x7ff != 0 {
                return Err(PaRisc11Error::UnsupportedInstruction { pc, word });
            }
            let index = read_gpr(state, base_or_index).wrapping_shl(3);
            let target = if extension == 2 {
                write_gpr(state, target_register, old_next.wrapping_add(4));
                pc.wrapping_add(8).wrapping_add(index)
            } else {
                // PA-RISC encodes the privilege level in the low two IAOQ
                // bits separately from address bits 0..29. This flat
                // problem-state model has no privilege levels, so BV keeps
                // the address portion and normalizes the PL bits to zero.
                read_gpr(state, target_register).wrapping_add(index) & !0b11
            };
            state.pc = old_next;
            state.next_pc = target;
            state.nullify_next = n;
            state.in_delay_slot = true;
            Ok(if extension == 2 { "BLR" } else { "BV" })
        }
        _ => Err(PaRisc11Error::UnsupportedInstruction { pc, word }),
    }
}

#[cfg(test)]
mod allocation_regressions {
    use super::{PaRisc11Error, PaRisc11Simulator};

    const LDO_R1_7: [u8; 4] = [0x34, 0x01, 0x00, 0x0e];
    const BREAK: [u8; 4] = [0; 4];

    #[test]
    fn snapshot_and_restore_allocation_failures_are_typed_and_atomic() {
        let mut cpu = PaRisc11Simulator::new();
        let original = cpu.state().clone();
        cpu.fail_snapshot_after(0);
        assert_eq!(cpu.snapshot(), Err(PaRisc11Error::AllocationFailed));
        assert_eq!(cpu.state(), &original);

        let mut replacement = original.clone();
        replacement.gpr[1] = 7;
        cpu.fail_snapshot_after(0);
        assert_eq!(
            cpu.restore(&replacement),
            Err(PaRisc11Error::AllocationFailed)
        );
        assert_eq!(cpu.state(), &original);
    }

    #[test]
    fn every_step_snapshot_failure_precedes_commit() {
        for successful_snapshots in 0..=2 {
            let mut cpu = PaRisc11Simulator::new();
            cpu.load(&LDO_R1_7, 0).unwrap();
            let before = cpu.state().clone();
            cpu.fail_snapshot_after(successful_snapshots);
            assert_eq!(cpu.step(), Err(PaRisc11Error::AllocationFailed));
            assert_eq!(cpu.state(), &before);
        }
    }

    #[test]
    fn final_run_and_execute_snapshot_failures_restore_whole_transactions() {
        let mut run_cpu = PaRisc11Simulator::new();
        run_cpu.load(&BREAK, 0).unwrap();
        let run_before = run_cpu.state().clone();
        run_cpu.fail_snapshot_after(4);
        assert_eq!(run_cpu.run(1), Err(PaRisc11Error::AllocationFailed));
        assert_eq!(run_cpu.state(), &run_before);

        let mut execute_cpu = PaRisc11Simulator::new();
        let execute_before = execute_cpu.state().clone();
        execute_cpu.fail_snapshot_after(5);
        assert_eq!(
            execute_cpu.execute(&BREAK, 0, 1),
            Err(PaRisc11Error::AllocationFailed)
        );
        assert_eq!(execute_cpu.state(), &execute_before);
    }
}
