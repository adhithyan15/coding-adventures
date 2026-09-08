//! SPARC V8 ALU — all operations route through logic gates.
//!
//! # Condition codes (PSR bits 23:20)
//!
//! ```text
//!  N (negative): result bit 31
//!  Z (zero):     NOR of all result bits
//!  V (overflow): XOR(carry_into_bit31, carry_out_of_bit31)
//!  C (carry):    carry_out for ADD; NOT(carry_out) for SUB (i.e., borrow)
//! ```
//!
//! Instructions with the `cc` suffix update these flags; others leave PSR alone.

use crate::bits::{
    add_32, add_32c, and_32, andn_32, bits_to_u32, compute_zero, not_32, or_32, orn_32,
    overflow_add, overflow_sub, sll_32, sra_32, srl_32, sub_32, sub_32b, u32_to_bits, u64_to_bits,
    xnor_32, xor_32,
};
use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

/// Condition codes produced by an ALU operation.
#[derive(Debug, Clone, Copy, Default)]
pub struct Cc {
    pub n: u8,
    pub z: u8,
    pub v: u8,
    pub c: u8,
}

// ─── Integer arithmetic ───────────────────────────────────────────────────────

/// ADD: `rd = rs1 + src2` — no CC update.
pub fn add32(a: u32, b: u32) -> u32 {
    let (sum, _) = add_32(&u32_to_bits(a), &u32_to_bits(b));
    bits_to_u32(&sum)
}

/// ADDcc: `rd = rs1 + src2`, update N, Z, V, C.
pub fn addcc32(a: u32, b: u32) -> (u32, Cc) {
    let ab = u32_to_bits(a);
    let bb = u32_to_bits(b);
    let (sum, c_out) = add_32(&ab, &bb);
    let result = bits_to_u32(&sum);
    let v = overflow_add(&ab, &bb, 0);
    let cc = Cc {
        n: sum[31],
        z: compute_zero(&sum),
        v,
        c: c_out,
    };
    (result, cc)
}

/// ADDX: `rd = rs1 + src2 + C` — no CC update (adds carry-in from PSR.C).
pub fn addx32(a: u32, b: u32, c_in: u8) -> u32 {
    let (sum, _) = add_32c(&u32_to_bits(a), &u32_to_bits(b), c_in);
    bits_to_u32(&sum)
}

/// ADDXcc: `rd = rs1 + src2 + C`, update CC.
pub fn addxcc32(a: u32, b: u32, c_in: u8) -> (u32, Cc) {
    let ab = u32_to_bits(a);
    let bb = u32_to_bits(b);
    let (sum, c_out) = add_32c(&ab, &bb, c_in);
    let result = bits_to_u32(&sum);
    let v = overflow_add(&ab, &bb, c_in);
    let cc = Cc {
        n: sum[31],
        z: compute_zero(&sum),
        v,
        c: c_out,
    };
    (result, cc)
}

/// SUB: `rd = rs1 - src2` — no CC update.
pub fn sub32(a: u32, b: u32) -> u32 {
    let (diff, _) = sub_32(&u32_to_bits(a), &u32_to_bits(b));
    bits_to_u32(&diff)
}

/// SUBcc: `rd = rs1 - src2`, update N, Z, V, C.
pub fn subcc32(a: u32, b: u32) -> (u32, Cc) {
    let ab = u32_to_bits(a);
    let bb = u32_to_bits(b);
    let (diff, borrow) = sub_32(&ab, &bb);
    let result = bits_to_u32(&diff);
    let v = overflow_sub(&ab, &bb);
    let cc = Cc {
        n: diff[31],
        z: compute_zero(&diff),
        v,
        c: borrow,
    };
    (result, cc)
}

/// SUBX: `rd = rs1 - src2 - C` — no CC update.
pub fn subx32(a: u32, b: u32, borrow_in: u8) -> u32 {
    let (diff, _) = sub_32b(&u32_to_bits(a), &u32_to_bits(b), borrow_in);
    bits_to_u32(&diff)
}

