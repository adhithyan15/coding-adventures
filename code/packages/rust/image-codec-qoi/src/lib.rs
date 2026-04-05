// # image-codec-qoi
//
// QOI "Quite OK Image Format" encoder and decoder.
// Designed by Dominic Szablewski, 2021. Canonical spec: qoiformat.org.
//
// ## Format Overview
//
// QOI is a lossless, non-streaming image codec. A QOI file is:
//
//   ┌──────────────────────────────┐
//   │  Header      (14 bytes)      │
//   ├──────────────────────────────┤
//   │  Chunks      (variable)      │
//   ├──────────────────────────────┤
//   │  End marker  (8 bytes)       │
//   └──────────────────────────────┘
//
// ## Why QOI?
//
// QOI demonstrates three compression primitives that recur in all major codecs:
//
//   1. Run-length encoding  (QOI_OP_RUN)    — like PackBits, TIFF, fax
//   2. Hash back-references (QOI_OP_INDEX)  — like LZ77 (ZIP, PNG deflate, WebP)
//   3. Delta coding         (QOI_OP_DIFF/LUMA) — like JPEG DC coefficients, PNG filters
//
// ## Encoder/Decoder State
//
// Both encoder and decoder carry identical state across pixels:
//
//   previous_pixel: (r, g, b, a) = (0, 0, 0, 255)
//   hash_table:     [(r, g, b, a); 64] = all zero
//
// The hash function: index = (r*3 + g*5 + b*7 + a*11) % 64
//
// After emitting any non-RUN chunk, the current pixel is written into the
// hash table at its hash index.

use pixel_container::{ImageCodec, PixelContainer};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// QOI magic bytes ("qoif").
const MAGIC: &[u8; 4] = b"qoif";

/// QOI end-of-stream sentinel (8 bytes).
const END_MARKER: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];

// Operation byte tags.
const OP_RGB:   u8 = 0xFE;
const OP_RGBA:  u8 = 0xFF;
const TAG_INDEX: u8 = 0b00;
const TAG_DIFF:  u8 = 0b01;
const TAG_LUMA:  u8 = 0b10;
const TAG_RUN:   u8 = 0b11;

// ---------------------------------------------------------------------------
// QoiCodec
// ---------------------------------------------------------------------------

/// QOI image encoder and decoder.
pub struct QoiCodec;

impl ImageCodec for QoiCodec {
    fn mime_type(&self) -> &'static str {
        "image/qoi"
    }

    fn encode(&self, container: &PixelContainer) -> Vec<u8> {
        encode_qoi_impl(container)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_qoi_impl(bytes)
    }
}

// ---------------------------------------------------------------------------
// Convenience functions
// ---------------------------------------------------------------------------

/// Encode a `PixelContainer` to QOI bytes.
///
/// # Examples
///
/// ```
/// use pixel_container::PixelContainer;
/// use image_codec_qoi::encode_qoi;
///
/// let mut buf = PixelContainer::new(4, 4);
/// buf.fill(128, 0, 128, 255);
/// let qoi = encode_qoi(&buf);
/// assert_eq!(&qoi[0..4], b"qoif");
/// ```
pub fn encode_qoi(container: &PixelContainer) -> Vec<u8> {
    encode_qoi_impl(container)
}

/// Decode QOI bytes into a `PixelContainer`.
///
/// # Errors
///
/// Returns `Err` if the bytes are not valid QOI format.
///
/// # Examples
///
/// ```
/// use pixel_container::PixelContainer;
/// use image_codec_qoi::{encode_qoi, decode_qoi};
///
/// let mut buf = PixelContainer::new(8, 8);
/// buf.fill(100, 150, 200, 255);
/// let encoded = encode_qoi(&buf);
/// let decoded = decode_qoi(&encoded).unwrap();
/// assert_eq!(decoded.pixel_at(3, 3), (100, 150, 200, 255));
/// ```
pub fn decode_qoi(bytes: &[u8]) -> Result<PixelContainer, String> {
    decode_qoi_impl(bytes)
}

// ---------------------------------------------------------------------------
// Hash function
// ---------------------------------------------------------------------------

/// QOI pixel hash: maps an RGBA pixel to a slot in the 64-entry hash table.
///
/// This is a fast mixing function, not a cryptographic hash. It spreads pixel
/// values across 64 buckets so that frequently repeated pixels can be
/// referenced by index in a single byte (QOI_OP_INDEX).
#[inline]
fn hash(r: u8, g: u8, b: u8, a: u8) -> usize {
    ((r as usize * 3) + (g as usize * 5) + (b as usize * 7) + (a as usize * 11)) % 64
}

// ---------------------------------------------------------------------------
// Encoder
// ---------------------------------------------------------------------------

