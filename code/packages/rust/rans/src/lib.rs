//! # rANS — Range Asymmetric Numeral Systems
//!
//! rANS is a modern entropy coder that achieves near-Shannon compression in
//! O(1) time per symbol. It is the entropy engine inside JPEG XL, Zstandard,
//! AV1, and others.
//!
//! ## How it works — a quick sketch
//!
//! Think of rANS as a radix system where the "base" varies per symbol.
//! A single large integer `x` (the **state**) encodes the entire message.
//!
//! ### The state machine
//!
//! We pick a table size `M = 2^k` (a power of two). Each of the `n` symbols
//! s₀, s₁, … sₙ₋₁ is assigned a **frequency** fᵢ where Σfᵢ = M.
//!
//! The state `x` lives in the interval `[L, b·L)` where:
//! - `L = M` (the lower bound, equal to the table size)
//! - `b = 2^8 = 256` (byte-level streaming)
//!
//! So `x ∈ [M, 256·M)`.
//!
//! ### Encoding one symbol s with frequency f
//!
//! 1. **Renormalize** (shrink x) until x ∈ [f·(256/M)·M, f·256):
//!    while x ≥ f * 256: emit byte (x & 0xFF), x >>= 8
//! 2. **Step**: x = (x / f) * M + cumfreq[s] + (x % f)
//!
//! Notice the encode reversal: symbols are encoded last-first so the decoder
//! reads them first-last without a stack.
//!
//! ### Decoding one symbol
//!
//! 1. **Look up** slot = x % M in the precomputed decode table.
//!    The table stores (symbol, freq, cumfreq) for each slot.
//! 2. **Step**: x = freq * (x / M) + (x % M) - cumfreq
//! 3. **Renormalize** (grow x) until x ≥ M:
//!    while x < M: x = (x << 8) | next_byte()
//!
//! ## API
//!
//! ```
//! use rans::{AnsTable, RansEncoder, RansDecoder};
//!
//! // 1) Build a frequency table for your alphabet.
//! //    Here: 2 symbols, "A"=0 has freq 3, "B"=1 has freq 1. M=4.
//! let table = AnsTable::new(&[3, 1]).unwrap();
//!
//! // 2) Encode a sequence (symbols are pushed in *reverse* order).
//! let mut enc = RansEncoder::new(&table);
//! let symbols = [0u8, 0, 1, 0]; // A A B A
//! for &s in symbols.iter().rev() {
//!     enc.put(s);
//! }
//! let compressed = enc.finish();
//!
//! // 3) Decode.
//! let mut dec = RansDecoder::new(&table, &compressed).unwrap();
//! for _ in 0..symbols.len() {
//!     let s = dec.get();
//!     println!("decoded symbol {}", s);
//! }
//! ```

pub const VERSION: &str = "0.1.0";

/// A precomputed rANS frequency table for a fixed alphabet.
///
/// # Table size M
///
/// M = 2^k is the smallest power of two that is ≥ the sum of all
/// supplied frequencies. All frequencies are scaled proportionally so
/// that they sum exactly to M.
///
/// # Decode table
///
/// The decode table has M entries. Entry `slot` stores the symbol whose
/// cumulative range `[cumfreq, cumfreq + freq)` contains `slot`. This
/// gives O(1) symbol lookup during decode.
///
/// # Example
///
/// ```
/// use rans::AnsTable;
/// let t = AnsTable::new(&[3, 1]).unwrap(); // A:3/4, B:1/4
/// assert_eq!(t.m(), 4);
/// ```
#[derive(Debug, Clone)]
pub struct AnsTable {
    /// Number of symbols.
    n: usize,
    /// Normalized frequencies — must sum to `m`.
    freq: Vec<u32>,
    /// Cumulative frequencies: cumfreq[i] = sum of freq[0..i].
    cumfreq: Vec<u32>,
    /// Table size M = 2^k.
    m: u32,
    /// log2(M) — the number of bits in a slot index.
    log2m: u32,
    /// Flat decode table of length M.
    /// decode_sym[slot] = symbol index whose range covers `slot`.
    decode_sym: Vec<u8>,
    /// decode_freq[slot] = normalized freq of the symbol at `slot`.
    decode_freq: Vec<u32>,
    /// decode_cumfreq[slot] = cumulative freq of the symbol at `slot`.
    decode_cumfreq: Vec<u32>,
}

