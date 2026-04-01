//! # deflate — Zero-dependency deflate/zlib compression
//!
//! This crate implements RFC 1951 (DEFLATE compressed data format) and
//! RFC 1950 (ZLIB compressed data format) with zero external dependencies.
//!
//! ## How deflate compression works
//!
//! Deflate combines two techniques:
//!
//! 1. **LZ77** — finds repeated byte sequences and replaces them with
//!    back-references: "copy N bytes from M positions back."  This is
//!    the core of the compression — it turns redundant data into short
//!    (length, distance) pairs.
//!
//! 2. **Huffman coding** — encodes the LZ77 output (literals and
//!    length/distance codes) using variable-length bit codes.  Frequent
//!    symbols get shorter codes, rare symbols get longer codes.
//!
//!    We use **fixed Huffman codes** (pre-defined by the deflate spec,
//!    RFC 1951 §3.2.6) rather than building custom trees.  This avoids
//!    transmitting the code table and simplifies the implementation.
//!    The compression ratio is slightly worse than dynamic codes but
//!    still much better than no compression.
//!
//! ## Bit ordering
//!
//! Deflate packs bits LSB-first within each byte.  This is the opposite
//! of most network protocols (which are MSB-first / big-endian).
//!
//! For example, the 5-bit code `10011` is stored as:
//! ```text
//! Bit position:  7  6  5  4  3  2  1  0
//! Value:         -  -  -  1  1  0  0  1
//! ```
//!
//! The `BitWriter` handles this packing automatically.

pub const VERSION: &str = "0.1.0";

// ---------------------------------------------------------------------------
// BitWriter — packs bits LSB-first into a byte buffer
// ---------------------------------------------------------------------------

/// Writes individual bits into a byte buffer, LSB-first.
///
/// Deflate requires sub-byte bit manipulation because Huffman codes have
/// variable lengths (3–15 bits).  The BitWriter accumulates bits in a
/// 32-bit buffer and flushes complete bytes to the output.
struct BitWriter {
    output: Vec<u8>,
    /// Bit accumulator — holds up to 32 bits waiting to be flushed.
    bits: u32,
    /// Number of valid bits in the accumulator (0–32).
    count: u8,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            output: Vec::new(),
            bits: 0,
            count: 0,
        }
    }

    /// Write `n` bits from `value` (LSB-first, n <= 16).
    fn write_bits(&mut self, value: u32, n: u8) {
        self.bits |= value << self.count;
        self.count += n;
        while self.count >= 8 {
            self.output.push(self.bits as u8);
            self.bits >>= 8;
            self.count -= 8;
        }
    }

    /// Write bits in MSB-first order (used for Huffman codes, which are
    /// stored reversed compared to other deflate fields).
    fn write_bits_reversed(&mut self, value: u32, n: u8) {
        let mut reversed = 0u32;
        for i in 0..n {
            if value & (1 << i) != 0 {
                reversed |= 1 << (n - 1 - i);
            }
        }
        self.write_bits(reversed, n);
    }

    /// Flush any remaining bits (pad with zeros to byte boundary).
    fn finish(mut self) -> Vec<u8> {
        if self.count > 0 {
            self.output.push(self.bits as u8);
        }
        self.output
    }
}

// ---------------------------------------------------------------------------
// Fixed Huffman tables (RFC 1951 §3.2.6)
// ---------------------------------------------------------------------------
//
// The fixed Huffman code assigns bit lengths to literal/length codes:
//
//   Value range     Bits    Code range
//   0–143           8       00110000 – 10111111
//   144–255         9       110010000 – 111111111
//   256–279         7       0000000 – 0010111
//   280–287         8       11000000 – 11000111
//
// The actual Huffman codes are computed from these lengths.

/// Get the fixed Huffman code and bit length for a literal/length value.
///
/// Returns (code, bit_length) where `code` is stored MSB-first.
fn fixed_huffman_code(value: u16) -> (u32, u8) {
    match value {
        // 0–143: 8-bit codes starting at 0b00110000
        0..=143 => (0x30 + value as u32, 8),
        // 144–255: 9-bit codes starting at 0b110010000
        144..=255 => (0x190 + (value - 144) as u32, 9),
        // 256–279: 7-bit codes starting at 0b0000000
        256..=279 => (value as u32 - 256, 7),
        // 280–287: 8-bit codes starting at 0b11000000
        280..=287 => (0xC0 + (value - 280) as u32, 8),
        _ => panic!("invalid literal/length value: {}", value),
    }
}

