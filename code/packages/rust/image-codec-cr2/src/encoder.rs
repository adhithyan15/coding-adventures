// # encoder.rs — Minimal Synthetic CR2 Encoder
//
// This encoder exists solely for round-trip testing. It does NOT produce
// files that Canon cameras can write to or that Canon software can read —
// CR2 is a proprietary format and this encoder only covers the minimum
// needed to test the decoder.
//
// ## Why Not Reuse image_codec_tiff::encode_tiff?
//
// `image_codec_tiff::encode_tiff` always places IFD0 at byte offset 8 —
// the standard TIFF layout. However, CR2 requires its 4-byte signature
// ("CR\x02\x00") at bytes 8–11 of the file. This means IFD0 can't start
// at byte 8; it must be pushed back to byte 16 (after the signature area).
//
// Rather than patching the TIFF encoder's output (which would corrupt the
// IFD because the data at offset 8 is both the CR2 sig and the IFD start),
// we write a custom TIFF with IFD0 at offset 16.
//
// ## Output Layout
//
// ```text
// Offset  Size  Content
//  0       2    "II" — TIFF little-endian marker
//  2       2    42   — TIFF magic
//  4       4    16   — IFD0 file offset (points past the CR2 sig area)
//  8       2    "CR" — CR2 identifier
// 10       1    0x02 — CR2 major version
// 11       1    0x00 — CR2 minor version
// 12       4    0    — padding (or could be IFD3 strip offset high word)
// 16       N    IFD0 entries + external data + pixel data
// ```
//
// The IFD itself is identical to a standard TIFF except all file offsets
// within it are shifted by 8 bytes (because the header is 16 bytes instead
// of 8 bytes).

use pixel_container::PixelContainer;

// ─── Write helpers ────────────────────────────────────────────────────────────

fn w16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn w32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// Write a 12-byte IFD entry with an inline SHORT value.
///
/// SHORT = type 3, size 2 bytes. count=1 → 2 bytes ≤ 4 → inline.
fn short_entry(buf: &mut Vec<u8>, tag: u16, value: u16) {
    w16(buf, tag);
    w16(buf, 3);     // type = SHORT
    w32(buf, 1);     // count = 1
    w16(buf, value);
    w16(buf, 0);     // padding
}

/// Write a 12-byte IFD entry with an inline LONG value.
///
/// LONG = type 4, size 4 bytes. count=1 → 4 bytes ≤ 4 → inline.
fn long_entry(buf: &mut Vec<u8>, tag: u16, value: u32) {
    w16(buf, tag);
    w16(buf, 4);     // type = LONG
    w32(buf, 1);     // count = 1
    w32(buf, value);
}

/// Write a 12-byte IFD entry pointing to external data at `offset`.
fn ext_entry(buf: &mut Vec<u8>, tag: u16, typ: u16, count: u32, offset: u32) {
    w16(buf, tag);
    w16(buf, typ);
    w32(buf, count);
    w32(buf, offset);
}

// ─── Encoder ─────────────────────────────────────────────────────────────────

