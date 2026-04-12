//! LZSS lossless compression algorithm (1982) — CMP02.
//!
//! LZSS (Lempel-Ziv-Storer-Szymanski) refines LZ77 by replacing the mandatory
//! `next_char` byte after every token with a flag-bit scheme:
//!
//! - `Literal` → 1 byte  (flag bit = 0)
//! - `Match`   → 3 bytes (flag bit = 1: offset u16 BE + length u8)
//!
//! Tokens are grouped in blocks of 8. Each block starts with a flag byte
//! (bit 0 = first token, bit 7 = eighth token).
//!
//! # Wire Format (CMP02)
//!
//! ```text
//! Bytes 0–3:  original_length (big-endian u32)
//! Bytes 4–7:  block_count     (big-endian u32)
//! Bytes 8+:   blocks
//!   Each block: [1-byte flag] [1 or 3 bytes per symbol]
//! ```
//!
//! # Series
//!
//! ```text
//! CMP00 (LZ77, 1977) — Sliding-window backreferences.
//! CMP01 (LZ78, 1978) — Explicit dictionary (trie).
//! CMP02 (LZSS, 1982) — LZ77 + flag bits. ← this crate
//! CMP03 (LZW,  1984) — LZ78 + pre-initialised alphabet; GIF.
//! CMP04 (Huffman, 1952) — Entropy coding.
//! CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
//! ```
//!
//! # Examples
//!
//! ```
//! use lzss::{compress, decompress};
//!
//! let data = b"hello hello hello";
//! let compressed = compress(data);
//! assert_eq!(decompress(&compressed), data);
//! ```

// ─── Token types ─────────────────────────────────────────────────────────────

/// A single LZSS token: either a literal byte or a back-reference match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// A single literal byte (no match in search buffer).
    Literal(u8),
    /// A back-reference: copy `length` bytes from `offset` positions back.
    Match {
        /// Distance back in the output where the match starts (1..window_size).
        offset: u16,
        /// Number of bytes to copy (min_match..max_match).
        length: u8,
    },
}

/// Default sliding-window size (matches CMP02 spec).
pub const DEFAULT_WINDOW_SIZE: usize = 4096;
/// Default maximum match length (fits in u8).
pub const DEFAULT_MAX_MATCH: usize = 255;
/// Default minimum match length for a Match token (break-even at 3 bytes).
pub const DEFAULT_MIN_MATCH: usize = 3;

// ─── Sliding-window encoder ───────────────────────────────────────────────────

/// Find the longest match for `data[cursor..]` in `data[win_start..cursor]`.
///
/// Returns `(best_offset, best_length)`. Matches may overlap (extend past
/// cursor) to allow run-length encoding as a degenerate case.
fn find_longest_match(
    data: &[u8],
    cursor: usize,
    win_start: usize,
    max_match: usize,
) -> (u16, u8) {
    let mut best_len = 0usize;
    let mut best_off = 0usize;
    let lookahead_end = (cursor + max_match).min(data.len());

    for pos in win_start..cursor {
        let mut len = 0;
        while cursor + len < lookahead_end && data[pos + len] == data[cursor + len] {
            len += 1;
        }
        if len > best_len {
            best_len = len;
            best_off = cursor - pos;
        }
    }

    (best_off as u16, best_len as u8)
}

/// Encode bytes into an LZSS token stream.
///
/// At each cursor position, searches the last `window_size` bytes for the
/// longest match. If the match is at least `min_match` bytes, emits a
/// [`Token::Match`] and advances by that length. Otherwise emits a
/// [`Token::Literal`] and advances by 1.
///
/// # Examples
///
/// ```
/// use lzss::{Token, encode, DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH};
///
/// let tokens = encode(b"ABABAB", DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH);
/// assert_eq!(tokens[0], Token::Literal(b'A'));
/// assert_eq!(tokens[1], Token::Literal(b'B'));
/// assert!(matches!(tokens[2], Token::Match { offset: 2, length: 4 }));
/// ```
pub fn encode(data: &[u8], window_size: usize, max_match: usize, min_match: usize) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut cursor = 0;

    while cursor < data.len() {
        let win_start = cursor.saturating_sub(window_size);
        let (offset, length) = find_longest_match(data, cursor, win_start, max_match);

        if length as usize >= min_match {
            tokens.push(Token::Match { offset, length });
            cursor += length as usize;
        } else {
            tokens.push(Token::Literal(data[cursor]));
            cursor += 1;
        }
    }

    tokens
}

