//! # qr-decoder
//!
//! QR Code **decoder** — the inverse of `qr-code`'s encoder. Takes a
//! [`ModuleGrid`] (an already-located, already-sampled black/white module
//! grid, exactly as `qr_code::encode` produces one) and recovers the
//! original bytes.
//!
//! ## Scope: `ModuleGrid → Vec<u8>` only
//!
//! This crate does **not** locate a QR code inside a photograph or
//! screenshot — no pixel binarization, no finder-pattern scanning, no
//! perspective correction. That is real, separable follow-up work (see
//! `code/specs/MA04-qr-decoder.md` §6). The pipeline this crate implements
//! starts one step later than a real-world scanner would: someone has
//! already handed you a clean module grid.
//!
//! ```text
//! ModuleGrid  →  qr_decoder::decode  →  Vec<u8>
//!      ▲
//!      └── (deferred: locate a QR code within an arbitrary raster image)
//! ```
//!
//! ## Pipeline
//!
//! ```text
//! ModuleGrid
//!   → validate size (rows == cols == 4V+17 for some V in 1..=40)
//!   → qr_code::reserved_modules(V)               (function-pattern map)
//!   → read format info (Copy 1, then Copy 2)      → (ecc_level, mask)
//!   → read version info if V>=7 (Copy 1, then 2)  → must match V from size
//!   → unmask every non-reserved module            (XOR, self-inverse)
//!   → walk qr_code::data_module_order(V)          → raw bit stream
//!   → pack into codewords, discard remainder bits
//!   → de-interleave into per-block (data, ecc)     (mirrors compute_blocks)
//!   → reed_solomon::decode_with_base(_, ecc_len, 0) per block
//!   → concatenate corrected data codewords in block order
//!   → parse one data segment (numeric/alphanumeric/byte)
//!   → Vec<u8>
//! ```
//!
//! Every step reuses `qr-code`'s own tables/geometry/traversal (via its
//! `pub` surface added alongside this crate) rather than reimplementing
//! them, so encode and decode agree by construction, not by two
//! independently-written implementations that merely happen to match.
//!
//! ## Usage
//!
//! ```rust
//! use qr_code::{encode, EccLevel};
//! use qr_decoder::decode;
//!
//! let grid = encode("HELLO WORLD", EccLevel::M).unwrap();
//! let recovered = decode(&grid).unwrap();
//! assert_eq!(recovered, b"HELLO WORLD");
//! ```

#![forbid(unsafe_code)]

use barcode_2d::ModuleGrid;

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

/// Errors produced by [`decode`]. Every variant is structural — none carries
/// decoded content, matching this crate's general-purpose-library posture
/// (see `MA04-qr-decoder.md` §4.5): a caller might feed this a secret-shaped
/// payload (e.g. an `otpauth://` TOTP seed), and keeping errors content-free
/// costs nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrDecodeError {
    /// `rows != cols`, or the (square) size isn't `4V+17` for any `V` in
    /// `1..=40`.
    InvalidSize,
    /// Both format-info copies failed BCH validation.
    FormatInfoUnreadable,
    /// Version >= 7 and both version-info copies failed BCH validation, or
    /// the recovered version disagreed with the size-derived version.
    VersionInfoUnreadable,
    /// Reed-Solomon decoding exhausted its correction capacity for the
    /// block at this index (0-based, in block-construction order).
    UnrecoverableBlock(usize),
    /// The data segment's mode indicator was Kanji (`0b1000`) or ECI
    /// (`0b0111`) — or any other value `qr-code`'s own encoder never
    /// produces — carried here as the raw 4-bit value.
    UnsupportedMode(u8),
    /// The bit stream ran out before a character count, mode payload, or
    /// codeword read could complete.
    Truncated,
}

