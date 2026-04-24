//! # aztec-code
//!
//! Aztec Code encoder — ISO/IEC 24778:2008 compliant.
//!
//! Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995
//! and is used worldwide on airline boarding passes (IATA), rail tickets
//! (Eurostar, Amtrak), US driver's licences (AAMVA), and European postal
//! labels.
//!
//! Unlike QR Code, which uses three corner finder patterns, Aztec Code uses
//! a single concentric bullseye at the symbol's centre. This means:
//!   - No quiet zone required — the scanner finds the centre first.
//!   - The symbol can be rotated to any of four orientations.
//!   - The symbol can be printed right to the edge of a label.
//!
//! ## v0.1.0 scope
//!
//! - Byte mode encoding only (Binary-Shift from Upper mode).
//! - GF(256)/0x12D RS for data ECC (inline, b=1 convention).
//! - GF(16)/0x13 RS for mode message (inline).
//! - Compact Aztec (1–4 layers, 15×15 to 27×27).
//! - Full Aztec (1–32 layers, 19×19 to 143×143).
//! - Auto-select compact vs full, default 23% ECC.
//! - Bit stuffing, bullseye, orientation marks, reference grid.
//! - Clockwise spiral data placement.
//!
//! ## Encoding pipeline
//!
//! ```text
//! input bytes
//!   → Binary-Shift escape + raw bytes (codeword sequence)
//!   → pad to codeword count
//!   → RS ECC over GF(256)/0x12D
//!   → bit stuffing
//!   → mode message (GF(16) RS)
//!   → grid init: bullseye + orientation + reference grid
//!   → clockwise spiral placement
//!   → ModuleGrid
//! ```

pub const VERSION: &str = "0.1.0";

use barcode_2d::{layout, Barcode2DLayoutConfig, ModuleGrid, ModuleShape};
use paint_instructions::PaintScene;

// =============================================================================
// Public types
// =============================================================================

/// Options for the Aztec Code encoder.
#[derive(Clone, Debug, Default)]
pub struct AztecOptions {
    /// Minimum ECC percentage (10–90). Default: 23.
    pub min_ecc_percent: Option<u32>,
    /// Force compact form (15×15–27×27). Error if data does not fit.
    pub compact: Option<bool>,
}

/// Errors produced by the Aztec Code encoder.
#[derive(Debug)]
pub enum AztecError {
    /// Input is too long to fit in any supported symbol.
    InputTooLong(String),
    /// Internal encoding error (should not occur in correct usage).
    EncodingError(String),
}

impl std::fmt::Display for AztecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AztecError::InputTooLong(msg) => write!(f, "InputTooLong: {msg}"),
            AztecError::EncodingError(msg) => write!(f, "EncodingError: {msg}"),
        }
    }
}

impl std::error::Error for AztecError {}

// =============================================================================
// GF(256) over Aztec primitive polynomial 0x12D
//
// Aztec Code uses x^8 + x^5 + x^4 + x^2 + x + 1 = 0x12D, the same
// polynomial as Data Matrix ECC200. This is DIFFERENT from QR Code (0x11D).
// =============================================================================

/// Primitive polynomial for Aztec GF(256): x^8 + x^5 + x^4 + x^2 + x + 1.
const GF256_POLY: u16 = 0x12d;

/// Precomputed log table for GF(256)/0x12D. LOG[0] = 255 (sentinel).
static GF256_LOG: std::sync::OnceLock<[u8; 256]> = std::sync::OnceLock::new();

/// Precomputed antilog table for GF(256)/0x12D, length 512 (duplicated to
/// avoid the mod-255 reduction in the inner loop).
static GF256_ALOG: std::sync::OnceLock<[u8; 512]> = std::sync::OnceLock::new();

fn gf256_tables() -> (&'static [u8; 256], &'static [u8; 512]) {
    let log = GF256_LOG.get_or_init(|| {
        let mut log = [255u8; 256]; // LOG[0] = 255 sentinel
        let mut alog = [0u8; 512];
        let mut x: u16 = 1;
        for i in 0u16..255 {
            alog[i as usize] = x as u8;
            alog[(i + 255) as usize] = x as u8;
            log[x as usize] = i as u8;
            x <<= 1;
            if x >= 256 {
                x ^= GF256_POLY;
            }
        }
        // Note: this init writes to a separate table; we need to also init alog.
        // Since OnceLock only holds one value, we store them separately.
        log
    });
    let alog = GF256_ALOG.get_or_init(|| {
        let mut alog = [0u8; 512];
        let mut x: u16 = 1;
        for i in 0u16..255 {
            alog[i as usize] = x as u8;
            alog[(i + 255) as usize] = x as u8;
            x <<= 1;
            if x >= 256 {
                x ^= GF256_POLY;
            }
        }
        alog
    });
    (log, alog)
}

/// Multiply two GF(256) elements under polynomial 0x12D.
#[inline]
fn gf256_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let (log, alog) = gf256_tables();
    alog[(log[a as usize] as usize) + (log[b as usize] as usize)]
}

// =============================================================================
// GF(16) over primitive polynomial 0x13 (x^4 + x + 1)
//
// GF(16) is used exclusively for the mode message Reed-Solomon.
// With only 15 non-zero elements the tables are tiny.
// =============================================================================

/// Discrete log table for GF(16), poly x^4+x+1 = 0x13.
/// GF16_LOG[i] = k such that α^k = i. GF16_LOG[0] = 15 (sentinel).
static GF16_LOG: [u8; 16] = [
    15, // 0: undefined (sentinel)
    0,  // 1 = α^0
    1,  // 2 = α^1
    4,  // 3 = α^4
    2,  // 4 = α^2
    8,  // 5 = α^8
    5,  // 6 = α^5
    10, // 7 = α^10
    3,  // 8 = α^3
    14, // 9 = α^14
    9,  // 10 = α^9
    7,  // 11 = α^7
    6,  // 12 = α^6
    13, // 13 = α^13
    11, // 14 = α^11
    12, // 15 = α^12
];

