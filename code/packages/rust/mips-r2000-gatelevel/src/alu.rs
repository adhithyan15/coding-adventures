//! alu.rs — 32-bit gate-level ALU for the MIPS R2000 simulator.
//!
//! Every data-path operation routes through:
//! - `and_gate`, `or_gate`, `xor_gate`, `not_gate` from `logic-gates`
//! - `ripple_carry_adder` from `arithmetic` (via `bits.rs` helpers)
//!
//! No native Rust arithmetic operators (`+`, `-`, `*`, `/`, `&`, `|`, `^`)
//! appear in any computation here.  Only control flow (`if`, `for`) and
//! array indexing.
//!
//! # ALU pipeline
//!
//! ```text
//! integer inputs
//!     ↓  int_to_bits32()
//! bit arrays (LSB-first)
//!     ↓  gate functions
//! result bit array
//!     ↓  bits_to_u32()
//! integer output + flags
//! ```
//!
//! # Two's complement subtraction
//!
//! SUB A, B ≡ ADD A, NOT(B), carry_in=1
//!
//! 32 NOT gates invert B; the ripple-carry adder adds A + NOT(B) + 1 = A − B.
//!
//! # Overflow (V flag)
//!
//! V = XOR(carry_into_MSB, carry_out_of_MSB)
//!
//! For SUB, the overflow of A − B is detected in the gate-level add of
//! A + NOT(B) + 1 — the same XOR test applies.
//!
//! # Multiplication (MULT / MULTU)
//!
//! Classical shift-and-add: for each of the 32 bits of b, if bit[i] is 1,
//! add (a << i) into the 64-bit accumulator via `add_64bit`.
//! Exactly 32 iterations.
//!
//! # Division (DIV / DIVU)
//!
//! Non-restoring long division in 32 iterations (one per quotient bit,
//! from MSB to LSB).  For each bit position `i` from 31..=0:
//!   1. `shifted_b = b << i` (may overflow 32 bits; skip if so)
//!   2. If `remainder >= shifted_b` (sub32 carry=1 → no borrow): subtract,
//!      set quotient bit.
//!
//! Exactly 32 outer iterations.

use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

use crate::bits::{
    add_32bit, add_64bit, bits_to_u32, bits_to_u64, int_to_bits32, int_to_bits64, invert_32bit,
    shl_32, shr_32_arith, shr_32_logical,
};

// ── ALU result type ───────────────────────────────────────────────────────────

/// Result of a 32-bit gate-level ALU operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AluResult32 {
    /// 32-bit unsigned result.
    pub result: u32,
    /// Carry out of bit 31 (unsigned overflow indicator, C flag).
    pub carry: u8,
    /// Signed overflow (V flag): 1 if two's-complement overflow occurred.
    pub overflow: u8,
    /// Zero flag: 1 if result == 0.
    pub zero: u8,
    /// Negative flag: sign bit (bit 31) of result.
    pub negative: u8,
}

fn make_result(value: u32, carry: u8, overflow: u8) -> AluResult32 {
    let bits = int_to_bits32(value);
    let zero = crate::bits::compute_zero(value);
    let negative = bits[31];
    AluResult32 { result: value, carry, overflow, zero, negative }
}

// ── Arithmetic operations ──────────────────────────────────────────────────────

/// 32-bit addition via ripple-carry adder.
///
/// Routes through 32 full adders.  Overflow is detected by comparing
/// carry_into_bit31 vs carry_out_of_bit31.
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::alu::add32;
/// let r = add32(1, 2, 0);
/// assert_eq!(r.result, 3); assert_eq!(r.zero, 0); assert_eq!(r.overflow, 0);
///
/// // Unsigned wrap
/// let r = add32(0xFFFF_FFFF, 1, 0);
/// assert_eq!(r.result, 0); assert_eq!(r.carry, 1); assert_eq!(r.zero, 1);
///
/// // Signed overflow: MAX_INT + 1
/// let r = add32(0x7FFF_FFFF, 1, 0);
/// assert_eq!(r.overflow, 1);
/// ```
pub fn add32(a: u32, b: u32, carry_in: u8) -> AluResult32 {
    let (result, carry, overflow) = add_32bit(a, b, carry_in);
    make_result(result, carry, overflow)
}

