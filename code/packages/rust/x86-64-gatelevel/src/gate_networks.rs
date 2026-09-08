//! Fixed-width combinational networks used by the independent execution path.

use crate::bits::{add_64, bits_to_u64, invert_64, u64_to_bits, zero_64};
use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};
use x86_simulator::state::Flags;

#[derive(Clone, Copy)]
pub(crate) enum ShiftKind {
    Left,
    Right,
    ArithmeticRight,
    RotateLeft,
    RotateRight,
}

pub(crate) fn shift(value: u64, amount: u32, kind: ShiftKind) -> u64 {
    let select = u64_to_bits(u64::from(amount & 63));
    let mut stage = u64_to_bits(value);
    for (selector, distance) in [1, 2, 4, 8, 16, 32].into_iter().enumerate() {
        let sign = stage[63];
        let shifted: [u8; 64] = std::array::from_fn(|bit| match kind {
            ShiftKind::Left => bit.checked_sub(distance).map_or(0, |source| stage[source]),
            ShiftKind::Right => stage.get(bit + distance).copied().unwrap_or(0),
            ShiftKind::ArithmeticRight => stage.get(bit + distance).copied().unwrap_or(sign),
            ShiftKind::RotateLeft => stage[(bit + 64 - distance) & 63],
            ShiftKind::RotateRight => stage[(bit + distance) & 63],
        });
        stage = std::array::from_fn(|bit| mux2(stage[bit], shifted[bit], select[selector]));
    }
    bits_to_u64(&stage)
}

pub(crate) fn multiply_unsigned(a: u64, b: u64) -> u128 {
    let a = u64_to_bits(a);
    let b = u64_to_bits(b);
    let mut accumulator = [0_u8; 128];
    for multiplier_bit in 0..64 {
        let partial: [u8; 128] = std::array::from_fn(|output_bit| {
            if output_bit >= multiplier_bit && output_bit - multiplier_bit < 64 {
                and_gate(a[output_bit - multiplier_bit], b[multiplier_bit])
            } else {
                0
            }
        });
        let sum = ripple_carry_adder_with_carry(&accumulator, &partial, 0).sum;
        accumulator.copy_from_slice(&sum);
    }
    bits_to_u128(&accumulator)
}

pub(crate) fn multiply_signed(a: u64, b: u64) -> i128 {
    let a_negative = u64_to_bits(a)[63];
    let b_negative = u64_to_bits(b)[63];
    let a_magnitude = magnitude64(a, a_negative);
    let b_magnitude = magnitude64(b, b_negative);
    let product = multiply_unsigned(a_magnitude, b_magnitude);
    if xor_gate(a_negative, b_negative) == 0 {
        product as i128
    } else {
        negate128(product) as i128
    }
}

pub(crate) fn divide_unsigned(dividend: u128, divisor: u64) -> Option<(u128, u64)> {
    if zero_64(divisor) != 0 {
        return None;
    }
    let dividend_bits = u128_to_bits(dividend);
    let divisor_bits: [u8; 129] = std::array::from_fn(|bit| {
        if bit < 64 {
            ((divisor >> bit) & 1) as u8
        } else {
            0
        }
    });
    let inverted = divisor_bits.map(not_gate);
    let mut remainder = [0_u8; 129];
    let mut quotient = [0_u8; 128];
    for bit in (0..128).rev() {
        remainder.copy_within(0..128, 1);
        remainder[0] = dividend_bits[bit];
        let difference = ripple_carry_adder_with_carry(&remainder, &inverted, 1);
        quotient[bit] = difference.carry_out;
        remainder = std::array::from_fn(|index| {
            mux2(
                remainder[index],
                difference.sum[index],
                difference.carry_out,
            )
        });
    }
    Some((bits_to_u128(&quotient), bits_to_u64(&remainder[..64])))
}

pub(crate) fn divide_signed(dividend: u128, divisor: u64) -> Option<(i128, i64)> {
    if zero_64(divisor) != 0 {
        return None;
    }
    let dividend_negative = u128_to_bits(dividend)[127];
    let divisor_negative = u64_to_bits(divisor)[63];
    let dividend_magnitude = if dividend_negative == 0 {
        dividend
    } else {
        negate128(dividend)
    };
    let divisor_magnitude = magnitude64(divisor, divisor_negative);
    let (quotient, remainder) = divide_unsigned(dividend_magnitude, divisor_magnitude)?;
    let quotient = if xor_gate(dividend_negative, divisor_negative) == 0 {
        quotient
    } else {
        negate128(quotient)
    };
    let remainder = if dividend_negative == 0 {
        remainder
    } else {
        add_64(invert_64(remainder), 0, 1).0
    };
    Some((quotient as i128, remainder as i64))
}

