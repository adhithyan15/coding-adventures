// # header.rs — RAF outer header parser
//
// The first 116 bytes of a RAF file form the "outer header".  Every codec
// starts here: check the magic bytes, then extract the byte-range triplets
// that tell us where the JPEG thumbnail, CFA metadata block, and raw pixel
// data live.
//
// All multi-byte integers in the outer header are **big-endian**.  This is
// unusual for a format from the early 2000s (most were little-endian), but
// Fujifilm chose network byte order for their header fields.
//
// ## Header layout (116 bytes total)
//
// ```text
// Offset  Size  Field
//      0    16  Magic: "FUJIFILMCCD-RAW " (with trailing space)
//     16     4  Format version (ASCII "0100", "0101", "0200", "0201")
//     20     8  Camera model ID (ASCII, NUL-padded)
//     28    32  Camera model string (ASCII, NUL-padded)
//     60     4  Directory version (ASCII)
//     64    20  Reserved / unknown
//     84     4  JPEG thumbnail offset  (u32 BE)
//     88     4  JPEG thumbnail length  (u32 BE)
//     92     4  CFA header offset      (u32 BE)
//     96     4  CFA header length      (u32 BE)
//    100     4  CFA pixel data offset  (u32 BE)
//    104     4  CFA pixel data length  (u32 BE)
//    108     4  Second CFA offset      (u32 BE) — often 0; ignored
//    112     4  Second CFA length      (u32 BE) — often 0; ignored
// ```

/// The 16-byte magic that begins every RAF file, including the trailing space.
pub const RAF_MAGIC: &[u8; 16] = b"FUJIFILMCCD-RAW ";

/// The minimum number of bytes required for a complete outer header.
pub const HEADER_SIZE: usize = 116;

/// Parsed outer header extracted from the first 116 bytes of a RAF file.
///
/// All offsets and lengths here refer to byte positions within the original
/// file buffer.
#[derive(Debug)]
pub struct RafHeader {
    /// Byte offset of the embedded JPEG thumbnail.
    pub jpeg_offset: usize,
    /// Byte length of the embedded JPEG thumbnail.
    pub jpeg_length: usize,
    /// Byte offset of the CFA metadata header block.
    pub cfa_header_offset: usize,
    /// Byte length of the CFA metadata header block.
    pub cfa_header_length: usize,
    /// Byte offset of the raw CFA pixel data.
    pub cfa_offset: usize,
    /// Byte length of the raw CFA pixel data.
    pub cfa_length: usize,
}

/// Check the magic bytes and parse the outer RAF header.
///
/// # Errors
///
/// Returns `Err` if:
/// - `bytes.len() < 116` (header is truncated)
/// - The first 16 bytes are not `"FUJIFILMCCD-RAW "` (not a RAF file)
/// - Any computed offset or length would exceed `bytes.len()` (corrupt file)
pub fn parse_header(bytes: &[u8]) -> Result<RafHeader, String> {
    // ── length guard ────────────────────────────────────────────────────────
    if bytes.len() < HEADER_SIZE {
        return Err(format!(
            "RAF: file too short ({} bytes); outer header requires {} bytes",
            bytes.len(),
            HEADER_SIZE
        ));
    }

    // ── magic check ─────────────────────────────────────────────────────────
    // "FUJIFILMCCD-RAW " — the trailing space is part of the signature.
    // We use a byte-level comparison so the check works even if the camera
    // writes the bytes with a different ASCII code page.
    if &bytes[0..16] != RAF_MAGIC.as_ref() {
        return Err("RAF: invalid magic — not a Fujifilm RAF file".into());
    }

    // ── read the six u32 BE fields that matter to the decoder ───────────────
    // (bytes 64–83 are "reserved / unknown" and bytes 108–115 are the second
    // CFA pair; both are skipped.)
    let jpeg_offset  = read_u32_be(bytes, 84) as usize;
    let jpeg_length  = read_u32_be(bytes, 88) as usize;
    let cfa_header_offset = read_u32_be(bytes, 92) as usize;
    let cfa_header_length = read_u32_be(bytes, 96) as usize;
    let cfa_offset   = read_u32_be(bytes, 100) as usize;
    let cfa_length   = read_u32_be(bytes, 104) as usize;

    // ── bounds checks ────────────────────────────────────────────────────────
    // Validate that every region the decoder will read actually fits inside
    // the buffer we received.  These checks also defend against crafted headers
    // that point wildly outside the file.
    validate_region(bytes, "JPEG",       jpeg_offset, jpeg_length)?;
    validate_region(bytes, "CFA header", cfa_header_offset, cfa_header_length)?;
    validate_region(bytes, "CFA pixels", cfa_offset, cfa_length)?;

    Ok(RafHeader {
        jpeg_offset,
        jpeg_length,
        cfa_header_offset,
        cfa_header_length,
        cfa_offset,
        cfa_length,
    })
}

// ── private helpers ──────────────────────────────────────────────────────────

/// Read a big-endian u32 from `bytes[offset..offset+4]`.
///
/// Panics if `offset + 4 > bytes.len()`; callers must ensure the outer header
/// is long enough before calling.
#[inline]
pub(crate) fn read_u32_be(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Read a little-endian u32 from `bytes[offset..offset+4]`.
#[inline]
pub(crate) fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Read a big-endian u16 from `bytes[offset..offset+2]`.
#[inline]
pub(crate) fn read_u16_be(bytes: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([bytes[offset], bytes[offset + 1]])
}

/// Return `Err` if `offset + length > bytes.len()` (region out of bounds).
fn validate_region(
    bytes: &[u8],
    name: &str,
    offset: usize,
    length: usize,
) -> Result<(), String> {
    // Use checked arithmetic to guard against usize overflow on 32-bit hosts.
    let end = offset
        .checked_add(length)
        .ok_or_else(|| format!("RAF: {name} region offset+length overflows usize"))?;
    if end > bytes.len() {
        return Err(format!(
            "RAF: {name} region [{offset}..{end}] exceeds file length {}",
            bytes.len()
        ));
    }
    Ok(())
}
