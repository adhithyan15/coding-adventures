//! Fixed-width OpenRISC combinational networks.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

#[must_use]
pub fn u8_to_bits(value: u8) -> [u8; 8] {
    std::array::from_fn(|bit| (value >> bit) & 1)
}

#[must_use]
pub fn u32_to_bits(value: u32) -> [u8; 32] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

#[must_use]
pub fn bits_to_u8(bits: &[u8]) -> u8 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (*q << bit))
}

#[must_use]
pub fn bits_to_u32(bits: &[u8]) -> u32 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u32::from(*q) << bit))
}

/// Six-to-sixty-four one-hot decoder used by primary instruction dispatch.
#[must_use]
pub fn decode_primary(opcode: u8) -> [u8; 64] {
    let inputs: [u8; 6] = std::array::from_fn(|bit| (opcode >> bit) & 1);
    std::array::from_fn(|line| {
        (0..6).fold(1, |selected, bit| {
            let signal = if (line >> bit) & 1 == 1 {
                inputs[bit]
            } else {
                not_gate(inputs[bit])
            };
            and_gate(selected, signal)
        })
    })
}

#[must_use]
pub fn primary_is_implemented(opcode: u8) -> u8 {
    let lines = decode_primary(opcode);
    [
        0x00_u8, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x08, 0x09, 0x11, 0x12, 0x13, 0x1b, 0x21,
        0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30,
        0x31, 0x33, 0x35, 0x36, 0x37, 0x38, 0x39,
    ]
    .into_iter()
    .fold(0, |selected, line| or_gate(selected, lines[line as usize]))
}

#[must_use]
pub fn primary_is_unsupported(opcode: u8) -> u8 {
    let lines = decode_primary(opcode);
    [
        0x0a_u8, 0x1a, 0x20, 0x32, 0x1c, 0x1d, 0x1e, 0x1f, 0x3c, 0x3d, 0x3e, 0x3f,
    ]
    .into_iter()
    .fold(0, |selected, line| or_gate(selected, lines[line as usize]))
}

#[must_use]
pub fn bit_not_32(value: u32) -> u32 {
    bits_to_u32(&u32_to_bits(value).map(not_gate))
}

#[must_use]
pub fn bit_and_32(left: u32, right: u32) -> u32 {
    let left = u32_to_bits(left);
    let right = u32_to_bits(right);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        and_gate(left[bit], right[bit])
    }))
}

#[must_use]
pub fn bit_or_32(left: u32, right: u32) -> u32 {
    let left = u32_to_bits(left);
    let right = u32_to_bits(right);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        or_gate(left[bit], right[bit])
    }))
}

#[must_use]
pub fn bit_xor_32(left: u32, right: u32) -> u32 {
    let left = u32_to_bits(left);
    let right = u32_to_bits(right);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        xor_gate(left[bit], right[bit])
    }))
}

/// Result, carry, and signed-overflow from a 32-bit ripple adder.
#[must_use]
pub fn add_32(left: u32, right: u32, carry_in: u8) -> (u32, u8, u8) {
    let left_bits = u32_to_bits(left);
    let right_bits = u32_to_bits(right);
    let full = ripple_carry_adder_with_carry(&left_bits, &right_bits, carry_in);
    let low = ripple_carry_adder_with_carry(&left_bits[..31], &right_bits[..31], carry_in);
    (
        bits_to_u32(&full.sum),
        full.carry_out,
        xor_gate(low.carry_out, full.carry_out),
    )
}

#[must_use]
pub fn sub_32(left: u32, right: u32) -> (u32, u8, u8) {
    let (value, carry, overflow) = add_32(left, bit_not_32(right), 1);
    (value, not_gate(carry), overflow)
}

#[must_use]
pub fn equal_32(left: u32, right: u32) -> u8 {
    not_gate(
        u32_to_bits(bit_xor_32(left, right))
            .into_iter()
            .fold(0, or_gate),
    )
}

#[must_use]
pub fn select_32(d0: u32, d1: u32, select: u8) -> u32 {
    let d0 = u32_to_bits(d0);
    let d1 = u32_to_bits(d1);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        mux2(d0[bit], d1[bit], select)
    }))
}

#[must_use]
pub fn byte_changed(left: u8, right: u8) -> u8 {
    u8_to_bits(bits_to_u8(&std::array::from_fn::<_, 8, _>(|bit| {
        xor_gate((left >> bit) & 1, (right >> bit) & 1)
    })))
    .into_iter()
    .fold(0, or_gate)
}

#[must_use]
pub fn is_zero_8(value: u8) -> u8 {
    not_gate(u8_to_bits(value).into_iter().fold(0, or_gate))
}

/// Shift through a five-stage mux network.
#[must_use]
pub fn shift_left_32(mut value: u32, amount: u32) -> u32 {
    for stage in 0..5 {
        let shifted = value.wrapping_shl(1 << stage);
        value = select_32(value, shifted, ((amount >> stage) & 1) as u8);
    }
    value
}

#[must_use]
pub fn shift_right_32(mut value: u32, amount: u32) -> u32 {
    for stage in 0..5 {
        let shifted = value.wrapping_shr(1 << stage);
        value = select_32(value, shifted, ((amount >> stage) & 1) as u8);
    }
    value
}