impl std::fmt::Display for QrDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QrDecodeError::InvalidSize => write!(f, "InvalidSize: grid is not a valid 4V+17 square for V in 1..=40"),
            QrDecodeError::FormatInfoUnreadable => write!(f, "FormatInfoUnreadable: both format-info copies failed BCH validation"),
            QrDecodeError::VersionInfoUnreadable => write!(f, "VersionInfoUnreadable: both version-info copies failed BCH validation, or disagreed with grid size"),
            QrDecodeError::UnrecoverableBlock(idx) => write!(f, "UnrecoverableBlock: Reed-Solomon block {idx} exceeded its correction capacity"),
            QrDecodeError::UnsupportedMode(mode) => write!(f, "UnsupportedMode: mode indicator {mode:#06b} is not numeric/alphanumeric/byte"),
            QrDecodeError::Truncated => write!(f, "Truncated: bit stream ended before the data segment could be fully read"),
        }
    }
}

impl std::error::Error for QrDecodeError {}

// ─────────────────────────────────────────────────────────────────────────────
// Bit reader — every read is bounds-checked (no panics on malformed input;
// `ModuleGrid` is publicly hand-constructible, so a caller could build one
// with garbage content, not only via `qr_code::encode`).
// ─────────────────────────────────────────────────────────────────────────────

struct BitReader<'a> {
    bits: &'a [bool],
    pos: usize,
}

impl<'a> BitReader<'a> {
    fn new(bits: &'a [bool]) -> Self {
        Self { bits, pos: 0 }
    }

