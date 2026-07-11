//! VP8L canonical Huffman code storage and lookup tables.
//!
//! VP8L uses a custom Huffman code storage format that is more compact than
//! simply listing all code lengths.  This module implements:
//!
//! 1. **Code-length serialisation** — writing the 5 Huffman group tables into
//!    the bitstream using VP8L's "simple code" and "complex code" formats.
//!
//! 2. **Code-length deserialisation** — reading those tables back from the
//!    bitstream.
//!
//! 3. **Canonical Huffman decode tables** — fast decode tables built from
//!    code lengths for an LSB-first bit stream.
//!
//! ## Bit ordering — LSB-first canonical codes
//!
//! VP8L uses LSB-first bit packing (like DEFLATE): the first bit written is
//! the LSB of the first byte.  Canonical Huffman codes in VP8L are stored with
//! the MSB of the canonical code first in the stream — but since the stream is
//! LSB-first, the code is **bit-reversed** before writing.
//!
//! Concretely: for a canonical code `c` of length `l`:
//! - The encoder writes `reverse_bits(c, l)` using `write_bits`.
//! - The decoder's `peek_bits(max_len)` returns the bit-reversed canonical code
//!   in the low `l` bits (with the suffix bits in bits above position `l`).
//! - The fast table is indexed by `peek_bits(max_code_len)`, mapping to the
//!   correct symbol.
//!
//! ## Huffman groups
//!
//! VP8L uses **five** Huffman trees (groups G, R, B, A, Dist):
//!
//! - **G (green)** — 280 symbols: 0-255 literal green, 256-279 LZ77 length.
//! - **R (red)** — 256 symbols.
//! - **B (blue)** — 256 symbols.
//! - **A (alpha)** — 256 symbols.
//! - **Dist** — 40 symbols (distance codes).
//!
//! ## VP8L Huffman code storage
//!
//! ### Simple code (≤ 2 distinct symbols)
//!
//! ```text
//! 1 bit:  simple_flag = 1
//! 1 bit:  num_symbols_minus_1 (0=one symbol, 1=two symbols)
//! If one symbol:
//!   8 bits: symbol0
//! If two symbols:
//!   1 bit:  symbol0_is_8bit (0=symbol0 < 2 and fits in 1 bit; 1=use 8 bits)
//!   (1 or 8) bits: symbol0
//!   8 bits: symbol1
//! ```
//!
//! symbol0 gets code 0 (1 bit), symbol1 gets code 1 (1 bit).
//!
//! ### Complex code (> 2 distinct symbols)
//!
//! Code lengths are stored using a meta-Huffman tree over the 19-symbol
//! meta-alphabet { 0-15, 16, 17, 18 }, where:
//!  - 0-15 = literal code length values.
//!  - 16 = repeat last non-zero length 3..=6 times.
//!  - 17 = repeat zero 3..=10 times.
//!  - 18 = repeat zero 11..=138 times.
//!
//! The meta-tree itself is stored as 19 raw 3-bit code lengths (in
//! `CODE_LENGTH_ORDER`).  We use a fixed meta-tree where symbols 0-7 each
//! have meta-length 4 and symbols 8-15 each have meta-length 4, but in
//! practice we use a simpler uniform meta-tree of meta-length = 4 for the
//! 16 symbols we actually need (0-15) and 0 for symbols 16-18.

use super::bitstream::{BitReader, BitWriter};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Size of the G (green + length) Huffman group alphabet (256 + 24 = 280).
pub const G_ALPHABET_SIZE: usize = 280;

/// Size of R, B, A Huffman group alphabets (one entry per byte value).
pub const RGBA_ALPHABET_SIZE: usize = 256;

/// Size of the Dist (distance) Huffman group alphabet.
pub const DIST_ALPHABET_SIZE: usize = 40;

/// Fixed order in which meta-alphabet code lengths are stored in the bitstream.
const CODE_LENGTH_ORDER: [usize; 19] = [
    17, 18, 0, 1, 2, 3, 4, 5, 16, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];

// ---------------------------------------------------------------------------
// HuffmanTable — fast decode lookup table for LSB-first streams
// ---------------------------------------------------------------------------

