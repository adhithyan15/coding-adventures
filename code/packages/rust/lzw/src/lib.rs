//! # LZW — CMP03
//!
//! LZW (Lempel-Ziv-Welch, 1984) lossless compression algorithm.
//! Part of the CMP compression series in the coding-adventures monorepo.
//!
//! ## What Is LZW?
//!
//! LZW is LZ78 with a pre-seeded dictionary: all 256 single-byte sequences are
//! added before encoding begins (codes 0–255). This eliminates LZ78's mandatory
//! `next_char` byte — every symbol is already in the dictionary, so the encoder
//! can emit pure codes.
//!
//! With only codes to transmit, LZW uses variable-width bit-packing: codes start
//! at 9 bits and grow as the dictionary expands. This is exactly how GIF works.
//!
//! ## Reserved Codes
//!
//! ```text
//! 0–255:  Pre-seeded single-byte entries.
//! 256:    CLEAR_CODE — reset to initial 256-entry state.
//! 257:    STOP_CODE  — end of code stream.
//! 258+:   Dynamically added entries.
//! ```
//!
//! ## Wire Format (CMP03)
//!
//! ```text
//! Bytes 0–3:  original_length (big-endian u32)
//! Bytes 4+:   bit-packed variable-width codes, LSB-first
//! ```
//!
//! ## The Tricky Token
//!
//! During decoding the decoder may receive code C == next_code (not yet added).
//! This happens when the input has the form xyx...x. The fix:
//!
//! ```text
//! entry = dict[prev_code].clone() + [dict[prev_code][0]]
//! ```
//!
//! ## The Series
//!
//! ```text
//! CMP00 (LZ77,    1977) — Sliding-window backreferences.
//! CMP01 (LZ78,    1978) — Explicit dictionary (trie).
//! CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
//! CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; GIF. (this crate)
//! CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//! CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
//! ```

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Reset code — instructs the decoder to clear its dictionary and restart.
pub const CLEAR_CODE: u32 = 256;

/// End-of-stream code — the decoder stops reading after this code.
pub const STOP_CODE: u32 = 257;

/// First dynamically assigned dictionary code.
pub const INITIAL_NEXT_CODE: u32 = 258;

/// Starting bit-width for codes (covers 0–511, more than enough for 258).
pub const INITIAL_CODE_SIZE: u32 = 9;

/// Maximum bit-width; dictionary caps at 2^16 = 65536 entries.
pub const MAX_CODE_SIZE: u32 = 16;

// ---------------------------------------------------------------------------
// Bit I/O
// ---------------------------------------------------------------------------

/// Accumulates variable-width codes into a byte vector, LSB-first.
///
/// Bits within each byte are filled from the least-significant end upward.
/// This matches the GIF and Unix `compress` conventions.
struct BitWriter {
    buf: u64,
    bit_pos: u32,
    out: Vec<u8>,
}

impl BitWriter {
    fn new() -> Self {
        BitWriter { buf: 0, bit_pos: 0, out: Vec::new() }
    }

    /// Write `code` using exactly `code_size` bits.
    fn write(&mut self, code: u32, code_size: u32) {
        self.buf |= (code as u64) << self.bit_pos;
        self.bit_pos += code_size;
        while self.bit_pos >= 8 {
            self.out.push((self.buf & 0xFF) as u8);
            self.buf >>= 8;
            self.bit_pos -= 8;
        }
    }

    fn flush(&mut self) {
        if self.bit_pos > 0 {
            self.out.push((self.buf & 0xFF) as u8);
            self.buf = 0;
            self.bit_pos = 0;
        }
    }
}

/// Reads variable-width codes from a byte slice, LSB-first.
struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    buf: u64,
    bit_pos: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader { data, pos: 0, buf: 0, bit_pos: 0 }
    }

    /// Read and return the next `code_size`-bit code.
    /// Returns `None` when the stream is exhausted.
    fn read(&mut self, code_size: u32) -> Option<u32> {
        while self.bit_pos < code_size {
            if self.pos >= self.data.len() {
                return None;
            }
            self.buf |= (self.data[self.pos] as u64) << self.bit_pos;
            self.pos += 1;
            self.bit_pos += 8;
        }
        let mask = (1u64 << code_size) - 1;
        let code = (self.buf & mask) as u32;
        self.buf >>= code_size;
        self.bit_pos -= code_size;
        Some(code)
    }

    fn exhausted(&self) -> bool {
        self.pos >= self.data.len() && self.bit_pos == 0
    }
}