/// 32-bit subtraction via two's complement: A + NOT(B) + 1.
///
/// Hardware: 32 NOT gates invert B, then feed into ripple-carry adder
/// with carry_in=1.  No subtraction hardware needed.
///
/// Carry interpretation: carry=1 → no borrow (A ≥ B unsigned).
/// carry=0 → borrow occurred (A < B unsigned).
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::alu::sub32;
/// let r = sub32(5, 3);
/// assert_eq!(r.result, 2); assert_eq!(r.carry, 1); // no borrow
///
/// let r = sub32(3, 5);
/// assert_eq!(r.carry, 0); // borrow: 3 < 5 unsigned
/// ```
pub fn sub32(a: u32, b: u32) -> AluResult32 {
    let not_b = invert_32bit(b);
    let (result, carry, overflow) = add_32bit(a, not_b, 1);
    make_result(result, carry, overflow)
}

// ── Bitwise operations ─────────────────────────────────────────────────────────

/// 32-bit bitwise AND: 32 AND gates in parallel.
///
/// carry=0, overflow=0 (bitwise ops don't set these flags).
pub fn and32(a: u32, b: u32) -> AluResult32 {
    let a_bits = int_to_bits32(a);
    let b_bits = int_to_bits32(b);
    let mut result_bits = [0u8; 32];
    for i in 0..32 {
        result_bits[i] = and_gate(a_bits[i], b_bits[i]);
    }
    make_result(bits_to_u32(result_bits), 0, 0)
}

/// 32-bit bitwise OR: 32 OR gates in parallel.
pub fn or32(a: u32, b: u32) -> AluResult32 {
    let a_bits = int_to_bits32(a);
    let b_bits = int_to_bits32(b);
    let mut result_bits = [0u8; 32];
    for i in 0..32 {
        result_bits[i] = or_gate(a_bits[i], b_bits[i]);
    }
    make_result(bits_to_u32(result_bits), 0, 0)
}

/// 32-bit bitwise XOR: 32 XOR gates in parallel.
pub fn xor32(a: u32, b: u32) -> AluResult32 {
    let a_bits = int_to_bits32(a);
    let b_bits = int_to_bits32(b);
    let mut result_bits = [0u8; 32];
    for i in 0..32 {
        result_bits[i] = xor_gate(a_bits[i], b_bits[i]);
    }
    make_result(bits_to_u32(result_bits), 0, 0)
}

/// 32-bit bitwise NOR: OR then NOT, 32 bit positions.
///
/// NOR(a, b) = NOT(OR(a, b)).  Used by the MIPS NOR instruction and
/// to implement bitwise NOT: `NOR rd, rs, $zero`.
pub fn nor32(a: u32, b: u32) -> AluResult32 {
    let a_bits = int_to_bits32(a);
    let b_bits = int_to_bits32(b);
    let mut result_bits = [0u8; 32];
    for i in 0..32 {
        result_bits[i] = not_gate(or_gate(a_bits[i], b_bits[i]));
    }
    make_result(bits_to_u32(result_bits), 0, 0)
}

// ── Comparison operations ──────────────────────────────────────────────────────

/// Set Less Than (signed): result = 1 if signed(a) < signed(b), else 0.
///
/// Implementation: `less = XOR(diff.negative, diff.overflow)` where
/// diff = sub32(a, b).
///
/// Truth table:
/// ```text
/// N=0, V=0 → result ≥ 0, no overflow → a >= b → 0
/// N=1, V=0 → result < 0, no overflow → a < b  → 1
/// N=0, V=1 → result ≥ 0, overflow    → a < b  → 1
/// N=1, V=1 → result < 0, overflow    → a >= b → 0
/// ```
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::alu::slt32;
/// let r = slt32(3, 5); assert_eq!(r.result, 1);
/// let r = slt32(5, 3); assert_eq!(r.result, 0);
/// let r = slt32(0x8000_0000, 1); assert_eq!(r.result, 1); // -MIN < 1 (signed)
/// ```
pub fn slt32(a: u32, b: u32) -> AluResult32 {
    let diff = sub32(a, b);
    let less = xor_gate(diff.negative, diff.overflow);
    make_result(less as u32, 0, 0)
}

/// Set Less Than Unsigned: result = 1 if unsigned(a) < unsigned(b), else 0.
///
/// Uses borrow from sub32: carry=0 means borrow → a < b unsigned.
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::alu::sltu32;
/// let r = sltu32(3, 5); assert_eq!(r.result, 1);
/// let r = sltu32(5, 3); assert_eq!(r.result, 0);
/// let r = sltu32(0xFFFF_FFFF, 1); assert_eq!(r.result, 0); // 0xFFFF > 1 unsigned
/// ```
pub fn sltu32(a: u32, b: u32) -> AluResult32 {
    let diff = sub32(a, b);
    let less = not_gate(diff.carry);
    make_result(less as u32, 0, 0)
}

// ── Shift operations ───────────────────────────────────────────────────────────

