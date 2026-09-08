//! Combinational fixed-word Alpha instruction field decoder.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub raw: u32,
    pub op: u8,
    pub ra: usize,
    pub rb: usize,
    pub rc: usize,
    pub function: u8,
    pub literal: Option<u8>,
    pub displacement16: i16,
    pub displacement21: i32,
    pub jump_function: u8,
    pub palcode: u32,
}

pub fn decode(raw: u32) -> DecodedInstruction {
    let displacement = raw & 0x1f_ffff;
    let displacement21 = if displacement & 0x10_0000 != 0 {
        (displacement | 0xffe0_0000) as i32
    } else {
        displacement as i32
    };
    DecodedInstruction {
        raw,
        op: (raw >> 26) as u8,
        ra: ((raw >> 21) & 0x1f) as usize,
        rb: ((raw >> 16) & 0x1f) as usize,
        rc: (raw & 0x1f) as usize,
        function: ((raw >> 5) & 0x7f) as u8,
        literal: (raw & (1 << 12) != 0).then_some(((raw >> 13) & 0xff) as u8),
        displacement16: raw as u16 as i16,
        displacement21,
        jump_function: ((raw >> 14) & 3) as u8,
        palcode: raw & 0x03ff_ffff,
    }
}
