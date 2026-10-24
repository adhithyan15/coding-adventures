//! Checked, deterministic OpenRISC 1000 ORBIS32 functional simulator.
//!
//! OpenRISC is pleasantly regular: every instruction is one big-endian word,
//! the primary opcode occupies the upper six bits, and most operands keep the
//! same positions.  The simulator keeps that structure visible instead of
//! hiding it behind a generated decoder.  Each match arm below corresponds to
//! one row (or one closely related family) in the architecture manual.

use core::fmt;

/// Size of this teaching machine's flat physical address space.
pub const MEMORY_SIZE: usize = 65_536;
/// Maximum complete-state traces retained by one execution.
pub const MAX_RETAINED_STEPS: usize = 1_024;

pub const SR_SM: u32 = 1 << 0;
const SR_TEE: u32 = 1 << 1;
const SR_IEE: u32 = 1 << 2;
const SR_DME: u32 = 1 << 5;
const SR_IME: u32 = 1 << 6;
pub const SR_F: u32 = 1 << 9;
pub const SR_CY: u32 = 1 << 10;
pub const SR_OV: u32 = 1 << 11;
pub const SR_DSX: u32 = 1 << 13;
pub const SR_FO: u32 = 1 << 15;

const SPR_VR: u32 = 0;
const SPR_UPR: u32 = 1;
const SPR_CPUCFGR: u32 = 2;
const SPR_AVR: u32 = 10;
const SPR_NPC: u32 = 16;
const SPR_SR: u32 = 17;
const SPR_PPC: u32 = 18;
const SPR_EPCR0: u32 = 32;
const SPR_EEAR0: u32 = 48;
const SPR_ESR0: u32 = 64;
const SPR_COREID: u32 = 128;
const SPR_NUMCORES: u32 = 129;
const SPR_GPR_BASE: u32 = 1_024;
const SPR_MACLO: u32 = (5 << 11) | 1;
const SPR_MACHI: u32 = (5 << 11) | 2;

/// Report whether a primary opcode belongs to the implemented ORBIS32 boundary.
#[must_use]
pub fn primary_opcode_is_implemented(opcode: u8) -> bool {
    matches!(
        opcode,
        0x00 | 0x01
            | 0x02
            | 0x03
            | 0x04
            | 0x05
            | 0x06
            | 0x08
            | 0x09
            | 0x11
            | 0x12
            | 0x13
            | 0x1b
            | 0x21..=0x31
            | 0x33
            | 0x35..=0x39
    )
}

/// Complete owned architectural state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenRisc1000State {
    pub gpr: [u32; 32],
    pub pc: u32,
    pub ppc: u32,
    pub sr: u32,
    pub epcr0: u32,
    pub eear0: u32,
    pub esr0: u32,
    pub maclo: u32,
    pub machi: u32,
    pub reservation: Option<u32>,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u32,
    pub loaded_len: usize,
}

impl Default for OpenRisc1000State {
    fn default() -> Self {
        Self {
            gpr: [0; 32],
            pc: 0,
            ppc: 0,
            sr: SR_FO | SR_SM,
            epcr0: 0,
            eear0: 0,
            esr0: 0,
            maclo: 0,
            machi: 0,
            reservation: None,
            memory: vec![0; MEMORY_SIZE],
            halted: false,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }
}

/// One successful instruction transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: [u8; 4],
    pub raw_word: u32,
    pub mnemonic: &'static str,
    pub state_after: OpenRisc1000State,
}

/// Result of a successful bounded execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: OpenRisc1000State,
}

/// Checked lifecycle, transport, decode, and execution failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenRisc1000Error {
    EmptyProgram,
    MisalignedProgram { origin: u32, len: usize },
    ProgramOutOfRange { origin: u32, len: usize },
    MisalignedFetch { pc: u32 },
    FetchOutsideProgram { pc: u32 },
    MisalignedData { address: u32, width: usize },
    DataOutOfRange { address: u32, width: usize },
    UnknownInstruction { word: u32, pc: u32 },
    UnsupportedInstruction { word: u32, pc: u32 },
    PrivilegedInstruction { word: u32, pc: u32 },
    InvalidSprRead { address: u32 },
    InvalidSprWrite { address: u32 },
    InvalidRegister { register: usize },
    InvalidState { reason: &'static str },
    DivideByZero,
    SignedDivisionOverflow,
    Halted,
    RetentionLimitExceeded { requested: usize, limit: usize },
    AllocationFailed,
    StepLimitExceeded { limit: usize },
}

