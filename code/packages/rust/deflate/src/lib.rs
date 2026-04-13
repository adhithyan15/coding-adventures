//! deflate — CMP05: DEFLATE lossless compression algorithm (1996).
//!
//! DEFLATE is the dominant general-purpose lossless compression algorithm,
//! powering ZIP, gzip, PNG, and HTTP/2 HPACK header compression. It combines:
//!
//! 1. **LZSS tokenization** (CMP02) — replace repeated substrings with
//!    back-references into a 4096-byte sliding window. Each back-reference
//!    is a (offset, length) pair where offset is 1–4096 and length is 3–255.
//!
//! 2. **Dual canonical Huffman coding** (DT27/CMP04) — entropy-code the
//!    resulting token stream with TWO separate Huffman trees:
//!    - LL tree: literals (0–255), end-of-data (256), length codes (257–284)
//!    - Dist tree: distance codes (0–23, for offsets 1–4096)
//!
//! # The Expanded LL Alphabet
//!
//! DEFLATE merges literal bytes and match lengths into one alphabet:
//!
//! ```text
//! Symbols 0–255:   literal byte values
//! Symbol  256:     end-of-data marker
//! Symbols 257–284: length codes (each covers a range via extra bits)
//! ```
//!
//! # Wire Format (CMP05)
//!
//! ```text
//! [4B] original_length    big-endian uint32
//! [2B] ll_entry_count     big-endian uint16
//! [2B] dist_entry_count   big-endian uint16 (0 if no matches)
//! [ll_entry_count × 3B]   (symbol uint16 BE, code_length uint8)
//! [dist_entry_count × 3B] same format
//! [remaining bytes]       LSB-first packed bit stream
//! ```
//!
//! # Series
//!
//! ```text
//! CMP00 (LZ77,    1977) — Sliding-window backreferences.
//! CMP01 (LZ78,    1978) — Explicit dictionary (trie).
//! CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
//! CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; GIF.
//! CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//! CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.  (this crate)
//! ```

use std::collections::HashMap;

use huffman_tree::HuffmanTree;
use lzss::Token;

// ---------------------------------------------------------------------------
// Length code table (LL symbols 257–284)
// ---------------------------------------------------------------------------
//
// Each length symbol covers a range of match lengths (3–255). The exact length
// within the range is encoded as `extra_bits` raw bits after the Huffman code.
//
// Example: length=13 → symbol 266 (base=13, extra=1, extra_value=0 → bit "0")
//          length=14 → symbol 266 (base=13, extra=1, extra_value=1 → bit "1")

struct LengthEntry {
    symbol: u16,
    base: u32,
    extra_bits: u32,
}

const LENGTH_TABLE: &[LengthEntry] = &[
    LengthEntry { symbol: 257, base:   3, extra_bits: 0 },
    LengthEntry { symbol: 258, base:   4, extra_bits: 0 },
    LengthEntry { symbol: 259, base:   5, extra_bits: 0 },
    LengthEntry { symbol: 260, base:   6, extra_bits: 0 },
    LengthEntry { symbol: 261, base:   7, extra_bits: 0 },
    LengthEntry { symbol: 262, base:   8, extra_bits: 0 },
    LengthEntry { symbol: 263, base:   9, extra_bits: 0 },
    LengthEntry { symbol: 264, base:  10, extra_bits: 0 },
    LengthEntry { symbol: 265, base:  11, extra_bits: 1 },
    LengthEntry { symbol: 266, base:  13, extra_bits: 1 },
    LengthEntry { symbol: 267, base:  15, extra_bits: 1 },
    LengthEntry { symbol: 268, base:  17, extra_bits: 1 },
    LengthEntry { symbol: 269, base:  19, extra_bits: 2 },
    LengthEntry { symbol: 270, base:  23, extra_bits: 2 },
    LengthEntry { symbol: 271, base:  27, extra_bits: 2 },
    LengthEntry { symbol: 272, base:  31, extra_bits: 2 },
    LengthEntry { symbol: 273, base:  35, extra_bits: 3 },
    LengthEntry { symbol: 274, base:  43, extra_bits: 3 },
    LengthEntry { symbol: 275, base:  51, extra_bits: 3 },
    LengthEntry { symbol: 276, base:  59, extra_bits: 3 },
    LengthEntry { symbol: 277, base:  67, extra_bits: 4 },
    LengthEntry { symbol: 278, base:  83, extra_bits: 4 },
    LengthEntry { symbol: 279, base:  99, extra_bits: 4 },
    LengthEntry { symbol: 280, base: 115, extra_bits: 4 },
    LengthEntry { symbol: 281, base: 131, extra_bits: 5 },
    LengthEntry { symbol: 282, base: 163, extra_bits: 5 },
    LengthEntry { symbol: 283, base: 195, extra_bits: 5 },
    LengthEntry { symbol: 284, base: 227, extra_bits: 5 },
];

