//! Fixed-width combinational networks used by the Cortex-M3 transition engine.

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

/// Sign-extend one byte through a fixed 32-bit gate fan-out.
#[must_use]
pub fn sign_extend_8(value: u8) -> u32 {
    sign_extend(u32::from(value), 8)
}

/// Sign-extend one halfword through a fixed 32-bit gate fan-out.
#[must_use]
pub fn sign_extend_16(value: u16) -> u32 {
    sign_extend(u32::from(value), 16)
}

/// Unsigned less-than from the borrow output of a ripple subtractor.
#[must_use]
pub fn unsigned_less_32(left: u32, right: u32) -> u8 {
    sub_32(left, right, 0).1
}

/// Reverse all bits through a fixed permutation network.
#[must_use]
pub fn reverse_bits_32(value: u32) -> u32 {
    let input = u32_to_bits(value);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| input[31 - bit]))
}

/// Count leading zeroes with a gate-built priority scan.
#[must_use]
pub fn leading_zeros_32(value: u32) -> u32 {
    let input = u32_to_bits(value);
    let mut seen = 0;
    let mut count = 0;
    for bit in input.into_iter().rev() {
        let count_this = and_gate(not_gate(seen), not_gate(bit));
        count = add_32(count, u32::from(count_this), 0).0;
        seen = or_gate(seen, bit);
    }
    count
}

/// Unsigned quotient from a restoring 33-bit divide network.
#[must_use]
pub fn divide_unsigned_32(dividend: u32, divisor: u32) -> u32 {
    let dividend_bits = u32_to_bits(dividend);
    let divisor_bits = u32_to_bits(divisor);
    let mut remainder = [0_u8; 33];
    let mut quotient = [0_u8; 32];
    let mut extended_divisor = [0_u8; 33];
    extended_divisor[..32].copy_from_slice(&divisor_bits);
    for bit in (0..32).rev() {
        for position in (1..33).rev() {
            remainder[position] = remainder[position - 1];
        }
        remainder[0] = dividend_bits[bit];
        let inverted = extended_divisor.map(not_gate);
        let difference = ripple_carry_adder_with_carry(&remainder, &inverted, 1);
        quotient[bit] = difference.carry_out;
        remainder = std::array::from_fn(|position| {
            mux2(
                remainder[position],
                difference.sum[position],
                difference.carry_out,
            )
        });
    }
    bits_to_u32(&quotient)
}

/// Signed quotient with Arm's zero-divisor and MIN/-1 wrapping behavior.
#[must_use]
pub fn divide_signed_32(dividend: u32, divisor: u32) -> u32 {
    let dividend_sign = u32_to_bits(dividend)[31];
    let divisor_sign = u32_to_bits(divisor)[31];
    let dividend_abs = select_32(dividend, sub_32(0, dividend, 0).0, dividend_sign);
    let divisor_abs = select_32(divisor, sub_32(0, divisor, 0).0, divisor_sign);
    let magnitude = divide_unsigned_32(dividend_abs, divisor_abs);
    select_32(
        magnitude,
        sub_32(0, magnitude, 0).0,
        xor_gate(dividend_sign, divisor_sign),
    )
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