impl fmt::Display for OpenRisc1000Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => write!(f, "program image is empty"),
            Self::MisalignedProgram { origin, len } => write!(
                f,
                "program origin {origin:#010x} and length {len} must be word aligned"
            ),
            Self::ProgramOutOfRange { origin, len } => {
                write!(f, "program of {len} bytes does not fit at {origin:#010x}")
            }
            Self::MisalignedFetch { pc } => write!(f, "fetch at {pc:#010x} is not word aligned"),
            Self::FetchOutsideProgram { pc } => {
                write!(f, "fetch at {pc:#010x} is outside the installed program")
            }
            Self::MisalignedData { address, width } => write!(
                f,
                "{width}-byte data access at {address:#010x} is misaligned"
            ),
            Self::DataOutOfRange { address, width } => write!(
                f,
                "{width}-byte data access at {address:#010x} is out of range"
            ),
            Self::UnknownInstruction { word, pc } => {
                write!(f, "unknown instruction {word:#010x} at {pc:#010x}")
            }
            Self::UnsupportedInstruction { word, pc } => {
                write!(f, "unsupported instruction {word:#010x} at {pc:#010x}")
            }
            Self::PrivilegedInstruction { word, pc } => {
                write!(f, "privileged instruction {word:#010x} at {pc:#010x}")
            }
            Self::InvalidSprRead { address } => {
                write!(f, "special-purpose register {address:#06x} is not readable")
            }
            Self::InvalidSprWrite { address } => {
                write!(f, "special-purpose register {address:#06x} is not writable")
            }
            Self::InvalidRegister { register } => write!(f, "register r{register} does not exist"),
            Self::InvalidState { reason } => write!(f, "invalid OpenRISC state: {reason}"),
            Self::DivideByZero => write!(f, "integer division by zero"),
            Self::SignedDivisionOverflow => write!(f, "signed integer division overflow"),
            Self::Halted => write!(f, "processor is halted"),
            Self::RetentionLimitExceeded { requested, limit } => {
                write!(f, "requested {requested} traces exceeds limit {limit}")
            }
            Self::AllocationFailed => write!(f, "trace allocation failed"),
            Self::StepLimitExceeded { limit } => write!(f, "step limit {limit} exceeded"),
        }
    }
}

impl std::error::Error for OpenRisc1000Error {}

/// Functional simulator for the 32-bit ORBIS32 integer architecture.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct OpenRisc1000Simulator {
    state: OpenRisc1000State,
}