// ---------------------------------------------------------------------------
// Distance code table (codes 0–23)
// ---------------------------------------------------------------------------

struct DistEntry {
    code: u16,
    base: u32,
    extra_bits: u32,
}

const DIST_TABLE: &[DistEntry] = &[
    DistEntry { code:  0, base:    1, extra_bits:  0 },
    DistEntry { code:  1, base:    2, extra_bits:  0 },
    DistEntry { code:  2, base:    3, extra_bits:  0 },
    DistEntry { code:  3, base:    4, extra_bits:  0 },
    DistEntry { code:  4, base:    5, extra_bits:  1 },
    DistEntry { code:  5, base:    7, extra_bits:  1 },
    DistEntry { code:  6, base:    9, extra_bits:  2 },
    DistEntry { code:  7, base:   13, extra_bits:  2 },
    DistEntry { code:  8, base:   17, extra_bits:  3 },
    DistEntry { code:  9, base:   25, extra_bits:  3 },
    DistEntry { code: 10, base:   33, extra_bits:  4 },
    DistEntry { code: 11, base:   49, extra_bits:  4 },
    DistEntry { code: 12, base:   65, extra_bits:  5 },
    DistEntry { code: 13, base:   97, extra_bits:  5 },
    DistEntry { code: 14, base:  129, extra_bits:  6 },
    DistEntry { code: 15, base:  193, extra_bits:  6 },
    DistEntry { code: 16, base:  257, extra_bits:  7 },
    DistEntry { code: 17, base:  385, extra_bits:  7 },
    DistEntry { code: 18, base:  513, extra_bits:  8 },
    DistEntry { code: 19, base:  769, extra_bits:  8 },
    DistEntry { code: 20, base: 1025, extra_bits:  9 },
    DistEntry { code: 21, base: 1537, extra_bits:  9 },
    DistEntry { code: 22, base: 2049, extra_bits: 10 },
    DistEntry { code: 23, base: 3073, extra_bits: 10 },
];

// ---------------------------------------------------------------------------
// Length / distance symbol lookup helpers
// ---------------------------------------------------------------------------

fn length_symbol(length: u32) -> u16 {
    for e in LENGTH_TABLE {
        let max_len = e.base + (1 << e.extra_bits) - 1;
        if length <= max_len {
            return e.symbol;
        }
    }
    284
}

fn dist_code_for(offset: u32) -> u16 {
    for e in DIST_TABLE {
        let max_dist = e.base + (1 << e.extra_bits) - 1;
        if offset <= max_dist {
            return e.code;
        }
    }
    23
}

fn length_base(sym: u16) -> u32 {
    LENGTH_TABLE.iter().find(|e| e.symbol == sym).map(|e| e.base).unwrap_or(0)
}

fn length_extra(sym: u16) -> u32 {
    LENGTH_TABLE.iter().find(|e| e.symbol == sym).map(|e| e.extra_bits).unwrap_or(0)
}

fn dist_base(code: u16) -> u32 {
    DIST_TABLE.iter().find(|e| e.code == code).map(|e| e.base).unwrap_or(0)
}

