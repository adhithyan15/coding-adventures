// # encoder.rs — Minimal Uncompressed TIFF Writer
//
// This encoder writes a valid TIFF 6.0 file for any RGB8 image.
// It always uses:
//   - Little-endian byte order (most common, consistent with x86/ARM)
//   - Uncompressed pixel data (Compression=1)
//   - Chunky RGB layout (PlanarConfiguration=1)
//   - A single strip containing all rows
//
// ## TIFF Inline vs. External Values
//
// This is the tricky part of the TIFF format. Each IFD entry has 12 bytes:
//   [tag:2][type:2][count:4][value_or_offset:4]
//
// If `count * type_size <= 4`, the value is stored INLINE in the last 4 bytes,
// left-justified. Otherwise, the last 4 bytes are a FILE OFFSET to the data.
//
// Common cases:
//   - SHORT (2 bytes), count=1: inline (value occupies bytes 0-1 of the 4-byte field)
//   - LONG  (4 bytes), count=1: inline (value fills all 4 bytes)
//   - SHORT (2 bytes), count=3: external (6 bytes > 4)
//   - RATIONAL (8 bytes), count=1: external (8 bytes > 4)
//
// The encoder must follow this rule faithfully — any violation causes the
// decoder to misinterpret values.
//
// ## File Layout
//
// We use 11 IFD entries. The tags that need external data are BitsPerSample
// (3 × SHORT = 6 bytes) and the two RATIONAL resolution tags (8 bytes each).
// StripOffsets (1 × LONG) and StripByteCounts (1 × LONG) are INLINE.
//
// ```text
// Offset  Content
// 0       "II" (little-endian marker)
// 2       42 (magic)
// 4       8 (IFD offset)
//
// Offset 8: IFD
//   8     11 (number of IFD entries)
//   10    11 × 12-byte entries = 132 bytes
//   142   0 (no next IFD)
//
// Offset 146: external data
//   146   BitsPerSample: [8, 8, 8] (3 × u16 = 6 bytes)
//   152   XResolution: 72/1 (2 × u32 = 8 bytes)
//   160   YResolution: 72/1 (2 × u32 = 8 bytes)
//
// Offset 168: pixel data (RGB, width × height × 3 bytes)
// ```

use pixel_container::PixelContainer;

// ─── Little-endian write helpers ──────────────────────────────────────────────

fn write_u16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn write_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// Write an IFD entry with a SHORT inline value.
///
/// SHORT = type code 3, size = 2 bytes.
/// count=1 → total_size=2 ≤ 4 → INLINE.
/// The value occupies bytes 0-1 of the 4-byte value_or_offset field (left-justified).
fn write_short_entry(buf: &mut Vec<u8>, tag: u16, value: u16) {
    write_u16(buf, tag);   // Tag
    write_u16(buf, 3);     // Type = SHORT
    write_u32(buf, 1);     // Count = 1
    write_u16(buf, value); // Value (left-justified in 4-byte field)
    write_u16(buf, 0);     // Padding (upper 2 bytes unused)
}

/// Write an IFD entry with a LONG inline value.
///
/// LONG = type code 4, size = 4 bytes.
/// count=1 → total_size=4 ≤ 4 → INLINE.
/// The 4-byte value exactly fills the value_or_offset field.
fn write_long_entry(buf: &mut Vec<u8>, tag: u16, value: u32) {
    write_u16(buf, tag); // Tag
    write_u16(buf, 4);   // Type = LONG
    write_u32(buf, 1);   // Count = 1
    write_u32(buf, value); // Value (fills all 4 bytes)
}

/// Write an IFD entry that points to external data at `data_offset`.
///
/// Use this when `count * type_size > 4` so the value must be stored elsewhere.
/// The last 4 bytes of the entry are the file offset of the external data.
fn write_external_entry(buf: &mut Vec<u8>, tag: u16, type_code: u16, count: u32, data_offset: u32) {
    write_u16(buf, tag);
    write_u16(buf, type_code);
    write_u32(buf, count);
    write_u32(buf, data_offset); // file offset to the actual data
}

// ─── Encoder ─────────────────────────────────────────────────────────────────

