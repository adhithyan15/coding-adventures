//! # BitReader — MSB-first bit extraction
//!
//! The decoder counterpart to [`BitWriter`](super::bitwriter::BitWriter).
//!
//! Reads individual bits from a byte slice in MSB-first order.  Attempting
//! to read past the end returns `Err` rather than silently producing garbage.
//!
//! ## Alignment helper
//!
//! After reading the variable-width SizeHeader, the remaining bytes start on
//! an arbitrary bit boundary.  `align_to_byte()` advances the cursor to the
//! next byte boundary so subsequent reads start cleanly.

/// Extracts bits MSB-first from a borrowed byte slice.
///
/// # Example
///
/// ```rust
/// use image_codec_jxl::bitreader::BitReader;
///
/// let data = &[0b1010_1010u8];
/// let mut br = BitReader::new(data);
/// assert_eq!(br.read_bit().unwrap(), true);   // MSB of 0xAA
/// assert_eq!(br.read_bit().unwrap(), false);
/// assert_eq!(br.read_bit().unwrap(), true);
/// ```
pub struct BitReader<'a> {
    data: &'a [u8],
    /// Index of the byte currently being read from.
    byte_pos: usize,
    /// Which bit within that byte to read next.  0 = MSB, 7 = LSB.
    bit_pos: u8,
}

impl<'a> BitReader<'a> {
    /// Create a reader over `data`, positioned at the very first bit.
    pub fn new(data: &'a [u8]) -> Self {
        BitReader { data, byte_pos: 0, bit_pos: 0 }
    }

    /// Read the next bit.
    ///
    /// Returns `Err` if the bitstream is exhausted.
    pub fn read_bit(&mut self) -> Result<bool, String> {
        if self.byte_pos >= self.data.len() {
            return Err("JXL: unexpected end of bitstream".into());
        }
        // Extract the bit at position `bit_pos` counting from the MSB.
        let bit = (self.data[self.byte_pos] >> (7 - self.bit_pos)) & 1 == 1;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        Ok(bit)
    }

    /// Read `count` bits (≤ 64), assembling them MSB-first into a `u64`.
    ///
    /// The first bit read becomes the most significant of the returned value.
    pub fn read_bits(&mut self, count: u8) -> Result<u64, String> {
        let mut value = 0u64;
        for _ in 0..count {
            value = (value << 1) | (self.read_bit()? as u64);
        }
        Ok(value)
    }

    /// Advance the cursor to the next byte boundary.
    ///
    /// If we are already byte-aligned (`bit_pos == 0`) this is a no-op.
    /// The skipped bits (padding) are discarded.
    pub fn align_to_byte(&mut self) {
        if self.bit_pos > 0 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
    }

    /// Number of *bytes* consumed so far (rounds up if partially into a byte).
    ///
    /// After `align_to_byte()` this equals the byte offset of the next clean
    /// data byte.
    pub fn bytes_consumed(&self) -> usize {
        if self.bit_pos > 0 {
            self.byte_pos + 1
        } else {
            self.byte_pos
        }
    }

    /// The remaining bytes starting from the next byte boundary.
    ///
    /// If the reader is in the middle of a byte, that byte is skipped.
    /// Useful for switching to a byte-oriented parser after reading the
    /// bit-packed SizeHeader.
    pub fn remaining_bytes_from_boundary(&self) -> &'a [u8] {
        let pos = self.bytes_consumed();
        let pos = pos.min(self.data.len());
        &self.data[pos..]
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_single_bit_msb() {
        let mut br = BitReader::new(&[0x80]); // 1000_0000
        assert!(br.read_bit().unwrap());
        assert!(!br.read_bit().unwrap());
    }

    #[test]
    fn read_eight_bits() {
        let mut br = BitReader::new(&[0xAB]);
        assert_eq!(br.read_bits(8).unwrap(), 0xAB);
    }

    #[test]
    fn read_across_byte_boundary() {
        let mut br = BitReader::new(&[0xFF, 0x00]);
        assert_eq!(br.read_bits(4).unwrap(), 0x0F);  // top 4 bits of 0xFF
        assert_eq!(br.read_bits(4).unwrap(), 0x0F);  // bottom 4 bits of 0xFF
        assert_eq!(br.read_bits(8).unwrap(), 0x00);  // second byte
    }

    #[test]
    fn exhausted_returns_err() {
        let mut br = BitReader::new(&[]);
        assert!(br.read_bit().is_err());
    }

    #[test]
    fn align_to_byte_skips_partial() {
        let mut br = BitReader::new(&[0xFF, 0xAB]);
        let _ = br.read_bits(3).unwrap();
        br.align_to_byte();
        assert_eq!(br.read_bits(8).unwrap(), 0xAB);
    }

    #[test]
    fn align_already_aligned_is_noop() {
        let mut br = BitReader::new(&[0xDE, 0xAD]);
        br.align_to_byte(); // already at bit 0 of byte 0
        assert_eq!(br.read_bits(8).unwrap(), 0xDE);
    }

    #[test]
    fn bytes_consumed_partial() {
        let mut br = BitReader::new(&[0xFF]);
        let _ = br.read_bits(3).unwrap();
        assert_eq!(br.bytes_consumed(), 1); // partially into byte 0
    }

    #[test]
    fn remaining_bytes_after_align() {
        let mut br = BitReader::new(&[0xFF, 0xAA, 0xBB]);
        let _ = br.read_bits(1).unwrap();
        br.align_to_byte();
        assert_eq!(br.remaining_bytes_from_boundary(), &[0xAA, 0xBB]);
    }
}