impl OpenRisc1000Simulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.state = OpenRisc1000State::default();
    }

    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), OpenRisc1000Error> {
        validate_image(program, origin)?;
        let start = origin as usize;
        let end = start + program.len();
        let mut next = OpenRisc1000State {
            pc: origin,
            ppc: origin,
            loaded_origin: origin,
            loaded_len: program.len(),
            ..OpenRisc1000State::default()
        };
        next.memory[start..end].copy_from_slice(program);
        self.state = next;
        Ok(())
    }

    pub fn restore(&mut self, mut state: OpenRisc1000State) -> Result<(), OpenRisc1000Error> {
        validate_state(&state)?;
        state.sr |= SR_FO;
        self.state = state;
        Ok(())
    }

    #[must_use]
    pub fn get_state(&self) -> OpenRisc1000State {
        self.state.clone()
    }

    pub fn read_register(&self, register: usize) -> Result<u32, OpenRisc1000Error> {
        self.state
            .gpr
            .get(register)
            .copied()
            .ok_or(OpenRisc1000Error::InvalidRegister { register })
    }

    pub fn write_register(&mut self, register: usize, value: u32) -> Result<(), OpenRisc1000Error> {
        let target = self
            .state
            .gpr
            .get_mut(register)
            .ok_or(OpenRisc1000Error::InvalidRegister { register })?;
        *target = value;
        Ok(())
    }

    pub fn read_memory(&self, address: u32, len: usize) -> Result<Vec<u8>, OpenRisc1000Error> {
        let range = checked_range(address, len)?;
        Ok(self.state.memory[range].to_vec())
    }

    pub fn write_memory(&mut self, address: u32, bytes: &[u8]) -> Result<(), OpenRisc1000Error> {
        let range = checked_range(address, bytes.len())?;
        self.state.memory[range].copy_from_slice(bytes);
        invalidate_overlapping_reservation(&mut self.state, address, bytes.len());
        Ok(())
    }

    pub fn step(&mut self) -> Result<StepTrace, OpenRisc1000Error> {
        if self.state.halted {
            return Err(OpenRisc1000Error::Halted);
        }
        let mut next = self.state.clone();
        let trace = execute_one(&mut next)?;
        self.state = next;
        Ok(trace)
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, OpenRisc1000Error> {
        preflight_retention(max_steps)?;
        let original = self.state.clone();
        let mut traces = Vec::new();
        traces
            .try_reserve(max_steps)
            .map_err(|_| OpenRisc1000Error::AllocationFailed)?;
        for _ in 0..max_steps {
            match self.step() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.state = original;
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
        self.state = original;
        Err(OpenRisc1000Error::StepLimitExceeded { limit: max_steps })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, OpenRisc1000Error> {
        preflight_retention(max_steps)?;
        validate_image(program, origin)?;
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

fn validate_image(program: &[u8], origin: u32) -> Result<(), OpenRisc1000Error> {
    if program.is_empty() {
        return Err(OpenRisc1000Error::EmptyProgram);
    }
    if origin & 3 != 0 || program.len() & 3 != 0 {
        return Err(OpenRisc1000Error::MisalignedProgram {
            origin,
            len: program.len(),
        });
    }
    let end = (origin as usize).checked_add(program.len());
    if end.is_none_or(|value| value > MEMORY_SIZE) {
        return Err(OpenRisc1000Error::ProgramOutOfRange {
            origin,
            len: program.len(),
        });
    }
    Ok(())
}

fn validate_state(state: &OpenRisc1000State) -> Result<(), OpenRisc1000Error> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(OpenRisc1000Error::InvalidState {
            reason: "memory must contain exactly 65536 bytes",
        });
    }
    if state.loaded_len == 0 || state.loaded_len & 3 != 0 || state.loaded_origin & 3 != 0 {
        return Err(OpenRisc1000Error::InvalidState {
            reason: "installed program must be nonempty and word aligned",
        });
    }
    let end = (state.loaded_origin as usize).checked_add(state.loaded_len);
    if end.is_none_or(|value| value > MEMORY_SIZE) {
        return Err(OpenRisc1000Error::InvalidState {
            reason: "installed program range is invalid",
        });
    }
    if state.pc & 3 != 0 {
        return Err(OpenRisc1000Error::InvalidState {
            reason: "program counter must be word aligned",
        });
    }
    if state.reservation.is_some_and(|address| {
        address & 3 != 0
            || (address as usize)
                .checked_add(4)
                .is_none_or(|end| end > MEMORY_SIZE)
    }) {
        return Err(OpenRisc1000Error::InvalidState {
            reason: "atomic reservation must name an aligned in-range word",
        });
    }
    Ok(())
}

fn preflight_retention(max_steps: usize) -> Result<(), OpenRisc1000Error> {
    if max_steps > MAX_RETAINED_STEPS {
        return Err(OpenRisc1000Error::RetentionLimitExceeded {
            requested: max_steps,
            limit: MAX_RETAINED_STEPS,
        });
    }
    Ok(())
}

fn checked_range(address: u32, width: usize) -> Result<std::ops::Range<usize>, OpenRisc1000Error> {
    let start = address as usize;
    let end = start.checked_add(width);
    if end.is_none_or(|value| value > MEMORY_SIZE) {
        return Err(OpenRisc1000Error::DataOutOfRange { address, width });
    }
    Ok(start..end.expect("range end was checked"))
}

fn check_alignment(address: u32, width: usize) -> Result<(), OpenRisc1000Error> {
    if width > 1 && !(address as usize).is_multiple_of(width) {
        return Err(OpenRisc1000Error::MisalignedData { address, width });
    }
    Ok(())
}

fn invalidate_overlapping_reservation(state: &mut OpenRisc1000State, address: u32, width: usize) {
    let Some(reserved) = state.reservation else {
        return;
    };
    let store_start = u64::from(address);
    let store_end = store_start + width as u64;
    let reservation_start = u64::from(reserved);
    let reservation_end = reservation_start + 4;
    if store_start < reservation_end && reservation_start < store_end {
        state.reservation = None;
    }
}

fn require_supervisor(
    state: &OpenRisc1000State,
    word: u32,
    pc: u32,
) -> Result<(), OpenRisc1000Error> {
    if state.sr & SR_SM == 0 {
        return Err(OpenRisc1000Error::PrivilegedInstruction { word, pc });
    }
    Ok(())
}

fn fetch_word(state: &OpenRisc1000State) -> Result<([u8; 4], u32), OpenRisc1000Error> {
    if state.pc & 3 != 0 {
        return Err(OpenRisc1000Error::MisalignedFetch { pc: state.pc });
    }
    let start = state.pc as usize;
    let installed_start = state.loaded_origin as usize;
    let installed_end = installed_start + state.loaded_len;
    let Some(end) = start.checked_add(4) else {
        return Err(OpenRisc1000Error::FetchOutsideProgram { pc: state.pc });
    };
    if start < installed_start || end > installed_end {
        return Err(OpenRisc1000Error::FetchOutsideProgram { pc: state.pc });
    }
    let raw: [u8; 4] = state.memory[start..end]
        .try_into()
        .expect("four-byte fetch slice");
    Ok((raw, u32::from_be_bytes(raw)))
}

fn execute_one(state: &mut OpenRisc1000State) -> Result<StepTrace, OpenRisc1000Error> {
    let pc_before = state.pc;
    let (raw, word) = fetch_word(state)?;
    state.ppc = pc_before;
    state.pc = pc_before.wrapping_add(4);
    let mnemonic = execute_word(state, word, pc_before)?;
    state.sr |= SR_FO;
    Ok(StepTrace {
        pc_before,
        pc_after: state.pc,
        raw,
        raw_word: word,
        mnemonic,
        state_after: state.clone(),
    })
}

fn execute_word(
    state: &mut OpenRisc1000State,
    word: u32,
    pc: u32,
) -> Result<&'static str, OpenRisc1000Error> {
    let opcode = (word >> 26) as u8;
    let rd = ((word >> 21) & 0x1f) as usize;
    let ra = ((word >> 16) & 0x1f) as usize;
    let rb = ((word >> 11) & 0x1f) as usize;
    let immediate = word as u16;
    match opcode {
        0x00 => {
            state.pc = branch_target(pc, word & 0x03ff_ffff);
            Ok("l.j")
        }
        0x01 => {
            state.gpr[9] = pc.wrapping_add(4);
            state.pc = branch_target(pc, word & 0x03ff_ffff);
            Ok("l.jal")
        }
        0x02 => {
            let offset = sign_extend(word & 0x7ffff, 19).wrapping_shl(13);
            state.gpr[rd] = (pc & !0x1fff).wrapping_add(offset);
            Ok("l.adrp")
        }
        0x03 => {
            if state.sr & SR_F == 0 {
                state.pc = branch_target(pc, word & 0x03ff_ffff);
            }
            Ok("l.bnf")
        }
        0x04 => {
            if state.sr & SR_F != 0 {
                state.pc = branch_target(pc, word & 0x03ff_ffff);
            }
            Ok("l.bf")
        }
        0x05 if word >> 24 == 0x15 => {
            if immediate == 1 {
                state.halted = true;
            }
            Ok("l.nop")
        }
        0x06 if word & 0x001f_ffff == 0x0001_0000 => {
            state.gpr[rd] = state.maclo;
            state.maclo = 0;
            state.machi = 0;
            Ok("l.macrc")
        }
        0x06 if word & 0x0001_0000 == 0 => {
            state.gpr[rd] = u32::from(immediate) << 16;
            Ok("l.movhi")
        }
        0x08 => execute_system(state, word, pc),
        0x09 => {
            state.pc = state.epcr0;
            state.sr = state.esr0 | SR_FO;
            Ok("l.rfe")
        }
        0x11 => {
            state.pc = state.gpr[rb];
            Ok("l.jr")
        }
        0x12 => {
            if rb == 9 {
                return Err(OpenRisc1000Error::UnknownInstruction { word, pc });
            }
            state.gpr[9] = pc.wrapping_add(4);
            state.pc = state.gpr[rb];
            Ok("l.jalr")
        }
        0x13 => {
            let product = i64::from(state.gpr[ra] as i32) * i64::from(immediate as i16);
            accumulate_signed(state, product, false);
            Ok("l.maci")
        }
        0x1b => {
            let address = effective_address(state.gpr[ra], immediate);
            state.gpr[rd] = load_u32(state, address)?;
            state.reservation = Some(address);
            Ok("l.lwa")
        }
        0x21 => {
            let address = effective_address(state.gpr[ra], immediate);
            state.gpr[rd] = load_u32(state, address)?;
            Ok("l.lwz")
        }
        0x22 => {
            let address = effective_address(state.gpr[ra], immediate);
            state.gpr[rd] = load_u32(state, address)?;
            Ok("l.lws")
        }
        0x23 => {
            let address = effective_address(state.gpr[ra], immediate);
            state.gpr[rd] = u32::from(load_u8(state, address)?);
            Ok("l.lbz")
        }
        0x24 => {
            let address = effective_address(state.gpr[ra], immediate);
            state.gpr[rd] = (load_u8(state, address)? as i8 as i32) as u32;
            Ok("l.lbs")
        }
        0x25 => {
            let address = effective_address(state.gpr[ra], immediate);
            state.gpr[rd] = u32::from(load_u16(state, address)?);
            Ok("l.lhz")
        }
        0x26 => {
            let address = effective_address(state.gpr[ra], immediate);
            state.gpr[rd] = (load_u16(state, address)? as i16 as i32) as u32;
            Ok("l.lhs")
        }
        0x27 => {
            let rhs = sign_extend16(immediate);
            let result = state.gpr[ra].wrapping_add(rhs);
            update_add_flags(state, state.gpr[ra], rhs, 0);
            state.gpr[rd] = result;
            Ok("l.addi")
        }
        0x28 => {
            let rhs = sign_extend16(immediate);
            let carry = u32::from(state.sr & SR_CY != 0);
            let result = state.gpr[ra].wrapping_add(rhs).wrapping_add(carry);
            update_add_flags(state, state.gpr[ra], rhs, carry);
            state.gpr[rd] = result;
            Ok("l.addic")
        }
        0x29 => {
            state.gpr[rd] = state.gpr[ra] & u32::from(immediate);
            Ok("l.andi")
        }
        0x2a => {
            state.gpr[rd] = state.gpr[ra] | u32::from(immediate);
            Ok("l.ori")
        }
        0x2b => {
            state.gpr[rd] = state.gpr[ra] ^ sign_extend16(immediate);
            Ok("l.xori")
        }
        0x2c => {
            let product = i64::from(state.gpr[ra] as i32) * i64::from(immediate as i16);
            state.gpr[rd] = product as u32;
            set_flag(
                &mut state.sr,
                SR_OV,
                product < i64::from(i32::MIN) || product > i64::from(i32::MAX),
            );
            Ok("l.muli")
        }
        0x2d => {
            require_supervisor(state, word, pc)?;
            let address = state.gpr[ra] | u32::from(immediate);
            state.gpr[rd] = read_spr(state, address)?;
            Ok("l.mfspr")
        }
        0x2e => execute_shift_immediate(state, word, rd, ra, pc),
        0x2f => execute_compare_immediate(state, word, ra, immediate, pc),
        0x30 => {
            require_supervisor(state, word, pc)?;
            let spr = (((word >> 21) & 0x1f) << 11) | (word & 0x7ff);
            let address = state.gpr[ra] | spr;
            write_spr(state, address, state.gpr[rb])?;
            Ok("l.mtspr")
        }
        0x31 => execute_mac(state, word, ra, rb, pc),
        0x33 => {
            let address = effective_store_address(state.gpr[ra], word);
            check_alignment(address, 4)?;
            checked_range(address, 4)?;
            let succeeds = state.reservation == Some(address);
            state.reservation = None;
            set_flag(&mut state.sr, SR_F, succeeds);
            if succeeds {
                store_u32(state, address, state.gpr[rb])?;
            }
            Ok("l.swa")
        }
        0x35 => {
            let address = effective_store_address(state.gpr[ra], word);
            store_u32(state, address, state.gpr[rb])?;
            Ok("l.sw")
        }
        0x36 => {
            let address = effective_store_address(state.gpr[ra], word);
            store_u8(state, address, state.gpr[rb] as u8)?;
            Ok("l.sb")
        }
        0x37 => {
            let address = effective_store_address(state.gpr[ra], word);
            store_u16(state, address, state.gpr[rb] as u16)?;
            Ok("l.sh")
        }
        0x38 => execute_register_alu(state, word, rd, ra, rb, pc),
        0x39 => execute_compare_register(state, word, ra, rb, pc),
        0x0a | 0x1a | 0x20 | 0x32 => Err(OpenRisc1000Error::UnsupportedInstruction { word, pc }),
        _ if matches!(opcode, 0x1c..=0x1f | 0x3c..=0x3f) => {
            Err(OpenRisc1000Error::UnsupportedInstruction { word, pc })
        }
        _ => Err(OpenRisc1000Error::UnknownInstruction { word, pc }),
    }
}

fn execute_system(
    state: &mut OpenRisc1000State,
    word: u32,
    pc: u32,
) -> Result<&'static str, OpenRisc1000Error> {
    match word >> 16 {
        0x2000 => {
            enter_exception(state, pc.wrapping_add(4), 0x0c00);
            Ok("l.sys")
        }
        0x2100 => {
            enter_exception(state, pc, 0x0e00);
            Ok("l.trap")
        }
        _ if word == 0x2200_0000 => Ok("l.msync"),
        _ if word == 0x2280_0000 => Ok("l.psync"),
        _ if word == 0x2300_0000 => Ok("l.csync"),
        _ => Err(OpenRisc1000Error::UnknownInstruction { word, pc }),
    }
}

