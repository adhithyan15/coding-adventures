//! huffman-compression — CMP04: Huffman lossless compression algorithm (1952).
//!
//! Huffman coding is an **entropy coding** algorithm: it assigns variable-length,
//! prefix-free binary codes to symbols based on their frequency of occurrence.
//! Frequent symbols get short codes; rare symbols get long codes. The resulting
//! code is provably optimal — no other prefix-free code can achieve a smaller
//! expected bit-length for the same symbol frequency distribution.
//!
//! Unlike the LZ-family algorithms (CMP00–CMP03) which exploit **repetition**
//! (duplicate substrings), Huffman coding exploits **symbol statistics**. It
//! works on individual symbol frequencies, not patterns of repetition. This
//! makes it complementary to LZ compression and explains why DEFLATE (CMP05)
//! combines both: LZ77 to eliminate repeated substrings, then Huffman to
//! optimally encode the remaining symbol stream.
//!
//! # Dependency on DT27
//!
//! This crate does **not** build its own Huffman tree. It delegates all tree
//! construction and code derivation to [`huffman_tree::HuffmanTree`] (DT27).
//! This mirrors the pattern used by LZ78 (CMP01) which delegates trie operations
//! to the `trie` crate (DT13):
//!
//! ```text
//! CMP01 (LZ78)    →  uses DT13 (Trie)        for dictionary management
//! CMP04 (Huffman) →  uses DT27 (HuffmanTree)  for code construction/decode
//! ```
//!
//! # Wire Format (CMP04)
//!
//! The CMP04 wire format is self-contained: all information needed to reconstruct
//! the original data is embedded in the header.
//!
//! ```text
//! Bytes 0–3:    original_length  (big-endian uint32)
//! Bytes 4–7:    symbol_count     (big-endian uint32) — number of distinct bytes
//! Bytes 8–8+2N: code-lengths table — N entries, each 2 bytes:
//!                 [0] symbol value  (uint8, 0–255)
//!                 [1] code length   (uint8, 1–16)
//!               Sorted by (code_length, symbol_value) ascending.
//! Bytes 8+2N+:  bit stream — packed LSB-first, zero-padded to byte boundary.
//! ```
//!
//! The code-lengths table lets the decompressor reconstruct the **canonical**
//! Huffman codes without transmitting the tree structure — the same trick DEFLATE
//! uses to save space.
//!
//! ## LSB-first bit packing
//!
//! The bit stream uses LSB-first packing (same convention as GIF and LZW/CMP03):
//! the first bit of the Huffman-encoded stream occupies bit 0 (least-significant
//! bit) of the first byte of the payload.
//!
//! ```text
//! Bit string "000101011" (9 bits):
//!   Byte 0: bits[0..7] → 0b10101000 = 0xA8
//!     bit 0 ('0') → byte bit 0
//!     bit 1 ('0') → byte bit 1
//!     bit 2 ('0') → byte bit 2
//!     bit 3 ('1') → byte bit 3
//!     bit 4 ('0') → byte bit 4
//!     bit 5 ('1') → byte bit 5
//!     bit 6 ('0') → byte bit 6
//!     bit 7 ('1') → byte bit 7
//!   Byte 1: bit[8] ('1') → 0b00000001 = 0x01
//! ```
//!
//! # Canonical Code Reconstruction
//!
//! Given a sorted list of `(symbol, code_length)` pairs, canonical codes are
//! reconstructed deterministically (DEFLATE-style):
//!
//! ```text
//! code = 0
//! prev_len = lengths[0].1
//! for (sym, len) in &lengths:
//!     if len > prev_len: code <<= (len - prev_len)
//!     bit_string = zero-padded binary of code to `len` digits
//!     code += 1
//!     prev_len = len
//! ```
//!
//! # Series
//!
//! ```text
//! CMP00 (LZ77,    1977) — Sliding-window backreferences.
//! CMP01 (LZ78,    1978) — Explicit dictionary (trie), no sliding window.
//! CMP02 (LZSS,    1982) — LZ77 + flag bits; eliminates wasted literals.
//! CMP03 (LZW,     1984) — LZ78 + pre-initialised dict; powers GIF.
//! CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.  ← this crate
//! CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
//! ```
//!
//! # Historical Context
//!
//! David A. Huffman published the algorithm in *Proceedings of the IRE* (September
//! 1952) as a student at MIT. He discovered it while taking a course and proved it
//! was optimal — beating the Fano-Shannon coding scheme his professor had developed.
//! The algorithm is used in DEFLATE, JPEG, MP3, and most modern compressed formats.
//!
//! # Examples
//!
//! ```
//! use huffman_compression::{compress, decompress};
//!
//! let data = b"AAABBC";
//! let compressed = compress(data).unwrap();
//! let original = decompress(&compressed).unwrap();
//! assert_eq!(original, data);
//! ```

