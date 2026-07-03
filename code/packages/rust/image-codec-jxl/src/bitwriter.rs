//! # BitWriter — MSB-first bit packing
//!
//! JPEG XL's SizeHeader and a handful of other early fields are packed as raw
//! bits in MSB-first order before the rANS-coded sections begin.  This module
//! provides a simple append-only writer that streams individual bits into a
//! `Vec<u8>`.
//!
//! ## Bit order within a byte
//!
//! The most-significant bit of each output byte is written first.  So if you
//! call `write_bit(1)` then `write_bit(0)` then `write_bit(0)` ... (8 calls)
//! you get one byte whose top bit is 1 and whose lower seven bits are 0:
//!
//! ```text
//! bit 7  bit 6  bit 5  bit 4  bit 3  bit 2  bit 1  bit 0
//!  1      0      0      0      0      0      0      0
//! ```
//! which equals the byte 0x80.
//!
//! ## Flushing
//!
//! Call `finish()` to obtain all the bytes.  A partial final byte is
//! zero-padded in the low bits before being appended.

/// Appends bits MSB-first into a growing byte buffer.
///
/// # Example
///
/// ```rust
/// use image_codec_jxl::bitwriter::BitWriter;
///
/// let mut bw = BitWriter::new();
/// bw.write_bits(0b101, 3);  // writes bits 1, 0, 1 in that order
/// let bytes = bw.finish();
/// // three bits written → one partial byte: 0b101x_xxxx = 0xA0 (zero-padded)
/// assert_eq!(bytes[0], 0xA0);
/// ```
pub struct BitWriter {
    /// Completed bytes.
    bytes: Vec<u8>,
    /// Byte currently being assembled.
    current: u8,
    /// How many bits of `current` are already occupied (0 = none, 7 = all but LSB).
    bits_used: u8,
}

impl BitWriter {
    /// Create an empty writer.
    pub fn new() -> Self {
        BitWriter { bytes: Vec::new(), current: 0, bits_used: 0 }
    }

    /// Append one bit.  `bit = true` → 1, `bit = false` → 0.
    ///
    /// Bits are placed from the MSB side of each byte downward.
    pub fn write_bit(&mut self, bit: bool) {
        // The next free bit position within `current` is (7 - bits_used),
        // counting from the MSB.
        if bit {
            self.current |= 1 << (7 - self.bits_used);
        }
        self.bits_used += 1;
        if self.bits_used == 8 {
            // The byte is complete — flush it and start a new one.
            self.bytes.push(self.current);
            self.current = 0;
            self.bits_used = 0;
        }
    }

    /// Append the `count` least-significant bits of `value`, MSB-first.
    ///
    /// For example `write_bits(0b101, 3)` writes the sequence 1 → 0 → 1.
    ///
    /// `count` must be ≤ 64.
    pub fn write_bits(&mut self, value: u64, count: u8) {
        // Emit from the most-significant bit of the requested range downward.
        for shift in (0..count).rev() {
            self.write_bit((value >> shift) & 1 == 1);
        }
    }

    /// Flush any partial byte (zero-padded) and return the complete byte buffer.
    ///
    /// The writer is consumed by this call.
    pub fn finish(mut self) -> Vec<u8> {
        if self.bits_used > 0 {
            // Zero-pad the remaining bits on the low side.
            self.bytes.push(self.current);
        }
        self.bytes
    }

    /// Total bytes that `finish()` would produce right now, without consuming.
    pub fn byte_count(&self) -> usize {
        self.bytes.len() + if self.bits_used > 0 { 1 } else { 0 }
    }
}

impl Default for BitWriter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_bit_high() {
        let mut bw = BitWriter::new();
        bw.write_bit(true);
        let b = bw.finish();
        // 1 bit at MSB → byte 0x80
        assert_eq!(b, &[0x80]);
    }

    #[test]
    fn eight_bits_exact_byte() {
        let mut bw = BitWriter::new();
        bw.write_bits(0xAB, 8);
        let b = bw.finish();
        assert_eq!(b, &[0xAB]);
    }

    #[test]
    fn multiple_bytes() {
        let mut bw = BitWriter::new();
        bw.write_bits(0xDEAD, 16);
        let b = bw.finish();
        assert_eq!(b, &[0xDE, 0xAD]);
    }

    #[test]
    fn partial_byte_zero_padded() {
        let mut bw = BitWriter::new();
        bw.write_bits(0b101, 3);
        let b = bw.finish();
        // bits: 1,0,1,0,0,0,0,0 → 0xA0
        assert_eq!(b, &[0xA0]);
    }

    #[test]
    fn zero_bits_no_output() {
        let bw = BitWriter::new();
        assert_eq!(bw.finish(), &[] as &[u8]);
    }

    #[test]
    fn byte_count_matches_finish_len() {
        let mut bw = BitWriter::new();
        bw.write_bits(0xFF, 8);
        bw.write_bit(true);
        assert_eq!(bw.byte_count(), 2);
        let b = bw.finish();
        assert_eq!(b.len(), 2);
    }
}
