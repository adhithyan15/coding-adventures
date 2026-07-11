// # header.rs — RW2 magic validation and IFD parsing
//
// ## RW2 File Header
//
// RW2 looks like TIFF but is distinguished by a different version byte:
//
//   Offset  Size  Field
//   0       2     Byte order marker: "II" (0x49 0x49) — always little-endian
//   2       2     Version: 0x0055 (85 decimal) — NOT 42 like standard TIFF
//   4       4     Offset of first IFD (u32 LE, usually 8)
//
// This 4-byte structure ("IIU\0") is the discriminating magic.
//
// ## IFD (Image File Directory) Structure
//
// The TIFF IFD format is a compact directory used here for Panasonic private
// tags. Each directory is:
//
//   u16 LE: entry_count
//   [entry_count × 12-byte entries]
//   u32 LE: next_ifd_offset (0 = no more IFDs)
//
// Each 12-byte entry:
//
//   Offset  Size  Field
//   0       2     tag (u16 LE)
//   2       2     type (u16 LE): 1=BYTE, 3=SHORT, 4=LONG, 5=RATIONAL, …
//   4       4     count (u32 LE): how many values
//   8       4     value_or_offset (u32 LE):
//                   • if count × sizeof(type) ≤ 4 bytes: the value is stored inline
//                   • otherwise: a file offset to the actual value bytes
//
// For this crate we only need SHORT (u16) and LONG (u32) inline values, which
// is the case for all Panasonic private tags we parse.

/// Parsed fields extracted from the RW2 IFD.
///
/// All fields that are absent from the IFD remain `None`. The decoder
/// applies sensible defaults when tags are missing (e.g. border = full sensor,
/// white balance = neutral 1.0).
#[derive(Debug, Default)]
pub struct Rw2Ifd {
    /// Full sensor width in pixels (tag 0x0002).
    pub sensor_width: Option<u32>,
    /// Full sensor height in pixels (tag 0x0003).
    pub sensor_height: Option<u32>,
    /// Top row of active image area (tag 0x0004).
    pub sensor_top_border: Option<u32>,
    /// Left column of active image area (tag 0x0005).
    pub sensor_left_border: Option<u32>,
    /// Exclusive bottom row of active area (tag 0x0006).
    pub sensor_bottom_border: Option<u32>,
    /// Exclusive right column of active area (tag 0x0007).
    pub sensor_right_border: Option<u32>,
    /// White-balance red multiplier × 256 (tag 0x0011).
    pub red_balance: Option<u32>,
    /// White-balance blue multiplier × 256 (tag 0x0012).
    pub blue_balance: Option<u32>,
    /// Bits per raw pixel (tag 0x0024). Supported: 12.
    pub image_depth: Option<u32>,
    /// File offset of the raw pixel strip (tag 0x0097 or StripOffsets 273).
    pub raw_data_offset: Option<u32>,
}

// ---------------------------------------------------------------------------
// TIFF type sizes
// ---------------------------------------------------------------------------

/// Return the byte size of a single TIFF value of the given type code.
///
/// | Code | Name     | Bytes |
/// |------|----------|-------|
/// |  1   | BYTE     |   1   |
/// |  2   | ASCII    |   1   |
/// |  3   | SHORT    |   2   |
/// |  4   | LONG     |   4   |
/// |  5   | RATIONAL |   8   |
/// |  6   | SBYTE    |   1   |
/// |  7   | UNDEFINED|   1   |
/// |  8   | SSHORT   |   2   |
/// |  9   | SLONG    |   4   |
/// | 10   | SRATIONAL|   8   |
/// | 11   | FLOAT    |   4   |
/// | 12   | DOUBLE   |   8   |
fn tiff_type_size(type_code: u16) -> usize {
    match type_code {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 => 4,
        5 | 10 | 12 => 8,
        _ => 1, // unknown → treat as byte so we don't panic
    }
}

// ---------------------------------------------------------------------------
// Little-endian read helpers
// ---------------------------------------------------------------------------