// ---------------------------------------------------------------------------
// Encoder
// ---------------------------------------------------------------------------

/// Encode `data` into a Vec of LZW codes including CLEAR_CODE and STOP_CODE.
///
/// The encode dictionary maps byte sequences (stored as `Vec<u8>` keys) to
/// codes. Starting with all 256 single-byte entries, the encoder extends the
/// current prefix byte-by-byte. When the prefix + new byte is not in the
/// dict, the current prefix's code is emitted, the new sequence is added
/// (if room), and the prefix resets to just the new byte.
fn encode_codes(data: &[u8]) -> (Vec<u32>, usize) {
    let original_length = data.len();
    let mut enc_dict: std::collections::HashMap<Vec<u8>, u32> =
        std::collections::HashMap::with_capacity(512);
    for b in 0u8..=255 {
        enc_dict.insert(vec![b], b as u32);
    }

    let mut next_code = INITIAL_NEXT_CODE;
    let max_entries = 1u32 << MAX_CODE_SIZE;
    let mut codes = vec![CLEAR_CODE];
    let mut w: Vec<u8> = Vec::new();

    for &byte in data {
        let mut wb = w.clone();
        wb.push(byte);
        if enc_dict.contains_key(&wb) {
            w = wb;
        } else {
            codes.push(*enc_dict.get(&w).unwrap());

            if next_code < max_entries {
                enc_dict.insert(wb, next_code);
                next_code += 1;
            } else if next_code == max_entries {
                // Dictionary full — emit CLEAR and reset.
                codes.push(CLEAR_CODE);
                enc_dict.clear();
                for b in 0u8..=255 {
                    enc_dict.insert(vec![b], b as u32);
                }
                next_code = INITIAL_NEXT_CODE;
            }

            w = vec![byte];
        }
    }

    if !w.is_empty() {
        codes.push(*enc_dict.get(&w).unwrap());
    }
    codes.push(STOP_CODE);
    (codes, original_length)
}

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

/// Decode a slice of LZW codes back to a byte vector.
///
/// The decode dictionary is a `Vec<Vec<u8>>` indexed by code. New entries are
/// built as `dict[prev_code] + [entry[0]]`.
///
/// The tricky-token case (code == next_code) is handled by constructing the
/// missing entry from the previous entry extended by its own first byte.
fn decode_codes(codes: &[u32]) -> Vec<u8> {
    let mut dec_dict: Vec<Vec<u8>> = (0u8..=255).map(|b| vec![b]).collect();
    dec_dict.push(vec![]); // 256 = CLEAR placeholder
    dec_dict.push(vec![]); // 257 = STOP  placeholder

    let mut next_code = INITIAL_NEXT_CODE;
    let mut output: Vec<u8> = Vec::new();
    let mut prev_code: Option<u32> = None;

    for &code in codes {
        if code == CLEAR_CODE {
            dec_dict.truncate(258);
            for b in 0u8..=255 {
                dec_dict[b as usize] = vec![b];
            }
            dec_dict[256] = vec![];
            dec_dict[257] = vec![];
            next_code = INITIAL_NEXT_CODE;
            prev_code = None;
            continue;
        }

        if code == STOP_CODE {
            break;
        }

        let entry: Vec<u8>;

        if (code as usize) < dec_dict.len() {
            entry = dec_dict[code as usize].clone();
        } else if code == next_code {
            // Tricky token.
            match prev_code {
                None => continue, // malformed
                Some(prev) => {
                    let prev_entry = &dec_dict[prev as usize];
                    if prev_entry.is_empty() {
                        continue; // malformed
                    }
                    let first = prev_entry[0];
                    let mut e = prev_entry.clone();
                    e.push(first);
                    entry = e;
                }
            }
        } else {
            continue; // invalid code — skip
        }

        output.extend_from_slice(&entry);

        if let Some(prev) = prev_code {
            if next_code < (1 << MAX_CODE_SIZE) {
                let mut new_entry = dec_dict[prev as usize].clone();
                new_entry.push(entry[0]);
                dec_dict.push(new_entry);
                next_code += 1;
            }
        }

        prev_code = Some(code);
    }

    output
}

// ---------------------------------------------------------------------------
// Serialisation
// ---------------------------------------------------------------------------

