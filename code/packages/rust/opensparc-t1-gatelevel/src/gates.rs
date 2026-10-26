//! Fixed-width OpenSPARC T1 combinational networks.

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
pub fn u64_to_bits(value: u64) -> [u8; 64] {
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
pub fn bits_to_u32(bits: &[u8]) -> u32 {
    bits.iter()
        .take(32)
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u32::from(*q) << bit))
}

#[must_use]
pub fn bits_to_u64(bits: &[u8]) -> u64 {
    bits.iter()
        .take(64)
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u64::from(*q) << bit))
}

#[must_use]
fn selected(value: u32, expected: u32, width: usize) -> u8 {
    let value = u32_to_bits(value);
    let expected = u32_to_bits(expected);
    (0..width).fold(1, |line, bit| {
        let signal = if expected[bit] == 1 {
            value[bit]
        } else {
            not_gate(value[bit])
        };
        and_gate(line, signal)
    })
}

/// Two-to-four decoder for SPARC's primary format field.
#[must_use]
pub fn decode_format(word: u32) -> [u8; 4] {
    let inputs = [((word >> 30) & 1) as u8, ((word >> 31) & 1) as u8];
    std::array::from_fn(|line| {
        (0..2).fold(1, |active, bit| {
            let signal = if (line >> bit) & 1 == 1 {
                inputs[bit]
            } else {
                not_gate(inputs[bit])
            };
            and_gate(active, signal)
        })
    })
}

/// Six-to-sixty-four decoder for format-3 opcodes.
#[must_use]
pub fn decode_op3(word: u32) -> [u8; 64] {
    let op3 = (word >> 19) & 0x3f;
    std::array::from_fn(|line| selected(op3, line as u32, 6))
}

#[must_use]
pub fn bit_not_64(value: u64) -> u64 {
    bits_to_u64(&u64_to_bits(value).map(not_gate))
}

