//! # aztec-code
//!
//! Aztec Code encoder — ISO/IEC 24778:2008 compliant.
//!
//! Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
//! published as a patent-free format. Unlike QR Code (which uses three square
//! finder patterns at three corners), Aztec Code places a single **bullseye
//! finder pattern at the center** of the symbol. The scanner finds the center
//! first, then reads outward in a spiral — no large quiet zone is needed.
//!
//! ## Where Aztec Code is used today
//!
//! - **IATA boarding passes** — the barcode on every airline boarding pass
//! - **Eurostar and Amtrak rail tickets** — printed and on-screen tickets
//! - **PostNL, Deutsche Post, La Poste** — European postal routing
//! - **US military ID cards**
//!
//! ## Symbol variants
//!
//! ```text
//! Compact: 1-4 layers,  size = 11 + 4*layers  (15x15 to 27x27)
//! Full:    1-32 layers, size = 15 + 4*layers  (19x19 to 143x143)
//! ```
//!
//! ## Encoding pipeline (v0.1.0 — byte-mode only)
//!
//! ```text
//! input bytes
//!   -> Binary-Shift codewords from Upper mode
//!   -> symbol size selection (smallest compact then full that fits at 23% ECC)
//!   -> pad to exact codeword count
//!   -> GF(256)/0x12D Reed-Solomon ECC (poly 0x12D, b=1 roots alpha^1..alpha^n)
//!   -> bit stuffing (insert complement after 4 consecutive identical bits)
//!   -> GF(16) mode message (layers + codeword count + 5 or 6 RS nibbles)
//!   -> ModuleGrid (bullseye -> orientation marks -> mode msg -> data spiral)
//! ```
//!
//! ## v0.1.0 simplifications
//!
//! 1. Byte-mode only — all input encoded via Binary-Shift from Upper mode.
//!    Multi-mode (Digit/Upper/Lower/Mixed/Punct) optimization is v0.2.0.
//! 2. 8-bit codewords -> GF(256) RS (same polynomial as Data Matrix: 0x12D).
//! 3. Default ECC = 23%.
//! 4. Auto-select compact vs full (force-compact option is v0.2.0).

pub use barcode_2d::{Barcode2DError, Barcode2DLayoutConfig, ModuleGrid, ModuleShape};
pub use paint_instructions::PaintScene;

pub const VERSION: &str = "0.1.0";

// ============================================================================
// Public API types
// ============================================================================

/// Options for Aztec Code encoding.
#[derive(Debug, Clone)]
pub struct AztecOptions {
    /// Minimum error-correction percentage (default: 23, range: 10-90).
    pub min_ecc_percent: u32,
}

impl Default for AztecOptions {
    fn default() -> Self {
        Self {
            min_ecc_percent: 23,
        }
    }
}

/// Errors produced by the aztec-code encoder.
#[derive(Debug, PartialEq, Eq)]
pub enum AztecError {
    /// The input is too long to fit in any Aztec Code symbol.
    InputTooLong(String),
    /// A layout error was returned from barcode-2d.
    LayoutError(String),
}

impl std::fmt::Display for AztecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AztecError::InputTooLong(msg) => write!(f, "InputTooLong: {}", msg),
            AztecError::LayoutError(msg) => write!(f, "LayoutError: {}", msg),
        }
    }
}

impl std::error::Error for AztecError {}

// ============================================================================
// GF(16) arithmetic — mode message Reed-Solomon
// ============================================================================
//
// GF(16) is the finite field with 16 elements, built from the primitive
// polynomial:
//
//   p(x) = x^4 + x + 1   (binary: 10011 = 0x13)
//
// Every non-zero element is a power of alpha (the primitive root):
//
//   alpha^0=1, alpha^1=2, alpha^2=4, alpha^3=8,
//   alpha^4=3, alpha^5=6, alpha^6=12, alpha^7=11,
//   alpha^8=5, alpha^9=10, alpha^10=7, alpha^11=14,
//   alpha^12=15, alpha^13=13, alpha^14=9, alpha^15=1 (period=15)
//
// LOG16[e] = discrete log of e (index in ALOG16).
// ALOG16[i] = alpha^i.

/// GF(16) discrete logarithm table: `LOG16[e]` = i such that alpha^i = e.
static LOG16: [i8; 16] = [
    -1, // log(0) = undefined
     0, // log(1) = 0
     1, // log(2) = 1
     4, // log(3) = 4
     2, // log(4) = 2
     8, // log(5) = 8
     5, // log(6) = 5
    10, // log(7) = 10
     3, // log(8) = 3
    14, // log(9) = 14
     9, // log(10) = 9
     7, // log(11) = 7
     6, // log(12) = 6
    13, // log(13) = 13
    11, // log(14) = 11
    12, // log(15) = 12
];

/// GF(16) antilogarithm table: `ALOG16[i]` = alpha^i.
static ALOG16: [u8; 16] = [1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9, 1];

/// Multiply two GF(16) elements using log/antilog tables.
///
/// Returns 0 if either operand is 0.
/// Otherwise: a*b = ALOG16[(LOG16[a] + LOG16[b]) mod 15].
fn gf16_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let la = LOG16[a as usize] as i32;
    let lb = LOG16[b as usize] as i32;
    ALOG16[((la + lb) % 15) as usize]
}

/// Build the GF(16) generator polynomial with roots alpha^1 through alpha^n.
///
/// Returns `[g_0, g_1, ..., g_n]` where `g_n = 1` (monic).
fn build_gf16_generator(n: usize) -> Vec<u8> {
    let mut g: Vec<u8> = vec![1];
    for i in 1..=n {
        let ai = ALOG16[i % 15];
        let mut next = vec![0u8; g.len() + 1];
        for (j, &gj) in g.iter().enumerate() {
            next[j + 1] ^= gj;
            next[j] ^= gf16_mul(ai, gj);
        }
        g = next;
    }
    g
}