/// A canonical Huffman decode table for VP8L's LSB-first bit streams.
///
/// ## Fast decode algorithm
///
/// 1. `peek_bits(max_code_len)` — reads up to `max_code_len` bits without consuming.
/// 2. `fast[peeked]` — gives `(symbol, actual_code_len)`.
/// 3. `consume_bits(actual_code_len)` — advances the stream.
///
/// For codes shorter than `max_code_len`, the same entry appears in multiple
/// consecutive slots (one per possible "don't care" suffix).
///
/// For trivial (0 or 1 symbol) codes, no bits are consumed; the symbol is
/// returned directly.
#[derive(Debug, Clone)]
pub struct HuffmanTable {
    /// Maximum code length in this table.  0 = trivial (≤1-symbol) code.
    pub max_code_len: u32,
    /// The single symbol for trivial codes.  `None` if the table has 0 or ≥2 symbols.
    pub trivial_symbol: Option<u16>,
    /// Fast decode table.  Size = `1 << max_code_len`.
    /// `fast[peeked_bits]` = `(symbol, code_length)`.
    /// Indexed by the bit-reversed canonical code padded to `max_code_len` bits.
    /// Unused slots hold `(u16::MAX, 0)`.
    fast: Vec<(u16, u32)>,
}

impl HuffmanTable {
    /// Build a `HuffmanTable` from code lengths.
    ///
    /// `code_lengths[i]` is the number of bits for symbol `i`.
    /// A length of 0 means the symbol is absent.
    pub fn from_lengths(code_lengths: &[u32]) -> Result<Self, String> {
        let max_code_len = code_lengths.iter().copied().max().unwrap_or(0);

        // Collect active symbols.
        let mut active: Vec<(u16, u32)> = code_lengths
            .iter()
            .enumerate()
            .filter(|&(_, &l)| l > 0)
            .map(|(i, &l)| (i as u16, l))
            .collect();

        // 0 or 1 active symbol → trivial table.
        // We also treat the single-symbol case with any length as trivial:
        // the encoder writes write_simple_1(sym) and the decoder always returns
        // that symbol without consuming any bits.
        if active.len() <= 1 {
            return Ok(HuffmanTable {
                max_code_len: 0,
                trivial_symbol: active.first().map(|&(sym, _)| sym),
                fast: vec![],
            });
        }

        // Sort by (length, symbol) for deterministic canonical assignment.
        active.sort_by_key(|&(sym, len)| (len, sym));

        if max_code_len > 15 {
            return Err(format!(
                "VP8L: max code length {max_code_len} exceeds VP8L limit of 15"
            ));
        }

        // Assign canonical codes (standard MSB-first algorithm).
        let mut canonical_codes: Vec<u32> = Vec::with_capacity(active.len());
        {
            let mut code = 0u32;
            let mut prev_len = active[0].1;
            for &(_, len) in &active {
                if len > prev_len {
                    code <<= len - prev_len;
                }
                canonical_codes.push(code);
                code += 1;
                prev_len = len;
            }
        }

        // Build the fast table.
        //
        // The table is indexed by `peek_bits(max_code_len)`.  Since the bit
        // stream is LSB-first, `peek_bits(n)` returns the next `n` bits with
        // bit[0] in position 0 of the result.  When the encoder writes a
        // canonical code it bit-reverses it first, so what's in the stream for
        // a code of length `l` is `reverse_bits(canonical, l)`.
        //
        // For a code of length `l < max_code_len`, peeking `max_code_len` bits
        // gives `reverse_bits(canonical, l)` in the low `l` bits, with the high
        // `max_code_len - l` bits coming from the next symbol in the stream
        // ("don't care" bits for the current lookup).
        //
        // So for each symbol with code `canonical` of length `l`:
        //   reversed = reverse_bits(canonical, l)
        //   for each possible suffix k in 0..2^(max_code_len - l):
        //     table[reversed | (k << l)] = (symbol, l)

        let table_size = 1usize << max_code_len;
        let mut fast = vec![(u16::MAX, 0u32); table_size];

        for (i, &(sym, len)) in active.iter().enumerate() {
            let canonical = canonical_codes[i];
            let reversed = reverse_bits_n(canonical, len);
            let suffix_bits = max_code_len - len;
            let suffix_count = 1usize << suffix_bits;
            for k in 0..suffix_count {
                let idx = reversed as usize | (k << len);
                fast[idx] = (sym, len);
            }
        }

        Ok(HuffmanTable {
            max_code_len,
            trivial_symbol: None,
            fast,
        })
    }

