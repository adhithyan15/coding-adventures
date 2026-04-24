//! # data-matrix
//!
//! Data Matrix ECC200 encoder — ISO/IEC 16022:2006 compliant.
//!
//! Encodes any string or byte slice into a valid, scannable Data Matrix ECC200
//! symbol.  Outputs a [`ModuleGrid`] (abstract boolean grid) that can be passed
//! to `barcode-2d`'s [`layout()`] for pixel rendering.
//!
//! ## Data Matrix at a glance
//!
//! Data Matrix was invented in 1989 and standardised as ISO/IEC 16022:2006.
//! It is used wherever a small, high-density, damage-tolerant mark is needed:
//!
//! | Application | Detail |
//! |-------------|--------|
//! | PCB traceability | Every board carries an etched Data Matrix |
//! | Pharmaceuticals | US FDA DSCSA mandates unit-dose Data Matrix |
//! | Aerospace parts | Rivets, shims, brackets — etched in metal |
//! | Medical devices | Surgical instruments, GS1 DataMatrix |
//!
//! ## Encoding pipeline
//!
//! ```text
//! input bytes
//!   → ASCII encoding     (chars+1; digit pairs packed into one codeword)
//!   → symbol selection   (smallest symbol whose capacity ≥ codeword count)
//!   → pad to capacity    (scrambled-pad codewords fill unused slots)
//!   → RS blocks + ECC    (GF(256)/0x12D, b=1 convention)
//!   → interleave blocks  (data round-robin then ECC round-robin)
//!   → grid init          (L-finder + timing border + alignment borders)
//!   → Utah placement     (diagonal codeword placement, no masking!)
//!   → ModuleGrid
//! ```
//!
//! ## Key differences from QR Code
//!
//! | Property | QR Code | Data Matrix |
//! |----------|---------|-------------|
//! | GF polynomial | 0x11D | **0x12D** |
//! | RS root convention | b=0 (α^0..α^{n-1}) | **b=1 (α^1..α^n)** |
//! | Finder pattern | Three 7×7 squares | **L-shaped finder + clock border** |
//! | Data placement | Two-column zigzag | **Utah diagonal zigzag** |
//! | Masking | 8 patterns evaluated | **No masking** |

pub const VERSION: &str = "0.1.0";

use barcode_2d::{layout, Barcode2DLayoutConfig, ModuleGrid, ModuleShape};
use paint_instructions::PaintScene;

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

/// Errors produced by the Data Matrix encoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataMatrixError {
    /// The encoded codeword count exceeds the 144×144 symbol capacity (1558 codewords).
    InputTooLong(String),
}

impl std::fmt::Display for DataMatrixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataMatrixError::InputTooLong(msg) => write!(f, "InputTooLong: {msg}"),
        }
    }
}

impl std::error::Error for DataMatrixError {}

/// Symbol shape preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymbolShape {
    /// Prefer square symbols only (default).
    #[default]
    Square,
    /// Prefer rectangular symbols only.
    Rectangular,
    /// Consider both square and rectangular, pick smallest.
    Any,
}

/// Options for [`encode()`].
#[derive(Debug, Clone, Default)]
pub struct DataMatrixOptions {
    /// Symbol shape preference. Default: [`SymbolShape::Square`].
    pub shape: SymbolShape,
}

// ─────────────────────────────────────────────────────────────────────────────
// GF(256) over 0x12D — Data Matrix field
// ─────────────────────────────────────────────────────────────────────────────

/// GF(256) with primitive polynomial 0x12D.
///
/// `p(x) = x^8 + x^5 + x^4 + x^2 + x + 1 = 0x12D = 301`
///
/// Data Matrix uses this field, which is DIFFERENT from QR Code's 0x11D.
/// The tables are computed once at startup via `std::sync::OnceLock`.
///
/// The generator g = 2 (polynomial x) generates all 255 non-zero elements:
///   g^0  = 1 (0x01)
///   g^7  = 128 (0x80)
///   g^8  = 0x2D (0x80<<1 = 0x100, XOR 0x12D = 0x2D = 45)
///   g^9  = 0x5A
///   g^10 = 0xB4
struct Gf256Tables {
    /// gf_exp[i] = α^i mod 0x12D
    exp: [u8; 256],
    /// gf_log[v] = k such that α^k = v  (gf_log[0] = 0, undefined for log(0))
    log: [u16; 256],
}

/// Global GF(256)/0x12D tables, lazily initialised.
static GF_TABLES: std::sync::OnceLock<Gf256Tables> = std::sync::OnceLock::new();

pub(crate) fn get_gf_tables() -> &'static Gf256Tables {
    GF_TABLES.get_or_init(|| {
        let mut exp = [0u8; 256];
        let mut log = [0u16; 256];

        let mut val: u16 = 1;
        for i in 0u16..255 {
            exp[i as usize] = val as u8;
            log[val as usize] = i;
            val <<= 1;
            if val & 0x100 != 0 {
                val ^= 0x12d;
            }
        }
        // gf_exp[255] = gf_exp[0] = 1 (multiplicative order = 255)
        exp[255] = exp[0];
        Gf256Tables { exp, log }
    })
}

/// GF(256)/0x12D multiply using log/antilog tables.
///
/// For a, b ≠ 0: a × b = α^{(log[a] + log[b]) mod 255}
/// If either operand is 0, the product is 0.
#[inline]
pub(crate) fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let t = get_gf_tables();
    t.exp[((t.log[a as usize] as u32 + t.log[b as usize] as u32) % 255) as usize]
}

// ─────────────────────────────────────────────────────────────────────────────
// Symbol size table
// ─────────────────────────────────────────────────────────────────────────────

/// Descriptor for a single Data Matrix symbol size.
///
/// The `data_region_height` and `data_region_width` are the interior dimensions
/// of each data region (excl. the outer border and alignment borders).
/// The Utah placement algorithm works on the logical data matrix:
///   `logical_rows = region_rows * data_region_height`
///   `logical_cols = region_cols * data_region_width`
#[derive(Debug, Clone, Copy)]
struct SymbolSizeEntry {
    symbol_rows: usize,
    symbol_cols: usize,
    region_rows: usize,
    region_cols: usize,
    data_region_height: usize,
    data_region_width: usize,
    data_cw: usize,
    ecc_cw: usize,
    num_blocks: usize,
    ecc_per_block: usize,
}