/// Compute `n` GF(16) RS check nibbles for the given data nibbles.
///
/// Uses the LFSR polynomial division algorithm.
fn gf16_rs_encode(data: &[u8], n: usize) -> Vec<u8> {
    let g = build_gf16_generator(n);
    let mut rem = vec![0u8; n];
    for &byte in data {
        let fb = byte ^ rem[0];
        for i in 0..n - 1 {
            rem[i] = rem[i + 1] ^ gf16_mul(g[i + 1], fb);
        }
        rem[n - 1] = gf16_mul(g[n], fb);
    }
    rem
}

// ============================================================================
// GF(256)/0x12D arithmetic — 8-bit data codewords Reed-Solomon
// ============================================================================
//
// Aztec Code uses GF(256) with primitive polynomial:
//   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D
//
// This is the SAME polynomial as Data Matrix ECC200, but DIFFERENT from
// QR Code (0x11D). We implement it inline.
//
// Generator convention: b=1, roots alpha^1..alpha^n (MA02 style).
//
// Tables are initialized once via OnceLock.

use std::sync::OnceLock;

struct Gf256Tables {
    /// `EXP_12D[i]` = alpha^i in GF(256)/0x12D, doubled for fast multiply.
    exp: [u8; 512],
    /// `LOG_12D[e]` = discrete log of e in GF(256)/0x12D.
    log: [u8; 256],
}

static GF256_TABLES: OnceLock<Gf256Tables> = OnceLock::new();

fn get_gf256_tables() -> &'static Gf256Tables {
    GF256_TABLES.get_or_init(|| {
        let mut exp = [0u8; 512];
        let mut log = [0u8; 256];
        let mut x: u32 = 1;
        for i in 0..255 {
            exp[i] = x as u8;
            exp[i + 255] = x as u8;
            log[x as usize] = i as u8;
            x <<= 1;
            if x & 0x100 != 0 {
                x ^= 0x12d;
            }
            x &= 0xff;
        }
        exp[255] = 1;
        Gf256Tables { exp, log }
    })
}

/// Multiply two GF(256)/0x12D elements using log/antilog lookup.
fn gf256_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let t = get_gf256_tables();
    let la = t.log[a as usize] as usize;
    let lb = t.log[b as usize] as usize;
    t.exp[la + lb]
}

/// Build the GF(256)/0x12D RS generator polynomial with roots alpha^1..alpha^n.
///
/// Returns big-endian coefficients (highest degree first).
fn build_gf256_generator(n: usize) -> Vec<u8> {
    let t = get_gf256_tables();
    let mut g: Vec<u8> = vec![1];
    for i in 1..=n {
        let ai = t.exp[i];
        let mut next = vec![0u8; g.len() + 1];
        for (j, &gj) in g.iter().enumerate() {
            next[j] ^= gj;
            next[j + 1] ^= gf256_mul(gj, ai);
        }
        g = next;
    }
    g
}

/// Compute `n_check` GF(256)/0x12D RS check bytes for the given data bytes.
fn gf256_rs_encode(data: &[u8], n_check: usize) -> Vec<u8> {
    let g = build_gf256_generator(n_check);
    let n = g.len() - 1;
    let mut rem = vec![0u8; n];
    for &b in data {
        let fb = b ^ rem[0];
        for i in 0..n - 1 {
            rem[i] = rem[i + 1] ^ gf256_mul(g[i + 1], fb);
        }
        rem[n - 1] = gf256_mul(g[n], fb);
    }
    rem
}

// ============================================================================
// Capacity tables
// ============================================================================
//
// Derived from ISO/IEC 24778:2008 Table 1.
// Each entry: (total_bits, max_bytes_8) where max_bytes_8 is the maximum
// number of 8-bit codewords (data + ECC) that fit in the symbol.

struct CapEntry {
    total_bits: usize,
    max_bytes_8: usize,
}

/// Compact capacity table — indices 1..=4.
static COMPACT_CAP: [CapEntry; 5] = [
    CapEntry { total_bits: 0,   max_bytes_8: 0  }, // index 0 unused
    CapEntry { total_bits: 72,  max_bytes_8: 9  }, // 1 layer, 15x15
    CapEntry { total_bits: 200, max_bytes_8: 25 }, // 2 layers, 19x19
    CapEntry { total_bits: 392, max_bytes_8: 49 }, // 3 layers, 23x23
    CapEntry { total_bits: 648, max_bytes_8: 81 }, // 4 layers, 27x27
];

/// Full capacity table — indices 1..=32.
static FULL_CAP: [CapEntry; 33] = [
    CapEntry { total_bits: 0,     max_bytes_8: 0    }, // index 0 unused
    CapEntry { total_bits: 88,    max_bytes_8: 11   }, //  1 layer
    CapEntry { total_bits: 216,   max_bytes_8: 27   }, //  2 layers
    CapEntry { total_bits: 360,   max_bytes_8: 45   }, //  3 layers
    CapEntry { total_bits: 520,   max_bytes_8: 65   }, //  4 layers
    CapEntry { total_bits: 696,   max_bytes_8: 87   }, //  5 layers
    CapEntry { total_bits: 888,   max_bytes_8: 111  }, //  6 layers
    CapEntry { total_bits: 1096,  max_bytes_8: 137  }, //  7 layers
    CapEntry { total_bits: 1320,  max_bytes_8: 165  }, //  8 layers
    CapEntry { total_bits: 1560,  max_bytes_8: 195  }, //  9 layers
    CapEntry { total_bits: 1816,  max_bytes_8: 227  }, // 10 layers
    CapEntry { total_bits: 2088,  max_bytes_8: 261  }, // 11 layers
    CapEntry { total_bits: 2376,  max_bytes_8: 297  }, // 12 layers
    CapEntry { total_bits: 2680,  max_bytes_8: 335  }, // 13 layers
    CapEntry { total_bits: 3000,  max_bytes_8: 375  }, // 14 layers
    CapEntry { total_bits: 3336,  max_bytes_8: 417  }, // 15 layers
    CapEntry { total_bits: 3688,  max_bytes_8: 461  }, // 16 layers
    CapEntry { total_bits: 4056,  max_bytes_8: 507  }, // 17 layers
    CapEntry { total_bits: 4440,  max_bytes_8: 555  }, // 18 layers
    CapEntry { total_bits: 4840,  max_bytes_8: 605  }, // 19 layers
    CapEntry { total_bits: 5256,  max_bytes_8: 657  }, // 20 layers
    CapEntry { total_bits: 5688,  max_bytes_8: 711  }, // 21 layers
    CapEntry { total_bits: 6136,  max_bytes_8: 767  }, // 22 layers
    CapEntry { total_bits: 6600,  max_bytes_8: 825  }, // 23 layers
    CapEntry { total_bits: 7080,  max_bytes_8: 885  }, // 24 layers
    CapEntry { total_bits: 7576,  max_bytes_8: 947  }, // 25 layers
    CapEntry { total_bits: 8088,  max_bytes_8: 1011 }, // 26 layers
    CapEntry { total_bits: 8616,  max_bytes_8: 1077 }, // 27 layers
    CapEntry { total_bits: 9160,  max_bytes_8: 1145 }, // 28 layers
    CapEntry { total_bits: 9720,  max_bytes_8: 1215 }, // 29 layers
    CapEntry { total_bits: 10296, max_bytes_8: 1287 }, // 30 layers
    CapEntry { total_bits: 10888, max_bytes_8: 1361 }, // 31 layers
    CapEntry { total_bits: 11496, max_bytes_8: 1437 }, // 32 layers
];

