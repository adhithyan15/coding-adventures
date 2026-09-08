//! Combinational fixed-word PowerPC instruction field decoder.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub raw: u32,
    pub opcode: u8,
    pub rd: usize,
    pub ra: usize,
    pub rb: usize,
    pub xo10: u16,
    pub xo9: u16,
    pub immediate: u16,
    pub record: bool,
    pub absolute: bool,
    pub link: bool,
}

pub fn decode(raw: u32) -> DecodedInstruction {
    let xo10 = ((raw >> 1) & 0x3ff) as u16;
    DecodedInstruction {
        raw,
        opcode: (raw >> 26) as u8,
        rd: ((raw >> 21) & 0x1f) as usize,
        ra: ((raw >> 16) & 0x1f) as usize,
        rb: ((raw >> 11) & 0x1f) as usize,
        xo10,
        xo9: xo10 & 0x1ff,
        immediate: raw as u16,
        record: raw & 1 != 0,
        absolute: raw & 2 != 0,
        link: raw & 1 != 0,
    }
}

pub fn decode_spr(raw: u32) -> u16 {
    let encoded = ((raw >> 11) & 0x3ff) as u16;
    ((encoded & 0x1f) << 5) | (encoded >> 5)
}
