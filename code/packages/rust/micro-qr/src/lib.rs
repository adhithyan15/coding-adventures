//! # micro-qr
//!
//! Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.
//!
//! Micro QR Code is the compact variant of QR Code, designed for applications
//! where even the smallest standard QR (21×21 at version 1) is too large.
//! Common use cases include surface-mount component labels, circuit board
//! markings, and miniature industrial tags.
//!
//! ## Symbol sizes
//!
//! ```text
//! M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
//! formula: size = 2 × version_number + 9
//! ```
//!
//! ## Key differences from regular QR Code
//!
//! - **Single finder pattern** at top-left only (one 7×7 square, not three).
//! - **Timing at row 0 / col 0** (not row 6 / col 6).
//! - **Only 4 mask patterns** (not 8).
//! - **Format XOR mask 0x4445** (not 0x5412).
//! - **Single copy of format info** (not two).
//! - **2-module quiet zone** (not 4).
//! - **Narrower mode indicators** (0–3 bits instead of 4).
//! - **Single block** (no interleaving).
//!
//! ## Encoding pipeline
//!
//! ```text
//! input string
//!   → auto-select smallest symbol (M1..M4) and mode
//!   → build bit stream (mode indicator + char count + data + terminator + padding)
//!   → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
//!   → initialize grid (finder, L-shaped separator, timing at row0/col0, format reserved)
//!   → zigzag data placement (two-column snake from bottom-right)
//!   → evaluate 4 mask patterns, pick lowest penalty
//!   → write format information (15 bits, single copy, XOR 0x4445)
//!   → ModuleGrid
//! ```

pub const VERSION: &str = "0.1.0";

use barcode_2d::{layout, Barcode2DLayoutConfig, ModuleGrid, ModuleShape};
use gf256::multiply as gf_mul;

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

/// Micro QR symbol designator.
///
/// Each step up adds two rows/columns (size = 2×version_number+9):
/// M1=11×11, M2=13×13, M3=15×15, M4=17×17.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MicroQRVersion {
    M1,
    M2,
    M3,
    M4,
}

/// Error correction level for Micro QR.
///
/// | Level     | Available in | Recovery |
/// |-----------|-------------|---------|
/// | Detection | M1 only     | detects errors only |
/// | L         | M2, M3, M4  | ~7% of codewords |
/// | M         | M2, M3, M4  | ~15% of codewords |
/// | Q         | M4 only     | ~25% of codewords |
///
/// Level H is not available in any Micro QR symbol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MicroQREccLevel {
    Detection,
    L,
    M,
    Q,
}

/// Error types for the Micro QR encoder.
#[derive(Debug)]
pub enum MicroQRError {
    /// Input is too long to fit in any M1–M4 symbol at any ECC level.
    InputTooLong(String),
    /// The requested encoding mode is not available for the chosen symbol.
    UnsupportedMode(String),
    /// The requested ECC level is not available for the chosen symbol.
    ECCNotAvailable(String),
    /// A character cannot be encoded in the selected mode.
    InvalidCharacter(String),
    /// Layout/rendering error.
    LayoutError(String),
}

impl std::fmt::Display for MicroQRError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MicroQRError::InputTooLong(m)    => write!(f, "InputTooLong: {m}"),
            MicroQRError::UnsupportedMode(m) => write!(f, "UnsupportedMode: {m}"),
            MicroQRError::ECCNotAvailable(m) => write!(f, "ECCNotAvailable: {m}"),
            MicroQRError::InvalidCharacter(m)=> write!(f, "InvalidCharacter: {m}"),
            MicroQRError::LayoutError(m)     => write!(f, "LayoutError: {m}"),
        }
    }
}

impl std::error::Error for MicroQRError {}

// ─────────────────────────────────────────────────────────────────────────────
// Symbol configurations
// ─────────────────────────────────────────────────────────────────────────────

/// All the compile-time constants for one (version, ECC) combination.
///
/// There are exactly 8 valid combinations:
/// M1/Detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q.
struct SymbolConfig {
    version: MicroQRVersion,
    ecc: MicroQREccLevel,
    /// 3-bit symbol indicator placed in format information (0..7).
    symbol_indicator: u8,
    /// Symbol side length in modules (11, 13, 15, or 17).
    size: usize,
    /// Number of data codewords (full 8-bit bytes, except M1 which uses 2.5 bytes).
    data_cw: usize,
    /// Number of ECC codewords.
    ecc_cw: usize,
    /// Maximum numeric characters. 0 = not supported.
    numeric_cap: usize,
    /// Maximum alphanumeric characters. 0 = not supported.
    alpha_cap: usize,
    /// Maximum byte characters. 0 = not supported.
    byte_cap: usize,
    /// Terminator bit count (3/5/7/9).
    terminator_bits: usize,
    /// Mode indicator bit width (0=M1, 1=M2, 2=M3, 3=M4).
    mode_indicator_bits: usize,
    /// Character count field width for numeric mode.
    cc_bits_numeric: usize,
    /// Character count field width for alphanumeric mode.
    cc_bits_alpha: usize,
    /// Character count field width for byte mode.
    cc_bits_byte: usize,
    /// True for M1 only: last data "codeword" is 4 bits, total = 20 bits.
    m1_half_cw: bool,
}

