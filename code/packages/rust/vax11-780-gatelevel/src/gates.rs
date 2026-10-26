//! Fixed-width VAX combinational gate networks.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ArithmeticResult {
    pub(crate) value: u32,
    pub(crate) carry_or_borrow: bool,
    pub(crate) overflow: bool,
}

pub(crate) const fn width_mask(width: usize) -> u32 {
    match width {
        1 => 0xff,
        2 => 0xffff,
        4 => u32::MAX,
        _ => panic!("VAX integer width must be one, two, or four bytes"),
    }
}

pub(crate) const fn sign_bit(width: usize) -> u32 {
    1_u32 << (width * 8 - 1)
}

fn bits_u32(value: u32, bit_count: usize) -> Vec<u8> {
    (0..bit_count)
        .map(|bit| ((value >> bit) & 1) as u8)
        .collect()
}

fn bits_u64(value: u64) -> [u8; 64] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn from_bits_u32(bits: &[u8]) -> u32 {
    bits.iter()
        .enumerate()
        .fold(0, |word, (bit, q)| word | (u32::from(*q) << bit))
}

fn from_bits_u64(bits: &[u8]) -> u64 {
    bits.iter()
        .enumerate()
        .fold(0, |word, (bit, q)| word | (u64::from(*q) << bit))
}

fn add_u64(a: u64, b: u64, carry: u8) -> (u64, u8) {
    let sum = ripple_carry_adder_with_carry(&bits_u64(a), &bits_u64(b), carry);
    (from_bits_u64(&sum.sum), sum.carry_out)
}

fn not_u64(value: u64) -> u64 {
    from_bits_u64(&bits_u64(value).map(not_gate))
}

fn negate_u64(value: u64) -> u64 {
    add_u64(not_u64(value), 0, 1).0
}

pub(crate) fn decode_byte(value: u8) -> [u8; 256] {
    let inputs: [u8; 8] = std::array::from_fn(|bit| (value >> bit) & 1);
    std::array::from_fn(|line| {
        (0..8).fold(1, |selected, bit| {
            let expected = ((line >> bit) & 1) as u8;
            let signal = mux2(not_gate(inputs[bit]), inputs[bit], expected);
            and_gate(selected, signal)
        })
    })
}

/// Reconstruct a byte exclusively from the asserted one-hot decoder output.
///
/// Returning `None` makes a malformed control network fail closed instead of
/// allowing the fetched host integer to bypass the decoder.
pub(crate) fn decode_byte_value(value: u8) -> Option<u8> {
    let decoded = decode_byte(value);
    if decoded.iter().copied().map(u16::from).sum::<u16>() != 1 {
        return None;
    }
    let mut bits = [0_u8; 8];
    for (line, selected) in decoded.into_iter().enumerate() {
        for (bit, output) in bits.iter_mut().enumerate() {
            *output = or_gate(
                *output,
                and_gate(selected, u8::from((line >> bit) & 1 != 0)),
            );
        }
    }
    Some(
        bits.into_iter()
            .enumerate()
            .fold(0, |byte, (bit, q)| byte | (q << bit)),
    )
}

/// Gate-derived write enable for one packed byte of DFF state.
pub(crate) fn byte_changed(before: u8, after: u8) -> bool {
    (0..8)
        .map(|bit| xor_gate((before >> bit) & 1, (after >> bit) & 1))
        .fold(0, or_gate)
        != 0
}

