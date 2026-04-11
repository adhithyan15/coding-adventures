//! lz78 — LZ78 Lossless Compression Algorithm (1978)
//!
//! LZ78 (Lempel & Ziv, 1978) builds an explicit trie-based dictionary of byte
//! sequences as it encodes. Both encoder and decoder build the same dictionary
//! independently — no dictionary is transmitted.
//!
//! # Token
//!
//! Each token is a `(dict_index, next_char)` pair:
//! - `dict_index`: ID of the longest dictionary prefix (0 = literal).
//! - `next_char`:  Byte following the match. `0` is the flush sentinel when
//!   input ends mid-match.
//!
//! # Wire Format
//!
//! ```text
//! Bytes 0–3:  original length (big-endian u32)
//! Bytes 4–7:  token count (big-endian u32)
//! Bytes 8+:   token_count × 4 bytes:
//!               [0..1]  dict_index (big-endian u16)
//!               [2]     next_char (u8)
//!               [3]     reserved (0x00)
//! ```
//!
//! # Example
//!
//! ```
//! use lz78::{compress, decompress};
//!
//! let data = b"hello hello hello";
//! let compressed = compress(data, 65536);
//! assert_eq!(decompress(&compressed), data);
//! ```

use std::collections::HashMap;

// ─── Token ────────────────────────────────────────────────────────────────────

/// One LZ78 token: a `(dict_index, next_char)` pair.
///
/// - `dict_index`: ID of the longest dictionary prefix that matches current input.
///   `0` = pure literal (no dictionary match).
/// - `next_char`:  Byte following the match. `0` is also the flush sentinel
///   when input ends mid-match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub dict_index: u16,
    pub next_char: u8,
}

// ─── Internal trie ────────────────────────────────────────────────────────────

struct TrieNode {
    dict_id: u16,
    children: HashMap<u8, usize>, // byte → arena index
}

impl TrieNode {
    fn new(dict_id: u16) -> Self {
        Self {
            dict_id,
            children: HashMap::new(),
        }
    }
}

// ─── Encoder ──────────────────────────────────────────────────────────────────

/// Encode bytes into an LZ78 token stream.
///
/// Scans the input left-to-right, following trie edges. When a byte has no
/// child edge from the current node, emits a token and resets to root.
/// If the input ends mid-match, a flush token with `next_char=0` is emitted.
///
/// # Arguments
///
/// * `data`          - Input bytes.
/// * `max_dict_size` - Maximum dictionary entries (use 65536 for default).
///
/// # Example
///
/// ```
/// use lz78::{encode, Token};
///
/// let tokens = encode(b"ABCDE", 65536);
/// assert_eq!(tokens.len(), 5);
/// assert!(tokens.iter().all(|t| t.dict_index == 0)); // all literals
/// ```
pub fn encode(data: &[u8], max_dict_size: usize) -> Vec<Token> {
    // Arena-based trie: nodes are stored in a Vec, referenced by index.
    // Index 0 = root.
    let mut arena: Vec<TrieNode> = vec![TrieNode::new(0)];
    let mut next_id: u16 = 1;
    let mut current: usize = 0; // index into arena (0 = root)
    let mut tokens = Vec::new();

    for &byte in data {
        if let Some(&child_idx) = arena[current].children.get(&byte) {
            current = child_idx;
        } else {
            tokens.push(Token {
                dict_index: arena[current].dict_id,
                next_char: byte,
            });

            if (next_id as usize) < max_dict_size {
                let new_idx = arena.len();
                arena.push(TrieNode::new(next_id));
                arena[current].children.insert(byte, new_idx);
                next_id += 1;
            }

            current = 0; // reset to root
        }
    }

    // Flush partial match at end of stream.
    if current != 0 {
        tokens.push(Token {
            dict_index: arena[current].dict_id,
            next_char: 0,
        });
    }

    tokens
}

// ─── Decoder ──────────────────────────────────────────────────────────────────

