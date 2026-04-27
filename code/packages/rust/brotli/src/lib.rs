//! brotli — CMP06: Brotli lossless compression algorithm (2013).
//!
//! Brotli (RFC 7932) is Google's lossless compression format, used for HTTP
//! `Content-Encoding: br` and WOFF2 fonts. It delivers significantly better
//! compression than DEFLATE (CMP05) on web content by combining three ideas:
//!
//! 1. **Context-dependent literal trees** — instead of one Huffman tree for
//!    all literal bytes, Brotli uses four trees, one per *context bucket*.
//!    The context bucket for a literal is determined by the character class
//!    of the immediately preceding output byte (space/punct=0, digit=1,
//!    uppercase=2, lowercase=3). Each tree is tuned to the byte distribution
//!    in its context, so e.g. the tree for "follows a space" can give short
//!    codes to common word-initial letters like 't', 'a', 'i'.
//!
//! 2. **Insert-and-copy commands** — DEFLATE encodes literals and back-
//!    references as separate stream tokens, each with its own Huffman symbol.
//!    Brotli instead uses *commands* that bundle an insert run (raw literals)
//!    with a copy back-reference into a single Huffman symbol called an
//!    **Insert-Copy Code (ICC)**. The ICC implies the *ranges* of both the
//!    insert length and the copy length; extra bits select the exact values
//!    within those ranges. This cuts per-command overhead substantially.
//!
//! 3. **Larger sliding window** — DEFLATE's window is 4096 bytes. Brotli
//!    extends this to 65535 bytes (this implementation), allowing long-
//!    distance matches across boilerplate that recurs far apart in large
//!    HTML pages.
//!
//! # Encoding Order (per command)
//!
//! Each command is encoded in the bit stream as:
//! ```text
//! [ICC Huffman code] [insert_extra bits] [copy_extra bits]
//! [literal_0 Huffman code] ... [literal_{N-1} Huffman code]
//! [distance Huffman code] [dist_extra bits]
//! ```
//!
//! The sentinel (ICC code 63) ends the stream.
//!
//! # Trailing Literals
//!
//! The ICC table has no code for "insert N literals, copy 0 bytes." To handle
//! trailing literals (bytes after the last match), this implementation appends
//! a minimal synthetic copy (length=4, distance=1 = copy the last byte four
//! times). The decompressor stops at `original_length` bytes so the extra
//! synthesized bytes are silently dropped. `original_length` is stored in the
//! wire-format header, so the correct length is always known.
//!
//! # CodingAdventures Simplifications
//!
//! - Window size capped at 65535 bytes.
//! - 4 context buckets (vs RFC 7932's 64).
//! - No static dictionary (RFC 7932 includes 122,784 word forms).
//! - Minimum match length 4 (same as RFC 7932).
//!
//! # Wire Format (CMP06)
//!
//! ```text
//! Header (10 bytes):
//!   [4B BE] original_length
//!   [1B]    icc_entry_count   — entries in ICC Huffman code-length table
//!   [1B]    dist_entry_count  — entries in dist code-length table (0 = no copies)
//!   [1B]    ctx0_entry_count  — entries in literal tree 0
//!   [1B]    ctx1_entry_count
//!   [1B]    ctx2_entry_count
//!   [1B]    ctx3_entry_count
//!
//! ICC table  (icc_entry_count  × 2B): symbol u8, code_length u8
//! Dist table (dist_entry_count × 2B): symbol u8, code_length u8
//! Lit tree 0 (ctx0_entry_count × 3B): symbol u16 BE, code_length u8
//! Lit tree 1 (ctx1_entry_count × 3B): same
//! Lit tree 2 (ctx2_entry_count × 3B): same
//! Lit tree 3 (ctx3_entry_count × 3B): same
//! Bit stream (remaining bytes):       LSB-first, zero-padded to byte boundary
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
//! CMP05 (DEFLATE, 1996) — LZSS + dual Huffman; ZIP/gzip/PNG/zlib.
//! CMP06 (Brotli,  2013) — Context modeling + insert-copy + large window.  ← this crate
//! ```

use std::collections::HashMap;

use huffman_tree::HuffmanTree;

// ---------------------------------------------------------------------------
// Insert-Copy Code (ICC) table
// ---------------------------------------------------------------------------
//
// An ICC code is a single Huffman symbol that implicitly encodes *ranges* for
// both the insert length and the copy length. After decoding the ICC symbol,
// the decoder reads `insert_extra` bits and `copy_extra` bits to determine the
// exact lengths within those ranges.
//
// Table structure: 64 codes (0–62 regular, 63 = end-of-data sentinel).
//
//   Code range   Insert base  Insert extra  Copy pattern
//   0–15         0            0             4, 5, 6, 8+1, 10+1, 14+2, 18+2, 26+3, 34+3, 50+4, 66+4, 98+5, 130+5, 194+6, 258+7, 514+8
//   16–23        1            0             same copy pattern (first 8 entries)
//   24–31        2            0             same copy pattern (first 8 entries)
//   32–39        3            1             same copy pattern (first 8 entries)
//   40–47        5            2             same copy pattern (first 8 entries)
//   48–55        9            3             same copy pattern (first 8 entries)
//   56–62        17           4             first 7 copy entries (4..=18+2)
//   63           0            0             0 (end-of-data sentinel)

#[derive(Clone, Copy)]
struct IccEntry {
    insert_base:  u32,
    insert_extra: u32,
    copy_base:    u32,
    copy_extra:   u32,
}