fn enter_exception(state: &mut OpenRisc1000State, return_pc: u32, vector: u32) {
    state.epcr0 = return_pc;
    state.esr0 = state.sr;
    state.eear0 = 0;
    state.sr = (state.sr | SR_SM | SR_FO) & !(SR_DSX | SR_DME | SR_IME | SR_IEE | SR_TEE);
    state.pc = vector;
    state.reservation = None;
}

fn execute_shift_immediate(
    state: &mut OpenRisc1000State,
    word: u32,
    rd: usize,
    ra: usize,
    pc: u32,
) -> Result<&'static str, OpenRisc1000Error> {
    if word & 0x0000_0f00 != 0 {
        return Err(OpenRisc1000Error::UnknownInstruction { word, pc });
    }
    let amount = word & 0x1f;
    match (word >> 6) & 3 {
        0 => {
            state.gpr[rd] = state.gpr[ra].wrapping_shl(amount);
            Ok("l.slli")
        }
        1 => {
            state.gpr[rd] = state.gpr[ra].wrapping_shr(amount);
            Ok("l.srli")
        }
        2 => {
            state.gpr[rd] = ((state.gpr[ra] as i32) >> amount) as u32;
            Ok("l.srai")
        }
        3 => {
            state.gpr[rd] = state.gpr[ra].rotate_right(amount);
            Ok("l.rori")
        }
        _ => unreachable!(),
    }
}

