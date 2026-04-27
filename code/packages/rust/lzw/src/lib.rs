//! LZW lossless compression algorithm (1984) — CMP03.
//!
//! LZW (Lempel-Ziv-Welch) is a refinement of LZ78 (CMP01) that eliminates
//! LZ78's mandatory `next_char` byte by **pre-seeding the dictionary** with
//! all 256 single-byte sequences. Because every possible byte already has a
//! code (0–255), the encoder never emits raw literals — every symbol in the
//! output is a dictionary code.
//!
//! This small change has large consequences:
//!
//! - Tokens shrink from `(dict_index, next_char)` tuples to just **codes**
//! - Output is a pure code stream, enabling **variable-width bit-packing**
//! - Compression typically improves 10–30% over LZ78 on typical text
//!
//! # Series
//!
//! ```text
//! CMP00 (LZ77,    1977) — Sliding-window backreferences.
//! CMP01 (LZ78,    1978) — Explicit dictionary (trie).
//! CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
//! CMP03 (LZW,     1984) — LZ78 + pre-initialised alphabet; GIF.  ← this crate
//! CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//! CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
//! ```
//!
//! # Historical Context
//!
//! Terry Welch published the algorithm in *IEEE Computer* (June 1984). Sperry
//! (later Unisys) held a patent until 2003, causing controversy when GIF used
//! LZW. The patent's expiry opened LZW to royalty-free use.
//!
//! # Wire Format (CMP03)
//!
//! ```text
//! Bytes 0–3:  original_length — BE uint32.
//! Bytes 4+:   bit-packed variable-width codes, LSB-first within each byte.
//!
//! Code stream:
//!   1. CLEAR_CODE (256) at code_size = 9
//!   2. Data codes at current code_size
//!   3. STOP_CODE  (257) at current code_size
//!   4. Zero-padding bits to align the final byte
//! ```
//!
//! # Examples
//!
//! ```
//! use lzw::{compress, decompress};
//!
//! let data = b"hello hello hello";
//! let compressed = compress(data);
//! assert_eq!(decompress(&compressed).unwrap(), data);
//! ```

use std::collections::HashMap;

// ─── Constants ────────────────────────────────────────────────────────────────

/// Instructs the decoder to reset the dictionary to its initial 256-entry
/// state and restart with `next_code = 258` and `code_size = 9`.
pub const CLEAR_CODE: u16 = 256;

/// Marks the end of the compressed code stream. The decoder stops here.
pub const STOP_CODE: u16 = 257;

/// First dynamically assigned code (0–255 are pre-seeded, 256/257 are
/// control codes).
pub const INITIAL_NEXT_CODE: u16 = 258;

/// Starting bit-width of codes. 9 bits can represent 0–511, comfortably
/// covering the initial 258 entries (0–257).
pub const INITIAL_CODE_SIZE: u8 = 9;

/// Maximum bit-width. Codes grow up to 16 bits (65 536 max entries). When
/// `next_code` would exceed 65 535 the encoder emits CLEAR_CODE and resets.
pub const MAX_CODE_SIZE: u8 = 16;

// ─── Bit writer ───────────────────────────────────────────────────────────────

/// Accumulates variable-width codes into a byte stream, LSB-first.
///
/// GIF, Unix `compress`, and TIFF all use LSB-first packing: the first code
/// fills the *least-significant* bits of the first byte, then spills into
/// higher bytes. Example with code=0b101 (3 bits):
///
/// ```text
///   byte:   [  bit7 .. bit0  ]
///   after:  [  xxxxx  1  0  1]   ← bits fill from LSB upward
/// ```
struct BitWriter {
    /// Bits waiting to be flushed into `bytes`. At most 63 valid bits at once.
    buffer: u64,
    /// How many valid bits are currently in `buffer`.
    bit_pos: u8,
    /// Fully emitted output bytes.
    bytes: Vec<u8>,
}

impl BitWriter {
    fn new() -> Self {
        BitWriter { buffer: 0, bit_pos: 0, bytes: Vec::new() }
    }

