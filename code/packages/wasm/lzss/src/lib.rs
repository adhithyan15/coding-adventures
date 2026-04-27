//! WebAssembly bindings for the LZSS compression crate (CMP02).
//!
//! # What is LZSS?
//!
//! LZSS (Lempel-Ziv-Storer-Szymanski, 1982) is a lossless data-compression
//! algorithm. It scans input bytes and, wherever it finds a sequence that
//! appeared earlier, replaces that sequence with a back-reference
//! `(offset, length)`. A flag-bit scheme distinguishes literal bytes from
//! back-references without paying a "next char" tax on every token.
//!
//! ```text
//!  Original:    A  A  B  C  B  B  A  B  C
//!  Tokens:     [L:'A'][L:'A'][L:'B'][L:'C'][L:'B'][L:'B'][M:offset=5,len=3]
//!                                                          ↑──────────────┘
//!                            copies "ABC" from 5 bytes back
//! ```
//!
//! # This crate
//!
//! This is a thin [wasm-bindgen] wrapper around the pure-Rust `lzss` crate.
//! It exposes two free functions to JavaScript/TypeScript:
//!
//! - [`compress`] — encode bytes into the CMP02 wire format.
//! - [`decompress`] — recover the original bytes from CMP02 wire format.
//!
//! The wire format is self-describing: the first 4 bytes store the original
//! length (big-endian u32), so callers never need to pass the size separately.
//!
//! # CMP series context
//!
//! ```text
//! CMP00 (LZ77,    1977) — Sliding-window backreferences.
//! CMP01 (LZ78,    1978) — Explicit trie dictionary.
//! CMP02 (LZSS,    1982) — LZ77 + flag bits. ← this crate
//! CMP03 (LZW,     1984) — LZ78 + pre-seeded alphabet; used in GIF.
//! CMP04 (Huffman, 1952) — Entropy coding.
//! CMP05 (DEFLATE, 1996) — LZ77 + Huffman; used in ZIP/gzip/PNG.
//! ```
//!
//! [wasm-bindgen]: https://github.com/rustwasm/wasm-bindgen

use wasm_bindgen::prelude::*;

// ─── compress ────────────────────────────────────────────────────────────────

/// Compress bytes using LZSS and return the CMP02 wire-format bytes.
///
/// Uses the standard parameters from the CMP02 spec:
/// - **Window size**: 4096 bytes (the search buffer size).
/// - **Max match length**: 255 bytes.
/// - **Min match length**: 3 bytes (a match shorter than 3 bytes would take
///   more space than the literal bytes it replaces, so we emit literals
///   instead).
///
/// The returned byte slice begins with an 8-byte header:
///
/// ```text
/// Bytes 0–3:  original_length  (big-endian u32)
/// Bytes 4–7:  block_count      (big-endian u32)
/// Bytes 8+:   encoded blocks
/// ```
///
/// Each block contains a 1-byte flag word followed by 1–3 bytes per symbol
/// (1 byte for a literal, 3 bytes for a back-reference match).
///
/// # Examples (JavaScript)
///
/// ```js
/// import init, { compress, decompress } from './lzss_wasm.js';
/// await init();
///
/// const enc = new TextEncoder();
/// const dec = new TextDecoder();
///
/// const original  = enc.encode("hello hello hello");
/// const compressed = compress(original);
/// const recovered  = decompress(compressed);
///
/// console.assert(dec.decode(recovered) === "hello hello hello");
/// ```
///
/// # Panics
///
/// Does not panic on any well-formed input. Empty input returns a valid
/// 8-byte header with `original_length = 0` and `block_count = 0`.
#[wasm_bindgen]
pub fn compress(data: &[u8]) -> Vec<u8> {
    // Delegate entirely to the pure-Rust `lzss` crate.  The crate handles
    // the encode pass (building Token stream) and serialise pass (packing
    // the flag-bit blocks) in one call.
    lzss::compress(data)
}

// ─── decompress ──────────────────────────────────────────────────────────────

