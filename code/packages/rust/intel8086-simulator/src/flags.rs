//! Flag computation helpers for the Intel 8086.
//!
//! Direct Rust transcription of `code/packages/python/
//! intel-8086-simulator/src/intel_8086_simulator/flags.py` — see that
//! module's docstring for the full derivation of each flag. These are
//! pure functions with no side effects (mirroring the Python original),
//! composed by `simulator::Intel8086Simulator::alu16`.
//!
//! ## The six arithmetic/logical flags
//!
//! | Flag | Meaning |
//! |------|---------|
//! | CF | Carry — unsigned overflow/borrow out of the MSB |
//! | PF | Parity — 1 if the low byte of the result has an even number of 1-bits |
//! | AF | Auxiliary carry — carry/borrow out of bit 3 (BCD arithmetic) |
//! | ZF | Zero — result is zero |
//! | SF | Sign — copy of the MSB of the result |
//! | OF | Overflow — signed result lies outside the representable range |
//!
//! `word` selects 8-bit vs 16-bit width (MSB/mask position). This crate's
//! curated opcode subset (see `opcodes.rs`) only ever calls these with
//! `word=true` (16-bit `AX`/general-purpose-register operands) — the
//! `word` parameter is kept for fidelity with the Python original and so
//! a future 8-bit-ALU increment doesn't need to touch this module.

// ── Carry ────────────────────────────────────────────────────────────────

/// Carry flag for ADD: the raw (unmasked, pre-truncation) sum exceeds the
/// representable unsigned range.
///
/// `raw_result` must be the *unmasked* sum (e.g. `a as u32 + b as u32`),
/// not the truncated 16-bit result — mirrors the Python original's
/// `compute_cf_add(result, *, word)`, which is called with the raw
/// unmasked Python int.
pub fn compute_cf_add(raw_result: u32, word: bool) -> bool {
    let limit: u32 = if word { 0xFFFF } else { 0xFF };
    raw_result > limit
}

/// Carry flag for SUB/CMP (as a borrow indicator): `minuend < subtrahend
/// + borrow`. Operands must already be in unsigned range.
pub fn compute_cf_sub(minuend: u32, subtrahend: u32, borrow: u32) -> bool {
    minuend < subtrahend + borrow
}

// ── Auxiliary carry (BCD) ───────────────────────────────────────────────

/// Auxiliary carry flag for ADD: carry out of bit 3 into bit 4.
pub fn compute_af_add(a: u16, b: u16, carry_in: u16) -> bool {
    ((a & 0xF) + (b & 0xF) + carry_in) > 0xF
}

/// Auxiliary carry flag for SUB/CMP: borrow from bit 4 into bit 3.
pub fn compute_af_sub(a: u16, b: u16, borrow: u16) -> bool {
    (a & 0xF) < (b & 0xF) + borrow
}

// ── Overflow ─────────────────────────────────────────────────────────────

/// Overflow flag for ADD: both operands have the same sign, but the
/// result's sign differs.
pub fn compute_of_add(a: u16, b: u16, result: u16, word: bool) -> bool {
    let msb: u16 = if word { 0x8000 } else { 0x80 };
    let mask: u16 = if word { 0xFFFF } else { 0xFF };
    let a_sign = a & msb;
    let b_sign = b & msb;
    let r_sign = result & msb & mask;
    a_sign == b_sign && r_sign != a_sign
}

/// Overflow flag for SUB/CMP: operands have different signs and the
/// result's sign doesn't match the minuend's.
pub fn compute_of_sub(a: u16, b: u16, result: u16, word: bool) -> bool {
    let msb: u16 = if word { 0x8000 } else { 0x80 };
    let mask: u16 = if word { 0xFFFF } else { 0xFF };
    let a_sign = a & msb;
    let b_sign = b & msb;
    let r_sign = result & msb & mask;
    a_sign != b_sign && r_sign != a_sign
}

// ── Sign, Zero, Parity ───────────────────────────────────────────────────

/// Sign flag: copy of the MSB of the masked result.
pub fn compute_sf(result: u16, word: bool) -> bool {
    let msb: u16 = if word { 0x8000 } else { 0x80 };
    let mask: u16 = if word { 0xFFFF } else { 0xFF };
    (result & mask & msb) != 0
}

/// Zero flag: masked result is zero.
pub fn compute_zf(result: u16, word: bool) -> bool {
    let mask: u16 = if word { 0xFFFF } else { 0xFF };
    (result & mask) == 0
}

/// Parity flag: `true` (PF=1) when the LOW BYTE of `result` has an even
/// number of 1-bits — mirrors the Python original's `bin(low).count("1")`.
pub fn compute_parity(result: u16) -> bool {
    let low = (result & 0xFF) as u8;
    low.count_ones().is_multiple_of(2)
}

/// Compute `(SF, ZF, PF)` together from a result value.
pub fn compute_szp(result: u16, word: bool) -> (bool, bool, bool) {
    (
        compute_sf(result, word),
        compute_zf(result, word),
        compute_parity(result),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cf_add_16bit() {
        assert!(!compute_cf_add(0xFFFF, true));
        assert!(compute_cf_add(0x1_0000, true));
    }

    #[test]
    fn cf_add_8bit() {
        assert!(!compute_cf_add(0xFF, false));
        assert!(compute_cf_add(0x100, false));
    }

    #[test]
    fn cf_sub_borrow() {
        assert!(!compute_cf_sub(5, 3, 0));
        assert!(compute_cf_sub(3, 5, 0));
        assert!(compute_cf_sub(5, 5, 1));
    }

    #[test]
    fn af_add_nibble_carry() {
        assert!(compute_af_add(0x0F, 0x01, 0));
        assert!(!compute_af_add(0x01, 0x01, 0));
    }

    #[test]
    fn af_sub_nibble_borrow() {
        assert!(compute_af_sub(0x10, 0x01, 0));
        assert!(!compute_af_sub(0x05, 0x03, 0));
    }

    #[test]
    fn of_add_signed_overflow_16bit() {
        // 0x7FFF + 1 = 0x8000: +32767 + 1 = -32768 signed -- overflow.
        assert!(compute_of_add(0x7FFF, 0x0001, 0x8000, true));
        assert!(!compute_of_add(0x0001, 0x0001, 0x0002, true));
    }

    #[test]
    fn of_sub_signed_overflow_16bit() {
        // 0x8000 - 1 = 0x7FFF: -32768 - 1 "=" +32767 signed -- overflow.
        assert!(compute_of_sub(0x8000, 0x0001, 0x7FFF, true));
        assert!(!compute_of_sub(0x0005, 0x0003, 0x0002, true));
    }

    #[test]
    fn sf_zf_16bit() {
        assert!(compute_sf(0x8000, true));
        assert!(!compute_sf(0x7FFF, true));
        assert!(compute_zf(0, true));
        assert!(!compute_zf(1, true));
        // 0x10000 doesn't fit u16, so use the masked equivalent (0):
        assert!(compute_zf(0x0000, true));
    }

    #[test]
    fn parity_examples() {
        assert!(compute_parity(0)); // 0 ones -- even -- PF=1
        assert!(!compute_parity(1)); // 1 one -- odd -- PF=0
        assert!(compute_parity(3)); // 2 ones -- even -- PF=1
        assert!(compute_parity(0x100)); // high byte ignored; low byte=0 -- PF=1
    }

    #[test]
    fn szp_bundle() {
        assert_eq!(compute_szp(0, true), (false, true, true));
        assert_eq!(compute_szp(0x00FF, false), (true, false, true));
    }
}
