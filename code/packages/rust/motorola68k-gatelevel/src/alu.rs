//! ALU operations for the Motorola 68000 — all arithmetic through gate primitives.
//!
//! ## 68000 Condition Codes (CCR)
//!
//! The 68000 has five condition code bits stored in the low 5 bits of SR:
//!
//! ```text
//! bit 4: X — eXtend (set same as C for ADD/SUB; preserved by logic ops)
//! bit 3: N — Negative (MSB of result)
//! bit 2: Z — Zero (result == 0)
//! bit 1: V — oVerflow (signed overflow)
//! bit 0: C — Carry/borrow
//! ```
//!
//! Key differences from the Intel 8086:
//! - **No AF flag** — the 68000 handles BCD without nibble-carry.
//! - **No PF flag** — parity not tracked.
//! - **X flag** — a separate extend bit used by ADDX/SUBX/NEGX for
//!   multi-precision arithmetic chains.  Logic ops and shifts preserve X
//!   (it is only set by ADD/SUB/NEG and their extended variants).
//!
//! ## Subtraction model
//!
//! SUB uses two's-complement addition: `A - B = A + NOT(B) + 1`.
//!
//! ```text
//! NOT(B) via N NOT gates → inverts every bit
//! carry-in = 1          → adds 1 to produce two's complement
//! C flag   = NOT(carry_out of MSB stage)  [carry out = 1 means no borrow]
//! V flag   = XOR(carries[N-2], carries[N-1])  [same XOR gate as ADD]
//! ```
//!
//! For SUBX (subtract with extend): `A - B - X`.
//! If X=0: carry-in = 1 (normal subtraction).
//! If X=1: carry-in = NOT(1) + invert... actually:
//!   `A - B - X = A + NOT(B) + (1 - X)`.
//!   carry-in = 1 when X=0, carry-in = 0 when X=1 → carry-in = NOT(X).

use crate::bits::{
    add_16bit_full, add_32bit_full, add_8bit_full, compute_c_neg, compute_n16, compute_n32,
    compute_n8, compute_v_from_carries, compute_v_neg, compute_z, compute_z16, compute_z32,
    compute_z8, int_to_bits16, int_to_bits32, int_to_bits8, not_16bit, not_32bit, not_8bit,
};
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

// ── Result struct ─────────────────────────────────────────────────────────────

/// Result of an ALU operation, with all 68000 condition code flags.
///
/// The `result` field holds the full 32-bit result (masked appropriately by
/// the caller for 8-bit and 16-bit operations).
///
/// For logic operations (AND, OR, XOR, NOT): V=0, C=0, X is unchanged
/// (callers must preserve the old X flag when updating SR).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AluResult68K {
    /// Operation result (mask to u8/u16 as appropriate).
    pub result: u32,
    /// Carry/borrow flag (bit 0 of CCR).
    pub flag_c: u8,
    /// Overflow flag (bit 1 of CCR).
    pub flag_v: u8,
    /// Zero flag (bit 2 of CCR).
    pub flag_z: u8,
    /// Negative flag (bit 3 of CCR).
    pub flag_n: u8,
    /// Extend flag (bit 4 of CCR).  Set same as C for ADD/SUB/NEG.
    pub flag_x: u8,
}

// ── 32-bit ADD ────────────────────────────────────────────────────────────────

/// 32-bit ADD: `result = a + b + carry_in`.
///
/// Use `carry_in = 0` for plain ADD; `carry_in = x_flag` for ADDX.
///
/// ```
/// use coding_adventures_motorola68k_gatelevel::alu::add32;
/// let r = add32(5, 3, 0);
/// assert_eq!(r.result, 8);
/// assert_eq!(r.flag_c, 0);
/// assert_eq!(r.flag_v, 0);
/// assert_eq!(r.flag_z, 0);
/// assert_eq!(r.flag_n, 0);
/// ```
pub fn add32(a: u32, b: u32, carry_in: u8) -> AluResult68K {
    let (result, carries) = add_32bit_full(a, b, carry_in);
    let flag_c = carries[31];
    let flag_v = compute_v_from_carries(&carries);
    let flag_n = compute_n32(result);
    let flag_z = compute_z32(result);
    AluResult68K {
        result,
        flag_c,
        flag_v,
        flag_n,
        flag_z,
        flag_x: flag_c,
    }
}