/// Decode an LZ78 token stream back into the original bytes.
///
/// Mirrors `encode`: maintains a parallel dictionary as a `Vec<(u16, u8)>`.
/// For each token, reconstructs the sequence for `dict_index`, emits it,
/// emits `next_char`, then adds a new dictionary entry.
///
/// # Arguments
///
/// * `tokens`          - Token stream from `encode`.
/// * `original_length` - If `Some(n)`, truncates output to `n` bytes (strips
///   the flush sentinel). Pass `None` for all bytes.
///
/// # Example
///
/// ```
/// use lz78::{encode, decode};
///
/// let tokens = encode(b"hello", 65536);
/// assert_eq!(decode(&tokens, Some(5)), b"hello");
/// ```
pub fn decode(tokens: &[Token], original_length: Option<usize>) -> Vec<u8> {
    // dict_table[i] = (parent_id, byte). Index 0 = root sentinel.
    let mut dict_table: Vec<(u16, u8)> = vec![(0, 0)];
    let mut output = Vec::new();

    for tok in tokens {
        let seq = reconstruct(&dict_table, tok.dict_index);
        output.extend_from_slice(&seq);

        if original_length.map_or(true, |n| output.len() < n) {
            output.push(tok.next_char);
        }

        dict_table.push((tok.dict_index, tok.next_char));
    }

    if let Some(n) = original_length {
        output.truncate(n);
    }
    output
}

fn reconstruct(table: &[(u16, u8)], index: u16) -> Vec<u8> {
    if index == 0 {
        return Vec::new();
    }
    let mut rev = Vec::new();
    let mut idx = index as usize;
    while idx != 0 {
        let (parent_id, byte) = table[idx];
        rev.push(byte);
        idx = parent_id as usize;
    }
    rev.reverse();
    rev
}

// ─── Serialisation ────────────────────────────────────────────────────────────

fn serialise_tokens(tokens: &[Token], original_length: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8 + tokens.len() * 4);
    buf.extend_from_slice(&(original_length as u32).to_be_bytes());
    buf.extend_from_slice(&(tokens.len() as u32).to_be_bytes());
    for tok in tokens {
        buf.extend_from_slice(&tok.dict_index.to_be_bytes());
        buf.push(tok.next_char);
        buf.push(0x00);
    }
    buf
}

fn deserialise_tokens(data: &[u8]) -> (Vec<Token>, usize) {
    if data.len() < 8 {
        return (Vec::new(), 0);
    }
    let original_length = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let token_count = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let mut tokens = Vec::with_capacity(token_count);
    for i in 0..token_count {
        let base = 8 + i * 4;
        if base + 4 > data.len() {
            break;
        }
        let dict_index = u16::from_be_bytes([data[base], data[base + 1]]);
        let next_char = data[base + 2];
        tokens.push(Token { dict_index, next_char });
    }
    (tokens, original_length)
}

// ─── One-shot API ─────────────────────────────────────────────────────────────

/// Compress bytes using LZ78 and serialise to the CMP01 wire format.
///
/// # Example
///
/// ```
/// use lz78::{compress, decompress};
///
/// let data = b"AAAAAAA";
/// assert_eq!(decompress(&compress(data, 65536)), data);
/// ```
pub fn compress(data: &[u8], max_dict_size: usize) -> Vec<u8> {
    let tokens = encode(data, max_dict_size);
    serialise_tokens(&tokens, data.len())
}

