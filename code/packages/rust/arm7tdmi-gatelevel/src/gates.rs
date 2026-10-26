//! Fixed-width combinational networks used by the ARM7TDMI transition engine.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xnor_gate, xor_gate};

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

/// Assemble two little-endian bytes through fixed wiring.
#[must_use]
pub fn assemble_le_u16(bytes: [u8; 2]) -> u16 {
    let wires = bytes.map(u8_to_bits);
    bits_to_u16(&std::array::from_fn::<_, 16, _>(|bit| {
        wires[bit / 8][bit % 8]
    }))
}

/// Assemble four little-endian bytes through fixed wiring.
#[must_use]
pub fn assemble_le_u32(bytes: [u8; 4]) -> u32 {
    let wires = bytes.map(u8_to_bits);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        wires[bit / 8][bit % 8]
    }))
}

/// Decompose a halfword to little-endian bytes through fixed wiring.
#[must_use]
pub fn split_le_u16(value: u16) -> [u8; 2] {
    let wires = u16_to_bits(value);
    std::array::from_fn(|byte| bits_to_u8(&wires[byte * 8..byte * 8 + 8]))
}

/// Decompose a word to little-endian bytes through fixed wiring.
#[must_use]
pub fn split_le_u32(value: u32) -> [u8; 4] {
    let wires = u32_to_bits(value);
    std::array::from_fn(|byte| bits_to_u8(&wires[byte * 8..byte * 8 + 8]))
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

/// Gate equality after a fixed mask. Each bit is qualified independently;
/// there is no host integer equality decision in the decoder.
#[must_use]
pub(crate) fn masked_eq_32(value: u32, mask: u32, expected: u32) -> u8 {
    let value = u32_to_bits(value);
    let mask = u32_to_bits(mask);
    let expected = u32_to_bits(expected);
    (0..32).fold(1, |line, bit| {
        let equal = xnor_gate(value[bit], expected[bit]);
        and_gate(line, mux2(1, equal, mask[bit]))
    })
}

#[must_use]
pub(crate) fn masked_eq_16(value: u16, mask: u16, expected: u16) -> u8 {
    let value = u16_to_bits(value);
    let mask = u16_to_bits(mask);
    let expected = u16_to_bits(expected);
    (0..16).fold(1, |line, bit| {
        let equal = xnor_gate(value[bit], expected[bit]);
        and_gate(line, mux2(1, equal, mask[bit]))
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

/// ARM's sixteen condition inputs selected by gate-derived one-hot lines.
#[must_use]
pub fn condition_holds(cpsr: u32, condition: u8) -> u8 {
    let bits = u32_to_bits(cpsr);
    let n = bits[31];
    let z = bits[30];
    let c = bits[29];
    let v = bits[28];
    let ge = xnor_gate(n, v);
    let lt = xor_gate(n, v);
    let predicates = [
        z,
        not_gate(z),
        c,
        not_gate(c),
        n,
        not_gate(n),
        v,
        not_gate(v),
        and_gate(c, not_gate(z)),
        or_gate(not_gate(c), z),
        ge,
        lt,
        and_gate(not_gate(z), ge),
        or_gate(z, lt),
        1,
        0,
    ];
    predicates
        .into_iter()
        .enumerate()
        .fold(0, |result, (index, predicate)| {
            or_gate(
                result,
                and_gate(predicate, selected(u32::from(condition), index as u32, 4)),
            )
        })
}

#[must_use]
pub fn status_with_nz(cpsr: u32, value: u32) -> u32 {
    let mut status = u32_to_bits(cpsr);
    let value = u32_to_bits(value);
    status[31] = value[31];
    status[30] = not_gate(value.into_iter().fold(0, or_gate));
    bits_to_u32(&status)
}

#[must_use]
pub fn status_with_nzc(cpsr: u32, value: u32, carry: u8) -> u32 {
    let mut status = u32_to_bits(status_with_nz(cpsr, value));
    status[29] = carry;
    bits_to_u32(&status)
}

#[must_use]
pub fn status_with_nzcv(cpsr: u32, value: u32, carry: u8, overflow: u8) -> u32 {
    let mut status = u32_to_bits(status_with_nzc(cpsr, value, carry));
    status[28] = overflow;
    bits_to_u32(&status)
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
pub fn select_bit_32(value: u32, index: u32) -> u8 {
    let bits = u32_to_bits(value);
    bits.into_iter().enumerate().fold(0, |line, (bit, input)| {
        or_gate(line, and_gate(input, selected(index, bit as u32, 32)))
    })
}

/// Complete ARM barrel-shifter result/carry network. Runtime kind and amount
/// only select fixed candidates through one-hot gate lines.
#[must_use]
pub fn arm_shift(
    value: u32,
    kind: u32,
    amount: u32,
    immediate: bool,
    old_carry: bool,
) -> (u32, bool) {
    let mut outputs = [0u32; 4];
    let mut carries = [0u8; 4];
    for shift_kind in 0..4u32 {
        let mut result = 0;
        let mut carry = 0;
        for encoded in 0..256u32 {
            let selected_amount = selected(amount, encoded, 8);
            let effective = if immediate && encoded == 0 && (shift_kind == 1 || shift_kind == 2) {
                32
            } else {
                encoded
            };
            let (candidate, candidate_carry) = match shift_kind {
                0 if effective == 0 => (value, u8::from(old_carry)),
                0 if effective < 32 => (
                    shift_left_32(value, effective),
                    select_bit_32(value, 32 - effective),
                ),
                0 if effective == 32 => (0, select_bit_32(value, 0)),
                0 => (0, 0),
                1 if effective == 0 => (value, u8::from(old_carry)),
                1 if effective < 32 => (
                    shift_right_32(value, effective),
                    select_bit_32(value, effective - 1),
                ),
                1 if effective == 32 => (0, select_bit_32(value, 31)),
                1 => (0, 0),
                2 if effective == 0 => (value, u8::from(old_carry)),
                2 if effective < 32 => (
                    shift_arithmetic_right_32(value, effective),
                    select_bit_32(value, effective - 1),
                ),
                2 => {
                    let sign = select_bit_32(value, 31);
                    (select_32(0, u32::MAX, sign), sign)
                }
                3 if immediate && encoded == 0 => {
                    let top = select_32(0, 0x8000_0000, u8::from(old_carry));
                    (
                        bit_or_32(top, shift_right_32(value, 1)),
                        select_bit_32(value, 0),
                    )
                }
                3 if effective == 0 => (value, u8::from(old_carry)),
                3 => {
                    let rotate = effective & 31;
                    let c = if rotate == 0 {
                        select_bit_32(value, 31)
                    } else {
                        select_bit_32(value, rotate - 1)
                    };
                    (rotate_right_32(value, rotate), c)
                }
                _ => unreachable!("four shift kinds"),
            };
            result = select_32(result, candidate, selected_amount);
            carry = mux2(carry, candidate_carry, selected_amount);
        }
        outputs[shift_kind as usize] = result;
        carries[shift_kind as usize] = carry;
    }
    let kind_lines = std::array::from_fn::<_, 4, _>(|index| selected(kind, index as u32, 2));
    let result = (0..4).fold(0, |out, index| {
        bit_or_32(out, select_32(0, outputs[index], kind_lines[index]))
    });
    let carry = (0..4).fold(0, |out, index| {
        or_gate(out, and_gate(carries[index], kind_lines[index]))
    });
    (result, carry == 1)
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

fn u64_to_bits(value: u64) -> [u8; 64] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn bits_to_u64(bits: &[u8]) -> u64 {
    bits.iter()
        .take(64)
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u64::from(*q) << bit))
}

#[must_use]
pub fn add_64(left: u64, right: u64) -> u64 {
    bits_to_u64(&ripple_carry_adder_with_carry(&u64_to_bits(left), &u64_to_bits(right), 0).sum)
}

#[must_use]
pub fn multiply_unsigned_32(left: u32, right: u32) -> u64 {
    let left = u64::from(left);
    let right_bits = u32_to_bits(right);
    let mut product = 0;
    for (bit, selected) in right_bits.into_iter().enumerate() {
        let partial = if selected == 1 { left << bit } else { 0 };
        product = add_64(product, partial);
    }
    product
}

#[must_use]
pub fn multiply_signed_32(left: u32, right: u32) -> u64 {
    let left_negative = select_bit_32(left, 31);
    let right_negative = select_bit_32(right, 31);
    let left_magnitude = select_32(left, add_32(bit_not_32(left), 1, 0).0, left_negative);
    let right_magnitude = select_32(right, add_32(bit_not_32(right), 1, 0).0, right_negative);
    let magnitude = multiply_unsigned_32(left_magnitude, right_magnitude);
    let negative = xor_gate(left_negative, right_negative);
    let negated = add_64(!magnitude, 1);
    let m = u64_to_bits(magnitude);
    let n = u64_to_bits(negated);
    bits_to_u64(&std::array::from_fn::<_, 64, _>(|bit| {
        mux2(m[bit], n[bit], negative)
    }))
}

#[must_use]
pub fn is_zero_64(value: u64) -> u8 {
    not_gate(u64_to_bits(value).into_iter().fold(0, or_gate))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn condition_oracle(cpsr: u32, condition: u8) -> u8 {
        let n = cpsr >> 31 != 0;
        let z = cpsr >> 30 & 1 != 0;
        let c = cpsr >> 29 & 1 != 0;
        let v = cpsr >> 28 & 1 != 0;
        u8::from(match condition {
            0 => z,
            1 => !z,
            2 => c,
            3 => !c,
            4 => n,
            5 => !n,
            6 => v,
            7 => !v,
            8 => c && !z,
            9 => !c || z,
            10 => n == v,
            11 => n != v,
            12 => !z && n == v,
            13 => z || n != v,
            14 => true,
            15 => false,
            _ => false,
        })
    }

    #[test]
    fn condition_network_exhausts_all_nzcv_and_selector_values() {
        for flags in 0..16u32 {
            let cpsr = flags << 28;
            for condition in 0..16u8 {
                assert_eq!(
                    condition_holds(cpsr, condition),
                    condition_oracle(cpsr, condition)
                );
            }
        }
    }

    #[test]
    fn bit_selector_fails_closed_outside_the_word() {
        let value = 0x8000_0001;
        assert_eq!(select_bit_32(value, 0), 1);
        assert_eq!(select_bit_32(value, 31), 1);
        for index in [32, 33, u32::MAX] {
            assert_eq!(select_bit_32(value, index), 0);
        }
    }

    #[test]
    fn multiplier_edges_match_widened_oracles() {
        for left in [0, 1, u32::MAX, 0x8000_0000, 0x7fff_ffff, 0x1234_5678] {
            for right in [0, 1, u32::MAX, 0x8000_0000, 0x7fff_ffff, 0x89ab_cdef] {
                assert_eq!(
                    multiply_unsigned_32(left, right),
                    u64::from(left) * u64::from(right)
                );
                assert_eq!(
                    multiply_signed_32(left, right),
                    ((i64::from(left as i32) * i64::from(right as i32)) as u64)
                );
            }
        }
    }
}
