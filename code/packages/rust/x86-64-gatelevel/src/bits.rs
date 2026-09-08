//! LSB-first bridges and 64-bit combinational gate networks.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

pub fn u64_to_bits(value: u64) -> [u8; 64] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

pub fn bits_to_u64(bits: &[u8]) -> u64 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u64::from(*q) << bit))
}

pub fn add_64(a: u64, b: u64, carry_in: u8) -> (u64, u8, u8) {
    let a_bits = u64_to_bits(a);
    let b_bits = u64_to_bits(b);
    let full = ripple_carry_adder_with_carry(&a_bits, &b_bits, carry_in);
    let low = ripple_carry_adder_with_carry(&a_bits[..63], &b_bits[..63], carry_in);
    (
        bits_to_u64(&full.sum),
        full.carry_out,
        xor_gate(low.carry_out, full.carry_out),
    )
}

pub fn invert_64(value: u64) -> u64 {
    bits_to_u64(&u64_to_bits(value).map(not_gate))
}

pub fn and_64(a: u64, b: u64) -> u64 {
    let a = u64_to_bits(a);
    let b = u64_to_bits(b);
    bits_to_u64(&std::array::from_fn::<_, 64, _>(|bit| {
        and_gate(a[bit], b[bit])
    }))
}

pub fn or_64(a: u64, b: u64) -> u64 {
    let a = u64_to_bits(a);
    let b = u64_to_bits(b);
    bits_to_u64(&std::array::from_fn::<_, 64, _>(|bit| {
        or_gate(a[bit], b[bit])
    }))
}

pub fn xor_64(a: u64, b: u64) -> u64 {
    let a = u64_to_bits(a);
    let b = u64_to_bits(b);
    bits_to_u64(&std::array::from_fn::<_, 64, _>(|bit| {
        xor_gate(a[bit], b[bit])
    }))
}

pub fn zero_64(value: u64) -> u8 {
    not_gate(u64_to_bits(value).into_iter().fold(0, or_gate))
}
