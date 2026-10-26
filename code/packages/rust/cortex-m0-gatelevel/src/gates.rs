//! Fixed-width combinational networks used by the Cortex-M0 transition engine.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

#[must_use]
pub fn u8_to_bits(value: u8) -> [u8; 8] {
    std::array::from_fn(|bit| (value >> bit) & 1)
}

#[must_use]
pub fn u16_to_bits(value: u16) -> [u8; 16] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

#[must_use]
pub fn u32_to_bits(value: u32) -> [u8; 32] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

#[must_use]
pub fn bits_to_u8(bits: &[u8]) -> u8 {
    bits.iter()
        .take(8)
        .enumerate()
        .fold(0, |value, (bit, q)| value | (*q << bit))
}

#[must_use]
pub fn bits_to_u16(bits: &[u8]) -> u16 {
    bits.iter()
        .take(16)
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u16::from(*q) << bit))
}

#[must_use]
pub fn bits_to_u32(bits: &[u8]) -> u32 {
    bits.iter()
        .take(32)
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u32::from(*q) << bit))
}

/// Gate equality against a constant over the selected low-width input.
#[must_use]
pub(crate) fn selected(value: u32, expected: u32, width: usize) -> u8 {
    assert!((1..=32).contains(&width), "selection width must be 1..=32");
    let value = u32_to_bits(value);
    let expected = u32_to_bits(expected);
    (0..width).fold(1, |line, bit| {
        let signal = mux2(not_gate(value[bit]), value[bit], expected[bit]);
        and_gate(line, signal)
    })
}

#[must_use]
pub(crate) fn field(value: u32, offset: usize, width: usize) -> u32 {
    assert!(
        offset <= 32 && width <= 32 && offset.saturating_add(width) <= 32,
        "field must fit in 32 bits"
    );
    let bits = u32_to_bits(value);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|output| {
        if output < width && offset + output < 32 {
            bits[offset + output]
        } else {
            0
        }
    }))
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

/// Result, carry, and signed overflow from a 32-bit ripple adder.
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
pub fn sub_32(left: u32, right: u32, borrow: u8) -> (u32, u8, u8) {
    let (value, carry, overflow) = add_32(left, bit_not_32(right), not_gate(borrow));
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
pub fn is_zero_32(value: u32) -> u8 {
    not_gate(u32_to_bits(value).into_iter().fold(0, or_gate))
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
    let left = u8_to_bits(left);
    let right = u8_to_bits(right);
    (0..8).fold(0, |changed, bit| {
        or_gate(changed, xor_gate(left[bit], right[bit]))
    })
}

#[must_use]
pub fn is_zero_8(value: u8) -> u8 {
    not_gate(u8_to_bits(value).into_iter().fold(0, or_gate))
}

#[must_use]
pub fn shift_left_32(mut value: u32, amount: u32) -> u32 {
    for stage in 0..5 {
        let distance = 1 << stage;
        let bits = u32_to_bits(value);
        let shifted = bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
            if bit >= distance {
                bits[bit - distance]
            } else {
                0
            }
        }));
        value = select_32(value, shifted, u32_to_bits(amount)[stage]);
    }
    value
}

#[must_use]
pub fn shift_right_32(mut value: u32, amount: u32) -> u32 {
    for stage in 0..5 {
        let distance = 1 << stage;
        let bits = u32_to_bits(value);
        let shifted = bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
            if bit + distance < 32 {
                bits[bit + distance]
            } else {
                0
            }
        }));
        value = select_32(value, shifted, u32_to_bits(amount)[stage]);
    }
    value
}

#[must_use]
pub fn shift_arithmetic_right_32(mut value: u32, amount: u32) -> u32 {
    for stage in 0..5 {
        let distance = 1 << stage;
        let bits = u32_to_bits(value);
        let shifted = bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
            if bit + distance < 32 {
                bits[bit + distance]
            } else {
                bits[31]
            }
        }));
        value = select_32(value, shifted, u32_to_bits(amount)[stage]);
    }
    value
}

#[must_use]
pub fn rotate_right_32(mut value: u32, amount: u32) -> u32 {
    for stage in 0..5 {
        let distance = 1 << stage;
        let bits = u32_to_bits(value);
        let shifted = bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
            bits[(bit + distance) & 31]
        }));
        value = select_32(value, shifted, u32_to_bits(amount)[stage]);
    }
    value
}

#[must_use]
pub(crate) fn sign_extend(value: u32, width: usize) -> u32 {
    assert!((1..=32).contains(&width), "sign width must be 1..=32");
    let input = u32_to_bits(value);
    let sign = input[width - 1];
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        if bit < width {
            input[bit]
        } else {
            sign
        }
    }))
}

#[must_use]
pub fn multiply_32(left: u32, right: u32) -> u32 {
    let right = u32_to_bits(right);
    let mut product = 0;
    for (bit, selected) in right.into_iter().enumerate() {
        let partial = select_32(0, shift_left_32(left, bit as u32), selected);
        product = add_32(product, partial, 0).0;
    }
    product
}

#[must_use]
pub fn reverse_bytes(value: u32) -> u32 {
    let input = u32_to_bits(value);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        input[(3 - bit / 8) * 8 + bit % 8]
    }))
}

#[must_use]
pub fn reverse_halfwords(value: u32) -> u32 {
    let input = u32_to_bits(value);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        input[(bit / 16) * 16 + (1 - (bit % 16) / 8) * 8 + bit % 8]
    }))
}

#[must_use]
pub fn popcount_8(value: u8) -> u32 {
    u8_to_bits(value)
        .into_iter()
        .fold(0, |count, bit| add_32(count, u32::from(bit), 0).0)
}
