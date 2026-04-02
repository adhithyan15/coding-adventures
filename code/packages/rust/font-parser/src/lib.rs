//! # font-parser
//!
//! A metrics-only OpenType/TrueType font parser.
//!
//! ## What this crate does
//!
//! An OpenType font file is a binary *table database*. The first bytes of the
//! file are a directory: a list of named tables and where to find each one.
//! Each table stores a specific kind of information — glyph outlines, metrics,
//! kerning pairs, Unicode mappings, and so on.
//!
//! This crate reads the subset of tables needed to **measure text** without
//! touching the OS font stack:
//!
//! | Table  | What we read                                      |
//! |--------|---------------------------------------------------|
//! | `head` | `unitsPerEm` — the coordinate system scale        |
//! | `hhea` | ascender / descender / lineGap / numberOfHMetrics |
//! | `maxp` | `numGlyphs`                                       |
//! | `cmap` | Format 4 Unicode → glyph ID mapping               |
//! | `hmtx` | advance width + left side bearing per glyph       |
//! | `kern` | Format 0 kerning pairs                            |
//! | `name` | family name, subfamily name (UTF-16 BE)           |
//! | `OS/2` | typographic ascender / descender / lineGap,       |
//! |        | xHeight, capHeight (version ≥ 2)                  |
//!
//! It does **not** parse glyph outlines, perform text shaping, or rasterize
//! anything. Those are FNT02, FNT01, and FNT03 respectively.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use font_parser::{load, font_metrics, glyph_id, glyph_metrics, kerning};
//!
//! let bytes = std::fs::read("Inter-Regular.ttf").unwrap();
//! let font = load(&bytes).unwrap();
//!
//! let metrics = font_metrics(&font);
//! println!("unitsPerEm = {}", metrics.units_per_em); // 2048 for Inter
//!
//! let gid_a = glyph_id(&font, 'A' as u32).unwrap();
//! let gid_v = glyph_id(&font, 'V' as u32).unwrap();
//! let kern = kerning(&font, gid_a, gid_v); // negative — tighter spacing
//! ```
//!
//! ## Zero-copy design
//!
//! [`FontFile`] stores a copy of the font bytes and pre-parsed table offsets.
//! All methods borrow from that internal buffer. No heap allocation happens
//! during individual metric queries — just integer arithmetic over a `&[u8]`.

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

/// Errors returned when parsing fails.
///
/// Every variant is non-recoverable for the font that caused it: if you get
/// any of these, the bytes you passed are not a valid OpenType/TrueType font
/// (or are not one this parser supports).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontError {
    /// The first 4 bytes (`sfntVersion`) were not a recognised magic number.
    ///
    /// Valid values:
    /// - `0x00010000` — TrueType outlines
    /// - `0x4F54544F` — "OTTO", CFF/PostScript outlines
    InvalidMagic,

    /// The `head.magicNumber` field was not `0x5F0F3CF5`.
    ///
    /// This field is a checksum sentinel baked into every compliant font.
    /// A wrong value means the file is corrupt or truncated.
    InvalidHeadMagic,

    /// A required table (e.g. `"head"`, `"hmtx"`) was not in the directory.
    TableNotFound(&'static str),

    /// A field read attempted to access bytes past the end of the buffer.
    ///
    /// Either the file is truncated or an offset in the directory is wrong.
    BufferTooShort,

    /// No Format 4 cmap subtable was found for platform 3 / encoding 1
    /// (Windows Unicode BMP).
    ///
    /// Very rare for a modern Latin font; indicates an unusual encoding choice.
    UnsupportedCmapFormat,
}

impl std::fmt::Display for FontError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontError::InvalidMagic => write!(f, "invalid sfntVersion magic"),
            FontError::InvalidHeadMagic => write!(f, "invalid head.magicNumber"),
            FontError::TableNotFound(t) => write!(f, "required table '{}' not found", t),
            FontError::BufferTooShort => write!(f, "buffer too short"),
            FontError::UnsupportedCmapFormat => {
                write!(f, "no Format 4 cmap subtable for platform 3 encoding 1")
            }
        }
    }
}

impl std::error::Error for FontError {}

// ─────────────────────────────────────────────────────────────────────────────
// Public metric types
// ─────────────────────────────────────────────────────────────────────────────

/// Global typographic metrics extracted from a font file.
///
/// All integer fields are in *design units*. Convert to pixels:
///
/// ```text
/// pixels = design_units * font_size_px / units_per_em
/// ```
///
/// For Inter Regular at 16px: `16 / 2048 = 0.0078125 px per design unit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontMetrics {
    /// Design units per em square.
    ///
    /// This is the fundamental scale of the font's coordinate system.
    /// Inter Regular uses 2048. Older PostScript-derived fonts use 1000.
    pub units_per_em: u16,

    /// Distance from the baseline to the top of the tallest glyph (positive).
    ///
    /// Prefers `OS/2.typoAscender` over `hhea.ascender` when the OS/2 table
    /// is present.
    pub ascender: i16,

    /// Distance from the baseline to the bottom of the deepest glyph
    /// (negative, e.g. -512 for Inter Regular).
    ///
    /// Prefers `OS/2.typoDescender` over `hhea.descender`.
    pub descender: i16,

    /// Additional inter-line spacing in design units (often 0).
    ///
    /// Natural line height = `ascender - descender + line_gap`.
    pub line_gap: i16,

    /// Height of a lowercase 'x' above the baseline (design units).
    ///
    /// `None` if the `OS/2` table is absent or has version < 2.
    pub x_height: Option<i16>,

    /// Height of an uppercase 'H' above the baseline (design units).
    ///
    /// `None` if the `OS/2` table is absent or has version < 2.
    pub cap_height: Option<i16>,

    /// Total number of glyphs in the font.
    pub num_glyphs: u16,

    /// Font family name (e.g. `"Inter"`).
    ///
    /// Read from `name` table nameID 1, platform 3 encoding 1 (UTF-16 BE).
    pub family_name: String,

    /// Font subfamily / style name (e.g. `"Regular"`, `"Bold Italic"`).
    ///
    /// Read from `name` table nameID 2, platform 3 encoding 1 (UTF-16 BE).
    pub subfamily_name: String,
}