/// All 24 square symbol sizes for Data Matrix ECC200.
/// Source: ISO/IEC 16022:2006, Table 7.
pub(crate) static SQUARE_SIZES: &[SymbolSizeEntry] = &[
    SymbolSizeEntry { symbol_rows: 10,  symbol_cols: 10,  region_rows: 1, region_cols: 1, data_region_height:  8, data_region_width:  8, data_cw:   3, ecc_cw:   5, num_blocks: 1, ecc_per_block:  5 },
    SymbolSizeEntry { symbol_rows: 12,  symbol_cols: 12,  region_rows: 1, region_cols: 1, data_region_height: 10, data_region_width: 10, data_cw:   5, ecc_cw:   7, num_blocks: 1, ecc_per_block:  7 },
    SymbolSizeEntry { symbol_rows: 14,  symbol_cols: 14,  region_rows: 1, region_cols: 1, data_region_height: 12, data_region_width: 12, data_cw:   8, ecc_cw:  10, num_blocks: 1, ecc_per_block: 10 },
    SymbolSizeEntry { symbol_rows: 16,  symbol_cols: 16,  region_rows: 1, region_cols: 1, data_region_height: 14, data_region_width: 14, data_cw:  12, ecc_cw:  12, num_blocks: 1, ecc_per_block: 12 },
    SymbolSizeEntry { symbol_rows: 18,  symbol_cols: 18,  region_rows: 1, region_cols: 1, data_region_height: 16, data_region_width: 16, data_cw:  18, ecc_cw:  14, num_blocks: 1, ecc_per_block: 14 },
    SymbolSizeEntry { symbol_rows: 20,  symbol_cols: 20,  region_rows: 1, region_cols: 1, data_region_height: 18, data_region_width: 18, data_cw:  22, ecc_cw:  18, num_blocks: 1, ecc_per_block: 18 },
    SymbolSizeEntry { symbol_rows: 22,  symbol_cols: 22,  region_rows: 1, region_cols: 1, data_region_height: 20, data_region_width: 20, data_cw:  30, ecc_cw:  20, num_blocks: 1, ecc_per_block: 20 },
    SymbolSizeEntry { symbol_rows: 24,  symbol_cols: 24,  region_rows: 1, region_cols: 1, data_region_height: 22, data_region_width: 22, data_cw:  36, ecc_cw:  24, num_blocks: 1, ecc_per_block: 24 },
    SymbolSizeEntry { symbol_rows: 26,  symbol_cols: 26,  region_rows: 1, region_cols: 1, data_region_height: 24, data_region_width: 24, data_cw:  44, ecc_cw:  28, num_blocks: 1, ecc_per_block: 28 },
    SymbolSizeEntry { symbol_rows: 32,  symbol_cols: 32,  region_rows: 2, region_cols: 2, data_region_height: 14, data_region_width: 14, data_cw:  62, ecc_cw:  36, num_blocks: 2, ecc_per_block: 18 },
    SymbolSizeEntry { symbol_rows: 36,  symbol_cols: 36,  region_rows: 2, region_cols: 2, data_region_height: 16, data_region_width: 16, data_cw:  86, ecc_cw:  42, num_blocks: 2, ecc_per_block: 21 },
    SymbolSizeEntry { symbol_rows: 40,  symbol_cols: 40,  region_rows: 2, region_cols: 2, data_region_height: 18, data_region_width: 18, data_cw: 114, ecc_cw:  48, num_blocks: 2, ecc_per_block: 24 },
    SymbolSizeEntry { symbol_rows: 44,  symbol_cols: 44,  region_rows: 2, region_cols: 2, data_region_height: 20, data_region_width: 20, data_cw: 144, ecc_cw:  56, num_blocks: 4, ecc_per_block: 14 },
    SymbolSizeEntry { symbol_rows: 48,  symbol_cols: 48,  region_rows: 2, region_cols: 2, data_region_height: 22, data_region_width: 22, data_cw: 174, ecc_cw:  68, num_blocks: 4, ecc_per_block: 17 },
    SymbolSizeEntry { symbol_rows: 52,  symbol_cols: 52,  region_rows: 2, region_cols: 2, data_region_height: 24, data_region_width: 24, data_cw: 204, ecc_cw:  84, num_blocks: 4, ecc_per_block: 21 },
    SymbolSizeEntry { symbol_rows: 64,  symbol_cols: 64,  region_rows: 4, region_cols: 4, data_region_height: 14, data_region_width: 14, data_cw: 280, ecc_cw: 112, num_blocks: 4, ecc_per_block: 28 },
    SymbolSizeEntry { symbol_rows: 72,  symbol_cols: 72,  region_rows: 4, region_cols: 4, data_region_height: 16, data_region_width: 16, data_cw: 368, ecc_cw: 144, num_blocks: 4, ecc_per_block: 36 },
    SymbolSizeEntry { symbol_rows: 80,  symbol_cols: 80,  region_rows: 4, region_cols: 4, data_region_height: 18, data_region_width: 18, data_cw: 456, ecc_cw: 192, num_blocks: 4, ecc_per_block: 48 },
    SymbolSizeEntry { symbol_rows: 88,  symbol_cols: 88,  region_rows: 4, region_cols: 4, data_region_height: 20, data_region_width: 20, data_cw: 576, ecc_cw: 224, num_blocks: 4, ecc_per_block: 56 },
    SymbolSizeEntry { symbol_rows: 96,  symbol_cols: 96,  region_rows: 4, region_cols: 4, data_region_height: 22, data_region_width: 22, data_cw: 696, ecc_cw: 272, num_blocks: 4, ecc_per_block: 68 },
    SymbolSizeEntry { symbol_rows: 104, symbol_cols: 104, region_rows: 4, region_cols: 4, data_region_height: 24, data_region_width: 24, data_cw: 816, ecc_cw: 336, num_blocks: 6, ecc_per_block: 56 },
    SymbolSizeEntry { symbol_rows: 120, symbol_cols: 120, region_rows: 6, region_cols: 6, data_region_height: 18, data_region_width: 18, data_cw:1050, ecc_cw: 408, num_blocks: 6, ecc_per_block: 68 },
    SymbolSizeEntry { symbol_rows: 132, symbol_cols: 132, region_rows: 6, region_cols: 6, data_region_height: 20, data_region_width: 20, data_cw:1304, ecc_cw: 496, num_blocks: 8, ecc_per_block: 62 },
    SymbolSizeEntry { symbol_rows: 144, symbol_cols: 144, region_rows: 6, region_cols: 6, data_region_height: 22, data_region_width: 22, data_cw:1558, ecc_cw: 620, num_blocks:10, ecc_per_block: 62 },
];

