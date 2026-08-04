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
        if self.bits < 24 {
            self.reload();
        }
        val
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

/// Peek the symbol encoded at the current FSE decode state, WITHOUT
/// consuming any bits.
///
/// The FSE state itself IS the decode-table index — `de[state]` is a bare
/// table lookup that costs nothing. Only the subsequent state UPDATE
/// ([`fse_update_state`]) reads bits. RFC 8878 §3.1.1.3.2.1.2 requires all
/// three symbols (LL, ML, OF) to be peeked from their CURRENT states before
/// any extra bits or state updates are read — see [`decompress_block`].
fn fse_peek(state: u16, de: &[FseDe]) -> FseDe {
    de[state as usize]
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
    entry.base + br.read_bits(entry.nb) as u16
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

/// Decode literals section, returning (literals, bytes_consumed).
fn decode_literals_section(data: &[u8]) -> Result<(Vec<u8>, usize), String> {
    if data.is_empty() {
        return Err("empty literals section".into());
    }

    let b0 = data[0];
    let ltype = b0 & 0b11; // bottom 2 bits = Literals_Block_Type

    if ltype != 0 {
        // Only Raw_Literals (type=0) is implemented in this crate.
        // Huffman-coded literals (type=2,3) are not emitted by our encoder,
        // so if we see them here it means the input came from another encoder.
        return Err(format!("unsupported literals type {ltype} (only Raw=0 supported)"));
    }

    // Decode size_format from bits [3:2] of b0
    let size_format = (b0 >> 2) & 0b11;

    // Decode the literal length and header byte count from size_format.
    //
    // Raw_Literals size_format encoding (RFC 8878 §3.1.1.2.1):
    //   0b00 or 0b10 → 1-byte header: size = b0[7:3] (5 bits, values 0..31)
    //   0b01          → 2-byte LE header: size in bits [11:4] (12 bits, values 0..4095)
    //   0b11          → 3-byte LE header: size in bits [19:4] (20 bits, values 0..1MB)
    let (n, header_bytes) = match size_format {
        0 | 2 => {
            // 1-byte header: size in bits [7:3] (5 bits = values 0..31)
            let n = (b0 >> 3) as usize;
            (n, 1usize)
        }
        1 => {
            // 2-byte header: 12-bit size
            if data.len() < 2 {
                return Err("truncated literals header (2-byte)".into());
            }
            let n = ((b0 >> 4) as usize) | ((data[1] as usize) << 4);
            (n, 2usize)
        }
        3 => {
            // 3-byte header: 20-bit size (enough for blocks up to 1 MB)
            if data.len() < 3 {
                return Err("truncated literals header (3-byte)".into());
            }
            let n = ((b0 >> 4) as usize) | ((data[1] as usize) << 4) | ((data[2] as usize) << 12);
            (n, 3usize)
        }
        _ => unreachable!(), // size_format is 2 bits, all cases covered above
    };

    let start = header_bytes;
    let end = start + n;
    if end > data.len() {
        return Err(format!("literals data truncated: need {end}, have {}", data.len()));
    }

    Ok((data[start..end].to_vec(), end))
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

/// Decompress one ZStd compressed block.
///
/// Reads the literals section, sequences section, and applies the sequences
/// to the output buffer to reconstruct the original data.
fn decompress_block(data: &[u8], out: &mut Vec<u8>) -> Result<(), String> {
    // ── Literals section ─────────────────────────────────────────────────
    let (lits, lit_consumed) = decode_literals_section(data)?;
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

    // Check that all modes are Predefined (0).
    let ll_mode = (modes_byte >> 6) & 3;
    let of_mode = (modes_byte >> 4) & 3;
    let ml_mode = (modes_byte >> 2) & 3;
    if ll_mode != 0 || of_mode != 0 || ml_mode != 0 {
        return Err(format!(
            "unsupported FSE modes: LL={ll_mode} OF={of_mode} ML={ml_mode} (only Predefined=0 supported)"
        ));
    }

    // ── FSE bitstream ────────────────────────────────────────────────────
    let bitstream = &data[pos..];
    let mut br = RevBitReader::new(bitstream)?;

    // Build decode tables from predefined distributions.
    let dt_ll = build_decode_table(&LL_NORM, LL_ACC_LOG);
    let dt_ml = build_decode_table(&ML_NORM, ML_ACC_LOG);
    let dt_of = build_decode_table(&OF_NORM, OF_ACC_LOG);

    // Initialise FSE states from the bitstream. RFC 8878 §3.1.1.3.2.1.2: the
    // initial states are read in order LL, OF, ML (note: this is a
    // DIFFERENT order from the per-sequence symbol decode below, which is
    // OF, ML, LL for extras and LL, ML, OF for updates — the RFC is
    // asymmetric here; verified against the real `zstd` CLI, see Lesson 96).
    let mut state_ll = br.read_bits(LL_ACC_LOG) as u16;
    let mut state_of = br.read_bits(OF_ACC_LOG) as u16;
    let mut state_ml = br.read_bits(ML_ACC_LOG) as u16;

    // Track position in the literals buffer.
    let mut lit_pos = 0usize;

    // Apply each sequence.
    for i in 0..n_seqs {
        // Step 1 — PEEK symbols from the current states. This is a bare
        // table lookup (table[state].sym) and consumes NO bits — the FSE
        // state itself already IS the decode-table index. Only the
        // subsequent state UPDATE (step 3 below) reads bits.
        let ll_entry = fse_peek(state_ll, &dt_ll);
        let ml_entry = fse_peek(state_ml, &dt_ml);
        let of_entry = fse_peek(state_of, &dt_of);
        let ll_code = ll_entry.sym;
        let ml_code = ml_entry.sym;
        let of_code = of_entry.sym;

        if ll_code as usize >= LL_CODES.len() {
            return Err(format!("invalid LL code {ll_code}"));
        }
        if ml_code as usize >= ML_CODES.len() {
            return Err(format!("invalid ML code {ml_code}"));
        }
        let ll_info = LL_CODES[ll_code as usize];
        let ml_info = ML_CODES[ml_code as usize];

        // Step 2 — read the VALUE extra bits, order OF, ML, LL (RFC 8878
        // §3.1.1.3.2.1.2 — "Decoding starts by reading the Number_of_Bits
        // required to decode offset. It does the same for Match_Length and
        // then for Literals_Length.").
        // Offset: raw = (1 << of_code) | extra_bits; offset = raw - 3
        let of_raw = (1u32 << of_code) | br.read_bits(of_code) as u32;
        let ml = ml_info.0 + br.read_bits(ml_info.1) as u32;
        let ll = ll_info.0 + br.read_bits(ll_info.1) as u32;
        let offset = of_raw.checked_sub(3).ok_or_else(|| {
            format!("decoded offset underflow: of_raw={of_raw}")
        })?;

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
/// Accepts any valid ZStd frame with:
/// - Single-segment or multi-segment layout
/// - Raw, RLE, or Compressed blocks
/// - Predefined FSE modes (no per-frame table description)
///
/// # Errors
///
/// Returns an error string if the input is truncated, has a bad magic number,
/// or contains unsupported features (non-predefined FSE tables, Huffman
/// literals, reserved block types).
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
    let dict_id_bytes = [0usize, 1, 2, 4][dict_flag as usize];
    pos += dict_id_bytes; // skip dict ID (we don't support custom dicts)

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
                decompress_block(block_data, &mut out)?;
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
    // Gracefully no-ops (rather than failing) when the `zstd` binary isn't
    // on `PATH`, since CI/dev environments vary.

    /// Checks whether the `zstd` CLI binary is reachable on `PATH`.
    fn is_zstd_cli_available() -> bool {
        std::process::Command::new("zstd")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Runs `zstd` with the given arguments, returning captured stdout.
    /// Panics (failing the calling test) if the CLI exits non-zero.
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

    /// Writes `data` to a fresh temp file and returns its path. The caller
    /// is responsible for deleting it (tests use a `finally`-style cleanup
    /// via a guard so the file is removed even if an assertion panics).
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
    struct TempFileGuard(std::path::PathBuf);
    impl Drop for TempFileGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn tc9_cli_interop() {
        if !is_zstd_cli_available() {
            eprintln!("zstd CLI not found on PATH — skipping TC-9 interop test");
            return;
        }

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

    #[test]
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
        if !is_zstd_cli_available() {
            eprintln!("zstd CLI not found on PATH — skipping interop test");
            return;
        }

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
        let (decoded, _) = decode_literals_section(&encoded).unwrap();
        assert_eq!(decoded, lits);
    }

    #[test]
    fn test_literals_section_roundtrip_medium() {
        let lits: Vec<u8> = (0..200).map(|i| (i % 256) as u8).collect();
        let encoded = encode_literals_section(&lits);
        let (decoded, _) = decode_literals_section(&encoded).unwrap();
        assert_eq!(decoded, lits);
    }

    #[test]
    fn test_literals_section_roundtrip_large() {
        let lits: Vec<u8> = (0..5000).map(|i| (i % 256) as u8).collect();
        let encoded = encode_literals_section(&lits);
        let (decoded, _) = decode_literals_section(&encoded).unwrap();
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
        let dt_ll = build_decode_table(&LL_NORM, LL_ACC_LOG);
        let dt_ml = build_decode_table(&ML_NORM, ML_ACC_LOG);
        let dt_of = build_decode_table(&OF_NORM, OF_ACC_LOG);

        let mut br = RevBitReader::new(bitstream).unwrap();
        let mut state_ll = br.read_bits(LL_ACC_LOG) as u16;
        let mut state_of = br.read_bits(OF_ACC_LOG) as u16;
        let mut state_ml = br.read_bits(ML_ACC_LOG) as u16;

        let mut out = Vec::with_capacity(n_seqs);
        for i in 0..n_seqs {
            let ll_entry = fse_peek(state_ll, &dt_ll);
            let ml_entry = fse_peek(state_ml, &dt_ml);
            let of_entry = fse_peek(state_of, &dt_of);

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
}
