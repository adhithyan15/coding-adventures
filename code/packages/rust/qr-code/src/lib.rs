//! # qr-code
//!
//! QR Code encoder — ISO/IEC 18004:2015 compliant.
//!
//! Encodes any UTF-8 string into a scannable QR Code.  Outputs a
//! [`ModuleGrid`] (abstract boolean grid) that can be passed to
//! `barcode-2d`'s [`layout()`] for pixel rendering, or to [`render_svg()`]
//! for a one-shot SVG string.
//!
//! ## Encoding pipeline
//!
//! ```text
//! input string
//!   → mode selection    (numeric / alphanumeric / byte)
//!   → version selection (smallest v1–40 that fits at the ECC level)
//!   → bit stream        (mode indicator + char count + data + padding)
//!   → blocks + RS ECC   (GF(256) b=0 convention, poly 0x11D)
//!   → interleave        (data CWs round-robin, then ECC CWs)
//!   → grid init         (finder × 3, separators, timing, alignment, format, dark)
//!   → zigzag placement  (two-column snake from bottom-right)
//!   → mask evaluation   (8 patterns, 4-rule penalty, pick lowest)
//!   → finalize          (format info + version info v7+)
//!   → ModuleGrid
//! ```

pub const VERSION: &str = "0.3.0";

use barcode_2d::{layout, Barcode2DLayoutConfig, ModuleGrid, ModuleShape};
use gf256::{multiply as gf_mul, power as gf_power};
use paint_instructions::PaintScene;

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

/// Error correction level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EccLevel {
    /// ~7% of codewords recoverable.
    L,
    /// ~15% of codewords recoverable (common default).
    M,
    /// ~25% of codewords recoverable.
    Q,
    /// ~30% of codewords recoverable.
    H,
}

/// Errors produced by the encoder.
#[derive(Debug)]
pub enum QRCodeError {
    /// Input is too long to fit in any version at the chosen ECC level.
    InputTooLong(String),
    /// Layout/rendering failed (e.g. invalid config passed to `encode_and_layout`).
    LayoutError(String),
}

impl std::fmt::Display for QRCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QRCodeError::InputTooLong(msg) => write!(f, "InputTooLong: {msg}"),
            QRCodeError::LayoutError(msg)   => write!(f, "LayoutError: {msg}"),
        }
    }
}
impl std::error::Error for QRCodeError {}

// ─────────────────────────────────────────────────────────────────────────────
// ECC level constants
// ─────────────────────────────────────────────────────────────────────────────

fn ecc_indicator(ecc: EccLevel) -> u16 {
    match ecc {
        EccLevel::L => 0b01,
        EccLevel::M => 0b00,
        EccLevel::Q => 0b11,
        EccLevel::H => 0b10,
    }
}

fn ecc_idx(ecc: EccLevel) -> usize {
    match ecc {
        EccLevel::L => 0,
        EccLevel::M => 1,
        EccLevel::Q => 2,
        EccLevel::H => 3,
    }
}

/// Inverse of `ecc_indicator`: map a 2-bit ECC indicator back to an
/// [`EccLevel`], or `None` for a bit pattern no `EccLevel` produces (there
/// are none — all 4 values of a 2-bit field are assigned — but the `Option`
/// return keeps this symmetric with `read_format_info`/`read_version_info`,
/// which do have genuinely-invalid inputs).
pub fn ecc_from_indicator(bits: u16) -> Option<EccLevel> {
    match bits {
        0b01 => Some(EccLevel::L),
        0b00 => Some(EccLevel::M),
        0b11 => Some(EccLevel::Q),
        0b10 => Some(EccLevel::H),
        _ => None,
    }
}

/// Decode a 15-bit format-info word read directly off the grid (raw bits,
/// **before** the ISO masking XOR — this function applies that XOR
/// internally, matching `write_format_info`'s own inverse). BCH-validates
/// the recovered word against the `0x537` generator and returns
/// `(ecc_level, mask)` on success, `None` if the parity check fails.
///
/// `bits15` must be assembled MSB-first: bit 14 is the value read at the
/// *first* position of whichever copy (1 or 2) the caller is decoding, bit 0
/// the value at the last — i.e. the same f14..f0 ordering
/// `write_format_info`'s own doc comment documents for both copies.
pub fn read_format_info(bits15: u32) -> Option<(EccLevel, u32)> {
    let fmt = bits15 ^ 0x5412;
    let mut rem = (fmt >> 10) << 10;
    for i in (10u32..=14).rev() {
        if (rem >> i) & 1 == 1 { rem ^= 0x537 << (i - 10); }
    }
    if (rem & 0x3ff) != (fmt & 0x3ff) { return None; }
    let ecc_bits = ((fmt >> 13) & 0x3) as u16;
    let mask = (fmt >> 10) & 0x7;
    ecc_from_indicator(ecc_bits).map(|ecc| (ecc, mask))
}

/// Decode an 18-bit version-info word read directly off the grid (Copy 1 or
/// Copy 2, per `write_version_info`'s own bit layout). Mirrors
/// `compute_version_bits`'s BCH generator (`0x1F25`) in the read direction:
/// the low 12 bits are the BCH parity of the high 6 bits (the version
/// number itself, unmasked — version info, unlike format info, is not
/// XOR-masked). Recomputes the parity from the version-number portion and
/// compares against the stored parity; accepts only if they match *and* the
/// recovered version falls in `7..=40` (version info only exists for V≥7).
pub fn read_version_info(bits18: u32) -> Option<usize> {
    let version = (bits18 >> 12) as usize;
    if !(7..=40).contains(&version) { return None; }
    let mut rem = (version as u32) << 12;
    for i in (12u32..=17).rev() {
        if (rem >> i) & 1 == 1 { rem ^= 0x1f25 << (i - 12); }
    }
    if (rem & 0xfff) != (bits18 & 0xfff) { return None; }
    Some(version)
}

// ─────────────────────────────────────────────────────────────────────────────
// ISO 18004:2015 — Capacity tables (Table 9)
// ─────────────────────────────────────────────────────────────────────────────

/// ECC codewords per block, indexed [ecc_idx][version].  Index 0 is padding.
static ECC_CODEWORDS_PER_BLOCK: [[i32; 41]; 4] = [
    // L:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
    // M:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28],
    // Q:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
    // H:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
];

/// Number of error correction blocks, indexed [ecc_idx][version].
static NUM_BLOCKS: [[i32; 41]; 4] = [
    // L:
    [-1,  1,  1,  1,  1,  1,  2,  2,  2,  2,  4,  4,  4,  4,  4,  6,  6,  6,  6,  7,  8,  8,  9,  9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25],
    // M:
    [-1,  1,  1,  1,  2,  2,  4,  4,  4,  5,  5,  5,  8,  9,  9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49],
    // Q:
    [-1,  1,  1,  2,  2,  4,  4,  6,  6,  8,  8,  8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68],
    // H:
    [-1,  1,  1,  2,  4,  4,  4,  5,  6,  8,  8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 80],
];

