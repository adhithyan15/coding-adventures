// # decoder.rs — Top-level RW2 decode pipeline
//
// This module ties together all the sub-modules:
//
//   1. Validate magic bytes (`header::check_magic`)
//   2. Parse IFD tags (`header::parse_ifd`)
//   3. Validate ImageDepth (only 12-bit supported in v0.1)
//   4. Detect Panasonic lossless compression and return a friendly Err
//   5. Unpack 12-bit LE pixel data (`unpack::unpack_12bit_le`)
//   6. Crop to the active sensor area (remove optical-black borders)
//   7. Bayer demosaicing (`bayer::demosaic_rggb`)
//   8. White balance + colour matrix + sRGB gamma (`color::apply_color_pipeline`)
//   9. Build PixelContainer (RGBA8, A=255)
//
// ## Security
//
// * Sensor dimensions are capped at 4096×4096 (hard) to prevent huge allocs.
// * The raw data offset and byte count are bounds-checked before slicing.
// * All pixel buffer size calculations use checked arithmetic.

use crate::bayer::demosaic_rggb;
use crate::color::{apply_color_pipeline, white_balance_from_tags, PANASONIC_COLOR_MATRIX};
use crate::header::{check_magic, parse_ifd};
use crate::unpack::{row_stride_bytes, unpack_12bit_le};
use pixel_container::PixelContainer;

/// Maximum sensor dimension we accept (width or height).
///
/// A real Panasonic Lumix GH6 sensor is 5728×4296. Capping at 4096 is
/// conservative but safe for a v0.1 decoder — it prevents gigabyte-scale allocs
/// from malformed files.
const MAX_SENSOR_DIM: u32 = 4096;

/// Typical black level for 12-bit Panasonic RW2 files.
const BLACK_LEVEL_12BIT: u32 = 240;

/// Maximum 12-bit pixel value (2^12 − 1).
const WHITE_LEVEL_12BIT: u32 = 4095;

