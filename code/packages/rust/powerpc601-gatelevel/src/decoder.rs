//! Combinational fixed-word PowerPC instruction field decoder.

use logic_gates::gates::{and_gate, or_gate, xnor_gate};

/// One-hot opcode lines consumed by the status networks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusOpcodeWires {
    pub subfic: u8,
    pub andi: u8,
    pub andis: u8,
    pub compare_immediate: u8,
    pub compare_register: u8,
    pub recordable_x: u8,
    pub mtcrf: u8,
    pub addc: u8,
    pub adde: u8,
    pub sraw: u8,
    pub srawi: u8,
}

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
    /// Rc as a one-bit combinational wire.
    pub rc_wire: u8,
    /// XO-form OE as a one-bit combinational wire.
    pub oe_wire: u8,
    /// CR field selector, least-significant selector wire first.
    pub cr_field_wires: [u8; 3],
    /// `mtcrf` mask, least-significant encoded wire first.
    pub mtcrf_mask_wires: [u8; 8],
    /// Gate-decoded status-producing instruction lines.
    pub status: StatusOpcodeWires,
    pub absolute: bool,
    pub link: bool,
}

pub fn decode(raw: u32) -> DecodedInstruction {
    let xo10 = ((raw >> 1) & 0x3ff) as u16;
    let opcode_value = raw >> 26;
    let opcode = |expected| equal_line(opcode_value, 6, expected);
    let primary31 = opcode(31);
    let xo10_line = |expected| and_gate(primary31, equal_line(u32::from(xo10), 10, expected));
    let xo9_line = |expected| and_gate(primary31, equal_line(u32::from(xo10 & 0x1ff), 9, expected));
    let recordable_x = [24, 26, 28, 124, 316, 444, 476, 536, 792, 824]
        .into_iter()
        .fold(0, |line, xo| or_gate(line, xo10_line(xo)));
    DecodedInstruction {
        raw,
        opcode: opcode_value as u8,
        rd: ((raw >> 21) & 0x1f) as usize,
        ra: ((raw >> 16) & 0x1f) as usize,
        rb: ((raw >> 11) & 0x1f) as usize,
        xo10,
        xo9: xo10 & 0x1ff,
        immediate: raw as u16,
        record: raw & 1 != 0,
        rc_wire: (raw & 1) as u8,
        oe_wire: ((raw >> 10) & 1) as u8,
        cr_field_wires: std::array::from_fn(|bit| ((raw >> (23 + bit)) & 1) as u8),
        mtcrf_mask_wires: std::array::from_fn(|bit| ((raw >> (12 + bit)) & 1) as u8),
        status: StatusOpcodeWires {
            subfic: opcode(8),
            andi: opcode(28),
            andis: opcode(29),
            compare_immediate: or_gate(opcode(10), opcode(11)),
            compare_register: or_gate(xo10_line(0), xo10_line(32)),
            recordable_x,
            mtcrf: xo10_line(144),
            addc: xo9_line(10),
            adde: xo9_line(138),
            sraw: xo10_line(792),
            srawi: xo10_line(824),
        },
        absolute: raw & 2 != 0,
        link: raw & 1 != 0,
    }
}

fn equal_line(value: u32, width: usize, expected: u32) -> u8 {
    (0..width).fold(1, |selected, bit| {
        let actual = ((value >> bit) & 1) as u8;
        let wanted = ((expected >> bit) & 1) as u8;
        and_gate(selected, xnor_gate(actual, wanted))
    })
}

pub fn decode_spr(raw: u32) -> u16 {
    let encoded = ((raw >> 11) & 0x3ff) as u16;
    ((encoded & 0x1f) << 5) | (encoded >> 5)
}