use std::collections::HashMap;

use huffman_tree::HuffmanTree;

// ─── Bit I/O helpers ──────────────────────────────────────────────────────────

/// Pack a bit string (e.g. "001011") into bytes, filling each byte from LSB upward.
///
/// This is the same convention used by LZW (CMP03) and GIF: the first bit of the
/// stream occupies bit 0 (the least-significant bit) of the first output byte.
///
/// # Example — "000101011" (9 bits) → [0xA8, 0x01]
///
/// ```text
/// Byte 0 (bits 0-7):
///   bit 0 ('0') → position 0: 0b00000000
///   bit 1 ('0') → position 1: 0b00000000
///   bit 2 ('0') → position 2: 0b00000000
///   bit 3 ('1') → position 3: 0b00001000
///   bit 4 ('0') → position 4: 0b00001000
///   bit 5 ('1') → position 5: 0b00101000
///   bit 6 ('0') → position 6: 0b00101000
///   bit 7 ('1') → position 7: 0b10101000 = 0xA8
/// Byte 1 (bit 8):
///   bit 8 ('1') → position 0: 0b00000001 = 0x01
/// ```
fn pack_bits_lsb_first(bits: &str) -> Vec<u8> {
    let mut output = Vec::new();
    let mut buffer: u8 = 0;
    let mut bit_pos = 0u8;
    for b in bits.chars() {
        if b == '1' {
            buffer |= 1 << bit_pos;
        }
        bit_pos += 1;
        if bit_pos == 8 {
            output.push(buffer);
            buffer = 0;
            bit_pos = 0;
        }
    }
    // Final partial byte — remaining high bits are zero-padded.
    if bit_pos > 0 {
        output.push(buffer);
    }
    output
}

/// Unpack bytes into a bit string, reading each byte from LSB upward.
///
/// Mirrors [`pack_bits_lsb_first`] exactly. The decoder reads only as many bits
/// as it needs and ignores any zero-padding in the final byte.
///
/// Each byte b produces 8 bits: `(b>>0)&1`, `(b>>1)&1`, ..., `(b>>7)&1` —
/// so the LSB of each byte becomes the "leftmost" (earliest in stream) bit.
fn unpack_bits_lsb_first(data: &[u8]) -> String {
    let mut bits = String::with_capacity(data.len() * 8);
    for byte in data {
        for i in 0..8 {
            bits.push(if (byte >> i) & 1 == 1 { '1' } else { '0' });
        }
    }
    bits
}

// ─── Encoder ─────────────────────────────────────────────────────────────────

