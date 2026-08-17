//! Condition Code Register (CCR) computation helpers.
//!
//! Direct Rust transcription of `code/packages/python/
//! motorola-68000-simulator/src/motorola_68000_simulator/flags.py` — see
//! that module's docstring for the full N/Z/V/C/X derivation (this module
//! documents the port, not the flag semantics again).
//!
//! # Why `raw` is `i64`, not `u32`
//!
//! The Python original computes `raw = a + b` / `raw = a - b` as an
//! arbitrary-precision Python `int`, which can be negative (for SUB) or
//! exceed the operand width (for ADD), and only masks it down to the
//! operand size *after* deriving the carry flag from the unmasked value.
//! Rust's `u32` wraps silently instead of growing, which would lose
//! exactly the information the carry flag needs.  Every helper here takes
//! `raw: i64` instead — wide enough to hold the exact sum/difference of
//! two `u32`s without overflow, and bitwise-AND with a positive mask on a
//! negative `i64` correctly extracts its two's-complement low bits (Rust
//! integers are two's complement, same as the bit pattern real 68000
//! silicon computes), so `(raw & mask_for(sz) as i64) as u32` reproduces
//! the Python original's `raw & mask` exactly, including for negative
//! `raw`.

use crate::opcodes::{mask_for, msb_for};

/// Carry for ADD/ADDQ/ADDI — unsigned result exceeds the operand's
/// representable range.
pub fn compute_c_add(raw: i64, sz: u8) -> bool {
    raw > i64::from(mask_for(sz))
}

/// Carry (borrow) for SUB/SUBQ/SUBI/CMP/CMPI — `a < b`.  Operands must
/// already be masked to `sz` bytes.
pub fn compute_c_sub(a: u32, b: u32) -> bool {
    a < b
}

/// Overflow for ADD/ADDQ/ADDI: signed overflow when both operands share a
/// sign but the result's sign differs — `V = (~(a^b)) & (a^result)` on the
/// MSB.
pub fn compute_v_add(a: u32, b: u32, result: u32, sz: u8) -> bool {
    let msb = msb_for(sz);
    (!(a ^ b)) & (a ^ result) & msb != 0
}

/// Overflow for SUB/SUBQ/SUBI/CMP/CMPI (`a - b`): signed overflow when
/// operands have different signs and the result's sign differs from `a`.
pub fn compute_v_sub(a: u32, b: u32, result: u32, sz: u8) -> bool {
    let msb = msb_for(sz);
    (a ^ b) & (a ^ result) & msb != 0
}

/// Negative flag — copy of the MSB of the `sz`-byte-masked result.
pub fn compute_n(result: u32, sz: u8) -> bool {
    result & mask_for(sz) & msb_for(sz) != 0
}

/// Zero flag — result is zero after masking to `sz` bytes.
pub fn compute_z(result: u32, sz: u8) -> bool {
    result & mask_for(sz) == 0
}

/// Compute `(N, Z, V, C, X)` for ADD/ADDQ/ADDI.  `X` always mirrors `C`.
pub fn compute_nzvc_add(a: u32, b: u32, raw: i64, sz: u8) -> (bool, bool, bool, bool, bool) {
    let result = (raw & i64::from(mask_for(sz))) as u32;
    let n = compute_n(result, sz);
    let z = compute_z(result, sz);
    let v = compute_v_add(a, b, result, sz);
    let c = compute_c_add(raw, sz);
    (n, z, v, c, c)
}

/// Compute `(N, Z, V, C, X)` for SUB/SUBQ/SUBI (`a - b`).  `X` always
/// mirrors `C`.
pub fn compute_nzvc_sub(a: u32, b: u32, raw: i64, sz: u8) -> (bool, bool, bool, bool, bool) {
    let result = (raw & i64::from(mask_for(sz))) as u32;
    let n = compute_n(result, sz);
    let z = compute_z(result, sz);
    let v = compute_v_sub(a, b, result, sz);
    let c = compute_c_sub(a, b);
    (n, z, v, c, c)
}

/// Compute `(N, Z)` for AND/OR/EOR/NOT/CLR/MOVE/MOVEQ.  `V`/`C` are
/// always cleared by the caller; `X` is left unchanged.
pub fn compute_nz_logic(result: u32, sz: u8) -> (bool, bool) {
    (compute_n(result, sz), compute_z(result, sz))
}

/// Compute `(N, Z, V, C, X)` for NEG (`result = 0 - src`).  NEG's carry
/// and overflow rules differ from plain SUB — see the Python original's
/// `compute_nzvc_neg` docstring for the derivation.
pub fn compute_nzvc_neg(src: u32, result: u32, sz: u8) -> (bool, bool, bool, bool, bool) {
    let mask = mask_for(sz);
    let msb = msb_for(sz);
    let result = result & mask;
    let n = result & msb != 0;
    let z = result == 0;
    let v = src == msb; // overflow negating the most-negative representable value
    let c = result != 0; // carry iff the result is non-zero
    (n, z, v, c, c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_overflow_positive_plus_one() {
        // +127 (byte) + 1 -> -128: signed overflow.
        let (n, z, v, c, x) = compute_nzvc_add(0x7F, 0x01, 0x80, 1);
        assert!(n && !z && v && !c && !x);
    }

    #[test]
    fn add_carry_out_of_byte() {
        let (n, z, v, c, x) = compute_nzvc_add(0xFF, 0x01, 0x100, 1);
        assert!(!n && z && !v && c && x);
    }

    #[test]
    fn sub_no_borrow() {
        let raw = 0x05i64 - 0x03i64;
        let (n, z, v, c, x) = compute_nzvc_sub(0x05, 0x03, raw, 1);
        assert!(!n && !z && !v && !c && !x);
    }

    #[test]
    fn sub_borrow_negative_raw() {
        // 0x00 - 0x01 -> raw = -1, masked to byte = 0xFF (negative).
        let raw = 0x00i64 - 0x01i64;
        let (n, z, v, c, x) = compute_nzvc_sub(0x00, 0x01, raw, 1);
        assert!(n && !z && !v && c && x);
    }

    #[test]
    fn logic_flags_zero_and_negative() {
        assert_eq!(compute_nz_logic(0x00, 1), (false, true));
        assert_eq!(compute_nz_logic(0x80, 1), (true, false));
        assert_eq!(compute_nz_logic(0xFF, 1), (true, false));
    }

    #[test]
    fn neg_zero_no_carry() {
        let (n, z, v, c, x) = compute_nzvc_neg(0x00, 0x00, 1);
        assert!(!n && z && !v && !c && !x);
    }

    #[test]
    fn neg_one_produces_carry() {
        let (n, z, v, c, x) = compute_nzvc_neg(0x01, 0xFF, 1);
        assert!(n && !z && !v && c && x);
    }

    #[test]
    fn neg_most_negative_overflows() {
        let (n, z, v, c, x) = compute_nzvc_neg(0x80, 0x80, 1);
        assert!(n && !z && v && c && x);
    }

    #[test]
    fn long_size_add_overflow() {
        let a = 0x7FFF_FFFFu32;
        let b = 1u32;
        let raw = i64::from(a) + i64::from(b);
        let (n, z, v, c, _x) = compute_nzvc_add(a, b, raw, 4);
        assert!(n && !z && v && !c);
    }
}
