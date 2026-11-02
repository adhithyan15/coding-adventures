//! Fixed-width combinational networks for the bounded System/370 extension.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

/// One-hot extension lines ordered MVCL, CLCL, CLM, STCM, ICM.
#[must_use]
pub fn decode_extension(opcode: u8) -> [u8; 5] {
    let decoded = ibm_system360_gatelevel::gates::decode_byte(opcode);
    [
        decoded[0x0e],
        decoded[0x0f],
        decoded[0xbd],
        decoded[0xbe],
        decoded[0xbf],
    ]
}

#[must_use]
pub fn bits_u24(value: u32) -> [u8; 24] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

#[must_use]
pub fn from_bits(bits: &[u8]) -> u32 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, wire)| value | (u32::from(*wire) << bit))
}

#[must_use]
pub fn add_u24(a: u32, b: u32) -> u32 {
    from_bits(&ripple_carry_adder_with_carry(&bits_u24(a), &bits_u24(b), 0).sum)
}

#[must_use]
pub fn sub_u24(a: u32, b: u32) -> u32 {
    let inverted = bits_u24(b).map(not_gate);
    from_bits(&ripple_carry_adder_with_carry(&bits_u24(a), &inverted, 1).sum)
}

#[must_use]
pub fn nonzero_u24(value: u32) -> u8 {
    bits_u24(value).into_iter().fold(0, or_gate)
}

/// Architectural compare code: zero equal, one low, two high.
#[must_use]
pub fn compare_u24(a: u32, b: u32) -> u8 {
    compare_bits(&bits_u24(a), &bits_u24(b))
}

/// Architectural compare code: zero equal, one low, two high.
#[must_use]
pub fn compare_byte(a: u8, b: u8) -> u8 {
    let left: [u8; 8] = std::array::from_fn(|bit| (a >> bit) & 1);
    let right: [u8; 8] = std::array::from_fn(|bit| (b >> bit) & 1);
    compare_bits(&left, &right)
}

fn compare_bits(left: &[u8], right: &[u8]) -> u8 {
    let mut equal_prefix = 1;
    let mut low = 0;
    let mut high = 0;
    for bit in (0..left.len()).rev() {
        low = or_gate(
            low,
            and_gate(equal_prefix, and_gate(not_gate(left[bit]), right[bit])),
        );
        high = or_gate(
            high,
            and_gate(equal_prefix, and_gate(left[bit], not_gate(right[bit]))),
        );
        equal_prefix = and_gate(equal_prefix, not_gate(xor_gate(left[bit], right[bit])));
    }
    low | (high << 1)
}

#[must_use]
pub fn select_u8(zero: u8, one: u8, select: u8) -> u8 {
    (0..8).fold(0, |value, bit| {
        value | (mux2((zero >> bit) & 1, (one >> bit) & 1, select) << bit)
    })
}

#[must_use]
pub fn select_u24(zero: u32, one: u32, select: u8) -> u32 {
    from_bits(&std::array::from_fn::<_, 24, _>(|bit| {
        mux2(((zero >> bit) & 1) as u8, ((one >> bit) & 1) as u8, select)
    }))
}

#[must_use]
pub fn select_u32(zero: u32, one: u32, select: u8) -> u32 {
    (0..32).fold(0, |value, bit| {
        value
            | (u32::from(mux2(
                ((zero >> bit) & 1) as u8,
                ((one >> bit) & 1) as u8,
                select,
            )) << bit)
    })
}

/// Select one big-endian register byte through a two-level mux tree.
#[must_use]
pub fn select_register_byte(value: u32, byte_index: usize) -> u8 {
    let bytes = value.to_be_bytes();
    let low_pair = select_u8(bytes[0], bytes[1], (byte_index & 1) as u8);
    let high_pair = select_u8(bytes[2], bytes[3], (byte_index & 1) as u8);
    select_u8(low_pair, high_pair, ((byte_index >> 1) & 1) as u8)
}

#[must_use]
pub fn destructive_overlap(destination: u32, source: u32, copied: u32) -> u8 {
    let destination_high = (compare_u24(destination, source) >> 1) & 1;
    let distance = sub_u24(destination, source);
    let distance_low = compare_u24(distance, copied) & 1;
    and_gate(
        nonzero_u24(copied),
        and_gate(destination_high, distance_low),
    )
}

#[must_use]
pub fn mask_enabled(mask: u8, byte_index: usize) -> u8 {
    (mask >> (3 - byte_index)) & 1
}

#[must_use]
pub fn mask_prefix(mask: u8, byte_index: usize) -> u8 {
    (0..byte_index).fold(0, |count, index| {
        let addend = mask_enabled(mask, index);
        let a = [count & 1, (count >> 1) & 1, (count >> 2) & 1];
        let b = [addend, 0, 0];
        from_bits(&ripple_carry_adder_with_carry(&a, &b, 0).sum) as u8
    })
}

#[must_use]
pub fn merge_register_byte(original: u32, byte: u8, byte_index: usize, enable: u8) -> u32 {
    let shift = 24 - byte_index * 8;
    let replacement = (original & !(0xff << shift)) | (u32::from(byte) << shift);
    select_u32(original, replacement, enable)
}

#[must_use]
pub fn leftmost_enabled_byte(mask: u8, bytes: [u8; 4]) -> u8 {
    let mut selected = 0;
    let mut found = 0;
    for (index, byte) in bytes.into_iter().enumerate() {
        let choose = and_gate(mask_enabled(mask, index), not_gate(found));
        selected = select_u8(selected, byte, choose);
        found = or_gate(found, mask_enabled(mask, index));
    }
    selected
}

#[must_use]
pub fn any_enabled_one(mask: u8, bytes: [u8; 4]) -> u8 {
    (0..4).fold(0, |present, index| {
        let any = (0..8).fold(0, |byte_any, bit| {
            or_gate(byte_any, (bytes[index] >> bit) & 1)
        });
        or_gate(present, and_gate(mask_enabled(mask, index), any))
    })
}

#[must_use]
pub fn icm_condition_code(mask: u8, bytes: [u8; 4]) -> u8 {
    let any = any_enabled_one(mask, bytes);
    let sign = (leftmost_enabled_byte(mask, bytes) >> 7) & 1;
    let negative = and_gate(any, sign);
    let positive = and_gate(any, not_gate(sign));
    negative | (positive << 1)
}