/// Compress bytes using Huffman coding and return CMP04 wire-format bytes.
///
/// # Algorithm
///
/// 1. **Frequency count** — Build a histogram of byte values: `HashMap<u8, u32>`.
/// 2. **Build Huffman tree** — Delegate to [`HuffmanTree::build`] (DT27), passing
///    `(symbol, frequency)` pairs.
/// 3. **Canonical code table** — Call `tree.canonical_code_table()`, which returns
///    a `HashMap<u16, String>` mapping each symbol to its canonical bit string.
/// 4. **Sort lengths** — Collect `(symbol, code_length)` pairs and sort by
///    `(code_length, symbol)` ascending. This sorted order is exactly what the
///    decompressor needs to reconstruct canonical codes.
/// 5. **Encode** — Concatenate the canonical bit string for every input byte.
/// 6. **Pack bits** — Use [`pack_bits_lsb_first`] to convert the bit string to bytes.
/// 7. **Assemble** — Write the CMP04 header + code-lengths table + bit stream.
///
/// # Edge Cases
///
/// - **Empty input**: Returns an 8-byte header with `original_length=0`,
///   `symbol_count=0`, and no bit data.
/// - **Single distinct byte**: DT27 assigns it code `"0"`; each occurrence
///   encodes to 1 bit. The decompressor reconstructs this trivially.
///
/// # Errors
///
/// Returns `Err(String)` if the tree build fails (e.g., internal DT27 error).
///
/// # Examples
///
/// ```
/// use huffman_compression::{compress, decompress};
///
/// let compressed = compress(b"AAABBC").unwrap();
/// assert_eq!(decompress(&compressed).unwrap(), b"AAABBC");
/// ```
pub fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
    let original_length = data.len() as u32;

    // Edge case: empty input — return 8-byte header, no bit stream.
    if data.is_empty() {
        let mut out = Vec::with_capacity(8);
        out.extend_from_slice(&0u32.to_be_bytes()); // original_length = 0
        out.extend_from_slice(&0u32.to_be_bytes()); // symbol_count = 0
        return Ok(out);
    }

    // ── Step 1: Build a frequency histogram ──────────────────────────────────
    //
    // Count how many times each byte value appears in the input. These counts
    // are the "weights" the Huffman tree uses to decide code lengths.
    let mut freq: HashMap<u8, u32> = HashMap::new();
    for &b in data {
        *freq.entry(b).or_insert(0) += 1;
    }

    // ── Step 2: Build the Huffman tree via DT27 ───────────────────────────────
    //
    // Convert the histogram to `(symbol, frequency)` pairs. DT27 uses u16 for
    // symbols (to support larger alphabets), so we cast from u8.
    let pairs: Vec<(u16, u32)> = freq.iter().map(|(&sym, &cnt)| (sym as u16, cnt)).collect();
    let tree = HuffmanTree::build(&pairs)?;

    // ── Step 3: Canonical code table ─────────────────────────────────────────
    //
    // `canonical_code_table()` returns a HashMap<u16, String> where each value
    // is a bit string like "0", "10", "110". These codes are canonical: for a
    // given set of code lengths, the canonical assignment is unique and
    // deterministic. This means the decompressor only needs the lengths table —
    // not the full tree structure — to reconstruct the exact same codes.
    let table = tree.canonical_code_table();

    // ── Step 4: Build the sorted code-lengths list ────────────────────────────
    //
    // The wire format's code-lengths table must be sorted by (code_length, symbol)
    // so the decompressor can run the canonical reconstruction algorithm in
    // exactly the same order the encoder used.
    let mut lengths: Vec<(u16, usize)> = table
        .iter()
        .map(|(&sym, bits)| (sym, bits.len()))
        .collect();
    lengths.sort_by_key(|&(sym, len)| (len, sym));

    let symbol_count = lengths.len() as u32;

    // ── Step 5: Encode each byte using its canonical code ─────────────────────
    //
    // Concatenate the canonical bit strings for every input byte. The result is
    // a long bit string that will be packed into bytes in the next step.
    let mut bit_string = String::new();
    for &b in data {
        let code = table.get(&(b as u16)).ok_or_else(|| {
            format!("symbol {} not found in code table", b)
        })?;
        bit_string.push_str(code);
    }

    // ── Step 6: Pack bits LSB-first ───────────────────────────────────────────
    //
    // The bit string "001011..." becomes a compact byte array, packed LSB-first.
    // The final byte is zero-padded to a byte boundary if necessary.
    let bit_bytes = pack_bits_lsb_first(&bit_string);

    // ── Step 7: Assemble the CMP04 wire format ────────────────────────────────
    //
    // Layout:
    //   [0..4]      original_length   (4 bytes, big-endian u32)
    //   [4..8]      symbol_count      (4 bytes, big-endian u32)
    //   [8..8+2N]   code-lengths table  (N entries × 2 bytes each)
    //   [8+2N..]    bit stream          (variable)
    let mut out = Vec::with_capacity(8 + 2 * lengths.len() + bit_bytes.len());
    out.extend_from_slice(&original_length.to_be_bytes());
    out.extend_from_slice(&symbol_count.to_be_bytes());
    for (sym, len) in &lengths {
        out.push(*sym as u8);   // symbol value (fits in u8 for byte-level data)
        out.push(*len as u8);   // code length (1–16)
    }
    out.extend_from_slice(&bit_bytes);

    Ok(out)
}

// ─── Decoder ─────────────────────────────────────────────────────────────────