/// Encode a PixelContainer as an uncompressed TIFF file.
///
/// Outputs 8-bit per channel RGB (alpha channel is stripped — TIFF's ExtraSamples
/// tag would be needed to carry alpha, and we keep this encoder minimal).
///
/// # File Layout
///
/// ```text
/// 0..8    header: "II" + 42 + 8 (IFD offset)
/// 8..146  IFD: 11 entries × 12 bytes + 4-byte next-IFD offset
/// 146..152  BitsPerSample: [8, 8, 8] (3 × u16)
/// 152..160  XResolution: 72/1 (RATIONAL)
/// 160..168  YResolution: 72/1 (RATIONAL)
/// 168..N    pixel data (RGB)
/// ```
pub fn encode_tiff(pixels: &PixelContainer) -> Vec<u8> {
    let width = pixels.width;
    let height = pixels.height;
    let pixel_bytes = (width as usize) * (height as usize) * 3;

    // ── Compute file layout offsets ─────────────────────────────────────────
    //
    // Everything must be placed before writing the IFD, because the IFD
    // entries contain file offsets to external data.
    //
    // IFD: 11 entries
    // At offset 8:
    //   8     → 2 bytes: entry count (11)
    //   10    → 11 × 12 = 132 bytes: entries
    //   142   → 4 bytes: next IFD offset (0)
    // Total IFD section: 2 + 132 + 4 = 138 bytes → IFD ends at offset 146.
    //
    // External data starts at 146:
    //   146   → 6 bytes: BitsPerSample = [8, 8, 8]
    //   152   → 8 bytes: XResolution = 72/1 (RATIONAL: 2 × LONG)
    //   160   → 8 bytes: YResolution = 72/1
    // External data ends at 168.
    //
    // Pixel data starts at 168.

    const NUM_ENTRIES: u16 = 11;
    const IFD_SECTION_SIZE: u32 = 2 + (NUM_ENTRIES as u32 * 12) + 4; // 138
    const IFD_END: u32 = 8 + IFD_SECTION_SIZE; // 146

    const BITS_OFFSET: u32 = IFD_END;        // 146 — BitsPerSample: 3 × SHORT = 6 bytes
    const XRES_OFFSET: u32 = BITS_OFFSET + 6; // 152 — XResolution: RATIONAL = 8 bytes
    const YRES_OFFSET: u32 = XRES_OFFSET + 8; // 160 — YResolution: RATIONAL = 8 bytes
    const PIXEL_DATA_OFFSET: u32 = YRES_OFFSET + 8; // 168

    let mut buf = Vec::with_capacity(PIXEL_DATA_OFFSET as usize + pixel_bytes);

    // ── TIFF header (8 bytes) ───────────────────────────────────────────────
    buf.extend_from_slice(b"II"); // little-endian byte order ("II" = Intel)
    write_u16(&mut buf, 42);      // magic number (TIFF classic = 42)
    write_u32(&mut buf, 8);       // first IFD starts at byte 8

    // ── IFD: 11 entries ─────────────────────────────────────────────────────
    //
    // IFD entries MUST be in ascending tag-number order (TIFF 6.0 spec §2).
    write_u16(&mut buf, NUM_ENTRIES);

    // 256 — ImageWidth: pixel columns. LONG, count=1 → inline.
    write_long_entry(&mut buf, 256, width);

    // 257 — ImageLength: pixel rows. LONG, count=1 → inline.
    write_long_entry(&mut buf, 257, height);

    // 258 — BitsPerSample: [8, 8, 8] for 24-bit RGB. SHORT, count=3 → external.
    //       3 × 2 = 6 bytes > 4, so we store the offset to external data.
    write_external_entry(&mut buf, 258, 3, 3, BITS_OFFSET);

    // 259 — Compression: 1 = no compression. SHORT, count=1 → inline.
    write_short_entry(&mut buf, 259, 1);

    // 262 — PhotometricInterpretation: 2 = RGB. SHORT, count=1 → inline.
    write_short_entry(&mut buf, 262, 2);

    // 273 — StripOffsets: file offset to the one strip. LONG, count=1 → INLINE.
    //       We write PIXEL_DATA_OFFSET directly as the value, not as an offset
    //       to external data, because 1 × LONG = 4 bytes ≤ 4 → inline rule.
    write_long_entry(&mut buf, 273, PIXEL_DATA_OFFSET);

    // 277 — SamplesPerPixel: 3 (R, G, B). SHORT, count=1 → inline.
    write_short_entry(&mut buf, 277, 3);

    // 278 — RowsPerStrip: height (one big strip). LONG, count=1 → inline.
    write_long_entry(&mut buf, 278, height);

    // 279 — StripByteCounts: number of bytes in the one strip. LONG, count=1 → INLINE.
    //       Same inline rule as StripOffsets.
    write_long_entry(&mut buf, 279, pixel_bytes as u32);

    // 282 — XResolution: 72/1 pixels per inch. RATIONAL, count=1 → external.
    //       RATIONAL = 2 × LONG = 8 bytes > 4 → offset to external data.
    write_external_entry(&mut buf, 282, 5, 1, XRES_OFFSET);

    // 283 — YResolution: 72/1 pixels per inch. RATIONAL, count=1 → external.
    write_external_entry(&mut buf, 283, 5, 1, YRES_OFFSET);

    // Next IFD offset = 0 (no more IFDs in this file).
    write_u32(&mut buf, 0);

    // ── External data ───────────────────────────────────────────────────────
    debug_assert_eq!(
        buf.len(),
        IFD_END as usize,
        "BUG: IFD section size mismatch (got {}, expected {})",
        buf.len(),
        IFD_END
    );

    // BitsPerSample = [8, 8, 8] — three u16 values.
    write_u16(&mut buf, 8); // Red channel: 8 bits
    write_u16(&mut buf, 8); // Green channel: 8 bits
    write_u16(&mut buf, 8); // Blue channel: 8 bits

    // XResolution = 72/1 (72 pixels per inch).
    // RATIONAL is stored as two LONGs: numerator, then denominator.
    write_u32(&mut buf, 72); // numerator
    write_u32(&mut buf, 1);  // denominator

    // YResolution = 72/1.
    write_u32(&mut buf, 72);
    write_u32(&mut buf, 1);

    debug_assert_eq!(
        buf.len(),
        PIXEL_DATA_OFFSET as usize,
        "BUG: pixel data offset mismatch (got {}, expected {})",
        buf.len(),
        PIXEL_DATA_OFFSET
    );

    // ── Pixel data (RGB, no alpha) ──────────────────────────────────────────
    //
    // We store pixels in chunky RGB order: R0 G0 B0 R1 G1 B1 ...
    // Alpha (the 4th byte in RGBA) is dropped — the TIFF file is opaque.
    for chunk in pixels.data.chunks_exact(4) {
        buf.push(chunk[0]); // R
        buf.push(chunk[1]); // G
        buf.push(chunk[2]); // B
        // chunk[3] = A — intentionally omitted
    }

    buf
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use pixel_container::PixelContainer;

    #[test]
    fn encode_produces_valid_tiff_header() {
        let pc = PixelContainer::new(2, 2);
        let bytes = encode_tiff(&pc);

        // Byte-order marker: "II" = little-endian.
        assert_eq!(&bytes[0..2], b"II", "Should be little-endian");

        // Magic number: 42.
        let magic = u16::from_le_bytes([bytes[2], bytes[3]]);
        assert_eq!(magic, 42, "Magic must be 42");

        // IFD offset: 8.
        let ifd_off = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        assert_eq!(ifd_off, 8, "IFD must start at byte 8");
    }

    #[test]
    fn encode_1x1_pixel() {
        let mut pc = PixelContainer::new(1, 1);
        pc.set_pixel(0, 0, 100, 150, 200, 255);
        let bytes = encode_tiff(&pc);

        // Pixel data starts at offset 168.
        assert!(
            bytes.len() >= 168 + 3,
            "File should be at least 171 bytes for 1×1 RGB, got {}",
            bytes.len()
        );
        assert_eq!(bytes[168], 100, "R component at pixel offset");
        assert_eq!(bytes[169], 150, "G component at pixel offset");
        assert_eq!(bytes[170], 200, "B component at pixel offset");
    }

    #[test]
    fn encode_zero_size_image() {
        let pc = PixelContainer::new(0, 0);
        let bytes = encode_tiff(&pc);
        // Should produce a valid but empty TIFF (at least the header).
        assert!(bytes.len() >= 8);
    }

    #[test]
    fn encode_outputs_rgb_not_rgba() {
        // A 1×1 image: 3 bytes per pixel (RGB), not 4.
        let pc = PixelContainer::from_data(1, 1, vec![10, 20, 30, 128]); // RGBA input
        let bytes = encode_tiff(&pc);

        // Find the StripByteCounts entry (tag 279) and read its inline value.
        // StripByteCounts is inline LONG (count=1, type=4, total_size=4 ≤ 4).
        // Scan the IFD for tag 279.
        let entry_count = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
        let mut found_byte_count = None;
        for i in 0..entry_count {
            let entry_off = 10 + i * 12;
            let tag = u16::from_le_bytes([bytes[entry_off], bytes[entry_off + 1]]);
            if tag == 279 {
                // Inline LONG value.
                found_byte_count = Some(u32::from_le_bytes([
                    bytes[entry_off + 8],
                    bytes[entry_off + 9],
                    bytes[entry_off + 10],
                    bytes[entry_off + 11],
                ]));
                break;
            }
        }
        assert_eq!(found_byte_count, Some(3), "StripByteCount should be 3 (RGB, no alpha)");

        // Actual pixel bytes at offset 168.
        assert_eq!(bytes[168], 10, "R");
        assert_eq!(bytes[169], 20, "G");
        assert_eq!(bytes[170], 30, "B");
    }

    #[test]
    fn encode_strip_offset_is_correct() {
        // For a 1×1 image, StripOffsets[0] should be 168 (PIXEL_DATA_OFFSET).
        let pc = PixelContainer::new(1, 1);
        let bytes = encode_tiff(&pc);

        let entry_count = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
        let mut strip_offset = None;
        for i in 0..entry_count {
            let entry_off = 10 + i * 12;
            let tag = u16::from_le_bytes([bytes[entry_off], bytes[entry_off + 1]]);
            if tag == 273 {
                strip_offset = Some(u32::from_le_bytes([
                    bytes[entry_off + 8],
                    bytes[entry_off + 9],
                    bytes[entry_off + 10],
                    bytes[entry_off + 11],
                ]));
                break;
            }
        }
        assert_eq!(strip_offset, Some(168), "StripOffsets[0] should be 168");
    }
}
