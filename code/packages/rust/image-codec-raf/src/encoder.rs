// # encoder.rs — Minimal RAF test encoder
//
// This encoder writes just enough of the RAF binary format to produce files
// that `decode_raf` can round-trip.  It is NOT a production encoder — it
// produces synthetic files with no embedded JPEG and a simplified CFA header,
// which is fine for unit testing but would not be accepted by Fujifilm's own
// software.
//
// ## What the encoder writes
//
// 1. 116-byte outer header with correct magic and correct region offsets.
// 2. A zero-length JPEG section (offset points at the CFA header, length 0).
// 3. CFA header block containing three tags:
//    - 0x0100: image size (width × height, u16 BE)
//    - 0x0110: raw image size (same as image size)
//    - 0x0111: RGGB Bayer pattern (4 bytes, values 0/1/1/2)
//    - 0x0130: neutral WB (R=1024, G=1024, B=1024, u32 LE each)
//    - 0x0141: zero black levels (4 × u32 LE = 0)
//    - 0x0142: white level = 4095 (u32 LE)
// 4. 12-bit big-endian packed pixel data derived from the PixelContainer.
//
// ## Pixel encoding strategy
//
// The PixelContainer holds 8-bit sRGB pixels.  To produce a mosaic the
// encoder:
//   1. Converts each pixel to a single 12-bit grey value by averaging R, G, B
//      and scaling to [0, 4095].
//   2. Lays out the raw grid using the RGGB Bayer pattern, assigning each
//      grid cell the encoded value of the colour channel it represents in the
//      source pixel.
//
// This approach does *not* invert the full colour pipeline (that would require
// the inverse colour matrix and inverse gamma), but it produces a consistent
// round-trip for solid-colour test images where all pixels have the same hue.

use pixel_container::PixelContainer;

use crate::unpack::pack_12bit_be;

/// Build a synthetic RAF byte stream from a `PixelContainer`.
///
/// The output is accepted by `decode_raf` and can be used for round-trip
/// tests.  The encoding is intentionally simple: RGGB Bayer pattern with
/// neutral WB and zero black level.
pub fn encode_raf(pixels: &PixelContainer) -> Vec<u8> {
    let w = pixels.width as usize;
    let h = pixels.height as usize;

    // ── Build CFA header block ────────────────────────────────────────────────
    let cfa_header = build_cfa_header(w, h);

    // ── Build raw pixel data ─────────────────────────────────────────────────
    let raw_pixels = build_raw_pixels(pixels);
    let packed = pack_12bit_be(&raw_pixels);

    // ── Compute region offsets ───────────────────────────────────────────────
    // Layout: [outer header (116)] [CFA header] [CFA pixels]
    // There is no embedded JPEG (length 0, offset = start of CFA header).
    let outer_header_size = 116usize;
    let jpeg_offset  = outer_header_size;          // points at CFA header
    let jpeg_length  = 0usize;                      // no JPEG
    let cfa_hdr_offset = outer_header_size;
    let cfa_hdr_length = cfa_header.len();
    let cfa_pix_offset = cfa_hdr_offset + cfa_hdr_length;
    let cfa_pix_length = packed.len();

    // ── Assemble file ─────────────────────────────────────────────────────────
    let mut out = Vec::with_capacity(outer_header_size + cfa_hdr_length + cfa_pix_length);

    // --- outer header (116 bytes) ---
    // [0..16]   magic
    out.extend_from_slice(b"FUJIFILMCCD-RAW ");
    // [16..20]  format version
    out.extend_from_slice(b"0100");
    // [20..28]  camera model ID (8 bytes, NUL-padded)
    out.extend_from_slice(b"TESTCAM\x00");
    // [28..60]  camera model string (32 bytes, NUL-padded)
    let mut cam_str = [0u8; 32];
    cam_str[..15].copy_from_slice(b"TestCamera 0.1\x00");
    out.extend_from_slice(&cam_str);
    // [60..64]  directory version
    out.extend_from_slice(b"0100");
    // [64..84]  reserved (20 bytes, all zero)
    out.extend_from_slice(&[0u8; 20]);
    // [84..88]  JPEG offset (u32 BE)
    out.extend_from_slice(&(jpeg_offset as u32).to_be_bytes());
    // [88..92]  JPEG length (u32 BE)
    out.extend_from_slice(&(jpeg_length as u32).to_be_bytes());
    // [92..96]  CFA header offset (u32 BE)
    out.extend_from_slice(&(cfa_hdr_offset as u32).to_be_bytes());
    // [96..100] CFA header length (u32 BE)
    out.extend_from_slice(&(cfa_hdr_length as u32).to_be_bytes());
    // [100..104] CFA pixel offset (u32 BE)
    out.extend_from_slice(&(cfa_pix_offset as u32).to_be_bytes());
    // [104..108] CFA pixel length (u32 BE)
    out.extend_from_slice(&(cfa_pix_length as u32).to_be_bytes());
    // [108..112] second CFA offset (u32 BE, 0 = unused)
    out.extend_from_slice(&0u32.to_be_bytes());
    // [112..116] second CFA length (u32 BE, 0 = unused)
    out.extend_from_slice(&0u32.to_be_bytes());

    assert_eq!(out.len(), outer_header_size, "outer header must be exactly 116 bytes");

    // --- CFA header ---
    out.extend_from_slice(&cfa_header);
    // --- raw pixel data ---
    out.extend_from_slice(&packed);

    out
}