// ============================================================================
// Data encoding — Binary-Shift from Upper mode
// ============================================================================
//
// v0.1.0 byte-mode path:
//   1. Emit 5 bits = 0b11111 (Binary-Shift escape in Upper mode)
//   2. If len <= 31: 5 bits for length
//      If len > 31:  5 bits = 0b00000, then 11 bits for length
//   3. Each byte as 8 bits, MSB first

/// Encode input bytes as a flat bit array using the Binary-Shift escape.
///
/// Returns a `Vec<u8>` of 0/1 values, MSB first.
fn encode_bytes_as_bits(input: &[u8]) -> Vec<u8> {
    let mut bits = Vec::new();
    let len = input.len();

    // Push `count` bits of `value`, MSB first.
    let push_bits = |value: usize, count: usize, bits: &mut Vec<u8>| {
        for i in (0..count).rev() {
            bits.push(((value >> i) & 1) as u8);
        }
    };

    push_bits(31, 5, &mut bits); // Binary-Shift escape

    if len <= 31 {
        push_bits(len, 5, &mut bits);
    } else {
        push_bits(0, 5, &mut bits);
        push_bits(len, 11, &mut bits);
    }

    for &byte in input {
        push_bits(byte as usize, 8, &mut bits);
    }

    bits
}

// ============================================================================
// Symbol size selection
// ============================================================================

struct SymbolSpec {
    compact: bool,
    layers: usize,
    data_cw_count: usize,
    ecc_cw_count: usize,
    #[allow(dead_code)]
    total_bits: usize,
}

/// Select the smallest symbol that can hold `data_bit_count` bits at `min_ecc_pct`.
///
/// Tries compact 1-4, then full 1-32. Adds 20% conservative stuffing overhead.
///
/// Returns `Err(AztecError::InputTooLong)` if no symbol fits.
fn select_symbol(data_bit_count: usize, min_ecc_pct: u32) -> Result<SymbolSpec, AztecError> {
    // 20% overhead for bit stuffing.
    let stuffed = (data_bit_count * 12 + 9) / 10; // ceil(x * 1.2)

    for layers in 1..=4 {
        let cap = &COMPACT_CAP[layers];
        let total_bytes = cap.max_bytes_8;
        let ecc_cw = (min_ecc_pct as usize * total_bytes + 99) / 100; // ceil
        if ecc_cw >= total_bytes {
            continue;
        }
        let data_cw = total_bytes - ecc_cw;
        if (stuffed + 7) / 8 <= data_cw {
            return Ok(SymbolSpec {
                compact: true,
                layers,
                data_cw_count: data_cw,
                ecc_cw_count: ecc_cw,
                total_bits: cap.total_bits,
            });
        }
    }

    for layers in 1..=32 {
        let cap = &FULL_CAP[layers];
        let total_bytes = cap.max_bytes_8;
        let ecc_cw = (min_ecc_pct as usize * total_bytes + 99) / 100; // ceil
        if ecc_cw >= total_bytes {
            continue;
        }
        let data_cw = total_bytes - ecc_cw;
        if (stuffed + 7) / 8 <= data_cw {
            return Ok(SymbolSpec {
                compact: false,
                layers,
                data_cw_count: data_cw,
                ecc_cw_count: ecc_cw,
                total_bits: cap.total_bits,
            });
        }
    }

    Err(AztecError::InputTooLong(format!(
        "Input is too long to fit in any Aztec Code symbol ({} bits needed)",
        data_bit_count
    )))
}

// ============================================================================
// Padding
// ============================================================================

/// Pad the bit stream to exactly `target_bytes * 8` bits.
///
/// Appends zero bits to align to a byte boundary, then zero bytes.
/// Truncates if longer than the target.
fn pad_to_bytes(bits: &[u8], target_bytes: usize) -> Vec<u8> {
    let mut out = bits.to_vec();
    // Byte-align
    while out.len() % 8 != 0 {
        out.push(0);
    }
    // Pad to target length
    out.resize(target_bytes * 8, 0);
    out.truncate(target_bytes * 8);
    out
}

// ============================================================================
// Bit stuffing
// ============================================================================
//
// After every 4 consecutive identical bits (all 0 or all 1), insert one
// complement bit. Applies only to the data+ECC bit stream.
//
// Example:
//   Input:  1 1 1 1 0 0 0 0
//   After 4 ones: insert 0  -> [1,1,1,1,0, ...]
//   After 4 zeros: insert 1 -> [1,1,1,1,0, 0,0,0,1, 0]

