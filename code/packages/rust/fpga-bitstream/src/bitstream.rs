//! iCE40 bitstream emitter (Project IceStorm record-stream format).
//!
//! ## Record structure
//!
//! Every record in the IceStorm bitstream follows this layout:
//!
//! ```text
//! Byte 0  total_len   = len(payload) + 2  (counts itself + command byte)
//! Byte 1  command     = one of CMD_* constants below
//! Byte 2… payload     = command-specific data
//! ```
//!
//! Example: `_cmd(0x05, &[0x12, 0x34])` → `[0x04, 0x05, 0x12, 0x34]`
//! because total_len = 2 (payload) + 2 = 4.
//!
//! ## Emitted record order
//!
//! 1. Preamble bytes: `0xFF 0x00` (not a record, just two raw bytes)
//! 2. `CMD_CRAM_RESET`  — reset the CRAM write address
//! 3. `CMD_CRAM_BANK 0` — select CRAM bank 0
//! 4. For each CLB (sorted by (row, col)):
//!    a. `CMD_CRAM_OFFSET (row, col)` — tile address
//!    b. `CMD_BRAM_DATA <cram_bytes zeros>` — stub configuration bits
//! 5. `CMD_CRC 0x0000` — placeholder CRC (real IceStorm verifies this)
//! 6. End marker: raw `0xFFFF` (not a record)
//!
//! ## Part dimensions
//!
//! | Part   | Rows | Cols | CRAM bits/tile |
//! |--------|------|------|----------------|
//! | HX1K   |  33  |  17  |    1024        |
//! | HX8K   |  33  |  33  |    1024        |
//! | UP5K   |  33  |  33  |    1024        |
//! | LP1K   |  33  |  17  |    1024        |

use std::collections::HashMap;
use std::path::Path;

// ---------------------------------------------------------------------------
// Command codes (IceStorm subset)
// ---------------------------------------------------------------------------

const CMD_CRAM_BANK:   u8 = 0x05;
const CMD_CRAM_OFFSET: u8 = 0x06;
const CMD_CRAM_RESET:  u8 = 0x07;
const CMD_BRAM_DATA:   u8 = 0x08;
const CMD_CRC:         u8 = 0x80;
#[allow(dead_code)]
const END_MARKER:     u16 = 0xFFFF;

// ---------------------------------------------------------------------------
// Ice40Part — supported parts
// ---------------------------------------------------------------------------

/// Supported iCE40 part codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ice40Part {
    Hx1k,
    Hx8k,
    Up5k,
    Lp1k,
}

/// Part dimensions: `(rows, cols, cram_bits_per_tile)`.
///
/// These match the Project IceStorm chip database for approximate accuracy;
/// exact tile dimensions differ by tile type in real hardware.
pub const PART_SPECS: &[(Ice40Part, u32, u32, u32)] = &[
    (Ice40Part::Hx1k, 33, 17, 1024),
    (Ice40Part::Hx8k, 33, 33, 1024),
    (Ice40Part::Up5k, 33, 33, 1024),
    (Ice40Part::Lp1k, 33, 17, 1024),
];

/// Look up `(rows, cols, cram_bits_per_tile)` for a part.
pub fn part_specs(part: Ice40Part) -> (u32, u32, u32) {
    PART_SPECS.iter()
        .find(|(p, ..)| *p == part)
        .map(|(_, rows, cols, bits)| (*rows, *cols, *bits))
        .expect("all Ice40Part variants are in PART_SPECS")
}

// ---------------------------------------------------------------------------
// FpgaConfig + ClbConfig
// ---------------------------------------------------------------------------

/// Per-tile CLB configuration.
///
/// `lut_a_truth_table` and `lut_b_truth_table` hold 16-entry 4-input LUT
/// truth tables (index = 4-bit input combo, value = 0 or 1).
#[derive(Debug, Clone)]
pub struct ClbConfig {
    pub lut_a_truth_table: Vec<u8>,
    pub lut_b_truth_table: Vec<u8>,
    pub ff_a_enabled:      bool,
    pub ff_b_enabled:      bool,
}

impl Default for ClbConfig {
    fn default() -> Self {
        Self {
            lut_a_truth_table: vec![0u8; 16],
            lut_b_truth_table: vec![0u8; 16],
            ff_a_enabled:      false,
            ff_b_enabled:      false,
        }
    }
}

