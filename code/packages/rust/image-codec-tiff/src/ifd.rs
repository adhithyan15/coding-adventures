// # ifd.rs — TIFF Image File Directory (IFD) Parser
//
// TIFF files are structured as a linked list of Image File Directories.
// Each IFD describes one image (or sub-image) in the file via typed
// key-value "tags". Understanding IFDs is the key to understanding TIFF.
//
// ## TIFF File Structure
//
// ```text
// Offset 0: "II" (little-endian) or "MM" (big-endian)
// Offset 2: 42 (magic number for classic TIFF)
// Offset 4: u32 — file offset of the first IFD
//
// At each IFD offset:
//   u16:  number of entries in this IFD
//   [12 bytes each]: IFD entries (tag, type, count, value_or_offset)
//   u32:  offset of next IFD (0 = end of chain)
// ```
//
// ## IFD Entry Layout (12 bytes)
//
// ```text
// Bytes  0–1:  Tag   (u16) — field identifier
// Bytes  2–3:  Type  (u16) — data type code (see TIFF types table below)
// Bytes  4–7:  Count (u32) — number of values of that type
// Bytes  8–11: Value or Offset
//               if (count * type_size) <= 4: value stored inline, left-justified
//               else: this is a file offset pointing to the actual data
// ```
//
// ## TIFF Data Types
//
// | Code | Name       | Size | Description                |
// |------|------------|------|----------------------------|
// | 1    | BYTE       |  1   | Unsigned 8-bit             |
// | 2    | ASCII      |  1   | 7-bit ASCII, NUL-terminated|
// | 3    | SHORT      |  2   | Unsigned 16-bit            |
// | 4    | LONG       |  4   | Unsigned 32-bit            |
// | 5    | RATIONAL   |  8   | Two u32: numerator/denom   |
// | 6    | SBYTE      |  1   | Signed 8-bit               |
// | 7    | UNDEFINED  |  1   | Raw bytes                  |
// | 8    | SSHORT     |  2   | Signed 16-bit              |
// | 9    | SLONG      |  4   | Signed 32-bit              |
// | 10   | SRATIONAL  |  8   | Two i32: numerator/denom   |
// | 11   | FLOAT      |  4   | IEEE 754 single            |
// | 12   | DOUBLE     |  8   | IEEE 754 double            |

use std::collections::HashMap;

// ─── Maximum safety limits ────────────────────────────────────────────────────

/// Maximum number of IFDs to parse. Prevents infinite loops on corrupted files.
/// A 256-page TIFF is already extremely unusual.
const MAX_IFD_COUNT: usize = 256;

/// Maximum number of entries per IFD. 65536 tags in one IFD is unreasonable.
const MAX_IFD_ENTRIES: usize = 65536;

// ─── IfdValue — typed tag value ───────────────────────────────────────────────

/// The decoded value of an IFD tag.
///
/// TIFF tags carry a type code alongside their data. We decode them into this
/// typed enum so callers can extract whatever they need without manual parsing.
///
/// Note: `Bytes` is used for both BYTE and UNDEFINED type codes.
#[derive(Debug, Clone, PartialEq)]
pub enum IfdValue {
    /// BYTE (1) or UNDEFINED (7) — raw bytes.
    Bytes(Vec<u8>),
    /// ASCII (2) — NUL-terminated string.
    Ascii(String),
    /// SHORT (3) — unsigned 16-bit integers.
    Shorts(Vec<u16>),
    /// LONG (4) — unsigned 32-bit integers.
    Longs(Vec<u32>),
    /// RATIONAL (5) — pairs of u32: (numerator, denominator).
    Rationals(Vec<(u32, u32)>),
    /// SBYTE (6) — signed 8-bit integers (stored as bytes for simplicity).
    SBytes(Vec<i8>),
    /// SSHORT (8) — signed 16-bit integers.
    SShorts(Vec<i16>),
    /// SLONG (9) — signed 32-bit integers.
    SLongs(Vec<i32>),
    /// SRATIONAL (10) — pairs of i32: (numerator, denominator).
    SRationals(Vec<(i32, i32)>),
    /// FLOAT (11) — IEEE 754 single-precision.
    Floats(Vec<f32>),
    /// DOUBLE (12) — IEEE 754 double-precision.
    Doubles(Vec<f64>),
}

