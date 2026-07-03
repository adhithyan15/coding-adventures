//! LSB-first bit reader and writer for the VP8L bitstream.
//!
//! VP8L packs bits LSB-first: the first bit written appears in bit 0 of the
//! first byte, the second bit in bit 1, and so on — exactly like DEFLATE.
//!
//! ## Why LSB-first?
//!
//! WebP inherits the bit-packing convention from VP8 (the video codec), which
//! itself follows the convention used in many image/video codecs. LSB-first
//! means that the lowest-significance bit of each symbol is written first into
//! the lowest-significance bit of the current byte. This makes "byte-aligning"
//! very cheap: just flush the accumulator.
//!
//! ## Example
//!
//! Writing 3 bits `0b101` then 5 bits `0b11001`:
//! ```text
//! buf = 0b00000000
//! write 101: buf |= 101 << 0 → 0b00000101, bits=3
//! write 11001: buf |= 11001 << 3 → 0b11001_101, bits=8
//! flush byte → output [0b11001101]
//! ```

/// A bit-level writer that accumulates bits in an internal u64 buffer and
/// flushes complete bytes to the output vector when they fill up.
///
/// The internal accumulator `buf` holds bits starting from the LSB.
/// `bits` tracks how many bits are currently buffered.
pub struct BitWriter {
    /// Accumulator: bits are packed starting at bit 0 (LSB-first).
    buf: u64,
    /// Number of valid bits currently held in `buf`.
    bits: u32,
    /// Flushed output bytes.
    output: Vec<u8>,
}

impl BitWriter {
    /// Create a new, empty `BitWriter`.
    pub fn new() -> Self {
        Self { buf: 0, bits: 0, output: Vec::new() }
    }

    /// Write the low `count` bits of `value` into the stream.
    ///
    /// Bits are placed starting at the current LSB position of the accumulator.
    /// Complete bytes are flushed to `output` automatically.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if `count > 56` (the safe limit for a u64 accumulator
    /// that might already hold up to 7 buffered bits).
    pub fn write_bits(&mut self, value: u64, count: u32) {
        debug_assert!(count <= 56, "write_bits: count={count} exceeds 56-bit safe limit");
        self.buf |= value << self.bits;
        self.bits += count;
        // Flush complete bytes.
        while self.bits >= 8 {
            self.output.push((self.buf & 0xFF) as u8);
            self.buf >>= 8;
            self.bits -= 8;
        }
    }

    /// Finish writing and return all buffered bytes.
    ///
    /// Any remaining buffered bits (fewer than 8) are flushed as a final
    /// zero-padded byte.
    pub fn finish(mut self) -> Vec<u8> {
        if self.bits > 0 {
            self.output.push((self.buf & 0xFF) as u8);
        }
        self.output
    }

    /// Current number of bytes written (not counting any partial byte still in
    /// the accumulator). Useful for alignment checks.
    pub fn bytes_written(&self) -> usize {
        self.output.len()
    }
}

impl Default for BitWriter {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// BitReader
// ---------------------------------------------------------------------------

/// A bit-level reader that refills its accumulator from a byte slice on demand.
///
/// Like `BitWriter`, bits are packed LSB-first. `read_bits(n)` returns the
/// next `n` bits as a `u32`.
pub struct BitReader<'a> {
    /// Source byte slice.
    data: &'a [u8],
    /// Current read position in `data` (next byte to load into the accumulator).
    pos: usize,
    /// Bit accumulator — bits available for reading start at bit 0.
    buf: u64,
    /// Number of valid bits currently in `buf`.
    bits: u32,
}

impl<'a> BitReader<'a> {
    /// Create a reader over `data`.
    ///
    /// The reader starts at the beginning of the slice with an empty accumulator.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, buf: 0, bits: 0 }
    }

    /// Refill the accumulator by loading bytes from `data` until at least 57
    /// bits are available (or the data is exhausted).
    ///
    /// We refill when bits ≤ 56 so that a subsequent `read_bits(k)` with
    /// k ≤ 56 is always safe.
    fn refill(&mut self) {
        while self.bits <= 56 && self.pos < self.data.len() {
            self.buf |= (self.data[self.pos] as u64) << self.bits;
            self.bits += 8;
            self.pos += 1;
        }
    }

    /// Read the next `count` bits from the stream, returning them as a `u32`.
    ///
    /// The bits are consumed from the accumulator LSB-first.
    ///
    /// # Panics
    ///
    /// Panics if the stream is exhausted before `count` bits can be read.
    pub fn read_bits(&mut self, count: u32) -> u32 {
        self.refill();
        debug_assert!(
            count <= self.bits,
            "BitReader: requested {count} bits but only {} available",
            self.bits
        );
        let mask = if count < 64 { (1u64 << count) - 1 } else { u64::MAX };
        let val = (self.buf & mask) as u32;
        self.buf >>= count;
        self.bits -= count;
        val
    }

    /// Peek at the next `count` bits without consuming them.
    ///
    /// Useful for Huffman decoding lookahead.
    pub fn peek_bits(&mut self, count: u32) -> u32 {
        self.refill();
        let mask = if count < 64 { (1u64 << count) - 1 } else { u64::MAX };
        (self.buf & mask) as u32
    }

    /// Consume `count` bits (must call `peek_bits` first to ensure availability).
    pub fn consume_bits(&mut self, count: u32) {
        self.buf >>= count;
        self.bits -= count;
    }

    /// Return `true` when all data has been read and the accumulator is empty.
    pub fn is_exhausted(&self) -> bool {
        self.bits == 0 && self.pos >= self.data.len()
    }

    /// Number of bits remaining (including buffered bits and bytes not yet loaded).
    pub fn bits_remaining(&self) -> usize {
        self.bits as usize + (self.data.len() - self.pos) * 8
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_single_byte() {
        let mut w = BitWriter::new();
        w.write_bits(0xAB, 8);
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read_bits(8), 0xAB);
    }

    #[test]
    fn round_trip_partial_bits() {
        let mut w = BitWriter::new();
        w.write_bits(0b101, 3);
        w.write_bits(0b11001, 5);
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read_bits(3), 0b101);
        assert_eq!(r.read_bits(5), 0b11001);
    }

    #[test]
    fn round_trip_multi_byte() {
        let mut w = BitWriter::new();
        for i in 0u64..16 {
            w.write_bits(i, 4);
        }
        let bytes = w.finish();
        assert_eq!(bytes.len(), 8); // 16 * 4 bits = 8 bytes
        let mut r = BitReader::new(&bytes);
        for i in 0u64..16 {
            assert_eq!(r.read_bits(4), i as u32);
        }
    }

    #[test]
    fn peek_does_not_consume() {
        let mut w = BitWriter::new();
        w.write_bits(0b1100_1010, 8);
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        let peeked = r.peek_bits(4);
        assert_eq!(peeked, 0b1010); // low 4 bits
        let read_val = r.read_bits(4);
        assert_eq!(read_val, 0b1010); // same bits still there
    }

    #[test]
    fn is_exhausted_after_full_read() {
        let mut w = BitWriter::new();
        w.write_bits(0xFF, 8);
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        r.read_bits(8);
        assert!(r.is_exhausted());
    }
}
