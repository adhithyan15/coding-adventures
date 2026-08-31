//! Zstandard (ZStd) lossless compression algorithm — CMP07.
//!
//! Zstandard (RFC 8878) is a high-ratio, fast compression format created by
//! Yann Collet at Facebook (2015). It combines:
//!
//! - **LZ77 back-references** (via LZSS token generation) to exploit
//!   repetition in the data — the same "copy from earlier in the output"
//!   trick as DEFLATE, but with a 32 KB window.
//! - **FSE (Finite State Entropy)** coding instead of Huffman for the
//!   sequence descriptor symbols. FSE is an asymmetric numeral system that
//!   approaches the Shannon entropy limit in a single pass.
//! - **Predefined decode tables** (RFC 8878 Appendix B) so short frames
//!   need no table description overhead.
//!
//! # The encoder and decoder cover deliberately different ground
//!
//! The **encoder** here emits one narrow, readable subset of the format:
//! Raw literals, predefined FSE tables, explicit offsets. That is an
//! educational simplification, and its output is still a valid `.zst` frame
//! any conforming decoder reads.
//!
//! The **decoder** covers the format as real encoders actually use it:
//!
//! - **Huffman-coded literals** (§4.2.1) — `Compressed_Literals_Block` and
//!   `Treeless_Literals_Block`, single-stream and 4-stream, with tree
//!   descriptions in both the direct-weight and FSE-coded-weight forms.
//! - **FSE table descriptions** (§4.1.1) and all four
//!   `Symbol_Compression_Mode`s — Predefined, RLE, FSE_Compressed, Repeat.
//! - **Repeated offsets** (R1/R2/R3, §3.1.1.3.2.1.1).
//! - **RLE literal blocks** (§3.1.1.2).
//!
//! None of that is reachable from this crate's own `compress()`, which is
//! exactly why it needs its own testing strategy. A codec whose two halves
//! only ever talk to each other is blind to everything they get wrong (or
//! omit) in the same way — this crate shipped four separate conformance
//! bugs behind that illusion; see lessons.md Lessons 95, 96 and 98. So the
//! decoder is tested against frames it cannot produce: live `zstd` CLI
//! interop, plus committed golden vectors for machines without the binary.
//!
//! # Frame layout (RFC 8878 §3)
//!
//! ```text
//! ┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
//! │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
//! │ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
//! └────────┴─────┴──────────────────────┴────────┴──────────────────┘
//! ```
//!
//! Each **block** has a 3-byte header:
//! ```text
//! bit 0      = Last_Block flag
//! bits [2:1] = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
//! bits [23:3] = Block_Size
//! ```
//!
//! # Compression strategy (this implementation)
//!
//! 1. Split data into 128 KB blocks (MAX_BLOCK_SIZE).
//! 2. For each block, try:
//!    a. **RLE** — all bytes identical → 5 bytes total.
//!    b. **Compressed** (LZ77 + FSE) — if output < input length.
//!    c. **Raw** — verbatim copy as fallback.
//!
//! # Series
//!
//! ```text
//! CMP00 (LZ77)     — Sliding-window back-references
//! CMP01 (LZ78)     — Explicit dictionary (trie)
//! CMP02 (LZSS)     — LZ77 + flag bits
//! CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
//! CMP04 (Huffman)  — Entropy coding
//! CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
//! CMP06 (Brotli)   — DEFLATE + context modelling + static dict
//! CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed ← this crate
//! ```
//!
//! # Examples
//!
//! ```
//! use zstd::{compress, decompress};
//!
//! let data = b"the quick brown fox jumps over the lazy dog";
//! let compressed = compress(data);
//! assert_eq!(decompress(&compressed).unwrap(), data);
//! ```

// ─── Constants ────────────────────────────────────────────────────────────────

/// ZStd magic number: `0xFD2FB528` (little-endian: `28 B5 2F FD`).
///
/// Every valid ZStd frame starts with these 4 bytes. The value was chosen to
/// be unlikely to appear at the start of plaintext files.
const MAGIC: u32 = 0xFD2FB528;

/// Maximum block size: 128 KB.
///
/// ZStd allows blocks up to 128 KB. Larger inputs are split across multiple
/// blocks. The spec maximum is actually `min(WindowSize, 128 KB)`.
const MAX_BLOCK_SIZE: usize = 128 * 1024;

/// Decompression-bomb guard: maximum total decompressed output size (256 MB).
///
/// A Compressed block's WIRE size is capped at [`MAX_BLOCK_SIZE`] (128 KB),
/// but that says nothing about how large the block can EXPAND to: a single
/// FSE-coded sequence's match length can be up to ~131 KB (ML code 52), and
/// one 128 KB block can carry tens of thousands of sequences. This limit
/// must therefore be checked incrementally, at every point output can grow
/// — inside the per-sequence loop of [`decompress_block`], not only once per
/// top-level block (Raw/RLE) as an earlier revision of this decoder did.
const MAX_OUTPUT: usize = 256 * 1024 * 1024;

/// Returns an error if adding `additional` more bytes to output (currently
/// `current_size` bytes) would exceed [`MAX_OUTPUT`].
///
/// `additional` is always derived from bounded wire fields (`ll`/`ml`
/// values, themselves at most ~131070 per RFC 8878's LL/ML code tables), so
/// `current_size + additional` cannot overflow a `usize` before this check
/// fires (`MAX_OUTPUT` itself is 2^28).
fn check_output_budget(current_size: usize, additional: usize) -> Result<(), String> {
    if current_size.saturating_add(additional) > MAX_OUTPUT {
        return Err(format!("decompressed size exceeds limit of {MAX_OUTPUT} bytes"));
    }
    Ok(())
}

// ─── LL / ML / OF code tables (RFC 8878 §3.1.1.3) ────────────────────────────
//
// These tables map a *code number* to a (baseline, extra_bits) pair.
//
// For example, LL code 17 means literal_length = 18 + read(1 extra bit),
// so it covers literal lengths 18 and 19.
//
// The FSE state machine tracks one code number per field; extra bits are
// read directly from the bitstream after state transitions.

/// Literal Length code table: `(baseline, extra_bits)` for codes 0..=35.
///
/// Literal length 0..15 each have their own code (0 extra bits).
/// Larger lengths are grouped with increasing ranges.
const LL_CODES: [(u32, u8); 36] = [
    // code: value = baseline + read(extra_bits)
    (0, 0),  (1, 0),  (2, 0),  (3, 0),  (4, 0),  (5, 0),
    (6, 0),  (7, 0),  (8, 0),  (9, 0),  (10, 0), (11, 0),
    (12, 0), (13, 0), (14, 0), (15, 0),
    // Grouped ranges start at code 16
    (16, 1), (18, 1), (20, 1), (22, 1),
    (24, 2), (28, 2),
    (32, 3), (40, 3),
    (48, 4), (64, 6),
    (128, 7), (256, 8), (512, 9), (1024, 10), (2048, 11), (4096, 12),
    (8192, 13), (16384, 14), (32768, 15), (65536, 16),
];

/// Match Length code table: `(baseline, extra_bits)` for codes 0..=52.
///
/// Minimum match length in ZStd is 3 (not 0). Code 0 = match length 3.
const ML_CODES: [(u32, u8); 53] = [
    // codes 0..31: individual values 3..34
    (3, 0),  (4, 0),  (5, 0),  (6, 0),  (7, 0),  (8, 0),
    (9, 0),  (10, 0), (11, 0), (12, 0), (13, 0), (14, 0),
    (15, 0), (16, 0), (17, 0), (18, 0), (19, 0), (20, 0),
    (21, 0), (22, 0), (23, 0), (24, 0), (25, 0), (26, 0),
    (27, 0), (28, 0), (29, 0), (30, 0), (31, 0), (32, 0),
    (33, 0), (34, 0),
    // codes 32+: grouped ranges
    (35, 1), (37, 1),  (39, 1),  (41, 1),
    (43, 2), (47, 2),
    (51, 3), (59, 3),
    (67, 4), (83, 4),
    (99, 5), (131, 7),
    (259, 8), (515, 9), (1027, 10), (2051, 11),
    (4099, 12), (8195, 13), (16387, 14), (32771, 15), (65539, 16),
];

// ─── FSE predefined distributions (RFC 8878 Appendix B) ──────────────────────
//
// "Predefined_Mode" means no per-frame table description is transmitted.
// The decoder builds the same table from these fixed distributions.
//
// Entries of -1 mean "probability 1/table_size" — these symbols get one slot
// in the decode table and their encoder state never needs extra bits.

/// Predefined normalised distribution for Literal Length FSE.
/// Table accuracy log = 6 → 64 slots.
const LL_NORM: [i16; 36] = [
     4,  3,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  1,  1,  1,
     2,  2,  2,  2,  2,  2,  2,  2,  2,  3,  2,  1,  1,  1,  1,  1,
    -1, -1, -1, -1,
];
const LL_ACC_LOG: u8 = 6; // table_size = 64

/// Predefined normalised distribution for Match Length FSE.
/// Table accuracy log = 6 → 64 slots.
const ML_NORM: [i16; 53] = [
     1,  4,  3,  2,  2,  2,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1, -1, -1,
    -1, -1, -1, -1, -1,
];
const ML_ACC_LOG: u8 = 6;

/// Predefined normalised distribution for Offset FSE.
/// Table accuracy log = 5 → 32 slots.
const OF_NORM: [i16; 29] = [
     1,  1,  1,  1,  1,  1,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1, -1, -1, -1, -1, -1,
];
const OF_ACC_LOG: u8 = 5; // table_size = 32

// ─── FSE decode table entry ───────────────────────────────────────────────────

/// One cell in the FSE decode table.
///
/// To decode a symbol from state S:
///   1. `sym` is the output symbol.
///   2. Read `nb` bits from the bitstream as `bits`.
///   3. New state = `base + bits`.
#[derive(Clone, Copy, Default, Debug)]
struct FseDe {
    sym: u8,  // decoded symbol
    nb: u8,   // number of extra bits to read for next state
    base: u16, // base value for next state computation
}

/// Build an FSE decode table from a normalised probability distribution.
///
/// The algorithm:
///  1. Place symbols with probability -1 (very rare) at the top of the table.
///  2. Spread remaining symbols using a deterministic step function derived
///     from the table size. This ensures each symbol occupies the correct
///     fraction of slots.
///  3. Assign `nb` (number of state bits) and `base` to each slot so that
///     the decoder can reconstruct the next state.
///
/// The step function `step = (sz >> 1) + (sz >> 3) + 3` is co-prime to `sz`
/// when `sz` is a power of two (which it always is in ZStd), ensuring that
/// the walk visits every slot exactly once.
fn build_decode_table(norm: &[i16], acc_log: u8) -> Vec<FseDe> {
    let sz = 1usize << acc_log;
    let step = (sz >> 1) + (sz >> 3) + 3;
    let mut tbl = vec![FseDe::default(); sz];
    let mut sym_next = vec![0u16; norm.len()];

    // Phase 1: symbols with probability -1 go at the top (high indices).
    // These symbols each get exactly 1 slot, and their state transition uses
    // the full acc_log bits (they can go to any state).
    let mut high = sz - 1;
    for (s, &c) in norm.iter().enumerate() {
        if c == -1 {
            tbl[high].sym = s as u8;
            high = high.saturating_sub(1);
            sym_next[s] = 1;
        }
    }

    // Phase 2: spread remaining symbols into the lower portion of the table.
    //
    // A SINGLE pass over symbols in ascending order `0..norm.len()`, placing
    // each symbol's full count immediately when encountered — this is the
    // real algorithm (`FSE_buildDTable_internal`'s low-probability branch,
    // verified against the reference C source at
    // `github.com/facebook/zstd/lib/decompress/fse_decompress.c`).
    //
    // An earlier revision of this codec used a fabricated TWO-PASS split
    // (all symbols with count > 1 first, in ascending order, then all
    // symbols with count == 1) — a plausible-looking but entirely invented
    // convention with no basis in the reference algorithm. It produced a
    // completely different (but internally self-consistent) table layout:
    // our own decoder mirrored our own encoder, so every round-trip test
    // against OURSELVES passed, yet the real `zstd` CLI rejected the output
    // as corrupt. See lessons.md Lesson 96.
    let mut pos = 0usize;
    for (s, &c) in norm.iter().enumerate() {
        if c <= 0 {
            continue;
        }
        let cnt = c as usize;
        sym_next[s] = cnt as u16;
        for _ in 0..cnt {
            tbl[pos].sym = s as u8;
            pos = (pos + step) & (sz - 1);
            while pos > high {
                pos = (pos + step) & (sz - 1);
            }
        }
    }

    // Phase 3: assign nb (number of state bits to read) and base.
    //
    // For a symbol with count `cnt` occupying slots i₀, i₁, ...:
    //   The next_state counter starts at `cnt` and increments.
    //   nb = acc_log - floor(log2(next_state))
    //   base = next_state * (1 << nb) - sz
    //
    // This ensures that when we reconstruct state = base + read(nb bits),
    // we land in the range [sz, 2*sz), which is the valid encoder state range.
    let mut sn = sym_next.clone();
    for entry in tbl.iter_mut().take(sz) {
        let s = entry.sym as usize;
        let ns = sn[s] as u32;
        sn[s] += 1;
        debug_assert!(ns > 0, "FSE: sym_next must be positive");
        // floor(log2(ns)) = 31 - leading_zeros(ns)
        let nb = acc_log - (31 - ns.leading_zeros()) as u8;
        // base = ns * (1 << nb) - sz
        let base = ((ns << nb) as usize).wrapping_sub(sz) as u16;
        entry.nb = nb;
        entry.base = base;
    }

    tbl
}

// ─── Forward bit-reader (table descriptions) ─────────────────────────────────
//
// ZStd contains TWO bitstream conventions, and mixing them up is a classic
// source of "decodes our own output, rejects everyone else's" bugs:
//
//   * The *payload* bitstreams (sequences, Huffman literals, Huffman weights)
//     are written BACKWARD and read from the end — that's `RevBitReader`.
//   * The *table description* bitstreams (RFC 8878 §4.1.1, the FSE
//     distribution header) are written FORWARD, low bit of byte 0 first,
//     bytes in little-endian order — that's this reader.
//
// Concretely, if the first two description bytes are `0xA7 0x03`, the value
// as a little-endian integer is `0x03A7 = 0b11_1010_0111`, and successive
// reads peel bits off the BOTTOM: `read(4)` yields `0b0111 = 7`,
// then `read(4)` yields `0b1010 = 10`, then `read(2)` yields `0b11 = 3`.
//
// The reader deliberately treats bytes past the end of `data` as zero rather
// than failing on the spot. The reference implementation does the same (it
// reads a fixed 4-byte window and only validates afterwards), because the
// header parser legitimately *peeks* more bits than the final symbol needs
// before deciding how many to consume. Over-reads are caught once, at the
// end, by [`FwdBitReader::finish`].
struct FwdBitReader<'a> {
    data: &'a [u8],
    /// Bit cursor: bit `n` of the stream is bit `n % 8` of byte `n / 8`.
    bitpos: usize,
}

impl<'a> FwdBitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        FwdBitReader { data, bitpos: 0 }
    }

    /// Read the next `nb` bits (`nb <= 25`) without advancing the cursor.
    ///
    /// Gathers 5 bytes so that even a 7-bit misalignment plus a 25-bit field
    /// (32 bits total) is covered by the assembled 40-bit window.
    fn peek(&self, nb: u32) -> u32 {
        debug_assert!(nb <= 25, "FwdBitReader::peek supports at most 25 bits");
        if nb == 0 {
            return 0;
        }
        let byte = self.bitpos >> 3;
        let shift = (self.bitpos & 7) as u32;
        let mut window: u64 = 0;
        for i in 0..5usize {
            let b = self.data.get(byte + i).copied().unwrap_or(0) as u64;
            window |= b << (8 * i);
        }
        ((window >> shift) & ((1u64 << nb) - 1)) as u32
    }

    /// Advance the cursor by `nb` bits.
    fn skip(&mut self, nb: u32) {
        self.bitpos += nb as usize;
    }

    /// Peek-and-advance in one step.
    fn read(&mut self, nb: u32) -> u32 {
        let v = self.peek(nb);
        self.skip(nb);
        v
    }

    /// Finish parsing: return how many whole bytes the description occupied,
    /// or an error if the parse ran past the end of the buffer.
    ///
    /// Rounding UP is correct and required: a description that ends
    /// mid-byte still owns that whole byte on the wire — the next field
    /// starts at the following byte boundary.
    fn finish(self) -> Result<usize, String> {
        let bytes = self.bitpos.div_ceil(8);
        if bytes > self.data.len() {
            return Err(format!(
                "FSE table description overruns its buffer (needs {bytes} bytes, have {})",
                self.data.len()
            ));
        }
        Ok(bytes)
    }
}

// ─── FSE table description (RFC 8878 §4.1.1) ─────────────────────────────────

/// Absolute maximum accuracy_log this decoder will build a table for.
///
/// RFC 8878 caps the sequence tables individually (LL 9, ML 9, OF 8) and the
/// Huffman-weight table at 6; this is the ceiling across all of them, used to
/// bound the table allocation an attacker-controlled `accuracy_log` can
/// request. The wire field is 4 bits plus the constant 5, so the raw range is
/// 5..=20 — a 20-bit accuracy_log would mean a 1M-entry table built from a
/// handful of bytes, which is exactly the kind of amplification a malformed
/// `.apkg` would use.
const MAX_ACC_LOG: u8 = 9;

/// Decode an FSE table description into a normalised distribution.
///
/// Returns `(normalised_counts, accuracy_log, bytes_consumed)`.
///
/// # The encoding (and why it is shaped this way)
///
/// The description transmits one count per symbol, in symbol order, such that
/// the counts sum to `2^accuracy_log`. The clever part is that it spends
/// *fewer bits per count as it goes*: once the decoder has read some counts it
/// knows how much probability mass is still unassigned, so the remaining
/// counts cannot be arbitrarily large, so they need fewer bits. Both sides
/// derive the field width from state they already share — nothing about the
/// width is transmitted.
///
/// Mechanically, with `remaining` = mass still to assign (starting at
/// `2^accuracy_log + 1`) and `threshold` = the largest power of two `<=
/// remaining`:
///
/// - A count is drawn from `0 ..= remaining`, which needs `log2(threshold)+1`
///   bits in general — but only the `max = (2*threshold-1) - remaining`
///   smallest values need the full width. So the decoder first peeks
///   `nbits-1` bits: if that short value is `< max` it IS the value, and only
///   `nbits-1` bits are consumed. Otherwise the full `nbits` are consumed and
///   `max` is subtracted back off if the value landed in the upper half.
///   This is a self-synchronising variable-width integer, saving roughly one
///   bit per symbol.
/// - The transmitted value is one MORE than the count, so that value `0` can
///   mean "probability less than one" — the RFC's `-1` count, a symbol that
///   gets exactly one table slot.
/// - A count of exactly `0` (symbol absent) is followed by a 2-bit repeat
///   field, because absent symbols cluster: `0b11` means "three more absent
///   symbols, and another repeat field follows", any smaller value means
///   "that many more absent symbols, and we're done repeating". Encoding a
///   run of 30 unused symbols therefore costs ~20 bits instead of ~180.
///
/// # Safety of the loop bound
///
/// Every iteration either advances `charnum` (bounded by `max_symbol`) or
/// reduces `remaining` (bounded below by the `remaining <= 1` break), and the
/// repeat-run loop advances `charnum` by 3 each time it spins. A malformed
/// description therefore always terminates with an `Err`, never a hang.
fn read_fse_table_description(
    data: &[u8],
    max_acc_log: u8,
    max_symbol: usize,
) -> Result<(Vec<i16>, u8, usize), String> {
    debug_assert!(max_acc_log <= MAX_ACC_LOG);
    debug_assert!(max_symbol < 256);

    if data.is_empty() {
        return Err("empty FSE table description".into());
    }

    let mut br = FwdBitReader::new(data);

    // accuracy_log is stored biased by 5 (the RFC's FSE_MIN_TABLELOG).
    let acc_log = br.read(4) as u8 + 5;
    if acc_log > max_acc_log {
        return Err(format!(
            "FSE accuracy_log {acc_log} exceeds maximum {max_acc_log}"
        ));
    }

    let table_size = 1i32 << acc_log;
    let mut remaining = table_size + 1;
    let mut threshold = table_size;
    let mut nbits = acc_log as u32 + 1;

    let mut norm: Vec<i16> = Vec::with_capacity(max_symbol + 1);
    let mut previous0 = false;

    while remaining > 1 && norm.len() <= max_symbol {
        if previous0 {
            // Run of absent symbols, coded as 2-bit chunks; 0b11 = "three
            // more, and keep reading".
            loop {
                let flag = br.read(2) as usize;
                let run = flag.min(3);
                if norm.len() + run > max_symbol + 1 {
                    return Err(format!(
                        "FSE zero-run overruns symbol space (max symbol {max_symbol})"
                    ));
                }
                norm.resize(norm.len() + run, 0);
                if flag < 3 {
                    break;
                }
            }
            if norm.len() > max_symbol {
                break;
            }
            // `previous0` is re-derived from the count read just below, so
            // there is nothing to clear here.
        }

        // Variable-width count field — see the doc comment above.
        let max = (2 * threshold - 1) - remaining;
        let short = br.peek(nbits - 1) as i32;
        let value = if short < max {
            br.skip(nbits - 1);
            short
        } else {
            let mut v = br.peek(nbits) as i32;
            br.skip(nbits);
            if v >= threshold {
                v -= max;
            }
            v
        };

        // Transmitted value is count+1, so that 0 encodes the "-1" count.
        let count = value - 1;
        remaining -= count.unsigned_abs() as i32;
        norm.push(count as i16);
        previous0 = count == 0;

        // Shrink the field width to match the mass that is still unassigned.
        if remaining < threshold {
            if remaining <= 1 {
                break;
            }
            // nbits = floor(log2(remaining)) + 1
            nbits = 32 - (remaining as u32).leading_zeros();
            threshold = 1 << (nbits - 1);
        }
    }

    if remaining != 1 {
        return Err(format!(
            "FSE table description probabilities sum to the wrong total \
             (residual {remaining}, expected 1)"
        ));
    }
    if norm.is_empty() {
        return Err("FSE table description contains no symbols".into());
    }
    if norm.len() > max_symbol + 1 {
        return Err(format!(
            "FSE table description has {} symbols, maximum is {}",
            norm.len(),
            max_symbol + 1
        ));
    }

    let consumed = br.finish()?;
    Ok((norm, acc_log, consumed))
}