fn execute_compare_immediate(
    state: &mut OpenRisc1000State,
    word: u32,
    ra: usize,
    immediate: u16,
    pc: u32,
) -> Result<&'static str, OpenRisc1000Error> {
    let condition = (word >> 21) & 0x1f;
    let lhs = state.gpr[ra];
    let signed = sign_extend16(immediate);
    let (result, mnemonic) = match condition {
        0x00 => (lhs == signed, "l.sfeqi"),
        0x01 => (lhs != signed, "l.sfnei"),
        0x02 => (lhs > signed, "l.sfgtui"),
        0x03 => (lhs >= signed, "l.sfgeui"),
        0x04 => (lhs < signed, "l.sfltui"),
        0x05 => (lhs <= signed, "l.sfleui"),
        0x0a => ((lhs as i32) > (signed as i32), "l.sfgtsi"),
        0x0b => ((lhs as i32) >= (signed as i32), "l.sfgesi"),
        0x0c => ((lhs as i32) < (signed as i32), "l.sfltsi"),
        0x0d => ((lhs as i32) <= (signed as i32), "l.sflesi"),
        _ => return Err(OpenRisc1000Error::UnknownInstruction { word, pc }),
    };
    set_flag(&mut state.sr, SR_F, result);
    Ok(mnemonic)
}