// ── private helpers ───────────────────────────────────────────────────────────

/// Build the CFA header block with six tag entries.
fn build_cfa_header(w: usize, h: usize) -> Vec<u8> {
    let mut buf = Vec::new();

    // Helper closure: write a tag block.
    let mut write_tag = |tag: u16, value: &[u8]| {
        buf.extend_from_slice(&tag.to_be_bytes());
        buf.extend_from_slice(&(value.len() as u16).to_be_bytes());
        buf.extend_from_slice(value);
    };

    // 0x0100: displayed image size (u16 BE width, u16 BE height)
    let mut sz = [0u8; 4];
    sz[0..2].copy_from_slice(&(w as u16).to_be_bytes());
    sz[2..4].copy_from_slice(&(h as u16).to_be_bytes());
    write_tag(0x0100, &sz);

    // 0x0110: raw image size (same as display size for our synthetic files)
    write_tag(0x0110, &sz);

    // 0x0111: RGGB Bayer pattern (4 bytes: 0=R, 1=G, 1=G, 2=B)
    write_tag(0x0111, &[0u8, 1, 1, 2]);

    // 0x0130: auto WB (R=1024, G=1024, B=1024, u32 LE each)
    let wb: u32 = 1024;
    let mut wb_bytes = [0u8; 12];
    wb_bytes[0..4].copy_from_slice(&wb.to_le_bytes());
    wb_bytes[4..8].copy_from_slice(&wb.to_le_bytes());
    wb_bytes[8..12].copy_from_slice(&wb.to_le_bytes());
    write_tag(0x0130, &wb_bytes);

    // 0x0141: black levels (4× u32 LE = 0)
    write_tag(0x0141, &[0u8; 16]);

    // 0x0142: white level = 4095 (u32 LE)
    write_tag(0x0142, &4095u32.to_le_bytes());

    buf
}

/// Convert `PixelContainer` RGBA pixels into 12-bit raw Bayer mosaic values.
///
/// For each pixel `(x, y)`, determine which Bayer channel it maps to (RGGB),
/// then extract that channel from the source pixel and scale from [0, 255]
/// to [0, 4095].
///
/// This is a simplified forward direction: the full round-trip is not
/// bit-perfect (due to demosaicing interpolation) but is close enough for
/// solid-colour test images.
fn build_raw_pixels(pixels: &PixelContainer) -> Vec<u16> {
    let w = pixels.width as usize;
    let h = pixels.height as usize;
    let mut raw = Vec::with_capacity(w * h);

    // RGGB pattern: (row%2, col%2) → (0,0)=R, (0,1)=G, (1,0)=G, (1,1)=B
    for y in 0..h {
        for x in 0..w {
            let (r, g, b, _) = pixels.pixel_at(x as u32, y as u32);
            let ch_val: u8 = match (y % 2, x % 2) {
                (0, 0) => r, // R pixel
                (0, 1) => g, // G pixel (top-right)
                (1, 0) => g, // G pixel (bottom-left)
                (1, 1) => b, // B pixel
                _      => g, // unreachable, but satisfy the compiler
            };
            // Scale 8-bit [0, 255] → 12-bit [0, 4095].
            // Use u32 intermediate to avoid overflow (255 * 4095 > u16::MAX).
            let val_12bit = ((ch_val as u32) * 4095 / 255) as u16;
            raw.push(val_12bit);
        }
    }

    raw
}