    /// Append `code` (lowest `size` bits) to the bit stream.
    fn write(&mut self, code: u16, size: u8) {
        // Shift `code` into the buffer at the current bit position.
        self.buffer |= (code as u64) << self.bit_pos;
        self.bit_pos += size;

        // Drain complete bytes from the LSB side.
        while self.bit_pos >= 8 {
            self.bytes.push((self.buffer & 0xFF) as u8);
            self.buffer >>= 8;
            self.bit_pos -= 8;
        }
    }

    /// Flush any remaining partial byte (zero-padded to the byte boundary).
    fn flush(&mut self) {
        if self.bit_pos > 0 {
            self.bytes.push((self.buffer & 0xFF) as u8);
            self.bit_pos = 0;
            self.buffer = 0;
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

// ─── Bit reader ───────────────────────────────────────────────────────────────

/// Reads variable-width codes from a byte slice, LSB-first.
///
/// Mirrors [`BitWriter`]: bits are consumed from the least-significant side
/// first. When the internal buffer runs low, the next byte from `data` is
/// OR-ed into the high bits of `buffer`.
struct BitReader<'a> {
    data: &'a [u8],
    /// Current byte index into `data`.
    pos: usize,
    /// Bits waiting to be consumed. At most 63 valid bits.
    buffer: u64,
    /// How many valid bits are in `buffer`.
    bit_pos: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader { data, pos: 0, buffer: 0, bit_pos: 0 }
    }

    /// Read the next `size`-bit code. Returns `None` if the byte slice is
    /// exhausted before all `size` bits can be filled.
    fn read(&mut self, size: u8) -> Option<u16> {
        // Refill buffer until we have at least `size` bits or exhaust input.
        while self.bit_pos < size {
            if self.pos >= self.data.len() {
                // No more bytes available. If we have nothing, signal EOF.
                if self.bit_pos == 0 {
                    return None;
                }
                break;
            }
            self.buffer |= (self.data[self.pos] as u64) << self.bit_pos;
            self.pos += 1;
            self.bit_pos += 8;
        }

        if self.bit_pos < size {
            return None;
        }

        let mask = (1u64 << size) - 1;
        let code = (self.buffer & mask) as u16;
        self.buffer >>= size;
        self.bit_pos -= size;
        Some(code)
    }
}

// ─── Encoder ─────────────────────────────────────────────────────────────────

/// Compress bytes using LZW and return the CMP03 wire-format bytes.
///
/// The encoder maintains a dictionary of `Vec<u8> → code` mappings, seeded
/// with all 256 single-byte sequences. It greedily extends the current prefix
/// `w` with each incoming byte until no dictionary entry matches, then emits
/// the code for `w`, adds `w + [b]` to the dictionary, and restarts with
/// `w = [b]`.
///
/// Code width starts at 9 bits and grows by 1 each time `next_code` crosses
/// the next power-of-two boundary. When `next_code` would reach 2^16 the
/// dictionary is full; the encoder emits CLEAR_CODE and starts fresh.
///
/// # Examples
///
/// ```
/// use lzw::{compress, decompress};
///
/// assert_eq!(decompress(&compress(b"ABABAB")).unwrap(), b"ABABAB");
/// ```
pub fn compress(data: &[u8]) -> Vec<u8> {
    // ── Build encoder dictionary ──────────────────────────────────────────────
    // Keys are byte sequences; values are assigned codes. Pre-seeded with
    // all 256 single-byte sequences (the key LZW innovation over LZ78).
    let mut dict: HashMap<Vec<u8>, u16> = HashMap::with_capacity(4096);
    for b in 0u16..=255 {
        dict.insert(vec![b as u8], b);
    }

    let mut next_code: u32 = INITIAL_NEXT_CODE as u32;
    let mut code_size: u8 = INITIAL_CODE_SIZE;
    let mut writer = BitWriter::new();

    // Every well-formed LZW stream opens with CLEAR_CODE so the decoder can
    // initialise its state without out-of-band knowledge.
    writer.write(CLEAR_CODE, code_size);

    // `w` holds the longest current prefix that is in the dictionary.
    let mut w: Vec<u8> = Vec::new();

    for &b in data {
        w.push(b);

        if !dict.contains_key(&w) {
            // `w` is not in the dict. Pop `b` — `w[..-1]` IS in the dict.
            w.pop();
            let code = dict[&w];
            writer.write(code, code_size);

            // Add `w + [b]` to the dictionary if below the 2^MAX_CODE_SIZE cap.
            let max_entry = 1u32 << MAX_CODE_SIZE; // 65 536
            if next_code < max_entry {
                let mut new_entry = w.clone();
                new_entry.push(b);
                dict.insert(new_entry, next_code as u16);
                next_code += 1;

                // Grow code width when `next_code` crosses a power-of-two.
                // Example: when next_code becomes 513, codes 0–512 fit in 9
                // bits, but code 513 needs 10. We bump code_size when
                // next_code > 2^code_size.
                if next_code > (1u32 << code_size) && code_size < MAX_CODE_SIZE {
                    code_size += 1;
                }
            } else {
                // Dictionary is full — reset and start fresh. The decoder will
                // mirror this reset when it reads CLEAR_CODE.
                writer.write(CLEAR_CODE, code_size);
                dict.clear();
                for b2 in 0u16..=255 {
                    dict.insert(vec![b2 as u8], b2);
                }
                next_code = INITIAL_NEXT_CODE as u32;
                code_size = INITIAL_CODE_SIZE;
            }

            // Restart prefix with the unmatched byte.
            w = vec![b];
        }
        // If `w` IS still in the dict, continue extending the prefix.
    }

    // Flush the last prefix (the greedy loop always leaves a pending prefix).
    if !w.is_empty() {
        let code = dict[&w];
        writer.write(code, code_size);
    }

    writer.write(STOP_CODE, code_size);
    writer.flush();

    // ── Assemble CMP03 wire format ────────────────────────────────────────────
    let bit_bytes = writer.into_bytes();
    let original_length = data.len() as u32;

    // The 4-byte big-endian header lets the decoder trim zero-padding
    // artefacts from the final partial byte of the bit stream.
    let mut out = Vec::with_capacity(4 + bit_bytes.len());
    out.extend_from_slice(&original_length.to_be_bytes());
    out.extend_from_slice(&bit_bytes);
    out
}

// ─── Decoder ─────────────────────────────────────────────────────────────────

/// Decompress CMP03 wire-format bytes back to the original data.
///
/// Returns `Ok(Vec<u8>)` on success, `Err(String)` on malformed input.
///
/// The decoder maintains a dictionary indexed by code number. It is seeded
/// with the same 256 single-byte sequences as the encoder. Each step:
///
/// 1. Read the next code at the current `code_size`.
/// 2. Resolve the entry (`dict[code]`), handling the tricky-token edge case.
/// 3. Append the entry to output.
/// 4. If `prev_code` is set, add `dict[prev_code] + entry[0]` as the next
///    dictionary entry and bump `next_code`/`code_size` as needed.
///
/// ## The Tricky-Token Edge Case
///
/// When encoding `"AAAAAA..."` the encoder emits a code (call it `C`) that
/// the decoder hasn't *finished* adding to its own dict yet. Specifically,
/// `C == next_code` — a self-referential code. The resolution is:
///
/// ```text
/// entry = dict[prev_code] + [dict[prev_code][0]]
/// ```
///
/// This works because any self-referential code must represent a sequence
/// that starts and ends with the same byte as the previous match.
///
/// # Examples
///
/// ```
/// use lzw::{compress, decompress};
///
/// let data: Vec<u8> = b"AAAAAAA".to_vec();
/// assert_eq!(decompress(&compress(&data)).unwrap(), data);
/// ```
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 4 {
        return Err("input too short: missing 4-byte header".into());
    }

