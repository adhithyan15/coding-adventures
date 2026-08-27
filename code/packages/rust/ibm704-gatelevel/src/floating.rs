//! Gate networks for the IBM 704 floating instruction family.
//!
//! Fractions, signed binary scales, priority encoding, alignment, normalization,
//! rounding, multiplication, and division all use fixed-width bit vectors and
//! digital primitives. Host loops describe repeated wiring; they do not choose
//! shifts, calculate characteristics, or perform an architectural result.

use arithmetic::adders::{ripple_carry_adder, ripple_carry_adder_with_carry};
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

use crate::{MAGNITUDE_MASK, SIGN_BIT};

const IBM_PRECISION: usize = 27;
const BINARY64_PRECISION: usize = 53;
const MAG_WIDTH: usize = 384;
const SCALE_WIDTH: usize = 10;
const DIV_WIDTH: usize = 128;
const DIV_SHIFT: usize = 85;

type Magnitude = [u8; MAG_WIDTH];
type Scale = [u8; SCALE_WIDTH];

#[derive(Clone, Debug, Eq, PartialEq)]
struct BinaryValue {
    sign: u8,
    /// Unsigned significand, least-significant bit first.
    magnitude: Magnitude,
    /// Signed two's-complement scale: `value = magnitude * 2^-scale`.
    scale: Scale,
}

impl BinaryValue {
    fn with_zero_sign(mut self) -> Self {
        self.sign = and_gate(self.sign, not_gate(is_zero(&self.magnitude)));
        self
    }
}

pub(crate) fn add_words(left: u64, right: u64, subtract: bool) -> u64 {
    let left = from_word(left);
    let mut right = from_word(right);
    right.sign = xor_gate(right.sign, subtract as u8);
    right = right.with_zero_sign();
    to_word(round_precision(
        add_exact(&left, &right),
        BINARY64_PRECISION,
    ))
}

pub(crate) fn multiply_words(left: u64, right: u64) -> u64 {
    let product = multiply_exact(&from_word(left), &from_word(right));
    to_word(round_precision(product, BINARY64_PRECISION))
}

pub(crate) fn divide_words(dividend: u64, divisor: u64) -> Option<(u64, u64)> {
    let dividend = from_word(dividend);
    let divisor = from_word(divisor);
    if is_zero(&divisor.magnitude) == 1 {
        return None;
    }

    // Match the oracle's binary64 division, rounded multiplication, and
    // rounded subtraction before converting quotient and remainder to IBM.
    let quotient = divide_rounded(&dividend, &divisor);
    let product = round_precision(multiply_exact(&quotient, &divisor), BINARY64_PRECISION);
    let mut negative_product = product;
    negative_product.sign = xor_gate(
        negative_product.sign,
        not_gate(is_zero(&negative_product.magnitude)),
    );
    let remainder = round_precision(add_exact(&dividend, &negative_product), BINARY64_PRECISION);
    Some((to_word(quotient), to_word(remainder)))
}

fn from_word(word: u64) -> BinaryValue {
    let word_bits = bits_from_u64::<36>(word & ((1u64 << 36) - 1));
    let mut magnitude = [0; MAG_WIDTH];
    magnitude[..IBM_PRECISION].copy_from_slice(&word_bits[..IBM_PRECISION]);
    let mut characteristic = [0; SCALE_WIDTH];
    characteristic[..8].copy_from_slice(&word_bits[27..35]);
    BinaryValue {
        sign: word_bits[35],
        magnitude,
        scale: sub_bits(&signed_constant(155), &characteristic),
    }
    .with_zero_sign()
}

fn to_word(value: BinaryValue) -> u64 {
    let rounded = round_precision(value, IBM_PRECISION);
    let zero = is_zero(&rounded.magnitude);
    let characteristic = sub_bits(&signed_constant(155), &rounded.scale);
    let underflow = characteristic[SCALE_WIDTH - 1];
    let overflow = signed_greater(&characteristic, &signed_constant(255));

    let mut normal = [0; 36];
    normal[..IBM_PRECISION].copy_from_slice(&rounded.magnitude[..IBM_PRECISION]);
    normal[27..35].copy_from_slice(&characteristic[..8]);
    normal[35] = rounded.sign;

    let mut underflow_word = [0; 36];
    underflow_word[35] = rounded.sign;
    let mut overflow_word = [1; 36];
    overflow_word[35] = rounded.sign;
    let bounded = mux_bits(overflow, &normal, &overflow_word);
    let bounded = mux_bits(underflow, &bounded, &underflow_word);
    let word = mux_bits(zero, &bounded, &[0; 36]);
    bits_to_u64(&word) & (SIGN_BIT | MAGNITUDE_MASK)
}

