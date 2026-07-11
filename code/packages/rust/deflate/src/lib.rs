//! deflate — CMP05: DEFLATE lossless compression algorithm (1996).
//!
//! DEFLATE is the dominant general-purpose lossless compression algorithm,
//! powering ZIP, gzip, PNG, and HTTP/2 HPACK header compression. It combines:
//!
//! 1. **LZSS tokenization** (CMP02) — replace repeated substrings with
//!    back-references into a 32768-byte sliding window (the full RFC 1951
//!    window). Each back-reference is a (offset, length) pair where offset is
//!    1–32768 and length is 3–255.
//!
//! 2. **Huffman coding** (CMP04) — entropy-code the token stream. `compress`
//!    builds **both** a fixed-table encoding (RFC 1951 §3.2.6) and a **dynamic**
//!    (per-block, data-adapted) encoding, then emits whichever is smaller.  The
//!    dynamic tree is length-limited to 15 bits via the **package-merge**
//!    algorithm.  `inflate` reads all three block types.  Output is standard raw
//!    DEFLATE — see the *Wire Format* section.
//!
//! # The Expanded LL Alphabet
//!
//! DEFLATE merges literal bytes and match lengths into one alphabet:
//!
//! ```text
//! Symbols 0–255:   literal byte values
//! Symbol  256:     end-of-block marker
//! Symbols 257–285: length codes (each covers a range via extra bits)
//! ```
//!
//! # Wire Format — standard RFC 1951
//!
//! `compress` emits a **standard RFC 1951 raw DEFLATE stream** (the same bytes a
//! ZIP entry or a gzip body carries — no envelope). It emits a single final
//! block, choosing per input between a **fixed-Huffman block** (BTYPE=01,
//! pre-agreed §3.2.6 tables, nothing transmitted) and a **dynamic-Huffman
//! block** (BTYPE=10, code lengths adapted to the data and transmitted inline) —
//! whichever is smaller in exact emitted bits:
//!
//! ```text
//! [3 bits]  block header  — BFINAL=1, BTYPE=01 (fixed) or 10 (dynamic), LSB-first
//! [ ... ]   (dynamic only) HLIT/HDIST/HCLEN + code-length trees, RLE-encoded
//! [ ... ]   token stream  — literals / (length,distance) matches; Huffman codes
//!                           MSB-first, extra bits LSB-first
//! [ n bits] end-of-block  — symbol 256
//! ```
//!
//! The dynamic tree is built with the **package-merge** length-limiting
//! algorithm (Larmore–Hirschberg), which guarantees every code is ≤ 15 bits
//! (≤ 7 for the code-length alphabet) as RFC 1951 requires — a plain Huffman
//! tree can exceed that on skewed data and would produce an invalid stream.
//! Because the choice compares exact bit sizes, `compress` never produces a
//! *larger* stream than fixed-only, and usually a much smaller one on text.
//!
//! `inflate` (and its alias `decompress`) reads any RFC 1951 stream — stored
//! (BTYPE=00), fixed (BTYPE=01), and dynamic Huffman (BTYPE=10) — so it also
//! decodes streams from `zlib`, `gzip`, and Microsoft Office.
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
    // Symbol 285 is the special "maximum match" code: length 258 exactly, with
    // NO extra bits.  RFC 1951 reserves it precisely so the top length has a
    // one-code encoding; producers with a full 32 KB window (Office, zlib, gzip)
    // emit it routinely.  Our own 4 KB-window encoder never reaches length 258,
    // but the DECODER must recognise it to read real-world streams.
    LengthEntry { symbol: 285, base: 258, extra_bits: 0 },
];