/// SUBXcc: `rd = rs1 - src2 - C`, update CC.
pub fn subxcc32(a: u32, b: u32, borrow_in: u8) -> (u32, Cc) {
    let ab = u32_to_bits(a);
    let bb = u32_to_bits(b);
    let (diff, borrow_out) = sub_32b(&ab, &bb, borrow_in);
    let result = bits_to_u32(&diff);
    // For SUBXcc, overflow is the same XOR-of-carries logic using the borrow_in.
    // Convert borrow_in to carry_in for the two's-complement sub circuit:
    // sub_32b uses carry_in = not(borrow_in) internally.
    let b_inv = not_32(&bb);
    let carry_in = not_gate(borrow_in);
    let v = overflow_add(&ab, &b_inv, carry_in);
    let cc = Cc {
        n: diff[31],
        z: compute_zero(&diff),
        v,
        c: borrow_out,
    };
    (result, cc)
}

// ─── Logical operations ───────────────────────────────────────────────────────

/// Derive CC from a logic result (no carry or overflow for logical ops).
fn logic_cc(bits: &[u8]) -> Cc {
    Cc {
        n: bits[31],
        z: compute_zero(bits),
        v: 0,
        c: 0,
    }
}

/// AND: no CC update.
pub fn and32(a: u32, b: u32) -> u32 {
    bits_to_u32(&and_32(&u32_to_bits(a), &u32_to_bits(b)))
}

/// ANDcc: update CC.
pub fn andcc32(a: u32, b: u32) -> (u32, Cc) {
    let bits = and_32(&u32_to_bits(a), &u32_to_bits(b));
    let r = bits_to_u32(&bits);
    (r, logic_cc(&bits))
}

/// ANDN: `rd = rs1 & ~src2`, no CC.
pub fn andn32(a: u32, b: u32) -> u32 {
    bits_to_u32(&andn_32(&u32_to_bits(a), &u32_to_bits(b)))
}

/// ANDNcc: update CC.
pub fn andncc32(a: u32, b: u32) -> (u32, Cc) {
    let bits = andn_32(&u32_to_bits(a), &u32_to_bits(b));
    let r = bits_to_u32(&bits);
    (r, logic_cc(&bits))
}

/// OR: no CC.
pub fn or32(a: u32, b: u32) -> u32 {
    bits_to_u32(&or_32(&u32_to_bits(a), &u32_to_bits(b)))
}

/// ORcc: update CC.
pub fn orcc32(a: u32, b: u32) -> (u32, Cc) {
    let bits = or_32(&u32_to_bits(a), &u32_to_bits(b));
    let r = bits_to_u32(&bits);
    (r, logic_cc(&bits))
}

/// ORN: `rd = rs1 | ~src2`, no CC.
pub fn orn32(a: u32, b: u32) -> u32 {
    bits_to_u32(&orn_32(&u32_to_bits(a), &u32_to_bits(b)))
}

/// ORNcc: update CC.
pub fn orncc32(a: u32, b: u32) -> (u32, Cc) {
    let bits = orn_32(&u32_to_bits(a), &u32_to_bits(b));
    let r = bits_to_u32(&bits);
    (r, logic_cc(&bits))
}

/// XOR: no CC.
pub fn xor32(a: u32, b: u32) -> u32 {
    bits_to_u32(&xor_32(&u32_to_bits(a), &u32_to_bits(b)))
}

/// XORcc: update CC.
pub fn xorcc32(a: u32, b: u32) -> (u32, Cc) {
    let bits = xor_32(&u32_to_bits(a), &u32_to_bits(b));
    let r = bits_to_u32(&bits);
    (r, logic_cc(&bits))
}

/// XNOR: `rd = ~(rs1 ^ src2)`, no CC.
pub fn xnor32(a: u32, b: u32) -> u32 {
    bits_to_u32(&xnor_32(&u32_to_bits(a), &u32_to_bits(b)))
}

/// XNORcc: update CC.
pub fn xnorcc32(a: u32, b: u32) -> (u32, Cc) {
    let bits = xnor_32(&u32_to_bits(a), &u32_to_bits(b));
    let r = bits_to_u32(&bits);
    (r, logic_cc(&bits))
}

// ─── Shifts ───────────────────────────────────────────────────────────────────

/// SLL: shift left logical.
pub fn sll32(a: u32, shamt: u32) -> u32 {
    bits_to_u32(&sll_32(&u32_to_bits(a), shamt))
}

