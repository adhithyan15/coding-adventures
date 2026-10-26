//! Independent table-driven LM32 instruction decoder.

/// An executable operation in the fixed `Lm32BaseFunctionalV1` profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Sru,
    Nor,
    Mul,
    Sh,
    Lb,
    Sr,
    Xor,
    Lh,
    And,
    Xnor,
    Lw,
    Lhu,
    Sb,
    Add,
    Or,
    Sl,
    Lbu,
    Be,
    Bg,
    Bge,
    Bgeu,
    Bgu,
    Sw,
    Bne,
    AndHi,
    Cmpe,
    Cmpg,
    Cmpge,
    Cmpgeu,
    Cmpgu,
    OrHi,
    Cmpne,
    Divu,
    Rcsr,
    Sextb,
    Modu,
    Sub,
    Wcsr,
    Call,
    Sexth,
    Bi,
    Calli,
    Branch,
    Break,
    Scall,
}

/// Decoded fields. Source and destination interpretation is operation-specific.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub operation: Operation,
    pub mnemonic: &'static str,
    pub source_0: u8,
    pub source_1: u8,
    pub destination: u8,
    pub immediate: u32,
    pub raw: u32,
}

/// Decode-only classification failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Unknown,
    Unsupported,
}

const fn sign_extend_16(value: u16) -> u32 {
    (value as i16 as i32) as u32
}

const fn sign_extend_26(value: u32) -> u32 {
    (((value << 6) as i32) >> 6) as u32
}

const fn operation(opcode: u8) -> Result<(Operation, &'static str), DecodeError> {
    use Operation::*;
    let pair = match opcode {
        0x00 => (Sru, "srui"),
        0x01 => (Nor, "nori"),
        0x02 => (Mul, "muli"),
        0x03 => (Sh, "sh"),
        0x04 => (Lb, "lb"),
        0x05 => (Sr, "sri"),
        0x06 => (Xor, "xori"),
        0x07 => (Lh, "lh"),
        0x08 => (And, "andi"),
        0x09 => (Xnor, "xnori"),
        0x0a => (Lw, "lw"),
        0x0b => (Lhu, "lhu"),
        0x0c => (Sb, "sb"),
        0x0d => (Add, "addi"),
        0x0e => (Or, "ori"),
        0x0f => (Sl, "sli"),
        0x10 => (Lbu, "lbu"),
        0x11 => (Be, "be"),
        0x12 => (Bg, "bg"),
        0x13 => (Bge, "bge"),
        0x14 => (Bgeu, "bgeu"),
        0x15 => (Bgu, "bgu"),
        0x16 => (Sw, "sw"),
        0x17 => (Bne, "bne"),
        0x18 => (AndHi, "andhi"),
        0x19 => (Cmpe, "cmpei"),
        0x1a => (Cmpg, "cmpgi"),
        0x1b => (Cmpge, "cmpgei"),
        0x1c => (Cmpgeu, "cmpgeui"),
        0x1d => (Cmpgu, "cmpgui"),
        0x1e => (OrHi, "orhi"),
        0x1f => (Cmpne, "cmpnei"),
        0x20 => (Sru, "sru"),
        0x21 => (Nor, "nor"),
        0x22 => (Mul, "mul"),
        0x23 => (Divu, "divu"),
        0x24 => (Rcsr, "rcsr"),
        0x25 => (Sr, "sr"),
        0x26 => (Xor, "xor"),
        0x27 | 0x35 => return Err(DecodeError::Unsupported),
        0x28 => (And, "and"),
        0x29 => (Xnor, "xnor"),
        0x2a | 0x33 => return Err(DecodeError::Unknown),
        0x2b => return Err(DecodeError::Unknown),
        0x2c => (Sextb, "sextb"),
        0x2d => (Add, "add"),
        0x2e => (Or, "or"),
        0x2f => (Sl, "sl"),
        0x30 => (Branch, "b"),
        0x31 => (Modu, "modu"),
        0x32 => (Sub, "sub"),
        0x34 => (Wcsr, "wcsr"),
        0x36 => (Call, "call"),
        0x37 => (Sexth, "sexth"),
        0x38 => (Bi, "bi"),
        0x39 => (Cmpe, "cmpe"),
        0x3a => (Cmpg, "cmpg"),
        0x3b => (Cmpge, "cmpge"),
        0x3c => (Cmpgeu, "cmpgeu"),
        0x3d => (Cmpgu, "cmpgu"),
        0x3e => (Calli, "calli"),
        0x3f => (Cmpne, "cmpne"),
        _ => return Err(DecodeError::Unknown),
    };
    Ok(pair)
}

/// Decode and validate one big-endian instruction word.
pub fn decode(raw: u32) -> Result<DecodedInstruction, DecodeError> {
    if raw == 0xac00_0002 {
        return Ok(DecodedInstruction {
            operation: Operation::Break,
            mnemonic: "break",
            source_0: 0,
            source_1: 0,
            destination: 0,
            immediate: 0,
            raw,
        });
    }
    if raw == 0xac00_0007 {
        return Ok(DecodedInstruction {
            operation: Operation::Scall,
            mnemonic: "scall",
            source_0: 0,
            source_1: 0,
            destination: 0,
            immediate: 0,
            raw,
        });
    }
    let opcode = (raw >> 26) as u8;
    let (operation, mut mnemonic) = operation(opcode)?;
    let source_0 = ((raw >> 21) & 0x1f) as u8;
    let source_1 = ((raw >> 16) & 0x1f) as u8;
    let destination = if opcode & 0x20 == 0 {
        source_1
    } else {
        ((raw >> 11) & 0x1f) as u8
    };
    let register_form = opcode & 0x20 != 0;
    let immediate = if opcode == 0x38 || opcode == 0x3e {
        sign_extend_26(raw & 0x03ff_ffff)
    } else if matches!(opcode, 0x18 | 0x1e) {
        (raw & 0xffff) << 16
    } else if matches!(opcode, 0x01 | 0x06 | 0x08 | 0x09 | 0x0e | 0x1c | 0x1d) {
        raw & 0xffff
    } else {
        sign_extend_16(raw as u16)
    };

    let reserved_ok = match opcode {
        0x34 => raw & 0xffff == 0,
        // The RTL selects these operations solely from the primary opcode.
        // Their unused source/destination-position bits are architecturally
        // ignored rather than reserved encodings.
        0x24 | 0x2c | 0x37 => true,
        0x30 | 0x36 => raw & 0x001f_ffff == 0,
        0x38 | 0x3e => true,
        0x11..=0x17 | 0x19..=0x1d | 0x1f => true,
        _ if register_form => raw & 0x7ff == 0,
        _ => true,
    };
    if !reserved_ok {
        return Err(DecodeError::Unknown);
    }
    if opcode == 0x30 && source_0 == 30 {
        mnemonic = "eret";
    }
    if opcode == 0x30 && source_0 == 31 {
        mnemonic = "bret";
    }
    Ok(DecodedInstruction {
        operation,
        mnemonic,
        source_0,
        source_1,
        destination,
        immediate,
        raw,
    })
}

/// Return the fixed classification of one primary opcode.
#[must_use]
pub const fn opcode_class(opcode: u8) -> &'static str {
    match opcode {
        0x27 | 0x35 => "unsupported",
        0x2a | 0x33 => "reserved",
        0x2b => "canonical-only",
        0x00..=0x3f => "implemented",
        _ => "out-of-range",
    }
}