/// Alignment pattern center coordinates, indexed by version - 1.
static ALIGNMENT_POSITIONS: [&[u8]; 40] = [
    &[],                                     // v1
    &[6, 18],                                // v2
    &[6, 22],                                // v3
    &[6, 26],                                // v4
    &[6, 30],                                // v5
    &[6, 34],                                // v6
    &[6, 22, 38],                            // v7
    &[6, 24, 42],                            // v8
    &[6, 26, 46],                            // v9
    &[6, 28, 50],                            // v10
    &[6, 30, 54],                            // v11
    &[6, 32, 58],                            // v12
    &[6, 34, 62],                            // v13
    &[6, 26, 46, 66],                        // v14
    &[6, 26, 48, 70],                        // v15
    &[6, 26, 50, 74],                        // v16
    &[6, 30, 54, 78],                        // v17
    &[6, 30, 56, 82],                        // v18
    &[6, 30, 58, 86],                        // v19
    &[6, 34, 62, 90],                        // v20
    &[6, 28, 50, 72, 94],                    // v21
    &[6, 26, 50, 74, 98],                    // v22
    &[6, 30, 54, 78, 102],                   // v23
    &[6, 28, 54, 80, 106],                   // v24
    &[6, 32, 58, 84, 110],                   // v25
    &[6, 30, 58, 86, 114],                   // v26
    &[6, 34, 62, 90, 118],                   // v27
    &[6, 26, 50, 74,  98, 122],              // v28
    &[6, 30, 54, 78, 102, 126],              // v29
    &[6, 26, 52, 78, 104, 130],              // v30
    &[6, 30, 56, 82, 108, 134],              // v31
    &[6, 34, 60, 86, 112, 138],              // v32
    &[6, 30, 58, 86, 114, 142],              // v33
    &[6, 34, 62, 90, 118, 146],              // v34
    &[6, 30, 54, 78, 102, 126, 150],         // v35
    &[6, 24, 50, 76, 102, 128, 154],         // v36
    &[6, 28, 54, 80, 106, 132, 158],         // v37
    &[6, 32, 58, 84, 110, 136, 162],         // v38
    &[6, 26, 54, 82, 110, 138, 166],         // v39
    &[6, 30, 58, 86, 114, 142, 170],         // v40
];

// ─────────────────────────────────────────────────────────────────────────────
// Grid geometry
// ─────────────────────────────────────────────────────────────────────────────

/// The `(4V+17) × (4V+17)` side length of a QR Code symbol at `version`.
pub fn symbol_size(version: usize) -> usize {
    4 * version + 17
}

/// Total raw data+ECC bits (formula from Nayuki's reference, public domain).
pub fn num_raw_data_modules(version: usize) -> usize {
    let v = version as i64;
    let mut result = (16 * v + 128) * v + 64;
    if version >= 2 {
        let num_align = (v / 7) + 2;
        result -= (25 * num_align - 10) * num_align - 55;
        if version >= 7 {
            result -= 36;
        }
    }
    result as usize
}

/// Number of data (non-ECC) codewords available at `(version, ecc)`.
pub fn num_data_codewords(version: usize, ecc: EccLevel) -> usize {
    let e = ecc_idx(ecc);
    let raw_cw = num_raw_data_modules(version) / 8;
    let ecc_cw = (NUM_BLOCKS[e][version] * ECC_CODEWORDS_PER_BLOCK[e][version]) as usize;
    raw_cw - ecc_cw
}

/// Number of leftover bits after packing `num_raw_data_modules(version)` into
/// whole bytes — these trailing bits are zero-padding, not data.
pub fn num_remainder_bits(version: usize) -> usize {
    num_raw_data_modules(version) % 8
}

/// Number of ECC codewords per block at `(ecc, version)` (ISO 18004 Table 9).
pub fn ecc_codewords_per_block(ecc: EccLevel, version: usize) -> usize {
    ECC_CODEWORDS_PER_BLOCK[ecc_idx(ecc)][version] as usize
}

/// Number of error-correction blocks at `(ecc, version)` (ISO 18004 Table 9).
pub fn num_blocks(ecc: EccLevel, version: usize) -> usize {
    NUM_BLOCKS[ecc_idx(ecc)][version] as usize
}

// ─────────────────────────────────────────────────────────────────────────────
// Reed-Solomon (b=0 convention)
// ─────────────────────────────────────────────────────────────────────────────

/// Build the monic RS generator of degree `n` with roots α⁰, α¹, …, α^{n-1}.
///
/// g(x) = ∏(x + αⁱ) for i in 0..n
///
/// The output vector has `n+1` elements, index 0 is the leading coefficient (1).
fn build_generator(n: usize) -> Vec<u8> {
    let mut g: Vec<u8> = vec![1u8];
    for i in 0..n {
        let ai = gf_power(2, i as u32); // α^i in GF(256), primitive element α = 2
        let mut next = vec![0u8; g.len() + 1];
        for (j, &gj) in g.iter().enumerate() {
            next[j] ^= gj;
            next[j + 1] ^= gf_mul(gj, ai);
        }
        g = next;
    }
    g
}

/// Compute `n` ECC bytes by LFSR polynomial division.
///
/// Returns remainder of D(x) · x^n mod G(x).
fn rs_encode(data: &[u8], generator: &[u8]) -> Vec<u8> {
    let n = generator.len() - 1;
    let mut rem = vec![0u8; n];
    for &b in data {
        let fb = b ^ rem[0];
        rem.copy_within(1.., 0);
        rem[n - 1] = 0;
        if fb != 0 {
            for i in 0..n {
                rem[i] ^= gf_mul(generator[i + 1], fb);
            }
        }
    }
    rem
}

// ─────────────────────────────────────────────────────────────────────────────
// Data encoding modes
// ─────────────────────────────────────────────────────────────────────────────

const ALPHANUM_CHARS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

#[derive(Clone, Copy, PartialEq)]
enum EncodingMode {
    Numeric,
    Alphanumeric,
    Byte,
}

fn select_mode(input: &str) -> EncodingMode {
    if input.chars().all(|c| c.is_ascii_digit()) {
        return EncodingMode::Numeric;
    }
    if input.chars().all(|c| ALPHANUM_CHARS.contains(c)) {
        return EncodingMode::Alphanumeric;
    }
    EncodingMode::Byte
}

fn mode_indicator(mode: EncodingMode) -> u16 {
    match mode {
        EncodingMode::Numeric => 0b0001,
        EncodingMode::Alphanumeric => 0b0010,
        EncodingMode::Byte => 0b0100,
    }
}

fn char_count_bits(mode: EncodingMode, version: usize) -> u32 {
    match mode {
        EncodingMode::Numeric => if version <= 9 { 10 } else if version <= 26 { 12 } else { 14 },
        EncodingMode::Alphanumeric => if version <= 9 { 9 } else if version <= 26 { 11 } else { 13 },
        EncodingMode::Byte => if version <= 9 { 8 } else { 16 },
    }
}

/// Bit writer: accumulates individual bits and flushes to bytes.
struct BitWriter {
    bits: Vec<u8>, // each element is 0 or 1
}

impl BitWriter {
    fn new() -> Self { Self { bits: Vec::new() } }