    let original_length =
        u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let payload = &data[4..];

    // ── Build decoder dictionary ──────────────────────────────────────────────
    // Indexed by code; value is the byte sequence that code represents.
    // Slots 256 and 257 are reserved for CLEAR_CODE / STOP_CODE — we store
    // empty vecs there so the index arithmetic stays simple.
    let mut dict: Vec<Vec<u8>> = Vec::with_capacity(4096);
    for b in 0u8..=255 {
        dict.push(vec![b]);
    }
    dict.push(vec![]); // slot 256 = CLEAR_CODE placeholder
    dict.push(vec![]); // slot 257 = STOP_CODE placeholder

    // `next_code` tracks the code-size boundary — it is incremented for
    // EVERY data code received, mirroring the encoder's emit-side counter.
    // This must NOT be confused with dict.len(): the first data code after
    // CLEAR bumps next_code (for code_size purposes) but adds no dict entry
    // (because there is no prev_code yet). Keeping these separate is the key
    // to correct code_size growth in both encoder and decoder.
    let mut next_code: usize = INITIAL_NEXT_CODE as usize;
    let mut code_size: u8 = INITIAL_CODE_SIZE;
    let mut prev_code: Option<usize> = None;
    let mut output: Vec<u8> = Vec::new();

    let mut reader = BitReader::new(payload);