fn add_exact(left: &BinaryValue, right: &BinaryValue) -> BinaryValue {
    let left_scale_greater = signed_greater(&left.scale, &right.scale);
    let common_scale = mux_bits(left_scale_greater, &right.scale, &left.scale);
    let left_shift = sub_bits(&common_scale, &left.scale);
    let right_shift = sub_bits(&common_scale, &right.scale);
    let left_magnitude = barrel_shift_left(&left.magnitude, &left_shift);
    let right_magnitude = barrel_shift_left(&right.magnitude, &right_shift);

    let sum = add_bits(&left_magnitude, &right_magnitude);
    let left_minus_right = sub_bits(&left_magnitude, &right_magnitude);
    let right_minus_left = sub_bits(&right_magnitude, &left_magnitude);
    let left_ge_right = not_gate(unsigned_greater(&right_magnitude, &left_magnitude));
    let difference = mux_bits(left_ge_right, &right_minus_left, &left_minus_right);
    let difference_sign = mux_bit(left_ge_right, right.sign, left.sign);
    let same_sign = not_gate(xor_gate(left.sign, right.sign));
    let magnitude = mux_bits(same_sign, &difference, &sum);
    let sign = mux_bit(same_sign, difference_sign, left.sign);

    BinaryValue {
        sign,
        magnitude,
        scale: common_scale,
    }
    .with_zero_sign()
}

fn multiply_exact(left: &BinaryValue, right: &BinaryValue) -> BinaryValue {
    let mut product = [0; MAG_WIDTH];
    // All callers provide a raw 27-bit IBM fraction or a normalized 53-bit
    // binary64 intermediate; higher multiplier wires are provably zero.
    for multiplier_bit in 0..BINARY64_PRECISION {
        let partial: Magnitude = std::array::from_fn(|bit| {
            if bit >= multiplier_bit && bit - multiplier_bit < BINARY64_PRECISION {
                and_gate(
                    left.magnitude[bit - multiplier_bit],
                    right.magnitude[multiplier_bit],
                )
            } else {
                0
            }
        });
        product = add_bits(&product, &partial);
    }
    BinaryValue {
        sign: xor_gate(left.sign, right.sign),
        magnitude: product,
        scale: add_bits(&left.scale, &right.scale),
    }
    .with_zero_sign()
}

fn divide_rounded(left: &BinaryValue, right: &BinaryValue) -> BinaryValue {
    let numerator: [u8; DIV_WIDTH] = std::array::from_fn(|bit| {
        if bit >= DIV_SHIFT && bit - DIV_SHIFT < IBM_PRECISION {
            left.magnitude[bit - DIV_SHIFT]
        } else {
            0
        }
    });
    let denominator: [u8; DIV_WIDTH] = std::array::from_fn(|bit| right.magnitude[bit]);
    let (mut quotient, remainder) = divide_magnitudes(numerator, denominator);
    quotient[0] = or_gate(quotient[0], not_gate(is_zero(&remainder)));
    let mut magnitude = [0; MAG_WIDTH];
    magnitude[..DIV_WIDTH].copy_from_slice(&quotient);
    let scale_difference = sub_bits(&left.scale, &right.scale);
    let scale = add_bits(&scale_difference, &signed_constant(DIV_SHIFT as i16));
    round_precision(
        BinaryValue {
            sign: xor_gate(left.sign, right.sign),
            magnitude,
            scale,
        },
        BINARY64_PRECISION,
    )
}

fn divide_magnitudes(
    numerator: [u8; DIV_WIDTH],
    denominator: [u8; DIV_WIDTH],
) -> ([u8; DIV_WIDTH], [u8; DIV_WIDTH + 1]) {
    let denominator_wide: [u8; DIV_WIDTH + 1] =
        std::array::from_fn(|bit| if bit < DIV_WIDTH { denominator[bit] } else { 0 });
    let mut remainder = [0; DIV_WIDTH + 1];
    let mut quotient = [0; DIV_WIDTH];
    for numerator_bit in (0..DIV_WIDTH).rev() {
        for bit in (1..remainder.len()).rev() {
            remainder[bit] = remainder[bit - 1];
        }
        remainder[0] = numerator[numerator_bit];
        let subtract = not_gate(unsigned_greater(&denominator_wide, &remainder));
        let difference = sub_bits(&remainder, &denominator_wide);
        remainder = mux_bits(subtract, &remainder, &difference);
        quotient[numerator_bit] = subtract;
    }
    (quotient, remainder)
}