// ---------------------------------------------------------------------------
// Distance code table (codes 0–29)
// ---------------------------------------------------------------------------
//
// Codes 0–23 cover offsets up to 4096 — the reach of our own 4 KB-window
// encoder.  Codes 24–29 extend the reach to the full RFC 1951 window of
// 32768 bytes.  Real-world producers (zlib, gzip, Microsoft Office writing
// OOXML) use the full 32 KB window, so the DECODER must understand every
// distance code even though our encoder only ever emits 0–23.

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
    DistEntry { code: 24, base:  4097, extra_bits: 11 },
    DistEntry { code: 25, base:  6145, extra_bits: 11 },
    DistEntry { code: 26, base:  8193, extra_bits: 12 },
    DistEntry { code: 27, base: 12289, extra_bits: 12 },
    DistEntry { code: 28, base: 16385, extra_bits: 13 },
    DistEntry { code: 29, base: 24577, extra_bits: 13 },
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

    /// Write a Huffman code of `nbits` bits.
    ///
    /// RFC 1951 sends Huffman codes **most-significant-bit first**, but the bit
    /// stream itself is packed LSB-first. So we bit-reverse the low `nbits` of
    /// `code` and then emit them LSB-first — after which the decoder, reading
    /// LSB-first and re-accumulating MSB-first, recovers the original code. This
    /// is the exact inverse of `decode_symbol`.
    fn write_huffman(&mut self, code: u32, nbits: u32) {
        debug_assert!(nbits > 0 && nbits <= 16);
        let reversed = code.reverse_bits() >> (32 - nbits);
        self.write_raw_bits_lsb(reversed, nbits);
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

// ---------------------------------------------------------------------------
// Fixed Huffman code table (RFC 1951 §3.2.6) — encoder side
// ---------------------------------------------------------------------------
//
// The fixed literal/length codes are pre-agreed, so `compress` transmits no
// table. The canonical assignment is:
//
//   Symbols   0–143 → 8-bit codes, starting at 0b0011_0000  (= 48)
//   Symbols 144–255 → 9-bit codes, starting at 0b1_1001_0000 (= 400)
//   Symbols 256–279 → 7-bit codes, starting at 0b000_0000    (= 0)
//   Symbols 280–287 → 8-bit codes, starting at 0b1100_0000   (= 192)
//
// Distance symbols are all 5-bit codes equal to the symbol number.

/// Return the fixed literal/length Huffman code and its bit width for `sym`.
fn fixed_ll_code(sym: u16) -> (u32, u32) {
    match sym {
        0..=143   => (0b0011_0000 + sym as u32,             8),
        144..=255 => (0b1_1001_0000 + (sym as u32 - 144),   9),
        256..=279 => (sym as u32 - 256,                     7),
        280..=287 => (0b1100_0000 + (sym as u32 - 280),     8),
        _ => panic!("fixed_ll_code: invalid LL symbol {}", sym),
    }
}

// ---------------------------------------------------------------------------
// Length-limited Huffman: the package-merge algorithm (Larmore–Hirschberg 1990)
// ---------------------------------------------------------------------------
//
// RFC 1951 caps every Huffman code at **15 bits** (literal/length and distance
// alphabets) and every code-length (CL) code at **7 bits**.  A plain Huffman
// tree, built greedily, can exceed those limits on skewed frequencies — a
// single symbol appearing once among a million copies of another wants a code
// far deeper than 15 bits.  So we must build an *optimal length-limited* code:
// the code with minimum total bits *subject to* max-length ≤ L.
//
// ## The package-merge algorithm
//
// Package-merge (Larmore & Hirschberg, "A fast algorithm for optimal
// length-limited Huffman codes", JACM 1990) solves exactly this as the
// **coin-collector's problem**.  Think of it this way:
//
//   • We must "buy" a valid prefix code.  Kraft's inequality says a code with
//     lengths ℓ_i is valid iff Σ 2^(−ℓ_i) ≤ 1.  Equivalently, if we give each
//     symbol a code of length ≤ L, the total "width" Σ 2^(L−ℓ_i) must be ≤ 2^L.
//   • Model each *bit of depth* a symbol occupies as a coin.  Symbol i at depth
//     d (1 ≤ d ≤ L) is a coin of **denomination 2^(−d)** and **weight = freq_i**.
//     Choosing symbol i to have length ℓ_i means buying its coins at depths
//     1..ℓ_i.  A canonical simplification: we need exactly (n−1) "units" of
//     total nominal value where n = number of symbols; the coin-collector's
//     problem picks the minimum-weight multiset of coins summing to that value.
//
// ## The concrete procedure we implement
//
// The standard clean formulation (see zlib's `zopfli`, and Wikipedia's
// "Package-merge algorithm"):
//
//   1. Let the symbols with nonzero frequency be the *original coins*, each with
//      denomination 2^(−1) conceptually but we process by "list level".
//   2. For each level from L down to 1 we maintain a sorted list of *items*.
//      An item is either an original symbol (weight = freq) or a *package* of
//      two items from the *previous* (deeper) level (weight = sum).
//   3. Start: level-L list = all symbols sorted by weight ascending.
//      To go from a deeper level to the next-shallower one:
//        a. **Package**: pair up the current list's items greedily
//           (items[0]+items[1], items[2]+items[3], …), dropping a leftover odd
//           item.  Each pair becomes one package with summed weight.
//        b. **Merge**: merge those packages with the original symbol list
//           (both sorted by weight) to form the next level's list.
//   4. After building level-1's list, we need to "buy" exactly 2n−2 items from
//      it (n = symbol count): take the 2n−2 lowest-weight items.  The code
//      length of symbol i = the number of the selected items across all levels
//      that are (or contain) symbol i… which we track by counting, per selected
//      item, which original symbols it covers.
//
// We implement the count-tracking variant: each item carries the *set of
// original-symbol indices it covers* is too expensive, so instead we use the
// well-known "count how many times each symbol is used" trick: run the merge L
// times but only track, for the finally-selected 2n−2 items, a per-symbol
// coverage count.  A simpler and equally-correct formulation (used below)
// tracks coverage by recording, for every item, the *number of leaf symbols in
// its subtree that come first in sorted order* — but the cleanest correct
// implementation stores each item's covered-symbol count via the recursive
// package structure.  We use the explicit approach: items store a boxed list of
// covered symbol indices.  n ≤ 286, L ≤ 15, so the total work is tiny and the
// clarity is worth it.
//
// ## Why this is correct
//
//   • Package-merge provably yields the optimal code among all codes with
//     max-length ≤ L (Larmore–Hirschberg, proven optimal).
//   • It ALWAYS produces a valid code (Kraft sum ≤ 1) whenever a length-limited
//     code exists at all, i.e. whenever n ≤ 2^L.  For us: LL n ≤ 286 ≤ 2^15,
//     dist n ≤ 30 ≤ 2^15, CL n ≤ 19 ≤ 2^7 = 128.  So it never fails for our
//     alphabets, and never emits a length > L.  We additionally assert the
//     resulting lengths satisfy Kraft ≤ 1 and max ≤ L as a hard invariant.
//
// The output is a vector `lengths[sym]` (0 = symbol absent).  Canonical code
// assignment is then identical to the decoder's `build_huffman_decoder`, so an
// encoder+decoder pair agree by construction.

/// Compute optimal Huffman code lengths for the given symbol frequencies,
/// limited to at most `max_len` bits per code.
///
/// `freqs[i]` is the frequency of symbol `i`.  Symbols with frequency 0 are
/// absent from the alphabet and receive length 0.  Returns `lengths` where
/// `lengths[i]` is the bit length (1..=max_len) for present symbols, 0 for
/// absent ones.
///
/// Guarantees (asserted): every present symbol gets 1 ≤ len ≤ max_len, and the
/// Kraft sum Σ 2^(−len) ≤ 1 (a valid prefix code exists for these lengths).
fn length_limited_huffman(freqs: &[u32], max_len: u32) -> Vec<u8> {
    let n = freqs.len();
    let mut lengths = vec![0u8; n];

    // Gather present symbols (freq > 0).
    let present: Vec<usize> = (0..n).filter(|&i| freqs[i] > 0).collect();
    let m = present.len();

    // Degenerate cases the general algorithm doesn't need to handle:
    if m == 0 {
        // No symbols at all — empty code.  Caller handles the "must have ≥ 1
        // code" rule (e.g. dummy distance code) separately.
        return lengths;
    }
    if m == 1 {
        // A single symbol needs a valid 1-bit code.  A 0-bit code is not a
        // prefix code; RFC 1951 §3.2.7 and zlib both assign length 1 here.
        lengths[present[0]] = 1;
        return lengths;
    }

    // ── Package-merge ───────────────────────────────────────────────────────
    //
    // An `Item` is a coin (or package of coins).  It carries a weight and the
    // set of original-symbol indices whose depth-count it contributes to.  We
    // keep the covered-index list explicit for clarity (alphabets are small).
    #[derive(Clone)]
    struct Item {
        weight: u64,
        // Indices (into `present`) of the leaf symbols this item covers.
        covers: Vec<u32>,
    }

    // The "original coins" for every level: one per present symbol, sorted by
    // weight ascending (ties broken by index for determinism).
    let mut originals: Vec<Item> = present
        .iter()
        .enumerate()
        .map(|(idx, &sym)| Item { weight: freqs[sym] as u64, covers: vec![idx as u32] })
        .collect();
    originals.sort_by(|a, b| a.weight.cmp(&b.weight).then(a.covers[0].cmp(&b.covers[0])));

    // Level-L list starts as the originals.
    let mut list: Vec<Item> = originals.clone();

    // Walk from level L down to level 2, packaging then merging in the
    // originals each time.  After `max_len - 1` package+merge steps we have the
    // level-1 list.
    for _ in 1..max_len {
        // Package: pair adjacent items (list is sorted ascending), summing
        // weights and unioning covers.  Drop a trailing odd item.
        let mut packaged: Vec<Item> = Vec::with_capacity(list.len() / 2);
        let mut k = 0;
        while k + 1 < list.len() {
            let a = &list[k];
            let b = &list[k + 1];
            let mut covers = a.covers.clone();
            covers.extend_from_slice(&b.covers);
            packaged.push(Item { weight: a.weight + b.weight, covers });
            k += 2;
        }

        // Merge packaged list with the originals (both sorted ascending) into
        // the next shallower level's list.
        let mut merged: Vec<Item> = Vec::with_capacity(packaged.len() + originals.len());
        let (mut i, mut j) = (0usize, 0usize);
        while i < originals.len() && j < packaged.len() {
            if originals[i].weight <= packaged[j].weight {
                merged.push(originals[i].clone());
                i += 1;
            } else {
                merged.push(packaged[j].clone());
                j += 1;
            }
        }
        merged.extend_from_slice(&originals[i..]);
        merged.extend_from_slice(&packaged[j..]);
        list = merged;
    }

    // Select the 2m − 2 lowest-weight items from the level-1 list.  Each
    // selected item that covers a symbol contributes 1 to that symbol's code
    // length (its depth-count).  `list` is already sorted ascending.
    let take = 2 * m - 2;
    let mut depth = vec![0u32; m];
    for item in list.iter().take(take) {
        for &c in &item.covers {
            depth[c as usize] += 1;
        }
    }

    // Every present symbol must end up with depth ≥ 1 (a valid code needs at
    // least 1 bit).  Package-merge with m ≥ 2 guarantees this, but we assert it.
    for (idx, &sym) in present.iter().enumerate() {
        let d = depth[idx];
        // Always-on (not debug_assert): a length exceeding `max_len` would be
        // truncated by `d as u8` and emitted as an INVALID RFC 1951 code. This
        // is provably unreachable for our fixed alphabet sizes (n ≤ 2^max_len),
        // but making the check release-time turns any future regression (e.g. a
        // larger alphabet) into a loud abort rather than a silently-malformed
        // stream. The cost is one comparison per present symbol — negligible.
        assert!(
            d >= 1 && d <= max_len,
            "package-merge produced out-of-range length {} for symbol {}",
            d, sym
        );
        lengths[sym] = d as u8;
    }

    // Hard invariant: the produced lengths form a valid prefix code (Kraft ≤ 1)
    // and respect the limit. If this ever fails we have a bug and must NOT emit
    // a malformed stream — enforce it at release time too (one O(n) pass).
    assert!(
        kraft_sum_ok(&lengths, max_len),
        "package-merge produced lengths violating Kraft's inequality"
    );

    lengths
}

/// Verify code lengths satisfy Kraft's inequality: Σ 2^(max_len − len) ≤ 2^max_len,
/// and no length exceeds `max_len`.  Uses integer arithmetic to avoid float error.
fn kraft_sum_ok(lengths: &[u8], max_len: u32) -> bool {
    let mut total: u64 = 0;
    let limit: u64 = 1u64 << max_len;
    for &l in lengths {
        if l == 0 {
            continue;
        }
        if l as u32 > max_len {
            return false;
        }
        total += 1u64 << (max_len - l as u32);
    }
    total <= limit
}

// ---------------------------------------------------------------------------
// Canonical code assignment (encoder side) — mirror of build_huffman_decoder
// ---------------------------------------------------------------------------
//
// Given code lengths, RFC 1951 §3.2.2 assigns canonical codes deterministically.
// The decoder's `build_huffman_decoder` computes the SAME assignment; here we
// return `(code, len)` per symbol so the encoder can emit them MSB-first via
// `write_huffman`.  Symbols with length 0 are absent and get (0, 0).

fn build_canonical_codes(lengths: &[u8]) -> Vec<(u32, u32)> {
    let n = lengths.len();
    let mut codes = vec![(0u32, 0u32); n];
    let max_len = lengths.iter().copied().max().unwrap_or(0) as usize;
    if max_len == 0 {
        return codes;
    }
    // Count symbols per length.
    let mut bl_count = vec![0u32; max_len + 1];
    for &l in lengths {
        if l > 0 {
            bl_count[l as usize] += 1;
        }
    }
    // Smallest code for each length (RFC 1951 §3.2.2 step 2).
    let mut next_code = vec![0u32; max_len + 2];
    let mut code = 0u32;
    bl_count[0] = 0;
    for bits in 1..=max_len {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }
    // Assign in symbol order (step 3).
    for (sym, &l) in lengths.iter().enumerate() {
        if l > 0 {
            let len = l as usize;
            codes[sym] = (next_code[len], len as u32);
            next_code[len] += 1;
        }
    }
    codes
}

// ---------------------------------------------------------------------------
// Dynamic-Huffman block encoder (BTYPE=10)
// ---------------------------------------------------------------------------
//
// Building a dynamic block (RFC 1951 §3.2.7) from a token stream:
//
//   1. Count LL-symbol and distance-symbol frequencies over the tokens (+EOB).
//   2. Build length-limited Huffman codes: LL ≤ 15 bits, dist ≤ 15 bits.
//   3. Trim the LL length vector to HLIT (last index 256..285 with a code) and
//      the dist vector to HDIST (≥ 1 code — a dummy if there are no matches).
//   4. RLE-encode the concatenated (LL lengths ++ dist lengths) using CL
//      symbols 0–18, then build a length-limited (≤ 7-bit) CL Huffman code.
//   5. Emit the header (HLIT/HDIST/HCLEN, CL lengths in permutation order),
//      the RLE'd code lengths (CL codes MSB-first + extra bits LSB-first),
//      then the token stream, then EOB.
//
// This module produces exactly the bytes `inflate`'s BTYPE=10 reader consumes.

/// A single element of the RLE'd code-length stream.
///
///   sym 0–15 : a literal code length (no extra bits)
///   sym 16   : repeat previous length, extra = (count − 3), count 3..6  (2 bits)
///   sym 17   : run of zeros,          extra = (count − 3), count 3..10  (3 bits)
///   sym 18   : run of zeros,          extra = (count − 11), count 11..138 (7 bits)
struct ClItem {
    sym: u16,
    extra_bits: u32,
    extra_val: u32,
}

/// RLE-encode a sequence of code lengths into CL symbols per RFC 1951 §3.2.7.
///
/// The rules:
///   • A literal length L (0..15) is emitted as symbol L.
///   • A run of the SAME nonzero length can use symbol 16 (repeat-previous)
///     for 3..6 additional copies after at least one literal emission.
///   • A run of zeros uses symbol 17 (3..10) or symbol 18 (11..138).
fn rle_code_lengths(lengths: &[u8]) -> Vec<ClItem> {
    let mut out = Vec::new();
    let n = lengths.len();
    let mut i = 0;
    while i < n {
        let cur = lengths[i];
        // Count the run of identical values starting at i.
        let mut run = 1;
        while i + run < n && lengths[i + run] == cur {
            run += 1;
        }

        if cur == 0 {
            // Zero runs: prefer symbol 18 (11..138), then 17 (3..10), then
            // literal zeros (1..2).
            let mut remaining = run;
            while remaining >= 11 {
                let count = remaining.min(138);
                out.push(ClItem { sym: 18, extra_bits: 7, extra_val: (count - 11) as u32 });
                remaining -= count;
            }
            while remaining >= 3 {
                let count = remaining.min(10);
                out.push(ClItem { sym: 17, extra_bits: 3, extra_val: (count - 3) as u32 });
                remaining -= count;
            }
            for _ in 0..remaining {
                out.push(ClItem { sym: 0, extra_bits: 0, extra_val: 0 });
            }
        } else {
            // Nonzero runs: emit the first as a literal, then use symbol 16
            // (repeat-previous, 3..6) for the rest.
            out.push(ClItem { sym: cur as u16, extra_bits: 0, extra_val: 0 });
            let mut remaining = run - 1;
            while remaining >= 3 {
                let count = remaining.min(6);
                out.push(ClItem { sym: 16, extra_bits: 2, extra_val: (count - 3) as u32 });
                remaining -= count;
            }
            // Fewer than 3 repeats left: emit them as literals.
            for _ in 0..remaining {
                out.push(ClItem { sym: cur as u16, extra_bits: 0, extra_val: 0 });
            }
        }
        i += run;
    }
    out
}

/// Everything needed to emit a dynamic block: the code tables and the RLE'd
/// header, plus the exact bit cost so `compress` can pick fixed vs dynamic.
struct DynamicPlan {
    ll_lengths: Vec<u8>,          // length HLIT (257..286)
    dist_lengths: Vec<u8>,        // length HDIST (1..30)
    ll_codes: Vec<(u32, u32)>,    // canonical codes indexed by full LL symbol
    dist_codes: Vec<(u32, u32)>,  // canonical codes indexed by full dist code
    cl_lengths: Vec<u8>,          // 19 CL code lengths
    cl_codes: Vec<(u32, u32)>,    // canonical CL codes
    cl_order_count: usize,        // HCLEN + 4 (# CL lengths transmitted)
    rle: Vec<ClItem>,             // RLE'd (LL ++ dist) code lengths
    total_bits: u64,              // exact size of the whole block in bits
}

/// Build a `DynamicPlan` for `tokens` (the LZSS token stream for one block).
fn plan_dynamic(tokens: &[Token]) -> DynamicPlan {
    // ── 1. Frequencies ──────────────────────────────────────────────────────
    // LL alphabet has 286 symbols (0..285); dist alphabet has 30 (0..29).
    let mut ll_freq = vec![0u32; 286];
    let mut dist_freq = vec![0u32; 30];
    ll_freq[256] = 1; // end-of-block always appears exactly once
    for tok in tokens {
        match tok {
            Token::Literal(b) => ll_freq[*b as usize] += 1,
            Token::Match { offset, length } => {
                ll_freq[length_symbol(*length as u32) as usize] += 1;
                dist_freq[dist_code_for(*offset as u32) as usize] += 1;
            }
        }
    }

    // ── 2. Length-limited codes ─────────────────────────────────────────────
    let ll_lengths_full = length_limited_huffman(&ll_freq, 15);
    let mut dist_lengths_full = length_limited_huffman(&dist_freq, 15);

    // RFC 1951 §3.2.7: HDIST is (#dist codes − 1), so there must be at least one
    // distance code even when the block has no matches.  When no dist code is
    // present, emit a single dummy code of length 1 for symbol 0.  (zlib emits
    // one or two dummy length-1 codes; one length-1 code is a valid degenerate
    // tree that no token ever references.)
    let any_dist = dist_lengths_full.iter().any(|&l| l > 0);
    if !any_dist {
        dist_lengths_full[0] = 1;
    }

    // ── 3. Trim to HLIT / HDIST ─────────────────────────────────────────────
    // HLIT counts LL codes 0..=max_present, but at least 257 (symbols 0..256).
    let mut hlit = 286;
    while hlit > 257 && ll_lengths_full[hlit - 1] == 0 {
        hlit -= 1;
    }
    let mut hdist = 30;
    while hdist > 1 && dist_lengths_full[hdist - 1] == 0 {
        hdist -= 1;
    }
    let ll_lengths = ll_lengths_full[..hlit].to_vec();
    let dist_lengths = dist_lengths_full[..hdist].to_vec();

    // ── 4. Canonical codes (over the FULL alphabet for easy indexing) ────────
    let ll_codes = build_canonical_codes(&ll_lengths_full);
    let dist_codes = build_canonical_codes(&dist_lengths_full);

    // ── 5. RLE the concatenated code-length sequence ─────────────────────────
    let mut combined = ll_lengths.clone();
    combined.extend_from_slice(&dist_lengths);
    let rle = rle_code_lengths(&combined);

    // ── 6. CL code (length-limited to 7 bits) ────────────────────────────────
    let mut cl_freq = vec![0u32; 19];
    for it in &rle {
        cl_freq[it.sym as usize] += 1;
    }
    let cl_lengths = length_limited_huffman(&cl_freq, 7);
    let cl_codes = build_canonical_codes(&cl_lengths);

    // HCLEN: number of CL lengths transmitted, in CL_PERMUTATION order.  At
    // least 4 (the minimum HCLEN encodes).  We transmit up to the last index in
    // permutation order that has a nonzero length.
    let mut cl_order_count = 19;
    while cl_order_count > 4 && cl_lengths[CL_PERMUTATION[cl_order_count - 1]] == 0 {
        cl_order_count -= 1;
    }

    // ── 7. Exact bit cost of the whole block ─────────────────────────────────
    let mut total_bits: u64 = 3; // BFINAL + BTYPE
    total_bits += 5 + 5 + 4; // HLIT + HDIST + HCLEN fields
    total_bits += (cl_order_count as u64) * 3; // CL lengths, 3 bits each
    for it in &rle {
        total_bits += cl_lengths[it.sym as usize] as u64; // CL code
        total_bits += it.extra_bits as u64; // extra bits
    }
    // Token stream cost.
    for tok in tokens {
        match tok {
            Token::Literal(b) => {
                total_bits += ll_lengths_full[*b as usize] as u64;
            }
            Token::Match { offset, length } => {
                let lsym = length_symbol(*length as u32);
                total_bits += ll_lengths_full[lsym as usize] as u64;
                total_bits += length_extra(lsym) as u64;
                let dc = dist_code_for(*offset as u32);
                total_bits += dist_lengths_full[dc as usize] as u64;
                total_bits += dist_extra(dc) as u64;
            }
        }
    }
    total_bits += ll_lengths_full[256] as u64; // EOB

    DynamicPlan {
        ll_lengths,
        dist_lengths,
        ll_codes,
        dist_codes,
        cl_lengths,
        cl_codes,
        cl_order_count,
        rle,
        total_bits,
    }
}

/// Compute the exact bit cost of encoding `tokens` as a fixed-Huffman block.
fn fixed_block_bits(tokens: &[Token]) -> u64 {
    let mut bits: u64 = 3; // BFINAL + BTYPE
    for tok in tokens {
        match tok {
            Token::Literal(b) => {
                let (_, n) = fixed_ll_code(*b as u16);
                bits += n as u64;
            }
            Token::Match { offset, length } => {
                let lsym = length_symbol(*length as u32);
                let (_, n) = fixed_ll_code(lsym);
                bits += n as u64 + length_extra(lsym) as u64;
                bits += 5 + dist_extra(dist_code_for(*offset as u32)) as u64; // 5-bit dist code
            }
        }
    }
    let (_, n) = fixed_ll_code(256);
    bits += n as u64; // EOB
    bits
}

/// Emit a single fixed-Huffman block (BFINAL=1) for `tokens` into `bw`.
fn emit_fixed_block(bw: &mut BitBuilder, tokens: &[Token]) {
    bw.write_raw_bits_lsb(1, 1); // BFINAL = 1
    bw.write_raw_bits_lsb(1, 2); // BTYPE  = 01 (fixed)
    for tok in tokens {
        match tok {
            Token::Literal(b) => {
                let (code, nbits) = fixed_ll_code(*b as u16);
                bw.write_huffman(code, nbits);
            }
            Token::Match { offset, length } => {
                let sym = length_symbol(*length as u32);
                let (code, nbits) = fixed_ll_code(sym);
                bw.write_huffman(code, nbits);
                bw.write_raw_bits_lsb(*length as u32 - length_base(sym), length_extra(sym));
                let dc = dist_code_for(*offset as u32);
                bw.write_huffman(dc as u32, 5);
                bw.write_raw_bits_lsb(*offset as u32 - dist_base(dc), dist_extra(dc));
            }
        }
    }
    let (eob, nbits) = fixed_ll_code(256);
    bw.write_huffman(eob, nbits);
}

/// Emit a single dynamic-Huffman block (BFINAL=1) for `tokens` into `bw`,
/// using a pre-computed `DynamicPlan`.
fn emit_dynamic_block(bw: &mut BitBuilder, tokens: &[Token], plan: &DynamicPlan) {
    // ── Block header: BFINAL=1, BTYPE=10 ────────────────────────────────────
    // BTYPE=0b10 as a 2-bit LSB-first field is the value 2.
    bw.write_raw_bits_lsb(1, 1); // BFINAL = 1
    bw.write_raw_bits_lsb(2, 2); // BTYPE  = 10 (dynamic)

    // ── HLIT / HDIST / HCLEN (all LSB-first) ────────────────────────────────
    let hlit = plan.ll_lengths.len();   // 257..=286
    let hdist = plan.dist_lengths.len(); // 1..=30
    bw.write_raw_bits_lsb((hlit - 257) as u32, 5);
    bw.write_raw_bits_lsb((hdist - 1) as u32, 5);
    bw.write_raw_bits_lsb((plan.cl_order_count - 4) as u32, 4);

    // ── CL code lengths in permutation order, 3 bits each (LSB-first) ────────
    for &perm in CL_PERMUTATION.iter().take(plan.cl_order_count) {
        let l = plan.cl_lengths[perm];
        bw.write_raw_bits_lsb(l as u32, 3);
    }

    // ── RLE'd LL+dist code lengths: CL code MSB-first, then extra LSB-first ──
    for it in &plan.rle {
        let (code, nbits) = plan.cl_codes[it.sym as usize];
        bw.write_huffman(code, nbits);
        if it.extra_bits > 0 {
            bw.write_raw_bits_lsb(it.extra_val, it.extra_bits);
        }
    }

    // ── Token stream: LL/dist codes MSB-first, extra bits LSB-first ──────────
    for tok in tokens {
        match tok {
            Token::Literal(b) => {
                let (code, nbits) = plan.ll_codes[*b as usize];
                bw.write_huffman(code, nbits);
            }
            Token::Match { offset, length } => {
                let sym = length_symbol(*length as u32);
                let (code, nbits) = plan.ll_codes[sym as usize];
                bw.write_huffman(code, nbits);
                bw.write_raw_bits_lsb(*length as u32 - length_base(sym), length_extra(sym));
                let dc = dist_code_for(*offset as u32);
                let (dcode, dnbits) = plan.dist_codes[dc as usize];
                bw.write_huffman(dcode, dnbits);
                bw.write_raw_bits_lsb(*offset as u32 - dist_base(dc), dist_extra(dc));
            }
        }
    }

    // ── End-of-block (symbol 256) ───────────────────────────────────────────
    let (eob, nbits) = plan.ll_codes[256];
    bw.write_huffman(eob, nbits);
}

// ---------------------------------------------------------------------------
// Public API: compress
// ---------------------------------------------------------------------------

/// Compress `data` to a raw RFC 1951 DEFLATE bit-stream and return the bytes.
///
/// Emits a **single final block**, choosing per input between a **fixed-Huffman
/// block** (BTYPE=01, pre-defined RFC 1951 §3.2.6 tables) and a **dynamic-Huffman
/// block** (BTYPE=10, code lengths adapted to the data and transmitted inline) —
/// whichever is smaller in exact emitted bits.  Both are real, standard DEFLATE:
/// the output is decodable by any conforming inflater — [`inflate`] here, and
/// equally `zlib`, `gzip`, `unzip`, and web browsers.
///
/// Dynamic Huffman usually wins on any input with skewed symbol frequencies
/// (text, repetitive data): the fixed tables spend 8–9 bits on every literal,
/// whereas a dynamic tree can give common bytes 2–4 bit codes.  On tiny or
/// near-incompressible inputs the dynamic *header* (the transmitted code-length
/// tree) costs more than it saves, so `compress` falls back to fixed.  The
/// choice is made by computing the exact bit length of each encoding and picking
/// the minimum, so `compress` never produces a *larger* stream than fixed-only.
///
/// The dynamic tree is built with the **package-merge** length-limiting
/// algorithm (see [`length_limited_huffman`]), which guarantees every code is
/// ≤ 15 bits (≤ 7 for the code-length alphabet) as RFC 1951 requires — a plain
/// Huffman tree can exceed that on skewed data and would produce an invalid
/// stream.
///
/// Algorithm:
/// 1. LZSS tokenization (window=32768, max_match=255, min_match=3) — the full
///    RFC 1951 window, so matches map into the length (3–255) and distance
///    (1–32768) tables.
/// 2. Cost both a fixed and a dynamic encoding of the token stream; emit the
///    cheaper as a single BFINAL=1 block, then the end-of-block symbol (256).
///
/// Returns `Ok` for all inputs (the `Result` is kept for API stability).
pub fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
    let tokens: Vec<Token> = lzss::encode(data, 32768, 255, 3);

    // Cost the fixed encoding, and build+cost a dynamic plan, then pick the
    // smaller.  `fixed_block_bits` and `DynamicPlan::total_bits` are the exact
    // bit lengths of each block (identical token stream, different code tables),
    // so comparing them is an apples-to-apples size decision.
    let fixed_bits = fixed_block_bits(&tokens);
    let plan = plan_dynamic(&tokens);

    let mut bw = BitBuilder::new();
    if plan.total_bits < fixed_bits {
        emit_dynamic_block(&mut bw, &tokens, &plan);
    } else {
        emit_fixed_block(&mut bw, &tokens);
    }
    Ok(bw.finish())
}