    // ── First code must be CLEAR_CODE ─────────────────────────────────────────
    let first = reader.read(code_size).ok_or("empty payload")?;
    if first != CLEAR_CODE {
        return Err(format!(
            "expected CLEAR_CODE (256) at start, got {}",
            first
        ));
    }

    // ── Main decode loop ──────────────────────────────────────────────────────
    loop {
        let code = match reader.read(code_size) {
            Some(c) => c as usize,
            None => break,
        };

        // ── Control codes ─────────────────────────────────────────────────────

        if code == CLEAR_CODE as usize {
            // Reset dictionary and all decoder state.
            dict.truncate(258); // keep pre-seeded slots 0–257
            next_code = INITIAL_NEXT_CODE as usize;
            code_size = INITIAL_CODE_SIZE;
            prev_code = None;
            continue;
        }

        if code == STOP_CODE as usize {
            break;
        }

        // ── Resolve entry ─────────────────────────────────────────────────────
        //
        // The tricky-token case: code == dict.len() (the slot *about* to be
        // added). This happens for runs like "AAAA..." where the encoder emits
        // a code that refers to the entry being built in this very step.
        //
        //   entry = dict[prev_code] + [dict[prev_code][0]]
        //
        // Any self-referential code must represent a sequence that starts and
        // ends with the same byte as the previous match (by construction).

        let entry: Vec<u8> = if code < dict.len() {
            dict[code].clone()
        } else if code == dict.len() {
            // Tricky token: dict.len() is exactly the next-to-be-added slot.
            let prev = prev_code.ok_or("tricky token but no prev_code")?;
            if dict[prev].is_empty() {
                return Err("tricky token: prev entry is empty".into());
            }
            let first_byte = dict[prev][0];
            let mut e = dict[prev].clone();
            e.push(first_byte);
            e
        } else {
            return Err(format!(
                "invalid code {}: dict.len() is {} (not a tricky token)",
                code,
                dict.len()
            ));
        };

        output.extend_from_slice(&entry);

        // ── Update code_size tracking ─────────────────────────────────────────
        //
        // Increment next_code for EVERY data code received — including the
        // first one after CLEAR (when prev_code is None and no dict entry is
        // added). This mirrors the encoder, which increments its own next_code
        // for every data code emitted, ensuring code_size bumps in lockstep.
        if next_code < (1usize << MAX_CODE_SIZE) {
            next_code += 1;
            if next_code > (1usize << code_size) && code_size < MAX_CODE_SIZE {
                code_size += 1;
            }
        }

        // ── Add new dictionary entry ──────────────────────────────────────────
        //
        // New entry = dict[prev_code] + [entry[0]]. Only possible when
        // prev_code is set (i.e., not on the first data code after CLEAR).
        if let Some(prev) = prev_code {
            if dict.len() < (1usize << MAX_CODE_SIZE) {
                let first_byte = entry[0];
                let mut new_entry = dict[prev].clone();
                new_entry.push(first_byte);
                dict.push(new_entry);
            }
        }

        prev_code = Some(code);
    }

    // Trim to the original length (removes zero-padding artefacts from the
    // final partial byte of the bit stream).
    output.truncate(original_length);
    Ok(output)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Shorthand round-trip helper.
    fn rt(data: &[u8]) -> Vec<u8> {
        decompress(&compress(data)).expect("decompress failed")
    }

    // ── Spec vectors ──────────────────────────────────────────────────────────