fn round_precision(value: BinaryValue, precision: usize) -> BinaryValue {
    let (highest, nonzero) = priority_encode(&value.magnitude);
    let target = unsigned_constant::<SCALE_WIDTH>(precision - 1);
    let shift_right_select = unsigned_greater(&highest, &target);
    let shift_left_select = unsigned_greater(&target, &highest);
    let right_amount = sub_bits(&highest, &target);
    let left_amount = sub_bits(&target, &highest);

    let shifted_right = barrel_shift_right(&value.magnitude, &right_amount);
    let guard_amount = sub_bits(&right_amount, &unsigned_constant::<SCALE_WIDTH>(1));
    let guard_vector = barrel_shift_right(&value.magnitude, &guard_amount);
    let guard = and_gate(shift_right_select, guard_vector[0]);
    let sticky = value
        .magnitude
        .iter()
        .copied()
        .enumerate()
        .fold(0, |acc, (bit, input)| {
            let below_guard =
                unsigned_greater(&guard_amount, &unsigned_constant::<SCALE_WIDTH>(bit));
            or_gate(acc, and_gate(input, below_guard))
        });
    let round_up = and_gate(guard, or_gate(sticky, shifted_right[0]));
    let mut increment = [0; MAG_WIDTH];
    increment[0] = round_up;
    let rounded = add_bits(&shifted_right, &increment);
    let carry = rounded[precision];
    let rounded_after_carry: Magnitude = std::array::from_fn(|bit| {
        if bit + 1 < MAG_WIDTH {
            rounded[bit + 1]
        } else {
            0
        }
    });
    let rounded = mux_bits(carry, &rounded, &rounded_after_carry);

    let shifted_left = barrel_shift_left(&value.magnitude, &left_amount);
    let right_scale = sub_bits(&value.scale, &right_amount);
    let right_scale_after_carry = sub_bits(&right_scale, &unsigned_constant(1));
    let right_scale = mux_bits(carry, &right_scale, &right_scale_after_carry);
    let left_scale = add_bits(&value.scale, &left_amount);

    let non_right_magnitude = mux_bits(shift_left_select, &value.magnitude, &shifted_left);
    let magnitude = mux_bits(shift_right_select, &non_right_magnitude, &rounded);
    let non_right_scale = mux_bits(shift_left_select, &value.scale, &left_scale);
    let scale = mux_bits(shift_right_select, &non_right_scale, &right_scale);
    let magnitude = mux_bits(nonzero, &[0; MAG_WIDTH], &magnitude);
    let scale = mux_bits(nonzero, &[0; SCALE_WIDTH], &scale);

    BinaryValue {
        sign: and_gate(value.sign, nonzero),
        magnitude,
        scale,
    }
}

fn priority_encode(bits: &Magnitude) -> (Scale, u8) {
    let mut encoded = [0; SCALE_WIDTH];
    let mut any = 0;
    for (index, input) in bits.iter().copied().enumerate() {
        encoded = mux_bits(input, &encoded, &unsigned_constant(index));
        any = or_gate(any, input);
    }
    (encoded, any)
}

fn barrel_shift_left(bits: &Magnitude, amount: &Scale) -> Magnitude {
    let mut result = *bits;
    for (stage, select) in amount.iter().copied().enumerate() {
        let distance = 1usize << stage;
        let shifted: Magnitude = std::array::from_fn(|bit| {
            if bit >= distance {
                result[bit - distance]
            } else {
                0
            }
        });
        result = mux_bits(select, &result, &shifted);
    }
    result
}

fn barrel_shift_right(bits: &Magnitude, amount: &Scale) -> Magnitude {
    let mut result = *bits;
    for (stage, select) in amount.iter().copied().enumerate() {
        let distance = 1usize << stage;
        let shifted: Magnitude = std::array::from_fn(|bit| {
            if bit + distance < MAG_WIDTH {
                result[bit + distance]
            } else {
                0
            }
        });
        result = mux_bits(select, &result, &shifted);
    }
    result
}

fn add_bits<const WIDTH: usize>(left: &[u8; WIDTH], right: &[u8; WIDTH]) -> [u8; WIDTH] {
    ripple_carry_adder(left, right)
        .sum
        .try_into()
        .expect("a ripple adder preserves its fixed width")
}

fn sub_bits<const WIDTH: usize>(left: &[u8; WIDTH], right: &[u8; WIDTH]) -> [u8; WIDTH] {
    ripple_carry_adder_with_carry(left, &right.map(not_gate), 1)
        .sum
        .try_into()
        .expect("a ripple subtractor preserves its fixed width")
}