    /// Decode one symbol from the bit reader.
    pub fn decode(&self, reader: &mut BitReader) -> Result<u16, String> {
        // Trivial 1-symbol code: return the symbol without reading any bits.
        if let Some(sym) = self.trivial_symbol {
            return Ok(sym);
        }

        if self.max_code_len == 0 {
            return Err("VP8L Huffman: empty decode table".to_string());
        }

        let peeked = reader.peek_bits(self.max_code_len);
        let (sym, len) = self.fast[peeked as usize];
        if len == 0 {
            return Err(format!(
                "VP8L Huffman: invalid code (peeked {peeked:#b} with max_len={})",
                self.max_code_len
            ));
        }
        reader.consume_bits(len);
        Ok(sym)
    }
}

/// Reverse the low `n` bits of `code`.
///
/// `reverse_bits_n(0b110, 3)` = `0b011`.
pub fn reverse_bits_n(code: u32, n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut result = 0u32;
    let mut c = code;
    for _ in 0..n {
        result = (result << 1) | (c & 1);
        c >>= 1;
    }
    result
}

// ---------------------------------------------------------------------------
// Write Huffman code to bitstream
// ---------------------------------------------------------------------------

/// Write the Huffman code table for one group into the bitstream.
///
/// Chooses simple (≤ 2 distinct symbols) or complex (> 2) format automatically.
pub fn write_huffman_code(bw: &mut BitWriter, code_lengths: &[u32]) {
    let active: Vec<(usize, u32)> = code_lengths
        .iter()
        .enumerate()
        .filter(|&(_, &l)| l > 0)
        .map(|(i, &l)| (i, l))
        .collect();

    // Simple codes only support 8-bit symbol values (0..=255).  If any active
    // symbol is ≥ 256 (G-group length codes 256..=279), use complex code format.
    match active.len() {
        0 => write_simple_1(bw, 0),
        1 => {
            if active[0].0 < 256 {
                write_simple_1(bw, active[0].0 as u32)
            } else {
                write_complex_code(bw, code_lengths)
            }
        }
        2 => {
            if active[0].0 < 256 && active[1].0 < 256 {
                write_simple_2(bw, active[0].0 as u32, active[1].0 as u32)
            } else {
                write_complex_code(bw, code_lengths)
            }
        }
        _ => write_complex_code(bw, code_lengths),
    }
}

/// Write a simple 1-symbol code.
///
/// ```text
/// 1 bit:  simple_flag = 1
/// 1 bit:  num_symbols_minus_1 = 0
/// 8 bits: symbol0
/// ```
pub fn write_simple_1(bw: &mut BitWriter, symbol: u32) {
    bw.write_bits(1, 1); // simple_flag = 1
    bw.write_bits(0, 1); // 1 symbol
    bw.write_bits(symbol as u64, 8);
}

/// Write a simple 2-symbol code.
///
/// ```text
/// 1 bit:  simple_flag = 1
/// 1 bit:  num_symbols_minus_1 = 1
/// 1 bit:  symbol0_is_8bit
/// (1 or 8) bits: symbol0
/// 8 bits: symbol1
/// ```
pub fn write_simple_2(bw: &mut BitWriter, sym0: u32, sym1: u32) {
    bw.write_bits(1, 1); // simple_flag = 1
    bw.write_bits(1, 1); // 2 symbols
    if sym0 < 2 {
        bw.write_bits(0, 1); // symbol0 fits in 1 bit
        bw.write_bits(sym0 as u64, 1);
    } else {
        bw.write_bits(1, 1); // symbol0 needs 8 bits
        bw.write_bits(sym0 as u64, 8);
    }
    bw.write_bits(sym1 as u64, 8);
}