    /// Vector 1 — Empty input.
    /// Codes: CLEAR(256), STOP(257). Output: 4-byte header + 3 packed bytes.
    #[test]
    fn test_rt_empty() {
        assert_eq!(rt(b""), b"");
    }

    /// Vector 2 — Single byte.
    /// Codes: CLEAR, 65('A'), STOP.
    #[test]
    fn test_rt_single_byte() {
        assert_eq!(rt(b"A"), b"A");
    }

    /// Vector 3 — Two distinct bytes (no repetition).
    /// Codes: CLEAR, 65, 66, STOP. Dict adds 258="AB" but never emits it.
    #[test]
    fn test_rt_two_distinct() {
        assert_eq!(rt(b"AB"), b"AB");
    }

    /// Vector 4 — Repeated pair "ABABAB".
    ///
    /// Encoding trace:
    ///   w=""
    ///   b=A: w="A"  → in dict
    ///   b=B: w="AB" → not in dict → emit 65("A"); add 258="AB"; w="B"
    ///   b=A: w="BA" → not in dict → emit 66("B"); add 259="BA"; w="A"
    ///   b=B: w="AB" → in dict (258)
    ///   b=A: w="ABA"→ not in dict → emit 258("AB"); add 260="ABA"; w="A"
    ///   b=B: w="AB" → in dict (258)
    ///   EOF: emit 258("AB"); STOP
    ///
    /// Codes: CLEAR, 65, 66, 258, 258, STOP
    #[test]
    fn test_rt_ababab() {
        assert_eq!(rt(b"ABABAB"), b"ABABAB");
    }

    /// Vector 5 — All-same bytes "AAAAAAA" (7 bytes).
    ///
    /// Exercises the tricky-token edge case in the decoder.
    /// Codes: CLEAR, 65, 258(tricky), 259(tricky), 65, STOP.
    ///
    /// Decoding:
    ///   65  → "A";  prev=None → no entry added; prev=65
    ///   258 == next_code(258): tricky! entry = dict[65]+"A" = "AA";
    ///         add dict[258]="AA"; next=259; prev=258
    ///   259 == next_code(259): tricky! entry = dict[258]+"A" = "AAA";
    ///         add dict[259]="AAA"; next=260; prev=259
    ///   65  → "A";  add dict[260]="AAAA"; prev=65
    ///   STOP
    ///   Output: "A"+"AA"+"AAA"+"A" = "AAAAAAA" ✓
    #[test]
    fn test_rt_all_same_seven() {
        assert_eq!(rt(b"AAAAAAA"), b"AAAAAAA");
    }

    /// Vector 6 — All 256 distinct bytes in order.
    #[test]
    fn test_rt_full_byte_range() {
        let data: Vec<u8> = (0u8..=255).collect();
        assert_eq!(rt(&data), data);
    }

    // ── Additional round-trips ─────────────────────────────────────────────────

    #[test]
    fn test_rt_hello_world() {
        assert_eq!(rt(b"hello world"), b"hello world");
    }

    #[test]
    fn test_rt_binary_nulls() {
        assert_eq!(rt(&[0, 0, 0, 255, 255]), &[0, 0, 0, 255, 255]);
    }

    #[test]
    fn test_rt_repeated_pattern_300() {
        let data: Vec<u8> = (0..300usize).map(|i| (i % 3) as u8).collect();
        assert_eq!(rt(&data), data);
    }

    #[test]
    fn test_rt_long_cycle() {
        let data: Vec<u8> = b"ABCDEF".iter().cloned().cycle().take(3000).collect();
        assert_eq!(rt(&data), data);
    }

    #[test]
    fn test_rt_large_same_byte() {
        let data = vec![0x42u8; 10_000];
        assert_eq!(rt(&data), data);
    }

    #[test]
    fn test_rt_lorem_ipsum() {
        let text = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                     Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.";
        assert_eq!(rt(text), text.as_ref());
    }

    #[test]
    fn test_rt_high_entropy() {
        // Pseudo-random-ish data with no obvious repetition.
        let data: Vec<u8> = (0u8..=255).cycle().take(512).collect();
        assert_eq!(rt(&data), data);
    }

    // ── Wire-format checks ────────────────────────────────────────────────────