// ─── Ifd — decoded Image File Directory ──────────────────────────────────────

/// A decoded Image File Directory (IFD).
///
/// Each IFD describes one image (or sub-image/thumbnail) in the TIFF file.
/// The fields here cover the baseline TIFF tags needed to decode pixel data.
/// Any unrecognised tags are collected in `extra_tags` for downstream codecs
/// like DNG, CR2, NEF — they carry camera-specific metadata in custom tags.
///
/// # Key fields for decoding
///
/// - `width`, `height`: pixel dimensions
/// - `bits_per_sample`: depth per channel (e.g., [8, 8, 8] for RGB, [16] for
///   16-bit grayscale, [12] for a 12-bit RAW)
/// - `compression`: 1=uncompressed, 5=LZW, 32773=PackBits
/// - `photometric`: colour space (1=grayscale, 2=RGB, 32803=CFA/Bayer)
/// - `strip_offsets` + `strip_byte_counts`: where compressed strips live
/// - `tile_offsets` + `tile_byte_counts`: if using tile layout instead
#[derive(Debug, Clone)]
pub struct Ifd {
    /// Image width in pixels (tag 256).
    pub width: u32,
    /// Image height in pixels (tag 257).
    pub height: u32,
    /// Bits per sample, one entry per channel (tag 258).
    /// E.g. [8, 8, 8] for 24-bit RGB, [16] for 16-bit gray, [12] for 12-bit CFA.
    pub bits_per_sample: Vec<u16>,
    /// Compression scheme (tag 259).
    /// 1=uncompressed, 5=LZW, 32773=PackBits, 7=JPEG.
    pub compression: u16,
    /// Photometric interpretation (tag 262).
    /// 0=WhiteIsZero, 1=BlackIsZero, 2=RGB, 32803=CFA (Bayer).
    pub photometric: u16,
    /// Number of channels per pixel (tag 277).
    /// 1=grayscale, 3=RGB, 4=RGBA or CMYK.
    pub samples_per_pixel: u16,
    /// Number of rows in each strip (tag 278).
    pub rows_per_strip: u32,
    /// File offsets to each strip's compressed bytes (tag 273).
    pub strip_offsets: Vec<u64>,
    /// Byte count of each compressed strip (tag 279).
    pub strip_byte_counts: Vec<u64>,
    /// Tile width in pixels (tag 322). None if not tile layout.
    pub tile_width: Option<u32>,
    /// Tile height in pixels (tag 323). None if not tile layout.
    pub tile_length: Option<u32>,
    /// File offsets to each tile (tag 324).
    pub tile_offsets: Vec<u64>,
    /// Byte count of each tile (tag 325).
    pub tile_byte_counts: Vec<u64>,
    /// Planar configuration (tag 284).
    /// 1=chunky (RGBRGB…), 2=planar (RRR…GGG…BBB…).
    pub planar_config: u16,
    /// Bayer/CFA pattern from tags 33421/33422.
    /// The 4 bytes describe a 2×2 pattern tile in row-major order.
    /// Values: 0=R, 1=G, 2=B.
    /// E.g. RGGB = [0, 1, 1, 2].
    pub cfa_pattern: Option<[u8; 4]>,
    /// Predictor tag (317). 1=none, 2=horizontal differencing.
    pub predictor: u16,
    /// All other tags, stored as raw decoded values for downstream use.
    /// DNG/CR2/NEF parsers mine these for camera-specific metadata.
    pub extra_tags: HashMap<u16, IfdValue>,
}

