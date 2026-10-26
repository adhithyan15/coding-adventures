//! Fixed-width System/360 combinational gate networks.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

pub fn decode_byte(value: u8) -> [u8; 256] {
    let inputs: [u8; 8] = std::array::from_fn(|bit| (value >> bit) & 1);
    std::array::from_fn(|line| {
        (0..8).fold(1, |selected, bit| {
            let expected = ((line >> bit) & 1) as u8;
            let signal = if expected == 1 {
                inputs[bit]
            } else {
                not_gate(inputs[bit])
            };
            and_gate(selected, signal)
        })
    })
}

pub fn select_u32(d0: u32, d1: u32, select: u8) -> u32 {
    let d0 = u32_to_bits(d0);
    let d1 = u32_to_bits(d1);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        mux2(d0[bit], d1[bit], select)
    }))
}

pub fn branch_selected(mask: u8, condition_code: u8) -> u8 {
    let condition = [condition_code & 1, (condition_code >> 1) & 1];
    let mask = [mask & 1, (mask >> 1) & 1, (mask >> 2) & 1, (mask >> 3) & 1];
    let cc_0_or_1 = mux2(mask[3], mask[2], condition[0]);
    let cc_2_or_3 = mux2(mask[1], mask[0], condition[0]);
    mux2(cc_0_or_1, cc_2_or_3, condition[1])
}

/// One when a four-bit architectural register field is not register zero.
pub fn register_field_nonzero(field: u8) -> u8 {
    (0..4).fold(0, |present, bit| or_gate(present, (field >> bit) & 1))
}

/// One when a 32-bit count contains any asserted wire.
pub fn nonzero_32(value: u32) -> u8 {
    not_gate(is_zero_32(value))
}

/// The combined BCTR predicate: nonzero decremented count and nonzero target.
pub fn count_branch_selected(count: u32, target_register: u8) -> u8 {
    and_gate(nonzero_32(count), register_field_nonzero(target_register))
}

/// The BXH/BXLE predicate from one-hot opcode lines and comparator CC wires.
pub fn index_branch_selected(opcode: u8, compare_condition_code: u8) -> u8 {
    let decode = decode_byte(opcode);
    let low = compare_condition_code & 1;
    let high_bit = (compare_condition_code >> 1) & 1;
    let high = and_gate(high_bit, not_gate(low));
    or_gate(
        and_gate(decode[0x86], high),
        and_gate(decode[0x87], not_gate(high)),
    )
}

pub fn u32_to_bits(value: u32) -> [u8; 32] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

pub fn bits_to_u32(bits: &[u8]) -> u32 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u32::from(*q) << bit))
}

pub fn u64_to_bits(value: u64) -> [u8; 64] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

pub fn bits_to_u64(bits: &[u8]) -> u64 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u64::from(*q) << bit))
}

pub fn add_32(a: u32, b: u32, carry_in: u8) -> (u32, u8, u8) {
    let a_bits = u32_to_bits(a);
    let b_bits = u32_to_bits(b);
    let full = ripple_carry_adder_with_carry(&a_bits, &b_bits, carry_in);
    let low = ripple_carry_adder_with_carry(&a_bits[..31], &b_bits[..31], carry_in);
    (
        bits_to_u32(&full.sum),
        full.carry_out,
        xor_gate(low.carry_out, full.carry_out),
    )
}

pub fn add_24(a: u32, b: u32) -> u32 {
    let a_bits = u32_to_bits(a);
    let b_bits = u32_to_bits(b);
    bits_to_u32(&ripple_carry_adder_with_carry(&a_bits[..24], &b_bits[..24], 0).sum)
}

pub fn add_64(a: u64, b: u64, carry_in: u8) -> (u64, u8, u8) {
    let a_bits = u64_to_bits(a);
    let b_bits = u64_to_bits(b);
    let full = ripple_carry_adder_with_carry(&a_bits, &b_bits, carry_in);
    let low = ripple_carry_adder_with_carry(&a_bits[..63], &b_bits[..63], carry_in);
    (
        bits_to_u64(&full.sum),
        full.carry_out,
        xor_gate(low.carry_out, full.carry_out),
    )
}