/// All 8 valid Micro QR symbol configurations from ISO 18004:2015 Annex E.
///
/// Data capacities, codeword counts, and field widths are compile-time constants.
static SYMBOL_CONFIGS: &[SymbolConfig] = &[
    // M1 / Detection
    SymbolConfig {
        version: MicroQRVersion::M1, ecc: MicroQREccLevel::Detection,
        symbol_indicator: 0, size: 11,
        data_cw: 3, ecc_cw: 2,
        numeric_cap: 5, alpha_cap: 0, byte_cap: 0,
        terminator_bits: 3, mode_indicator_bits: 0,
        cc_bits_numeric: 3, cc_bits_alpha: 0, cc_bits_byte: 0,
        m1_half_cw: true,
    },
    // M2 / L
    SymbolConfig {
        version: MicroQRVersion::M2, ecc: MicroQREccLevel::L,
        symbol_indicator: 1, size: 13,
        data_cw: 5, ecc_cw: 5,
        numeric_cap: 10, alpha_cap: 6, byte_cap: 4,
        terminator_bits: 5, mode_indicator_bits: 1,
        cc_bits_numeric: 4, cc_bits_alpha: 3, cc_bits_byte: 4,
        m1_half_cw: false,
    },
    // M2 / M
    SymbolConfig {
        version: MicroQRVersion::M2, ecc: MicroQREccLevel::M,
        symbol_indicator: 2, size: 13,
        data_cw: 4, ecc_cw: 6,
        numeric_cap: 8, alpha_cap: 5, byte_cap: 3,
        terminator_bits: 5, mode_indicator_bits: 1,
        cc_bits_numeric: 4, cc_bits_alpha: 3, cc_bits_byte: 4,
        m1_half_cw: false,
    },
    // M3 / L
    SymbolConfig {
        version: MicroQRVersion::M3, ecc: MicroQREccLevel::L,
        symbol_indicator: 3, size: 15,
        data_cw: 11, ecc_cw: 6,
        numeric_cap: 23, alpha_cap: 14, byte_cap: 9,
        terminator_bits: 7, mode_indicator_bits: 2,
        cc_bits_numeric: 5, cc_bits_alpha: 4, cc_bits_byte: 4,
        m1_half_cw: false,
    },
    // M3 / M
    SymbolConfig {
        version: MicroQRVersion::M3, ecc: MicroQREccLevel::M,
        symbol_indicator: 4, size: 15,
        data_cw: 9, ecc_cw: 8,
        numeric_cap: 18, alpha_cap: 11, byte_cap: 7,
        terminator_bits: 7, mode_indicator_bits: 2,
        cc_bits_numeric: 5, cc_bits_alpha: 4, cc_bits_byte: 4,
        m1_half_cw: false,
    },
    // M4 / L
    SymbolConfig {
        version: MicroQRVersion::M4, ecc: MicroQREccLevel::L,
        symbol_indicator: 5, size: 17,
        data_cw: 16, ecc_cw: 8,
        numeric_cap: 35, alpha_cap: 21, byte_cap: 15,
        terminator_bits: 9, mode_indicator_bits: 3,
        cc_bits_numeric: 6, cc_bits_alpha: 5, cc_bits_byte: 5,
        m1_half_cw: false,
    },
    // M4 / M
    SymbolConfig {
        version: MicroQRVersion::M4, ecc: MicroQREccLevel::M,
        symbol_indicator: 6, size: 17,
        data_cw: 14, ecc_cw: 10,
        numeric_cap: 30, alpha_cap: 18, byte_cap: 13,
        terminator_bits: 9, mode_indicator_bits: 3,
        cc_bits_numeric: 6, cc_bits_alpha: 5, cc_bits_byte: 5,
        m1_half_cw: false,
    },
    // M4 / Q
    SymbolConfig {
        version: MicroQRVersion::M4, ecc: MicroQREccLevel::Q,
        symbol_indicator: 7, size: 17,
        data_cw: 10, ecc_cw: 14,
        numeric_cap: 21, alpha_cap: 13, byte_cap: 9,
        terminator_bits: 9, mode_indicator_bits: 3,
        cc_bits_numeric: 6, cc_bits_alpha: 5, cc_bits_byte: 5,
        m1_half_cw: false,
    },
];

// ─────────────────────────────────────────────────────────────────────────────
// RS generator polynomials (compile-time constants)
// ─────────────────────────────────────────────────────────────────────────────