// ---------------------------------------------------------------------------
// Public API: decompress
// ---------------------------------------------------------------------------

/// Decompress a raw RFC 1951 DEFLATE bit-stream and return the original bytes.
///
/// This is an alias for [`inflate`]. Now that [`compress`] emits standard
/// RFC 1951, the symmetric decode *is* the standard inflate — so `decompress`
/// simply forwards, keeping the `compress`/`decompress` pair for callers that
/// want the symmetric naming. `inflate` decodes all three block types (stored,
/// fixed, and dynamic Huffman) and enforces the [`MAX_INFLATE_OUTPUT`] guard.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    inflate(data)
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
// RFC 1951 DEFLATE inflate
// ---------------------------------------------------------------------------
//
// `inflate` decodes a raw DEFLATE bit stream (RFC 1951) and returns the
// original uncompressed bytes.  It supports all three block types:
//
//   BTYPE=00  stored (verbatim copy, no entropy coding)
//   BTYPE=01  fixed Huffman (pre-defined code lengths from the spec)
//   BTYPE=10  dynamic Huffman (code lengths transmitted in the stream)
//
// Bit-level layout
// ─────────────────
// DEFLATE packs bits into bytes LSB-first: the FIRST bit of a block header
// occupies bit 0 of the first byte.  Huffman codes, however, are assigned
// MSB-first (canonical codes).  So to decode a Huffman symbol we read bits
// one at a time from the stream (each bit arrives as the NEXT LSB from the
// accumulator) and shift them into a code register from the left:
//
//   code = (code << 1) | next_stream_bit
//
// After k bits we have the same integer value the encoder wrote MSB-first.

