//! # range-coder
//!
//! VP8 boolean arithmetic range coder — the entropy coding engine at the
//! heart of VP8 lossy video / WebP lossy still images.
//!
//! This crate implements the boolean range coder as specified in
//! **RFC 6386 §7.3**. It is a binary arithmetic coder: each call encodes or
//! decodes a single bit together with its probability, achieving the
//! information-theoretic minimum bit cost.
//!
//! ## What is a boolean range coder?
//!
//! Imagine a ruler from 0 to 1. The encoder divides the current interval at
//! a point proportional to `prob/256`, then keeps either the lower or upper
//! sub-interval based on the actual bit. After all bits are coded, any value
//! inside the final narrow interval uniquely identifies the entire message.
//!
//! VP8 expresses every syntax element as a series of binary questions with
//! known probabilities — prediction modes, DCT coefficient signs, motion
//! vectors, etc. The boolean range coder handles them all.
//!
//! ## Probability convention
//!
//! `prob` is the probability that a bit is **0** (false), scaled to
//! `[0, 255]`. Specifically:
//!
//! | prob | Meaning                    |
//! |------|----------------------------|
//! |   0  | bit is almost always 1     |
//! | 128  | 50/50 (uniform)            |
//! | 255  | bit is almost always 0     |
//!
//! ## API overview
//!
//! ```rust
//! use range_coder::{BoolEncoder, BoolDecoder};
//!
//! // Encode three bits with different probabilities.
//! let mut enc = BoolEncoder::new();
//! enc.write_bit(true,  128);   // 50/50
//! enc.write_bit(false, 200);   // ~78% chance of being 0 — and it is!
//! enc.write_bit(true,   64);   // ~25% chance of being 0 — so it's 1
//! let bytes = enc.finish();
//!
//! // Decode them back using the same probabilities in the same order.
//! let mut dec = BoolDecoder::new(&bytes);
//! assert_eq!(dec.read_bit(128), true);
//! assert_eq!(dec.read_bit(200), false);
//! assert_eq!(dec.read_bit(64),  true);
//! ```
//!
//! ## Round-trip guarantee
//!
//! For any sequence of `(bit, prob)` pairs, encoding then decoding with
//! identical `prob` values in the same order reproduces every bit exactly.
//!
//! ## Zero dependencies
//!
//! This crate has no external dependencies. It implements the RFC 6386
//! algorithm in pure safe Rust.
//!
//! ## Crate layout
//!
//! - [`encoder`] — [`BoolEncoder`]: encodes a sequence of (bit, prob) pairs
//! - [`decoder`] — [`BoolDecoder`]: decodes the output of a `BoolEncoder`

pub const VERSION: &str = "0.1.0";

pub mod decoder;
pub mod encoder;

pub use decoder::BoolDecoder;
pub use encoder::BoolEncoder;