fn dist_extra(code: u16) -> u32 {
    DIST_TABLE.iter().find(|e| e.code == code).map(|e| e.extra_bits).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Bit I/O
// ---------------------------------------------------------------------------

/// Accumulates bits into a byte buffer, LSB-first.
///
/// "LSB-first" means: the first bit written occupies bit 0 (the
/// least-significant bit) of the first byte. Bits fill each byte from low
/// to high before moving to the next byte.
struct BitBuilder {
    buf: u64,
    bit_pos: u32,
    out: Vec<u8>,
}

impl BitBuilder {
    fn new() -> Self {
        Self { buf: 0, bit_pos: 0, out: Vec::new() }
    }

    /// Write a bit string (e.g. "1010") LSB-first.
    fn write_bit_string(&mut self, s: &str) {
        for ch in s.chars() {
            if ch == '1' {
                self.buf |= 1u64 << self.bit_pos;
            }
            self.bit_pos += 1;
            if self.bit_pos == 64 {
                for _ in 0..8 {
                    self.out.push((self.buf & 0xFF) as u8);
                    self.buf >>= 8;
                }
                self.bit_pos = 0;
            }
        }
    }

    /// Write `n` raw bits from `val`, LSB of val first.
    fn write_raw_bits_lsb(&mut self, val: u32, n: u32) {
        for i in 0..n {
            if (val >> i) & 1 == 1 {
                self.buf |= 1u64 << self.bit_pos;
            }
            self.bit_pos += 1;
            if self.bit_pos == 64 {
                for _ in 0..8 {
                    self.out.push((self.buf & 0xFF) as u8);
                    self.buf >>= 8;
                }
                self.bit_pos = 0;
            }
        }
    }

    fn flush(&mut self) {
        while self.bit_pos > 0 {
            self.out.push((self.buf & 0xFF) as u8);
            self.buf >>= 8;
            if self.bit_pos >= 8 {
                self.bit_pos -= 8;
            } else {
                self.bit_pos = 0;
            }
        }
    }

    fn finish(mut self) -> Vec<u8> {
        self.flush();
        self.out
    }
}

fn unpack_bits(data: &[u8]) -> Vec<u8> {
    // Returns a vector of 0/1 bytes.
    let mut bits = Vec::with_capacity(data.len() * 8);
    for &byte in data {
        for i in 0..8 {
            bits.push((byte >> i) & 1);
        }
    }
    bits
}

// ---------------------------------------------------------------------------
// Canonical code reconstruction
// ---------------------------------------------------------------------------

fn build_canonical_codes(pairs: &[(u16, usize)]) -> HashMap<u16, String> {
    let mut result = HashMap::new();
    if pairs.is_empty() {
        return result;
    }
    if pairs.len() == 1 {
        result.insert(pairs[0].0, "0".to_string());
        return result;
    }
    let mut code: u32 = 0;
    let mut prev_len = pairs[0].1;
    for &(symbol, code_len) in pairs {
        if code_len > prev_len {
            code <<= code_len - prev_len;
        }
        let bit_str = format!("{:0>width$b}", code, width = code_len);
        result.insert(symbol, bit_str);
        code += 1;
        prev_len = code_len;
    }
    result
}

fn reverse_code_map(m: &HashMap<u16, String>) -> HashMap<String, u16> {
    m.iter().map(|(&sym, bits)| (bits.clone(), sym)).collect()
}

// ---------------------------------------------------------------------------
// Public API: compress
// ---------------------------------------------------------------------------

/// Compress `data` using DEFLATE (CMP05) and return wire-format bytes.
///
/// Two-pass algorithm:
/// 1. LZSS tokenization (window=4096, max_match=255, min_match=3).
/// 2. Dual canonical Huffman coding (LL tree + dist tree).
///
/// Returns `Err(String)` if the underlying tree build fails.
pub fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
    let original_length = data.len();

    if original_length == 0 {
        // Empty input: LL tree has only symbol 256 (end-of-data), code "0".
        let mut out = Vec::with_capacity(12);
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes()); // ll_entry_count = 1
        out.extend_from_slice(&0u16.to_be_bytes()); // dist_entry_count = 0
        out.extend_from_slice(&256u16.to_be_bytes()); // symbol = 256
        out.push(1u8); // code_length = 1
        out.push(0x00); // bit stream: "0" → 0x00
        return Ok(out);
    }

    // ── Pass 1: LZSS tokenization ────────────────────────────────────────────
    let tokens = lzss::encode(data, 4096, 255, 3);

    // ── Pass 2a: Tally frequencies ───────────────────────────────────────────
    let mut ll_freq: HashMap<u16, u32> = HashMap::new();
    let mut dist_freq: HashMap<u16, u32> = HashMap::new();

    for tok in &tokens {
        match tok {
            Token::Literal(b) => {
                *ll_freq.entry(*b as u16).or_insert(0) += 1;
            }
            Token::Match { offset, length } => {
                let sym = length_symbol(*length as u32);
                *ll_freq.entry(sym).or_insert(0) += 1;
                let dc = dist_code_for(*offset as u32);
                *dist_freq.entry(dc).or_insert(0) += 1;
            }
        }
    }
    *ll_freq.entry(256).or_insert(0) += 1;

    // ── Pass 2b: Build canonical Huffman trees ───────────────────────────────
    let ll_weights: Vec<(u16, u32)> = ll_freq.iter().map(|(&sym, &freq)| (sym, freq)).collect();
    let ll_tree = HuffmanTree::build(&ll_weights)?;
    let ll_code_table = ll_tree.canonical_code_table(); // HashMap<u16, String>

    let mut dist_code_table: HashMap<u16, String> = HashMap::new();
    if !dist_freq.is_empty() {
        let dist_weights: Vec<(u16, u32)> = dist_freq.iter().map(|(&sym, &freq)| (sym, freq)).collect();
        let dist_tree = HuffmanTree::build(&dist_weights)?;
        dist_code_table = dist_tree.canonical_code_table();
    }

    // ── Pass 2c: Encode token stream ─────────────────────────────────────────
    let mut bb = BitBuilder::new();
    for tok in &tokens {
        match tok {
            Token::Literal(b) => {
                let code = ll_code_table.get(&(*b as u16))
                    .ok_or_else(|| format!("no LL code for literal {}", b))?;
                bb.write_bit_string(code);
            }
            Token::Match { offset, length } => {
                let sym = length_symbol(*length as u32);
                let code = ll_code_table.get(&sym)
                    .ok_or_else(|| format!("no LL code for length symbol {}", sym))?;
                bb.write_bit_string(code);
                let extra = length_extra(sym);
                let extra_val = (*length as u32) - length_base(sym);
                bb.write_raw_bits_lsb(extra_val, extra);

                let dc = dist_code_for(*offset as u32);
                let dcode = dist_code_table.get(&dc)
                    .ok_or_else(|| format!("no dist code for code {}", dc))?;
                bb.write_bit_string(dcode);
                let dextra = dist_extra(dc);
                let dextra_val = (*offset as u32) - dist_base(dc);
                bb.write_raw_bits_lsb(dextra_val, dextra);
            }
        }
    }
    let eod_code = ll_code_table.get(&256)
        .ok_or("no LL code for end-of-data symbol 256")?;
    bb.write_bit_string(eod_code);
    let packed_bits = bb.finish();

    // ── Assemble wire format ─────────────────────────────────────────────────
    let mut ll_pairs: Vec<(u16, usize)> = ll_code_table.iter()
        .map(|(&sym, code)| (sym, code.len()))
        .collect();
    ll_pairs.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));

    let mut dist_pairs: Vec<(u16, usize)> = dist_code_table.iter()
        .map(|(&sym, code)| (sym, code.len()))
        .collect();
    dist_pairs.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));

    let mut out = Vec::with_capacity(
        8 + 3 * ll_pairs.len() + 3 * dist_pairs.len() + packed_bits.len()
    );
    out.extend_from_slice(&(original_length as u32).to_be_bytes());
    out.extend_from_slice(&(ll_pairs.len() as u16).to_be_bytes());
    out.extend_from_slice(&(dist_pairs.len() as u16).to_be_bytes());

    for (sym, len) in &ll_pairs {
        out.extend_from_slice(&sym.to_be_bytes());
        out.push(*len as u8);
    }
    for (sym, len) in &dist_pairs {
        out.extend_from_slice(&sym.to_be_bytes());
        out.push(*len as u8);
    }
    out.extend_from_slice(&packed_bits);

    Ok(out)
}

