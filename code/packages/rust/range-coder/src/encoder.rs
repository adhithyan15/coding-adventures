//! VP8 boolean range encoder.
//!
//! # Encoding algorithm
//!
//! The encoder is the inverse of the decoder. It maintains a representation
//! of the current coding interval and emits bytes to the output stream as the
//! interval narrows enough that high-order bytes are determined.
//!
//! ## State
//!
//! ```text
//! bottom        : u32  — low end of the current interval (24-bit working register)
//! range         : u32  — interval width, kept in [128, 255] after normalization
//! bit_count     : i32  — counts renormalization shifts; output when it hits 0
//! output        : Vec<u8>
//! ```
//!
//! ## Encoding one bit
//!
//! ```text
//! split = 1 + (((range - 1) * prob as u32) >> 8)
//!
//! if bit == true:
//!     bottom += split   // take the upper sub-interval
//!     range  -= split
//! else:
//!     range   = split   // take the lower sub-interval
//!
//! // Renormalize while range < 128:
//! while range < 128:
//!     range     <<= 1
//!     bit_count  += 1
//!     if bit_count == 0:
//!         output.push((bottom >> 16) as u8)
//!         bottom = (bottom << 1) & 0xFF_FFFF  // keep 24-bit working register
//! ```
//!
//! The unusual `bit_count` initialisation of −24 gives the encoder 24
//! free normalization steps before it emits the first byte. This matches
//! the decoder's initial 16-bit seed: the encoder will have emitted exactly
//! the bytes needed to reconstruct the decoder's `value` register.
//!
//! ## Flushing
//!
//! After all bits are written, the encoder clocks out ~32 extra zero bits.
//! This pushes the remaining bytes through the normalization pipeline and
//! ensures the decoder can read all coded bits without starving.

/// VP8 boolean range encoder.
///
/// Encodes a sequence of (bit, probability) pairs into a byte vector.
/// The output is a valid VP8 boolean-coded data partition: the first two
/// bytes seed the decoder's `value` register, and subsequent bytes provide
/// the renormalized bitstream.
///
/// # Example
///
/// ```rust
/// use range_coder::{BoolEncoder, BoolDecoder};
///
/// let mut enc = BoolEncoder::new();
/// enc.write_bit(false, 200);  // almost certainly false
/// enc.write_bit(true,  64);   // more likely true
/// let bytes = enc.finish();
///
/// let mut dec = BoolDecoder::new(&bytes);
/// assert_eq!(dec.read_bit(200), false);
/// assert_eq!(dec.read_bit(64),  true);
/// ```
pub struct BoolEncoder {
    /// The lower bound of the current coding interval.
    ///
    /// Kept as u64 to avoid overflow during normalization shifts. Between
    /// emits, `bottom` is always in [0, 2^32) because after each emit we
    /// mask to 24 bits and then shift at most 8 more times before the next
    /// emit (8 shifts of a 24-bit value = 32-bit max).
    bottom: u64,
    /// Current interval width. Kept in [128, 255] after renormalization.
    range: u32,
    /// Renormalization shift counter. Starts at -24; increments on each
    /// shift; when it reaches 0 we emit a byte from the top of `bottom`
    /// and reset to -8 (ready for the next byte).
    bit_count: i32,
    /// Encoded output bytes.
    output: Vec<u8>,
}

impl BoolEncoder {
    /// Create a new encoder in its initial state.
    pub fn new() -> Self {
        BoolEncoder {
            bottom: 0,
            range: 255,
            bit_count: -24,
            output: Vec::new(),
        }
    }

    /// Encode one bit. `prob` is the probability that the bit is **0**,
    /// scaled to [0, 255] where 128 ≈ 50/50.
    pub fn write_bit(&mut self, bit: bool, prob: u8) {
        // Compute the interval split point.
        let split = 1 + (((self.range - 1) * prob as u32) >> 8);

        if bit {
            // The true (1) branch occupies the upper sub-interval.
            self.bottom += split as u64;
            self.range -= split;
        } else {
            // The false (0) branch occupies the lower sub-interval.
            self.range = split;
        }

        // Renormalize: shift left until range >= 128, emitting one byte for
        // every 8 normalization steps.
        //
        // `bottom` and `range` are shifted together — this mirrors the
        // decoder's: while range < 128: range <<= 1; value = (value << 1) | bit.
        while self.range < 128 {
            self.range <<= 1;
            self.bottom <<= 1; // always shift; never mask mid-loop
            self.bit_count += 1;

            if self.bit_count == 0 {
                // Top byte of `bottom` is fully determined — emit it.
                // `bottom` has been shifted 24 times since the last emit
                // (or from initialisation), so bit 31 is the oldest bit.
                self.output.push((self.bottom >> 24) as u8);
                // Discard the emitted byte; keep the lower 24 bits for the
                // next round. No extra shift here — the loop already shifted.
                self.bottom &= 0x00FF_FFFF;
                self.bit_count = -8; // 8 more shifts until next emit
            }
        }
    }

    /// Encode `n` bits from `value` (MSB first) with uniform probability
    /// (prob = 128). `n` must be ≤ 32.
    pub fn write_bits(&mut self, value: u32, n: u8) {
        for i in (0..n).rev() {
            let bit = (value >> i) & 1 == 1;
            self.write_bit(bit, 128);
        }
    }

    /// Flush the encoder and return the encoded byte vector.
    ///
    /// This clocks out 32 zero bits to push all pending output through the
    /// normalization pipeline. The extra trailing zeros are harmless — the
    /// decoder stops reading once it has decoded the expected number of bits.
    pub fn finish(mut self) -> Vec<u8> {
        // Flush remaining output by encoding 32 zero bits.
        for _ in 0..32 {
            self.write_bit(false, 128);
        }
        self.output
    }
}

impl Default for BoolEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state() {
        let enc = BoolEncoder::new();
        assert_eq!(enc.bottom, 0u64);
        assert_eq!(enc.range, 255);
        assert_eq!(enc.bit_count, -24);
        assert!(enc.output.is_empty());
    }

    /// finish() on a fresh encoder should emit a non-empty output (the
    /// flush zeros drive bytes out through the normalization pipeline).
    #[test]
    fn finish_produces_output() {
        let enc = BoolEncoder::new();
        let out = enc.finish();
        assert!(!out.is_empty());
    }

    /// Encoding the same sequence twice gives the same bytes.
    #[test]
    fn deterministic_output() {
        let seq = [(true, 128u8), (false, 200), (true, 64), (false, 128)];
        let encode = || {
            let mut enc = BoolEncoder::new();
            for &(bit, prob) in &seq {
                enc.write_bit(bit, prob);
            }
            enc.finish()
        };
        assert_eq!(encode(), encode());
    }
}
