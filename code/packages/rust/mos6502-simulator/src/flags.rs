//! Flag computation helpers for the MOS 6502 processor.
//!
//! Direct Rust transcription of `code/packages/python/mos6502-simulator/
//! src/mos6502_simulator/flags.py`.
//!
//! The 6502 has 7 active flag bits in the processor status register (P):
//!
//! ```text
//! Bit 7  N  Negative   — bit 7 of result
//! Bit 6  V  Overflow   — signed overflow
//! Bit 5  -  (always 1)
//! Bit 4  B  Break      — set only in stack copy during BRK/PHP
//! Bit 3  D  Decimal    — BCD mode
//! Bit 2  I  IRQ disable
//! Bit 1  Z  Zero       — result == 0
//! Bit 0  C  Carry      — carry out / not-borrow
//! ```
//!
//! Unlike the Intel 8080, the 6502 does **not** have an auxiliary-carry or
//! parity flag.  The overflow flag `V` uses a different formula:
//! `V = (A7 ^ result7) & (M7 ^ result7)` (7 = bit 7) — this detects signed
//! overflow in a single expression.

/// Return `(N, Z)` flags for an 8-bit result.
///
/// `N` is set when bit 7 is 1 (the result would be negative in two's
/// complement).  `Z` is set when the result is zero.
pub fn compute_nz(result: u8) -> (bool, bool) {
    (result & 0x80 != 0, result == 0)
}

/// Compute the `V` (overflow) flag for addition (`ADC`).
///
/// Signed overflow occurs when two same-sign operands produce a
/// different-sign result:
///
/// ```text
/// +  +  +  -> can't overflow  (two positives can't give negative)
/// -  +  -  -> can't overflow
/// +  +  -  -> overflow        (e.g. 127 + 1 = 128 = -128)
/// -  +  +  -> overflow        (e.g. -128 + (-1) = -129 = 127)
/// ```
///
/// Single-expression form: "inputs had the same sign AND result has a
/// different sign" — `V = NOT(A7 XOR B7) AND (A7 XOR result7)`.
pub fn compute_overflow_add(a: u8, b: u8, result: u8) -> bool {
    let a7 = (a >> 7) & 1;
    let b7 = (b >> 7) & 1;
    let r7 = (result >> 7) & 1;
    (!(a7 ^ b7) & (a7 ^ r7) & 1) != 0
}

/// Compute the `V` flag for subtraction (`SBC`).
///
/// `SBC` internally computes `A + ~B + C`, so the overflow check is the
/// same as for `ADC` but with the operand inverted.
pub fn compute_overflow_sub(a: u8, b: u8, result: u8) -> bool {
    compute_overflow_add(a, !b, result)
}

/// Pack 7 flag booleans into the P status register byte.  Bit 5 (unused)
/// is always 1 on the 6502.
///
/// ```text
/// 7 6 5 4 3 2 1 0
/// N V 1 B D I Z C
/// ```
#[allow(clippy::too_many_arguments)]
pub fn pack_p(n: bool, v: bool, b: bool, d: bool, i: bool, z: bool, c: bool) -> u8 {
    (u8::from(n) << 7)
        | (u8::from(v) << 6)
        | 0x20 // bit 5 always 1
        | (u8::from(b) << 4)
        | (u8::from(d) << 3)
        | (u8::from(i) << 2)
        | (u8::from(z) << 1)
        | u8::from(c)
}

/// Unpack a P byte into `(N, V, B, D, I, Z, C)` flag booleans.
pub fn unpack_p(p: u8) -> (bool, bool, bool, bool, bool, bool, bool) {
    (
        p & 0x80 != 0,
        p & 0x40 != 0,
        p & 0x10 != 0,
        p & 0x08 != 0,
        p & 0x04 != 0,
        p & 0x02 != 0,
        p & 0x01 != 0,
    )
}