// ---------------------------------------------------------------------------
// Public API: decompress
// ---------------------------------------------------------------------------

/// Decompress CMP05 wire-format `data` and return the original bytes.
///
/// Stops decoding at the end-of-data symbol (256). Copies are done byte-by-byte
/// to correctly handle overlapping matches (where offset < length), which encode
/// run-length sequences.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 8 {
        return Err(format!("deflate: data too short: {} bytes", data.len()));
    }

    let original_length = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let ll_entry_count = u16::from_be_bytes([data[4], data[5]]) as usize;
    let dist_entry_count = u16::from_be_bytes([data[6], data[7]]) as usize;

    if original_length == 0 {
        return Ok(Vec::new());
    }

    let mut off = 8usize;

    // Parse LL code-length table.
    let mut ll_lengths: Vec<(u16, usize)> = Vec::with_capacity(ll_entry_count);
    for _ in 0..ll_entry_count {
        let sym = u16::from_be_bytes([data[off], data[off + 1]]);
        let clen = data[off + 2] as usize;
        ll_lengths.push((sym, clen));
        off += 3;
    }

    // Parse dist code-length table.
    let mut dist_lengths: Vec<(u16, usize)> = Vec::with_capacity(dist_entry_count);
    for _ in 0..dist_entry_count {
        let sym = u16::from_be_bytes([data[off], data[off + 1]]);
        let clen = data[off + 2] as usize;
        dist_lengths.push((sym, clen));
        off += 3;
    }

    // Reconstruct canonical codes.
    let ll_code_map = build_canonical_codes(&ll_lengths);
    let dist_code_map = build_canonical_codes(&dist_lengths);
    let ll_rev_map = reverse_code_map(&ll_code_map);
    let dist_rev_map = reverse_code_map(&dist_code_map);

    // Unpack bit stream.
    let bits = unpack_bits(&data[off..]);
    let mut bit_pos = 0usize;

    let read_bits = |bits: &[u8], pos: &mut usize, n: u32| -> u32 {
        let mut val = 0u32;
        for i in 0..n {
            val |= (bits[*pos] as u32) << i;
            *pos += 1;
        }
        val
    };

    let next_huffman_symbol = |bits: &[u8], pos: &mut usize, rev_map: &HashMap<String, u16>| -> Result<u16, String> {
        let mut acc = String::new();
        loop {
            if *pos >= bits.len() {
                return Err("deflate: bit stream exhausted".to_string());
            }
            acc.push(if bits[*pos] == 1 { '1' } else { '0' });
            *pos += 1;
            if let Some(&sym) = rev_map.get(&acc) {
                return Ok(sym);
            }
        }
    };

    // Decode token stream.
    let mut output: Vec<u8> = Vec::with_capacity(original_length);
    loop {
        let ll_sym = next_huffman_symbol(&bits, &mut bit_pos, &ll_rev_map)?;

        if ll_sym == 256 {
            break; // end-of-data
        } else if ll_sym < 256 {
            output.push(ll_sym as u8);
        } else {
            // Length code 257–284.
            let extra = length_extra(ll_sym);
            let length = length_base(ll_sym) + read_bits(&bits, &mut bit_pos, extra);

            let dist_sym = next_huffman_symbol(&bits, &mut bit_pos, &dist_rev_map)?;
            let dextra = dist_extra(dist_sym);
            let dist_offset = dist_base(dist_sym) + read_bits(&bits, &mut bit_pos, dextra);

            // Copy byte-by-byte (supports overlapping matches).
            let start = output.len() - dist_offset as usize;
            for i in 0..length as usize {
                let b = output[start + i];
                output.push(b);
            }
        }
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// zlib compatibility shim
// ---------------------------------------------------------------------------
//
// The rust/png package (and other packages) depend on `deflate::zlib_compress`,
// which was part of the original zero-dependency deflate implementation.
// We provide it here as a stored-block DEFLATE stream wrapped in a zlib envelope
// (RFC 1950). Stored blocks (BTYPE=00) are always RFC 1951-compatible and require
// no Huffman coding — the data is copied verbatim with a minimal block header.
//
// zlib envelope:
//   [CMF=0x78][FLG=0x9C]   — deflate method, default compression
//   [DEFLATE data]          — one or more stored blocks
//   [Adler-32 checksum BE]  — integrity check over the uncompressed data

/// Compute Adler-32 checksum (RFC 1950 §2.2).
pub fn adler32(data: &[u8]) -> u32 {
    const MOD_ADLER: u32 = 65521;
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + byte as u32) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }
    (b << 16) | a
}