/// Decompress CMP04 wire-format bytes back to the original data.
///
/// # Algorithm
///
/// 1. **Parse header** — Read `original_length` and `symbol_count` from bytes 0–7.
/// 2. **Parse code-lengths table** — Read `symbol_count` × 2-byte entries from
///    bytes 8 through 8+2N. Each entry is `[symbol, code_length]`.
/// 3. **Reconstruct canonical codes** — Apply the canonical assignment algorithm
///    to the sorted `(symbol, code_length)` list, producing `code_string → symbol`.
/// 4. **Unpack bit stream** — Convert bytes 8+2N onward from LSB-first bytes to
///    a bit string using [`unpack_bits_lsb_first`].
/// 5. **Decode** — Scan the bit string left-to-right, accumulating bits until a
///    match is found in the canonical code table. Repeat for `original_length` symbols.
///
/// # Canonical Reconstruction
///
/// The reconstruction mirrors the encoder's assignment, ensuring we get identical
/// codes for each symbol:
///
/// ```text
/// code = 0
/// prev_len = lengths[0].1
/// for (sym, len) in &lengths:
///     if len > prev_len: code <<= (len - prev_len)
///     assign: format!("{:0>len$b}", code, len = len) → sym
///     code += 1
///     prev_len = len
/// ```
///
/// # Errors
///
/// Returns `Err(String)` if:
/// - Input is shorter than 8 bytes (missing header).
/// - The bit stream is exhausted before decoding all `original_length` symbols.
///
/// # Examples
///
/// ```
/// use huffman_compression::{compress, decompress};
///
/// let data = b"hello world";
/// assert_eq!(decompress(&compress(data).unwrap()).unwrap(), data);
/// ```
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    // ── Step 1: Parse the 8-byte header ──────────────────────────────────────
    if data.len() < 8 {
        return Err(format!(
            "input too short: need at least 8 bytes for header, got {}",
            data.len()
        ));
    }

    let original_length =
        u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let symbol_count =
        u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;

    // Edge case: empty original — nothing to decode.
    if original_length == 0 {
        return Ok(Vec::new());
    }

    // ── Step 2: Parse the code-lengths table ─────────────────────────────────
    //
    // The table begins at byte 8 and contains `symbol_count` entries, each
    // 2 bytes: [symbol_value (u8), code_length (u8)].
    // The table is sorted by (code_length, symbol_value) — the same order the
    // encoder wrote it.
    let table_start = 8;
    let table_end = table_start + 2 * symbol_count;
    if data.len() < table_end {
        return Err(format!(
            "input too short: need {} bytes for code-lengths table, got {}",
            table_end,
            data.len()
        ));
    }

    let mut lengths: Vec<(u16, usize)> = Vec::with_capacity(symbol_count);
    for i in 0..symbol_count {
        let off = table_start + 2 * i;
        let symbol = data[off] as u16;
        let length = data[off + 1] as usize;
        lengths.push((symbol, length));
    }

    // ── Step 3: Reconstruct canonical codes ──────────────────────────────────
    //
    // Using the sorted (symbol, length) list, we re-derive the exact same
    // canonical bit strings the encoder assigned. The algorithm is DEFLATE-style:
    //
    //   Start with code=0 at the first length.
    //   For each entry: if the length grew, left-shift the code to "make room"
    //   for the longer codes. This is because canonical codes at longer lengths
    //   fill in the gaps left by shorter-length codes in a complete binary tree.
    //
    // Example for lengths [(A,1), (B,2), (C,2)]:
    //   A: len=1, code=0b0  → "0";  code becomes 1
    //   B: len=2, code=0b10 → "10"; code becomes 3
    //   C: len=2, code=0b11 → "11"; code becomes 4
    //
    // This guarantees prefix-free assignment: no code string is a prefix of another.
    let mut code_to_sym: HashMap<String, u16> = HashMap::with_capacity(symbol_count);

    if !lengths.is_empty() {
        let mut code: u32 = 0;
        let mut prev_len = lengths[0].1;
        for (sym, len) in &lengths {
            if *len > prev_len {
                code <<= len - prev_len;
            }
            let bits = format!("{:0>width$b}", code, width = len);
            code_to_sym.insert(bits, *sym);
            code += 1;
            prev_len = *len;
        }
    }

    // ── Step 4: Unpack the bit stream ─────────────────────────────────────────
    //
    // Bytes from `table_end` onward are the LSB-first packed bit stream. We
    // expand them to a full bit string ("010110...") for easy prefix matching.
    let bit_string = unpack_bits_lsb_first(&data[table_end..]);

    // ── Step 5: Decode `original_length` symbols ──────────────────────────────
    //
    // The code is prefix-free: scan left-to-right, accumulate bits until we hit
    // a match in `code_to_sym`, emit the symbol, reset the accumulator.
    // The zero-padding at the end of the bit stream is harmless: once we've
    // decoded `original_length` symbols we stop, never reading into padding bits.
    let mut output: Vec<u8> = Vec::with_capacity(original_length);
    let mut pos = 0usize;
    let mut accumulated = String::new();

    while output.len() < original_length {
        if pos >= bit_string.len() {
            return Err(format!(
                "bit stream exhausted after {} symbols, expected {}",
                output.len(),
                original_length
            ));
        }
        accumulated.push(bit_string.chars().nth(pos).unwrap());
        pos += 1;
        if let Some(&sym) = code_to_sym.get(&accumulated) {
            output.push(sym as u8);
            accumulated.clear();
        }
    }

    Ok(output)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Round-trip helper: compress then decompress, asserting success.
    fn rt(data: &[u8]) -> Vec<u8> {
        let compressed = compress(data).expect("compress failed");
        decompress(&compressed).expect("decompress failed")
    }

    // ── Round-trip tests ──────────────────────────────────────────────────────

    /// Vector 1 — Empty input.
    ///
    /// Expected behaviour: compress returns an 8-byte header (length=0,
    /// symbol_count=0). Decompress reads the header, sees length=0, and returns
    /// an empty Vec without touching the bit stream.
    #[test]
    fn test_rt_empty() {
        let compressed = compress(b"").unwrap();
        assert_eq!(compressed.len(), 8, "empty input should produce 8-byte header only");
        assert_eq!(rt(b""), b"");
    }

    /// Vector 2 — Single byte repeated ('A' × 1).
    ///
    /// A single distinct symbol gets code "0" from DT27. The bit stream will be
    /// a single '0' bit packed into one byte (zero-padded to 0x00).
    #[test]
    fn test_rt_single_byte() {
        assert_eq!(rt(b"A"), b"A");
    }

    /// Vector 3 — Single symbol repeated many times.
    ///
    /// "AAAA" has only one distinct byte. DT27 assigns it code "0", so 4 symbols
    /// → 4 bits → 1 byte payload.
    #[test]
    fn test_rt_single_symbol_repeated() {
        assert_eq!(rt(b"AAAA"), b"AAAA");
    }

    /// Vector 4 — Classic "AAABBC" test case from the CMP04 spec.
    ///
    /// Frequencies: A=3, B=2, C=1.
    ///
    /// Huffman tree (DT27 canonical, tie-breaking: leaves before internals,
    /// lower symbol wins):
    ///
    ///   Min-heap start: C(1), B(2), A(3)
    ///   1. Pop C(1) + B(2) → Internal I1(3)
    ///   2. Pop A(3) + I1(3) → Root I2(6)
    ///   Codes (tree walk):
    ///     A  → left of root  → "0"
    ///     C  → right, left   → "10"
    ///     B  → right, right  → "11"
    ///   Canonical (sorted by (len, sym)):
    ///     A(1): code=0 → "0"
    ///     B(2): code=2 → "10"  (after shift: 1<<1=2)
    ///     C(2): code=3 → "11"
    ///
    /// Wait — canonical sorted by (len, sym) means B comes before C:
    ///   Sort: [(A,1),(B,2),(C,2)]
    ///   A: len=1, code=0 → "0";  code→1
    ///   B: len=2, shift: code=1<<1=2 → "10"; code→3
    ///   C: len=2, no shift: code=3 → "11"; code→4
    ///
    /// Encoding "AAABBC":
    ///   A="0"  A="0"  A="0"  B="10"  B="10"  C="11"
    ///   bits: "000101011" (wait, wrong order — "000" + "10" + "10" + "11"
    ///         = "0001010 11" = "000101011" — 9 bits)
    ///
    /// Hmm, let me recheck: "A"+"A"+"A"+"B"+"B"+"C" → "0"+"0"+"0"+"10"+"10"+"11"
    ///   = "000101011" — 9 bits.
    ///
    /// Pack LSB-first into bytes:
    ///   bits: 0,0,0,1,0,1,0,1 | 1
    ///   Byte 0: bit0=0, bit1=0, bit2=0, bit3=1, bit4=0, bit5=1, bit6=0, bit7=1
    ///         = 0b10101000 = 0xA8
    ///   Byte 1: bit8=1 → 0b00000001 = 0x01
    ///
    /// Wire format:
    ///   [0,0,0,6]      original_length=6
    ///   [0,0,0,3]      symbol_count=3
    ///   [65,1]         A, len=1
    ///   [66,2]         B, len=2
    ///   [67,2]         C, len=2
    ///   [0xA8, 0x01]   bit stream
    #[test]
    fn test_rt_aaabbc() {
        assert_eq!(rt(b"AAABBC"), b"AAABBC");
    }

    /// Vector 5 — Wire format verification for "AAABBC".
    ///
    /// This test verifies the exact wire bytes, confirming the CMP04 spec is
    /// implemented correctly down to the bit level.
    #[test]
    fn test_wire_format_aaabbc() {
        let compressed = compress(b"AAABBC").unwrap();
        // Header
        assert_eq!(&compressed[0..4], &[0, 0, 0, 6], "original_length=6");
        assert_eq!(&compressed[4..8], &[0, 0, 0, 3], "symbol_count=3");
        // Code-lengths table: A(65,1), B(66,2), C(67,2)
        assert_eq!(&compressed[8..10],  &[65, 1], "A: len=1");
        assert_eq!(&compressed[10..12], &[66, 2], "B: len=2");
        assert_eq!(&compressed[12..14], &[67, 2], "C: len=2");
        // Bit stream: "000101011" packed LSB-first → [0xA8, 0x01]
        assert_eq!(&compressed[14..], &[0xA8, 0x01], "packed bits");
    }

    /// Vector 6 — "hello world".
    #[test]
    fn test_rt_hello_world() {
        assert_eq!(rt(b"hello world"), b"hello world");
    }

    /// Vector 7 — All 256 distinct byte values in sequence.
    ///
    /// This exercises the full byte alphabet. Every symbol has frequency 1, so
    /// all codes should be the same length (or near the same).
    #[test]
    fn test_rt_all_256_bytes() {
        let data: Vec<u8> = (0u8..=255).collect();
        assert_eq!(rt(&data), data);
    }

    /// Vector 8 — Binary data with null bytes.
    #[test]
    fn test_rt_binary_nulls() {
        assert_eq!(rt(&[0, 0, 0, 255, 255]), &[0, 0, 0, 255, 255]);
    }

    /// Vector 9 — Long repetitive pattern.
    ///
    /// Skewed input: 'A' repeated 1000 times. Only one symbol, so the Huffman
    /// tree has a single leaf and assigns code "0". The compressed bit stream
    /// is 1000 bits ≈ 125 bytes, compared to 1000 bytes raw — about 8× smaller.
    #[test]
    fn test_rt_long_repeated_byte() {
        let data = vec![b'A'; 1000];
        assert_eq!(rt(&data), data);
    }

    /// Vector 10 — Longer varied input.
    #[test]
    fn test_rt_lorem_ipsum() {
        let text = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                     Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.";
        assert_eq!(rt(text), text.as_ref());
    }

    // ── Wire-format checks ────────────────────────────────────────────────────

    /// Header stores the correct original_length.
    #[test]
    fn test_header_original_length() {
        let data = b"hello";
        let compressed = compress(data).unwrap();
        let stored = u32::from_be_bytes([
            compressed[0], compressed[1], compressed[2], compressed[3],
        ]);
        assert_eq!(stored, 5);
    }

    /// Header stores the correct symbol_count.
    #[test]
    fn test_header_symbol_count_aaabbc() {
        let compressed = compress(b"AAABBC").unwrap();
        let count = u32::from_be_bytes([
            compressed[4], compressed[5], compressed[6], compressed[7],
        ]);
        assert_eq!(count, 3, "3 distinct symbols: A, B, C");
    }

    /// Empty input produces exactly 8 bytes.
    #[test]
    fn test_empty_produces_8_bytes() {
        let compressed = compress(b"").unwrap();
        assert_eq!(compressed.len(), 8);
        assert_eq!(&compressed[0..4], &[0, 0, 0, 0]); // original_length=0
        assert_eq!(&compressed[4..8], &[0, 0, 0, 0]); // symbol_count=0
    }

    // ── Compression effectiveness ─────────────────────────────────────────────

    /// Skewed input (one dominant byte) compresses well.
    ///
    /// 'A' × 10000 with sprinkled 'B's. The Huffman tree assigns 'A' a very
    /// short code (1 bit) and 'B' a longer code. Net result: well under 50% of
    /// original size.
    #[test]
    fn test_skewed_input_compresses() {
        let mut data = vec![b'A'; 9900];
        data.extend_from_slice(b"BBBBBBBBBB"); // 10 B's, 9900 A's
        let compressed = compress(&data).unwrap();
        // Each A encodes to ~1 bit, each B to ~14 bits. Total ≈ 9900+140=10040 bits
        // ≈ 1256 bytes. Raw is 9910 bytes. Should be much smaller.
        assert!(
            compressed.len() < data.len(),
            "compressed ({}) should be smaller than raw ({})",
            compressed.len(),
            data.len()
        );
    }

    /// Uniform distribution does not compress as well as skewed, but round-trips.
    #[test]
    fn test_uniform_distribution_roundtrip() {
        let data: Vec<u8> = (0u8..=255).cycle().take(1024).collect();
        assert_eq!(rt(&data), data);
    }

    // ── Edge cases ────────────────────────────────────────────────────────────

    /// Two distinct symbols — minimal tree.
    #[test]
    fn test_two_symbols() {
        let data = b"ABABABAB";
        assert_eq!(rt(data), data);
    }

    /// Single occurrence of each of many symbols.
    #[test]
    fn test_all_unique_symbols() {
        let data: Vec<u8> = (0u8..=127).collect();
        assert_eq!(rt(&data), data);
    }

    // ── Error handling ─────────────────────────────────────────────────────────

    /// Input shorter than 8 bytes returns Err.
    #[test]
    fn test_decompress_too_short() {
        assert!(decompress(b"").is_err());
        assert!(decompress(b"short").is_err());
        assert!(decompress(&[0u8; 7]).is_err());
    }

    /// Valid 8-byte header with original_length=0 returns empty output.
    #[test]
    fn test_decompress_zero_length_header() {
        let header = [0u8; 8]; // original_length=0, symbol_count=0
        assert_eq!(decompress(&header).unwrap(), b"");
    }

    // ── Determinism ───────────────────────────────────────────────────────────

    /// Compress is deterministic: two calls with the same input produce identical output.
    #[test]
    fn test_compress_deterministic() {
        let data = b"hello world test";
        assert_eq!(compress(data).unwrap(), compress(data).unwrap());
    }

    // ── Bit I/O unit tests ────────────────────────────────────────────────────

    /// Pack "0" (1 bit) → [0x00] (zero-padded to full byte).
    #[test]
    fn test_pack_single_zero_bit() {
        assert_eq!(pack_bits_lsb_first("0"), vec![0x00]);
    }

    /// Pack "1" (1 bit) → [0x01].
    #[test]
    fn test_pack_single_one_bit() {
        assert_eq!(pack_bits_lsb_first("1"), vec![0x01]);
    }

    /// Pack "000101011" (9 bits) → [0xA8, 0x01].
    ///
    /// This is the expected payload for "AAABBC" (see test_wire_format_aaabbc).
    #[test]
    fn test_pack_9bits_aaabbc() {
        // "AAABBC" encodes to: A="0",A="0",A="0",B="10",B="10",C="11"
        // bit string: "000101011"
        assert_eq!(pack_bits_lsb_first("000101011"), vec![0xA8, 0x01]);
    }

    /// Pack an empty string → empty Vec.
    #[test]
    fn test_pack_empty() {
        assert!(pack_bits_lsb_first("").is_empty());
    }

    /// Pack exactly 8 bits → 1 byte.
    #[test]
    fn test_pack_8bits_exact() {
        // "11111111" → 0xFF
        assert_eq!(pack_bits_lsb_first("11111111"), vec![0xFF]);
    }

    /// Unpack then pack is the identity for aligned bytes.
    #[test]
    fn test_unpack_pack_roundtrip() {
        let bytes = vec![0xA8u8, 0x01u8];
        let bits = unpack_bits_lsb_first(&bytes);
        assert_eq!(bits.len(), 16);
        // Re-pack all 16 bits → same bytes
        assert_eq!(pack_bits_lsb_first(&bits), bytes);
    }

    /// Unpack [0xA8] gives bits in LSB-first order: 0,0,0,1,0,1,0,1.
    #[test]
    fn test_unpack_0xa8() {
        // 0xA8 = 0b10101000
        // LSB-first: bit0=0, bit1=0, bit2=0, bit3=1, bit4=0, bit5=1, bit6=0, bit7=1
        assert_eq!(unpack_bits_lsb_first(&[0xA8]), "00010101");
    }
}