/// The full 64-entry ICC table, indexed by ICC code 0–63.
///
/// For any regular ICC code k (0–62):
///   actual_insert_length = insert_base + read_lsb(insert_extra bits)
///   actual_copy_length   = copy_base   + read_lsb(copy_extra bits)
///
/// ICC code 63 is the end-of-data sentinel: insert=0, copy=0.
const ICC_TABLE: &[IccEntry; 64] = &[
    // ── Codes 0–15: insert=0, copy varies (16 codes) ──────────────────────
    IccEntry { insert_base:  0, insert_extra: 0, copy_base:   4, copy_extra: 0 }, //  0
    IccEntry { insert_base:  0, insert_extra: 0, copy_base:   5, copy_extra: 0 }, //  1
    IccEntry { insert_base:  0, insert_extra: 0, copy_base:   6, copy_extra: 0 }, //  2
    IccEntry { insert_base:  0, insert_extra: 0, copy_base:   8, copy_extra: 1 }, //  3
    IccEntry { insert_base:  0, insert_extra: 0, copy_base:  10, copy_extra: 1 }, //  4
    IccEntry { insert_base:  0, insert_extra: 0, copy_base:  14, copy_extra: 2 }, //  5
    IccEntry { insert_base:  0, insert_extra: 0, copy_base:  18, copy_extra: 2 }, //  6
    IccEntry { insert_base:  0, insert_extra: 0, copy_base:  26, copy_extra: 3 }, //  7
    IccEntry { insert_base:  0, insert_extra: 0, copy_base:  34, copy_extra: 3 }, //  8
    IccEntry { insert_base:  0, insert_extra: 0, copy_base:  50, copy_extra: 4 }, //  9
    IccEntry { insert_base:  0, insert_extra: 0, copy_base:  66, copy_extra: 4 }, // 10
    IccEntry { insert_base:  0, insert_extra: 0, copy_base:  98, copy_extra: 5 }, // 11
    IccEntry { insert_base:  0, insert_extra: 0, copy_base: 130, copy_extra: 5 }, // 12
    IccEntry { insert_base:  0, insert_extra: 0, copy_base: 194, copy_extra: 6 }, // 13
    IccEntry { insert_base:  0, insert_extra: 0, copy_base: 258, copy_extra: 7 }, // 14
    IccEntry { insert_base:  0, insert_extra: 0, copy_base: 514, copy_extra: 8 }, // 15
    // ── Codes 16–23: insert=1, copy varies (8 codes) ──────────────────────
    IccEntry { insert_base:  1, insert_extra: 0, copy_base:   4, copy_extra: 0 }, // 16
    IccEntry { insert_base:  1, insert_extra: 0, copy_base:   5, copy_extra: 0 }, // 17
    IccEntry { insert_base:  1, insert_extra: 0, copy_base:   6, copy_extra: 0 }, // 18
    IccEntry { insert_base:  1, insert_extra: 0, copy_base:   8, copy_extra: 1 }, // 19
    IccEntry { insert_base:  1, insert_extra: 0, copy_base:  10, copy_extra: 1 }, // 20
    IccEntry { insert_base:  1, insert_extra: 0, copy_base:  14, copy_extra: 2 }, // 21
    IccEntry { insert_base:  1, insert_extra: 0, copy_base:  18, copy_extra: 2 }, // 22
    IccEntry { insert_base:  1, insert_extra: 0, copy_base:  26, copy_extra: 3 }, // 23
    // ── Codes 24–31: insert=2, copy varies (8 codes) ──────────────────────
    IccEntry { insert_base:  2, insert_extra: 0, copy_base:   4, copy_extra: 0 }, // 24
    IccEntry { insert_base:  2, insert_extra: 0, copy_base:   5, copy_extra: 0 }, // 25
    IccEntry { insert_base:  2, insert_extra: 0, copy_base:   6, copy_extra: 0 }, // 26
    IccEntry { insert_base:  2, insert_extra: 0, copy_base:   8, copy_extra: 1 }, // 27
    IccEntry { insert_base:  2, insert_extra: 0, copy_base:  10, copy_extra: 1 }, // 28
    IccEntry { insert_base:  2, insert_extra: 0, copy_base:  14, copy_extra: 2 }, // 29
    IccEntry { insert_base:  2, insert_extra: 0, copy_base:  18, copy_extra: 2 }, // 30
    IccEntry { insert_base:  2, insert_extra: 0, copy_base:  26, copy_extra: 3 }, // 31
    // ── Codes 32–39: insert=3+1bit, copy varies (8 codes) ─────────────────
    IccEntry { insert_base:  3, insert_extra: 1, copy_base:   4, copy_extra: 0 }, // 32
    IccEntry { insert_base:  3, insert_extra: 1, copy_base:   5, copy_extra: 0 }, // 33
    IccEntry { insert_base:  3, insert_extra: 1, copy_base:   6, copy_extra: 0 }, // 34
    IccEntry { insert_base:  3, insert_extra: 1, copy_base:   8, copy_extra: 1 }, // 35
    IccEntry { insert_base:  3, insert_extra: 1, copy_base:  10, copy_extra: 1 }, // 36
    IccEntry { insert_base:  3, insert_extra: 1, copy_base:  14, copy_extra: 2 }, // 37
    IccEntry { insert_base:  3, insert_extra: 1, copy_base:  18, copy_extra: 2 }, // 38
    IccEntry { insert_base:  3, insert_extra: 1, copy_base:  26, copy_extra: 3 }, // 39
    // ── Codes 40–47: insert=5+2bits, copy varies (8 codes) ────────────────
    IccEntry { insert_base:  5, insert_extra: 2, copy_base:   4, copy_extra: 0 }, // 40
    IccEntry { insert_base:  5, insert_extra: 2, copy_base:   5, copy_extra: 0 }, // 41
    IccEntry { insert_base:  5, insert_extra: 2, copy_base:   6, copy_extra: 0 }, // 42
    IccEntry { insert_base:  5, insert_extra: 2, copy_base:   8, copy_extra: 1 }, // 43
    IccEntry { insert_base:  5, insert_extra: 2, copy_base:  10, copy_extra: 1 }, // 44
    IccEntry { insert_base:  5, insert_extra: 2, copy_base:  14, copy_extra: 2 }, // 45
    IccEntry { insert_base:  5, insert_extra: 2, copy_base:  18, copy_extra: 2 }, // 46
    IccEntry { insert_base:  5, insert_extra: 2, copy_base:  26, copy_extra: 3 }, // 47
    // ── Codes 48–55: insert=9+3bits, copy varies (8 codes) ────────────────
    IccEntry { insert_base:  9, insert_extra: 3, copy_base:   4, copy_extra: 0 }, // 48
    IccEntry { insert_base:  9, insert_extra: 3, copy_base:   5, copy_extra: 0 }, // 49
    IccEntry { insert_base:  9, insert_extra: 3, copy_base:   6, copy_extra: 0 }, // 50
    IccEntry { insert_base:  9, insert_extra: 3, copy_base:   8, copy_extra: 1 }, // 51
    IccEntry { insert_base:  9, insert_extra: 3, copy_base:  10, copy_extra: 1 }, // 52
    IccEntry { insert_base:  9, insert_extra: 3, copy_base:  14, copy_extra: 2 }, // 53
    IccEntry { insert_base:  9, insert_extra: 3, copy_base:  18, copy_extra: 2 }, // 54
    IccEntry { insert_base:  9, insert_extra: 3, copy_base:  26, copy_extra: 3 }, // 55
    // ── Codes 56–62: insert=17+4bits, copy varies (7 codes) ───────────────
    IccEntry { insert_base: 17, insert_extra: 4, copy_base:   4, copy_extra: 0 }, // 56
    IccEntry { insert_base: 17, insert_extra: 4, copy_base:   5, copy_extra: 0 }, // 57
    IccEntry { insert_base: 17, insert_extra: 4, copy_base:   6, copy_extra: 0 }, // 58
    IccEntry { insert_base: 17, insert_extra: 4, copy_base:   8, copy_extra: 1 }, // 59
    IccEntry { insert_base: 17, insert_extra: 4, copy_base:  10, copy_extra: 1 }, // 60
    IccEntry { insert_base: 17, insert_extra: 4, copy_base:  14, copy_extra: 2 }, // 61
    IccEntry { insert_base: 17, insert_extra: 4, copy_base:  18, copy_extra: 2 }, // 62
    // ── Code 63: end-of-data sentinel ─────────────────────────────────────
    IccEntry { insert_base:  0, insert_extra: 0, copy_base:   0, copy_extra: 0 }, // 63
];

// ---------------------------------------------------------------------------
// Distance code table
// ---------------------------------------------------------------------------
//
// Brotli uses the same distance code scheme as DEFLATE but extends it to 32
// codes, covering offsets 1–65535 (vs DEFLATE's 24 codes covering 1–4096).
//
// Each code covers a range [base, base + 2^extra - 1]. Extra bits are read
// LSB-first after the distance Huffman code.