/// Pack a Vec of LZW codes into the CMP03 wire format.
///
/// Header: 4-byte big-endian original_length.
/// Body:   LSB-first variable-width bit-packed codes.
fn pack_codes(codes: &[u32], original_length: usize) -> Vec<u8> {
    let mut writer = BitWriter::new();
    let mut code_size = INITIAL_CODE_SIZE;
    let mut next_code = INITIAL_NEXT_CODE;

    for &code in codes {
        writer.write(code, code_size);

        match code {
            CLEAR_CODE => {
                code_size = INITIAL_CODE_SIZE;
                next_code = INITIAL_NEXT_CODE;
            }
            STOP_CODE => {}
            _ => {
                if next_code < (1 << MAX_CODE_SIZE) {
                    next_code += 1;
                    if next_code > (1 << code_size) && code_size < MAX_CODE_SIZE {
                        code_size += 1;
                    }
                }
            }
        }
    }
    writer.flush();

    let mut out = Vec::with_capacity(4 + writer.out.len());
    out.extend_from_slice(&(original_length as u32).to_be_bytes());
    out.extend_from_slice(&writer.out);
    out
}

/// Unpack CMP03 wire-format bytes into a Vec of LZW codes.
///
/// Returns `(codes, original_length)`. Stops at STOP_CODE or stream exhaustion.
fn unpack_codes(data: &[u8]) -> (Vec<u32>, usize) {
    if data.len() < 4 {
        return (vec![CLEAR_CODE, STOP_CODE], 0);
    }

    let original_length = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let mut reader = BitReader::new(&data[4..]);

    let mut codes = Vec::new();
    let mut code_size = INITIAL_CODE_SIZE;
    let mut next_code = INITIAL_NEXT_CODE;

    while !reader.exhausted() {
        let Some(code) = reader.read(code_size) else { break };
        codes.push(code);

        match code {
            STOP_CODE => break,
            CLEAR_CODE => {
                code_size = INITIAL_CODE_SIZE;
                next_code = INITIAL_NEXT_CODE;
            }
            _ => {
                if next_code < (1 << MAX_CODE_SIZE) {
                    next_code += 1;
                    if next_code > (1 << code_size) && code_size < MAX_CODE_SIZE {
                        code_size += 1;
                    }
                }
            }
        }
    }

    (codes, original_length)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compress `data` using LZW and return CMP03 wire-format bytes.
pub fn compress(data: &[u8]) -> Vec<u8> {
    let (codes, original_length) = encode_codes(data);
    pack_codes(&codes, original_length)
}

/// Decompress CMP03 wire-format `data` and return the original bytes.
pub fn decompress(data: &[u8]) -> Vec<u8> {
    let (codes, original_length) = unpack_codes(data);
    let mut result = decode_codes(&codes);
    result.truncate(original_length);
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(data: &[u8]) -> Vec<u8> {
        decompress(&compress(data))
    }

    // -- Constants

    #[test]
    fn test_constants() {
        assert_eq!(CLEAR_CODE, 256);
        assert_eq!(STOP_CODE, 257);
        assert_eq!(INITIAL_NEXT_CODE, 258);
        assert_eq!(INITIAL_CODE_SIZE, 9);
        assert_eq!(MAX_CODE_SIZE, 16);
    }

    // -- encode_codes

    #[test]
    fn test_encode_empty() {
        let (codes, orig) = encode_codes(b"");
        assert_eq!(orig, 0);
        assert_eq!(codes, vec![CLEAR_CODE, STOP_CODE]);
    }

    #[test]
    fn test_encode_single_byte() {
        let (codes, orig) = encode_codes(b"A");
        assert_eq!(orig, 1);
        assert_eq!(codes[0], CLEAR_CODE);
        assert_eq!(*codes.last().unwrap(), STOP_CODE);
        assert!(codes.contains(&65));
    }

    #[test]
    fn test_encode_two_distinct() {
        let (codes, _) = encode_codes(b"AB");
        assert_eq!(codes, vec![CLEAR_CODE, 65, 66, STOP_CODE]);
    }

    #[test]
    fn test_encode_repeated_pair() {
        let (codes, _) = encode_codes(b"ABABAB");
        assert_eq!(codes, vec![CLEAR_CODE, 65, 66, 258, 258, STOP_CODE]);
    }

    #[test]
    fn test_encode_all_same() {
        let (codes, _) = encode_codes(b"AAAAAAA");
        assert_eq!(codes, vec![CLEAR_CODE, 65, 258, 259, 65, STOP_CODE]);
    }

    // -- decode_codes

    #[test]
    fn test_decode_empty_stream() {
        assert!(decode_codes(&[CLEAR_CODE, STOP_CODE]).is_empty());
    }

    #[test]
    fn test_decode_single_byte() {
        assert_eq!(decode_codes(&[CLEAR_CODE, 65, STOP_CODE]), b"A");
    }

    #[test]
    fn test_decode_two_distinct() {
        assert_eq!(decode_codes(&[CLEAR_CODE, 65, 66, STOP_CODE]), b"AB");
    }

    #[test]
    fn test_decode_repeated_pair() {
        let result = decode_codes(&[CLEAR_CODE, 65, 66, 258, 258, STOP_CODE]);
        assert_eq!(result, b"ABABAB");
    }

    #[test]
    fn test_decode_tricky_token() {
        let result = decode_codes(&[CLEAR_CODE, 65, 258, 259, 65, STOP_CODE]);
        assert_eq!(result, b"AAAAAAA");
    }

    #[test]
    fn test_decode_clear_mid_stream() {
        let result = decode_codes(&[CLEAR_CODE, 65, CLEAR_CODE, 66, STOP_CODE]);
        assert_eq!(result, b"AB");
    }

    #[test]
    fn test_decode_invalid_code_skipped() {
        let result = decode_codes(&[CLEAR_CODE, 9999, 65, STOP_CODE]);
        assert_eq!(result, b"A");
    }

    // -- pack / unpack

    #[test]
    fn test_header_original_length() {
        let packed = pack_codes(&[CLEAR_CODE, STOP_CODE], 42);
        let stored = u32::from_be_bytes([packed[0], packed[1], packed[2], packed[3]]);
        assert_eq!(stored, 42);
    }

    #[test]
    fn test_roundtrip_pack_unpack_ababab() {
        let codes = vec![CLEAR_CODE, 65, 66, 258, 258, STOP_CODE];
        let packed = pack_codes(&codes, 6);
        let (unpacked, orig) = unpack_codes(&packed);
        assert_eq!(orig, 6);
        assert_eq!(unpacked, codes);
    }

    #[test]
    fn test_roundtrip_pack_unpack_all_same() {
        let codes = vec![CLEAR_CODE, 65, 258, 259, 65, STOP_CODE];
        let packed = pack_codes(&codes, 7);
        let (unpacked, orig) = unpack_codes(&packed);
        assert_eq!(orig, 7);
        assert_eq!(unpacked, codes);
    }

    #[test]
    fn test_unpack_truncated() {
        let (codes, orig) = unpack_codes(&[0u8, 0u8]);
        assert!(codes.is_empty() || codes.contains(&CLEAR_CODE));
        assert_eq!(orig, 0);
    }

    // -- compress / decompress

    #[test]
    fn test_empty() { assert_eq!(rt(b""), b""); }

    #[test]
    fn test_single_byte() { assert_eq!(rt(b"A"), b"A"); }

    #[test]
    fn test_two_distinct() { assert_eq!(rt(b"AB"), b"AB"); }

    #[test]
    fn test_repeated_pair() { assert_eq!(rt(b"ABABAB"), b"ABABAB"); }

    #[test]
    fn test_all_same_bytes() {
        let data = b"AAAAAAA";
        assert_eq!(rt(data), data);
    }

    #[test]
    fn test_long_repetitive() {
        let data = b"ABCABC".repeat(200);
        assert_eq!(rt(&data), data);
    }

    #[test]
    fn test_binary_data() {
        let data: Vec<u8> = (0..=255u8).cycle().take(512).collect();
        assert_eq!(rt(&data), data);
    }

    #[test]
    fn test_all_zeros() {
        let data = vec![0u8; 100];
        assert_eq!(rt(&data), data);
    }

    #[test]
    fn test_all_ff() {
        let data = vec![0xFFu8; 100];
        assert_eq!(rt(&data), data);
    }

    #[test]
    fn test_aababc() { assert_eq!(rt(b"AABABC"), b"AABABC"); }

    #[test]
    fn test_compresses_repetitive() {
        let data = b"ABCABC".repeat(100);
        let compressed = compress(&data);
        assert!(compressed.len() < data.len(), "expected compression");
    }

    #[test]
    fn test_header_stored_length() {
        let data = b"hello world";
        let compressed = compress(data);
        let stored = u32::from_be_bytes([compressed[0], compressed[1], compressed[2], compressed[3]]);
        assert_eq!(stored as usize, data.len());
    }
}