/// 32-bit SUB: `result = a - b`.
///
/// Implemented as `A + NOT(B) + 1` (two's complement).
/// C = NOT(carry_out) — borrow indicator; 1 means borrow occurred.
///
/// ```
/// use coding_adventures_motorola68k_gatelevel::alu::sub32;
/// let r = sub32(10, 3, 0);
/// assert_eq!(r.result, 7);
/// assert_eq!(r.flag_c, 0); // no borrow
/// assert_eq!(r.flag_v, 0);
/// ```
pub fn sub32(a: u32, b: u32, borrow_in: u8) -> AluResult68K {
    // SUB: A + NOT(B) + (1 - borrow_in)
    // borrow_in=0 → carry-in=1 (normal sub); borrow_in=1 → carry-in=0 (SUBX)
    let b_inv = not_32bit(b);
    let cin = not_gate(borrow_in); // 1 when no borrow, 0 when borrow (SUBX)
    let (result, carries) = add_32bit_full(a, b_inv, cin);
    // C = NOT(carry_out): carry_out=1 means no borrow (a >= b)
    let flag_c = not_gate(carries[31]);
    let flag_v = compute_v_from_carries(&carries);
    let flag_n = compute_n32(result);
    let flag_z = compute_z32(result);
    AluResult68K {
        result,
        flag_c,
        flag_v,
        flag_n,
        flag_z,
        flag_x: flag_c,
    }
}

/// 32-bit NEG: `result = 0 - src` (two's complement negation).
///
/// NEG sets C = (result != 0) and V = (src == 0x8000_0000).
/// These differ from SUB carry rules and must be computed directly.
pub fn neg32(src: u32) -> AluResult68K {
    // 0 - src = NOT(src) + 1
    let src_inv = not_32bit(src);
    let (result, _) = add_32bit_full(0, src_inv, 1);
    let src_bits = int_to_bits32(src);
    let res_bits = int_to_bits32(result);
    // C = result != 0 (OR of all result bits)
    let flag_c = compute_c_neg(&res_bits);
    // V = (src == 0x80000000)
    let flag_v = compute_v_neg(&src_bits, 32);
    let flag_n = compute_n32(result);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result,
        flag_c,
        flag_v,
        flag_n,
        flag_z,
        flag_x: flag_c,
    }
}

/// 32-bit NEGX: `result = 0 - src - x`.
///
/// NEGX Z-flag rule: Z is only *cleared* if result != 0 (never set by NEGX
/// alone).  The caller must AND the new Z with the old Z.
/// Returns Z = (result == 0) so caller can AND with previous Z.
pub fn negx32(src: u32, x_flag: u8) -> AluResult68K {
    // 0 - src - x = NOT(src) + NOT(x_flag)... wait:
    // 0 - src - x = NOT(src) + 1 - x
    // If x=0: NOT(src) + 1 (same as NEG)
    // If x=1: NOT(src) + 0
    let src_inv = not_32bit(src);
    let cin = not_gate(x_flag); // 1 when x=0, 0 when x=1
    let (result, carries) = add_32bit_full(0, src_inv, cin);
    let res_bits = int_to_bits32(result);
    // NEGX carry: borrow occurred = NOT(carry_out of MSB adder stage).
    let flag_c = not_gate(carries[31]);
    let flag_v = {
        // V = MSB result was obtained from negating MSB-only src (like NEG)
        // But for NEGX, overflow is similar: V = 1 if src+x == MSB-only
        // Approximation matching Python: V = bool(result == 0x80000000)
        // Gate-level: AND(result[31]=1, NOT(result[30..0]=all 0))
        let res_bits32 = int_to_bits32(result);
        compute_v_neg(&res_bits32, 32)
    };
    let flag_n = compute_n32(result);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result,
        flag_c,
        flag_v,
        flag_n,
        flag_z,
        flag_x: flag_c,
    }
}

// ── 32-bit logic ──────────────────────────────────────────────────────────────

/// 32-bit AND: `result = a & b`.  V=0, C=0; X unchanged.
pub fn and32(a: u32, b: u32) -> AluResult68K {
    let a_bits = int_to_bits32(a);
    let b_bits = int_to_bits32(b);
    let res_bits: Vec<u8> = (0..32).map(|i| and_gate(a_bits[i], b_bits[i])).collect();
    let result = crate::bits::bits_to_u32(&res_bits);
    let flag_n = compute_n32(result);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result,
        flag_c: 0,
        flag_v: 0,
        flag_n,
        flag_z,
        flag_x: 0,
    }
}

/// 32-bit OR: `result = a | b`.  V=0, C=0; X unchanged.
pub fn or32(a: u32, b: u32) -> AluResult68K {
    let a_bits = int_to_bits32(a);
    let b_bits = int_to_bits32(b);
    let res_bits: Vec<u8> = (0..32).map(|i| or_gate(a_bits[i], b_bits[i])).collect();
    let result = crate::bits::bits_to_u32(&res_bits);
    let flag_n = compute_n32(result);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result,
        flag_c: 0,
        flag_v: 0,
        flag_n,
        flag_z,
        flag_x: 0,
    }
}

