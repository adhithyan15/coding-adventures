//! Structured encoders for the Spec 07bd little-endian transport.

/// Serialize instruction words in architectural little-endian byte order.
pub fn assemble(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

/// Microwatt's exact simulation termination word.
pub const fn attn() -> u32 {
    0x0000_0200
}

pub const fn d_form(opcode: u32, rt: u32, ra: u32, immediate: i32) -> u32 {
    ((opcode & 0x3f) << 26)
        | ((rt & 0x1f) << 21)
        | ((ra & 0x1f) << 16)
        | (immediate as u32 & 0xffff)
}

pub const fn i_form(byte_offset: i32, absolute: bool, link: bool) -> u32 {
    (18 << 26) | (byte_offset as u32 & 0x03ff_fffc) | ((absolute as u32) << 1) | link as u32
}

pub const fn b_form(bo: u32, bi: u32, byte_offset: i32, absolute: bool, link: bool) -> u32 {
    (16 << 26)
        | ((bo & 0x1f) << 21)
        | ((bi & 0x1f) << 16)
        | (byte_offset as u32 & 0x0000_fffc)
        | ((absolute as u32) << 1)
        | link as u32
}

pub const fn x_form(opcode: u32, rs: u32, ra: u32, rb: u32, xo: u32, rc: bool) -> u32 {
    ((opcode & 0x3f) << 26)
        | ((rs & 0x1f) << 21)
        | ((ra & 0x1f) << 16)
        | ((rb & 0x1f) << 11)
        | ((xo & 0x3ff) << 1)
        | rc as u32
}

pub const fn xo_form(rt: u32, ra: u32, rb: u32, oe: bool, xo: u32, rc: bool) -> u32 {
    (31 << 26)
        | ((rt & 0x1f) << 21)
        | ((ra & 0x1f) << 16)
        | ((rb & 0x1f) << 11)
        | ((oe as u32) << 10)
        | ((xo & 0x1ff) << 1)
        | rc as u32
}

pub const fn xl_form(bo: u32, bi: u32, bh: u32, xo: u32, link: bool) -> u32 {
    (19 << 26)
        | ((bo & 0x1f) << 21)
        | ((bi & 0x1f) << 16)
        | ((bh & 3) << 11)
        | ((xo & 0x3ff) << 1)
        | link as u32
}

pub const fn xfx_form(rs: u32, spr: u32, xo: u32) -> u32 {
    let encoded = ((spr & 0x1f) << 5) | ((spr >> 5) & 0x1f);
    (31 << 26) | ((rs & 0x1f) << 21) | (encoded << 11) | ((xo & 0x3ff) << 1)
}

pub const fn sync(l: u32) -> u32 {
    (31 << 26) | ((l & 3) << 21) | (598 << 1)
}
