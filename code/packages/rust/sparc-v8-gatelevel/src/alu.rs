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
    add_32, add_32c, and_32, andn_32, bits_to_u32, compute_zero, not_32, or_32,
    orn_32, overflow_add, overflow_sub, sll_32, sra_32, srl_32, sub_32, sub_32b, u32_to_bits,
    u64_to_bits, xnor_32, xor_32,
};
use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::gates::{not_gate, xor_gate};

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
    Cc { n: bits[31], z: compute_zero(bits), v: 0, c: 0 }
}

/// AND: no CC update.
pub fn and32(a: u32, b: u32) -> u32 { bits_to_u32(&and_32(&u32_to_bits(a), &u32_to_bits(b))) }

/// ANDcc: update CC.
pub fn andcc32(a: u32, b: u32) -> (u32, Cc) {
    let bits = and_32(&u32_to_bits(a), &u32_to_bits(b));
    let r = bits_to_u32(&bits);
    (r, logic_cc(&bits))
}

/// ANDN: `rd = rs1 & ~src2`, no CC.
pub fn andn32(a: u32, b: u32) -> u32 { bits_to_u32(&andn_32(&u32_to_bits(a), &u32_to_bits(b))) }

/// ANDNcc: update CC.
pub fn andncc32(a: u32, b: u32) -> (u32, Cc) {
    let bits = andn_32(&u32_to_bits(a), &u32_to_bits(b));
    let r = bits_to_u32(&bits);
    (r, logic_cc(&bits))
}

/// OR: no CC.
pub fn or32(a: u32, b: u32) -> u32 { bits_to_u32(&or_32(&u32_to_bits(a), &u32_to_bits(b))) }

/// ORcc: update CC.
pub fn orcc32(a: u32, b: u32) -> (u32, Cc) {
    let bits = or_32(&u32_to_bits(a), &u32_to_bits(b));
    let r = bits_to_u32(&bits);
    (r, logic_cc(&bits))
}

/// ORN: `rd = rs1 | ~src2`, no CC.
pub fn orn32(a: u32, b: u32) -> u32 { bits_to_u32(&orn_32(&u32_to_bits(a), &u32_to_bits(b))) }

/// ORNcc: update CC.
pub fn orncc32(a: u32, b: u32) -> (u32, Cc) {
    let bits = orn_32(&u32_to_bits(a), &u32_to_bits(b));
    let r = bits_to_u32(&bits);
    (r, logic_cc(&bits))
}

/// XOR: no CC.
pub fn xor32(a: u32, b: u32) -> u32 { bits_to_u32(&xor_32(&u32_to_bits(a), &u32_to_bits(b))) }

/// XORcc: update CC.
pub fn xorcc32(a: u32, b: u32) -> (u32, Cc) {
    let bits = xor_32(&u32_to_bits(a), &u32_to_bits(b));
    let r = bits_to_u32(&bits);
    (r, logic_cc(&bits))
}

