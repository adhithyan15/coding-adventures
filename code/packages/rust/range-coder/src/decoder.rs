//! VP8 boolean range decoder (RFC 6386 §7.3).
//!
//! # How it works
//!
//! The decoder maintains two state variables that together represent a "window"
//! into the compressed bitstream:
//!
//! - `range`: the current coding interval width, kept in [128, 255] by
//!   periodic renormalization.
//! - `value`: the bits we have read from the stream so far, interpreted as a
//!   fixed-point fraction.
//!
//! To decode one bit with probability `prob` (prob = P(bit is 0) × 256):
//!
//! ```text
//! split     = 1 + (((range - 1) * prob as u32) >> 8)
//! bigsplit  = split << 8     // move to the same fixed-point scale as value
//!
//! if value >= bigsplit:
//!     bit   = 1  (true)
//!     range -= split
//!     value -= bigsplit
//! else:
//!     bit   = 0  (false)
//!     range = split
//!
//! // Re-normalise so range stays in [128, 255]
//! while range < 128:
//!     range <<= 1
//!     value  = (value << 1) | next_msb_bit_from_stream
//! ```
//!
//! The `+1` in the split formula is a VP8-specific bias that prevents
//! `split == 0` even when `prob == 0`, keeping the invariant that both
//! sub-intervals are non-empty.
//!
//! # Bit ordering
//!
//! VP8 compressed data is MSB-first: the most significant bit of the first
//! byte is the first logical bit. We track `bit_pos` (0 = MSB, 7 = LSB) to
//! consume bits in that order.
//!
//! # Seeding
//!
//! The decoder is seeded with the first two bytes of data:
//!   value  = (data[0] as u32) << 8 | (data[1] as u32)
//!   range  = 255
//!   pos    = 2  (byte cursor, pointing past the two seed bytes)
//!   bit_pos = 0 (bit cursor within current byte)
//!
//! These two bytes initialise the 16-bit window; renormalization then
//! slides additional bits in from the stream as needed.

/// VP8 boolean range decoder.
///
/// Decodes a sequence of (bit, probability) pairs from a compressed byte
/// slice. The byte slice is the raw VP8 boolean-coded data as it appears
/// in the bitstream — typically the first partition of a VP8 frame.
///
/// # Example
///
/// ```rust
/// use range_coder::{BoolEncoder, BoolDecoder};
///
/// let mut enc = BoolEncoder::new();
/// enc.write_bit(true,  128);
/// enc.write_bit(false, 200);
/// enc.write_bit(true,  64);
/// let bytes = enc.finish();
///
/// let mut dec = BoolDecoder::new(&bytes);
/// assert_eq!(dec.read_bit(128), true);
/// assert_eq!(dec.read_bit(200), false);
/// assert_eq!(dec.read_bit(64),  true);
/// ```
pub struct BoolDecoder<'a> {
    /// Source byte slice (the full VP8 bool-coded partition).
    data: &'a [u8],
    /// Byte cursor — points to the next byte whose bits have not yet been
    /// loaded into the `value` register. Starts at 2 (past the seed bytes).
    pos: usize,
    /// Bit offset within the current byte when reading sub-byte bits.
    /// 0 = MSB (bit 7), 7 = LSB (bit 0). Advances left-to-right.
    bit_pos: u8,
    /// Current coding interval width. Kept in the range [128, 255] by
    /// renormalization. Starts at 255.
    range: u32,
    /// Window register. After seeding, holds a 16-bit value where the high
    /// byte represents the integer part and the low byte the fractional part
    /// of the current interval position. Grows as renormalization reads more
    /// bits from the stream.
    value: u32,
}

impl<'a> BoolDecoder<'a> {
    /// Create a new decoder, seeded from the first two bytes of `data`.
    ///
    /// Panics if `data` is shorter than 2 bytes; callers must verify the
    /// buffer length before constructing a decoder (VP8 frames always have
    /// at least a frame header that is longer than 2 bytes).
    pub fn new(data: &'a [u8]) -> Self {
        // The spec seeds the 16-bit value register from bytes 0 and 1.
        // Byte 0 becomes the high byte, byte 1 the low byte.
        let value = if data.len() >= 2 {
            ((data[0] as u32) << 8) | (data[1] as u32)
        } else if data.len() == 1 {
            (data[0] as u32) << 8
        } else {
            0
        };
        BoolDecoder {
            data,
            pos: 2,
            bit_pos: 0,
            range: 255,
            value,
        }
    }