// ---------------------------------------------------------------------------
// Length and distance encoding (RFC 1951 §3.2.5)
// ---------------------------------------------------------------------------
//
// LZ77 matches are encoded as a length code (257–285) plus extra bits,
// followed by a distance code (0–29) plus extra bits.

/// Encode a match length (3–258) as a deflate length code + extra bits.
///
/// Returns (code, extra_bits_value, extra_bits_count).
fn encode_length(length: u16) -> (u16, u32, u8) {
    match length {
        3..=10 => (257 + length - 3, 0, 0),
        11..=18 => {
            let base = (length - 11) / 2;
            let extra = (length - 11) % 2;
            (265 + base, extra as u32, 1)
        }
        19..=34 => {
            let base = (length - 19) / 4;
            let extra = (length - 19) % 4;
            (269 + base, extra as u32, 2)
        }
        35..=66 => {
            let base = (length - 35) / 8;
            let extra = (length - 35) % 8;
            (273 + base, extra as u32, 3)
        }
        67..=130 => {
            let base = (length - 67) / 16;
            let extra = (length - 67) % 16;
            (277 + base, extra as u32, 4)
        }
        131..=257 => {
            let base = (length - 131) / 32;
            let extra = (length - 131) % 32;
            (281 + base, extra as u32, 5)
        }
        258 => (285, 0, 0),
        _ => panic!("invalid match length: {}", length),
    }
}

/// Encode a match distance (1–32768) as a deflate distance code + extra bits.
///
/// Returns (code, extra_bits_value, extra_bits_count).
/// Distance codes use fixed 5-bit codes (no Huffman — just plain 5-bit values).
fn encode_distance(distance: u16) -> (u16, u32, u8) {
    match distance {
        1..=4 => (distance - 1, 0, 0),
        5..=8 => {
            let base = (distance - 5) / 2;
            let extra = (distance - 5) % 2;
            (4 + base, extra as u32, 1)
        }
        9..=16 => {
            let base = (distance - 9) / 4;
            let extra = (distance - 9) % 4;
            (6 + base, extra as u32, 2)
        }
        17..=32 => {
            let base = (distance - 17) / 8;
            let extra = (distance - 17) % 8;
            (8 + base, extra as u32, 3)
        }
        33..=64 => {
            let base = (distance - 33) / 16;
            let extra = (distance - 33) % 16;
            (10 + base, extra as u32, 4)
        }
        65..=128 => {
            let base = (distance - 65) / 32;
            let extra = (distance - 65) % 32;
            (12 + base, extra as u32, 5)
        }
        129..=256 => {
            let base = (distance - 129) / 64;
            let extra = (distance - 129) % 64;
            (14 + base, extra as u32, 6)
        }
        257..=512 => {
            let base = (distance - 257) / 128;
            let extra = (distance - 257) % 128;
            (16 + base, extra as u32, 7)
        }
        513..=1024 => {
            let base = (distance - 513) / 256;
            let extra = (distance - 513) % 256;
            (18 + base, extra as u32, 8)
        }
        1025..=2048 => {
            let base = (distance - 1025) / 512;
            let extra = (distance - 1025) % 512;
            (20 + base, extra as u32, 9)
        }
        2049..=4096 => {
            let base = (distance - 2049) / 1024;
            let extra = (distance - 2049) % 1024;
            (22 + base, extra as u32, 10)
        }
        4097..=8192 => {
            let base = (distance - 4097) / 2048;
            let extra = (distance - 4097) % 2048;
            (24 + base, extra as u32, 11)
        }
        8193..=16384 => {
            let base = (distance - 8193) / 4096;
            let extra = (distance - 8193) % 4096;
            (26 + base, extra as u32, 12)
        }
        16385..=32768 => {
            let base = (distance - 16385) / 8192;
            let extra = (distance - 16385) % 8192;
            (28 + base, extra as u32, 13)
        }
        _ => panic!("invalid match distance: {}", distance),
    }
}

// ---------------------------------------------------------------------------
// LZ77 matching
// ---------------------------------------------------------------------------
//
// LZ77 scans the input looking for repeated byte sequences.  When it
// finds one, it emits a (length, distance) back-reference instead of
// the literal bytes.  The "sliding window" is the previous 32KB of output.

/// Simple hash function for 3-byte sequences.
/// Maps 3 bytes to a 15-bit hash for the hash table.
fn hash3(a: u8, b: u8, c: u8) -> usize {
    let h = (a as usize) << 10 ^ (b as usize) << 5 ^ (c as usize);
    h & 0x7FFF // 15-bit hash → 32768 entries
}