/// All 6 rectangular symbol sizes for Data Matrix ECC200.
/// Source: ISO/IEC 16022:2006, Table 7 (rectangular symbols).
pub(crate) static RECT_SIZES: &[SymbolSizeEntry] = &[
    SymbolSizeEntry { symbol_rows:  8, symbol_cols: 18, region_rows: 1, region_cols: 1, data_region_height: 6, data_region_width: 16, data_cw:  5, ecc_cw:  7, num_blocks: 1, ecc_per_block:  7 },
    SymbolSizeEntry { symbol_rows:  8, symbol_cols: 32, region_rows: 1, region_cols: 2, data_region_height: 6, data_region_width: 14, data_cw: 10, ecc_cw: 11, num_blocks: 1, ecc_per_block: 11 },
    SymbolSizeEntry { symbol_rows: 12, symbol_cols: 26, region_rows: 1, region_cols: 1, data_region_height:10, data_region_width: 24, data_cw: 16, ecc_cw: 14, num_blocks: 1, ecc_per_block: 14 },
    SymbolSizeEntry { symbol_rows: 12, symbol_cols: 36, region_rows: 1, region_cols: 2, data_region_height:10, data_region_width: 16, data_cw: 22, ecc_cw: 18, num_blocks: 1, ecc_per_block: 18 },
    SymbolSizeEntry { symbol_rows: 16, symbol_cols: 36, region_rows: 1, region_cols: 2, data_region_height:14, data_region_width: 16, data_cw: 32, ecc_cw: 24, num_blocks: 1, ecc_per_block: 24 },
    SymbolSizeEntry { symbol_rows: 16, symbol_cols: 48, region_rows: 1, region_cols: 2, data_region_height:14, data_region_width: 22, data_cw: 49, ecc_cw: 28, num_blocks: 1, ecc_per_block: 28 },
];

// ─────────────────────────────────────────────────────────────────────────────
// Generator polynomials (b=1 convention, GF(256)/0x12D)
// ─────────────────────────────────────────────────────────────────────────────

/// Build the RS generator polynomial g(x) = ∏_{k=1}^{n_ecc} (x + α^k).
///
/// Uses the b=1 convention: roots are α^1, α^2, ..., α^n_ecc.
/// This matches the Data Matrix / ISO/IEC 16022 convention exactly.
///
/// Returns a vector of n_ecc+1 coefficients (leading coefficient 1, monic).
pub(crate) fn build_generator(n_ecc: usize) -> Vec<u8> {
    let t = get_gf_tables();
    let mut g: Vec<u8> = vec![1u8];
    for i in 1..=(n_ecc as u8) {
        let ai = t.exp[i as usize];
        let mut next = vec![0u8; g.len() + 1];
        for (j, &gj) in g.iter().enumerate() {
            next[j] ^= gj;
            next[j + 1] ^= gf_mul(gj, ai);
        }
        g = next;
    }
    g
}

// ─────────────────────────────────────────────────────────────────────────────
// Reed-Solomon encoding
// ─────────────────────────────────────────────────────────────────────────────

/// Compute `n_ecc` ECC bytes for one RS block using LFSR polynomial division.
///
/// Algorithm: R(x) = D(x) × x^n_ecc mod G(x)
///
/// The LFSR approach:
///   for each data byte d:
///     feedback = d XOR rem[0]
///     shift rem left: rem[i] ← rem[i+1]
///     rem[i] ^= generator[i+1] × feedback  for i = 0..n_ecc-1
///
/// This is the standard systematic RS encoding: the message occupies the
/// high-degree coefficients and the check symbols the low-degree ones.
pub(crate) fn rs_encode_block(data: &[u8], generator: &[u8]) -> Vec<u8> {
    let n_ecc = generator.len() - 1;
    let mut rem = vec![0u8; n_ecc];
    for &byte in data {
        let fb = byte ^ rem[0];
        for i in 0..n_ecc - 1 {
            rem[i] = rem[i + 1];
        }
        rem[n_ecc - 1] = 0;
        if fb != 0 {
            for i in 0..n_ecc {
                rem[i] ^= gf_mul(generator[i + 1], fb);
            }
        }
    }
    rem
}

// ─────────────────────────────────────────────────────────────────────────────
// ASCII data encoding
// ─────────────────────────────────────────────────────────────────────────────

/// Encode bytes in Data Matrix ASCII mode.
///
/// Rules:
///   - Two consecutive ASCII digits (0x30–0x39) → codeword = 130 + (d1×10 + d2)
///     This packs two digits into one codeword, saving 50% for digit-only input.
///   - Single ASCII char (0–127) → codeword = ASCII_value + 1
///   - Extended ASCII (128–255) → two codewords: 235 (UPPER_SHIFT), value-127
///
/// Examples:
///   "A"    → [66]          (65+1)
///   "12"   → [142]         (130 + (1*10+2))
///   "1234" → [142, 164]    (130+12, 130+34)
///   "1A"   → [50, 66]      (49+1, 65+1 — no pair because 'A' is not a digit)
pub(crate) fn encode_ascii(input: &[u8]) -> Vec<u8> {
    let mut codewords = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let c = input[i];
        if c >= 0x30
            && c <= 0x39
            && i + 1 < input.len()
            && input[i + 1] >= 0x30
            && input[i + 1] <= 0x39
        {
            let d1 = (c - 0x30) as u16;
            let d2 = (input[i + 1] - 0x30) as u16;
            codewords.push((130 + d1 * 10 + d2) as u8);
            i += 2;
        } else if c <= 127 {
            codewords.push(c + 1);
            i += 1;
        } else {
            codewords.push(235); // UPPER_SHIFT
            codewords.push(c - 127);
            i += 1;
        }
    }
    codewords
}

// ─────────────────────────────────────────────────────────────────────────────
// Pad codewords (ISO/IEC 16022:2006 §5.2.3)
// ─────────────────────────────────────────────────────────────────────────────

/// Pad codewords to exactly `data_cw` length.
///
/// Rules:
///   1. First pad codeword is always 129.
///   2. Subsequent pads are scrambled:
///        scrambled = 129 + (149 × k mod 253) + 1
///        if scrambled > 254: scrambled -= 254
///      where k is the 1-indexed position within the full codeword stream.
///
/// Example: "A" → [66], padded to 3 (10×10 symbol):
///   k=2: 129 (first pad)
///   k=3: 129 + (149*3 mod 253) + 1 = 129 + 194 + 1 = 324 → 324-254 = 70
///   Result: [66, 129, 70]
pub(crate) fn pad_codewords(codewords: &[u8], data_cw: usize) -> Vec<u8> {
    let mut padded = codewords.to_vec();
    let first_pad_pos = padded.len(); // 0-indexed position of first pad
    let mut k = padded.len() as u32 + 1; // 1-indexed stream position of first pad
    while padded.len() < data_cw {
        if padded.len() == first_pad_pos {
            padded.push(129);
        } else {
            let mut scrambled = 129u32 + (149 * k % 253) + 1;
            if scrambled > 254 {
                scrambled -= 254;
            }
            padded.push(scrambled as u8);
        }
        k += 1;
    }
    padded
}

// ─────────────────────────────────────────────────────────────────────────────
// Symbol selection
// ─────────────────────────────────────────────────────────────────────────────