/// 32-bit XOR: `result = a ^ b`.  V=0, C=0; X unchanged.
pub fn xor32(a: u32, b: u32) -> AluResult68K {
    let a_bits = int_to_bits32(a);
    let b_bits = int_to_bits32(b);
    let res_bits: Vec<u8> = (0..32).map(|i| xor_gate(a_bits[i], b_bits[i])).collect();
    let result = crate::bits::bits_to_u32(&res_bits);
    let flag_n = compute_n32(result);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result,
        flag_c: 0,
        flag_v: 0,
        flag_n,
        flag_z,
        flag_x: 0,
    }
}

/// 32-bit NOT: bitwise complement via NOT gates.  Returns plain `u32`.
/// NOT sets N/Z like logic ops; V=0, C=0.
pub fn not32_flags(val: u32) -> AluResult68K {
    let bits = int_to_bits32(val);
    let res_bits: Vec<u8> = bits.iter().map(|&b| not_gate(b)).collect();
    let result = crate::bits::bits_to_u32(&res_bits);
    let flag_n = compute_n32(result);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result,
        flag_c: 0,
        flag_v: 0,
        flag_n,
        flag_z,
        flag_x: 0,
    }
}

// ── 16-bit operations ─────────────────────────────────────────────────────────

/// 16-bit ADD. Returns result as u32 (caller masks to u16).
pub fn add16(a: u16, b: u16, carry_in: u8) -> AluResult68K {
    let (result, carries) = add_16bit_full(a, b, carry_in);
    let flag_c = carries[15];
    let flag_v = compute_v_from_carries(&carries);
    let flag_n = compute_n16(result);
    let flag_z = compute_z16(result);
    AluResult68K {
        result: result as u32,
        flag_c,
        flag_v,
        flag_n,
        flag_z,
        flag_x: flag_c,
    }
}

/// 16-bit SUB: `a - b`.
pub fn sub16(a: u16, b: u16, borrow_in: u8) -> AluResult68K {
    let b_inv = not_16bit(b);
    let cin = not_gate(borrow_in);
    let (result, carries) = add_16bit_full(a, b_inv, cin);
    let flag_c = not_gate(carries[15]);
    let flag_v = compute_v_from_carries(&carries);
    let flag_n = compute_n16(result);
    let flag_z = compute_z16(result);
    AluResult68K {
        result: result as u32,
        flag_c,
        flag_v,
        flag_n,
        flag_z,
        flag_x: flag_c,
    }
}

/// 16-bit NEG.
pub fn neg16(src: u16) -> AluResult68K {
    let src_inv = not_16bit(src);
    let (result, _) = add_16bit_full(0, src_inv, 1);
    let src_bits = int_to_bits16(src);
    let res_bits = int_to_bits16(result);
    let flag_c = compute_c_neg(&res_bits);
    let flag_v = compute_v_neg(&src_bits, 16);
    let flag_n = compute_n16(result);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result: result as u32,
        flag_c,
        flag_v,
        flag_n,
        flag_z,
        flag_x: flag_c,
    }
}

/// 16-bit NEGX.
pub fn negx16(src: u16, x_flag: u8) -> AluResult68K {
    let src_inv = not_16bit(src);
    let cin = not_gate(x_flag);
    let (result, carries) = add_16bit_full(0, src_inv, cin);
    let res_bits = int_to_bits16(result);
    let flag_c = not_gate(carries[15]);
    let flag_v = compute_v_neg(&int_to_bits16(result), 16);
    let flag_n = compute_n16(result);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result: result as u32,
        flag_c,
        flag_v,
        flag_n,
        flag_z,
        flag_x: flag_c,
    }
}

/// 16-bit AND.
pub fn and16(a: u16, b: u16) -> AluResult68K {
    let a_bits = int_to_bits16(a);
    let b_bits = int_to_bits16(b);
    let res_bits: Vec<u8> = (0..16).map(|i| and_gate(a_bits[i], b_bits[i])).collect();
    let result = crate::bits::bits_to_u16(&res_bits) as u32;
    let flag_n = compute_n16(result as u16);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result,
        flag_c: 0,
        flag_v: 0,
        flag_n,
        flag_z,
        flag_x: 0,
    }
}

/// 16-bit OR.
pub fn or16(a: u16, b: u16) -> AluResult68K {
    let a_bits = int_to_bits16(a);
    let b_bits = int_to_bits16(b);
    let res_bits: Vec<u8> = (0..16).map(|i| or_gate(a_bits[i], b_bits[i])).collect();
    let result = crate::bits::bits_to_u16(&res_bits) as u32;
    let flag_n = compute_n16(result as u16);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result,
        flag_c: 0,
        flag_v: 0,
        flag_n,
        flag_z,
        flag_x: 0,
    }
}

/// 16-bit XOR.
pub fn xor16(a: u16, b: u16) -> AluResult68K {
    let a_bits = int_to_bits16(a);
    let b_bits = int_to_bits16(b);
    let res_bits: Vec<u8> = (0..16).map(|i| xor_gate(a_bits[i], b_bits[i])).collect();
    let result = crate::bits::bits_to_u16(&res_bits) as u32;
    let flag_n = compute_n16(result as u16);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result,
        flag_c: 0,
        flag_v: 0,
        flag_n,
        flag_z,
        flag_x: 0,
    }
}