/// Monic RS generator polynomials for GF(256)/0x11D with b=0 convention.
///
/// g(x) = (x+α⁰)(x+α¹)···(x+α^{n-1}).
/// Array length is n+1 (leading monic term included).
///
/// Only the counts {2, 5, 6, 8, 10, 14} are needed for Micro QR.
fn get_generator(ecc_count: usize) -> &'static [u8] {
    match ecc_count {
        2  => &[0x01, 0x03, 0x02],
        5  => &[0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68],
        6  => &[0x01, 0x3f, 0x4e, 0x17, 0x9b, 0x05, 0x37],
        8  => &[0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, 0xa2, 0xa3],
        10 => &[0x01, 0xf6, 0x75, 0xa8, 0xd0, 0xc3, 0xe3, 0x36, 0xe1, 0x3c, 0x45],
        14 => &[0x01, 0xf6, 0x9a, 0x60, 0x97, 0x8a, 0xf1, 0xa4, 0xa1, 0x8e, 0xfc, 0x7a, 0x52, 0xad, 0xac],
        _  => panic!("micro-qr: no generator for ecc_count={ecc_count}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pre-computed format information table
// ─────────────────────────────────────────────────────────────────────────────

/// All 32 pre-computed format words (after XOR with 0x4445).
///
/// Indexed as `FORMAT_TABLE[symbol_indicator][mask_pattern]`.
///
/// The 15-bit format word structure:
///   [symbol_indicator (3b)] [mask_pattern (2b)] [BCH-10 remainder]
/// XOR-masked with 0x4445 (Micro QR specific, not 0x5412 like regular QR).
static FORMAT_TABLE: [[u16; 4]; 8] = [
    [0x4445, 0x4172, 0x4E2B, 0x4B1C],  // M1
    [0x5528, 0x501F, 0x5F46, 0x5A71],  // M2-L
    [0x6649, 0x637E, 0x6C27, 0x6910],  // M2-M
    [0x7764, 0x7253, 0x7D0A, 0x783D],  // M3-L
    [0x06DE, 0x03E9, 0x0CB0, 0x0987],  // M3-M
    [0x17F3, 0x12C4, 0x1D9D, 0x18AA],  // M4-L
    [0x24B2, 0x2185, 0x2EDC, 0x2BEB],  // M4-M
    [0x359F, 0x30A8, 0x3FF1, 0x3AC6],  // M4-Q
];

// ─────────────────────────────────────────────────────────────────────────────
// Encoding mode
// ─────────────────────────────────────────────────────────────────────────────

/// 45-character alphanumeric set shared with regular QR Code.
const ALPHANUM_CHARS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

/// Encoding mode determines how input characters are packed into bits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EncodingMode {
    Numeric,
    Alphanumeric,
    Byte,
}

/// Select the most compact mode supported by the given config.
///
/// Selection priority: numeric > alphanumeric > byte.
/// If no supported mode can encode the input, returns an error.
fn select_mode(input: &str, cfg: &SymbolConfig) -> Result<EncodingMode, MicroQRError> {
    // Numeric: all digits 0-9
    let is_numeric = input.is_empty() || input.chars().all(|c| c.is_ascii_digit());
    if is_numeric && cfg.cc_bits_numeric > 0 {
        return Ok(EncodingMode::Numeric);
    }
    // Alphanumeric: all chars in the 45-char set
    let is_alpha = input.chars().all(|c| ALPHANUM_CHARS.contains(c));
    if is_alpha && cfg.alpha_cap > 0 {
        return Ok(EncodingMode::Alphanumeric);
    }
    // Byte: raw bytes (UTF-8 is valid here)
    if cfg.byte_cap > 0 {
        return Ok(EncodingMode::Byte);
    }
    Err(MicroQRError::UnsupportedMode(format!(
        "Input cannot be encoded in any mode supported by {:?}-{:?}",
        cfg.version, cfg.ecc
    )))
}

fn mode_indicator_value(mode: EncodingMode, cfg: &SymbolConfig) -> u32 {
    match cfg.mode_indicator_bits {
        0 => 0, // M1: no indicator
        1 => match mode { EncodingMode::Numeric => 0, _ => 1 },
        2 => match mode {
            EncodingMode::Numeric     => 0b00,
            EncodingMode::Alphanumeric=> 0b01,
            EncodingMode::Byte        => 0b10,
        },
        3 => match mode {
            EncodingMode::Numeric     => 0b000,
            EncodingMode::Alphanumeric=> 0b001,
            EncodingMode::Byte        => 0b010,
        },
        _ => 0,
    }
}

fn char_count_bits(mode: EncodingMode, cfg: &SymbolConfig) -> u32 {
    match mode {
        EncodingMode::Numeric     => cfg.cc_bits_numeric as u32,
        EncodingMode::Alphanumeric=> cfg.cc_bits_alpha as u32,
        EncodingMode::Byte        => cfg.cc_bits_byte as u32,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Bit-writer
// ─────────────────────────────────────────────────────────────────────────────

/// Accumulates bits MSB-first, flushes to bytes.
///
/// Each call to `write(value, count)` appends the `count` least-significant
/// bits of `value` to the stream, MSB first.  This matches the QR/Micro-QR
/// convention of big-endian bit ordering within each codeword.
struct BitWriter {
    bits: Vec<u8>, // 0 or 1
}

impl BitWriter {
    fn new() -> Self {
        Self { bits: Vec::new() }
    }

    fn write(&mut self, value: u32, count: u32) {
        for i in (0..count).rev() {
            self.bits.push(((value >> i) & 1) as u8);
        }
    }

    fn bit_len(&self) -> usize {
        self.bits.len()
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        let mut i = 0;
        while i < self.bits.len() {
            let mut byte = 0u8;
            for j in 0..8_usize {
                byte = (byte << 1) | self.bits.get(i + j).copied().unwrap_or(0);
            }
            result.push(byte);
            i += 8;
        }
        result
    }

    fn to_bit_vec(&self) -> Vec<u8> {
        self.bits.clone()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Data encoding helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Encode a numeric string: groups of 3 → 10 bits, pair → 7 bits, single → 4 bits.
fn encode_numeric(input: &str, w: &mut BitWriter) {
    let digits: Vec<u32> = input.chars()
        .map(|c| c as u32 - b'0' as u32)
        .collect();
    let mut i = 0;
    while i + 2 < digits.len() {
        w.write(digits[i] * 100 + digits[i + 1] * 10 + digits[i + 2], 10);
        i += 3;
    }
    if i + 1 < digits.len() {
        w.write(digits[i] * 10 + digits[i + 1], 7);
        i += 2;
    }
    if i < digits.len() {
        w.write(digits[i], 4);
    }
}

/// Encode an alphanumeric string: pairs → 11 bits, single → 6 bits.
fn encode_alphanumeric(input: &str, w: &mut BitWriter) {
    let indices: Vec<u32> = input.chars()
        .map(|c| ALPHANUM_CHARS.find(c).unwrap_or(0) as u32)
        .collect();
    let mut i = 0;
    while i + 1 < indices.len() {
        w.write(indices[i] * 45 + indices[i + 1], 11);
        i += 2;
    }
    if i < indices.len() {
        w.write(indices[i], 6);
    }
}

/// Encode byte mode: each UTF-8 byte → 8 bits.
fn encode_byte_mode(input: &str, w: &mut BitWriter) {
    for b in input.as_bytes() {
        w.write(*b as u32, 8);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Reed-Solomon encoder
// ─────────────────────────────────────────────────────────────────────────────

/// Compute ECC bytes using LFSR polynomial division over GF(256)/0x11D.
///
/// Returns the remainder of D(x)·x^n mod G(x).
/// Uses the b=0 convention (first root is α^0 = 1).
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
// Data codeword assembly
// ─────────────────────────────────────────────────────────────────────────────

/// Build the complete data codeword byte sequence.
///
/// For all symbols except M1:
///   [mode indicator] [char count] [data bits] [terminator] [byte-align] [0xEC/0x11 fill]
///   → exactly cfg.data_cw bytes.
///
/// For M1 (m1_half_cw = true):
///   Total capacity = 20 bits = 2 full bytes + 4-bit nibble.
///   The RS encoder receives 3 bytes where byte[2] has data in the upper 4 bits.
fn build_data_codewords(
    input: &str,
    cfg: &SymbolConfig,
    mode: EncodingMode,
) -> Vec<u8> {
    // Total usable data bit capacity
    let total_bits = if cfg.m1_half_cw {
        cfg.data_cw * 8 - 4  // M1: 3×8 − 4 = 20 bits
    } else {
        cfg.data_cw * 8
    };

    let mut w = BitWriter::new();

    // Mode indicator (0/1/2/3 bits depending on symbol)
    if cfg.mode_indicator_bits > 0 {
        w.write(mode_indicator_value(mode, cfg), cfg.mode_indicator_bits as u32);
    }

    // Character count
    let char_count = if mode == EncodingMode::Byte {
        input.len() as u32
    } else {
        input.chars().count() as u32
    };
    w.write(char_count, char_count_bits(mode, cfg));

    // Encoded data
    match mode {
        EncodingMode::Numeric     => encode_numeric(input, &mut w),
        EncodingMode::Alphanumeric=> encode_alphanumeric(input, &mut w),
        EncodingMode::Byte        => encode_byte_mode(input, &mut w),
    }

    // Terminator: up to terminator_bits zero bits (truncated if capacity full)
    let remaining = total_bits.saturating_sub(w.bit_len());
    if remaining > 0 {
        w.write(0, cfg.terminator_bits.min(remaining) as u32);
    }

    if cfg.m1_half_cw {
        // M1: pack into exactly 20 bits → 3 bytes (last byte: upper nibble = data, lower = 0)
        let mut bits = w.to_bit_vec();
        bits.resize(20, 0);
        let b0 = (bits[0]  << 7) | (bits[1]  << 6) | (bits[2]  << 5) | (bits[3]  << 4)
               | (bits[4]  << 3) | (bits[5]  << 2) | (bits[6]  << 1) | bits[7];
        let b1 = (bits[8]  << 7) | (bits[9]  << 6) | (bits[10] << 5) | (bits[11] << 4)
               | (bits[12] << 3) | (bits[13] << 2) | (bits[14] << 1) | bits[15];
        let b2 = (bits[16] << 7) | (bits[17] << 6) | (bits[18] << 5) | (bits[19] << 4);
        return vec![b0, b1, b2];
    }

    // Pad to byte boundary
    let rem = w.bit_len() % 8;
    if rem != 0 {
        w.write(0, (8 - rem) as u32);
    }

    // Fill remaining codewords with alternating 0xEC / 0x11
    let mut bytes = w.to_bytes();
    let mut pad = 0xecu8;
    while bytes.len() < cfg.data_cw {
        bytes.push(pad);
        pad = if pad == 0xec { 0x11 } else { 0xec };
    }
    bytes
}

// ─────────────────────────────────────────────────────────────────────────────
// Symbol selection
// ─────────────────────────────────────────────────────────────────────────────

/// Find the smallest symbol configuration that can hold the given input.
fn select_config<'a>(
    input: &str,
    version: Option<MicroQRVersion>,
    ecc: Option<MicroQREccLevel>,
) -> Result<&'a SymbolConfig, MicroQRError> {
    let candidates: Vec<&SymbolConfig> = SYMBOL_CONFIGS.iter()
        .filter(|c| {
            if let Some(v) = version { if c.version != v { return false; } }
            if let Some(e) = ecc     { if c.ecc    != e { return false; } }
            true
        })
        .collect();

    if candidates.is_empty() {
        return Err(MicroQRError::ECCNotAvailable(format!(
            "No symbol configuration matches version={version:?} ecc={ecc:?}"
        )));
    }

    for cfg in &candidates {
        if let Ok(mode) = select_mode(input, cfg) {
            let len = if mode == EncodingMode::Byte { input.len() } else { input.chars().count() };
            let cap = match mode {
                EncodingMode::Numeric      => cfg.numeric_cap,
                EncodingMode::Alphanumeric => cfg.alpha_cap,
                EncodingMode::Byte         => cfg.byte_cap,
            };
            if cap > 0 && len <= cap {
                return Ok(cfg);
            }
        }
    }

    Err(MicroQRError::InputTooLong(format!(
        "Input (length {}) does not fit in any Micro QR symbol (version={version:?}, ecc={ecc:?}). \
         Maximum is 35 numeric chars in M4-L.",
        input.len()
    )))
}

// ─────────────────────────────────────────────────────────────────────────────
// Grid construction
// ─────────────────────────────────────────────────────────────────────────────

/// Working grid holding module values and reservation flags.
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

    #[inline]
    fn set(&mut self, row: usize, col: usize, dark: bool, reserve: bool) {
        self.modules[row][col]  = dark;
        if reserve { self.reserved[row][col] = true; }
    }
}

/// Place the 7×7 finder pattern at the top-left corner (rows 0–6, cols 0–6).
///
/// ```text
/// ■ ■ ■ ■ ■ ■ ■
/// ■ □ □ □ □ □ ■
/// ■ □ ■ ■ ■ □ ■
/// ■ □ ■ ■ ■ □ ■
/// ■ □ ■ ■ ■ □ ■
/// ■ □ □ □ □ □ ■
/// ■ ■ ■ ■ ■ ■ ■
/// ```
fn place_finder(g: &mut WorkGrid) {
    for dr in 0..7 {
        for dc in 0..7 {
            let on_border = dr == 0 || dr == 6 || dc == 0 || dc == 6;
            let in_core   = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4;
            g.set(dr, dc, on_border || in_core, true);
        }
    }
}

/// Place the L-shaped separator (light modules at row 7 cols 0–7, col 7 rows 0–7).
///
/// Unlike regular QR which surrounds all three finders, Micro QR's single finder
/// only needs separation on its bottom and right sides (the top and left are the
/// symbol boundary itself).
fn place_separator(g: &mut WorkGrid) {
    for i in 0..=7 {
        g.set(7, i, false, true);  // bottom row
        g.set(i, 7, false, true);  // right column
    }
}

/// Place timing pattern extensions along row 0 and col 0.
///
/// Positions 0–6 are already the finder pattern. Position 7 is the separator.
/// Position 8 onward: dark at even index, light at odd index.
fn place_timing(g: &mut WorkGrid) {
    let sz = g.size;
    for c in 8..sz {
        g.set(0, c, c % 2 == 0, true);
    }
    for r in 8..sz {
        g.set(r, 0, r % 2 == 0, true);
    }
}

/// Reserve the 15 format information module positions.
///
/// Row 8, cols 1–8 → bits f14..f7 (MSB first)
/// Col 8, rows 1–7 → bits f6..f0 (f6 at row 7, f0 at row 1)
fn reserve_format_info(g: &mut WorkGrid) {
    for c in 1..=8 { g.set(8, c, false, true); }
    for r in 1..=7 { g.set(r, 8, false, true); }
}

/// Write the 15-bit format word into the reserved positions.
///
/// Bit f14 (MSB) → row 8 col 1, f13 → row 8 col 2, ..., f7 → row 8 col 8.
/// f6 → col 8 row 7, f5 → col 8 row 6, ..., f0 (LSB) → col 8 row 1.
fn write_format_info(g: &mut WorkGrid, fmt: u16) {
    // Row 8, cols 1–8: bits f14 down to f7
    for i in 0..8_u16 {
        g.modules[8][1 + i as usize] = ((fmt >> (14 - i)) & 1) == 1;
    }
    // Col 8, rows 7 down to 1: bits f6 down to f0
    for i in 0..7_u16 {
        g.modules[(7 - i) as usize][8] = ((fmt >> (6 - i)) & 1) == 1;
    }
}

/// Initialize the grid with all structural modules.
fn build_grid(cfg: &SymbolConfig) -> WorkGrid {
    let mut g = WorkGrid::new(cfg.size);
    place_finder(&mut g);
    place_separator(&mut g);
    place_timing(&mut g);
    reserve_format_info(&mut g);
    g
}

// ─────────────────────────────────────────────────────────────────────────────
// Data placement (two-column zigzag)
// ─────────────────────────────────────────────────────────────────────────────

/// Place bits from the final codeword stream into the grid via two-column zigzag.
///
/// Scans from the bottom-right corner, moving left two columns at a time,
/// alternating upward and downward directions. Reserved modules are skipped.
///
/// Note: unlike regular QR, there is no timing column at col 6 to hop over.
/// Micro QR's timing is at col 0, which is reserved and auto-skipped.
fn place_bits(g: &mut WorkGrid, bits: &[bool]) {
    let sz = g.size;
    let mut bit_idx = 0;
    let mut up = true;

    let mut col = sz as isize - 1;
    while col >= 1 {
        let col_u = col as usize;
        for vi in 0..sz {
            let row = if up { sz - 1 - vi } else { vi };
            for dc in 0..=1_usize {
                let c = col_u - dc;
                if g.reserved[row][c] { continue; }
                g.modules[row][c] = if bit_idx < bits.len() {
                    let b = bits[bit_idx];
                    bit_idx += 1;
                    b
                } else {
                    false
                };
            }
        }
        up = !up;
        col -= 2;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Masking
// ─────────────────────────────────────────────────────────────────────────────

/// Test if mask pattern `mask_idx` applies to module (row, col).
///
/// | Pattern | Condition |
/// |---------|-----------|
/// | 0       | (row + col) mod 2 == 0 |
/// | 1       | row mod 2 == 0 |
/// | 2       | col mod 3 == 0 |
/// | 3       | (row + col) mod 3 == 0 |
#[inline]
fn mask_condition(mask_idx: usize, row: usize, col: usize) -> bool {
    match mask_idx {
        0 => (row + col) % 2 == 0,
        1 => row % 2 == 0,
        2 => col % 3 == 0,
        3 => (row + col) % 3 == 0,
        _ => false,
    }
}

/// Apply mask pattern to all non-reserved modules. Returns a new module grid.
fn apply_mask(
    modules: &[Vec<bool>],
    reserved: &[Vec<bool>],
    sz: usize,
    mask_idx: usize,
) -> Vec<Vec<bool>> {
    let mut result = modules.to_vec();
    for r in 0..sz {
        for c in 0..sz {
            if !reserved[r][c] {
                result[r][c] = modules[r][c] != mask_condition(mask_idx, r, c);
            }
        }
    }
    result
}

// ─────────────────────────────────────────────────────────────────────────────
// Penalty scoring
// ─────────────────────────────────────────────────────────────────────────────

/// Compute the 4-rule penalty score (same rules as regular QR Code).
///
/// Rule 1: runs of ≥5 same-color modules → score += (run − 2)
/// Rule 2: 2×2 same-color blocks → score += 3
/// Rule 3: finder-like sequences → score += 40 each
/// Rule 4: dark proportion deviation from 50% → scaled penalty
fn compute_penalty(modules: &[Vec<bool>], sz: usize) -> u32 {
    let mut penalty = 0u32;

    // Rule 1 — adjacent same-color runs of ≥ 5
    for a in 0..sz {
        for horiz in [true, false] {
            let mut run = 1u32;
            let mut prev = if horiz { modules[a][0] } else { modules[0][a] };
            for i in 1..sz {
                let cur = if horiz { modules[a][i] } else { modules[i][a] };
                if cur == prev {
                    run += 1;
                } else {
                    if run >= 5 { penalty += run - 2; }
                    run = 1;
                    prev = cur;
                }
            }
            if run >= 5 { penalty += run - 2; }
        }
    }

    // Rule 2 — 2×2 same-color blocks
    for r in 0..sz - 1 {
        for c in 0..sz - 1 {
            let d = modules[r][c];
            if d == modules[r][c + 1] && d == modules[r + 1][c] && d == modules[r + 1][c + 1] {
                penalty += 3;
            }
        }
    }

    // Rule 3 — finder-pattern-like sequences
    const P1: [u8; 11] = [1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0];
    const P2: [u8; 11] = [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1];
    for a in 0..sz {
        let limit = if sz >= 11 { sz - 11 } else { 0 };
        for b in 0..=limit {
            let (mut mh1, mut mh2, mut mv1, mut mv2) = (true, true, true, true);
            for k in 0..11 {
                let bh = if modules[a][b + k] { 1u8 } else { 0 };
                let bv = if modules[b + k][a] { 1u8 } else { 0 };
                if bh != P1[k] { mh1 = false; }
                if bh != P2[k] { mh2 = false; }
                if bv != P1[k] { mv1 = false; }
                if bv != P2[k] { mv2 = false; }
            }
            if mh1 { penalty += 40; }
            if mh2 { penalty += 40; }
            if mv1 { penalty += 40; }
            if mv2 { penalty += 40; }
        }
    }

    // Rule 4 — dark proportion deviation from 50%
    let dark: usize = modules.iter().flatten().filter(|&&d| d).count();
    let total = sz * sz;
    let dark_pct = (dark * 100) / total;
    let prev5 = (dark_pct / 5) * 5;
    let next5 = prev5 + 5;
    let r4 = ((prev5 as i32 - 50).unsigned_abs())
        .min((next5 as i32 - 50).unsigned_abs());
    penalty += (r4 / 5) * 10;

    penalty
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Encode a string to a Micro QR Code [`ModuleGrid`].
///
/// Automatically selects the smallest symbol (M1..M4) and ECC level
/// that can hold the input. Pass `version` and/or `ecc` to override.
///
/// # Errors
///
/// - [`MicroQRError::InputTooLong`] if the input exceeds M4 capacity.
/// - [`MicroQRError::ECCNotAvailable`] if the requested version+ECC combination
///   does not exist.
/// - [`MicroQRError::UnsupportedMode`] if no encoding mode is available for
///   the input in the selected symbol.
///
/// # Examples
///
/// ```
/// use micro_qr::{encode, MicroQRVersion, MicroQREccLevel};
///
/// let grid = encode("HELLO", None, None).unwrap();
/// assert_eq!(grid.rows, 13); // M2 symbol
///
/// let m4 = encode("https://a.b", Some(MicroQRVersion::M4), Some(MicroQREccLevel::L)).unwrap();
/// assert_eq!(m4.rows, 17);
/// ```
pub fn encode(
    input: &str,
    version: Option<MicroQRVersion>,
    ecc: Option<MicroQREccLevel>,
) -> Result<ModuleGrid, MicroQRError> {
    let cfg = select_config(input, version, ecc)?;
    let mode = select_mode(input, cfg)?;

    // 1. Build data codewords
    let data_cw = build_data_codewords(input, cfg, mode);

    // 2. Compute RS ECC
    let gen = get_generator(cfg.ecc_cw);
    let ecc_cw = rs_encode(&data_cw, gen);

    // 3. Flatten to bit stream
    // For M1: data[2] has data in upper nibble only → contribute 4 bits from it
    let final_cw: Vec<u8> = data_cw.iter().chain(ecc_cw.iter()).copied().collect();
    let mut bits: Vec<bool> = Vec::new();
    for (cw_idx, &cw) in final_cw.iter().enumerate() {
        let bits_in_cw = if cfg.m1_half_cw && cw_idx == cfg.data_cw - 1 { 4 } else { 8 };
        for b in (0..bits_in_cw as u32).rev() {
            bits.push(((cw >> (b + (8 - bits_in_cw))) & 1) == 1);
        }
    }

    // 4. Build grid with structural modules
    let mut grid = build_grid(cfg);

    // 5. Place data bits
    place_bits(&mut grid, &bits);

    // 6. Evaluate all 4 masks, pick lowest penalty
    let mut best_mask = 0usize;
    let mut best_penalty = u32::MAX;
    for m in 0..4 {
        let masked = apply_mask(&grid.modules, &grid.reserved, cfg.size, m);
        let fmt = FORMAT_TABLE[cfg.symbol_indicator as usize][m];
        // Write format info into a temporary copy
        let mut tmp_modules = masked.clone();
        // Inline format write to avoid borrowing
        for i in 0..8u16 {
            tmp_modules[8][1 + i as usize] = ((fmt >> (14 - i)) & 1) == 1;
        }
        for i in 0..7u16 {
            tmp_modules[(7 - i) as usize][8] = ((fmt >> (6 - i)) & 1) == 1;
        }
        let p = compute_penalty(&tmp_modules, cfg.size);
        if p < best_penalty {
            best_penalty = p;
            best_mask = m;
        }
    }

    // 7. Apply best mask and write final format info
    let final_modules = apply_mask(&grid.modules, &grid.reserved, cfg.size, best_mask);
    let final_fmt = FORMAT_TABLE[cfg.symbol_indicator as usize][best_mask];
    // Replace grid modules with final masked version + format info
    let mut final_grid = WorkGrid {
        size: cfg.size,
        modules: final_modules,
        reserved: grid.reserved,
    };
    write_format_info(&mut final_grid, final_fmt);

    Ok(ModuleGrid {
        rows: cfg.size as u32,
        cols: cfg.size as u32,
        modules: final_grid.modules,
        module_shape: ModuleShape::Square,
    })
}

/// Convert a [`ModuleGrid`] to a `PaintScene` via `barcode-2d::layout()`.
///
/// Defaults to `quiet_zone_modules: 2` (Micro QR minimum, half of regular QR's 4).
pub fn layout_grid(
    grid: &ModuleGrid,
    config: Option<Barcode2DLayoutConfig>,
) -> Result<paint_instructions::PaintScene, MicroQRError> {
    let cfg = config.unwrap_or_else(|| Barcode2DLayoutConfig {
        quiet_zone_modules: 2,
        ..Barcode2DLayoutConfig::default()
    });
    layout(grid, &cfg).map_err(|e| MicroQRError::LayoutError(e.to_string()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_to_string(grid: &ModuleGrid) -> String {
        grid.modules.iter()
            .map(|row| row.iter().map(|&d| if d { '1' } else { '0' }).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    // ── Symbol dimensions ──────────────────────────────────────────────────

    #[test]
    fn test_m1_is_11x11() {
        let g = encode("1", None, None).unwrap();
        assert_eq!(g.rows, 11u32);
        assert_eq!(g.cols, 11u32);
    }

    #[test]
    fn test_m2_is_13x13_for_hello() {
        let g = encode("HELLO", None, None).unwrap();
        assert_eq!(g.rows, 13);
        assert_eq!(g.cols, 13);
    }

    #[test]
    fn test_m4_is_17x17_for_url() {
        let g = encode("https://a.b", None, None).unwrap();
        assert_eq!(g.rows, 17);
        assert_eq!(g.cols, 17);
    }

    #[test]
    fn test_module_shape_is_square() {
        let g = encode("1", None, None).unwrap();
        assert_eq!(g.module_shape, ModuleShape::Square);
    }

    // ── Auto-version selection ────────────────────────────────────────────

    #[test]
    fn test_auto_selects_m1_for_single_digit() {
        assert_eq!(encode("1", None, None).unwrap().rows, 11);
    }

    #[test]
    fn test_auto_selects_m1_for_12345() {
        assert_eq!(encode("12345", None, None).unwrap().rows, 11);
    }

    #[test]
    fn test_auto_selects_m2_for_6_digits() {
        assert_eq!(encode("123456", None, None).unwrap().rows, 13);
    }

    #[test]
    fn test_auto_selects_m2_for_hello() {
        assert_eq!(encode("HELLO", None, None).unwrap().rows, 13);
    }

    #[test]
    fn test_auto_selects_m3_for_hello_lowercase() {
        // "hello" is byte mode, M3-L has cap 9 bytes, M2-L has cap 4 → M3
        assert!(encode("hello", None, None).unwrap().rows >= 15);
    }

    #[test]
    fn test_auto_selects_m4_for_url() {
        assert_eq!(encode("https://a.b", None, None).unwrap().rows, 17);
    }

    #[test]
    fn test_forced_version_m4() {
        let g = encode("1", Some(MicroQRVersion::M4), None).unwrap();
        assert_eq!(g.rows, 17);
    }

    #[test]
    fn test_forced_ecc_m() {
        let g1 = encode("HELLO", None, Some(MicroQREccLevel::L)).unwrap();
        let g2 = encode("HELLO", None, Some(MicroQREccLevel::M)).unwrap();
        // Both M2 but different ECC → different grids
        assert_ne!(grid_to_string(&g1), grid_to_string(&g2));
    }

    // ── Test corpus ──────────────────────────────────────────────────────

    #[test]
    fn test_corpus_1() {
        assert_eq!(encode("1", None, None).unwrap().rows, 11);
    }

    #[test]
    fn test_corpus_12345() {
        assert_eq!(encode("12345", None, None).unwrap().rows, 11);
    }

    #[test]
    fn test_corpus_hello() {
        assert_eq!(encode("HELLO", None, None).unwrap().rows, 13);
    }

    #[test]
    fn test_corpus_a1b2c3_m2() {
        // 6 alphanumeric fits in M2-L (alphaCap=6)
        assert_eq!(encode("A1B2C3", None, None).unwrap().rows, 13);
    }

    #[test]
    fn test_corpus_hello_byte_m3() {
        // "hello" = 5 bytes → M3-L (byte_cap=9)
        assert!(encode("hello", None, None).unwrap().rows >= 15);
    }

    #[test]
    fn test_corpus_8_digit_numeric() {
        // 8 digits → M2-L (numeric_cap=10)
        assert_eq!(encode("01234567", None, None).unwrap().rows, 13);
    }

    #[test]
    fn test_corpus_micro_qr_test_m3l() {
        // 13 alphanumeric → M3-L (alpha_cap=14)
        assert_eq!(encode("MICRO QR TEST", None, None).unwrap().rows, 15);
    }

    // ── Structural modules ────────────────────────────────────────────────

    #[test]
    fn test_finder_pattern_m1() {
        let g = encode("1", None, None).unwrap();
        let m = &g.modules;
        // Top and bottom rows of finder (row 0, row 6): all dark
        for c in 0..7 { assert!(m[0][c], "row 0 col {c} should be dark"); }
        for c in 0..7 { assert!(m[6][c], "row 6 col {c} should be dark"); }
        // Left and right cols of finder (col 0, col 6): all dark
        for r in 0..7 { assert!(m[r][0], "col 0 row {r} should be dark"); }
        for r in 0..7 { assert!(m[r][6], "col 6 row {r} should be dark"); }
        // Inner ring: light
        for c in 1..=5 { assert!(!m[1][c], "inner ring row 1 col {c} should be light"); }
        // Core: dark
        for r in 2..=4 { for c in 2..=4 { assert!(m[r][c], "core ({r},{c}) should be dark"); } }
    }

    #[test]
    fn test_separator_m2() {
        let g = encode("HELLO", None, None).unwrap();
        let m = &g.modules;
        // Row 7, cols 0-7: light
        for c in 0..=7 { assert!(!m[7][c], "separator row 7 col {c} should be light"); }
        // Col 7, rows 0-7: light
        for r in 0..=7 { assert!(!m[r][7], "separator col 7 row {r} should be light"); }
    }

    #[test]
    fn test_timing_row_m4() {
        let g = encode("https://a.b", None, None).unwrap();
        let m = &g.modules;
        for c in 8..17 {
            assert_eq!(m[0][c], c % 2 == 0, "timing row 0 col {c}");
        }
    }

    #[test]
    fn test_timing_col_m4() {
        let g = encode("https://a.b", None, None).unwrap();
        let m = &g.modules;
        for r in 8..17 {
            assert_eq!(m[r][0], r % 2 == 0, "timing col 0 row {r}");
        }
    }

    // ── Determinism ──────────────────────────────────────────────────────

    #[test]
    fn test_deterministic() {
        for input in ["1", "12345", "HELLO", "A1B2C3", "hello", "https://a.b"] {
            let g1 = encode(input, None, None).unwrap();
            let g2 = encode(input, None, None).unwrap();
            assert_eq!(grid_to_string(&g1), grid_to_string(&g2), "non-deterministic for '{input}'");
        }
    }

    #[test]
    fn test_different_inputs_different_grids() {
        let g1 = encode("1", None, None).unwrap();
        let g2 = encode("2", None, None).unwrap();
        assert_ne!(grid_to_string(&g1), grid_to_string(&g2));
    }

    // ── ECC level constraints ─────────────────────────────────────────────

    #[test]
    fn test_m1_detection() {
        let g = encode("1", Some(MicroQRVersion::M1), Some(MicroQREccLevel::Detection)).unwrap();
        assert_eq!(g.rows, 11);
    }

    #[test]
    fn test_m4_q() {
        let g = encode("HELLO", Some(MicroQRVersion::M4), Some(MicroQREccLevel::Q)).unwrap();
        assert_eq!(g.rows, 17);
    }

    #[test]
    fn test_m4_all_ecc_differ() {
        let gl = encode("HELLO", Some(MicroQRVersion::M4), Some(MicroQREccLevel::L)).unwrap();
        let gm = encode("HELLO", Some(MicroQRVersion::M4), Some(MicroQREccLevel::M)).unwrap();
        let gq = encode("HELLO", Some(MicroQRVersion::M4), Some(MicroQREccLevel::Q)).unwrap();
        assert_ne!(grid_to_string(&gl), grid_to_string(&gm));
        assert_ne!(grid_to_string(&gm), grid_to_string(&gq));
        assert_ne!(grid_to_string(&gl), grid_to_string(&gq));
    }

    #[test]
    fn test_m1_rejects_ecc_l() {
        assert!(matches!(
            encode("1", Some(MicroQRVersion::M1), Some(MicroQREccLevel::L)),
            Err(MicroQRError::ECCNotAvailable(_))
        ));
    }

    #[test]
    fn test_m2_rejects_ecc_q() {
        assert!(matches!(
            encode("1", Some(MicroQRVersion::M2), Some(MicroQREccLevel::Q)),
            Err(MicroQRError::ECCNotAvailable(_))
        ));
    }

    #[test]
    fn test_m3_rejects_ecc_q() {
        assert!(matches!(
            encode("1", Some(MicroQRVersion::M3), Some(MicroQREccLevel::Q)),
            Err(MicroQRError::ECCNotAvailable(_))
        ));
    }

    // ── Error handling ────────────────────────────────────────────────────

    #[test]
    fn test_input_too_long() {
        let long = "1".repeat(36);
        assert!(matches!(
            encode(&long, None, None),
            Err(MicroQRError::InputTooLong(_))
        ));
    }

    #[test]
    fn test_empty_string_encodes_to_m1() {
        let g = encode("", None, None).unwrap();
        assert_eq!(g.rows, 11);
    }

    #[test]
    fn test_ecc_not_available_for_nonexistent_combo() {
        assert!(matches!(
            encode("1", Some(MicroQRVersion::M1), Some(MicroQREccLevel::Q)),
            Err(MicroQRError::ECCNotAvailable(_))
        ));
    }

    // ── Capacity boundaries ───────────────────────────────────────────────

    #[test]
    fn test_m1_max_5_digits() {
        assert_eq!(encode("12345", None, None).unwrap().rows, 11);
    }

    #[test]
    fn test_m1_overflow_6_digits() {
        assert_eq!(encode("123456", None, None).unwrap().rows, 13);
    }

    #[test]
    fn test_m4_max_35_digits() {
        let g = encode(&"1".repeat(35), None, None).unwrap();
        assert_eq!(g.rows, 17);
    }

    #[test]
    fn test_m4_overflow_36_digits() {
        assert!(matches!(
            encode(&"1".repeat(36), None, None),
            Err(MicroQRError::InputTooLong(_))
        ));
    }

    #[test]
    fn test_m4_max_byte_15_chars() {
        let g = encode(&"a".repeat(15), None, None).unwrap();
        assert_eq!(g.rows, 17);
    }

    #[test]
    fn test_m4_q_max_21_numeric() {
        let g = encode(&"1".repeat(21), None, Some(MicroQREccLevel::Q)).unwrap();
        assert_eq!(g.rows, 17);
    }

    // ── Format information ────────────────────────────────────────────────

    #[test]
    fn test_format_info_non_zero_m4() {
        let g = encode("HELLO", Some(MicroQRVersion::M4), Some(MicroQREccLevel::L)).unwrap();
        let m = &g.modules;
        let any_dark_row = (1..=8).any(|c| m[8][c]);
        let any_dark_col = (1..=7).any(|r| m[r][8]);
        assert!(any_dark_row || any_dark_col, "format info should have some dark modules");
    }

    #[test]
    fn test_format_info_non_zero_m1() {
        let g = encode("1", None, None).unwrap();
        let m = &g.modules;
        let count: usize = (1..=8).filter(|&c| m[8][c]).count()
            + (1..=7).filter(|&r| m[r][8]).count();
        assert!(count > 0, "M1 format info should have some dark modules");
    }

    // ── Grid completeness ─────────────────────────────────────────────────

    #[test]
    fn test_all_modules_are_bool() {
        for input in ["1", "HELLO", "hello", "https://a.b"] {
            let g = encode(input, None, None).unwrap();
            assert_eq!(g.rows, g.cols, "grid should be square for '{input}'");
            assert_eq!(g.modules.len(), g.rows as usize);
            for row in &g.modules {
                assert_eq!(row.len(), g.cols as usize);
            }
        }
    }

    #[test]
    fn test_cross_language_corpus() {
        // Verify the test corpus from the spec produces expected symbol sizes
        // (cross-language verification is done by comparing serialized grids)
        let cases: &[(&str, u32)] = &[
            ("1",            11),  // M1
            ("12345",        11),  // M1 max
            ("HELLO",        13),  // M2-L alphanumeric
            ("01234567",     13),  // M2-L numeric 8 digits
            ("https://a.b",  17),  // M4-L byte mode
            ("MICRO QR TEST",15),  // M3-L alphanumeric 13 chars
        ];
        for &(input, expected_size) in cases {
            let g = encode(input, None, None).unwrap();
            assert_eq!(g.rows, expected_size,
                "input '{}': expected {}×{} but got {}×{}",
                input, expected_size, expected_size, g.rows, g.cols);
        }
    }
}