fn execute_compare_register(
    state: &mut OpenRisc1000State,
    word: u32,
    ra: usize,
    rb: usize,
    pc: u32,
) -> Result<&'static str, OpenRisc1000Error> {
    let condition = (word >> 21) & 0x1f;
    let lhs = state.gpr[ra];
    let rhs = state.gpr[rb];
    let (result, mnemonic) = match condition {
        0x00 => (lhs == rhs, "l.sfeq"),
        0x01 => (lhs != rhs, "l.sfne"),
        0x02 => (lhs > rhs, "l.sfgtu"),
        0x03 => (lhs >= rhs, "l.sfgeu"),
        0x04 => (lhs < rhs, "l.sfltu"),
        0x05 => (lhs <= rhs, "l.sfleu"),
        0x0a => ((lhs as i32) > (rhs as i32), "l.sfgts"),
        0x0b => ((lhs as i32) >= (rhs as i32), "l.sfges"),
        0x0c => ((lhs as i32) < (rhs as i32), "l.sflts"),
        0x0d => ((lhs as i32) <= (rhs as i32), "l.sfles"),
        _ => return Err(OpenRisc1000Error::UnknownInstruction { word, pc }),
    };
    set_flag(&mut state.sr, SR_F, result);
    Ok(mnemonic)
}

fn execute_mac(
    state: &mut OpenRisc1000State,
    word: u32,
    ra: usize,
    rb: usize,
    pc: u32,
) -> Result<&'static str, OpenRisc1000Error> {
    let function = word & 0xf;
    match function {
        1 => {
            let product = i64::from(state.gpr[ra] as i32) * i64::from(state.gpr[rb] as i32);
            accumulate_signed(state, product, false);
            Ok("l.mac")
        }
        2 => {
            let product = i64::from(state.gpr[ra] as i32) * i64::from(state.gpr[rb] as i32);
            accumulate_signed(state, product, true);
            Ok("l.msb")
        }
        3 => {
            let product = u64::from(state.gpr[ra]) * u64::from(state.gpr[rb]);
            accumulate_unsigned(state, product, false);
            Ok("l.macu")
        }
        4 => {
            let product = u64::from(state.gpr[ra]) * u64::from(state.gpr[rb]);
            accumulate_unsigned(state, product, true);
            Ok("l.msbu")
        }
        _ => Err(OpenRisc1000Error::UnknownInstruction { word, pc }),
    }
}