// ─── FSE decode table (owned) ────────────────────────────────────────────────

/// A built FSE decode table plus the accuracy_log it was built at.
///
/// The accuracy_log has to travel WITH the table, because it is what the
/// sequences bitstream reads to prime the initial state — and with
/// `FSE_Compressed_Mode` and `Repeat_Mode` in play it is no longer the fixed
/// `LL_ACC_LOG` / `ML_ACC_LOG` / `OF_ACC_LOG` constants but a per-block value
/// (or a value inherited from an earlier block).
#[derive(Clone)]
struct FseTable {
    de: Vec<FseDe>,
    acc_log: u8,
}

impl FseTable {
    /// Build a table from a normalised distribution, validating it first.
    ///
    /// The validation is not optional politeness: [`build_decode_table`]
    /// assumes the counts sum to exactly `2^acc_log`, and silently produces a
    /// table with duplicate/unwritten cells when they don't. Reading such a
    /// table can then compute an out-of-range next state, so the check
    /// belongs here, at the one place attacker-controlled distributions
    /// enter.
    fn from_norm(norm: &[i16], acc_log: u8) -> Result<Self, String> {
        if acc_log > MAX_ACC_LOG {
            return Err(format!("FSE accuracy_log {acc_log} exceeds {MAX_ACC_LOG}"));
        }
        if norm.is_empty() || norm.len() > 256 {
            return Err(format!("FSE distribution has {} symbols", norm.len()));
        }
        let table_size = 1usize << acc_log;
        let mut total = 0usize;
        for (s, &c) in norm.iter().enumerate() {
            if c < -1 {
                return Err(format!("FSE symbol {s} has invalid count {c}"));
            }
            total += if c == -1 { 1 } else { c as usize };
            if total > table_size {
                return Err(format!(
                    "FSE distribution sums past table size {table_size} by symbol {s}"
                ));
            }
        }
        if total != table_size {
            return Err(format!(
                "FSE distribution sums to {total}, expected table size {table_size}"
            ));
        }
        Ok(FseTable { de: build_decode_table(norm, acc_log), acc_log })
    }

    /// The degenerate one-state table used by `RLE_Mode`: every state decodes
    /// the same symbol and consumes no bits, so the whole stream costs zero
    /// bits for this field. `accuracy_log` is 0, meaning the initial-state
    /// read is also zero bits wide.
    fn rle(sym: u8) -> Self {
        FseTable { de: vec![FseDe { sym, nb: 0, base: 0 }], acc_log: 0 }
    }
}

// ─── FSE encode symbol table entry ───────────────────────────────────────────

/// Encode transform for one symbol.
///
/// Given encoder state S for symbol `s`:
///   nb_out = (S + delta_nb) >> 16   (number of bits to emit)
///   emit low nb_out bits of S
///   new_S  = state_tbl[(S >> nb_out) + delta_fs]
///
/// The `delta_nb` and `delta_fs` values are precomputed from the distribution
/// so the hot-path encode loop needs only arithmetic and a table lookup.
#[derive(Clone, Copy, Default)]
struct FseEe {
    /// `(max_bits_out << 16) - (count << max_bits_out)`
    /// Used to derive nb_out: `nb_out = (state + delta_nb) >> 16`
    delta_nb: u32,
    /// `cumulative_count_before_sym - count`  (may be negative, hence i32)
    /// Used to index state_tbl: `new_S = state_tbl[(S >> nb_out) + delta_fs]`
    delta_fs: i32,
}

/// Build FSE encode tables from a normalised distribution.
///
/// Returns:
/// - `ee[sym]`: the FseEe transform for each symbol.
/// - `st[slot]`: the encoder state table (slot → output state in [sz, 2*sz)).
///
/// # The encode/decode symmetry
///
/// The FSE decoder assigns `(sym, nb, base)` to each table cell in INDEX ORDER.
/// For symbol `s`, the j-th cell (in ascending index order) has:
///   ns = count[s] + j
///   nb = acc_log - floor(log2(ns))
///   base = ns * (1<<nb) - sz
///
/// The FSE encoder must use the SAME indexing: slot `cumul[s]+j` maps to the
/// j-th table cell for symbol `s` (in ascending index order).
///
/// The encoder state after encoding sym `s` from slot `cumul[s]+j` is
/// `(j-th cell index for s) + sz`. The decoder at that cell index will read
/// the same bits and reconstruct the encoder's pre-encoding state.
fn build_encode_sym(norm: &[i16], acc_log: u8) -> (Vec<FseEe>, Vec<u16>) {
    let sz = 1u32 << acc_log;

    // Step 1: compute cumulative sums.
    let mut cumul = vec![0u32; norm.len()];
    let mut total = 0u32;
    for (s, &c) in norm.iter().enumerate() {
        cumul[s] = total;
        let cnt = if c == -1 { 1u32 } else { c.max(0) as u32 };
        total += cnt;
    }

    // Step 2: build the spread table (which symbol occupies each table slot).
    //
    // This uses the same spreading algorithm as build_decode_table, producing
    // a mapping from table index to symbol.
    let step = (sz >> 1) + (sz >> 3) + 3;
    let mut spread = vec![0u8; sz as usize]; // spread[index] = symbol
    let mut idx_high = sz as usize - 1;

    // Phase 1: probability -1 symbols at the high end
    for (s, &c) in norm.iter().enumerate() {
        if c == -1 {
            spread[idx_high] = s as u8;
            idx_high = idx_high.saturating_sub(1);
        }
    }
    let idx_limit = idx_high; // highest free slot

    // Phase 2: spread remaining symbols using the step function.
    //
    // A SINGLE pass over symbols in ascending order — MUST mirror
    // `build_decode_table`'s Phase 2 exactly (same reasoning: the real
    // algorithm has no count>1-vs-count==1 split; see Lesson 96).
    let mut pos = 0usize;
    for (s, &c) in norm.iter().enumerate() {
        if c <= 0 { continue; }
        let cnt = c as usize;
        for _ in 0..cnt {
            spread[pos] = s as u8;
            pos = (pos + step as usize) & (sz as usize - 1);
            while pos > idx_limit { pos = (pos + step as usize) & (sz as usize - 1); }
        }
    }

    // Step 3: build the state table by iterating spread in INDEX ORDER.
    //
    // For each table index `i` (in ascending order), determine which
    // occurrence of symbol `s = spread[i]` this is (j = 0, 1, 2, ...).
    // The encode slot is `cumul[s] + j`, and the encoder output state is
    // `i + sz` (so the decoder, in state `i`, will decode symbol `s`).
    //
    // We use `sym_occ[s]` to count how many times symbol `s` has appeared
    // so far (in index order), so j = sym_occ[s] when we see it at index i.
    let mut sym_occ = vec![0u32; norm.len()];
    let mut st = vec![0u16; sz as usize];

    for (i, &sp) in spread.iter().enumerate().take(sz as usize) {
        let s = sp as usize;
        let j = sym_occ[s] as usize;
        sym_occ[s] += 1;
        // Slot for this (sym, occurrence) pair
        let slot = cumul[s] as usize + j;
        // Encoder output state = decode table index + sz
        st[slot] = (i as u32 + sz) as u16;
    }

    // Step 4: build FseEe entries.
    //
    // For symbol s with count c and max_bits_out mbo:
    //   delta_nb = (mbo << 16) - (c << mbo)
    //   delta_fs = cumul[s] - c
    //
    // Encode step: given current encoder state E ∈ [sz, 2*sz):
    //   nb = (E + delta_nb) >> 16     (number of state bits to emit)
    //   emit low nb bits of E
    //   new_E = st[(E >> nb) + delta_fs]
    let mut ee = vec![FseEe::default(); norm.len()];
    for (s, &c) in norm.iter().enumerate() {
        let cnt = if c == -1 { 1u32 } else { c.max(0) as u32 };
        if cnt == 0 { continue; }
        let mbo = if cnt == 1 {
            acc_log as u32
        } else {
            // max_bits_out = acc_log - floor(log2(cnt))
            acc_log as u32 - (31 - cnt.leading_zeros())
        };
        ee[s].delta_nb = (mbo << 16).wrapping_sub(cnt << mbo);
        ee[s].delta_fs = cumul[s] as i32 - cnt as i32;
    }

    (ee, st)
}

// ─── Reverse bit-writer ───────────────────────────────────────────────────────
//
// ZStd's sequence bitstream is written *backwards* relative to the data flow:
// the encoder writes bits that the decoder will read last, first. This allows
// the decoder to read a forward-only stream while decoding sequences in order.
//
// Byte layout: `[byte0, byte1, ..., byteN]` where `byteN` is the last byte
// written, and it contains a **sentinel bit** (the highest set bit) that marks
// the end of meaningful data. The decoder initialises by finding this sentinel.
//
// Bit layout within each byte: LSB = first bit written.
//
// Example: write bits `1, 0, 1, 1` (4 bits) then flush:
//   reg = 0b1011, bits = 4
//   flush: sentinel at bit 4 → last byte = 0b0001_1011 = 0x1B
//   buf = [0x1B]
//
// The decoder reads this as: find MSB (bit 4 = sentinel), then read
// bits 3..0 = 0b1011 = the original 4 bits.

struct RevBitWriter {
    buf: Vec<u8>,
    reg: u64,  // accumulation register (bits fill from LSB)
    bits: u8,  // number of valid bits in reg
}

impl RevBitWriter {
    fn new() -> Self {
        Self { buf: Vec::new(), reg: 0, bits: 0 }
    }

    /// Add `nb` low-order bits of `val` to the stream.
    fn add_bits(&mut self, val: u64, nb: u8) {
        if nb == 0 {
            return;
        }
        let mask = if nb == 64 { u64::MAX } else { (1u64 << nb) - 1 };
        self.reg |= (val & mask) << self.bits;
        self.bits += nb;
        while self.bits >= 8 {
            self.buf.push(self.reg as u8);
            self.reg >>= 8;
            self.bits -= 8;
        }
    }

    /// Flush remaining bits with a sentinel and return the buffer.
    ///
    /// The sentinel is a `1` bit placed at position `self.bits` in the
    /// last byte. The decoder locates it with `leading_zeros` arithmetic.
    fn flush(&mut self) {
        let sentinel: u8 = 1 << self.bits; // bit above all remaining data bits
        let last_byte = (self.reg as u8) | sentinel;
        self.buf.push(last_byte);
        self.reg = 0;
        self.bits = 0;
    }

    fn finish(self) -> Vec<u8> {
        self.buf
    }
}

// ─── Reverse bit-reader ───────────────────────────────────────────────────────
//
// Mirrors RevBitWriter: reads bits from the END of the buffer going backwards.
// The stream is laid out so that the LAST bits written by the encoder are at the
// END of the byte buffer (in the sentinel-containing last byte). The reader
// initialises at the last byte and reads backward toward byte 0.
//
// Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side).
// `read_bits(n)` extracts the top n bits and shifts the register left by n.
//
// Why left-aligned? The writer accumulates bits LSB-first. Within each flushed
// byte, bit 0 = earliest written, bit N = latest written. To read the LATEST
// bits first (which were in the highest byte positions and in the high bits of
// each byte), we need a left-aligned register so that reading from the top
// gives the highest-position bits first.

struct RevBitReader<'a> {
    data: &'a [u8],
    reg: u64,   // shift register, valid bits packed at the TOP (MSB side)
    bits: u8,   // how many valid bits are loaded (count from MSB)
    pos: usize, // index of the next byte to load (decrements toward 0)
    /// How many payload bits are still UNREAD, as a *signed* count.
    ///
    /// This mirrors the `i64 offset` of RFC 8878's own reference "educational
    /// decoder": it starts at the total number of payload bits in the stream
    /// (everything below the sentinel), and every [`read_bits`] call
    /// subtracts the bits it consumed — even when the register had already
    /// run dry and those bits were zero-fill rather than real data.
    ///
    /// Going NEGATIVE is therefore the precise, checkable definition of "this
    /// stream is exhausted", which two RFC 8878 constructs need:
    ///
    /// - the **2-state interleaved FSE stream** that carries Huffman weights
    ///   (§4.2.1.1) has no symbol count on the wire at all; the decoder is
    ///   supposed to keep alternating between its two states until the
    ///   bitstream runs out, then emit one last symbol from the state whose
    ///   turn it was. Without a bit budget there is no way to know when to
    ///   stop, and a corrupt stream would spin forever.
    /// - every other bitstream here (Huffman literal streams, the sequences
    ///   bitstream) knows its symbol count up front, but a *conforming*
    ///   stream must end EXACTLY — `remaining == 0` after the last symbol.
    ///   The reference decoder enforces this via `BIT_endOfDStream`, and it
    ///   is a genuinely load-bearing corruption check: a truncated or
    ///   mis-parsed stream otherwise silently decodes zero-filled garbage.
    ///
    /// [`read_bits`]: RevBitReader::read_bits
    remaining: i64,
}

impl<'a> RevBitReader<'a> {
    fn new(data: &'a [u8]) -> Result<Self, String> {
        if data.is_empty() {
            return Err("empty bitstream".into());
        }

        // Find the sentinel bit in the last byte.
        // The sentinel is the highest set bit; valid data bits are below it.
        let last = *data.last().unwrap();
        if last == 0 {
            return Err("bitstream last byte is zero (no sentinel)".into());
        }

        // sentinel_pos = bit index (0 = LSB) of the sentinel in the last byte
        let sentinel_pos = 7 - last.leading_zeros() as u8;
        // valid_bits = number of data bits below the sentinel
        let valid_bits = sentinel_pos;

        // Place the valid bits of the sentinel byte at the TOP of the register.
        // Example: last=0b00011110, sentinel at bit4, valid_bits=4,
        //   data bits = last & 0b1111 = 0b1110.
        //   After shifting to top: reg bit63=1, bit62=1, bit61=1, bit60=0.
        let mask = if valid_bits == 0 { 0u64 } else { (1u64 << valid_bits) - 1 };
        let reg = if valid_bits == 0 {
            0u64
        } else {
            ((last as u64) & mask) << (64 - valid_bits)
        };

        let mut r = RevBitReader {
            data,
            reg,
            bits: valid_bits,
            pos: data.len() - 1, // sentinel byte already consumed; load from here-1
            // Total payload bits = every bit of every earlier byte, plus the
            // bits below the sentinel in the last byte. The sentinel itself
            // and the zero padding above it are NOT payload.
            remaining: (data.len() as i64 - 1) * 8 + valid_bits as i64,
        };

        // Fill the register from earlier bytes.
        r.reload();
        Ok(r)
    }

    /// Load more bytes into the register from the stream going backward.
    ///
    /// Each new byte is placed just BELOW the currently loaded bits (in the
    /// left-aligned register, that means at position `64 - bits - 8`).
    fn reload(&mut self) {
        while self.bits <= 56 && self.pos > 0 {
            self.pos -= 1;
            // Place this byte just below existing bits (MSB-aligned packing).
            // Current top `bits` bits are occupied; new byte goes just below.
            let shift = 64 - self.bits as u32 - 8;
            self.reg |= (self.data[self.pos] as u64) << shift;
            self.bits += 8;
        }
    }

    /// Read `nb` bits from the top of the register (returns 0 if nb == 0).
    ///
    /// This returns the most recently written bits first (highest stream
    /// positions first), mirroring the encoder's backward order.
    fn read_bits(&mut self, nb: u8) -> u64 {
        if nb == 0 {
            return 0;
        }
        // Extract the top `nb` bits.
        let val = self.reg >> (64 - nb);
        // Shift the register left to consume those bits.
        self.reg = if nb == 64 { 0 } else { self.reg << nb };
        self.bits = self.bits.saturating_sub(nb);
        // Charge the bit budget even when the register had already run dry:
        // an over-read is exactly what `remaining < 0` is meant to record.
        self.remaining -= nb as i64;
        if self.bits < 24 {
            self.reload();
        }
        val
    }

    /// Look at the next `nb` bits WITHOUT consuming them.
    ///
    /// Huffman decoding needs this: a canonical Huffman code is decoded by
    /// indexing a `2^max_bits`-entry table with the next `max_bits` bits and
    /// then consuming only as many bits as the matched code actually uses
    /// (RFC 8878 §4.2.1.3). Peeking more bits than remain is fine and
    /// deliberate — the register's unloaded low bits are zero, which is the
    /// same zero-fill the reference decoder performs past the stream start.
    fn peek_bits(&self, nb: u8) -> u64 {
        if nb == 0 {
            0
        } else {
            self.reg >> (64 - nb)
        }
    }

    /// Consume `nb` bits, discarding their value. Pairs with [`peek_bits`].
    ///
    /// [`peek_bits`]: RevBitReader::peek_bits
    fn skip_bits(&mut self, nb: u8) {
        let _ = self.read_bits(nb);
    }

    /// True once more bits have been requested than the stream actually
    /// contained. See the [`remaining`] field's doc comment.
    ///
    /// [`remaining`]: RevBitReader::remaining
    fn is_overrun(&self) -> bool {
        self.remaining < 0
    }
}

// ─── FSE encode/decode helpers ────────────────────────────────────────────────

/// Encode one symbol into the backward bitstream, updating the FSE state.
///
/// The encoder maintains state in `[sz, 2*sz)`. To emit symbol `sym`:
/// 1. Compute how many bits to flush: `nb = (state + delta_nb) >> 16`
/// 2. Write the low `nb` bits of `state` to the bitstream.
/// 3. New state = `st[(state >> nb) + delta_fs]`
///
/// Note: after all symbols are encoded, the final state (minus `sz`) is
/// written as `acc_log` bits to allow the decoder to initialise.
fn fse_encode_sym(
    state: &mut u32,
    sym: u8,
    ee: &[FseEe],
    st: &[u16],
    bw: &mut RevBitWriter,
) {
    let e = &ee[sym as usize];
    let nb = ((*state).wrapping_add(e.delta_nb) >> 16) as u8;
    bw.add_bits(*state as u64, nb);
    let slot_i = (*state >> nb) as i32 + e.delta_fs;
    // delta_fs is chosen during table build so slot_i is always in [0, sz),
    // but we guard with a saturating cast to prevent UB if invariants break.
    let slot = slot_i.max(0) as usize;
    debug_assert!(slot < st.len(), "FSE encoder slot out of range: {slot} >= {}", st.len());
    *state = st[slot] as u32;
}