/// Compress `data` into a raw (no-header) DEFLATE stream using stored blocks.
///
/// Stored blocks have BTYPE=00 and copy data verbatim; every standard DEFLATE
/// decompressor (zlib, zstd, etc.) handles them. Blocks are limited to 65535
/// bytes each per RFC 1951 §3.2.4.
fn deflate_compress_stored(data: &[u8]) -> Vec<u8> {
    // Each stored block: [BFINAL+BTYPE byte][LEN 2B LE][NLEN 2B LE][data]
    // BTYPE=00, BFINAL=1 only for the last block.
    let mut out = Vec::new();

    if data.is_empty() {
        // Empty stored block: BFINAL=1, BTYPE=00, LEN=0, NLEN=0xFFFF.
        out.extend_from_slice(&[0x01, 0x00, 0x00, 0xFF, 0xFF]);
        return out;
    }

    let chunks: Vec<&[u8]> = data.chunks(65535).collect();
    let n = chunks.len();
    for (i, chunk) in chunks.iter().enumerate() {
        let bfinal: u8 = if i + 1 == n { 1 } else { 0 };
        let btype: u8 = 0; // stored
        // First byte: bits 0=BFINAL, bits 1-2=BTYPE, rest=0.
        out.push(bfinal | (btype << 1));
        let len = chunk.len() as u16;
        let nlen = !len;
        out.push((len & 0xFF) as u8);
        out.push((len >> 8) as u8);
        out.push((nlen & 0xFF) as u8);
        out.push((nlen >> 8) as u8);
        out.extend_from_slice(chunk);
    }
    out
}