// ─── Decoder ─────────────────────────────────────────────────────────────────

/// Decode an LZSS token stream back into the original bytes.
///
/// For each `Literal`, appends the byte. For each `Match`, copies `length`
/// bytes from `offset` positions back, byte-by-byte for overlap safety.
///
/// `original_length` truncates output if ≥ 0; pass `None` to return all.
///
/// # Examples
///
/// ```
/// use lzss::{Token, decode};
///
/// // "A" + Match(offset=1, length=6) → "AAAAAAA"
/// let tokens = vec![Token::Literal(b'A'), Token::Match { offset: 1, length: 6 }];
/// assert_eq!(decode(&tokens, Some(7)), b"AAAAAAA");
/// ```
pub fn decode(tokens: &[Token], original_length: Option<usize>) -> Vec<u8> {
    let capacity = original_length.unwrap_or(tokens.len() * 4);
    let mut output = Vec::with_capacity(capacity);

    for tok in tokens {
        match tok {
            Token::Literal(b) => output.push(*b),
            Token::Match { offset, length } => {
                let start = output.len() - *offset as usize;
                for i in 0..*length as usize {
                    let byte = output[start + i];
                    output.push(byte);
                }
            }
        }
    }

    if let Some(n) = original_length {
        output.truncate(n);
    }
    output
}

// ─── Serialisation ───────────────────────────────────────────────────────────

/// Serialise an LZSS token list to the CMP02 wire format.
///
/// Header: `original_length` (BE u32) + `block_count` (BE u32).
/// Then `block_count` blocks: [1-byte flag] + symbol data.
pub fn serialise_tokens(tokens: &[Token], original_length: usize) -> Vec<u8> {
    let mut blocks: Vec<Vec<u8>> = Vec::new();

    for chunk in tokens.chunks(8) {
        let mut flag = 0u8;
        let mut symbol_data = Vec::new();

        for (bit, tok) in chunk.iter().enumerate() {
            match tok {
                Token::Match { offset, length } => {
                    flag |= 1 << bit;
                    symbol_data.push((offset >> 8) as u8);
                    symbol_data.push(*offset as u8);
                    symbol_data.push(*length);
                }
                Token::Literal(b) => {
                    symbol_data.push(*b);
                }
            }
        }

        let mut block = vec![flag];
        block.extend(symbol_data);
        blocks.push(block);
    }

    let total_body: usize = blocks.iter().map(|b| b.len()).sum();
    let mut buf = Vec::with_capacity(8 + total_body);

    buf.extend_from_slice(&(original_length as u32).to_be_bytes());
    buf.extend_from_slice(&(blocks.len() as u32).to_be_bytes());
    for block in blocks {
        buf.extend(block);
    }

    buf
}

/// Deserialise CMP02 wire-format bytes into tokens and original length.
///
/// Security: `block_count` is capped against actual payload size to prevent
/// DoS from a crafted header claiming more blocks than data can hold.
pub fn deserialise_tokens(data: &[u8]) -> (Vec<Token>, usize) {
    if data.len() < 8 {
        return (Vec::new(), 0);
    }

    let original_length = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let mut block_count = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;

    // 1 byte minimum per block — cap to prevent DoS.
    let max_possible = data.len() - 8;
    if block_count > max_possible {
        block_count = max_possible;
    }

    let mut tokens = Vec::new();
    let mut pos = 8usize;

    for _ in 0..block_count {
        if pos >= data.len() {
            break;
        }
        let flag = data[pos];
        pos += 1;

        for bit in 0..8 {
            if pos >= data.len() {
                break;
            }
            if flag & (1 << bit) != 0 {
                // Match: 3 bytes
                if pos + 3 > data.len() {
                    break;
                }
                let offset = u16::from_be_bytes([data[pos], data[pos + 1]]);
                let length = data[pos + 2];
                tokens.push(Token::Match { offset, length });
                pos += 3;
            } else {
                // Literal: 1 byte
                tokens.push(Token::Literal(data[pos]));
                pos += 1;
            }
        }
    }

    (tokens, original_length)
}

