//! LSB-first bridges and 32-bit combinational gate networks.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::gates::{not_gate, or_gate, xor_gate};

pub fn u32_to_bits(value: u32) -> [u8; 32] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

pub fn bits_to_u32(bits: &[u8]) -> u32 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u32::from(*q) << bit))
}

pub fn add_32(a: u32, b: u32, carry_in: u8) -> (u32, u8, u8) {
    let a_bits = u32_to_bits(a);
    let b_bits = u32_to_bits(b);
    let full = ripple_carry_adder_with_carry(&a_bits, &b_bits, carry_in);
    let low = ripple_carry_adder_with_carry(&a_bits[..31], &b_bits[..31], carry_in);
    (
        bits_to_u32(&full.sum),
        full.carry_out,
        xor_gate(low.carry_out, full.carry_out),
    )
}

pub fn invert_32(value: u32) -> u32 {
    bits_to_u32(&u32_to_bits(value).map(not_gate))
}

pub fn zero_32(value: u32) -> u8 {
    not_gate(u32_to_bits(value).into_iter().fold(0, or_gate))
}

pub fn shl_32(value: u32, amount: u32) -> u32 {
    let amount = (amount & 63) as usize;
    if amount >= 32 {
        return 0;
    }
    let source = u32_to_bits(value);
    let mut output = [0; 32];
    output[amount..].copy_from_slice(&source[..32 - amount]);
    bits_to_u32(&output)
}

pub fn shr_32(value: u32, amount: u32) -> u32 {
    let amount = (amount & 63) as usize;
    if amount >= 32 {
        return 0;
    }
    let source = u32_to_bits(value);
    let mut output = [0; 32];
    output[..32 - amount].copy_from_slice(&source[amount..]);
    bits_to_u32(&output)
}

pub fn sar_32(value: u32, amount: u32) -> u32 {
    let amount = (amount & 63).min(31) as usize;
    let source = u32_to_bits(value);
    let mut output = [source[31]; 32];
    output[..32 - amount].copy_from_slice(&source[amount..]);
    bits_to_u32(&output)
}
