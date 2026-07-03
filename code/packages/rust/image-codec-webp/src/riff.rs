//! RIFF container helpers for WebP files.
//!
//! A WebP file is a RIFF file with the WEBP fourCC.  The RIFF container is
//! a very simple binary framing format: a 12-byte fixed header followed by
//! one or more typed chunks.
//!
//! ## Layout
//!
//! ```text
//! Offset  Size  Field
//!   0       4   "RIFF"                (literal ASCII)
//!   4       4   file_size - 8         (u32 little-endian; excludes the RIFF+size fields)
//!   8       4   "WEBP"                (WebP fourCC)
//!  12       4   chunk_type            (e.g. "VP8L", "VP8 ", "VP8X")
//!  16       4   chunk_size            (u32 little-endian; bytes of chunk_data)
//!  20       ?   chunk_data            (chunk_size bytes, followed by a 0 pad if odd)
//! ```
//!
//! ## RIFF file size field
//!
//! The field at offset 4 is the total file size minus 8 bytes (the size of the
//! "RIFF" tag and the size field itself).  So:
//!
//! ```text
//! file_size_field = 4 (WEBP) + 4 (chunk_type) + 4 (chunk_size) + len(chunk_data)
//!                 = 12 + len(chunk_data)
//! ```
//!
//! If chunk_data has odd length, a padding byte of 0 is appended **after**
//! the data (the padding is not counted in chunk_size, but it IS counted in
//! file_size_field).
//!
//! ## References
//!
//! - RIFF spec: https://www.iana.org/assignments/wave-avi-codec-registry/wave-avi-codec-registry.xhtml
//! - WebP container spec: https://developers.google.com/speed/webp/docs/riff_container

/// Build a complete WebP RIFF file containing one chunk.
///
/// `chunk_type` is a 4-byte chunk identifier (e.g. `b"VP8L"`).
/// `chunk_data` is the raw chunk payload (not including the type or size fields).
///
/// Returns the complete file as a `Vec<u8>`, ready to write to disk.
///
/// ## Example
///
/// ```text
/// let bytes = build_riff(b"VP8L", &payload);
/// // bytes[0..4]  = b"RIFF"
/// // bytes[4..8]  = (bytes.len() - 8) as u32 le
/// // bytes[8..12] = b"WEBP"
/// // bytes[12..16] = b"VP8L"
/// // bytes[16..20] = payload.len() as u32 le
/// // bytes[20..]  = payload (+ optional pad byte)
/// ```
pub fn build_riff(chunk_type: &[u8; 4], chunk_data: &[u8]) -> Vec<u8> {
    // chunk_size field: actual chunk data length (does NOT include padding).
    let chunk_size = chunk_data.len() as u32;

    // Padding: RIFF chunks must be even-length.  If chunk_data has odd length,
    // append one zero byte.  This pad byte IS counted in the RIFF file size but
    // NOT in the chunk_size field.
    let padded_chunk_len = chunk_data.len() + (chunk_data.len() & 1);

    // RIFF file size field: "WEBP" (4) + chunk_type (4) + chunk_size_field (4) + padded_data.
    let file_size_field = (4u32 + 4 + 4) + padded_chunk_len as u32;

    let total_len = 12 + 8 + padded_chunk_len; // RIFF header + chunk header + data
    let mut out = Vec::with_capacity(total_len);

    // RIFF header.
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&file_size_field.to_le_bytes());
    out.extend_from_slice(b"WEBP");

    // Chunk header.
    out.extend_from_slice(chunk_type);
    out.extend_from_slice(&chunk_size.to_le_bytes());

    // Chunk data + padding.
    out.extend_from_slice(chunk_data);
    if chunk_data.len() & 1 != 0 {
        out.push(0);
    }

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riff_header_magic() {
        let out = build_riff(b"VP8L", &[]);
        assert_eq!(&out[0..4], b"RIFF");
        assert_eq!(&out[8..12], b"WEBP");
    }

    #[test]
    fn chunk_type_written_correctly() {
        let out = build_riff(b"VP8L", &[0u8; 10]);
        assert_eq!(&out[12..16], b"VP8L");
    }

    #[test]
    fn chunk_size_field_matches_data_length() {
        let data = vec![0u8; 17];
        let out = build_riff(b"VP8L", &data);
        let chunk_size = u32::from_le_bytes(out[16..20].try_into().unwrap());
        assert_eq!(chunk_size, 17);
    }

    #[test]
    fn file_size_field_even_data() {
        // chunk_data len = 4 (even)
        let data = vec![0u8; 4];
        let out = build_riff(b"VP8L", &data);
        let file_size_field = u32::from_le_bytes(out[4..8].try_into().unwrap());
        // Expected: 4 (WEBP) + 4 (type) + 4 (size) + 4 (data) = 16
        assert_eq!(file_size_field, 16);
        // Total file length = 8 (RIFF+size_field) + file_size_field = 24
        assert_eq!(out.len(), 24);
    }

    #[test]
    fn padding_added_for_odd_data() {
        let data = vec![0u8; 3]; // odd
        let out = build_riff(b"VP8L", &data);
        // chunk_size field must be 3 (no padding counted there).
        let chunk_size = u32::from_le_bytes(out[16..20].try_into().unwrap());
        assert_eq!(chunk_size, 3);
        // Total file should be even.
        assert_eq!(out.len() % 2, 0);
        // Pad byte must be 0.
        assert_eq!(out[23], 0);
    }

    #[test]
    fn empty_chunk_data() {
        let out = build_riff(b"VP8L", &[]);
        assert_eq!(&out[0..4], b"RIFF");
        let file_size_field = u32::from_le_bytes(out[4..8].try_into().unwrap());
        // 4 (WEBP) + 4 (type) + 4 (size field) + 0 (data) = 12
        assert_eq!(file_size_field, 12);
    }
}