#[must_use]
pub fn bit_not_32(value: u32) -> u32 {
    bits_to_u32(&u32_to_bits(value).map(not_gate))
}

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
pub fn sub_32(left: u32, right: u32, borrow_in: u8) -> (u32, u8, u8) {
    let (value, carry, overflow) = add_32(left, bit_not_32(right), not_gate(borrow_in));
    (value, not_gate(carry), overflow)
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
pub fn bit_and_64(left: u64, right: u64) -> u64 {
    let left = u64_to_bits(left);
    let right = u64_to_bits(right);
    bits_to_u64(&std::array::from_fn::<_, 64, _>(|bit| {
        and_gate(left[bit], right[bit])
    }))
}

#[must_use]
pub fn bit_or_64(left: u64, right: u64) -> u64 {
    let left = u64_to_bits(left);
    let right = u64_to_bits(right);
    bits_to_u64(&std::array::from_fn::<_, 64, _>(|bit| {
        or_gate(left[bit], right[bit])
    }))
}

#[must_use]
pub fn bit_xor_64(left: u64, right: u64) -> u64 {
    let left = u64_to_bits(left);
    let right = u64_to_bits(right);
    bits_to_u64(&std::array::from_fn::<_, 64, _>(|bit| {
        xor_gate(left[bit], right[bit])
    }))
}

/// Result, carry, and signed overflow from a 64-bit ripple adder.
#[must_use]
pub fn add_64(left: u64, right: u64, carry_in: u8) -> (u64, u8, u8) {
    let left_bits = u64_to_bits(left);
    let right_bits = u64_to_bits(right);
    let full = ripple_carry_adder_with_carry(&left_bits, &right_bits, carry_in);
    let low = ripple_carry_adder_with_carry(&left_bits[..63], &right_bits[..63], carry_in);
    (
        bits_to_u64(&full.sum),
        full.carry_out,
        xor_gate(low.carry_out, full.carry_out),
    )
}

#[must_use]
pub fn sub_64(left: u64, right: u64, borrow_in: u8) -> (u64, u8, u8) {
    let (value, carry, overflow) = add_64(left, bit_not_64(right), not_gate(borrow_in));
    (value, not_gate(carry), overflow)
}

#[must_use]
pub fn select_64(d0: u64, d1: u64, select: u8) -> u64 {
    let d0 = u64_to_bits(d0);
    let d1 = u64_to_bits(d1);
    bits_to_u64(&std::array::from_fn::<_, 64, _>(|bit| {
        mux2(d0[bit], d1[bit], select)
    }))
}

#[must_use]
pub fn equal_64(left: u64, right: u64) -> u8 {
    not_gate(
        u64_to_bits(bit_xor_64(left, right))
            .into_iter()
            .fold(0, or_gate),
    )
}

#[must_use]
pub fn is_zero_32(value: u32) -> u8 {
    not_gate(u32_to_bits(value).into_iter().fold(0, or_gate))
}

#[must_use]
pub fn is_zero_64(value: u64) -> u8 {
    not_gate(u64_to_bits(value).into_iter().fold(0, or_gate))
}

#[must_use]
pub fn unsigned_less_64(left: u64, right: u64) -> u8 {
    sub_64(left, right, 0).1
}

#[must_use]
pub fn signed_less_64(left: u64, right: u64) -> u8 {
    let left_sign = u64_to_bits(left)[63];
    let right_sign = u64_to_bits(right)[63];
    let signs_differ = xor_gate(left_sign, right_sign);
    let (difference, _, overflow) = sub_64(left, right, 0);
    let same_sign_result = xor_gate(u64_to_bits(difference)[63], overflow);
    mux2(same_sign_result, left_sign, signs_differ)
}

#[must_use]
pub fn shift_left_64(mut value: u64, amount: u32) -> u64 {
    for stage in 0..6 {
        let shifted = value.wrapping_shl(1 << stage);
        value = select_64(value, shifted, ((amount >> stage) & 1) as u8);
    }
    value
}

#[must_use]
pub fn shift_right_64(mut value: u64, amount: u32) -> u64 {
    for stage in 0..6 {
        let shifted = value.wrapping_shr(1 << stage);
        value = select_64(value, shifted, ((amount >> stage) & 1) as u8);
    }
    value
}

#[must_use]
pub fn shift_arithmetic_right_64(mut value: u64, amount: u32) -> u64 {
    for stage in 0..6 {
        let distance = 1 << stage;
        let bits = u64_to_bits(value);
        let shifted = bits_to_u64(&std::array::from_fn::<_, 64, _>(|bit| {
            if bit + distance < 64 {
                bits[bit + distance]
            } else {
                bits[63]
            }
        }));
        value = select_64(value, shifted, ((amount >> stage) & 1) as u8);
    }
    value
}

#[must_use]
pub(crate) fn sign_extend(value: u64, width: usize) -> u64 {
    let input = u64_to_bits(value);
    let sign = input[width - 1];
    bits_to_u64(&std::array::from_fn::<_, 64, _>(|bit| {
        if bit < width {
            input[bit]
        } else {
            sign
        }
    }))
}

#[must_use]
pub fn multiply_unsigned_64(left: u64, right: u64) -> u64 {
    let left = u64_to_bits(left);
    let right = u64_to_bits(right);
    let mut product = 0_u64;
    for (bit, selected) in right.into_iter().enumerate() {
        let partial = bits_to_u64(&std::array::from_fn::<_, 64, _>(|output| {
            if output >= bit {
                and_gate(left[output - bit], selected)
            } else {
                0
            }
        }));
        product = add_64(product, partial, 0).0;
    }
    product
}

#[must_use]
pub fn multiply_signed_64(left: u64, right: u64) -> u64 {
    let left_sign = u64_to_bits(left)[63];
    let right_sign = u64_to_bits(right)[63];
    let left_abs = select_64(left, add_64(bit_not_64(left), 0, 1).0, left_sign);
    let right_abs = select_64(right, add_64(bit_not_64(right), 0, 1).0, right_sign);
    let product = multiply_unsigned_64(left_abs, right_abs);
    select_64(
        product,
        add_64(bit_not_64(product), 0, 1).0,
        xor_gate(left_sign, right_sign),
    )
}

/// Restoring divider. Callers detect a zero divisor before evaluating it.
#[must_use]
pub fn divide_unsigned_64(dividend: u64, divisor: u64) -> u64 {
    let dividend_bits = u64_to_bits(dividend);
    let mut quotient_bits = [0_u8; 64];
    let mut remainder = 0_u64;
    for bit in (0..64).rev() {
        remainder = add_64(remainder, remainder, 0).0;
        remainder = add_64(remainder, u64::from(dividend_bits[bit]), 0).0;
        let (difference, borrow, _) = sub_64(remainder, divisor, 0);
        let take = not_gate(borrow);
        remainder = select_64(remainder, difference, take);
        quotient_bits[bit] = take;
    }
    bits_to_u64(&quotient_bits)
}

#[must_use]
pub fn divide_signed_64(dividend: u64, divisor: u64) -> u64 {
    let dividend_sign = u64_to_bits(dividend)[63];
    let divisor_sign = u64_to_bits(divisor)[63];
    let dividend_abs = select_64(
        dividend,
        add_64(bit_not_64(dividend), 0, 1).0,
        dividend_sign,
    );
    let divisor_abs = select_64(divisor, add_64(bit_not_64(divisor), 0, 1).0, divisor_sign);
    let quotient = divide_unsigned_64(dividend_abs, divisor_abs);
    select_64(
        quotient,
        add_64(bit_not_64(quotient), 0, 1).0,
        xor_gate(dividend_sign, divisor_sign),
    )
}

#[must_use]
pub fn byte_changed(left: u8, right: u8) -> u8 {
    u8_to_bits(left ^ right).into_iter().fold(0, or_gate)
}

#[must_use]
pub fn is_zero_8(value: u8) -> u8 {
    not_gate(u8_to_bits(value).into_iter().fold(0, or_gate))
}