/// Write a complex Huffman code table using the meta-Huffman scheme.
///
/// We use a fixed meta-tree where meta-symbols 0-15 each have meta-length 4
/// (giving 16 × 4-bit codes = valid Huffman tree), and meta-symbols 16-18
/// have meta-length 0 (absent — we don't emit any RLE codes).
///
/// With all 16 active meta-symbols at length 4, canonical codes are:
///   symbol 0 → canonical 0b0000 = 0, reversed = 0b0000 = 0
///   symbol 1 → canonical 0b0001 = 1, reversed = 0b1000 = 8
///   ...
///   symbol n → canonical n, reversed = reverse_bits_n(n, 4)
///
/// Each actual code length `cl` (value 0..=15) is encoded as 4 bits in the
/// stream: `reverse_bits_n(cl, 4)`.
///
/// ## Format
///
/// ```text
/// 0 bit:   simple_flag = 0
/// 4 bits:  num_stored - 4 = 15  (we store 19 entries, all 19 in CODE_LENGTH_ORDER)
///          Actually: num_stored = 5 (only 5 meta-code lengths = 4, rest = 0)
///          No: we store exactly the meta-lengths for symbols 16,18,0,1,...,15
///          using CODE_LENGTH_ORDER.
/// 19×3 bits: meta-code lengths in CODE_LENGTH_ORDER
/// N×4 bits:  actual code lengths encoded via meta-tree
/// ```
fn write_complex_code(bw: &mut BitWriter, code_lengths: &[u32]) {
    bw.write_bits(0, 1); // simple_flag = 0 (complex code)

    // Meta-tree: meta-symbols 0-15 all have meta-length 4; 16-18 have 0.
    // Store all 19 entries so the decoder can reconstruct the meta-tree.
    // num_stored field = count - 4; for 19 entries write 15.
    bw.write_bits(15, 4); // store all 19 meta-code lengths

    // Write meta-lengths in CODE_LENGTH_ORDER.
    // For the meta-tree: lengths 0-15 (indices 0-15) get meta-length 4.
    //                    lengths 16-18 (meta-symbols for RLE) get 0.
    let meta_lengths: [u32; 19] = {
        let mut ml = [0u32; 19];
        // Symbols 0-15 in our meta-tree get length 4.
        for i in 0usize..16 {
            ml[i] = 4;
        }
        // Symbols 16, 17, 18 get 0 (absent — we don't use RLE).
        // ml[16] = ml[17] = ml[18] = 0;  (already 0 from initialisation)
        ml
    };

    for &order_idx in &CODE_LENGTH_ORDER {
        bw.write_bits(meta_lengths[order_idx] as u64, 3);
    }

    // Now write each actual code length as a meta-tree symbol.
    // Meta-tree canonical codes: 16 symbols, each length 4.
    // Sorted by (len=4, sym): 0,1,2,...,15.
    // Canonical: sym i → code i  (in 4 bits, MSB-first).
    // Reversed for LSB-first writing: reverse_bits_n(i, 4).
    for i in 0..code_lengths.len() {
        let cl = code_lengths[i].min(15);
        // Encode meta-symbol `cl` using 4-bit reversed canonical code.
        let reversed = reverse_bits_n(cl, 4) as u64;
        bw.write_bits(reversed, 4);
    }
}

// ---------------------------------------------------------------------------
// Read Huffman code from bitstream
// ---------------------------------------------------------------------------

/// Read one Huffman code table from the bitstream.
pub fn read_huffman_code(
    br: &mut BitReader,
    alphabet_size: usize,
) -> Result<HuffmanTable, String> {
    let simple_flag = br.read_bits(1);
    if simple_flag == 1 {
        read_simple_code(br, alphabet_size)
    } else {
        read_complex_code(br, alphabet_size)
    }
}

/// Read a simple code (1 or 2 symbols).
fn read_simple_code(br: &mut BitReader, alphabet_size: usize) -> Result<HuffmanTable, String> {
    let num_symbols_minus_1 = br.read_bits(1);

    if num_symbols_minus_1 == 0 {
        // One symbol — trivial code.
        let symbol = br.read_bits(8) as usize;
        if symbol >= alphabet_size {
            return Err(format!(
                "VP8L: simple 1-symbol code: symbol {symbol} >= alphabet_size {alphabet_size}"
            ));
        }
        return Ok(HuffmanTable {
            max_code_len: 0,
            trivial_symbol: Some(symbol as u16),
            fast: vec![],
        });
    }

    // Two symbols.
    let symbol0_is_8bit = br.read_bits(1);
    let symbol0 = if symbol0_is_8bit == 0 {
        br.read_bits(1) as usize
    } else {
        br.read_bits(8) as usize
    };
    let symbol1 = br.read_bits(8) as usize;

    if symbol0 >= alphabet_size {
        return Err(format!(
            "VP8L: simple 2-symbol code: s0={symbol0} >= alphabet_size {alphabet_size}"
        ));
    }
    if symbol1 >= alphabet_size {
        return Err(format!(
            "VP8L: simple 2-symbol code: s1={symbol1} >= alphabet_size {alphabet_size}"
        ));
    }
    if symbol0 == symbol1 {
        return Err("VP8L: simple 2-symbol code has duplicate symbols".to_string());
    }

    let mut code_lengths = vec![0u32; alphabet_size];
    code_lengths[symbol0] = 1;
    code_lengths[symbol1] = 1;

    HuffmanTable::from_lengths(&code_lengths)
}