/// 16-bit NOT (with flags).
pub fn not16_flags(val: u16) -> AluResult68K {
    let bits = int_to_bits16(val);
    let res_bits: Vec<u8> = bits.iter().map(|&b| not_gate(b)).collect();
    let result = crate::bits::bits_to_u16(&res_bits) as u32;
    let flag_n = compute_n16(result as u16);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result,
        flag_c: 0,
        flag_v: 0,
        flag_n,
        flag_z,
        flag_x: 0,
    }
}

// ── 8-bit operations ──────────────────────────────────────────────────────────

/// 8-bit ADD.
pub fn add8(a: u8, b: u8, carry_in: u8) -> AluResult68K {
    let (result, carries) = add_8bit_full(a, b, carry_in);
    let flag_c = carries[7];
    let flag_v = compute_v_from_carries(&carries);
    let flag_n = compute_n8(result);
    let flag_z = compute_z8(result);
    AluResult68K {
        result: result as u32,
        flag_c,
        flag_v,
        flag_n,
        flag_z,
        flag_x: flag_c,
    }
}

/// 8-bit SUB: `a - b`.
pub fn sub8(a: u8, b: u8, borrow_in: u8) -> AluResult68K {
    let b_inv = not_8bit(b);
    let cin = not_gate(borrow_in);
    let (result, carries) = add_8bit_full(a, b_inv, cin);
    let flag_c = not_gate(carries[7]);
    let flag_v = compute_v_from_carries(&carries);
    let flag_n = compute_n8(result);
    let flag_z = compute_z8(result);
    AluResult68K {
        result: result as u32,
        flag_c,
        flag_v,
        flag_n,
        flag_z,
        flag_x: flag_c,
    }
}

/// 8-bit NEG.
pub fn neg8(src: u8) -> AluResult68K {
    let src_inv = not_8bit(src);
    let (result, _) = add_8bit_full(0, src_inv, 1);
    let src_bits = int_to_bits8(src);
    let res_bits = int_to_bits8(result);
    let flag_c = compute_c_neg(&res_bits);
    let flag_v = compute_v_neg(&src_bits, 8);
    let flag_n = compute_n8(result);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result: result as u32,
        flag_c,
        flag_v,
        flag_n,
        flag_z,
        flag_x: flag_c,
    }
}

/// 8-bit NEGX.
pub fn negx8(src: u8, x_flag: u8) -> AluResult68K {
    let src_inv = not_8bit(src);
    let cin = not_gate(x_flag);
    let (result, carries) = add_8bit_full(0, src_inv, cin);
    let res_bits = int_to_bits8(result);
    let flag_c = not_gate(carries[7]);
    let flag_v = compute_v_neg(&int_to_bits8(result), 8);
    let flag_n = compute_n8(result);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result: result as u32,
        flag_c,
        flag_v,
        flag_n,
        flag_z,
        flag_x: flag_c,
    }
}

/// 8-bit AND.
pub fn and8(a: u8, b: u8) -> AluResult68K {
    let a_bits = int_to_bits8(a);
    let b_bits = int_to_bits8(b);
    let res_bits: Vec<u8> = (0..8).map(|i| and_gate(a_bits[i], b_bits[i])).collect();
    let result = crate::bits::bits_to_u8(&res_bits);
    let flag_n = compute_n8(result);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result: result as u32,
        flag_c: 0,
        flag_v: 0,
        flag_n,
        flag_z,
        flag_x: 0,
    }
}

/// 8-bit OR.
pub fn or8(a: u8, b: u8) -> AluResult68K {
    let a_bits = int_to_bits8(a);
    let b_bits = int_to_bits8(b);
    let res_bits: Vec<u8> = (0..8).map(|i| or_gate(a_bits[i], b_bits[i])).collect();
    let result = crate::bits::bits_to_u8(&res_bits);
    let flag_n = compute_n8(result);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result: result as u32,
        flag_c: 0,
        flag_v: 0,
        flag_n,
        flag_z,
        flag_x: 0,
    }
}

/// 8-bit XOR.
pub fn xor8(a: u8, b: u8) -> AluResult68K {
    let a_bits = int_to_bits8(a);
    let b_bits = int_to_bits8(b);
    let res_bits: Vec<u8> = (0..8).map(|i| xor_gate(a_bits[i], b_bits[i])).collect();
    let result = crate::bits::bits_to_u8(&res_bits);
    let flag_n = compute_n8(result);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result: result as u32,
        flag_c: 0,
        flag_v: 0,
        flag_n,
        flag_z,
        flag_x: 0,
    }
}

