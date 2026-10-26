//! Fixed-width CDP1802 combinational networks.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

pub fn bits8(value: u8) -> [u8; 8] {
    std::array::from_fn(|bit| (value >> bit) & 1)
}

pub fn bits16(value: u16) -> [u8; 16] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

pub fn from_bits8(bits: &[u8]) -> u8 {
    bits.iter()
        .take(8)
        .enumerate()
        .fold(0, |value, (bit, wire)| value | (wire << bit))
}

pub fn from_bits16(bits: &[u8]) -> u16 {
    bits.iter()
        .take(16)
        .enumerate()
        .fold(0, |value, (bit, wire)| value | (u16::from(*wire) << bit))
}

pub fn add8(left: u8, right: u8, carry: bool) -> (u8, bool) {
    let result = ripple_carry_adder_with_carry(&bits8(left), &bits8(right), u8::from(carry));
    (from_bits8(&result.sum), result.carry_out == 1)
}

pub fn subtract8(minuend: u8, subtrahend: u8, borrow: bool) -> (u8, bool) {
    let inverted: [u8; 8] = bits8(subtrahend).map(not_gate);
    let carry_in = not_gate(u8::from(borrow));
    let result = ripple_carry_adder_with_carry(&bits8(minuend), &inverted, carry_in);
    (from_bits8(&result.sum), result.carry_out == 1)
}

pub fn increment16(value: u16) -> u16 {
    let one = bits16(1);
    from_bits16(&ripple_carry_adder_with_carry(&bits16(value), &one, 0).sum)
}

pub fn decrement16(value: u16) -> u16 {
    let minus_one = [1; 16];
    from_bits16(&ripple_carry_adder_with_carry(&bits16(value), &minus_one, 0).sum)
}

pub fn bit_or(left: u8, right: u8) -> u8 {
    from_bits8(&std::array::from_fn::<_, 8, _>(|bit| {
        or_gate((left >> bit) & 1, (right >> bit) & 1)
    }))
}

pub fn bit_and(left: u8, right: u8) -> u8 {
    from_bits8(&std::array::from_fn::<_, 8, _>(|bit| {
        and_gate((left >> bit) & 1, (right >> bit) & 1)
    }))
}

pub fn bit_xor(left: u8, right: u8) -> u8 {
    from_bits8(&std::array::from_fn::<_, 8, _>(|bit| {
        xor_gate((left >> bit) & 1, (right >> bit) & 1)
    }))
}

pub fn is_zero(value: u8) -> u8 {
    bits8(value)
        .into_iter()
        .fold(1, |zero, bit| and_gate(zero, not_gate(bit)))
}

pub fn select8(false_value: u8, true_value: u8, select: u8) -> u8 {
    let not_select = not_gate(select);
    from_bits8(&std::array::from_fn::<_, 8, _>(|bit| {
        or_gate(
            and_gate((false_value >> bit) & 1, not_select),
            and_gate((true_value >> bit) & 1, select),
        )
    }))
}

pub fn select16(false_value: u16, true_value: u16, select: u8) -> u16 {
    let not_select = not_gate(select);
    from_bits16(&std::array::from_fn::<_, 16, _>(|bit| {
        or_gate(
            and_gate(((false_value >> bit) & 1) as u8, not_select),
            and_gate(((true_value >> bit) & 1) as u8, select),
        )
    }))
}

pub fn decode4(value: u8) -> [u8; 16] {
    let input: [u8; 4] = std::array::from_fn(|bit| (value >> bit) & 1);
    std::array::from_fn(|line| {
        (0..4).fold(1, |selected, bit| {
            let expected = ((line >> bit) & 1) as u8;
            let wire = select8(not_gate(input[bit]), input[bit], expected) & 1;
            and_gate(selected, wire)
        })
    })
}

pub fn shift_right(value: u8, carry: bool) -> (u8, bool) {
    let bits = bits8(value);
    let output: [u8; 8] = std::array::from_fn(|bit| {
        if bit == 7 {
            u8::from(carry)
        } else {
            bits[bit + 1]
        }
    });
    (from_bits8(&output), bits[0] == 1)
}

pub fn shift_left(value: u8, carry: bool) -> (u8, bool) {
    let bits = bits8(value);
    let output: [u8; 8] = std::array::from_fn(|bit| {
        if bit == 0 {
            u8::from(carry)
        } else {
            bits[bit - 1]
        }
    });
    (from_bits8(&output), bits[7] == 1)
}

pub fn condition(wire: bool) -> u8 {
    u8::from(wire)
}