/// Encode a `PixelContainer` as a minimal synthetic CR2 file.
///
/// The output is a valid CR2 header (TIFF LE + CR2 signature at offset 8)
/// with IFD0 at byte 16, wrapping uncompressed 8-bit RGB pixel data.
///
/// Use this only for round-trip tests — not for production CR2 output.
pub fn encode_cr2(pixels: &PixelContainer) -> Vec<u8> {
    let width = pixels.width;
    let height = pixels.height;
    let pixel_bytes = (width as usize) * (height as usize) * 3;

    // ── File layout offsets ───────────────────────────────────────────────────
    //
    // The 16-byte CR2 header is:
    //   0..8  — standard TIFF header (byte order + magic + IFD0 offset)
    //   8..16 — CR2 signature area ("CR\x02\x00" + 4 bytes padding)
    //
    // IFD starts at 16 (= CR2_HEADER_SIZE).
    // We write 11 IFD entries (same as image_codec_tiff::encode_tiff).
    //
    // IFD section:
    //   16      → 2 bytes: entry count (11)
    //   18      → 11 × 12 = 132 bytes: entries
    //   150     → 4 bytes: next IFD offset (0)
    //   154     → IFD end
    //
    // External data:
    //   154     → 6 bytes: BitsPerSample [8, 8, 8]
    //   160     → 8 bytes: XResolution 72/1
    //   168     → 8 bytes: YResolution 72/1
    //   176     → pixel data start
    //
    const CR2_HEADER: u32 = 16;       // file bytes 0..16
    const NUM_ENTRIES: u16 = 11;
    const IFD_SECTION: u32 = 2 + (NUM_ENTRIES as u32 * 12) + 4; // 138
    const IFD_END: u32 = CR2_HEADER + IFD_SECTION;              // 154

    const BITS_OFF: u32 = IFD_END;          // 154 — BitsPerSample
    const XRES_OFF: u32 = BITS_OFF + 6;     // 160 — XResolution
    const YRES_OFF: u32 = XRES_OFF + 8;     // 168 — YResolution
    const PIXEL_OFF: u32 = YRES_OFF + 8;    // 176 — pixel data

    let mut buf = Vec::with_capacity(PIXEL_OFF as usize + pixel_bytes);

    // ── 16-byte CR2 / TIFF header ─────────────────────────────────────────────
    buf.extend_from_slice(b"II");   // little-endian TIFF marker
    w16(&mut buf, 42);              // TIFF magic
    w32(&mut buf, CR2_HEADER);      // IFD0 at byte 16 (past CR2 sig area)

    // CR2 signature at offset 8:
    buf.push(b'C');   // 0x43
    buf.push(b'R');   // 0x52
    buf.push(2);      // CR2 major version
    buf.push(0);      // CR2 minor version

    // 4 bytes padding (bytes 12–15). Real Canon files store IFD3 strip offset
    // high word here, but we don't need it for tests.
    w32(&mut buf, 0);

    // ── IFD0 ─────────────────────────────────────────────────────────────────
    w16(&mut buf, NUM_ENTRIES);

    // Tags in ascending order (required by TIFF spec §2).
    long_entry(&mut buf, 256, width);                        // ImageWidth
    long_entry(&mut buf, 257, height);                       // ImageLength
    ext_entry(&mut buf, 258, 3, 3, BITS_OFF);                // BitsPerSample → ext
    short_entry(&mut buf, 259, 1);                           // Compression = uncompressed
    short_entry(&mut buf, 262, 2);                           // PhotometricInterp = RGB
    long_entry(&mut buf, 273, PIXEL_OFF);                    // StripOffsets (inline)
    short_entry(&mut buf, 277, 3);                           // SamplesPerPixel = 3
    long_entry(&mut buf, 278, height);                       // RowsPerStrip
    long_entry(&mut buf, 279, pixel_bytes as u32);           // StripByteCounts (inline)
    ext_entry(&mut buf, 282, 5, 1, XRES_OFF);                // XResolution → ext
    ext_entry(&mut buf, 283, 5, 1, YRES_OFF);                // YResolution → ext

    w32(&mut buf, 0); // next IFD = 0 (end of chain)

    // ── External data ─────────────────────────────────────────────────────────
    debug_assert_eq!(buf.len(), BITS_OFF as usize, "BitsPerSample offset mismatch");
    w16(&mut buf, 8); w16(&mut buf, 8); w16(&mut buf, 8); // [8,8,8]

    debug_assert_eq!(buf.len(), XRES_OFF as usize, "XResolution offset mismatch");
    w32(&mut buf, 72); w32(&mut buf, 1); // 72/1

    debug_assert_eq!(buf.len(), YRES_OFF as usize, "YResolution offset mismatch");
    w32(&mut buf, 72); w32(&mut buf, 1); // 72/1

    // ── Pixel data ────────────────────────────────────────────────────────────
    debug_assert_eq!(buf.len(), PIXEL_OFF as usize, "Pixel data offset mismatch");
    for chunk in pixels.data.chunks_exact(4) {
        buf.push(chunk[0]); // R
        buf.push(chunk[1]); // G
        buf.push(chunk[2]); // B
        // chunk[3] = A — dropped (TIFF is opaque)
    }

    buf
}
