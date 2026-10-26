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

/// Split an instruction byte through fixed wires and settle both one-hot
/// decoders.  Callers route on these wires; they never decode the nibbles a
/// second time with host shifts or matches.
pub fn opcode_lines(opcode: u8) -> ([u8; 16], [u8; 16]) {
    let bits = bits8(opcode);
    let low = from_bits8(&bits[..4]);
    let high = from_bits8(&bits[4..]);
    (decode4(high), decode4(low))
}

/// Recover the index of an already-settled one-hot decoder for bounded array
/// addressing and trace text.  This does not decide legality or family
/// routing; those decisions consume the lines themselves.
pub fn one_hot_index(lines: &[u8; 16]) -> u8 {
    lines
        .iter()
        .position(|wire| *wire == 1)
        .expect("four-bit decoder is one-hot") as u8
}

pub fn any_line(lines: &[u8; 16], selected: &[usize]) -> u8 {
    selected
        .iter()
        .fold(0, |wire, index| or_gate(wire, lines[*index]))
}

fn select_predicate(lines: &[u8; 16], predicates: &[u8; 16]) -> u8 {
    lines
        .iter()
        .zip(predicates)
        .fold(0, |result, (selected, predicate)| {
            or_gate(result, and_gate(*selected, *predicate))
        })
}

/// Gate-settled condition for all sixteen short branch/skip encodings.
pub fn short_control_condition(low: &[u8; 16], q: bool, d: u8, df: bool, ef: [bool; 4]) -> u8 {
    let q = u8::from(q);
    let zero = is_zero(d);
    let df = u8::from(df);
    let ef = ef.map(u8::from);
    select_predicate(
        low,
        &[
            1,
            q,
            zero,
            df,
            ef[0],
            ef[1],
            ef[2],
            ef[3],
            0,
            not_gate(q),
            not_gate(zero),
            not_gate(df),
            not_gate(ef[0]),
            not_gate(ef[1]),
            not_gate(ef[2]),
            not_gate(ef[3]),
        ],
    )
}

/// Gate-settled condition for the seven three-byte long branches.
pub fn long_branch_condition(low: &[u8; 16], q: bool, d: u8, df: bool) -> u8 {
    let q = u8::from(q);
    let zero = is_zero(d);
    let df = u8::from(df);
    select_predicate(
        low,
        &[
            1,
            q,
            zero,
            df,
            0,
            0,
            0,
            0,
            0,
            not_gate(q),
            not_gate(zero),
            not_gate(df),
            0,
            0,
            0,
            0,
        ],
    )
}

/// Gate-settled condition for long skip encodings (NOP remains disabled).
pub fn long_skip_condition(low: &[u8; 16], q: bool, d: u8, df: bool, ie: bool) -> u8 {
    let q = u8::from(q);
    let zero = is_zero(d);
    let df = u8::from(df);
    select_predicate(
        low,
        &[
            0,
            0,
            0,
            0,
            0,
            not_gate(q),
            not_gate(zero),
            not_gate(df),
            1,
            0,
            0,
            0,
            u8::from(ie),
            q,
            zero,
            df,
        ],
    )
}

/// Settled interrupt control wires. Inhibition has explicit priority; the
/// eligibility line can rise only when inhibition is clear and IE is set.
pub fn interrupt_control(first_instruction_pending: u8, ie: u8) -> (u8, u8) {
    let inhibited = and_gate(first_instruction_pending, 1);
    let eligible = and_gate(not_gate(inhibited), ie);
    (inhibited, eligible)
}

/// Fixed wiring for T=(X<<4)|P without host arithmetic authority.
pub fn pack_selectors(x: u8, p: u8) -> u8 {
    let x = bits8(x);
    let p = bits8(p);
    from_bits8(&std::array::from_fn::<_, 8, _>(|bit| {
        if bit < 4 {
            p[bit]
        } else {
            x[bit - 4]
        }
    }))
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