impl Default for Ifd {
    fn default() -> Self {
        Ifd {
            width: 0,
            height: 0,
            bits_per_sample: vec![1],
            compression: 1,
            photometric: 1,
            samples_per_pixel: 1,
            rows_per_strip: u32::MAX,
            strip_offsets: Vec::new(),
            strip_byte_counts: Vec::new(),
            tile_width: None,
            tile_length: None,
            tile_offsets: Vec::new(),
            tile_byte_counts: Vec::new(),
            planar_config: 1,
            cfa_pattern: None,
            predictor: 1,
            extra_tags: HashMap::new(),
        }
    }
}

// ─── byte-order-aware reader helpers ─────────────────────────────────────────

/// Read a u16 from `bytes[offset..]` respecting byte order.
///
/// TIFF can be either little-endian (Intel, "II") or big-endian (Motorola, "MM").
/// Every numeric read must go through one of these helpers.
///
/// Returns Err if there aren't enough bytes.
fn read_u16(bytes: &[u8], offset: usize, le: bool) -> Result<u16, String> {
    if offset + 2 > bytes.len() {
        return Err(format!("read_u16: offset {} out of bounds (len={})", offset, bytes.len()));
    }
    let b = &bytes[offset..offset + 2];
    if le {
        Ok(u16::from_le_bytes([b[0], b[1]]))
    } else {
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
}

/// Read a u32 from `bytes[offset..]` respecting byte order.
fn read_u32(bytes: &[u8], offset: usize, le: bool) -> Result<u32, String> {
    if offset + 4 > bytes.len() {
        return Err(format!("read_u32: offset {} out of bounds (len={})", offset, bytes.len()));
    }
    let b = &bytes[offset..offset + 4];
    if le {
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    } else {
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// Read an i32 from `bytes[offset..]` respecting byte order.
fn read_i32(bytes: &[u8], offset: usize, le: bool) -> Result<i32, String> {
    read_u32(bytes, offset, le).map(|v| v as i32)
}

/// Return the byte size of one value of the given TIFF type code.
///
/// This is used to decide whether a value fits inline in the 4-byte
/// value_or_offset field of an IFD entry.
fn type_size(type_code: u16) -> usize {
    match type_code {
        1 | 2 | 6 | 7 => 1, // BYTE, ASCII, SBYTE, UNDEFINED
        3 | 8 => 2,          // SHORT, SSHORT
        4 | 9 | 11 => 4,     // LONG, SLONG, FLOAT
        5 | 10 | 12 => 8,    // RATIONAL, SRATIONAL, DOUBLE (note: DOUBLE is actually 8 bytes)
        _ => 0,              // unknown — treat as 0 so nothing is read
    }
}

// ─── Value reader — decode raw bytes into IfdValue ────────────────────────────

/// Decode `count` values of TIFF type `type_code` from `data`.
///
/// `data` is the raw bytes — either inline from the IFD entry's value_or_offset
/// field, or fetched from the file at an offset.
///
/// Security note: we already bounds-checked that `data` has the right length
/// before calling this function.
fn decode_values(data: &[u8], type_code: u16, count: usize, le: bool) -> Result<IfdValue, String> {
    match type_code {
        1 | 7 => {
            // BYTE or UNDEFINED — raw bytes
            Ok(IfdValue::Bytes(data[..count].to_vec()))
        }
        2 => {
            // ASCII — NUL-terminated string(s)
            // Trim trailing NUL before converting to String
            let raw = &data[..count];
            let s = String::from_utf8_lossy(raw).trim_end_matches('\0').to_string();
            Ok(IfdValue::Ascii(s))
        }
        3 => {
            // SHORT — array of u16
            let mut v = Vec::with_capacity(count);
            for i in 0..count {
                v.push(read_u16(data, i * 2, le)?);
            }
            Ok(IfdValue::Shorts(v))
        }
        4 => {
            // LONG — array of u32
            let mut v = Vec::with_capacity(count);
            for i in 0..count {
                v.push(read_u32(data, i * 4, le)?);
            }
            Ok(IfdValue::Longs(v))
        }
        5 => {
            // RATIONAL — pairs of u32
            let mut v = Vec::with_capacity(count);
            for i in 0..count {
                let n = read_u32(data, i * 8, le)?;
                let d = read_u32(data, i * 8 + 4, le)?;
                v.push((n, d));
            }
            Ok(IfdValue::Rationals(v))
        }
        6 => {
            // SBYTE — signed bytes
            let v: Vec<i8> = data[..count].iter().map(|&b| b as i8).collect();
            Ok(IfdValue::SBytes(v))
        }
        8 => {
            // SSHORT — array of i16
            let mut v = Vec::with_capacity(count);
            for i in 0..count {
                v.push(read_u16(data, i * 2, le)? as i16);
            }
            Ok(IfdValue::SShorts(v))
        }
        9 => {
            // SLONG — array of i32
            let mut v = Vec::with_capacity(count);
            for i in 0..count {
                v.push(read_i32(data, i * 4, le)?);
            }
            Ok(IfdValue::SLongs(v))
        }
        10 => {
            // SRATIONAL — pairs of i32
            let mut v = Vec::with_capacity(count);
            for i in 0..count {
                let n = read_i32(data, i * 8, le)?;
                let d = read_i32(data, i * 8 + 4, le)?;
                v.push((n, d));
            }
            Ok(IfdValue::SRationals(v))
        }
        11 => {
            // FLOAT — IEEE 754 single
            let mut v = Vec::with_capacity(count);
            for i in 0..count {
                let bits = read_u32(data, i * 4, le)?;
                v.push(f32::from_bits(bits));
            }
            Ok(IfdValue::Floats(v))
        }
        12 => {
            // DOUBLE — IEEE 754 double
            let mut v = Vec::with_capacity(count);
            for i in 0..count {
                if i * 8 + 8 > data.len() {
                    return Err("DOUBLE read out of bounds".into());
                }
                let b = &data[i * 8..i * 8 + 8];
                let bits = if le {
                    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
                } else {
                    u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
                };
                v.push(f64::from_bits(bits));
            }
            Ok(IfdValue::Doubles(v))
        }
        _ => {
            // Unknown type — store raw bytes
            Ok(IfdValue::Bytes(data[..data.len().min(count)].to_vec()))
        }
    }
}

// ─── Helper: extract u64 from IfdValue ───────────────────────────────────────

/// Extract a u64 from a tag value that is SHORT or LONG.
/// Used for width/height/offsets/counts.
pub(crate) fn ifd_value_as_u64_slice(v: &IfdValue) -> Vec<u64> {
    match v {
        IfdValue::Shorts(s) => s.iter().map(|&x| x as u64).collect(),
        IfdValue::Longs(l) => l.iter().map(|&x| x as u64).collect(),
        IfdValue::Bytes(b) => b.iter().map(|&x| x as u64).collect(),
        _ => Vec::new(),
    }
}

/// Extract a single u32 from a SHORT or LONG tag value (first element).
pub(crate) fn ifd_value_as_u32(v: &IfdValue) -> Option<u32> {
    match v {
        IfdValue::Shorts(s) => s.first().map(|&x| x as u32),
        IfdValue::Longs(l) => l.first().copied(),
        IfdValue::Bytes(b) => b.first().map(|&x| x as u32),
        _ => None,
    }
}

// ─── parse_ifd_chain — the main public entry point ───────────────────────────

/// Parse all IFDs from a TIFF byte stream.
///
/// # TIFF byte-order detection
///
/// The first two bytes tell us the byte order for all subsequent reads:
/// - `"II"` (0x49 0x49) = little-endian (Intel byte order)
/// - `"MM"` (0x4D 0x4D) = big-endian (Motorola byte order)
///
/// After byte order, we validate the magic number (42), then follow the IFD
/// chain starting at bytes[4..8].
///
/// # Security
///
/// - At most `MAX_IFD_COUNT` IFDs are parsed.
/// - All offsets are validated against `bytes.len()` before dereferencing.
/// - Checked arithmetic prevents overflow in size calculations.
///
/// # Returns
///
/// `Ok(Vec<Ifd>)` — one `Ifd` per IFD in the file, in linked-list order.
/// The first IFD (index 0) is usually the full-resolution image.
pub fn parse_ifd_chain(bytes: &[u8]) -> Result<Vec<Ifd>, String> {
    // Need at least 8 bytes for the TIFF header.
    if bytes.len() < 8 {
        return Err(format!("TIFF: file too short ({} bytes), need at least 8", bytes.len()));
    }

    // ── Step 1: Detect byte order ──────────────────────────────────────────
    //
    // The first two bytes are either "II" (0x4949) for little-endian or
    // "MM" (0x4D4D) for big-endian. This byte order applies to ALL numeric
    // fields in the file.
    let le = match &bytes[0..2] {
        b"II" => true,  // Intel (little-endian)
        b"MM" => false, // Motorola (big-endian)
        other => {
            return Err(format!(
                "TIFF: invalid byte-order marker {:02x}{:02x} (expected 'II' or 'MM')",
                other[0], other[1]
            ));
        }
    };

    // ── Step 2: Validate magic number ─────────────────────────────────────
    //
    // The magic number for classic TIFF is 42. BigTIFF uses 43, which we
    // don't support — it uses 64-bit offsets and a different IFD structure.
    let magic = read_u16(bytes, 2, le)?;
    if magic != 42 {
        return Err(format!(
            "TIFF: invalid magic number {} (expected 42 for classic TIFF; 43=BigTIFF not supported)",
            magic
        ));
    }

    // ── Step 3: Find first IFD offset ─────────────────────────────────────
    let first_ifd_offset = read_u32(bytes, 4, le)? as usize;
    if first_ifd_offset == 0 {
        return Err("TIFF: IFD offset is 0 (no IFDs in file)".into());
    }
    if first_ifd_offset >= bytes.len() {
        return Err(format!(
            "TIFF: first IFD offset {} is beyond file end ({})",
            first_ifd_offset, bytes.len()
        ));
    }

    // ── Step 4: Walk the IFD chain ────────────────────────────────────────
    let mut ifds = Vec::new();
    let mut next_offset = first_ifd_offset;

    while next_offset != 0 {
        if ifds.len() >= MAX_IFD_COUNT {
            return Err(format!("TIFF: IFD chain too long (max {})", MAX_IFD_COUNT));
        }
        if next_offset + 2 > bytes.len() {
            return Err(format!(
                "TIFF: IFD offset {} is beyond file end ({})",
                next_offset, bytes.len()
            ));
        }

        let (ifd, next) = parse_one_ifd(bytes, next_offset, le)?;
        ifds.push(ifd);
        next_offset = next;
    }

    if ifds.is_empty() {
        return Err("TIFF: no valid IFDs found".into());
    }

    Ok(ifds)
}

// ─── parse_one_ifd — parse a single IFD at a given file offset ───────────────

/// Parse one IFD at `offset` within `bytes`.
///
/// Returns `(Ifd, next_ifd_offset)`. If `next_ifd_offset == 0`, this is the
/// last IFD in the chain.
///
/// ## IFD Entry Layout (12 bytes)
///
/// ```text
/// [0..2]  tag    — u16 field identifier
/// [2..4]  type   — u16 data type code
/// [4..8]  count  — u32 number of values
/// [8..12] value_or_offset
///           if count * type_size <= 4: value stored inline, left-justified
///           else: u32 file offset pointing to the actual data
/// ```
///
/// Left-justified means: for SHORT (2 bytes), the value occupies bytes 8-9
/// and bytes 10-11 are padding. For BYTE, it's just byte 8.
fn parse_one_ifd(bytes: &[u8], offset: usize, le: bool) -> Result<(Ifd, usize), String> {
    // Need 2 bytes for the entry count.
    if offset + 2 > bytes.len() {
        return Err(format!("TIFF: IFD at offset {} truncated (need entry count)", offset));
    }

    let entry_count = read_u16(bytes, offset, le)? as usize;
    if entry_count > MAX_IFD_ENTRIES {
        return Err(format!("TIFF: IFD has {} entries (max {})", entry_count, MAX_IFD_ENTRIES));
    }

    // The IFD body is: 2 (count) + 12*N (entries) + 4 (next IFD offset)
    let body_size = 2 + entry_count
        .checked_mul(12)
        .and_then(|n| n.checked_add(4))
        .ok_or("TIFF: IFD size overflow")?;

    if offset + body_size > bytes.len() {
        return Err(format!(
            "TIFF: IFD at offset {} is truncated (need {} bytes, have {})",
            offset,
            body_size,
            bytes.len() - offset
        ));
    }

    let mut ifd = Ifd::default();

    // Track whether we saw CFARepeatPatternDim and CFAPattern separately,
    // since we need both to populate ifd.cfa_pattern.
    let mut cfa_pattern_bytes: Option<Vec<u8>> = None;

    // ── Parse each 12-byte entry ──────────────────────────────────────────
    for i in 0..entry_count {
        let entry_offset = offset + 2 + i * 12;

        let tag = read_u16(bytes, entry_offset, le)?;
        let type_code = read_u16(bytes, entry_offset + 2, le)?;
        let count = read_u32(bytes, entry_offset + 4, le)? as usize;

        // Calculate total byte size for this entry's data.
        // Use checked arithmetic to catch malformed files.
        let elem_size = type_size(type_code);
        let total_size = if elem_size == 0 {
            // Unknown type — skip this tag gracefully.
            // We store raw bytes in extra_tags if possible.
            0
        } else {
            count.checked_mul(elem_size).ok_or_else(|| {
                format!("TIFF: tag {} value size overflow ({} × {})", tag, count, elem_size)
            })?
        };

        // Decide: is the value inline or at an offset?
        //
        // Rule: if total_size <= 4, the data is stored inline in the last 4
        // bytes of the IFD entry (bytes 8-11), left-justified.
        //
        // If total_size > 4, bytes 8-11 are a u32 file offset pointing to
        // the actual data somewhere else in the file.
        let data: &[u8] = if elem_size == 0 || total_size == 0 {
            // Unknown or empty — use the inline field but mark as zero-length
            &[]
        } else if total_size <= 4 {
            // Inline value — directly in the entry at bytes 8-11.
            &bytes[entry_offset + 8..entry_offset + 12]
        } else {
            // External value — bytes 8-11 are a file offset.
            let data_offset = read_u32(bytes, entry_offset + 8, le)? as usize;

            // Security: validate that the offset + size is within the file.
            if data_offset.checked_add(total_size).is_none_or(|end| end > bytes.len()) {
                return Err(format!(
                    "TIFF: tag {} data offset {} + size {} exceeds file length {}",
                    tag, data_offset, total_size, bytes.len()
                ));
            }
            &bytes[data_offset..data_offset + total_size]
        };

        // Decode the value bytes into a typed IfdValue.
        let value = if elem_size == 0 || total_size == 0 {
            IfdValue::Bytes(Vec::new())
        } else {
            decode_values(data, type_code, count, le)?
        };

        // ── Assign known tags to Ifd fields ───────────────────────────────
        //
        // Tag numbers from TIFF 6.0 spec Table 1 (Required Fields for Bilevel
        // Images), Table 2 (Required Fields for Gray-Scale Images), Table 3
        // (Required Fields for Palette-Color Images), and Table 4 (RGB Images).
        match tag {
            256 => {
                // ImageWidth — pixel columns
                if let Some(w) = ifd_value_as_u32(&value) {
                    ifd.width = w;
                }
            }
            257 => {
                // ImageLength — pixel rows
                if let Some(h) = ifd_value_as_u32(&value) {
                    ifd.height = h;
                }
            }
            258 => {
                // BitsPerSample — one value per channel.
                // For a grayscale image, this is [8] (single element).
                // For RGB, this is [8, 8, 8] (three elements).
                // For 12-bit RAW CFA, this might be [12].
                ifd.bits_per_sample = match &value {
                    IfdValue::Shorts(s) => s.clone(),
                    IfdValue::Bytes(b) => b.iter().map(|&x| x as u16).collect(),
                    _ => vec![8],
                };
            }
            259 => {
                // Compression — which codec to use for strip/tile data.
                if let Some(c) = ifd_value_as_u32(&value) {
                    ifd.compression = c as u16;
                }
            }
            262 => {
                // PhotometricInterpretation — how to interpret pixel values.
                if let Some(p) = ifd_value_as_u32(&value) {
                    ifd.photometric = p as u16;
                }
            }
            273 => {
                // StripOffsets — one offset per strip.
                // Can be SHORT or LONG.
                ifd.strip_offsets = ifd_value_as_u64_slice(&value);
            }
            277 => {
                // SamplesPerPixel — channels per pixel.
                if let Some(s) = ifd_value_as_u32(&value) {
                    ifd.samples_per_pixel = s as u16;
                }
            }
            278 => {
                // RowsPerStrip — rows in each strip.
                // 0xFFFFFFFF means "the whole image is one strip".
                if let Some(r) = ifd_value_as_u32(&value) {
                    ifd.rows_per_strip = r;
                }
            }
            279 => {
                // StripByteCounts — compressed byte count per strip.
                ifd.strip_byte_counts = ifd_value_as_u64_slice(&value);
            }
            284 => {
                // PlanarConfiguration — 1=chunky, 2=planar.
                if let Some(p) = ifd_value_as_u32(&value) {
                    ifd.planar_config = p as u16;
                }
            }
            317 => {
                // Predictor — used with LZW and Deflate.
                // 1=none, 2=horizontal differencing.
                if let Some(p) = ifd_value_as_u32(&value) {
                    ifd.predictor = p as u16;
                }
            }
            322 => {
                // TileWidth — tile width in pixels.
                if let Some(w) = ifd_value_as_u32(&value) {
                    ifd.tile_width = Some(w);
                }
            }
            323 => {
                // TileLength — tile height in pixels.
                if let Some(l) = ifd_value_as_u32(&value) {
                    ifd.tile_length = Some(l);
                }
            }
            324 => {
                // TileOffsets — file offset to each tile.
                ifd.tile_offsets = ifd_value_as_u64_slice(&value);
            }
            325 => {
                // TileByteCounts — compressed byte count per tile.
                ifd.tile_byte_counts = ifd_value_as_u64_slice(&value);
            }
            33421 => {
                // CFARepeatPatternDim — [rows, cols] of the CFA pattern tile.
                // Almost always [2, 2] for standard Bayer sensors.
                // We note this but rely on tag 33422 for the actual bytes.
                // (Already handled — no field for this, it's implied by cfa_pattern.)
            }
            33422 => {
                // CFAPattern — the raw CFA pattern bytes.
                // In TIFF/TIFF-EP, this is BYTE type with count = rows*cols.
                if let IfdValue::Bytes(b) = &value {
                    cfa_pattern_bytes = Some(b.clone());
                }
            }
            _ => {
                // Unknown or optional tag — store for downstream use.
                // DNG, CR2, NEF parsers will fish out their camera-specific tags here.
                ifd.extra_tags.insert(tag, value);
            }
        }
    }

    // ── Resolve CFA pattern ───────────────────────────────────────────────
    //
    // If we got 4+ bytes for CFAPattern, populate cfa_pattern.
    // RGGB is [0, 1, 1, 2] — the most common 35mm pattern.
    if let Some(pat) = cfa_pattern_bytes {
        if pat.len() >= 4 {
            ifd.cfa_pattern = Some([pat[0], pat[1], pat[2], pat[3]]);
        }
    }

    // ── Read next IFD offset ──────────────────────────────────────────────
    //
    // Immediately after the last 12-byte entry is a u32 pointer to the next
    // IFD. Zero means this is the last IFD.
    let next_ifd_ptr_offset = offset + 2 + entry_count * 12;
    let next_offset = read_u32(bytes, next_ifd_ptr_offset, le)? as usize;

    Ok((ifd, next_offset))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal LE TIFF header with one IFD having zero entries.
    /// Used as a base for more specific tests.
    fn minimal_tiff_le() -> Vec<u8> {
        let mut b = Vec::new();
        // Header: II + 42 + IFD offset (8)
        b.extend_from_slice(b"II");
        b.extend_from_slice(&42u16.to_le_bytes());
        b.extend_from_slice(&8u32.to_le_bytes());
        // IFD at offset 8: 0 entries, next=0
        b.extend_from_slice(&0u16.to_le_bytes()); // entry count = 0
        b.extend_from_slice(&0u32.to_le_bytes()); // next IFD = 0
        b
    }

    #[test]
    fn parse_minimal_le_tiff() {
        let bytes = minimal_tiff_le();
        let ifds = parse_ifd_chain(&bytes).unwrap();
        assert_eq!(ifds.len(), 1);
        // Default values from Ifd::default()
        assert_eq!(ifds[0].width, 0);
        assert_eq!(ifds[0].compression, 1);
    }

    #[test]
    fn parse_bad_magic_returns_err() {
        let mut bytes = minimal_tiff_le();
        // Corrupt the magic number (bytes 2-3).
        bytes[2] = 0xFF;
        bytes[3] = 0xFF;
        assert!(parse_ifd_chain(&bytes).is_err());
    }

    #[test]
    fn parse_bad_byte_order_returns_err() {
        let mut bytes = minimal_tiff_le();
        bytes[0] = b'X';
        bytes[1] = b'X';
        assert!(parse_ifd_chain(&bytes).is_err());
    }

    #[test]
    fn parse_too_short_returns_err() {
        assert!(parse_ifd_chain(&[0x49, 0x49, 0x2A]).is_err());
    }

    #[test]
    fn parse_be_tiff_header() {
        // Build a minimal big-endian TIFF.
        let mut b = Vec::new();
        b.extend_from_slice(b"MM");                     // big-endian
        b.extend_from_slice(&42u16.to_be_bytes());      // magic
        b.extend_from_slice(&8u32.to_be_bytes());       // IFD offset
        b.extend_from_slice(&0u16.to_be_bytes());       // 0 entries
        b.extend_from_slice(&0u32.to_be_bytes());       // no next IFD
        let ifds = parse_ifd_chain(&b).unwrap();
        assert_eq!(ifds.len(), 1);
    }

    #[test]
    fn parse_truncated_ifd_returns_err() {
        let mut bytes = minimal_tiff_le();
        // Claim 5 entries but only have 2 bytes of IFD body.
        // Set entry count = 5 at offset 8.
        bytes[8] = 5;
        bytes[9] = 0;
        assert!(parse_ifd_chain(&bytes).is_err());
    }

    #[test]
    fn type_size_known_types() {
        assert_eq!(type_size(1), 1);  // BYTE
        assert_eq!(type_size(3), 2);  // SHORT
        assert_eq!(type_size(4), 4);  // LONG
        assert_eq!(type_size(5), 8);  // RATIONAL
        assert_eq!(type_size(12), 8); // DOUBLE
        assert_eq!(type_size(99), 0); // unknown
    }
}