fn encode_qoi_impl(c: &PixelContainer) -> Vec<u8> {
    let mut out = Vec::with_capacity(14 + c.width as usize * c.height as usize * 4 + 8);

    // --- Header (14 bytes, big-endian) ---
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&c.width.to_be_bytes());
    out.extend_from_slice(&c.height.to_be_bytes());
    out.push(4); // channels: 4 = RGBA
    out.push(0); // colorspace: 0 = sRGB

    // --- Encoding state ---
    let mut hash_table = [(0u8, 0u8, 0u8, 0u8); 64];
    let mut prev = (0u8, 0u8, 0u8, 255u8); // initial pixel: opaque black
    let mut run: u8 = 0;

    for y in 0..c.height {
        for x in 0..c.width {
            let (r, g, b, a) = c.pixel_at(x, y);
            let curr = (r, g, b, a);

            if curr == prev {
                // Run of identical pixels. Max run = 62.
                run += 1;
                if run == 62 {
                    // Flush: QOI_OP_RUN encodes run−1 in the low 6 bits.
                    out.push((TAG_RUN << 6) | (run - 1));
                    run = 0;
                }
                continue; // don't update hash or prev on a run
            }

            // Flush any pending run.
            if run > 0 {
                out.push((TAG_RUN << 6) | (run - 1));
                run = 0;
            }

            // Try QOI_OP_INDEX: check if this pixel is already in the hash table.
            let idx = hash(r, g, b, a);
            if hash_table[idx] == curr {
                out.push((TAG_INDEX << 6) | idx as u8);
            } else {
                // Update hash table regardless of which op we choose.
                hash_table[idx] = curr;

                // Compute per-channel deltas (wrapping i8 arithmetic).
                // Rust wrapping_sub on u8 gives the same bit pattern as
                // interpreting the result as a signed delta.
                let dr = r.wrapping_sub(prev.0) as i8;
                let dg = g.wrapping_sub(prev.1) as i8;
                let db = b.wrapping_sub(prev.2) as i8;

                if a == prev.3 {
                    // Alpha unchanged — try compact delta ops first.
                    if dr >= -2 && dr <= 1 && dg >= -2 && dg <= 1 && db >= -2 && db <= 1 {
                        // QOI_OP_DIFF: 1 byte, each delta biased by +2 to fit in 2 bits.
                        let byte = (TAG_DIFF << 6)
                            | (((dr + 2) as u8) << 4)
                            | (((dg + 2) as u8) << 2)
                            | ((db + 2) as u8);
                        out.push(byte);
                    } else if dg >= -32 && dg <= 31
                        && (dr - dg) >= -8 && (dr - dg) <= 7
                        && (db - dg) >= -8 && (db - dg) <= 7
                    {
                        // QOI_OP_LUMA: 2 bytes.
                        // Byte 0: tag + dg biased by +32
                        // Byte 1: (dr−dg) biased by +8 in high nibble,
                        //         (db−dg) biased by +8 in low nibble
                        let dr_dg = dr - dg;
                        let db_dg = db - dg;
                        out.push((TAG_LUMA << 6) | ((dg + 32) as u8));
                        out.push((((dr_dg + 8) as u8) << 4) | ((db_dg + 8) as u8));
                    } else {
                        // QOI_OP_RGB: fall back to full RGB.
                        out.push(OP_RGB);
                        out.push(r);
                        out.push(g);
                        out.push(b);
                    }
                } else {
                    // Alpha changed — must use QOI_OP_RGBA.
                    out.push(OP_RGBA);
                    out.push(r);
                    out.push(g);
                    out.push(b);
                    out.push(a);
                }
            }

            prev = curr;
        }
    }

    // Flush any remaining run.
    if run > 0 {
        out.push((TAG_RUN << 6) | (run - 1));
    }

    // End marker.
    out.extend_from_slice(&END_MARKER);
    out
}

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