/// 8-bit NOT (with flags).
pub fn not8_flags(val: u8) -> AluResult68K {
    let bits = int_to_bits8(val);
    let res_bits: Vec<u8> = bits.iter().map(|&b| not_gate(b)).collect();
    let result = crate::bits::bits_to_u8(&res_bits);
    let flag_n = compute_n8(result);
    let flag_z = compute_z(&res_bits);
    AluResult68K {
        result: result as u32,
        flag_c: 0,
        flag_v: 0,
        flag_n,
        flag_z,
        flag_x: 0,
    }
}

// ── Shift / Rotate ────────────────────────────────────────────────────────────
//
// The 68000 shift/rotate family (Line E):
//
//   Type  │ Code │ Left              │ Right
//   ───────┼──────┼───────────────────┼────────────────────
//   AS     │  00  │ ASL (arith left)  │ ASR (arith right)
//   LS     │  01  │ LSL (logic left)  │ LSR (logic right)
//   ROX    │   10  │ ROXL (thru X)     │ ROXR (thru X)
//   RO     │  11  │ ROL (circular)    │ ROR (circular)
//
// For ASL/LSL: C = last bit shifted out (MSB); X = C.
// For ASR/LSR: C = last bit shifted out (LSB); X = C.
// For ROXL/ROXR: C = last bit shifted out; X = C.
// For ROL/ROR: C = last bit rotated into C; X unchanged.
//
// Gate-level note: each shift step routes the bit through an appropriate
// multiplexer (AND/OR tree).  The loop below steps one bit at a time,
// tracking the last bit out.
//
// MUL/DIV use fixed partial-product and restoring gate networks below.

