//! Exhaustive first-byte classification for the CDP1802.

use crate::engine::Cdp1802Error;
use crate::gates::{any_line, one_hot_index, opcode_lines};
use logic_gates::gates::and_gate;

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
    let (high, low) = opcode_lines(opcode);
    decode_from_lines(opcode, &high, &low)
}

pub(crate) fn decode_from_lines(
    opcode: u8,
    high: &[u8; 16],
    low: &[u8; 16],
) -> Result<DecodedInstruction, Cdp1802Error> {
    let low_index = one_hot_index(low);
    if and_gate(high[6], low[8]) == 1 {
        return Err(Cdp1802Error::UnknownInstruction { raw: opcode });
    }
    let decoded = if and_gate(high[0], low[0]) == 1 {
        instruction(opcode, InstructionClass::Idle, None, 1, 2, "IDL")
    } else if high[0] == 1 {
        instruction(
            opcode,
            InstructionClass::LoadN,
            Some(low_index),
            1,
            2,
            "LDN",
        )
    } else if high[1] == 1 {
        instruction(
            opcode,
            InstructionClass::Increment,
            Some(low_index),
            1,
            2,
            "INC",
        )
    } else if high[2] == 1 {
        instruction(
            opcode,
            InstructionClass::Decrement,
            Some(low_index),
            1,
            2,
            "DEC",
        )
    } else if high[3] == 1 {
        instruction(
            opcode,
            InstructionClass::ShortControl,
            None,
            2,
            2,
            short_mnemonic(low_index),
        )
    } else if high[4] == 1 {
        instruction(
            opcode,
            InstructionClass::LoadAdvance,
            Some(low_index),
            1,
            2,
            "LDA",
        )
    } else if high[5] == 1 {
        instruction(
            opcode,
            InstructionClass::Store,
            Some(low_index),
            1,
            2,
            "STR",
        )
    } else if high[6] == 1 {
        instruction(
            opcode,
            InstructionClass::Io,
            None,
            1,
            2,
            control_mnemonic(low_index),
        )
    } else if high[7] == 1 {
        instruction(
            opcode,
            InstructionClass::Control,
            None,
            1 + any_line(low, &[0xc, 0xd, 0xf]),
            2,
            seven_mnemonic(low_index),
        )
    } else if high[8] == 1 {
        instruction(
            opcode,
            InstructionClass::GetLow,
            Some(low_index),
            1,
            2,
            "GLO",
        )
    } else if high[9] == 1 {
        instruction(
            opcode,
            InstructionClass::GetHigh,
            Some(low_index),
            1,
            2,
            "GHI",
        )
    } else if high[10] == 1 {
        instruction(
            opcode,
            InstructionClass::PutLow,
            Some(low_index),
            1,
            2,
            "PLO",
        )
    } else if high[11] == 1 {
        instruction(
            opcode,
            InstructionClass::PutHigh,
            Some(low_index),
            1,
            2,
            "PHI",
        )
    } else if high[12] == 1 {
        instruction(
            opcode,
            InstructionClass::LongControl,
            None,
            1 + 2 * any_line(low, &[0x0, 0x1, 0x2, 0x3, 0x9, 0xa, 0xb]),
            3,
            long_mnemonic(low_index),
        )
    } else if high[13] == 1 {
        instruction(
            opcode,
            InstructionClass::SetProgram,
            Some(low_index),
            1,
            2,
            "SEP",
        )
    } else if high[14] == 1 {
        instruction(
            opcode,
            InstructionClass::SetIndex,
            Some(low_index),
            1,
            2,
            "SEX",
        )
    } else {
        instruction(
            opcode,
            InstructionClass::Alu,
            None,
            1 + any_line(low, &[0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xf]),
            2,
            alu_mnemonic(low_index),
        )
    };
    Ok(decoded)
}