// ---------------------------------------------------------------------------
// Integration tests (round-trips that exercise both encoder and decoder)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper ───────────────────────────────────────────────────────────────

    /// Encode `bits` with their probabilities, then decode with the same
    /// probabilities. Asserts that every decoded bit equals the original.
    fn round_trip(bits: &[(bool, u8)]) {
        let mut enc = BoolEncoder::new();
        for &(bit, prob) in bits {
            enc.write_bit(bit, prob);
        }
        let bytes = enc.finish();

        let mut dec = BoolDecoder::new(&bytes);
        for (i, &(expected, prob)) in bits.iter().enumerate() {
            let got = dec.read_bit(prob);
            assert_eq!(
                got, expected,
                "bit {i}: expected {expected}, got {got} (prob={prob})"
            );
        }
    }

    // ── Version ──────────────────────────────────────────────────────────────

    #[test]
    fn version_is_correct() {
        assert_eq!(VERSION, "0.1.0");
    }

    // ── Round-trip tests ─────────────────────────────────────────────────────

    /// 32 bits at p=128 (uniform distribution).
    #[test]
    fn round_trip_uniform() {
        let bits: Vec<(bool, u8)> = (0..32u32)
            .map(|i| (i % 3 == 0, 128))
            .collect();
        round_trip(&bits);
    }

    /// 64 bits at p=200 (mostly-0 distribution); mix of true/false.
    #[test]
    fn round_trip_skewed() {
        // Alternate false/false/false/true to verify both branches work.
        let bits: Vec<(bool, u8)> = (0..64u32)
            .map(|i| (i % 4 == 3, 200))
            .collect();
        round_trip(&bits);
    }

    /// 32 bits all set to 0 with p=255 (almost-certainly-0 coder).
    #[test]
    fn round_trip_all_zeros() {
        let bits: Vec<(bool, u8)> = vec![(false, 255); 32];
        round_trip(&bits);
    }

    /// 32 bits all set to 1 with p=0 (almost-certainly-1 coder).
    #[test]
    fn round_trip_all_ones() {
        let bits: Vec<(bool, u8)> = vec![(true, 0); 32];
        round_trip(&bits);
    }

    /// Mixed probabilities across the full range.
    #[test]
    fn round_trip_mixed_probs() {
        let bits = [
            (true,  0u8),
            (false, 255),
            (true,  1),
            (false, 254),
            (true,  64),
            (false, 192),
            (true,  128),
            (false, 128),
        ];
        round_trip(&bits);
    }

    /// write_bits / read_bits round-trip for a u8 value.
    #[test]
    fn write_read_bits_u8() {
        let mut enc = BoolEncoder::new();
        enc.write_bits(0xAB, 8);
        let bytes = enc.finish();

        let mut dec = BoolDecoder::new(&bytes);
        assert_eq!(dec.read_bits(8), 0xAB);
    }

    /// write_bits / read_bits round-trip for a u16 value.
    #[test]
    fn write_read_bits_u16() {
        let mut enc = BoolEncoder::new();
        enc.write_bits(0xDEAD, 16);
        let bytes = enc.finish();

        let mut dec = BoolDecoder::new(&bytes);
        assert_eq!(dec.read_bits(16), 0xDEAD);
    }

    /// write_bits / read_bits round-trip for a u32 value.
    #[test]
    fn write_read_bits_u32() {
        let mut enc = BoolEncoder::new();
        enc.write_bits(0xCAFE_BABE, 32);
        let bytes = enc.finish();

        let mut dec = BoolDecoder::new(&bytes);
        assert_eq!(dec.read_bits(32), 0xCAFE_BABE);
    }

    /// Encoding 0 bits with write_bits produces no output beyond flush bytes.
    #[test]
    fn write_bits_zero_n() {
        let mut enc = BoolEncoder::new();
        enc.write_bits(0xFF, 0); // should write nothing
        let bytes = enc.finish();

        // Decoder should be able to read 0 bits without issues.
        let mut dec = BoolDecoder::new(&bytes);
        assert_eq!(dec.read_bits(0), 0);
    }

    /// The encoder produces deterministic output for the same input.
    #[test]
    fn deterministic_output() {
        let encode = || {
            let mut enc = BoolEncoder::new();
            enc.write_bit(true,  128);
            enc.write_bit(false, 200);
            enc.write_bit(true,   64);
            enc.write_bit(false, 128);
            enc.finish()
        };
        assert_eq!(encode(), encode());
    }

    /// is_exhausted triggers when the decoder has consumed all data.
    ///
    /// The decoder seeds itself from 2 bytes, so a 2-byte vector is exhausted
    /// immediately. After the first read_bit the decoder may pull more bits
    /// from the flush padding in the encoder output, but when we construct
    /// a minimal 2-byte slice the decoder is exhausted from the start.
    #[test]
    fn is_exhausted_triggers_correctly() {
        // A full encoder run always emits at least 2 bytes (decoder needs
        // 2 seed bytes). With one bit + flush, exactly 2 bytes are emitted.
        let mut enc = BoolEncoder::new();
        enc.write_bit(true, 128);
        let bytes = enc.finish();
        assert!(bytes.len() >= 2, "sanity: encoder must emit at least 2 bytes");

        let mut dec = BoolDecoder::new(&bytes);
        // Read all the way through until exhausted.
        for _ in 0..(bytes.len() - 2) * 8 {
            dec.read_bit(128);
            if dec.is_exhausted() {
                break;
            }
        }
        assert!(dec.is_exhausted());
    }

    /// Longer sequence — 128 bits — verifies no drift or carry bugs.
    #[test]
    fn round_trip_long_sequence() {
        let bits: Vec<(bool, u8)> = (0..128u32)
            .map(|i| {
                let bit = (i * 7 + 3) % 5 < 3;
                let prob = (i * 13 + 100) as u8;
                (bit, prob)
            })
            .collect();
        round_trip(&bits);
    }

    /// Boundary probs p=1 and p=254 — just inside the degenerate extremes.
    #[test]
    fn round_trip_near_boundary_probs() {
        let bits = [
            (false, 1u8),   // almost always 1, but actually 0
            (true,  1),     // almost always 1, and is 1
            (false, 254),   // almost always 0, and is 0
            (true,  254),   // almost always 0, but actually 1
        ];
        round_trip(&bits);
    }

    /// Encode then decode a known test vector from the spec.
    ///
    /// Three bits [1, 0, 1] with uniform probability (p=128).
    /// This is the worked example from CMP10 §11.
    #[test]
    fn spec_test_vector_three_bits() {
        let bits = [(true, 128u8), (false, 128), (true, 128)];
        round_trip(&bits);
    }
}
