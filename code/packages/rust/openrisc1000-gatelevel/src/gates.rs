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
pub fn u64_to_bits(value: u64) -> [u8; 64] {
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

#[must_use]
pub fn bits_to_u64(bits: &[u8]) -> u64 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u64::from(*q) << bit))
}

/// Gate equality against a constant. This is also used as the terminal line
/// selector for instruction sub-decoders and SPR address decoders.
#[must_use]
pub(crate) fn selected(value: u32, expected: u32, width: usize) -> u8 {
    debug_assert!(width <= 32);
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

/// Six-to-sixty-four one-hot decoder used by primary instruction dispatch.
#[must_use]
pub fn decode_primary(opcode: u8) -> [u8; 64] {
    if opcode > 0x3f {
        return [0; 64];
    }
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

/// Result, carry, and signed-overflow from a 64-bit ripple adder.
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
pub fn bit_not_64(value: u64) -> u64 {
    bits_to_u64(&u64_to_bits(value).map(not_gate))
}

#[must_use]
pub fn sub_64(left: u64, right: u64) -> (u64, u8, u8) {
    let (value, carry, overflow) = add_64(left, bit_not_64(right), 1);
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
pub fn unsigned_less_32(left: u32, right: u32) -> u8 {
    let (_, borrow, _) = sub_32(left, right);
    borrow
}

#[must_use]
pub fn signed_less_32(left: u32, right: u32) -> u8 {
    let left_sign = u32_to_bits(left)[31];
    let right_sign = u32_to_bits(right)[31];
    let signs_differ = xor_gate(left_sign, right_sign);
    let (difference, _, overflow) = sub_32(left, right);
    let result_sign = u32_to_bits(difference)[31];
    let same_sign_result = xor_gate(result_sign, overflow);
    mux2(same_sign_result, left_sign, signs_differ)
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
        value = select_32(value, shifted, ((amount >> stage) & 1) as u8);
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
        value = select_32(value, shifted, ((amount >> stage) & 1) as u8);
    }
    value
}

#[must_use]
pub(crate) fn sign_extend(value: u32, width: usize) -> u32 {
    debug_assert!((1..=32).contains(&width));
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
pub fn first_one(value: u32) -> u32 {
    let bits = u32_to_bits(value);
    let mut found = 0;
    let mut encoded = 0;
    for (bit, signal) in bits.into_iter().enumerate() {
        let select = and_gate(signal, not_gate(found));
        encoded = select_32(encoded, bit as u32 + 1, select);
        found = or_gate(found, signal);
    }
    encoded
}

#[must_use]
pub fn last_one(value: u32) -> u32 {
    let bits = u32_to_bits(value);
    let mut found = 0;
    let mut encoded = 0;
    for bit in (0..32).rev() {
        let select = and_gate(bits[bit], not_gate(found));
        encoded = select_32(encoded, bit as u32 + 1, select);
        found = or_gate(found, bits[bit]);
    }
    encoded
}

#[must_use]
pub fn multiply_unsigned_32(left: u32, right: u32) -> u64 {
    let left = u32_to_bits(left);
    let right = u32_to_bits(right);
    let mut product = 0_u64;
    for (bit, selected) in right.into_iter().enumerate() {
        let partial = bits_to_u64(&std::array::from_fn::<_, 64, _>(|output| {
            if (bit..bit + 32).contains(&output) {
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
pub fn multiply_signed_32(left: u32, right: u32) -> u64 {
    let left_sign = u32_to_bits(left)[31];
    let right_sign = u32_to_bits(right)[31];
    let left_abs = select_32(left, add_32(bit_not_32(left), 0, 1).0, left_sign);
    let right_abs = select_32(right, add_32(bit_not_32(right), 0, 1).0, right_sign);
    let product = multiply_unsigned_32(left_abs, right_abs);
    let negative = xor_gate(left_sign, right_sign);
    let negated = add_64(bit_not_64(product), 0, 1).0;
    let product_bits = u64_to_bits(product);
    let negated_bits = u64_to_bits(negated);
    bits_to_u64(&std::array::from_fn::<_, 64, _>(|bit| {
        mux2(product_bits[bit], negated_bits[bit], negative)
    }))
}

/// Restoring unsigned divider. The caller handles the typed zero-divisor
/// fault; this network returns quotient and remainder for a nonzero divisor.
#[must_use]
pub fn divide_unsigned_32(dividend: u32, divisor: u32) -> (u32, u32) {
    let dividend_bits = u32_to_bits(dividend);
    let mut quotient_bits = [0_u8; 32];
    let mut remainder = 0_u64;
    let divisor = u64::from(divisor);
    for bit in (0..32).rev() {
        remainder = add_64(remainder, remainder, 0).0;
        remainder = add_64(remainder, u64::from(dividend_bits[bit]), 0).0;
        let (difference, borrow, _) = sub_64(remainder, divisor);
        let take = not_gate(borrow);
        let old = u64_to_bits(remainder);
        let new = u64_to_bits(difference);
        remainder = bits_to_u64(&std::array::from_fn::<_, 64, _>(|index| {
            mux2(old[index], new[index], take)
        }));
        quotient_bits[bit] = take;
    }
    (bits_to_u32(&quotient_bits), remainder as u32)
}

#[must_use]
pub fn divide_signed_32(dividend: u32, divisor: u32) -> u32 {
    let dividend_sign = u32_to_bits(dividend)[31];
    let divisor_sign = u32_to_bits(divisor)[31];
    let dividend_abs = select_32(
        dividend,
        add_32(bit_not_32(dividend), 0, 1).0,
        dividend_sign,
    );
    let divisor_abs = select_32(divisor, add_32(bit_not_32(divisor), 0, 1).0, divisor_sign);
    let quotient = divide_unsigned_32(dividend_abs, divisor_abs).0;
    select_32(
        quotient,
        add_32(bit_not_32(quotient), 0, 1).0,
        xor_gate(dividend_sign, divisor_sign),
    )
}
