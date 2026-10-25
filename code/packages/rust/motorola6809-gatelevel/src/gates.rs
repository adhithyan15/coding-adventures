//! Fixed-width MC6809 combinational gate networks.

use crate::Motorola6809Error;
use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

#[must_use]
pub fn decode_byte(value: u8) -> [u8; 256] {
    let inputs = u8_to_bits(value);
    std::array::from_fn(|line| {
        (0..8).fold(1, |selected, bit| {
            let signal = if (line >> bit) & 1 == 1 {
                inputs[bit]
            } else {
                not_gate(inputs[bit])
            };
            and_gate(selected, signal)
        })
    })
}

/// Reconstruct one byte solely from the asserted one-hot decoder line.
pub(crate) fn decode_value(value: u8) -> Result<u8, Motorola6809Error> {
    let lines = decode_byte(value);
    let mut selected = None;
    for (index, line) in lines.into_iter().enumerate() {
        if line == 1 {
            if selected.is_some() {
                return Err(Motorola6809Error::InvalidState {
                    reason: "gate byte decoder asserted multiple lines",
                });
            }
            selected = Some(index as u8);
        }
    }
    selected.ok_or(Motorola6809Error::InvalidState {
        reason: "gate byte decoder asserted no line",
    })
}

#[must_use]
pub fn u8_to_bits(value: u8) -> [u8; 8] {
    std::array::from_fn(|bit| (value >> bit) & 1)
}

#[must_use]
pub fn bits_to_u8(bits: &[u8]) -> u8 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (*q << bit))
}

#[must_use]
pub fn u16_to_bits(value: u16) -> [u8; 16] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

#[must_use]
pub fn bits_to_u16(bits: &[u8]) -> u16 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u16::from(*q) << bit))
}

/// Eight-bit ripple addition: result, carry, signed overflow, and half carry.
#[must_use]
pub fn add_8(left: u8, right: u8, carry_in: u8) -> (u8, u8, u8, u8) {
    let left_bits = u8_to_bits(left);
    let right_bits = u8_to_bits(right);
    let full = ripple_carry_adder_with_carry(&left_bits, &right_bits, carry_in);
    let low = ripple_carry_adder_with_carry(&left_bits[..7], &right_bits[..7], carry_in);
    let nibble = ripple_carry_adder_with_carry(&left_bits[..4], &right_bits[..4], carry_in);
    (
        bits_to_u8(&full.sum),
        full.carry_out,
        xor_gate(low.carry_out, full.carry_out),
        nibble.carry_out,
    )
}

/// Eight-bit subtraction: result, borrow, and signed overflow.
#[must_use]
pub fn sub_8(left: u8, right: u8, borrow_in: u8) -> (u8, u8, u8) {
    let right = bit_not_8(right);
    let carry_in = not_gate(borrow_in);
    let (result, carry, overflow, _) = add_8(left, right, carry_in);
    (result, not_gate(carry), overflow)
}

#[must_use]
pub fn add_16(left: u16, right: u16, carry_in: u8) -> (u16, u8, u8) {
    let left_bits = u16_to_bits(left);
    let right_bits = u16_to_bits(right);
    let full = ripple_carry_adder_with_carry(&left_bits, &right_bits, carry_in);
    let low = ripple_carry_adder_with_carry(&left_bits[..15], &right_bits[..15], carry_in);
    (
        bits_to_u16(&full.sum),
        full.carry_out,
        xor_gate(low.carry_out, full.carry_out),
    )
}

#[must_use]
pub fn sub_16(left: u16, right: u16) -> (u16, u8, u8) {
    let (result, carry, overflow) = add_16(left, bit_not_16(right), 1);
    (result, not_gate(carry), overflow)
}

/// Two's-complement address addition through the 16-bit ripple network.
#[must_use]
pub fn add_signed_16(value: u16, offset: i16) -> u16 {
    add_16(value, offset as u16, 0).0
}

/// Eight-by-eight unsigned multiplication as gated partial products.
#[must_use]
pub fn multiply_8(left: u8, right: u8) -> u16 {
    let left_bits = u8_to_bits(left);
    let right_bits = u8_to_bits(right);
    let mut product = 0_u16;
    for (right_bit, select) in right_bits.into_iter().enumerate() {
        let mut partial = [0_u8; 16];
        for (left_bit, input) in left_bits.into_iter().enumerate() {
            partial[left_bit + right_bit] = and_gate(input, select);
        }
        product = add_16(product, bits_to_u16(&partial), 0).0;
    }
    product
}

