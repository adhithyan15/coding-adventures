//! Structured big-endian encoders for the Spec 07z Apple M1 teaching surface.

pub use aarch64_simulator::encoding::*;

/// Scalar FP data processing with one source.
#[must_use]
pub const fn fp_one_source(ftype: u32, opcode: u32, rn: u32, rd: u32) -> u32 {
    (0b000_11110 << 24)
        | ((ftype & 3) << 22)
        | (1 << 21)
        | ((opcode & 0x3f) << 15)
        | (0b10000 << 10)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}

/// Scalar FP data processing with two sources.
#[must_use]
pub const fn fp_two_source(ftype: u32, rm: u32, opcode: u32, rn: u32, rd: u32) -> u32 {
    (0b000_11110 << 24)
        | ((ftype & 3) << 22)
        | (1 << 21)
        | ((rm & 0x1f) << 16)
        | ((opcode & 0xf) << 12)
        | (0b10 << 10)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}

/// Scalar FP compare, including the `opc=3` compare-with-zero form.
#[must_use]
pub const fn fp_compare(ftype: u32, rm: u32, rn: u32, opc: u32) -> u32 {
    (0b000_11110 << 24)
        | ((ftype & 3) << 22)
        | (1 << 21)
        | ((rm & 0x1f) << 16)
        | (0b001000 << 10)
        | ((rn & 0x1f) << 5)
        | (opc & 7)
}

/// Move raw GPR bits to an FP register (`to_fp=true`) or back.
#[must_use]
pub const fn fp_gpr_move(double: bool, to_fp: bool, rn: u32, rd: u32) -> u32 {
    ((double as u32) << 31)
        | (0b00_11110 << 24)
        | ((double as u32) << 22)
        | (1 << 21)
        | ((if to_fp { 0b00111 } else { 0b00110 }) << 16)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}

/// Convert FP to signed integer, truncating toward zero.
#[must_use]
pub const fn fp_to_signed(sf: u32, ftype: u32, rn: u32, rd: u32) -> u32 {
    ((sf & 1) << 31)
        | (0b00_11110 << 24)
        | ((ftype & 3) << 22)
        | (1 << 21)
        | (0b11000 << 16)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}

/// Convert signed (`signed=true`) or unsigned integer to FP.
#[must_use]
pub const fn integer_to_fp(sf: u32, ftype: u32, signed: bool, rn: u32, rd: u32) -> u32 {
    ((sf & 1) << 31)
        | (0b00_11110 << 24)
        | ((ftype & 3) << 22)
        | (1 << 21)
        | ((if signed { 0b00010 } else { 0b00011 }) << 16)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}

/// FP load/store with unsigned scaled offset.
#[must_use]
pub const fn fp_load_store(size: u32, load: bool, offset: u32, rn: u32, rt: u32) -> u32 {
    ((size & 3) << 30)
        | (0b111 << 27)
        | (1 << 26)
        | (0b01 << 24)
        | ((load as u32) << 22)
        | ((offset & 0xfff) << 10)
        | ((rn & 0x1f) << 5)
        | (rt & 0x1f)
}

/// AdvSIMD three-register-same encoding.
#[must_use]
pub const fn neon_three_same(
    q: bool,
    unsigned: bool,
    size: u32,
    rm: u32,
    opcode: u32,
    rn: u32,
    rd: u32,
) -> u32 {
    ((q as u32) << 30)
        | ((unsigned as u32) << 29)
        | (0b01110 << 24)
        | ((size & 3) << 22)
        | (1 << 21)
        | ((rm & 0x1f) << 16)
        | ((opcode & 0x1f) << 11)
        | (1 << 10)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}

/// AdvSIMD duplicate-from-GPR encoding.
#[must_use]
pub const fn neon_duplicate(q: bool, imm5: u32, rn: u32, rd: u32) -> u32 {
    ((q as u32) << 30)
        | (0b01110 << 24)
        | ((imm5 & 0x1f) << 19)
        | (0b00001 << 14)
        | (1 << 13)
        | (1 << 10)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}