fn execute_register_alu(
    state: &mut OpenRisc1000State,
    word: u32,
    rd: usize,
    ra: usize,
    rb: usize,
    pc: u32,
) -> Result<&'static str, OpenRisc1000Error> {
    let function = word & 0x7ff;
    let lhs = state.gpr[ra];
    let rhs = state.gpr[rb];
    match function {
        0x000 => {
            update_add_flags(state, lhs, rhs, 0);
            state.gpr[rd] = lhs.wrapping_add(rhs);
            Ok("l.add")
        }
        0x001 => {
            let carry = u32::from(state.sr & SR_CY != 0);
            update_add_flags(state, lhs, rhs, carry);
            state.gpr[rd] = lhs.wrapping_add(rhs).wrapping_add(carry);
            Ok("l.addc")
        }
        0x002 => {
            state.gpr[rd] = lhs.wrapping_sub(rhs);
            set_flag(&mut state.sr, SR_CY, lhs < rhs);
            let overflow = ((lhs ^ rhs) & (lhs ^ state.gpr[rd]) & 0x8000_0000) != 0;
            set_flag(&mut state.sr, SR_OV, overflow);
            Ok("l.sub")
        }
        0x003 => {
            state.gpr[rd] = lhs & rhs;
            Ok("l.and")
        }
        0x004 => {
            state.gpr[rd] = lhs | rhs;
            Ok("l.or")
        }
        0x005 => {
            state.gpr[rd] = lhs ^ rhs;
            Ok("l.xor")
        }
        0x008 => {
            state.gpr[rd] = lhs.wrapping_shl(rhs & 0x1f);
            Ok("l.sll")
        }
        0x048 => {
            state.gpr[rd] = lhs.wrapping_shr(rhs & 0x1f);
            Ok("l.srl")
        }
        0x088 => {
            state.gpr[rd] = ((lhs as i32) >> (rhs & 0x1f)) as u32;
            Ok("l.sra")
        }
        0x0c8 => {
            state.gpr[rd] = lhs.rotate_right(rhs & 0x1f);
            Ok("l.ror")
        }
        0x00e => {
            state.gpr[rd] = if state.sr & SR_F != 0 { lhs } else { rhs };
            Ok("l.cmov")
        }
        0x00f => {
            state.gpr[rd] = if lhs == 0 {
                0
            } else {
                lhs.trailing_zeros() + 1
            };
            Ok("l.ff1")
        }
        0x04f => {
            state.gpr[rd] = if lhs == 0 {
                0
            } else {
                32 - lhs.leading_zeros()
            };
            Ok("l.fl1")
        }
        0x00c => {
            state.gpr[rd] = (lhs as u16 as i16 as i32) as u32;
            Ok("l.exths")
        }
        0x00d => {
            state.gpr[rd] = lhs;
            Ok("l.extws")
        }
        0x04c => {
            state.gpr[rd] = (lhs as u8 as i8 as i32) as u32;
            Ok("l.extbs")
        }
        0x04d => {
            state.gpr[rd] = lhs;
            Ok("l.extwz")
        }
        0x08c => {
            state.gpr[rd] = u32::from(lhs as u16);
            Ok("l.exthz")
        }
        0x0cc => {
            state.gpr[rd] = u32::from(lhs as u8);
            Ok("l.extbz")
        }
        0x306 => {
            let product = i64::from(lhs as i32) * i64::from(rhs as i32);
            state.gpr[rd] = product as u32;
            set_flag(
                &mut state.sr,
                SR_OV,
                product < i64::from(i32::MIN) || product > i64::from(i32::MAX),
            );
            Ok("l.mul")
        }
        0x307 => {
            set_mac(
                state,
                (i64::from(lhs as i32) * i64::from(rhs as i32)) as u64,
            );
            Ok("l.muld")
        }
        0x309 => {
            if rhs == 0 {
                return Err(OpenRisc1000Error::DivideByZero);
            }
            if lhs == i32::MIN as u32 && rhs == u32::MAX {
                return Err(OpenRisc1000Error::SignedDivisionOverflow);
            }
            state.gpr[rd] = ((lhs as i32) / (rhs as i32)) as u32;
            set_flag(&mut state.sr, SR_OV, false);
            Ok("l.div")
        }
        0x30a => {
            if rhs == 0 {
                return Err(OpenRisc1000Error::DivideByZero);
            }
            state.gpr[rd] = lhs / rhs;
            set_flag(&mut state.sr, SR_CY, false);
            Ok("l.divu")
        }
        0x30b => {
            let product = u64::from(lhs) * u64::from(rhs);
            state.gpr[rd] = product as u32;
            set_flag(&mut state.sr, SR_CY, product > u64::from(u32::MAX));
            Ok("l.mulu")
        }
        0x30c => {
            set_mac(state, u64::from(lhs) * u64::from(rhs));
            Ok("l.muldu")
        }
        _ => Err(OpenRisc1000Error::UnknownInstruction { word, pc }),
    }
}

fn read_spr(state: &OpenRisc1000State, address: u32) -> Result<u32, OpenRisc1000Error> {
    match address {
        SPR_VR => Ok(0x0104_0000),
        SPR_UPR => Ok((1 << 0) | (1 << 6)),
        // OB32S plus no-delay-slot support.
        SPR_CPUCFGR => Ok((1 << 0) | (1 << 10)),
        3..=9 => Ok(0),
        SPR_AVR => Ok(0x0104_0000),
        SPR_NPC => Ok(state.pc),
        SPR_SR => Ok(state.sr | SR_FO),
        SPR_PPC => Ok(state.ppc),
        SPR_EPCR0 => Ok(state.epcr0),
        SPR_EEAR0 => Ok(state.eear0),
        SPR_ESR0 => Ok(state.esr0),
        SPR_COREID => Ok(0),
        SPR_NUMCORES => Ok(1),
        SPR_MACLO => Ok(state.maclo),
        SPR_MACHI => Ok(state.machi),
        SPR_GPR_BASE..=1055 => Ok(state.gpr[(address - SPR_GPR_BASE) as usize]),
        _ if address <= u32::from(u16::MAX) => Ok(0),
        _ => Err(OpenRisc1000Error::InvalidSprRead { address }),
    }
}