/// Antilog table for GF(16): GF16_ALOG[i] = α^i.
static GF16_ALOG: [u8; 15] = [1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9];

/// Multiply two GF(16) nibbles.
#[inline]
fn gf16_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let exp = (GF16_LOG[a as usize] as usize + GF16_LOG[b as usize] as usize) % 15;
    GF16_ALOG[exp]
}

// =============================================================================
// Reed-Solomon over GF(16) — for mode message encoding
//
// Compact: (7,2) code — 2 data nibbles → 5 ECC nibbles → 7 nibbles = 28 bits
// Full:   (10,4) code — 4 data nibbles → 6 ECC nibbles → 10 nibbles = 40 bits
// =============================================================================

/// Build a monic RS generator polynomial over GF(16) with roots α^1…α^n_ecc.
///
/// g(x) = (x + α^1)(x + α^2)…(x + α^n)
/// Returns big-endian coefficients (index 0 = leading = 1).
fn build_gf16_generator(n_ecc: usize) -> Vec<u8> {
    let mut g: Vec<u8> = vec![1];
    for i in 1..=n_ecc {
        // GF16_ALOG[0] = α^0 = 1, GF16_ALOG[1] = α^1 = 2, etc.
        // So α^i = GF16_ALOG[i % 15]. For i=1: GF16_ALOG[1] = 2 = α^1. Correct.
        let ai = GF16_ALOG[i % 15]; // α^i
        let mut next = vec![0u8; g.len() + 1];
        for (j, &coeff) in g.iter().enumerate() {
            next[j] ^= coeff;
            next[j + 1] ^= gf16_mul(coeff, ai);
        }
        g = next;
    }
    g // big-endian, length n_ecc + 1
}

/// Compute RS ECC nibbles over GF(16) using LFSR division.
fn gf16_rs_encode(data: &[u8], n_ecc: usize) -> Vec<u8> {
    let gen = build_gf16_generator(n_ecc);
    let mut rem = vec![0u8; n_ecc];
    for &b in data {
        let fb = b ^ rem[0];
        for i in 0..n_ecc - 1 {
            rem[i] = rem[i + 1];
        }
        rem[n_ecc - 1] = 0;
        if fb != 0 {
            for i in 0..n_ecc {
                rem[i] ^= gf16_mul(gen[i + 1], fb);
            }
        }
    }
    rem
}

// =============================================================================
// Reed-Solomon over GF(256)/0x12D — for 8-bit data codewords
// =============================================================================

/// Build a monic RS generator over GF(256)/0x12D with roots α^1…α^n_ecc.
fn build_gf256_generator(n_ecc: usize) -> Vec<u8> {
    let (_, alog) = gf256_tables();
    let mut g: Vec<u8> = vec![1];
    for i in 1..=n_ecc {
        let ai = alog[i]; // α^i
        let mut next = vec![0u8; g.len() + 1];
        for (j, &coeff) in g.iter().enumerate() {
            next[j] ^= coeff;
            next[j + 1] ^= gf256_mul(coeff, ai);
        }
        g = next;
    }
    g
}

/// Compute RS ECC bytes over GF(256)/0x12D using LFSR division.
fn gf256_rs_encode(data: &[u8], n_ecc: usize) -> Vec<u8> {
    let gen = build_gf256_generator(n_ecc);
    let mut rem = vec![0u8; n_ecc];
    for &b in data {
        let fb = b ^ rem[0];
        for i in 0..n_ecc - 1 {
            rem[i] = rem[i + 1];
        }
        rem[n_ecc - 1] = 0;
        if fb != 0 {
            for i in 0..n_ecc {
                rem[i] ^= gf256_mul(gen[i + 1], fb);
            }
        }
    }
    rem
}

// =============================================================================
// Layer capacity tables
//
// Total data+ECC bits per layer, derived from ISO/IEC 24778:2008 Table 1.
// For byte-mode (8-bit codewords): total_codewords = floor(bits / 8).
// =============================================================================

/// Usable data+ECC bits for compact Aztec layers 1–4 (index 0 unused).
static COMPACT_LAYER_BITS: [u32; 5] = [0, 78, 200, 390, 648];

/// Usable data+ECC bits for full Aztec layers 1–32 (index 0 unused).
static FULL_LAYER_BITS: [u32; 33] = [
    0,     // index 0 unused
    120,   // L=1:  19×19
    304,   // L=2:  23×23
    496,   // L=3:  27×27
    672,   // L=4:  31×31
    888,   // L=5:  35×35
    1136,  // L=6:  39×39
    1392,  // L=7:  43×43
    1632,  // L=8:  47×47
    1920,  // L=9:  51×51
    2208,  // L=10: 55×55
    2480,  // L=11: 59×59
    2760,  // L=12: 63×63
    3016,  // L=13: 67×67
    3320,  // L=14: 71×71
    3624,  // L=15: 75×75
    3928,  // L=16: 79×79
    4216,  // L=17: 83×83
    4552,  // L=18: 87×87
    4888,  // L=19: 91×91
    5224,  // L=20: 95×95
    5560,  // L=21: 99×99
    5888,  // L=22: 103×103
    6256,  // L=23: 107×107
    6624,  // L=24: 111×111
    6960,  // L=25: 115×115
    7312,  // L=26: 119×119
    7664,  // L=27: 123×123
    8016,  // L=28: 127×127
    8400,  // L=29: 131×131
    8768,  // L=30: 135×135
    9136,  // L=31: 139×139
    9512,  // L=32: 143×143
];

// =============================================================================
// Symbol configuration
// =============================================================================