/// Find the longest match at the current position in the input.
///
/// Searches backward through the hash chain for positions that share
/// the same 3-byte hash.  Returns (length, distance) of the best match,
/// or (0, 0) if no match of length >= 3 is found.
fn find_match(
    data: &[u8],
    pos: usize,
    head: &[u16],
    prev: &[u16],
) -> (u16, u16) {
    if pos + 2 >= data.len() {
        return (0, 0);
    }

    let h = hash3(data[pos], data[pos + 1], data[pos + 2]);
    let mut chain = head[h];
    let mut best_len: u16 = 0;
    let mut best_dist: u16 = 0;
    let max_chain = 64; // limit chain walks to avoid O(n²)

    for _ in 0..max_chain {
        let candidate = chain as usize;
        if candidate == 0 || pos - candidate > 32768 {
            break;
        }

        // Compare bytes at candidate vs pos
        let max_len = std::cmp::min(258, data.len() - pos) as u16;
        let mut len: u16 = 0;
        while len < max_len && data[candidate + len as usize] == data[pos + len as usize] {
            len += 1;
        }

        if len > best_len && len >= 3 {
            best_len = len;
            best_dist = (pos - candidate) as u16;
            if len == max_len {
                break; // can't do better
            }
        }

        let next = prev[candidate & 0x7FFF];
        if next == chain || next as usize >= candidate {
            break;
        }
        chain = next;
    }

    (best_len, best_dist)
}

// ---------------------------------------------------------------------------
// Deflate compression (RFC 1951)
// ---------------------------------------------------------------------------

/// Compress data using the DEFLATE algorithm (RFC 1951).
///
/// Returns raw deflate-compressed bytes (no zlib/gzip wrapper).
/// Uses fixed Huffman codes with LZ77 matching.
pub fn deflate_compress(data: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter::new();

    // Block header: BFINAL=1 (last block), BTYPE=01 (fixed Huffman)
    writer.write_bits(1, 1); // BFINAL
    writer.write_bits(1, 2); // BTYPE = 01 (fixed Huffman)

    if data.is_empty() {
        // Emit end-of-block marker (code 256)
        let (code, bits) = fixed_huffman_code(256);
        writer.write_bits_reversed(code, bits);
        return writer.finish();
    }

    // Hash table for LZ77 matching
    // head[hash] = most recent position with this hash
    // prev[pos % 32768] = previous position with the same hash (chain)
    let mut head = vec![0u16; 32768];
    let mut prev = vec![0u16; 32768];

    let mut pos = 0;
    while pos < data.len() {
        let (match_len, match_dist) = find_match(data, pos, &head, &prev);

        if match_len >= 3 {
            // Emit length code
            let (len_code, len_extra, len_extra_bits) = encode_length(match_len);
            let (huff_code, huff_bits) = fixed_huffman_code(len_code);
            writer.write_bits_reversed(huff_code, huff_bits);
            if len_extra_bits > 0 {
                writer.write_bits(len_extra, len_extra_bits);
            }

            // Emit distance code (fixed 5-bit codes, reversed)
            let (dist_code, dist_extra, dist_extra_bits) = encode_distance(match_dist);
            writer.write_bits_reversed(dist_code as u32, 5);
            if dist_extra_bits > 0 {
                writer.write_bits(dist_extra, dist_extra_bits);
            }

            // Update hash table for all positions in the match
            for i in 0..match_len as usize {
                if pos + i + 2 < data.len() {
                    let h = hash3(data[pos + i], data[pos + i + 1], data[pos + i + 2]);
                    prev[(pos + i) & 0x7FFF] = head[h];
                    head[h] = (pos + i) as u16;
                }
            }
            pos += match_len as usize;
        } else {
            // Emit literal
            let (code, bits) = fixed_huffman_code(data[pos] as u16);
            writer.write_bits_reversed(code, bits);

            // Update hash table
            if pos + 2 < data.len() {
                let h = hash3(data[pos], data[pos + 1], data[pos + 2]);
                prev[pos & 0x7FFF] = head[h];
                head[h] = pos as u16;
            }
            pos += 1;
        }
    }

    // End-of-block marker (literal/length code 256)
    let (code, bits) = fixed_huffman_code(256);
    writer.write_bits_reversed(code, bits);

    writer.finish()
}