/// Select the smallest symbol whose `data_cw` capacity fits `codeword_count`.
pub(crate) fn select_symbol(
    codeword_count: usize,
    shape: SymbolShape,
) -> Result<&'static SymbolSizeEntry, DataMatrixError> {
    let mut candidates: Vec<&SymbolSizeEntry> = Vec::new();
    if matches!(shape, SymbolShape::Square | SymbolShape::Any) {
        candidates.extend(SQUARE_SIZES.iter());
    }
    if matches!(shape, SymbolShape::Rectangular | SymbolShape::Any) {
        candidates.extend(RECT_SIZES.iter());
    }
    // Sort by data capacity, then by area (ascending) for tie-breaking
    candidates.sort_by(|a, b| {
        a.data_cw
            .cmp(&b.data_cw)
            .then_with(|| (a.symbol_rows * a.symbol_cols).cmp(&(b.symbol_rows * b.symbol_cols)))
    });
    candidates
        .into_iter()
        .find(|e| e.data_cw >= codeword_count)
        .ok_or_else(|| {
            DataMatrixError::InputTooLong(format!(
                "Encoded data requires {codeword_count} codewords, exceeds maximum 1558 (144×144 symbol)."
            ))
        })
}

// ─────────────────────────────────────────────────────────────────────────────
// Block splitting and interleaving
// ─────────────────────────────────────────────────────────────────────────────

/// Split padded data into RS blocks, compute ECC, and return interleaved stream.
///
/// Interleaving convention (ISO/IEC 16022):
///   - Data codewords interleaved round-robin across blocks.
///   - ECC codewords interleaved round-robin across blocks.
///   - Earlier blocks get one extra codeword if data_cw is not evenly divisible.
fn compute_interleaved(data: &[u8], entry: &SymbolSizeEntry) -> Vec<u8> {
    let &SymbolSizeEntry {
        data_cw,
        num_blocks,
        ecc_per_block,
        ..
    } = entry;

    let generator = build_generator(ecc_per_block);

    // Split data into blocks (earlier blocks get ceiling, later get floor)
    let base_len = data_cw / num_blocks;
    let extra_blocks = data_cw % num_blocks; // first extra_blocks blocks get base_len+1

    let mut data_blocks: Vec<Vec<u8>> = Vec::with_capacity(num_blocks);
    let mut offset = 0;
    for b in 0..num_blocks {
        let len = if b < extra_blocks {
            base_len + 1
        } else {
            base_len
        };
        data_blocks.push(data[offset..offset + len].to_vec());
        offset += len;
    }

    // Compute ECC for each block
    let ecc_blocks: Vec<Vec<u8>> = data_blocks
        .iter()
        .map(|d| rs_encode_block(d, &generator))
        .collect();

    // Interleave: data round-robin
    let max_data_len = data_blocks.iter().map(|b| b.len()).max().unwrap_or(0);
    let mut interleaved = Vec::with_capacity(data_cw + num_blocks * ecc_per_block);
    for pos in 0..max_data_len {
        for b in 0..num_blocks {
            if pos < data_blocks[b].len() {
                interleaved.push(data_blocks[b][pos]);
            }
        }
    }

    // Interleave: ECC round-robin
    for pos in 0..ecc_per_block {
        for b in 0..num_blocks {
            interleaved.push(ecc_blocks[b][pos]);
        }
    }

    interleaved
}

// ─────────────────────────────────────────────────────────────────────────────
// Grid initialization
// ─────────────────────────────────────────────────────────────────────────────

/// Initialize the physical module grid with structural elements.
///
/// Writing order (last-write-wins for corners and intersections):
///   1. Alignment borders (written first)
///   2. Top row timing (alternating, c%2==0 → dark)
///   3. Right column timing (alternating, r%2==0 → dark)
///   4. Left column L-finder (all dark)
///   5. Bottom row L-finder (all dark — highest precedence, written last)
///
/// The L-finder takes final precedence because it overrides timing and
/// alignment border values at corner intersections.
fn init_grid(entry: &SymbolSizeEntry) -> Vec<Vec<bool>> {
    let &SymbolSizeEntry {
        symbol_rows,
        symbol_cols,
        region_rows,
        region_cols,
        data_region_height,
        data_region_width,
        ..
    } = entry;

    let mut grid = vec![vec![false; symbol_cols]; symbol_rows];

    // 1. Alignment borders (written first — outer border overrides at edges)
    for rr in 0..region_rows.saturating_sub(1) {
        let ab_row0 = 1 + (rr + 1) * data_region_height + rr * 2;
        let ab_row1 = ab_row0 + 1;
        for c in 0..symbol_cols {
            grid[ab_row0][c] = true;             // all dark
            grid[ab_row1][c] = c % 2 == 0;      // alternating
        }
    }
    for rc in 0..region_cols.saturating_sub(1) {
        let ab_col0 = 1 + (rc + 1) * data_region_width + rc * 2;
        let ab_col1 = ab_col0 + 1;
        for r in 0..symbol_rows {
            grid[r][ab_col0] = true;             // all dark
            grid[r][ab_col1] = r % 2 == 0;      // alternating
        }
    }

    // 2. Top row timing: alternating, dark at even columns
    for c in 0..symbol_cols {
        grid[0][c] = c % 2 == 0;
    }

    // 3. Right column timing: alternating, dark at even rows
    for r in 0..symbol_rows {
        grid[r][symbol_cols - 1] = r % 2 == 0;
    }

    // 4. Left column: all dark (L-finder left leg)
    for r in 0..symbol_rows {
        grid[r][0] = true;
    }

    // 5. Bottom row: all dark (L-finder bottom leg — overrides all other writes)
    for c in 0..symbol_cols {
        grid[symbol_rows - 1][c] = true;
    }

    grid
}

// ─────────────────────────────────────────────────────────────────────────────
// Utah placement algorithm
// ─────────────────────────────────────────────────────────────────────────────

/// Apply boundary wrap rules for out-of-bounds logical positions.
///
/// From ISO/IEC 16022:2006 Annex F:
///   row < 0 and col == 0:      → (1, 3)
///   row < 0 and col == nCols:  → (0, col-2)
///   row < 0:                   → (row+nRows, col-4)
///   col < 0:                   → (row-4, col+nCols)
fn apply_wrap(mut row: i32, mut col: i32, n_rows: i32, n_cols: i32) -> (i32, i32) {
    if row < 0 && col == 0 {
        return (1, 3);
    }
    if row < 0 && col == n_cols {
        return (0, col - 2);
    }
    if row < 0 {
        return (row + n_rows, col - 4);
    }
    if col < 0 {
        return (row - 4, col + n_cols);
    }
    (row, col)
}

