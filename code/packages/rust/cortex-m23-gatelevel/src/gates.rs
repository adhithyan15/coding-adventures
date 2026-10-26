//! Fixed-width combinational networks used by the Cortex-M23 transition engine.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

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

/// Unsigned less-than line from the borrow output of the subtract network.
#[must_use]
pub fn less_than_unsigned_32(left: u32, right: u32) -> u8 {
    sub_32(left, right, 0).1
}

/// Unsigned less-than-or-equal line.
#[must_use]
pub fn less_or_equal_unsigned_32(left: u32, right: u32) -> u8 {
    or_gate(less_than_unsigned_32(left, right), equal_32(left, right))
}

/// Alignment line for the supported power-of-two byte widths.
#[must_use]
pub fn aligned_32(address: u32, width: usize) -> u8 {
    let bits = u32_to_bits(address);
    match width {
        1 => 1,
        2 => not_gate(bits[0]),
        4 => and_gate(not_gate(bits[0]), not_gate(bits[1])),
        _ => 0,
    }
}

#[must_use]
pub fn is_zero_32(value: u32) -> u8 {
    not_gate(u32_to_bits(value).into_iter().fold(0, or_gate))
}

#[must_use]
pub fn select_32(d0: u32, d1: u32, select: u8) -> u32 {
    let d0 = u32_to_bits(d0);
    let d1 = u32_to_bits(d1);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        mux2(d0[bit], d1[bit], select)
    }))
}

/// Select one bit from a word through a 32-line one-hot decoder.
///
/// An out-of-range index asserts no line and therefore returns zero.  This is
/// useful for shift networks, where the architectural boundary logic chooses
/// separately between the decoded bit and the zero/sign-fill cases.
#[must_use]
pub fn select_bit_32(value: u32, index: u32) -> u8 {
    let bits = u32_to_bits(value);
    (0..32).fold(0, |output, bit| {
        or_gate(output, and_gate(bits[bit], selected(index, bit as u32, 5)))
    })
}

/// OR together independently decoded one-bit control lines.
#[must_use]
pub fn any_line<const N: usize>(lines: [u8; N]) -> u8 {
    lines.into_iter().fold(0, or_gate)
}

/// Select one 32-bit candidate from settled one-hot control lines.
#[must_use]
pub fn select_one_hot_32<const N: usize>(values: [u32; N], lines: [u8; N]) -> u32 {
    values
        .into_iter()
        .zip(lines)
        .fold(0, |output, (value, line)| {
            bit_or_32(output, select_32(0, value, line))
        })
}

/// Select one one-bit candidate from settled one-hot control lines.
#[must_use]
pub fn select_one_hot_bit<const N: usize>(values: [u8; N], lines: [u8; N]) -> u8 {
    values
        .into_iter()
        .zip(lines)
        .fold(0, |output, (value, line)| {
            or_gate(output, and_gate(value, line))
        })
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

/// Thumb immediate-shift result and carry from one-hot kind/boundary networks.
///
/// `kind` is 0=LSL, 1=LSR, and 2=ASR.  An encoded zero preserves carry for
/// LSL but denotes an architectural shift by 32 for LSR/ASR.
#[must_use]
pub fn shift_immediate_network(value: u32, kind: u8, encoded: u32, carry: u8) -> (u32, u8) {
    let kind_lines = [
        selected(u32::from(kind), 0, 2),
        selected(u32::from(kind), 1, 2),
        selected(u32::from(kind), 2, 2),
    ];
    let encoded_zero = selected(encoded, 0, 5);
    let encoded_nonzero = not_gate(encoded_zero);
    let amount = select_32(encoded, 32, encoded_zero);
    let amount_is_32 = selected(amount, 32, 6);
    let low_amount = field(amount, 0, 5);
    let sign = u32_to_bits(value)[31];
    let sign_fill = select_32(0, u32::MAX, sign);

    let lsl_result = select_32(value, shift_left_32(value, encoded), encoded_nonzero);
    let lsl_bit = select_bit_32(value, sub_32(32, encoded, 0).0);
    let lsl_carry = mux2(carry, lsl_bit, encoded_nonzero);

    let lsr_result = select_32(shift_right_32(value, low_amount), 0, amount_is_32);
    let right_bit = select_bit_32(value, sub_32(amount, 1, 0).0);

    let asr_result = select_32(
        shift_arithmetic_right_32(value, low_amount),
        sign_fill,
        amount_is_32,
    );

    (
        select_one_hot_32([lsl_result, lsr_result, asr_result], kind_lines),
        select_one_hot_bit([lsl_carry, right_bit, right_bit], kind_lines),
    )
}

/// Thumb register-shift result and carry from one-hot kind/boundary networks.
///
/// `kind` is 0=LSL, 1=LSR, 2=ASR, and 3=ROR.  The full low byte of the
/// register supplies the architectural amount; the barrel network itself uses
/// its low five bits only after zero/32/greater-than-32 lines have settled.
#[must_use]
pub fn shift_register_network(value: u32, kind: u8, amount: u32, carry: u8) -> (u32, u8) {
    let kind_lines = [
        selected(u32::from(kind), 0, 2),
        selected(u32::from(kind), 1, 2),
        selected(u32::from(kind), 2, 2),
        selected(u32::from(kind), 3, 2),
    ];
    let amount = field(amount, 0, 8);
    let amount_zero = selected(amount, 0, 8);
    let amount_nonzero = not_gate(amount_zero);
    let amount_lt_32 = less_than_unsigned_32(amount, 32);
    let amount_eq_32 = equal_32(amount, 32);
    let within_word = and_gate(amount_nonzero, amount_lt_32);
    let at_least_32 = not_gate(amount_lt_32);
    let low_amount = field(amount, 0, 5);
    let sign = u32_to_bits(value)[31];
    let sign_fill = select_32(0, u32::MAX, sign);
    let right_index = sub_32(amount, 1, 0).0;

    let lsl_result = select_32(0, shift_left_32(value, low_amount), within_word);
    let lsl_within_carry = select_bit_32(value, sub_32(32, amount, 0).0);
    let lsl_carry = select_one_hot_bit(
        [lsl_within_carry, u32_to_bits(value)[0]],
        [within_word, amount_eq_32],
    );

    let lsr_result = select_32(0, shift_right_32(value, low_amount), within_word);
    let lsr_within_carry = select_bit_32(value, right_index);
    let lsr_carry = select_one_hot_bit([lsr_within_carry, sign], [within_word, amount_eq_32]);

    let asr_result = select_32(
        shift_arithmetic_right_32(value, low_amount),
        sign_fill,
        at_least_32,
    );
    let asr_carry = mux2(select_bit_32(value, right_index), sign, at_least_32);

    let ror_result = rotate_right_32(value, low_amount);
    let ror_carry = u32_to_bits(ror_result)[31];
    let selected_result =
        select_one_hot_32([lsl_result, lsr_result, asr_result, ror_result], kind_lines);
    let selected_carry =
        select_one_hot_bit([lsl_carry, lsr_carry, asr_carry, ror_carry], kind_lines);

    (
        select_32(selected_result, value, amount_zero),
        mux2(selected_carry, carry, amount_zero),
    )
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
