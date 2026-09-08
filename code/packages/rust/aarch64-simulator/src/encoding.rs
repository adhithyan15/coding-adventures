//! Structured big-endian encoders for the documented AArch64 teaching surface.

/// Assemble fixed-width words into the simulator's big-endian transport.
#[must_use]
pub fn program(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_be_bytes()).collect()
}

#[must_use]
pub const fn halt() -> u32 {
    0
}

#[must_use]
pub const fn nop() -> u32 {
    0xd503_201f
}

#[must_use]
pub const fn add_sub_immediate(
    sf: u32,
    op: u32,
    set_flags: u32,
    immediate: u32,
    shift: u32,
    rn: u32,
    rd: u32,
) -> u32 {
    ((sf & 1) << 31)
        | ((op & 1) << 30)
        | ((set_flags & 1) << 29)
        | (0b100000 << 23)
        | ((shift & 1) << 22)
        | ((immediate & 0xfff) << 10)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}

#[must_use]
pub const fn add_sub_register(
    sf: u32,
    op: u32,
    set_flags: u32,
    shift: (u32, u32),
    rm: u32,
    rn: u32,
    rd: u32,
) -> u32 {
    ((sf & 1) << 31)
        | ((op & 1) << 30)
        | ((set_flags & 1) << 29)
        | (0b01011 << 24)
        | ((shift.0 & 3) << 22)
        | ((rm & 0x1f) << 16)
        | ((shift.1 & 0x3f) << 10)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}

#[must_use]
pub const fn logical_immediate(
    sf: u32,
    opcode: u32,
    n: u32,
    rotation: u32,
    size: u32,
    rn: u32,
    rd: u32,
) -> u32 {
    ((sf & 1) << 31)
        | ((opcode & 3) << 29)
        | (0b100100 << 22)
        | ((n & 1) << 22)
        | ((rotation & 0x3f) << 16)
        | ((size & 0x3f) << 10)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}

#[must_use]
pub const fn logical_register(
    sf: u32,
    opcode: u32,
    shift: (u32, u32),
    invert: u32,
    rm: u32,
    rn: u32,
    rd: u32,
) -> u32 {
    ((sf & 1) << 31)
        | ((opcode & 3) << 29)
        | (0b01010 << 24)
        | ((shift.0 & 3) << 22)
        | ((invert & 1) << 21)
        | ((rm & 0x1f) << 16)
        | ((shift.1 & 0x3f) << 10)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}

#[must_use]
pub const fn move_wide(sf: u32, opcode: u32, halfword: u32, immediate: u32, rd: u32) -> u32 {
    ((sf & 1) << 31)
        | ((opcode & 3) << 29)
        | (0b100101 << 23)
        | ((halfword & 3) << 21)
        | ((immediate & 0xffff) << 5)
        | (rd & 0x1f)
}

#[must_use]
pub const fn load_store_unsigned(
    size: u32,
    vector: u32,
    opcode: u32,
    offset: u32,
    rn: u32,
    rt: u32,
) -> u32 {
    ((size & 3) << 30)
        | (0b111 << 27)
        | ((vector & 1) << 26)
        | (0b01 << 24)
        | ((opcode & 3) << 22)
        | ((offset & 0xfff) << 10)
        | ((rn & 0x1f) << 5)
        | (rt & 0x1f)
}

#[must_use]
pub const fn branch(link: bool, displacement_words: i32) -> u32 {
    ((link as u32) << 31) | (0b00101 << 26) | (displacement_words as u32 & 0x03ff_ffff)
}

#[must_use]
pub const fn branch_conditional(displacement_words: i32, condition: u32) -> u32 {
    (0b0101_0100 << 24) | ((displacement_words as u32 & 0x7ffff) << 5) | (condition & 0xf)
}

#[must_use]
pub const fn compare_branch(sf: u32, nonzero: bool, displacement_words: i32, rt: u32) -> u32 {
    ((sf & 1) << 31)
        | (0b011010 << 25)
        | ((nonzero as u32) << 24)
        | ((displacement_words as u32 & 0x7ffff) << 5)
        | (rt & 0x1f)
}

#[must_use]
pub const fn test_branch(bit: u32, nonzero: bool, displacement_words: i32, rt: u32) -> u32 {
    (((bit >> 5) & 1) << 31)
        | (0b011011 << 25)
        | ((nonzero as u32) << 24)
        | ((bit & 0x1f) << 19)
        | ((displacement_words as u32 & 0x3fff) << 5)
        | (rt & 0x1f)
}

#[must_use]
pub const fn branch_register(operation: u32, rn: u32) -> u32 {
    (0b1101_0110 << 24) | ((operation & 7) << 21) | (0x1f << 16) | ((rn & 0x1f) << 5)
}

#[must_use]
pub const fn data_two_source(sf: u32, rm: u32, operation: u32, rn: u32, rd: u32) -> u32 {
    ((sf & 1) << 31)
        | (0b1101_0110 << 21)
        | ((rm & 0x1f) << 16)
        | ((operation & 0x3f) << 10)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}

#[must_use]
pub const fn data_one_source(sf: u32, operation: u32, rn: u32, rd: u32) -> u32 {
    ((sf & 1) << 31)
        | (1 << 30)
        | (0b1101_0110 << 21)
        | ((operation & 0x3f) << 10)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}

#[must_use]
pub const fn multiply_add(
    sf: u32,
    operation: u32,
    rm: u32,
    subtract: bool,
    ra: u32,
    rn: u32,
    rd: u32,
) -> u32 {
    ((sf & 1) << 31)
        | (0b00_11011 << 24)
        | ((operation & 7) << 21)
        | ((rm & 0x1f) << 16)
        | ((subtract as u32) << 15)
        | ((ra & 0x1f) << 10)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}

#[must_use]
pub const fn conditional_select(
    sf: u32,
    invert: bool,
    rm: u32,
    condition: u32,
    increment: bool,
    rn: u32,
    rd: u32,
) -> u32 {
    ((sf & 1) << 31)
        | ((invert as u32) << 30)
        | (0b1101_0100 << 21)
        | ((rm & 0x1f) << 16)
        | ((condition & 0xf) << 12)
        | ((increment as u32) << 10)
        | ((rn & 0x1f) << 5)
        | (rd & 0x1f)
}

#[must_use]
pub const fn svc(immediate: u32) -> u32 {
    (0b110_1010_0000 << 21) | ((immediate & 0xffff) << 5) | 1
}
