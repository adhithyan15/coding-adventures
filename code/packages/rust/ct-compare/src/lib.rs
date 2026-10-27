//! # coding_adventures_ct_compare — compiler-preserved full-scan equality
//!
//! ## Why a special equality primitive
//!
//! The naive byte-slice equality in Rust is roughly:
//!
//! ```ignore
//! for (a, b) in xs.iter().zip(ys.iter()) {
//!     if a != b { return false; }      // <-- early exit!
//! }
//! true
//! ```
//!
//! On the very first byte that differs, the function returns. An attacker
//! who can *time* the comparison (including by being on the same machine,
//! or over a network with enough samples) can walk a candidate byte by
//! byte:
//!
//! > "When the first byte of my guess matches the real tag, `eq` takes
//! > slightly longer than when it doesn't."
//!
//! This is exactly how the real-world Ruby/Rack `secure_compare` predecessors,
//! Keyczar's HMAC code, Google's 2009 timing-attack-on-passwords incident,
//! and many MAC-verification CVEs were broken. The standard countermeasure
//! is to fold every byte into an XOR accumulator and only look at the
//! accumulator at the end:
//!
//! ```ignore
//! let mut acc: u8 = 0;
//! for i in 0..n { acc |= xs[i] ^ ys[i]; }
//! acc == 0
//! ```
//!
//! Now the source-level loop runs the **same number of iterations regardless
//! of how many bytes match**, and the compiler boundary below preserves every
//! input read. This closes the early-exit prefix leak at the compiler level;
//! it is not a claim about hardware side channels.
//!
//! ## What this crate guarantees
//!
//! * `ct_eq(a, b) -> bool`: true iff `a` and `b` have the same length
//!   *and* the same bytes. The work is constant per-byte and independent
//!   of which byte (if any) differs.
//! * `ct_eq` does *not* short-circuit on differing bytes.
//! * Length mismatch is resolved by comparing `a.len() == b.len()`
//!   directly and returning `false` without iterating. Buffer lengths
//!   are treated as *public* — MAC tags, session ids, session keys all
//!   have a fixed, publicly known length, and leaking length alone
//!   doesn't help an attacker.
//!
//! A barrier only after the loop is insufficient: once an accumulator reaches
//! `0xff`, an optimizer may prove that later ordinary reads cannot change the
//! result and skip them. Each equality-loop iteration therefore performs
//! volatile reads of both input bytes and the accumulator followed by a
//! volatile accumulator write. Rust requires volatile operations to remain
//! observable and ordered with respect to one another, so later input reads
//! cannot be removed based on an earlier difference. An optimized LLVM-IR
//! integration test rejects builds where those operations disappear.
//!
//! ## What this crate does NOT do
//!
//! * **Does not prevent memory-subsystem timing attacks** (cache-timing,
//!   branch-predictor timing, speculative execution). Those need
//!   hardware or kernel countermeasures that are out of scope.
//! * **Does not hide the length** — see above. If you need to hide the
//!   length, pad the input to a fixed size before calling.
//! * **Does not zeroize** — see the sibling `coding_adventures_zeroize`
//!   crate.
//!
//! ## Where it is used
//!
//! * Comparing Poly1305 AEAD tags during decryption
//!   (`coding_adventures_chacha20_poly1305`).
//! * HMAC tag verification (`coding_adventures_hmac`).
//! * Argon2id verify: `ct_eq(computed_tag, stored_tag)`.
//! * Vault unlock: `ct_eq(derived_unlock_key, stored_kek)`.
//! * Any MAC / auth-tag / session-key / password-hash comparison.

#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

use core::hint::black_box;

/// Audited compiler-observability boundary for equality scans.
///
/// Crate-wide `unsafe_code` denial remains active everywhere else. Keeping the
/// narrowly allowed operations in this private module makes any expansion of
/// the unsafe boundary explicit in review.
#[allow(unsafe_code)]
mod volatile_barrier {
    use core::ptr::{read_volatile, write_volatile};

    /// Fold one byte difference into an accumulator using observable operations.
    ///
    /// Both byte references are valid for reads and the accumulator reference is
    /// valid and exclusive for the write for the duration of the call. Volatile
    /// access does not extend either lifetime or escape any pointer.
    #[inline(always)]
    pub(super) fn accumulate(accumulator: &mut u8, left: &u8, right: &u8) {
        // SAFETY: `left` and `right` are valid shared references to initialized
        // bytes. `accumulator` is a valid exclusive reference to an initialized
        // byte. The volatile operations remain within those one-byte objects.
        unsafe {
            let current = read_volatile(accumulator);
            let left_byte = read_volatile(left);
            let right_byte = read_volatile(right);
            write_volatile(accumulator, current | (left_byte ^ right_byte));
        }
    }