    /// Read `n` bits MSB-first as a `u32`, or `None` if fewer than `n` bits
    /// remain. `n` must be <= 32 (every caller in this crate uses widths of
    /// at most 16, well within range).
    fn read_bits(&mut self, n: u32) -> Option<u32> {
        if self.pos + n as usize > self.bits.len() {
            return None;
        }
        let mut value = 0u32;
        for _ in 0..n {
            value = (value << 1) | (self.bits[self.pos] as u32);
            self.pos += 1;
        }
        Some(value)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Format/version info: exact grid positions, mirroring qr-code's own
// write_format_info/write_version_info (see their doc comments in
// qr-code/src/lib.rs, which this module's comments quote/derive from).
// ─────────────────────────────────────────────────────────────────────────────

/// Copy 1 format-info positions (around the top-left finder), ordered f14
/// (index 0) down to f0 (index 14) — matches `qr-code`'s own promoted
/// `read_format_info`'s documented `bits15` contract.
const FORMAT_COPY_1: [(usize, usize); 15] = [
    (8, 0), (8, 1), (8, 2), (8, 3), (8, 4), (8, 5), (8, 7), (8, 8),
    (7, 8), (5, 8), (4, 8), (3, 8), (2, 8), (1, 8), (0, 8),
];

/// Copy 2 format-info positions (around the top-right/bottom-left
/// finders), same f14..f0 ordering as `FORMAT_COPY_1`. Derived directly
/// from `write_format_info`'s own Copy 2 bit-assignment loops:
/// `modules[8][sz-1-i] = bit i` for `i in 0..=7`, `modules[sz-15+i][8] =
/// bit i` for `i in 8..=14` — inverted here into "position of bit (14-k)
/// at array index k".
fn format_copy_2(sz: usize) -> [(usize, usize); 15] {
    [
        (sz - 1, 8), (sz - 2, 8), (sz - 3, 8), (sz - 4, 8), (sz - 5, 8), (sz - 6, 8), (sz - 7, 8),
        (8, sz - 8), (8, sz - 7), (8, sz - 6), (8, sz - 5), (8, sz - 4), (8, sz - 3), (8, sz - 2), (8, sz - 1),
    ]
}

/// Read a raw 15-bit word off the grid at `positions`, MSB (index 0) first.
/// Bounds are the caller's responsibility to guarantee (all positions here
/// are derived from a validated `sz`, so they're always in range for a
/// grid whose dimensions were already checked against `sz`).
fn read_raw15(modules: &[Vec<bool>], positions: &[(usize, usize); 15]) -> u32 {
    let mut raw = 0u32;
    for (i, &(r, c)) in positions.iter().enumerate() {
        if modules[r][c] {
            raw |= 1 << (14 - i);
        }
    }
    raw
}

/// Read an 18-bit version-info word. `transposed = false` reads Copy 1
/// (`modules[a][b]`), `transposed = true` reads Copy 2 (`modules[b][a]`) —
/// mirroring `write_version_info`'s own `g.modules[a][b] = dark;
/// g.modules[b][a] = dark;` pair.
fn read_version18(modules: &[Vec<bool>], sz: usize, transposed: bool) -> u32 {
    let mut bits18 = 0u32;
    for i in 0u32..18 {
        let a = 5 - (i / 3) as usize;
        let b = sz - 9 - (i % 3) as usize;
        let dark = if transposed { modules[b][a] } else { modules[a][b] };
        if dark {
            bits18 |= 1 << i;
        }
    }
    bits18
}

// ─────────────────────────────────────────────────────────────────────────────
// Data segment modes — inverse of qr-code's encode_numeric/
// encode_alphanumeric/encode_byte_mode. Mirrors the private char_count_bits
// table exactly (mode x version tier), since qr-code doesn't expose it
// (only the encode-direction bit-writing functions needed it before now).
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Numeric,
    Alphanumeric,
    Byte,
}

fn char_count_bits(mode: Mode, version: usize) -> u32 {
    match mode {
        Mode::Numeric => if version <= 9 { 10 } else if version <= 26 { 12 } else { 14 },
        Mode::Alphanumeric => if version <= 9 { 9 } else if version <= 26 { 11 } else { 13 },
        Mode::Byte => if version <= 9 { 8 } else { 16 },
    }
}

const ALPHANUM_CHARS: &[u8; 45] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

fn decode_numeric(reader: &mut BitReader, version: usize) -> Result<Vec<u8>, QrDecodeError> {
    let count = reader
        .read_bits(char_count_bits(Mode::Numeric, version))
        .ok_or(QrDecodeError::Truncated)? as usize;
    let mut digits = Vec::with_capacity(count);
    let mut remaining = count;
    while remaining >= 3 {
        let v = reader.read_bits(10).ok_or(QrDecodeError::Truncated)?;
        if v > 999 {
            return Err(QrDecodeError::Truncated);
        }
        digits.push((v / 100) as u8 + b'0');
        digits.push(((v / 10) % 10) as u8 + b'0');
        digits.push((v % 10) as u8 + b'0');
        remaining -= 3;
    }
    if remaining == 2 {
        let v = reader.read_bits(7).ok_or(QrDecodeError::Truncated)?;
        if v > 99 {
            return Err(QrDecodeError::Truncated);
        }
        digits.push((v / 10) as u8 + b'0');
        digits.push((v % 10) as u8 + b'0');
    } else if remaining == 1 {
        let v = reader.read_bits(4).ok_or(QrDecodeError::Truncated)?;
        if v > 9 {
            return Err(QrDecodeError::Truncated);
        }
        digits.push(v as u8 + b'0');
    }
    Ok(digits)
}

fn decode_alphanumeric(reader: &mut BitReader, version: usize) -> Result<Vec<u8>, QrDecodeError> {
    let count = reader
        .read_bits(char_count_bits(Mode::Alphanumeric, version))
        .ok_or(QrDecodeError::Truncated)? as usize;
    let mut out = Vec::with_capacity(count);
    let mut remaining = count;
    while remaining >= 2 {
        let v = reader.read_bits(11).ok_or(QrDecodeError::Truncated)?;
        let first = (v / 45) as usize;
        let second = (v % 45) as usize;
        if first >= ALPHANUM_CHARS.len() || second >= ALPHANUM_CHARS.len() {
            return Err(QrDecodeError::Truncated);
        }
        out.push(ALPHANUM_CHARS[first]);
        out.push(ALPHANUM_CHARS[second]);
        remaining -= 2;
    }
    if remaining == 1 {
        let v = reader.read_bits(6).ok_or(QrDecodeError::Truncated)? as usize;
        if v >= ALPHANUM_CHARS.len() {
            return Err(QrDecodeError::Truncated);
        }
        out.push(ALPHANUM_CHARS[v]);
    }
    Ok(out)
}

fn decode_byte(reader: &mut BitReader, version: usize) -> Result<Vec<u8>, QrDecodeError> {
    let count = reader
        .read_bits(char_count_bits(Mode::Byte, version))
        .ok_or(QrDecodeError::Truncated)? as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let b = reader.read_bits(8).ok_or(QrDecodeError::Truncated)?;
        out.push(b as u8);
    }
    Ok(out)
}