/// Read a complex code using the meta-Huffman scheme.
fn read_complex_code(
    br: &mut BitReader,
    alphabet_size: usize,
) -> Result<HuffmanTable, String> {
    // Read how many meta-code lengths are stored: 4 bits → count = value + 4.
    let num_stored = br.read_bits(4) as usize + 4;
    if num_stored > 19 {
        return Err(format!(
            "VP8L: num_code_lengths_to_store={num_stored} > 19"
        ));
    }

    // Read raw meta-code lengths (3 bits each) in CODE_LENGTH_ORDER.
    let mut meta_lengths = [0u32; 19];
    for i in 0..num_stored {
        let order_idx = CODE_LENGTH_ORDER[i];
        meta_lengths[order_idx] = br.read_bits(3);
    }

    // Build the meta-Huffman decode table.
    let meta_table = HuffmanTable::from_lengths(&meta_lengths)
        .map_err(|e| format!("VP8L: invalid meta-Huffman table: {e}"))?;

    // Decode actual code lengths using the meta-tree.
    let mut code_lengths = vec![0u32; alphabet_size];
    let mut i = 0;
    let mut last_non_zero = 8u32; // default used when RLE code 16 is first

    while i < alphabet_size {
        let meta_sym = meta_table.decode(br)
            .map_err(|e| format!("VP8L: error decoding code-length at i={i}: {e}"))?;

        match meta_sym {
            0..=15 => {
                code_lengths[i] = meta_sym as u32;
                if meta_sym > 0 {
                    last_non_zero = meta_sym as u32;
                }
                i += 1;
            }
            16 => {
                // Repeat last non-zero length 3..=6 times.
                let extra = br.read_bits(2);
                let count = (3 + extra) as usize;
                for _ in 0..count {
                    if i >= alphabet_size { break; }
                    code_lengths[i] = last_non_zero;
                    i += 1;
                }
            }
            17 => {
                // Repeat 0 length 3..=10 times.
                let extra = br.read_bits(3);
                let count = (3 + extra) as usize;
                i += count.min(alphabet_size - i);
            }
            18 => {
                // Repeat 0 length 11..=138 times.
                let extra = br.read_bits(7);
                let count = (11 + extra) as usize;
                i += count.min(alphabet_size - i);
            }
            _ => {
                return Err(format!("VP8L: unknown meta-symbol {meta_sym}"));
            }
        }
    }

    HuffmanTable::from_lengths(&code_lengths)
        .map_err(|e| format!("VP8L: failed to build Huffman table from decoded lengths: {e}"))
}

// ---------------------------------------------------------------------------
// Build code lengths from symbol frequencies
// ---------------------------------------------------------------------------

/// Compute canonical VP8L Huffman code lengths from symbol frequencies.
///
/// Uses the `huffman-tree` crate to build the optimal tree, then extracts
/// code lengths.  Symbols with zero frequency get length 0 (absent).
/// Lengths are capped at 15 bits (VP8L limit).
pub fn lengths_from_frequencies(freqs: &[u32]) -> Vec<u32> {
    let weights: Vec<(u16, u32)> = freqs
        .iter()
        .enumerate()
        .filter(|&(_, &f)| f > 0)
        .map(|(i, &f)| (i as u16, f))
        .collect();

    let mut lengths = vec![0u32; freqs.len()];

    if weights.is_empty() {
        return lengths;
    }
    if weights.len() == 1 {
        // Single active symbol: VP8L simple-1 code.
        // We give it length 1 so write_huffman_code sees 1 active symbol and
        // calls write_simple_1 with the correct symbol value.
        // The decoder reconstructs this as a trivial table via read_simple_code.
        lengths[weights[0].0 as usize] = 1;
        return lengths;
    }

    match huffman_tree::HuffmanTree::build(&weights) {
        Ok(tree) => {
            let table = tree.canonical_code_table();
            for (sym, code) in &table {
                let len = code.len() as u32;
                lengths[*sym as usize] = len.min(15);
            }
        }
        Err(_) => {
            // Fallback: uniform lengths.
            let bits = ((weights.len() as f64).log2().ceil() as u32).max(1).min(15);
            for &(sym, _) in &weights {
                lengths[sym as usize] = bits;
            }
        }
    }

    lengths
}