    /// Read the completed volatile accumulator without weakening its boundary.
    #[inline(always)]
    pub(super) fn read(accumulator: &u8) -> u8 {
        // SAFETY: `accumulator` is a valid shared reference to one initialized
        // byte, and the volatile read remains within that object.
        unsafe { read_volatile(accumulator) }
    }
}

/// Compare two byte slices with a compiler-preserved full scan.
///
/// Returns `true` iff `a` and `b` have the same length and the same
/// bytes. The loop iterates `len` times regardless of how early a
/// differing byte appears, and the work per iteration does not depend
/// on the byte values.
///
/// Length is treated as public. If the two slices have different
/// lengths, the function returns `false` without inspecting the bytes.
#[inline(never)]
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    ct_eq_equal_length(a, b)
}

/// Compare slices whose equal length has already been established.
///
/// Keeping the volatile scan in one non-inlined function gives both public
/// equality entry points the same audited compiler boundary and gives the
/// optimized-codegen test one exact body to inspect.
#[inline(never)]
fn ct_eq_equal_length(a: &[u8], b: &[u8]) -> bool {
    debug_assert_eq!(a.len(), b.len());
    // Fold every byte through volatile input and accumulator operations. Even
    // if the accumulator reaches 0xff, later reads and writes remain observable
    // and therefore cannot be elided as semantically irrelevant.
    let mut acc: u8 = 0;
    for i in 0..a.len() {
        volatile_barrier::accumulate(&mut acc, &a[i], &b[i]);
    }
    volatile_barrier::read(&acc) == 0
}

/// Compare two fixed-size byte arrays with a compiler-preserved full scan.
///
/// This is the type-safe version of `ct_eq` for the common case where
/// both inputs are the same compile-time size (tag lengths, key lengths,
/// etc.). It elides the length check.
#[inline(never)]
pub fn ct_eq_fixed<const N: usize>(a: &[u8; N], b: &[u8; N]) -> bool {
    ct_eq_equal_length(a, b)
}

/// Select between two same-length byte slices without a source-level branch.
///
/// If `choice` is `true`, returns a copy of `a`; if `false`, returns a
/// copy of `b`. The selection is branchless — no instruction in the
/// function body depends on `choice` via a conditional jump, only on a
/// masked arithmetic combination.
///
/// # Panics
/// Panics if `a.len() != b.len()`. Length is treated as public.
#[inline(never)]
pub fn ct_select_bytes(a: &[u8], b: &[u8], choice: bool) -> Vec<u8> {
    assert_eq!(
        a.len(),
        b.len(),
        "ct_select_bytes requires equal-length slices"
    );
    // Expand `choice` to an all-ones or all-zeros mask. `bool as u8` is
    // guaranteed to be exactly 0 or 1, so `0u8.wrapping_sub(choice as u8)`
    // gives 0x00 or 0xFF — no branch.
    let mask: u8 = 0u8.wrapping_sub(choice as u8);
    let mut out = vec![0u8; a.len()];
    for i in 0..a.len() {
        // When mask = 0xFF, out[i] = a[i].
        // When mask = 0x00, out[i] = b[i].
        // The XOR-and-mask trick: b ^ ((a ^ b) & mask).
        out[i] = b[i] ^ ((a[i] ^ b[i]) & mask);
    }
    out
}

