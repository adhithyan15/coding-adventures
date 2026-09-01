//! Structured encoders for the documented Spec 07x Thumb-2 surface.

/// One 16- or 32-bit Thumb-2 instruction in fetch-order halfwords.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    Thumb16(u16),
    Thumb32(u16, u16),
}

impl Instruction {
    /// Append the little-endian fetch bytes for this instruction.
    pub fn append_to(self, output: &mut Vec<u8>) {
        match self {
            Self::Thumb16(value) => output.extend_from_slice(&value.to_le_bytes()),
            Self::Thumb32(first, second) => {
                output.extend_from_slice(&first.to_le_bytes());
                output.extend_from_slice(&second.to_le_bytes());
            }
        }
    }
}

/// Assemble instructions into loadable fetch bytes.
#[must_use]
pub fn program(instructions: &[Instruction]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(instructions.len() * 2);
    for instruction in instructions {
        instruction.append_to(&mut bytes);
    }
    bytes
}

/// Simulator halt sentinel (`0x0000`).
#[must_use]
pub const fn halt() -> Instruction {
    Instruction::Thumb16(0)
}

/// `MOVS Rd, #imm8`.
#[must_use]
pub const fn mov_imm8(rd: u8, immediate: u8) -> Instruction {
    Instruction::Thumb16(0x2000 | (((rd as u16) & 7) << 8) | immediate as u16)
}

/// `CMP Rn, #imm8`.
#[must_use]
pub const fn cmp_imm8(rn: u8, immediate: u8) -> Instruction {
    Instruction::Thumb16(0x2800 | (((rn as u16) & 7) << 8) | immediate as u16)
}

/// `ADDS Rd, #imm8`.
#[must_use]
pub const fn add_imm8(rd: u8, immediate: u8) -> Instruction {
    Instruction::Thumb16(0x3000 | (((rd as u16) & 7) << 8) | immediate as u16)
}

/// `SUBS Rd, #imm8`.
#[must_use]
pub const fn sub_imm8(rd: u8, immediate: u8) -> Instruction {
    Instruction::Thumb16(0x3800 | (((rd as u16) & 7) << 8) | immediate as u16)
}

/// Register data-processing instruction selected by its four-bit operation.
#[must_use]
pub const fn data_register(operation: u8, rdn: u8, rm: u8) -> Instruction {
    Instruction::Thumb16(
        0x4000 | (((operation as u16) & 0xf) << 6) | (((rm as u16) & 7) << 3) | ((rdn as u16) & 7),
    )
}

/// Conditional branch using a signed halfword displacement.
#[must_use]
pub const fn branch_cond(condition: u8, displacement_halfwords: i8) -> Instruction {
    Instruction::Thumb16(
        0xd000 | (((condition as u16) & 0xf) << 8) | displacement_halfwords as u8 as u16,
    )
}

/// Unconditional branch using an eleven-bit signed halfword displacement.
#[must_use]
pub const fn branch(displacement_halfwords: i16) -> Instruction {
    Instruction::Thumb16(0xe000 | ((displacement_halfwords as u16) & 0x07ff))
}

/// `BL` with a signed byte displacement relative to the next instruction.
#[must_use]
pub const fn branch_link(displacement_bytes: i32) -> Instruction {
    let raw = (displacement_bytes as u32) & 0x01ff_ffff;
    let s = (raw >> 24) & 1;
    let i1 = (raw >> 23) & 1;
    let i2 = (raw >> 22) & 1;
    let j1 = (!(i1 ^ s)) & 1;
    let j2 = (!(i2 ^ s)) & 1;
    let first = 0xf000 | ((s as u16) << 10) | ((raw >> 13) as u16 & 0x03ff);
    let second = 0xd000 | ((j1 as u16) << 13) | ((j2 as u16) << 11) | ((raw >> 1) as u16 & 0x07ff);
    Instruction::Thumb32(first, second)
}

/// `MOVW Rd, #imm16`.
#[must_use]
pub const fn movw(rd: u8, immediate: u16) -> Instruction {
    let first =
        0xf240 | ((((immediate as u32) >> 11) as u16 & 1) << 10) | ((immediate >> 12) & 0xf);
    let second = (((immediate >> 8) & 7) << 12) | (((rd as u16) & 0xf) << 8) | (immediate & 0xff);
    Instruction::Thumb32(first, second)
}

/// `MOVT Rd, #imm16`.
#[must_use]
pub const fn movt(rd: u8, immediate: u16) -> Instruction {
    let first =
        0xf2c0 | ((((immediate as u32) >> 11) as u16 & 1) << 10) | ((immediate >> 12) & 0xf);
    let second = (((immediate >> 8) & 7) << 12) | (((rd as u16) & 0xf) << 8) | (immediate & 0xff);
    Instruction::Thumb32(first, second)
}

/// Construct a raw 32-bit Thumb-2 instruction from fetch-order halfwords.
#[must_use]
pub const fn raw32(first: u16, second: u16) -> Instruction {
    Instruction::Thumb32(first, second)
}