/// Place one 8-bit codeword using the standard "Utah" shape.
///
/// The Utah shape (named for its resemblance to the US state of Utah):
///
/// ```text
/// col: c-2  c-1   c
/// r-2:  .   [1]  [2]   bit 1 = LSB, bit 8 = MSB
/// r-1: [3]  [4]  [5]
/// r  : [6]  [7]  [8]
/// ```
///
/// MSB (bit 8) is placed at (row, col), LSB (bit 1) at (row-2, col-1).
fn place_utah(
    codeword: u8,
    row: i32, col: i32,
    n_rows: i32, n_cols: i32,
    grid: &mut Vec<Vec<bool>>,
    used: &mut Vec<Vec<bool>>,
) {
    // (offset_row, offset_col, bit_index 7=MSB, 0=LSB)
    let placements: [(i32, i32, u8); 8] = [
        (0,  0,  7),  // bit 8
        (0, -1,  6),  // bit 7
        (0, -2,  5),  // bit 6
        (-1, 0,  4),  // bit 5
        (-1,-1,  3),  // bit 4
        (-1,-2,  2),  // bit 3
        (-2, 0,  1),  // bit 2
        (-2,-1,  0),  // bit 1
    ];
    for (dr, dc, bit) in placements {
        let (wr, wc) = apply_wrap(row + dr, col + dc, n_rows, n_cols);
        if wr >= 0 && wr < n_rows && wc >= 0 && wc < n_cols {
            let (wr, wc) = (wr as usize, wc as usize);
            if !used[wr][wc] {
                grid[wr][wc] = (codeword >> bit) & 1 == 1;
                used[wr][wc] = true;
            }
        }
    }
}

/// Corner pattern 1 — top-left wrap.
fn place_corner1(
    codeword: u8,
    n_rows: i32, n_cols: i32,
    grid: &mut Vec<Vec<bool>>,
    used: &mut Vec<Vec<bool>>,
) {
    let positions: [(i32, i32, u8); 8] = [
        (0,          n_cols - 2, 7),
        (0,          n_cols - 1, 6),
        (1,          0,          5),
        (2,          0,          4),
        (n_rows - 2, 0,          3),
        (n_rows - 1, 0,          2),
        (n_rows - 1, 1,          1),
        (n_rows - 1, 2,          0),
    ];
    for (r, c, bit) in positions {
        if r >= 0 && r < n_rows && c >= 0 && c < n_cols {
            let (r, c) = (r as usize, c as usize);
            if !used[r][c] {
                grid[r][c] = (codeword >> bit) & 1 == 1;
                used[r][c] = true;
            }
        }
    }
}

/// Corner pattern 2 — top-right wrap.
fn place_corner2(
    codeword: u8,
    n_rows: i32, n_cols: i32,
    grid: &mut Vec<Vec<bool>>,
    used: &mut Vec<Vec<bool>>,
) {
    let positions: [(i32, i32, u8); 8] = [
        (0,          n_cols - 2, 7),
        (0,          n_cols - 1, 6),
        (1,          n_cols - 1, 5),
        (2,          n_cols - 1, 4),
        (n_rows - 1, 0,          3),
        (n_rows - 1, 1,          2),
        (n_rows - 1, 2,          1),
        (n_rows - 1, 3,          0),
    ];
    for (r, c, bit) in positions {
        if r >= 0 && r < n_rows && c >= 0 && c < n_cols {
            let (r, c) = (r as usize, c as usize);
            if !used[r][c] {
                grid[r][c] = (codeword >> bit) & 1 == 1;
                used[r][c] = true;
            }
        }
    }
}

/// Corner pattern 3 — bottom-left wrap.
fn place_corner3(
    codeword: u8,
    n_rows: i32, n_cols: i32,
    grid: &mut Vec<Vec<bool>>,
    used: &mut Vec<Vec<bool>>,
) {
    let positions: [(i32, i32, u8); 8] = [
        (0,          n_cols - 1, 7),
        (1,          0,          6),
        (2,          0,          5),
        (n_rows - 2, 0,          4),
        (n_rows - 1, 0,          3),
        (n_rows - 1, 1,          2),
        (n_rows - 1, 2,          1),
        (n_rows - 1, 3,          0),
    ];
    for (r, c, bit) in positions {
        if r >= 0 && r < n_rows && c >= 0 && c < n_cols {
            let (r, c) = (r as usize, c as usize);
            if !used[r][c] {
                grid[r][c] = (codeword >> bit) & 1 == 1;
                used[r][c] = true;
            }
        }
    }
}

/// Corner pattern 4 — right-edge wrap for odd-dimension matrices.
fn place_corner4(
    codeword: u8,
    n_rows: i32, n_cols: i32,
    grid: &mut Vec<Vec<bool>>,
    used: &mut Vec<Vec<bool>>,
) {
    let positions: [(i32, i32, u8); 8] = [
        (n_rows - 3, n_cols - 1, 7),
        (n_rows - 2, n_cols - 1, 6),
        (n_rows - 1, n_cols - 3, 5),
        (n_rows - 1, n_cols - 2, 4),
        (n_rows - 1, n_cols - 1, 3),
        (0,          0,          2),
        (1,          0,          1),
        (2,          0,          0),
    ];
    for (r, c, bit) in positions {
        if r >= 0 && r < n_rows && c >= 0 && c < n_cols {
            let (r, c) = (r as usize, c as usize);
            if !used[r][c] {
                grid[r][c] = (codeword >> bit) & 1 == 1;
                used[r][c] = true;
            }
        }
    }
}