#[derive(Clone, Copy)]
struct DistEntry {
    base:       u32,
    extra_bits: u32,
}

const DIST_TABLE: &[DistEntry; 32] = &[
    DistEntry { base:     1, extra_bits:  0 }, //  0
    DistEntry { base:     2, extra_bits:  0 }, //  1
    DistEntry { base:     3, extra_bits:  0 }, //  2
    DistEntry { base:     4, extra_bits:  0 }, //  3
    DistEntry { base:     5, extra_bits:  1 }, //  4
    DistEntry { base:     7, extra_bits:  1 }, //  5
    DistEntry { base:     9, extra_bits:  2 }, //  6
    DistEntry { base:    13, extra_bits:  2 }, //  7
    DistEntry { base:    17, extra_bits:  3 }, //  8
    DistEntry { base:    25, extra_bits:  3 }, //  9
    DistEntry { base:    33, extra_bits:  4 }, // 10
    DistEntry { base:    49, extra_bits:  4 }, // 11
    DistEntry { base:    65, extra_bits:  5 }, // 12
    DistEntry { base:    97, extra_bits:  5 }, // 13
    DistEntry { base:   129, extra_bits:  6 }, // 14
    DistEntry { base:   193, extra_bits:  6 }, // 15
    DistEntry { base:   257, extra_bits:  7 }, // 16
    DistEntry { base:   385, extra_bits:  7 }, // 17
    DistEntry { base:   513, extra_bits:  8 }, // 18
    DistEntry { base:   769, extra_bits:  8 }, // 19
    DistEntry { base:  1025, extra_bits:  9 }, // 20
    DistEntry { base:  1537, extra_bits:  9 }, // 21
    DistEntry { base:  2049, extra_bits: 10 }, // 22
    DistEntry { base:  3073, extra_bits: 10 }, // 23
    // Codes 24–31: extended range up to 65535.
    DistEntry { base:  4097, extra_bits: 11 }, // 24
    DistEntry { base:  6145, extra_bits: 11 }, // 25
    DistEntry { base:  8193, extra_bits: 12 }, // 26
    DistEntry { base: 12289, extra_bits: 12 }, // 27
    DistEntry { base: 16385, extra_bits: 13 }, // 28
    DistEntry { base: 24577, extra_bits: 13 }, // 29
    DistEntry { base: 32769, extra_bits: 14 }, // 30
    DistEntry { base: 49153, extra_bits: 14 }, // 31
];

// ---------------------------------------------------------------------------
// Lookup helpers
// ---------------------------------------------------------------------------

/// Return the distance code (0–31) for a back-reference offset (1–65535).
fn dist_code_for(offset: u32) -> usize {
    for (i, e) in DIST_TABLE.iter().enumerate() {
        let max_dist = e.base + (1 << e.extra_bits) - 1;
        if offset <= max_dist {
            return i;
        }
    }
    31
}

/// Find the best ICC code for (insert_length, copy_length).
///
/// "Best" = the code with the fewest total extra bits that covers both ranges.
/// Returns `None` if no single code covers both values.
fn find_icc(insert_len: u32, copy_len: u32) -> Option<u8> {
    let mut best: Option<(u8, u32)> = None; // (code, total_extra_bits)
    for icc in 0u8..63 {
        let e = &ICC_TABLE[icc as usize];
        let max_insert = e.insert_base + (1u32 << e.insert_extra) - 1;
        let max_copy   = e.copy_base   + (1u32 << e.copy_extra)   - 1;
        if insert_len >= e.insert_base && insert_len <= max_insert
            && copy_len  >= e.copy_base  && copy_len  <= max_copy
        {
            let total_extra = e.insert_extra + e.copy_extra;
            if best.map_or(true, |(_, prev_extra)| total_extra < prev_extra) {
                best = Some((icc, total_extra));
            }
        }
    }
    best.map(|(code, _)| code)
}

/// Find the ICC code that covers `copy_len` with the smallest insert_base.
///
/// Used when we need to handle trailing literals by splitting them: pick the
/// copy-length ICC with minimum insert, then overflow literals precede the cmd.
fn find_icc_for_copy(copy_len: u32) -> u8 {
    let mut best: Option<(u8, u32)> = None; // (code, insert_base)
    for icc in 0u8..63 {
        let e = &ICC_TABLE[icc as usize];
        let max_copy = e.copy_base + (1u32 << e.copy_extra) - 1;
        if copy_len >= e.copy_base && copy_len <= max_copy {
            if best.map_or(true, |(_, prev_ins)| e.insert_base < prev_ins) {
                best = Some((icc, e.insert_base));
            }
        }
    }
    best.map(|(code, _)| code).unwrap_or(0)
}

/// Return the maximum insert_length that any ICC code with insert_base ≤ target
/// can represent when also covering `copy_len`.
fn max_insert_for_copy(copy_len: u32) -> u32 {
    let mut best = 0u32;
    for icc in 0u8..63 {
        let e = &ICC_TABLE[icc as usize];
        let max_copy   = e.copy_base + (1u32 << e.copy_extra) - 1;
        if copy_len >= e.copy_base && copy_len <= max_copy {
            let max_ins = e.insert_base + (1u32 << e.insert_extra) - 1;
            if max_ins > best { best = max_ins; }
        }
    }
    best
}

/// Clamp a copy_length to the largest ICC-representable value ≤ `len`.
///
/// The ICC table does not cover all integers in [4, 769]. There are gaps:
/// e.g., lengths 7, 12–13, 22–25, 42–49, 82–97, 162–193, 386–513.
/// We reduce `len` to the largest value that some ICC code can represent.
/// This wastes a few bytes of compression but ensures correctness.
fn clamp_copy_len(len: u32) -> u32 {
    let mut best = 0u32;
    for icc in 0u8..63 {
        let e = &ICC_TABLE[icc as usize];
        if e.copy_base > len { continue; }
        let max_copy = e.copy_base + (1u32 << e.copy_extra) - 1;
        // The representable value is min(len, max_copy) if copy_base ≤ len.
        let candidate = len.min(max_copy);
        if candidate > best { best = candidate; }
    }
    if best == 0 { 4 } else { best } // fallback: copy 4 bytes (always valid)
}

// ---------------------------------------------------------------------------
// Context modeling
// ---------------------------------------------------------------------------
//
// The key insight: the letter that follows a space is very different from the
// letter that follows another letter. By using separate Huffman trees for
// each context, we can assign shorter codes to the most probable next bytes
// in each context.
//
// Context bucket (2 bits, from last output byte p1):
//
//   bucket 3 — p1 is lowercase 'a'–'z'   (p1 & 0xFF in [97,122])
//   bucket 2 — p1 is uppercase 'A'–'Z'   (p1 & 0xFF in [65,90])
//   bucket 1 — p1 is digit '0'–'9'       (p1 & 0xFF in [48,57])
//   bucket 0 — everything else
//
// At stream start (no previous byte), bucket 0 is used.

