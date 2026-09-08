//! Structured big-endian PowerPC 601 instruction encoders.

pub fn assemble(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_be_bytes()).collect()
}

pub const fn halt() -> u32 {
    0
}

pub const fn i_form(opcode: u32, byte_offset: i32, absolute: bool, link: bool) -> u32 {
    ((opcode & 0x3f) << 26)
        | ((((byte_offset >> 2) as u32) & 0x00ff_ffff) << 2)
        | ((absolute as u32) << 1)
        | link as u32
}

pub const fn b_form(
    opcode: u32,
    bo: u32,
    bi: u32,
    byte_offset: i32,
    absolute: bool,
    link: bool,
) -> u32 {
    ((opcode & 0x3f) << 26)
        | ((bo & 0x1f) << 21)
        | ((bi & 0x1f) << 16)
        | ((((byte_offset >> 2) as u32) & 0x3fff) << 2)
        | ((absolute as u32) << 1)
        | link as u32
}

pub const fn d_form(opcode: u32, rd: u32, ra: u32, immediate: i32) -> u32 {
    ((opcode & 0x3f) << 26)
        | ((rd & 0x1f) << 21)
        | ((ra & 0x1f) << 16)
        | (immediate as u32 & 0xffff)
}

pub const fn x_form(opcode: u32, rs: u32, ra: u32, rb: u32, xo: u32, record: bool) -> u32 {
    ((opcode & 0x3f) << 26)
        | ((rs & 0x1f) << 21)
        | ((ra & 0x1f) << 16)
        | ((rb & 0x1f) << 11)
        | ((xo & 0x3ff) << 1)
        | record as u32
}

pub const fn xo_form(
    opcode: u32,
    rd: u32,
    ra: u32,
    rb: u32,
    overflow: bool,
    xo: u32,
    record: bool,
) -> u32 {
    ((opcode & 0x3f) << 26)
        | ((rd & 0x1f) << 21)
        | ((ra & 0x1f) << 16)
        | ((rb & 0x1f) << 11)
        | ((overflow as u32) << 10)
        | ((xo & 0x1ff) << 1)
        | record as u32
}

pub const fn xfx_form(opcode: u32, rs: u32, spr: u32, xo: u32) -> u32 {
    let encoded_spr = ((spr & 0x1f) << 5) | ((spr >> 5) & 0x1f);
    ((opcode & 0x3f) << 26) | ((rs & 0x1f) << 21) | (encoded_spr << 11) | ((xo & 0x3ff) << 1)
}

pub const fn xl_form(opcode: u32, bo: u32, bi: u32, bh: u32, xo: u32, link: bool) -> u32 {
    ((opcode & 0x3f) << 26)
        | ((bo & 0x1f) << 21)
        | ((bi & 0x1f) << 16)
        | ((bh & 0x1f) << 11)
        | ((xo & 0x3ff) << 1)
        | link as u32
}