/// Decompress bytes that were compressed with `compress`.
///
/// # Example
///
/// ```
/// use lz78::{compress, decompress};
///
/// let original = b"hello hello hello";
/// assert_eq!(decompress(&compress(original, 65536)), original);
/// ```
pub fn decompress(data: &[u8]) -> Vec<u8> {
    let (tokens, original_length) = deserialise_tokens(data);
    decode(&tokens, Some(original_length))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_DICT: usize = 65536;

    fn rt(s: &[u8]) -> Vec<u8> {
        decompress(&compress(s, MAX_DICT))
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(encode(&[], MAX_DICT), vec![]);
        assert_eq!(decode(&[], Some(0)), b"");
    }

    #[test]
    fn test_single_byte() {
        let tokens = encode(b"A", MAX_DICT);
        assert_eq!(tokens, vec![Token { dict_index: 0, next_char: 65 }]);
        assert_eq!(decode(&tokens, Some(1)), b"A");
    }

    #[test]
    fn test_no_repetition() {
        let tokens = encode(b"ABCDE", MAX_DICT);
        assert_eq!(tokens.len(), 5);
        assert!(tokens.iter().all(|t| t.dict_index == 0));
    }

    #[test]
    fn test_aabcbbabc() {
        let want = vec![
            Token { dict_index: 0, next_char: 65 },
            Token { dict_index: 1, next_char: 66 },
            Token { dict_index: 0, next_char: 67 },
            Token { dict_index: 0, next_char: 66 },
            Token { dict_index: 4, next_char: 65 },
            Token { dict_index: 4, next_char: 67 },
        ];
        assert_eq!(encode(b"AABCBBABC", MAX_DICT), want);
        assert_eq!(rt(b"AABCBBABC"), b"AABCBBABC");
    }

    #[test]
    fn test_ababab() {
        let want = vec![
            Token { dict_index: 0, next_char: 65 },
            Token { dict_index: 0, next_char: 66 },
            Token { dict_index: 1, next_char: 66 },
            Token { dict_index: 3, next_char: 0 },
        ];
        assert_eq!(encode(b"ABABAB", MAX_DICT), want);
        assert_eq!(rt(b"ABABAB"), b"ABABAB");
    }

    #[test]
    fn test_all_identical() {
        let tokens = encode(b"AAAAAAA", MAX_DICT);
        assert_eq!(tokens.len(), 4);
    }

    #[test]
    fn test_round_trip() {
        let cases: &[&[u8]] = &[
            b"", b"A", b"ABCDE", b"AAAAAAA", b"ABABABAB", b"AABCBBABC",
            b"hello world", b"ababababab",
        ];
        for &s in cases {
            assert_eq!(rt(s), s, "round-trip failed for {:?}", s);
        }
    }

    #[test]
    fn test_binary_round_trip() {
        let cases: &[&[u8]] = &[
            &[0, 0, 0],
            &[255, 255, 255],
            &(0u8..=255).collect::<Vec<_>>(),
            &[0, 1, 2, 0, 1, 2],
            &[0, 0, 0, 255, 255],
        ];
        for &data in cases {
            assert_eq!(rt(data), data);
        }
    }

    #[test]
    fn test_max_dict_size_respected() {
        let tokens = encode(b"ABCABCABCABCABC", 10);
        assert!(tokens.iter().all(|t| (t.dict_index as usize) < 10));
    }

    #[test]
    fn test_max_dict_size_1() {
        let tokens = encode(b"AAAA", 1);
        assert!(tokens.iter().all(|t| t.dict_index == 0));
    }

    #[test]
    fn test_binary_with_nulls() {
        assert_eq!(rt(&[0, 0, 0, 255, 255]), &[0, 0, 0, 255, 255]);
    }

    #[test]
    fn test_compress_format_size() {
        let data = b"AB";
        let compressed = compress(data, MAX_DICT);
        let tokens = encode(data, MAX_DICT);
        assert_eq!(compressed.len(), 8 + tokens.len() * 4);
    }

    #[test]
    fn test_deterministic() {
        let data = b"hello world test repeated";
        assert_eq!(compress(data, MAX_DICT), compress(data, MAX_DICT));
    }

    #[test]
    fn test_repetitive_data_compresses() {
        let data: Vec<u8> = b"ABC".iter().cycle().take(3000).cloned().collect();
        let compressed = compress(&data, MAX_DICT);
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_all_same_byte_compresses() {
        let data = vec![65u8; 10000];
        let compressed = compress(&data, MAX_DICT);
        assert!(compressed.len() < data.len());
        assert_eq!(decompress(&compressed), data);
    }

    #[test]
    fn test_very_long_input() {
        let mut data: Vec<u8> = b"Hello, World! ".iter().cycle().take(1400).cloned().collect();
        data.extend(0u8..=255);
        assert_eq!(rt(&data), data);
    }
}
