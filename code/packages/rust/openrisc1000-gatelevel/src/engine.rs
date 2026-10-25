//! Independent OpenRISC 1000 ORBIS32 gate transition engine.
//!
//! OpenRISC is pleasantly regular: every instruction is one big-endian word,
//! the primary opcode occupies the upper six bits, and most operands keep the
//! same positions. The engine keeps that structure visible instead of
//! hiding it behind a generated decoder. Every operation selector comes from a
//! gate-built one-hot line, and every architectural datapath result is produced
//! by the fixed-width networks in `crate::gates`.

use core::fmt;

use logic_gates::gates::{and_gate, not_gate, or_gate};

use crate::gates;

/// Size of this teaching machine's flat physical address space.
pub const MEMORY_SIZE: usize = 65_536;
/// Maximum complete-state traces retained by one execution.
pub const MAX_RETAINED_STEPS: usize = 128;

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
    gates::primary_is_implemented(opcode) == 1
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

pub(crate) fn validate_image(program: &[u8], origin: u32) -> Result<(), OpenRisc1000Error> {
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

pub(crate) fn validate_state(state: &OpenRisc1000State) -> Result<(), OpenRisc1000Error> {
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

fn checked_range(address: u32, width: usize) -> Result<std::ops::Range<usize>, OpenRisc1000Error> {
    let (end, carry, _) = gates::add_32(address, width as u32, 0);
    let address_in_range = gates::unsigned_less_32(address, MEMORY_SIZE as u32);
    let end_in_range = not_gate(gates::unsigned_less_32(MEMORY_SIZE as u32, end));
    if or_gate(carry, not_gate(and_gate(address_in_range, end_in_range))) == 1 {
        return Err(OpenRisc1000Error::DataOutOfRange { address, width });
    }
    Ok(address as usize..end as usize)
}

fn check_alignment(address: u32, width: usize) -> Result<(), OpenRisc1000Error> {
    let bits = gates::u32_to_bits(address);
    let misaligned = match width {
        1 => 0,
        2 => bits[0],
        4 => or_gate(bits[0], bits[1]),
        _ => u8::from(!(address as usize).is_multiple_of(width)),
    };
    if misaligned == 1 {
        return Err(OpenRisc1000Error::MisalignedData { address, width });
    }
    Ok(())
}

fn invalidate_overlapping_reservation(state: &mut OpenRisc1000State, address: u32, width: usize) {
    let Some(reserved) = state.reservation else {
        return;
    };
    let store_end = gates::add_32(address, width as u32, 0).0;
    let reservation_end = gates::add_32(reserved, 4, 0).0;
    if and_gate(
        gates::unsigned_less_32(address, reservation_end),
        gates::unsigned_less_32(reserved, store_end),
    ) == 1
    {
        state.reservation = None;
    }
}

fn require_spr_privilege(
    state: &OpenRisc1000State,
    address: u32,
    word: u32,
    pc: u32,
) -> Result<(), OpenRisc1000Error> {
    let supervisor_only = [
        SPR_VR,
        SPR_UPR,
        SPR_CPUCFGR,
        SPR_AVR,
        SPR_NPC,
        SPR_SR,
        SPR_PPC,
        SPR_EPCR0,
        SPR_EEAR0,
        SPR_ESR0,
        SPR_COREID,
        SPR_NUMCORES,
    ]
    .into_iter()
    .fold(0, |selected, spr| or_gate(selected, spr_line(address, spr)));
    let supervisor_only = or_gate(
        supervisor_only,
        or_gate(
            gate_inclusive_range(address, 3, 9),
            gate_inclusive_range(address, SPR_GPR_BASE, 1055),
        ),
    );
    if and_gate(not_gate(gate_flag(state.sr, SR_SM)), supervisor_only) == 1 {
        return Err(OpenRisc1000Error::PrivilegedInstruction { word, pc });
    }
    Ok(())
}

fn fetch_word(state: &OpenRisc1000State) -> Result<([u8; 4], u32), OpenRisc1000Error> {
    let pc_bits = gates::u32_to_bits(state.pc);
    if or_gate(pc_bits[0], pc_bits[1]) == 1 {
        return Err(OpenRisc1000Error::MisalignedFetch { pc: state.pc });
    }
    let start = state.pc as usize;
    let installed_end = gates::add_32(state.loaded_origin, state.loaded_len as u32, 0).0;
    let (end, carry, _) = gates::add_32(state.pc, 4, 0);
    if carry == 1
        || gates::unsigned_less_32(state.pc, state.loaded_origin) == 1
        || gates::unsigned_less_32(installed_end, end) == 1
    {
        return Err(OpenRisc1000Error::FetchOutsideProgram { pc: state.pc });
    }
    let end = end as usize;
    let raw: [u8; 4] = state.memory[start..end]
        .try_into()
        .expect("four-byte fetch slice");
    Ok((raw, u32::from_be_bytes(raw)))
}

pub(crate) fn step_state(mut state: OpenRisc1000State) -> Result<StepTrace, OpenRisc1000Error> {
    if state.halted {
        return Err(OpenRisc1000Error::Halted);
    }
    let pc_before = state.pc;
    let (raw, word) = fetch_word(&state)?;
    state.ppc = pc_before;
    state.pc = gates::add_32(pc_before, 4, 0).0;
    let mnemonic = execute_word(&mut state, word, pc_before)?;
    state.sr = gates::bit_or_32(state.sr, SR_FO);
    Ok(StepTrace {
        pc_before,
        pc_after: state.pc,
        raw,
        raw_word: word,
        mnemonic,
        state_after: state,
    })
}

fn execute_word(
    state: &mut OpenRisc1000State,
    word: u32,
    pc: u32,
) -> Result<&'static str, OpenRisc1000Error> {
    let opcode = gate_field(word, 26, 6) as u8;
    let primary = gates::decode_primary(opcode);
    let opcode = primary
        .iter()
        .position(|&line| line == 1)
        .expect("six-input decoder always selects one line") as u8;
    let rd = gate_field(word, 21, 5) as usize;
    let ra = gate_field(word, 16, 5) as usize;
    let rb = gate_field(word, 11, 5) as usize;
    let immediate = gate_field(word, 0, 16) as u16;
    match opcode {
        0x00 => {
            state.pc = branch_target(pc, gate_field(word, 0, 26));
            Ok("l.j")
        }
        0x01 => {
            state.gpr[9] = gates::add_32(pc, 4, 0).0;
            state.pc = branch_target(pc, gate_field(word, 0, 26));
            Ok("l.jal")
        }
        0x02 => {
            let offset = gates::shift_left_32(gates::sign_extend(gate_field(word, 0, 19), 19), 13);
            let page = gates::bit_and_32(pc, gates::bit_not_32(0x1fff));
            state.gpr[rd] = gates::add_32(page, offset, 0).0;
            Ok("l.adrp")
        }
        0x03 => {
            let take = not_gate(gate_flag(state.sr, SR_F));
            let target = branch_target(pc, gate_field(word, 0, 26));
            state.pc = gates::select_32(state.pc, target, take);
            Ok("l.bnf")
        }
        0x04 => {
            let target = branch_target(pc, gate_field(word, 0, 26));
            state.pc = gates::select_32(state.pc, target, gate_flag(state.sr, SR_F));
            Ok("l.bf")
        }
        0x05 if gates::selected(gate_field(word, 24, 8), 0x15, 8) == 1 => {
            state.halted = or_gate(
                u8::from(state.halted),
                gates::selected(u32::from(immediate), 1, 16),
            ) == 1;
            Ok("l.nop")
        }
        0x06 if gates::selected(gate_field(word, 0, 21), 0x0001_0000, 21) == 1 => {
            state.gpr[rd] = state.maclo;
            state.maclo = 0;
            state.machi = 0;
            Ok("l.macrc")
        }
        0x06 if gates::selected(gate_field(word, 16, 1), 0, 1) == 1 => {
            state.gpr[rd] = gates::shift_left_32(u32::from(immediate), 16);
            Ok("l.movhi")
        }
        0x08 => execute_system(state, word, pc),
        0x09 => {
            state.pc = state.epcr0;
            state.sr = gates::bit_or_32(state.esr0, SR_FO);
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
            state.gpr[9] = gates::add_32(pc, 4, 0).0;
            state.pc = state.gpr[rb];
            Ok("l.jalr")
        }
        0x13 => {
            let product = gates::multiply_signed_32(
                state.gpr[ra],
                gates::sign_extend(u32::from(immediate), 16),
            );
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
            state.gpr[rd] = gates::sign_extend(u32::from(load_u8(state, address)?), 8);
            Ok("l.lbs")
        }
        0x25 => {
            let address = effective_address(state.gpr[ra], immediate);
            state.gpr[rd] = u32::from(load_u16(state, address)?);
            Ok("l.lhz")
        }
        0x26 => {
            let address = effective_address(state.gpr[ra], immediate);
            state.gpr[rd] = gates::sign_extend(u32::from(load_u16(state, address)?), 16);
            Ok("l.lhs")
        }
        0x27 => {
            let rhs = gates::sign_extend(u32::from(immediate), 16);
            let (result, carry, overflow) = gates::add_32(state.gpr[ra], rhs, 0);
            update_add_flags(state, carry, overflow);
            state.gpr[rd] = result;
            Ok("l.addi")
        }
        0x28 => {
            let rhs = gates::sign_extend(u32::from(immediate), 16);
            let carry = gate_flag(state.sr, SR_CY);
            let (result, carry_out, overflow) = gates::add_32(state.gpr[ra], rhs, carry);
            update_add_flags(state, carry_out, overflow);
            state.gpr[rd] = result;
            Ok("l.addic")
        }
        0x29 => {
            state.gpr[rd] = gates::bit_and_32(state.gpr[ra], u32::from(immediate));
            Ok("l.andi")
        }
        0x2a => {
            state.gpr[rd] = gates::bit_or_32(state.gpr[ra], u32::from(immediate));
            Ok("l.ori")
        }
        0x2b => {
            state.gpr[rd] =
                gates::bit_xor_32(state.gpr[ra], gates::sign_extend(u32::from(immediate), 16));
            Ok("l.xori")
        }
        0x2c => {
            let product = gates::multiply_signed_32(
                state.gpr[ra],
                gates::sign_extend(u32::from(immediate), 16),
            );
            let product_bits = gates::u64_to_bits(product);
            state.gpr[rd] = gates::bits_to_u32(&product_bits[..32]);
            let high = gates::bits_to_u32(&product_bits[32..]);
            let expected_high =
                gates::select_32(0, u32::MAX, gates::u32_to_bits(state.gpr[rd])[31]);
            set_flag(
                &mut state.sr,
                SR_OV,
                gates::equal_32(high, expected_high) == 0,
            );
            Ok("l.muli")
        }
        0x2d => {
            let address = gates::bit_or_32(state.gpr[ra], u32::from(immediate));
            require_spr_privilege(state, address, word, pc)?;
            state.gpr[rd] = read_spr(state, address)?;
            Ok("l.mfspr")
        }
        0x2e => execute_shift_immediate(state, word, rd, ra, pc),
        0x2f => execute_compare_immediate(state, word, ra, immediate, pc),
        0x30 => {
            let spr = gates::bit_or_32(
                gates::shift_left_32(gate_field(word, 21, 5), 11),
                gate_field(word, 0, 11),
            );
            let address = gates::bit_or_32(state.gpr[ra], spr);
            require_spr_privilege(state, address, word, pc)?;
            write_spr(state, address, state.gpr[rb])?;
            Ok("l.mtspr")
        }
        0x31 => execute_mac(state, word, ra, rb, pc),
        0x33 => {
            let address = effective_store_address(state.gpr[ra], word);
            check_alignment(address, 4)?;
            checked_range(address, 4)?;
            let succeeds = state
                .reservation
                .is_some_and(|reserved| gates::equal_32(reserved, address) == 1);
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
    if gates::selected(gate_field(word, 16, 16), 0x2000, 16) == 1 {
        enter_exception(state, gates::add_32(pc, 4, 0).0, 0x0c00);
        Ok("l.sys")
    } else if gates::selected(gate_field(word, 16, 16), 0x2100, 16) == 1 {
        enter_exception(state, pc, 0x0e00);
        Ok("l.trap")
    } else if gates::selected(word, 0x2200_0000, 32) == 1 {
        Ok("l.msync")
    } else if gates::selected(word, 0x2280_0000, 32) == 1 {
        Ok("l.psync")
    } else if gates::selected(word, 0x2300_0000, 32) == 1 {
        Ok("l.csync")
    } else {
        Err(OpenRisc1000Error::UnknownInstruction { word, pc })
    }
}

fn enter_exception(state: &mut OpenRisc1000State, return_pc: u32, vector: u32) {
    state.epcr0 = return_pc;
    state.esr0 = state.sr;
    state.eear0 = 0;
    state.sr = gates::bit_and_32(
        gates::bit_or_32(gates::bit_or_32(state.sr, SR_SM), SR_FO),
        gates::bit_not_32(SR_DSX | SR_DME | SR_IME | SR_IEE | SR_TEE),
    );
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
    if gates::selected(gate_field(word, 8, 4), 0, 4) == 0 {
        return Err(OpenRisc1000Error::UnknownInstruction { word, pc });
    }
    let amount = gate_field(word, 0, 5);
    let function = gate_decode_known(gate_field(word, 6, 2), 2, &[0, 1, 2, 3])
        .expect("two-input decoder always selects one line");
    match function {
        0 => {
            state.gpr[rd] = gates::shift_left_32(state.gpr[ra], amount);
            Ok("l.slli")
        }
        1 => {
            state.gpr[rd] = gates::shift_right_32(state.gpr[ra], amount);
            Ok("l.srli")
        }
        2 => {
            state.gpr[rd] = gates::shift_arithmetic_right_32(state.gpr[ra], amount);
            Ok("l.srai")
        }
        3 => {
            state.gpr[rd] = gates::rotate_right_32(state.gpr[ra], amount);
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
    let condition = gate_decode_known(
        gate_field(word, 21, 5),
        5,
        &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x0a, 0x0b, 0x0c, 0x0d],
    )
    .ok_or(OpenRisc1000Error::UnknownInstruction { word, pc })?;
    let lhs = state.gpr[ra];
    let signed = gates::sign_extend(u32::from(immediate), 16);
    let (result, mnemonic) = match condition {
        0x00 => (gates::equal_32(lhs, signed) == 1, "l.sfeqi"),
        0x01 => (gates::equal_32(lhs, signed) == 0, "l.sfnei"),
        0x02 => (gates::unsigned_less_32(signed, lhs) == 1, "l.sfgtui"),
        0x03 => (gates::unsigned_less_32(lhs, signed) == 0, "l.sfgeui"),
        0x04 => (gates::unsigned_less_32(lhs, signed) == 1, "l.sfltui"),
        0x05 => (gates::unsigned_less_32(signed, lhs) == 0, "l.sfleui"),
        0x0a => (gates::signed_less_32(signed, lhs) == 1, "l.sfgtsi"),
        0x0b => (gates::signed_less_32(lhs, signed) == 0, "l.sfgesi"),
        0x0c => (gates::signed_less_32(lhs, signed) == 1, "l.sfltsi"),
        0x0d => (gates::signed_less_32(signed, lhs) == 0, "l.sflesi"),
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
    let condition = gate_decode_known(
        gate_field(word, 21, 5),
        5,
        &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x0a, 0x0b, 0x0c, 0x0d],
    )
    .ok_or(OpenRisc1000Error::UnknownInstruction { word, pc })?;
    let lhs = state.gpr[ra];
    let rhs = state.gpr[rb];
    let (result, mnemonic) = match condition {
        0x00 => (gates::equal_32(lhs, rhs) == 1, "l.sfeq"),
        0x01 => (gates::equal_32(lhs, rhs) == 0, "l.sfne"),
        0x02 => (gates::unsigned_less_32(rhs, lhs) == 1, "l.sfgtu"),
        0x03 => (gates::unsigned_less_32(lhs, rhs) == 0, "l.sfgeu"),
        0x04 => (gates::unsigned_less_32(lhs, rhs) == 1, "l.sfltu"),
        0x05 => (gates::unsigned_less_32(rhs, lhs) == 0, "l.sfleu"),
        0x0a => (gates::signed_less_32(rhs, lhs) == 1, "l.sfgts"),
        0x0b => (gates::signed_less_32(lhs, rhs) == 0, "l.sfges"),
        0x0c => (gates::signed_less_32(lhs, rhs) == 1, "l.sflts"),
        0x0d => (gates::signed_less_32(rhs, lhs) == 0, "l.sfles"),
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
    let function = gate_decode_known(gate_field(word, 0, 4), 4, &[1, 2, 3, 4])
        .ok_or(OpenRisc1000Error::UnknownInstruction { word, pc })?;
    match function {
        1 => {
            let product = gates::multiply_signed_32(state.gpr[ra], state.gpr[rb]);
            accumulate_signed(state, product, false);
            Ok("l.mac")
        }
        2 => {
            let product = gates::multiply_signed_32(state.gpr[ra], state.gpr[rb]);
            accumulate_signed(state, product, true);
            Ok("l.msb")
        }
        3 => {
            let product = gates::multiply_unsigned_32(state.gpr[ra], state.gpr[rb]);
            accumulate_unsigned(state, product, false);
            Ok("l.macu")
        }
        4 => {
            let product = gates::multiply_unsigned_32(state.gpr[ra], state.gpr[rb]);
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
    let function = gate_decode_known(
        gate_field(word, 0, 11),
        11,
        &[
            0x000, 0x001, 0x002, 0x003, 0x004, 0x005, 0x008, 0x048, 0x088, 0x0c8, 0x00e, 0x00f,
            0x04f, 0x00c, 0x00d, 0x04c, 0x04d, 0x08c, 0x0cc, 0x306, 0x307, 0x309, 0x30a, 0x30b,
            0x30c,
        ],
    )
    .ok_or(OpenRisc1000Error::UnknownInstruction { word, pc })?;
    let lhs = state.gpr[ra];
    let rhs = state.gpr[rb];
    match function {
        0x000 => {
            let (result, carry, overflow) = gates::add_32(lhs, rhs, 0);
            update_add_flags(state, carry, overflow);
            state.gpr[rd] = result;
            Ok("l.add")
        }
        0x001 => {
            let carry = gate_flag(state.sr, SR_CY);
            let (result, carry, overflow) = gates::add_32(lhs, rhs, carry);
            update_add_flags(state, carry, overflow);
            state.gpr[rd] = result;
            Ok("l.addc")
        }
        0x002 => {
            let (result, borrow, overflow) = gates::sub_32(lhs, rhs);
            state.gpr[rd] = result;
            set_flag(&mut state.sr, SR_CY, borrow == 1);
            set_flag(&mut state.sr, SR_OV, overflow == 1);
            Ok("l.sub")
        }
        0x003 => {
            state.gpr[rd] = gates::bit_and_32(lhs, rhs);
            Ok("l.and")
        }
        0x004 => {
            state.gpr[rd] = gates::bit_or_32(lhs, rhs);
            Ok("l.or")
        }
        0x005 => {
            state.gpr[rd] = gates::bit_xor_32(lhs, rhs);
            Ok("l.xor")
        }
        0x008 => {
            state.gpr[rd] = gates::shift_left_32(lhs, gate_field(rhs, 0, 5));
            Ok("l.sll")
        }
        0x048 => {
            state.gpr[rd] = gates::shift_right_32(lhs, gate_field(rhs, 0, 5));
            Ok("l.srl")
        }
        0x088 => {
            state.gpr[rd] = gates::shift_arithmetic_right_32(lhs, gate_field(rhs, 0, 5));
            Ok("l.sra")
        }
        0x0c8 => {
            state.gpr[rd] = gates::rotate_right_32(lhs, gate_field(rhs, 0, 5));
            Ok("l.ror")
        }
        0x00e => {
            state.gpr[rd] = gates::select_32(rhs, lhs, gate_flag(state.sr, SR_F));
            Ok("l.cmov")
        }
        0x00f => {
            state.gpr[rd] = gates::first_one(lhs);
            Ok("l.ff1")
        }
        0x04f => {
            state.gpr[rd] = gates::last_one(lhs);
            Ok("l.fl1")
        }
        0x00c => {
            state.gpr[rd] = gates::sign_extend(gate_field(lhs, 0, 16), 16);
            Ok("l.exths")
        }
        0x00d => {
            state.gpr[rd] = lhs;
            Ok("l.extws")
        }
        0x04c => {
            state.gpr[rd] = gates::sign_extend(gate_field(lhs, 0, 8), 8);
            Ok("l.extbs")
        }
        0x04d => {
            state.gpr[rd] = lhs;
            Ok("l.extwz")
        }
        0x08c => {
            state.gpr[rd] = gate_field(lhs, 0, 16);
            Ok("l.exthz")
        }
        0x0cc => {
            state.gpr[rd] = gate_field(lhs, 0, 8);
            Ok("l.extbz")
        }
        0x306 => {
            let product = gates::multiply_signed_32(lhs, rhs);
            let product_bits = gates::u64_to_bits(product);
            state.gpr[rd] = gates::bits_to_u32(&product_bits[..32]);
            let high = gates::bits_to_u32(&product_bits[32..]);
            let expected_high =
                gates::select_32(0, u32::MAX, gates::u32_to_bits(state.gpr[rd])[31]);
            set_flag(
                &mut state.sr,
                SR_OV,
                gates::equal_32(high, expected_high) == 0,
            );
            Ok("l.mul")
        }
        0x307 => {
            set_mac(state, gates::multiply_signed_32(lhs, rhs));
            Ok("l.muld")
        }
        0x309 => {
            if gates::equal_32(rhs, 0) == 1 {
                return Err(OpenRisc1000Error::DivideByZero);
            }
            if and_gate(
                gates::selected(lhs, i32::MIN as u32, 32),
                gates::selected(rhs, u32::MAX, 32),
            ) == 1
            {
                return Err(OpenRisc1000Error::SignedDivisionOverflow);
            }
            state.gpr[rd] = gates::divide_signed_32(lhs, rhs);
            set_flag(&mut state.sr, SR_OV, false);
            Ok("l.div")
        }
        0x30a => {
            if gates::equal_32(rhs, 0) == 1 {
                return Err(OpenRisc1000Error::DivideByZero);
            }
            state.gpr[rd] = gates::divide_unsigned_32(lhs, rhs).0;
            set_flag(&mut state.sr, SR_CY, false);
            Ok("l.divu")
        }
        0x30b => {
            let product = gates::multiply_unsigned_32(lhs, rhs);
            let product_bits = gates::u64_to_bits(product);
            state.gpr[rd] = gates::bits_to_u32(&product_bits[..32]);
            set_flag(
                &mut state.sr,
                SR_CY,
                gates::equal_32(gates::bits_to_u32(&product_bits[32..]), 0) == 0,
            );
            Ok("l.mulu")
        }
        0x30c => {
            set_mac(state, gates::multiply_unsigned_32(lhs, rhs));
            Ok("l.muldu")
        }
        _ => Err(OpenRisc1000Error::UnknownInstruction { word, pc }),
    }
}

fn read_spr(state: &OpenRisc1000State, address: u32) -> Result<u32, OpenRisc1000Error> {
    if spr_line(address, SPR_VR) == 1 || spr_line(address, SPR_AVR) == 1 {
        Ok(0x0104_0000)
    } else if spr_line(address, SPR_UPR) == 1 {
        Ok((1 << 0) | (1 << 6))
    } else if spr_line(address, SPR_CPUCFGR) == 1 {
        // OB32S plus no-delay-slot support.
        Ok((1 << 0) | (1 << 10))
    } else if gate_inclusive_range(address, 3, 9) == 1 || spr_line(address, SPR_COREID) == 1 {
        Ok(0)
    } else if spr_line(address, SPR_NPC) == 1 {
        Ok(state.pc)
    } else if spr_line(address, SPR_SR) == 1 {
        Ok(gates::bit_or_32(state.sr, SR_FO))
    } else if spr_line(address, SPR_PPC) == 1 {
        Ok(state.ppc)
    } else if spr_line(address, SPR_EPCR0) == 1 {
        Ok(state.epcr0)
    } else if spr_line(address, SPR_EEAR0) == 1 {
        Ok(state.eear0)
    } else if spr_line(address, SPR_ESR0) == 1 {
        Ok(state.esr0)
    } else if spr_line(address, SPR_NUMCORES) == 1 {
        Ok(1)
    } else if spr_line(address, SPR_MACLO) == 1 {
        Ok(state.maclo)
    } else if spr_line(address, SPR_MACHI) == 1 {
        Ok(state.machi)
    } else if gate_inclusive_range(address, SPR_GPR_BASE, 1055) == 1 {
        Ok(state.gpr[gates::sub_32(address, SPR_GPR_BASE).0 as usize])
    } else if gates::unsigned_less_32(address, u32::from(u16::MAX) + 1) == 1 {
        Ok(0)
    } else {
        Err(OpenRisc1000Error::InvalidSprRead { address })
    }
}

fn write_spr(
    state: &mut OpenRisc1000State,
    address: u32,
    value: u32,
) -> Result<(), OpenRisc1000Error> {
    if spr_line(address, SPR_NPC) == 1 {
        state.pc = value;
    } else if spr_line(address, SPR_SR) == 1 {
        state.sr = gates::bit_or_32(value, SR_FO);
    } else if spr_line(address, SPR_EPCR0) == 1 {
        state.epcr0 = value;
    } else if spr_line(address, SPR_EEAR0) == 1 {
        state.eear0 = value;
    } else if spr_line(address, SPR_ESR0) == 1 {
        state.esr0 = value;
    } else if spr_line(address, SPR_MACLO) == 1 {
        state.maclo = value;
    } else if spr_line(address, SPR_MACHI) == 1 {
        state.machi = value;
    } else if gate_inclusive_range(address, SPR_GPR_BASE, 1055) == 1 {
        state.gpr[gates::sub_32(address, SPR_GPR_BASE).0 as usize] = value;
    } else if spr_line(address, SPR_VR) == 1
        || spr_line(address, SPR_UPR) == 1
        || spr_line(address, SPR_CPUCFGR) == 1
        || gate_inclusive_range(address, 3, 9) == 1
        || spr_line(address, SPR_AVR) == 1
        || spr_line(address, SPR_PPC) == 1
        || spr_line(address, SPR_COREID) == 1
        || spr_line(address, SPR_NUMCORES) == 1
        || gates::unsigned_less_32(address, u32::from(u16::MAX) + 1) == 0
    {
        return Err(OpenRisc1000Error::InvalidSprWrite { address });
    }
    Ok(())
}

fn spr_line(address: u32, expected: u32) -> u8 {
    gates::selected(address, expected, 32)
}

fn gate_inclusive_range(value: u32, start: u32, end: u32) -> u8 {
    and_gate(
        not_gate(gates::unsigned_less_32(value, start)),
        not_gate(gates::unsigned_less_32(end, value)),
    )
}

fn branch_target(pc: u32, displacement: u32) -> u32 {
    let offset = gates::shift_left_32(gates::sign_extend(displacement, 26), 2);
    gates::add_32(pc, offset, 0).0
}

fn gate_field(value: u32, start: usize, width: usize) -> u32 {
    let bits = gates::u32_to_bits(value);
    gates::bits_to_u32(&bits[start..start + width])
}

fn gate_decode_known(value: u32, width: usize, lines: &[u32]) -> Option<u32> {
    lines
        .iter()
        .copied()
        .find(|&line| gates::selected(value, line, width) == 1)
}

fn gate_flag(value: u32, flag: u32) -> u8 {
    let bit = flag.trailing_zeros() as usize;
    gates::u32_to_bits(value)[bit]
}

fn effective_address(base: u32, immediate: u16) -> u32 {
    gates::add_32(base, gates::sign_extend(u32::from(immediate), 16), 0).0
}

fn effective_store_address(base: u32, word: u32) -> u32 {
    let immediate = gates::bit_or_32(
        gates::shift_left_32(gate_field(word, 21, 5), 11),
        gate_field(word, 0, 11),
    );
    gates::add_32(base, gates::sign_extend(immediate, 16), 0).0
}

fn set_flag(sr: &mut u32, flag: u32, enabled: bool) {
    let set = gates::bit_or_32(*sr, flag);
    let cleared = gates::bit_and_32(*sr, gates::bit_not_32(flag));
    *sr = gates::select_32(cleared, set, u8::from(enabled));
}

fn update_add_flags(state: &mut OpenRisc1000State, carry: u8, overflow: u8) {
    set_flag(&mut state.sr, SR_CY, carry == 1);
    set_flag(&mut state.sr, SR_OV, overflow == 1);
}

fn mac(state: &OpenRisc1000State) -> u64 {
    let mut bits = [0_u8; 64];
    bits[..32].copy_from_slice(&gates::u32_to_bits(state.maclo));
    bits[32..].copy_from_slice(&gates::u32_to_bits(state.machi));
    gates::bits_to_u64(&bits)
}

fn set_mac(state: &mut OpenRisc1000State, value: u64) {
    let bits = gates::u64_to_bits(value);
    state.maclo = gates::bits_to_u32(&bits[..32]);
    state.machi = gates::bits_to_u32(&bits[32..]);
}

fn accumulate_signed(state: &mut OpenRisc1000State, product: u64, subtract: bool) {
    let current = mac(state);
    let (result, _, overflow) = if subtract {
        gates::sub_64(current, product)
    } else {
        gates::add_64(current, product, 0)
    };
    set_mac(state, result);
    set_flag(&mut state.sr, SR_OV, overflow == 1);
}

fn accumulate_unsigned(state: &mut OpenRisc1000State, product: u64, subtract: bool) {
    let current = mac(state);
    let (result, carry, _) = if subtract {
        let (value, borrow, overflow) = gates::sub_64(current, product);
        (value, borrow, overflow)
    } else {
        gates::add_64(current, product, 0)
    };
    set_mac(state, result);
    set_flag(&mut state.sr, SR_CY, carry == 1);
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
