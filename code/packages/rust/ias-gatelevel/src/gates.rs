//! Fixed-width IAS combinational gate networks.

use arithmetic::adders::ripple_carry_adder_with_carry;
use ias_simulator::{IasError, WORD_MASK};
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

const SIGN_BIT: u64 = 1_u64 << 39;

fn bits<const WIDTH: usize>(value: u128) -> [u8; WIDTH] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn from_bits(values: &[u8]) -> u128 {
    values
        .iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u128::from(*q) << bit))
}

fn add_width<const WIDTH: usize>(left: u128, right: u128, carry: u8) -> (u128, u8) {
    let sum = ripple_carry_adder_with_carry(&bits::<WIDTH>(left), &bits::<WIDTH>(right), carry);
    (from_bits(&sum.sum), sum.carry_out)
}

pub(crate) fn add40(left: u64, right: u64) -> u64 {
    add_width::<40>(u128::from(left), u128::from(right), 0).0 as u64 & WORD_MASK
}

pub(crate) fn negate40(value: u64) -> u64 {
    let inverted = from_bits(&bits::<40>(u128::from(value)).map(not_gate));
    add_width::<40>(inverted, 0, 1).0 as u64 & WORD_MASK
}

pub(crate) fn absolute40(value: u64) -> u64 {
    let negative = ((value >> 39) & 1) as u8;
    let negated = negate40(value);
    let selected: [u8; 40] = std::array::from_fn(|bit| {
        mux2(
            ((value >> bit) & 1) as u8,
            ((negated >> bit) & 1) as u8,
            negative,
        )
    });
    from_bits(&selected) as u64
}

pub(crate) fn is_negative40(value: u64) -> bool {
    and_gate(((value & SIGN_BIT) != 0).into(), 1) != 0
}

pub(crate) fn shift_left40(value: u64) -> u64 {
    let source = bits::<40>(u128::from(value));
    from_bits(&std::array::from_fn::<_, 40, _>(|bit| {
        if bit == 0 {
            0
        } else {
            source[bit - 1]
        }
    })) as u64
}

pub(crate) fn shift_right40(value: u64) -> u64 {
    let source = bits::<40>(u128::from(value));
    from_bits(&std::array::from_fn::<_, 40, _>(|bit| {
        if bit == 39 {
            source[39]
        } else {
            source[bit + 1]
        }
    })) as u64
}

fn decode8(value: u8) -> [u8; 256] {
    let input: [u8; 8] = std::array::from_fn(|bit| (value >> bit) & 1);
    std::array::from_fn(|line| {
        (0..8).fold(1, |selected, bit| {
            let expected = ((line >> bit) & 1) as u8;
            and_gate(selected, mux2(not_gate(input[bit]), input[bit], expected))
        })
    })
}

pub(crate) fn decode_opcode(value: u8) -> Option<u8> {
    let decoded = decode8(value);
    if decoded.iter().copied().map(u16::from).sum::<u16>() != 1 {
        return None;
    }
    let output: [u8; 8] = std::array::from_fn(|bit| {
        decoded
            .iter()
            .copied()
            .enumerate()
            .fold(0, |value, (line, selected)| {
                or_gate(value, and_gate(selected, ((line >> bit) & 1) as u8))
            })
    });
    Some(from_bits(&output) as u8)
}

pub(crate) fn decode_address(value: u16) -> u16 {
    let input = bits::<12>(u128::from(value));
    from_bits(&input.map(|signal| and_gate(signal, 1))) as u16
}

pub(crate) fn word_changed(before: u64, after: u64) -> bool {
    (0..40)
        .map(|bit| xor_gate(((before >> bit) & 1) as u8, ((after >> bit) & 1) as u8))
        .fold(0, or_gate)
        != 0
}

