//! # Container format detection
//!
//! A JPEG XL file arrives in one of two forms:
//!
//! 1. **Naked codestream** — the raw JXL codestream bytes, starting with the
//!    two-byte magic `FF 0A`.
//!
//! 2. **ISOBMFF container** — the codestream is wrapped in an ISO Base Media
//!    File Format (MP4-family) box structure.  The file starts with a 12-byte
//!    `JXL ` signature box, followed by a sequence of typed boxes.  The
//!    codestream itself lives in the `jxlc` box.
//!
//! This module's job is purely to locate the codestream bytes; all bit-level
//! parsing happens in the encoder and decoder modules.
//!
//! ## ISOBMFF box layout
//!
//! ```text
//! ┌──────────────────────────────────────────────────┐
//! │ 4 bytes: box_size (big-endian u32, includes hdr) │
//! │ 4 bytes: box_type (ASCII, e.g. "jxlc")           │
//! │ N bytes: box payload (box_size − 8 bytes)         │
//! └──────────────────────────────────────────────────┘
//! ```
//!
//! The signature box is always exactly 12 bytes long and carries no payload
//! beyond its own 12-byte structure (size=12, type=`JXL `, data=`\r\n\x87\n`).

/// Two-byte magic that marks a naked JXL codestream.
const NAKED_MAGIC: [u8; 2] = [0xFF, 0x0A];

/// Twelve-byte signature that opens every ISOBMFF-wrapped JXL file.
///
/// Layout:
/// - `\x00\x00\x00\x0C` — box_size = 12 (the box is only its header)
/// - `JXL ` (four ASCII bytes, space is 0x20)
/// - `\x0D\x0A\x87\x0A` — payload used as a corruption-detection canary
const ISOBMFF_SIG: [u8; 12] = [
    0x00, 0x00, 0x00, 0x0C,
    b'J', b'X', b'L', b' ',
    0x0D, 0x0A, 0x87, 0x0A,
];

/// Return the raw codestream bytes from a JXL file, stripping any container
/// wrapper.
///
/// Accepts both naked codestreams (`FF 0A …`) and ISOBMFF containers.  On
/// success the returned slice begins immediately after the two-byte naked magic
/// (for naked files) or after the `jxlc` box header (for ISOBMFF files).
///
/// # Errors
///
/// Returns `Err` if:
/// - The file is shorter than 2 bytes.
/// - The first bytes match neither the naked magic nor the ISOBMFF signature.
/// - An ISOBMFF container does not contain a `jxlc` box.
/// - A box's size field is malformed (< 8 bytes or extends past EOF).
pub fn extract_codestream(data: &[u8]) -> Result<&[u8], String> {
    if data.len() < 2 {
        return Err("JXL: file too short to be a valid JPEG XL image".into());
    }

    // ── Naked codestream ────────────────────────────────────────────────
    if data[0] == NAKED_MAGIC[0] && data[1] == NAKED_MAGIC[1] {
        // Skip the two-byte magic; the rest is the codestream.
        return Ok(&data[2..]);
    }

    // ── ISOBMFF container ───────────────────────────────────────────────
    if data.len() >= 12 && data[..12] == ISOBMFF_SIG {
        return find_jxlc_box(data);
    }

    Err("JXL: unrecognized file signature — not a JPEG XL file".into())
}

/// Scan the ISOBMFF box chain for a `jxlc` box and return its payload.
fn find_jxlc_box(data: &[u8]) -> Result<&[u8], String> {
    let mut pos = 0usize;

    while pos + 8 <= data.len() {
        // The first 4 bytes are box_size (big-endian u32).
        // box_size includes the 8-byte header (size + type fields).
        let box_size = u32::from_be_bytes(
            data[pos..pos + 4].try_into().unwrap()
        ) as usize;

        // box_size == 0 means "extends to end of file" (ISO 14496-12 §4.2).
        // We don't need to handle that case to decode our own output, but we
        // recognise it to avoid an infinite loop.
        if box_size == 0 {
            break;
        }

        if box_size < 8 {
            return Err(format!(
                "JXL: malformed ISOBMFF box at offset {} — size {} is less than 8",
                pos, box_size
            ));
        }

        let box_type = &data[pos + 4..pos + 8];

        if box_type == b"jxlc" {
            let payload_start = pos + 8;
            let payload_end = pos + box_size;
            if payload_end > data.len() {
                return Err(format!(
                    "JXL: `jxlc` box at offset {} claims size {} but file is only {} bytes",
                    pos, box_size, data.len()
                ));
            }
            return Ok(&data[payload_start..payload_end]);
        }

        pos += box_size;
    }

    Err("JXL: ISOBMFF container contains no `jxlc` (codestream) box".into())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naked_codestream_strips_magic() {
        let data = [0xFF, 0x0A, 0xDE, 0xAD, 0xBE, 0xEF];
        let cs = extract_codestream(&data).unwrap();
        assert_eq!(cs, &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn naked_codestream_empty_after_magic() {
        let data = [0xFF, 0x0A];
        let cs = extract_codestream(&data).unwrap();
        assert_eq!(cs, &[] as &[u8]);
    }

    #[test]
    fn too_short_returns_err() {
        assert!(extract_codestream(&[]).is_err());
        assert!(extract_codestream(&[0xFF]).is_err());
    }

    #[test]
    fn bad_magic_returns_err() {
        assert!(extract_codestream(&[0x89, 0x50, 0x4E, 0x47]).is_err()); // PNG
        assert!(extract_codestream(&[0xFF, 0xD8, 0xFF, 0xE0]).is_err()); // JPEG
    }

    #[test]
    fn isobmff_finds_jxlc_box() {
        // Minimal synthetic ISOBMFF file:
        // [signature box 12 bytes] [jxlc box 12 bytes payload = [0x11, 0x22]]
        let mut data = Vec::new();
        // Signature box
        data.extend_from_slice(&ISOBMFF_SIG);
        // jxlc box: size=10 (8 hdr + 2 payload), type=jxlc, payload=[0x11,0x22]
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x0A]); // box_size = 10
        data.extend_from_slice(b"jxlc");
        data.extend_from_slice(&[0x11, 0x22]);

        let cs = extract_codestream(&data).unwrap();
        assert_eq!(cs, &[0x11, 0x22]);
    }

    #[test]
    fn isobmff_skips_non_jxlc_boxes() {
        let mut data = Vec::new();
        data.extend_from_slice(&ISOBMFF_SIG);
        // ftyp box (not jxlc) — 8 bytes header only
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x08]);
        data.extend_from_slice(b"ftyp");
        // jxlc box
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x09]);
        data.extend_from_slice(b"jxlc");
        data.push(0xAB);

        let cs = extract_codestream(&data).unwrap();
        assert_eq!(cs, &[0xAB]);
    }

    #[test]
    fn isobmff_no_jxlc_returns_err() {
        let mut data = Vec::new();
        data.extend_from_slice(&ISOBMFF_SIG);
        // only an ftyp box
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x08]);
        data.extend_from_slice(b"ftyp");

        assert!(extract_codestream(&data).is_err());
    }
}