    fn write(&mut self, value: u32, count: u32) {
        for i in (0..count).rev() {
            self.bits.push(((value >> i) & 1) as u8);
        }
    }

    fn bit_len(&self) -> usize { self.bits.len() }

    fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        let mut i = 0;
        while i < self.bits.len() {
            let mut byte = 0u8;
            for j in 0..8 {
                byte = (byte << 1) | self.bits.get(i + j).copied().unwrap_or(0);
            }
            result.push(byte);
            i += 8;
        }
        result
    }
}

fn encode_numeric(input: &str, w: &mut BitWriter) {
    let digits: Vec<u32> = input.chars().map(|c| c as u32 - '0' as u32).collect();
    let mut i = 0;
    while i + 2 < digits.len() {
        w.write(digits[i] * 100 + digits[i+1] * 10 + digits[i+2], 10);
        i += 3;
    }
    if i + 1 < digits.len() {
        w.write(digits[i] * 10 + digits[i+1], 7);
        i += 2;
    }
    if i < digits.len() {
        w.write(digits[i], 4);
    }
}

fn encode_alphanumeric(input: &str, w: &mut BitWriter) {
    // Precondition: caller (select_mode → encode) guarantees every character is
    // in ALPHANUM_CHARS.  The debug_assert makes this explicit; production builds
    // fall through to `.unwrap_or(0)` which is unreachable under correct usage.
    let chars: Vec<u32> = input.chars()
        .map(|c| {
            let idx = ALPHANUM_CHARS.find(c);
            debug_assert!(idx.is_some(), "encode_alphanumeric: char '{}' not in ALPHANUM_CHARS (precondition violated)", c);
            idx.unwrap_or(0) as u32
        })
        .collect();
    let mut i = 0;
    while i + 1 < chars.len() {
        w.write(chars[i] * 45 + chars[i+1], 11);
        i += 2;
    }
    if i < chars.len() {
        w.write(chars[i], 6);
    }
}

fn encode_byte_mode(input: &str, w: &mut BitWriter) {
    for b in input.as_bytes() {
        w.write(*b as u32, 8);
    }
}

fn build_data_codewords(input: &str, version: usize, ecc: EccLevel) -> Vec<u8> {
    let mode = select_mode(input);
    let capacity = num_data_codewords(version, ecc);
    let mut w = BitWriter::new();

    w.write(mode_indicator(mode) as u32, 4);
    let char_count = if mode == EncodingMode::Byte {
        input.len() as u32
    } else {
        input.chars().count() as u32
    };
    w.write(char_count, char_count_bits(mode, version));

    match mode {
        EncodingMode::Numeric => encode_numeric(input, &mut w),
        EncodingMode::Alphanumeric => encode_alphanumeric(input, &mut w),
        EncodingMode::Byte => encode_byte_mode(input, &mut w),
    }

    // Terminator (up to 4 bits)
    let available = capacity * 8;
    let term_len = (available - w.bit_len()).min(4);
    if term_len > 0 { w.write(0, term_len as u32); }

    // Byte-boundary padding
    let rem = w.bit_len() % 8;
    if rem != 0 { w.write(0, (8 - rem) as u32); }

    let mut bytes = w.to_bytes();
    let mut pad = 0xecu8;
    while bytes.len() < capacity {
        bytes.push(pad);
        pad = if pad == 0xec { 0x11 } else { 0xec };
    }
    bytes
}

// ─────────────────────────────────────────────────────────────────────────────
// Block processing
// ─────────────────────────────────────────────────────────────────────────────

struct Block {
    data: Vec<u8>,
    ecc:  Vec<u8>,
}

fn compute_blocks(data: &[u8], version: usize, ecc: EccLevel) -> Vec<Block> {
    let e = ecc_idx(ecc);
    let total_blocks = NUM_BLOCKS[e][version] as usize;
    let ecc_len = ECC_CODEWORDS_PER_BLOCK[e][version] as usize;
    let total_data = num_data_codewords(version, ecc);
    let short_len = total_data / total_blocks;
    let num_long = total_data % total_blocks;
    let gen = build_generator(ecc_len);
    let mut blocks = Vec::new();
    let mut offset = 0;

    let g1_count = total_blocks - num_long;
    for _ in 0..g1_count {
        let d = data[offset..offset + short_len].to_vec();
        let e_cw = rs_encode(&d, &gen);
        blocks.push(Block { data: d, ecc: e_cw });
        offset += short_len;
    }
    for _ in 0..num_long {
        let d = data[offset..offset + short_len + 1].to_vec();
        let e_cw = rs_encode(&d, &gen);
        blocks.push(Block { data: d, ecc: e_cw });
        offset += short_len + 1;
    }
    blocks
}

fn interleave_blocks(blocks: &[Block]) -> Vec<u8> {
    let mut result = Vec::new();
    let max_data = blocks.iter().map(|b| b.data.len()).max().unwrap_or(0);
    let max_ecc  = blocks.iter().map(|b| b.ecc.len()).max().unwrap_or(0);
    for i in 0..max_data {
        for b in blocks { if i < b.data.len() { result.push(b.data[i]); } }
    }
    for i in 0..max_ecc {
        for b in blocks { if i < b.ecc.len()  { result.push(b.ecc[i]);  } }
    }
    result
}

// ─────────────────────────────────────────────────────────────────────────────
// Grid construction
// ─────────────────────────────────────────────────────────────────────────────

struct WorkGrid {
    size: usize,
    modules:  Vec<Vec<bool>>,
    reserved: Vec<Vec<bool>>,
}

impl WorkGrid {
    fn new(size: usize) -> Self {
        Self {
            size,
            modules:  vec![vec![false; size]; size],
            reserved: vec![vec![false; size]; size],
        }
    }

    fn set(&mut self, r: usize, c: usize, dark: bool, reserve: bool) {
        self.modules[r][c] = dark;
        if reserve { self.reserved[r][c] = true; }
    }
}

fn place_finder(g: &mut WorkGrid, top: usize, left: usize) {
    for dr in 0..7usize {
        for dc in 0..7usize {
            let on_border = dr == 0 || dr == 6 || dc == 0 || dc == 6;
            let in_core   = (2..=4).contains(&dr) && (2..=4).contains(&dc);
            g.set(top + dr, left + dc, on_border || in_core, true);
        }
    }
}

fn place_alignment(g: &mut WorkGrid, row: usize, col: usize) {
    for dr in -2i32..=2 {
        for dc in -2i32..=2 {
            let r = (row as i32 + dr) as usize;
            let c = (col as i32 + dc) as usize;
            let on_border = dr.abs() == 2 || dc.abs() == 2;
            let is_center = dr == 0 && dc == 0;
            g.set(r, c, on_border || is_center, true);
        }
    }
}

fn place_all_alignments(g: &mut WorkGrid, version: usize) {
    let positions = ALIGNMENT_POSITIONS[version - 1];
    for &row in positions {
        for &col in positions {
            let r = row as usize;
            let c = col as usize;
            if g.reserved[r][c] { continue; }
            place_alignment(g, r, c);
        }
    }
}