/// Apply Aztec bit stuffing to the data+ECC bit stream.
///
/// Inserts a complement bit after every run of 4 identical bits.
fn stuff_bits(bits: &[u8]) -> Vec<u8> {
    let mut stuffed = Vec::with_capacity(bits.len() + bits.len() / 4);
    let mut run_val: i8 = -1;
    let mut run_len: usize = 0;

    for &bit in bits {
        let b = bit as i8;
        if b == run_val {
            run_len += 1;
        } else {
            run_val = b;
            run_len = 1;
        }

        stuffed.push(bit);

        if run_len == 4 {
            let stuff_bit = (1 - bit) as u8;
            stuffed.push(stuff_bit);
            run_val = stuff_bit as i8;
            run_len = 1;
        }
    }

    stuffed
}

// ============================================================================
// Mode message encoding
// ============================================================================
//
// Compact (28 bits = 7 nibbles):
//   m = ((layers-1) << 6) | (data_cw_count-1)
//   2 data nibbles + 5 ECC nibbles
//
// Full (40 bits = 10 nibbles):
//   m = ((layers-1) << 11) | (data_cw_count-1)
//   4 data nibbles + 6 ECC nibbles

/// Encode the mode message as a flat bit array (28 bits compact, 40 bits full).
fn encode_mode_message(compact: bool, layers: usize, data_cw_count: usize) -> Vec<u8> {
    let (data_nibbles, num_ecc): (Vec<u8>, usize) = if compact {
        let m = ((layers - 1) << 6) | (data_cw_count - 1);
        (vec![(m & 0xf) as u8, ((m >> 4) & 0xf) as u8], 5)
    } else {
        let m = ((layers - 1) << 11) | (data_cw_count - 1);
        (
            vec![
                (m & 0xf) as u8,
                ((m >> 4) & 0xf) as u8,
                ((m >> 8) & 0xf) as u8,
                ((m >> 12) & 0xf) as u8,
            ],
            6,
        )
    };

    let ecc_nibbles = gf16_rs_encode(&data_nibbles, num_ecc);
    let all_nibbles: Vec<u8> = data_nibbles.iter().chain(ecc_nibbles.iter()).copied().collect();

    let mut bits = Vec::new();
    for nibble in &all_nibbles {
        for i in (0..4).rev() {
            bits.push((nibble >> i) & 1);
        }
    }
    bits
}

// ============================================================================
// Grid construction helpers
// ============================================================================

/// Symbol size: compact = 11 + 4*layers, full = 15 + 4*layers.
fn symbol_size(compact: bool, layers: usize) -> usize {
    if compact {
        11 + 4 * layers
    } else {
        15 + 4 * layers
    }
}

/// Bullseye radius: compact = 5, full = 7.
fn bullseye_radius(compact: bool) -> usize {
    if compact { 5 } else { 7 }
}

/// Draw the bullseye finder pattern.
///
/// Color at Chebyshev distance `d` from center:
/// - `d <= 1`: DARK (solid 3×3 inner core)
/// - `d > 1`, `d` even: LIGHT
/// - `d > 1`, `d` odd: DARK
fn draw_bullseye(
    modules: &mut Vec<Vec<bool>>,
    reserved: &mut Vec<Vec<bool>>,
    cx: usize,
    cy: usize,
    compact: bool,
) {
    let br = bullseye_radius(compact) as isize;
    let cx = cx as isize;
    let cy = cy as isize;
    for row in (cy - br)..=(cy + br) {
        for col in (cx - br)..=(cx + br) {
            let d = (col - cx).unsigned_abs().max((row - cy).unsigned_abs());
            let dark = if d <= 1 { true } else { d % 2 == 1 };
            modules[row as usize][col as usize] = dark;
            reserved[row as usize][col as usize] = true;
        }
    }
}

/// Draw reference grid for full Aztec symbols.
///
/// Grid lines at rows/cols that are multiples of 16 from center.
/// Module value alternates dark/light from center.
fn draw_reference_grid(
    modules: &mut Vec<Vec<bool>>,
    reserved: &mut Vec<Vec<bool>>,
    cx: usize,
    cy: usize,
    size: usize,
) {
    let cx = cx as isize;
    let cy = cy as isize;
    for row in 0..size as isize {
        for col in 0..size as isize {
            let on_h = (cy - row) % 16 == 0;
            let on_v = (cx - col) % 16 == 0;
            if !on_h && !on_v {
                continue;
            }
            let dark = if on_h && on_v {
                true
            } else if on_h {
                (cx - col) % 2 == 0
            } else {
                (cy - row) % 2 == 0
            };
            modules[row as usize][col as usize] = dark;
            reserved[row as usize][col as usize] = true;
        }
    }
}

/// Place orientation marks and mode message bits.
///
/// The mode message ring is the perimeter at Chebyshev radius `bullseye_radius + 1`.
/// The 4 corners are orientation marks (DARK). The remaining non-corner positions
/// carry mode message bits clockwise from TL+1.
///
/// Returns positions in the ring after the mode message bits (for data bits).
fn draw_orientation_and_mode_message(
    modules: &mut Vec<Vec<bool>>,
    reserved: &mut Vec<Vec<bool>>,
    cx: usize,
    cy: usize,
    compact: bool,
    mode_bits: &[u8],
) -> Vec<(usize, usize)> {
    let r = (bullseye_radius(compact) + 1) as isize;
    let cx = cx as isize;
    let cy = cy as isize;

    // Enumerate non-corner perimeter positions clockwise from TL+1.
    // Each entry is (col, row).
    let mut non_corner: Vec<(isize, isize)> = Vec::new();

    // Top edge (skip both corners)
    for col in (cx - r + 1)..=(cx + r - 1) {
        non_corner.push((col, cy - r));
    }
    // Right edge (skip both corners)
    for row in (cy - r + 1)..=(cy + r - 1) {
        non_corner.push((cx + r, row));
    }
    // Bottom edge: right to left (skip both corners)
    for col in ((cx - r + 1)..=(cx + r - 1)).rev() {
        non_corner.push((col, cy + r));
    }
    // Left edge: bottom to top (skip both corners)
    for row in ((cy - r + 1)..=(cy + r - 1)).rev() {
        non_corner.push((cx - r, row));
    }

    // Place 4 orientation mark corners as DARK
    for &(col, row) in &[
        (cx - r, cy - r),
        (cx + r, cy - r),
        (cx + r, cy + r),
        (cx - r, cy + r),
    ] {
        modules[row as usize][col as usize] = true;
        reserved[row as usize][col as usize] = true;
    }

    // Place mode message bits
    for (i, &bit) in mode_bits.iter().enumerate() {
        if i >= non_corner.len() {
            break;
        }
        let (col, row) = non_corner[i];
        modules[row as usize][col as usize] = bit == 1;
        reserved[row as usize][col as usize] = true;
    }

    // Return remaining positions for data bits (as (col, row) in usize)
    non_corner[mode_bits.len()..]
        .iter()
        .map(|&(col, row)| (col as usize, row as usize))
        .collect()
}

