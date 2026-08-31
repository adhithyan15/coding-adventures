//! Alpha's 64-bit integer datapaths built from repository gate primitives.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

use crate::bits::{
    add_64, bits_to_u64, invert_64, sar_64, sext_32, shl_64, shr_64, u64_to_bits, zero_64,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AluResult64 {
    pub result: u64,
    pub carry: u8,
    pub overflow: u8,
    pub zero: u8,
    pub negative: u8,
}

fn result(value: u64, carry: u8, overflow: u8) -> AluResult64 {
    AluResult64 {
        result: value,
        carry,
        overflow,
        zero: zero_64(value),
        negative: u64_to_bits(value)[63],
    }
}

pub fn addq(a: u64, b: u64) -> AluResult64 {
    let (value, carry, overflow) = add_64(a, b, 0);
    result(value, carry, overflow)
}

pub fn subq(a: u64, b: u64) -> AluResult64 {
    let (value, carry, overflow) = add_64(a, invert_64(b), 1);
    result(value, carry, overflow)
}

fn bitwise(a: u64, b: u64, gate: fn(u8, u8) -> u8) -> AluResult64 {
    let a = u64_to_bits(a);
    let b = u64_to_bits(b);
    result(
        bits_to_u64(&std::array::from_fn::<_, 64, _>(|i| gate(a[i], b[i]))),
        0,
        0,
    )
}

pub fn andq(a: u64, b: u64) -> AluResult64 {
    bitwise(a, b, and_gate)
}

pub fn bis(a: u64, b: u64) -> AluResult64 {
    bitwise(a, b, or_gate)
}

pub fn xorq(a: u64, b: u64) -> AluResult64 {
    bitwise(a, b, xor_gate)
}

pub fn bic(a: u64, b: u64) -> AluResult64 {
    bitwise(a, invert_64(b), and_gate)
}

pub fn ornot(a: u64, b: u64) -> AluResult64 {
    bitwise(a, invert_64(b), or_gate)
}

pub fn eqv(a: u64, b: u64) -> AluResult64 {
    bitwise(a, invert_64(b), xor_gate)
}

pub fn addl(a: u64, b: u64) -> AluResult64 {
    let sum = addq(a, b);
    result(sext_32(sum.result), sum.carry, sum.overflow)
}

pub fn subl(a: u64, b: u64) -> AluResult64 {
    let difference = subq(a, b);
    result(
        sext_32(difference.result),
        difference.carry,
        difference.overflow,
    )
}

pub fn sll(a: u64, amount: u32) -> AluResult64 {
    result(shl_64(a, amount), 0, 0)
}

pub fn srl(a: u64, amount: u32) -> AluResult64 {
    result(shr_64(a, amount), 0, 0)
}

pub fn sra(a: u64, amount: u32) -> AluResult64 {
    result(sar_64(a, amount), 0, 0)
}

pub fn cmpeq(a: u64, b: u64) -> u64 {
    u64::from(subq(a, b).zero)
}

pub fn cmplt(a: u64, b: u64) -> u64 {
    let difference = subq(a, b);
    u64::from(xor_gate(difference.negative, difference.overflow))
}

pub fn cmple(a: u64, b: u64) -> u64 {
    let difference = subq(a, b);
    u64::from(or_gate(
        xor_gate(difference.negative, difference.overflow),
        difference.zero,
    ))
}

pub fn cmpult(a: u64, b: u64) -> u64 {
    u64::from(not_gate(subq(a, b).carry))
}

pub fn cmpule(a: u64, b: u64) -> u64 {
    let difference = subq(a, b);
    u64::from(or_gate(not_gate(difference.carry), difference.zero))
}

fn multiply_bits(a: u64, b: u64) -> [u8; 128] {
    let a = u64_to_bits(a);
    let b = u64_to_bits(b);
    let mut accumulator = [0; 128];
    for multiplier_bit in 0..64 {
        let partial: [u8; 128] = std::array::from_fn(|output_bit| {
            if output_bit >= multiplier_bit && output_bit - multiplier_bit < 64 {
                and_gate(a[output_bit - multiplier_bit], b[multiplier_bit])
            } else {
                0
            }
        });
        let sum = ripple_carry_adder_with_carry(&accumulator, &partial, 0).sum;
        accumulator.copy_from_slice(&sum);
    }
    accumulator
}

pub fn mulq(a: u64, b: u64) -> u64 {
    bits_to_u64(&multiply_bits(a, b)[..64])
}

pub fn umulh(a: u64, b: u64) -> u64 {
    bits_to_u64(&multiply_bits(a, b)[64..])
}

pub fn mull(a: u64, b: u64) -> u64 {
    sext_32(mulq(a, b))
}