/// Compress `data` using the zlib format (RFC 1950).
///
/// Returns: [CMF=0x78][FLG=0x9C][stored DEFLATE blocks][Adler-32 BE 4 bytes].
///
/// This uses stored (non-compressed) DEFLATE blocks, which are always valid
/// per RFC 1951. The output is decompressable by any zlib-compatible library.
///
/// Note: For CMP05 (educational) wire-format compression, use `compress` instead.
pub fn zlib_compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    // zlib header: CMF=0x78 (deflate, window=32768), FLG=0x9C (no dict, lvl=6).
    // (CMF * 256 + FLG) must be divisible by 31: 0x789C = 30876 = 996 × 31. ✓
    out.extend_from_slice(&[0x78, 0x9C]);
    out.extend_from_slice(&deflate_compress_stored(data));
    let checksum = adler32(data);
    out.push((checksum >> 24) as u8);
    out.push((checksum >> 16) as u8);
    out.push((checksum >> 8) as u8);
    out.push(checksum as u8);
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(data: &[u8]) {
        let compressed = compress(data).expect("compress failed");
        let decompressed = decompress(&compressed).expect("decompress failed");
        assert_eq!(decompressed, data, "roundtrip mismatch for {:?}", &data[..data.len().min(20)]);
    }

    #[test]
    fn test_empty() {
        let compressed = compress(b"").unwrap();
        let result = decompress(&compressed).unwrap();
        assert_eq!(result, b"");
    }

    #[test]
    fn test_single_byte() {
        roundtrip(b"\x00");
        roundtrip(b"\xff");
        roundtrip(b"A");
    }

    #[test]
    fn test_single_byte_repeated() {
        roundtrip(b"AAAAAAAAAAAAAAAAAAA");
        roundtrip(&vec![0u8; 100]);
    }

    #[test]
    fn test_all_literals_aaabbc() {
        let data = b"AAABBC";
        roundtrip(data);
        let compressed = compress(data).unwrap();
        let dist_count = u16::from_be_bytes([compressed[6], compressed[7]]);
        assert_eq!(dist_count, 0, "no matches expected for AAABBC");
    }

    #[test]
    fn test_one_match_aabcbbabc() {
        let data = b"AABCBBABC";
        roundtrip(data);
        let compressed = compress(data).unwrap();
        let orig_len = u32::from_be_bytes([compressed[0], compressed[1], compressed[2], compressed[3]]);
        assert_eq!(orig_len, 9);
        let dist_count = u16::from_be_bytes([compressed[6], compressed[7]]);
        assert!(dist_count > 0, "expected a match in AABCBBABC");
    }

    #[test]
    fn test_overlapping_match() {
        roundtrip(b"AAAAAAA");
        roundtrip(b"ABABABABABAB");
    }

    #[test]
    fn test_multiple_matches() {
        roundtrip(b"ABCABCABCABC");
        roundtrip(b"hello hello hello world");
    }

    #[test]
    fn test_all_bytes() {
        let data: Vec<u8> = (0..=255).collect();
        roundtrip(&data);
    }

    #[test]
    fn test_binary_data() {
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        roundtrip(&data);
    }

    #[test]
    fn test_compression_ratio() {
        let data: Vec<u8> = b"ABCABC".iter().cycle().take(600).copied().collect();
        let compressed = compress(&data).unwrap();
        assert!(
            compressed.len() < data.len() / 2,
            "expected significant compression: {} >= {}/2",
            compressed.len(), data.len()
        );
    }

    #[test]
    fn test_max_match_length() {
        let data = vec![b'A'; 300];
        roundtrip(&data);
    }

    #[test]
    fn test_various_lengths() {
        for &length in &[3usize, 4, 10, 11, 13, 19, 35, 67, 131, 227, 255] {
            let prefix: Vec<u8> = vec![b'A'; length];
            let separator = b"BBB";
            let mut data = prefix.clone();
            data.extend_from_slice(separator);
            data.extend_from_slice(&prefix);
            roundtrip(&data);
        }
    }

    #[test]
    fn test_longer_text() {
        let base = b"the quick brown fox jumps over the lazy dog ";
        let data: Vec<u8> = base.iter().cycle().take(base.len() * 10).copied().collect();
        roundtrip(&data);
    }
}