    #[test]
    fn test_header_stores_original_length() {
        let data = b"hello";
        let compressed = compress(data);
        let stored = u32::from_be_bytes([
            compressed[0], compressed[1], compressed[2], compressed[3],
        ]);
        assert_eq!(stored, 5);
    }

    #[test]
    fn test_compress_deterministic() {
        assert_eq!(
            compress(b"hello world test"),
            compress(b"hello world test")
        );
    }

    // ── Compression effectiveness ─────────────────────────────────────────────

    #[test]
    fn test_repetitive_data_compresses() {
        let data: Vec<u8> = b"ABC".iter().cloned().cycle().take(3000).collect();
        assert!(compress(&data).len() < data.len());
    }

    #[test]
    fn test_all_same_byte_compresses() {
        let data = vec![0x42u8; 10_000];
        let compressed = compress(&data);
        assert!(compressed.len() < data.len());
    }

    // ── Security / error handling ─────────────────────────────────────────────

    /// Input shorter than 4 bytes cannot contain the header.
    #[test]
    fn test_too_short_header_is_error() {
        assert!(decompress(b"abc").is_err());
    }

    /// 4-byte header but no payload bytes → cannot read CLEAR_CODE.
    #[test]
    fn test_empty_payload_is_error() {
        assert!(decompress(&[0u8; 4]).is_err());
    }

    /// If the first 9-bit code is not CLEAR_CODE (256), the stream is invalid.
    #[test]
    fn test_missing_clear_code_is_error() {
        // 9-bit code for 65 ('A') in LSB-first packing:
        //   65 = 0b0_0100_0001
        //   byte 0: bits 0-7 = 0b0100_0001 = 0x41
        //   byte 1: bit  8   = 0b0000_0000 = 0x00  (partial, flushed)
        let mut bad = vec![0u8; 4]; // original_length header
        bad.push(0x41);
        bad.push(0x00);
        assert!(decompress(&bad).is_err());
    }

    /// Corrupted payload must not panic; either returns data or an error.
    #[test]
    fn test_crafted_corrupted_payload_no_panic() {
        let mut compressed = compress(b"hello world");
        if compressed.len() > 7 {
            compressed[6] ^= 0xFF;
            compressed[7] ^= 0xFF;
        }
        let _ = decompress(&compressed); // must not panic
    }

    // ── Bit I/O unit tests ────────────────────────────────────────────────────

    /// A single 9-bit code 256 (0b1_0000_0000) packed LSB-first:
    ///   bits 0-7 → byte 0 = 0x00
    ///   bit  8   → byte 1 = 0x01 (flushed)
    #[test]
    fn test_bit_writer_9bit_clear_code() {
        let mut w = BitWriter::new();
        w.write(256, 9);
        w.flush();
        assert_eq!(w.into_bytes(), &[0x00, 0x01]);
    }

    /// Two consecutive 9-bit zero codes → 18 bits → 2 full bytes + 2-bit
    /// partial byte (zero-padded on flush).
    #[test]
    fn test_bit_writer_two_9bit_zeros() {
        let mut w = BitWriter::new();
        w.write(0, 9);
        w.write(0, 9);
        w.flush();
        assert_eq!(w.into_bytes(), &[0x00, 0x00, 0x00]);
    }