fn write_spr(
    state: &mut OpenRisc1000State,
    address: u32,
    value: u32,
) -> Result<(), OpenRisc1000Error> {
    match address {
        SPR_NPC => state.pc = value,
        SPR_SR => state.sr = value | SR_FO,
        SPR_EPCR0 => state.epcr0 = value,
        SPR_EEAR0 => state.eear0 = value,
        SPR_ESR0 => state.esr0 = value,
        SPR_MACLO => state.maclo = value,
        SPR_MACHI => state.machi = value,
        SPR_GPR_BASE..=1055 => state.gpr[(address - SPR_GPR_BASE) as usize] = value,
        SPR_VR | SPR_UPR | SPR_CPUCFGR | 3..=9 | SPR_AVR | SPR_PPC | SPR_COREID | SPR_NUMCORES => {
            return Err(OpenRisc1000Error::InvalidSprWrite { address })
        }
        _ if address <= u32::from(u16::MAX) => {}
        _ => return Err(OpenRisc1000Error::InvalidSprWrite { address }),
    }
    Ok(())
}

fn branch_target(pc: u32, displacement: u32) -> u32 {
    pc.wrapping_add(sign_extend(displacement, 26).wrapping_shl(2))
}

fn sign_extend(value: u32, bits: u32) -> u32 {
    (((value << (32 - bits)) as i32) >> (32 - bits)) as u32
}

fn sign_extend16(value: u16) -> u32 {
    (value as i16 as i32) as u32
}

fn effective_address(base: u32, immediate: u16) -> u32 {
    base.wrapping_add(sign_extend16(immediate))
}

fn effective_store_address(base: u32, word: u32) -> u32 {
    let immediate = (((word >> 21) & 0x1f) << 11) | (word & 0x7ff);
    base.wrapping_add(sign_extend16(immediate as u16))
}

fn set_flag(sr: &mut u32, flag: u32, enabled: bool) {
    if enabled {
        *sr |= flag;
    } else {
        *sr &= !flag;
    }
}

fn update_add_flags(state: &mut OpenRisc1000State, lhs: u32, rhs: u32, carry: u32) {
    let wide = u64::from(lhs) + u64::from(rhs) + u64::from(carry);
    let result = wide as u32;
    set_flag(&mut state.sr, SR_CY, wide > u64::from(u32::MAX));
    let signed_overflow = (!(lhs ^ rhs) & (lhs ^ result) & 0x8000_0000) != 0
        || (carry != 0 && lhs == 0x7fff_ffff && rhs == 0);
    set_flag(&mut state.sr, SR_OV, signed_overflow);
}

fn mac(state: &OpenRisc1000State) -> u64 {
    (u64::from(state.machi) << 32) | u64::from(state.maclo)
}

fn set_mac(state: &mut OpenRisc1000State, value: u64) {
    state.machi = (value >> 32) as u32;
    state.maclo = value as u32;
}

fn accumulate_signed(state: &mut OpenRisc1000State, product: i64, subtract: bool) {
    let current = mac(state) as i64;
    let (result, overflow) = if subtract {
        current.overflowing_sub(product)
    } else {
        current.overflowing_add(product)
    };
    set_mac(state, result as u64);
    set_flag(&mut state.sr, SR_OV, overflow);
}

fn accumulate_unsigned(state: &mut OpenRisc1000State, product: u64, subtract: bool) {
    let current = mac(state);
    let (result, carry) = if subtract {
        current.overflowing_sub(product)
    } else {
        current.overflowing_add(product)
    };
    set_mac(state, result);
    set_flag(&mut state.sr, SR_CY, carry);
}

fn load_u8(state: &OpenRisc1000State, address: u32) -> Result<u8, OpenRisc1000Error> {
    let range = checked_range(address, 1)?;
    Ok(state.memory[range.start])
}

fn load_u16(state: &OpenRisc1000State, address: u32) -> Result<u16, OpenRisc1000Error> {
    check_alignment(address, 2)?;
    let range = checked_range(address, 2)?;
    Ok(u16::from_be_bytes(
        state.memory[range].try_into().expect("two-byte load slice"),
    ))
}

fn load_u32(state: &OpenRisc1000State, address: u32) -> Result<u32, OpenRisc1000Error> {
    check_alignment(address, 4)?;
    let range = checked_range(address, 4)?;
    Ok(u32::from_be_bytes(
        state.memory[range]
            .try_into()
            .expect("four-byte load slice"),
    ))
}

fn store_u8(
    state: &mut OpenRisc1000State,
    address: u32,
    value: u8,
) -> Result<(), OpenRisc1000Error> {
    let range = checked_range(address, 1)?;
    state.memory[range.start] = value;
    invalidate_overlapping_reservation(state, address, 1);
    Ok(())
}

fn store_u16(
    state: &mut OpenRisc1000State,
    address: u32,
    value: u16,
) -> Result<(), OpenRisc1000Error> {
    check_alignment(address, 2)?;
    let range = checked_range(address, 2)?;
    state.memory[range].copy_from_slice(&value.to_be_bytes());
    invalidate_overlapping_reservation(state, address, 2);
    Ok(())
}

fn store_u32(
    state: &mut OpenRisc1000State,
    address: u32,
    value: u32,
) -> Result<(), OpenRisc1000Error> {
    check_alignment(address, 4)?;
    let range = checked_range(address, 4)?;
    state.memory[range].copy_from_slice(&value.to_be_bytes());
    invalidate_overlapping_reservation(state, address, 4);
    Ok(())
}