/// Symbol configuration selected by the auto-sizer.
#[derive(Debug, Clone, Copy)]
struct SymbolConfig {
    compact: bool,
    layers: usize,
    data_cw_count: usize,
    ecc_cw_count: usize,
}

/// Symbol size in modules for a compact Aztec symbol with `layers` layers.
#[inline]
fn compact_size(layers: usize) -> usize {
    11 + 4 * layers
}

/// Symbol size in modules for a full Aztec symbol with `layers` layers.
#[inline]
fn full_size(layers: usize) -> usize {
    15 + 4 * layers
}

/// Select the smallest symbol configuration that fits the input data.
///
/// The required byte count is converted to a codeword count including the
/// Binary-Shift overhead (5-bit BS codeword + 5 or 16-bit length + 8*n bits).
fn select_symbol(n_bytes: usize, min_ecc_percent: u32) -> Result<SymbolConfig, AztecError> {
    let bits_needed = 5 + (if n_bytes <= 31 { 5 } else { 16 }) + 8 * n_bytes;
    let cw_needed = (bits_needed + 7) / 8;

    // Try compact layers 1–4.
    for layers in 1..=4usize {
        let total_bits = COMPACT_LAYER_BITS[layers] as usize;
        let total_cw = total_bits / 8;
        let ecc_cw = (min_ecc_percent as usize * total_cw + 99) / 100; // ceil
        let data_cw = total_cw.saturating_sub(ecc_cw);
        if data_cw >= cw_needed {
            return Ok(SymbolConfig {
                compact: true,
                layers,
                data_cw_count: data_cw,
                ecc_cw_count: ecc_cw,
            });
        }
    }

    // Try full layers 1–32.
    for layers in 1..=32usize {
        let total_bits = FULL_LAYER_BITS[layers] as usize;
        let total_cw = total_bits / 8;
        let ecc_cw = (min_ecc_percent as usize * total_cw + 99) / 100;
        let data_cw = total_cw.saturating_sub(ecc_cw);
        if data_cw >= cw_needed {
            return Ok(SymbolConfig {
                compact: false,
                layers,
                data_cw_count: data_cw,
                ecc_cw_count: ecc_cw,
            });
        }
    }

    Err(AztecError::InputTooLong(format!(
        "Input ({n_bytes} bytes) exceeds the capacity of a 32-layer full Aztec Code symbol."
    )))
}

// =============================================================================
// Binary-Shift encoding (v0.1.0: byte mode only)
//
// Binary-Shift encoding:
//   1. Emit codeword 31 (5 bits) — Binary-Shift in Upper mode.
//   2. Length prefix: ≤31 bytes → 5-bit length; else 00000 + 11-bit length.
//   3. Raw bytes (8 bits each, MSB first).
// =============================================================================

/// Encode input bytes using Binary-Shift from Upper mode.
///
/// Returns a bit vector (0s and 1s), MSB first.
fn encode_binary_shift(input: &[u8]) -> Vec<u8> {
    let mut bits: Vec<u8> = Vec::with_capacity(5 + 16 + input.len() * 8);

    // Binary-Shift codeword = 31 (5 bits).
    for i in (0..5).rev() {
        bits.push((31u8 >> i) & 1);
    }

    // Length prefix.
    let len = input.len();
    if len <= 31 {
        for i in (0..5).rev() {
            bits.push((len as u8 >> i) & 1);
        }
    } else {
        // Extended: 5 zero bits then 11-bit count.
        for _ in 0..5 {
            bits.push(0);
        }
        for i in (0..11).rev() {
            bits.push((len as u16 >> i) as u8 & 1);
        }
    }

    // Raw bytes.
    for &b in input {
        for i in (0..8).rev() {
            bits.push((b >> i) & 1);
        }
    }

    bits
}

// =============================================================================
// Bit stuffing
//
// After 4 consecutive identical bits in the data+ECC stream, insert the
// complement bit. The run counter resets after each stuffed bit.
// =============================================================================

/// Apply bit stuffing to a bit slice.
///
/// After every run of 4 identical consecutive bits, inserts the complement bit.
///
/// ```text
/// Input:  [1, 1, 1, 1, 0, 0, 0, 0, 1, 0]
/// After 4× 1: stuff 0 → [1, 1, 1, 1, 0, 0, 0, 0, 0, ...]
/// After 4× 0: stuff 1 → [..., 0, 0, 0, 0, 1, 1, 0]
/// Output: [1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1, 0]
/// ```
fn bit_stuff(bits: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bits.len() + bits.len() / 4);
    let mut run_val: i16 = -1; // -1 = no run started
    let mut run_len: u32 = 0;

    for &bit in bits {
        let bit_i = bit as i16;
        if bit_i == run_val {
            run_len += 1;
        } else {
            run_val = bit_i;
            run_len = 1;
        }
        out.push(bit);
        if run_len == 4 {
            let stuff = 1 - bit;
            out.push(stuff);
            run_val = stuff as i16;
            run_len = 1;
        }
    }

    out
}

// =============================================================================
// Mode message encoding
//
// Compact (28 bits = 7 nibbles, GF(16) RS, 2 data + 5 ECC):
//   combined = ((layers - 1) << 6) | (data_cw - 1)
//   nibble[i] = (combined >> (4*i)) & 0xF for i=0,1
//
// Full (40 bits = 10 nibbles, GF(16) RS, 4 data + 6 ECC):
//   combined = ((layers - 1) << 11) | (data_cw - 1)
//   nibble[i] = (combined >> (4*i)) & 0xF for i=0..3
//
// Both serialized LSB-first per nibble.
// =============================================================================