/// Initialise an FSE encoder state directly from a symbol, WITHOUT flushing
/// any bits — the reverse-encoding-loop analogue of real zstd's
/// `FSE_initCState2`.
///
/// RFC 8878's decoder never performs a state-UPDATE read after the LAST
/// sequence in a block (there is no "next" sequence whose peek needs a
/// fresh state) — see the per-sequence loop in [`decompress_block`].
/// Symmetrically, the ENCODER's first symbol processed in its reverse loop
/// (which corresponds to that same last sequence) cannot derive its
/// starting state via a normal [`fse_encode_sym`] flush (there is no
/// bit-consuming update on the decode side to produce it) — it must be
/// computed directly.
///
/// Formula (mirrors `FSE_initCState2` in the reference C implementation):
/// `nb_bits_out = (delta_nb + (1<<15)) >> 16`,
/// `value = (nb_bits_out << 16) - delta_nb`, then a table lookup exactly
/// like [`fse_encode_sym`] but starting from that computed `value` instead
/// of a live running state.
///
/// An earlier revision of this codec always flushed a transition for every
/// sequence uniformly (no direct-init special case for the last sequence),
/// writing bits a real decoder would never read and shifting the
/// bit-alignment of everything that followed. See lessons.md Lesson 96.
fn fse_init_state(sym: u8, ee: &[FseEe], st: &[u16]) -> u32 {
    let e = &ee[sym as usize];
    let delta_nb = e.delta_nb as u64;
    let nb_bits_out = delta_nb.wrapping_add(1u64 << 15) >> 16;
    let value = (nb_bits_out << 16).wrapping_sub(delta_nb);
    let slot_i = (value >> nb_bits_out) as i64 + e.delta_fs as i64;
    let slot = slot_i.max(0) as usize;
    debug_assert!(slot < st.len(), "FSE init slot out of range: {slot} >= {}", st.len());
    st[slot] as u32
}

/// Consume `entry.nb` bits from the bitstream and compute the next FSE
/// decode state from a previously peeked table entry.
///
/// New state = `entry.base + read(entry.nb bits)`.
///
/// Per the reference decoder (`ZSTD_decodeSequence`), this update is SKIPPED
/// entirely for the LAST sequence in a block — there is no "next" sequence
/// to prepare a state for, and the encoder never flushed any bits for that
/// non-existent transition (see [`fse_init_state`]). Callers must guard the
/// call to this function with `if i != n_seqs - 1`; performing it
/// unconditionally consumes bits that were never written, corrupting the
/// position of every read that follows. See lessons.md Lesson 96.
fn fse_update_state(entry: FseDe, br: &mut RevBitReader) -> u16 {
    // Wrapping, not checked: for a validated table `base + bits` is always
    // inside the table, but a malformed one must produce an out-of-range
    // state that [`fse_cell`] rejects — never a debug-mode overflow panic.
    entry.base.wrapping_add(br.read_bits(entry.nb) as u16)
}

// ─── LL/ML/OF code number computation ────────────────────────────────────────

/// Map a literal length value to its LL code number (0..35).
///
/// Codes 0..15 are identity; codes 16+ cover ranges via lookup.
fn ll_to_code(ll: u32) -> usize {
    // Simple linear scan over LL_CODES table.
    // Codes are in increasing baseline order, so the last code whose
    // baseline ≤ ll is the correct code.
    let mut code = 0;
    for (i, &(base, _bits)) in LL_CODES.iter().enumerate() {
        if base <= ll {
            code = i;
        } else {
            break;
        }
    }
    code
}

/// Map a match length value to its ML code number (0..52).
fn ml_to_code(ml: u32) -> usize {
    let mut code = 0;
    for (i, &(base, _bits)) in ML_CODES.iter().enumerate() {
        if base <= ml {
            code = i;
        } else {
            break;
        }
    }
    code
}

// ─── Sequence struct ──────────────────────────────────────────────────────────

/// One ZStd sequence: (literal_length, match_length, match_offset).
///
/// A sequence means: emit `ll` literal bytes from the literals section,
/// then copy `ml` bytes starting `off` positions back in the output buffer.
/// After all sequences, any remaining literals are appended.
#[derive(Debug, Clone)]
struct Seq {
    ll: u32,  // literal length (bytes to copy from literal section before this match)
    ml: u32,  // match length (bytes to copy from output history)
    off: u32, // match offset (1-indexed: 1 = last byte written)
}

/// Convert LZSS tokens into ZStd sequences + a flat literals buffer.
///
/// LZSS produces a stream of `Literal(byte)` and `Match{offset, length}`.
/// ZStd groups consecutive literals before each match into a single sequence.
/// Any trailing literals (after the last match) go into the literals buffer
/// without a corresponding sequence entry.
fn tokens_to_seqs(tokens: &[lzss::Token]) -> (Vec<u8>, Vec<Seq>) {
    let mut lits = Vec::new();
    let mut seqs = Vec::new();
    let mut lit_run = 0u32;

    for tok in tokens {
        match tok {
            lzss::Token::Literal(b) => {
                lits.push(*b);
                lit_run += 1;
            }
            lzss::Token::Match { offset, length } => {
                seqs.push(Seq {
                    ll: lit_run,
                    ml: *length as u32,
                    off: *offset as u32,
                });
                lit_run = 0;
            }
        }
    }
    // Trailing literals stay in `lits`; no sequence for them.
    (lits, seqs)
}

// ─── Two-state interleaved FSE stream (Huffman weights) ──────────────────────

/// Decode the 2-state interleaved FSE bitstream that carries Huffman weights
/// (RFC 8878 §4.2.1.1, `FSE_decompress_usingDTable` in the reference).
///
/// # Why two states
///
/// A single FSE state has a serial dependency: you cannot compute symbol
/// `n+1`'s table lookup until symbol `n`'s state update has finished. ZStd
/// therefore runs TWO independent FSE states over ONE shared bitstream,
/// alternating: state A decodes symbol 0, state B decodes symbol 1, state A
/// decodes symbol 2, and so on. The two lookups in a pair are independent, so
/// a real decoder overlaps them.
///
/// # Why the termination rule is what it is
///
/// Nothing on the wire says how many weights there are — the count is
/// *implied* by where the bitstream runs out. The rule (verbatim from the
/// RFC's reference decoder) is: keep alternating; the moment a state update
/// reads past the start of the stream, stop, and emit ONE more symbol by
/// peeking the other state's current cell without consuming anything. That
/// trailing peek is not an off-by-one — it is how the final weight is
/// transmitted for free, and dropping it silently truncates every Huffman
/// table by one symbol.
///
/// `max_out` bounds the run so a corrupt table (e.g. one whose cells all read
/// zero bits) cannot produce an unbounded stream of symbols.
fn fse_decompress_interleaved2(
    tbl: &FseTable,
    src: &[u8],
    max_out: usize,
) -> Result<Vec<u8>, String> {
    let mut br = RevBitReader::new(src)?;

    // Both states are primed from the front of the (backward) stream.
    let mut s1 = br.read_bits(tbl.acc_log) as usize;
    let mut s2 = br.read_bits(tbl.acc_log) as usize;
    if br.is_overrun() {
        return Err("FSE weight stream too short to prime both states".into());
    }

    let cell = |state: usize| -> Result<FseDe, String> {
        tbl.de
            .get(state)
            .copied()
            .ok_or_else(|| format!("FSE state {state} out of range (table size {})", tbl.de.len()))
    };

    let mut out = Vec::new();
    loop {
        if out.len() + 2 > max_out {
            return Err(format!("FSE weight stream exceeds {max_out} symbols"));
        }

        let e1 = cell(s1)?;
        out.push(e1.sym);
        s1 = e1.base as usize + br.read_bits(e1.nb) as usize;
        if br.is_overrun() {
            out.push(cell(s2)?.sym);
            break;
        }

        let e2 = cell(s2)?;
        out.push(e2.sym);
        s2 = e2.base as usize + br.read_bits(e2.nb) as usize;
        if br.is_overrun() {
            out.push(cell(s1)?.sym);
            break;
        }
    }

    Ok(out)
}

// ─── Huffman table (RFC 8878 §4.2.1.1) ───────────────────────────────────────

/// One cell of the flattened Huffman decode table.
///
/// The table is indexed by the next `max_bits` bits of the stream, so a code
/// that is shorter than `max_bits` simply occupies several adjacent cells —
/// every cell reachable by "this code, followed by any suffix". Decoding is
/// then one array read plus a variable-width skip, with no bit-by-bit tree
/// walk.
#[derive(Clone, Copy, Default)]
struct HuffEntry {
    sym: u8,
    /// Bits this code actually uses (`<= max_bits`); the rest of the peeked
    /// window belongs to the following codes.
    nb: u8,
}

/// A built Huffman decode table.
struct HuffTable {
    /// `2^max_bits` cells.
    entries: Vec<HuffEntry>,
    /// Length of the longest code = number of bits to peek per symbol.
    max_bits: u8,
}

/// RFC 8878's ceiling on Huffman code length (`HUF_TABLELOG_MAX`).
///
/// Bounds the decode table at 4096 cells, and bounds any single weight.
const HUF_MAX_BITS: u8 = 12;

/// Parse a Huffman_Tree_Description, returning the table and the number of
/// bytes it occupied.
///
/// # Weights, not code lengths
///
/// The description does not transmit code lengths directly; it transmits
/// *weights*. A symbol with weight `w > 0` gets a code of length
/// `max_bits + 1 - w`, so a bigger weight means a SHORTER code. The point of
/// the indirection is that weights are small, dense, non-monotonic integers
/// that compress well, whereas code lengths are dominated by a few large
/// values.
///
/// The relation that makes it work: a code of length `L` occupies `2^-L` of
/// the code space, so weight `w` occupies `2^(w-1)` units out of `2^max_bits`.
/// Summing `2^(w-1)` over all symbols must therefore land exactly on
/// `2^max_bits` for the code to be complete (Kraft equality).
///
/// # The free last weight
///
/// The LAST symbol's weight is never transmitted. The decoder sums what it
/// received, rounds up to the next power of two to learn `max_bits`, and the
/// shortfall `left = 2^max_bits - total` *is* the last symbol's contribution
/// — so its weight is `log2(left) + 1`. This is only well-defined when `left`
/// is itself a power of two, which is precisely the check that rejects a
/// malformed description (an over-full or unfillable code space).
fn read_huffman_table(data: &[u8]) -> Result<(HuffTable, usize), String> {
    if data.is_empty() {
        return Err("empty Huffman tree description".into());
    }

    let header = data[0];
    let (mut weights, consumed) = if header >= 128 {
        // ── Direct representation ────────────────────────────────────────
        // Weights are stored raw, 4 bits each, two per byte, HIGH nibble
        // first. Used when FSE coding the weights would not pay for its own
        // table description (few symbols, or near-uniform weights).
        let n = header as usize - 127;
        let bytes = n.div_ceil(2);
        if data.len() < 1 + bytes {
            return Err(format!(
                "truncated direct Huffman weights: need {} bytes, have {}",
                1 + bytes,
                data.len()
            ));
        }
        let mut w = Vec::with_capacity(n);
        for i in 0..n {
            let b = data[1 + i / 2];
            w.push(if i % 2 == 0 { b >> 4 } else { b & 0x0F });
        }
        (w, 1 + bytes)
    } else {
        // ── FSE-compressed representation ────────────────────────────────
        // The header byte IS the compressed size, so a 1-byte header buys a
        // 0..127-byte payload: an FSE table description followed by the
        // 2-state interleaved weight stream.
        let csize = header as usize;
        if csize == 0 {
            return Err("FSE-compressed Huffman weights have zero size".into());
        }
        if data.len() < 1 + csize {
            return Err(format!(
                "truncated FSE Huffman weights: need {} bytes, have {}",
                1 + csize,
                data.len()
            ));
        }
        let body = &data[1..1 + csize];
        // The weight alphabet is 0..=12, but the RFC builds the table with
        // the generic FSE reader (max symbol 255, max accuracy_log 6).
        let (norm, acc_log, hdr_len) = read_fse_table_description(body, 6, 255)?;
        let tbl = FseTable::from_norm(&norm, acc_log)?;
        if hdr_len >= body.len() {
            return Err("FSE Huffman weights: table description leaves no payload".into());
        }
        // At most 255 weights: the 256th symbol's weight is always the
        // deduced one.
        let w = fse_decompress_interleaved2(&tbl, &body[hdr_len..], 255)?;
        (w, 1 + csize)
    };

    if weights.is_empty() || weights.len() > 255 {
        return Err(format!("Huffman description has {} weights", weights.len()));
    }

    // ── Kraft sum over the transmitted weights ───────────────────────────
    let mut total: u32 = 0;
    for (s, &w) in weights.iter().enumerate() {
        if w > HUF_MAX_BITS {
            return Err(format!(
                "Huffman weight {w} for symbol {s} exceeds maximum {HUF_MAX_BITS}"
            ));
        }
        if w > 0 {
            total += 1 << (w - 1);
        }
    }
    if total == 0 {
        return Err("Huffman description assigns no code space".into());
    }

    // max_bits = floor(log2(total)) + 1: the smallest power of two strictly
    // greater than `total`, expressed as an exponent.
    let max_bits = (32 - total.leading_zeros()) as u8;
    if max_bits > HUF_MAX_BITS {
        return Err(format!(
            "Huffman table log {max_bits} exceeds maximum {HUF_MAX_BITS}"
        ));
    }
    let left = (1u32 << max_bits) - total;
    if left == 0 || !left.is_power_of_two() {
        return Err(format!(
            "Huffman weights leave {left} units of code space, which is not a power of two \
             (description is over- or under-full)"
        ));
    }
    let last_weight = (32 - left.leading_zeros()) as u8; // log2(left) + 1
    weights.push(last_weight);

    if weights.len() > 256 {
        return Err(format!("Huffman description has {} symbols", weights.len()));
    }

    // ── Lay out the flattened table ──────────────────────────────────────
    //
    // Canonical order in ZStd runs from the LONGEST codes to the shortest:
    // rank 1 (weight 1, longest code) starts at index 0, then rank 2, and so
    // on; within a rank, symbols keep their natural order. Each symbol of
    // weight `w` claims `2^(w-1)` consecutive cells. Because the Kraft sum is
    // exactly `2^max_bits`, the cells tile the table with no gaps and no
    // overlap.
    let mut rank_count = [0u32; HUF_MAX_BITS as usize + 1];
    for &w in &weights {
        rank_count[w as usize] += 1;
    }
    let mut rank_start = [0u32; HUF_MAX_BITS as usize + 1];
    let mut next = 0u32;
    for w in 1..=max_bits as usize {
        rank_start[w] = next;
        next += rank_count[w] << (w - 1);
    }

    let table_size = 1usize << max_bits;
    let mut entries = vec![HuffEntry::default(); table_size];
    for (sym, &w) in weights.iter().enumerate() {
        if w == 0 {
            continue; // symbol absent from the alphabet
        }
        let span = 1usize << (w - 1);
        let start = rank_start[w as usize] as usize;
        if start + span > table_size {
            return Err("Huffman code assignment overflows its table".into());
        }
        let nb = max_bits + 1 - w;
        for cell in &mut entries[start..start + span] {
            *cell = HuffEntry { sym: sym as u8, nb };
        }
        rank_start[w as usize] += span as u32;
    }

    Ok((HuffTable { entries, max_bits }, consumed))
}

/// Decode exactly `n` symbols from one Huffman literal stream, appending them
/// to `out`.
///
/// Literal streams are backward bitstreams like the sequences stream: the
/// final byte carries a sentinel `1` bit marking the end of payload, and
/// decoding walks from there toward byte 0.
///
/// The stream must end EXACTLY on the `n`-th symbol. That check is what
/// distinguishes "decoded correctly" from "decoded plausible-looking garbage
/// out of a truncated stream", because zero-fill past the start of a stream
/// is indistinguishable from real data without it.
fn huff_decode_stream(
    tbl: &HuffTable,
    src: &[u8],
    n: usize,
    out: &mut Vec<u8>,
) -> Result<(), String> {
    if n == 0 {
        // A zero-length sub-stream still occupies a byte on the wire (its
        // sentinel), but there is nothing to decode from it.
        return Ok(());
    }
    let mut br = RevBitReader::new(src)?;
    out.reserve(n);
    for _ in 0..n {
        // Index by the next `max_bits` bits, then consume only the matched
        // code's own length. `peek_bits` masks to `max_bits`, so the index is
        // in range by construction.
        let idx = br.peek_bits(tbl.max_bits) as usize;
        let entry = tbl.entries[idx];
        br.skip_bits(entry.nb);
        out.push(entry.sym);
    }
    if br.remaining != 0 {
        return Err(format!(
            "Huffman literal stream did not end exactly ({} bits {})",
            br.remaining.abs(),
            if br.remaining < 0 { "over-read" } else { "left unread" }
        ));
    }
    Ok(())
}

// ─── Literals section encoding ────────────────────────────────────────────────
//
// ZStd literals can be Huffman-coded or raw. We use **Raw_Literals** (type=0),
// which is the simplest: no Huffman table, bytes are stored verbatim.
//
// Header format depends on literal count:
//   ≤ 31 bytes:   1-byte header  = (lit_len << 3) | 0b000
//   ≤ 4095 bytes: 2-byte header  = (lit_len << 4) | 0b0100
//   else:         3-byte header  = (lit_len << 4) | 0b1000
//
// The bottom 2 bits = Literals_Block_Type (0 = Raw).
// The next 2 bits = Size_Format.

fn encode_literals_section(lits: &[u8]) -> Vec<u8> {
    let n = lits.len();
    let mut out = Vec::with_capacity(n + 3);

    // Raw_Literals header format (RFC 8878 §3.1.1.2.1):
    // bits [1:0] = Literals_Block_Type = 00 (Raw)
    // bits [3:2] = Size_Format: 00 or 10 = 1-byte, 01 = 2-byte, 11 = 3-byte
    //
    // 1-byte:  size in bits [7:3] (5 bits) — header = (size << 3) | 0b000
    // 2-byte:  size in bits [11:4] (12 bits) — header = (size << 4) | 0b0100
    // 3-byte:  size in bits [19:4] (16 bits) — header = (size << 4) | 0b1100
    if n <= 31 {
        // 1-byte header: size_format=00, type=00
        out.push(((n as u32) << 3) as u8);
    } else if n <= 4095 {
        // 2-byte header: size_format=01, type=00 → `0b0100`
        let hdr = ((n as u32) << 4) | 0b0100;
        out.push(hdr as u8);
        out.push((hdr >> 8) as u8);
    } else {
        // 3-byte header: size_format=11, type=00 → `0b1100`
        let hdr = ((n as u32) << 4) | 0b1100;
        out.push(hdr as u8);
        out.push((hdr >> 8) as u8);
        out.push((hdr >> 16) as u8);
    }

    out.extend_from_slice(lits);
    out
}

/// Read a little-endian integer from up to 5 leading bytes of `data`.
///
/// The compressed-literals headers are bit fields packed across 3-5 bytes
/// with no byte alignment, so they are easiest to read as one little-endian
/// integer and then shift/mask. Returns an error rather than reading short.
fn le_uint(data: &[u8], nbytes: usize) -> Result<u64, String> {
    if data.len() < nbytes {
        return Err(format!(
            "truncated literals header: need {nbytes} bytes, have {}",
            data.len()
        ));
    }
    let mut v = 0u64;
    for (i, &b) in data[..nbytes].iter().enumerate() {
        v |= (b as u64) << (8 * i);
    }
    Ok(v)
}

