//! FPv4-SP binary32 combinational networks.
//!
//! The network expands finite operands into fixed-width magnitude/scale wires,
//! performs exact add/multiply or precision-extended divide/square-root, and
//! rounds once to binary32 with guard/sticky ties-to-even logic. No host
//! floating-point value participates in production execution.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

const WIDTH: usize = 384;
const SCALE_WIDTH: usize = 10;
const DIV_SHIFT: usize = 192;
const SQRT_SHIFT: usize = 100;
const CANONICAL_NAN: u32 = 0x7fc0_0000;

type Magnitude = [u8; WIDTH];
type Scale = [u8; SCALE_WIDTH];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Status {
    pub invalid: u8,
    pub divide_by_zero: u8,
    pub overflow: u8,
    pub underflow: u8,
    pub inexact: u8,
    pub input_denormal: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Result {
    pub bits: u32,
    pub status: Status,
}

#[derive(Clone, Copy)]
struct Binary {
    sign: u8,
    magnitude: Magnitude,
    /// `value = magnitude * 2^-scale` in signed two's-complement wires.
    scale: Scale,
}

#[derive(Clone, Copy)]
struct Parts {
    sign: u8,
    exponent: [u8; 8],
    fraction: [u8; 23],
    zero: u8,
    infinity: u8,
    nan: u8,
    signaling_nan: u8,
    denormal: u8,
    finite: Binary,
}

#[derive(Clone, Copy)]
pub enum BinaryOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

pub fn binary(left: u32, right: u32, operation: BinaryOperation) -> Result {
    let left = split(left);
    let mut right = split(right);
    let mut status = Status {
        input_denormal: or_gate(left.denormal, right.denormal),
        invalid: or_gate(left.signaling_nan, right.signaling_nan),
        ..Status::default()
    };
    if matches!(operation, BinaryOperation::Subtract) {
        right.sign = not_gate(right.sign);
        right.finite.sign = right.sign;
    }
    if or_gate(left.nan, right.nan) == 1 {
        return Result {
            bits: CANONICAL_NAN,
            status,
        };
    }

    let sign = xor_gate(left.sign, right.sign);
    match operation {
        BinaryOperation::Add | BinaryOperation::Subtract => {
            if and_gate(left.infinity, right.infinity) == 1 {
                if xor_gate(left.sign, right.sign) == 1 {
                    status.invalid = 1;
                    return Result {
                        bits: CANONICAL_NAN,
                        status,
                    };
                }
                return special_infinity(left.sign, status);
            }
            if left.infinity == 1 {
                return special_infinity(left.sign, status);
            }
            if right.infinity == 1 {
                return special_infinity(right.sign, status);
            }
            pack(add_exact(&left.finite, &right.finite), status)
        }
        BinaryOperation::Multiply => {
            if and_gate(
                or_gate(left.infinity, right.infinity),
                or_gate(left.zero, right.zero),
            ) == 1
            {
                status.invalid = 1;
                return Result {
                    bits: CANONICAL_NAN,
                    status,
                };
            }
            if or_gate(left.infinity, right.infinity) == 1 {
                return special_infinity(sign, status);
            }
            if or_gate(left.zero, right.zero) == 1 {
                return special_zero(sign, status);
            }
            pack(multiply_exact(&left.finite, &right.finite), status)
        }
        BinaryOperation::Divide => {
            if or_gate(
                and_gate(left.zero, right.zero),
                and_gate(left.infinity, right.infinity),
            ) == 1
            {
                status.invalid = 1;
                return Result {
                    bits: CANONICAL_NAN,
                    status,
                };
            }
            if left.infinity == 1 {
                return special_infinity(sign, status);
            }
            if right.infinity == 1 || left.zero == 1 {
                return special_zero(sign, status);
            }
            if right.zero == 1 {
                status.divide_by_zero = 1;
                return special_infinity(sign, status);
            }
            pack(divide_exact(&left.finite, &right.finite), status)
        }
    }
}

pub fn square_root(value: u32) -> Result {
    let value = split(value);
    let mut status = Status {
        input_denormal: value.denormal,
        invalid: value.signaling_nan,
        ..Status::default()
    };
    if value.nan == 1 {
        return Result {
            bits: CANONICAL_NAN,
            status,
        };
    }
    if and_gate(value.sign, not_gate(value.zero)) == 1 {
        status.invalid = 1;
        return Result {
            bits: CANONICAL_NAN,
            status,
        };
    }
    if value.infinity == 1 {
        return special_infinity(0, status);
    }
    if value.zero == 1 {
        return special_zero(value.sign, status);
    }
    pack(sqrt_exact(&value.finite), status)
}

/// FPSCR NZCV for a quiet comparison; signaling NaNs additionally set IOC.
pub fn compare(left: u32, right: u32) -> (u32, u8, u8) {
    let left = split(left);
    let right = split(right);
    let denormal = or_gate(left.denormal, right.denormal);
    let signaling = or_gate(left.signaling_nan, right.signaling_nan);
    if or_gate(left.nan, right.nan) == 1 {
        return (0x3000_0000, signaling, denormal);
    }
    if and_gate(left.zero, right.zero) == 1 || equal_bits(left, right) == 1 {
        return (0x6000_0000, 0, denormal);
    }
    let less = finite_less(left, right);
    (
        if less == 1 { 0x8000_0000 } else { 0x2000_0000 },
        0,
        denormal,
    )
}

fn split(bits: u32) -> Parts {
    let wires = bits_from_u32(bits);
    let sign = wires[31];
    let exponent: [u8; 8] = wires[23..31].try_into().expect("fixed exponent");
    let fraction: [u8; 23] = wires[..23].try_into().expect("fixed fraction");
    let exponent_zero = is_zero(&exponent);
    let exponent_full = exponent.iter().copied().fold(1, and_gate);
    let fraction_zero = is_zero(&fraction);
    let zero = and_gate(exponent_zero, fraction_zero);
    let infinity = and_gate(exponent_full, fraction_zero);
    let nan = and_gate(exponent_full, not_gate(fraction_zero));
    let signaling_nan = and_gate(nan, not_gate(fraction[22]));
    let denormal = and_gate(exponent_zero, not_gate(fraction_zero));
    let mut normal_magnitude = [0; WIDTH];
    normal_magnitude[..23].copy_from_slice(&fraction);
    normal_magnitude[23] = 1;
    let mut subnormal_magnitude = [0; WIDTH];
    subnormal_magnitude[..23].copy_from_slice(&fraction);
    let finite = Binary {
        sign,
        magnitude: mux_bits(exponent_zero, &normal_magnitude, &subnormal_magnitude),
        scale: mux_bits(
            exponent_zero,
            &sub_bits(&signed_constant(150), &extend::<8, SCALE_WIDTH>(&exponent)),
            &signed_constant(149),
        ),
    };
    Parts {
        sign,
        exponent,
        fraction,
        zero,
        infinity,
        nan,
        signaling_nan,
        denormal,
        finite,
    }
}

fn add_exact(left: &Binary, right: &Binary) -> Binary {
    let left_scale_greater = signed_greater(&left.scale, &right.scale);
    let common = mux_bits(left_scale_greater, &right.scale, &left.scale);
    let left_shift = sub_bits(&common, &left.scale);
    let right_shift = sub_bits(&common, &right.scale);
    let left_magnitude = barrel_shift_left(&left.magnitude, &left_shift);
    let right_magnitude = barrel_shift_left(&right.magnitude, &right_shift);
    let sum = add_bits(&left_magnitude, &right_magnitude);
    let left_minus_right = sub_bits(&left_magnitude, &right_magnitude);
    let right_minus_left = sub_bits(&right_magnitude, &left_magnitude);
    let left_ge_right = not_gate(unsigned_greater(&right_magnitude, &left_magnitude));
    let difference = mux_bits(left_ge_right, &right_minus_left, &left_minus_right);
    let difference_sign = mux_bit(left_ge_right, right.sign, left.sign);
    let same_sign = not_gate(xor_gate(left.sign, right.sign));
    Binary {
        sign: mux_bit(same_sign, difference_sign, left.sign),
        magnitude: mux_bits(same_sign, &difference, &sum),
        scale: common,
    }
    .without_negative_zero()
}

fn multiply_exact(left: &Binary, right: &Binary) -> Binary {
    let mut product = [0; WIDTH];
    for multiplier in 0..24 {
        let partial = std::array::from_fn(|bit| {
            if bit >= multiplier && bit - multiplier < 24 {
                and_gate(
                    left.magnitude[bit - multiplier],
                    right.magnitude[multiplier],
                )
            } else {
                0
            }
        });
        product = add_bits(&product, &partial);
    }
    Binary {
        sign: xor_gate(left.sign, right.sign),
        magnitude: product,
        scale: add_bits(&left.scale, &right.scale),
    }
}

fn divide_exact(left: &Binary, right: &Binary) -> Binary {
    let numerator = std::array::from_fn(|bit| {
        if bit >= DIV_SHIFT && bit - DIV_SHIFT < WIDTH {
            left.magnitude[bit - DIV_SHIFT]
        } else {
            0
        }
    });
    let (mut quotient, remainder) = divide_magnitude(numerator, right.magnitude);
    quotient[0] = or_gate(quotient[0], not_gate(is_zero(&remainder)));
    Binary {
        sign: xor_gate(left.sign, right.sign),
        magnitude: quotient,
        scale: add_bits(
            &sub_bits(&left.scale, &right.scale),
            &signed_constant(DIV_SHIFT as i16),
        ),
    }
}

fn sqrt_exact(value: &Binary) -> Binary {
    let odd_scale = value.scale[0];
    let doubled: Magnitude = std::array::from_fn(|bit| {
        if bit == 0 {
            0
        } else {
            value.magnitude[bit - 1]
        }
    });
    let magnitude = mux_bits(odd_scale, &value.magnitude, &doubled);
    let scale = mux_bits(
        odd_scale,
        &value.scale,
        &add_bits(&value.scale, &signed_constant(1)),
    );
    let radicand = std::array::from_fn(|bit| {
        if bit >= 2 * SQRT_SHIFT && bit - 2 * SQRT_SHIFT < WIDTH {
            magnitude[bit - 2 * SQRT_SHIFT]
        } else {
            0
        }
    });
    let (mut root, remainder) = integer_sqrt(radicand);
    root[0] = or_gate(root[0], not_gate(is_zero(&remainder)));
    Binary {
        sign: 0,
        magnitude: root,
        scale: add_bits(&arithmetic_half(scale), &signed_constant(SQRT_SHIFT as i16)),
    }
}

fn pack(value: Binary, mut status: Status) -> Result {
    let nonzero = not_gate(is_zero(&value.magnitude));
    if nonzero == 0 {
        return special_zero(value.sign, status);
    }
    let (normal, normal_inexact) = round_precision(value, 24);
    let characteristic = sub_bits(&signed_constant(150), &normal.scale);
    let characteristic_positive = signed_greater(&characteristic, &signed_constant(0));
    let overflow = signed_greater(&characteristic, &signed_constant(254));
    if overflow == 1 {
        status.overflow = 1;
        status.inexact = 1;
        return special_infinity(value.sign, status);
    }
    if characteristic_positive == 1 {
        status.inexact = normal_inexact;
        let mut bits = [0; 32];
        bits[..23].copy_from_slice(&normal.magnitude[..23]);
        bits[23..31].copy_from_slice(&characteristic[..8]);
        bits[31] = value.sign;
        return Result {
            bits: bits_to_u32(&bits),
            status,
        };
    }
    let (fraction, inexact) = quantize(value.magnitude, value.scale, signed_constant(149));
    status.inexact = inexact;
    status.underflow = and_gate(inexact, nonzero);
    let mut bits = [0; 32];
    bits[..23].copy_from_slice(&fraction[..23]);
    // Rounding the largest subnormal can carry into the minimum normal.
    bits[23] = fraction[23];
    bits[31] = value.sign;
    Result {
        bits: bits_to_u32(&bits),
        status,
    }
}

fn round_precision(value: Binary, precision: usize) -> (Binary, u8) {
    let (highest, _) = priority_encode(&value.magnitude);
    let target = unsigned_constant::<SCALE_WIDTH>(precision - 1);
    let shift_right = unsigned_greater(&highest, &target);
    let shift_left = unsigned_greater(&target, &highest);
    let right_amount = sub_bits(&highest, &target);
    let left_amount = sub_bits(&target, &highest);
    let shifted = barrel_shift_right(&value.magnitude, &right_amount);
    let (rounded, inexact, carry) = round_shifted(
        &value.magnitude,
        shifted,
        right_amount,
        shift_right,
        precision,
    );
    let normalized_right: Magnitude =
        std::array::from_fn(|bit| if bit + 1 < WIDTH { rounded[bit + 1] } else { 0 });
    let rounded = mux_bits(carry, &rounded, &normalized_right);
    let shifted_left = barrel_shift_left(&value.magnitude, &left_amount);
    let magnitude = mux_bits(
        shift_right,
        &mux_bits(shift_left, &value.magnitude, &shifted_left),
        &rounded,
    );
    let right_scale = sub_bits(&value.scale, &right_amount);
    let right_scale = mux_bits(
        carry,
        &right_scale,
        &sub_bits(&right_scale, &signed_constant(1)),
    );
    let left_scale = add_bits(&value.scale, &left_amount);
    let scale = mux_bits(
        shift_right,
        &mux_bits(shift_left, &value.scale, &left_scale),
        &right_scale,
    );
    (
        Binary {
            sign: value.sign,
            magnitude,
            scale,
        },
        inexact,
    )
}

fn quantize(magnitude: Magnitude, scale: Scale, target: Scale) -> (Magnitude, u8) {
    let right = signed_greater(&scale, &target);
    let left = signed_greater(&target, &scale);
    let right_amount = sub_bits(&scale, &target);
    let left_amount = sub_bits(&target, &scale);
    let shifted = barrel_shift_right(&magnitude, &right_amount);
    let (rounded, inexact, _) = round_shifted(&magnitude, shifted, right_amount, right, WIDTH - 1);
    let shifted_left = barrel_shift_left(&magnitude, &left_amount);
    (
        mux_bits(right, &mux_bits(left, &magnitude, &shifted_left), &rounded),
        inexact,
    )
}

fn round_shifted(
    original: &Magnitude,
    shifted: Magnitude,
    amount: Scale,
    enabled: u8,
    carry_bit: usize,
) -> (Magnitude, u8, u8) {
    let one = unsigned_constant::<SCALE_WIDTH>(1);
    let guard_amount = sub_bits(&amount, &one);
    let guard_vector = barrel_shift_right(original, &guard_amount);
    let guard = and_gate(enabled, guard_vector[0]);
    let sticky = original
        .iter()
        .copied()
        .enumerate()
        .fold(0, |acc, (bit, input)| {
            let below = unsigned_greater(&guard_amount, &unsigned_constant(bit));
            or_gate(acc, and_gate(input, and_gate(enabled, below)))
        });
    let inexact = or_gate(guard, sticky);
    let round_up = and_gate(guard, or_gate(sticky, shifted[0]));
    let mut increment = [0; WIDTH];
    increment[0] = round_up;
    let rounded = add_bits(&shifted, &increment);
    let carry = if carry_bit < WIDTH {
        rounded[carry_bit]
    } else {
        0
    };
    (rounded, inexact, carry)
}

fn divide_magnitude(numerator: Magnitude, denominator: Magnitude) -> (Magnitude, [u8; WIDTH + 1]) {
    let denominator: [u8; WIDTH + 1] =
        std::array::from_fn(|bit| if bit < WIDTH { denominator[bit] } else { 0 });
    let mut remainder = [0; WIDTH + 1];
    let mut quotient = [0; WIDTH];
    for bit in (0..WIDTH).rev() {
        for index in (1..=WIDTH).rev() {
            remainder[index] = remainder[index - 1];
        }
        remainder[0] = numerator[bit];
        let subtract = not_gate(unsigned_greater(&denominator, &remainder));
        remainder = mux_bits(subtract, &remainder, &sub_bits(&remainder, &denominator));
        quotient[bit] = subtract;
    }
    (quotient, remainder)
}

fn integer_sqrt(radicand: Magnitude) -> (Magnitude, [u8; WIDTH + 2]) {
    let mut remainder = [0; WIDTH + 2];
    let mut root = [0; WIDTH];
    for pair in (0..WIDTH / 2).rev() {
        for bit in (2..WIDTH + 2).rev() {
            remainder[bit] = remainder[bit - 2];
        }
        remainder[0] = radicand[pair * 2];
        remainder[1] = radicand[pair * 2 + 1];
        let mut shifted_root = [0; WIDTH + 2];
        for bit in 0..WIDTH {
            if bit + 2 < WIDTH + 2 {
                shifted_root[bit + 2] = root[bit];
            }
        }
        shifted_root[0] = 1;
        let take = not_gate(unsigned_greater(&shifted_root, &remainder));
        remainder = mux_bits(take, &remainder, &sub_bits(&remainder, &shifted_root));
        for bit in (1..WIDTH).rev() {
            root[bit] = root[bit - 1];
        }
        root[0] = take;
    }
    (root, remainder)
}

fn finite_less(left: Parts, right: Parts) -> u8 {
    let sign_diff = xor_gate(left.sign, right.sign);
    let magnitude_less = unsigned_greater(
        &join_exp_fraction(right.exponent, right.fraction),
        &join_exp_fraction(left.exponent, left.fraction),
    );
    let same_sign_less = mux_bit(left.sign, magnitude_less, not_gate(magnitude_less));
    mux_bit(sign_diff, same_sign_less, left.sign)
}

fn equal_bits(left: Parts, right: Parts) -> u8 {
    let same_sign = not_gate(xor_gate(left.sign, right.sign));
    let left = join_exp_fraction(left.exponent, left.fraction);
    let right = join_exp_fraction(right.exponent, right.fraction);
    and_gate(
        same_sign,
        and_gate(
            not_gate(unsigned_greater(&left, &right)),
            not_gate(unsigned_greater(&right, &left)),
        ),
    )
}

fn join_exp_fraction(exponent: [u8; 8], fraction: [u8; 23]) -> [u8; 31] {
    std::array::from_fn(|bit| {
        if bit < 23 {
            fraction[bit]
        } else {
            exponent[bit - 23]
        }
    })
}

fn special_infinity(sign: u8, status: Status) -> Result {
    Result {
        bits: 0x7f80_0000 | (u32::from(sign) << 31),
        status,
    }
}

fn special_zero(sign: u8, status: Status) -> Result {
    Result {
        bits: u32::from(sign) << 31,
        status,
    }
}

impl Binary {
    fn without_negative_zero(mut self) -> Self {
        self.sign = and_gate(self.sign, not_gate(is_zero(&self.magnitude)));
        self
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
        let shifted = std::array::from_fn(|bit| {
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
        let shifted = std::array::from_fn(|bit| {
            if bit + distance < WIDTH {
                result[bit + distance]
            } else {
                0
            }
        });
        result = mux_bits(select, &result, &shifted);
    }
    result
}

fn arithmetic_half(value: Scale) -> Scale {
    std::array::from_fn(|bit| {
        if bit + 1 < SCALE_WIDTH {
            value[bit + 1]
        } else {
            value[SCALE_WIDTH - 1]
        }
    })
}

fn add_bits<const N: usize>(left: &[u8; N], right: &[u8; N]) -> [u8; N] {
    ripple_carry_adder_with_carry(left, right, 0)
        .sum
        .try_into()
        .expect("a ripple network preserves width")
}

fn sub_bits<const N: usize>(left: &[u8; N], right: &[u8; N]) -> [u8; N] {
    ripple_carry_adder_with_carry(left, &right.map(not_gate), 1)
        .sum
        .try_into()
        .expect("a ripple network preserves width")
}

fn mux_bits<const N: usize>(select: u8, d0: &[u8; N], d1: &[u8; N]) -> [u8; N] {
    std::array::from_fn(|bit| mux_bit(select, d0[bit], d1[bit]))
}

fn mux_bit(select: u8, d0: u8, d1: u8) -> u8 {
    or_gate(and_gate(not_gate(select), d0), and_gate(select, d1))
}

fn unsigned_greater<const N: usize>(left: &[u8; N], right: &[u8; N]) -> u8 {
    let mut greater = 0;
    let mut equal = 1;
    for bit in (0..N).rev() {
        greater = or_gate(
            greater,
            and_gate(equal, and_gate(left[bit], not_gate(right[bit]))),
        );
        equal = and_gate(equal, not_gate(xor_gate(left[bit], right[bit])));
    }
    greater
}

fn signed_greater(left: &Scale, right: &Scale) -> u8 {
    let sign_diff = xor_gate(left[SCALE_WIDTH - 1], right[SCALE_WIDTH - 1]);
    let left_positive = and_gate(not_gate(left[SCALE_WIDTH - 1]), right[SCALE_WIDTH - 1]);
    mux_bit(sign_diff, unsigned_greater(left, right), left_positive)
}

fn is_zero<const N: usize>(bits: &[u8; N]) -> u8 {
    not_gate(bits.iter().copied().fold(0, or_gate))
}

fn bits_from_u32(value: u32) -> [u8; 32] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn bits_to_u32(bits: &[u8; 32]) -> u32 {
    bits.iter()
        .copied()
        .enumerate()
        .fold(0, |value, (bit, wire)| value | (u32::from(wire) << bit))
}

fn unsigned_constant<const N: usize>(value: usize) -> [u8; N] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn signed_constant(value: i16) -> Scale {
    let encoded = value as u16;
    std::array::from_fn(|bit| ((encoded >> bit) & 1) as u8)
}

fn extend<const FROM: usize, const TO: usize>(value: &[u8; FROM]) -> [u8; TO] {
    std::array::from_fn(|bit| if bit < FROM { value[bit] } else { 0 })
}