/// XNOR: `rd = ~(rs1 ^ src2)`, no CC.
pub fn xnor32(a: u32, b: u32) -> u32 { bits_to_u32(&xnor_32(&u32_to_bits(a), &u32_to_bits(b))) }

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
                for j in i..64 {
                    s[j] = ab[j - i];
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

// ─── Divide ───────────────────────────────────────────────────────────────────

/// UDIV: unsigned 64÷32 → 32-bit quotient.  Dividend is `Y:rs1`.
///
/// Saturates to `0xFFFF_FFFF` on overflow (quotient > 2^32-1).
/// Uses non-restoring shift-and-subtract via gate-level adders.
pub fn udiv64(y: u32, rs1: u32, src2: u32) -> u32 {
    if src2 == 0 {
        return 0xFFFF_FFFF;
    }
    let dividend = ((y as u64) << 32) | (rs1 as u64);
    let divisor = src2 as u64;
    // Quotient overflow check: if dividend / divisor > 0xFFFF_FFFF, saturate.
    if divisor != 0 && dividend / divisor > 0xFFFF_FFFF {
        return 0xFFFF_FFFF;
    }
    // Shift-and-subtract long division (64 steps, 1 bit of quotient per step).
    let divisor_bits = u64_to_bits(divisor);
    let mut remainder = u64_to_bits(0u64);
    let dividend_bits = u64_to_bits(dividend);
    let mut quotient = [0u8; 64];
    for i in (0..64).rev() {
        // Shift remainder left by 1, bring in next dividend bit.
        for j in (1..64).rev() {
            remainder[j] = remainder[j - 1];
        }
        remainder[0] = dividend_bits[i];
        // Try to subtract divisor from remainder.
        let not_div: Vec<u8> = divisor_bits.iter().map(|&b| not_gate(b)).collect();
        let r = ripple_carry_adder_with_carry(&remainder, &not_div, 1);
        // If no borrow (carry_out == 1), divisor fit → quotient bit = 1.
        if r.carry_out == 1 {
            quotient[i] = 1;
            remainder = r.sum;
        }
    }
    
    bits_to_u32(&quotient[..32])
}

/// SDIV: signed 64÷32 → 32-bit quotient.
///
/// Saturates to `0x7FFF_FFFF` (positive overflow) or `0x8000_0000` (negative
/// overflow) following SPARC V8 semantics.
pub fn sdiv64(y: u32, rs1: u32, src2: u32) -> u32 {
    if src2 == 0 {
        return 0x7FFF_FFFF;
    }
    let dividend = (((y as i32) as i64) << 32) | (rs1 as u64 as i64);
    let divisor = (src2 as i32) as i64;
    // i64::MIN / -1 would panic in debug (overflow) and give wrong result in
    // release.  Per SPARC V8 §5.2.9 this saturates to the positive maximum.
    if dividend == i64::MIN && divisor == -1 {
        return 0x7FFF_FFFF;
    }
    let q = dividend / divisor;
    if q > i32::MAX as i64 {
        0x7FFF_FFFF
    } else if q < i32::MIN as i64 {
        0x8000_0000
    } else {
        q as u32
    }
}

// ─── MULScc ───────────────────────────────────────────────────────────────────

/// MULScc: multiply step for the iterative signed multiply algorithm.
///
/// SPARC V8 §5.2.5: performs one step of the Booth-encoded multiply loop.
///
/// ```text
/// step_operand = (PSR.N XOR PSR.V) ? (Y:rd >> 1) + rs1 : (Y:rd >> 1)
///              adjusted by Y bit 0 as the Booth digit
/// ```
///
/// Simplified model matching the Python reference:
/// 1. The "partial product" operand is determined by `Y bit 0`.
/// 2. The 32-bit sum is `(rd >> 1) | (old_N_xor_V << 31)` ± rs1.
/// 3. Y is shifted right by 1; old rd bit 0 feeds into Y bit 31.
/// 4. CC are updated.
///
/// Returns `(new_rd, new_y, cc)`.
pub fn mulscc(rd: u32, y: u32, rs1: u32, psr_n: u8, psr_v: u8) -> (u32, u32, Cc) {
    // Operand = rs1 if Y[0] == 1, else 0.
    let y_bit0 = (y & 1) as u8;
    let operand = if y_bit0 == 1 { rs1 } else { 0u32 };

    // Shifted partial product: MSB from (N XOR V).
    let n_xor_v = xor_gate(psr_n, psr_v);
    let shifted_rd_bits = {
        let rb = u32_to_bits(rd);
        let mut s = srl_32(&rb, 1);
        s[31] = n_xor_v;
        s
    };
    let shifted_rd = bits_to_u32(&shifted_rd_bits);

    let (result, cc) = addcc32(shifted_rd, operand);

    // New Y: shift right, bring in old rd bit 0.
    let rd_bit0 = (rd & 1) as u8;
    let new_y_bits = {
        let yb = u32_to_bits(y);
        let mut s = srl_32(&yb, 1);
        s[31] = rd_bit0;
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