/// Decompress bytes produced by [`compress`], recovering the original data.
///
/// Reads the 8-byte CMP02 header to discover `original_length`, then
/// decodes each block's flag bits to determine whether each symbol is a
/// literal byte or a back-reference, and reconstructs the output
/// byte-by-byte.
///
/// # Security
///
/// The deserialiser caps `block_count` against the actual payload size to
/// prevent a crafted header from causing excessive allocation or an
/// infinite loop.  Malformed back-references (offset 0 or offset past the
/// current output) are silently skipped rather than panicking.
///
/// # Errors (JavaScript)
///
/// Returns an empty `Uint8Array` if `data` is fewer than 8 bytes (the
/// minimum valid CMP02 header size).
///
/// # Examples (JavaScript)
///
/// ```js
/// // Round-trip a binary buffer
/// const bytes      = new Uint8Array([0x00, 0xFF, 0x42, 0xAA]);
/// const compressed = compress(bytes);
/// const recovered  = decompress(compressed);
/// // recovered deep-equals bytes
/// ```
#[wasm_bindgen]
pub fn decompress(data: &[u8]) -> Vec<u8> {
    // The `lzss::decompress` function is the inverse of `lzss::compress`.
    // It reads the CMP02 header, deserialises the token blocks, then
    // calls decode() with the stored `original_length` to truncate
    // exactly to the right size.
    lzss::decompress(data)
}