fn width_mask(width: usize) -> u64 {
    if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

fn gate_add_width(a: u64, b: u64, carry_in: u8, width: usize) -> (u64, u8) {
    let mut result = 0u64;
    let mut carry = carry_in;
    for bit in 0..width {
        let (sum, next) =
            arithmetic::adders::full_adder(((a >> bit) & 1) as u8, ((b >> bit) & 1) as u8, carry);
        result |= u64::from(sum) << bit;
        carry = next;
    }
    (result & width_mask(width), carry)
}

fn gate_negate_width(value: u64, width: usize) -> u64 {
    let mut inverted = 0u64;
    for bit in 0..width {
        inverted |= u64::from(not_gate(((value >> bit) & 1) as u8)) << bit;
    }
    gate_add_width(inverted, 0, 1, width).0
}

fn gate_unsigned_multiply(left: u64, right: u64, width: usize) -> u64 {
    let product_width = width * 2;
    let mut product = 0u64;
    for right_bit in 0..width {
        let selector = ((right >> right_bit) & 1) as u8;
        let mut partial = 0u64;
        for left_bit in 0..width {
            let bit = and_gate(((left >> left_bit) & 1) as u8, selector);
            partial |= u64::from(bit) << (left_bit + right_bit);
        }
        product = gate_add_width(product, partial, 0, product_width).0;
    }
    product
}

fn gate_unsigned_divide(dividend: u64, divisor: u64, width: usize) -> Option<(u64, u64)> {
    if divisor == 0 {
        return None;
    }
    let remainder_width = width + 1;
    let mut quotient = 0u64;
    let mut remainder = 0u64;
    for bit in (0..width).rev() {
        remainder = ((remainder << 1) | ((dividend >> bit) & 1)) & width_mask(remainder_width);
        let inverted = gate_negate_width(divisor, remainder_width);
        let (difference, no_borrow) = gate_add_width(remainder, inverted, 0, remainder_width);
        if no_borrow == 1 {
            remainder = difference;
            quotient |= 1u64 << bit;
        }
    }
    Some((quotient, remainder))
}

/// Fixed 16×16 unsigned partial-product multiplier.
pub fn gate_mulu16(left: u16, right: u16) -> u32 {
    gate_unsigned_multiply(u64::from(left), u64::from(right), 16) as u32
}

/// Fixed 16×16 signed partial-product multiplier.
pub fn gate_muls16(left: u16, right: u16) -> u32 {
    let left_sign = ((left >> 15) & 1) as u8;
    let right_sign = ((right >> 15) & 1) as u8;
    let left_magnitude = if left_sign == 1 {
        gate_negate_width(u64::from(left), 16)
    } else {
        u64::from(left)
    };
    let right_magnitude = if right_sign == 1 {
        gate_negate_width(u64::from(right), 16)
    } else {
        u64::from(right)
    };
    let product = gate_unsigned_multiply(left_magnitude, right_magnitude, 16);
    if xor_gate(left_sign, right_sign) == 1 {
        gate_negate_width(product, 32) as u32
    } else {
        product as u32
    }
}

/// Fixed restoring unsigned 32÷16 divider returning full quotient and remainder.
pub fn gate_divu32_16(dividend: u32, divisor: u16) -> Option<(u32, u16)> {
    gate_unsigned_divide(u64::from(dividend), u64::from(divisor), 32)
        .map(|(quotient, remainder)| (quotient as u32, remainder as u16))
}

/// Fixed restoring signed 32÷16 divider.
///
/// Returns the low 16-bit quotient, signed remainder, and the architectural
/// quotient-overflow flag. Division by zero returns `None`.
pub fn gate_divs32_16(dividend: u32, divisor: u16) -> Option<(u16, u16, bool)> {
    if divisor == 0 {
        return None;
    }
    let dividend_sign = ((dividend >> 31) & 1) as u8;
    let divisor_sign = ((divisor >> 15) & 1) as u8;
    let dividend_magnitude = if dividend_sign == 1 {
        gate_negate_width(u64::from(dividend), 32)
    } else {
        u64::from(dividend)
    };
    let divisor_magnitude = if divisor_sign == 1 {
        gate_negate_width(u64::from(divisor), 16)
    } else {
        u64::from(divisor)
    };
    let (quotient_magnitude, remainder_magnitude) =
        gate_unsigned_divide(dividend_magnitude, divisor_magnitude, 32)?;
    let negative_quotient = xor_gate(dividend_sign, divisor_sign) == 1;
    let limit = if negative_quotient { 32_768 } else { 32_767 };
    let overflow = quotient_magnitude > limit;
    let quotient = if negative_quotient {
        gate_negate_width(quotient_magnitude, 16)
    } else {
        quotient_magnitude
    } as u16;
    let remainder = if dividend_sign == 1 {
        gate_negate_width(remainder_magnitude, 16)
    } else {
        remainder_magnitude
    } as u16;
    Some((quotient, remainder, overflow))
}

/// Shift/rotate result: `(new_val, flag_n, flag_z, flag_v, flag_c, flag_x)`.
pub struct ShiftResult {
    pub result: u32,
    pub flag_n: u8,
    pub flag_z: u8,
    pub flag_v: u8,
    pub flag_c: u8,
    pub flag_x: u8,
}

/// Perform a shift or rotate on a value of `bits` width.
///
/// - `val`: the unsigned value (already masked to `bits` width)
/// - `count`: how many positions to shift (0–63)
/// - `left`: true = shift/rotate left, false = shift/rotate right
/// - `shift_type`: 0=AS, 1=LS, 2=ROX, 3=RO
/// - `bits`: operand width (8, 16, or 32)
/// - `x_in`: current X flag (for ROX variants)
///
/// Returns a `ShiftResult`.  For ROL/ROR, X is unchanged (caller must
/// preserve old X when writing SR).
///
/// ASL overflow: V=1 if the MSB changed at any point during the shift
/// (i.e., if any bits were "squeezed" out of the sign position).
pub fn shift_op(
    val: u32,
    count: u32,
    left: bool,
    shift_type: u8,
    bits: u32,
    x_in: u8,
) -> ShiftResult {
    let mask: u32 = if bits == 32 {
        0xFFFF_FFFF
    } else {
        (1u32 << bits) - 1
    };
    let msb_mask: u32 = 1u32 << (bits - 1);
    let count = count & mask; // safety clamp (count already validated by caller)

    let mut result = val & mask;
    let mut last_out: u8 = 0;
    let mut v_flag: u8 = 0;

    match shift_type {
        0 => {
            // ── Arithmetic shift ───────────────────────────────────────────
            if left {
                // ASL: shift bits left; MSB pops out each step.
                let orig_msb = ((result & msb_mask) != 0) as u8;
                for _ in 0..count {
                    last_out = ((result & msb_mask) != 0) as u8;
                    result = (result << 1) & mask;
                    // V=1 if sign bit ever changed
                    let new_msb = ((result & msb_mask) != 0) as u8;
                    v_flag = or_gate(v_flag, xor_gate(orig_msb, new_msb));
                }
            } else {
                // ASR: replicate sign bit each step.
                let sign_bit = result & msb_mask;
                for _ in 0..count {
                    last_out = (result & 1) as u8;
                    result = ((result >> 1) | sign_bit) & mask;
                }
            }
        }
        1 => {
            // ── Logical shift ──────────────────────────────────────────────
            if left {
                for _ in 0..count {
                    last_out = ((result & msb_mask) != 0) as u8;
                    result = (result << 1) & mask;
                }
            } else {
                for _ in 0..count {
                    last_out = (result & 1) as u8;
                    result = (result >> 1) & mask;
                }
            }
        }
        2 => {
            // ── Rotate through X (ROX) ─────────────────────────────────────
            let mut x = x_in;
            if left {
                for _ in 0..count {
                    last_out = ((result & msb_mask) != 0) as u8;
                    result = ((result << 1) | (x as u32)) & mask;
                    x = last_out;
                }
            } else {
                for _ in 0..count {
                    last_out = (result & 1) as u8;
                    result = ((result >> 1) | ((x as u32) << (bits - 1))) & mask;
                    x = last_out;
                }
            }
        }
        _ => {
            // ── Circular rotate (RO) ──────────────────────────────────────
            if count == 0 {
                // No rotation: C = LSB for ROR, MSB for ROL... actually C cleared.
                // ROL/ROR with count=0: C=0.
                last_out = 0;
            } else if left {
                let count_mod = count % bits;
                // count_mod == 0 means a full-width rotation → identity; avoid shift-by-width panic.
                if count_mod != 0 {
                    result = ((result << count_mod) | (result >> (bits - count_mod))) & mask;
                }
                // C = last bit rotated into bit 0 (the new LSB)
                last_out = (result & 1) as u8;
            } else {
                let count_mod = count % bits;
                if count_mod != 0 {
                    result = ((result >> count_mod) | (result << (bits - count_mod))) & mask;
                }
                // C = last bit rotated into the MSB (the new MSB)
                last_out = ((result & msb_mask) != 0) as u8;
            }
        }
    }

    result &= mask;
    let flag_n = ((result & msb_mask) != 0) as u8;
    let flag_z = compute_z(&crate::bits::int_to_bits32(result)[..bits as usize]);
    let flag_c = last_out;
    // X unchanged for circular rotate (ROL/ROR); X = C for all others.
    let flag_x = if shift_type == 3 { x_in } else { flag_c };
    // For ROL/ROR with count=0: C is cleared.
    let flag_c = if shift_type == 3 && count == 0 {
        0
    } else {
        flag_c
    };

    ShiftResult {
        result,
        flag_n,
        flag_z,
        flag_v: v_flag,
        flag_c,
        flag_x,
    }
}

// ── CMP helper ────────────────────────────────────────────────────────────────

/// Compare `a - b` for CMP/CMPA/CMPI/CMPM: sets N/Z/V/C but NOT X.
/// Returns `AluResult68K` with `flag_x = 0` (caller preserves old X).
pub fn cmp32(a: u32, b: u32) -> AluResult68K {
    let r = sub32(a, b, 0);
    AluResult68K { flag_x: 0, ..r }
}

/// 16-bit CMP.
pub fn cmp16(a: u16, b: u16) -> AluResult68K {
    let r = sub16(a, b, 0);
    AluResult68K { flag_x: 0, ..r }
}

/// 8-bit CMP.
pub fn cmp8(a: u8, b: u8) -> AluResult68K {
    let r = sub8(a, b, 0);
    AluResult68K { flag_x: 0, ..r }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add32_basic() {
        let r = add32(5, 3, 0);
        assert_eq!(r.result, 8);
        assert_eq!(r.flag_c, 0);
        assert_eq!(r.flag_v, 0);
        assert_eq!(r.flag_n, 0);
        assert_eq!(r.flag_z, 0);
    }

    #[test]
    fn add32_overflow() {
        // 0x7FFF_FFFF + 1 = 0x8000_0000 → signed overflow
        let r = add32(0x7FFF_FFFF, 1, 0);
        assert_eq!(r.result, 0x8000_0000);
        assert_eq!(r.flag_v, 1);
        assert_eq!(r.flag_n, 1);
        assert_eq!(r.flag_c, 0);
    }

    #[test]
    fn add32_carry() {
        // 0xFFFF_FFFF + 1 → wrap to 0 with carry
        let r = add32(0xFFFF_FFFF, 1, 0);
        assert_eq!(r.result, 0);
        assert_eq!(r.flag_c, 1);
        assert_eq!(r.flag_z, 1);
        assert_eq!(r.flag_v, 0);
    }

    #[test]
    fn sub32_basic() {
        let r = sub32(10, 3, 0);
        assert_eq!(r.result, 7);
        assert_eq!(r.flag_c, 0);
        assert_eq!(r.flag_v, 0);
    }

    #[test]
    fn sub32_borrow() {
        let r = sub32(3, 10, 0);
        assert_eq!(r.result, 3u32.wrapping_sub(10));
        assert_eq!(r.flag_c, 1); // borrow
    }

    #[test]
    fn sub32_overflow() {
        // 0x8000_0000 - 1 = 0x7FFF_FFFF → signed overflow
        let r = sub32(0x8000_0000, 1, 0);
        assert_eq!(r.result, 0x7FFF_FFFF);
        assert_eq!(r.flag_v, 1);
        assert_eq!(r.flag_c, 0);
    }

    #[test]
    fn neg32_zero() {
        let r = neg32(0);
        assert_eq!(r.result, 0);
        assert_eq!(r.flag_c, 0); // NEG 0 → no carry
        assert_eq!(r.flag_z, 1);
    }

    #[test]
    fn neg32_one() {
        let r = neg32(1);
        assert_eq!(r.result, 0xFFFF_FFFF);
        assert_eq!(r.flag_c, 1); // result != 0 → carry
        assert_eq!(r.flag_v, 0);
    }

    #[test]
    fn neg32_most_negative() {
        // NEG 0x8000_0000 = 0x8000_0000 (wrap) → overflow
        let r = neg32(0x8000_0000);
        assert_eq!(r.result, 0x8000_0000);
        assert_eq!(r.flag_v, 1);
    }

    #[test]
    fn and32_basic() {
        let r = and32(0xFF00_FF00, 0x0F0F_0F0F);
        assert_eq!(r.result, 0x0F00_0F00);
        assert_eq!(r.flag_c, 0);
        assert_eq!(r.flag_v, 0);
    }

    #[test]
    fn or32_basic() {
        let r = or32(0xFF00_0000, 0x00FF_0000);
        assert_eq!(r.result, 0xFFFF_0000);
        assert_eq!(r.flag_n, 1);
    }

    #[test]
    fn xor32_basic() {
        let r = xor32(0xFFFF_0000, 0xFFFF_0000);
        assert_eq!(r.result, 0);
        assert_eq!(r.flag_z, 1);
    }

    #[test]
    fn not32_basic() {
        let r = not32_flags(0x0000_FFFF);
        assert_eq!(r.result, 0xFFFF_0000);
        assert_eq!(r.flag_n, 1);
    }

    #[test]
    fn add8_basic() {
        let r = add8(10, 20, 0);
        assert_eq!(r.result, 30);
        assert_eq!(r.flag_c, 0);
    }

    #[test]
    fn add8_overflow() {
        let r = add8(0x7F, 1, 0);
        assert_eq!(r.result, 0x80);
        assert_eq!(r.flag_v, 1);
        assert_eq!(r.flag_n, 1);
    }

    #[test]
    fn sub8_borrow() {
        let r = sub8(3, 10, 0);
        assert_eq!(r.result as u8, 3u8.wrapping_sub(10));
        assert_eq!(r.flag_c, 1);
    }

    #[test]
    fn add16_carry() {
        let r = add16(0xFFFF, 1, 0);
        assert_eq!(r.result, 0);
        assert_eq!(r.flag_c, 1);
        assert_eq!(r.flag_z, 1);
    }

    #[test]
    fn shift_asl_word() {
        // ASL.W D0 by 1: 0x0001 << 1 = 0x0002
        let r = shift_op(1, 1, true, 0, 16, 0);
        assert_eq!(r.result, 2);
        assert_eq!(r.flag_c, 0);
        assert_eq!(r.flag_v, 0);
    }

    #[test]
    fn shift_lsr_byte() {
        // LSR.B by 1: 0x02 → 0x01, C=0
        let r = shift_op(0x02, 1, false, 1, 8, 0);
        assert_eq!(r.result, 1);
        assert_eq!(r.flag_c, 0);
    }

    #[test]
    fn shift_ror_long() {
        // ROR.L by 1: 0x0000_0001 → 0x8000_0000
        let r = shift_op(1, 1, false, 3, 32, 0);
        assert_eq!(r.result, 0x8000_0000);
        assert_eq!(r.flag_c, 1); // last bit rotated into MSB = new MSB = 1
    }

    #[test]
    fn cmp32_equal() {
        let r = cmp32(5, 5);
        assert_eq!(r.flag_z, 1);
        assert_eq!(r.flag_c, 0);
    }

    #[test]
    fn cmp32_less_than() {
        let r = cmp32(3, 5);
        assert_eq!(r.flag_c, 1); // borrow
        assert_eq!(r.flag_z, 0);
    }

    #[test]
    fn negx32_no_borrow() {
        let r = negx32(1, 0);
        assert_eq!(r.result, 0xFFFF_FFFF);
        assert_eq!(r.flag_c, 1); // result != 0 → borrow
    }

    #[test]
    fn fixed_gate_multiply_and_divide_seeded_vectors() {
        let mut seed = 0x68_0000u32;
        for _ in 0..512 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let left = seed as u16;
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let right = seed as u16;
            assert_eq!(gate_mulu16(left, right), u32::from(left) * u32::from(right));
            assert_eq!(
                gate_muls16(left, right),
                ((left as i16 as i32) * (right as i16 as i32)) as u32
            );

            let divisor = right | 1;
            let (quotient, remainder) = gate_divu32_16(seed, divisor).unwrap();
            assert_eq!(quotient, seed / u32::from(divisor));
            assert_eq!(remainder, (seed % u32::from(divisor)) as u16);
        }
    }

    #[test]
    fn fixed_signed_divider_handles_signs_and_overflow() {
        assert_eq!(
            gate_divs32_16((-100i32) as u32, (-7i16) as u16),
            Some((14, (-2i16) as u16, false))
        );
        assert_eq!(
            gate_divs32_16(i32::MIN as u32, (-1i16) as u16),
            Some((0, 0, true))
        );
        assert_eq!(gate_divs32_16(1, 0), None);
    }
}