pub fn bit_not_64(value: u64) -> u64 {
    bits_to_u64(&u64_to_bits(value).map(not_gate))
}

pub fn negate_64(value: u64) -> u64 {
    add_64(bit_not_64(value), 0, 1).0
}

pub fn multiply_signed_32(a: u32, b: u32) -> u64 {
    let a_negative = u32_to_bits(a)[31];
    let b_negative = u32_to_bits(b)[31];
    let a_magnitude = if a_negative == 1 { negate_32(a).0 } else { a };
    let b_magnitude = if b_negative == 1 { negate_32(b).0 } else { b };
    let mut product = 0_u64;
    for bit in 0..32 {
        if u32_to_bits(b_magnitude)[bit] == 1 {
            product = add_64(product, u64::from(a_magnitude) << bit, 0).0;
        }
    }
    if xor_gate(a_negative, b_negative) == 1 {
        negate_64(product)
    } else {
        product
    }
}

pub fn divide_signed_64_by_32(dividend: u64, divisor: u32) -> Option<(u32, u32)> {
    if divisor == 0 {
        return None;
    }
    let dividend_negative = u64_to_bits(dividend)[63];
    let divisor_negative = u32_to_bits(divisor)[31];
    let dividend_magnitude = if dividend_negative == 1 {
        negate_64(dividend)
    } else {
        dividend
    };
    let divisor_magnitude = if divisor_negative == 1 {
        negate_32(divisor).0
    } else {
        divisor
    };
    let divisor_wide = u64::from(divisor_magnitude);
    let dividend_bits = u64_to_bits(dividend_magnitude);
    let mut remainder = 0_u64;
    let mut quotient = 0_u64;
    for bit in (0..64).rev() {
        remainder = shift_left_64(remainder, 1) | u64::from(dividend_bits[bit]);
        let (_, at_least, _) = add_64(remainder, bit_not_64(divisor_wide), 1);
        if at_least == 1 {
            remainder = add_64(remainder, bit_not_64(divisor_wide), 1).0;
            quotient |= 1_u64 << bit;
        }
    }
    let quotient_negative = xor_gate(dividend_negative, divisor_negative);
    let limit = if quotient_negative == 1 {
        0x8000_0000_u64
    } else {
        0x7fff_ffff_u64
    };
    if quotient > limit {
        return None;
    }
    if quotient_negative == 1 {
        quotient = negate_64(quotient);
    }
    if dividend_negative == 1 {
        remainder = negate_64(remainder);
    }
    Some((remainder as u32, quotient as u32))
}

pub fn negate_32(value: u32) -> (u32, u8) {
    let inverted = bit_not_32(value);
    let (result, _, overflow) = add_32(inverted, 0, 1);
    (result, overflow)
}

pub fn bit_not_32(value: u32) -> u32 {
    bits_to_u32(&u32_to_bits(value).map(not_gate))
}

pub fn bit_and_32(a: u32, b: u32) -> u32 {
    let a = u32_to_bits(a);
    let b = u32_to_bits(b);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        and_gate(a[bit], b[bit])
    }))
}

pub fn bit_or_32(a: u32, b: u32) -> u32 {
    let a = u32_to_bits(a);
    let b = u32_to_bits(b);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        or_gate(a[bit], b[bit])
    }))
}

pub fn bit_xor_32(a: u32, b: u32) -> u32 {
    let a = u32_to_bits(a);
    let b = u32_to_bits(b);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        xor_gate(a[bit], b[bit])
    }))
}

pub fn is_zero_32(value: u32) -> u8 {
    not_gate(u32_to_bits(value).into_iter().fold(0, or_gate))
}

pub fn is_zero_64(value: u64) -> u8 {
    not_gate(u64_to_bits(value).into_iter().fold(0, or_gate))
}

pub fn logical_condition_code(value: u32) -> u8 {
    not_gate(is_zero_32(value))
}

pub fn signed_condition_code(value: u32) -> u8 {
    let zero = is_zero_32(value);
    let negative = u32_to_bits(value)[31];
    let positive = and_gate(not_gate(zero), not_gate(negative));
    negative | (positive << 1)
}