/// Return one when the selected condition-code flag is asserted.
#[must_use]
pub fn flag(cc: u8, mask: u8) -> u8 {
    let masked = bit_and_8(cc, mask);
    not_gate(is_zero_8(masked))
}

#[must_use]
pub fn bit_not_8(value: u8) -> u8 {
    bits_to_u8(&u8_to_bits(value).map(not_gate))
}

#[must_use]
pub fn bit_not_16(value: u16) -> u16 {
    bits_to_u16(&u16_to_bits(value).map(not_gate))
}

#[must_use]
pub fn bit_and_8(left: u8, right: u8) -> u8 {
    let left = u8_to_bits(left);
    let right = u8_to_bits(right);
    bits_to_u8(&std::array::from_fn::<_, 8, _>(|bit| {
        and_gate(left[bit], right[bit])
    }))
}

#[must_use]
pub fn bit_or_8(left: u8, right: u8) -> u8 {
    let left = u8_to_bits(left);
    let right = u8_to_bits(right);
    bits_to_u8(&std::array::from_fn::<_, 8, _>(|bit| {
        or_gate(left[bit], right[bit])
    }))
}

#[must_use]
pub fn bit_xor_8(left: u8, right: u8) -> u8 {
    let left = u8_to_bits(left);
    let right = u8_to_bits(right);
    bits_to_u8(&std::array::from_fn::<_, 8, _>(|bit| {
        xor_gate(left[bit], right[bit])
    }))
}

#[must_use]
pub fn is_zero_8(value: u8) -> u8 {
    not_gate(u8_to_bits(value).into_iter().fold(0, or_gate))
}

#[must_use]
pub fn is_zero_16(value: u16) -> u8 {
    not_gate(u16_to_bits(value).into_iter().fold(0, or_gate))
}

#[must_use]
pub fn equal_8(left: u8, right: u8) -> u8 {
    is_zero_8(bit_xor_8(left, right))
}

#[must_use]
pub fn negative_8(value: u8) -> u8 {
    u8_to_bits(value)[7]
}

#[must_use]
pub fn negative_16(value: u16) -> u8 {
    u16_to_bits(value)[15]
}

#[must_use]
pub fn select_8(d0: u8, d1: u8, select: u8) -> u8 {
    let d0 = u8_to_bits(d0);
    let d1 = u8_to_bits(d1);
    bits_to_u8(&std::array::from_fn::<_, 8, _>(|bit| {
        mux2(d0[bit], d1[bit], select)
    }))
}

#[must_use]
pub fn select_16(d0: u16, d1: u16, select: u8) -> u16 {
    let d0 = u16_to_bits(d0);
    let d1 = u16_to_bits(d1);
    bits_to_u16(&std::array::from_fn::<_, 16, _>(|bit| {
        mux2(d0[bit], d1[bit], select)
    }))
}

/// Unsigned greater-than from a subtract network.
#[must_use]
pub fn greater_than_8(left: u8, right: u8) -> u8 {
    let (difference, borrow, _) = sub_8(left, right, 0);
    and_gate(not_gate(borrow), not_gate(is_zero_8(difference)))
}

#[must_use]
pub fn shift_right_8(value: u8, fill: u8) -> u8 {
    let source = u8_to_bits(value);
    let mut output = [fill; 8];
    output[..7].copy_from_slice(&source[1..]);
    bits_to_u8(&output)
}

#[must_use]
pub fn shift_left_8(value: u8, fill: u8) -> u8 {
    let source = u8_to_bits(value);
    let mut output = [fill; 8];
    output[1..].copy_from_slice(&source[..7]);
    bits_to_u8(&output)
}

/// MC6809 conditional-branch decision from NZVC condition bits.
#[must_use]
pub fn branch_selected(opcode: u8, cc: u8) -> u8 {
    let c = cc & 1;
    let v = (cc >> 1) & 1;
    let z = (cc >> 2) & 1;
    let n = (cc >> 3) & 1;
    let nxv = xor_gate(n, v);
    match opcode {
        0x20 => 1,
        0x22 => and_gate(not_gate(c), not_gate(z)),
        0x23 => or_gate(c, z),
        0x24 => not_gate(c),
        0x25 => c,
        0x26 => not_gate(z),
        0x27 => z,
        0x28 => not_gate(v),
        0x29 => v,
        0x2a => not_gate(n),
        0x2b => n,
        0x2c => not_gate(nxv),
        0x2d => nxv,
        0x2e => and_gate(not_gate(z), not_gate(nxv)),
        0x2f => or_gate(z, nxv),
        _ => 0,
    }
}