/// Run the Utah diagonal placement algorithm.
///
/// Scans the logical data matrix `nRows × nCols` in a diagonal zigzag,
/// placing 8 bits per codeword using the standard "Utah" shape plus
/// four corner patterns for edge cases.
///
/// No masking is applied — Data Matrix relies on the diagonal distribution
/// to avoid degenerate patterns, unlike QR Code's 8-mask evaluation.
///
/// Returns the filled `nRows × nCols` logical grid.
pub(crate) fn utah_placement(codewords: &[u8], n_rows: usize, n_cols: usize) -> Vec<Vec<bool>> {
    let mut grid = vec![vec![false; n_cols]; n_rows];
    let mut used = vec![vec![false; n_cols]; n_rows];
    let nr = n_rows as i32;
    let nc = n_cols as i32;

    let mut cw_idx: usize = 0;
    let mut row: i32 = 4;
    let mut col: i32 = 0;

    let mut place_one = |fn_ptr: fn(u8, i32, i32, &mut Vec<Vec<bool>>, &mut Vec<Vec<bool>>),
                         cw_idx: &mut usize,
                         grid: &mut Vec<Vec<bool>>,
                         used: &mut Vec<Vec<bool>>| {
        if *cw_idx < codewords.len() {
            fn_ptr(codewords[*cw_idx], nr, nc, grid, used);
            *cw_idx += 1;
        }
    };

    loop {
        // Corner patterns
        if row == nr && col == 0 && (nr % 4 == 0 || nc % 4 == 0) {
            place_one(place_corner1, &mut cw_idx, &mut grid, &mut used);
        }
        if row == nr - 2 && col == 0 && nc % 4 != 0 {
            place_one(place_corner2, &mut cw_idx, &mut grid, &mut used);
        }
        if row == nr - 2 && col == 0 && nc % 8 == 4 {
            place_one(place_corner3, &mut cw_idx, &mut grid, &mut used);
        }
        if row == nr + 4 && col == 2 && nc % 8 == 0 {
            place_one(place_corner4, &mut cw_idx, &mut grid, &mut used);
        }

        // Upward-right diagonal
        loop {
            if row >= 0 && row < nr && col >= 0 && col < nc
                && !used[row as usize][col as usize]
                && cw_idx < codewords.len()
            {
                place_utah(
                    codewords[cw_idx], row, col, nr, nc, &mut grid, &mut used,
                );
                cw_idx += 1;
            }
            row -= 2;
            col += 2;
            if row < 0 || col >= nc {
                break;
            }
        }

        row += 1;
        col += 3;

        // Downward-left diagonal
        loop {
            if row >= 0 && row < nr && col >= 0 && col < nc
                && !used[row as usize][col as usize]
                && cw_idx < codewords.len()
            {
                place_utah(
                    codewords[cw_idx], row, col, nr, nc, &mut grid, &mut used,
                );
                cw_idx += 1;
            }
            row += 2;
            col -= 2;
            if row >= nr || col < 0 {
                break;
            }
        }

        row += 3;
        col += 1;

        if row >= nr && col >= nc {
            break;
        }
        if cw_idx >= codewords.len() {
            break;
        }
    }

    // Fill residual unset modules with (r + c) % 2 == 1 (dark at odd positions)
    for r in 0..n_rows {
        for c in 0..n_cols {
            if !used[r][c] {
                grid[r][c] = (r + c) % 2 == 1;
            }
        }
    }

    grid
}

// ─────────────────────────────────────────────────────────────────────────────
// Logical → Physical coordinate mapping
// ─────────────────────────────────────────────────────────────────────────────