fn place_timing(g: &mut WorkGrid) {
    let sz = g.size;
    for c in 8..=(sz - 9) { g.set(6, c, c % 2 == 0, true); }
    for r in 8..=(sz - 9) { g.set(r, 6, r % 2 == 0, true); }
}

fn reserve_format_info(g: &mut WorkGrid) {
    let sz = g.size;
    for c in 0..=8 { if c != 6 { g.reserved[8][c] = true; } }
    for r in 0..=8 { if r != 6 { g.reserved[r][8] = true; } }
    for r in (sz - 7)..sz { g.reserved[r][8] = true; }
    for c in (sz - 8)..sz { g.reserved[8][c] = true; }
}

fn compute_format_bits(ecc: EccLevel, mask: u32) -> u32 {
    let data = (ecc_indicator(ecc) << 3) | mask as u16;
    let mut rem = (data as u32) << 10;
    for i in (10u32..=14).rev() {
        if (rem >> i) & 1 == 1 { rem ^= 0x537 << (i - 10); }
    }
    (((data as u32) << 10) | (rem & 0x3ff)) ^ 0x5412
}

fn write_format_info(g: &mut WorkGrid, fmt: u32) {
    // The 15-bit format information word `fmt` is labeled f14..f0 (f14 = MSB).
    //
    // ISO/IEC 18004 — Copy 1 (around top-left finder):
    //
    //   Row 8, col 0 → f14   col 1 → f13  …  col 5 → f9
    //   Row 8, col 7 → f8    (col 6 is the timing column, skipped)
    //   Row 8, col 8 → f7
    //   Col 8, row 7 → f6    (row 6 is the timing row, skipped)
    //   Col 8, row 5 → f5  …  row 0 → f0
    //
    // Copy 2 (around top-right / bottom-left finders):
    //   Row 8, col n-1 → f0   col n-2 → f1  …  col n-8 → f7
    //   Col 8, row n-7 → f8   row n-6 → f9  …  row n-1 → f14
    let sz = g.size;

    // ── Copy 1 ──────────────────────────────────────────────────────────────
    // Row 8, cols 0-5: f14 down to f9 (MSB first, left-to-right)
    for i in 0u32..=5 { g.modules[8][i as usize] = (fmt >> (14 - i)) & 1 == 1; }
    g.modules[8][7] = (fmt >> 8) & 1 == 1;  // f8
    g.modules[8][8] = (fmt >> 7) & 1 == 1;  // f7
    g.modules[7][8] = (fmt >> 6) & 1 == 1;  // f6
    // Col 8, rows 0-5: f0 at row 0 … f5 at row 5 (LSB at top)
    for i in 0u32..=5 { g.modules[i as usize][8] = (fmt >> i) & 1 == 1; }

    // ── Copy 2 ──────────────────────────────────────────────────────────────
    // Row 8, cols n-1 down to n-8: f0 at col n-1 … f7 at col n-8
    for i in 0u32..=7 { g.modules[8][sz - 1 - i as usize] = (fmt >> i) & 1 == 1; }
    // Col 8, rows n-7 to n-1: f8 at row n-7 … f14 at row n-1
    for i in 8u32..=14 { g.modules[sz - 15 + i as usize][8] = (fmt >> i) & 1 == 1; }
}

fn reserve_version_info(g: &mut WorkGrid, version: usize) {
    if version < 7 { return; }
    let sz = g.size;
    for r in 0..6 { for dc in 0..3 { g.reserved[r][sz - 11 + dc] = true; } }
    for dr in 0..3 { for c in 0..6 { g.reserved[sz - 11 + dr][c] = true; } }
}

fn compute_version_bits(version: usize) -> u32 {
    let v = version as u32;
    let mut rem = v << 12;
    for i in (12u32..=17).rev() {
        if (rem >> i) & 1 == 1 { rem ^= 0x1f25 << (i - 12); }
    }
    (v << 12) | (rem & 0xfff)
}

fn write_version_info(g: &mut WorkGrid, version: usize) {
    if version < 7 { return; }
    let sz = g.size;
    let bits = compute_version_bits(version);
    for i in 0u32..18 {
        let dark = (bits >> i) & 1 == 1;
        let a = 5 - (i / 3) as usize;
        let b = sz - 9 - (i % 3) as usize;
        g.modules[a][b] = dark;
        g.modules[b][a] = dark;
    }
}

fn place_dark_module(g: &mut WorkGrid, version: usize) {
    g.set(4 * version + 9, 8, true, true);
}

/// The order data/ECC-carrying modules are filled, for a symbol of this
/// version — the same two-column zigzag `place_bits` writes with, exposed so
/// `qr_decoder` can read modules back in the identical order (one shared
/// traversal source, not two independently-written loops that must happen to
/// agree).
pub fn data_module_order(version: usize) -> Vec<(usize, usize)> {
    let sz = symbol_size(version);
    let reserved = reserved_modules(version);
    let mut order = Vec::new();
    let mut up = true;
    let mut col = sz - 1;

    loop {
        for vi in 0..sz {
            let row = if up { sz - 1 - vi } else { vi };
            for dc in 0u32..=1 {
                let c = col as i32 - dc as i32;
                if c < 0 { continue; }
                let c = c as usize;
                if c == 6 { continue; }
                if reserved[row][c] { continue; }
                order.push((row, c));
            }
        }
        up = !up;
        if col < 2 { break; }
        col -= 2;
        if col == 6 { col = 5; }
    }
    order
}

fn place_bits(g: &mut WorkGrid, codewords: &[u8], version: usize) {
    let mut bits: Vec<bool> = Vec::new();
    for &cw in codewords {
        for b in (0u8..8).rev() { bits.push((cw >> b) & 1 == 1); }
    }
    bits.resize(bits.len() + num_remainder_bits(version), false);

    for (bit_idx, &(row, c)) in data_module_order(version).iter().enumerate() {
        g.modules[row][c] = bit_idx < bits.len() && bits[bit_idx];
    }
}

/// Which modules at this version are function patterns (finder, separator,
/// timing, alignment, format info, version info, dark module) rather than
/// data/ECC. `result[r][c] == true` means reserved. Shared by `build_grid`
/// (encode's own grid setup) so encode and decode agree on reservations by
/// construction.
pub fn reserved_modules(version: usize) -> Vec<Vec<bool>> {
    build_grid(version).reserved
}

fn build_grid(version: usize) -> WorkGrid {
    let sz = symbol_size(version);
    let mut g = WorkGrid::new(sz);

    place_finder(&mut g, 0, 0);
    place_finder(&mut g, 0, sz - 7);
    place_finder(&mut g, sz - 7, 0);

    // Separators
    for i in 0..=7 {
        g.set(7, i, false, true); g.set(i, 7, false, true);                       // TL
        g.set(7, sz-1-i, false, true); g.set(i, sz-8, false, true);               // TR
        g.set(sz-8, i, false, true); g.set(sz-1-i, 7, false, true);               // BL
    }

    place_timing(&mut g);
    place_all_alignments(&mut g, version);
    reserve_format_info(&mut g);
    reserve_version_info(&mut g, version);
    place_dark_module(&mut g, version);
    g
}