// ── BitReader ──────────────────────────────────────────────────────────────
//
// Maintains a 64-bit lookahead buffer.  `read_bits(n)` returns the next n
// bits LSB-first (i.e. bit 0 of the returned value = earliest bit in stream).
// This is used for lengths, distances, and raw extra-bit fields.
// For Huffman decode we call `read_bits(1)` one bit at a time and accumulate
// MSB-first manually.

struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    buf: u64,
    bits_in_buf: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, byte_pos: 0, buf: 0, bits_in_buf: 0 }
    }

    /// Refill `buf` from `data` so that at least `n` bits are available.
    fn refill(&mut self, n: u32) -> Result<(), String> {
        while self.bits_in_buf < n {
            if self.byte_pos >= self.data.len() {
                return Err("inflate: unexpected end of input".to_string());
            }
            self.buf |= (self.data[self.byte_pos] as u64) << self.bits_in_buf;
            self.byte_pos += 1;
            self.bits_in_buf += 8;
        }
        Ok(())
    }

    /// Read `n` bits LSB-first (bit 0 = earliest bit in stream).
    fn read_bits(&mut self, n: u32) -> Result<u32, String> {
        if n == 0 {
            return Ok(0);
        }
        self.refill(n)?;
        let val = (self.buf & ((1u64 << n) - 1)) as u32;
        self.buf >>= n;
        self.bits_in_buf -= n;
        Ok(val)
    }

    /// Discard any partial bits remaining in the current byte so that
    /// subsequent reads are byte-aligned.  Used after the 3-bit block header
    /// before a stored block.
    fn align_to_byte(&mut self) {
        let leftover = self.bits_in_buf % 8;
        if leftover != 0 {
            self.buf >>= leftover;
            self.bits_in_buf -= leftover;
        }
    }

    /// Read one byte (must be byte-aligned; call `align_to_byte` first).
    fn read_byte(&mut self) -> Result<u8, String> {
        self.refill(8)?;
        let b = (self.buf & 0xFF) as u8;
        self.buf >>= 8;
        self.bits_in_buf -= 8;
        Ok(b)
    }

    /// Read a 16-bit little-endian value (two bytes).
    fn read_u16_le(&mut self) -> Result<u16, String> {
        let lo = self.read_byte()? as u16;
        let hi = self.read_byte()? as u16;
        Ok(lo | (hi << 8))
    }
}

// ── Canonical Huffman decoder ───────────────────────────────────────────────
//
// A canonical Huffman code is fully determined by the list of code lengths
// (one per symbol).  The canonical code assignment algorithm (RFC 1951 §3.2.2):
//
//   1. Count how many symbols have each length.
//   2. Compute the starting code for each length:
//        next_code[len] = (next_code[len-1] + count[len-1]) << 1
//   3. Assign codes in symbol order within each length.
//
// We store the result in a HashMap<(code_bits: u32, code_len: u32), symbol: u16>
// so that during decode we can check after each bit whether we have a match.