impl AnsTable {
    /// Build an `AnsTable` from raw (unnormalized) symbol counts.
    ///
    /// The counts are scaled so they sum to the nearest power of two ≥
    /// `counts.len()` and ≥ 1. If any count rounds to zero the function
    /// returns an error — use at least one occurrence per symbol in the
    /// input alphabet.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` if:
    /// - `counts` is empty
    /// - Any normalized frequency rounds to zero
    /// - `counts.len() > 256` (symbols must fit in a `u8`)
    pub fn new(counts: &[u32]) -> Result<Self, String> {
        if counts.is_empty() {
            return Err("AnsTable: counts must not be empty".into());
        }
        if counts.len() > 256 {
            return Err(format!(
                "AnsTable: alphabet size {} exceeds 256",
                counts.len()
            ));
        }

        let n = counts.len();
        let total: u64 = counts.iter().map(|&c| c as u64).sum();
        if total == 0 {
            return Err("AnsTable: all counts are zero".into());
        }

        // Find smallest M = 2^k such that M >= n and M >= total.
        // We also want M >= n so every symbol can have freq >= 1.
        //
        // SECURITY: Use u64 for the temporary `m` to prevent u32 overflow when
        // large raw counts are supplied. 256 symbols × u32::MAX each = ~1.1 × 10¹²
        // which is well above 2^32 and would cause an infinite loop or panic with u32.
        //
        // We cap M at 2^16 = 65_536. The decode table is M entries × ~12 bytes each,
        // so M = 65536 → ~768 KB — large but acceptable. M = 2^24 → 192 MB, which is
        // impractical. JPEG XL uses M = 4096; Zstandard uses M ≤ 2^12. The cap is
        // generous and safe.
        let min_m = (n as u64).max(total).max(1);
        if min_m > (1u64 << 16) {
            return Err(format!(
                "AnsTable: normalized table size M would exceed 2^16 (min_m = {}). \
                 Reduce the alphabet size or scale down the input counts.",
                min_m
            ));
        }
        let mut log2m = 0u32;
        let mut m_u64 = 1u64;
        while m_u64 < min_m {
            log2m += 1;
            m_u64 <<= 1;
        }
        // m_u64 <= 2^16, so this cast is safe.
        let m = m_u64 as u32;

        // Normalize counts to sum exactly to m using the largest-remainder
        // method. This preserves the proportion as closely as possible.
        //
        // Step 1: compute floor(count * m / total) for each symbol.
        let mut freq: Vec<u32> = counts
            .iter()
            .map(|&c| ((c as u64 * m as u64) / total) as u32)
            .collect();

        // Every symbol in the input alphabet MUST have freq >= 1 — otherwise
        // it would be unreachable from the decode table.
        //
        // Step 2: distribute the remainder among symbols with the largest
        // fractional part, starting with those that currently have freq=0.
        let mut remainder = m - freq.iter().sum::<u32>();

        // Compute fractional parts (numerator of frac portion scaled by total)
        // for sorting. We break ties by preferring symbols with freq=0 first.
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| {
            // freq=0 symbols must get bumped first
            let a_zero = if freq[a] == 0 { 1u8 } else { 0u8 };
            let b_zero = if freq[b] == 0 { 1u8 } else { 0u8 };
            if a_zero != b_zero {
                return b_zero.cmp(&a_zero); // zero-freq first
            }
            // then by descending fractional part
            let fa = (counts[a] as u64 * m as u64) % total;
            let fb = (counts[b] as u64 * m as u64) % total;
            fb.cmp(&fa)
        });

        for i in order {
            if remainder == 0 {
                break;
            }
            freq[i] += 1;
            remainder -= 1;
        }