pub(crate) fn address(base: u64, index: u64, scale: u8, displacement: i64) -> u64 {
    let scaled = shift(index, scale.trailing_zeros(), ShiftKind::Left);
    let (partial, _, _) = add_64(base, scaled, 0);
    add_64(partial, displacement as u64, 0).0
}

pub(crate) fn condition_holds(condition: u8, flags: Flags) -> bool {
    let cf = u8::from(flags.cf);
    let zf = u8::from(flags.zf);
    let sf = u8::from(flags.sf);
    let of = u8::from(flags.of);
    let pf = u8::from(flags.pf);
    let value = match condition & 0xf {
        0x0 => of,
        0x1 => not_gate(of),
        0x2 => cf,
        0x3 => not_gate(cf),
        0x4 => zf,
        0x5 => not_gate(zf),
        0x6 => or_gate(cf, zf),
        0x7 => and_gate(not_gate(cf), not_gate(zf)),
        0x8 => sf,
        0x9 => not_gate(sf),
        0xa => pf,
        0xb => not_gate(pf),
        0xc => xor_gate(sf, of),
        0xd => not_gate(xor_gate(sf, of)),
        0xe => or_gate(zf, xor_gate(sf, of)),
        _ => and_gate(not_gate(zf), not_gate(xor_gate(sf, of))),
    };
    value != 0
}

fn magnitude64(value: u64, negative: u8) -> u64 {
    if negative == 0 {
        value
    } else {
        add_64(invert_64(value), 0, 1).0
    }
}

fn negate128(value: u128) -> u128 {
    let inverted = u128_to_bits(value).map(not_gate);
    let zero = [0_u8; 128];
    bits_to_u128(&ripple_carry_adder_with_carry(&inverted, &zero, 1).sum)
}

fn u128_to_bits(value: u128) -> [u8; 128] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn bits_to_u128(bits: &[u8]) -> u128 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u128::from(*q) << bit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barrel_shift_rotate_and_address_networks_match_edges() {
        for amount in [0, 1, 7, 32, 63, 64, 127] {
            let count = amount & 63;
            let value = 0x8001_0203_0405_0607;
            assert_eq!(shift(value, amount, ShiftKind::Left), value << count);
            assert_eq!(shift(value, amount, ShiftKind::Right), value >> count);
            assert_eq!(
                shift(value, amount, ShiftKind::ArithmeticRight),
                ((value as i64) >> count) as u64
            );
            assert_eq!(
                shift(value, amount, ShiftKind::RotateLeft),
                value.rotate_left(count)
            );
            assert_eq!(
                shift(value, amount, ShiftKind::RotateRight),
                value.rotate_right(count)
            );
        }
        assert_eq!(address(0x100, 3, 8, -4), 0x114);
    }

    #[test]
    fn fixed_round_multiply_and_divide_match_native_oracle() {
        for (a, b) in [
            (0, 1),
            (1, 1),
            (u64::MAX, 2),
            (0x8000_0000_0000_0000, 3),
            (0x1234_5678_9abc_def0, 0xfedc_ba98_7654_3210),
        ] {
            assert_eq!(multiply_unsigned(a, b), u128::from(a) * u128::from(b));
            assert_eq!(
                multiply_signed(a, b),
                i128::from(a as i64) * i128::from(b as i64)
            );
            if b != 0 {
                let dividend = (u128::from(a) << 64) | u128::from(!a);
                let (q, r) = divide_unsigned(dividend, b).unwrap();
                assert_eq!(q, dividend / u128::from(b));
                assert_eq!(r, (dividend % u128::from(b)) as u64);
            }
        }
    }

    #[test]
    fn all_condition_predicates_match_reference() {
        for mask in 0..32 {
            let flags = Flags {
                cf: mask & 1 != 0,
                zf: mask & 2 != 0,
                sf: mask & 4 != 0,
                of: mask & 8 != 0,
                pf: mask & 16 != 0,
                af: false,
            };
            for condition in 0..16 {
                assert_eq!(
                    condition_holds(condition, flags),
                    x86_simulator::flags::condition_holds(
                        x86_simulator::flags::Cond::from_nibble(condition),
                        &flags
                    )
                );
            }
        }
    }
}