    /// Decode one bit. `prob` is the probability that the bit is **0**,
    /// encoded as a fixed-point u8 where 0 → 0/256 and 255 → 255/256.
    /// `prob = 128` represents a 50/50 distribution.
    ///
    /// Returns `true` (bit = 1) or `false` (bit = 0).
    pub fn read_bit(&mut self, prob: u8) -> bool {
        // Split the current interval at the position corresponding to prob.
        // The +1 prevents split == 0, which would leave the 1-branch with
        // zero width and break the invariant.
        let split = 1 + (((self.range - 1) * prob as u32) >> 8);

        // Scale split to the same 16-bit fixed-point coordinate as value.
        let bigsplit = split << 8;

        let bit = if self.value >= bigsplit {
            // The encoded value falls in the upper (1) sub-interval.
            self.range -= split;
            self.value -= bigsplit;
            true
        } else {
            // The encoded value falls in the lower (0) sub-interval.
            self.range = split;
            false
        };

        // Renormalize: shift left until range >= 128, reading one fresh bit
        // from the input stream on each shift to keep value valid.
        while self.range < 128 {
            self.range <<= 1;
            self.value = (self.value << 1) | self.next_msb_bit();
        }

        bit
    }

    /// Decode `n` bits with uniform probability (prob = 128), assembling
    /// the result MSB-first into a u32.
    ///
    /// `n` must be ≤ 32. Passing `n = 0` returns 0 without reading anything.
    pub fn read_bits(&mut self, n: u8) -> u32 {
        let mut result = 0u32;
        for _ in 0..n {
            result = (result << 1) | (self.read_bit(128) as u32);
        }
        result
    }

    /// Returns `true` if the byte cursor has reached or passed the end of the
    /// data slice. Note that the decoder may still successfully read bits
    /// after this returns `true`, because the value register holds a window
    /// of already-loaded bits; missing bytes are treated as zeros.
    pub fn is_exhausted(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Pull the next MSB-first bit from the input stream.
    ///
    /// Bits within each byte are read from bit 7 (MSB) down to bit 0 (LSB).
    /// When the stream is exhausted, returns 0 — this pads the stream with
    /// trailing zero bits, which is the correct VP8 behaviour.
    fn next_msb_bit(&mut self) -> u32 {
        if self.pos >= self.data.len() {
            // Pad exhausted stream with zeros.
            return 0;
        }

        // Extract the bit at position `bit_pos` within the current byte.
        // bit_pos == 0 → MSB (shift by 7); bit_pos == 7 → LSB (shift by 0).
        let byte = self.data[self.pos];
        let bit = ((byte >> (7 - self.bit_pos)) & 1) as u32;

        // Advance the bit cursor; move to the next byte when exhausted.
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.pos += 1;
        }

        bit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BoolEncoder;

    #[test]
    fn new_seeds_correctly() {
        let data = [0xAB, 0xCD];
        let dec = BoolDecoder::new(&data);
        assert_eq!(dec.range, 255);
        assert_eq!(dec.value, 0xABCD);
        assert_eq!(dec.pos, 2);
        assert_eq!(dec.bit_pos, 0);
    }

    #[test]
    fn new_empty_data_does_not_panic() {
        let dec = BoolDecoder::new(&[]);
        assert_eq!(dec.value, 0);
        assert_eq!(dec.range, 255);
    }

    #[test]
    fn new_single_byte_does_not_panic() {
        let dec = BoolDecoder::new(&[0xFF]);
        assert_eq!(dec.value, 0xFF00);
        assert_eq!(dec.range, 255);
    }

    #[test]
    fn is_exhausted_after_seed() {
        let data = [0x00, 0x00]; // 2 bytes consumed by seed
        let dec = BoolDecoder::new(&data);
        assert!(dec.is_exhausted());
    }

    #[test]
    fn is_exhausted_false_with_extra_bytes() {
        let data = [0x00, 0x00, 0xFF];
        let dec = BoolDecoder::new(&data);
        assert!(!dec.is_exhausted());
    }

    /// Round-trip a single 1-bit with p=128 (50/50).
    #[test]
    fn round_trip_single_true() {
        let mut enc = BoolEncoder::new();
        enc.write_bit(true, 128);
        let bytes = enc.finish();
        let mut dec = BoolDecoder::new(&bytes);
        assert!(dec.read_bit(128));
    }

    #[test]
    fn round_trip_single_false() {
        let mut enc = BoolEncoder::new();
        enc.write_bit(false, 128);
        let bytes = enc.finish();
        let mut dec = BoolDecoder::new(&bytes);
        assert!(!dec.read_bit(128));
    }

    /// Decode read_bits returns 0 when n == 0.
    #[test]
    fn read_bits_zero_len() {
        let data = [0x00, 0x00, 0x00, 0x00];
        let mut dec = BoolDecoder::new(&data);
        assert_eq!(dec.read_bits(0), 0);
    }
}
