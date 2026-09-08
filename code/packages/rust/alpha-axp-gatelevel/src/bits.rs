//! LSB-first bit-vector bridges and 64-bit gate networks.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::gates::{not_gate, or_gate, xor_gate};

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

pub fn zero_64(value: u64) -> u8 {
    let any = u64_to_bits(value).into_iter().fold(0, or_gate);
    not_gate(any)
}

pub fn shl_64(value: u64, amount: u32) -> u64 {
    let source = u64_to_bits(value);
    let mut output = [0; 64];
    let amount = (amount & 63) as usize;
    output[amount..].copy_from_slice(&source[..64 - amount]);
    bits_to_u64(&output)
}

pub fn shr_64(value: u64, amount: u32) -> u64 {
    let source = u64_to_bits(value);
    let mut output = [0; 64];
    let amount = (amount & 63) as usize;
    output[..64 - amount].copy_from_slice(&source[amount..]);
    bits_to_u64(&output)
}

pub fn sar_64(value: u64, amount: u32) -> u64 {
    let source = u64_to_bits(value);
    let amount = (amount & 63) as usize;
    let mut output = [source[63]; 64];
    output[..64 - amount].copy_from_slice(&source[amount..]);
    bits_to_u64(&output)
}

pub fn sext_32(value: u64) -> u64 {
    let source = u64_to_bits(value);
    let mut output = [source[31]; 64];
    output[..32].copy_from_slice(&source[..32]);
    bits_to_u64(&output)
}

pub fn sext_16(value: u64) -> u64 {
    let source = u64_to_bits(value);
    let mut output = [source[15]; 64];
    output[..16].copy_from_slice(&source[..16]);
    bits_to_u64(&output)
}

pub fn sext_8(value: u64) -> u64 {
    let source = u64_to_bits(value);
    let mut output = [source[7]; 64];
    output[..8].copy_from_slice(&source[..8]);
    bits_to_u64(&output)
}