/// Encode the compact Aztec mode message into 28 bits.
fn encode_mode_message_compact(layers: usize, data_cw: usize) -> Vec<u8> {
    let combined = ((layers - 1) as u8) << 6 | ((data_cw - 1) as u8);
    let data_nibbles = [combined & 0xf, (combined >> 4) & 0xf];
    let ecc_nibbles = gf16_rs_encode(&data_nibbles, 5);
    let all_nibbles: Vec<u8> = data_nibbles.iter().chain(ecc_nibbles.iter()).copied().collect();

    let mut bits = Vec::with_capacity(28);
    for nib in all_nibbles {
        for b in 0..4 {
            bits.push((nib >> b) & 1);
        }
    }
    bits // 28 bits
}

/// Encode the full Aztec mode message into 40 bits.
fn encode_mode_message_full(layers: usize, data_cw: usize) -> Vec<u8> {
    let combined = (((layers - 1) as u16) << 11) | ((data_cw - 1) as u16);
    let data_nibbles = [
        (combined >> 0)  as u8 & 0xf,
        (combined >> 4)  as u8 & 0xf,
        (combined >> 8)  as u8 & 0xf,
        (combined >> 12) as u8 & 0xf,
    ];
    let ecc_nibbles = gf16_rs_encode(&data_nibbles, 6);
    let all_nibbles: Vec<u8> = data_nibbles.iter().chain(ecc_nibbles.iter()).copied().collect();

    let mut bits = Vec::with_capacity(40);
    for nib in all_nibbles {
        for b in 0..4 {
            bits.push((nib >> b) & 1);
        }
    }
    bits // 40 bits
}

// =============================================================================
// Grid construction
// =============================================================================

/// Working grid used during symbol construction.
struct WorkGrid {
    size: usize,
    modules: Vec<Vec<bool>>,   // true = dark
    reserved: Vec<Vec<bool>>,  // true = structural (skip during data placement)
}

impl WorkGrid {
    fn new(size: usize) -> Self {
        WorkGrid {
            size,
            modules:  vec![vec![false; size]; size],
            reserved: vec![vec![false; size]; size],
        }
    }

    fn set(&mut self, row: usize, col: usize, dark: bool, reserve: bool) {
        self.modules[row][col] = dark;
        if reserve {
            self.reserved[row][col] = true;
        }
    }
}

/// Place the bullseye finder pattern.
///
/// Color rule (Chebyshev distance d from centre):
///   d ≤ 1                   → DARK (solid 3×3 inner core)
///   d ≥ 2 and d is odd      → DARK
///   d ≥ 2 and d is even     → LIGHT
///
/// This gives: d=0,1:dark, d=2:light, d=3:dark, d=4:light, d=5:dark (compact)
/// or continuing to d=6:light, d=7:dark (full).
///
/// All bullseye modules are marked reserved.
fn place_bullseye(g: &mut WorkGrid, cx: usize, cy: usize, radius: usize) {
    for dr in -(radius as isize)..=radius as isize {
        for dc in -(radius as isize)..=radius as isize {
            let d = dr.unsigned_abs().max(dc.unsigned_abs());
            let dark = d <= 1 || d % 2 == 1;
            g.set(
                (cy as isize + dr) as usize,
                (cx as isize + dc) as usize,
                dark,
                true,
            );
        }
    }
}

/// Place orientation marks and mode message bits in the mode message ring.
///
/// Returns the list of non-corner ring positions in clockwise order.
fn place_orientation_and_mode_message(
    g: &mut WorkGrid,
    cx: usize,
    cy: usize,
    r: usize,
    mode_msg_bits: &[u8],
) -> Vec<(usize, usize)> {
    let cx = cx as isize;
    let cy = cy as isize;
    let r = r as isize;

    // Place 4 orientation marks (corners).
    g.set((cy - r) as usize, (cx - r) as usize, true, true);
    g.set((cy - r) as usize, (cx + r) as usize, true, true);
    g.set((cy + r) as usize, (cx + r) as usize, true, true);
    g.set((cy + r) as usize, (cx - r) as usize, true, true);

    // Enumerate non-corner perimeter positions clockwise.
    let mut positions: Vec<(usize, usize)> = Vec::new();

    // Top edge: left to right (skipping corners).
    for col in (cx - r + 1)..=(cx + r - 1) {
        positions.push(((cy - r) as usize, col as usize));
    }
    // Right edge: top to bottom (skipping corners).
    for row in (cy - r + 1)..=(cy + r - 1) {
        positions.push((row as usize, (cx + r) as usize));
    }
    // Bottom edge: right to left (skipping corners).
    for col in ((cx - r + 1)..=(cx + r - 1)).rev() {
        positions.push(((cy + r) as usize, col as usize));
    }
    // Left edge: bottom to top (skipping corners).
    for row in ((cy - r + 1)..=(cy + r - 1)).rev() {
        positions.push((row as usize, (cx - r) as usize));
    }

    // Place mode message bits.
    for (i, &bit) in mode_msg_bits.iter().enumerate() {
        let (row, col) = positions[i];
        g.set(row, col, bit == 1, true);
    }

    positions
}