/// SRL: shift right logical.
pub fn srl32(a: u32, shamt: u32) -> u32 {
    bits_to_u32(&srl_32(&u32_to_bits(a), shamt))
}

/// SRA: shift right arithmetic.
pub fn sra32(a: u32, shamt: u32) -> u32 {
    bits_to_u32(&sra_32(&u32_to_bits(a), shamt))
}

// ─── Multiply ─────────────────────────────────────────────────────────────────

/// UMUL: unsigned 32×32 → 64-bit product.  Returns `(y_high32, rd_low32)`.
///
/// Implements shift-and-add multiplication using gate-level adders:
///
/// ```text
///  product = 0
///  for each bit i of multiplier:
///      if bit i == 1: product += (multiplicand << i)
///  Y = product[63:32], rd = product[31:0]
/// ```
pub fn umul32(a: u32, b: u32) -> (u32, u32) {
    let ab = u64_to_bits(a as u64);
    let bb = u32_to_bits(b);
    let mut acc = vec![0u8; 64];
    for i in 0..32 {
        if bb[i] == 1 {
            // Partial product: multiplicand shifted left by i positions.
            let shifted: Vec<u8> = {
                let mut s = vec![0u8; 64];
                for j in i..64 {
                    if j - i < ab.len() {
                        s[j] = ab[j - i];
                    }
                }
                s
            };
            let r = ripple_carry_adder_with_carry(&acc, &shifted, 0);
            acc = r.sum;
        }
    }
    let rd = bits_to_u32(&acc[..32]);
    let y = bits_to_u32(&acc[32..64]);
    (y, rd)
}

/// SMUL: signed 32×32 → 64-bit product.  Returns `(y_high32, rd_low32)`.
///
/// Sign-extends both operands to 64 bits, then performs the same shift-and-add.
pub fn smul32(a: u32, b: u32) -> (u32, u32) {
    // Sign-extend 32-bit operands to 64 bits.
    let a64 = (a as i32) as i64 as u64;
    let b64 = (b as i32) as i64 as u64;
    let ab = u64_to_bits(a64);
    let bb = u64_to_bits(b64);
    let mut acc = vec![0u8; 64];
    for i in 0..64 {
        if bb[i] == 1 {
            let shifted: Vec<u8> = {
                let mut s = vec![0u8; 64];
                s[i..64].copy_from_slice(&ab[..64 - i]);
                s
            };
            let r = ripple_carry_adder_with_carry(&acc, &shifted, 0);
            acc = r.sum;
        }
    }
    let rd = bits_to_u32(&acc[..32]);
    let y = bits_to_u32(&acc[32..64]);
    (y, rd)
}

// ─── Divide ───────────────────────────────────────────────────────────────────

fn twos_complement(bits: &[u8]) -> Vec<u8> {
    let inverted: Vec<u8> = bits.iter().copied().map(not_gate).collect();
    let mut one = vec![0; bits.len()];
    one[0] = 1;
    ripple_carry_adder_with_carry(&inverted, &one, 0).sum
}

fn divide_unsigned_bits(dividend: &[u8], divisor: &[u8]) -> Vec<u8> {
    let mut remainder = vec![0; 64];
    let mut quotient = vec![0; 64];
    let inverted_divisor: Vec<u8> = divisor.iter().copied().map(not_gate).collect();
    for i in (0..64).rev() {
        for j in (1..64).rev() {
            remainder[j] = remainder[j - 1];
        }
        remainder[0] = dividend[i];
        let subtraction = ripple_carry_adder_with_carry(&remainder, &inverted_divisor, 1);
        quotient[i] = subtraction.carry_out;
        for (remainder_bit, sum_bit) in remainder.iter_mut().zip(&subtraction.sum) {
            *remainder_bit = if subtraction.carry_out == 1 {
                *sum_bit
            } else {
                *remainder_bit
            };
        }
    }
    quotient
}

fn any_high(bits: &[u8]) -> u8 {
    bits.iter().copied().fold(0, or_gate)
}