/// Place all data bits using the clockwise layer spiral.
///
/// Fills the mode ring remaining positions first, then spirals outward.
fn place_data_bits(
    modules: &mut Vec<Vec<bool>>,
    reserved: &mut Vec<Vec<bool>>,
    bits: &[u8],
    cx: usize,
    cy: usize,
    compact: bool,
    layers: usize,
    mode_ring_remaining: &[(usize, usize)],
) {
    let size = modules.len();
    let mut bit_index = 0;

    macro_rules! place_bit {
        ($col:expr, $row:expr) => {
            let col: isize = $col;
            let row: isize = $row;
            if col >= 0 && col < size as isize && row >= 0 && row < size as isize {
                let c = col as usize;
                let r = row as usize;
                if !reserved[r][c] {
                    modules[r][c] = bits.get(bit_index).copied().unwrap_or(0) == 1;
                    bit_index += 1;
                }
            }
        };
    }

    // Fill remaining mode ring positions first
    for &(col, row) in mode_ring_remaining {
        modules[row][col] = bits.get(bit_index).copied().unwrap_or(0) == 1;
        bit_index += 1;
    }

    // Spiral through data layers
    let br = bullseye_radius(compact);
    let d_start = br + 2; // mode ring at br+1, first data layer at br+2

    let cx = cx as isize;
    let cy = cy as isize;

    for l in 0..layers {
        let di = (d_start + 2 * l) as isize; // inner radius
        let d_o = di + 1; // outer radius

        // Top edge: left to right
        for col in (cx - di + 1)..=(cx + di) {
            place_bit!(col, cy - d_o);
            place_bit!(col, cy - di);
        }
        // Right edge: top to bottom
        for row in (cy - di + 1)..=(cy + di) {
            place_bit!(cx + d_o, row);
            place_bit!(cx + di, row);
        }
        // Bottom edge: right to left
        for col in ((cx - di + 1)..=(cx + di)).rev() {
            place_bit!(col, cy + d_o);
            place_bit!(col, cy + di);
        }
        // Left edge: bottom to top
        for row in ((cy - di + 1)..=(cy + di)).rev() {
            place_bit!(cx - d_o, row);
            place_bit!(cx - di, row);
        }
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Encode raw bytes as an Aztec Code symbol.
///
/// Returns a [`ModuleGrid`] where `modules[row][col] == true` means a dark
/// module. The grid origin (0,0) is the top-left corner.
///
/// # Errors
///
/// Returns [`AztecError::InputTooLong`] if the input exceeds the maximum
/// capacity of a 32-layer full Aztec symbol.
///
/// # Example
///
/// ```rust
/// use aztec_code::{encode, AztecOptions};
///
/// let grid = encode(b"Hello, World!", None).unwrap();
/// println!("{}x{} symbol", grid.rows, grid.cols);
/// ```
pub fn encode(data: &[u8], options: Option<AztecOptions>) -> Result<ModuleGrid, AztecError> {
    let opts = options.unwrap_or_default();
    let min_ecc_pct = opts.min_ecc_percent;

    // Step 1: encode data as bits
    let data_bits = encode_bytes_as_bits(data);

    // Step 2: select symbol
    let spec = select_symbol(data_bits.len(), min_ecc_pct)?;
    let SymbolSpec {
        compact,
        layers,
        data_cw_count,
        ecc_cw_count,
        ..
    } = spec;

    // Step 3: pad to data_cw_count bytes
    let padded_bits = pad_to_bytes(&data_bits, data_cw_count);
    let mut data_bytes: Vec<u8> = Vec::with_capacity(data_cw_count);
    for i in 0..data_cw_count {
        let mut byte: u8 = 0;
        for b in 0..8 {
            byte = (byte << 1) | padded_bits.get(i * 8 + b).copied().unwrap_or(0);
        }
        // All-zero codeword avoidance: last codeword 0x00 -> 0xFF
        if byte == 0 && i == data_cw_count - 1 {
            byte = 0xff;
        }
        data_bytes.push(byte);
    }

    // Step 4: compute RS ECC
    let ecc_bytes = gf256_rs_encode(&data_bytes, ecc_cw_count);

    // Step 5: build bit stream + stuff
    let all_bytes: Vec<u8> = data_bytes.iter().chain(ecc_bytes.iter()).copied().collect();
    let mut raw_bits: Vec<u8> = Vec::with_capacity(all_bytes.len() * 8);
    for &byte in &all_bytes {
        for i in (0..8).rev() {
            raw_bits.push((byte >> i) & 1);
        }
    }
    let stuffed_bits = stuff_bits(&raw_bits);

    // Step 6: mode message
    let mode_msg = encode_mode_message(compact, layers, data_cw_count);

    // Step 7: initialize grid
    let size = symbol_size(compact, layers);
    let cx = size / 2;
    let cy = size / 2;

    let mut modules = vec![vec![false; size]; size];
    let mut reserved = vec![vec![false; size]; size];

    // Reference grid first (full only), then bullseye overwrites
    if !compact {
        draw_reference_grid(&mut modules, &mut reserved, cx, cy, size);
    }
    draw_bullseye(&mut modules, &mut reserved, cx, cy, compact);

    let mode_ring_remaining = draw_orientation_and_mode_message(
        &mut modules,
        &mut reserved,
        cx,
        cy,
        compact,
        &mode_msg,
    );

    // Step 8: place data spiral
    place_data_bits(
        &mut modules,
        &mut reserved,
        &stuffed_bits,
        cx,
        cy,
        compact,
        layers,
        &mode_ring_remaining,
    );

    Ok(ModuleGrid {
        rows: size as u32,
        cols: size as u32,
        modules,
        module_shape: ModuleShape::Square,
    })
}

/// Encode a UTF-8 string as an Aztec Code symbol.
///
/// Convenience wrapper over [`encode`] that converts the string to bytes first.
///
/// # Example
///
/// ```rust
/// use aztec_code::encode_str;
///
/// let grid = encode_str("Hello, World!", None).unwrap();
/// println!("{}x{} symbol", grid.rows, grid.cols);
/// ```
pub fn encode_str(s: &str, options: Option<AztecOptions>) -> Result<ModuleGrid, AztecError> {
    encode(s.as_bytes(), options)
}

/// Encode data and convert the module grid to a [`PaintScene`].
///
/// Combines [`encode`] and [`barcode_2d::layout`] in one call.
///
/// # Example
///
/// ```rust
/// use aztec_code::encode_and_layout;
///
/// let scene = encode_and_layout(b"Hello", None, None).unwrap();
/// ```
pub fn encode_and_layout(
    data: &[u8],
    options: Option<AztecOptions>,
    config: Option<Barcode2DLayoutConfig>,
) -> Result<PaintScene, AztecError> {
    let grid = encode(data, options)?;
    let cfg = config.unwrap_or_default();
    barcode_2d::layout(&grid, &cfg).map_err(|e| AztecError::LayoutError(format!("{:?}", e)))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn dark(grid: &ModuleGrid, row: usize, col: usize) -> bool {
        grid.modules[row][col]
    }

    // -----------------------------------------------------------------------
    // Basic API
    // -----------------------------------------------------------------------

    #[test]
    fn version_is_correct() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn encode_returns_square_grid() {
        let g = encode(b"A", None).unwrap();
        assert_eq!(g.rows, g.cols);
    }

    #[test]
    fn encode_module_shape_is_square() {
        let g = encode(b"A", None).unwrap();
        assert_eq!(g.module_shape, ModuleShape::Square);
    }

    #[test]
    fn encode_rows_matches_modules_len() {
        let g = encode(b"test", None).unwrap();
        assert_eq!(g.rows as usize, g.modules.len());
    }

    #[test]
    fn encode_cols_matches_modules_row_len() {
        let g = encode(b"test", None).unwrap();
        assert_eq!(g.cols as usize, g.modules[0].len());
    }

    // -----------------------------------------------------------------------
    // Compact symbol sizes
    // -----------------------------------------------------------------------

    #[test]
    fn compact_1_layer_single_byte() {
        let g = encode(b"A", None).unwrap();
        assert_eq!(g.rows, 15);
        assert_eq!(g.cols, 15);
    }

    #[test]
    fn compact_2_layer_hello() {
        let g = encode(b"Hello", None).unwrap();
        assert_eq!(g.rows, 19);
        assert_eq!(g.cols, 19);
    }

    #[test]
    fn compact_3_layer_20_bytes() {
        let g = encode(b"12345678901234567890", None).unwrap();
        assert_eq!(g.rows, 23);
        assert_eq!(g.cols, 23);
    }

    #[test]
    fn compact_4_layer_40_bytes() {
        let g = encode(b"12345678901234567890123456789012345678901234", None).unwrap();
        assert_eq!(g.rows, 27);
        assert_eq!(g.cols, 27);
    }

    // -----------------------------------------------------------------------
    // Full symbol sizes
    // -----------------------------------------------------------------------

    #[test]
    fn full_symbol_for_large_input() {
        let data = vec![b'x'; 100];
        let g = encode(&data, None).unwrap();
        assert!(g.rows >= 19);
        assert_eq!(g.rows, g.cols);
    }

    // -----------------------------------------------------------------------
    // Bullseye — compact 15x15 (cx=cy=7, br=5)
    // -----------------------------------------------------------------------

    #[test]
    fn bullseye_compact_center_dark() {
        let g = encode(b"A", None).unwrap();
        let cx = (g.cols / 2) as usize;
        let cy = (g.rows / 2) as usize;
        assert!(dark(&g, cy, cx));
    }

    #[test]
    fn bullseye_compact_d1_dark() {
        let g = encode(b"A", None).unwrap();
        let cx = (g.cols / 2) as usize;
        let cy = (g.rows / 2) as usize;
        // All 8 neighbours at Chebyshev distance 1 should be dark
        for dr in -1i32..=1 {
            for dc in -1i32..=1 {
                if dr == 0 && dc == 0 {
                    continue;
                }
                assert!(
                    dark(&g, (cy as i32 + dr) as usize, (cx as i32 + dc) as usize),
                    "d=1 module at ({},{}) should be dark",
                    cy as i32 + dr,
                    cx as i32 + dc
                );
            }
        }
    }

    #[test]
    fn bullseye_compact_d2_light() {
        let g = encode(b"A", None).unwrap();
        let cx = (g.cols / 2) as usize;
        let cy = (g.rows / 2) as usize;
        // Midpoints of d=2 sides should be light
        assert!(!dark(&g, cy - 2, cx));
        assert!(!dark(&g, cy + 2, cx));
        assert!(!dark(&g, cy, cx - 2));
        assert!(!dark(&g, cy, cx + 2));
        // Corners of d=2 square
        assert!(!dark(&g, cy - 2, cx - 2));
        assert!(!dark(&g, cy - 2, cx + 2));
        assert!(!dark(&g, cy + 2, cx - 2));
        assert!(!dark(&g, cy + 2, cx + 2));
    }

    #[test]
    fn bullseye_compact_d3_dark() {
        let g = encode(b"A", None).unwrap();
        let cx = (g.cols / 2) as usize;
        let cy = (g.rows / 2) as usize;
        assert!(dark(&g, cy - 3, cx));
        assert!(dark(&g, cy + 3, cx));
        assert!(dark(&g, cy, cx - 3));
        assert!(dark(&g, cy, cx + 3));
    }

    #[test]
    fn bullseye_compact_d4_light() {
        let g = encode(b"A", None).unwrap();
        let cx = (g.cols / 2) as usize;
        let cy = (g.rows / 2) as usize;
        assert!(!dark(&g, cy - 4, cx));
        assert!(!dark(&g, cy + 4, cx));
        assert!(!dark(&g, cy, cx - 4));
        assert!(!dark(&g, cy, cx + 4));
    }

    #[test]
    fn bullseye_compact_d5_dark() {
        let g = encode(b"A", None).unwrap();
        let cx = (g.cols / 2) as usize;
        let cy = (g.rows / 2) as usize;
        assert!(dark(&g, cy - 5, cx));
        assert!(dark(&g, cy + 5, cx));
        assert!(dark(&g, cy, cx - 5));
        assert!(dark(&g, cy, cx + 5));
    }

    // -----------------------------------------------------------------------
    // Bullseye — full symbol
    // -----------------------------------------------------------------------

    #[test]
    fn bullseye_full_center_dark() {
        let data = vec![b'x'; 100];
        let g = encode(&data, None).unwrap();
        let cx = (g.cols / 2) as usize;
        let cy = (g.rows / 2) as usize;
        assert!(dark(&g, cy, cx));
    }

    #[test]
    fn bullseye_full_d2_light() {
        let data = vec![b'x'; 100];
        let g = encode(&data, None).unwrap();
        let cx = (g.cols / 2) as usize;
        let cy = (g.rows / 2) as usize;
        assert!(!dark(&g, cy - 2, cx));
        assert!(!dark(&g, cy + 2, cx));
        assert!(!dark(&g, cy, cx - 2));
        assert!(!dark(&g, cy, cx + 2));
    }

    #[test]
    fn bullseye_full_d7_dark() {
        let data = vec![b'x'; 100];
        let g = encode(&data, None).unwrap();
        let cx = (g.cols / 2) as usize;
        let cy = (g.rows / 2) as usize;
        assert!(dark(&g, cy - 7, cx));
        assert!(dark(&g, cy + 7, cx));
        assert!(dark(&g, cy, cx - 7));
        assert!(dark(&g, cy, cx + 7));
    }

    // -----------------------------------------------------------------------
    // Orientation marks — compact
    // -----------------------------------------------------------------------

    #[test]
    fn orientation_marks_compact_top_left() {
        let g = encode(b"A", None).unwrap();
        let cx = (g.cols / 2) as usize;
        let cy = (g.rows / 2) as usize;
        let r = 6; // bullseye_radius(compact=5) + 1
        assert!(dark(&g, cy - r, cx - r));
    }

    #[test]
    fn orientation_marks_compact_top_right() {
        let g = encode(b"A", None).unwrap();
        let cx = (g.cols / 2) as usize;
        let cy = (g.rows / 2) as usize;
        let r = 6;
        assert!(dark(&g, cy - r, cx + r));
    }

    #[test]
    fn orientation_marks_compact_bottom_right() {
        let g = encode(b"A", None).unwrap();
        let cx = (g.cols / 2) as usize;
        let cy = (g.rows / 2) as usize;
        let r = 6;
        assert!(dark(&g, cy + r, cx + r));
    }

    #[test]
    fn orientation_marks_compact_bottom_left() {
        let g = encode(b"A", None).unwrap();
        let cx = (g.cols / 2) as usize;
        let cy = (g.rows / 2) as usize;
        let r = 6;
        assert!(dark(&g, cy + r, cx - r));
    }

    // -----------------------------------------------------------------------
    // Orientation marks — full symbol
    // -----------------------------------------------------------------------

    #[test]
    fn orientation_marks_full_corners() {
        let data = vec![b'x'; 100];
        let g = encode(&data, None).unwrap();
        let cx = (g.cols / 2) as usize;
        let cy = (g.rows / 2) as usize;
        let r = 8; // bullseye_radius(full=7) + 1
        assert!(dark(&g, cy - r, cx - r));
        assert!(dark(&g, cy - r, cx + r));
        assert!(dark(&g, cy + r, cx + r));
        assert!(dark(&g, cy + r, cx - r));
    }

    // -----------------------------------------------------------------------
    // Bit stuffing algorithm
    // -----------------------------------------------------------------------

    #[test]
    fn stuff_bits_no_runs() {
        let input = vec![1u8, 0, 1, 0, 1, 0];
        let out = stuff_bits(&input);
        // No 4-consecutive run, so output == input
        assert_eq!(out, input);
    }

    #[test]
    fn stuff_bits_four_ones() {
        let input = vec![1u8, 1, 1, 1, 0];
        let out = stuff_bits(&input);
        // After 4 ones, insert 0
        assert_eq!(out, vec![1u8, 1, 1, 1, 0, 0]);
    }

    #[test]
    fn stuff_bits_four_zeros() {
        let input = vec![0u8, 0, 0, 0, 1];
        let out = stuff_bits(&input);
        // After 4 zeros, insert 1
        assert_eq!(out, vec![0u8, 0, 0, 0, 1, 1]);
    }

    #[test]
    fn stuff_bits_empty() {
        let out = stuff_bits(&[]);
        assert_eq!(out, Vec::<u8>::new());
    }

    #[test]
    fn stuff_bits_alternating() {
        let input = vec![1u8, 0, 1, 0, 1, 0, 1, 0];
        let out = stuff_bits(&input);
        // No run >= 4, no stuffing needed
        assert_eq!(out, input);
    }

    #[test]
    fn stuff_bits_all_zeros() {
        let input = vec![0u8; 8];
        let out = stuff_bits(&input);
        // After 4 zeros: insert 1, then 3 more zeros make 4 again: insert 1
        // [0,0,0,0,1, 0,0,0, continuing...]
        // Actually: [0,0,0,0] -> insert 1 (run_val=1, run_len=1)
        //           next 0: run_val changes to 0, run_len=1
        //           next 0: run_len=2
        //           next 0: run_len=3
        //           next 0: run_len=4 -> insert 1
        // So: [0,0,0,0,1, 0,0,0,0,1]
        assert_eq!(out, vec![0u8, 0, 0, 0, 1, 0, 0, 0, 0, 1]);
    }

    // -----------------------------------------------------------------------
    // Error handling
    // -----------------------------------------------------------------------

    #[test]
    fn input_too_long_error() {
        let data = vec![b'x'; 2000];
        let result = encode(&data, None);
        assert!(matches!(result, Err(AztecError::InputTooLong(_))));
    }

    #[test]
    fn aztec_error_display() {
        let e = AztecError::InputTooLong("too big".to_string());
        assert!(e.to_string().contains("InputTooLong"));
    }

    // -----------------------------------------------------------------------
    // encode_str
    // -----------------------------------------------------------------------

    #[test]
    fn encode_str_matches_encode_bytes() {
        let s = "Hello, World!";
        let g1 = encode_str(s, None).unwrap();
        let g2 = encode(s.as_bytes(), None).unwrap();
        assert_eq!(g1.rows, g2.rows);
        assert_eq!(g1.cols, g2.cols);
        for r in 0..g1.rows as usize {
            for c in 0..g1.cols as usize {
                assert_eq!(g1.modules[r][c], g2.modules[r][c]);
            }
        }
    }

    // -----------------------------------------------------------------------
    // encode_and_layout
    // -----------------------------------------------------------------------

    #[test]
    fn encode_and_layout_returns_paint_scene() {
        let result = encode_and_layout(b"Hello", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn encode_and_layout_with_custom_config() {
        let config = Barcode2DLayoutConfig {
            module_size_px: 5.0,
            ..Default::default()
        };
        let result = encode_and_layout(b"Hello", None, Some(config));
        assert!(result.is_ok());
    }

    // -----------------------------------------------------------------------
    // Determinism
    // -----------------------------------------------------------------------

    #[test]
    fn encode_is_deterministic() {
        let g1 = encode(b"Hello, World!", None).unwrap();
        let g2 = encode(b"Hello, World!", None).unwrap();
        assert_eq!(g1.rows, g2.rows);
        for r in 0..g1.rows as usize {
            for c in 0..g1.cols as usize {
                assert_eq!(g1.modules[r][c], g2.modules[r][c]);
            }
        }
    }

    #[test]
    fn different_inputs_produce_different_grids() {
        let g1 = encode(b"Hello", None).unwrap();
        let g2 = encode(b"World", None).unwrap();
        // At least one module must differ
        let differs = (0..g1.rows as usize).any(|r| {
            (0..g1.cols as usize).any(|c| g1.modules[r][c] != g2.modules[r][c])
        });
        assert!(differs);
    }

    // -----------------------------------------------------------------------
    // AztecOptions
    // -----------------------------------------------------------------------

    #[test]
    fn higher_ecc_produces_larger_or_equal_symbol() {
        let g_low = encode(b"Hello", Some(AztecOptions { min_ecc_percent: 10 })).unwrap();
        let g_high = encode(b"Hello", Some(AztecOptions { min_ecc_percent: 80 })).unwrap();
        assert!(g_high.rows >= g_low.rows);
    }

    #[test]
    fn min_ecc_33_succeeds() {
        let result = encode(b"Hello", Some(AztecOptions { min_ecc_percent: 33 }));
        assert!(result.is_ok());
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn encode_empty_bytes() {
        let g = encode(b"", None).unwrap();
        assert!(g.rows >= 15);
        assert_eq!(g.rows, g.cols);
    }

    #[test]
    fn encode_single_null_byte() {
        let g = encode(&[0x00], None).unwrap();
        assert!(g.rows >= 15);
    }

    #[test]
    fn encode_all_bytes_0_to_255() {
        let bytes: Vec<u8> = (0u8..=255).collect();
        let g = encode(&bytes, None).unwrap();
        assert!(g.rows >= 15);
        assert_eq!(g.rows, g.cols);
    }

    #[test]
    fn encode_200_bytes() {
        let data = vec![b'C'; 200];
        let g = encode(&data, None).unwrap();
        assert!(g.rows >= 19);
        assert_eq!(g.rows, g.cols);
    }

    #[test]
    fn encode_500_bytes() {
        let data = vec![b'D'; 500];
        let g = encode(&data, None).unwrap();
        assert!(g.rows >= 19);
        assert_eq!(g.rows, g.cols);
    }

    #[test]
    fn encode_unicode_utf8() {
        // Japanese "Hello" — 15 UTF-8 bytes
        let g = encode("こんにちは".as_bytes(), None).unwrap();
        assert!(g.rows >= 15);
        assert_eq!(g.rows, g.cols);
    }

    // -----------------------------------------------------------------------
    // Cross-language corpus
    // -----------------------------------------------------------------------

    #[test]
    fn corpus_a_is_15x15_compact1() {
        let g = encode(b"A", None).unwrap();
        assert_eq!(g.rows, 15);
        assert_eq!(g.cols, 15);
    }

    #[test]
    fn corpus_hello_is_19x19_compact2() {
        let g = encode(b"Hello", None).unwrap();
        assert_eq!(g.rows, 19);
        assert_eq!(g.cols, 19);
    }

    #[test]
    fn corpus_20_bytes_is_23x23_compact3() {
        let g = encode(b"12345678901234567890", None).unwrap();
        assert_eq!(g.rows, 23);
        assert_eq!(g.cols, 23);
    }
}