/// The complete configuration for one FPGA image.
///
/// `clbs` maps `(row, col)` tile coordinates to their `ClbConfig`.
#[derive(Debug, Clone)]
pub struct FpgaConfig {
    pub part: Ice40Part,
    pub clbs: HashMap<(u32, u32), ClbConfig>,
}

impl FpgaConfig {
    pub fn new(part: Ice40Part) -> Self {
        Self { part, clbs: HashMap::new() }
    }
}

// ---------------------------------------------------------------------------
// BitstreamReport
// ---------------------------------------------------------------------------

/// Summary of what `emit_bitstream` produced.
#[derive(Debug, Clone)]
pub struct BitstreamReport {
    pub part:          Ice40Part,
    pub bytes_written: usize,
    pub clb_count:     usize,
    pub cram_size:     usize,
}

// ---------------------------------------------------------------------------
// emit_bitstream — the main emitter
// ---------------------------------------------------------------------------

/// Emit a structurally correct iCE40 record-stream bitstream.
///
/// The CRAM image is a stub (all zeros) in v0.1.0.  See the module-level
/// docs for a note on real-hardware limitations.
///
/// Returns `(bitstream_bytes, report)`.
pub fn emit_bitstream(config: &FpgaConfig) -> (Vec<u8>, BitstreamReport) {
    let (_, _, cram_bits) = part_specs(config.part);
    let cram_bytes = cram_bits.div_ceil(8) as usize;

    let mut out: Vec<u8> = Vec::new();

    // Preamble: two raw magic bytes (not a record)
    out.push(0xFF);
    out.push(0x00);

    // CRAM bank reset
    out.extend_from_slice(&cmd(CMD_CRAM_RESET, &[]));

    // CRAM bank 0 setup
    out.extend_from_slice(&cmd(CMD_CRAM_BANK, &[0u8]));

    // Per-CLB tile records — sorted by (row, col) for determinism
    let mut sorted_clbs: Vec<((u32, u32), &ClbConfig)> =
        config.clbs.iter().map(|(k, v)| (*k, v)).collect();
    sorted_clbs.sort_by_key(|((r, c), _)| (*r, *c));

    for ((row, col), _clb) in &sorted_clbs {
        // Tile address: big-endian u16 row, u16 col
        let offset_payload = vec![
            (*row >> 8) as u8,
            *row as u8,
            (*col >> 8) as u8,
            *col as u8,
        ];
        out.extend_from_slice(&cmd(CMD_CRAM_OFFSET, &offset_payload));

        // CRAM data: stub zeros of the correct byte count
        let cram_stub = vec![0u8; cram_bytes];
        out.extend_from_slice(&cmd(CMD_BRAM_DATA, &cram_stub));
    }

    // CRC placeholder (two zero bytes)
    out.extend_from_slice(&cmd(CMD_CRC, &[0u8, 0u8]));

    // End marker: raw 0xFFFF
    out.push(0xFF);
    out.push(0xFF);

    let report = BitstreamReport {
        part:          config.part,
        bytes_written: out.len(),
        clb_count:     config.clbs.len(),
        cram_size:     cram_bytes,
    };

    (out, report)
}

// ---------------------------------------------------------------------------
// write_bin
// ---------------------------------------------------------------------------

/// Write the emitted bitstream to a binary file.
///
/// # Errors
///
/// Returns `Err` if the file cannot be opened or written.
pub fn write_bin(path: &Path, config: &FpgaConfig) -> std::io::Result<BitstreamReport> {
    let (data, report) = emit_bitstream(config);
    std::fs::write(path, &data)?;
    Ok(report)
}

// ---------------------------------------------------------------------------
// Internal: record emitter
// ---------------------------------------------------------------------------

/// Build one command record.
///
/// # Panics
///
/// Panics when `payload.len() > 253` (record total would exceed 255 bytes).
pub fn cmd(command: u8, payload: &[u8]) -> Vec<u8> {
    assert!(
        payload.len() <= 253,
        "command payload too long: {} bytes (max 253)", payload.len()
    );
    let total_len = (payload.len() + 2) as u8;
    let mut rec = Vec::with_capacity(total_len as usize);
    rec.push(total_len);
    rec.push(command);
    rec.extend_from_slice(payload);
    rec
}