/// UDIV plus the quotient-overflow signal used by UDIVcc.
pub fn udiv64_with_overflow(y: u32, rs1: u32, src2: u32) -> (u32, bool) {
    if src2 == 0 {
        return (u32::MAX, false);
    }
    let mut dividend = u32_to_bits(rs1);
    dividend.extend(u32_to_bits(y));
    let mut divisor = u32_to_bits(src2);
    divisor.resize(64, 0);
    let quotient = divide_unsigned_bits(&dividend, &divisor);
    let overflow = any_high(&quotient[32..]) != 0;
    (
        if overflow {
            u32::MAX
        } else {
            bits_to_u32(&quotient[..32])
        },
        overflow,
    )
}

/// UDIV: unsigned 64÷32 → saturated 32-bit quotient.
pub fn udiv64(y: u32, rs1: u32, src2: u32) -> u32 {
    udiv64_with_overflow(y, rs1, src2).0
}

/// SDIV plus the quotient-overflow signal used by SDIVcc.
pub fn sdiv64_with_overflow(y: u32, rs1: u32, src2: u32) -> (u32, bool) {
    if src2 == 0 {
        return (i32::MAX as u32, false);
    }
    let mut dividend = u32_to_bits(rs1);
    dividend.extend(u32_to_bits(y));
    let dividend_negative = dividend[63];
    if dividend_negative == 1 {
        dividend = twos_complement(&dividend);
    }
    let mut divisor = u32_to_bits(src2);
    let divisor_negative = divisor[31];
    if divisor_negative == 1 {
        divisor = twos_complement(&divisor);
    }
    divisor.resize(64, 0);

    let quotient = divide_unsigned_bits(&dividend, &divisor);
    let negative = xor_gate(dividend_negative, divisor_negative);
    let high = any_high(&quotient[32..]);
    let positive_overflow = or_gate(high, quotient[31]);
    let negative_overflow = or_gate(high, and_gate(quotient[31], any_high(&quotient[..31])));
    let overflow = if negative == 1 {
        negative_overflow == 1
    } else {
        positive_overflow == 1
    };
    if overflow {
        return (
            if negative == 1 {
                i32::MIN as u32
            } else {
                i32::MAX as u32
            },
            true,
        );
    }
    let low = if negative == 1 {
        twos_complement(&quotient[..32])
    } else {
        quotient[..32].to_vec()
    };
    (bits_to_u32(&low), false)
}

/// SDIV: signed 64÷32 → saturated 32-bit quotient.
pub fn sdiv64(y: u32, rs1: u32, src2: u32) -> u32 {
    sdiv64_with_overflow(y, rs1, src2).0
}

// ─── MULScc ───────────────────────────────────────────────────────────────────

/// MULScc: multiply step for the iterative signed multiply algorithm.
///
/// SPARC V8 §5.2.5: performs one step of the Booth-encoded multiply loop.
///
/// The repository contract shifts `src2` right with old Y bit zero entering
/// bit 31, then conditionally adds `rs1` when old N XOR V is set. Y shifts the
/// old destination right with result bit zero entering bit 31. Condition codes
/// come from the addition.
///
/// Returns `(new_rd, new_y, cc)`.
pub fn mulscc(rd: u32, y: u32, rs1: u32, src2: u32, psr_n: u8, psr_v: u8) -> (u32, u32, Cc) {
    let y_bit0 = (y & 1) as u8;
    let n_xor_v = xor_gate(psr_n, psr_v);
    let operand = if n_xor_v == 1 { rs1 } else { 0u32 };

    // Shift src2 right; Y[0] enters its high bit before the conditional add.
    let shifted_bits = {
        let mut s = srl_32(&u32_to_bits(src2), 1);
        s[31] = y_bit0;
        s
    };
    let (result, cc) = addcc32(bits_to_u32(&shifted_bits), operand);

    // New Y: shift the old destination right; result bit zero enters bit 31.
    let result_bit0 = (result & 1) as u8;
    let new_y_bits = {
        let mut s = srl_32(&u32_to_bits(rd), 1);
        s[31] = result_bit0;
        s
    };
    let new_y = bits_to_u32(&new_y_bits);

    (result, new_y, cc)
}

// ─── SETHI ────────────────────────────────────────────────────────────────────

/// SETHI: `rd = imm22 << 10`.  The low 10 bits are zeroed.
pub fn sethi(imm22: u32) -> u32 {
    sll32(imm22 & 0x003F_FFFF, 10)
}
