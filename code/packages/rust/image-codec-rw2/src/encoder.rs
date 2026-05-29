// # encoder.rs — Minimal RW2 test encoder
//
// This encoder writes a synthetic but valid RW2 file. It is NOT a production
// encoder — Panasonic RW2 is a read-only proprietary format. The purpose here
// is to produce test fixtures that the decoder can exercise, verifying the full
// round-trip.
//
// ## What We Emit
//
//   ┌─────────────────────────────────────────────────────┐
//   │ 8-byte RW2 header                                   │
//   ├─────────────────────────────────────────────────────┤
//   │ IFD: 2-byte entry count (10 entries)                │
//   │      10 × 12-byte IFD entries                       │
//   │      4-byte next_ifd = 0                            │
//   ├─────────────────────────────────────────────────────┤
//   │ 12-bit LE packed raw pixel data                     │
//   └─────────────────────────────────────────────────────┘
//
// ## IFD Tags Written
//
// We write these Panasonic private tags:
//   0x0002 SensorWidth       = container.width
//   0x0003 SensorHeight      = container.height
//   0x0004 SensorTopBorder   = 0   (no border — active area = full sensor)
//   0x0005 SensorLeftBorder  = 0
//   0x0006 SensorBottomBorder= height
//   0x0007 SensorRightBorder = width
//   0x0011 RedBalance        = 256 (neutral, 1.0 × 256)
//   0x0012 BlueBalance       = 256 (neutral)
//   0x0024 ImageDepth        = 12  (bits per pixel)
//   0x0097 RawDataOffset     = offset of raw data (computed below)
//
// ## Pixel Encoding
//
// We convert from the PixelContainer's u8 sRGB values back to an approximate
// 12-bit linear value for testing. The goal is purely round-trip fidelity: any
// pixel that goes through encode → decode should produce a similar colour.
//
// For simplicity we use a linear approximation:
//
//   raw_12bit = channel_u8 / 255.0 * (4095 - 240) + 240
//            ≈ channel_u8 * 15 + 240
//
// This is intentionally simple — we just need sensible test data, not a perfect
// inverse of the colour pipeline.

use crate::unpack::row_stride_bytes;
use pixel_container::PixelContainer;

// ---------------------------------------------------------------------------
// IFD entry writing helpers
// ---------------------------------------------------------------------------

/// Append a 12-byte SHORT IFD entry to `out`.
///
/// TIFF type code 3 = SHORT (u16). For a single SHORT value the 4-byte
/// value_or_offset field holds the u16 inline in the first 2 bytes.
fn push_short_entry(out: &mut Vec<u8>, tag: u16, value: u16) {
    // tag (u16 LE)
    out.extend_from_slice(&tag.to_le_bytes());
    // type = 3 (SHORT) (u16 LE)
    out.extend_from_slice(&3u16.to_le_bytes());
    // count = 1 (u32 LE)
    out.extend_from_slice(&1u32.to_le_bytes());
    // value_or_offset: value in low 2 bytes, high 2 bytes = 0
    let mut val_field = [0u8; 4];
    val_field[0..2].copy_from_slice(&value.to_le_bytes());
    out.extend_from_slice(&val_field);
}

/// Append a 12-byte LONG IFD entry to `out`.
///
/// TIFF type code 4 = LONG (u32). For a single LONG value the 4-byte
/// value_or_offset field holds the u32 inline.
fn push_long_entry(out: &mut Vec<u8>, tag: u16, value: u32) {
    // tag (u16 LE)
    out.extend_from_slice(&tag.to_le_bytes());
    // type = 4 (LONG) (u16 LE)
    out.extend_from_slice(&4u16.to_le_bytes());
    // count = 1 (u32 LE)
    out.extend_from_slice(&1u32.to_le_bytes());
    // value inline
    out.extend_from_slice(&value.to_le_bytes());
}

// ---------------------------------------------------------------------------
// Pixel conversion
// ---------------------------------------------------------------------------

/// Convert an 8-bit sRGB channel value to an approximate 12-bit camera-linear
/// value suitable for encoding in the synthetic RW2 raw data.
///
/// Formula: raw ≈ u8_val × 15 + 240   (maps [0, 255] → [240, 4065])
///
/// The exact inverse of our colour pipeline would require inverting sRGB gamma
/// and the colour matrix. For test purposes, this linear approximation is
/// sufficient to verify the encoder/decoder integration.
#[inline]
fn u8_to_raw12(v: u8) -> u16 {
    // Scale into [240, 4095]
    let raw = v as u32 * 15 + 240;
    raw.min(4095) as u16
}

// ---------------------------------------------------------------------------
// Public encoder
// ---------------------------------------------------------------------------