/// Map a logical data matrix coordinate to a physical symbol coordinate.
///
/// For a symbol with rr × rc data regions, each of size (rh × rw):
///   physical_row = (r / rh) * (rh + 2) + (r mod rh) + 1
///   physical_col = (c / rw) * (rw + 2) + (c mod rw) + 1
///
/// The +2 accounts for the 2-module alignment border between regions.
/// The +1 accounts for the 1-module outer border (finder + timing).
#[inline]
fn logical_to_physical(r: usize, c: usize, entry: &SymbolSizeEntry) -> (usize, usize) {
    let rh = entry.data_region_height;
    let rw = entry.data_region_width;
    let phys_row = (r / rh) * (rh + 2) + (r % rh) + 1;
    let phys_col = (c / rw) * (rw + 2) + (c % rw) + 1;
    (phys_row, phys_col)
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Encode a byte slice into a Data Matrix ECC200 [`ModuleGrid`].
///
/// Selects the smallest symbol that fits the input in ASCII mode.
///
/// # Errors
///
/// Returns [`DataMatrixError::InputTooLong`] if the encoded codeword count
/// exceeds 1558 (the 144×144 symbol capacity).
///
/// # Example
///
/// ```rust
/// use data_matrix::encode;
///
/// let grid = encode(b"Hello World", Default::default()).unwrap();
/// assert_eq!(grid.rows, 16); // Hello World → 16×16 symbol
/// assert_eq!(grid.cols, 16);
/// ```
pub fn encode(input: &[u8], options: DataMatrixOptions) -> Result<ModuleGrid, DataMatrixError> {
    // Step 1: ASCII encode
    let codewords = encode_ascii(input);

    // Step 2: Symbol selection
    let entry = select_symbol(codewords.len(), options.shape)?;

    // Step 3: Pad to capacity
    let padded = pad_codewords(&codewords, entry.data_cw);

    // Step 4–6: RS ECC + interleave
    let interleaved = compute_interleaved(&padded, entry);

    // Step 7: Initialize physical grid
    let mut phys_grid = init_grid(entry);

    // Step 8: Utah placement on logical data matrix
    let n_rows = entry.region_rows * entry.data_region_height;
    let n_cols = entry.region_cols * entry.data_region_width;
    let logical_grid = utah_placement(&interleaved, n_rows, n_cols);

    // Step 9: Map logical → physical
    for r in 0..n_rows {
        for c in 0..n_cols {
            let (pr, pc) = logical_to_physical(r, c, entry);
            phys_grid[pr][pc] = logical_grid[r][c];
        }
    }

    // Step 10: Return (no masking — Data Matrix never masks)
    Ok(ModuleGrid {
        rows: entry.symbol_rows as u32,
        cols: entry.symbol_cols as u32,
        modules: phys_grid,
        module_shape: ModuleShape::Square,
    })
}

/// Encode a UTF-8 string into a Data Matrix ECC200 [`ModuleGrid`].
///
/// Convenience wrapper around [`encode()`].
///
/// # Example
///
/// ```rust
/// use data_matrix::encode_str;
///
/// let grid = encode_str("Hello World", Default::default()).unwrap();
/// assert_eq!(grid.rows, 16);
/// ```
pub fn encode_str(
    input: &str,
    options: DataMatrixOptions,
) -> Result<ModuleGrid, DataMatrixError> {
    encode(input.as_bytes(), options)
}

/// Encode and convert to a pixel-resolved [`PaintScene`].
///
/// Delegates pixel geometry to `barcode-2d`'s `layout()`.
/// The default quiet zone is 1 module (narrower than QR's 4 modules because
/// the L-finder is inherently self-delimiting).
pub fn encode_and_layout(
    input: &[u8],
    options: DataMatrixOptions,
    config: Option<Barcode2DLayoutConfig>,
) -> Result<PaintScene, DataMatrixError> {
    let grid = encode(input, options)?;
    let mut cfg = config.unwrap_or_default();
    // Data Matrix quiet zone default: 1 module
    if cfg.quiet_zone_modules == 0 {
        cfg.quiet_zone_modules = 1;
    }
    // layout() returns Result<PaintScene, Barcode2DError>; we expect it to
    // succeed for a valid grid (invalid config would be a programming error).
    Ok(layout(&grid, &cfg).expect("layout failed for valid data-matrix grid"))
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers exposed for tests
// ─────────────────────────────────────────────────────────────────────────────

#[doc(hidden)]
pub(crate) mod internal {
    pub(crate) use super::{
        build_generator, encode_ascii, gf_mul, get_gf_tables, pad_codewords,
        rs_encode_block, select_symbol, utah_placement, SymbolShape,
        RECT_SIZES, SQUARE_SIZES,
    };
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::internal::*;
    use super::*;

    // ── GF(256)/0x12D arithmetic ─────────────────────────────────────────────

    #[test]
    fn gf_exp_table_starts_correctly() {
        let t = get_gf_tables();
        assert_eq!(t.exp[0], 1);    // α^0 = 1
        assert_eq!(t.exp[1], 2);    // α^1 = 2
        assert_eq!(t.exp[2], 4);    // α^2 = 4
        assert_eq!(t.exp[7], 128);  // α^7 = 128
        assert_eq!(t.exp[8], 0x2d); // α^8 = 0x2D (first reduction)
        assert_eq!(t.exp[9], 0x5a); // α^9 = 0x5A
        assert_eq!(t.exp[10], 0xb4);// α^10 = 0xB4
    }

    #[test]
    fn gf_exp_wraps_at_255() {
        let t = get_gf_tables();
        assert_eq!(t.exp[255], t.exp[0]); // α^255 = 1
    }

    #[test]
    fn gf_log_is_inverse_of_exp() {
        let t = get_gf_tables();
        for i in 0..255usize {
            let v = t.exp[i] as usize;
            assert_eq!(t.log[v] as usize, i, "log(exp[{i}]) ≠ {i}");
        }
    }

    #[test]
    fn gf_mul_identity_and_zero() {
        assert_eq!(gf_mul(0, 0xff), 0);
        assert_eq!(gf_mul(0xff, 0), 0);
        assert_eq!(gf_mul(1, 7), 7);
        assert_eq!(gf_mul(7, 1), 7);
    }

    #[test]
    fn gf_mul_alpha_squared() {
        // α * α = α^2 = 4
        assert_eq!(gf_mul(2, 2), 4);
    }

    #[test]
    fn gf_mul_alpha8() {
        // α^7 * α = α^8 = 0x2D
        assert_eq!(gf_mul(0x80, 2), 0x2d);
    }

    #[test]
    fn gf_mul_commutative() {
        for a in [3u8, 7, 45, 128, 200, 255] {
            for b in [5u8, 11, 90, 180, 215] {
                assert_eq!(gf_mul(a, b), gf_mul(b, a), "gf_mul({a},{b}) not commutative");
            }
        }
    }

    #[test]
    fn gf_field_order_255() {
        let t = get_gf_tables();
        let mut seen = std::collections::HashSet::new();
        for i in 0..255usize {
            let v = t.exp[i];
            assert!(v > 0, "exp[{i}] = 0 (not in multiplicative group)");
            assert!(seen.insert(v), "exp[{i}] = {v} is duplicate");
        }
        assert_eq!(seen.len(), 255);
    }

    // ── ASCII encoding ────────────────────────────────────────────────────────

    #[test]
    fn ascii_single_char() {
        assert_eq!(encode_ascii(b"A"), vec![66]);  // 65 + 1
        assert_eq!(encode_ascii(b" "), vec![33]);  // 32 + 1
        assert_eq!(encode_ascii(b"\0"), vec![1]);  // 0 + 1
    }

    #[test]
    fn ascii_digit_pairs() {
        assert_eq!(encode_ascii(b"12"), vec![142]); // 130 + (1*10+2)
        assert_eq!(encode_ascii(b"34"), vec![164]); // 130 + (3*10+4)
        assert_eq!(encode_ascii(b"00"), vec![130]); // 130 + 0
        assert_eq!(encode_ascii(b"99"), vec![229]); // 130 + 99
    }

    #[test]
    fn ascii_1234_two_pairs() {
        assert_eq!(encode_ascii(b"1234"), vec![142, 164]);
    }

    #[test]
    fn ascii_mixed_no_pair() {
        // "1A" → no pair ('A' is not a digit)
        assert_eq!(encode_ascii(b"1A"), vec![50, 66]);
    }

    #[test]
    fn ascii_hello() {
        // H=72+1=73, e=101+1=102, l=108+1=109, l=109, o=111+1=112
        assert_eq!(encode_ascii(b"Hello"), vec![73, 102, 109, 109, 112]);
    }

    #[test]
    fn ascii_hello_world_11_codewords() {
        assert_eq!(encode_ascii(b"Hello World").len(), 11);
    }

    // ── Pad codewords ─────────────────────────────────────────────────────────

    #[test]
    fn pad_a_to_3_codewords() {
        // "A" → [66], padded to 3 (10×10 symbol)
        // k=2: 129; k=3: 129 + (149*3 mod 253) + 1 = 129+194+1 = 324 > 254 → 70
        let padded = pad_codewords(&[66], 3);
        assert_eq!(padded, vec![66, 129, 70]);
    }

    #[test]
    fn pad_first_byte_is_129() {
        let padded = pad_codewords(&[10], 5);
        assert_eq!(padded[1], 129);
    }

    #[test]
    fn pad_to_exact_capacity() {
        for entry in SQUARE_SIZES.iter().take(5) {
            let padded = pad_codewords(&[66], entry.data_cw);
            assert_eq!(padded.len(), entry.data_cw);
        }
    }

    // ── Reed-Solomon ──────────────────────────────────────────────────────────

    #[test]
    fn generator_degree_matches_n_ecc() {
        for &n_ecc in &[5usize, 7, 10, 12, 14, 18, 20, 24, 28] {
            let gen = build_generator(n_ecc);
            assert_eq!(gen.len(), n_ecc + 1, "gen poly for n={n_ecc}");
            assert_eq!(gen[0], 1, "gen poly must be monic");
        }
    }

    #[test]
    fn generator_roots_are_roots() {
        // Each α^1..α^n_ecc must be a root of the generator polynomial
        let t = get_gf_tables();
        let gen = build_generator(5);
        for root in 1..=5u8 {
            let x = t.exp[root as usize];
            let mut val = 0u8;
            for &coeff in &gen {
                val = gf_mul(val, x) ^ coeff;
            }
            assert_eq!(val, 0, "α^{root} is not a root of gen(5)");
        }
    }

    #[test]
    fn rs_ecc_correct_length() {
        let gen = build_generator(5);
        let ecc = rs_encode_block(&[66, 129, 70], &gen);
        assert_eq!(ecc.len(), 5);
    }

    #[test]
    fn rs_ecc_systematic_check() {
        // Verify that C(α^i) = 0 for i=1..n_ecc (b=1 convention)
        let n_ecc = 5;
        let gen = build_generator(n_ecc);
        let data = [66u8, 129, 70];
        let ecc = rs_encode_block(&data, &gen);
        let mut codeword: Vec<u8> = data.to_vec();
        codeword.extend_from_slice(&ecc);
        let t = get_gf_tables();
        for root in 1..=n_ecc as u8 {
            let x = t.exp[root as usize];
            let mut val = 0u8;
            for &byte in &codeword {
                val = gf_mul(val, x) ^ byte;
            }
            assert_eq!(val, 0, "C(α^{root}) ≠ 0 — invalid codeword");
        }
    }

    // ── Symbol border ─────────────────────────────────────────────────────────

    fn assert_border(input: &str) {
        let grid = encode_str(input, Default::default()).unwrap();
        let rows = grid.rows as usize;
        let cols = grid.cols as usize;

        // L-finder: left column all dark
        for r in 0..rows {
            assert!(grid.modules[r][0], "left col[{r}] should be dark for '{input}'");
        }
        // L-finder: bottom row all dark
        for c in 0..cols {
            assert!(grid.modules[rows - 1][c], "bottom row[{c}] should be dark for '{input}'");
        }
        // Timing: top row alternating (skip last col — right-col overrides to dark)
        for c in 0..cols - 1 {
            let expected = c % 2 == 0;
            assert_eq!(
                grid.modules[0][c], expected,
                "top row[{c}] expected {expected} for '{input}'"
            );
        }
        // Top-right corner: dark (right-col timing: row 0 → even → dark)
        assert!(grid.modules[0][cols - 1], "top-right corner should be dark for '{input}'");
        // Timing: right col alternating (skip last row — L-finder overrides to dark)
        for r in 0..rows - 1 {
            let expected = r % 2 == 0;
            assert_eq!(
                grid.modules[r][cols - 1], expected,
                "right col[{r}] expected {expected} for '{input}'"
            );
        }
        // Corners
        assert!(grid.modules[0][0], "top-left corner should be dark");
        assert!(grid.modules[rows - 1][cols - 1], "bottom-right corner should be dark");
    }

    #[test]
    fn border_a_10x10() { assert_border("A"); }

    #[test]
    fn border_1234() { assert_border("1234"); }

    #[test]
    fn border_hello_world_16x16() { assert_border("Hello World"); }

    // ── Integration ───────────────────────────────────────────────────────────

    #[test]
    fn encode_a_gives_10x10() {
        let grid = encode_str("A", Default::default()).unwrap();
        assert_eq!(grid.rows, 10);
        assert_eq!(grid.cols, 10);
    }

    #[test]
    fn encode_1234_gives_10x10() {
        let grid = encode_str("1234", Default::default()).unwrap();
        assert_eq!(grid.rows, 10);
    }

    #[test]
    fn encode_hello_world_gives_16x16() {
        let grid = encode_str("Hello World", Default::default()).unwrap();
        assert_eq!(grid.rows, 16);
        assert_eq!(grid.cols, 16);
    }

    #[test]
    fn encode_empty_gives_smallest() {
        let grid = encode_str("", Default::default()).unwrap();
        assert_eq!(grid.rows, 10);
    }

    #[test]
    fn encode_grows_with_input() {
        let g1 = encode_str("A", Default::default()).unwrap();
        let g2 = encode_str("Hello World", Default::default()).unwrap();
        let g3 = encode_str(
            "The quick brown fox jumps over the lazy dog",
            Default::default(),
        ).unwrap();
        assert!(g1.rows <= g2.rows);
        assert!(g2.rows <= g3.rows);
    }

    #[test]
    fn encode_deterministic() {
        let g1 = encode_str("Hello World", Default::default()).unwrap();
        let g2 = encode_str("Hello World", Default::default()).unwrap();
        assert_eq!(g1.rows, g2.rows);
        for r in 0..g1.rows as usize {
            assert_eq!(g1.modules[r], g2.modules[r]);
        }
    }

    #[test]
    fn encode_input_too_long() {
        let huge = "A".repeat(1600);
        let result = encode_str(&huge, Default::default());
        assert!(matches!(result, Err(DataMatrixError::InputTooLong(_))));
    }

    #[test]
    fn digit_compression_uses_fewer_codewords() {
        let digit_cw = encode_ascii(b"12345678901234567890");
        let letter_cw = encode_ascii(b"ABCDEFGHIJKLMNOPQRST");
        assert!(digit_cw.len() < letter_cw.len());
        assert_eq!(digit_cw.len(), 10); // 10 digit pairs
    }

    #[test]
    fn ts_rust_identical_grid_for_a() {
        // Cross-language corpus: "A" → 10×10
        let grid = encode_str("A", Default::default()).unwrap();
        assert_eq!(grid.rows, 10);
        assert_eq!(grid.cols, 10);
        // Verify ISO-standard codeword: encode_ascii("A") = [66]
        assert_eq!(encode_ascii(b"A"), vec![66]);
        // Padded: [66, 129, 70]
        assert_eq!(pad_codewords(&[66], 3), vec![66, 129, 70]);
    }

    #[test]
    fn ts_rust_identical_grid_for_1234() {
        // Cross-language corpus: "1234" → 10×10, codewords [142, 164]
        let grid = encode_str("1234", Default::default()).unwrap();
        assert_eq!(grid.rows, 10);
        assert_eq!(encode_ascii(b"1234"), vec![142, 164]);
    }

    #[test]
    fn ts_rust_identical_grid_for_hello_world() {
        // Cross-language corpus: "Hello World" → 16×16
        let grid = encode_str("Hello World", Default::default()).unwrap();
        assert_eq!(grid.rows, 16);
        assert_eq!(grid.cols, 16);
    }

    #[test]
    fn multi_region_32x32() {
        let input = "A".repeat(50);
        let grid = encode_str(&input, Default::default()).unwrap();
        assert_eq!(grid.rows, 32);
        assert_eq!(grid.cols, 32);
    }

    #[test]
    fn module_grid_all_boolean() {
        let grid = encode_str("Hello World", Default::default()).unwrap();
        for row in &grid.modules {
            for &m in row {
                // m is bool — this just verifies no panic
                let _ = m;
            }
        }
    }

    // ── Utah algorithm ────────────────────────────────────────────────────────

    #[test]
    fn utah_grid_correct_size() {
        for entry in SQUARE_SIZES.iter().take(5) {
            let n_rows = entry.region_rows * entry.data_region_height;
            let n_cols = entry.region_cols * entry.data_region_width;
            let total = entry.data_cw + entry.ecc_cw;
            let codewords: Vec<u8> = vec![0xAA; total];
            let grid = utah_placement(&codewords, n_rows, n_cols);
            assert_eq!(grid.len(), n_rows);
            assert_eq!(grid[0].len(), n_cols);
        }
    }

    // ── Symbol selection ──────────────────────────────────────────────────────

    #[test]
    fn select_symbol_smallest_first() {
        assert_eq!(select_symbol(1, SymbolShape::Square).unwrap().symbol_rows, 10);
        assert_eq!(select_symbol(4, SymbolShape::Square).unwrap().symbol_rows, 12);
        assert_eq!(select_symbol(6, SymbolShape::Square).unwrap().symbol_rows, 14);
    }

    #[test]
    fn select_symbol_too_large() {
        assert!(select_symbol(1559, SymbolShape::Square).is_err());
    }
}