    /// Write CLEAR and STOP, then read them back.
    #[test]
    fn test_bit_roundtrip_clear_and_stop() {
        let mut w = BitWriter::new();
        w.write(CLEAR_CODE, 9);
        w.write(STOP_CODE, 9);
        w.flush();
        let bytes = w.into_bytes();

        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read(9), Some(CLEAR_CODE));
        assert_eq!(r.read(9), Some(STOP_CODE));
    }

    /// Write a sequence of 9-bit codes at varying values and read them back.
    #[test]
    fn test_bit_roundtrip_sequence() {
        let codes: &[u16] = &[256, 65, 66, 258, 258, 257];
        let mut w = BitWriter::new();
        for &code in codes {
            w.write(code, 9);
        }
        w.flush();
        let bytes = w.into_bytes();

        let mut r = BitReader::new(&bytes);
        for &expected in codes {
            assert_eq!(r.read(9), Some(expected));
        }
    }

    /// Verify that BitReader returns None gracefully on empty input.
    #[test]
    fn test_bit_reader_empty_returns_none() {
        let mut r = BitReader::new(&[]);
        assert_eq!(r.read(9), None);
    }

    // ── Code-size growth ──────────────────────────────────────────────────────

    /// Encoding 512+ distinct sequences forces next_code past 512, which
    /// requires the encoder to grow code_size to 10. The round-trip must
    /// still succeed.
    #[test]
    fn test_code_size_grows_past_9_bits() {
        // 1024 bytes cycling through 256 values forces dict past 256 entries.
        let data: Vec<u8> = (0u8..=255).cycle().take(1024).collect();
        assert_eq!(rt(&data), data);
    }

    /// Stress test: 100 000 bytes of repeating pattern. Exercises dictionary
    /// growth and potential reset cycles.
    #[test]
    fn test_rt_stress_large_repeating() {
        let data: Vec<u8> = b"ABCDEFGHIJ".iter().cloned().cycle().take(100_000).collect();
        assert_eq!(rt(&data), data);
    }

    // ── BitReader edge cases (coverage for EOF paths) ─────────────────────────

    /// BitReader::read returns None when there are not enough bits to fill the
    /// requested width, even if a partial byte remains in the buffer.
    #[test]
    fn test_bit_reader_truncated_partial_byte() {
        // Write one 9-bit code (CLEAR=256), which produces 2 bytes after flush.
        // Then ask for a second 9-bit code — not enough data.
        let mut w = BitWriter::new();
        w.write(CLEAR_CODE, 9);
        w.flush();
        let bytes = w.into_bytes();

        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read(9), Some(CLEAR_CODE)); // first code: ok
        assert_eq!(r.read(9), None);             // second code: not enough bits
    }

    /// BitReader::read returns None immediately on truly empty input.
    #[test]
    fn test_bit_reader_one_byte_insufficient_for_9bits() {
        // One byte = 8 bits — not enough for a 9-bit code.
        let data = [0xFFu8];
        let mut r = BitReader::new(&data);
        assert_eq!(r.read(9), None);
    }

    // ── Truncated stream (no STOP_CODE) ──────────────────────────────────────

    /// A valid stream with the trailing STOP_CODE byte(s) removed should still
    /// decode correctly — the decoder breaks on `reader.read` returning None.
    #[test]
    fn test_decompress_truncated_stream_no_panic() {
        let compressed = compress(b"AB");
        // Drop the last 1–2 bytes to strip the STOP_CODE.
        if compressed.len() > 5 {
            let truncated = &compressed[..compressed.len() - 1];
            // Either decodes (with original_length truncation) or returns an
            // error — must not panic.
            let _ = decompress(truncated);
        }
    }

    // ── Second CLEAR_CODE in the main decode loop ─────────────────────────────

    /// Craft a stream that contains a mid-stream CLEAR_CODE (code 256) after
    /// the initial CLEAR. This exercises lines 376-380 (the CLEAR branch inside
    /// the main loop). The crafted stream encodes "AA":
    ///
    ///   CLEAR(256,9) | 65('A',9) | CLEAR(256,9) | 65('A',9) | STOP(257,9)
    ///
    /// The decoder resets its dict and state on the mid-stream CLEAR, then
    /// produces the second 'A', yielding "AA".
    #[test]
    fn test_decompress_mid_stream_clear_code() {
        let codes: &[(u16, u8)] = &[
            (CLEAR_CODE, 9),
            (65, 9),          // 'A'
            (CLEAR_CODE, 9),  // mid-stream reset
            (65, 9),          // 'A' again (fresh dict)
            (STOP_CODE, 9),
        ];
        let mut w = BitWriter::new();
        for &(code, size) in codes {
            w.write(code, size);
        }
        w.flush();
        let bit_bytes = w.into_bytes();

        // Build a CMP03-format payload: 4-byte header (original_length=2) + bits.
        let mut payload = Vec::new();
        payload.extend_from_slice(&2u32.to_be_bytes());
        payload.extend_from_slice(&bit_bytes);

        let result = decompress(&payload).unwrap();
        assert_eq!(result, b"AA");
    }
}