// ─── One-shot API ─────────────────────────────────────────────────────────────

/// Compress bytes using LZSS, returning the CMP02 wire format.
///
/// # Examples
///
/// ```
/// use lzss::{compress, decompress};
///
/// let original = b"AAAAAAA";
/// assert_eq!(decompress(&compress(original)), original);
/// ```
pub fn compress(data: &[u8]) -> Vec<u8> {
    let tokens = encode(data, DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH);
    serialise_tokens(&tokens, data.len())
}

/// Decompress bytes produced by [`compress`].
pub fn decompress(data: &[u8]) -> Vec<u8> {
    let (tokens, original_length) = deserialise_tokens(data);
    decode(&tokens, Some(original_length))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(data: &[u8]) -> Vec<u8> {
        decompress(&compress(data))
    }

    // ── Spec vectors ──────────────────────────────────────────────────────────

    #[test]
    fn test_encode_empty() {
        assert!(encode(b"", DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH).is_empty());
    }

    #[test]
    fn test_encode_single_byte() {
        assert_eq!(
            encode(b"A", DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH),
            vec![Token::Literal(b'A')]
        );
    }

    #[test]
    fn test_encode_no_repetition() {
        let tokens = encode(b"ABCDE", DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH);
        assert_eq!(tokens.len(), 5);
        assert!(tokens.iter().all(|t| matches!(t, Token::Literal(_))));
    }

    #[test]
    fn test_encode_aabcbbabc() {
        let tokens = encode(b"AABCBBABC", DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH);
        assert_eq!(tokens.len(), 7);
        assert_eq!(tokens[6], Token::Match { offset: 5, length: 3 });
    }

    #[test]
    fn test_encode_ababab() {
        let tokens = encode(b"ABABAB", DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH);
        assert_eq!(tokens, vec![
            Token::Literal(b'A'),
            Token::Literal(b'B'),
            Token::Match { offset: 2, length: 4 },
        ]);
    }

    #[test]
    fn test_encode_all_identical() {
        let tokens = encode(b"AAAAAAA", DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH);
        assert_eq!(tokens, vec![
            Token::Literal(b'A'),
            Token::Match { offset: 1, length: 6 },
        ]);
    }

    // ── Encode properties ─────────────────────────────────────────────────────

    #[test]
    fn test_match_offset_positive() {
        for tok in encode(b"ABABABAB", DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH) {
            if let Token::Match { offset, .. } = tok {
                assert!(offset >= 1);
            }
        }
    }

    #[test]
    fn test_match_length_ge_min() {
        for tok in encode(b"ABABABABABAB", DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH) {
            if let Token::Match { length, .. } = tok {
                assert!(length >= DEFAULT_MIN_MATCH as u8);
            }
        }
    }

    #[test]
    fn test_match_offset_within_window() {
        let ws = 8;
        for tok in encode(b"ABCABCABCABC", ws, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH) {
            if let Token::Match { offset, .. } = tok {
                assert!(offset as usize <= ws);
            }
        }
    }

    #[test]
    fn test_match_length_within_max() {
        let max = 5;
        for tok in encode(&vec![b'A'; 100], DEFAULT_WINDOW_SIZE, max, DEFAULT_MIN_MATCH) {
            if let Token::Match { length, .. } = tok {
                assert!(length as usize <= max);
            }
        }
    }

    #[test]
    fn test_min_match_large_forces_literals() {
        let tokens = encode(b"ABABAB", DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, 100);
        assert!(tokens.iter().all(|t| matches!(t, Token::Literal(_))));
    }

    // ── Decode ────────────────────────────────────────────────────────────────

    #[test]
    fn test_decode_empty() {
        assert!(decode(&[], Some(0)).is_empty());
    }

    #[test]
    fn test_decode_single_literal() {
        assert_eq!(decode(&[Token::Literal(b'A')], Some(1)), b"A");
    }

    #[test]
    fn test_decode_overlapping_match() {
        let tokens = vec![Token::Literal(b'A'), Token::Match { offset: 1, length: 6 }];
        assert_eq!(decode(&tokens, Some(7)), b"AAAAAAA");
    }

    #[test]
    fn test_decode_ababab() {
        let tokens = vec![
            Token::Literal(b'A'), Token::Literal(b'B'),
            Token::Match { offset: 2, length: 4 },
        ];
        assert_eq!(decode(&tokens, Some(6)), b"ABABAB");
    }

    // ── Round-trip ────────────────────────────────────────────────────────────

    #[test]
    fn test_rt_empty()         { assert_eq!(rt(b""), b""); }
    #[test]
    fn test_rt_single()        { assert_eq!(rt(b"A"), b"A"); }
    #[test]
    fn test_rt_no_repetition() { assert_eq!(rt(b"ABCDE"), b"ABCDE"); }
    #[test]
    fn test_rt_all_identical() { assert_eq!(rt(b"AAAAAAA"), b"AAAAAAA"); }
    #[test]
    fn test_rt_ababab()        { assert_eq!(rt(b"ABABAB"), b"ABABAB"); }
    #[test]
    fn test_rt_aabcbbabc()     { assert_eq!(rt(b"AABCBBABC"), b"AABCBBABC"); }
    #[test]
    fn test_rt_hello_world()   { assert_eq!(rt(b"hello world"), b"hello world"); }
    #[test]
    fn test_rt_binary_nulls()  { assert_eq!(rt(&[0, 0, 0, 255, 255]), &[0, 0, 0, 255, 255]); }

    #[test]
    fn test_rt_full_byte_range() {
        let data: Vec<u8> = (0u8..=255).collect();
        assert_eq!(rt(&data), data);
    }

    #[test]
    fn test_rt_repeated_pattern() {
        let data: Vec<u8> = (0..300_usize).map(|i| (i % 3) as u8).collect();
        assert_eq!(rt(&data), data);
    }

    #[test]
    fn test_rt_long() {
        let data: Vec<u8> = b"ABCDEF".iter().cloned().cycle().take(3000).collect();
        assert_eq!(rt(&data), data);
    }

    #[test]
    fn test_rt_all_same() {
        let data = vec![0x42u8; 10000];
        assert_eq!(rt(&data), data);
    }

    // ── Wire format ───────────────────────────────────────────────────────────

    #[test]
    fn test_compress_stores_original_length() {
        let compressed = compress(b"hello");
        let stored = u32::from_be_bytes([compressed[0], compressed[1], compressed[2], compressed[3]]);
        assert_eq!(stored, 5);
    }

    #[test]
    fn test_compress_deterministic() {
        assert_eq!(compress(b"hello world test"), compress(b"hello world test"));
    }

    #[test]
    fn test_crafted_large_block_count_safe() {
        let mut bad = vec![0u8; 16];
        bad[4] = 0x40; // block_count = 2^30
        let result = decompress(&bad);
        let _ = result; // must not panic
    }

    // ── Compression effectiveness ─────────────────────────────────────────────

    #[test]
    fn test_repetitive_data_compresses() {
        let data: Vec<u8> = b"ABC".iter().cloned().cycle().take(3000).collect();
        assert!(compress(&data).len() < data.len());
    }

    #[test]
    fn test_all_same_byte_compresses() {
        let data = vec![0x42u8; 10000];
        let compressed = compress(&data);
        assert!(compressed.len() < data.len());
        assert_eq!(decompress(&compressed), data);
    }
}