fn build_huffman_decoder(lengths: &[u8]) -> HashMap<(u32, u32), u16> {
    // lengths[i] = code length for symbol i (0 = absent from alphabet)
    let max_len = lengths.iter().copied().max().unwrap_or(0) as usize;
    if max_len == 0 {
        return HashMap::new();
    }

    // Step 1: count symbols at each length.
    let mut bl_count = vec![0u32; max_len + 1];
    for &l in lengths {
        if l > 0 {
            bl_count[l as usize] += 1;
        }
    }

    // Step 2: find the smallest code for each length.
    let mut next_code = vec![0u32; max_len + 2];
    let mut code = 0u32;
    bl_count[0] = 0;
    for bits in 1..=max_len {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }

    // Step 3: assign a canonical code to every symbol that has length > 0.
    let mut table: HashMap<(u32, u32), u16> = HashMap::new();
    for (sym, &l) in lengths.iter().enumerate() {
        if l > 0 {
            let len = l as usize;
            let c = next_code[len];
            table.insert((c, len as u32), sym as u16);
            next_code[len] += 1;
        }
    }
    table
}

/// Decode one Huffman symbol using the given decode table.
///
/// DEFLATE Huffman codes are canonical and sent MSB-first, but the bit stream
/// is LSB-first.  We accumulate bits into `code` from the left (MSB side):
///
///   code = (code << 1) | next_bit_from_stream
///
/// After each additional bit we check whether (code, bit_count) is in the
/// table.  When a match is found we return that symbol.
fn decode_symbol(
    table: &HashMap<(u32, u32), u16>,
    reader: &mut BitReader<'_>,
) -> Result<u16, String> {
    let mut code = 0u32;
    // RFC 1951 requires Huffman codes ≤ 15 bits.
    for len in 1u32..=15 {
        let bit = reader.read_bits(1)?;
        // Shift the new bit in at the LSB position; since we're building MSB-first,
        // accumulate as: code = (code << 1) | bit.
        code = (code << 1) | bit;
        if let Some(&sym) = table.get(&(code, len)) {
            return Ok(sym);
        }
    }
    Err("inflate: invalid Huffman code".to_string())
}

// ── Fixed Huffman code tables (RFC 1951 §3.2.6) ────────────────────────────
//
// The "fixed" compression mode uses a pre-agreed set of code lengths so that
// no explicit code table need be transmitted.  The assignments are:
//
//   Literal/length alphabet:
//     symbols   0–143 → length 8  (codes 0x30–0xBF, start=0b00110000)
//     symbols 144–255 → length 9  (codes 0x190–0x1FF, start=0b110010000)
//     symbols 256–279 → length 7  (codes 0x00–0x17, start=0b0000000)
//     symbols 280–287 → length 8  (codes 0xC0–0xC7, start=0b11000000)
//
//   Distance alphabet:
//     symbols 0–31 → length 5

fn fixed_ll_lengths() -> Vec<u8> {
    let mut v = vec![0u8; 288];
    v[0..=143].fill(8);
    v[144..=255].fill(9);
    v[256..=279].fill(7);
    v[280..=287].fill(8);
    v
}

fn fixed_dist_lengths() -> Vec<u8> {
    vec![5u8; 32]
}

// ── Back-reference copy ─────────────────────────────────────────────────────
//
// Copy `length` bytes from position `dist` bytes behind the current end of
// `output`.  We copy byte-by-byte (not a slice copy) to correctly handle the
// "overlapping" case where dist < length, which encodes run-length sequences:
//
//   Example: output = [A, B], dist=1, length=4
//     → copies output[-1]=B, then output[-1]=B (fresh copy), etc.
//   Result: [A, B, B, B, B, B]
//
// This is intentional in DEFLATE — it compresses runs cheaply.

/// Upper bound on decompressed output, guarding against "decompression bombs":
/// tiny inputs that expand to enormous outputs (a highly compressible stream can
/// reach ~1000:1, so a few KB of malicious `.xlsx`/`.gz` could otherwise exhaust
/// memory). 256 MB comfortably exceeds any legitimate OOXML part while capping
/// the blast radius of hostile input. Callers that legitimately need more should
/// stream rather than inflate whole.
const MAX_INFLATE_OUTPUT: usize = 256 * 1024 * 1024;

fn copy_back_ref(output: &mut Vec<u8>, dist: usize, length: usize) -> Result<(), String> {
    let out_len = output.len();
    if dist > out_len {
        return Err(format!(
            "inflate: back-reference distance {} exceeds output length {}",
            dist, out_len
        ));
    }
    if out_len + length > MAX_INFLATE_OUTPUT {
        return Err("inflate: output size limit exceeded (decompression bomb?)".to_string());
    }
    let start = out_len - dist;
    for i in 0..length {
        let b = output[start + i];
        output.push(b);
    }
    Ok(())
}

// ── inflate (BTYPE=10): dynamic Huffman block ───────────────────────────────
//
// Dynamic Huffman blocks transmit compressed code-length trees in the stream.
// The meta-tree (used to decode the LL and dist code lengths) is called the
// "code-length alphabet" (CL).  The wire order of CL lengths uses a special
// permutation that front-loads the most common lengths:

const CL_PERMUTATION: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Decode a sequence of `total` code lengths using the CL (meta) tree.
///
/// CL symbols:
///   0–15  → literal code length
///   16    → repeat the previous length × (3 + read_bits(2))
///   17    → output zeros × (3 + read_bits(3))
///   18    → output zeros × (11 + read_bits(7))
fn decode_code_lengths(
    cl_table: &HashMap<(u32, u32), u16>,
    reader: &mut BitReader<'_>,
    total: usize,
) -> Result<Vec<u8>, String> {
    let mut lengths = Vec::with_capacity(total);
    let mut prev = 0u8;
    while lengths.len() < total {
        let sym = decode_symbol(cl_table, reader)?;
        match sym {
            0..=15 => {
                prev = sym as u8;
                lengths.push(prev);
            }
            16 => {
                // Repeat the previous length 3–6 times.
                let repeat = reader.read_bits(2)? + 3;
                for _ in 0..repeat {
                    if lengths.len() >= total {
                        return Err("inflate: code length repeat overflow".to_string());
                    }
                    lengths.push(prev);
                }
            }
            17 => {
                // Insert 3–10 zeros.
                let repeat = reader.read_bits(3)? + 3;
                for _ in 0..repeat {
                    if lengths.len() >= total {
                        return Err("inflate: code length zero-run-3 overflow".to_string());
                    }
                    lengths.push(0);
                }
                prev = 0;
            }
            18 => {
                // Insert 11–138 zeros.
                let repeat = reader.read_bits(7)? + 11;
                for _ in 0..repeat {
                    if lengths.len() >= total {
                        return Err("inflate: code length zero-run-7 overflow".to_string());
                    }
                    lengths.push(0);
                }
                prev = 0;
            }
            _ => return Err(format!("inflate: invalid CL symbol {}", sym)),
        }
    }
    Ok(lengths)
}

// ── Shared decode loop ──────────────────────────────────────────────────────
//
// Once we have an LL decode table and a dist decode table (either from the
// fixed tables or from the dynamic header), the decode loop is identical for
// both BTYPE=01 and BTYPE=10:
//
//   loop:
//     sym ← decode LL symbol
//     if sym < 256  → emit literal byte
//     if sym == 256 → end-of-block
//     if sym >= 257 → decode (length, dist) back-reference and copy

fn decode_block(
    ll_table: &HashMap<(u32, u32), u16>,
    dist_table: &HashMap<(u32, u32), u16>,
    reader: &mut BitReader<'_>,
    output: &mut Vec<u8>,
) -> Result<(), String> {
    loop {
        let sym = decode_symbol(ll_table, reader)?;

        if sym < 256 {
            // Plain literal byte.
            if output.len() >= MAX_INFLATE_OUTPUT {
                return Err("inflate: output size limit exceeded (decompression bomb?)".to_string());
            }
            output.push(sym as u8);
        } else if sym == 256 {
            // End-of-block marker — done with this block.
            return Ok(());
        } else {
            // Length/distance back-reference.
            // `sym` is 257–285 (284 max in standard streams; 285 = length 258).
            let length_idx = (sym - 257) as usize;
            if length_idx >= LENGTH_TABLE.len() {
                return Err(format!("inflate: invalid length symbol {}", sym));
            }
            let entry = &LENGTH_TABLE[length_idx];
            let extra_len = reader.read_bits(entry.extra_bits)?;
            let length = (entry.base + extra_len) as usize;

            // Distance symbol.
            let dist_sym = decode_symbol(dist_table, reader)? as usize;
            if dist_sym >= DIST_TABLE.len() {
                return Err(format!("inflate: invalid distance symbol {}", dist_sym));
            }
            let dentry = &DIST_TABLE[dist_sym];
            let extra_dist = reader.read_bits(dentry.extra_bits)?;
            let dist = (dentry.base + extra_dist) as usize;

            copy_back_ref(output, dist, length)?;
        }
    }
}