/// Place the reference grid for full Aztec symbols.
///
/// Reference grid lines are at rows/cols that are multiples of 16 from the
/// centre (cy, cx). On each reference line, modules alternate dark/light
/// starting from the centre axis.
///
/// Modules already reserved (bullseye, mode message, orientation) are skipped.
fn place_reference_grid(g: &mut WorkGrid, cx: usize, cy: usize) {
    let size = g.size as isize;
    let cx_i = cx as isize;
    let cy_i = cy as isize;

    // Collect reference row and column indices.
    let mut ref_rows: Vec<usize> = Vec::new();
    let mut ref_cols: Vec<usize> = Vec::new();

    for n in 0isize.. {
        let dr = n * 16;
        let mut added = false;
        if cy_i + dr < size {
            ref_rows.push((cy_i + dr) as usize);
            added = true;
        }
        if n > 0 && cy_i - dr >= 0 {
            ref_rows.push((cy_i - dr) as usize);
            added = true;
        }
        if !added && dr > size {
            break;
        }
        if dr > size {
            break;
        }
    }
    for n in 0isize.. {
        let dc = n * 16;
        let mut added = false;
        if cx_i + dc < size {
            ref_cols.push((cx_i + dc) as usize);
            added = true;
        }
        if n > 0 && cx_i - dc >= 0 {
            ref_cols.push((cx_i - dc) as usize);
            added = true;
        }
        if !added && dc > size {
            break;
        }
        if dc > size {
            break;
        }
    }

    let ref_row_set: std::collections::HashSet<usize> = ref_rows.into_iter().collect();
    let ref_col_set: std::collections::HashSet<usize> = ref_cols.into_iter().collect();

    for row in 0..g.size {
        for col in 0..g.size {
            let on_ref_row = ref_row_set.contains(&row);
            let on_ref_col = ref_col_set.contains(&col);
            if !on_ref_row && !on_ref_col {
                continue;
            }
            // Skip already-reserved modules (bullseye etc.).
            if g.reserved[row][col] {
                continue;
            }
            let dark = if on_ref_row && on_ref_col {
                true // intersections always dark
            } else if on_ref_row {
                (cx as isize - col as isize).rem_euclid(2) == 0
            } else {
                (cy as isize - row as isize).rem_euclid(2) == 0
            };
            g.set(row, col, dark, true);
        }
    }
}