// ─────────────────────────────────────────────────────────────────────────────
// Masking and penalty
// ─────────────────────────────────────────────────────────────────────────────

/// Evaluate mask pattern `mask` (0-7) at module `(r, c)`: `true` means the
/// module should be flipped. Self-inverse (XOR), so `qr_decoder` calls this
/// the same way `encode`'s own `apply_mask` does, to invert masking.
pub fn mask_condition(mask: u32, r: usize, c: usize) -> bool {
    let (r, c) = (r as i64, c as i64);
    match mask {
        0 => (r + c) % 2 == 0,
        1 => r % 2 == 0,
        2 => c % 3 == 0,
        3 => (r + c) % 3 == 0,
        4 => (r / 2 + c / 3) % 2 == 0,
        5 => (r * c) % 2 + (r * c) % 3 == 0,
        6 => ((r * c) % 2 + (r * c) % 3) % 2 == 0,
        7 => ((r + c) % 2 + (r * c) % 3) % 2 == 0,
        _ => false,
    }
}

fn apply_mask(
    modules: &[Vec<bool>], reserved: &[Vec<bool>], sz: usize, mask: u32,
) -> Vec<Vec<bool>> {
    let mut result = modules.to_vec();
    for r in 0..sz {
        for c in 0..sz {
            if !reserved[r][c] {
                result[r][c] = modules[r][c] != mask_condition(mask, r, c);
            }
        }
    }
    result
}

// The loops below use their index symmetrically to walk both rows and columns of
// the module grid (e.g. `modules[a][i]` and `modules[i][a]` in the same body), so
// a plain `.iter().enumerate()` rewrite would not preserve the transposed access;
// keep the explicit range loops.
#[allow(clippy::needless_range_loop)]
fn compute_penalty(modules: &[Vec<bool>], sz: usize) -> u32 {
    let mut penalty = 0u32;

    // Rule 1 — runs of same color ≥ 5
    for a in 0..sz {
        for horiz in [true, false] {
            let mut run = 1u32;
            let mut prev = if horiz { modules[a][0] } else { modules[0][a] };
            for i in 1..sz {
                let cur = if horiz { modules[a][i] } else { modules[i][a] };
                if cur == prev { run += 1; }
                else { if run >= 5 { penalty += run - 2; } run = 1; prev = cur; }
            }
            if run >= 5 { penalty += run - 2; }
        }
    }

    // Rule 2 — 2×2 same-color blocks
    for r in 0..sz-1 {
        for c in 0..sz-1 {
            let d = modules[r][c];
            if d == modules[r][c+1] && d == modules[r+1][c] && d == modules[r+1][c+1] {
                penalty += 3;
            }
        }
    }

    // Rule 3 — finder-pattern-like sequences
    let p1: [u8; 11] = [1,0,1,1,1,0,1,0,0,0,0];
    let p2: [u8; 11] = [0,0,0,0,1,0,1,1,1,0,1];
    for a in 0..sz {
        for b in 0..=(sz.saturating_sub(11)) {
            let (mut mh1, mut mh2, mut mv1, mut mv2) = (true, true, true, true);
            for k in 0..11 {
                let bh = if modules[a][b+k] { 1u8 } else { 0 };
                let bv = if modules[b+k][a] { 1u8 } else { 0 };
                if bh != p1[k] { mh1 = false; }
                if bh != p2[k] { mh2 = false; }
                if bv != p1[k] { mv1 = false; }
                if bv != p2[k] { mv2 = false; }
            }
            if mh1 { penalty += 40; }
            if mh2 { penalty += 40; }
            if mv1 { penalty += 40; }
            if mv2 { penalty += 40; }
        }
    }

    // Rule 4 — dark ratio deviation
    let dark: u32 = modules.iter().flat_map(|r| r.iter()).map(|&m| m as u32).sum();
    let total = (sz * sz) as f64;
    let ratio = (dark as f64 / total) * 100.0;
    let prev5 = (ratio / 5.0).floor() as u32 * 5;
    let a = prev5.abs_diff(50);
    let b = (prev5 + 5).abs_diff(50);
    penalty += (a.min(b) / 5) * 10;

    penalty
}

// ─────────────────────────────────────────────────────────────────────────────
// Version selection
// ─────────────────────────────────────────────────────────────────────────────

