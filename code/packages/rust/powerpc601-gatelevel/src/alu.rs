//! PowerPC 601 32-bit datapaths built from repository gate primitives.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

use crate::bits::{add_32, bits_to_u32, invert_32, sar_32, shl_32, shr_32, u32_to_bits, zero_32};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AluResult32 {
    pub result: u32,
    pub carry: u8,
    pub overflow: u8,
    pub zero: u8,
    pub negative: u8,
}

fn result(value: u32, carry: u8, overflow: u8) -> AluResult32 {
    AluResult32 {
        result: value,
        carry,
        overflow,
        zero: zero_32(value),
        negative: u32_to_bits(value)[31],
    }
}

pub fn add(a: u32, b: u32, carry_in: u8) -> AluResult32 {
    let (value, carry, overflow) = add_32(a, b, carry_in);
    result(value, carry, overflow)
}

pub fn sub(a: u32, b: u32) -> AluResult32 {
    let (value, carry, overflow) = add_32(a, invert_32(b), 1);
    result(value, carry, overflow)
}

fn bitwise(a: u32, b: u32, gate: fn(u8, u8) -> u8) -> AluResult32 {
    let a = u32_to_bits(a);
    let b = u32_to_bits(b);
    result(
        bits_to_u32(&std::array::from_fn::<_, 32, _>(|i| gate(a[i], b[i]))),
        0,
        0,
    )
}

pub fn and(a: u32, b: u32) -> AluResult32 {
    bitwise(a, b, and_gate)
}
pub fn or(a: u32, b: u32) -> AluResult32 {
    bitwise(a, b, or_gate)
}
pub fn xor(a: u32, b: u32) -> AluResult32 {
    bitwise(a, b, xor_gate)
}
pub fn nand(a: u32, b: u32) -> AluResult32 {
    bitwise(a, b, |x, y| not_gate(and_gate(x, y)))
}
pub fn nor(a: u32, b: u32) -> AluResult32 {
    bitwise(a, b, |x, y| not_gate(or_gate(x, y)))
}
pub fn shift_left(a: u32, amount: u32) -> AluResult32 {
    result(shl_32(a, amount), 0, 0)
}
pub fn shift_right(a: u32, amount: u32) -> AluResult32 {
    result(shr_32(a, amount), 0, 0)
}
pub fn shift_right_arithmetic(a: u32, amount: u32) -> AluResult32 {
    result(sar_32(a, amount), 0, 0)
}
pub fn count_leading_zeros(a: u32) -> u32 {
    u32_to_bits(a)
        .iter()
        .rev()
        .take_while(|bit| **bit == 0)
        .count() as u32
}

pub fn compare_signed(a: u32, b: u32) -> std::cmp::Ordering {
    let difference = sub(a, b);
    if difference.zero != 0 {
        std::cmp::Ordering::Equal
    } else if xor_gate(difference.negative, difference.overflow) != 0 {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Greater
    }
}

pub fn compare_unsigned(a: u32, b: u32) -> std::cmp::Ordering {
    let difference = sub(a, b);
    if difference.zero != 0 {
        std::cmp::Ordering::Equal
    } else if difference.carry == 0 {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Greater
    }
}

fn multiply_bits(a: u32, b: u32) -> [u8; 64] {
    let a = u32_to_bits(a);
    let b = u32_to_bits(b);
    let mut accumulator = [0; 64];
    for multiplier_bit in 0..32 {
        let partial: [u8; 64] = std::array::from_fn(|output_bit| {
            if output_bit >= multiplier_bit && output_bit - multiplier_bit < 32 {
                and_gate(a[output_bit - multiplier_bit], b[multiplier_bit])
            } else {
                0
            }
        });
        let sum = ripple_carry_adder_with_carry(&accumulator, &partial, 0).sum;
        accumulator.copy_from_slice(&sum);
    }
    accumulator
}

pub fn multiply_low(a: u32, b: u32) -> u32 {
    bits_to_u32(&multiply_bits(a, b)[..32])
}

pub fn divide_unsigned(dividend: u32, divisor: u32) -> Option<u32> {
    if zero_32(divisor) != 0 {
        return None;
    }
    let dividend_bits = u32_to_bits(dividend);
    let divisor_bits: [u8; 33] = std::array::from_fn(|bit| {
        if bit < 32 {
            (divisor >> bit) as u8 & 1
        } else {
            0
        }
    });
    let inverted = divisor_bits.map(not_gate);
    let mut remainder = [0_u8; 33];
    let mut quotient = [0_u8; 32];
    for bit in (0..32).rev() {
        remainder.copy_within(0..32, 1);
        remainder[0] = dividend_bits[bit];
        let difference = ripple_carry_adder_with_carry(&remainder, &inverted, 1);
        quotient[bit] = difference.carry_out;
        if difference.carry_out != 0 {
            remainder.copy_from_slice(&difference.sum);
        }
    }
    Some(bits_to_u32(&quotient))
}

pub fn divide_signed(dividend: u32, divisor: u32) -> Option<u32> {
    if zero_32(divisor) != 0 {
        return None;
    }
    let dividend_negative = u32_to_bits(dividend)[31];
    let divisor_negative = u32_to_bits(divisor)[31];
    let absolute = |value: u32, negative: u8| {
        if negative == 0 {
            value
        } else {
            add(invert_32(value), 0, 1).result
        }
    };
    let quotient = divide_unsigned(
        absolute(dividend, dividend_negative),
        absolute(divisor, divisor_negative),
    )?;
    Some(if xor_gate(dividend_negative, divisor_negative) == 0 {
        quotient
    } else {
        add(invert_32(quotient), 0, 1).result
    })
}