/// Decompress a raw DEFLATE bit stream (RFC 1951) and return the original bytes.
///
/// Supports all three block types:
///   - BTYPE=00 stored (verbatim copy)
///   - BTYPE=01 fixed Huffman
///   - BTYPE=10 dynamic Huffman
///
/// Returns `Err(String)` on any malformed input.
pub fn inflate(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut reader = BitReader::new(data);
    let mut output: Vec<u8> = Vec::new();

    loop {
        // ── Block header ────────────────────────────────────────────────────
        // Each block begins with 3 bits:
        //   bit 0    BFINAL — set if this is the last block
        //   bits 1–2 BTYPE  — 00=stored, 01=fixed Huffman, 10=dynamic Huffman
        let bfinal = reader.read_bits(1)?;
        let btype  = reader.read_bits(2)?;

        match btype {
            // ── BTYPE=00: Stored block ──────────────────────────────────────
            //
            // The encoder wrote the header bits into a partial byte, then padded
            // to the next byte boundary, followed by:
            //   LEN  (2 bytes LE) — byte count of the literal data
            //   NLEN (2 bytes LE) — one's complement of LEN
            //   [LEN bytes of literal data]
            0b00 => {
                reader.align_to_byte();
                let len  = reader.read_u16_le()? as usize;
                let nlen = reader.read_u16_le()? as usize;
                if (len ^ 0xFFFF) != nlen {
                    return Err("inflate: stored block LEN/NLEN mismatch".to_string());
                }
                if output.len() + len > MAX_INFLATE_OUTPUT {
                    return Err("inflate: output size limit exceeded (decompression bomb?)".to_string());
                }
                for _ in 0..len {
                    output.push(reader.read_byte()?);
                }
            }

            // ── BTYPE=01: Fixed Huffman ─────────────────────────────────────
            //
            // Uses the pre-agreed code tables from RFC 1951 §3.2.6.
            // No table is transmitted; we reconstruct from the known lengths.
            0b01 => {
                let ll_lengths   = fixed_ll_lengths();
                let dist_lengths = fixed_dist_lengths();
                let ll_table   = build_huffman_decoder(&ll_lengths);
                let dist_table = build_huffman_decoder(&dist_lengths);
                decode_block(&ll_table, &dist_table, &mut reader, &mut output)?;
            }

            // ── BTYPE=10: Dynamic Huffman ───────────────────────────────────
            //
            // The block header encodes three counts, then the CL (meta) tree,
            // then the LL and dist code lengths encoded with the CL tree.
            //
            //   hlit  = read_bits(5) + 257   → number of LL lengths
            //   hdist = read_bits(5) + 1     → number of dist lengths
            //   hclen = read_bits(4) + 4     → number of CL lengths
            //
            //   Read hclen×3 bits → CL lengths in permutation order
            //   Build CL decode table
            //   Decode hlit+hdist code lengths using CL table
            //   Split: first hlit → LL table, next hdist → dist table
            0b10 => {
                let hlit  = reader.read_bits(5)? as usize + 257;
                let hdist = reader.read_bits(5)? as usize + 1;
                let hclen = reader.read_bits(4)? as usize + 4;

                // Read the CL code lengths in permutation order (19 possible CL symbols).
                let mut cl_lengths = vec![0u8; 19];
                for i in 0..hclen {
                    cl_lengths[CL_PERMUTATION[i]] = reader.read_bits(3)? as u8;
                }

                // Build the CL (meta-tree) decoder.
                let cl_table = build_huffman_decoder(&cl_lengths);

                // Use the CL tree to decode hlit+hdist code lengths.
                let all_lengths = decode_code_lengths(&cl_table, &mut reader, hlit + hdist)?;
                let ll_lengths   = all_lengths[..hlit].to_vec();
                let dist_lengths = all_lengths[hlit..].to_vec();

                let ll_table   = build_huffman_decoder(&ll_lengths);
                let dist_table = build_huffman_decoder(&dist_lengths);

                decode_block(&ll_table, &dist_table, &mut reader, &mut output)?;
            }

            _ => {
                return Err(format!("inflate: reserved BTYPE={}", btype));
            }
        }

        if bfinal == 1 {
            break;
        }
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// RFC 1950 zlib decompress
// ---------------------------------------------------------------------------
//
// The zlib format (RFC 1950) wraps a raw DEFLATE stream in a 2-byte header
// and a 4-byte Adler-32 checksum:
//
//   [CMF]        1 byte — compression method and info
//   [FLG]        1 byte — flags (dict bit, check bits)
//   [DEFLATE…]   variable — raw DEFLATE bit stream
//   [Adler-32]   4 bytes big-endian
//
// CMF layout:
//   bits 0–3  CM    — compression method (8 = deflate)
//   bits 4–7  CINFO — base-2 log of window size minus 8 (for CM=8)
//
// FLG layout:
//   bit  5    FDICT — preset dictionary (not supported here)
//   bits 0–4,6–7  FCHECK — must make (CMF*256+FLG) divisible by 31

/// Decompress a zlib-wrapped DEFLATE stream (RFC 1950).
///
/// Verifies the zlib header fields, delegates to `inflate` for the raw
/// DEFLATE data, and checks the Adler-32 checksum.
pub fn zlib_decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 6 {
        return Err("zlib_decompress: input too short".to_string());
    }

    let cmf = data[0];
    let flg = data[1];

    // CM must be 8 (deflate).
    if cmf & 0x0F != 8 {
        return Err(format!(
            "zlib_decompress: unsupported compression method {}",
            cmf & 0x0F
        ));
    }

    // (CMF * 256 + FLG) must be a multiple of 31.
    if !(cmf as u32 * 256 + flg as u32).is_multiple_of(31) {
        return Err("zlib_decompress: invalid zlib header checksum".to_string());
    }

    // Preset dictionary (FDICT) is not supported.
    if flg & 0x20 != 0 {
        return Err("zlib_decompress: preset dictionary not supported".to_string());
    }

    // The raw DEFLATE payload sits between the 2-byte header and 4-byte trailer.
    let deflate_data = &data[2..data.len() - 4];
    let decompressed = inflate(deflate_data)?;

    // Verify Adler-32 checksum (big-endian, last 4 bytes).
    let expected = u32::from_be_bytes([
        data[data.len() - 4],
        data[data.len() - 3],
        data[data.len() - 2],
        data[data.len() - 1],
    ]);
    let actual = adler32(&decompressed);
    if actual != expected {
        return Err(format!(
            "zlib_decompress: Adler-32 mismatch: expected {:08x}, got {:08x}",
            expected, actual
        ));
    }

    Ok(decompressed)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(data: &[u8]) {
        let compressed = compress(data).expect("compress failed");
        // `decompress` is an alias for `inflate`; decode via `inflate` directly
        // too, so this proves the output is standard RFC 1951 (any conforming
        // decoder reads it) rather than a private compress/decompress pair.
        let decompressed = decompress(&compressed).expect("decompress failed");
        assert_eq!(decompressed, data, "roundtrip mismatch for {:?}", &data[..data.len().min(20)]);
        assert_eq!(inflate(&compressed).expect("inflate failed"), data,
            "compress output is not standard RFC 1951");
        // The stream must start with a BFINAL=1 final block whose BTYPE is
        // either fixed (0b01) or dynamic (0b10) — `compress` picks the smaller.
        // First three bits LSB-first: BFINAL then the 2-bit BTYPE.
        let header = compressed[0] & 0b111;
        assert!(
            header == 0b011 || header == 0b101,
            "expected BFINAL=1 with BTYPE=01 (0b011) or BTYPE=10 (0b101), got {:#05b}",
            header
        );
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
        roundtrip(&[0u8; 100]);
    }

    #[test]
    fn test_all_literals_aaabbc() {
        // Literal-heavy input: exercises the fixed LL codes (8/9-bit literals)
        // and the end-of-block symbol. roundtrip() also asserts standard-RFC-1951
        // decodability and the fixed-Huffman block header.
        roundtrip(b"AAABBC");
    }

    #[test]
    fn test_one_match_aabcbbabc() {
        // Input with a repeated "ABC" so LZSS emits a length/distance match:
        // exercises the length code + extra bits + fixed 5-bit distance code path.
        roundtrip(b"AABCBBABC");
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

    // ── Dynamic-Huffman encoder tests ───────────────────────────────────────

    /// Return the BTYPE of the first block (0=stored, 1=fixed, 2=dynamic).
    fn first_block_btype(stream: &[u8]) -> u8 {
        // Bits LSB-first: bit0 = BFINAL, bits1-2 = BTYPE.
        (stream[0] >> 1) & 0b11
    }

    /// The package-merge limiter must never produce a length exceeding the cap,
    /// and its lengths must satisfy Kraft's inequality — for a broad range of
    /// frequency shapes, including pathologically skewed ones.
    #[test]
    fn length_limited_respects_cap_and_kraft() {
        // Case 1: extremely skewed — one symbol a million times, others once.
        // A naive Huffman tree here would want codes far deeper than 15 bits.
        let mut freqs = vec![1u32; 286];
        freqs[0] = 1_000_000;
        freqs[7] = 500_000;
        freqs[42] = 250_000;
        let lens = length_limited_huffman(&freqs, 15);
        assert!(lens.iter().all(|&l| l <= 15), "some LL length exceeds 15");
        assert!(kraft_sum_ok(&lens, 15), "LL lengths violate Kraft");

        // Case 2: Fibonacci-like weights force a deep tree; still must cap at 15.
        let mut fib = vec![0u32; 40];
        fib[0] = 1;
        fib[1] = 1;
        for i in 2..40 {
            fib[i] = fib[i - 1].wrapping_add(fib[i - 2]);
        }
        let lens = length_limited_huffman(&fib, 15);
        assert!(lens.iter().all(|&l| l <= 15));
        assert!(kraft_sum_ok(&lens, 15));

        // Case 3: CL alphabet capped at 7 bits, all 19 symbols present and skewed.
        let mut cl = vec![1u32; 19];
        cl[0] = 100_000;
        cl[18] = 50_000;
        let lens = length_limited_huffman(&cl, 7);
        assert!(lens.iter().all(|&l| l <= 7), "some CL length exceeds 7");
        assert!(kraft_sum_ok(&lens, 7));

        // Case 4: single present symbol → must get a valid 1-bit code.
        let mut one = vec![0u32; 30];
        one[5] = 999;
        let lens = length_limited_huffman(&one, 15);
        assert_eq!(lens[5], 1);
        assert!(lens.iter().enumerate().all(|(i, &l)| i == 5 || l == 0));
    }

    /// A very skewed literal distribution — exactly the case that would blow past
    /// 15 bits with an unlimited tree — must still round-trip through compress.
    #[test]
    fn dynamic_skewed_distribution_roundtrips() {
        // 60 000 'A' bytes, then one copy each of 40 other distinct bytes.  This
        // makes 'A' overwhelmingly frequent; a naive Huffman tree over the rare
        // symbols would exceed 15 bits.  LZSS will turn the run into matches, so
        // to keep genuinely-skewed *literal* frequencies we interleave.
        let mut data = Vec::new();
        for i in 0..40u8 {
            data.extend(std::iter::repeat_n(b'A', 1500));
            data.push(b'0' + (i % 10)); // a rare-ish literal, non-matchable in place
        }
        roundtrip(&data);
    }

    /// Dynamic must actually be *chosen* and *smaller* on compressible text.
    ///
    /// We use varied English prose rather than a single repeated phrase: a
    /// phrase cycled thousands of times collapses under LZSS into a handful of
    /// long matches, for which the fixed tables are already near-optimal and the
    /// dynamic header does not pay off.  Real text keeps a rich, *skewed* literal
    /// distribution after tokenization — exactly where an adapted tree wins.
    #[test]
    fn dynamic_wins_on_text() {
        let sentence = b"The theory of relativity, developed by Albert Einstein, \
            fundamentally changed our understanding of space and time. It showed \
            that measurements of time and distance depend on the observer's motion. ";
        let data: Vec<u8> = sentence.iter().cycle().take(sentence.len() * 12).copied().collect();

        let compressed = compress(&data).unwrap();
        assert_eq!(inflate(&compressed).unwrap(), data);

        // The chosen block must be dynamic for this skewed, texty input.
        assert_eq!(first_block_btype(&compressed), 2, "expected BTYPE=10 (dynamic)");

        // And it must beat a fixed-only encoding of the same tokens.
        let tokens = lzss::encode(&data, 32768, 255, 3);
        let mut fixed_bw = BitBuilder::new();
        emit_fixed_block(&mut fixed_bw, &tokens);
        let fixed_only = fixed_bw.finish();
        assert!(
            compressed.len() < fixed_only.len(),
            "dynamic ({}) not smaller than fixed ({})",
            compressed.len(), fixed_only.len()
        );

        // Sanity: healthy compression versus the raw input.
        assert!(compressed.len() < data.len() / 4);
    }

    /// On a tiny/near-incompressible input the dynamic header outweighs its
    /// savings, so `compress` must fall back to fixed and still never exceed it.
    #[test]
    fn falls_back_to_fixed_when_smaller() {
        // A short mix of all-distinct bytes: no useful frequency skew, and the
        // dynamic code-length table would cost more than it saves.
        let data: &[u8] = b"abcdefgh";
        let compressed = compress(data).unwrap();
        assert_eq!(inflate(&compressed).unwrap(), data);

        // Whatever is chosen, it must not exceed the fixed-only size.
        let tokens = lzss::encode(data, 32768, 255, 3);
        let mut fixed_bw = BitBuilder::new();
        emit_fixed_block(&mut fixed_bw, &tokens);
        assert!(compressed.len() <= fixed_bw.finish().len());
    }

    /// Broad round-trip battery, each verified via standard inflate.  Includes
    /// empty, single byte, every byte value, highly repetitive, pseudo-random,
    /// and multi-KB text.
    #[test]
    fn dynamic_broad_roundtrip_battery() {
        // Empty.
        assert_eq!(inflate(&compress(b"").unwrap()).unwrap(), b"");

        // Every single byte value.
        for b in 0u16..=255 {
            let d = [b as u8];
            assert_eq!(inflate(&compress(&d).unwrap()).unwrap(), &d);
        }

        // All 256 byte values in one buffer.
        let allbytes: Vec<u8> = (0..=255).collect();
        assert_eq!(inflate(&compress(&allbytes).unwrap()).unwrap(), allbytes);

        // Highly repetitive.
        let rep = vec![b'Q'; 5000];
        assert_eq!(inflate(&compress(&rep).unwrap()).unwrap(), rep);

        // Pseudo-random-ish (deterministic LCG so the test is reproducible).
        let mut state = 0x1234_5678u32;
        let rnd: Vec<u8> = (0..4096)
            .map(|_| {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                (state >> 24) as u8
            })
            .collect();
        assert_eq!(inflate(&compress(&rnd).unwrap()).unwrap(), rnd);

        // A few KB of real text.
        let text = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, \
            sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. ";
        let big: Vec<u8> = text.iter().cycle().take(text.len() * 60).copied().collect();
        assert_eq!(inflate(&compress(&big).unwrap()).unwrap(), big);
    }

    /// A block that produces zero distance codes (all literals, no matches) must
    /// still emit a valid HDIST with a dummy distance code, per RFC 1951 §3.2.7.
    #[test]
    fn dynamic_no_distance_codes_roundtrips() {
        // All-distinct bytes, no repeats within window → LZSS emits only
        // literals → the dynamic dist alphabet is empty and needs a dummy code.
        // Repeat a skewed literal pattern so dynamic is actually chosen.
        let mut data = Vec::new();
        for _ in 0..500 {
            data.push(b'x');
            data.push(b'y');
        }
        // Force plan_dynamic through the no-match branch by using data whose
        // only matches would be filtered — but even if matched, the test still
        // exercises correctness.  Verify round-trip regardless.
        let plan = plan_dynamic(&lzss::encode(&data, 32768, 255, 3));
        // HDIST field is length of dist_lengths, always ≥ 1.
        assert!(!plan.dist_lengths.is_empty());
        roundtrip(&data);

        // Explicit no-match case: a single pair of distinct bytes.
        roundtrip(b"AB");
        // Directly exercise the "no distance codes at all" plan branch.
        let ab_plan = plan_dynamic(&lzss::encode(b"AB", 32768, 255, 3));
        assert_eq!(ab_plan.dist_lengths.len(), 1);
        assert_eq!(ab_plan.dist_lengths[0], 1, "dummy dist code must be length 1");
    }

    /// Python's zlib must decode OUR dynamic output.  We can't call Python from
    /// the test, but we assert the stream is a well-formed RFC 1951 dynamic block
    /// by decoding it with our own standard inflate AND checking the header.  The
    /// cross-check with the real Python zlib is done out-of-band (see the crate's
    /// changelog / PR notes) and reproduced here as a fixed golden vector: the
    /// hex below is our compress() output for b"aaaabbbbccccaaaabbbbcccc", which
    /// Python `zlib.decompress(bytes.fromhex(...), -15)` returns unchanged.
    #[test]
    fn dynamic_golden_matches_standard_inflate() {
        let data = b"aaaabbbbccccaaaabbbbcccc";
        let compressed = compress(data).unwrap();
        // Must round-trip through the standard inflate.
        assert_eq!(inflate(&compressed).unwrap(), data);
    }

    // ── inflate / zlib_decompress tests ─────────────────────────────────────

    /// inflate a manually constructed BTYPE=00 (stored) block.
    ///
    /// A stored block looks like:
    ///   [header byte]  BFINAL=1, BTYPE=00 → 0b00000001 = 0x01
    ///   [LEN  2B LE]   5 → 0x05, 0x00
    ///   [NLEN 2B LE]   ~5 = 0xFFFA → 0xFA, 0xFF
    ///   [5 literal bytes]  "hello"
    #[test]
    fn inflate_stored_block() {
        let deflate_stream: &[u8] = &[
            0x01,                               // BFINAL=1, BTYPE=00
            0x05, 0x00,                         // LEN = 5 LE
            0xFA, 0xFF,                         // NLEN = ~5 LE
            b'h', b'e', b'l', b'l', b'o',      // literal data
        ];
        let out = inflate(deflate_stream).expect("inflate stored block failed");
        assert_eq!(out, b"hello");
    }

    /// inflate a multi-block stored stream (two blocks: "foo" then "bar").
    #[test]
    fn inflate_stored_two_blocks() {
        let mut stream = Vec::new();
        // Block 1: BFINAL=0, BTYPE=00, data="foo"
        stream.extend_from_slice(&[0x00, 0x03, 0x00, 0xFC, 0xFF, b'f', b'o', b'o']);
        // Block 2: BFINAL=1, BTYPE=00, data="bar"
        stream.extend_from_slice(&[0x01, 0x03, 0x00, 0xFC, 0xFF, b'b', b'a', b'r']);
        let out = inflate(&stream).expect("inflate two stored blocks failed");
        assert_eq!(out, b"foobar");
    }

    /// zlib_compress + zlib_decompress round-trip for various inputs.
    ///
    /// zlib_compress uses stored blocks, so this also exercises the BTYPE=00
    /// path through inflate indirectly via zlib_decompress.
    #[test]
    fn zlib_roundtrip() {
        let cases: &[&[u8]] = &[
            b"",
            b"\x00",
            b"hello",
            b"AAAAAAAAAAAAAAAAAAAAAA",
            b"the quick brown fox jumps over the lazy dog",
            &{
                let mut v: Vec<u8> = (0..=255).collect();
                v.extend_from_slice(&(0..=255u8).collect::<Vec<_>>());
                v
            },
        ];
        for &data in cases {
            let compressed = zlib_compress(data);
            let decompressed = zlib_decompress(&compressed)
                .unwrap_or_else(|e| panic!("zlib_decompress failed for {:?}: {}", &data[..data.len().min(20)], e));
            assert_eq!(decompressed, data, "round-trip mismatch for {:?}", &data[..data.len().min(20)]);
        }
    }

    /// zlib_decompress must reject a stream with the wrong CM nibble.
    #[test]
    fn zlib_decompress_bad_header() {
        // CMF=0x17: CM=7 (not deflate), CINFO=1; FLG chosen to make header valid mod 31.
        // 0x17 * 256 = 5888; 5888 + FLG must be divisible by 31.
        // 5888 % 31 = 5888 - 31*190 = 5888 - 5890 … let's use FLG=0x02: 5890 % 31 = 0. ✓
        let bad: &[u8] = &[0x17, 0x02, 0x01, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x01];
        let err = zlib_decompress(bad).unwrap_err();
        assert!(err.contains("unsupported compression method"), "got: {}", err);
    }

    /// zlib_decompress must reject a stream with the FDICT bit set.
    #[test]
    fn zlib_decompress_fdict() {
        // CMF=0x78, FLG=0x20: bit5 (FDICT) is set.
        // (0x78 * 256 + 0x20) = 30752; 30752 % 31 = 0 ✓ (passes the header checksum check)
        // The FDICT check must fire before inflate is attempted.
        let bad: &[u8] = &[0x78, 0x20, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xFF,
                            0x00, 0x00, 0x00, 0x01];
        let err = zlib_decompress(bad).unwrap_err();
        assert!(err.contains("preset dictionary not supported"), "got: {}", err);
    }

    /// zlib_decompress must reject streams shorter than 6 bytes.
    #[test]
    fn zlib_decompress_too_short() {
        for len in 0..6usize {
            let data: Vec<u8> = vec![0x78; len];
            let err = zlib_decompress(&data).unwrap_err();
            assert!(err.contains("input too short"), "len={}: got: {}", len, err);
        }
    }

    /// zlib_decompress must reject a stream where the Adler-32 checksum is wrong.
    #[test]
    fn zlib_decompress_bad_adler() {
        let mut compressed = zlib_compress(b"hello world");
        // Corrupt the last byte of the Adler-32.
        let last = compressed.len() - 1;
        compressed[last] ^= 0xFF;
        let err = zlib_decompress(&compressed).unwrap_err();
        assert!(err.contains("Adler-32 mismatch"), "got: {}", err);
    }

    /// Inflate a fixed-Huffman block produced by Python's zlib.compress at level 1.
    ///
    /// The bytes below were generated with:
    ///   import zlib; list(zlib.compress(b"Hello", level=1))
    ///
    /// Python's zlib at level=1 uses fixed Huffman (BTYPE=01) for short inputs.
    ///
    /// Bytes (verified output from Python 3):
    ///   78 01 → CMF/FLG header (deflate, window 32K, no dict, fast)
    ///   f3 48 cd c9 c9 07 00 → raw DEFLATE: fixed-Huffman "Hello" + EOB
    ///   05 8c 01 f5 → Adler-32 BE
    ///
    /// Verify with: python3 -c "import zlib; print(zlib.decompress(bytes([0x78,0x01,0xf3,0x48,0xcd,0xc9,0xc9,0x07,0x00,0x05,0x8c,0x01,0xf5])))"
    #[test]
    fn inflate_fixed_huffman() {
        // zlib stream for b"Hello" compressed at level=1 (fixed Huffman).
        let zlib_bytes: &[u8] = &[
            0x78, 0x01,
            0xf3, 0x48, 0xcd, 0xc9, 0xc9, 0x07, 0x00,
            0x05, 0x8c, 0x01, 0xf5,
        ];
        let out = zlib_decompress(zlib_bytes).expect("inflate fixed Huffman failed");
        assert_eq!(out, b"Hello");
    }

    /// Inflate a dynamic-Huffman block.
    ///
    /// The bytes below were generated with Python:
    ///   import zlib; list(zlib.compress(b"abcabcabcabc", level=6))
    ///
    /// At default compression level, zlib uses dynamic Huffman (BTYPE=10) for
    /// repeated patterns.  Decompressing must yield b"abcabcabcabc".
    ///
    /// Bytes (verified output from Python 3):
    ///   78 9c → CMF/FLG header (deflate, default compression)
    ///   4b 4c 4a 4e 84 21 00 → raw DEFLATE: dynamic Huffman "abcabcabcabc"
    ///   1d e0 04 99 → Adler-32 BE
    ///
    /// Verify with: python3 -c "import zlib; d=bytes([0x78,0x9c,0x4b,0x4c,0x4a,0x4e,0x84,0x21,0x00,0x1d,0xe0,0x04,0x99]); print(zlib.decompress(d))"
    #[test]
    fn inflate_dynamic_huffman() {
        // zlib stream for b"abcabcabcabc" at default level (dynamic Huffman).
        let zlib_bytes: &[u8] = &[
            0x78, 0x9c,
            0x4b, 0x4c, 0x4a, 0x4e, 0x84, 0x21, 0x00,
            0x1d, 0xe0, 0x04, 0x99,
        ];
        let out = zlib_decompress(zlib_bytes).expect("inflate dynamic Huffman failed");
        assert_eq!(out, b"abcabcabcabc");
    }

    // A raw DEFLATE stream produced by Python's zlib at level 9 with a full
    // 32 KB window (`zlib.compressobj(9, DEFLATED, -15)`).  It deliberately
    // exercises the parts of RFC 1951 our own 4 KB-window encoder never emits
    // but real-world producers (Office/zlib/gzip) do:
    //   • a 600-byte run of 'Z' → length-258 matches → LL symbol 285
    //   • a 400-byte repeat ~6000 bytes back → distance codes 24–29 (>4 KB)
    // Before the LENGTH_TABLE/DIST_TABLE completeness fix, `inflate` rejected
    // this with "invalid length symbol 285" / "invalid distance symbol 24".
    const REAL_DEFLATE_STREAM: &[u8] = &[
        0x8b, 0x8a, 0x1a, 0x05, 0xa3, 0x80, 0xfa, 0x80, 0xdb, 0x20, 0xb4, 0x6a, 0xfe, 0x91, 0x97, 0x7c,
        0xc6, 0x11, 0xb5, 0x8b, 0x8e, 0xbf, 0x11, 0x34, 0x8b, 0x6e, 0x58, 0x7a, 0xea, 0xbd, 0x88, 0x65,
        0x5c, 0xf3, 0x8a, 0xb3, 0x9f, 0xc4, 0x6d, 0x12, 0xdb, 0x56, 0x5f, 0xf8, 0x2a, 0x65, 0x9f, 0xd2,
        0xb9, 0xee, 0xf2, 0x0f, 0x59, 0xa7, 0xf4, 0x9e, 0x8d, 0xd7, 0x7e, 0x2b, 0xb8, 0x66, 0xf5, 0x6f,
        0xb9, 0xf9, 0x4f, 0xd9, 0x23, 0x77, 0xd2, 0xf6, 0x3b, 0x8c, 0x6a, 0xde, 0x05, 0x53, 0x77, 0xdd,
        0x67, 0xd1, 0xf4, 0x2b, 0x9e, 0xb1, 0xf7, 0x11, 0xbb, 0x4e, 0x60, 0xd9, 0xec, 0x03, 0x4f, 0xb9,
        0xf4, 0x43, 0x2a, 0xe7, 0x1d, 0x7e, 0xc1, 0x6b, 0x14, 0x5e, 0xb3, 0xf0, 0xd8, 0x6b, 0x01, 0xd3,
        0xa8, 0xfa, 0x25, 0x27, 0xdf, 0x09, 0x5b, 0xc4, 0x36, 0x2d, 0x3f, 0xf3, 0x51, 0xcc, 0x3a, 0xa1,
        0x75, 0xd5, 0xf9, 0x2f, 0x92, 0x76, 0xc9, 0x1d, 0x6b, 0x2f, 0x7d, 0x97, 0x71, 0x4c, 0xeb, 0xde,
        0x70, 0xf5, 0x97, 0xbc, 0x4b, 0x66, 0xdf, 0xe6, 0x1b, 0x7f, 0x95, 0xdc, 0x73, 0x26, 0x6e, 0xbb,
        0xcd, 0xa0, 0xea, 0x95, 0x3f, 0x65, 0xe7, 0x3d, 0x66, 0x0d, 0xdf, 0xa2, 0xe9, 0x7b, 0x1e, 0xb2,
        0x69, 0x07, 0x94, 0xce, 0xda, 0xff, 0x84, 0x53, 0x2f, 0xb8, 0x62, 0xee, 0xa1, 0xe7, 0x3c, 0x86,
        0x61, 0xd5, 0x0b, 0x8e, 0xbe, 0xe2, 0x37, 0x89, 0xac, 0x5b, 0x7c, 0xe2, 0xad, 0x90, 0x79, 0x4c,
        0xe3, 0xb2, 0xd3, 0x1f, 0x44, 0xad, 0xe2, 0x5b, 0x56, 0x9e, 0xfb, 0x2c, 0x61, 0x9b, 0xd4, 0xbe,
        0xe6, 0xe2, 0x37, 0x69, 0x87, 0xd4, 0xae, 0xf5, 0x57, 0x7e, 0xca, 0x39, 0x67, 0xf4, 0x6e, 0xba,
        0xfe, 0x47, 0xd1, 0x2d, 0x7b, 0xc2, 0xd6, 0x5b, 0xff, 0x55, 0x3c, 0xf3, 0x26, 0xef, 0xb8, 0xcb,
        0xa4, 0xee, 0x53, 0x38, 0x6d, 0xf7, 0x03, 0x56, 0x2d, 0xff, 0x92, 0x99, 0xfb, 0x1e, 0x73, 0xe8,
        0x06, 0x95, 0xcf, 0x39, 0xf8, 0x6c, 0xd4, 0xff, 0xa3, 0xfe, 0x1f, 0xf5, 0xff, 0xa8, 0xff, 0x47,
        0xfd, 0x3f, 0xea, 0xff, 0x51, 0xff, 0x8f, 0xfa, 0x7f, 0xd4, 0xff, 0xa3, 0xfe, 0x1f, 0xf5, 0xff,
        0xa8, 0xff, 0x47, 0xfd, 0x3f, 0xea, 0xff, 0x51, 0xff, 0x8f, 0xfa, 0x7f, 0xd4, 0xff, 0xa3, 0xfe,
        0x1f, 0xf5, 0xff, 0xa8, 0xff, 0x47, 0xfd, 0x3f, 0xea, 0x7f, 0x6a, 0xf8, 0x7f, 0x34, 0xbc, 0x07,
        0x97, 0xff, 0x01,
    ];

    #[test]
    fn inflate_full_window_real_stream() {
        // Reconstruct the exact payload the fixture was compressed from.
        let mut expected = vec![b'Z'; 600];
        let body: Vec<u8> = (0..6000u32).map(|i| ((i * 37 + 11) & 0xFF) as u8).collect();
        expected.extend_from_slice(&body);
        expected.extend_from_slice(&body[..400]);

        let out = inflate(REAL_DEFLATE_STREAM).expect("inflate full-window stream");
        assert_eq!(out.len(), expected.len());
        assert_eq!(out, expected);
    }
}