/// Read a u16 from `bytes[offset..]` in little-endian order.
///
/// # Panics
///
/// Panics if `offset + 2 > bytes.len()`. Callers must validate bounds first.
pub(crate) fn read_u16_le(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

/// Read a u32 from `bytes[offset..]` in little-endian order.
///
/// # Panics
///
/// Panics if `offset + 4 > bytes.len()`. Callers must validate bounds first.
pub(crate) fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

// ---------------------------------------------------------------------------
// Magic check
// ---------------------------------------------------------------------------

/// Validate the 8-byte RW2 file header.
///
/// Returns `Ok(ifd_offset)` where `ifd_offset` is the file offset to the first
/// IFD (always u32 LE at bytes 4–7).
///
/// # Errors
///
/// - `"RW2: file too short"` — fewer than 8 bytes.
/// - `"RW2: not a little-endian file"` — bytes 0–1 are not "II".
/// - `"RW2: not an RW2 file (TIFF version byte is N, expected 85)"` — bytes 2–3
///   hold the TIFF version marker 42 (standard TIFF) or something else. The
///   version 85 (0x0055) is the discriminator that makes a file RW2 vs TIFF.
pub fn check_magic(bytes: &[u8]) -> Result<u32, String> {
    // Need at least the 8-byte header.
    if bytes.len() < 8 {
        return Err("RW2: file too short".into());
    }

    // Bytes 0–1: byte-order marker. RW2 is always little-endian ("II").
    // "MM" would indicate big-endian TIFF, which RW2 never uses.
    if &bytes[0..2] != b"II" {
        return Err(format!(
            "RW2: not a little-endian file (got {:02X} {:02X}, expected 49 49)",
            bytes[0], bytes[1]
        ));
    }

    // Bytes 2–3: version. Standard TIFF = 42 (0x002A). RW2 = 85 (0x0055).
    let version = read_u16_le(bytes, 2);
    if version != 85 {
        return Err(format!(
            "RW2: not an RW2 file (version byte is {version}, expected 85)"
        ));
    }

    // Bytes 4–7: offset of first IFD.
    let ifd_offset = read_u32_le(bytes, 4);
    Ok(ifd_offset)
}

// ---------------------------------------------------------------------------
// IFD parser
// ---------------------------------------------------------------------------

/// Parse the Panasonic private IFD tags from the RW2 file.
///
/// `ifd_offset` is obtained from `check_magic`. All Panasonic tags we care
/// about fit in a single IFD; the `next_ifd_offset` pointer at the end of
/// the directory is ignored (sub-IFDs are not recursed into for now).
///
/// ## Security
///
/// - IFD entry count is capped at 512 to prevent huge loops on corrupted files.
/// - Every byte range access is bounds-checked before indexing.
///
/// ## Tag extraction strategy
///
/// For SHORT tags: the 4-byte value_or_offset field holds the u16 value
/// inline (little-endian in the first 2 bytes).
///
/// For LONG tags: the 4-byte value_or_offset field holds the u32 value inline
/// (when count == 1 and type == LONG, the whole value fits in 4 bytes).
pub fn parse_ifd(bytes: &[u8], ifd_offset: u32) -> Result<Rw2Ifd, String> {
    let off = ifd_offset as usize;

    // Need at least 2 bytes for the entry count.
    if off + 2 > bytes.len() {
        return Err("RW2: IFD offset out of bounds".into());
    }

    let entry_count = read_u16_le(bytes, off) as usize;

    // Security cap: legitimate cameras have fewer than 100 IFD entries; cap at
    // 512 to guard against malformed files that claim millions of entries.
    if entry_count > 512 {
        return Err(format!(
            "RW2: IFD entry count {entry_count} exceeds maximum 512"
        ));
    }

    // IFD body: entry_count × 12 bytes, starting at off+2.
    // Use checked arithmetic to prevent usize overflow if ifd_offset is near
    // usize::MAX (possible on a crafted file on a 32-bit host).
    let entries_start = off.checked_add(2).ok_or("RW2: IFD offset arithmetic overflow")?;
    let entries_end = entry_count
        .checked_mul(12)
        .and_then(|n| entries_start.checked_add(n))
        .ok_or("RW2: IFD entry range arithmetic overflow")?;
    if entries_end > bytes.len() {
        return Err("RW2: IFD extends past end of file".into());
    }

    let mut ifd = Rw2Ifd::default();

    for i in 0..entry_count {
        let e = entries_start + i * 12;

        // Each IFD entry is exactly 12 bytes:
        //   [0..2]  tag       (u16 LE)
        //   [2..4]  type      (u16 LE)
        //   [4..8]  count     (u32 LE)
        //   [8..12] value_or_offset (u32 LE)
        let tag       = read_u16_le(bytes, e);
        let type_code = read_u16_le(bytes, e + 2);
        let count     = read_u32_le(bytes, e + 4);
        let val_field = read_u32_le(bytes, e + 8); // raw 4-byte value field

        // Determine whether the value is inline (stored in the value_or_offset
        // field) or at a file offset. Value is inline if:
        //   count × type_size ≤ 4 bytes
        let type_size = tiff_type_size(type_code);
        let is_inline = (count as usize).saturating_mul(type_size) <= 4;

        // Extract scalar value (u32) for SHORT and LONG tags.
        // For SHORT (2 bytes, inline): the first 2 bytes of the value field.
        // For LONG (4 bytes, inline):  the whole 4-byte value field.
        // If the value is at an offset, we read from there.
        let scalar: u32 = match (type_code, is_inline) {
            // SHORT (type 3) inline: low 16 bits of value_or_offset
            (3, true) => val_field & 0xFFFF,
            // LONG (type 4) inline
            (4, true) => val_field,
            // BYTE (type 1) inline
            (1, true) => val_field & 0xFF,
            // Indirect: read from file offset (only for LONG, 4 bytes)
            (4, false) => {
                let offset = val_field as usize;
                if offset + 4 > bytes.len() {
                    continue; // bounds violation → skip this tag
                }
                read_u32_le(bytes, offset)
            }
            // Indirect SHORT: read u16 from file offset
            (3, false) => {
                let offset = val_field as usize;
                if offset + 2 > bytes.len() {
                    continue;
                }
                read_u16_le(bytes, offset) as u32
            }
            _ => continue, // unsupported type for our tags → skip
        };

        // Populate the relevant field based on the Panasonic private tag number.
        match tag {
            0x0002 => ifd.sensor_width        = Some(scalar),
            0x0003 => ifd.sensor_height       = Some(scalar),
            0x0004 => ifd.sensor_top_border   = Some(scalar),
            0x0005 => ifd.sensor_left_border  = Some(scalar),
            0x0006 => ifd.sensor_bottom_border= Some(scalar),
            0x0007 => ifd.sensor_right_border = Some(scalar),
            0x0011 => ifd.red_balance         = Some(scalar),
            0x0012 => ifd.blue_balance        = Some(scalar),
            0x0024 => ifd.image_depth         = Some(scalar),
            // Tag 0x0097 — Panasonic raw data strip offset
            0x0097 => ifd.raw_data_offset     = Some(scalar),
            // Standard TIFF StripOffsets (273 = 0x0111) — fallback
            0x0111 => {
                if ifd.raw_data_offset.is_none() {
                    ifd.raw_data_offset = Some(scalar);
                }
            }
            _ => {} // unrecognised tag → ignore
        }
    }

    Ok(ifd)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_too_short() {
        assert!(check_magic(&[0x49, 0x49]).is_err());
    }

    #[test]
    fn magic_wrong_byte_order() {
        // "MM" big-endian header
        let mut b = vec![0u8; 8];
        b[0] = 0x4D; b[1] = 0x4D; // "MM"
        b[2] = 0x00; b[3] = 0x2A; // TIFF version 42
        assert!(check_magic(&b).is_err());
    }

    #[test]
    fn magic_tiff_version_rejected() {
        // "II" + version 42 = standard TIFF → must be rejected as not RW2
        let mut b = vec![0u8; 8];
        b[0] = 0x49; b[1] = 0x49; // "II"
        b[2] = 0x2A; b[3] = 0x00; // version 42 LE
        let result = check_magic(&b);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("version byte is 42"));
    }

    #[test]
    fn magic_rw2_accepted() {
        // "II" + version 85 (0x55) = valid RW2
        let mut b = vec![0u8; 8];
        b[0] = 0x49; b[1] = 0x49;
        b[2] = 0x55; b[3] = 0x00; // 85 LE
        b[4] = 0x08; b[5] = 0x00; b[6] = 0x00; b[7] = 0x00; // IFD at offset 8
        let result = check_magic(&b);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 8);
    }
}