// ─── Tests ───────────────────────────────────────────────────────────────────
//
// wasm-bindgen tests (run with `wasm-pack test --node`) target wasm32.
// Native tests (run with `cargo test`) use the `#[cfg(not(target_arch = ...))]`
// guard so they compile and run on the host without a browser or Node.
//
// Both test suites exercise the same logical properties:
//   1. Round-trip fidelity — compress then decompress yields the original.
//   2. Empty input safety — no panics, no garbage output.
//   3. Binary data — non-UTF8 bytes survive unmodified.
//   4. Repetitive data compresses smaller than the original.

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper ────────────────────────────────────────────────────────────────

    /// Compress then decompress — should recover exactly the original bytes.
    fn roundtrip(data: &[u8]) -> Vec<u8> {
        decompress(&compress(data))
    }

    // ── Round-trip correctness ────────────────────────────────────────────────

    #[test]
    fn roundtrip_empty() {
        // Empty input: the header is still written (original_length = 0),
        // and decompress must return an empty Vec, not panic.
        assert_eq!(roundtrip(b""), b"");
    }

    #[test]
    fn roundtrip_single_byte() {
        assert_eq!(roundtrip(b"A"), b"A");
    }

    #[test]
    fn roundtrip_no_repetition() {
        // No back-references possible: every byte is unique.
        assert_eq!(roundtrip(b"ABCDEFGH"), b"ABCDEFGH");
    }

    #[test]
    fn roundtrip_repeated_char() {
        // Run of the same byte: the encoder emits one Literal then one Match.
        // The decoder must reproduce every byte exactly.
        let data = b"AAAAAAAAAA";
        assert_eq!(roundtrip(data), data);
    }

    #[test]
    fn roundtrip_alternating_pattern() {
        // Periodic pattern: "ABABABABAB"
        // The encoder finds a 2-byte repeat and collapses the tail.
        assert_eq!(roundtrip(b"ABABABABAB"), b"ABABABABAB");
    }

    #[test]
    fn roundtrip_hello_hello() {
        // The classic LZSS demonstration input.
        assert_eq!(roundtrip(b"hello hello hello"), b"hello hello hello");
    }

    #[test]
    fn roundtrip_binary_bytes() {
        // Non-text bytes — LZSS is byte-oriented and must not mangle them.
        let data: Vec<u8> = vec![0x00, 0xFF, 0x80, 0x01, 0x7F, 0xAA, 0x55];
        assert_eq!(roundtrip(&data), data);
    }

    #[test]
    fn roundtrip_full_byte_range() {
        // Every possible byte value 0–255 in order.
        let data: Vec<u8> = (0u8..=255).collect();
        assert_eq!(roundtrip(&data), data);
    }

    #[test]
    fn roundtrip_long_repeated_pattern() {
        // 3000 bytes cycling over a 3-byte alphabet.
        let data: Vec<u8> = (0..3000_usize).map(|i| (i % 3) as u8).collect();
        assert_eq!(roundtrip(&data), data);
    }

    #[test]
    fn roundtrip_long_random_like() {
        // 3000-byte sequence where bytes cycle over a larger alphabet
        // so there are fewer long matches.  Tests the encoder does not
        // corrupt data when the compression ratio is close to 1.
        let data: Vec<u8> = (0..3000_usize).map(|i| (i % 251) as u8).collect();
        assert_eq!(roundtrip(&data), data);
    }

    #[test]
    fn roundtrip_large_uniform() {
        // 10 000 identical bytes — maximum compression ratio for LZSS.
        let data = vec![0x42u8; 10_000];
        assert_eq!(roundtrip(&data), data);
    }

    // ── Wire-format integrity ─────────────────────────────────────────────────

    #[test]
    fn compress_header_stores_original_length() {
        // The first 4 bytes of the CMP02 payload must be the original length
        // as a big-endian u32.  This lets the decompressor allocate exactly
        // the right output buffer without an end-of-stream sentinel.
        let compressed = compress(b"hello");
        let stored_len =
            u32::from_be_bytes([compressed[0], compressed[1], compressed[2], compressed[3]]);
        assert_eq!(stored_len, 5);
    }

    #[test]
    fn compress_is_deterministic() {
        // The same input must always produce the same compressed output.
        // (LZSS is a deterministic algorithm; no randomness is involved.)
        let input = b"hello world determinism test";
        assert_eq!(compress(input), compress(input));
    }

    // ── Compression effectiveness ─────────────────────────────────────────────

    #[test]
    fn compress_reduces_size_for_repetitive_data() {
        // Highly repetitive data must compress to fewer bytes than the
        // original.  If it does not, the algorithm (or the serialiser) is
        // broken.
        let data: Vec<u8> = b"ABC".iter().cloned().cycle().take(3000).collect();
        assert!(compress(&data).len() < data.len());
    }

    #[test]
    fn compress_uniform_data_compresses_well() {
        // 10 000 identical bytes should compress dramatically.
        let data = vec![0x42u8; 10_000];
        let compressed = compress(&data);
        // The compressed form should be much smaller (less than 1/10th).
        assert!(compressed.len() < data.len() / 10);
    }

    // ── Safety / robustness ───────────────────────────────────────────────────

    #[test]
    fn decompress_too_short_input_does_not_panic() {
        // Any input shorter than 8 bytes lacks a complete CMP02 header.
        // The decompressor must return an empty result without panicking.
        assert_eq!(decompress(b""), b"");
        assert_eq!(decompress(b"short"), b"");
    }

    #[test]
    fn decompress_crafted_large_block_count_does_not_panic() {
        // A malformed header claiming an astronomically large block_count
        // must be capped against the actual payload size, not trusted blindly.
        //
        //   Bytes 0–3:  original_length = 0  (we don't care about the value)
        //   Bytes 4–7:  block_count = 2^30   (far more than the payload)
        let mut bad = vec![0u8; 16];
        bad[4] = 0x40; // sets the top byte of block_count to 0x40 → 2^30
        let result = decompress(&bad);
        let _ = result; // must not panic or hang
    }
}

// ─── wasm-bindgen tests (run under Node via wasm-pack test --node) ────────────

#[cfg(target_arch = "wasm32")]
#[cfg(test)]
mod wasm_tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn wasm_roundtrip_hello() {
        let data = b"hello hello hello";
        assert_eq!(decompress(&compress(data)), data);
    }

    #[wasm_bindgen_test]
    fn wasm_roundtrip_empty() {
        assert_eq!(decompress(&compress(b"")), b"");
    }

    #[wasm_bindgen_test]
    fn wasm_compress_reduces_repetitive_data() {
        let data: Vec<u8> = b"ABCABC".iter().cloned().cycle().take(600).collect();
        assert!(compress(&data).len() < data.len());
    }
}