fn decode_qoi_impl(bytes: &[u8]) -> Result<PixelContainer, String> {
    // Minimum: 14-byte header + 8-byte end marker = 22 bytes.
    if bytes.len() < 22 {
        return Err("QOI: file too short".into());
    }

    // Verify magic.
    if &bytes[0..4] != MAGIC {
        return Err("QOI: invalid magic".into());
    }

    // Read header (big-endian u32 for width/height).
    let width  = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let height = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    // bytes[12] = channels, bytes[13] = colorspace (informational only)

    if width == 0 || height == 0 {
        return Err("QOI: invalid dimensions".into());
    }

    // Use checked arithmetic to prevent u32 overflow in release mode.
    let total_pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or("QOI: image dimensions overflow")?;
    let data_cap = total_pixels.checked_mul(4)
        .ok_or("QOI: image dimensions overflow")?;
    let mut data = Vec::with_capacity(data_cap);

    // Decode state.
    let mut hash_table = [(0u8, 0u8, 0u8, 0u8); 64];
    let mut prev = (0u8, 0u8, 0u8, 255u8); // opaque black

    let mut pos = 14usize; // start after header
    let mut pixels_written = 0usize;

    while pixels_written < total_pixels {
        if pos >= bytes.len() {
            return Err("QOI: unexpected end of data".into());
        }

        let tag = bytes[pos];
        pos += 1;

        let curr: (u8, u8, u8, u8);

        if tag == OP_RGB {
            // QOI_OP_RGB: 3 more bytes (R, G, B), alpha unchanged.
            if pos + 3 > bytes.len() {
                return Err("QOI: unexpected end of data".into());
            }
            curr = (bytes[pos], bytes[pos + 1], bytes[pos + 2], prev.3);
            pos += 3;
        } else if tag == OP_RGBA {
            // QOI_OP_RGBA: 4 more bytes (R, G, B, A).
            if pos + 4 > bytes.len() {
                return Err("QOI: unexpected end of data".into());
            }
            curr = (bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]);
            pos += 4;
        } else {
            match tag >> 6 {
                t if t == TAG_INDEX => {
                    // QOI_OP_INDEX: look up the hash table.
                    let idx = (tag & 0x3F) as usize;
                    curr = hash_table[idx];
                    // Note: do NOT update hash_table for INDEX — the slot
                    // already has this pixel.
                    data.push(curr.0);
                    data.push(curr.1);
                    data.push(curr.2);
                    data.push(curr.3);
                    pixels_written += 1;
                    prev = curr;
                    continue;
                }
                t if t == TAG_DIFF => {
                    // QOI_OP_DIFF: 2-bit deltas biased by +2.
                    let dr = ((tag >> 4) & 0x3) as i8 - 2;
                    let dg = ((tag >> 2) & 0x3) as i8 - 2;
                    let db = ((tag >> 0) & 0x3) as i8 - 2;
                    // Apply deltas with wrapping u8 arithmetic.
                    curr = (
                        prev.0.wrapping_add(dr as u8),
                        prev.1.wrapping_add(dg as u8),
                        prev.2.wrapping_add(db as u8),
                        prev.3,
                    );
                }
                t if t == TAG_LUMA => {
                    // QOI_OP_LUMA: dg in low 6 bits (biased +32), then a
                    // second byte with dr-dg and db-dg (each biased +8).
                    if pos >= bytes.len() {
                        return Err("QOI: unexpected end of data".into());
                    }
                    let next = bytes[pos];
                    pos += 1;
                    let dg: i8    = (tag & 0x3F) as i8 - 32;
                    let dr_dg: i8 = ((next >> 4) & 0xF) as i8 - 8;
                    let db_dg: i8 = ((next >> 0) & 0xF) as i8 - 8;
                    let dr = dr_dg + dg;
                    let db = db_dg + dg;
                    curr = (
                        prev.0.wrapping_add(dr as u8),
                        prev.1.wrapping_add(dg as u8),
                        prev.2.wrapping_add(db as u8),
                        prev.3,
                    );
                }
                _ => {
                    // QOI_OP_RUN: repeat previous pixel (run_length+1) times.
                    let run = ((tag & 0x3F) + 1) as usize;
                    let remaining = total_pixels - pixels_written;
                    let actual_run = run.min(remaining);
                    for _ in 0..actual_run {
                        data.push(prev.0);
                        data.push(prev.1);
                        data.push(prev.2);
                        data.push(prev.3);
                    }
                    pixels_written += actual_run;
                    // prev is unchanged; hash_table is not updated for runs.
                    continue;
                }
            }
        }

        // Update hash table and emit pixel for RGB, RGBA, DIFF, LUMA.
        hash_table[hash(curr.0, curr.1, curr.2, curr.3)] = curr;
        data.push(curr.0);
        data.push(curr.1);
        data.push(curr.2);
        data.push(curr.3);
        pixels_written += 1;
        prev = curr;
    }

    // Verify end marker.
    if pos + 8 > bytes.len() || &bytes[pos..pos + 8] != END_MARKER {
        return Err("QOI: missing end marker".into());
    }

    Ok(PixelContainer::from_data(width, height, data))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pixel_container::PixelContainer;

    // Helper: fill a container with a solid colour.
    fn solid(w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) -> PixelContainer {
        let mut buf = PixelContainer::new(w, h);
        buf.fill(r, g, b, a);
        buf
    }

    // --- Header ---

    #[test]
    fn encoded_starts_with_magic() {
        let qoi = encode_qoi(&solid(4, 4, 0, 0, 0, 255));
        assert_eq!(&qoi[0..4], b"qoif");
    }

    #[test]
    fn encoded_width_height_big_endian() {
        let qoi = encode_qoi(&solid(640, 480, 0, 0, 0, 255));
        let w = u32::from_be_bytes([qoi[4], qoi[5], qoi[6], qoi[7]]);
        let h = u32::from_be_bytes([qoi[8], qoi[9], qoi[10], qoi[11]]);
        assert_eq!(w, 640);
        assert_eq!(h, 480);
    }

    #[test]
    fn encoded_ends_with_end_marker() {
        let qoi = encode_qoi(&solid(2, 2, 10, 20, 30, 255));
        let n = qoi.len();
        assert_eq!(&qoi[n - 8..], &END_MARKER);
    }

    // --- Round-trip lossless ---

    #[test]
    fn round_trip_solid_colour() {
        let original = solid(8, 8, 200, 100, 50, 255);
        let decoded = decode_qoi(&encode_qoi(&original)).unwrap();
        assert_eq!(decoded.width,  original.width);
        assert_eq!(decoded.height, original.height);
        assert_eq!(decoded.data,   original.data);
    }

    #[test]
    fn round_trip_with_transparency() {
        let mut original = PixelContainer::new(4, 4);
        for y in 0..4u32 {
            for x in 0..4u32 {
                original.set_pixel(x, y, x as u8 * 64, y as u8 * 64, 128, (x + y) as u8 * 32);
            }
        }
        let decoded = decode_qoi(&encode_qoi(&original)).unwrap();
        assert_eq!(decoded.data, original.data);
    }

    #[test]
    fn round_trip_gradient() {
        // Image where each pixel is incrementally different — exercises delta ops.
        let mut original = PixelContainer::new(16, 1);
        for x in 0..16u32 {
            original.set_pixel(x, 0, x as u8 * 16, 128, 128, 255);
        }
        let decoded = decode_qoi(&encode_qoi(&original)).unwrap();
        assert_eq!(decoded.data, original.data);
    }

    #[test]
    fn round_trip_checkerboard() {
        let mut original = PixelContainer::new(8, 8);
        for y in 0..8u32 {
            for x in 0..8u32 {
                if (x + y) % 2 == 0 {
                    original.set_pixel(x, y, 255, 255, 255, 255);
                } else {
                    original.set_pixel(x, y, 0, 0, 0, 255);
                }
            }
        }
        let decoded = decode_qoi(&encode_qoi(&original)).unwrap();
        assert_eq!(decoded.data, original.data);
    }

    // --- Compression: solid-colour images should be tiny ---

    #[test]
    fn solid_colour_is_compressed() {
        // A 100×100 solid red image should encode to far fewer than 100*100*4 bytes.
        let qoi = encode_qoi(&solid(100, 100, 255, 0, 0, 255));
        // Raw size = 40000 bytes. With RUN encoding, it should be << 1000 bytes.
        assert!(qoi.len() < 1000, "expected compressed size < 1000, got {}", qoi.len());
    }

    // --- Hash function ---

    #[test]
    fn hash_is_in_range() {
        for r in [0u8, 127, 255] {
            for g in [0u8, 127, 255] {
                for b in [0u8, 127, 255] {
                    for a in [0u8, 255] {
                        assert!(hash(r, g, b, a) < 64);
                    }
                }
            }
        }
    }

    // --- Decode errors ---

    #[test]
    fn decode_too_short_returns_error() {
        let result = decode_qoi(&[0x71, 0x6F, 0x69, 0x66]); // only "qoif"
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too short"));
    }

    #[test]
    fn decode_wrong_magic_returns_error() {
        let mut qoi = encode_qoi(&solid(2, 2, 0, 0, 0, 255));
        qoi[0] = 0xFF; // corrupt magic
        let result = decode_qoi(&qoi);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid magic"));
    }

    #[test]
    fn decode_zero_dimensions_returns_error() {
        // Build a header with width=0.
        let mut header = vec![
            b'q', b'o', b'i', b'f',
            0, 0, 0, 0,  // width = 0
            0, 0, 0, 1,  // height = 1
            4, 0,        // channels, colorspace
        ];
        header.extend_from_slice(&END_MARKER);
        let result = decode_qoi(&header);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid dimensions"));
    }

    // --- ImageCodec trait ---

    #[test]
    fn codec_mime_type() {
        assert_eq!(QoiCodec.mime_type(), "image/qoi");
    }

    #[test]
    fn codec_encode_decode_via_trait() {
        let original = solid(5, 5, 1, 2, 3, 255);
        let encoded = QoiCodec.encode(&original);
        let decoded  = QoiCodec.decode(&encoded).unwrap();
        assert_eq!(decoded.data, original.data);
    }
}