/// Return the literal context bucket (0–3) given the last output byte.
pub fn literal_context(last_byte: Option<u8>) -> usize {
    match last_byte {
        None => 0,
        Some(p1) => {
            if p1 >= b'a' && p1 <= b'z' { return 3; }
            if p1 >= b'A' && p1 <= b'Z' { return 2; }
            if p1 >= b'0' && p1 <= b'9' { return 1; }
            0
        }
    }
}

// ---------------------------------------------------------------------------
// LZ matching (sliding window 65535, min match 4)
// ---------------------------------------------------------------------------
//
// We use an O(n²) backward scan identical in spirit to LZSS (CMP02), but
// extended to a 65535-byte window and a 4-byte minimum match.
//
// Maximum representable copy length from the ICC table: the largest
// representable value is copy_base=514 + 2^8-1 = 769. We cap at 769.

const MAX_MATCH: usize = 769; // max copy_length representable by ICC table
const MIN_MATCH: usize = 4;   // Brotli minimum match length
const WINDOW:    usize = 65535;

/// Find the longest match at `pos` within a 65535-byte sliding window.
///
/// Returns `(distance, length)` with distance ≥ 1 and length ≥ MIN_MATCH,
/// or `(0, 0)` if no qualifying match exists.
fn find_longest_match(data: &[u8], pos: usize) -> (u32, u32) {
    let window_start = pos.saturating_sub(WINDOW);
    let max_len = (data.len() - pos).min(MAX_MATCH);

    if max_len < MIN_MATCH {
        return (0, 0);
    }

    let mut best_len = 0usize;
    let mut best_dist = 0usize;

    // Scan backward: start from pos-1 toward window_start.
    // Closer positions are checked first; a near match has a smaller distance
    // code, which tends to use fewer bits.
    let search_end = window_start;
    let mut search = if pos > 0 { pos - 1 } else { return (0, 0); };
    loop {
        // Count matching bytes, allowing overlapping (e.g., "AAAA…").
        let mut len = 0;
        while len < max_len {
            let src_idx = search + len;
            let dst_idx = pos   + len;
            if dst_idx >= data.len() { break; }
            if data[src_idx] != data[dst_idx] { break; }
            len += 1;
        }

        if len >= MIN_MATCH && len > best_len {
            best_len  = len;
            best_dist = pos - search;
            if best_len == MAX_MATCH { break; }
        }

        if search == search_end { break; }
        search -= 1;
    }

    if best_len >= MIN_MATCH {
        (best_dist as u32, best_len as u32)
    } else {
        (0, 0)
    }
}

// ---------------------------------------------------------------------------
// A raw Brotli command
// ---------------------------------------------------------------------------

/// A command produced by the LZ pass (Pass 1).
///
/// Every command maps to one ICC symbol in the stream. `copy_distance == 0`
/// is a sentinel meaning "no actual copy" — the ICC still encodes
/// `copy_length = 4` (smallest valid code) but the distance code 32 is emitted
/// to signal the decoder to skip the copy step. This allows pure-literal blocks
/// (with no matching back-reference) to be encoded correctly without injecting
/// spurious bytes into the output.
struct Command {
    literals:      Vec<u8>,
    copy_length:   u32,
    copy_distance: u32, // 0 = "no copy" sentinel; ≥1 = back-reference distance
}

// ---------------------------------------------------------------------------
// Bit I/O
// ---------------------------------------------------------------------------
//
// Both DEFLATE and Brotli use LSB-first bit packing: the first bit written
// occupies bit 0 (the least-significant bit) of the first byte.
//
// Example: writing bits "1" "0" "1" "1" in order →
//   byte 0, bit 0 = 1; bit 1 = 0; bit 2 = 1; bit 3 = 1 → 0x0D.

/// Accumulates bits into a byte buffer, LSB-first.
struct BitBuilder {
    buf:     u64,
    bit_pos: u32,
    out:     Vec<u8>,
}

impl BitBuilder {
    fn new() -> Self {
        Self { buf: 0, bit_pos: 0, out: Vec::new() }
    }

    /// Write a Huffman code string (e.g. "101") bit by bit, LSB-first.
    fn write_bit_string(&mut self, s: &str) {
        for ch in s.chars() {
            if ch == '1' {
                self.buf |= 1u64 << self.bit_pos;
            }
            self.bit_pos += 1;
            self.drain_full_bytes();
        }
    }

    /// Write `n` raw bits from `val`, emitting bit 0 of `val` first.
    fn write_raw_bits_lsb(&mut self, val: u32, n: u32) {
        for i in 0..n {
            if (val >> i) & 1 == 1 {
                self.buf |= 1u64 << self.bit_pos;
            }
            self.bit_pos += 1;
            self.drain_full_bytes();
        }
    }

