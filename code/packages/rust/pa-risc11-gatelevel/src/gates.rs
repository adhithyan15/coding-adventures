//! PA-RISC combinational networks. Bit zero is always the least-significant bit.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

pub(crate) fn u32_bits(value: u32) -> [u8; 32] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

pub(crate) fn bits_u32(bits: &[u8]) -> u32 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u32::from(*q) << bit))
}

pub(crate) fn selected(value: u32, expected: u32, width: usize) -> u8 {
    let value = u32_bits(value);
    let expected = u32_bits(expected);
    (0..width).fold(1, |line, bit| {
        let signal = mux2(not_gate(value[bit]), value[bit], expected[bit]);
        and_gate(line, signal)
    })
}

pub(crate) fn decode6(value: u8) -> [u8; 64] {
    let bits: [u8; 6] = std::array::from_fn(|bit| (value >> bit) & 1);
    std::array::from_fn(|line| {
        (0..6).fold(1, |enabled, bit| {
            let wanted = ((line >> bit) & 1) as u8;
            and_gate(enabled, mux2(not_gate(bits[bit]), bits[bit], wanted))
        })
    })
}

pub(crate) fn not32(value: u32) -> u32 {
    bits_u32(&u32_bits(value).map(not_gate))
}

pub(crate) fn and32(left: u32, right: u32) -> u32 {
    let left = u32_bits(left);
    let right = u32_bits(right);
    bits_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        and_gate(left[bit], right[bit])
    }))
}

pub(crate) fn or32(left: u32, right: u32) -> u32 {
    let left = u32_bits(left);
    let right = u32_bits(right);
    bits_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        or_gate(left[bit], right[bit])
    }))
}

pub(crate) fn xor32(left: u32, right: u32) -> u32 {
    let left = u32_bits(left);
    let right = u32_bits(right);
    bits_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        xor_gate(left[bit], right[bit])
    }))
}

pub(crate) fn mux32(zero: u32, one: u32, select: u8) -> u32 {
    let zero = u32_bits(zero);
    let one = u32_bits(one);
    bits_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        mux2(zero[bit], one[bit], select)
    }))
}

/// Gate-derived sum, carry-out, overflow, and architectural nibble carries.
pub(crate) fn add32(left: u32, right: u32, carry_in: u8) -> (u32, u8, u8, [u8; 8]) {
    let left_bits = u32_bits(left);
    let right_bits = u32_bits(right);
    let full = ripple_carry_adder_with_carry(&left_bits, &right_bits, carry_in);
    let low = ripple_carry_adder_with_carry(&left_bits[..31], &right_bits[..31], carry_in);
    let nibble = std::array::from_fn(|architectural| {
        let little_nibble = 7 - architectural;
        ripple_carry_adder_with_carry(
            &left_bits[..little_nibble * 4 + 4],
            &right_bits[..little_nibble * 4 + 4],
            carry_in,
        )
        .carry_out
    });
    (
        bits_u32(&full.sum),
        full.carry_out,
        xor_gate(low.carry_out, full.carry_out),
        nibble,
    )
}

pub(crate) fn sub32(left: u32, right: u32) -> (u32, u8, u8, [u8; 8]) {
    let (value, no_borrow, overflow, carries) = add32(left, not32(right), 1);
    (value, no_borrow, overflow, carries)
}

pub(crate) fn zero32(value: u32) -> u8 {
    not_gate(u32_bits(value).into_iter().fold(0, or_gate))
}

pub(crate) fn register_mux(registers: &[u32; 32], index: u8) -> u32 {
    let lines = decode6(index & 31);
    (1..32).fold(0, |value, register| {
        or32(
            value,
            and32(registers[register], mux32(0, u32::MAX, lines[register])),
        )
    })
}

pub(crate) fn register_enables(index: u8, write: u8) -> [u8; 32] {
    let lines = decode6(index & 31);
    std::array::from_fn(|register| {
        let nonzero = not_gate(selected(register as u32, 0, 5));
        and_gate(and_gate(lines[register], write), nonzero)
    })
}

pub(crate) fn sign_extend_low(field: u32, width: usize) -> u32 {
    let source = u32_bits(field);
    let sign = source[0];
    let assembled: [u8; 32] = std::array::from_fn(|bit| {
        if bit + 1 < width {
            source[bit + 1]
        } else {
            sign
        }
    });
    bits_u32(&assembled)
}

pub(crate) fn assemble21(field: u32) -> u32 {
    let source = u32_bits(field);
    let assembled: [u8; 32] = std::array::from_fn(|bit| match bit {
        20 => source[0],
        9..=19 => source[bit - 8],
        7..=8 => source[bit + 7],
        2..=6 => source[bit + 14],
        0..=1 => source[bit + 12],
        _ => 0,
    });
    bits_u32(&assembled)
}

pub(crate) fn branch_disp12(word: u32) -> u32 {
    let w = u32_bits(word);
    let assembled: [u8; 32] = std::array::from_fn(|bit| match bit {
        0..=1 => 0,
        2..=11 => w[bit + 1],
        12 => w[2],
        _ => w[0],
    });
    bits_u32(&assembled)
}

pub(crate) fn branch_disp17(word: u32) -> u32 {
    let w = u32_bits(word);
    let assembled: [u8; 32] = std::array::from_fn(|bit| match bit {
        0..=1 => 0,
        2..=11 => w[bit + 1],
        12 => w[2],
        13..=17 => w[bit + 3],
        _ => w[0],
    });
    bits_u32(&assembled)
}