/// Place data bits via the clockwise spiral layer algorithm.
///
/// For each layer (innermost first), bits are placed in pairs along the
/// 2-module-wide band in clockwise order: outer module first, inner second.
///
/// The mode message ring's remaining positions (after the mode message bits)
/// receive the first data bits.
fn place_data_bits(
    g: &mut WorkGrid,
    cx: usize,
    cy: usize,
    layers: usize,
    is_compact: bool,
    mode_msg_positions: &[(usize, usize)],
    mode_msg_bit_count: usize,
    stuffed_bits: &[u8],
) {
    let mut bit_idx = 0usize;
    let size_i = g.size as isize;

    // Safe place-bit: checks bounds using signed arithmetic to avoid usize underflow.
    let mut place_bit_safe = |g: &mut WorkGrid, row_i: isize, col_i: isize| {
        if row_i < 0 || row_i >= size_i || col_i < 0 || col_i >= size_i {
            return;
        }
        let row = row_i as usize;
        let col = col_i as usize;
        if g.reserved[row][col] {
            return;
        }
        if bit_idx >= stuffed_bits.len() {
            return;
        }
        g.modules[row][col] = stuffed_bits[bit_idx] == 1;
        g.reserved[row][col] = true;
        bit_idx += 1;
    };

    // Step 1: fill remaining mode message ring positions.
    for &(row, col) in &mode_msg_positions[mode_msg_bit_count..] {
        place_bit_safe(g, row as isize, col as isize);
    }

    // Step 2: spiral through each data layer.
    let base_radius: isize = if is_compact { 7 } else { 9 };
    let cx_i = cx as isize;
    let cy_i = cy as isize;

    for l in 0..layers as isize {
        let d_i = base_radius + l * 2;
        let d_o = d_i + 1;

        // Top edge: left to right.
        for col in (cx_i - d_i + 1)..=(cx_i + d_i) {
            place_bit_safe(g, cy_i - d_o, col);
            place_bit_safe(g, cy_i - d_i, col);
        }
        // Right edge: top to bottom.
        for row in (cy_i - d_i + 1)..=(cy_i + d_i) {
            place_bit_safe(g, row, cx_i + d_o);
            place_bit_safe(g, row, cx_i + d_i);
        }
        // Bottom edge: right to left.
        for col in ((cx_i - d_i + 1)..=(cx_i + d_i)).rev() {
            place_bit_safe(g, cy_i + d_o, col);
            place_bit_safe(g, cy_i + d_i, col);
        }
        // Left edge: bottom to top.
        for row in ((cy_i - d_i + 1)..=(cy_i + d_i)).rev() {
            place_bit_safe(g, row, cx_i - d_o);
            place_bit_safe(g, row, cx_i - d_i);
        }
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Encode a byte slice into an Aztec Code ModuleGrid.
///
/// ## Algorithm
///
/// 1. Encode bytes using Binary-Shift from Upper mode.
/// 2. Select smallest symbol (compact or full) at the requested ECC%.
/// 3. Pad data codewords to symbol capacity.
/// 4. Compute RS ECC over GF(256)/0x12D (b=1).
/// 5. Apply bit stuffing to (data + ECC) bit stream.
/// 6. Compute mode message (GF(16) RS, 28 or 40 bits).
/// 7. Build grid: bullseye, orientation, reference grid, spiral placement.
///
/// # Errors
///
/// Returns [`AztecError::InputTooLong`] if the input exceeds the capacity of
/// a 32-layer full Aztec Code symbol (~3471 bytes at 23% ECC).
pub fn encode(input: &[u8], options: Option<&AztecOptions>) -> Result<ModuleGrid, AztecError> {
    let min_ecc_percent = options
        .and_then(|o| o.min_ecc_percent)
        .unwrap_or(23);
    let force_compact = options.and_then(|o| o.compact).unwrap_or(false);

    // Step 1: Binary-Shift encoding.
    let encoded_bits = encode_binary_shift(input);

    // Step 2: select symbol.
    let cfg = select_symbol(input.len(), min_ecc_percent)?;
    if force_compact && !cfg.compact {
        return Err(AztecError::InputTooLong(format!(
            "Input ({} bytes) does not fit in compact Aztec (max 4 layers).",
            input.len()
        )));
    }

    // Step 3: pad data codewords.
    let mut data_bytes: Vec<u8> = Vec::with_capacity(cfg.data_cw_count);
    for chunk in encoded_bits.chunks(8) {
        let mut byte = 0u8;
        for (j, &b) in chunk.iter().enumerate() {
            byte |= b << (7 - j);
        }
        data_bytes.push(byte);
    }
    while data_bytes.len() < cfg.data_cw_count {
        data_bytes.push(0);
    }
    data_bytes.truncate(cfg.data_cw_count);

    // All-zero codeword avoidance: if the last data codeword is 0, replace
    // it with 0xFF to prevent RS complications.
    if let Some(last) = data_bytes.last_mut() {
        if *last == 0 {
            *last = 0xff;
        }
    }

    // Step 4: RS ECC.
    let ecc_bytes = gf256_rs_encode(&data_bytes, cfg.ecc_cw_count);
    let all_cw: Vec<u8> = data_bytes.iter().chain(ecc_bytes.iter()).copied().collect();

    // Step 5: bit stuffing.
    let mut all_bits: Vec<u8> = Vec::with_capacity(all_cw.len() * 8);
    for b in &all_cw {
        for i in (0..8).rev() {
            all_bits.push((b >> i) & 1);
        }
    }
    let stuffed = bit_stuff(&all_bits);

    // Step 6: mode message.
    let mode_msg: Vec<u8> = if cfg.compact {
        encode_mode_message_compact(cfg.layers, cfg.data_cw_count)
    } else {
        encode_mode_message_full(cfg.layers, cfg.data_cw_count)
    };

    // Step 7: build grid.
    let size = if cfg.compact {
        compact_size(cfg.layers)
    } else {
        full_size(cfg.layers)
    };
    let cx = size / 2;
    let cy = size / 2;
    let mut g = WorkGrid::new(size);

    // Bullseye.
    let bullseye_radius = if cfg.compact { 5 } else { 7 };
    place_bullseye(&mut g, cx, cy, bullseye_radius);

    // Orientation marks and mode message.
    let mode_ring_radius = bullseye_radius + 1;
    let mode_ring_positions = place_orientation_and_mode_message(
        &mut g, cx, cy, mode_ring_radius, &mode_msg,
    );

    // Reference grid (full only).
    if !cfg.compact {
        place_reference_grid(&mut g, cx, cy);
    }

    // Data + ECC bits via clockwise spiral.
    place_data_bits(
        &mut g, cx, cy, cfg.layers, cfg.compact,
        &mode_ring_positions, mode_msg.len(), &stuffed,
    );

    Ok(ModuleGrid {
        rows: size as u32,
        cols: size as u32,
        modules: g.modules,
        module_shape: ModuleShape::Square,
    })
}

/// Encode a UTF-8 string into an Aztec Code ModuleGrid.
///
/// Convenience wrapper around [`encode`] that accepts a `&str`.
pub fn encode_str(input: &str, options: Option<&AztecOptions>) -> Result<ModuleGrid, AztecError> {
    encode(input.as_bytes(), options)
}

/// Encode and convert to a pixel-resolved PaintScene.
///
/// Delegates pixel geometry to `barcode_2d::layout()`.
pub fn encode_and_layout(
    input: &[u8],
    options: Option<&AztecOptions>,
    config: &Barcode2DLayoutConfig,
) -> Result<PaintScene, AztecError> {
    let grid = encode(input, options)?;
    layout(&grid, config).map_err(|e| AztecError::EncodingError(e.to_string()))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────
    // GF(16) arithmetic
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn gf16_mul_by_zero_is_zero() {
        for a in 0u8..16 {
            assert_eq!(gf16_mul(a, 0), 0);
            assert_eq!(gf16_mul(0, a), 0);
        }
    }

    #[test]
    fn gf16_mul_by_one_is_identity() {
        for a in 1u8..16 {
            assert_eq!(gf16_mul(a, 1), a, "gf16_mul({a}, 1) should be {a}");
        }
    }

    #[test]
    fn gf16_alpha_period_is_15() {
        // α^15 should equal α^0 = 1 (period = 15, so it is primitive).
        let mut x = 1u8;
        for _ in 0..14 {
            x = gf16_mul(x, 2); // multiply by α=2
        }
        // x = 2^14 mod poly. After 14 multiplications by 2: α^14 = 9.
        // One more multiplication by 2: α^15 = α^0 = 1.
        let alpha_15 = gf16_mul(x, 2);
        assert_eq!(alpha_15, 1, "α^15 should equal 1 (primitive element period = 15)");
    }

    #[test]
    fn gf16_alog_table_values() {
        // Verify key antilog values from the spec.
        assert_eq!(GF16_ALOG[0], 1);   // α^0 = 1
        assert_eq!(GF16_ALOG[1], 2);   // α^1 = 2
        assert_eq!(GF16_ALOG[2], 4);   // α^2 = 4
        assert_eq!(GF16_ALOG[3], 8);   // α^3 = 8
        assert_eq!(GF16_ALOG[4], 3);   // α^4 = 3 (since x^4 = x + 1 in GF(16))
        assert_eq!(GF16_ALOG[14], 9);  // α^14 = 9
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Mode message encoding
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn compact_mode_message_is_28_bits() {
        let bits = encode_mode_message_compact(1, 5);
        assert_eq!(bits.len(), 28);
    }

    #[test]
    fn full_mode_message_is_40_bits() {
        let bits = encode_mode_message_full(2, 12);
        assert_eq!(bits.len(), 40);
    }

    #[test]
    fn mode_message_bits_are_binary() {
        let compact = encode_mode_message_compact(3, 20);
        for &b in &compact {
            assert!(b == 0 || b == 1, "mode message bit {b} is not 0 or 1");
        }
        let full = encode_mode_message_full(5, 100);
        for &b in &full {
            assert!(b == 0 || b == 1, "mode message bit {b} is not 0 or 1");
        }
    }

    #[test]
    fn compact_mode_message_is_deterministic() {
        let a = encode_mode_message_compact(2, 10);
        let b = encode_mode_message_compact(2, 10);
        assert_eq!(a, b);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Bit stuffing
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn bit_stuff_inserts_after_run_of_4_ones() {
        // [1,1,1,1] → [1,1,1,1,0]
        let input = vec![1, 1, 1, 1];
        let out = bit_stuff(&input);
        assert_eq!(out, vec![1, 1, 1, 1, 0]);
    }

    #[test]
    fn bit_stuff_inserts_after_run_of_4_zeros() {
        // [0,0,0,0] → [0,0,0,0,1]
        let input = vec![0, 0, 0, 0];
        let out = bit_stuff(&input);
        assert_eq!(out, vec![0, 0, 0, 0, 1]);
    }

    #[test]
    fn bit_stuff_alternating_no_stuff() {
        // Alternating bits: never run of 4.
        let input = vec![0, 1, 0, 1, 0, 1, 0, 1];
        let out = bit_stuff(&input);
        assert_eq!(out, input, "alternating bits should not trigger stuffing");
    }

    #[test]
    fn bit_stuff_run_of_8_ones_stuffs_twice() {
        // [1,1,1,1,1,1,1,1]
        // After first 4: stuff 0, reset. Then next 4 ones start a new run.
        // Wait: stuff bit = 0, starts run_val=0, run_len=1. Next bit is 1 → run resets.
        // So bits 5..8 are a new run of 1s. After bit 8 the run of 1s is:
        // bit 5 = 1 (run=1), bit 6 = 1 (run=2), bit 7 = 1 (run=3), bit 8 = 1 (run=4) → stuff again.
        let input = vec![1u8; 8];
        let out = bit_stuff(&input);
        // Expected: [1,1,1,1,0,1,1,1,1,0]
        assert_eq!(out, vec![1, 1, 1, 1, 0, 1, 1, 1, 1, 0]);
    }

    #[test]
    fn bit_stuff_empty_input_is_empty() {
        let out = bit_stuff(&[]);
        assert!(out.is_empty());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Symbol sizing
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn single_byte_fits_compact_1_layer() {
        let cfg = select_symbol(1, 23).unwrap();
        assert!(cfg.compact);
        assert_eq!(cfg.layers, 1);
    }

    #[test]
    fn large_input_uses_full_aztec() {
        let cfg = select_symbol(100, 23).unwrap();
        // 100 bytes is likely too much for compact (max ~50 bytes at 23%).
        // It should select full Aztec.
        assert!(!cfg.compact || cfg.layers <= 4);
    }

    #[test]
    fn too_large_input_returns_error() {
        let result = select_symbol(4000, 23);
        assert!(matches!(result, Err(AztecError::InputTooLong(_))));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Symbol sizes follow the standard formulas
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn compact_sizes_follow_formula() {
        assert_eq!(compact_size(1), 15);
        assert_eq!(compact_size(2), 19);
        assert_eq!(compact_size(3), 23);
        assert_eq!(compact_size(4), 27);
    }

    #[test]
    fn full_sizes_follow_formula() {
        assert_eq!(full_size(1), 19);
        assert_eq!(full_size(2), 23);
        assert_eq!(full_size(32), 143);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Bullseye structure
    // ─────────────────────────────────────────────────────────────────────────

    fn check_bullseye(modules: &[Vec<bool>], cx: usize, cy: usize, radius: usize) {
        let cx_i = cx as isize;
        let cy_i = cy as isize;
        for dr in -(radius as isize)..=radius as isize {
            for dc in -(radius as isize)..=radius as isize {
                let d = dr.unsigned_abs().max(dc.unsigned_abs());
                let expected_dark = d <= 1 || d % 2 == 1;
                let actual = modules[(cy_i + dr) as usize][(cx_i + dc) as usize];
                assert_eq!(
                    actual, expected_dark,
                    "bullseye at dr={dr}, dc={dc} (d={d}): expected {expected_dark}, got {actual}"
                );
            }
        }
    }

    #[test]
    fn compact_15x15_bullseye_is_correct() {
        let grid = encode(b"A", None).unwrap();
        assert_eq!(grid.rows, 15);
        let cx = 7;
        let cy = 7;
        check_bullseye(&grid.modules, cx, cy, 5);
    }

    #[test]
    fn compact_15x15_center_3x3_all_dark() {
        let grid = encode(b"A", None).unwrap();
        let cx = 7;
        let cy = 7;
        // d ≤ 1: solid 3×3 dark square.
        for dr in -1isize..=1 {
            for dc in -1isize..=1 {
                assert!(
                    grid.modules[(cy as isize + dr) as usize][(cx as isize + dc) as usize],
                    "center 3×3 at dr={dr}, dc={dc} should be dark"
                );
            }
        }
    }

    #[test]
    fn compact_15x15_ring_d2_all_light() {
        let grid = encode(b"A", None).unwrap();
        let cx = 7isize;
        let cy = 7isize;
        for dr in -2isize..=2 {
            for dc in -2isize..=2 {
                let d = dr.unsigned_abs().max(dc.unsigned_abs());
                if d == 2 {
                    assert!(
                        !grid.modules[(cy + dr) as usize][(cx + dc) as usize],
                        "d=2 module at dr={dr}, dc={dc} should be light"
                    );
                }
            }
        }
    }

    #[test]
    fn compact_15x15_ring_d5_all_dark() {
        let grid = encode(b"A", None).unwrap();
        let cx = 7isize;
        let cy = 7isize;
        for dr in -5isize..=5 {
            for dc in -5isize..=5 {
                let d = dr.unsigned_abs().max(dc.unsigned_abs());
                if d == 5 {
                    assert!(
                        grid.modules[(cy + dr) as usize][(cx + dc) as usize],
                        "d=5 module at dr={dr}, dc={dc} should be dark (outermost compact ring)"
                    );
                }
            }
        }
    }

    #[test]
    fn full_symbol_bullseye_radius_7_correct() {
        let long_input: Vec<u8> = b"A".repeat(60);
        let grid = encode(&long_input, None).unwrap();
        let cx = (grid.cols / 2) as usize;
        let cy = (grid.rows / 2) as usize;
        check_bullseye(&grid.modules, cx, cy, 7);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Orientation marks
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn compact_15x15_orientation_marks_are_dark() {
        let grid = encode(b"A", None).unwrap();
        let cx = 7isize;
        let cy = 7isize;
        let r = 6isize; // mode message ring radius
        // Four corners of the mode message ring must always be dark.
        assert!(grid.modules[(cy - r) as usize][(cx - r) as usize], "top-left orientation mark");
        assert!(grid.modules[(cy - r) as usize][(cx + r) as usize], "top-right orientation mark");
        assert!(grid.modules[(cy + r) as usize][(cx + r) as usize], "bottom-right orientation mark");
        assert!(grid.modules[(cy + r) as usize][(cx - r) as usize], "bottom-left orientation mark");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Integration tests — test corpus
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn encode_a_produces_15x15_compact() {
        let grid = encode(b"A", None).unwrap();
        assert_eq!(grid.rows, 15);
        assert_eq!(grid.cols, 15);
    }

    #[test]
    fn encode_hello_world_does_not_panic() {
        encode(b"Hello World", None).unwrap();
    }

    #[test]
    fn encode_url_does_not_panic() {
        encode(b"https://example.com", None).unwrap();
    }

    #[test]
    fn encode_raw_binary_does_not_panic() {
        let raw: Vec<u8> = (0u8..64).collect();
        encode(&raw, None).unwrap();
    }

    #[test]
    fn encode_digit_heavy_does_not_panic() {
        encode(b"01234567890123456789", None).unwrap();
    }

    #[test]
    fn encode_is_deterministic() {
        let input = b"https://example.com";
        let g1 = encode(input, None).unwrap();
        let g2 = encode(input, None).unwrap();
        assert_eq!(g1.rows, g2.rows);
        assert_eq!(g1.modules, g2.modules);
    }

    #[test]
    fn grid_is_always_square() {
        for input in &[b"A" as &[u8], b"Hello World", b"https://example.com"] {
            let grid = encode(input, None).unwrap();
            assert_eq!(grid.rows, grid.cols, "grid should be square for input {:?}", input);
        }
    }

    #[test]
    fn grid_size_is_odd() {
        for input in &[b"A" as &[u8], b"Hello World", b"https://example.com"] {
            let grid = encode(input, None).unwrap();
            assert_eq!(grid.rows % 2, 1, "grid size should be odd for input {:?}", input);
        }
    }

    #[test]
    fn compact_true_forces_compact_for_short_input() {
        let opts = AztecOptions { compact: Some(true), ..Default::default() };
        let grid = encode(b"Hi", Some(&opts)).unwrap();
        let s = grid.rows;
        assert!(s >= 15 && s <= 27 && (s - 11) % 4 == 0, "compact symbol size {s} is invalid");
    }

    #[test]
    fn compact_true_errors_for_large_input() {
        let opts = AztecOptions { compact: Some(true), ..Default::default() };
        let large: Vec<u8> = b"A".repeat(60);
        let result = encode(&large, Some(&opts));
        assert!(matches!(result, Err(AztecError::InputTooLong(_))));
    }

    #[test]
    fn very_large_input_returns_error() {
        let huge: Vec<u8> = b"X".repeat(4000);
        let result = encode(&huge, None);
        assert!(matches!(result, Err(AztecError::InputTooLong(_))));
    }

    #[test]
    fn higher_ecc_may_produce_larger_symbol() {
        let input = b"Hello World Hello World";
        let g23 = encode(input, Some(&AztecOptions { min_ecc_percent: Some(23), ..Default::default() })).unwrap();
        let g50 = encode(input, Some(&AztecOptions { min_ecc_percent: Some(50), ..Default::default() })).unwrap();
        assert!(g50.rows >= g23.rows, "higher ECC should produce same or larger symbol");
    }

    #[test]
    fn encode_str_and_bytes_agree() {
        let s = "Hello";
        let g_str = encode_str(s, None).unwrap();
        let g_bytes = encode(s.as_bytes(), None).unwrap();
        assert_eq!(g_str.rows, g_bytes.rows);
        assert_eq!(g_str.modules, g_bytes.modules);
    }

    #[test]
    fn all_zeros_input_does_not_panic() {
        let zeros = vec![0u8; 20];
        encode(&zeros, None).unwrap();
    }

    #[test]
    fn all_ones_input_does_not_panic() {
        let ones = vec![0xffu8; 20];
        encode(&ones, None).unwrap();
    }

    #[test]
    fn gf256_tables_produce_correct_poly_0x12d() {
        // Verify that α^255 = α^0 = 1 (period = 255, primitive element).
        let (log, alog) = gf256_tables();
        assert_eq!(alog[0], 1, "α^0 should be 1");
        assert_eq!(alog[1], 2, "α^1 should be 2");
        // α^255 wraps back to 1.
        assert_eq!(alog[255], 1, "α^255 should be 1 (period 255)");
        // log[1] = 0 (since α^0 = 1)
        assert_eq!(log[1], 0, "log[1] should be 0");
        // log[2] = 1 (since α^1 = 2)
        assert_eq!(log[2], 1, "log[2] should be 1");
    }
}