    fn drain_full_bytes(&mut self) {
        while self.bit_pos >= 8 {
            self.out.push((self.buf & 0xFF) as u8);
            self.buf >>= 8;
            self.bit_pos -= 8;
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.bit_pos > 0 {
            self.out.push((self.buf & 0xFF) as u8);
        }
        self.out
    }
}

/// Expand a byte slice to a vector of 0/1 bytes, LSB-first.
fn unpack_bits(data: &[u8]) -> Vec<u8> {
    let mut bits = Vec::with_capacity(data.len() * 8);
    for &byte in data {
        for i in 0..8u8 {
            bits.push((byte >> i) & 1);
        }
    }
    bits
}

// ---------------------------------------------------------------------------
// Canonical Huffman code reconstruction (for decompression)
// ---------------------------------------------------------------------------
//
// The wire format stores (symbol, code_length) pairs sorted by
// (code_length ASC, symbol ASC). We reconstruct canonical codes:
//
//   code[0] = 0  (padded to length[0] bits)
//   code[i] = (code[i-1] + 1) << (length[i] - length[i-1])

fn build_canonical_codes(pairs: &[(u16, usize)]) -> HashMap<u16, String> {
    let mut result = HashMap::new();
    if pairs.is_empty() {
        return result;
    }
    if pairs.len() == 1 {
        // Single-symbol tree: the one symbol gets code "0" (length=1).
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

/// Invert a code map: code-string → symbol, for fast prefix decoding.
fn reverse_code_map(m: &HashMap<u16, String>) -> HashMap<String, u16> {
    m.iter().map(|(&sym, bits)| (bits.clone(), sym)).collect()
}

// ---------------------------------------------------------------------------
// Public API: compress
// ---------------------------------------------------------------------------

/// Flush `pending` literals as one or more "no-copy" commands.
///
/// Each command has `copy_length=4` (the smallest valid ICC copy code) but
/// `copy_distance=0`, which is the CMP06 sentinel meaning "skip the copy."
/// The encoder emits distance symbol 32 (out of the normal 0–31 range) for
/// these commands; the decoder recognises symbol 32 and skips the copy step,
/// so no spurious bytes enter the output.
///
/// We chunk at 32 bytes because `max_insert_for_copy(4) = 32`: the ICC code
/// with the largest insert range that also covers copy_length=4 is code 56
/// (insert_base=17, insert_extra=4 → max insert = 17+15 = 32).
fn flush_literals_as_no_copy(pending: Vec<u8>, out: &mut Vec<Command>) {
    const MAX_INSERT_PER_CMD: usize = 32; // max_insert_for_copy(4)
    let mut remaining = pending;
    while !remaining.is_empty() {
        let chunk_size = remaining.len().min(MAX_INSERT_PER_CMD);
        let chunk: Vec<u8> = remaining.drain(0..chunk_size).collect();
        out.push(Command {
            literals:      chunk,
            copy_length:   4,
            copy_distance: 0, // sentinel: "no copy"
        });
    }
}

/// Compress `data` using CMP06 Brotli and return wire-format bytes.
///
/// # Algorithm Overview
///
/// **Pass 1 — LZ matching:**
/// Walk the input left-to-right. At each position, search the 65535-byte
/// sliding window for the longest match (≥ 4 bytes). If found, we attempt
/// to bundle the pending literal buffer into the copy command as its insert.
///
/// Because the ICC table has no entry for copy_length=0, every command MUST
/// have copy_length ≥ 4. When pending literals can't be bundled with the
/// current copy (because the copy is too large for any ICC that also allows
/// inserts > 0), we flush the pending literals as a *synthetic copy command*
/// of length 4 BEFORE the main copy. The decoder truncates at
/// `original_length`, discarding the synthetic bytes.
///
/// Trailing literals (after the last match) are similarly flushed.
///
/// **Pass 2a — Frequency tallying:**
/// Count literal/ICC/distance symbol frequencies, simulating output context.
///
/// **Pass 2b — Huffman tree construction:**
/// Build canonical Huffman trees (DT27) for ICC, distance, and 4 literal
/// context buckets.
///
/// **Pass 2c — Encoding:**
/// For each command:
///   `[ICC code][insert_extra bits][copy_extra bits][literals...][dist code][dist_extra bits]`
/// Then emit sentinel ICC code 63.
pub fn compress(data: &[u8]) -> Vec<u8> {
    let original_length = data.len();

    // ── Special case: empty input ─────────────────────────────────────────
    if original_length == 0 {
        let mut out = Vec::with_capacity(13);
        out.extend_from_slice(&0u32.to_be_bytes());
        out.push(1u8); // icc_entry_count = 1 (sentinel only)
        out.push(0u8); // dist_entry_count = 0
        out.push(0u8); out.push(0u8); out.push(0u8); out.push(0u8);
        out.push(63u8); out.push(1u8); // sentinel: symbol=63, code_length=1
        out.push(0x00u8);              // bit stream: "0" padded to byte
        return out;
    }

    // ── Pass 1: LZ matching → final_commands ─────────────────────────────
    //
    // Produces a sequence of Commands, each with copy_length ≥ 4 (ICC
    // requirement) and copy_distance ≥ 1 for real back-references, OR
    // copy_distance == 0 for "no-copy" commands (pure literal blocks).
    //
    // When pending literals overflow the maximum insert range of any ICC code
    // that covers the current copy_length, we emit the overflow as "no-copy"
    // commands (dist=0), which the decoder and encoder treat as pure-literal
    // blocks. This avoids injecting spurious bytes into the output stream.

    const MAX_INSERT_PER_CMD: usize = 32; // max_insert_for_copy(4)

    let mut final_commands: Vec<Command> = Vec::new();
    let mut insert_buf: Vec<u8>         = Vec::new();
    let mut pos = 0usize;

    while pos < data.len() {
        let (dist, len) = find_longest_match(data, pos);

        if len >= MIN_MATCH as u32 {
            // Clamp len to the largest ICC-representable copy length ≤ len.
            // The ICC table has gaps (lengths 7, 12-13, 22-25, 42-49, etc.)
            // that cannot be encoded. We reduce to the nearest valid value;
            // the scanner resumes from pos+clamped_len on the next iteration.
            let len = clamp_copy_len(len);
            let pending = std::mem::take(&mut insert_buf);
            let max_ins = max_insert_for_copy(len) as usize;

            if pending.len() <= max_ins {
                // All pending literals fit in one ICC command with this copy.
                final_commands.push(Command {
                    literals:      pending,
                    copy_length:   len,
                    copy_distance: dist,
                });
            } else {
                // Pending literals exceed this copy's insert capacity.
                // Emit all but the last max_ins pending bytes as "no-copy"
                // blocks (copy_distance=0), then bundle the remaining
                // max_ins bytes with the real copy.
                let split_point = pending.len() - max_ins;
                let (overflow, tail) = pending.split_at(split_point);
                flush_literals_as_no_copy(overflow.to_vec(), &mut final_commands);
                final_commands.push(Command {
                    literals:      tail.to_vec(),
                    copy_length:   len,
                    copy_distance: dist,
                });
            }
            pos += len as usize;
        } else {
            // No match at this position: accumulate into the pending buffer.
            insert_buf.push(data[pos]);
            pos += 1;
        }
    }

    // Flush any remaining pending literals as "no-copy" blocks.
    if !insert_buf.is_empty() {
        flush_literals_as_no_copy(insert_buf, &mut final_commands);
    }

    // Safety check (debug builds): verify all real back-references are valid.
    #[cfg(debug_assertions)]
    {
        let mut hist_len = 0usize;
        for cmd in &final_commands {
            hist_len += cmd.literals.len();
            assert!(
                cmd.copy_length >= 4,
                "command has copy_length {} < 4", cmd.copy_length
            );
            if cmd.copy_distance > 0 {
                assert!(
                    hist_len >= cmd.copy_distance as usize,
                    "copy_distance {} > hist_len {} (literals={})",
                    cmd.copy_distance, hist_len, cmd.literals.len()
                );
                hist_len += cmd.copy_length as usize;
            }
            // copy_distance==0: no copy bytes added, hist_len only grows by literals
        }
    }

    // ── Pass 2a: Tally frequencies ────────────────────────────────────────

    let mut lit_freq:  [HashMap<u16, u32>; 4] = Default::default();
    let mut icc_freq:  HashMap<u16, u32> = HashMap::new();
    let mut dist_freq: HashMap<u16, u32> = HashMap::new();

    let mut history: Vec<u8> = Vec::with_capacity(original_length + 64);

    for cmd in &final_commands {
        for &byte in &cmd.literals {
            let ctx = literal_context(history.last().copied());
            *lit_freq[ctx].entry(byte as u16).or_insert(0) += 1;
            history.push(byte);
        }

        let ins_len = cmd.literals.len() as u32;
        let icc = find_icc(ins_len, cmd.copy_length)
            .unwrap_or_else(|| find_icc_for_copy(cmd.copy_length));
        *icc_freq.entry(icc as u16).or_insert(0) += 1;

        if cmd.copy_distance == 0 {
            // "No-copy" command: distance symbol 32 signals the decoder to
            // skip the copy step entirely. No history bytes are appended.
            *dist_freq.entry(32u16).or_insert(0) += 1;
        } else {
            let dc = dist_code_for(cmd.copy_distance);
            *dist_freq.entry(dc as u16).or_insert(0) += 1;

            let start = history.len() - cmd.copy_distance as usize;
            for i in 0..cmd.copy_length as usize {
                let b = history[start + i];
                history.push(b);
            }
        }
    }

    *icc_freq.entry(63u16).or_insert(0) += 1; // sentinel

    // ── Pass 2b: Build Huffman trees ──────────────────────────────────────

    let icc_weights: Vec<(u16, u32)> = icc_freq.iter().map(|(&s, &f)| (s, f)).collect();
    let icc_tree = HuffmanTree::build(&icc_weights).expect("ICC tree build");
    let icc_code_table = icc_tree.canonical_code_table();

    // dist_freq is never empty because every command has a copy (dist_code).
    let dist_weights: Vec<(u16, u32)> = dist_freq.iter().map(|(&s, &f)| (s, f)).collect();
    let dist_tree = HuffmanTree::build(&dist_weights).expect("dist tree build");
    let dist_code_table = dist_tree.canonical_code_table();

    let mut lit_code_tables: [HashMap<u16, String>; 4] = Default::default();
    for ctx in 0..4 {
        if !lit_freq[ctx].is_empty() {
            let weights: Vec<(u16, u32)> = lit_freq[ctx].iter().map(|(&s, &f)| (s, f)).collect();
            let tree = HuffmanTree::build(&weights).expect("lit tree build");
            lit_code_tables[ctx] = tree.canonical_code_table();
        }
    }

    // ── Pass 2c: Encode bit stream ────────────────────────────────────────
    //
    // Encoding order per command (must mirror the decoder's read order):
    //   [ICC Huffman code]
    //   [insert_extra bits, LSB-first]  (0 bits if insert_extra = 0)
    //   [copy_extra bits, LSB-first]    (0 bits if copy_extra = 0)
    //   [literal_0 Huffman code in its context bucket]
    //   ...
    //   [literal_{N-1} Huffman code]
    //   [distance Huffman code]
    //   [dist_extra bits, LSB-first]    (0 bits if dist extra = 0)

    let mut bb = BitBuilder::new();
    let mut history2: Vec<u8> = Vec::with_capacity(original_length + 64);

    for cmd in &final_commands {
        let ins_len = cmd.literals.len() as u32;
        let icc = find_icc(ins_len, cmd.copy_length)
            .unwrap_or_else(|| find_icc_for_copy(cmd.copy_length));
        let e = &ICC_TABLE[icc as usize];

        // ICC symbol.
        bb.write_bit_string(icc_code_table.get(&(icc as u16)).expect("ICC code"));

        // Insert extra bits (LSB-first).
        if e.insert_extra > 0 {
            bb.write_raw_bits_lsb(ins_len - e.insert_base, e.insert_extra);
        }
        // Copy extra bits (LSB-first).
        if e.copy_extra > 0 {
            bb.write_raw_bits_lsb(cmd.copy_length - e.copy_base, e.copy_extra);
        }

        // Literals (context-bucketed Huffman codes).
        for &byte in &cmd.literals {
            let ctx = literal_context(history2.last().copied());
            bb.write_bit_string(
                lit_code_tables[ctx].get(&(byte as u16)).expect("lit code")
            );
            history2.push(byte);
        }

        // Distance symbol + extra bits.
        if cmd.copy_distance == 0 {
            // No-copy sentinel: emit distance symbol 32 with no extra bits.
            // The decoder recognises symbol 32 and skips the copy step.
            bb.write_bit_string(dist_code_table.get(&32u16).expect("no-copy dist code 32"));
            // No history update: no copy bytes are produced.
        } else {
            let dc      = dist_code_for(cmd.copy_distance);
            let d_entry = &DIST_TABLE[dc];
            bb.write_bit_string(dist_code_table.get(&(dc as u16)).expect("dist code"));
            if d_entry.extra_bits > 0 {
                bb.write_raw_bits_lsb(cmd.copy_distance - d_entry.base, d_entry.extra_bits);
            }

            // Simulate copy for history tracking.
            let start = history2.len() - cmd.copy_distance as usize;
            for i in 0..cmd.copy_length as usize {
                let b = history2[start + i];
                history2.push(b);
            }
        }
    }

    // End-of-data sentinel.
    bb.write_bit_string(icc_code_table.get(&63u16).expect("sentinel code"));
    let packed_bits = bb.finish();

    // ── Assemble wire format ──────────────────────────────────────────────

    let sorted_pairs = |map: &HashMap<u16, String>| -> Vec<(u16, usize)> {
        let mut v: Vec<(u16, usize)> = map.iter().map(|(&s, c)| (s, c.len())).collect();
        v.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
        v
    };

    let icc_pairs  = sorted_pairs(&icc_code_table);
    let dist_pairs = sorted_pairs(&dist_code_table);
    let lit_pairs: [Vec<(u16, usize)>; 4] = [
        sorted_pairs(&lit_code_tables[0]),
        sorted_pairs(&lit_code_tables[1]),
        sorted_pairs(&lit_code_tables[2]),
        sorted_pairs(&lit_code_tables[3]),
    ];

    let mut out = Vec::with_capacity(
        10 + 2 * (icc_pairs.len() + dist_pairs.len())
        + lit_pairs.iter().map(|v| 3 * v.len()).sum::<usize>()
        + packed_bits.len()
    );

    out.extend_from_slice(&(original_length as u32).to_be_bytes());
    out.push(icc_pairs.len()    as u8);
    out.push(dist_pairs.len()   as u8);
    out.push(lit_pairs[0].len() as u8);
    out.push(lit_pairs[1].len() as u8);
    out.push(lit_pairs[2].len() as u8);
    out.push(lit_pairs[3].len() as u8);

    for (sym, len) in &icc_pairs  { out.push(*sym as u8); out.push(*len as u8); }
    for (sym, len) in &dist_pairs { out.push(*sym as u8); out.push(*len as u8); }
    for pairs in &lit_pairs {
        for (sym, len) in pairs {
            out.extend_from_slice(&sym.to_be_bytes());
            out.push(*len as u8);
        }
    }
    out.extend_from_slice(&packed_bits);
    out
}

// ---------------------------------------------------------------------------
// Public API: decompress
// ---------------------------------------------------------------------------

/// Decompress CMP06 Brotli wire-format `data` and return the original bytes.
///
/// Stops when output reaches `original_length` bytes (stored in the header)
/// OR when the sentinel ICC code 63 is decoded — whichever comes first.
/// The early-stop-at-length behavior handles the synthetic trailing copies
/// appended by the encoder to terminate the insert-run of the last command.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 10 {
        return Err(format!("brotli: data too short ({} bytes)", data.len()));
    }

    let original_length = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;

    if original_length == 0 {
        return Ok(Vec::new());
    }

    let icc_entry_count  = data[4] as usize;
    let dist_entry_count = data[5] as usize;
    let ctx_entry_counts = [
        data[6] as usize,
        data[7] as usize,
        data[8] as usize,
        data[9] as usize,
    ];

    let mut off = 10usize;

    // Parse ICC code-length table.
    let mut icc_lengths: Vec<(u16, usize)> = Vec::with_capacity(icc_entry_count);
    for _ in 0..icc_entry_count {
        if off + 2 > data.len() {
            return Err("brotli: ICC table truncated".to_string());
        }
        icc_lengths.push((data[off] as u16, data[off + 1] as usize));
        off += 2;
    }

    // Parse distance code-length table.
    let mut dist_lengths: Vec<(u16, usize)> = Vec::with_capacity(dist_entry_count);
    for _ in 0..dist_entry_count {
        if off + 2 > data.len() {
            return Err("brotli: dist table truncated".to_string());
        }
        dist_lengths.push((data[off] as u16, data[off + 1] as usize));
        off += 2;
    }

    // Parse literal code-length tables 0–3.
    let mut lit_lengths: [Vec<(u16, usize)>; 4] = Default::default();
    for ctx in 0..4 {
        for _ in 0..ctx_entry_counts[ctx] {
            if off + 3 > data.len() {
                return Err(format!("brotli: lit table {} truncated", ctx));
            }
            let sym  = u16::from_be_bytes([data[off], data[off + 1]]);
            let clen = data[off + 2] as usize;
            lit_lengths[ctx].push((sym, clen));
            off += 3;
        }
    }

    // Reconstruct canonical code maps.
    let icc_code_map  = build_canonical_codes(&icc_lengths);
    let dist_code_map = build_canonical_codes(&dist_lengths);
    let lit_code_maps: [HashMap<u16, String>; 4] = [
        build_canonical_codes(&lit_lengths[0]),
        build_canonical_codes(&lit_lengths[1]),
        build_canonical_codes(&lit_lengths[2]),
        build_canonical_codes(&lit_lengths[3]),
    ];

    let icc_rev  = reverse_code_map(&icc_code_map);
    let dist_rev = reverse_code_map(&dist_code_map);
    let lit_revs: [HashMap<String, u16>; 4] = [
        reverse_code_map(&lit_code_maps[0]),
        reverse_code_map(&lit_code_maps[1]),
        reverse_code_map(&lit_code_maps[2]),
        reverse_code_map(&lit_code_maps[3]),
    ];

    let bits = unpack_bits(&data[off..]);
    let mut bit_pos = 0usize;

    // Read n bits LSB-first as u32.
    let read_bits = |bits: &[u8], pos: &mut usize, n: u32| -> Result<u32, String> {
        let mut val = 0u32;
        for i in 0..n {
            if *pos >= bits.len() {
                return Err("brotli: bit stream exhausted".to_string());
            }
            val |= (bits[*pos] as u32) << i;
            *pos += 1;
        }
        Ok(val)
    };

    // Decode one Huffman symbol by accumulating prefix bits.
    let next_huffman = |bits: &[u8], pos: &mut usize, rev: &HashMap<String, u16>|
        -> Result<u16, String>
    {
        let mut acc = String::new();
        loop {
            if *pos >= bits.len() {
                return Err("brotli: bit stream exhausted (huffman)".to_string());
            }
            acc.push(if bits[*pos] == 1 { '1' } else { '0' });
            *pos += 1;
            if let Some(&sym) = rev.get(&acc) {
                return Ok(sym);
            }
            if acc.len() > 32 {
                return Err("brotli: Huffman decode overrun".to_string());
            }
        }
    };

    let mut output: Vec<u8> = Vec::with_capacity(original_length);

    loop {
        // We stop as soon as we have original_length bytes, even mid-command.
        // This handles synthetic trailing copies added by the encoder.
        if output.len() >= original_length {
            break;
        }

        // Decode ICC symbol.
        let icc = next_huffman(&bits, &mut bit_pos, &icc_rev)?;
        if icc == 63 {
            break; // end-of-data sentinel
        }

        let e = &ICC_TABLE[icc as usize];

        // Decode insert_length and copy_length.
        let ins_extra  = read_bits(&bits, &mut bit_pos, e.insert_extra)?;
        let insert_len = e.insert_base + ins_extra;
        let copy_extra = read_bits(&bits, &mut bit_pos, e.copy_extra)?;
        let copy_len   = e.copy_base + copy_extra;

        // Decode insert_len literals.
        for _ in 0..insert_len {
            if output.len() >= original_length {
                break;
            }
            let ctx      = literal_context(output.last().copied());
            let byte_sym = next_huffman(&bits, &mut bit_pos, &lit_revs[ctx])?;
            output.push(byte_sym as u8);
        }

        // Decode distance and perform copy.
        {
            let dc = next_huffman(&bits, &mut bit_pos, &dist_rev)?;
            if dc == 32 {
                // No-copy sentinel (distance symbol 32): the ICC copy_length
                // is encoded in the stream but the copy itself is skipped.
                // No bytes are added to output; no extra bits follow.
            } else if copy_len > 0 {
                let d_entry   = &DIST_TABLE[dc as usize];
                let dist_xtra = read_bits(&bits, &mut bit_pos, d_entry.extra_bits)?;
                let copy_dist = d_entry.base + dist_xtra;

                let start = output.len().checked_sub(copy_dist as usize)
                    .ok_or_else(|| format!(
                        "brotli: copy distance {} > output length {}",
                        copy_dist, output.len()
                    ))?;

                for i in 0..copy_len as usize {
                    if output.len() >= original_length {
                        break;
                    }
                    let b = output[start + i];
                    output.push(b);
                }
            }
        }
    }

    if output.len() != original_length {
        return Err(format!(
            "brotli: length mismatch: expected {}, got {}",
            original_length, output.len()
        ));
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(data: &[u8]) {
        let compressed   = compress(data);
        let decompressed = decompress(&compressed).unwrap_or_else(|e| {
            panic!(
                "decompress failed for {:?}: {}",
                &data[..data.len().min(20)],
                e
            )
        });
        assert_eq!(
            decompressed, data,
            "roundtrip mismatch for {:?}",
            &data[..data.len().min(20)]
        );
    }

    // ── Test 1: Round-trip empty ──────────────────────────────────────────

    #[test]
    fn test_roundtrip_empty() {
        let compressed   = compress(b"");
        let decompressed = decompress(&compressed).expect("decompress empty");
        assert_eq!(decompressed, b"");
    }

    // ── Test 2: Round-trip single byte ────────────────────────────────────

    #[test]
    fn test_roundtrip_single_byte() {
        roundtrip(b"\x42"); // 'B'
        roundtrip(b"\x00");
        roundtrip(b"\xFF");
        roundtrip(b"A");
    }

    // ── Test 3: Round-trip all 256 distinct bytes ─────────────────────────
    //
    // All 256 byte values, one of each. This is maximally incompressible data
    // for a byte-level coder. The round-trip must be exact; we don't require
    // any compression ratio improvement.

    #[test]
    fn test_roundtrip_all_256_bytes() {
        let data: Vec<u8> = (0u8..=255).collect();
        roundtrip(&data);
    }

    // ── Test 4: Round-trip 1024 × 'A' ────────────────────────────────────
    //
    // After a 4-byte literal run ("AAAA"), the encoder finds long overlapping
    // matches. The copy reproduces the remaining bytes with a compact ICC code.

    #[test]
    fn test_roundtrip_1024_as() {
        let data = vec![b'A'; 1024];
        roundtrip(&data);
        // Pure-repetition input should compress very well.
        let compressed = compress(&data);
        assert!(
            compressed.len() < 200,
            "1024×'A' should compress well, got {} bytes",
            compressed.len()
        );
    }

    // ── Test 5: Round-trip English prose ≥ 1024 bytes ────────────────────
    //
    // Compressed size must be < 80% of input size.

    #[test]
    fn test_roundtrip_english_prose() {
        let prose = concat!(
            "In the beginning God created the heavens and the earth. ",
            "Now the earth was formless and empty, darkness was over the surface ",
            "of the deep, and the Spirit of God was hovering over the waters. ",
            "And God said, Let there be light, and there was light. ",
            "God saw that the light was good, and he separated the light from the darkness. ",
            "God called the light day, and the darkness he called night. ",
            "And there was evening, and there was morning, the first day. ",
            "The quick brown fox jumps over the lazy dog. ",
            "Pack my box with five dozen liquor jugs. ",
            "How valiantly little Jack Rabbit would fight against great odds, ",
            "lying on his stomach, kicking his feet, and screaming loudly. ",
            "Peter Piper picked a peck of pickled peppers. A peck of pickled peppers ",
            "Peter Piper picked. If Peter Piper picked a peck of pickled peppers, ",
            "where is the peck that Peter Piper picked? ",
            "She sells seashells by the seashore. ",
            "How much wood would a woodchuck chuck if a woodchuck could chuck wood. ",
            "A woodchuck would chuck as much wood as a woodchuck could chuck if ",
            "a woodchuck could chuck wood. The rain in Spain stays mainly in the plain. ",
            "I am the very model of a modern Major-General, ",
            "I have information vegetable, animal, and mineral.",
        );
        let data = prose.as_bytes();
        assert!(data.len() >= 1024, "prose must be >= 1024 bytes, got {}", data.len());
        roundtrip(data);
        let compressed = compress(data);
        let ratio = compressed.len() as f64 / data.len() as f64;
        assert!(
            ratio < 0.80,
            "English prose should compress below 80%: {:.1}% ({}/{} bytes)",
            ratio * 100.0, compressed.len(), data.len()
        );
    }

    // ── Test 6: Round-trip 512-byte deterministic binary blob ─────────────

    #[test]
    fn test_roundtrip_512_byte_deterministic() {
        // Simple LCG: x_{n+1} = (1664525 * x_n + 1013904223) mod 2^32.
        let mut state: u32 = 0xDEAD_BEEF;
        let data: Vec<u8> = (0..512).map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 24) as u8
        }).collect();
        roundtrip(&data);
    }

    // ── Test 7: Context bucket transitions ───────────────────────────────
    //
    // "abc123ABC" exercises buckets 0 (start), 3 (lower), 1 (digit), 2 (upper).

    #[test]
    fn test_roundtrip_context_transitions() {
        roundtrip(b"abc123ABC");

        // Verify context classification.
        assert_eq!(literal_context(None),        0, "start → bucket 0");
        assert_eq!(literal_context(Some(b' ')),  0, "space → bucket 0");
        assert_eq!(literal_context(Some(b'!')),  0, "punct → bucket 0");
        assert_eq!(literal_context(Some(b'0')),  1, "digit '0' → bucket 1");
        assert_eq!(literal_context(Some(b'9')),  1, "digit '9' → bucket 1");
        assert_eq!(literal_context(Some(b'A')),  2, "upper 'A' → bucket 2");
        assert_eq!(literal_context(Some(b'Z')),  2, "upper 'Z' → bucket 2");
        assert_eq!(literal_context(Some(b'a')),  3, "lower 'a' → bucket 3");
        assert_eq!(literal_context(Some(b'z')),  3, "lower 'z' → bucket 3");
    }

    // ── Test 8: Long-distance match (offset > 4096) ───────────────────────
    //
    // Exercises distance codes 24–31 (CMP06 extension, absent in CMP05).

    #[test]
    fn test_long_distance_match() {
        let signature: &[u8] = b"XYZXYZXYZ!"; // 10 unique-ish bytes
        // Fill the gap with non-repeating data that won't accidentally match.
        let mut data: Vec<u8> = Vec::with_capacity(5200);
        data.extend_from_slice(signature);
        // Use bytes in range 30..229, avoiding 'X'=88, 'Y'=89, 'Z'=90, '!'=33.
        for i in 0..(4096 + 50) {
            let b = (i % 170 + 40) as u8; // 40..209, skips overlap with signature
            // Avoid accidental matches with the signature bytes.
            let b = if b == 88 || b == 89 || b == 90 || b == 33 { 200 } else { b };
            data.push(b);
        }
        // Plant the signature again at offset > 4096.
        data.extend_from_slice(signature);

        roundtrip(&data);

        // The compressed form should be smaller (the long match saves bytes).
        let compressed = compress(&data);
        assert!(
            compressed.len() < data.len(),
            "long-distance data should compress: {} vs {}",
            compressed.len(), data.len()
        );
    }

    // ── Test 9: Wire format header check ─────────────────────────────────

    #[test]
    fn test_wire_format_header() {
        let data = b"hello world hello world";
        let compressed = compress(data);

        let orig_len = u32::from_be_bytes([
            compressed[0], compressed[1], compressed[2], compressed[3],
        ]);
        assert_eq!(orig_len as usize, data.len(), "original_length in header");
        assert!(compressed[4] > 0 && compressed[4] <= 64, "icc_entry_count");
        // dist_entry_count: 0–31 normal dist codes + optional code 32 (no-copy)
        assert!(compressed[5] <= 33, "dist_entry_count");

        roundtrip(data);
    }

    // ── Test: empty wire format exact bytes ───────────────────────────────

    #[test]
    fn test_empty_wire_format() {
        let compressed = compress(b"");
        assert_eq!(&compressed[0..4], &[0, 0, 0, 0], "orig len");
        assert_eq!(compressed[4],  1,  "icc_entry_count");
        assert_eq!(compressed[5],  0,  "dist_entry_count");
        assert_eq!(compressed[6],  0,  "ctx0_entry_count");
        assert_eq!(compressed[7],  0,  "ctx1_entry_count");
        assert_eq!(compressed[8],  0,  "ctx2_entry_count");
        assert_eq!(compressed[9],  0,  "ctx3_entry_count");
        assert_eq!(compressed[10], 63, "ICC symbol");
        assert_eq!(compressed[11], 1,  "ICC code_length");
        assert_eq!(compressed[12], 0,  "bit stream byte");
        assert_eq!(compressed.len(), 13);
    }

    // ── Additional round-trip tests ───────────────────────────────────────

    #[test]
    fn test_repeated_pattern() {
        let base = b"the quick brown fox ";
        let data: Vec<u8> = base.iter().cycle().take(base.len() * 20).copied().collect();
        roundtrip(&data);
    }

    #[test]
    fn test_alternating_bytes() {
        let data: Vec<u8> = (0..200).map(|i| if i % 2 == 0 { b'A' } else { b'B' }).collect();
        roundtrip(&data);
    }

    #[test]
    fn test_binary_sequence() {
        let data: Vec<u8> = (0..300).map(|i| (i % 256) as u8).collect();
        roundtrip(&data);
    }

    #[test]
    fn test_short_strings() {
        for s in [
            b"A" as &[u8], b"AB", b"ABC", b"ABCD",
            b"hello", b"hello world", b"AABCBBABC",
        ] {
            roundtrip(s);
        }
    }

    #[test]
    fn test_various_run_lengths() {
        for &n in &[1usize, 2, 3, 4, 5, 8, 10, 17, 32, 100, 256] {
            let data = vec![b'X'; n];
            roundtrip(&data);
        }
    }
}
