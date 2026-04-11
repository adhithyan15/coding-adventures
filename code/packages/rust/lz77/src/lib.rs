//! # lz77
//!
//! LZ77 lossless compression algorithm (Lempel & Ziv, 1977). Part of the CMP
//! compression series in the coding-adventures monorepo.
//!
//! ## What Is LZ77?
//!
//! LZ77 replaces repeated byte sequences with compact backreferences into a
//! sliding window of recently seen data. It is the foundation of DEFLATE,
//! gzip, PNG, and zlib.
//!
//! ## The Sliding Window Model
//!
//! ```text
//! ┌─────────────────────────────────┬──────────────────┐
//! │         SEARCH BUFFER           │ LOOKAHEAD BUFFER  │
//! │  (already processed — the       │  (not yet seen —  │
//! │   last window_size bytes)       │  next max_match)  │
//! └─────────────────────────────────┴──────────────────┘
//!                                    ↑
//!                                cursor (current position)
//! ```
//!
//! At each step the encoder finds the longest match in the search buffer.
//! If found and long enough (≥ min_match), emit a backreference token.
//! Otherwise emit a literal token.
//!
//! ## Token: (offset, length, next_char)
//!
//! - `offset`:    distance back the match starts (1..window_size), or 0.
//! - `length`:    number of bytes the match covers (0 = literal).
//! - `next_char`: literal byte immediately after the match.
//!
//! ## Overlapping Matches
//!
//! When `offset < length` the match extends into bytes not yet decoded.
//! The decoder copies byte-by-byte (not bulk) to handle this correctly.
//!
//! ## Usage
//!
//! ```rust
//! use lz77::{compress, decompress, encode, decode};
//!
//! let data = b"hello hello hello world";
//! let compressed = compress(data, 4096, 255, 3);
//! assert_eq!(decompress(&compressed), data);
//! ```
//!
//! ## The Series: CMP00 → CMP05
//!
//! - CMP00 (LZ77, 1977) — Sliding-window backreferences. This crate.
//! - CMP01 (LZ78, 1978) — Explicit dictionary (trie), no sliding window.
//! - CMP02 (LZSS, 1982) — LZ77 + flag bits; eliminates wasted literals.
//! - CMP03 (LZW,  1984) — Pre-initialized dictionary; powers GIF.
//! - CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//! - CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.

/// A single LZ77 token: `(offset, length, next_char)`.
///
/// Represents one unit of the compressed stream.
///
/// - `offset`:    distance back the match starts (1..window_size), or 0.
/// - `length`:    number of bytes the match covers (0 = no match).
/// - `next_char`: literal byte immediately after the match (0..255).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub offset: u16,
    pub length: u8,
    pub next_char: u8,
}

impl Token {
    /// Creates a new `Token`.
    pub fn new(offset: u16, length: u8, next_char: u8) -> Self {
        Token { offset, length, next_char }
    }
}

/// Finds the longest match in the search buffer.
///
/// Scans the last `window_size` bytes before `cursor` for the longest
/// substring that matches the start of the lookahead buffer.
///
/// Returns `(best_offset, best_length)` both 0 if no match found.
fn find_longest_match(
    data: &[u8],
    cursor: usize,
    window_size: usize,
    max_match: usize,
) -> (usize, usize) {
    let mut best_offset = 0;
    let mut best_length = 0;

    // The search buffer starts at most window_size bytes back.
    let search_start = cursor.saturating_sub(window_size);

    // The lookahead cannot extend past the end of input.
    // Reserve 1 byte for next_char.
    let lookahead_end = (cursor + max_match).min(data.len().saturating_sub(1));

    for pos in search_start..cursor {
        let mut length = 0;
        // Match byte by byte. Matches may overlap (extend past cursor).
        while cursor + length < lookahead_end
            && data[pos + length] == data[cursor + length]
        {
            length += 1;
        }
        if length > best_length {
            best_length = length;
            best_offset = cursor - pos; // Distance back from cursor.
        }
    }

    (best_offset, best_length)
}