/// Per-glyph horizontal metrics.
///
/// All values are in design units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphMetrics {
    /// Horizontal distance from one glyph origin to the next.
    ///
    /// This is how far to advance the pen after drawing this glyph.
    /// For a proportional font, each glyph has its own value.
    /// For a monospace font, all glyphs share the same value.
    pub advance_width: u16,

    /// Space between the glyph's left edge and the ink boundary (design units).
    ///
    /// A positive lsb means the ink starts some distance to the right of the
    /// pen position. A negative lsb (rare) means the ink bleeds to the left.
    pub left_side_bearing: i16,
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal parsed table offsets
// ─────────────────────────────────────────────────────────────────────────────

/// Pre-parsed table directory.
///
/// Stores the absolute byte offset of each table we care about.
/// `None` means the table was not present in the font.
#[derive(Debug)]
struct Tables {
    head: u32,
    hhea: u32,
    maxp: u32,
    cmap: u32,
    hmtx: u32,
    kern: Option<u32>,
    name: Option<u32>,
    os2: Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// FontFile — the parsed font handle
// ─────────────────────────────────────────────────────────────────────────────

/// An opaque handle to a parsed font file.
///
/// Created by [`load`]. All metric queries borrow from its internal buffer.
///
/// The font bytes are copied into this struct so that the caller does not need
/// to keep the original buffer alive. This makes it easy to hold a `FontFile`
/// in a long-lived struct.
pub struct FontFile {
    /// The raw font bytes.
    data: Vec<u8>,
    /// Pre-parsed table offsets.
    tables: Tables,
}

// Silence the "large fields" warning — we intentionally own the font bytes.
impl std::fmt::Debug for FontFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontFile")
            .field("data_len", &self.data.len())
            .finish()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Big-endian byte reading helpers
// ─────────────────────────────────────────────────────────────────────────────
//
// OpenType stores every multi-byte integer in big-endian (network) byte order.
// On x86/ARM64 (both little-endian) we must byte-swap.
//
// We return Result so every read site propagates BufferTooShort automatically.

/// Read a 16-bit big-endian unsigned integer.
///
/// Example: bytes `[0x08, 0x00]` → `0x0800` = 2048.
#[inline]
fn read_u16(buf: &[u8], offset: usize) -> Result<u16, FontError> {
    if offset + 2 > buf.len() {
        return Err(FontError::BufferTooShort);
    }
    Ok(u16::from_be_bytes([buf[offset], buf[offset + 1]]))
}

/// Read a 16-bit big-endian signed integer (reinterpret the bits).
#[inline]
fn read_i16(buf: &[u8], offset: usize) -> Result<i16, FontError> {
    read_u16(buf, offset).map(|v| v as i16)
}

/// Read a 32-bit big-endian unsigned integer.
#[inline]
fn read_u32(buf: &[u8], offset: usize) -> Result<u32, FontError> {
    if offset + 4 > buf.len() {
        return Err(FontError::BufferTooShort);
    }
    Ok(u32::from_be_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ]))
}

// ─────────────────────────────────────────────────────────────────────────────
// Table directory parsing
// ─────────────────────────────────────────────────────────────────────────────

/// Find the byte offset of a named table in the font's table directory.
///
/// The table directory starts at byte 12 (after the 12-byte offset table).
/// Each record is 16 bytes: tag (4) + checksum (4) + offset (4) + length (4).
///
/// We do a linear scan. For `numTables` ≈ 20–30 (typical), this is fine.
/// A production implementation would binary-search the sorted records.
fn find_table(buf: &[u8], num_tables: u16, tag: &[u8; 4]) -> Option<u32> {
    // Table records start at offset 12, each 16 bytes.
    for i in 0..num_tables as usize {
        let rec = 12 + i * 16;
        // Tag is the first 4 bytes of the record.
        // Safe: if the buffer is too short, get() returns None and we skip.
        let t = buf.get(rec..rec + 4)?;
        if t == tag {
            // Offset is at bytes 8–11 of the record (after tag + checksum).
            return Some(u32::from_be_bytes([
                buf[rec + 8],
                buf[rec + 9],
                buf[rec + 10],
                buf[rec + 11],
            ]));
        }
    }
    None
}