/// BCD (decimal mode) addition: `a + b + carry_in`.
///
/// The NMOS 6502 performs BCD correction *after* the binary add, which
/// means that in decimal mode the N/V/Z flags still reflect the *binary*
/// result (not the BCD-corrected result) — only `C` is computed correctly
/// from the BCD result.  Callers must compute N/V/Z from the binary sum
/// separately (see `execute.rs`'s `ADC`/`SBC` handlers); this function
/// returns only `(bcd_result, carry_out)`.
///
/// Algorithm (NMOS 6502 behaviour):
/// 1. Add the low nibbles.  If > 9, add 6 (carries into the high nibble).
/// 2. Add the high nibbles + the carry from step 1.  If > 9, add 6.
/// 3. Final carry = 1 if the high nibble carried out.
pub fn bcd_add(a: u8, b: u8, carry_in: bool) -> (u8, bool) {
    let mut low = (a & 0x0F) + (b & 0x0F) + u8::from(carry_in);
    let carry_low = low > 9;
    if carry_low {
        low = (low + 6) & 0x0F;
    }

    let mut high = (a >> 4) + (b >> 4) + u8::from(carry_low);
    let carry_out = high > 9;
    if carry_out {
        high = (high + 6) & 0x0F;
    }

    ((high << 4) | low, carry_out)
}

/// BCD subtraction: `a - b - (1 - carry_in)`.
///
/// The 6502 implements `SBC` as `A + ~B + C`; in decimal mode this still
/// uses BCD correction on the subtraction path.  Returns
/// `(bcd_result, carry_out)` where `carry_out = true` means "no borrow".
pub fn bcd_sub(a: u8, b: u8, carry_in: bool) -> (u8, bool) {
    let mut low = i16::from(a & 0x0F) - i16::from(b & 0x0F) - i16::from(!carry_in);
    let borrow_low = low < 0;
    if borrow_low {
        low = (low - 6) & 0x0F;
    }

    let mut high = i16::from(a >> 4) - i16::from(b >> 4) - i16::from(borrow_low);
    let borrow_out = high < 0;
    if borrow_out {
        high = (high - 6) & 0x0F;
    }

    let carry_out = !borrow_out;
    ((((high << 4) | low) & 0xFF) as u8, carry_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nz_zero() {
        assert_eq!(compute_nz(0x00), (false, true));
    }

    #[test]
    fn nz_negative() {
        assert_eq!(compute_nz(0xFF), (true, false));
        assert_eq!(compute_nz(0x80), (true, false));
    }

    #[test]
    fn nz_positive() {
        assert_eq!(compute_nz(0x42), (false, false));
    }

    #[test]
    fn overflow_add_positive_overflow() {
        // 127 + 1 = 128 (overflow: positive + positive = negative)
        assert!(compute_overflow_add(127, 1, 128));
    }

    #[test]
    fn overflow_add_no_overflow_mixed_signs() {
        // -1 + 1 = 0, no overflow (mixed-sign inputs never overflow)
        assert!(!compute_overflow_add(0xFF, 0x01, 0x00));
    }

    #[test]
    fn overflow_add_negative_overflow() {
        // -128 + -1 = -129 (wraps to 127) -- overflow
        assert!(compute_overflow_add(0x80, 0xFF, 0x7F));
    }

    #[test]
    fn pack_unpack_round_trip() {
        let p = pack_p(true, false, true, false, true, false, true);
        assert_eq!(unpack_p(p), (true, false, true, false, true, false, true));
    }

    #[test]
    fn pack_bit5_always_set() {
        let p = pack_p(false, false, false, false, false, false, false);
        assert_eq!(p, 0x20);
    }

    #[test]
    fn unpack_reset_value() {
        // 0x24 = 0b00100100 = bit5=1, I=1
        assert_eq!(unpack_p(0x24), (false, false, false, false, true, false, false));
    }

    #[test]
    fn bcd_add_basic() {
        // 9 + 1 = 10 in BCD
        assert_eq!(bcd_add(0x09, 0x01, false), (0x10, false));
    }

    #[test]
    fn bcd_add_carries_out() {
        // 99 + 1 = 100 -> carry out, result wraps to 0x00
        assert_eq!(bcd_add(0x99, 0x01, false), (0x00, true));
    }

    #[test]
    fn bcd_sub_basic() {
        // 10 - 1 = 9, no borrow (carry_in = true means "no incoming borrow")
        assert_eq!(bcd_sub(0x10, 0x01, true), (0x09, true));
    }

    #[test]
    fn bcd_sub_borrows() {
        // 0 - 1 = 99 (borrow), carry_out = false
        assert_eq!(bcd_sub(0x00, 0x01, true), (0x99, false));
    }
}