/// Encodes data into an LZ77 token stream.
///
/// Scans the input left-to-right. For each position, finds the longest
/// match in the search buffer. If the match is long enough (≥ min_match),
/// emits a backreference token; otherwise emits a literal token.
///
/// # Parameters
///
/// - `window_size`: maximum lookback distance (default 4096).
/// - `max_match`:   maximum match length (default 255).
/// - `min_match`:   minimum match length for a backreference (default 3).
///
/// # Examples
///
/// ```rust
/// use lz77::encode;
/// let tokens = encode(b"ABABABAB", 4096, 255, 3);
/// assert_eq!(tokens.len(), 3); // Two literals + one backreference.
/// ```
pub fn encode(
    data: &[u8],
    window_size: usize,
    max_match: usize,
    min_match: usize,
) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut cursor = 0;

    while cursor < data.len() {
        // Edge case: last byte has no room for next_char after a match.
        if cursor == data.len() - 1 {
            tokens.push(Token::new(0, 0, data[cursor]));
            cursor += 1;
            continue;
        }

        let (offset, length) = find_longest_match(data, cursor, window_size, max_match);

        if length >= min_match {
            // Emit a backreference token.
            let next_char = data[cursor + length];
            tokens.push(Token::new(offset as u16, length as u8, next_char));
            cursor += length + 1;
        } else {
            // Emit a literal token (no match or too short).
            tokens.push(Token::new(0, 0, data[cursor]));
            cursor += 1;
        }
    }

    tokens
}

/// Decodes an LZ77 token stream back into the original data.
///
/// Processes each token: if `length > 0`, copies `length` bytes byte-by-byte
/// from the search buffer (handling overlapping matches), then appends
/// `next_char`.
///
/// # Parameters
///
/// - `initial_buffer`: optional seed for the search buffer (streaming use).
///
/// # Examples
///
/// ```rust
/// use lz77::{Token, decode};
/// let tokens = vec![Token::new(0, 0, 65), Token::new(1, 3, 68)];
/// // Token(1,3,68): start=0, copy output[0..3] → AAA, then append D
/// assert_eq!(decode(&tokens, &[]), b"AAAAD");
/// ```
pub fn decode(tokens: &[Token], initial_buffer: &[u8]) -> Vec<u8> {
    let mut output: Vec<u8> = initial_buffer.to_vec();

    for token in tokens {
        if token.length > 0 {
            // Copy length bytes from position (output.len() - offset).
            let start = output.len() - token.offset as usize;
            // Copy byte-by-byte to handle overlapping matches (offset < length).
            for i in 0..token.length as usize {
                let byte = output[start + i];
                output.push(byte);
            }
        }
        // Always append next_char.
        output.push(token.next_char);
    }

    output
}

/// Serialises a token list to bytes using a fixed-width format.
///
/// Format:
/// - 4 bytes: token count (big-endian u32)
/// - N × 4 bytes: each token as `(offset: u16 BE, length: u8, next_char: u8)`
///
/// This is a teaching format. Production compressors use variable-width
/// bit-packing (see DEFLATE, zstd).
pub fn serialise_tokens(tokens: &[Token]) -> Vec<u8> {
    // 4-byte header + 4 bytes per token.
    let mut buf = Vec::with_capacity(4 + tokens.len() * 4);

    // Write token count as big-endian u32.
    buf.extend_from_slice(&(tokens.len() as u32).to_be_bytes());

    for token in tokens {
        buf.extend_from_slice(&token.offset.to_be_bytes());
        buf.push(token.length);
        buf.push(token.next_char);
    }

    buf
}

/// Deserialises bytes back into a token list.
///
/// Inverse of `serialise_tokens`.
pub fn deserialise_tokens(data: &[u8]) -> Vec<Token> {
    if data.len() < 4 {
        return Vec::new();
    }

    let count = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let mut tokens = Vec::with_capacity(count);

    for i in 0..count {
        let base = 4 + i * 4;
        if base + 4 > data.len() {
            break;
        }
        let offset = u16::from_be_bytes([data[base], data[base + 1]]);
        let length = data[base + 2];
        let next_char = data[base + 3];
        tokens.push(Token::new(offset, length, next_char));
    }

    tokens
}

/// Compresses data using LZ77.
///
/// One-shot API: `encode` then serialise the token stream to bytes.
///
/// # Examples
///
/// ```rust
/// use lz77::{compress, decompress};
/// let original = b"AAAAAAA";
/// let compressed = compress(original, 4096, 255, 3);
/// assert_eq!(decompress(&compressed), original);
/// ```
pub fn compress(
    data: &[u8],
    window_size: usize,
    max_match: usize,
    min_match: usize,
) -> Vec<u8> {
    let tokens = encode(data, window_size, max_match, min_match);
    serialise_tokens(&tokens)
}