/// Decode a literals section, returning `(literals, bytes_consumed)`.
///
/// Handles all four `Literals_Block_Type`s of RFC 8878 §3.1.1.2:
///
/// | Type | Name       | Content                                            |
/// |------|------------|----------------------------------------------------|
/// | 0    | Raw        | literal bytes verbatim                             |
/// | 1    | RLE        | one byte, repeated `Regenerated_Size` times        |
/// | 2    | Compressed | Huffman tree description, then Huffman bitstream(s)|
/// | 3    | Treeless   | Huffman bitstream(s), reusing the PREVIOUS tree    |
///
/// `huff` is the frame-scoped Huffman table slot: type 2 overwrites it, type
/// 3 requires it to already hold a table. That statefulness is why real zstd
/// output cannot be decoded block-by-block in isolation — a Treeless block is
/// meaningless without the block that defined the tree.
fn decode_literals_section(
    data: &[u8],
    huff: &mut Option<HuffTable>,
) -> Result<(Vec<u8>, usize), String> {
    if data.is_empty() {
        return Err("empty literals section".into());
    }

    let b0 = data[0];
    let ltype = b0 & 0b11; // bottom 2 bits = Literals_Block_Type
    let size_format = (b0 >> 2) & 0b11; // bits [3:2]

    match ltype {
        // ── Raw (0) and RLE (1) ──────────────────────────────────────────
        //
        // Both use the same size encoding; they differ only in whether the
        // payload is `Regenerated_Size` bytes or a single byte to repeat.
        //
        //   0b00 or 0b10 → 1-byte header: size = b0[7:3]  (5 bits, 0..31)
        //   0b01         → 2-byte LE header: size in bits [11:4]  (12 bits)
        //   0b11         → 3-byte LE header: size in bits [19:4]  (20 bits)
        0 | 1 => {
            let (n, header_bytes) = match size_format {
                0 | 2 => ((b0 >> 3) as usize, 1usize),
                1 => {
                    let v = le_uint(data, 2)?;
                    (((v >> 4) & 0xFFF) as usize, 2usize)
                }
                _ => {
                    let v = le_uint(data, 3)?;
                    (((v >> 4) & 0xF_FFFF) as usize, 3usize)
                }
            };
            if n > MAX_BLOCK_SIZE {
                return Err(format!(
                    "literals Regenerated_Size {n} exceeds block maximum {MAX_BLOCK_SIZE}"
                ));
            }

            if ltype == 0 {
                let end = header_bytes + n;
                if end > data.len() {
                    return Err(format!(
                        "raw literals truncated: need {end}, have {}",
                        data.len()
                    ));
                }
                Ok((data[header_bytes..end].to_vec(), end))
            } else {
                // RLE: exactly one payload byte, whatever the size field says.
                if data.len() < header_bytes + 1 {
                    return Err("RLE literals missing their payload byte".into());
                }
                Ok((vec![data[header_bytes]; n], header_bytes + 1))
            }
        }

        // ── Compressed (2) and Treeless (3) ──────────────────────────────
        //
        // Header carries BOTH a Regenerated_Size (decoded byte count) and a
        // Compressed_Size (wire byte count of everything after the header),
        // packed as bit fields after the 4 type/format bits:
        //
        //   format 00 → 3 bytes,  10+10 bits, ONE  bitstream
        //   format 01 → 3 bytes,  10+10 bits, FOUR bitstreams
        //   format 10 → 4 bytes,  14+14 bits, FOUR bitstreams
        //   format 11 → 5 bytes,  18+18 bits, FOUR bitstreams
        //
        // Note that format 00 and 01 are byte-identical apart from the stream
        // count — the only place in the format where the number of streams is
        // signalled.
        _ => {
            let (regen, comp, header_bytes, four_streams) = match size_format {
                0 | 1 => {
                    let v = le_uint(data, 3)?;
                    (
                        ((v >> 4) & 0x3FF) as usize,
                        ((v >> 14) & 0x3FF) as usize,
                        3usize,
                        size_format == 1,
                    )
                }
                2 => {
                    let v = le_uint(data, 4)?;
                    (
                        ((v >> 4) & 0x3FFF) as usize,
                        ((v >> 18) & 0x3FFF) as usize,
                        4usize,
                        true,
                    )
                }
                _ => {
                    let v = le_uint(data, 5)?;
                    (
                        ((v >> 4) & 0x3_FFFF) as usize,
                        ((v >> 22) & 0x3_FFFF) as usize,
                        5usize,
                        true,
                    )
                }
            };

            if regen > MAX_BLOCK_SIZE {
                return Err(format!(
                    "literals Regenerated_Size {regen} exceeds block maximum {MAX_BLOCK_SIZE}"
                ));
            }
            let end = header_bytes + comp;
            if end > data.len() {
                return Err(format!(
                    "compressed literals truncated: need {end}, have {}",
                    data.len()
                ));
            }
            let body = &data[header_bytes..end];

            // Type 2 defines a new tree; type 3 (Treeless) reuses the last
            // one defined in this frame.
            let streams = if ltype == 2 {
                let (table, used) = read_huffman_table(body)?;
                *huff = Some(table);
                &body[used..]
            } else {
                if huff.is_none() {
                    return Err(
                        "Treeless_Literals_Block with no preceding Huffman tree in this frame"
                            .into(),
                    );
                }
                body
            };
            let table = huff.as_ref().expect("Huffman table present by construction");

            let mut lits = Vec::with_capacity(regen);
            if !four_streams {
                huff_decode_stream(table, streams, regen, &mut lits)?;
            } else {
                decode_four_huffman_streams(table, streams, regen, &mut lits)?;
            }

            if lits.len() != regen {
                return Err(format!(
                    "Huffman literals produced {} bytes, header promised {regen}",
                    lits.len()
                ));
            }
            Ok((lits, end))
        }
    }
}

/// Decode the 4-stream Huffman literals layout (RFC 8878 §3.1.1.2.2).
///
/// # Why four streams
///
/// Huffman decoding is inherently serial — you cannot start symbol `n+1`
/// until symbol `n`'s length is known. ZStd therefore splits the literal run
/// into four quarters and gives each its own independent bitstream, so a
/// decoder can run four of these serial chains at once. The cost is a 6-byte
/// **jump table** at the front: three little-endian `u16` sizes for streams
/// 1-3. Stream 4's size is whatever is left over, which is why only three are
/// transmitted.
///
/// The split is by OUTPUT bytes, not input bytes: streams 1-3 each regenerate
/// `ceil(regen/4)` literals and stream 4 regenerates the remainder. A
/// `Regenerated_Size` below 6 cannot be split this way and is rejected by the
/// reference decoder, so it is rejected here too.
fn decode_four_huffman_streams(
    tbl: &HuffTable,
    data: &[u8],
    regen: usize,
    out: &mut Vec<u8>,
) -> Result<(), String> {
    // 6-byte jump table + a minimum of one byte (the sentinel) per stream.
    if data.len() < 10 {
        return Err(format!(
            "4-stream Huffman literals need at least 10 bytes, have {}",
            data.len()
        ));
    }
    if regen < 6 {
        return Err(format!(
            "4-stream Huffman literals cannot regenerate only {regen} bytes"
        ));
    }

    let s1 = u16::from_le_bytes([data[0], data[1]]) as usize;
    let s2 = u16::from_le_bytes([data[2], data[3]]) as usize;
    let s3 = u16::from_le_bytes([data[4], data[5]]) as usize;
    let payload = &data[6..];
    let used = s1
        .checked_add(s2)
        .and_then(|v| v.checked_add(s3))
        .ok_or("4-stream Huffman jump table sizes overflow")?;
    if used > payload.len() {
        return Err(format!(
            "4-stream Huffman jump table claims {used} bytes of a {}-byte payload",
            payload.len()
        ));
    }
    let s4 = payload.len() - used;

    // Streams 1-3 regenerate a quarter each (rounded up); stream 4 takes the
    // remainder, which the `regen >= 6` guard keeps non-negative.
    let quarter = regen.div_ceil(4);
    let last = regen
        .checked_sub(3 * quarter)
        .ok_or("4-stream Huffman literals: quarters exceed Regenerated_Size")?;

    let mut start = 0usize;
    for (i, (size, count)) in [(s1, quarter), (s2, quarter), (s3, quarter), (s4, last)]
        .into_iter()
        .enumerate()
    {
        let stream = &payload[start..start + size];
        huff_decode_stream(tbl, stream, count, out)
            .map_err(|e| format!("Huffman literal stream {}: {e}", i + 1))?;
        start += size;
    }

    Ok(())
}

// ─── Sequences section encoding ───────────────────────────────────────────────
//
// Layout:
//   [sequence_count: 1-3 bytes]
//   [symbol_compression_modes: 1 byte]  (0x00 = all Predefined)
//   [FSE bitstream: variable]
//
// Symbol compression modes byte:
//   bits [7:6] = LL mode
//   bits [5:4] = OF mode
//   bits [3:2] = ML mode
//   bits [1:0] = reserved (0)
// Mode 0 = Predefined, Mode 1 = RLE, Mode 2 = FSE_Compressed, Mode 3 = Repeat.
// We always write 0x00 (all Predefined).
//
// The FSE bitstream is a backward bit-stream (reverse bit writer). Per RFC
// 8878 §3.1.1.3.2.1.2, cross-checked against the real `zstd` CLI (TC-9) and
// the reference C source (`ZSTD_decodeSequence` / `FSE_encodeSymbol` /
// `FSE_initCState2` in `github.com/facebook/zstd`):
//
// A FORWARD-READING decoder processes each sequence as:
//   1. PEEK all three symbols (LL, ML, OF) from their CURRENT states. This
//      is a bare table lookup — the FSE state itself IS the decode-table
//      index — and consumes NO bits.
//   2. Read extra bits, in order OF, ML, LL.
//   3. Update states (consumes bits), in order LL, ML, OF — preparing the
//      states the NEXT sequence's peek will use. This step is SKIPPED for
//      the LAST sequence in the block: there is no "next" sequence to
//      prepare a state for.
//
// The very first states a forward decoder sees (used to peek the FIRST
// sequence) are read up front, in order LL, OF, ML — note this is a
// DIFFERENT order from the per-sequence update order above; RFC 8878 is
// asymmetric here.
//
// Our encoder writes the mirror image of that, backwards (since the LAST
// bits written are the FIRST bits a forward reader consumes) and with
// sequences themselves processed in reverse order (last real sequence
// first):
//   - For the first-processed sequence (semantically the LAST real
//     sequence): no incoming transition to flush — states are computed
//     directly via `fse_init_state` (mirrors `FSE_initCState2`), writing NO
//     bits.
//   - For every other sequence: flush a state transition, order OF, ML, LL
//     (write order; a forward decoder consumes this as update order LL, ML,
//     OF after decoding the PREVIOUS — i.e. next-processed — sequence).
//   - Then write extra bits, order LL, ML, OF (a forward decoder reads
//     these as OF, ML, LL immediately after peeking symbols).
//   - After all sequences, flush the initial states in write order ML, OF,
//     LL, so a forward reader sees them in order LL, OF, ML.
//   - Add sentinel and flush.
//
// An earlier revision of this codec (a) combined peek-and-update into one
// step, getting the extras/updates relative order AND the OF/ML sub-order
// wrong, and (b) always flushed a transition for every sequence instead of
// special-casing the last one. Both are self-cancelling as long as encode
// and decode agree on the (wrong) convention — every internal round-trip
// test passed regardless — but the real `zstd` CLI rejected the output as
// corrupt. See lessons.md Lesson 96.

/// Encode the Number_of_Sequences field per RFC 8878 §3.1.1.3.1.
///
/// Encoding (verified against the real `zstd` CLI and the reference C
/// source, `lib/compress/zstd_compress_sequences.c`):
/// - `0`             : 1 byte = `0x00`
/// - `1..=127`       : 1 byte = `count`
/// - `128..=32511`   : 2 bytes; `byte0 = (count >> 8) | 0x80`, `byte1 = count & 0xFF`
/// - `32512..`       : 3 bytes; `byte0 = 0xFF`, then `(count - 0x7F00)` as a
///   little-endian `u16`
///
/// **The marker/high byte MUST come first on the wire, regardless of host
/// endianness.** An earlier revision of this codec wrote the plain
/// little-endian byte pair of `count | 0x8000` — i.e. the LOW byte first and
/// the marker+high byte SECOND. That is a self-consistent round-trip (our
/// own encoder paired with our own decoder always agreed with itself, so
/// every internal test passed) but not the real wire format: any block with
/// 128+ sequences decompressed fine with THIS implementation while being
/// misparsed by the real `zstd` CLI or any other RFC 8878 decoder. See
/// lessons.md's variable-length-integer/format-marker-byte lesson — a
/// round-trip test on a self-consistent broken codec is blind to byte-order
/// bugs by construction; only cross-implementation interop testing (TC-9)
/// catches this class of bug.
fn encode_seq_count(count: usize) -> Vec<u8> {
    if count < 128 {
        vec![count as u8]
    } else if count < 0x7F00 {
        // 128..=32511: byte0 = marker + high byte, byte1 = low byte.
        let hi = ((count >> 8) as u8) | 0x80;
        let lo = (count & 0xFF) as u8;
        vec![hi, lo]
    } else {
        // 32512+: byte0 = 0xFF, next two bytes = (count - 0x7F00) as LE u16.
        let r = count - 0x7F00;
        vec![0xFF, (r & 0xFF) as u8, ((r >> 8) & 0xFF) as u8]
    }
}

/// Decode the Number_of_Sequences field. Mirrors [`encode_seq_count`].
///
/// Returns `(count, bytes_consumed)`.
fn decode_seq_count(data: &[u8]) -> Result<(usize, usize), String> {
    if data.is_empty() {
        return Err("empty sequence count".into());
    }
    let b0 = data[0];
    if b0 < 128 {
        // 1-byte encoding: value is in [0, 127]
        Ok((b0 as usize, 1))
    } else if b0 < 0xFF {
        // 2-byte encoding: byte0 = marker + high byte, byte1 = low byte
        // (RFC 8878 §3.1.1.3.1 — NOT a plain little-endian u16; see the
        // Lesson-96-referenced comment on encode_seq_count).
        if data.len() < 2 {
            return Err("truncated sequence count".into());
        }
        let count = (((b0 & 0x7F) as usize) << 8) | (data[1] as usize);
        Ok((count, 2))
    } else {
        // 3-byte encoding: byte0=0xFF, then (count - 0x7F00) as LE u16
        if data.len() < 3 {
            return Err("truncated sequence count (3-byte)".into());
        }
        let count = 0x7F00 + data[1] as usize + ((data[2] as usize) << 8);
        Ok((count, 3))
    }
}

/// Encode the sequences section using predefined FSE tables.
///
/// Callers must pass a non-empty `seqs` slice — `compress_block` never calls
/// this with an empty sequence list.
fn encode_sequences_section(seqs: &[Seq]) -> Vec<u8> {
    debug_assert!(!seqs.is_empty(), "encode_sequences_section requires at least one sequence");

    // Build encode tables (these are precomputed from the predefined distributions).
    let (ee_ll, st_ll) = build_encode_sym(&LL_NORM, LL_ACC_LOG);
    let (ee_ml, st_ml) = build_encode_sym(&ML_NORM, ML_ACC_LOG);
    let (ee_of, st_of) = build_encode_sym(&OF_NORM, OF_ACC_LOG);

    let sz_ll = 1u32 << LL_ACC_LOG;
    let sz_ml = 1u32 << ML_ACC_LOG;
    let sz_of = 1u32 << OF_ACC_LOG;

    // Placeholder initial values — always overwritten by `fse_init_state` in
    // the first loop iteration (guaranteed by the non-empty-`seqs` contract
    // above) before ever being read.
    let mut state_ll = sz_ll;
    let mut state_ml = sz_ml;
    let mut state_of = sz_of;

    let mut bw = RevBitWriter::new();

    // Encode sequences in reverse order (last real sequence first). See the
    // module-level comment above `encode_seq_count` for the full field-order
    // derivation (RFC 8878 §3.1.1.3.2.1.2 / Lesson 96).
    let mut first = true;
    for seq in seqs.iter().rev() {
        let ll_code = ll_to_code(seq.ll);
        let ml_code = ml_to_code(seq.ml);

        // Offset encoding: raw = offset + 3 (RFC 8878 §3.1.1.3.2.1)
        // code = floor(log2(raw)); extra = raw - (1 << code)
        let raw_off = seq.off + 3;
        let of_code = if raw_off <= 1 {
            0u8
        } else {
            (31 - raw_off.leading_zeros()) as u8
        };
        let of_extra = raw_off - (1u32 << of_code);
        let ml_extra = seq.ml - ML_CODES[ml_code].0;
        let ll_extra = seq.ll - LL_CODES[ll_code].0;

        if !first {
            // Transition state FROM "state used to peek the sequence
            // processed in the PREVIOUS iteration" TO "state used to peek
            // THIS sequence" — write order OF, ML, LL (a forward decoder
            // consumes this as update order LL, ML, OF, right after
            // decoding the sequence processed in the previous iteration).
            fse_encode_sym(&mut state_of, of_code, &ee_of, &st_of, &mut bw);
            fse_encode_sym(&mut state_ml, ml_code as u8, &ee_ml, &st_ml, &mut bw);
            fse_encode_sym(&mut state_ll, ll_code as u8, &ee_ll, &st_ll, &mut bw);
        } else {
            // Last real sequence: no incoming transition to flush.
            // Initialise state directly from the symbol (no bits written).
            state_of = fse_init_state(of_code, &ee_of, &st_of);
            state_ml = fse_init_state(ml_code as u8, &ee_ml, &st_ml);
            state_ll = fse_init_state(ll_code as u8, &ee_ll, &st_ll);
            first = false;
        }

        // Extra bits, write order LL, ML, OF (a forward decoder reads these
        // in order OF, ML, LL immediately after peeking symbols).
        bw.add_bits(ll_extra as u64, LL_CODES[ll_code].1);
        bw.add_bits(ml_extra as u64, ML_CODES[ml_code].1);
        bw.add_bits(of_extra as u64, of_code);
    }

    // Flush initial states (the state used to peek the FIRST real sequence).
    // A forward-reading decoder reads these FIRST, in order LL, OF, ML.
    // Since these are the very LAST bits written overall, they become the
    // FIRST bits a forward reader sees; to get read order [LL, OF, ML] we
    // write the reverse: [ML, OF, LL].
    bw.add_bits((state_ml - sz_ml) as u64, ML_ACC_LOG);
    bw.add_bits((state_of - sz_of) as u64, OF_ACC_LOG);
    bw.add_bits((state_ll - sz_ll) as u64, LL_ACC_LOG);
    bw.flush();

    bw.finish()
}

// ─── Block-level compress ─────────────────────────────────────────────────────

/// Compress one block into ZStd compressed block format.
///
/// Returns `None` if the compressed form is larger than the input (in which
/// case the caller should use a Raw block instead).
fn compress_block(block: &[u8]) -> Option<Vec<u8>> {
    // Use LZSS to generate LZ77 tokens.
    // Window = 32 KB, max match = 255, min match = 3 (same as LZSS defaults
    // but with a bigger window to improve compression ratio).
    let tokens = lzss::encode(block, 32768, 255, 3);

    // Convert tokens to ZStd sequences.
    let (lits, seqs) = tokens_to_seqs(&tokens);

    // If no sequences were found, LZ77 had nothing to compress.
    // A compressed block with 0 sequences still has overhead, so fall back.
    if seqs.is_empty() {
        return None;
    }

    let mut out = Vec::new();

    // Encode literals section (Raw_Literals).
    out.extend_from_slice(&encode_literals_section(&lits));

    // Encode sequences section.
    out.extend_from_slice(&encode_seq_count(seqs.len()));
    out.push(0x00); // Symbol_Compression_Modes = all Predefined

    let bitstream = encode_sequences_section(&seqs);
    out.extend_from_slice(&bitstream);

    if out.len() >= block.len() {
        None // Not beneficial
    } else {
        Some(out)
    }
}

// ─── Frame-scoped decoder state ──────────────────────────────────────────────

/// Everything a Compressed block may inherit from earlier blocks in the SAME
/// frame.
///
/// ZStd blocks are deliberately *not* independent. Three separate mechanisms
/// carry state forward, and a decoder that resets any of them per block will
/// mis-decode most real-world files:
///
/// 1. **Repeated offsets** (§3.1.1.3.2.1.1) — a 3-slot history of recently
///    used match offsets, so a periodic file can say "same distance as last
///    time" instead of respelling the offset.
/// 2. **The Huffman table** (§3.1.1.2) — a `Treeless_Literals_Block` ships
///    literal bitstreams with NO tree description, reusing the tree from an
///    earlier block. Re-sending a 100+ byte tree for every 128 KB block is
///    pure waste when the literal distribution barely moves.
/// 3. **The three sequence FSE tables** (§3.1.1.3.2.1) — `Repeat_Mode` says
///    "same distribution as last block", for the same reason.
///
/// Bundling them in one struct keeps "what survives a block boundary"
/// answerable by reading one type, rather than by auditing a parameter list.
struct FrameState {
    /// Most recent match offset. Starts at 1 for a new frame.
    rep1: u32,
    /// Second most recent. Starts at 4.
    rep2: u32,
    /// Third most recent. Starts at 8.
    rep3: u32,
    /// Huffman table from the last `Compressed_Literals_Block`, if any.
    huff: Option<HuffTable>,
    /// Literal-length FSE table from the last Compressed block, if any.
    ll_table: Option<FseTable>,
    /// Offset FSE table from the last Compressed block, if any.
    of_table: Option<FseTable>,
    /// Match-length FSE table from the last Compressed block, if any.
    ml_table: Option<FseTable>,
}

impl FrameState {
    /// The state at the start of a frame. The 1/4/8 offset seeds are
    /// mandated by RFC 8878 — they are not arbitrary, and a file may rely on
    /// them from its very first sequence.
    fn new() -> Self {
        FrameState {
            rep1: 1,
            rep2: 4,
            rep3: 8,
            huff: None,
            ll_table: None,
            of_table: None,
            ml_table: None,
        }
    }
}

// ─── Sequence FSE table selection (RFC 8878 §3.1.1.3.2.1) ────────────────────

/// The per-field constants that differ between the three sequence FSE tables.
struct SeqTableSpec {
    /// Human-readable field name, used only in error messages.
    name: &'static str,
    /// Predefined distribution used by `Predefined_Mode`.
    norm: &'static [i16],
    /// Accuracy log of the predefined distribution.
    acc_log: u8,
    /// Largest legal symbol value for this field. Enforcing it at table-build
    /// time is what keeps the decode loop's code-table indexing in range.
    max_symbol: usize,
    /// Largest accuracy log RFC 8878 allows a transmitted table to use.
    max_acc_log: u8,
}