/// Decode a Panasonic RW2 file from a byte slice into a PixelContainer.
///
/// ## Errors
///
/// Returns `Err(String)` with a descriptive message for any of:
/// - File too short or wrong magic.
/// - IFD parsing failure.
/// - Missing required tags (sensor width/height).
/// - 16-bit ImageDepth (not supported in v0.1).
/// - Panasonic lossless compression detected (not supported in v0.1).
/// - Sensor dimensions exceeding `MAX_SENSOR_DIM`.
/// - Raw data offset out of file bounds.
pub fn decode_rw2(bytes: &[u8]) -> Result<PixelContainer, String> {
    // ── Step 1: Check the 8-byte RW2 magic ───────────────────────────────────
    let ifd_offset = check_magic(bytes)?;

    // ── Step 2: Parse the IFD for Panasonic private tags ─────────────────────
    let ifd = parse_ifd(bytes, ifd_offset)?;

    // ── Step 3: Extract required dimensions ──────────────────────────────────
    let sensor_width = ifd
        .sensor_width
        .ok_or("RW2: missing SensorWidth tag (0x0002)")?;
    let sensor_height = ifd
        .sensor_height
        .ok_or("RW2: missing SensorHeight tag (0x0003)")?;

    // Safety: reject absurdly large sensors that would exhaust RAM.
    if sensor_width > MAX_SENSOR_DIM {
        return Err(format!(
            "RW2: SensorWidth {sensor_width} exceeds maximum {MAX_SENSOR_DIM}"
        ));
    }
    if sensor_height > MAX_SENSOR_DIM {
        return Err(format!(
            "RW2: SensorHeight {sensor_height} exceeds maximum {MAX_SENSOR_DIM}"
        ));
    }

    // ── Step 4: Check ImageDepth ──────────────────────────────────────────────
    // We support 12-bit packed. 16-bit requires a different unpack scheme.
    if let Some(depth) = ifd.image_depth {
        if depth == 16 {
            return Err("RW2: 16-bit ImageDepth not supported in v0.1".into());
        }
    }
    // If ImageDepth is missing, assume 12-bit (most common case).

    // ── Step 5: Locate the raw pixel strip ────────────────────────────────────
    let raw_offset = ifd
        .raw_data_offset
        .ok_or("RW2: could not locate raw pixel data (tags 0x0097 / StripOffsets both missing)")?;
    let raw_offset = raw_offset as usize;

    // Compute the expected uncompressed byte count:
    //   stride × sensor_height
    let stride = row_stride_bytes(sensor_width);
    let expected_bytes = stride
        .checked_mul(sensor_height as usize)
        .ok_or("RW2: sensor dimensions overflow usize")?;

    // Validate that the raw offset is within the file.
    if raw_offset >= bytes.len() {
        return Err(format!(
            "RW2: raw data offset {raw_offset} is beyond file end ({})",
            bytes.len()
        ));
    }

    let available = bytes.len() - raw_offset;

    // ── Step 6: Detect Panasonic lossless ─────────────────────────────────────
    // If the available byte count is less than 80% of the expected uncompressed
    // size, we assume the file uses Panasonic lossless compression (row-by-row
    // variable-length). v0.1 does not implement the decompressor.
    if available < expected_bytes * 4 / 5 {
        return Err(format!(
            "RW2: Panasonic lossless compression detected \
             (available bytes {available} < 80% of expected {expected_bytes}). \
             Lossless RW2 is not supported in v0.1."
        ));
    }

    let raw_bytes = &bytes[raw_offset..raw_offset + expected_bytes.min(available)];

    // ── Step 7: Unpack 12-bit packed pixels ───────────────────────────────────
    let total_pixels = sensor_width as usize * sensor_height as usize;
    let raw_pixels = unpack_12bit_le(raw_bytes, total_pixels);

    // ── Step 8: Crop to active area ───────────────────────────────────────────
    //
    // The sensor records some optical-black columns and rows used for black-level
    // estimation. The border tags define which pixels are the "active image area":
    //   rows:  [top_border .. bottom_border)
    //   cols:  [left_border .. right_border)
    //
    // If any border tag is missing, we use the full sensor extent.
    let top    = ifd.sensor_top_border  .unwrap_or(0)             as usize;
    let left   = ifd.sensor_left_border .unwrap_or(0)             as usize;
    let bottom = ifd.sensor_bottom_border.unwrap_or(sensor_height) as usize;
    let right  = ifd.sensor_right_border .unwrap_or(sensor_width)  as usize;

    // Clamp borders to sensor dimensions to handle malformed tags.
    let top    = top   .min(sensor_height as usize);
    let left   = left  .min(sensor_width  as usize);
    let bottom = bottom.min(sensor_height as usize).max(top);
    let right  = right .min(sensor_width  as usize).max(left);

    let crop_w = right  - left;
    let crop_h = bottom - top;

    if crop_w == 0 || crop_h == 0 {
        return Err("RW2: active area is empty after border crop".into());
    }

    // Extract the cropped Bayer grid.
    let mut cropped = Vec::with_capacity(crop_w * crop_h);
    for row in top..bottom {
        for col in left..right {
            let idx = row * sensor_width as usize + col;
            cropped.push(raw_pixels.get(idx).copied().unwrap_or(0));
        }
    }

    // ── Step 9: Bayer demosaicing ─────────────────────────────────────────────
    let demosaiced = demosaic_rggb(&cropped, crop_w, crop_h);

    // ── Step 10: Colour pipeline ──────────────────────────────────────────────
    let wb = white_balance_from_tags(ifd.red_balance, ifd.blue_balance);
    let srgb = apply_color_pipeline(
        demosaiced,
        BLACK_LEVEL_12BIT,
        WHITE_LEVEL_12BIT,
        wb,
        PANASONIC_COLOR_MATRIX,
    );

    // ── Step 11: Build PixelContainer (RGBA8, A=255) ──────────────────────────
    let mut container = PixelContainer::new(crop_w as u32, crop_h as u32);
    for (i, (r, g, b)) in srgb.into_iter().enumerate() {
        let x = (i % crop_w) as u32;
        let y = (i / crop_w) as u32;
        container.set_pixel(x, y, r, g, b, 255);
    }

    Ok(container)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_too_short() {
        let result = decode_rw2(&[0x49, 0x49, 0x55]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too short"));
    }
}
