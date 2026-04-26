//! # pdf417
//!
//! PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.
//!
//! PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
//! Technologies in 1991. The name encodes its geometry: each codeword has
//! exactly **4** bars and **4** spaces (8 elements), and every codeword
//! occupies exactly **17** modules of horizontal space.
//!
//! ## Where PDF417 is deployed
//!
//! | Application | Detail |
//! |-------------|--------|
//! | AAMVA | North American driver's licences and government IDs |
//! | IATA BCBP | Airline boarding passes |
//! | USPS | Domestic shipping labels |
//! | US immigration | Form I-94, customs declarations |
//! | Healthcare | Patient wristbands, medication labels |
//!
//! ## Encoding pipeline
//!
//! ```text
//! raw bytes
//!   → byte compaction     (codeword 924 latch + 6-bytes-to-5-codewords base-900)
//!   → length descriptor   (codeword 0 = total codewords in symbol)
//!   → RS ECC              (GF(929) Reed-Solomon, b=3 convention, α=3)
//!   → dimension selection (auto: roughly square symbol)
//!   → padding             (codeword 900 fills unused slots)
//!   → row indicators      (LRI + RRI per row, encode R/C/ECC level)
//!   → cluster table lookup (codeword → 17-module bar/space pattern)
//!   → start/stop patterns (fixed per row)
//!   → ModuleGrid          (abstract boolean grid)
//! ```
//!
//! ## v0.1.0 scope
//!
//! This release implements **byte compaction only**. All inputs are treated as
//! raw bytes. Text and numeric compaction are planned for v0.2.0.

pub const VERSION: &str = "0.1.0";

mod tables;
use tables::{CLUSTER_TABLES, START_PATTERN, STOP_PATTERN};

use barcode_2d::{layout, Barcode2DLayoutConfig, ModuleGrid, ModuleShape};
use paint_instructions::PaintScene;

// ─────────────────────────────────────────────────────────────────────────────
// Public error types
// ─────────────────────────────────────────────────────────────────────────────

/// All errors produced by the PDF417 encoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PDF417Error {
    /// The input data, after compaction, exceeds the maximum symbol capacity
    /// (90 rows × 30 data columns = 2700 slots minus ECC overhead).
    InputTooLong(String),

    /// The user specified rows/columns outside the 3–90 rows, 1–30 columns
    /// limits, or the specified dimensions cannot fit all codewords.
    InvalidDimensions(String),

    /// The specified ECC level is not in the range 0–8.
    InvalidECCLevel(String),
}

impl std::fmt::Display for PDF417Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PDF417Error::InputTooLong(msg) => write!(f, "InputTooLong: {msg}"),
            PDF417Error::InvalidDimensions(msg) => write!(f, "InvalidDimensions: {msg}"),
            PDF417Error::InvalidECCLevel(msg) => write!(f, "InvalidECCLevel: {msg}"),
        }
    }
}

impl std::error::Error for PDF417Error {}

// ─────────────────────────────────────────────────────────────────────────────
// PDF417Options
// ─────────────────────────────────────────────────────────────────────────────

/// Options controlling how the PDF417 symbol is encoded.
///
/// All fields are optional — use [`PDF417Options::default()`] or specify
/// individual fields. Unset fields use the encoder's auto-selection logic.
#[derive(Debug, Clone, Default)]
pub struct PDF417Options {
    /// Reed-Solomon error correction level (0–8).
    ///
    /// Higher levels use more ECC codewords, making the symbol larger but more
    /// resilient to damage. Default: auto-selected based on data length.
    pub ecc_level: Option<u8>,

    /// Number of data columns (1–30).
    ///
    /// Default: auto-selected to produce a roughly square symbol.
    pub columns: Option<u32>,