const LL_SPEC: SeqTableSpec = SeqTableSpec {
    name: "literal-length",
    norm: &LL_NORM,
    acc_log: LL_ACC_LOG,
    max_symbol: 35,
    max_acc_log: 9,
};
const OF_SPEC: SeqTableSpec = SeqTableSpec {
    name: "offset",
    norm: &OF_NORM,
    acc_log: OF_ACC_LOG,
    max_symbol: 31,
    max_acc_log: 8,
};
const ML_SPEC: SeqTableSpec = SeqTableSpec {
    name: "match-length",
    norm: &ML_NORM,
    acc_log: ML_ACC_LOG,
    max_symbol: 52,
    max_acc_log: 9,
};

/// Resolve one field's `Symbol_Compression_Mode` into an actual decode table,
/// advancing `pos` past whatever the mode consumed on the wire.
///
/// The four modes trade description size against adaptivity:
///
/// | Mode | Name            | Wire cost | Meaning                            |
/// |------|-----------------|-----------|------------------------------------|
/// | 0    | Predefined      | 0 bytes   | RFC's fixed distribution           |
/// | 1    | RLE             | 1 byte    | one symbol, every time, 0 bits     |
/// | 2    | FSE_Compressed  | variable  | distribution described in-band     |
/// | 3    | Repeat          | 0 bytes   | reuse the previous block's table    |
///
/// A tiny block cannot afford mode 2 (the description would cost more than
/// the sequences), which is exactly why modes 0/1/3 exist — and why a decoder
/// that only implements mode 0 fails on small real-world files just as badly
/// as on large ones.
fn decode_seq_table(
    data: &[u8],
    pos: &mut usize,
    mode: u8,
    spec: &SeqTableSpec,
    previous: Option<&FseTable>,
) -> Result<FseTable, String> {
    if *pos > data.len() {
        return Err(format!("truncated block before {} table", spec.name));
    }
    match mode {
        0 => FseTable::from_norm(spec.norm, spec.acc_log),
        1 => {
            let sym = *data
                .get(*pos)
                .ok_or_else(|| format!("truncated {} RLE symbol", spec.name))?;
            *pos += 1;
            if sym as usize > spec.max_symbol {
                return Err(format!(
                    "{} RLE symbol {sym} exceeds maximum {}",
                    spec.name, spec.max_symbol
                ));
            }
            Ok(FseTable::rle(sym))
        }
        2 => {
            let (norm, acc_log, used) =
                read_fse_table_description(&data[*pos..], spec.max_acc_log, spec.max_symbol)
                    .map_err(|e| format!("{} table: {e}", spec.name))?;
            *pos += used;
            FseTable::from_norm(&norm, acc_log).map_err(|e| format!("{} table: {e}", spec.name))
        }
        _ => previous.cloned().ok_or_else(|| {
            format!(
                "{} table uses Repeat_Mode but no previous table exists in this frame",
                spec.name
            )
        }),
    }
}

/// Look up the decode-table cell an FSE state points at, with a bounds check.
///
/// A well-formed table can never produce an out-of-range state, but a
/// malformed one can — and this decoder is meant to survive hostile input
/// (`.apkg`/`.colpkg` archives), where an index panic is a denial of service
/// rather than a debugging aid. On `wasm32-unknown-unknown`, built with
/// `panic = "abort"`, it would be an unrecoverable trap.
fn fse_cell(state: u16, tbl: &FseTable, name: &str) -> Result<FseDe, String> {
    tbl.de.get(state as usize).copied().ok_or_else(|| {
        format!(
            "{name} FSE state {state} out of range (table size {})",
            tbl.de.len()
        )
    })
}

/// Decompress one ZStd compressed block.
///
/// Reads the literals section, sequences section, and applies the sequences
/// to the output buffer to reconstruct the original data.
///
/// `rep1`/`rep2`/`rep3` are the three Repeated_Offset registers (RFC 8878
/// §3.1.1.3.2.1.1) — IN/OUT, because they are FRAME-scoped, not
/// block-scoped: "For the first block, the starting offset history is
/// populated with Repeated_Offset1=1, Repeated_Offset2=4,
/// Repeated_Offset3=8" (RFC 8878), and every later Compressed block in the
/// same frame continues from wherever the previous Compressed block's
/// sequences left them. The caller ([`decompress`]) owns the three
/// registers and threads them through every Compressed block in a frame
/// (Raw/RLE blocks don't touch them).
///
/// WHY THIS DECODER NEEDS THIS EVEN THOUGH ITS OWN ENCODER NEVER EMITS
/// REPEAT-OFFSET SEQUENCES: [`encode_sequences_section`] always writes an
/// explicit offset code (`raw_off = offset + 3 >= 4` always, since the
/// minimum LZ77 match offset is 1), so this crate's own compress()/
/// decompress() round trip never touches the repeat-offset path — the "no
/// repeat-offset shortcuts" simplification is entirely an ENCODER-side
/// choice (see the module doc comment). But the real `zstd` CLI's encoder
/// uses repeat offsets constantly (one of its main entropy wins, especially
/// for periodic/repetitive data), so a decoder that only understands
/// explicit offset codes will systematically fail to decode a large
/// fraction of real-world `.zst` files — caught here by real CLI interop
/// (see `tc11_repeat_offset_cli_interop_constant_byte`, which reproduces the
/// exact repro found while building `code/packages/c/zstd`: 4713 bytes of a
/// single repeated byte compresses to one Compressed block whose one
/// sequence has Offset_Value=1, i.e. "reuse Repeated_Offset1"). Algorithm
/// cross-checked against both RFC 8878 §3.1.1.3.2.1.1 and the literal
/// reference C source (`ZSTD_decodeSequence` in `zstd_decompress_block.c`,
/// fetched directly rather than recalled from memory) — see lessons.md
/// Lesson 98.
fn decompress_block(
    data: &[u8],
    out: &mut Vec<u8>,
    frame: &mut FrameState,
) -> Result<(), String> {
    // Split the frame state into independent field borrows so the rest of
    // this function can hold several of them at once.
    let FrameState { rep1, rep2, rep3, huff, ll_table, of_table, ml_table } = frame;

    // ── Literals section ─────────────────────────────────────────────────
    let (lits, lit_consumed) = decode_literals_section(data, huff)?;
    let mut pos = lit_consumed;

    // ── Sequences count ──────────────────────────────────────────────────
    if pos >= data.len() {
        // Block has only literals, no sequences.
        out.extend_from_slice(&lits);
        return Ok(());
    }

    let (n_seqs, sc_bytes) = decode_seq_count(&data[pos..])?;
    pos += sc_bytes;

    if n_seqs == 0 {
        // No sequences — all content is in literals.
        out.extend_from_slice(&lits);
        return Ok(());
    }

    // ── Symbol compression modes ─────────────────────────────────────────
    if pos >= data.len() {
        return Err("missing symbol compression modes byte".into());
    }
    let modes_byte = data[pos];
    pos += 1;

    let ll_mode = (modes_byte >> 6) & 3;
    let of_mode = (modes_byte >> 4) & 3;
    let ml_mode = (modes_byte >> 2) & 3;
    if modes_byte & 3 != 0 {
        return Err("reserved bits set in Symbol_Compression_Modes".into());
    }

    // ── Per-field FSE tables ─────────────────────────────────────────────
    //
    // The three table descriptions, when present, appear on the wire in the
    // order Literals_Lengths, Offsets, Match_Lengths — note that this is NOT
    // the order the mode bits are packed in (LL, OF, ML is the same, but the
    // per-sequence decode order below is different again). Each is parsed
    // in-place, advancing `pos`, so a Repeat/Predefined/RLE field contributes
    // 0/0/1 bytes and an FSE_Compressed field contributes as many as its
    // description needs.
    let ll_t = decode_seq_table(data, &mut pos, ll_mode, &LL_SPEC, ll_table.as_ref())?;
    let of_t = decode_seq_table(data, &mut pos, of_mode, &OF_SPEC, of_table.as_ref())?;
    let ml_t = decode_seq_table(data, &mut pos, ml_mode, &ML_SPEC, ml_table.as_ref())?;

    // Publish them for a later block's Repeat_Mode. Predefined and RLE
    // tables count as "the previous table" too: RFC 8878's Repeat_Mode
    // repeats whatever table the previous Compressed block ended up using,
    // not specifically an FSE_Compressed one.
    *ll_table = Some(ll_t.clone());
    *of_table = Some(of_t.clone());
    *ml_table = Some(ml_t.clone());

    // ── FSE bitstream ────────────────────────────────────────────────────
    if pos > data.len() {
        return Err("sequence table descriptions overran the block".into());
    }
    let bitstream = &data[pos..];
    let mut br = RevBitReader::new(bitstream)?;

    // Initialise FSE states from the bitstream. RFC 8878 §3.1.1.3.2.1.2: the
    // initial states are read in order LL, OF, ML (note: this is a
    // DIFFERENT order from the per-sequence symbol decode below, which is
    // OF, ML, LL for extras and LL, ML, OF for updates — the RFC is
    // asymmetric here; verified against the real `zstd` CLI, see Lesson 96).
    // Each state is as wide as ITS OWN table's accuracy_log, which with
    // FSE_Compressed/RLE modes is a per-block value rather than the fixed
    // predefined constant.
    let mut state_ll = br.read_bits(ll_t.acc_log) as u16;
    let mut state_of = br.read_bits(of_t.acc_log) as u16;
    let mut state_ml = br.read_bits(ml_t.acc_log) as u16;
    if br.is_overrun() {
        return Err("sequence bitstream too short to prime the FSE states".into());
    }

    // Track position in the literals buffer.
    let mut lit_pos = 0usize;

    // Apply each sequence.
    for i in 0..n_seqs {
        // Step 1 — PEEK symbols from the current states. This is a bare
        // table lookup (table[state].sym) and consumes NO bits — the FSE
        // state itself already IS the decode-table index. Only the
        // subsequent state UPDATE (step 3 below) reads bits.
        let ll_entry = fse_cell(state_ll, &ll_t, LL_SPEC.name)?;
        let ml_entry = fse_cell(state_ml, &ml_t, ML_SPEC.name)?;
        let of_entry = fse_cell(state_of, &of_t, OF_SPEC.name)?;
        let ll_code = ll_entry.sym;
        let ml_code = ml_entry.sym;
        let of_code = of_entry.sym;

        if ll_code as usize >= LL_CODES.len() {
            return Err(format!("invalid LL code {ll_code}"));
        }
        if ml_code as usize >= ML_CODES.len() {
            return Err(format!("invalid ML code {ml_code}"));
        }
        // Offset codes are the exponent of a power of two, so anything above
        // 31 would shift a u32 out of existence. `OF_SPEC.max_symbol` already
        // rejects such tables at build time; this is the second line of
        // defence at the point of use.
        if of_code > 31 {
            return Err(format!("invalid offset code {of_code}"));
        }
        let ll_info = LL_CODES[ll_code as usize];
        let ml_info = ML_CODES[ml_code as usize];

        // `ll_is_zero` is needed for the repeat-offset interpretation below
        // (RFC 8878's "when Literals_Length is 0, repeated offsets are
        // shifted by 1" rule) and is knowable right now, from the PEEKED
        // `ll_code` alone — LL code 0 is the only code with baseline 0 and 0
        // extra bits, so `ll_code == 0` iff the eventual decoded `ll` value
        // is 0. No extra bits need to be read yet to know this.
        let ll_is_zero = ll_code == 0;

        // Step 2 — read the VALUE extra bits, order OF, ML, LL (RFC 8878
        // §3.1.1.3.2.1.2 — "Decoding starts by reading the Number_of_Bits
        // required to decode offset. It does the same for Match_Length and
        // then for Literals_Length."). The NUMBER of bits read for the
        // offset field is always exactly `of_code` regardless of the
        // repeat-offset interpretation below (the reference decoder never
        // varies bit-consumption on `ll_is_zero` — only how the resulting
        // value maps to an actual offset changes).
        let of_raw = (1u32 << of_code) | br.read_bits(of_code) as u32;
        let ml = ml_info.0 + br.read_bits(ml_info.1) as u32;
        let ll = ll_info.0 + br.read_bits(ll_info.1) as u32;

        // Offset_Value -> actual offset (RFC 8878 §3.1.1.3.2.1.1), including
        // the Repeated_Offset (R1/R2/R3) mechanism — see the doc comment on
        // `decompress_block` and lessons.md Lesson 98.
        //
        // `of_code >= 2` guarantees `of_raw = (1<<of_code)+extra >= 4`, i.e.
        // Offset_Value > 3: an ordinary explicit offset. `of_code <= 1`
        // guarantees `of_raw` in `{1, 2, 3}`: a repeat-offset reference.
        //
        // The repeat case collapses to one selector in `[0, 3]`:
        //     selector = ll_is_zero + of_raw - 1
        // (cross-checked against both RFC 8878 prose and the reference
        // decoder's `ofBase + ll0 + extra_bit` in `ZSTD_decodeSequence`):
        //   0 -> reuse rep1 unchanged (no rotation)
        //   1 -> use rep2 (rep1,rep2 swap; rep3 untouched)
        //   2 -> use rep3 (full rotate: rep1,rep2,rep3 <- new,old_rep1,old_rep2)
        //   3 -> use rep1-1 (full rotate, same shape as selector 2)
        let offset = if of_code >= 2 {
            let offset = of_raw - 3;
            *rep3 = *rep2;
            *rep2 = *rep1;
            *rep1 = offset;
            offset
        } else {
            let selector = ll_is_zero as u32 + of_raw - 1;
            match selector {
                0 => *rep1,
                1 => {
                    // rep1 <- old rep2, rep2 <- old rep1; the returned
                    // offset is the new rep1 (== old rep2).
                    std::mem::swap(rep1, rep2);
                    *rep1
                }
                2 => {
                    let offset = *rep3;
                    *rep3 = *rep2;
                    *rep2 = *rep1;
                    *rep1 = offset;
                    offset
                }
                _ => {
                    // selector == 3: "rep1 - 1" — the RFC's special case for
                    // when the ordinary rep1/rep2/rep3 slots would otherwise
                    // collide with Literals_Length==0. Real zstd's decoder
                    // saturates at 0 rather than underflowing (an offset of
                    // 0 is then rejected below by the ordinary
                    // offset-bounds check, same as any other malformed
                    // offset would be).
                    let offset = rep1.saturating_sub(1);
                    *rep3 = *rep2;
                    *rep2 = *rep1;
                    *rep1 = offset;
                    offset
                }
            }
        };

        // Step 3 — update FSE states (consumes bits), order LL, ML, OF (RFC
        // 8878 §3.1.1.3.2.1.2 — "Literals_Length_State is updated, followed
        // by Match_Length_State, and then Offset_State"), preparing the
        // states the NEXT sequence's peek (step 1) will use.
        //
        // Per the reference decoder (`ZSTD_decodeSequence`): this update is
        // skipped entirely for the LAST sequence — there is no "next"
        // sequence to prepare a state for, and (symmetrically) the encoder
        // never flushed any bits for that non-existent transition (see
        // `fse_init_state` in `encode_sequences_section`). Performing this
        // read unconditionally, as an earlier revision of this codec did,
        // consumes bits that were never written, corrupting the position of
        // every read that follows. See lessons.md Lesson 96.
        if i != n_seqs - 1 {
            state_ll = fse_update_state(ll_entry, &mut br);
            state_ml = fse_update_state(ml_entry, &mut br);
            state_of = fse_update_state(of_entry, &mut br);
        }

        // Emit `ll` literal bytes from the literals buffer.
        let lit_end = lit_pos + ll as usize;
        if lit_end > lits.len() {
            return Err(format!(
                "literal run {ll} overflows literals buffer (pos={lit_pos} len={})",
                lits.len()
            ));
        }
        check_output_budget(out.len(), ll as usize)?;
        out.extend_from_slice(&lits[lit_pos..lit_end]);
        lit_pos = lit_end;

        // Copy `ml` bytes from `offset` back in the output buffer.
        // Note: offset = 0 would be a back-reference to (out.len() - 0),
        // which is past the end. The minimum valid offset here is 1.
        if offset == 0 || offset as usize > out.len() {
            return Err(format!(
                "bad match offset {} (output len {})",
                offset,
                out.len()
            ));
        }
        // Decompression-bomb guard: see the doc comment on `check_output_budget`.
        check_output_budget(out.len(), ml as usize)?;
        let copy_start = out.len() - offset as usize;
        for j in 0..ml as usize {
            let byte = out[copy_start + j];
            out.push(byte);
        }
    }

    // The sequences bitstream must end EXACTLY where the last sequence left
    // it — the reference decoder's `BIT_endOfDStream` check. Leftover bits
    // mean the block claimed fewer sequences than it encoded; missing bits
    // mean the decoder read zero-fill past the front of the stream and the
    // sequences it produced are fiction.
    if br.remaining != 0 {
        return Err(format!(
            "sequence bitstream did not end exactly ({} bits {})",
            br.remaining.abs(),
            if br.remaining < 0 { "over-read" } else { "left unread" }
        ));
    }

    // Any remaining literals after the last sequence.
    check_output_budget(out.len(), lits.len() - lit_pos)?;
    out.extend_from_slice(&lits[lit_pos..]);

    Ok(())
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Compress `data` to ZStd format (RFC 8878).
///
/// The output is a valid ZStd frame that can be decompressed by the `zstd`
/// CLI tool or any conforming implementation.
///
/// # Examples
///
/// ```
/// use zstd::{compress, decompress};
///
/// // Repeated content compresses well once past the frame overhead.
/// let data = "the quick brown fox ".repeat(20);
/// let compressed = compress(data.as_bytes());
/// assert!(compressed.len() < data.len());
/// assert_eq!(decompress(&compressed).unwrap(), data.as_bytes());
/// ```
pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    // ── ZStd frame header ────────────────────────────────────────────────
    // Magic number (4 bytes LE).
    out.extend_from_slice(&MAGIC.to_le_bytes());

    // Frame Header Descriptor (FHD):
    //   bit 7-6: FCS_Field_Size flag = 11 → 8-byte FCS
    //   bit 5:   Single_Segment_Flag = 1 (no Window_Descriptor follows)
    //   bit 4:   Unused_bit = 0
    //   bit 3:   Reserved_bit = 0
    //   bit 2:   Content_Checksum_Flag = 0 (we don't append a checksum)
    //   bit 1-0: Dict_ID_Flag = 0
    // = 0b1110_0000 = 0xE0
    out.push(0xE0);

    // Frame_Content_Size (8 bytes LE) — the uncompressed size.
    // A decoder can use this to pre-allocate the output buffer.
    out.extend_from_slice(&(data.len() as u64).to_le_bytes());

    // ── Blocks ───────────────────────────────────────────────────────────
    // Handle the special case of completely empty input: emit one empty raw block.
    if data.is_empty() {
        // Last=1, Type=Raw(00), Size=0 → header = 0b0000_0001 = 0x01
        let hdr: u32 = 0b001; // last=1, type=00, size=0
        out.extend_from_slice(&hdr.to_le_bytes()[..3]);
        return out;
    }

    let mut offset = 0;
    while offset < data.len() {
        let end = (offset + MAX_BLOCK_SIZE).min(data.len());
        let block = &data[offset..end];
        let last = end == data.len();

        // ── Try RLE block ─────────────────────────────────────────────
        // If all bytes in the block are identical, a single-byte RLE block
        // encodes it in just 1 byte (plus 3-byte header = 4 bytes total).
        if !block.is_empty() && block.iter().all(|&b| b == block[0]) {
            let hdr = ((block.len() as u32) << 3) | (0b01 << 1) | (last as u32);
            out.extend_from_slice(&hdr.to_le_bytes()[..3]);
            out.push(block[0]);
        } else {
            // ── Try compressed block ──────────────────────────────────
            let maybe_compressed = compress_block(block);
            if let Some(compressed) = maybe_compressed {
                let hdr = ((compressed.len() as u32) << 3) | (0b10 << 1) | (last as u32);
                out.extend_from_slice(&hdr.to_le_bytes()[..3]);
                out.extend_from_slice(&compressed);
            } else {
                // ── Raw block (fallback) ──────────────────────────────
                let hdr = ((block.len() as u32) << 3) | (last as u32);
                out.extend_from_slice(&hdr.to_le_bytes()[..3]);
                out.extend_from_slice(block);
            }
        }

        offset = end;
    }

    out
}