fn mux_bits<const WIDTH: usize>(select_b: u8, a: &[u8; WIDTH], b: &[u8; WIDTH]) -> [u8; WIDTH] {
    std::array::from_fn(|bit| mux_bit(select_b, a[bit], b[bit]))
}

fn mux_bit(select_b: u8, a: u8, b: u8) -> u8 {
    or_gate(and_gate(not_gate(select_b), a), and_gate(select_b, b))
}

fn unsigned_greater<const WIDTH: usize>(left: &[u8; WIDTH], right: &[u8; WIDTH]) -> u8 {
    let mut greater = 0;
    let mut equal = 1;
    for bit in (0..WIDTH).rev() {
        greater = or_gate(
            greater,
            and_gate(equal, and_gate(left[bit], not_gate(right[bit]))),
        );
        equal = and_gate(equal, not_gate(xor_gate(left[bit], right[bit])));
    }
    greater
}

fn signed_greater(left: &Scale, right: &Scale) -> u8 {
    let left_negative = left[SCALE_WIDTH - 1];
    let right_negative = right[SCALE_WIDTH - 1];
    let signs_differ = xor_gate(left_negative, right_negative);
    let left_positive = and_gate(not_gate(left_negative), right_negative);
    mux_bit(signs_differ, unsigned_greater(left, right), left_positive)
}

fn is_zero<const WIDTH: usize>(bits: &[u8; WIDTH]) -> u8 {
    not_gate(bits.iter().copied().fold(0, or_gate))
}

fn unsigned_constant<const WIDTH: usize>(value: usize) -> [u8; WIDTH] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn signed_constant(value: i16) -> Scale {
    let encoded = value as u16;
    std::array::from_fn(|bit| ((encoded >> bit) & 1) as u8)
}

fn bits_from_u64<const WIDTH: usize>(value: u64) -> [u8; WIDTH] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn bits_to_u64(bits: &[u8]) -> u64 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, input)| value | u64::from(*input) << bit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{float_to_fp, fp_to_float, make_word};

    #[test]
    fn priority_encoder_and_barrel_shifters_are_gate_selected() {
        let mut value = [0; MAG_WIDTH];
        value[3] = 1;
        value[127] = 1;
        let (highest, any) = priority_encode(&value);
        assert_eq!(bits_to_u64(&highest), 127);
        assert_eq!(any, 1);
        let shifted = barrel_shift_right(&value, &unsigned_constant(124));
        assert_eq!(shifted[3], 1);
        let restored = barrel_shift_left(&shifted, &unsigned_constant(124));
        assert_eq!(restored[127], 1);
    }

    #[test]
    fn gate_floating_paths_match_simple_oracle_values() {
        assert_eq!(
            fp_to_float(add_words(float_to_fp(1.5), float_to_fp(2.5), false)),
            4.0
        );
        assert_eq!(
            fp_to_float(add_words(float_to_fp(1.5), float_to_fp(2.5), true)),
            -1.0
        );
        assert_eq!(
            fp_to_float(multiply_words(float_to_fp(-3.0), float_to_fp(4.0))),
            -12.0
        );
        let (quotient, remainder) = divide_words(float_to_fp(7.0), float_to_fp(2.0)).unwrap();
        assert_eq!(fp_to_float(quotient), 3.5);
        assert_eq!(remainder, 0);
        assert_eq!(to_word(from_word(make_word(true, 0))), 0);
    }

    #[test]
    fn seeded_gate_networks_match_binary64_rounding() {
        let mut seed = 0x0704_1954_d15e_a5e5_u64;
        let mut next_word = || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let sign = seed >> 63 != 0;
            let characteristic = ((seed >> 27) & 0xff) as u8;
            let fraction = (seed & ((1 << IBM_PRECISION) - 1)) | (1 << 26);
            make_word(sign, (u64::from(characteristic) << 27) | fraction)
        };

        for _ in 0..64 {
            let left = next_word();
            let right = next_word();
            let left_float = fp_to_float(left);
            let right_float = fp_to_float(right);
            assert_eq!(
                add_words(left, right, false),
                float_to_fp(left_float + right_float)
            );
            assert_eq!(
                add_words(left, right, true),
                float_to_fp(left_float - right_float)
            );
            assert_eq!(
                multiply_words(left, right),
                float_to_fp(left_float * right_float)
            );
            let (quotient, remainder) = divide_words(left, right).unwrap();
            let expected_quotient = left_float / right_float;
            assert_eq!(quotient, float_to_fp(expected_quotient));
            assert_eq!(
                remainder,
                float_to_fp(left_float - expected_quotient * right_float)
            );
        }
    }
}
