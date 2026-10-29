//! Complete byte decoder for the pinned ZPU4-small profile.

/// One operation selected by an instruction byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Im,
    LoadSp,
    StoreSp,
    Emulate,
    AddSp,
    Break,
    PushSp,
    PopPc,
    Add,
    And,
    Or,
    Load,
    Not,
    Flip,
    Nop,
    Store,
    PopSp,
}

/// Complete production decode result. Every byte is legal in this profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub operation: Operation,
    pub parameter: u8,
}

/// Decode one byte according to `zpu_core_small.vhd`.
#[must_use]
pub const fn decode(raw: u8) -> DecodedInstruction {
    use Operation::*;
    let (operation, parameter) = if raw & 0x80 != 0 {
        (Im, raw & 0x7f)
    } else if raw & 0xe0 == 0x60 {
        (LoadSp, offset5(raw))
    } else if raw & 0xe0 == 0x40 {
        (StoreSp, offset5(raw))
    } else if raw & 0xe0 == 0x20 {
        (Emulate, raw & 0x1f)
    } else if raw & 0xf0 == 0x10 {
        (AddSp, raw & 0x0f)
    } else {
        let operation = match raw & 0x0f {
            0x0 => Break,
            0x2 => PushSp,
            0x4 => PopPc,
            0x5 => Add,
            0x6 => And,
            0x7 => Or,
            0x8 => Load,
            0x9 => Not,
            0xa => Flip,
            0xc => Store,
            0xd => PopSp,
            0x1 | 0x3 | 0xb | 0xe | 0xf => Nop,
            _ => Nop,
        };
        (operation, 0)
    };
    DecodedInstruction {
        operation,
        parameter,
    }
}

/// The VHDL inverts encoded bit four before using the five-bit word offset.
#[must_use]
pub const fn offset5(raw: u8) -> u8 {
    (raw & 0x0f) | ((!raw) & 0x10)
}