pub(crate) fn bit_not(value: u32, width: usize) -> u32 {
    from_bits_u32(
        &bits_u32(value, width * 8)
            .into_iter()
            .map(not_gate)
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn bit_and(a: u32, b: u32, width: usize) -> u32 {
    let a = bits_u32(a, width * 8);
    let b = bits_u32(b, width * 8);
    from_bits_u32(
        &(0..width * 8)
            .map(|bit| and_gate(a[bit], b[bit]))
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn bit_or(a: u32, b: u32, width: usize) -> u32 {
    let a = bits_u32(a, width * 8);
    let b = bits_u32(b, width * 8);
    from_bits_u32(
        &(0..width * 8)
            .map(|bit| or_gate(a[bit], b[bit]))
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn bit_xor(a: u32, b: u32, width: usize) -> u32 {
    let a = bits_u32(a, width * 8);
    let b = bits_u32(b, width * 8);
    from_bits_u32(
        &(0..width * 8)
            .map(|bit| xor_gate(a[bit], b[bit]))
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn add(a: u32, b: u32, width: usize) -> ArithmeticResult {
    let bit_count = width * 8;
    let a = bits_u32(a, bit_count);
    let b = bits_u32(b, bit_count);
    let full = ripple_carry_adder_with_carry(&a, &b, 0);
    let low = ripple_carry_adder_with_carry(&a[..bit_count - 1], &b[..bit_count - 1], 0);
    ArithmeticResult {
        value: from_bits_u32(&full.sum),
        carry_or_borrow: full.carry_out != 0,
        overflow: xor_gate(low.carry_out, full.carry_out) != 0,
    }
}

pub(crate) fn add_with_carry(a: u32, b: u32, width: usize, carry: u8) -> ArithmeticResult {
    let bit_count = width * 8;
    let a = bits_u32(a, bit_count);
    let b = bits_u32(b, bit_count);
    let full = ripple_carry_adder_with_carry(&a, &b, carry);
    let low = ripple_carry_adder_with_carry(&a[..bit_count - 1], &b[..bit_count - 1], carry);
    ArithmeticResult {
        value: from_bits_u32(&full.sum),
        carry_or_borrow: full.carry_out != 0,
        overflow: xor_gate(low.carry_out, full.carry_out) != 0,
    }
}

pub(crate) fn subtract(a: u32, b: u32, width: usize) -> ArithmeticResult {
    let sum = add_with_carry(a, bit_not(b, width), width, 1);
    ArithmeticResult {
        value: sum.value,
        carry_or_borrow: !sum.carry_or_borrow,
        overflow: sum.overflow,
    }
}

pub(crate) fn negate(value: u32, width: usize) -> ArithmeticResult {
    subtract(0, value, width)
}

pub(crate) fn is_zero(value: u32, width: usize) -> bool {
    let any = bits_u32(value, width * 8).into_iter().fold(0, or_gate);
    not_gate(any) != 0
}

pub(crate) fn is_negative(value: u32, width: usize) -> bool {
    bits_u32(value, width * 8)[width * 8 - 1] != 0
}

pub(crate) fn unsigned_less(a: u32, b: u32, width: usize) -> bool {
    subtract(a, b, width).carry_or_borrow
}

pub(crate) fn select_u32(d0: u32, d1: u32, select: bool) -> u32 {
    let d0 = bits_u32(d0, 32);
    let d1 = bits_u32(d1, 32);
    from_bits_u32(
        &(0..32)
            .map(|bit| mux2(d0[bit], d1[bit], u8::from(select)))
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn multiply(a: u32, b: u32, width: usize) -> ArithmeticResult {
    let mask = u64::from(width_mask(width));
    let a_negative = is_negative(a, width);
    let b_negative = is_negative(b, width);
    let a_magnitude = if a_negative {
        negate(a, width).value
    } else {
        a & width_mask(width)
    };
    let b_magnitude = if b_negative {
        negate(b, width).value
    } else {
        b & width_mask(width)
    };
    let multiplier = bits_u32(b_magnitude, width * 8);
    let mut product = 0_u64;
    for (bit, selected) in multiplier.into_iter().enumerate() {
        let partial = u64::from(a_magnitude) << bit;
        product = add_u64(product, if selected == 1 { partial } else { 0 }, 0).0;
    }
    let negative = xor_gate(u8::from(a_negative), u8::from(b_negative)) != 0;
    if negative {
        product = negate_u64(product);
    }
    let value = (product & mask) as u32;
    let sign_extended = if is_negative(value, width) {
        u64::from(value) | !mask
    } else {
        u64::from(value)
    };
    ArithmeticResult {
        value,
        carry_or_borrow: false,
        overflow: bit_xor_u64(product, sign_extended) != 0,
    }
}

fn bit_xor_u64(a: u64, b: u64) -> u64 {
    let a = bits_u64(a);
    let b = bits_u64(b);
    from_bits_u64(&std::array::from_fn::<_, 64, _>(|bit| {
        xor_gate(a[bit], b[bit])
    }))
}

pub(crate) fn divide(dividend: u32, divisor: u32, width: usize) -> Option<ArithmeticResult> {
    if is_zero(divisor, width) {
        return None;
    }
    let dividend_negative = is_negative(dividend, width);
    let divisor_negative = is_negative(divisor, width);
    let dividend_magnitude = if dividend_negative {
        negate(dividend, width).value
    } else {
        dividend & width_mask(width)
    };
    let divisor_magnitude = if divisor_negative {
        negate(divisor, width).value
    } else {
        divisor & width_mask(width)
    };
    let dividend_bits = bits_u32(dividend_magnitude, width * 8);
    let mut remainder = 0_u32;
    let mut quotient = 0_u32;
    for bit in (0..width * 8).rev() {
        remainder = (remainder << 1) | u32::from(dividend_bits[bit]);
        let difference = subtract(remainder, divisor_magnitude, 4);
        if !difference.carry_or_borrow {
            remainder = difference.value;
            quotient |= 1_u32 << bit;
        }
    }
    let negative = xor_gate(u8::from(dividend_negative), u8::from(divisor_negative)) != 0;
    let signed_limit = if negative {
        sign_bit(width)
    } else {
        sign_bit(width) - 1
    };
    let overflow = unsigned_less(signed_limit, quotient, width);
    if negative {
        quotient = negate(quotient, width).value;
    }
    Some(ArithmeticResult {
        value: quotient & width_mask(width),
        carry_or_borrow: false,
        overflow,
    })
}

pub(crate) fn branch_condition(opcode: u8, n: bool, z: bool, v: bool, c: bool) -> bool {
    let n = u8::from(n);
    let z = u8::from(z);
    let v = u8::from(v);
    let c = u8::from(c);
    let selected = match opcode {
        0x12 => not_gate(z),
        0x13 => z,
        0x14 => and_gate(not_gate(n), not_gate(z)),
        0x15 => or_gate(n, z),
        0x18 => not_gate(n),
        0x19 => n,
        0x1a => and_gate(not_gate(c), not_gate(z)),
        0x1b => or_gate(c, z),
        0x1c => not_gate(v),
        0x1d => v,
        0x1e => not_gate(c),
        0x1f => c,
        _ => 0,
    };
    selected != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_width_arithmetic_and_decode_are_gate_derived() {
        assert_eq!(add(0xff, 1, 1).value, 0);
        assert!(add(0xff, 1, 1).carry_or_borrow);
        assert_eq!(subtract(0, 1, 2).value, 0xffff);
        assert!(subtract(0, 1, 2).carry_or_borrow);
        assert_eq!(multiply(0xffff_fffe, 3, 4).value, 0xffff_fffa);
        assert_eq!(multiply(3, 0xffff_fffe, 4).value, 0xffff_fffa);
        assert_eq!(divide(0xffff_fff7, 3, 4).unwrap().value, 0xffff_fffd);
        assert_eq!(divide(9, 0xffff_fffd, 4).unwrap().value, 0xffff_fffd);
        assert!(divide(9, 0, 4).is_none());
        assert!(!branch_condition(0, false, false, false, false));
        assert!(std::panic::catch_unwind(|| width_mask(3)).is_err());
        let decoded = decode_byte(0xd0);
        assert_eq!(decoded.into_iter().map(u16::from).sum::<u16>(), 1);
        assert_eq!(decoded[0xd0], 1);
        assert_eq!(decode_byte_value(0xd0), Some(0xd0));
        assert!(!byte_changed(0x55, 0x55));
        assert!(byte_changed(0x55, 0x54));
    }
}