/// Shift Left Logical by `shamt` (0–31).
pub fn sll32(a: u32, shamt: u32) -> AluResult32 {
    make_result(shl_32(a, shamt), 0, 0)
}

/// Shift Right Logical by `shamt` (0–31): zero-fills from MSB.
pub fn srl32(a: u32, shamt: u32) -> AluResult32 {
    make_result(shr_32_logical(a, shamt), 0, 0)
}

/// Shift Right Arithmetic by `shamt` (0–31): sign-fills from MSB.
pub fn sra32(a: u32, shamt: u32) -> AluResult32 {
    make_result(shr_32_arith(a, shamt), 0, 0)
}

// ── Multiplication ─────────────────────────────────────────────────────────────

/// Unsigned 32×32 → 64-bit multiply via shift-and-add.
///
/// Algorithm: for each of the 32 bits of b, if bit[i] is 1, add (a << i)
/// into the 64-bit accumulator using `add_64bit` (gate-level).
/// Exactly 32 iterations.
///
/// Returns `(hi, lo)`: upper and lower 32 bits of the 64-bit product.
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::alu::multu32;
/// let (hi, lo) = multu32(6, 7);
/// assert_eq!(lo, 42); assert_eq!(hi, 0);
///
/// // Large product: 0xFFFF_FFFF * 0xFFFF_FFFF
/// let (hi, lo) = multu32(0xFFFF_FFFF, 0xFFFF_FFFF);
/// assert_eq!(hi, 0xFFFF_FFFE);
/// assert_eq!(lo, 0x0000_0001);
/// ```
pub fn multu32(a: u32, b: u32) -> (u32, u32) {
    let b_bits = int_to_bits32(b);
    let mut product = 0u64;
    for bit_idx in 0..32usize {
        if b_bits[bit_idx] == 1 {
            // Shift a left by bit_idx positions using 64-bit bit manipulation.
            // bit_idx is in 0..32 so [bit_idx..] is always in-bounds.
            let a_64 = int_to_bits64(a as u64);
            let mut shifted = [0u8; 64];
            shifted[bit_idx..].copy_from_slice(&a_64[..64 - bit_idx]);
            let partial = bits_to_u64(shifted);
            let (new_product, _) = add_64bit(product, partial, 0);
            product = new_product;
        }
    }
    let hi = (product >> 32) as u32;
    let lo = product as u32;
    (hi, lo)
}

/// Signed 32×32 → 64-bit multiply.
///
/// Handles signs manually: compute |a|×|b| (unsigned via `multu32`),
/// then negate the 64-bit result if signs differ.
///
/// Returns `(hi, lo)`: upper and lower 32 bits of the 64-bit signed product.
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::alu::mult32;
/// // -1 * 1 = -1 (64-bit: 0xFFFF_FFFF_FFFF_FFFF)
/// let (hi, lo) = mult32(0xFFFF_FFFFu32, 1);
/// assert_eq!(hi, 0xFFFF_FFFF);
/// assert_eq!(lo, 0xFFFF_FFFF);
///
/// // -1 * -1 = 1
/// let (hi, lo) = mult32(0xFFFF_FFFFu32, 0xFFFF_FFFFu32);
/// assert_eq!(hi, 0); assert_eq!(lo, 1);
/// ```
pub fn mult32(a: u32, b: u32) -> (u32, u32) {
    let a_bits = int_to_bits32(a);
    let b_bits = int_to_bits32(b);
    let sign_a = a_bits[31];
    let sign_b = b_bits[31];

    // Compute |a|: if negative, negate via invert + 1
    let a_abs = if sign_a == 1 {
        let (neg, _, _) = add_32bit(invert_32bit(a), 0, 1);
        neg
    } else {
        a
    };

    let b_abs = if sign_b == 1 {
        let (neg, _, _) = add_32bit(invert_32bit(b), 0, 1);
        neg
    } else {
        b
    };

    let (hi, lo) = multu32(a_abs, b_abs);

    // Negate 64-bit result if signs differ
    let result_negative = xor_gate(sign_a, sign_b);
    if result_negative == 1 {
        // Negate 64-bit: invert all 64 bits, add 1
        let combined: u64 = ((hi as u64) << 32) | (lo as u64);
        let combined_bits = int_to_bits64(combined);
        let mut inv_bits = [0u8; 64];
        for i in 0..64 {
            inv_bits[i] = not_gate(combined_bits[i]);
        }
        let inv_val = bits_to_u64(inv_bits);
        let (neg_val, _) = add_64bit(inv_val, 0, 1);
        let hi_out = (neg_val >> 32) as u32;
        let lo_out = neg_val as u32;
        (hi_out, lo_out)
    } else {
        (hi, lo)
    }
}