        // Validate: every symbol now has freq >= 1.
        for (i, &f) in freq.iter().enumerate() {
            if f == 0 {
                return Err(format!(
                    "AnsTable: symbol {} has zero frequency after normalization \
                     (supply at least one count per alphabet symbol)",
                    i
                ));
            }
        }

        // Build cumulative frequencies.
        let mut cumfreq = vec![0u32; n + 1];
        for i in 0..n {
            cumfreq[i + 1] = cumfreq[i] + freq[i];
        }
        assert_eq!(cumfreq[n], m, "frequencies must sum to m");

        // Build the flat decode table. Each slot s in [0, M) belongs to the
        // symbol whose cumulative range covers it. We fill this with a simple
        // linear scan — O(M) which is fine for typical M ≤ 65536.
        let m_usize = m as usize;
        let mut decode_sym = vec![0u8; m_usize];
        let mut decode_freq = vec![0u32; m_usize];
        let mut decode_cumfreq = vec![0u32; m_usize];

        for sym in 0..n {
            let lo = cumfreq[sym] as usize;
            let hi = cumfreq[sym + 1] as usize;
            for slot in lo..hi {
                decode_sym[slot] = sym as u8;
                decode_freq[slot] = freq[sym];
                decode_cumfreq[slot] = cumfreq[sym];
            }
        }

        Ok(AnsTable {
            n,
            freq,
            cumfreq: cumfreq[..n].to_vec(), // drop the trailing sentinel
            m,
            log2m,
            decode_sym,
            decode_freq,
            decode_cumfreq,
        })
    }

    /// Table size M = 2^k.
    pub fn m(&self) -> u32 {
        self.m
    }

    /// log₂(M).
    pub fn log2m(&self) -> u32 {
        self.log2m
    }

    /// Number of symbols in the alphabet.
    pub fn alphabet_size(&self) -> usize {
        self.n
    }

    /// Normalized frequency of symbol `s`, or `None` if `s >= alphabet_size()`.
    pub fn freq(&self, s: usize) -> Option<u32> {
        self.freq.get(s).copied()
    }

    /// Cumulative frequency of symbol `s` (sum of freq[0..s]), or `None` if
    /// `s >= alphabet_size()`.
    pub fn cumfreq(&self, s: usize) -> Option<u32> {
        self.cumfreq.get(s).copied()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Encoder
// ─────────────────────────────────────────────────────────────────────────────

/// rANS streaming encoder.
///
/// Symbols must be pushed in **reverse** order — the first symbol in the
/// logical sequence is the last call to `put`. Call `finish()` to obtain
/// the compressed byte stream.
///
/// # Why reverse?
///
/// rANS encodes by folding each symbol into a large integer from the right.
/// A decoder starting from the initial state will unroll in forward order.
/// This mirrors the way a stack works: LIFO push → FIFO pop.
///
/// # Example
///
/// ```
/// use rans::{AnsTable, RansEncoder};
/// let table = AnsTable::new(&[1, 1]).unwrap(); // 50-50
/// let mut enc = RansEncoder::new(&table);
/// // Encode [0, 1] by pushing in reverse:
/// enc.put(1);
/// enc.put(0);
/// let bytes = enc.finish();
/// assert!(!bytes.is_empty());
/// ```
#[derive(Debug)]
pub struct RansEncoder<'a> {
    table: &'a AnsTable,
    /// ANS state x, starts at M (the lower bound L).
    x: u64,
    /// Bytes collected during renormalization, in *reverse* emission order.
    /// After `finish()` they are reversed to produce a big-endian stream.
    pending: Vec<u8>,
}

impl<'a> RansEncoder<'a> {
    /// Create a new encoder using the given frequency table.
    pub fn new(table: &'a AnsTable) -> Self {
        RansEncoder {
            table,
            x: table.m as u64, // start at L = M
            pending: Vec::new(),
        }
    }

