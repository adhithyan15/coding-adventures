//! Exhaustive first-byte classification for the CDP1802.

use crate::engine::Cdp1802Error;

/// Instruction families visible at the functional boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstructionClass {
    Idle,
    LoadN,
    Increment,
    Decrement,
    ShortControl,
    LoadAdvance,
    Store,
    Io,
    Control,
    GetLow,
    GetHigh,
    PutLow,
    PutHigh,
    LongControl,
    SetProgram,
    SetIndex,
    Alu,
}

/// Stable decode metadata for one first byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub opcode: u8,
    pub class: InstructionClass,
    pub register: Option<u8>,
    pub width: u8,
    pub cycles: u8,
    pub mnemonic: &'static str,
}

const fn instruction(
    opcode: u8,
    class: InstructionClass,
    register: Option<u8>,
    width: u8,
    cycles: u8,
    mnemonic: &'static str,
) -> DecodedInstruction {
    DecodedInstruction {
        opcode,
        class,
        register,
        width,
        cycles,
        mnemonic,
    }
}

const fn short_mnemonic(low: u8) -> &'static str {
    match low {
        0x0 => "BR",
        0x1 => "BQ",
        0x2 => "BZ",
        0x3 => "BDF",
        0x4 => "B1",
        0x5 => "B2",
        0x6 => "B3",
        0x7 => "B4",
        0x8 => "SKP",
        0x9 => "BNQ",
        0xa => "BNZ",
        0xb => "BNF",
        0xc => "BN1",
        0xd => "BN2",
        0xe => "BN3",
        _ => "BN4",
    }
}

const fn control_mnemonic(low: u8) -> &'static str {
    match low {
        0x0 => "IRX",
        0x1 => "OUT 1",
        0x2 => "OUT 2",
        0x3 => "OUT 3",
        0x4 => "OUT 4",
        0x5 => "OUT 5",
        0x6 => "OUT 6",
        0x7 => "OUT 7",
        0x9 => "INP 1",
        0xa => "INP 2",
        0xb => "INP 3",
        0xc => "INP 4",
        0xd => "INP 5",
        0xe => "INP 6",
        _ => "INP 7",
    }
}

const fn seven_mnemonic(low: u8) -> &'static str {
    match low {
        0x0 => "RET",
        0x1 => "DIS",
        0x2 => "LDXA",
        0x3 => "STXD",
        0x4 => "ADC",
        0x5 => "SDB",
        0x6 => "SHRC",
        0x7 => "SMB",
        0x8 => "SAV",
        0x9 => "MARK",
        0xa => "REQ",
        0xb => "SEQ",
        0xc => "ADCI",
        0xd => "SDBI",
        0xe => "SHLC",
        _ => "SMBI",
    }
}

const fn long_mnemonic(low: u8) -> &'static str {
    match low {
        0x0 => "LBR",
        0x1 => "LBQ",
        0x2 => "LBZ",
        0x3 => "LBDF",
        0x4 => "NOP",
        0x5 => "LSNQ",
        0x6 => "LSNZ",
        0x7 => "LSNF",
        0x8 => "LSKP",
        0x9 => "LBNQ",
        0xa => "LBNZ",
        0xb => "LBNF",
        0xc => "LSIE",
        0xd => "LSQ",
        0xe => "LSZ",
        _ => "LSDF",
    }
}

const fn alu_mnemonic(low: u8) -> &'static str {
    match low {
        0x0 => "LDX",
        0x1 => "OR",
        0x2 => "AND",
        0x3 => "XOR",
        0x4 => "ADD",
        0x5 => "SD",
        0x6 => "SHR",
        0x7 => "SM",
        0x8 => "LDI",
        0x9 => "ORI",
        0xa => "ANI",
        0xb => "XRI",
        0xc => "ADI",
        0xd => "SDI",
        0xe => "SHL",
        _ => "SMI",
    }
}

/// Decode one first byte. Opcode `0x68` is the only unassigned encoding.
pub fn decode(opcode: u8) -> Result<DecodedInstruction, Cdp1802Error> {
    let high = opcode >> 4;
    let low = opcode & 0x0f;
    let decoded = match high {
        0x0 if low == 0 => instruction(opcode, InstructionClass::Idle, None, 1, 2, "IDL"),
        0x0 => instruction(opcode, InstructionClass::LoadN, Some(low), 1, 2, "LDN"),
        0x1 => instruction(opcode, InstructionClass::Increment, Some(low), 1, 2, "INC"),
        0x2 => instruction(opcode, InstructionClass::Decrement, Some(low), 1, 2, "DEC"),
        0x3 => instruction(
            opcode,
            InstructionClass::ShortControl,
            None,
            2,
            2,
            short_mnemonic(low),
        ),
        0x4 => instruction(
            opcode,
            InstructionClass::LoadAdvance,
            Some(low),
            1,
            2,
            "LDA",
        ),
        0x5 => instruction(opcode, InstructionClass::Store, Some(low), 1, 2, "STR"),
        0x6 if low == 8 => return Err(Cdp1802Error::UnknownInstruction { raw: opcode }),
        0x6 => instruction(
            opcode,
            InstructionClass::Io,
            None,
            1,
            2,
            control_mnemonic(low),
        ),
        0x7 => instruction(
            opcode,
            InstructionClass::Control,
            None,
            if matches!(low, 0xc | 0xd | 0xf) { 2 } else { 1 },
            2,
            seven_mnemonic(low),
        ),
        0x8 => instruction(opcode, InstructionClass::GetLow, Some(low), 1, 2, "GLO"),
        0x9 => instruction(opcode, InstructionClass::GetHigh, Some(low), 1, 2, "GHI"),
        0xa => instruction(opcode, InstructionClass::PutLow, Some(low), 1, 2, "PLO"),
        0xb => instruction(opcode, InstructionClass::PutHigh, Some(low), 1, 2, "PHI"),
        0xc => {
            let branch = matches!(low, 0x0..=0x3 | 0x9..=0xb);
            instruction(
                opcode,
                InstructionClass::LongControl,
                None,
                if branch { 3 } else { 1 },
                3,
                long_mnemonic(low),
            )
        }
        0xd => instruction(opcode, InstructionClass::SetProgram, Some(low), 1, 2, "SEP"),
        0xe => instruction(opcode, InstructionClass::SetIndex, Some(low), 1, 2, "SEX"),
        _ => instruction(
            opcode,
            InstructionClass::Alu,
            None,
            if matches!(low, 0x8..=0xd | 0xf) { 2 } else { 1 },
            2,
            alu_mnemonic(low),
        ),
    };
    Ok(decoded)
}