pub(crate) fn replace_address(word: u64, address: u64, left: bool) -> u64 {
    let old = bits::<40>(u128::from(word));
    let replacement = bits::<12>(u128::from(address));
    let start = if left { 20 } else { 0 };
    let selected: [u8; 40] = std::array::from_fn(|bit| {
        if (start..start + 12).contains(&bit) {
            replacement[bit - start]
        } else {
            old[bit]
        }
    });
    from_bits(&selected) as u64
}

pub(crate) fn multiply40(left: u64, right: u64) -> (u64, u64) {
    let left_negative = is_negative40(left);
    let right_negative = is_negative40(right);
    let left_magnitude = absolute40(left);
    let right_magnitude = absolute40(right);
    let multiplier = bits::<40>(u128::from(right_magnitude));
    let mut product = 0_u128;
    for (bit, selected) in multiplier.into_iter().enumerate() {
        let partial = (u128::from(left_magnitude) << bit) * u128::from(selected);
        product = add_width::<80>(product, partial, 0).0;
    }
    if xor_gate(left_negative.into(), right_negative.into()) != 0 {
        let inverted = from_bits(&bits::<80>(product).map(not_gate));
        product = add_width::<80>(inverted, 0, 1).0;
    }
    (
        ((product >> 40) as u64) & WORD_MASK,
        product as u64 & WORD_MASK,
    )
}

fn unsigned_less40(left: u64, right: u64) -> bool {
    let inverted = from_bits(&bits::<40>(u128::from(right)).map(not_gate));
    add_width::<40>(u128::from(left), inverted, 1).1 == 0
}

pub(crate) fn divide40(dividend: u64, divisor: u64) -> Result<(u64, u64), IasError> {
    if divisor & WORD_MASK == 0 {
        return Err(IasError::DivideByZero);
    }
    let dividend_negative = is_negative40(dividend);
    let divisor_negative = is_negative40(divisor);
    let numerator = absolute40(dividend);
    let denominator = absolute40(divisor);
    let mut quotient = 0_u64;
    let mut remainder = 0_u64;
    for bit in (0..40).rev() {
        remainder = shift_left40(remainder) | ((numerator >> bit) & 1);
        if !unsigned_less40(remainder, denominator) {
            remainder = add40(remainder, negate40(denominator));
            quotient |= 1_u64 << bit;
        }
    }
    let result_negative = xor_gate(dividend_negative.into(), divisor_negative.into()) != 0;
    let positive_limit = (1_u64 << 39) - 1;
    let negative_limit = 1_u64 << 39;
    if (result_negative && quotient > negative_limit)
        || (!result_negative && quotient > positive_limit)
    {
        return Err(IasError::QuotientOverflow);
    }
    if result_negative {
        quotient = negate40(quotient);
    }
    if dividend_negative {
        remainder = negate40(remainder);
    }
    Ok((remainder, quotient))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_arithmetic_and_decode_cover_edges() {
        assert_eq!(decode_opcode(21), Some(21));
        assert_eq!(decode_address(0x0abc), 0x0abc);
        assert_eq!(add40(WORD_MASK, 1), 0);
        assert_eq!(negate40(1), WORD_MASK);
        assert_eq!(absolute40(WORD_MASK), 1);
        assert!(is_negative40(WORD_MASK));
        assert_eq!(shift_left40(3), 6);
        assert_eq!(shift_right40(WORD_MASK - 1), WORD_MASK);
        assert!(word_changed(0, 1));
        assert!(!word_changed(7, 7));
        assert_eq!(replace_address(WORD_MASK, 0x123, true) >> 20 & 0xfff, 0x123);
        assert_eq!(replace_address(0, 0x456, false) & 0xfff, 0x456);
        assert_eq!(multiply40(3, negate40(2)), (WORD_MASK, negate40(6)));
        assert_eq!(divide40(17, 5), Ok((2, 3)));
        assert_eq!(divide40(1, 0), Err(IasError::DivideByZero));
        assert_eq!(
            divide40(1 << 39, WORD_MASK),
            Err(IasError::QuotientOverflow)
        );
    }
}