/// Decompress a ZStd frame, returning the original data.
///
/// Accepts a single ZStd frame using any of:
/// - Single-segment or multi-segment layout, with or without a content
///   checksum (the checksum's presence is parsed; its value is not verified,
///   as this crate has no xxHash64)
/// - Raw, RLE and Compressed blocks
/// - All four literals block types: Raw, RLE, Compressed (Huffman) and
///   Treeless, in both the single-stream and 4-stream forms
/// - All four sequence table modes: Predefined, RLE, FSE_Compressed and
///   Repeat, for each of the LL/OF/ML fields
/// - Repeated offsets (R1/R2/R3)
///
/// Not supported, and reported as such rather than mis-decoded: frames
/// compressed against a dictionary (a non-zero `Dictionary_ID` is an
/// explicit error, since the dictionary pre-seeds match history and all four
/// entropy tables). Also unsupported: skippable frames, and multiple
/// concatenated frames — only the first frame in the buffer is decoded.
///
/// # Errors
///
/// Returns an error string, never a panic, for any malformed input:
/// truncation, a bad magic number, a reserved block type, a table
/// description that does not describe a valid distribution, a Huffman
/// description whose code space cannot be completed, a Repeat_Mode field
/// with nothing to repeat, a bitstream that does not end exactly, an offset
/// pointing before the start of the output, or output exceeding
/// [`MAX_OUTPUT`].
///
/// # Examples
///
/// ```
/// use zstd::{compress, decompress};
///
/// let original = b"hello, world!";
/// assert_eq!(decompress(&compress(original)).unwrap(), original);
/// ```
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 5 {
        return Err("frame too short".into());
    }

    // ── Validate magic ───────────────────────────────────────────────────
    let magic = u32::from_le_bytes(data[0..4].try_into().unwrap());
    if magic != MAGIC {
        return Err(format!("bad magic: {magic:#010x} (expected {MAGIC:#010x})"));
    }

    let mut pos = 4;

    // ── Parse Frame Header Descriptor ───────────────────────────────────
    // FHD encodes several flags that control the header layout.
    let fhd = data[pos];
    pos += 1;

    // FCS_Field_Size: bits [7:6] of FHD.
    //   00 → 0 bytes if Single_Segment=0, else 1 byte
    //   01 → 2 bytes (value + 256)
    //   10 → 4 bytes
    //   11 → 8 bytes
    let fcs_flag = (fhd >> 6) & 3;

    // Single_Segment_Flag: bit 5. When set, the window descriptor is omitted.
    let single_seg = (fhd >> 5) & 1;

    // Content_Checksum_Flag: bit 2. When set, a 4-byte xxHash64 checksum
    // follows the last block. We don't validate the checksum value, but we
    // need to know it's there so we can skip past it correctly.
    //
    // This bit was previously (incorrectly) read from bit 4. Verified
    // empirically against the real `zstd` CLI: `zstd -c file.txt`
    // (checksum on by default) emits FHD byte 0x64; `zstd -c --no-check
    // file.txt` emits FHD byte 0x60 — the differing bit is bit 2. RFC 8878
    // §3.1.1.1 agrees: bit 4 is `Unused_bit`, bit 2 is `Content_Checksum_Flag`.
    // See lessons.md Lesson 95.
    let checksum_flag = (fhd >> 2) & 1;

    // Dict_ID_Flag: bits [1:0]. Indicates how many bytes the dict ID occupies.
    let dict_flag = fhd & 3;

    // ── Window Descriptor ────────────────────────────────────────────────
    // Present only if Single_Segment_Flag = 0. We skip it (we don't enforce
    // window size limits in this implementation).
    if single_seg == 0 {
        pos += 1; // skip Window_Descriptor byte
    }

    // ── Dict ID ──────────────────────────────────────────────────────────
    //
    // A frame compressed against a dictionary is NOT decodable without that
    // dictionary, and the failure is not merely "some bytes are missing":
    // the dictionary pre-seeds the match history, the three sequence FSE
    // tables and the Huffman table, so the frame's very first block may
    // legitimately use `Repeat_Mode` or a `Treeless_Literals_Block` with no
    // preceding block to inherit from, and its first offsets may point back
    // into dictionary content that was never in this frame.
    //
    // Skipping the field and pressing on — as this decoder used to — turns
    // that into whatever error the missing state happens to trip first (in
    // practice a baffling "offset table uses Repeat_Mode but no previous
    // table exists in this frame"), or, on a frame that happens not to trip
    // one, into silently wrong output. Both are worse than saying what is
    // actually true, so this is checked and reported explicitly.
    let dict_id_bytes = [0usize, 1, 2, 4][dict_flag as usize];
    if pos + dict_id_bytes > data.len() {
        return Err("truncated Dictionary_ID".into());
    }
    let mut dict_id: u32 = 0;
    for i in 0..dict_id_bytes {
        dict_id |= (data[pos + i] as u32) << (8 * i);
    }
    pos += dict_id_bytes;
    // Dictionary_ID 0 means "no dictionary" even when the field is present.
    if dict_id != 0 {
        return Err(format!(
            "frame requires dictionary {dict_id}; dictionaries are not supported"
        ));
    }

    // ── Frame Content Size ───────────────────────────────────────────────
    // We read but don't validate FCS (we trust the blocks to be correct).
    let fcs_bytes = match fcs_flag {
        0 => {
            if single_seg == 1 { 1 } else { 0 }
        }
        1 => 2,
        2 => 4,
        3 => 8,
        _ => unreachable!(),
    };
    pos += fcs_bytes; // skip FCS

    // ── Blocks ───────────────────────────────────────────────────────────
    // Guard against decompression bombs: cap total output at MAX_OUTPUT.
    // See the doc comment on `check_output_budget` for why Compressed
    // blocks need this checked incrementally (inside `decompress_block`),
    // not just once per Raw/RLE block here.
    let mut out = Vec::new();

    // Frame-scoped decoder state: repeated offsets, the Huffman table, and
    // the three sequence FSE tables all survive block boundaries. See
    // [`FrameState`] for why each of them has to.
    let mut frame = FrameState::new();

    loop {
        if pos + 3 > data.len() {
            return Err("truncated block header".into());
        }

        // 3-byte little-endian block header.
        let hdr = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], 0]);
        pos += 3;

        let last = (hdr & 1) != 0;
        let btype = (hdr >> 1) & 3;
        let bsize = (hdr >> 3) as usize;

        match btype {
            0 => {
                // Raw block: `bsize` bytes of verbatim content.
                if pos + bsize > data.len() {
                    return Err(format!("raw block truncated: need {bsize} bytes at pos {pos}"));
                }
                check_output_budget(out.len(), bsize)?;
                out.extend_from_slice(&data[pos..pos + bsize]);
                pos += bsize;
            }
            1 => {
                // RLE block: 1 byte repeated `bsize` times.
                if pos >= data.len() {
                    return Err("RLE block missing byte".into());
                }
                check_output_budget(out.len(), bsize)?;
                let byte = data[pos];
                pos += 1;
                out.extend(std::iter::repeat_n(byte, bsize));
            }
            2 => {
                // Compressed block.
                if pos + bsize > data.len() {
                    return Err(format!("compressed block truncated: need {bsize} bytes"));
                }
                let block_data = &data[pos..pos + bsize];
                pos += bsize;
                decompress_block(block_data, &mut out, &mut frame)?;
            }
            3 => {
                return Err("reserved block type 3".into());
            }
            _ => unreachable!(),
        }

        if last {
            break;
        }
    }

    // ── Content checksum ─────────────────────────────────────────────────
    // If Content_Checksum_Flag was set, a 4-byte xxHash64 checksum of the
    // decompressed content follows the last block. We don't verify the
    // checksum value (no xxHash64 implementation in this crate), but we
    // must skip past it so callers that inspect `data` past this point (or
    // a future trailing-bytes-must-be-empty check) don't misinterpret it as
    // corruption. Real `zstd` writes this by default (`zstd -c` without
    // `--no-check`), so any real-world interop input is likely to have it.
    if checksum_flag == 1 && pos + 4 > data.len() {
        return Err("truncated content checksum".into());
    }

    Ok(out)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: round-trip via our own compress/decompress.
    fn rt(data: &[u8]) -> Vec<u8> {
        decompress(&compress(data)).expect("round-trip failed")
    }

    // ── TC-1: empty input ─────────────────────────────────────────────────────

    #[test]
    fn tc1_empty() {
        // An empty input must produce a valid ZStd frame and decompress back
        // to empty bytes without panic or error.
        assert_eq!(rt(b""), b"");
    }

    // ── TC-2: single byte ─────────────────────────────────────────────────────

    #[test]
    fn tc2_single() {
        // The smallest non-empty input: one byte.
        assert_eq!(rt(b"\x42"), b"\x42");
    }

    // ── TC-3: all 256 byte values ─────────────────────────────────────────────

    #[test]
    fn tc3_all_bytes() {
        // Every possible byte value 0x00..=0xFF in order. This exercises
        // literal encoding of non-ASCII and zero bytes.
        let input: Vec<u8> = (0u8..=255).collect();
        assert_eq!(rt(&input), input);
    }

    // ── TC-4: RLE block ───────────────────────────────────────────────────────

    #[test]
    fn tc4_rle() {
        // 1024 identical bytes should be detected as an RLE block.
        // Expected compressed size: 4 (magic) + 1 (FHD) + 8 (FCS) + 3 (block header)
        //                         + 1 (RLE byte) = 17 bytes < 30.
        let input = vec![b'A'; 1024];
        let compressed = compress(&input);
        assert_eq!(decompress(&compressed).unwrap(), input);
        assert!(
            compressed.len() < 30,
            "RLE of 1024 bytes compressed to {} (expected < 30)",
            compressed.len()
        );
    }

    // ── TC-5: English prose ───────────────────────────────────────────────────

    #[test]
    fn tc5_prose() {
        // Repeated English text has strong LZ77 matches. Must achieve ≥ 20%
        // compression (output ≤ 80% of input size).
        let text = "the quick brown fox jumps over the lazy dog ".repeat(25);
        let input = text.as_bytes();
        let compressed = compress(input);
        assert_eq!(decompress(&compressed).unwrap(), input);
        let threshold = input.len() * 80 / 100;
        assert!(
            compressed.len() < threshold,
            "prose: compressed {} bytes (input {}), expected < {} (80%)",
            compressed.len(), input.len(), threshold
        );
    }

    // ── TC-6: pseudo-random data ──────────────────────────────────────────────

    #[test]
    fn tc6_random() {
        // LCG pseudo-random bytes. No significant compression expected, but
        // round-trip must be exact regardless of block type chosen.
        let mut seed = 42u32;
        let input: Vec<u8> = (0..512)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed & 0xFF) as u8
            })
            .collect();
        assert_eq!(rt(&input), input);
    }

    // ── TC-7: 200 KB single-byte run ──────────────────────────────────────────

    #[test]
    fn tc7_multiblock() {
        // 200 KB > MAX_BLOCK_SIZE (128 KB), so this requires at least 2 blocks.
        // Both should be RLE blocks since all bytes are identical.
        let input = vec![b'x'; 200 * 1024];
        assert_eq!(rt(&input), input);
    }

    // ── TC-8: repeat-offset pattern ───────────────────────────────────────────
    //
    // NOTE on the name: per spec TC-8, this input is constructed so a
    // repeat-offset-AWARE encoder could compress it efficiently — but this
    // crate's own encoder never emits repeat-offset codes (see the crate
    // doc comment / lessons.md Lesson 98), so this test only exercises our
    // encoder's ordinary explicit-offset path, self round-tripped through
    // our own decoder. It does NOT exercise repeat-offset DECODING at all.
    // For a test that actually proves this decoder understands
    // repeat-offset sequences (as emitted by a real repeat-offset-aware
    // encoder), see `tc11_repeat_offset_cli_interop_constant_byte` /
    // `_periodic` below, which decode real `zstd`-CLI output.

    #[test]
    fn tc8_repeat_offset() {
        // Alternating pattern with long runs of 'X' and repeated "ABCDEFGH".
        // The 'X' runs and repeated patterns both give strong LZ77 matches.
        let pattern = b"ABCDEFGH";
        let mut input = pattern.to_vec();
        for _ in 0..10 {
            input.extend_from_slice(&[b'X'; 128]);
            input.extend_from_slice(pattern);
        }
        let compressed = compress(&input);
        assert_eq!(decompress(&compressed).unwrap(), input);
        let threshold = input.len() * 70 / 100;
        assert!(
            compressed.len() < threshold,
            "repeat-offset: compressed {} (input {}), expected < {} (70%)",
            compressed.len(), input.len(), threshold
        );
    }

    // ── Extra: deterministic output ───────────────────────────────────────────
    // (Not one of the spec's numbered TCs — see TC-9 below for the real
    // spec TC-9, cross-language interoperability.)

    #[test]
    fn determinism_reproducible_output() {
        // Compressing the same data twice must produce identical bytes.
        // This is required for reproducible builds and cache invalidation.
        let data = b"hello, ZStd world! ".repeat(50);
        assert_eq!(compress(data.as_slice()), compress(data.as_slice()));
    }

    // ── TC-9: Cross-language / interoperability ───────────────────────────────
    //
    // Per `code/specs/CMP07-zstd.md` TC-9: compress with the standard `zstd`
    // CLI, decompress with ours, AND compress with ours, decompress with the
    // standard `zstd -d` CLI — both directions must round-trip exactly.
    //
    // This is the test that actually proves the wire format is real RFC
    // 8878, not just a self-consistent internal format. A codec whose
    // encoder and decoder always agree with each other can still be
    // silently wrong — see lessons.md Lesson 96 for three compounding bugs
    // of exactly this shape (a fabricated FSE table-spread algorithm, wrong
    // per-sequence field order, and a missing last-sequence update-skip)
    // that survived this crate's entire history because this test simply
    // didn't exist: every internal round-trip test, including a dedicated
    // low-level "encode two sequences, decode them, check they match" unit
    // test, passed regardless of which bugs were present, because both
    // sides of the comparison were wrong in the identical way.
    //
    // ── A missing `zstd` binary is a FAILURE, not a skip ──────────────────
    //
    // These tests used to open with `if !is_zstd_cli_available() { return; }`.
    // That is not a gate; it is a gate-shaped no-op. On any machine — or CI
    // runner — without the binary, every cross-implementation test in this
    // file reported PASS while checking nothing at all. That is exactly the
    // failure mode they exist to prevent: this crate's history holds four
    // separate wire-format bugs (Lessons 95/96/98, plus the Huffman/FSE gap
    // this suite was extended for) that self-round-trip tests could not see
    // and only the CLI oracle caught. A silently skipped oracle is
    // indistinguishable from no oracle.
    //
    // So [`require_zstd_cli`] panics instead.
    //
    // The live-CLI tests are additionally scoped to `#[cfg(unix)]`. That is
    // NOT a runtime escape hatch — it is a compile-time statement that the
    // conformance gate on Windows is a different, equally real one: the
    // GOLDEN VECTOR suite further down, which is genuine `zstd` CLI output
    // committed as bytes and decoded unconditionally on every platform with
    // no subprocess involved. Removing the binary from a machine can make
    // the build fail or narrow coverage; it can never make a conformance
    // failure look like a pass.

    /// Returns once the `zstd` CLI is confirmed present, and panics with an
    /// actionable message otherwise. It never reports "unavailable" — see the
    /// commentary above for why that option was deliberately removed.
    #[cfg(unix)]
    fn require_zstd_cli() {
        let present = std::process::Command::new("zstd")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        assert!(
            present,
            "the real `zstd` CLI is REQUIRED by this crate's interop tests — it is \
             the conformance oracle, and skipping it would turn every \
             cross-implementation check in this file into a silent no-op. Install \
             it (`apt-get install zstd`, `brew install zstd`, `dnf install zstd`) \
             and re-run."
        );
    }

    /// Runs `zstd` with the given arguments, returning captured stdout.
    /// Panics (failing the calling test) if the CLI exits non-zero.
    #[cfg(unix)]
    fn run_zstd_capture_stdout(args: &[&str]) -> Vec<u8> {
        let output = std::process::Command::new("zstd")
            .args(args)
            .output()
            .expect("failed to spawn zstd CLI");
        assert!(
            output.status.success(),
            "zstd CLI failed (args={args:?}): {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    /// Runs `zstd` with `data` piped in on STDIN, returning captured stdout.
    ///
    /// This is a genuinely different frame shape from compressing a named
    /// file, not a stylistic variation: with no file to `stat`, `zstd`
    /// cannot know the content size up front, so it omits the
    /// `Frame_Content_Size` field and emits a `Window_Descriptor` instead
    /// (`Single_Segment_Flag = 0`). Every library that streams — including
    /// the one that writes Anki's `.colpkg` payloads — produces frames of
    /// this shape, and a decoder can parse the file-shaped header perfectly
    /// while mis-parsing this one.
    #[cfg(unix)]
    fn run_zstd_stdin(args: &[&str], data: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let mut child = std::process::Command::new("zstd")
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn zstd CLI");
        child
            .stdin
            .as_mut()
            .expect("zstd stdin")
            .write_all(data)
            .expect("failed to write to zstd stdin");
        let output = child.wait_with_output().expect("zstd did not exit");
        assert!(
            output.status.success(),
            "zstd CLI failed (args={args:?}): {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    /// Writes `data` to a fresh temp file and returns its path. The caller
    /// is responsible for deleting it (tests use a `finally`-style cleanup
    /// via a guard so the file is removed even if an assertion panics).
    #[cfg(unix)]
    fn write_temp_file(prefix: &str, data: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "zstd-rust-{prefix}-{}-{}",
            std::process::id(),
            data.len(),
        ));
        std::fs::write(&path, data).expect("failed to write temp file");
        path
    }

    /// Deletes a temp file on drop, even if the test panics partway through
    /// — mirrors the Java interop tests' `finally { Files.deleteIfExists }`.
    #[cfg(unix)]
    struct TempFileGuard(std::path::PathBuf);
    #[cfg(unix)]
    impl Drop for TempFileGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    #[cfg(unix)]
    fn tc9_cli_interop() {
        require_zstd_cli();

        let text = "the quick brown fox jumps over the lazy dog ".repeat(25);
        let original = text.as_bytes();

        // ── Direction 1: compress with ours, decompress with `zstd -d` ────
        let our_compressed = compress(original);
        let ours_zst = write_temp_file("tc9-ours", &our_compressed);
        let _guard1 = TempFileGuard(ours_zst.clone());
        let decoded_by_cli = run_zstd_capture_stdout(&[
            "-d",
            "-q",
            "-c",
            ours_zst.to_str().unwrap(),
        ]);
        assert_eq!(
            decoded_by_cli, original,
            "real `zstd -d` failed to decode our compressed output"
        );

        // ── Direction 2: compress with `zstd`, decompress with ours ───────
        let theirs_input = write_temp_file("tc9-theirs-input", original);
        let _guard2 = TempFileGuard(theirs_input.clone());
        let their_compressed =
            run_zstd_capture_stdout(&["-q", "-c", theirs_input.to_str().unwrap()]);
        let decoded_by_us =
            decompress(&their_compressed).expect("our decompress() failed on real zstd output");
        assert_eq!(
            decoded_by_us, original,
            "our decompress() failed to decode real `zstd`'s compressed output"
        );
    }

    // ── Repeated-Offset (R1/R2/R3) decode interop ─────────────────────────────
    //
    // This crate's own encoder (`encode_sequences_section`, via
    // `raw_off = seq.off + 3 >= 4` always) never emits an Offset_Value <= 3,
    // so it never emits a repeat-offset code — an explicit "no repeat-offset
    // shortcuts" educational simplification. That means TC-8 above (and any
    // other self round-trip test in this file) NEVER exercises the
    // repeat-offset DECODE path, no matter how repetitive its input is: our
    // own round trip is blind to this by construction, same as it was blind
    // to the FSE-codec bugs in Lesson 96 until real CLI interop (TC-9) was
    // added. But the real `zstd` CLI's encoder uses repeat offsets
    // constantly — they're one of its main entropy wins, especially for
    // periodic/constant data — so a decoder that only understands explicit
    // offset codes (`offset = of_raw - 3` unconditionally) will fail on a
    // large fraction of real-world `.zst` files. See lessons.md Lesson 98
    // (found while building `code/packages/c/zstd`, PR #9941) for the full
    // writeup and the reference-C-source cross-check
    // (`ZSTD_decodeSequence` in `zstd_decompress_block.c`).
    #[test]
    #[cfg(unix)]
    fn tc11_repeat_offset_cli_interop_constant_byte() {
        // The exact Lesson 98 repro: 4713 bytes of a single repeated byte.
        // Real `zstd` picks a Compressed block (not RLE) with one sequence:
        // 2 literal bytes ("ZZ") + a match with Offset_Value=1 — "reuse
        // Repeated_Offset1", whose default value (1) happens to already be
        // the right distance for constant data, an unmistakable
        // RLE-via-repeat-offset pattern.
        require_zstd_cli();

        let original = vec![b'Z'; 4713];
        let theirs_input = write_temp_file("tc11-repoff-const-input", &original);
        let _guard = TempFileGuard(theirs_input.clone());
        let their_compressed =
            run_zstd_capture_stdout(&["-q", "-c", theirs_input.to_str().unwrap()]);

        let decoded_by_us = decompress(&their_compressed).expect(
            "our decompress() failed on real zstd's repeat-offset output — \
             this is Lesson 98's gap: offset codes 1-3 must be interpreted as \
             Repeated_Offset (R1/R2/R3) references, not as of_raw - 3",
        );
        assert_eq!(
            decoded_by_us, original,
            "our decompress() decoded real zstd's repeat-offset output to the \
             wrong bytes"
        );
    }

    #[test]
    #[cfg(unix)]
    fn tc11_repeat_offset_cli_interop_periodic() {
        // A periodic pattern at a FIXED distance is the other classic
        // repeat-offset trigger: after the first match establishes the
        // distance, every subsequent match at that same distance is cheaper
        // to encode as "reuse R1" than as a fresh explicit offset. Real
        // `zstd`'s encoder is very likely to do exactly that here.
        require_zstd_cli();

        let pattern = b"ABCDEFGHIJ0123456789";
        let mut original = Vec::new();
        for _ in 0..300 {
            original.extend_from_slice(pattern);
        }
        let theirs_input = write_temp_file("tc11-repoff-periodic-input", &original);
        let _guard = TempFileGuard(theirs_input.clone());
        let their_compressed =
            run_zstd_capture_stdout(&["-q", "-c", theirs_input.to_str().unwrap()]);

        let decoded_by_us = decompress(&their_compressed)
            .expect("our decompress() failed on real zstd's periodic-pattern output");
        assert_eq!(
            decoded_by_us, original,
            "our decompress() decoded real zstd's periodic-pattern output to \
             the wrong bytes"
        );
    }

    #[test]
    #[cfg(unix)]
    fn rt_cli_interop_high_sequence_count() {
        // Real `zstd` CLI interop on an input large enough to push our
        // compressor's single-block sequence count past 128 — the exact
        // boundary where the sequence-count wire encoding switches from its
        // 1-byte form to its 2-byte form (RFC 8878 §3.1.1.3.1). A
        // marker-byte-order bug in that 2-byte form (found and fixed
        // alongside the FSE bugs during this same audit — see the doc
        // comment on `encode_seq_count`) round-trips fine against ITSELF
        // but silently produces a non-conformant frame, so only a real
        // cross-implementation check like this one can catch it. Not one of
        // the spec's numbered TCs; extra regression coverage for the fix.
        require_zstd_cli();

        // A repeating 6-byte cycle across 9 KB gives LZSS plenty of short,
        // distinct matches — comfortably more than 128 sequences in one
        // block, while staying well under the 128 KB block cap.
        let src = b"ABCDEF";
        let original: Vec<u8> = src.iter().cloned().cycle().take(9000).collect();

        let our_compressed = compress(&original);
        let ours_zst = write_temp_file("rt-highseq", &our_compressed);
        let _guard = TempFileGuard(ours_zst.clone());
        let decoded_by_cli =
            run_zstd_capture_stdout(&["-d", "-q", "-c", ours_zst.to_str().unwrap()]);
        assert_eq!(
            decoded_by_cli, original,
            "real `zstd -d` failed to decode our high-sequence-count output \
             (likely a sequence-count wire-format regression)"
        );
    }

    // ── TC-10: manual minimal raw-block frame ─────────────────────────────────

    #[test]
    fn tc10_wire_format() {
        // Manually constructed ZStd frame to verify our decoder reads the
        // wire format correctly without depending on our encoder.
        //
        // Frame layout:
        //   [0..3]  Magic = 0xFD2FB528 LE = [0x28, 0xB5, 0x2F, 0xFD]
        //   [4]     FHD = 0x20:
        //             bits [7:6] = 00 → FCS flag 0
        //             bit  [5]   = 1  → Single_Segment = 1
        //             bits [4:0] = 0  → no checksum, no dict
        //           With Single_Segment=1 and FCS_flag=00, FCS is 1 byte.
        //   [5]     FCS = 0x05 (content_size = 5)
        //   [6..8]  Block header: Last=1, Type=Raw, Size=5
        //             = (5 << 3) | (0 << 1) | 1 = 41 = 0x29
        //             = [0x29, 0x00, 0x00]
        //   [9..13] b"hello"
        let frame = [
            0x28u8, 0xB5, 0x2F, 0xFD, // magic
            0x20,                       // FHD: Single_Segment=1, FCS=1byte
            0x05,                       // FCS = 5
            0x29, 0x00, 0x00,           // block header: last=1, raw, size=5
            b'h', b'e', b'l', b'l', b'o',
        ];
        assert_eq!(decompress(&frame).unwrap(), b"hello");
    }

    // ── Additional round-trip tests ───────────────────────────────────────────

    #[test]
    fn rt_binary_data() {
        // Binary data with lots of zeros and 0xFF bytes.
        let input: Vec<u8> = (0..300).map(|i| (i % 256) as u8).collect();
        assert_eq!(rt(&input), input);
    }

    #[test]
    fn rt_all_zeros() {
        let input = vec![0u8; 1000];
        assert_eq!(rt(&input), input);
    }

    #[test]
    fn rt_all_ff() {
        let input = vec![0xFFu8; 1000];
        assert_eq!(rt(&input), input);
    }

    #[test]
    fn rt_hello_world() {
        assert_eq!(rt(b"hello world"), b"hello world");
    }

    #[test]
    fn rt_repeated_pattern() {
        let data: Vec<u8> = b"ABCDEF".iter().cloned().cycle().take(3000).collect();
        assert_eq!(rt(&data), data);
    }

    // ── Unit tests for internal helpers ───────────────────────────────────────

    #[test]
    fn test_ll_to_code_small() {
        for i in 0usize..16 {
            assert_eq!(ll_to_code(i as u32), i, "LL code for {i}");
        }
    }

    #[test]
    fn test_ml_to_code_small() {
        for i in 3usize..35 {
            assert_eq!(ml_to_code(i as u32), i - 3, "ML code for {i}");
        }
    }

    #[test]
    fn test_literals_section_roundtrip_short() {
        let lits: Vec<u8> = (0..20).map(|i| i as u8).collect();
        let encoded = encode_literals_section(&lits);
        let (decoded, _) = decode_literals_section(&encoded, &mut None).unwrap();
        assert_eq!(decoded, lits);
    }

    #[test]
    fn test_literals_section_roundtrip_medium() {
        let lits: Vec<u8> = (0..200).map(|i| (i % 256) as u8).collect();
        let encoded = encode_literals_section(&lits);
        let (decoded, _) = decode_literals_section(&encoded, &mut None).unwrap();
        assert_eq!(decoded, lits);
    }

    #[test]
    fn test_literals_section_roundtrip_large() {
        let lits: Vec<u8> = (0..5000).map(|i| (i % 256) as u8).collect();
        let encoded = encode_literals_section(&lits);
        let (decoded, _) = decode_literals_section(&encoded, &mut None).unwrap();
        assert_eq!(decoded, lits);
    }

    #[test]
    fn test_revbitwriter_revbitreader_roundtrip() {
        // The backward bit stream stores bits so the LAST-written bits are
        // read FIRST by the decoder. This mirrors how ZStd's sequence codec
        // writes the initial FSE states last (so the decoder reads them first).
        //
        // Write order:  A=0b101 (3 bits), B=0b11001100 (8 bits), C=0b1 (1 bit)
        // Read order:   C first, then B, then A  (reversed)
        let mut bw = RevBitWriter::new();
        bw.add_bits(0b101, 3);      // A — written first → read last
        bw.add_bits(0b11001100, 8); // B
        bw.add_bits(0b1, 1);        // C — written last → read first
        bw.flush();
        let buf = bw.finish();

        let mut br = RevBitReader::new(&buf).unwrap();
        assert_eq!(br.read_bits(1), 0b1,        "C: last written, first read");
        assert_eq!(br.read_bits(8), 0b11001100, "B");
        assert_eq!(br.read_bits(3), 0b101,      "A: first written, last read");
    }

    #[test]
    fn test_fse_decode_table_coverage() {
        // Every slot in the decode table should be reachable (sym is valid).
        let dt = build_decode_table(&LL_NORM, LL_ACC_LOG);
        assert_eq!(dt.len(), 1 << LL_ACC_LOG);
        for cell in &dt {
            assert!((cell.sym as usize) < LL_NORM.len());
        }
    }

    #[test]
    fn test_seq_count_roundtrip() {
        for &n in &[0usize, 1, 50, 127, 128, 1000, 0x7FFE] {
            let enc = encode_seq_count(n);
            let (dec, _) = decode_seq_count(&enc).unwrap();
            assert_eq!(dec, n, "seq count {n}");
        }
    }

    /// Low-level decode of a sequences-section bitstream, mirroring the
    /// corrected per-sequence loop in `decompress_block` exactly (peek all
    /// three symbols first, read extras in order OF/ML/LL, then update
    /// states in order LL/ML/OF — skipped for the last sequence). Used by
    /// the isolated FSE-codec unit tests below so they exercise the SAME
    /// order `decompress_block` uses, rather than a hand-rolled convention
    /// that could silently drift from it (exactly the trap described in
    /// lessons.md Lesson 96: an isolated low-level test that agrees with
    /// itself proves nothing about wire conformance — only the CLI interop
    /// tests below do that).
    fn decode_seqs_for_test(bitstream: &[u8], n_seqs: usize) -> Vec<Seq> {
        let dt_ll = FseTable::from_norm(&LL_NORM, LL_ACC_LOG).unwrap();
        let dt_ml = FseTable::from_norm(&ML_NORM, ML_ACC_LOG).unwrap();
        let dt_of = FseTable::from_norm(&OF_NORM, OF_ACC_LOG).unwrap();

        let mut br = RevBitReader::new(bitstream).unwrap();
        let mut state_ll = br.read_bits(LL_ACC_LOG) as u16;
        let mut state_of = br.read_bits(OF_ACC_LOG) as u16;
        let mut state_ml = br.read_bits(ML_ACC_LOG) as u16;

        let mut out = Vec::with_capacity(n_seqs);
        for i in 0..n_seqs {
            let ll_entry = fse_cell(state_ll, &dt_ll, "LL").unwrap();
            let ml_entry = fse_cell(state_ml, &dt_ml, "ML").unwrap();
            let of_entry = fse_cell(state_of, &dt_of, "OF").unwrap();

            let ll_info = LL_CODES[ll_entry.sym as usize];
            let ml_info = ML_CODES[ml_entry.sym as usize];

            let of_raw = (1u32 << of_entry.sym) | br.read_bits(of_entry.sym) as u32;
            let ml = ml_info.0 + br.read_bits(ml_info.1) as u32;
            let ll = ll_info.0 + br.read_bits(ll_info.1) as u32;
            let off = of_raw - 3;

            if i != n_seqs - 1 {
                state_ll = fse_update_state(ll_entry, &mut br);
                state_ml = fse_update_state(ml_entry, &mut br);
                state_of = fse_update_state(of_entry, &mut br);
            }

            out.push(Seq { ll, ml, off });
        }
        out
    }

    #[test]
    fn test_fse_two_sequence_roundtrip() {
        // Test encoding and decoding two sequences to verify FSE state
        // transitions, including the last-sequence update-skip (Lesson 96).
        let seqs = vec![
            Seq { ll: 2, ml: 4, off: 1 },
            Seq { ll: 0, ml: 3, off: 2 },
        ];
        let bitstream = encode_sequences_section(&seqs);
        let decoded = decode_seqs_for_test(&bitstream, seqs.len());

        for (i, (expected, actual)) in seqs.iter().zip(decoded.iter()).enumerate() {
            assert_eq!(actual.ll, expected.ll, "seq {i} LL");
            assert_eq!(actual.ml, expected.ml, "seq {i} ML");
            assert_eq!(actual.off, expected.off, "seq {i} OFF");
        }
    }

    #[test]
    fn test_fse_single_sequence_roundtrip() {
        // Encode a single sequence and verify that decoding it gives back
        // the exact same (ll, ml, of) values. This isolates the FSE codec,
        // including the direct `fse_init_state` path (there is no
        // "previous" iteration for a single sequence, so this is the ONLY
        // sequence and must use init, not a transition — Lesson 96).
        let seqs = [Seq { ll: 3, ml: 5, off: 2 }];
        let bitstream = encode_sequences_section(&seqs);
        let decoded = decode_seqs_for_test(&bitstream, seqs.len());

        assert_eq!(decoded[0].ll, 3, "LL");
        assert_eq!(decoded[0].ml, 5, "ML");
        assert_eq!(decoded[0].off, 2, "OFF");
    }

    #[test]
    fn test_fse_many_sequence_roundtrip() {
        // A longer sequence list exercises multiple non-last transitions
        // (fse_encode_sym / fse_update_state) in addition to the single
        // fse_init_state / last-sequence-skip case covered above.
        let seqs = vec![
            Seq { ll: 1, ml: 3, off: 1 },
            Seq { ll: 5, ml: 10, off: 4 },
            Seq { ll: 0, ml: 6, off: 2 },
            Seq { ll: 12, ml: 40, off: 100 },
            Seq { ll: 3, ml: 3, off: 1 },
        ];
        let bitstream = encode_sequences_section(&seqs);
        let decoded = decode_seqs_for_test(&bitstream, seqs.len());

        for (i, (expected, actual)) in seqs.iter().zip(decoded.iter()).enumerate() {
            assert_eq!(actual.ll, expected.ll, "seq {i} LL");
            assert_eq!(actual.ml, expected.ml, "seq {i} ML");
            assert_eq!(actual.off, expected.off, "seq {i} OFF");
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Corpus generators shared by the CLI-interop and golden-vector suites
    // ═══════════════════════════════════════════════════════════════════════
    //
    // These are deterministic so that a golden vector needs only the
    // COMPRESSED bytes checked in: the plaintext is regenerated here and
    // compared against what the decoder produced. Checking in a 240 KB
    // plaintext alongside its 6 KB compressed form would be pure weight, and
    // would let the two drift apart.
    //
    // Each generator targets a different part of the format:
    //
    // | Generator        | What it forces real `zstd` to emit                |
    // |------------------|---------------------------------------------------|
    // | `corpus_prose`   | Huffman literals over a full-ish byte alphabet,    |
    // |                  | FSE-compressed weights, FSE sequence tables        |
    // | `corpus_skewed`  | few distinct bytes → DIRECT (4-bit) weights        |
    // | `corpus_random`  | incompressible → Raw literals / Raw blocks         |
    //
    // and passing >128 KB of any of them additionally forces multi-block
    // frames, which is where Treeless literals and Repeat_Mode tables appear.

    /// Word-salad text: a 32-bit LCG picks words from a fixed list.
    ///
    /// English-like enough that `zstd` finds both matches and a skewed
    /// literal distribution — the combination that makes Huffman coding
    /// worthwhile, which is exactly the path this crate could not decode.
    fn corpus_prose(n: usize) -> Vec<u8> {
        const WORDS: [&str; 16] = [
            "the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog",
            "while", "zstd", "huffman", "fse", "entropy", "coder", "builds",
            "tables",
        ];
        let mut x: u64 = 12345;
        let mut out = Vec::with_capacity(n + 16);
        while out.len() < n {
            x = (1103515245u64.wrapping_mul(x).wrapping_add(12345)) & 0x7FFF_FFFF;
            out.extend_from_slice(WORDS[(x as usize) % WORDS.len()].as_bytes());
            out.push(b' ');
            if x.is_multiple_of(11) {
                out.push(b'\n');
            }
        }
        out.truncate(n);
        out
    }

    /// `n` bytes drawn from the alphabet `0..k` with a geometric-ish skew
    /// (symbol 0 about half the time, symbol 1 about a quarter, ...).
    ///
    /// A small, sharply skewed alphabet is what pushes `zstd` into the
    /// DIRECT Huffman weight representation: with a handful of symbols the
    /// FSE table description for the weights would cost more than the
    /// weights themselves.
    fn corpus_skewed(n: usize, k: u8) -> Vec<u8> {
        let mut x: u64 = 987654321;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            x = (1103515245u64.wrapping_mul(x).wrapping_add(12345)) & 0x7FFF_FFFF;
            let mut v = ((x >> 7) % 100) as i32;
            let mut w = 50i32;
            let mut i = 0u8;
            while i + 1 < k && v >= w {
                v -= w;
                w = (w / 2).max(1);
                i += 1;
            }
            out.push(i);
        }
        out
    }

    /// Incompressible bytes from a xorshift32 generator.
    fn corpus_random(n: usize) -> Vec<u8> {
        let mut x: u32 = 2463534242;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            out.push((x >> 8) as u8);
        }
        out
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Live CLI interop over a corpus chosen to force the new code paths
    // ═══════════════════════════════════════════════════════════════════════

    /// The corpus, as `(name, plaintext)` pairs.
    ///
    /// Sizes straddle the 128 KB block boundary on purpose: a single-block
    /// frame can never exercise Treeless literals or Repeat_Mode tables,
    /// because both mean "same as the previous block".
    fn interop_corpus() -> Vec<(&'static str, Vec<u8>)> {
        vec![
            ("prose-4k", corpus_prose(4_000)),
            ("prose-20k", corpus_prose(20_000)),
            ("prose-240k", corpus_prose(240_000)),
            ("skewed-3sym-6k", corpus_skewed(6_000, 3)),
            ("skewed-6sym-8k", corpus_skewed(8_000, 6)),
            ("skewed-5sym-300k", corpus_skewed(300_000, 5)),
            ("random-2k", corpus_random(2_000)),
            ("all-bytes", (0u8..=255).cycle().take(9_000).collect()),
        ]
    }

    /// Compress every corpus entry with the real `zstd` CLI at several
    /// levels and require our decoder to reproduce the plaintext EXACTLY.
    ///
    /// The levels are not decoration. They pick different internal
    /// strategies, and those strategies emit structurally different frames:
    /// `-1` favours cheap literal handling and predefined tables, `-3` is
    /// the default balance, and `-19` searches hard enough to produce long
    /// matches, RLE literal blocks and Repeat_Mode tables. A decoder can be
    /// perfectly correct at one level and broken at another.
    ///
    /// This is the test that would have caught the entire gap this suite was
    /// written for: before Huffman/FSE-description support, every single one
    /// of these cases failed with "unsupported literals type 2".
    #[test]
    #[cfg(unix)]
    fn cli_interop_corpus_forces_huffman_and_fse_tables() {
        require_zstd_cli();

        for (name, original) in interop_corpus() {
            let input = write_temp_file(&format!("corpus-{name}"), &original);
            let _guard = TempFileGuard(input.clone());

            for level in ["-1", "-3", "-19"] {
                let compressed =
                    run_zstd_capture_stdout(&[level, "-q", "-c", input.to_str().unwrap()]);
                let decoded = decompress(&compressed).unwrap_or_else(|e| {
                    panic!("decompress() failed on real zstd output ({name} at {level}): {e}")
                });
                assert_eq!(
                    decoded.len(),
                    original.len(),
                    "length mismatch decoding real zstd output ({name} at {level})"
                );
                assert!(
                    decoded == original,
                    "byte mismatch decoding real zstd output ({name} at {level})"
                );
            }
        }
    }

    /// Both directions, on inputs large enough to be multi-block: our output
    /// must still satisfy the real CLI, and its output must still satisfy us.
    ///
    /// Guards against a one-sided fix — it would be easy to make the decoder
    /// accept everything while quietly breaking what the encoder emits.
    #[test]
    #[cfg(unix)]
    fn cli_interop_both_directions_multiblock() {
        require_zstd_cli();

        let original = corpus_prose(300_000);
        let ours = compress(&original);
        let path = write_temp_file("bidir-ours", &ours);
        let _guard = TempFileGuard(path.clone());
        let by_cli = run_zstd_capture_stdout(&["-d", "-q", "-c", path.to_str().unwrap()]);
        assert!(by_cli == original, "real `zstd -d` mis-decoded our multi-block frame");
    }

    /// Frames with NO `Frame_Content_Size`, produced by streaming through
    /// `zstd`'s stdin.
    ///
    /// The decoder must take its output size purely from the blocks, and
    /// must skip the `Window_Descriptor` byte that only appears when
    /// `Single_Segment_Flag` is 0. Getting that byte count wrong shifts
    /// every subsequent field by one and produces a "reserved block type"
    /// error — or, worse, a plausible-looking wrong answer.
    #[test]
    #[cfg(unix)]
    fn cli_interop_streaming_frames_without_content_size() {
        require_zstd_cli();

        for (name, original) in interop_corpus() {
            for level in ["-1", "-3", "-19"] {
                let compressed = run_zstd_stdin(&[level, "-q", "-c"], &original);
                // Sanity-check that this really is the header shape we mean
                // to be testing, so the test cannot quietly stop covering it
                // if a future `zstd` starts buffering small inputs.
                let single_segment = (compressed[4] >> 5) & 1;
                assert_eq!(
                    single_segment, 0,
                    "expected a streaming (multi-segment) frame for {name} at {level}"
                );
                let decoded = decompress(&compressed).unwrap_or_else(|e| {
                    panic!("decompress() failed on a streaming frame ({name} at {level}): {e}")
                });
                assert!(
                    decoded == original,
                    "byte mismatch on a streaming frame ({name} at {level})"
                );
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Golden vectors — real CLI output, committed as bytes
    // ═══════════════════════════════════════════════════════════════════════
    //
    // `include_bytes!` embeds these at COMPILE time, so the suite runs
    // identically on a machine with no `zstd` binary, no network and no
    // writable temp directory — including the Windows CI leg, where the live
    // interop tests above are not compiled at all.
    //
    // Every one of these files was produced by `zstd` CLI v1.5.7 from the
    // deterministic generators above; `regenerate_golden_vectors` below
    // recreates them byte-for-byte on demand, and documents the exact
    // command line for each.
    //
    // Their job is to pin the decoder against frames THIS crate cannot
    // produce. The encoder here only ever emits Raw literals and predefined
    // FSE tables, so no amount of self-round-tripping can exercise Huffman
    // literals, FSE table descriptions, Treeless blocks or Repeat_Mode — the
    // exact blind spot that let three earlier wire-format bugs survive
    // (lessons.md Lessons 95/96/98).

    /// `(vector bytes, expected plaintext, what the frame exercises)`.
    #[allow(clippy::type_complexity)]
    fn golden_vectors() -> Vec<(&'static [u8], Vec<u8>, &'static str)> {
        vec![
            (
                include_bytes!("../tests/vectors/prose-4k-level1.zst"),
                corpus_prose(4_000),
                "Compressed_Literals_Block, 1 stream, FSE-coded Huffman weights; \
                 all three sequence tables FSE_Compressed",
            ),
            (
                include_bytes!("../tests/vectors/prose-4k-level19.zst"),
                corpus_prose(4_000),
                "Compressed_Literals_Block, 1 stream, FSE-coded Huffman weights; \
                 match-length table falls back to Predefined_Mode",
            ),
            (
                include_bytes!("../tests/vectors/skewed-6sym-8k-level1.zst"),
                corpus_skewed(8_000, 6),
                "Compressed_Literals_Block, 4 streams (jump table), DIRECT 4-bit \
                 Huffman weights",
            ),
            (
                include_bytes!("../tests/vectors/skewed-3sym-6k-level19.zst"),
                corpus_skewed(6_000, 3),
                "Compressed_Literals_Block, DIRECT weights, size format 2 (14-bit \
                 sizes); all three sequence tables Predefined_Mode",
            ),
            (
                include_bytes!("../tests/vectors/prose-240k-level15.zst"),
                corpus_prose(240_000),
                "multi-block: RLE_Literals_Block, Treeless_Literals_Block reusing an \
                 earlier block's Huffman tree, and Repeat_Mode sequence tables",
            ),
            (
                include_bytes!("../tests/vectors/random-2k-level1.zst"),
                corpus_random(2_000),
                "incompressible input: Raw block, no literals section at all",
            ),
            (
                include_bytes!("../tests/vectors/prose-20k-level3-streamed.zst"),
                corpus_prose(20_000),
                "streamed frame: no Frame_Content_Size, Window_Descriptor present \
                 (the shape every streaming library emits, including Anki's)",
            ),
        ]
    }

    #[test]
    fn golden_vectors_decode_exactly() {
        for (i, (compressed, expected, what)) in golden_vectors().into_iter().enumerate() {
            let decoded = decompress(compressed)
                .unwrap_or_else(|e| panic!("golden vector {i} ({what}) failed to decode: {e}"));
            assert_eq!(
                decoded.len(),
                expected.len(),
                "golden vector {i} ({what}): length mismatch"
            );
            assert!(decoded == expected, "golden vector {i} ({what}): byte mismatch");
        }
    }

    /// Rewrites `tests/vectors/*.zst` from the real CLI.
    ///
    /// Ignored by default — it shells out and writes into the source tree.
    /// Run it deliberately, and only when a vector needs to change:
    ///
    /// ```text
    /// cargo test -p zstd -- --ignored regenerate_golden_vectors
    /// ```
    ///
    /// Keeping the recipe as executable code rather than prose is the point:
    /// a checked-in binary fixture nobody can reproduce is a fixture nobody
    /// can audit.
    #[test]
    #[ignore = "writes into the source tree; run explicitly when a vector changes"]
    #[cfg(unix)]
    fn regenerate_golden_vectors() {
        require_zstd_cli();

        let specs: Vec<(&str, Vec<u8>, &str)> = vec![
            ("prose-4k-level1", corpus_prose(4_000), "-1"),
            ("prose-4k-level19", corpus_prose(4_000), "-19"),
            ("skewed-6sym-8k-level1", corpus_skewed(8_000, 6), "-1"),
            ("skewed-3sym-6k-level19", corpus_skewed(6_000, 3), "-19"),
            ("prose-240k-level15", corpus_prose(240_000), "-15"),
            ("random-2k-level1", corpus_random(2_000), "-1"),
        ];

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors");
        std::fs::create_dir_all(&dir).expect("create tests/vectors");

        for (name, data, level) in specs {
            let input = write_temp_file(&format!("regen-{name}"), &data);
            let _guard = TempFileGuard(input.clone());
            let compressed =
                run_zstd_capture_stdout(&[level, "-q", "-c", input.to_str().unwrap()]);
            let out = dir.join(format!("{name}.zst"));
            std::fs::write(&out, &compressed).expect("write vector");
            eprintln!("wrote {} ({} bytes, `zstd {level} -c FILE`)", out.display(), compressed.len());
        }

        // The streamed vector comes from `zstd` reading STDIN, which is what
        // makes it omit Frame_Content_Size — it cannot be produced by the
        // file-based command line above.
        let streamed = run_zstd_stdin(&["-3", "-q", "-c"], &corpus_prose(20_000));
        let out = dir.join("prose-20k-level3-streamed.zst");
        std::fs::write(&out, &streamed).expect("write vector");
        eprintln!("wrote {} ({} bytes, `zstd -3 -c < FILE`)", out.display(), streamed.len());
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Unit tests for the new table-description decoders
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn fwd_bit_reader_reads_little_endian_from_the_bottom() {
        // 0xA7 0x03 as a little-endian integer is 0b11_1010_0111; reads peel
        // bits off the BOTTOM, which is the opposite end from RevBitReader.
        let data = [0xA7u8, 0x03];
        let mut br = FwdBitReader::new(&data);
        assert_eq!(br.read(4), 0b0111);
        assert_eq!(br.read(4), 0b1010);
        assert_eq!(br.read(2), 0b11);
        // 6 more bits were consumed than the 10 that carry data; the header
        // still owns both whole bytes.
        assert_eq!(br.finish().unwrap(), 2);
    }

    #[test]
    fn fwd_bit_reader_rejects_reading_past_the_end() {
        let data = [0xFFu8];
        let mut br = FwdBitReader::new(&data);
        br.skip(9); // one bit past the single byte
        assert!(br.finish().is_err(), "over-read must be reported, not zero-filled silently");
    }

    #[test]
    fn fse_table_description_roundtrips_the_predefined_offset_table() {
        // Hand-encode the predefined OFFSET distribution as an RFC 8878
        // §4.1.1 table description, then read it back. This exercises the
        // variable-width count field AND the "-1" (probability below one)
        // encoding, which the offset table is full of.
        //
        // The encoder is the minimum needed to produce a legal
        // description; the point of the test is that the DECODER recovers
        // the exact distribution, both field widths included.
        let acc_log = OF_ACC_LOG;
        let mut bits: Vec<(u32, u32)> = vec![((acc_log - 5) as u32, 4)];
        let table_size = 1i32 << acc_log;
        let mut remaining = table_size + 1;
        let mut threshold = table_size;
        let mut nbits = acc_log as u32 + 1;
        for &c in OF_NORM.iter() {
            // Mirrors `FSE_writeNCount_generic`: transmit count+1, bias by
            // `max` once the value reaches `threshold`, and spend one fewer
            // bit whenever the result lands below `max` (which is exactly
            // when the decoder's short-form peek is unambiguous).
            let max = (2 * threshold - 1) - remaining;
            let mut value = (c as i32) + 1;
            if value >= threshold {
                value += max;
            }
            let width = if value < max { nbits - 1 } else { nbits };
            bits.push((value as u32, width));
            remaining -= (c as i32).unsigned_abs() as i32;
            if remaining < threshold {
                if remaining <= 1 {
                    break;
                }
                nbits = 32 - (remaining as u32).leading_zeros();
                threshold = 1 << (nbits - 1);
            }
        }

        // Pack forward, low bit first.
        let mut bytes = Vec::new();
        let mut acc: u64 = 0;
        let mut n = 0u32;
        for (v, w) in bits {
            acc |= (v as u64) << n;
            n += w;
            while n >= 8 {
                bytes.push(acc as u8);
                acc >>= 8;
                n -= 8;
            }
        }
        if n > 0 {
            bytes.push(acc as u8);
        }

        let (norm, log, _used) = read_fse_table_description(&bytes, 8, 31)
            .expect("hand-built offset table description must parse");
        assert_eq!(log, OF_ACC_LOG);
        assert_eq!(&norm[..], &OF_NORM[..]);
        // And it must build into a usable table.
        let tbl = FseTable::from_norm(&norm, log).expect("valid distribution");
        assert_eq!(tbl.de.len(), 1 << OF_ACC_LOG);
    }

    #[test]
    fn fse_table_description_rejects_oversized_accuracy_log() {
        // Low nibble 15 → accuracy_log 20, far past the 9 the sequence
        // tables allow. Accepting it would allocate a 1 M-entry table from
        // two bytes of attacker-controlled input.
        let data = [0x0Fu8, 0x00, 0x00, 0x00];
        let err = read_fse_table_description(&data, 9, 35).unwrap_err();
        assert!(err.contains("accuracy_log"), "unexpected error: {err}");
    }

    #[test]
    fn fse_table_build_rejects_distributions_that_do_not_sum_to_the_table() {
        // One slot short.
        assert!(FseTable::from_norm(&[2, 1], 2).is_err());
        // One slot over.
        assert!(FseTable::from_norm(&[3, 2], 2).is_err());
        // Counts below -1 are not a thing.
        assert!(FseTable::from_norm(&[-2, 6], 2).is_err());
        // Exact fit is accepted, including the "-1" one-slot symbols.
        assert!(FseTable::from_norm(&[2, 1, -1], 2).is_ok());
    }

    #[test]
    fn huffman_direct_weights_build_a_complete_code() {
        // Header 129 = 127 + 2, i.e. two transmitted weights, packed high
        // nibble first.
        // Weights [1, 1] sum to 2^1, so max_bits is 2 and the deduced third
        // weight is 2 — giving symbol 2 a 1-bit code and symbols 0 and 1
        // 2-bit codes. Kraft: 1/2 + 1/4 + 1/4 = 1.
        let desc = [129u8, 0x11];
        let (tbl, used) = read_huffman_table(&desc).expect("valid direct description");
        assert_eq!(used, 2);
        assert_eq!(tbl.max_bits, 2);
        assert_eq!(tbl.entries.len(), 4);
        // Longest codes come first in ZStd's canonical order: symbols 0 and 1
        // (weight 1, 2 bits) occupy cells 0 and 1; symbol 2 (weight 2, 1 bit)
        // occupies cells 2 and 3.
        assert_eq!((tbl.entries[0].sym, tbl.entries[0].nb), (0, 2));
        assert_eq!((tbl.entries[1].sym, tbl.entries[1].nb), (1, 2));
        assert_eq!((tbl.entries[2].sym, tbl.entries[2].nb), (2, 1));
        assert_eq!((tbl.entries[3].sym, tbl.entries[3].nb), (2, 1));
    }

    #[test]
    fn huffman_description_rejects_an_incomplete_code_space() {
        // Weights [1, 1, 1] sum to 3; the shortfall to the next power of two
        // is 1, which IS a power of two, so this one is legal (last weight 1,
        // four symbols of 2 bits each).
        assert!(read_huffman_table(&[130u8, 0x11, 0x10]).is_ok());
        // Weights [3, 1, 1] sum to 4+1+1 = 6; shortfall to 8 is 2 — legal.
        assert!(read_huffman_table(&[130u8, 0x31, 0x10]).is_ok());
        // Weights [3, 2, 1] sum to 4+2+1 = 7; shortfall to 8 is 1 — legal.
        assert!(read_huffman_table(&[130u8, 0x32, 0x10]).is_ok());
        // Weights [3, 3, 1] sum to 4+4+1 = 9; next power of two is 16, so the
        // shortfall is 7, which is NOT a power of two: the code cannot be
        // completed by a single symbol, so the description is corrupt.
        assert!(read_huffman_table(&[130u8, 0x33, 0x10]).is_err());
        // All-zero weights assign no code space at all.
        assert!(read_huffman_table(&[129u8, 0x00]).is_err());
        // A weight above the 12-bit ceiling.
        assert!(read_huffman_table(&[129u8, 0xF1]).is_err());
    }

    #[test]
    fn huffman_description_rejects_truncation() {
        assert!(read_huffman_table(&[]).is_err());
        // Direct form claiming 8 weights but carrying one byte.
        assert!(read_huffman_table(&[135u8, 0x11]).is_err());
        // FSE form claiming a 60-byte payload it does not have.
        assert!(read_huffman_table(&[60u8, 0x00]).is_err());
        // FSE form with a zero-size payload.
        assert!(read_huffman_table(&[0u8]).is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Adversarial input — every malformed frame must Err, never panic
    // ═══════════════════════════════════════════════════════════════════════
    //
    // This decoder is destined to read untrusted `.apkg`/`.colpkg` archives,
    // so a panic is a denial of service rather than a debugging aid — and on
    // `wasm32-unknown-unknown`, built with `panic = "abort"`, it is an
    // unrecoverable trap that no caller can catch. Every one of these cases
    // must come back as an `Err`.
    //
    // Note that the assertion is simply "the call returned". A panic inside
    // `decompress` fails the test by unwinding out of it; there is no need
    // to catch anything.

    #[test]
    fn malformed_truncations_never_panic() {
        for (i, (vector, _, _)) in golden_vectors().into_iter().enumerate() {
            // Truncating at every length exercises every parser boundary:
            // mid-frame-header, mid-block-header, mid-Huffman-description,
            // mid-jump-table, mid-bitstream.
            for len in 0..vector.len().min(400) {
                let _ = decompress(&vector[..len]);
            }
            // And a few truncations deep inside the payload.
            for cut in [vector.len() / 4, vector.len() / 2, vector.len() - 1] {
                let _ = decompress(&vector[..cut]);
            }
            assert!(decompress(vector).is_ok(), "vector {i} must still decode intact");
        }
    }

    #[test]
    fn malformed_byte_mutations_never_panic() {
        // Systematically corrupt the header/table region of the small
        // vectors — the bytes that steer table sizes, stream counts and
        // symbol counts, where a bad value does the most damage.
        let vectors: Vec<&[u8]> = vec![
            include_bytes!("../tests/vectors/prose-4k-level1.zst"),
            include_bytes!("../tests/vectors/prose-4k-level19.zst"),
            include_bytes!("../tests/vectors/skewed-3sym-6k-level19.zst"),
        ];
        for vector in vectors {
            for pos in 0..vector.len().min(96) {
                for value in [0x00u8, 0x01, 0x0F, 0x55, 0x80, 0xAA, 0xFE, 0xFF] {
                    let mut mutated = vector.to_vec();
                    mutated[pos] = value;
                    let _ = decompress(&mutated);
                }
            }
        }
    }

    #[test]
    fn malformed_hand_built_frames_are_rejected() {
        /// Wraps a compressed-block payload in a minimal single-segment
        /// frame so it reaches `decompress_block`.
        fn frame(block: &[u8]) -> Vec<u8> {
            let mut out = MAGIC.to_le_bytes().to_vec();
            out.push(0x20); // single segment, 1-byte FCS, no checksum
            out.push(0x00); // FCS = 0
            let hdr = ((block.len() as u32) << 3) | (0b10 << 1) | 1;
            out.extend_from_slice(&hdr.to_le_bytes()[..3]);
            out.extend_from_slice(block);
            out
        }

        // Literals header says "Compressed, 1 stream, regenerated 1023,
        // compressed 1023" but the block carries nothing.
        let mut block = vec![0b0000_0010u8, 0xFF, 0xFF];
        assert!(decompress(&frame(&block)).is_err());

        // Treeless literals with no preceding tree in the frame.
        // Header bits: type 3, size format 0, Regenerated_Size 6,
        // Compressed_Size 3 -> 0x00C063 little-endian.
        block = vec![0x63u8, 0xC0, 0x00, 0xAA, 0xBB, 0xCC];
        assert!(decompress(&frame(&block)).is_err());

        // Raw literals (0 bytes), 1 sequence, reserved bits set in the
        // Symbol_Compression_Modes byte.
        block = vec![0x00, 0x01, 0b0000_0011, 0x01];
        assert!(decompress(&frame(&block)).is_err());

        // Raw literals, 1 sequence, all three tables in Repeat_Mode with no
        // previous block to repeat.
        block = vec![0x00, 0x01, 0b1111_1100, 0x01];
        let err = decompress(&frame(&block)).unwrap_err();
        assert!(err.contains("Repeat_Mode"), "unexpected error: {err}");

        // Offset table in RLE mode with symbol 200 — far past the 31 that
        // offset codes allow, and a value that would shift a u32 into
        // oblivion if it reached the decode loop.
        block = vec![0x00, 0x01, 0b0001_0000, 200, 0x01];
        assert!(decompress(&frame(&block)).is_err());

        // An empty sequences bitstream (no sentinel byte to anchor the
        // backward reader).
        block = vec![0x00, 0x01, 0x00];
        assert!(decompress(&frame(&block)).is_err());

        // A sequences bitstream whose last byte is zero: no sentinel bit, so
        // the payload length is undefined.
        block = vec![0x00, 0x01, 0x00, 0x00];
        assert!(decompress(&frame(&block)).is_err());
    }

    #[test]
    fn dictionary_frames_are_refused_by_name() {
        // A frame carrying a non-zero Dictionary_ID cannot be decoded
        // without that dictionary, because the dictionary pre-seeds the
        // match history AND all four entropy tables — so such a frame may
        // legitimately open with Repeat_Mode tables or a Treeless literals
        // block. Pressing on regardless produced a baffling "offset table
        // uses Repeat_Mode but no previous table exists in this frame", or
        // on a luckier frame, silently wrong bytes.
        //
        // Header: magic, FHD 0x03 (Single_Segment_Flag clear so a
        // Window_Descriptor follows; FCS_Field_Size 0 so no content size;
        // Dict_ID_Flag 3 = a 4-byte Dictionary_ID), then the descriptor and
        // the dictionary ID, in that wire order.
        let mut frame = MAGIC.to_le_bytes().to_vec();
        frame.push(0x03);
        frame.push(0x40); // Window_Descriptor
        frame.extend_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        frame.extend_from_slice(&[0x01, 0x00, 0x00]); // an empty last raw block
        let err = decompress(&frame).unwrap_err();
        assert!(
            err.contains("dictionary") && err.contains("3735928559"),
            "a dictionary frame must be refused by name, got: {err}"
        );

        // A Dictionary_ID field that is PRESENT but zero means "no
        // dictionary" and must still decode.
        let mut frame = MAGIC.to_le_bytes().to_vec();
        frame.push(0x21); // single segment, 1-byte FCS, 1-byte Dict_ID
        frame.push(0x00); // Dictionary_ID = 0
        frame.push(0x00); // Frame_Content_Size = 0
        frame.extend_from_slice(&[0x01, 0x00, 0x00]); // empty last raw block
        assert_eq!(decompress(&frame).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn degenerate_fse_and_huffman_streams_terminate() {
        // A 2-state interleaved weight stream over a table whose cells all
        // read zero bits would never advance the bit budget. The RLE table
        // is exactly that shape, so this pins the `max_out` guard that keeps
        // `fse_decompress_interleaved2` from looping forever.
        let tbl = FseTable::rle(7);
        let err = fse_decompress_interleaved2(&tbl, &[0x02], 16)
            .expect_err("a zero-bit table must hit the output ceiling, not spin");
        assert!(err.contains("exceeds"), "unexpected error: {err}");
    }

    #[test]
    fn oversized_rle_literals_are_capped() {
        // RLE literals with a 20-bit size field can claim up to 1 MB from a
        // single payload byte. Anything past the 128 KB block ceiling is
        // corrupt by definition, and rejecting it caps the amplification a
        // malicious frame can achieve per byte of input.
        let mut huff = None;
        let header = [0b0000_1101u8, 0xFF, 0xFF, b'x']; // type 1, format 3, size 0xFFFFF
        assert!(decode_literals_section(&header, &mut huff).is_err());

        // A legal size does work, and expands the single byte.
        let ok = [0b0000_0101u8, 0x0A, b'q']; // type 1, format 1, size 160
        let (lits, used) = decode_literals_section(&ok, &mut huff).unwrap();
        assert_eq!(used, 3);
        assert_eq!(lits.len(), 160);
        assert!(lits.iter().all(|&b| b == b'q'));
    }
}
