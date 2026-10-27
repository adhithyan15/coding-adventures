//! Fixed-width Cortex-M4 extension networks.
//!
//! Host loops describe repeated wiring. Architectural values, flags, selects,
//! and write-enables emerge from repository gates and ripple networks.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

#[must_use]
pub fn u32_to_bits(value: u32) -> [u8; 32] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

#[must_use]
pub fn bits_to_u32(bits: &[u8]) -> u32 {
    bits.iter()
        .take(32)
        .enumerate()
        .fold(0, |value, (bit, wire)| value | (u32::from(*wire) << bit))
}

#[must_use]
pub fn select_bit(d0: u8, d1: u8, select: u8) -> u8 {
    mux2(d0, d1, select)
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
pub fn not_32(value: u32) -> u32 {
    bits_to_u32(&u32_to_bits(value).map(not_gate))
}

#[must_use]
pub fn add_32(left: u32, right: u32, carry: u8) -> (u32, u8, u8) {
    let left = u32_to_bits(left);
    let right = u32_to_bits(right);
    let full = ripple_carry_adder_with_carry(&left, &right, carry);
    let low = ripple_carry_adder_with_carry(&left[..31], &right[..31], carry);
    (
        bits_to_u32(&full.sum),
        full.carry_out,
        xor_gate(low.carry_out, full.carry_out),
    )
}

#[must_use]
pub fn sub_32(left: u32, right: u32) -> (u32, u8, u8) {
    let (value, carry, overflow) = add_32(left, not_32(right), 1);
    (value, not_gate(carry), overflow)
}

#[must_use]
pub fn is_zero_32(value: u32) -> u8 {
    not_gate(u32_to_bits(value).into_iter().fold(0, or_gate))
}

#[must_use]
pub fn equal_32(left: u32, right: u32) -> u8 {
    let left = u32_to_bits(left);
    let right = u32_to_bits(right);
    not_gate(
        left.into_iter()
            .zip(right)
            .map(|(a, b)| xor_gate(a, b))
            .fold(0, or_gate),
    )
}

#[must_use]
pub fn unsigned_less_32(left: u32, right: u32) -> u8 {
    sub_32(left, right).1
}

#[must_use]
pub fn signed_less_32(left: u32, right: u32) -> u8 {
    let left_bits = u32_to_bits(left);
    let right_bits = u32_to_bits(right);
    let signs_differ = xor_gate(left_bits[31], right_bits[31]);
    mux2(unsigned_less_32(left, right), left_bits[31], signs_differ)
}

#[must_use]
pub fn shift_left_32(mut value: u32, amount: u32) -> u32 {
    let amount = u32_to_bits(amount);
    for (stage, select) in amount.into_iter().take(5).enumerate() {
        let distance = 1 << stage;
        let input = u32_to_bits(value);
        let shifted = bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
            if bit >= distance {
                input[bit - distance]
            } else {
                0
            }
        }));
        value = select_32(value, shifted, select);
    }
    value
}

#[must_use]
pub fn shift_arithmetic_right_32(mut value: u32, amount: u32) -> u32 {
    let amount = u32_to_bits(amount);
    for (stage, select) in amount.into_iter().take(5).enumerate() {
        let distance = 1 << stage;
        let input = u32_to_bits(value);
        let shifted = bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
            if bit + distance < 32 {
                input[bit + distance]
            } else {
                input[31]
            }
        }));
        value = select_32(value, shifted, select);
    }
    value
}

/// Saturating signed add/subtract. The returned select is the sticky-Q input.
#[must_use]
pub fn saturating_add(left: u32, right: u32, subtract: u8) -> (u32, u8) {
    let (sum, _, add_overflow) = add_32(left, right, 0);
    let (difference, _, sub_overflow) = sub_32(left, right);
    let value = select_32(sum, difference, subtract);
    let overflow = mux2(add_overflow, sub_overflow, subtract);
    let left_sign = u32_to_bits(left)[31];
    let saturation = select_32(0x7fff_ffff, 0x8000_0000, left_sign);
    (select_32(value, saturation, overflow), overflow)
}

/// Gate-select signed saturation to `width` bits (1..=32).
#[must_use]
pub fn signed_saturate(value: u32, width: u32) -> (u32, u8) {
    let amount = add_32(32, not_32(width), 1).0;
    let narrowed = shift_arithmetic_right_32(shift_left_32(value, amount), amount);
    let changed = not_gate(equal_32(value, narrowed));
    let sign = u32_to_bits(value)[31];
    let one = shift_left_32(1, add_32(width, u32::MAX, 0).0);
    let maximum = add_32(one, u32::MAX, 0).0;
    let minimum = not_32(maximum);
    let bounded = select_32(maximum, minimum, sign);
    (select_32(value, bounded, changed), changed)
}

/// Gate-select unsigned saturation to `width` bits (0..=31).
#[must_use]
pub fn unsigned_saturate(value: u32, width: u32) -> (u32, u8) {
    let sign = u32_to_bits(value)[31];
    let maximum = add_32(shift_left_32(1, width), u32::MAX, 0).0;
    let too_large = unsigned_less_32(maximum, value);
    let positive = select_32(value, maximum, too_large);
    let result = select_32(positive, 0, sign);
    (result, or_gate(sign, too_large))
}