pub fn signed_condition_code_64(value: u64) -> u8 {
    let zero = is_zero_64(value);
    let negative = u64_to_bits(value)[63];
    let positive = and_gate(not_gate(zero), not_gate(negative));
    negative | (positive << 1)
}

pub fn unsigned_compare_condition_code(a: u32, b: u32) -> u8 {
    let (difference, carry, _) = add_32(a, bit_not_32(b), 1);
    let equal = is_zero_32(difference);
    let less = not_gate(carry);
    let greater = and_gate(carry, not_gate(equal));
    less | (greater << 1)
}

pub fn signed_compare_condition_code(a: u32, b: u32) -> u8 {
    let a_sign = u32_to_bits(a)[31];
    let b_sign = u32_to_bits(b)[31];
    let signs_differ = xor_gate(a_sign, b_sign);
    let (difference, _, _) = add_32(a, bit_not_32(b), 1);
    let same_sign_less = u32_to_bits(difference)[31];
    let less = or_gate(
        and_gate(signs_differ, a_sign),
        and_gate(not_gate(signs_differ), same_sign_less),
    );
    let equal = is_zero_32(difference);
    let greater = and_gate(not_gate(less), not_gate(equal));
    less | (greater << 1)
}

pub fn shift_left_32(value: u32, amount: u32) -> u32 {
    let amount = (amount & 63) as usize;
    if amount >= 32 {
        return 0;
    }
    let source = u32_to_bits(value);
    let mut output = [0; 32];
    output[amount..].copy_from_slice(&source[..32 - amount]);
    bits_to_u32(&output)
}

pub fn shift_right_32(value: u32, amount: u32) -> u32 {
    let amount = (amount & 63) as usize;
    if amount >= 32 {
        return 0;
    }
    let source = u32_to_bits(value);
    let mut output = [0; 32];
    output[..32 - amount].copy_from_slice(&source[amount..]);
    bits_to_u32(&output)
}

pub fn arithmetic_right_32(value: u32, amount: u32) -> u32 {
    let amount = (amount & 63).min(31) as usize;
    let source = u32_to_bits(value);
    let mut output = [source[31]; 32];
    output[..32 - amount].copy_from_slice(&source[amount..]);
    bits_to_u32(&output)
}

pub fn shift_left_64(value: u64, amount: u32) -> u64 {
    let amount = (amount & 63) as usize;
    let source = u64_to_bits(value);
    let mut output = [0; 64];
    output[amount..].copy_from_slice(&source[..64 - amount]);
    bits_to_u64(&output)
}

pub fn shift_right_64(value: u64, amount: u32) -> u64 {
    let amount = (amount & 63) as usize;
    let source = u64_to_bits(value);
    let mut output = [0; 64];
    output[..64 - amount].copy_from_slice(&source[amount..]);
    bits_to_u64(&output)
}

pub fn arithmetic_right_64(value: u64, amount: u32) -> u64 {
    let amount = (amount & 63) as usize;
    let source = u64_to_bits(value);
    let mut output = [source[63]; 64];
    output[..64 - amount].copy_from_slice(&source[amount..]);
    bits_to_u64(&output)
}

pub fn hfp_fraction_nonzero(raw: u64, long: bool) -> u8 {
    let bits = u64_to_bits(raw);
    let range = if long { 0..56 } else { 32..56 };
    range.fold(0, |present, bit| or_gate(present, bits[bit]))
}

pub fn hfp_with_sign(raw: u64, long: bool, negative: bool) -> u64 {
    if hfp_fraction_nonzero(raw, long) == 0 {
        return 0;
    }
    let mut bits = u64_to_bits(raw);
    bits[63] = u8::from(negative);
    if !long {
        bits[..32].fill(0);
    }
    bits_to_u64(&bits)
}

pub fn hfp_condition_code(raw: u64, long: bool) -> u8 {
    let present = hfp_fraction_nonzero(raw, long);
    let negative = and_gate(present, u64_to_bits(raw)[63]);
    let positive = and_gate(present, not_gate(negative));
    negative | (positive << 1)
}