pub(crate) fn shift_left(mut value: u32, amount: u32) -> u32 {
    for stage in 0..5 {
        value = mux32(
            value,
            value.wrapping_shl(1 << stage),
            ((amount >> stage) & 1) as u8,
        );
    }
    value
}

pub(crate) fn shift_right(mut value: u32, amount: u32) -> u32 {
    for stage in 0..5 {
        value = mux32(
            value,
            value.wrapping_shr(1 << stage),
            ((amount >> stage) & 1) as u8,
        );
    }
    value
}

pub(crate) fn shd(left: u32, right: u32, count: u32) -> u32 {
    let right_part = shift_right(right, count);
    let complement = and32(not32(count), 31);
    let left_part = shift_left(left, add32(complement, 1, 0).0);
    mux32(or32(left_part, right_part), right, selected(count, 0, 5))
}

pub(crate) fn condition(lines: [u8; 8], selector: u8) -> u8 {
    let select = decode6(selector & 7);
    (0..8).fold(0, |value, line| {
        or_gate(value, and_gate(lines[line], select[line]))
    })
}

pub(crate) fn compare_condition(selector: u8, left: u32, right: u32) -> u8 {
    let (result, no_borrow, overflow, _) = sub32(left, right);
    let zero = zero32(result);
    let signed_less = xor_gate(u32_bits(result)[31], overflow);
    condition(
        [
            0,
            zero,
            signed_less,
            or_gate(zero, signed_less),
            not_gate(no_borrow),
            or_gate(zero, not_gate(no_borrow)),
            overflow,
            u32_bits(result)[0],
        ],
        selector,
    )
}

pub(crate) fn add_condition(selector: u8, result: u32, carry: u8, overflow: u8) -> u8 {
    let zero = zero32(result);
    let negative_xor_overflow = xor_gate(u32_bits(result)[31], overflow);
    condition(
        [
            0,
            zero,
            negative_xor_overflow,
            or_gate(zero, negative_xor_overflow),
            not_gate(carry),
            or_gate(zero, not_gate(carry)),
            overflow,
            u32_bits(result)[0],
        ],
        selector,
    )
}

#[cfg(test)]
mod tests {
    use super::{add32, not32, sub32};

    #[test]
    fn every_nibble_truth_table_and_carry_input_is_exact() {
        for nibble in 0..8 {
            let shift = nibble * 4;
            for left_nibble in 0..16_u32 {
                for right_nibble in 0..16_u32 {
                    for incoming in 0..=1_u8 {
                        let (low_left, low_right, carry_in) = if nibble == 0 {
                            (0, 0, incoming)
                        } else if incoming == 1 {
                            ((1_u32 << shift) - 1, 1, 0)
                        } else {
                            (0, 0, 0)
                        };
                        let left = (left_nibble << shift) | low_left;
                        let right = (right_nibble << shift) | low_right;
                        let (_, _, _, carries) = add32(left, right, carry_in);
                        let expected = left_nibble + right_nibble + u32::from(incoming) > 15;
                        assert_eq!(carries[7 - nibble], u8::from(expected));
                    }
                }
            }
        }
    }

    #[test]
    fn every_nibble_subtract_truth_table_borrow_and_cb_order_is_exact() {
        for nibble in 0..8 {
            let shift = nibble * 4;
            for left_nibble in 0..16_u32 {
                for right_nibble in 0..16_u32 {
                    for borrow_in in 0..=1_u8 {
                        let (low_left, low_right, carry_in) = if nibble == 0 {
                            (0, 0, 1 - borrow_in)
                        } else if borrow_in == 1 {
                            (0, 1, 1)
                        } else {
                            (0, 0, 1)
                        };
                        let left = (left_nibble << shift) | low_left;
                        let right = (right_nibble << shift) | low_right;
                        let (result, _, _, carries) = add32(left, not32(right), carry_in);
                        let expected_nibble = left_nibble
                            .wrapping_sub(right_nibble)
                            .wrapping_sub(u32::from(borrow_in))
                            & 15;
                        let expected_no_borrow = left_nibble >= right_nibble + u32::from(borrow_in);
                        assert_eq!((result >> shift) & 15, expected_nibble);
                        assert_eq!(carries[7 - nibble], u8::from(expected_no_borrow));
                    }
                }
            }
        }
    }

    #[test]
    fn full_word_carry_borrow_and_signed_overflow_edges_are_directed() {
        for (left, right, sum, carry, overflow, cb) in [
            (u32::MAX, 1, 0, 1, 0, [1; 8]),
            (0x7fff_ffff, 1, 0x8000_0000, 0, 1, [0, 1, 1, 1, 1, 1, 1, 1]),
            (0x8000_0000, 0x8000_0000, 0, 1, 1, [1, 0, 0, 0, 0, 0, 0, 0]),
        ] {
            let actual = add32(left, right, 0);
            assert_eq!(
                (actual.0, actual.1, actual.2, actual.3),
                (sum, carry, overflow, cb)
            );
        }
        for (left, right, difference, no_borrow, overflow, cb) in [
            (0, 1, u32::MAX, 0, 0, [0; 8]),
            (0x10, 1, 0x0f, 1, 0, [1, 1, 1, 1, 1, 1, 1, 0]),
            (0x8000_0000, 1, 0x7fff_ffff, 1, 1, [1, 0, 0, 0, 0, 0, 0, 0]),
        ] {
            let actual = sub32(left, right);
            assert_eq!(
                (actual.0, actual.1, actual.2, actual.3),
                (difference, no_borrow, overflow, cb)
            );
        }
    }
}