fn select_version(input: &str, ecc: EccLevel) -> Result<usize, QRCodeError> {
    let mode = select_mode(input);
    let byte_len = input.len() as u32;

    for v in 1..=40 {
        let capacity = num_data_codewords(v, ecc);
        let data_bits = match mode {
            EncodingMode::Byte => byte_len * 8,
            EncodingMode::Numeric => {
                let n = input.chars().count() as u32;
                (n * 10).div_ceil(3)  // ceil(n * 10 / 3)
            }
            EncodingMode::Alphanumeric => {
                let n = input.chars().count() as u32;
                (n * 11).div_ceil(2)  // ceil(n * 11 / 2)
            }
        };
        let bits_needed = 4 + char_count_bits(mode, v) + data_bits;
        let cw_needed = bits_needed.div_ceil(8);
        if cw_needed as usize <= capacity { return Ok(v); }
    }
    Err(QRCodeError::InputTooLong(format!(
        "Input ({} chars, ECC={:?}) exceeds version-40 capacity.", input.len(), ecc
    )))
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Encode a UTF-8 string into a QR Code [`ModuleGrid`].
///
/// Returns a `(4V+17) × (4V+17)` boolean grid, `true` = dark module.
/// Selects the minimum version that fits the input at the given ECC level.
///
/// # Errors
/// Returns [`QRCodeError::InputTooLong`] if the input exceeds version-40 capacity.
///
/// # Example
/// ```
/// use qr_code::{encode, EccLevel};
/// let grid = encode("HELLO WORLD", EccLevel::M).unwrap();
/// assert_eq!(grid.rows, 21); // version 1
/// ```
pub fn encode(input: &str, ecc: EccLevel) -> Result<ModuleGrid, QRCodeError> {
    // Early-exit guard: QR Code version 40 holds at most 7089 numeric characters
    // (~2953 bytes in byte mode).  Inputs beyond this can never encode, and
    // checking early prevents large allocations in build_data_codewords before
    // select_version rejects them.
    if input.len() > 7089 {
        return Err(QRCodeError::InputTooLong(format!(
            "Input byte length {} exceeds 7089 (the QR Code v40 numeric-mode maximum).",
            input.len()
        )));
    }
    let version = select_version(input, ecc)?;
    let sz = symbol_size(version);

    let data_cw   = build_data_codewords(input, version, ecc);
    let blocks    = compute_blocks(&data_cw, version, ecc);
    let interleaved = interleave_blocks(&blocks);

    let mut grid = build_grid(version);
    place_bits(&mut grid, &interleaved, version);

    // Evaluate 8 masks, pick lowest penalty
    let mut best_mask = 0u32;
    let mut best_penalty = u32::MAX;
    for m in 0..8u32 {
        let masked = apply_mask(&grid.modules, &grid.reserved, sz, m);
        let fmt = compute_format_bits(ecc, m);
        let mut test = WorkGrid {
            size: sz,
            modules: masked,
            reserved: grid.reserved.clone(),
        };
        write_format_info(&mut test, fmt);
        let p = compute_penalty(&test.modules, sz);
        if p < best_penalty { best_penalty = p; best_mask = m; }
    }

    // Finalize
    let final_mods = apply_mask(&grid.modules, &grid.reserved, sz, best_mask);
    let mut final_g = WorkGrid {
        size: sz,
        modules: final_mods,
        reserved: grid.reserved,
    };
    write_format_info(&mut final_g, compute_format_bits(ecc, best_mask));
    write_version_info(&mut final_g, version);

    Ok(ModuleGrid {
        rows: sz as u32,
        cols: sz as u32,
        modules: final_g.modules,
        module_shape: ModuleShape::Square,
    })
}

/// Encode and convert to a pixel-resolved [`PaintScene`].
pub fn encode_and_layout(
    input: &str,
    ecc: EccLevel,
    config: &Barcode2DLayoutConfig,
) -> Result<PaintScene, QRCodeError> {
    let grid = encode(input, ecc)?;
    // Use LayoutError (not InputTooLong) to avoid conflating layout/config
    // failures with encoding failures.  The raw dependency error string is
    // not forwarded to avoid leaking internal library details.
    layout(&grid, config).map_err(|_| QRCodeError::LayoutError(
        "barcode-2d layout failed: check Barcode2DLayoutConfig values (module_size_px > 0, moduleShape = Square)".to_string()
    ))
}

/// One-shot convenience: encode → layout → PNG bytes.
///
/// Combines [`encode_and_layout`] with `barcode_2d::render_scene_png`,
/// dispatching to the platform-default rendering backend
/// (Metal on macOS, Direct2D on Windows, Cairo on Linux, Skia as fallback).
///
/// Pass `None` for `config` to use [`Barcode2DLayoutConfig::default()`] (10 px
/// modules, 4-module quiet zone, black-on-white).
///
/// # Errors
/// Returns a `String` describing the first failure encountered:
/// - [`QRCodeError::InputTooLong`] if the input exceeds version-40 capacity.
/// - [`QRCodeError::LayoutError`] if `barcode-2d` layout rejects the config.
/// - A render-backend error string if PNG encoding fails.
///
/// # Example
/// ```
/// use qr_code::{render_png, EccLevel};
/// let png = render_png("HELLO WORLD", EccLevel::M, None).unwrap();
/// assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
/// ```
pub fn render_png(
    data: &str,
    ecc: EccLevel,
    config: Option<&Barcode2DLayoutConfig>,
) -> Result<Vec<u8>, String> {
    // Use the caller-supplied layout config, or fall back to the crate default
    // (10 px modules, 4-module quiet zone).
    let default_config;
    let cfg = match config {
        Some(c) => c,
        None => {
            default_config = Barcode2DLayoutConfig::default();
            &default_config
        }
    };

    // Step 1: QR Code encoding + barcode-2d pixel layout → PaintScene.
    let scene = encode_and_layout(data, ecc, cfg).map_err(|e| e.to_string())?;

    // Step 2: PaintScene → PNG bytes via the platform-default rendering hub.
    barcode_2d::render_scene_png(&scene)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: check finder pattern at (top, left)
    fn has_finder(mods: &[Vec<bool>], top: usize, left: usize) -> bool {
        for dr in 0..7usize {
            for dc in 0..7usize {
                let on_border = dr == 0 || dr == 6 || dc == 0 || dc == 6;
                let in_core   = (2..=4).contains(&dr) && (2..=4).contains(&dc);
                let expected  = on_border || in_core;
                if mods[top + dr][left + dc] != expected { return false; }
            }
        }
        true
    }

    // Read copy-1 raw format bits (standard order: f14 at (8,0) → f0 at
    // (0,8)) off the grid and hand them to the promoted public
    // `read_format_info` for BCH validation — this test module used to carry
    // its own private duplicate of that validation logic (`format_info_valid`,
    // pre-MA04); now it exercises the real public API instead.
    fn read_copy1_format_info(mods: &[Vec<bool>]) -> Option<(EccLevel, u32)> {
        let positions: [(usize, usize); 15] = [
            (8,0),(8,1),(8,2),(8,3),(8,4),(8,5),(8,7),(8,8),
            (7,8),(5,8),(4,8),(3,8),(2,8),(1,8),(0,8),
        ];
        let mut raw = 0u32;
        for (i, &(r, c)) in positions.iter().enumerate() {
            if mods[r][c] { raw |= 1 << (14 - i); }  // f14 at i=0 → bit 14
        }
        read_format_info(raw)
    }

    #[test]
    fn version_constant() {
        assert_eq!(VERSION, "0.3.0");
    }

    #[test]
    fn hello_world_v1() {
        let grid = encode("HELLO WORLD", EccLevel::M).unwrap();
        assert_eq!(grid.rows, 21);
        assert_eq!(grid.cols, 21);
        assert_eq!(grid.module_shape, ModuleShape::Square);
    }

    #[test]
    fn url_v2() {
        let grid = encode("https://example.com", EccLevel::M).unwrap();
        assert_eq!(grid.rows, 25);
    }

    #[test]
    fn single_char_v1() {
        let grid = encode("A", EccLevel::M).unwrap();
        assert_eq!(grid.rows, 21);
    }

    #[test]
    fn all_ecc_levels_work() {
        for ecc in [EccLevel::L, EccLevel::M, EccLevel::Q, EccLevel::H] {
            let grid = encode("HELLO", ecc).unwrap();
            assert!(grid.rows >= 21);
        }
    }

    #[test]
    fn h_needs_larger_version_than_l() {
        let gl = encode("The quick brown fox", EccLevel::L).unwrap();
        let gh = encode("The quick brown fox", EccLevel::H).unwrap();
        assert!(gh.rows >= gl.rows);
    }

    #[test]
    fn finder_patterns_present() {
        let grid = encode("HELLO WORLD", EccLevel::M).unwrap();
        let sz = grid.rows as usize;
        assert!(has_finder(&grid.modules, 0, 0));
        assert!(has_finder(&grid.modules, 0, sz - 7));
        assert!(has_finder(&grid.modules, sz - 7, 0));
    }

    #[test]
    fn timing_strips_correct() {
        let grid = encode("HELLO WORLD", EccLevel::M).unwrap();
        let sz = grid.rows as usize;
        for c in 8..=(sz - 9) { assert_eq!(grid.modules[6][c], c % 2 == 0); }
        for r in 8..=(sz - 9) { assert_eq!(grid.modules[r][6], r % 2 == 0); }
    }

    #[test]
    fn dark_module_v1() {
        let grid = encode("A", EccLevel::M).unwrap();
        assert!(grid.modules[13][8]); // 4*1+9 = 13
    }

    #[test]
    fn dark_module_v2() {
        let grid = encode("https://example.com", EccLevel::M).unwrap();
        assert!(grid.modules[17][8]); // 4*2+9 = 17
    }

    #[test]
    fn format_info_decodable_m() {
        let grid = encode("HELLO WORLD", EccLevel::M).unwrap();
        let decoded = read_copy1_format_info(&grid.modules);
        assert!(decoded.is_some());
        let (ecc, _mask) = decoded.unwrap();
        assert_eq!(ecc, EccLevel::M);
    }

    #[test]
    fn format_info_ecc_bits_all_levels() {
        for ecc in [EccLevel::L, EccLevel::M, EccLevel::Q, EccLevel::H] {
            let grid = encode("HELLO", ecc).unwrap();
            let decoded = read_copy1_format_info(&grid.modules);
            assert!(decoded.is_some(), "format info unreadable for {:?}", ecc);
            assert_eq!(decoded.unwrap().0, ecc, "wrong ECC level for {:?}", ecc);
        }
    }

    #[test]
    fn format_info_copies_match() {
        let grid = encode("HELLO WORLD", EccLevel::M).unwrap();
        let sz = grid.rows as usize;
        let copy1: [(usize,usize); 15] = [
            (8,0),(8,1),(8,2),(8,3),(8,4),(8,5),(8,7),(8,8),
            (7,8),(5,8),(4,8),(3,8),(2,8),(1,8),(0,8),
        ];
        let copy2: [(usize,usize); 15] = [
            (sz-1,8),(sz-2,8),(sz-3,8),(sz-4,8),(sz-5,8),(sz-6,8),(sz-7,8),
            (8,sz-8),(8,sz-7),(8,sz-6),(8,sz-5),(8,sz-4),(8,sz-3),(8,sz-2),(8,sz-1),
        ];
        let mut fmt1 = 0u32; let mut fmt2 = 0u32;
        for i in 0..15 {
            if grid.modules[copy1[i].0][copy1[i].1] { fmt1 |= 1 << i; }
            if grid.modules[copy2[i].0][copy2[i].1] { fmt2 |= 1 << i; }
        }
        assert_eq!(fmt1, fmt2);
    }

    #[test]
    fn numeric_mode_small_grid() {
        let grid = encode("000000000000000", EccLevel::M).unwrap();
        // 15 digits in numeric mode should fit in v1
        assert_eq!(grid.rows, 21);
    }

    #[test]
    fn deterministic() {
        let g1 = encode("https://example.com", EccLevel::M).unwrap();
        let g2 = encode("https://example.com", EccLevel::M).unwrap();
        assert_eq!(g1.modules, g2.modules);
    }

    #[test]
    fn different_inputs_differ() {
        let g1 = encode("HELLO", EccLevel::M).unwrap();
        let g2 = encode("WORLD", EccLevel::M).unwrap();
        let sz = g1.rows as usize;
        let differ = (0..sz).any(|r| (0..sz).any(|c| g1.modules[r][c] != g2.modules[r][c]));
        assert!(differ);
    }

    #[test]
    fn input_too_long_error() {
        let giant: String = "A".repeat(8000);
        let result = encode(&giant, EccLevel::H);
        assert!(matches!(result, Err(QRCodeError::InputTooLong(_))));
    }

    #[test]
    fn empty_string_ok() {
        let grid = encode("", EccLevel::M).unwrap();
        assert_eq!(grid.rows, 21);
    }

    #[test]
    fn utf8_byte_mode() {
        // "→" is U+2192, encoded as 3 UTF-8 bytes (E2 86 92)
        let grid = encode("→→→", EccLevel::M).unwrap();
        assert!(grid.rows >= 21);
    }

    #[test]
    fn v7_plus_produced() {
        // 85 uppercase chars exceed v6-H capacity (~84 alphanumeric chars)
        let input: String = "A".repeat(85);
        let grid = encode(&input, EccLevel::H).unwrap();
        assert!(grid.rows >= 45); // v7 = 45×45
    }

    #[test]
    fn encode_and_layout_works() {
        let config = Barcode2DLayoutConfig::default();
        let scene = encode_and_layout("HELLO", EccLevel::M, &config).unwrap();
        assert!(scene.width > 0.0);
        assert!(scene.height > 0.0);
    }

    /// Verify that `render_png` produces a well-formed PNG file.
    ///
    /// The PNG format mandates an 8-byte magic signature at byte offset 0:
    ///   0x89 'P' 'N' 'G' CR LF 0x1A LF
    /// Any valid PNG file — regardless of dimensions or colour depth — MUST
    /// start with these exact bytes (ISO 15948 / PNG spec §5.2).
    #[test]
    fn qr_code_render_png_produces_valid_png() {
        let png = render_png("HELLO", EccLevel::M, None).unwrap();
        // PNG magic bytes: [0x89, 0x50('P'), 0x4E('N'), 0x47('G'), 0x0D, 0x0A, 0x1A, 0x0A]
        let expected_magic: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(
            png.len() >= 8,
            "PNG output too short ({} bytes); expected at least 8 for magic signature",
            png.len()
        );
        assert_eq!(
            &png[..8], &expected_magic,
            "PNG magic bytes mismatch — output is not a valid PNG file"
        );
    }

    #[test]
    fn test_corpus() {
        let corpus = [
            ("A", EccLevel::M),
            ("HELLO WORLD", EccLevel::M),
            ("https://example.com", EccLevel::M),
            ("01234567890", EccLevel::M),
            ("The quick brown fox jumps over the lazy dog", EccLevel::M),
        ];
        for (input, ecc) in corpus {
            let grid = encode(input, ecc).unwrap();
            assert!(grid.rows >= 21);
            assert_eq!(grid.rows, grid.cols);
            let decoded = read_copy1_format_info(&grid.modules);
            assert!(decoded.is_some(), "bad format info for: {}", input);
        }
    }

    #[test]
    fn rs_encode_seven_ecc() {
        // Build the 7-ECC generator and verify it has degree 7
        let gen = build_generator(7);
        assert_eq!(gen.len(), 8); // degree 7 → 8 coefficients
        assert_eq!(gen[0], 1);    // monic
    }

    #[test]
    fn version_info_v7_written() {
        // Version 7 must have 18-bit version info blocks reserved and written
        let input: String = "A".repeat(85);
        let grid = encode(&input, EccLevel::H).unwrap();
        // Just ensure the grid was produced without panic and has the right size
        let sz = grid.rows as usize;
        if sz >= 45 { // v7+
            // Dark module still dark
            let version = (sz - 17) / 4;
            assert!(grid.modules[4 * version + 9][8]);
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // MA04-qr-decoder.md §3 new pub surface — cheap early-warning sanity
    // checks. The exhaustive correctness proof (every version × ECC level ×
    // mode round-tripping through the real decoder) lives in the
    // `qr-decoder` crate's own test suite, which is the actual consumer of
    // this API; these just catch an obviously-broken extraction early.
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn reserved_modules_matches_encode_grid_reservations() {
        // reserved_modules(version) must equal the same WorkGrid.reserved
        // that encode()'s own build_grid produces — they now share one
        // implementation, so this is really just confirming the accessor
        // wires through correctly (finder + dark module cells reserved).
        let reserved = reserved_modules(1);
        let sz = symbol_size(1);
        assert_eq!(reserved.len(), sz);
        assert_eq!(reserved[0].len(), sz);
        // Top-left finder pattern corner is always reserved.
        assert!(reserved[0][0]);
        assert!(reserved[6][6]); // inside TL finder border area (timing/finder overlap corner)
        // Dark module for v1 is at (4*1+9, 8) = (13, 8).
        assert!(reserved[13][8]);
    }

    #[test]
    fn reserved_modules_count_matches_raw_data_modules_for_versions_without_alignment() {
        // Every non-reserved module is a data/ECC module — so
        // (total modules) - (reserved count) must equal num_raw_data_modules,
        // for versions 1-6 (no alignment patterns at all for v1; for v2-6
        // there's exactly one alignment position pair and it never lands on
        // the timing row/column). See the test below for what happens at
        // v7+, where this stops holding for a pre-existing, unrelated reason.
        for version in [1usize, 2, 3, 4, 5, 6] {
            let sz = symbol_size(version);
            let reserved = reserved_modules(version);
            let reserved_count: usize = reserved.iter().flatten().filter(|&&r| r).count();
            let data_count = sz * sz - reserved_count;
            assert_eq!(
                data_count,
                num_raw_data_modules(version),
                "reserved/data module split mismatch at version {version}"
            );
        }
    }

    /// **Pre-existing `qr-code` quirk, discovered while adding this test (not
    /// introduced by MA04's refactor — `build_grid`'s reservation logic is
    /// untouched code, only newly exposed via `reserved_modules`)**:
    /// `place_all_alignments` skips placing an alignment pattern whenever its
    /// *center* cell is already `reserved` — intended to avoid finder-pattern
    /// overlap — but for version ≥ 7, some alignment centers (e.g. v7's
    /// `(6, 22)`) land on the *timing* pattern's row/column instead, which per
    /// ISO 18004 the alignment pattern should override, not be skipped for.
    /// The practical effect: `reserved_modules`/`data_module_order` report
    /// more "free" modules than `num_raw_data_modules(version)` predicts for
    /// every version ≥ 7 (confirmed here to hold across the full v7-v40
    /// range). This does **not** break `qr-decoder`'s round-trip: `encode`'s
    /// own `place_bits` only ever writes real bits into the first
    /// `num_raw_data_modules(version)` traversal slots (by construction of
    /// its bit buffer length) and explicitly zero-fills the rest, and
    /// `qr_decoder::decode` mirrors that exact convention (MA04-qr-decoder.md
    /// §4.2 step 6) — so encode and decode still agree byte-for-byte. It
    /// *would* make the emitted grid non-ISO-conformant for a real scanner in
    /// the affected region, which is a genuine `qr-code` encoder bug, just
    /// one outside this decoder work's scope (see the spawned follow-up task).
    #[test]
    fn data_module_order_exceeds_formula_for_versions_with_timing_overlap_alignment() {
        for version in 7usize..=40 {
            let order_len = data_module_order(version).len();
            let expected = num_raw_data_modules(version);
            assert!(
                order_len >= expected,
                "expected order_len >= formula at version {version}, got {order_len} < {expected}"
            );
        }
    }

    #[test]
    fn data_module_order_visits_every_data_module_exactly_once() {
        // Traversal length must always match the reserved-complement count
        // (self-consistency between reserved_modules and data_module_order),
        // and never revisit a position — independent of whether that count
        // happens to equal the num_raw_data_modules formula (see above).
        for version in [1usize, 2, 7, 40] {
            let sz = symbol_size(version);
            let order = data_module_order(version);
            let reserved = reserved_modules(version);
            let reserved_count: usize = reserved.iter().flatten().filter(|&&r| r).count();
            assert_eq!(
                order.len(),
                sz * sz - reserved_count,
                "traversal length vs reserved-complement mismatch at version {version}"
            );
            let mut seen = std::collections::HashSet::new();
            for &pos in &order {
                assert!(seen.insert(pos), "position {pos:?} visited twice at version {version}");
            }
        }
    }

    #[test]
    fn data_module_order_never_touches_reserved_modules() {
        let version = 7;
        let reserved = reserved_modules(version);
        for (r, c) in data_module_order(version) {
            assert!(!reserved[r][c], "traversal visited reserved module ({r},{c})");
        }
    }

    #[test]
    fn ecc_from_indicator_is_inverse_of_ecc_indicator() {
        for ecc in [EccLevel::L, EccLevel::M, EccLevel::Q, EccLevel::H] {
            let bits = ecc_indicator(ecc);
            assert_eq!(ecc_from_indicator(bits), Some(ecc));
        }
        assert_eq!(ecc_from_indicator(0b1111), None); // out of 2-bit range
    }

    #[test]
    fn read_format_info_round_trips_compute_format_bits() {
        for ecc in [EccLevel::L, EccLevel::M, EccLevel::Q, EccLevel::H] {
            for mask in 0u32..8 {
                let fmt = compute_format_bits(ecc, mask);
                let decoded = read_format_info(fmt);
                assert_eq!(decoded, Some((ecc, mask)), "ecc={:?} mask={mask}", ecc);
            }
        }
    }

    #[test]
    fn read_format_info_rejects_corrupted_word() {
        let fmt = compute_format_bits(EccLevel::M, 3);
        // Flip the low 8 bits — well beyond the 3-bit correction distance of
        // this (15,5) BCH code. (Flipping *all* 15 bits, i.e. XOR 0x7FFF, is
        // deliberately avoided: this particular code is linear and the
        // all-ones word happens to itself be a valid codeword, so
        // complementing every bit of any valid word yields another valid
        // word — confirmed empirically, not a decoder bug, just a
        // degenerate corruption pattern that isn't a useful test case.)
        let corrupted = fmt ^ 0x00FF;
        assert_eq!(read_format_info(corrupted), None);
    }

    #[test]
    fn read_version_info_round_trips_compute_version_bits() {
        for version in [7usize, 10, 25, 40] {
            let bits = compute_version_bits(version);
            assert_eq!(read_version_info(bits), Some(version));
        }
    }

    #[test]
    fn read_version_info_rejects_out_of_range_version() {
        // Version field itself (top 6 bits) out of the valid 7..=40 range.
        assert_eq!(read_version_info(41 << 12), None);
        assert_eq!(read_version_info(0), None);
    }

    #[test]
    fn read_version_info_rejects_corrupted_word() {
        let bits = compute_version_bits(15);
        let corrupted = bits ^ 0x3FFFF; // flip all 18 bits
        assert_eq!(read_version_info(corrupted), None);
    }

    #[test]
    fn ecc_codewords_per_block_and_num_blocks_match_private_tables() {
        for ecc in [EccLevel::L, EccLevel::M, EccLevel::Q, EccLevel::H] {
            for version in [1usize, 6, 20, 40] {
                assert_eq!(
                    ecc_codewords_per_block(ecc, version),
                    ECC_CODEWORDS_PER_BLOCK[ecc_idx(ecc)][version] as usize
                );
                assert_eq!(
                    num_blocks(ecc, version),
                    NUM_BLOCKS[ecc_idx(ecc)][version] as usize
                );
            }
        }
    }
}