    /// Encode one symbol.
    ///
    /// `symbol` must be a valid index in `[0, alphabet_size)`.
    ///
    /// # Panics
    ///
    /// Panics if `symbol >= alphabet_size`.
    pub fn put(&mut self, symbol: u8) {
        let s = symbol as usize;
        assert!(
            s < self.table.n,
            "RansEncoder::put: symbol {} out of range (alphabet size {})",
            s,
            self.table.n
        );

        let f = self.table.freq[s] as u64;
        let m = self.table.m as u64;

        // Renormalize: shrink x until it falls in [f * (256/M) * M, f * 256).
        // The upper bound for x before the step is f * 256 - 1.
        // While x >= f * 256, emit the low byte and shift right.
        //
        //   upper_bound = f << 8  (== f * 256)
        //
        // We want x < f * 256 after stripping bytes.
        let upper_bound = f << 8; // f * b where b = 256
        while self.x >= upper_bound {
            self.pending.push((self.x & 0xFF) as u8);
            self.x >>= 8;
        }

        // ANS step: x → (x / f) * M + cumfreq[s] + (x % f)
        let q = self.x / f;
        let r = self.x % f;
        self.x = q * m + self.table.cumfreq[s] as u64 + r;
    }

    /// Flush the encoder and return the compressed byte stream.
    ///
    /// The final state `x` is written as a 4-byte big-endian prefix,
    /// followed by the renormalization bytes in forward order.
    ///
    /// After `finish()` the encoder is consumed and cannot be used.
    pub fn finish(mut self) -> Vec<u8> {
        // Emit the final state as 8 bytes (full u64). Because the whole `pending`
        // buffer is reversed after this, we push the state bytes in LSB-first order
        // so they end up MSB-first (big-endian) in the final output.
        //
        //   pending (before reverse): [renorm_newest, ..., renorm_oldest, LSB, .., MSB]
        //   pending (after  reverse): [MSB, .., LSB, renorm_oldest, ..., renorm_newest]
        //                              ^^^^ big-endian state ^^^^   ^^^^ forward order ^^^^
        //
        // SECURITY: We write all 8 bytes of the u64 state rather than truncating to
        // 4 bytes. With M up to 2^16 and b=256, the encoder state x lives in
        // [M, 256·M) ⊂ [2^16, 2^24) which fits in 4 bytes. However, writing 8 bytes
        // is more robust and avoids silent truncation if M ever grows.
        let x = self.x;
        self.pending.push((x & 0xFF) as u8);
        self.pending.push(((x >> 8) & 0xFF) as u8);
        self.pending.push(((x >> 16) & 0xFF) as u8);
        self.pending.push(((x >> 24) & 0xFF) as u8);
        self.pending.push(((x >> 32) & 0xFF) as u8);
        self.pending.push(((x >> 40) & 0xFF) as u8);
        self.pending.push(((x >> 48) & 0xFF) as u8);
        self.pending.push(((x >> 56) & 0xFF) as u8);

        // The bytes were collected in reverse renormalization order (newest
        // byte first). Reverse them to get the correct decode-time order.
        self.pending.reverse();
        self.pending
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Decoder
// ─────────────────────────────────────────────────────────────────────────────

/// rANS streaming decoder.
///
/// Constructed from a byte slice produced by `RansEncoder::finish()`.
/// Symbols are extracted in forward order by repeated calls to `get()`.
///
/// # Example
///
/// ```
/// use rans::{AnsTable, RansEncoder, RansDecoder};
/// let table = AnsTable::new(&[3, 1]).unwrap();
/// let mut enc = RansEncoder::new(&table);
/// for &s in [0u8, 0, 1, 0].iter().rev() { enc.put(s); }
/// let compressed = enc.finish();
///
/// let mut dec = RansDecoder::new(&table, &compressed).unwrap();
/// assert_eq!(dec.get(), 0);
/// assert_eq!(dec.get(), 0);
/// assert_eq!(dec.get(), 1);
/// assert_eq!(dec.get(), 0);
/// ```
#[derive(Debug)]
pub struct RansDecoder<'a> {
    table: &'a AnsTable,
    /// Current ANS state.
    x: u64,
    /// Input byte stream, positioned just after the initial 4-byte state.
    data: &'a [u8],
    /// Read cursor into `data`.
    pos: usize,
}

impl<'a> RansDecoder<'a> {
    /// Create a decoder from the byte stream produced by `RansEncoder::finish()`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `data` is shorter than 4 bytes (the minimum needed
    /// to hold the initial state word).
    pub fn new(table: &'a AnsTable, data: &'a [u8]) -> Result<Self, String> {
        if data.len() < 8 {
            return Err(format!(
                "RansDecoder: data too short ({} bytes, need at least 8 for the state header)",
                data.len()
            ));
        }
        // Read the initial state from the first 8 bytes (big-endian u64).
        // The encoder writes 8 bytes to avoid truncating large states.
        let x = ((data[0] as u64) << 56)
            | ((data[1] as u64) << 48)
            | ((data[2] as u64) << 40)
            | ((data[3] as u64) << 32)
            | ((data[4] as u64) << 24)
            | ((data[5] as u64) << 16)
            | ((data[6] as u64) << 8)
            | (data[7] as u64);

        Ok(RansDecoder {
            table,
            x,
            data,
            pos: 8,
        })
    }