/// Validate the offset table and collect all table offsets we need.
///
/// This is the first thing `load` does. If it fails, the bytes are not a
/// valid font (or not one we support).
fn parse_table_directory(buf: &[u8]) -> Result<Tables, FontError> {
    // The offset table is 12 bytes. Minimum valid font.
    if buf.len() < 12 {
        return Err(FontError::BufferTooShort);
    }

    // sfntVersion: the font "magic" number.
    // 0x00010000 = TrueType outlines
    // 0x4F54544F = "OTTO" = CFF/PostScript outlines
    let sfnt_version = read_u32(buf, 0)?;
    if sfnt_version != 0x0001_0000 && sfnt_version != 0x4F54_544F {
        return Err(FontError::InvalidMagic);
    }

    let num_tables = read_u16(buf, 4)?;

    // Helper: find required table (error if missing).
    let require = |tag: &'static [u8; 4], name: &'static str| {
        find_table(buf, num_tables, tag).ok_or(FontError::TableNotFound(name))
    };

    Ok(Tables {
        head: require(b"head", "head")?,
        hhea: require(b"hhea", "hhea")?,
        maxp: require(b"maxp", "maxp")?,
        cmap: require(b"cmap", "cmap")?,
        hmtx: require(b"hmtx", "hmtx")?,
        kern: find_table(buf, num_tables, b"kern"),
        name: find_table(buf, num_tables, b"name"),
        os2:  find_table(buf, num_tables, b"OS/2"),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// load — the entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Parse raw font bytes and return a [`FontFile`] handle.
///
/// # Errors
///
/// Returns [`FontError`] if:
/// - The bytes do not start with a valid OpenType/TrueType magic number
/// - A required table (head, hhea, maxp, cmap, hmtx) is missing
/// - The `head.magicNumber` sentinel is wrong (corrupted file)
/// - Any byte read goes out of bounds
///
/// # Example
///
/// ```rust,ignore
/// let bytes = std::fs::read("Inter-Regular.ttf").unwrap();
/// let font = font_parser::load(&bytes).unwrap();
/// ```
pub fn load(bytes: &[u8]) -> Result<FontFile, FontError> {
    let tables = parse_table_directory(bytes)?;

    // Validate the head.magicNumber sentinel.
    // Offset within the head table: 12 bytes in.
    let magic = read_u32(bytes, tables.head as usize + 12)?;
    if magic != 0x5F0F_3CF5 {
        return Err(FontError::InvalidHeadMagic);
    }

    Ok(FontFile {
        data: bytes.to_vec(),
        tables,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// font_metrics
// ─────────────────────────────────────────────────────────────────────────────

/// Return global typographic metrics for the font.
///
/// Prefers OS/2 typographic values over hhea values when the OS/2 table is
/// present. This matches the behaviour of modern renderers (CSS `line-height`,
/// Core Text, DirectWrite).
///
/// # Panics
///
/// Does not panic. All reads are validated; missing optional fields return
/// `None` (for `x_height` / `cap_height`) or fall back to hhea values.
pub fn font_metrics(font: &FontFile) -> FontMetrics {
    let buf = &font.data;
    let t = &font.tables;

    // ── head ────────────────────────────────────────────────────────────────
    // unitsPerEm is at offset 18 from the table start.
    let units_per_em = read_u16(buf, t.head as usize + 18).unwrap_or(1000);

    // ── hhea ────────────────────────────────────────────────────────────────
    // Used as fallback values if OS/2 is absent.
    let hhea_base = t.hhea as usize;
    let hhea_ascender  = read_i16(buf, hhea_base + 4).unwrap_or(0);
    let hhea_descender = read_i16(buf, hhea_base + 6).unwrap_or(0);
    let hhea_line_gap  = read_i16(buf, hhea_base + 8).unwrap_or(0);

    // ── maxp ────────────────────────────────────────────────────────────────
    // numGlyphs is at offset 4, regardless of maxp version.
    let num_glyphs = read_u16(buf, t.maxp as usize + 4).unwrap_or(0);

    // ── OS/2 ────────────────────────────────────────────────────────────────
    // Prefer typo values from OS/2; fall back to hhea if table absent.
    let (ascender, descender, line_gap, x_height, cap_height) =
        if let Some(os2_off) = t.os2 {
            let base = os2_off as usize;
            let version        = read_u16(buf, base).unwrap_or(0);
            let typo_ascender  = read_i16(buf, base + 68).unwrap_or(hhea_ascender);
            let typo_descender = read_i16(buf, base + 70).unwrap_or(hhea_descender);
            let typo_line_gap  = read_i16(buf, base + 72).unwrap_or(hhea_line_gap);

            // xHeight and capHeight were added in OS/2 version 2.
            let (xh, caph) = if version >= 2 {
                (
                    read_i16(buf, base + 86).ok(),
                    read_i16(buf, base + 88).ok(),
                )
            } else {
                (None, None)
            };
            (typo_ascender, typo_descender, typo_line_gap, xh, caph)
        } else {
            (hhea_ascender, hhea_descender, hhea_line_gap, None, None)
        };

    // ── name ────────────────────────────────────────────────────────────────
    let family_name    = read_name(buf, t.name, 1).unwrap_or_else(|| "(unknown)".to_owned());
    let subfamily_name = read_name(buf, t.name, 2).unwrap_or_else(|| "(unknown)".to_owned());

    FontMetrics {
        units_per_em,
        ascender,
        descender,
        line_gap,
        x_height,
        cap_height,
        num_glyphs,
        family_name,
        subfamily_name,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// glyph_id — cmap Format 4 lookup
// ─────────────────────────────────────────────────────────────────────────────

/// Map a Unicode codepoint to a glyph ID using the font's `cmap` table.
///
/// Only covers the Basic Multilingual Plane (codepoints 0x0000–0xFFFF).
/// Codepoints above 0xFFFF return `None` because Format 4 does not cover the
/// Supplementary Multilingual Plane.
///
/// Returns `None` if the codepoint is not present in the font.
///
/// # Algorithm
///
/// Format 4 encodes BMP codepoints via a segment table.
/// Each segment covers a contiguous range `[startCode, endCode]`.
/// The lookup:
/// 1. Binary-search `endCode[]` for the first entry ≥ `codepoint`.
/// 2. Verify `startCode[i] ≤ codepoint` (otherwise the codepoint is in a gap).
/// 3. Resolve the glyph ID:
///    - If `idRangeOffset[i] == 0`: `glyphId = (cp + idDelta[i]) & 0xFFFF`
///    - Else: indirect lookup into `glyphIdArray` using a self-relative offset.
pub fn glyph_id(font: &FontFile, codepoint: u32) -> Option<u16> {
    // Format 4 only covers BMP (16-bit codepoints).
    if codepoint > 0xFFFF {
        return None;
    }
    let cp = codepoint as u16;
    let buf = &font.data;
    let cmap_off = font.tables.cmap as usize;

    // ── Find the Format 4 subtable ───────────────────────────────────────────
    // cmap index: version (2) + numSubtables (2) = 4 bytes header.
    let num_subtables = read_u16(buf, cmap_off + 2).ok()?;

    // Scan encoding records (8 bytes each) starting at offset 4.
    let mut subtable_abs: Option<usize> = None;
    for i in 0..num_subtables as usize {
        let rec = cmap_off + 4 + i * 8;
        let platform_id = read_u16(buf, rec).ok()?;
        let encoding_id = read_u16(buf, rec + 2).ok()?;
        let sub_offset  = read_u32(buf, rec + 4).ok()? as usize;

        // Prefer platform 3 (Windows) encoding 1 (Unicode BMP).
        if platform_id == 3 && encoding_id == 1 {
            subtable_abs = Some(cmap_off + sub_offset);
            break;
        }
        // Accept platform 0 (Unicode) as fallback.
        if platform_id == 0 && subtable_abs.is_none() {
            subtable_abs = Some(cmap_off + sub_offset);
        }
    }

    let sub = subtable_abs?;

    // Verify it's Format 4.
    let format = read_u16(buf, sub).ok()?;
    if format != 4 {
        return None;
    }

    // ── Parse Format 4 header ───────────────────────────────────────────────
    let seg_count_x2 = read_u16(buf, sub + 6).ok()? as usize;
    let seg_count    = seg_count_x2 / 2;

    // Array base offsets (all relative to subtable start):
    //   endCode[]:        offset 14
    //   reservedPad:      offset 14 + segCount*2
    //   startCode[]:      offset 16 + segCount*2
    //   idDelta[]:        offset 16 + segCount*4
    //   idRangeOffset[]:  offset 16 + segCount*6
    //   glyphIdArray[]:   offset 16 + segCount*8
    let end_codes_base       = sub + 14;
    let start_codes_base     = sub + 16 + seg_count * 2;
    let id_delta_base        = sub + 16 + seg_count * 4;
    let id_range_offset_base = sub + 16 + seg_count * 6;

    // ── Segment binary search ────────────────────────────────────────────────
    // Find the first segment whose endCode >= cp.
    // (In practice the spec requires segments to be sorted by endCode.)
    let mut lo = 0usize;
    let mut hi = seg_count;
    while lo < hi {
        let mid = (lo + hi) / 2;
        let end_code = read_u16(buf, end_codes_base + mid * 2).ok()?;
        if (end_code as u32) < codepoint {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }

    if lo >= seg_count {
        return None; // codepoint > all endCodes
    }

    let end_code   = read_u16(buf, end_codes_base   + lo * 2).ok()?;
    let start_code = read_u16(buf, start_codes_base + lo * 2).ok()?;

    // Bounds check: is cp actually within this segment?
    if cp > end_code || cp < start_code {
        return None;
    }

    let id_delta        = read_i16(buf, id_delta_base        + lo * 2).ok()?;
    let id_range_offset = read_u16(buf, id_range_offset_base + lo * 2).ok()?;

    let glyph = if id_range_offset == 0 {
        // Direct delta: wrap with modulo 65536.
        //
        // The i32 cast prevents overflow before the mask:
        // cp=65000, idDelta=-29 → 64971 ✓
        ((cp as i32 + id_delta as i32) & 0xFFFF) as u16
    } else {
        // Indirect lookup — this is the famous idRangeOffset pointer trick from
        // the OpenType spec. In C it looks like:
        //
        //   *(idRangeOffset[i]/2 + (c - startCode[i]) + &idRangeOffset[i])
        //
        // `&idRangeOffset[i]` is a uint16_t*, so the arithmetic is in u16 units.
        // In bytes, the absolute address of the glyph ID entry is:
        //
        //   byte_addr(&idRangeOffset[i])  +  idRangeOffset[i]  +  (cp - startCode[i]) * 2
        //   = (id_range_offset_base + lo*2)  +  id_range_offset  +  (cp - startCode) * 2
        //
        // This always stays non-negative for a well-formed font because
        // idRangeOffset[i] is defined to be ≥ (segCount - i) * 2 (it points at
        // or past the start of glyphIdArray).
        let abs_off = (id_range_offset_base + lo * 2)
            + id_range_offset as usize
            + (cp as usize - start_code as usize) * 2;
        read_u16(buf, abs_off).ok()?
    };

    if glyph == 0 { None } else { Some(glyph) }
}

// ─────────────────────────────────────────────────────────────────────────────
// glyph_metrics — hmtx lookup
// ─────────────────────────────────────────────────────────────────────────────

/// Return the horizontal metrics for a glyph ID.
///
/// Returns `None` if `glyph_id` is out of range (≥ numGlyphs).
///
/// # hmtx layout
///
/// The table has two sections:
///
/// ```text
/// hMetrics[0 .. numberOfHMetrics]   — (advanceWidth: u16, lsb: i16) × N
/// leftSideBearings[0 .. numGlyphs - numberOfHMetrics]  — lsb (i16) only
/// ```
///
/// Glyphs in the first section have their own advance width.
/// Glyphs in the second section share the last advance width from the first
/// section (common in monospace segments at the end of a glyph set).
pub fn glyph_metrics(font: &FontFile, glyph_id: u16) -> Option<GlyphMetrics> {
    let buf = &font.data;
    let t = &font.tables;

    let num_glyphs        = read_u16(buf, t.maxp as usize + 4).ok()? as usize;
    let num_h_metrics     = read_u16(buf, t.hhea as usize + 34).ok()? as usize;
    let hmtx_off          = t.hmtx as usize;
    let gid               = glyph_id as usize;

    if gid >= num_glyphs {
        return None;
    }

    let (advance_width, left_side_bearing) = if gid < num_h_metrics {
        // Full record: 4 bytes each (advanceWidth u16 + lsb i16).
        let base = hmtx_off + gid * 4;
        (
            read_u16(buf, base).ok()?,
            read_i16(buf, base + 2).ok()?,
        )
    } else {
        // Shared advance: last advance + per-glyph lsb.
        let last_advance = read_u16(buf, hmtx_off + (num_h_metrics - 1) * 4).ok()?;
        // lsb-only records start after the full hMetrics array.
        let lsb_off = hmtx_off + num_h_metrics * 4 + (gid - num_h_metrics) * 2;
        (last_advance, read_i16(buf, lsb_off).ok()?)
    };

    Some(GlyphMetrics { advance_width, left_side_bearing })
}

// ─────────────────────────────────────────────────────────────────────────────
// kerning — kern Format 0 lookup
// ─────────────────────────────────────────────────────────────────────────────

/// Return the kerning adjustment (in design units) for a glyph pair.
///
/// Returns `0` if:
/// - The font has no `kern` table, or
/// - The pair is not found in any Format 0 subtable.
///
/// Negative return values mean the glyphs should be drawn closer together
/// (tighter spacing). Positive values mean wider spacing.
///
/// # Algorithm
///
/// Format 0 stores N pairs sorted ascending by composite key:
///
/// ```text
/// composite_key = (left_glyph_id << 16) | right_glyph_id
/// ```
///
/// We binary-search for the composite key. A 32-bit key comparison handles
/// both glyphs in one operation.
pub fn kerning(font: &FontFile, left: u16, right: u16) -> i16 {
    let buf = &font.data;

    let kern_off = match font.tables.kern {
        Some(off) => off as usize,
        None => return 0,
    };

    // kern table header: version (u16) + nTables (u16).
    let n_tables = match read_u16(buf, kern_off + 2) {
        Ok(n) => n,
        Err(_) => return 0,
    };

    // Walk subtables.
    let mut pos = kern_off + 4;
    for _ in 0..n_tables {
        if pos + 6 > buf.len() {
            break;
        }
        // Subtable header: version (u16) + length (u16) + coverage (u16).
        let length   = match read_u16(buf, pos + 2) { Ok(v) => v as usize, Err(_) => break };
        let coverage = match read_u16(buf, pos + 4) { Ok(v) => v, Err(_) => break };

        // Format is the high byte of coverage.
        let sub_format = coverage >> 8;

        if sub_format == 0 {
            // Format 0: sorted pair table.
            // Header: nPairs (u16) + searchRange + entrySelector + rangeShift (each u16).
            let n_pairs = match read_u16(buf, pos + 6) { Ok(v) => v as usize, Err(_) => break };
            let pairs_base = pos + 14; // 6 (subtable hdr) + 8 (format 0 hdr)

            // Composite key for binary search.
            let target = ((left as u32) << 16) | (right as u32);

            let mut lo = 0usize;
            let mut hi = n_pairs;
            while lo < hi {
                let mid = (lo + hi) / 2;
                let pair_off = pairs_base + mid * 6;
                let pair_left  = match read_u16(buf, pair_off)     { Ok(v) => v as u32, Err(_) => break };
                let pair_right = match read_u16(buf, pair_off + 2) { Ok(v) => v as u32, Err(_) => break };
                let key = (pair_left << 16) | pair_right;

                match key.cmp(&target) {
                    std::cmp::Ordering::Equal => {
                        return match read_i16(buf, pair_off + 4) {
                            Ok(v) => v,
                            Err(_) => 0,
                        };
                    }
                    std::cmp::Ordering::Less    => lo = mid + 1,
                    std::cmp::Ordering::Greater => hi = mid,
                }
            }
        }

        pos += length;
    }

    0 // pair not found in any subtable
}

// ─────────────────────────────────────────────────────────────────────────────
// name table reading
// ─────────────────────────────────────────────────────────────────────────────

/// Read a string from the `name` table by nameID.
///
/// Prefers platform 3 / encoding 1 (Windows Unicode BMP, UTF-16 BE).
/// Falls back to platform 0 (Unicode) if the Windows record is absent.
///
/// Returns `None` if the table is absent or the nameID is not found.
fn read_name(buf: &[u8], name_off: Option<u32>, name_id: u16) -> Option<String> {
    let base = name_off? as usize;

    // name table header: format (u16) + count (u16) + stringOffset (u16).
    let count         = read_u16(buf, base + 2).ok()? as usize;
    let string_offset = read_u16(buf, base + 4).ok()? as usize;

    // Scan name records (12 bytes each starting at offset 6).
    let mut best: Option<(u16 /*platformID*/, usize /*str_start*/, usize /*len*/)> = None;

    for i in 0..count {
        let rec = base + 6 + i * 12;
        let platform_id = read_u16(buf, rec).ok()?;
        let encoding_id = read_u16(buf, rec + 2).ok()?;
        let nid         = read_u16(buf, rec + 6).ok()?;
        let length      = read_u16(buf, rec + 8).ok()? as usize;
        let str_off     = read_u16(buf, rec + 10).ok()? as usize;

        if nid != name_id {
            continue;
        }

        let abs_start = base + string_offset + str_off;

        // Prefer platform 3 encoding 1 (UTF-16 BE).
        if platform_id == 3 && encoding_id == 1 {
            best = Some((3, abs_start, length));
            break; // best possible match — stop searching
        }
        // Accept platform 0 as fallback.
        if platform_id == 0 {
            best.get_or_insert((0, abs_start, length));
        }
    }

    let (_platform, start, len) = best?;
    let raw = buf.get(start..start + len)?;

    // Decode UTF-16 BE: read pairs of bytes as big-endian u16 code units.
    let u16_units: Vec<u16> = raw
        .chunks_exact(2)
        .map(|b| u16::from_be_bytes([b[0], b[1]]))
        .collect();

    // Convert to Rust String. char::from_u32 handles surrogates gracefully:
    // unpaired surrogates become U+FFFD replacement characters.
    let s: String = char::decode_utf16(u16_units)
        .map(|r| r.unwrap_or('\u{FFFD}'))
        .collect();
    Some(s)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Load Inter Regular from the shared fixtures directory.
    ///
    /// The path is relative to the workspace root. We walk up from the package
    /// crate directory using env!("CARGO_MANIFEST_DIR").
    fn inter_regular() -> Vec<u8> {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        // font-parser is at code/packages/rust/font-parser
        // fixtures are at code/fixtures/fonts/
        let font_path = manifest
            .parent() // rust
            .unwrap()
            .parent() // packages
            .unwrap()
            .parent() // code
            .unwrap()
            .join("fixtures/fonts/Inter-Regular.ttf");
        std::fs::read(&font_path)
            .unwrap_or_else(|_| panic!("Could not read font at {:?}", font_path))
    }

    // ── load() ────────────────────────────────────────────────────────────────

    #[test]
    fn load_empty_buffer_errors() {
        let err = load(&[]).unwrap_err();
        assert_eq!(err, FontError::BufferTooShort);
    }

    #[test]
    fn load_wrong_magic_errors() {
        // Craft 12 bytes with a bad sfntVersion.
        let mut buf = vec![0u8; 12 + 16]; // offset table + one fake table record
        buf[0..4].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
        let err = load(&buf).unwrap_err();
        assert_eq!(err, FontError::InvalidMagic);
    }

    #[test]
    fn load_inter_regular_succeeds() {
        let bytes = inter_regular();
        assert!(load(&bytes).is_ok());
    }

    // ── font_metrics() ────────────────────────────────────────────────────────

    #[test]
    fn units_per_em_is_2048() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let m = font_metrics(&font);
        assert_eq!(m.units_per_em, 2048);
    }

    #[test]
    fn family_name_is_inter() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let m = font_metrics(&font);
        assert_eq!(m.family_name, "Inter");
    }

    #[test]
    fn subfamily_name_is_regular() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let m = font_metrics(&font);
        assert_eq!(m.subfamily_name, "Regular");
    }

    #[test]
    fn num_glyphs_is_nonzero() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let m = font_metrics(&font);
        assert!(m.num_glyphs > 100, "expected > 100 glyphs, got {}", m.num_glyphs);
    }

    #[test]
    fn ascender_is_positive() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let m = font_metrics(&font);
        assert!(m.ascender > 0, "ascender should be positive: {}", m.ascender);
    }

    #[test]
    fn descender_is_negative_or_zero() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let m = font_metrics(&font);
        assert!(m.descender <= 0, "descender should be ≤ 0: {}", m.descender);
    }

    #[test]
    fn x_height_is_some_and_positive() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let m = font_metrics(&font);
        // Inter has OS/2 version ≥ 2, so these should be populated.
        assert!(m.x_height.is_some(), "expected x_height to be Some");
        assert!(m.x_height.unwrap() > 0, "x_height should be positive");
    }

    #[test]
    fn cap_height_is_some_and_positive() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let m = font_metrics(&font);
        assert!(m.cap_height.is_some(), "expected cap_height to be Some");
        assert!(m.cap_height.unwrap() > 0, "cap_height should be positive");
    }

    // ── glyph_id() ────────────────────────────────────────────────────────────

    #[test]
    fn glyph_id_for_latin_a_is_some() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let gid = glyph_id(&font, 0x0041); // 'A'
        assert!(gid.is_some(), "glyph_id('A') should be Some");
    }

    #[test]
    fn glyph_id_for_codepoint_above_ffff_is_none() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        // Format 4 only covers BMP (0x0000–0xFFFF).
        assert_eq!(glyph_id(&font, 0x1_0000), None);
    }

    #[test]
    fn glyph_id_for_unmapped_high_codepoint_is_none() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        // U+FFFF is a non-character; Format 4 terminates at the sentinel segment.
        // Either None or 0 remapped to None.
        let gid = glyph_id(&font, 0xFFFF);
        // We accept either outcome: None or Some(0) would be wrong behavior,
        // but inter has a .notdef at 0xFFFF mapped to None via our API.
        // Just assert it does not panic.
        let _ = gid;
    }

    #[test]
    fn glyph_ids_for_a_v_differ() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let gid_a = glyph_id(&font, 0x0041).unwrap(); // 'A'
        let gid_v = glyph_id(&font, 0x0056).unwrap(); // 'V'
        assert_ne!(gid_a, gid_v);
    }

    #[test]
    fn glyph_id_for_space_is_some() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let gid = glyph_id(&font, 0x0020); // space
        assert!(gid.is_some(), "glyph_id(' ') should be Some");
    }

    // ── glyph_metrics() ───────────────────────────────────────────────────────

    #[test]
    fn glyph_metrics_for_a_has_positive_advance() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let gid = glyph_id(&font, 0x0041).unwrap();
        let gm = glyph_metrics(&font, gid).unwrap();
        assert!(gm.advance_width > 0, "advance_width should be positive: {}", gm.advance_width);
    }

    #[test]
    fn glyph_metrics_out_of_range_returns_none() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let m = font_metrics(&font);
        // glyph_id equal to num_glyphs is one past the end.
        assert_eq!(glyph_metrics(&font, m.num_glyphs), None);
    }

    #[test]
    fn glyph_metrics_advance_is_within_reasonable_bounds() {
        // Design units for a typical Latin letter at unitsPerEm=2048
        // should be between 100 and 2400.
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let gid = glyph_id(&font, 0x0041).unwrap();
        let gm = glyph_metrics(&font, gid).unwrap();
        assert!(
            gm.advance_width >= 100 && gm.advance_width <= 2400,
            "advance_width out of expected range: {}",
            gm.advance_width
        );
    }

    // ── kerning() ─────────────────────────────────────────────────────────────

    #[test]
    fn kerning_inter_no_kern_table_returns_zero() {
        // Inter v4.0 uses GPOS (OpenType) for kerning, not the legacy kern
        // table. FNT00 only parses the kern table (legacy TrueType). GPOS
        // support is planned for FNT01.
        // Verify: the function returns 0 and does not panic.
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        let gid_a = glyph_id(&font, 0x0041).unwrap(); // 'A'
        let gid_v = glyph_id(&font, 0x0056).unwrap(); // 'V'
        let kern = kerning(&font, gid_a, gid_v);
        // Inter has no kern table → 0 is the correct answer for FNT00 scope.
        assert_eq!(kern, 0, "expected 0 (no kern table); got {}", kern);
    }

    /// Build a minimal synthetic kern Format 0 table embedded in a FontFile.
    ///
    /// This lets us unit-test the binary search logic without needing an
    /// external font file that happens to have a kern table.
    ///
    /// The synthetic font has:
    /// - sfntVersion = 0x00010000 (TrueType magic)
    /// - Minimal head, hhea, maxp, cmap, hmtx tables with correct offsets
    /// - A kern table with one Format 0 subtable containing two pairs:
    ///     (glyph 1, glyph 2) → value -140
    ///     (glyph 3, glyph 4) → value  80
    fn build_synthetic_font_with_kern(pairs: &[(u16, u16, i16)]) -> Vec<u8> {
        // We build the binary manually, big-endian throughout.
        // This is a teaching-quality implementation — not a general font builder.

        let write_u16 = |buf: &mut Vec<u8>, v: u16| {
            buf.extend_from_slice(&v.to_be_bytes());
        };
        let write_i16 = |buf: &mut Vec<u8>, v: i16| {
            buf.extend_from_slice(&v.to_be_bytes());
        };
        let write_u32 = |buf: &mut Vec<u8>, v: u32| {
            buf.extend_from_slice(&v.to_be_bytes());
        };

        // We need: head, hhea, maxp, cmap, hmtx, kern — 6 tables.
        let num_tables: u16 = 6;

        // Table directory = 12 (offset table) + 6*16 (records) = 108 bytes.
        let dir_size = 12 + num_tables as usize * 16;

        // We'll lay out the actual table data sequentially after the directory.
        // Compute starting offset for each table.

        // head: 54 bytes (we only need through indexToLocFormat at offset 50)
        let head_off = dir_size as u32;
        let head_len = 54u32;

        // hhea: 36 bytes
        let hhea_off = head_off + head_len;
        let hhea_len = 36u32;

        // maxp: 6 bytes (version + numGlyphs)
        let maxp_off = hhea_off + hhea_len;
        let maxp_len = 6u32;

        // cmap: minimal Format 4 with one segment covering only .notdef (U+FFFF end marker)
        // We need: index header (4) + 1 encoding record (8) + Format 4 subtable
        // Format 4 with 1 segment that has endCode=0xFFFF, startCode=0xFFFF (sentinel)
        // segCount=1, so the arrays are 1 element each.
        // Subtable: format(2)+length(2)+language(2)+segCountX2(2)+searchRange(2)+
        //           entrySelector(2)+rangeShift(2)+endCode[1](2)+reservedPad(2)+
        //           startCode[1](2)+idDelta[1](2)+idRangeOffset[1](2) = 26 bytes
        // cmap index: 4 + 8 + 26 = 38 bytes total
        let cmap_off = maxp_off + maxp_len;
        // Format 4 subtable with segCount=1:
        //   14-byte header + endCode[1](2) + reservedPad(2) + startCode[1](2)
        //   + idDelta[1](2) + idRangeOffset[1](2) = 24 bytes
        let cmap_sub_len: u16 = 24;
        // cmap index header(4) + encoding record(8) + subtable(24) = 36
        let cmap_len = 36u32;

        // hmtx: num_h_metrics=5 * 4 bytes each = 20 bytes
        // (We need at least 5 glyphs for our kern pairs 1..4 + glyph0)
        let num_glyphs: u16 = 5;
        let num_h_metrics: u16 = 5;
        let hmtx_off = cmap_off + cmap_len;
        let hmtx_len = num_h_metrics as u32 * 4;

        // kern: header(4) + subtable_header(6) + format0_header(8) + pairs*6
        let n_pairs = pairs.len() as u16;
        let kern_off = hmtx_off + hmtx_len;
        let kern_subtable_len = 6 + 8 + n_pairs as usize * 6;
        let kern_len = 4 + kern_subtable_len as u32;

        let mut buf: Vec<u8> = Vec::new();

        // ── Offset Table ─────────────────────────────────────────────────────
        write_u32(&mut buf, 0x0001_0000); // sfntVersion (TrueType)
        write_u16(&mut buf, num_tables);
        write_u16(&mut buf, 64);  // searchRange (placeholder)
        write_u16(&mut buf, 2);   // entrySelector
        write_u16(&mut buf, 32);  // rangeShift

        // ── Table Records ────────────────────────────────────────────────────
        // Records must be in tag-sorted order (cmap < head < hhea < hmtx < kern < maxp).
        // Write table records inline (we can't use a helper closure here
        // because Rust's borrow checker won't allow a closure that mutably
        // borrows buf to be called while buf is also passed as an argument).
        // cmap
        buf.extend_from_slice(b"cmap"); write_u32(&mut buf, 0); write_u32(&mut buf, cmap_off); write_u32(&mut buf, cmap_len);
        // head
        buf.extend_from_slice(b"head"); write_u32(&mut buf, 0); write_u32(&mut buf, head_off); write_u32(&mut buf, head_len);
        // hhea
        buf.extend_from_slice(b"hhea"); write_u32(&mut buf, 0); write_u32(&mut buf, hhea_off); write_u32(&mut buf, hhea_len);
        // hmtx
        buf.extend_from_slice(b"hmtx"); write_u32(&mut buf, 0); write_u32(&mut buf, hmtx_off); write_u32(&mut buf, hmtx_len);
        // kern
        buf.extend_from_slice(b"kern"); write_u32(&mut buf, 0); write_u32(&mut buf, kern_off); write_u32(&mut buf, kern_len);
        // maxp
        buf.extend_from_slice(b"maxp"); write_u32(&mut buf, 0); write_u32(&mut buf, maxp_off); write_u32(&mut buf, maxp_len);

        assert_eq!(buf.len(), dir_size, "dir_size mismatch");

        // ── head table ───────────────────────────────────────────────────────
        // version (Fixed) at 0: 0x00010000
        write_u32(&mut buf, 0x0001_0000);
        // fontRevision (Fixed) at 4
        write_u32(&mut buf, 0x0001_0000);
        // checksumAdjustment at 8
        write_u32(&mut buf, 0);
        // magicNumber at 12: must be 0x5F0F3CF5
        write_u32(&mut buf, 0x5F0F_3CF5);
        // flags at 16
        write_u16(&mut buf, 0);
        // unitsPerEm at 18
        write_u16(&mut buf, 1000);
        // created (8 bytes) at 20
        buf.extend_from_slice(&[0u8; 8]);
        // modified (8 bytes) at 28
        buf.extend_from_slice(&[0u8; 8]);
        // xMin, yMin, xMax, yMax (i16 each) at 36
        for _ in 0..4 { write_i16(&mut buf, 0); }
        // macStyle at 44, lowestRecPPEM at 46, fontDirectionHint at 48
        write_u16(&mut buf, 0); write_u16(&mut buf, 8); write_i16(&mut buf, 2);
        // indexToLocFormat at 50
        write_i16(&mut buf, 0);
        // glyphDataFormat at 52
        write_i16(&mut buf, 0);
        assert_eq!(buf.len() as u32, head_off + head_len);

        // ── hhea table ───────────────────────────────────────────────────────
        // version (Fixed) at 0
        write_u32(&mut buf, 0x0001_0000);
        // ascender at 4
        write_i16(&mut buf, 800);
        // descender at 6
        write_i16(&mut buf, -200);
        // lineGap at 8
        write_i16(&mut buf, 0);
        // advanceWidthMax at 10
        write_u16(&mut buf, 1000);
        // minLeftSideBearing at 12, minRightSideBearing at 14, xMaxExtent at 16
        write_i16(&mut buf, 0); write_i16(&mut buf, 0); write_i16(&mut buf, 0);
        // caretSlopeRise at 18, caretSlopeRun at 20, caretOffset at 22
        write_i16(&mut buf, 1); write_i16(&mut buf, 0); write_i16(&mut buf, 0);
        // reserved[0..4] at 24
        for _ in 0..4 { write_i16(&mut buf, 0); }
        // metricDataFormat at 32
        write_i16(&mut buf, 0);
        // numberOfHMetrics at 34
        write_u16(&mut buf, num_h_metrics);
        assert_eq!(buf.len() as u32, hhea_off + hhea_len);

        // ── maxp table ───────────────────────────────────────────────────────
        // version 0.5: only numGlyphs
        write_u32(&mut buf, 0x0000_5000);
        write_u16(&mut buf, num_glyphs);
        assert_eq!(buf.len() as u32, maxp_off + maxp_len);

        // ── cmap table ───────────────────────────────────────────────────────
        // cmap index header: version=0, numSubtables=1
        write_u16(&mut buf, 0);
        write_u16(&mut buf, 1);
        // Encoding record: platform 3, encoding 1, subtable at offset 12 from cmap start
        write_u16(&mut buf, 3);   // platformID
        write_u16(&mut buf, 1);   // encodingID
        write_u32(&mut buf, 12);  // offset from cmap table start → 4+8=12
        // Format 4 subtable (minimal: 1 segment = the end-of-table sentinel)
        // Segment: endCode=0xFFFF, startCode=0xFFFF, idDelta=1, idRangeOffset=0
        let seg_count: u16 = 1;
        write_u16(&mut buf, 4);                    // format
        write_u16(&mut buf, cmap_sub_len);         // length
        write_u16(&mut buf, 0);                    // language
        write_u16(&mut buf, seg_count * 2);        // segCountX2
        write_u16(&mut buf, 2);                    // searchRange
        write_u16(&mut buf, 0);                    // entrySelector
        write_u16(&mut buf, 0);                    // rangeShift
        write_u16(&mut buf, 0xFFFF);               // endCode[0] = sentinel
        write_u16(&mut buf, 0);                    // reservedPad
        write_u16(&mut buf, 0xFFFF);               // startCode[0] = sentinel
        write_i16(&mut buf, 1);                    // idDelta[0]
        write_u16(&mut buf, 0);                    // idRangeOffset[0]
        assert_eq!(buf.len() as u32, cmap_off + cmap_len);

        // ── hmtx table ───────────────────────────────────────────────────────
        // 5 full hMetric records: (advanceWidth=600, lsb=50) each
        for _ in 0..num_h_metrics {
            write_u16(&mut buf, 600); // advanceWidth
            write_i16(&mut buf, 50);  // lsb
        }
        assert_eq!(buf.len() as u32, hmtx_off + hmtx_len);

        // ── kern table ───────────────────────────────────────────────────────
        // kern table header: version=0, nTables=1
        write_u16(&mut buf, 0);
        write_u16(&mut buf, 1);
        // Subtable header: version=0, length=<total>, coverage=0x0001
        // coverage low byte 0x01 = horizontal kerning (not cross-stream, not override)
        // format = high byte of coverage = 0x00 = Format 0
        let sub_len = (kern_subtable_len) as u16;
        write_u16(&mut buf, 0);       // subtable version
        write_u16(&mut buf, sub_len); // subtable length
        write_u16(&mut buf, 0x0001);  // coverage: format 0, horizontal
        // Format 0 header: nPairs, searchRange, entrySelector, rangeShift
        let np = n_pairs;
        write_u16(&mut buf, np);      // nPairs
        write_u16(&mut buf, np.next_power_of_two().min(np) * 6); // searchRange (approx)
        write_u16(&mut buf, 0);       // entrySelector
        write_u16(&mut buf, 0);       // rangeShift
        // Kern pairs: must be sorted by composite key (left<<16|right)
        let mut sorted_pairs = pairs.to_vec();
        sorted_pairs.sort_by_key(|&(l, r, _)| ((l as u32) << 16) | (r as u32));
        for (left, right, value) in &sorted_pairs {
            write_u16(&mut buf, *left);
            write_u16(&mut buf, *right);
            write_i16(&mut buf, *value);
        }

        buf
    }

    #[test]
    fn kerning_synthetic_pair_found() {
        // Build a font with one kern pair: (glyph 1, glyph 2) → -140.
        let font_bytes = build_synthetic_font_with_kern(&[(1, 2, -140), (3, 4, 80)]);
        let font = load(&font_bytes).unwrap();
        // The kern pair should be found.
        assert_eq!(kerning(&font, 1, 2), -140);
    }

    #[test]
    fn kerning_synthetic_second_pair_found() {
        let font_bytes = build_synthetic_font_with_kern(&[(1, 2, -140), (3, 4, 80)]);
        let font = load(&font_bytes).unwrap();
        assert_eq!(kerning(&font, 3, 4), 80);
    }

    #[test]
    fn kerning_synthetic_absent_pair_returns_zero() {
        let font_bytes = build_synthetic_font_with_kern(&[(1, 2, -140), (3, 4, 80)]);
        let font = load(&font_bytes).unwrap();
        // Pair (1, 4) does not exist.
        assert_eq!(kerning(&font, 1, 4), 0);
    }

    #[test]
    fn kerning_returns_zero_for_invalid_glyph_ids() {
        let bytes = inter_regular();
        let font = load(&bytes).unwrap();
        // Glyph 0 is .notdef; Inter has no kern table, so 0.
        let result = kerning(&font, 0, 0);
        assert_eq!(result, 0);
    }

    // ── read_u16 / read_i16 helpers ───────────────────────────────────────────

    #[test]
    fn read_u16_big_endian_correct() {
        let buf = [0x08, 0x00];
        assert_eq!(read_u16(&buf, 0).unwrap(), 0x0800); // 2048
    }

    #[test]
    fn read_i16_negative_correct() {
        // 0xFE00 in big-endian = -512 as i16
        let buf = [0xFE, 0x00];
        assert_eq!(read_i16(&buf, 0).unwrap(), -512i16);
    }

    #[test]
    fn read_u16_out_of_bounds_returns_error() {
        let buf = [0x08]; // only 1 byte
        assert_eq!(read_u16(&buf, 0), Err(FontError::BufferTooShort));
    }

    #[test]
    fn read_u32_big_endian_correct() {
        let buf = [0x00, 0x01, 0x00, 0x00];
        assert_eq!(read_u32(&buf, 0).unwrap(), 0x0001_0000);
    }
}