/// Decompresses data that was compressed with `compress`.
///
/// Deserialises the byte stream into tokens, then decodes.
pub fn decompress(data: &[u8]) -> Vec<u8> {
    let tokens = deserialise_tokens(data);
    decode(&tokens, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Specification Test Vectors ----

    #[test]
    fn test_empty_input() {
        assert!(encode(&[], 4096, 255, 3).is_empty());
        assert!(decode(&[], &[]).is_empty());
    }

    #[test]
    fn test_no_repetition() {
        // "ABCDE" — no repeated substrings → all literal tokens.
        let tokens = encode(b"ABCDE", 4096, 255, 3);
        assert_eq!(tokens.len(), 5);
        for t in &tokens {
            assert_eq!(t.offset, 0);
            assert_eq!(t.length, 0);
        }
    }

    #[test]
    fn test_all_identical_bytes() {
        // "AAAAAAA" → literal A + backreference (offset=1, length=5, next_char=A).
        let tokens = encode(b"AAAAAAA", 4096, 255, 3);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], Token::new(0, 0, b'A'));
        assert_eq!(tokens[1].offset, 1);
        assert_eq!(tokens[1].length, 5);
        assert_eq!(tokens[1].next_char, b'A');
        assert_eq!(decode(&tokens, &[]), b"AAAAAAA");
    }

    #[test]
    fn test_repeated_pair() {
        // "ABABABAB" → [A literal, B literal, (offset=2, length=5, next_char='B')].
        let tokens = encode(b"ABABABAB", 4096, 255, 3);
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::new(0, 0, b'A'));
        assert_eq!(tokens[1], Token::new(0, 0, b'B'));
        assert_eq!(tokens[2].offset, 2);
        assert_eq!(tokens[2].length, 5);
        assert_eq!(tokens[2].next_char, b'B');
        assert_eq!(decode(&tokens, &[]), b"ABABABAB");
    }

    #[test]
    fn test_substring_reuse_no_match() {
        // "AABCBBABC" with min_match=3 → all literals.
        let tokens = encode(b"AABCBBABC", 4096, 255, 3);
        assert_eq!(tokens.len(), 9);
        for t in &tokens {
            assert_eq!(t.offset, 0);
            assert_eq!(t.length, 0);
        }
        assert_eq!(decode(&tokens, &[]), b"AABCBBABC");
    }

    #[test]
    fn test_substring_reuse_lower_min_match() {
        let tokens = encode(b"AABCBBABC", 4096, 255, 2);
        assert_eq!(decode(&tokens, &[]), b"AABCBBABC");
    }

    // ---- Round-Trip Tests ----

    #[test]
    fn test_round_trip_various() {
        let cases: &[&[u8]] = &[
            b"",
            b"A",
            b"\x00",
            b"\xff",
            b"hello world",
            b"the quick brown fox",
            b"ababababab",
            b"aaaaaaaaaa",
            b"\x00\x00\x00",
            b"\xff\xff\xff",
            b"\x00\x01\x02\x00\x01\x02",
        ];
        for data in cases {
            let tokens = encode(data, 4096, 255, 3);
            assert_eq!(&decode(&tokens, &[]), data, "round-trip failed for {:?}", data);
        }
    }

    #[test]
    fn test_all_256_bytes() {
        let data: Vec<u8> = (0..=255).collect();
        let tokens = encode(&data, 4096, 255, 3);
        assert_eq!(decode(&tokens, &[]), data);
    }

    #[test]
    fn test_compress_decompress_round_trip() {
        let cases: &[&[u8]] = &[b"", b"A", b"ABCDE", b"AAAAAAA", b"ABABABAB", b"hello world"];
        for data in cases {
            assert_eq!(&decompress(&compress(data, 4096, 255, 3)), data);
        }
    }

    // ---- Parameter Tests ----

    #[test]
    fn test_window_size_limit() {
        let mut data = vec![b'X'];
        data.extend(vec![b'Y'; 5000]);
        data.push(b'X');
        let tokens = encode(&data, 100, 255, 3);
        for t in &tokens {
            assert!(t.offset <= 100, "offset {} exceeds window_size 100", t.offset);
        }
    }

    #[test]
    fn test_max_match_limit() {
        let data = vec![b'A'; 1000];
        let tokens = encode(&data, 4096, 50, 3);
        for t in &tokens {
            assert!(t.length <= 50, "length {} exceeds max_match 50", t.length);
        }
    }

    #[test]
    fn test_min_match_threshold() {
        let tokens = encode(b"AABAA", 4096, 255, 2);
        for t in &tokens {
            assert!(t.length == 0 || t.length >= 2);
        }
    }

    // ---- Edge Cases ----

    #[test]
    fn test_single_byte_literal() {
        let tokens = encode(b"X", 4096, 255, 3);
        assert_eq!(tokens, vec![Token::new(0, 0, b'X')]);
    }

    #[test]
    fn test_exact_window_boundary() {
        let data: Vec<u8> = vec![b'X'; 11]; // 10 Xs + 1 more X
        let tokens = encode(&data, 10, 255, 3);
        assert!(tokens.iter().any(|t| t.offset > 0), "expected at least one match");
        assert_eq!(decode(&tokens, &[]), data);
    }

    #[test]
    fn test_overlapping_match_decode() {
        // [A, B] + (offset=2, length=5, next_char='Z') → ABABABAZ.
        let tokens = vec![
            Token::new(0, 0, b'A'),
            Token::new(0, 0, b'B'),
            Token::new(2, 5, b'Z'),
        ];
        assert_eq!(decode(&tokens, &[]), b"ABABABAZ");
    }

    #[test]
    fn test_binary_with_nulls() {
        let data = &[0u8, 0, 0, 255, 255];
        let tokens = encode(data, 4096, 255, 3);
        assert_eq!(decode(&tokens, &[]), data);
    }

    #[test]
    fn test_very_long_input() {
        let chunk = b"Hello, World! ".repeat(100);
        let extra = vec![b'X'; 500];
        let data: Vec<u8> = [chunk.as_slice(), extra.as_slice()].concat();
        let tokens = encode(&data, 4096, 255, 3);
        assert_eq!(decode(&tokens, &[]), data);
    }

    #[test]
    fn test_all_same_byte_compresses() {
        let data = vec![b'A'; 10000];
        let tokens = encode(&data, 4096, 255, 3);
        // ~41 tokens expected: 1 literal + ~39 × 255 + 1 partial.
        assert!(tokens.len() < 50, "expected < 50 tokens, got {}", tokens.len());
        assert_eq!(decode(&tokens, &[]), data);
    }

    #[test]
    fn test_initial_buffer() {
        // Seed [A, B] and apply (offset=2, length=3, next_char='Z').
        // start = 2 - 2 = 0; copy output[0]=A, output[1]=B, output[2]=A → ABA
        // then append Z → ABABAZ.
        let tokens = vec![Token::new(2, 3, b'Z')];
        assert_eq!(decode(&tokens, b"AB"), b"ABABAZ");
    }

    // ---- Serialisation Tests ----

    #[test]
    fn test_serialise_format() {
        let tokens = vec![Token::new(0, 0, 65), Token::new(2, 5, 66)];
        let serialised = serialise_tokens(&tokens);
        // 4 bytes header + 2 × 4 bytes = 12 bytes.
        assert_eq!(serialised.len(), 12);
    }

    #[test]
    fn test_serialise_deserialise_round_trip() {
        let tokens = vec![Token::new(0, 0, 65), Token::new(1, 3, 66), Token::new(2, 5, 67)];
        let serialised = serialise_tokens(&tokens);
        let got = deserialise_tokens(&serialised);
        assert_eq!(got, tokens);
    }

    #[test]
    fn test_deserialise_empty() {
        assert!(deserialise_tokens(&[]).is_empty());
    }

    // ---- Behaviour Tests ----

    #[test]
    fn test_no_expansion_on_incompressible_data() {
        let data: Vec<u8> = (0..=255).collect();
        let compressed = compress(&data, 4096, 255, 3);
        assert!(compressed.len() <= 4 * data.len() + 10);
    }

    #[test]
    fn test_compression_of_repetitive_data() {
        let data = b"ABC".repeat(100);
        let compressed = compress(&data, 4096, 255, 3);
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_deterministic_compression() {
        let data = b"hello world test";
        assert_eq!(
            compress(data, 4096, 255, 3),
            compress(data, 4096, 255, 3)
        );
    }
}