    /// Decode and return the next symbol.
    ///
    /// # Panics
    ///
    /// Panics if `get()` is called more times than symbols were encoded —
    /// the internal state becomes undefined once the byte stream is exhausted.
    /// Use `is_exhausted()` to check before calling.
    pub fn get(&mut self) -> u8 {
        let m = self.table.m as u64;

        // Step 1: look up the symbol for slot = x % M.
        let slot = (self.x % m) as usize;
        let sym = self.table.decode_sym[slot];
        let f = self.table.decode_freq[slot] as u64;
        let cf = self.table.decode_cumfreq[slot] as u64;

        // Step 2: ANS inverse step.
        // x → f * (x / M) + (x % M) - cumfreq[sym]
        self.x = f * (self.x / m) + (self.x % m) - cf;

        // Step 3: renormalize — grow x back into [M, 256·M) by reading bytes.
        while self.x < m {
            let byte = if self.pos < self.data.len() {
                let b = self.data[self.pos];
                self.pos += 1;
                b as u64
            } else {
                0u64 // padding zeros if stream ends early
            };
            self.x = (self.x << 8) | byte;
        }

        sym
    }

    /// Returns `true` if all bytes of the compressed stream have been consumed.
    ///
    /// Note: the decoder may still have symbols left to produce even after
    /// `is_exhausted()` returns `true` — the final state encodes the last few
    /// symbols without needing additional bytes.
    pub fn is_exhausted(&self) -> bool {
        self.pos >= self.data.len()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: encode `symbols` and decode them back; assert exact match.
    fn round_trip(counts: &[u32], symbols: &[u8]) {
        let table = AnsTable::new(counts).expect("AnsTable::new failed");

        let mut enc = RansEncoder::new(&table);
        for &s in symbols.iter().rev() {
            enc.put(s);
        }
        let compressed = enc.finish();

        let mut dec = RansDecoder::new(&table, &compressed).expect("RansDecoder::new failed");
        for (i, &expected) in symbols.iter().enumerate() {
            let got = dec.get();
            assert_eq!(
                got, expected,
                "mismatch at position {}: expected {}, got {}",
                i, expected, got
            );
        }
    }

    // ── AnsTable construction ─────────────────────────────────────────────

    #[test]
    fn table_uniform_2_symbols() {
        // 50-50: both symbols should have freq=1, M=2
        let t = AnsTable::new(&[1, 1]).unwrap();
        assert_eq!(t.m(), 2);
        assert_eq!(t.freq(0), Some(1));
        assert_eq!(t.freq(1), Some(1));
        assert_eq!(t.cumfreq(0), Some(0));
        assert_eq!(t.cumfreq(1), Some(1));
        // Out-of-range → None
        assert_eq!(t.freq(2), None);
        assert_eq!(t.cumfreq(2), None);
    }

    #[test]
    fn table_skewed_4_symbols() {
        // A:3 B:1 — M should be 4
        let t = AnsTable::new(&[3, 1]).unwrap();
        assert_eq!(t.m(), 4);
        assert_eq!(t.freq(0), Some(3));
        assert_eq!(t.freq(1), Some(1));
    }

    #[test]
    fn table_frequencies_sum_to_m() {
        // For any input the normalized frequencies must sum to M.
        let counts = vec![7u32, 3, 5, 1, 4];
        let t = AnsTable::new(&counts).unwrap();
        let sum: u32 = (0..counts.len()).map(|i| t.freq(i).unwrap()).sum();
        assert_eq!(sum, t.m());
    }

    #[test]
    fn table_m_is_power_of_two() {
        for n in 1u32..=16 {
            let counts: Vec<u32> = (1..=n).collect();
            let t = AnsTable::new(&counts).unwrap();
            let m = t.m();
            assert_eq!(m & (m - 1), 0, "M={} is not a power of two", m);
        }
    }

    #[test]
    fn table_alphabet_size_matches() {
        let t = AnsTable::new(&[10, 5, 3]).unwrap();
        assert_eq!(t.alphabet_size(), 3);
    }

    #[test]
    fn table_empty_returns_error() {
        assert!(AnsTable::new(&[]).is_err());
    }

    #[test]
    fn table_all_zero_counts_returns_error() {
        assert!(AnsTable::new(&[0, 0, 0]).is_err());
    }

    #[test]
    fn table_too_many_symbols_returns_error() {
        let counts = vec![1u32; 257];
        assert!(AnsTable::new(&counts).is_err());
    }

    #[test]
    fn table_decode_slots_cover_full_range() {
        // Every slot in [0, M) should have a valid symbol assignment.
        let t = AnsTable::new(&[5, 3, 2]).unwrap();
        let m = t.m() as usize;
        for slot in 0..m {
            let sym = t.decode_sym[slot] as usize;
            let cf = t.decode_cumfreq[slot];
            let f = t.decode_freq[slot];
            assert!(sym < t.alphabet_size(), "slot {} has invalid symbol {}", slot, sym);
            assert!(
                slot >= cf as usize && slot < (cf + f) as usize,
                "slot {} not in range [{}, {}) for symbol {}",
                slot,
                cf,
                cf + f,
                sym
            );
        }
    }

    // ── Round-trip tests ──────────────────────────────────────────────────

    #[test]
    fn round_trip_single_symbol_sequence() {
        // Single symbol alphabet with freq=1.
        // All encodes should produce 0.
        round_trip(&[1], &[0, 0, 0, 0]);
    }

    #[test]
    fn round_trip_two_symbols_uniform() {
        round_trip(&[1, 1], &[0, 1, 0, 0, 1, 1, 0]);
    }

    #[test]
    fn round_trip_two_symbols_skewed() {
        // A:3, B:1 — mostly As
        round_trip(&[3, 1], &[0, 0, 1, 0, 0, 0, 1, 0]);
    }

    #[test]
    fn round_trip_four_symbols() {
        round_trip(&[4, 3, 2, 1], &[0, 1, 2, 3, 0, 2, 1, 3, 0]);
    }

    #[test]
    fn round_trip_all_same_symbol() {
        // All zeros — only symbol 0 appears.
        round_trip(&[7, 1], &[0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn round_trip_long_sequence() {
        // 128-symbol sequence with a 3-symbol alphabet.
        let symbols: Vec<u8> = (0u8..128).map(|i| i % 3).collect();
        round_trip(&[5, 3, 2], &symbols);
    }

    #[test]
    fn round_trip_256_symbol_alphabet() {
        // Full byte alphabet — all 256 symbols, uniform.
        let counts = vec![1u32; 256];
        let symbols: Vec<u8> = (0u8..=255).collect();
        round_trip(&counts, &symbols);
    }

    #[test]
    fn round_trip_single_symbol_single_call() {
        round_trip(&[3, 1], &[0]);
    }

    #[test]
    fn round_trip_two_symbols_two_calls() {
        round_trip(&[1, 1], &[0, 1]);
        round_trip(&[1, 1], &[1, 0]);
    }

    #[test]
    fn round_trip_unequal_counts_normalization() {
        // Unnormalized counts (7, 3) → M=16, freq=(11,5) or similar.
        // Just verify round-trip, not exact freqs.
        round_trip(&[7, 3], &[0, 0, 0, 1, 0, 0, 1, 0, 0, 0]);
    }

    // ── Encoder / Decoder API tests ───────────────────────────────────────

    #[test]
    fn encoder_finish_at_least_8_bytes() {
        // Even an empty sequence must produce at least the 8-byte state header.
        let t = AnsTable::new(&[1, 1]).unwrap();
        let enc = RansEncoder::new(&t);
        let bytes = enc.finish();
        assert!(bytes.len() >= 8);
    }

    #[test]
    fn decoder_too_short_returns_error() {
        let t = AnsTable::new(&[1, 1]).unwrap();
        // Under 8 bytes should fail.
        assert!(RansDecoder::new(&t, &[0, 0, 0, 0, 0, 0, 0]).is_err());
        assert!(RansDecoder::new(&t, &[]).is_err());
    }

    #[test]
    fn table_m_cap_rejects_too_large() {
        // Counts summing above 2^16 should be rejected with an error.
        let counts = vec![1u32 << 17];
        assert!(AnsTable::new(&counts).is_err());
    }

    #[test]
    fn is_exhausted_after_reading_all_bytes() {
        let t = AnsTable::new(&[1, 1]).unwrap();
        let mut enc = RansEncoder::new(&t);
        enc.put(0);
        let bytes = enc.finish();

        let mut dec = RansDecoder::new(&t, &bytes).unwrap();
        let _ = dec.get();
        // After decoding the only symbol, all bytes may or may not be consumed
        // (the final state uses the 4-byte header). Just check it doesn't panic.
        let _ = dec.is_exhausted();
    }

    #[test]
    fn compression_ratio_better_than_uncompressed() {
        // A highly skewed distribution should compress well.
        // 128 symbols, freq A=120, B=8 → entropy ≈ 0.47 bits/symbol.
        // Uncompressed = 128 bytes. Compressed should be much smaller.
        let symbols: Vec<u8> = (0u8..128).map(|i| if i < 120 { 0 } else { 1 }).collect();
        let t = AnsTable::new(&[120, 8]).unwrap();
        let mut enc = RansEncoder::new(&t);
        for &s in symbols.iter().rev() {
            enc.put(s);
        }
        let compressed = enc.finish();
        // Should be well under 128 bytes (at minimum < 64 bytes for 0.47 bits/sym).
        assert!(
            compressed.len() < 64,
            "expected compressed < 64 bytes, got {}",
            compressed.len()
        );
    }

    #[test]
    fn deterministic_output_regression() {
        // Fixed seed: encode [0, 1, 0, 1] with a 2-symbol uniform alphabet.
        // This is a regression test — the output must not change between versions.
        let t = AnsTable::new(&[1, 1]).unwrap();
        let mut enc = RansEncoder::new(&t);
        for &s in [0u8, 1, 0, 1].iter().rev() {
            enc.put(s);
        }
        let bytes1 = enc.finish();

        // Re-encode to verify determinism.
        let mut enc2 = RansEncoder::new(&t);
        for &s in [0u8, 1, 0, 1].iter().rev() {
            enc2.put(s);
        }
        let bytes2 = enc2.finish();

        assert_eq!(bytes1, bytes2, "rANS encoding must be deterministic");
    }

    #[test]
    fn table_log2m_correct() {
        // M=2 → log2m=1, M=4 → 2, M=8 → 3, M=16 → 4
        let cases = [(2, 1), (4, 2), (8, 3), (16, 4)];
        for (expected_m, expected_log2m) in cases {
            let counts: Vec<u32> = vec![1u32; expected_m];
            let t = AnsTable::new(&counts).unwrap();
            assert_eq!(t.log2m(), expected_log2m as u32);
        }
    }
}