/// Parse the one data segment `qr-code`'s own encoder can ever produce:
/// 4-bit mode indicator, mode-and-version-dependent character count, then
/// mode-specific payload. Anything after the segment (terminator, byte
/// padding) is not inspected — matching the encoder's own single-segment
/// behavior (MA04-qr-decoder.md §4.4).
fn parse_data_segment(data: &[u8], version: usize) -> Result<Vec<u8>, QrDecodeError> {
    let bits: Vec<bool> = data
        .iter()
        .flat_map(|&byte| (0..8).rev().map(move |i| (byte >> i) & 1 == 1))
        .collect();
    let mut reader = BitReader::new(&bits);

    let mode = reader.read_bits(4).ok_or(QrDecodeError::Truncated)?;
    match mode {
        0b0000 => Ok(Vec::new()), // valid empty-message terminator
        0b0001 => decode_numeric(&mut reader, version),
        0b0010 => decode_alphanumeric(&mut reader, version),
        0b0100 => decode_byte(&mut reader, version),
        other => Err(QrDecodeError::UnsupportedMode(other as u8)),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Decode a QR Code [`ModuleGrid`] — exactly as `qr_code::encode` produces
/// one (mask already applied, format/version info already written, no
/// quiet zone) — back into the original bytes.
///
/// # Errors
///
/// See [`QrDecodeError`] for every distinct failure mode.
///
/// # Example
///
/// ```rust
/// use qr_code::{encode, EccLevel};
/// use qr_decoder::decode;
///
/// let grid = encode("https://example.com", EccLevel::Q).unwrap();
/// assert_eq!(decode(&grid).unwrap(), b"https://example.com");
/// ```
pub fn decode(grid: &ModuleGrid) -> Result<Vec<u8>, QrDecodeError> {
    // [1] Validate size: rows == cols == 4V+17 for some V in 1..=40.
    let rows = grid.rows as usize;
    let cols = grid.cols as usize;
    if rows != cols {
        return Err(QrDecodeError::InvalidSize);
    }
    let sz = rows;
    let version = (1..=40usize)
        .find(|&v| qr_code::symbol_size(v) == sz)
        .ok_or(QrDecodeError::InvalidSize)?;
    // Defend against a hand-built ModuleGrid whose `modules` field doesn't
    // actually match its declared `rows`/`cols` — every later index into
    // `grid.modules` assumes this holds.
    if grid.modules.len() != sz || grid.modules.iter().any(|row| row.len() != sz) {
        return Err(QrDecodeError::InvalidSize);
    }

    // [2] Function-pattern map.
    let reserved = qr_code::reserved_modules(version);

    // [3] Format info: Copy 1, then Copy 2.
    let raw_fmt1 = read_raw15(&grid.modules, &FORMAT_COPY_1);
    let (ecc, mask) = qr_code::read_format_info(raw_fmt1)
        .or_else(|| {
            let copy2 = format_copy_2(sz);
            let raw_fmt2 = read_raw15(&grid.modules, &copy2);
            qr_code::read_format_info(raw_fmt2)
        })
        .ok_or(QrDecodeError::FormatInfoUnreadable)?;

    // [4] Version info (V >= 7 only): Copy 1, then Copy 2; must match V.
    if version >= 7 {
        let v1 = qr_code::read_version_info(read_version18(&grid.modules, sz, false));
        let recovered = v1.or_else(|| qr_code::read_version_info(read_version18(&grid.modules, sz, true)));
        match recovered {
            Some(v) if v == version => {}
            _ => return Err(QrDecodeError::VersionInfoUnreadable),
        }
    }

    // [5] + [6] Unmask and walk the traversal order, one bit per non-reserved
    // module, for exactly num_raw_data_modules(V) positions. (The traversal
    // can be longer than this for some versions >= 7 — a pre-existing
    // qr-code quirk where certain alignment-pattern placements are skipped;
    // see qr-code's own data_module_order_exceeds_formula_for_versions_with_
    // timing_overlap_alignment test. encode()'s place_bits only ever writes
    // real bits into the first num_raw_data_modules(V) slots and force-zeros
    // the rest, so taking only that many here keeps encode/decode in exact
    // agreement regardless.)
    let raw_bit_count = qr_code::num_raw_data_modules(version);
    let order = qr_code::data_module_order(version);
    if order.len() < raw_bit_count {
        return Err(QrDecodeError::Truncated);
    }
    let mut bits: Vec<bool> = Vec::with_capacity(raw_bit_count);
    for &(r, c) in order.iter().take(raw_bit_count) {
        if r >= sz || c >= sz || reserved.get(r).and_then(|row| row.get(c)).is_none() {
            return Err(QrDecodeError::Truncated);
        }
        let raw = grid.modules[r][c];
        bits.push(raw != qr_code::mask_condition(mask, r, c));
    }

    let remainder_bits = qr_code::num_remainder_bits(version);
    if remainder_bits > bits.len() {
        return Err(QrDecodeError::Truncated);
    }
    let data_bit_count = bits.len() - remainder_bits;
    let codeword_count = data_bit_count / 8;
    let mut codewords: Vec<u8> = Vec::with_capacity(codeword_count);
    for i in 0..codeword_count {
        let mut byte = 0u8;
        for b in 0..8 {
            byte = (byte << 1) | (bits[i * 8 + b] as u8);
        }
        codewords.push(byte);
    }

    // [7] De-interleave into per-block (data, ecc) sequences, using the same
    // block-size arithmetic as qr-code's own (private) compute_blocks.
    let total_data = qr_code::num_data_codewords(version, ecc);
    let total_blocks = qr_code::num_blocks(ecc, version);
    let ecc_len = qr_code::ecc_codewords_per_block(ecc, version);
    if total_blocks == 0 || ecc_len == 0 {
        return Err(QrDecodeError::Truncated);
    }
    let short_len = total_data / total_blocks;
    let num_long = total_data % total_blocks;
    let g1_count = total_blocks - num_long;
    let block_data_len = |b: usize| -> usize {
        if b < g1_count { short_len } else { short_len + 1 }
    };
    let max_data = if num_long > 0 { short_len + 1 } else { short_len };

    let expected_codewords = total_data + total_blocks * ecc_len;
    if codewords.len() < expected_codewords {
        return Err(QrDecodeError::Truncated);
    }

    let mut data_per_block: Vec<Vec<u8>> = (0..total_blocks).map(|_| Vec::new()).collect();
    let mut ecc_per_block: Vec<Vec<u8>> = (0..total_blocks).map(|_| Vec::new()).collect();
    let mut cursor = 0usize;
    for i in 0..max_data {
        for (b, block) in data_per_block.iter_mut().enumerate() {
            if i < block_data_len(b) {
                block.push(codewords[cursor]);
                cursor += 1;
            }
        }
    }
    for _ in 0..ecc_len {
        for block in ecc_per_block.iter_mut() {
            block.push(codewords[cursor]);
            cursor += 1;
        }
    }

    // [8] Per-block Reed-Solomon decode (b=0, QR's convention).
    let mut corrected_data: Vec<u8> = Vec::with_capacity(total_data);
    for (idx, (data, ecc_bytes)) in data_per_block.iter().zip(ecc_per_block.iter()).enumerate() {
        let mut received = data.clone();
        received.extend_from_slice(ecc_bytes);
        match reed_solomon::decode_with_base(&received, ecc_len, 0) {
            Ok(corrected) => corrected_data.extend_from_slice(&corrected),
            Err(_) => return Err(QrDecodeError::UnrecoverableBlock(idx)),
        }
    }

    // [9] + [10] Concatenate (already done above) and parse the one segment.
    parse_data_segment(&corrected_data, version)
}

#[cfg(test)]
mod tests;