/// Source-level branchless equality on two `u64` values.
///
/// Exposes the same primitive one level up from byte slices, for
/// counters and lease IDs where the comparison must not leak ordering.
#[inline(never)]
pub fn ct_eq_u64(a: u64, b: u64) -> bool {
    let diff = a ^ b;
    // Fold all bits of `diff` into the low bit. If `diff == 0`, the
    // result is 0; otherwise it's 1. All branch-free.
    let folded = (diff | diff.wrapping_neg()) >> 63;
    black_box(folded) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    // === ct_eq ============================================================

    #[test]
    fn equal_slices() {
        assert!(ct_eq(b"abcdef", b"abcdef"));
        assert!(ct_eq(&[0u8; 32], &[0u8; 32]));
        assert!(ct_eq(&[0xFFu8; 64], &[0xFFu8; 64]));
    }

    #[test]
    fn different_last_byte() {
        let a = b"abcdef";
        let b = b"abcdeg";
        assert!(!ct_eq(a, b));
    }

    #[test]
    fn different_first_byte() {
        let a = b"abcdef";
        let b = b"bbcdef";
        assert!(!ct_eq(a, b));
    }

    #[test]
    fn completely_different() {
        let a = [0x00u8; 16];
        let b = [0xFFu8; 16];
        assert!(!ct_eq(&a, &b));
    }

    #[test]
    fn length_mismatch_returns_false() {
        assert!(!ct_eq(b"abc", b"abcd"));
        assert!(!ct_eq(b"abcd", b"abc"));
        assert!(!ct_eq(b"", b"x"));
        assert!(!ct_eq(b"x", b""));
    }

    #[test]
    fn empty_slices_are_equal() {
        assert!(ct_eq(b"", b""));
    }

    #[test]
    fn high_bit_differences_are_detected() {
        // Make sure the accumulator keeps all 8 bits, not just low bits.
        let a: [u8; 4] = [0b0000_0000, 0, 0, 0];
        let b: [u8; 4] = [0b1000_0000, 0, 0, 0];
        assert!(!ct_eq(&a, &b));
    }

    #[test]
    fn every_single_byte_position_is_detected() {
        // For a 32-byte tag, flip one bit of one byte at each position
        // and make sure every flip is detected. This is the test that
        // a short-circuit bug would fail, because a position-dependent
        // short-circuit might miss later positions.
        let base = [0x42u8; 32];
        for i in 0..32 {
            for bit in 0..8 {
                let mut flipped = base;
                flipped[i] ^= 1 << bit;
                assert!(
                    !ct_eq(&base, &flipped),
                    "flip at byte {}, bit {} not detected",
                    i,
                    bit
                );
            }
        }
    }

    // === ct_eq_fixed ======================================================

    #[test]
    fn ct_eq_fixed_matches_ct_eq_for_same_inputs() {
        let a = [0x11u8; 16];
        let b = [0x11u8; 16];
        assert!(ct_eq_fixed(&a, &b));

        let mut c = [0x11u8; 16];
        c[15] ^= 1;
        assert!(!ct_eq_fixed(&a, &c));
    }

    #[test]
    fn ct_eq_fixed_handles_zero_length() {
        let a: [u8; 0] = [];
        let b: [u8; 0] = [];
        assert!(ct_eq_fixed(&a, &b));
    }

    // === ct_select_bytes ==================================================

    #[test]
    fn ct_select_true_picks_a() {
        let a = [0xAAu8; 8];
        let b = [0xBBu8; 8];
        assert_eq!(ct_select_bytes(&a, &b, true), vec![0xAAu8; 8]);
    }

    #[test]
    fn ct_select_false_picks_b() {
        let a = [0xAAu8; 8];
        let b = [0xBBu8; 8];
        assert_eq!(ct_select_bytes(&a, &b, false), vec![0xBBu8; 8]);
    }

    #[test]
    fn ct_select_empty() {
        let a: [u8; 0] = [];
        let b: [u8; 0] = [];
        assert_eq!(ct_select_bytes(&a, &b, true), Vec::<u8>::new());
        assert_eq!(ct_select_bytes(&a, &b, false), Vec::<u8>::new());
    }

    #[test]
    fn ct_select_preserves_every_byte_value() {
        // Feed the full 0..=255 range through to make sure the mask
        // doesn't clobber any bit position.
        let a: Vec<u8> = (0..=255).collect();
        let b: Vec<u8> = (0..=255).rev().collect();
        assert_eq!(ct_select_bytes(&a, &b, true), a);
        assert_eq!(ct_select_bytes(&a, &b, false), b);
    }

    #[test]
    #[should_panic(expected = "equal-length slices")]
    fn ct_select_length_mismatch_panics() {
        let a = [0u8; 4];
        let b = [0u8; 5];
        let _ = ct_select_bytes(&a, &b, true);
    }

    // === ct_eq_u64 ========================================================

    #[test]
    fn ct_eq_u64_equal() {
        assert!(ct_eq_u64(0, 0));
        assert!(ct_eq_u64(u64::MAX, u64::MAX));
        assert!(ct_eq_u64(0xDEAD_BEEF, 0xDEAD_BEEF));
    }

    #[test]
    fn ct_eq_u64_differing_in_any_single_bit() {
        let base: u64 = 0x1234_5678_9ABC_DEF0;
        for bit in 0..64 {
            let flipped = base ^ (1u64 << bit);
            assert!(
                !ct_eq_u64(base, flipped),
                "flip at bit {} not detected",
                bit
            );
        }
    }

    #[test]
    fn ct_eq_u64_differing_in_high_bit() {
        assert!(!ct_eq_u64(0, 1u64 << 63));
    }
}