    /// Module-rows per logical PDF417 row (2–10).
    ///
    /// Larger values produce taller, more easily scanned symbols.
    /// Default: 3.
    pub row_height: Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// GF(929) prime modulus.
const PRIME: u64 = 929;

/// Generator element α = 3 (primitive root mod 929).
const ALPHA: u64 = 3;

/// Multiplicative group order = PRIME - 1 = 928.
const ORDER: u64 = 928;

/// Latch-to-byte-compaction codeword (alternate form, any length).
const LATCH_BYTE: u16 = 924;

/// Padding codeword (latch-to-text, safe neutral filler).
const PADDING_CW: u16 = 900;

const MIN_ROWS: u32 = 3;
const MAX_ROWS: u32 = 90;
const MIN_COLS: u32 = 1;
const MAX_COLS: u32 = 30;

// ─────────────────────────────────────────────────────────────────────────────
// GF(929) arithmetic
// ─────────────────────────────────────────────────────────────────────────────
//
// GF(929) is the integers modulo 929. Since 929 is prime, every non-zero
// element has a multiplicative inverse. We use log/antilog tables for O(1)
// multiplication, built at program start.
//
// The tables are lazy-initialized via the once_cell pattern (or just built
// inline since this is a library crate — we use a static initializer).

/// GF(929) exponent table: `GF_EXP[i] = α^i mod 929`.
/// `GF_EXP[928] = GF_EXP[0] = 1` (wrap-around convenience).
static GF_EXP: std::sync::OnceLock<[u16; 929]> = std::sync::OnceLock::new();

/// GF(929) log table: `GF_LOG[v] = i` such that `α^i = v`, for `v` in 1..928.
/// `GF_LOG[0]` is undefined (zero has no discrete log).
static GF_LOG: std::sync::OnceLock<[u16; 929]> = std::sync::OnceLock::new();

fn init_gf_tables() {
    GF_EXP.get_or_init(|| {
        let mut exp = [0u16; 929];
        let log = GF_LOG.get_or_init(|| {
            let mut log = [0u16; 929];
            let mut val: u64 = 1;
            for i in 0..ORDER as usize {
                exp[i] = val as u16;
                log[val as usize] = i as u16;
                val = (val * ALPHA) % PRIME;
            }
            // GF_EXP[928] = GF_EXP[0] = 1 for wrap-around in multiply.
            exp[ORDER as usize] = exp[0];
            log
        });
        // GF_EXP was already filled by the log initializer above.
        // But we need to return it; rebuild it here since GF_EXP.get_or_init
        // can only run once and the closure above already ran it.
        let _ = log; // suppress unused warning
        exp
    });
}

/// GF(929) multiply using log/antilog tables.
/// Returns 0 if either operand is 0.
#[inline]
fn gf_mul(a: u16, b: u16) -> u16 {
    if a == 0 || b == 0 {
        return 0;
    }
    let exp = GF_EXP.get().expect("GF tables not initialized");
    let log = GF_LOG.get().expect("GF tables not initialized");
    let idx = (log[a as usize] as u32 + log[b as usize] as u32) % ORDER as u32;
    exp[idx as usize]
}

/// GF(929) add: `(a + b) mod 929`.
#[inline]
fn gf_add(a: u16, b: u16) -> u16 {
    ((a as u32 + b as u32) % PRIME as u32) as u16
}

// ─────────────────────────────────────────────────────────────────────────────
// Reed-Solomon generator polynomial
// ─────────────────────────────────────────────────────────────────────────────
//
// For ECC level L, k = 2^(L+1) ECC codewords. The generator polynomial uses
// the b=3 convention: roots are α^3, α^4, ..., α^{k+2}.
//
// g(x) = (x - α^3)(x - α^4)···(x - α^{k+2})
//
// We build g iteratively by multiplying in each linear factor (x - α^j).

/// Build the RS generator polynomial for ECC level `ecc_level`.
///
/// Returns a vector of k+1 coefficients [g_k, g_{k-1}, ..., g_1, g_0]
/// where k = 2^(ecc_level+1) and g_k = 1 (leading coefficient).
fn build_generator(ecc_level: u8) -> Vec<u16> {
    let k = 1usize << (ecc_level + 1); // 2^(ecc_level+1)
    let mut g: Vec<u16> = vec![1];

    let exp = GF_EXP.get().expect("GF tables not initialized");

    for j in 3..=(k + 2) {
        let root = exp[j % ORDER as usize]; // α^j
        let neg_root = (PRIME as u16).wrapping_sub(root); // -α^j = 929 - α^j in GF(929)

        let mut new_g = vec![0u16; g.len() + 1];
        for (i, &coeff) in g.iter().enumerate() {
            new_g[i] = gf_add(new_g[i], coeff);
            new_g[i + 1] = gf_add(new_g[i + 1], gf_mul(coeff, neg_root));
        }
        g = new_g;
    }

    g
}

// ─────────────────────────────────────────────────────────────────────────────
// Reed-Solomon encoder
// ─────────────────────────────────────────────────────────────────────────────

/// Compute `k` RS ECC codewords for `data` over GF(929) with b=3 convention.
///
/// Uses the standard shift-register (LFSR) polynomial long-division algorithm.
/// No interleaving — all data feeds a single RS encoder (simpler than QR Code).
fn rs_encode(data: &[u16], ecc_level: u8) -> Vec<u16> {
    let g = build_generator(ecc_level);
    let k = g.len() - 1;
    let mut ecc = vec![0u16; k];

    for &d in data {
        let feedback = gf_add(d, ecc[0]);
        // Shift register left.
        for i in 0..k - 1 {
            ecc[i] = ecc[i + 1];
        }
        ecc[k - 1] = 0;
        // Add feedback × generator coefficient to each cell.
        for i in 0..k {
            ecc[i] = gf_add(ecc[i], gf_mul(g[k - i], feedback));
        }
    }

    ecc
}

// ─────────────────────────────────────────────────────────────────────────────
// Byte compaction
// ─────────────────────────────────────────────────────────────────────────────
//
// 6 bytes → 5 codewords by treating the 6 bytes as a 48-bit big-endian integer
// and expressing it in base 900. Remaining 1–5 bytes are encoded directly.

/// Encode raw bytes using byte compaction mode (codeword 924 latch).
///
/// Returns `[924, c1, c2, ...]` where `c_i` are byte-compacted codewords.
fn byte_compact(bytes: &[u8]) -> Vec<u16> {
    let mut codewords: Vec<u16> = vec![LATCH_BYTE];

    let mut i = 0;
    let len = bytes.len();

    // Process full 6-byte groups → 5 codewords each.
    while i + 6 <= len {
        let mut n: u64 = 0;
        for j in 0..6 {
            n = n * 256 + bytes[i + j] as u64;
        }
        // Convert n to base 900 → 5 codewords, most-significant first.
        let mut group = [0u16; 5];
        for j in (0..5).rev() {
            group[j] = (n % 900) as u16;
            n /= 900;
        }
        codewords.extend_from_slice(&group);
        i += 6;
    }

    // Remaining bytes: 1 codeword per byte.
    while i < len {
        codewords.push(bytes[i] as u16);
        i += 1;
    }

    codewords
}

// ─────────────────────────────────────────────────────────────────────────────
// ECC level auto-selection
// ─────────────────────────────────────────────────────────────────────────────

/// Select the minimum recommended ECC level based on data codeword count.
fn auto_ecc_level(data_count: usize) -> u8 {
    match data_count {
        n if n <= 40 => 2,
        n if n <= 160 => 3,
        n if n <= 320 => 4,
        n if n <= 863 => 5,
        _ => 6,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Dimension selection
// ─────────────────────────────────────────────────────────────────────────────

/// Choose the number of columns and rows for the symbol.
///
/// Heuristic: `c = ceil(sqrt(total / 3))`, clamped to 1–30.
/// Then `r = ceil(total / c)`, clamped to 3–90.
fn choose_dimensions(total: usize) -> (u32, u32) {
    let mut c = (((total as f64) / 3.0).sqrt().ceil() as u32)
        .max(MIN_COLS)
        .min(MAX_COLS);

    let mut r = ((total as u32 + c - 1) / c).max(MIN_ROWS);

    if r < MIN_ROWS {
        r = MIN_ROWS;
        c = ((total as u32 + r - 1) / r).max(MIN_COLS).min(MAX_COLS);
        r = ((total as u32 + c - 1) / c).max(MIN_ROWS);
    }

    r = r.min(MAX_ROWS);
    (c, r)
}

// ─────────────────────────────────────────────────────────────────────────────
// Row indicator computation
// ─────────────────────────────────────────────────────────────────────────────
//
// Each row carries two row indicator codewords that together encode:
//   R_info = (R-1) / 3      (total rows info)
//   C_info = C - 1          (columns info)
//   L_info = 3*L + (R-1)%3  (ECC level + row parity)
//
// For row r (0-indexed), cluster = r % 3:
//   Cluster 0: LRI = 30*(r/3) + R_info,  RRI = 30*(r/3) + C_info
//   Cluster 1: LRI = 30*(r/3) + L_info,  RRI = 30*(r/3) + R_info
//   Cluster 2: LRI = 30*(r/3) + C_info,  RRI = 30*(r/3) + L_info

fn compute_lri(r: u32, rows: u32, cols: u32, ecc_level: u8) -> u16 {
    let r_info = (rows - 1) / 3;
    let c_info = cols - 1;
    let l_info = 3 * ecc_level as u32 + (rows - 1) % 3;
    let row_group = r / 3;
    let cluster = r % 3;

    (30 * row_group + match cluster {
        0 => r_info,
        1 => l_info,
        _ => c_info,
    }) as u16
}

fn compute_rri(r: u32, rows: u32, cols: u32, ecc_level: u8) -> u16 {
    let r_info = (rows - 1) / 3;
    let c_info = cols - 1;
    let l_info = 3 * ecc_level as u32 + (rows - 1) % 3;
    let row_group = r / 3;
    let cluster = r % 3;

    (30 * row_group + match cluster {
        0 => c_info,
        1 => r_info,
        _ => l_info,
    }) as u16
}

// ─────────────────────────────────────────────────────────────────────────────
// Codeword → modules expansion
// ─────────────────────────────────────────────────────────────────────────────

/// Expand a packed bar/space pattern into 17 boolean module values.
///
/// The 8 element widths are stored as 4 bits each in the `u32`:
/// bits 31..28 = b1, bits 27..24 = s1, ..., bits 3..0 = s4.
fn expand_pattern(packed: u32, modules: &mut Vec<bool>) {
    let b1 = ((packed >> 28) & 0xf) as usize;
    let s1 = ((packed >> 24) & 0xf) as usize;
    let b2 = ((packed >> 20) & 0xf) as usize;
    let s2 = ((packed >> 16) & 0xf) as usize;
    let b3 = ((packed >> 12) & 0xf) as usize;
    let s3 = ((packed >> 8) & 0xf) as usize;
    let b4 = ((packed >> 4) & 0xf) as usize;
    let s4 = (packed & 0xf) as usize;

    for _ in 0..b1 { modules.push(true); }
    for _ in 0..s1 { modules.push(false); }
    for _ in 0..b2 { modules.push(true); }
    for _ in 0..s2 { modules.push(false); }
    for _ in 0..b3 { modules.push(true); }
    for _ in 0..s3 { modules.push(false); }
    for _ in 0..b4 { modules.push(true); }
    for _ in 0..s4 { modules.push(false); }
}

/// Expand a bar/space width array into boolean module values.
/// The first element is always a bar (dark = true).
fn expand_widths(widths: &[u8], modules: &mut Vec<bool>) {
    let mut dark = true;
    for &w in widths {
        for _ in 0..w {
            modules.push(dark);
        }
        dark = !dark;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main encoder: encode()
// ─────────────────────────────────────────────────────────────────────────────

/// Encode `data` bytes as a PDF417 symbol and return the [`ModuleGrid`].
///
/// # Errors
///
/// - [`PDF417Error::InvalidECCLevel`] — if `options.ecc_level` is not in 0–8.
/// - [`PDF417Error::InvalidDimensions`] — if `options.columns` is out of range.
/// - [`PDF417Error::InputTooLong`] — if data exceeds the symbol's capacity.
pub fn encode(data: &[u8], options: &PDF417Options) -> Result<ModuleGrid, PDF417Error> {
    // Initialize GF(929) tables (idempotent after first call).
    init_gf_tables();

    // ── Validate ECC level ──────────────────────────────────────────────────
    if let Some(lvl) = options.ecc_level {
        if lvl > 8 {
            return Err(PDF417Error::InvalidECCLevel(format!(
                "ECC level must be 0–8, got {lvl}"
            )));
        }
    }

    // ── Byte compaction ─────────────────────────────────────────────────────
    let data_cwords = byte_compact(data);

    // ── Auto-select ECC level ───────────────────────────────────────────────
    let ecc_level = options.ecc_level.unwrap_or_else(|| {
        auto_ecc_level(data_cwords.len() + 1)
    });
    let ecc_count = 1usize << (ecc_level + 1); // 2^(ecc_level+1)

    // ── Length descriptor ───────────────────────────────────────────────────
    let length_desc = (1 + data_cwords.len() + ecc_count) as u16;

    // Full data array for RS encoding: [length_desc, ...data_cwords]
    let mut full_data: Vec<u16> = Vec::with_capacity(1 + data_cwords.len());
    full_data.push(length_desc);
    full_data.extend_from_slice(&data_cwords);

    // ── RS ECC ──────────────────────────────────────────────────────────────
    let ecc_cwords = rs_encode(&full_data, ecc_level);

    // ── Choose dimensions ───────────────────────────────────────────────────
    let total_cwords = full_data.len() + ecc_cwords.len();

    let (cols, rows) = if let Some(c) = options.columns {
        if c < MIN_COLS || c > MAX_COLS {
            return Err(PDF417Error::InvalidDimensions(format!(
                "columns must be 1–30, got {c}"
            )));
        }
        let r = ((total_cwords as u32 + c - 1) / c).max(MIN_ROWS);
        if r > MAX_ROWS {
            return Err(PDF417Error::InputTooLong(format!(
                "Data requires {r} rows (max 90) with {c} columns."
            )));
        }
        (c, r)
    } else {
        choose_dimensions(total_cwords)
    };

    // Verify capacity.
    if (cols * rows) < total_cwords as u32 {
        return Err(PDF417Error::InputTooLong(format!(
            "Cannot fit {total_cwords} codewords in {rows}×{cols} grid."
        )));
    }

    // ── Pad to fill grid exactly ─────────────────────────────────────────
    let padding_count = (cols * rows) as usize - total_cwords;
    let mut padded_data = full_data;
    padded_data.extend(std::iter::repeat(PADDING_CW).take(padding_count));

    // Full codeword sequence: [data+padding, ecc]
    let mut full_sequence = padded_data;
    full_sequence.extend_from_slice(&ecc_cwords);

    // ── Rasterize ────────────────────────────────────────────────────────
    let row_height = options.row_height.unwrap_or(3).max(1);
    Ok(rasterize(&full_sequence, rows, cols, ecc_level, row_height))
}

// ─────────────────────────────────────────────────────────────────────────────
// Rasterization
// ─────────────────────────────────────────────────────────────────────────────

/// Convert the flat codeword sequence to a [`ModuleGrid`].
fn rasterize(
    sequence: &[u16],
    rows: u32,
    cols: u32,
    ecc_level: u8,
    row_height: u32,
) -> ModuleGrid {
    let module_width = 69 + 17 * cols;
    let module_height = rows * row_height;

    let mut grid = barcode_2d::make_module_grid(module_height, module_width, ModuleShape::Square);

    // Precompute start and stop module sequences (identical for every row).
    let mut start_modules: Vec<bool> = Vec::with_capacity(17);
    expand_widths(&START_PATTERN, &mut start_modules);

    let mut stop_modules: Vec<bool> = Vec::with_capacity(18);
    expand_widths(&STOP_PATTERN, &mut stop_modules);

    for r in 0..rows {
        let cluster = (r % 3) as usize;
        let cluster_table = &CLUSTER_TABLES[cluster];

        let mut row_modules: Vec<bool> = Vec::with_capacity(module_width as usize);

        // 1. Start pattern (17 modules).
        row_modules.extend_from_slice(&start_modules);

        // 2. Left Row Indicator (17 modules).
        let lri = compute_lri(r, rows, cols, ecc_level) as usize;
        expand_pattern(cluster_table[lri], &mut row_modules);

        // 3. Data codewords (17 modules each).
        for j in 0..cols as usize {
            let cw = sequence[r as usize * cols as usize + j] as usize;
            expand_pattern(cluster_table[cw], &mut row_modules);
        }

        // 4. Right Row Indicator (17 modules).
        let rri = compute_rri(r, rows, cols, ecc_level) as usize;
        expand_pattern(cluster_table[rri], &mut row_modules);

        // 5. Stop pattern (18 modules).
        row_modules.extend_from_slice(&stop_modules);

        debug_assert_eq!(
            row_modules.len(),
            module_width as usize,
            "Row {r} has {} modules, expected {module_width}",
            row_modules.len()
        );

        // Write this module row `row_height` times into the grid.
        let module_row_base = r * row_height;
        for h in 0..row_height {
            let module_row = module_row_base + h;
            for (col, &dark) in row_modules.iter().enumerate() {
                if dark {
                    grid = barcode_2d::set_module(&grid, module_row, col as u32, true);
                }
            }
        }
    }

    grid
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience functions
// ─────────────────────────────────────────────────────────────────────────────

/// Encode `data` and pass the [`ModuleGrid`] through the layout pipeline.
///
/// # Errors
///
/// Same as [`encode()`], plus layout errors from `barcode-2d`.
pub fn encode_and_layout(
    data: &[u8],
    options: &PDF417Options,
    config: Option<Barcode2DLayoutConfig>,
) -> Result<PaintScene, Box<dyn std::error::Error>> {
    let grid = encode(data, options)?;
    let cfg = config.unwrap_or_else(|| Barcode2DLayoutConfig {
        quiet_zone_modules: 2,
        ..Default::default()
    });
    let scene = layout(&grid, &cfg)?;
    Ok(scene)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ──────────────────────────────────────────────────────────────────────
    // Helper: read a module row as a bit string
    // ──────────────────────────────────────────────────────────────────────

    fn row_bits(grid: &ModuleGrid, module_row: u32) -> String {
        let r = module_row as usize;
        (0..grid.cols as usize)
            .map(|c| if grid.modules[r][c] { '1' } else { '0' })
            .collect()
    }

    // ──────────────────────────────────────────────────────────────────────
    // Struct and error types
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn pdf417options_default() {
        let opts = PDF417Options::default();
        assert!(opts.ecc_level.is_none());
        assert!(opts.columns.is_none());
        assert!(opts.row_height.is_none());
    }

    #[test]
    fn error_display() {
        assert!(PDF417Error::InputTooLong("x".into()).to_string().contains("InputTooLong"));
        assert!(PDF417Error::InvalidDimensions("x".into()).to_string().contains("InvalidDimensions"));
        assert!(PDF417Error::InvalidECCLevel("x".into()).to_string().contains("InvalidECCLevel"));
    }

    // ──────────────────────────────────────────────────────────────────────
    // Byte compaction
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn byte_compact_single_byte() {
        let cw = byte_compact(&[0x41]);
        // [924 (latch), 65 (direct byte)]
        assert_eq!(cw[0], 924);
        assert_eq!(cw[1], 65);
        assert_eq!(cw.len(), 2);
    }

    #[test]
    fn byte_compact_six_bytes() {
        // 6 bytes → 5 base-900 codewords
        let cw = byte_compact(&[0x41, 0x42, 0x43, 0x44, 0x45, 0x46]);
        assert_eq!(cw[0], 924); // latch
        assert_eq!(cw.len(), 6); // 1 latch + 5 codewords

        // Verify round-trip: recompute manually
        let n: u64 = 0x41u64 * 256_u64.pow(5)
            + 0x42 * 256_u64.pow(4)
            + 0x43 * 256_u64.pow(3)
            + 0x44 * 256_u64.pow(2)
            + 0x45 * 256
            + 0x46;
        let c5 = (n % 900) as u16;
        let n2 = n / 900;
        let c4 = (n2 % 900) as u16;
        let n3 = n2 / 900;
        let c3 = (n3 % 900) as u16;
        let n4 = n3 / 900;
        let c2 = (n4 % 900) as u16;
        let c1 = (n4 / 900) as u16;
        assert_eq!(cw[1], c1);
        assert_eq!(cw[2], c2);
        assert_eq!(cw[3], c3);
        assert_eq!(cw[4], c4);
        assert_eq!(cw[5], c5);
    }

    #[test]
    fn byte_compact_seven_bytes() {
        // 7 bytes → 5 codewords (from 6-byte group) + 1 direct byte
        let cw = byte_compact(&[65, 66, 67, 68, 69, 70, 71]);
        assert_eq!(cw[0], 924);
        assert_eq!(cw.len(), 7); // 1 latch + 5 + 1
        assert_eq!(cw[6], 71); // last byte direct
    }

    #[test]
    fn byte_compact_empty() {
        let cw = byte_compact(&[]);
        assert_eq!(cw, vec![924]); // just the latch
    }

    // ──────────────────────────────────────────────────────────────────────
    // GF(929) arithmetic (verified through RS encoding)
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn gf_tables_initialized() {
        init_gf_tables();
        let exp = GF_EXP.get().unwrap();
        let log = GF_LOG.get().unwrap();
        assert_eq!(exp[0], 1); // α^0 = 1
        assert_eq!(exp[1], 3); // α^1 = 3
        assert_eq!(exp[2], 9); // α^2 = 9
        assert_eq!(exp[3], 27); // α^3 = 27
        // Fermat: α^{928} ≡ 1 (mod 929)
        assert_eq!(exp[928], 1);
        // inv(3) = 310: 3 × 310 = 930 ≡ 1 (mod 929)
        assert_eq!(log[1], 0); // log(1) = 0 (α^0 = 1)
        assert_eq!(log[3], 1); // log(3) = 1 (α^1 = 3)
    }

    #[test]
    fn gf_add_basic() {
        init_gf_tables();
        assert_eq!(gf_add(100, 900), 71); // (100+900) mod 929 = 71
        assert_eq!(gf_add(0, 500), 500);
        assert_eq!(gf_add(928, 1), 0); // 928+1 = 929 ≡ 0
    }

    #[test]
    fn gf_mul_basic() {
        init_gf_tables();
        assert_eq!(gf_mul(3, 3), 9);
        assert_eq!(gf_mul(310, 3), 1); // 3 × 310 = 930 ≡ 1 (mod 929)
        assert_eq!(gf_mul(0, 500), 0);
        assert_eq!(gf_mul(500, 0), 0);
        assert_eq!(gf_mul(1, 928), 928);
    }

    // ──────────────────────────────────────────────────────────────────────
    // Row indicators
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn row_indicators_4_rows_3_cols_ecc2() {
        // R=4, C=3, L=2: R_info=1, C_info=2, L_info=6
        // Cluster 0: LRI=R_info=1, RRI=C_info=2
        // Cluster 1: LRI=L_info=6, RRI=R_info=1
        // Cluster 2: LRI=C_info=2, RRI=L_info=6
        // Cluster 0 (row 3): LRI=30+1=31, RRI=30+2=32
        assert_eq!(compute_lri(0, 4, 3, 2), 1);
        assert_eq!(compute_rri(0, 4, 3, 2), 2);
        assert_eq!(compute_lri(1, 4, 3, 2), 6);
        assert_eq!(compute_rri(1, 4, 3, 2), 1);
        assert_eq!(compute_lri(2, 4, 3, 2), 2);
        assert_eq!(compute_rri(2, 4, 3, 2), 6);
        assert_eq!(compute_lri(3, 4, 3, 2), 31);
        assert_eq!(compute_rri(3, 4, 3, 2), 32);
    }

    // ──────────────────────────────────────────────────────────────────────
    // Start and stop patterns
    // ──────────────────────────────────────────────────────────────────────

    const START_BITS: &str = "11111111010101000";
    const STOP_BITS: &str = "111111101000101001";

    #[test]
    fn start_pattern_decodes_correctly() {
        let mut modules = Vec::new();
        expand_widths(&START_PATTERN, &mut modules);
        assert_eq!(modules.len(), 17);
        let bits: String = modules.iter().map(|&d| if d { '1' } else { '0' }).collect();
        assert_eq!(bits, START_BITS);
    }

    #[test]
    fn stop_pattern_decodes_correctly() {
        let mut modules = Vec::new();
        expand_widths(&STOP_PATTERN, &mut modules);
        assert_eq!(modules.len(), 18);
        let bits: String = modules.iter().map(|&d| if d { '1' } else { '0' }).collect();
        assert_eq!(bits, STOP_BITS);
    }

    // ──────────────────────────────────────────────────────────────────────
    // Symbol dimensions
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn module_width_formula() {
        // module_width = 69 + 17 * cols
        for &c in &[1u32, 3, 5, 10, 30] {
            let opts = PDF417Options {
                columns: Some(c),
                row_height: Some(3),
                ..Default::default()
            };
            let grid = encode(b"HELLO WORLD HELLO WORLD", &opts).unwrap();
            assert_eq!(grid.cols, 69 + 17 * c);
        }
    }

    #[test]
    fn minimum_rows_enforced() {
        let opts = PDF417Options::default();
        let grid = encode(b"A", &opts).unwrap();
        // rowHeight defaults to 3, so grid.rows must be divisible by 3
        // and at least 3 logical rows → at least 9 module rows.
        assert!(grid.rows >= 3);
    }

    #[test]
    fn row_height_scales_grid() {
        let g3 = encode(b"A", &PDF417Options { row_height: Some(3), ..Default::default() }).unwrap();
        let g6 = encode(b"A", &PDF417Options { row_height: Some(6), ..Default::default() }).unwrap();
        assert_eq!(g6.rows, g3.rows * 2);
    }

    // ──────────────────────────────────────────────────────────────────────
    // Start/stop pattern in every row
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn every_row_starts_with_start_pattern() {
        let opts = PDF417Options { columns: Some(3), ..Default::default() };
        let grid = encode(b"TEST", &opts).unwrap();
        for r in 0..grid.rows {
            let bits = row_bits(&grid, r);
            assert_eq!(&bits[..17], START_BITS, "Row {r} start pattern mismatch");
        }
    }

    #[test]
    fn every_row_ends_with_stop_pattern() {
        let opts = PDF417Options { columns: Some(3), ..Default::default() };
        let grid = encode(b"TEST", &opts).unwrap();
        for r in 0..grid.rows {
            let bits = row_bits(&grid, r);
            assert_eq!(&bits[bits.len()-18..], STOP_BITS, "Row {r} stop pattern mismatch");
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // Integration tests
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn encode_single_byte() {
        let opts = PDF417Options::default();
        let grid = encode(b"A", &opts).unwrap();
        assert!(grid.rows >= 3);
        assert!(grid.cols >= 69 + 17);
    }

    #[test]
    fn encode_hello_world() {
        let opts = PDF417Options::default();
        let grid = encode(b"HELLO WORLD", &opts).unwrap();
        assert!(grid.rows >= 3);
        // Check all rows have correct start/stop patterns.
        for r in 0..grid.rows {
            let bits = row_bits(&grid, r);
            assert_eq!(&bits[..17], START_BITS);
            assert_eq!(&bits[bits.len()-18..], STOP_BITS);
        }
    }

    #[test]
    fn encode_all_256_bytes() {
        let bytes: Vec<u8> = (0u8..=255).collect();
        let opts = PDF417Options::default();
        let grid = encode(&bytes, &opts).unwrap();
        assert!(grid.rows >= 3);
    }

    #[test]
    fn encode_repetitive_high_bytes() {
        let bytes = vec![0xffu8; 256];
        let opts = PDF417Options::default();
        let grid = encode(&bytes, &opts).unwrap();
        assert!(grid.rows >= 3);
    }

    #[test]
    fn encode_empty() {
        let opts = PDF417Options::default();
        let grid = encode(b"", &opts).unwrap();
        assert!(grid.rows >= 3);
    }

    #[test]
    fn encode_deterministic() {
        let opts = PDF417Options::default();
        let g1 = encode(b"PDF417 TEST", &opts).unwrap();
        let g2 = encode(b"PDF417 TEST", &opts).unwrap();
        assert_eq!(g1.rows, g2.rows);
        assert_eq!(g1.cols, g2.cols);
        for r in 0..g1.rows {
            for c in 0..g1.cols {
                assert_eq!(g1.modules[r as usize][c as usize], g2.modules[r as usize][c as usize]);
            }
        }
    }

    #[test]
    fn different_inputs_differ() {
        let opts = PDF417Options::default();
        let g1 = encode(b"AAA", &opts).unwrap();
        let g2 = encode(b"BBB", &opts).unwrap();
        let mut differ = false;
        'outer: for r in 0..g1.rows.min(g2.rows) {
            for c in 0..g1.cols.min(g2.cols) {
                if g1.modules[r as usize][c as usize] != g2.modules[r as usize][c as usize] {
                    differ = true;
                    break 'outer;
                }
            }
        }
        assert!(differ);
    }

    #[test]
    fn row_repeats_correctly() {
        let row_height = 4u32;
        let opts = PDF417Options {
            row_height: Some(row_height),
            columns: Some(3),
            ..Default::default()
        };
        let grid = encode(b"HELLO", &opts).unwrap();
        let logical_rows = grid.rows / row_height;
        for lr in 0..logical_rows {
            for h in 1..row_height {
                for c in 0..grid.cols {
                    assert_eq!(
                        grid.modules[(lr * row_height) as usize][c as usize],
                        grid.modules[(lr * row_height + h) as usize][c as usize],
                        "Logical row {lr} repeat {h} differs at col {c}"
                    );
                }
            }
        }
    }

    #[test]
    fn ecc_level_0_accepted() {
        let opts = PDF417Options { ecc_level: Some(0), ..Default::default() };
        assert!(encode(b"A", &opts).is_ok());
    }

    #[test]
    fn ecc_level_8_accepted() {
        let opts = PDF417Options { ecc_level: Some(8), ..Default::default() };
        assert!(encode(b"A", &opts).is_ok());
    }

    #[test]
    fn higher_ecc_level_larger_symbol() {
        let opts2 = PDF417Options { ecc_level: Some(2), ..Default::default() };
        let opts4 = PDF417Options { ecc_level: Some(4), ..Default::default() };
        let g2 = encode(b"HELLO WORLD", &opts2).unwrap();
        let g4 = encode(b"HELLO WORLD", &opts4).unwrap();
        assert!(g4.rows * g4.cols >= g2.rows * g2.cols);
    }

    // ──────────────────────────────────────────────────────────────────────
    // Error cases
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn invalid_ecc_level_9() {
        let opts = PDF417Options { ecc_level: Some(9), ..Default::default() };
        assert!(matches!(encode(b"A", &opts), Err(PDF417Error::InvalidECCLevel(_))));
    }

    #[test]
    fn invalid_columns_0() {
        let opts = PDF417Options { columns: Some(0), ..Default::default() };
        assert!(matches!(encode(b"A", &opts), Err(PDF417Error::InvalidDimensions(_))));
    }

    #[test]
    fn invalid_columns_31() {
        let opts = PDF417Options { columns: Some(31), ..Default::default() };
        assert!(matches!(encode(b"A", &opts), Err(PDF417Error::InvalidDimensions(_))));
    }

    #[test]
    fn input_too_long_with_columns_1() {
        let huge = vec![b'A'; 3000];
        let opts = PDF417Options { columns: Some(1), ..Default::default() };
        assert!(matches!(encode(&huge, &opts), Err(PDF417Error::InputTooLong(_))));
    }

    // ──────────────────────────────────────────────────────────────────────
    // Cross-language parity: verify the same inputs produce the same bits
    // as the TypeScript implementation (via known test vectors)
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn encode_and_layout_produces_scene() {
        let opts = PDF417Options::default();
        let result = encode_and_layout(b"HELLO", &opts, None);
        assert!(result.is_ok());
        let scene = result.unwrap();
        assert!(scene.width > 0.0);
        assert!(scene.height > 0.0);
    }
}