// ---------------------------------------------------------------------------
// Adler-32 checksum (RFC 1950)
// ---------------------------------------------------------------------------
//
// Adler-32 uses two 16-bit sums:
//   s1 = 1 + sum of all bytes (mod 65521)
//   s2 = sum of all s1 values (mod 65521)
//   result = (s2 << 16) | s1
//
// 65521 is the largest prime smaller than 2^16.

/// Compute the Adler-32 checksum of a byte slice.
pub fn adler32(data: &[u8]) -> u32 {
    let mut s1: u32 = 1;
    let mut s2: u32 = 0;

    for &byte in data {
        s1 = (s1 + byte as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }

    (s2 << 16) | s1
}

// ---------------------------------------------------------------------------
// Zlib compression (RFC 1950)
// ---------------------------------------------------------------------------
//
// Zlib is a thin wrapper around deflate that adds:
//   - 2-byte header (CMF + FLG)
//   - Adler-32 checksum at the end
//
// CMF byte: compression method (8 = deflate) + window size info
// FLG byte: check bits + compression level + optional dict flag

/// Compress data using the zlib format (RFC 1950).
///
/// Returns zlib-compressed bytes: [CMF][FLG][deflate data][Adler-32].
/// This is the format PNG's IDAT chunks expect.
pub fn zlib_compress(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();

    // CMF byte: CM=8 (deflate), CINFO=7 (32K window)
    // CINFO=7 means window size = 2^(7+8) = 32768
    let cmf: u8 = 0x78;

    // FLG byte: must satisfy (CMF*256 + FLG) % 31 == 0
    // With CMF=0x78, FLG=0x01 gives 0x7801 % 31 = 0 ✓
    let flg: u8 = 0x01;

    output.push(cmf);
    output.push(flg);

    // Deflate-compressed data
    let compressed = deflate_compress(data);
    output.extend_from_slice(&compressed);

    // Adler-32 checksum of the uncompressed data (big-endian)
    let checksum = adler32(data);
    output.push((checksum >> 24) as u8);
    output.push((checksum >> 16) as u8);
    output.push((checksum >> 8) as u8);
    output.push(checksum as u8);

    output
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn adler32_empty() {
        assert_eq!(adler32(b""), 1);
    }

    #[test]
    fn adler32_known_values() {
        // Wikipedia test vector: adler32("Wikipedia") = 0x11E60398
        assert_eq!(adler32(b"Wikipedia"), 0x11E60398);
    }

    #[test]
    fn deflate_empty_produces_valid_output() {
        let compressed = deflate_compress(b"");
        // Should have at least the block header and end-of-block marker
        assert!(!compressed.is_empty());
    }

    #[test]
    fn zlib_header_is_valid() {
        let compressed = zlib_compress(b"test");
        // CMF=0x78, FLG=0x01
        assert_eq!(compressed[0], 0x78);
        assert_eq!(compressed[1], 0x01);
        // (CMF*256 + FLG) % 31 == 0
        assert_eq!((0x78u32 * 256 + 0x01) % 31, 0);
    }

    #[test]
    fn zlib_checksum_is_correct() {
        let data = b"Hello, World!";
        let compressed = zlib_compress(data);
        let len = compressed.len();
        // Last 4 bytes are Adler-32 (big-endian)
        let stored_checksum = u32::from_be_bytes([
            compressed[len - 4],
            compressed[len - 3],
            compressed[len - 2],
            compressed[len - 1],
        ]);
        assert_eq!(stored_checksum, adler32(data));
    }

    /// Verify compression of repetitive data produces smaller output.
    #[test]
    fn repetitive_data_compresses() {
        let data = vec![0xAA; 10000]; // 10KB of repeated bytes
        let compressed = zlib_compress(&data);
        assert!(
            compressed.len() < data.len() / 2,
            "10KB of repeated bytes should compress to less than 5KB, got {} bytes",
            compressed.len()
        );
    }

    /// Verify the fixed Huffman codes for some known values.
    #[test]
    fn fixed_huffman_code_boundaries() {
        // Code 0 should be 8 bits (0x30)
        assert_eq!(fixed_huffman_code(0), (0x30, 8));
        // Code 143 should be 8 bits (0xBF)
        assert_eq!(fixed_huffman_code(143), (0xBF, 8));
        // Code 144 should be 9 bits (0x190)
        assert_eq!(fixed_huffman_code(144), (0x190, 9));
        // Code 256 (end of block) should be 7 bits (0)
        assert_eq!(fixed_huffman_code(256), (0, 7));
    }
}