// ---------------------------------------------------------------------------
// Build encode table from code lengths
// ---------------------------------------------------------------------------

/// Build an encode table from canonical code lengths.
///
/// Returns a vector where `encode_table[symbol]` = `(bit_pattern, bit_count)`.
/// `bit_pattern` is the **bit-reversed canonical code** ready for `write_bits`.
/// For absent symbols (length 0), `bit_count = 0`.
pub fn build_encode_table(code_lengths: &[u32]) -> Vec<(u64, u32)> {
    let n = code_lengths.len();
    let mut table = vec![(0u64, 0u32); n];

    let mut active: Vec<(usize, u32)> = code_lengths
        .iter()
        .enumerate()
        .filter(|&(_, &l)| l > 0)
        .map(|(i, &l)| (i, l))
        .collect();

    if active.is_empty() {
        return table;
    }

    // Single-symbol trivial code: the VP8L simple-1 format means the decoder
    // returns the symbol without consuming any bits.  The encoder must likewise
    // emit 0 bits per occurrence of this symbol.
    if active.len() == 1 {
        // table[sym] already = (0, 0) → 0 bits emitted.  Nothing to do.
        return table;
    }

    active.sort_by_key(|&(sym, len)| (len, sym));

    let mut code = 0u32;
    let mut prev_len = active[0].1;
    for &(sym, len) in &active {
        if len > prev_len {
            code <<= len - prev_len;
        }
        let reversed = reverse_bits_n(code, len) as u64;
        table[sym] = (reversed, len);
        code += 1;
        prev_len = len;
    }

    table
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trivial_table_decodes_without_bits() {
        let table = HuffmanTable {
            max_code_len: 0,
            trivial_symbol: Some(42),
            fast: vec![],
        };
        let data = &[];
        let mut br = BitReader::new(data);
        assert_eq!(table.decode(&mut br).unwrap(), 42);
    }

    #[test]
    fn simple_1_round_trip() {
        let mut bw = BitWriter::new();
        write_simple_1(&mut bw, 100);
        let bytes = bw.finish();
        let mut br = BitReader::new(&bytes);
        let table = read_huffman_code(&mut br, 256).unwrap();
        assert_eq!(table.trivial_symbol, Some(100));
    }

    #[test]
    fn simple_2_round_trip_small_sym0() {
        let mut bw = BitWriter::new();
        write_simple_2(&mut bw, 1, 200);
        let bytes = bw.finish();
        let mut br = BitReader::new(&bytes);
        let table = read_huffman_code(&mut br, 256).unwrap();
        assert!(table.trivial_symbol.is_none());
        assert_eq!(table.max_code_len, 1);
    }

    #[test]
    fn simple_2_round_trip_large_sym0() {
        let mut bw = BitWriter::new();
        write_simple_2(&mut bw, 50, 200);
        let bytes = bw.finish();
        let mut br = BitReader::new(&bytes);
        let table = read_huffman_code(&mut br, 256).unwrap();
        assert!(table.trivial_symbol.is_none());
        assert_eq!(table.max_code_len, 1);
    }

    #[test]
    fn from_lengths_two_symbols_decodes() {
        // sym 50 and sym 100, both length 1.
        // Canonical: 50→0, 100→1.
        // Reversed (len=1): 50→0, 100→1.
        let mut lengths = vec![0u32; 256];
        lengths[50] = 1;
        lengths[100] = 1;
        let table = HuffmanTable::from_lengths(&lengths).unwrap();

        let mut bw = BitWriter::new();
        bw.write_bits(0, 1); // sym50 (reversed canonical 0)
        bw.write_bits(1, 1); // sym100 (reversed canonical 1)
        let bytes = bw.finish();

        let mut br = BitReader::new(&bytes);
        assert_eq!(table.decode(&mut br).unwrap(), 50);
        assert_eq!(table.decode(&mut br).unwrap(), 100);
    }

    #[test]
    fn from_lengths_three_symbols_decodes() {
        // Symbol 0 (len=1), symbol 1 (len=2), symbol 2 (len=2).
        // Canonical: 0→0b0=0, 1→0b10=2, 2→0b11=3.
        // Reversed:  0→0 (1 bit), 1→0b01=1 (2 bits), 2→0b11=3 (2 bits).
        let mut lengths = vec![0u32; 10];
        lengths[0] = 1;
        lengths[1] = 2;
        lengths[2] = 2;
        let table = HuffmanTable::from_lengths(&lengths).unwrap();
        assert_eq!(table.max_code_len, 2);

        let mut bw = BitWriter::new();
        bw.write_bits(0, 1); // sym0
        bw.write_bits(1, 2); // sym1 (reversed "10" = 0b01 = 1)
        bw.write_bits(3, 2); // sym2 (reversed "11" = 0b11 = 3)
        let bytes = bw.finish();

        let mut br = BitReader::new(&bytes);
        assert_eq!(table.decode(&mut br).unwrap(), 0);
        assert_eq!(table.decode(&mut br).unwrap(), 1);
        assert_eq!(table.decode(&mut br).unwrap(), 2);
    }

    #[test]
    fn build_encode_table_round_trip() {
        // 3 symbols: lengths [1, 2, 2].
        let mut lengths = vec![0u32; 10];
        lengths[0] = 1;
        lengths[1] = 2;
        lengths[2] = 2;

        let enc = build_encode_table(&lengths);
        let table = HuffmanTable::from_lengths(&lengths).unwrap();

        let symbols = [0usize, 1, 2, 0, 2, 1];
        let mut bw = BitWriter::new();
        for &sym in &symbols {
            let (bits, count) = enc[sym];
            if count > 0 {
                bw.write_bits(bits, count);
            }
        }
        let bytes = bw.finish();

        let mut br = BitReader::new(&bytes);
        for &expected in &symbols {
            assert_eq!(table.decode(&mut br).unwrap() as usize, expected);
        }
    }

    #[test]
    fn complex_code_round_trip() {
        // Build code lengths with > 2 active symbols.
        let mut lengths = vec![0u32; 256];
        lengths[0] = 1;
        lengths[1] = 2;
        lengths[2] = 3;
        lengths[3] = 3;

        let enc = build_encode_table(&lengths);

        let mut bw = BitWriter::new();
        write_huffman_code(&mut bw, &lengths);
        let bytes = bw.finish();

        let mut br = BitReader::new(&bytes);
        let table = read_huffman_code(&mut br, 256).unwrap();

        // Encode and decode all 4 symbols.
        let symbols = [0usize, 1, 2, 3];
        let mut bw2 = BitWriter::new();
        for &sym in &symbols {
            let (bits, count) = enc[sym];
            if count > 0 {
                bw2.write_bits(bits, count);
            }
        }
        let bytes2 = bw2.finish();

        let mut br2 = BitReader::new(&bytes2);
        for &expected in &symbols {
            assert_eq!(table.decode(&mut br2).unwrap() as usize, expected);
        }
    }

    #[test]
    fn lengths_from_frequencies_uniform() {
        let mut freqs = vec![0u32; 256];
        freqs[10] = 5;
        freqs[20] = 3;
        freqs[30] = 1;
        let lengths = lengths_from_frequencies(&freqs);
        assert_eq!(lengths.len(), 256);
        assert!(lengths[10] > 0);
        assert!(lengths[20] > 0);
        assert!(lengths[30] > 0);
        assert_eq!(lengths[0], 0);
    }

    #[test]
    fn reverse_bits_sanity() {
        assert_eq!(reverse_bits_n(0b110, 3), 0b011);
        assert_eq!(reverse_bits_n(0b1010, 4), 0b0101);
        assert_eq!(reverse_bits_n(1, 8), 0b10000000);
    }

    #[test]
    fn write_huffman_code_1_symbol_sets_simple_flag() {
        let lengths = vec![0u32; 256]; // all zero
        let mut bw = BitWriter::new();
        write_huffman_code(&mut bw, &lengths); // 0 active → write_simple_1(0)
        let bytes = bw.finish();
        let mut br = BitReader::new(&bytes);
        assert_eq!(br.read_bits(1), 1); // simple_flag
    }

    #[test]
    fn write_huffman_code_2_symbols_sets_simple_flag() {
        let mut lengths = vec![0u32; 256];
        lengths[10] = 1;
        lengths[20] = 1;
        let mut bw = BitWriter::new();
        write_huffman_code(&mut bw, &lengths);
        let bytes = bw.finish();
        let mut br = BitReader::new(&bytes);
        assert_eq!(br.read_bits(1), 1); // simple_flag
        assert_eq!(br.read_bits(1), 1); // 2 symbols
    }
}