// ── Division ───────────────────────────────────────────────────────────────────

/// Unsigned 32-bit division via 32-iteration non-restoring long division.
///
/// For each bit position from 31 down to 0:
///   1. `shifted_b = b << bit_idx` (as a 64-bit value; skip if > 32 bits)
///   2. If `remainder >= shifted_b` (sub32 carry=1): subtract, set quotient bit.
///
/// Exactly 32 outer iterations.
///
/// If b == 0, returns `(0xFFFF_FFFF, a)` matching hardware undefined behavior.
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::alu::divu32;
/// let (q, r) = divu32(10, 3); assert_eq!(q, 3); assert_eq!(r, 1);
/// let (q, r) = divu32(7, 2);  assert_eq!(q, 3); assert_eq!(r, 1);
/// let (q, r) = divu32(0, 5);  assert_eq!(q, 0); assert_eq!(r, 0);
/// ```
pub fn divu32(a: u32, b: u32) -> (u32, u32) {
    if b == 0 {
        return (0xFFFF_FFFF, a);
    }

    let mut quotient = 0u32;
    let mut remainder = a;

    // 32 iterations: one per quotient bit, from MSB (bit 31) to LSB (bit 0)
    for bit_idx in (0..32u32).rev() {
        // Shift b left by bit_idx using 64-bit representation.
        // If the shifted value overflows 32 bits, the shifted divisor is
        // larger than any 32-bit remainder — skip this bit position.
        let b_64 = int_to_bits64(b as u64);
        let bidx = bit_idx as usize;
        // bidx is in 0..32 so [bidx..] is always in-bounds on the 64-element array.
        let mut shifted_64 = [0u8; 64];
        shifted_64[bidx..].copy_from_slice(&b_64[..64 - bidx]);
        let shifted_b_val = bits_to_u64(shifted_64);

        if shifted_b_val > 0xFFFF_FFFF {
            continue; // shifted divisor doesn't fit in 32 bits; skip
        }
        let shifted_b = shifted_b_val as u32;

        // sub32 carry=1 means no borrow → remainder >= shifted_b
        let diff = sub32(remainder, shifted_b);
        if diff.carry == 1 {
            remainder = diff.result;
            // Set quotient bit at bit_idx via bit-list manipulation
            let mut q_bits = int_to_bits32(quotient);
            q_bits[bit_idx as usize] = 1;
            quotient = bits_to_u32(q_bits);
        }
    }

    (quotient, remainder)
}

/// Signed 32-bit division.
///
/// Handles signs manually: compute |a| / |b| (unsigned), then apply
/// sign rules:
/// - Quotient is negative if operands have opposite signs.
/// - Remainder has the same sign as the dividend (MIPS convention).
///
/// If b == 0, returns `(0xFFFF_FFFF, a)` matching hardware.
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::alu::div32;
/// let (q, r) = div32(10, 3); assert_eq!(q, 3); assert_eq!(r, 1);
///
/// // -10 / 3 = -3 remainder -1 (sign of dividend)
/// let (q, r) = div32(0xFFFF_FFF6u32, 3); // -10 / 3
/// assert_eq!(q, 0xFFFF_FFFD); // -3
/// assert_eq!(r, 0xFFFF_FFFF); // -1
/// ```
pub fn div32(a: u32, b: u32) -> (u32, u32) {
    let a_bits = int_to_bits32(a);
    let b_bits = int_to_bits32(b);
    let sign_a = a_bits[31];
    let sign_b = b_bits[31];

    let a_abs = if sign_a == 1 {
        let (neg, _, _) = add_32bit(invert_32bit(a), 0, 1);
        neg
    } else {
        a
    };

    let b_abs = if sign_b == 1 {
        let (neg, _, _) = add_32bit(invert_32bit(b), 0, 1);
        neg
    } else {
        b
    };

    if b_abs == 0 {
        return (0xFFFF_FFFF, a);
    }

    let (q_abs, r_abs) = divu32(a_abs, b_abs);

    // Quotient is negative if signs differ
    let quot_negative = xor_gate(sign_a, sign_b);
    let quotient = if quot_negative == 1 {
        let (neg, _, _) = add_32bit(invert_32bit(q_abs), 0, 1);
        neg
    } else {
        q_abs
    };

    // Remainder has same sign as dividend
    let remainder = if sign_a == 1 && r_abs != 0 {
        let (neg, _, _) = add_32bit(invert_32bit(r_abs), 0, 1);
        neg
    } else {
        r_abs
    };

    (quotient, remainder)
}