/// Encode a `PixelContainer` into a minimal synthetic RW2 file.
///
/// This is primarily intended for unit tests. The output is a structurally
/// valid RW2 file that `decode_rw2` can parse back.
///
/// The encoding uses R→raw12(R) for R sites, G→raw12(G) for G sites,
/// B→raw12(B) for B sites according to the RGGB Bayer pattern.
pub fn encode_rw2(pixels: &PixelContainer) -> Vec<u8> {
    let width  = pixels.width;
    let height = pixels.height;

    // We build the file in three passes so we know the raw data offset.
    //
    //   Header:   8 bytes
    //   IFD:      2 (entry_count) + 10*12 (entries) + 4 (next_ifd) = 126 bytes
    //   Total before raw data: 8 + 126 = 134 bytes

    let header_ifd_size: u32 = 8 + 2 + 10 * 12 + 4;
    let raw_data_offset = header_ifd_size;

    let mut out = Vec::new();

    // ── 8-byte RW2 header ────────────────────────────────────────────────────
    //
    //   "II"  — little-endian byte order marker
    //   0x0055 LE — RW2 version (85)
    //   offset_to_ifd — always 8 (IFD immediately follows header)
    out.extend_from_slice(b"II");
    out.extend_from_slice(&85u16.to_le_bytes());
    out.extend_from_slice(&8u32.to_le_bytes()); // IFD at offset 8

    // ── IFD: 10 entries ──────────────────────────────────────────────────────
    //
    // TIFF IFD starts with a 2-byte count. Each entry is 12 bytes. The
    // directory ends with a 4-byte next_ifd = 0.
    out.extend_from_slice(&10u16.to_le_bytes()); // entry_count

    push_short_entry(&mut out, 0x0002, width  as u16); // SensorWidth
    push_short_entry(&mut out, 0x0003, height as u16); // SensorHeight
    push_short_entry(&mut out, 0x0004, 0);              // SensorTopBorder
    push_short_entry(&mut out, 0x0005, 0);              // SensorLeftBorder
    push_short_entry(&mut out, 0x0006, height as u16); // SensorBottomBorder
    push_short_entry(&mut out, 0x0007, width  as u16); // SensorRightBorder
    push_short_entry(&mut out, 0x0011, 256);            // RedBalance (neutral)
    push_short_entry(&mut out, 0x0012, 256);            // BlueBalance (neutral)
    push_short_entry(&mut out, 0x0024, 12);             // ImageDepth = 12 bpp
    push_long_entry (&mut out, 0x0097, raw_data_offset);// RawDataOffset

    out.extend_from_slice(&0u32.to_le_bytes()); // next_ifd = 0 (no more IFDs)

    // ── 12-bit packed raw pixel data ─────────────────────────────────────────
    //
    // We emit one Bayer-patterned pixel per sensor position. The RGGB mosaic
    // assigns each site a colour:
    //   (even_row, even_col) → R channel
    //   (even_row, odd_col)  → G channel
    //   (odd_row,  even_col) → G channel
    //   (odd_row,  odd_col)  → B channel
    //
    // We then pack pairs of adjacent raw values into 3-byte little-endian groups.

    let stride = row_stride_bytes(width);
    let mut raw_data = vec![0u8; stride * height as usize];

    // Build a flat array of raw 12-bit values (width × height, row-major).
    let mut raw_pixels = vec![0u16; width as usize * height as usize];
    for y in 0..height {
        for x in 0..width {
            let (r, g, b, _) = pixels.pixel_at(x, y);
            let raw_val = match (y % 2, x % 2) {
                (0, 0) => u8_to_raw12(r), // R site
                (0, 1) => u8_to_raw12(g), // Gr site
                (1, 0) => u8_to_raw12(g), // Gb site
                _      => u8_to_raw12(b), // B site
            };
            raw_pixels[(y as usize) * (width as usize) + (x as usize)] = raw_val;
        }
    }

    // Pack pairs of raw_pixels into 3-byte groups row by row.
    for row in 0..height as usize {
        let row_start  = row * width as usize;
        let byte_start = row * stride;
        let mut col = 0usize;
        let mut byte_idx = byte_start;

        while col < width as usize {
            let p0 = raw_pixels[row_start + col];
            let p1 = if col + 1 < width as usize {
                raw_pixels[row_start + col + 1]
            } else {
                0
            };

            // Pack p0 and p1 into 3 bytes:
            //   byte0 = p0[7:0]
            //   byte1 = p0[11:8] | (p1[3:0] << 4)
            //   byte2 = p1[11:4]
            let b0 = (p0 & 0xFF) as u8;
            let b1 = ((p0 >> 8) & 0x0F) as u8 | (((p1 & 0x0F) << 4) as u8);
            let b2 = ((p1 >> 4) & 0xFF) as u8;

            if byte_idx + 2 < raw_data.len() {
                raw_data[byte_idx]     = b0;
                raw_data[byte_idx + 1] = b1;
                raw_data[byte_idx + 2] = b2;
            }

            col += 2;
            byte_idx += 3;
        }
    }

    out.extend_from_slice(&raw_data);

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_starts_with_rw2_magic() {
        let pixels = PixelContainer::new(4, 4);
        let rw2 = encode_rw2(&pixels);
        assert_eq!(&rw2[0..2], b"II");
        let version = u16::from_le_bytes([rw2[2], rw2[3]]);
        assert_eq!(version, 85);
    }

    #[test]
    fn encoded_ifd_offset_is_8() {
        let pixels = PixelContainer::new(4, 4);
        let rw2 = encode_rw2(&pixels);
        let ifd_offset = u32::from_le_bytes([rw2[4], rw2[5], rw2[6], rw2[7]]);
        assert_eq!(ifd_offset, 8);
    }
}