/// Four independent byte-lane additions and the Arm GE lane wires.
#[must_use]
pub fn parallel_add8(left: u32, right: u32, unsigned: u8) -> (u32, u8) {
    let left = u32_to_bits(left);
    let right = u32_to_bits(right);
    let mut result = [0; 32];
    let mut ge = [0; 4];
    for (lane, ge_wire) in ge.iter_mut().enumerate() {
        let start = lane * 8;
        let sum =
            ripple_carry_adder_with_carry(&left[start..start + 8], &right[start..start + 8], 0);
        result[start..start + 8].copy_from_slice(&sum.sum);
        let left_signed: [u8; 9] = std::array::from_fn(|bit| {
            if bit < 8 {
                left[start + bit]
            } else {
                left[start + 7]
            }
        });
        let right_signed: [u8; 9] = std::array::from_fn(|bit| {
            if bit < 8 {
                right[start + bit]
            } else {
                right[start + 7]
            }
        });
        let signed_sum = ripple_carry_adder_with_carry(&left_signed, &right_signed, 0);
        let signed_nonnegative = not_gate(signed_sum.sum[8]);
        *ge_wire = mux2(signed_nonnegative, sum.carry_out, unsigned);
    }
    (
        bits_to_u32(&result),
        ge.into_iter()
            .enumerate()
            .fold(0, |value, (lane, bit)| value | (bit << lane)),
    )
}

fn sign_extend_half(value: u32, upper: usize) -> [u8; 64] {
    let source = u32_to_bits(value);
    let start = upper * 16;
    let sign = source[start + 15];
    std::array::from_fn(|bit| if bit < 16 { source[start + bit] } else { sign })
}

fn add_64(left: &[u8; 64], right: &[u8; 64]) -> [u8; 64] {
    ripple_carry_adder_with_carry(left, right, 0)
        .sum
        .try_into()
        .expect("a fixed-width ripple adder preserves width")
}

fn multiply_signed_half(left: u32, right: u32, upper: usize) -> [u8; 64] {
    let left = sign_extend_half(left, upper);
    let right = sign_extend_half(right, upper);
    let left_sign = left[63];
    let right_sign = right[63];
    let left_abs = twos_complement_64(&left, left_sign);
    let right_abs = twos_complement_64(&right, right_sign);
    let mut product = [0; 64];
    for multiplier in 0..16 {
        let partial = std::array::from_fn(|bit| {
            if bit >= multiplier {
                and_gate(left_abs[bit - multiplier], right_abs[multiplier])
            } else {
                0
            }
        });
        product = add_64(&product, &partial);
    }
    twos_complement_64(&product, xor_gate(left_sign, right_sign))
}

fn twos_complement_64(value: &[u8; 64], select: u8) -> [u8; 64] {
    let inverted = value.map(not_gate);
    let mut one = [0; 64];
    one[0] = 1;
    let negative = add_64(&inverted, &one);
    std::array::from_fn(|bit| mux2(value[bit], negative[bit], select))
}

/// Dual signed-halfword multiply with an optional signed accumulator.
#[must_use]
pub fn dual_multiply_add(left: u32, right: u32, accumulator: u32, add_acc: u8) -> (u32, u8) {
    let low = multiply_signed_half(left, right, 0);
    let high = multiply_signed_half(left, right, 1);
    let products = add_64(&low, &high);
    let accumulator = sign_extend_halfword32(accumulator);
    let accumulated = add_64(
        &products,
        &std::array::from_fn(|bit| mux2(0, accumulator[bit], add_acc)),
    );
    let overflow = accumulated[32..]
        .iter()
        .copied()
        .map(|bit| xor_gate(bit, accumulated[31]))
        .fold(0, or_gate);
    (bits_to_u32(&accumulated[..32]), overflow)
}

fn sign_extend_halfword32(value: u32) -> [u8; 64] {
    let value = u32_to_bits(value);
    std::array::from_fn(|bit| if bit < 32 { value[bit] } else { value[31] })
}

/// Gate equality against a masked 16-bit instruction constant.
#[must_use]
pub fn masked_eq16(value: u16, mask: u16, expected: u16) -> u8 {
    (0..16).fold(1, |selected, bit| {
        let care = ((mask >> bit) & 1) as u8;
        let actual = ((value >> bit) & 1) as u8;
        let wanted = ((expected >> bit) & 1) as u8;
        let equal = not_gate(xor_gate(actual, wanted));
        and_gate(selected, mux2(1, equal, care))
    })
}

/// Pack a gate-observed contiguous instruction field for bounded indexing.
#[must_use]
pub fn field_u16(value: u16, low: usize, width: usize) -> u8 {
    (0..width).fold(0, |packed, output| {
        let wire = ((value >> (low + output)) & 1) as u8;
        packed | (wire << output)
    })
}
