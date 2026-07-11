// # decoder.rs — TIFF Top-Level Decoder
//
// This module wires together all the other modules:
//   ifd.rs         → parse the header and tags
//   strips.rs      → decompress and assemble pixel bytes
//   bayer.rs       → demosaic CFA images
//   color.rs       → apply white balance, colour matrix, gamma
//
// The final output is always a PixelContainer with RGBA8 pixels (alpha=255).
//
// ## PhotometricInterpretation Values Handled
//
// | Value | Name         | How we decode                                 |
// |-------|--------------|-----------------------------------------------|
// | 0     | WhiteIsZero  | Grayscale, 0=white. Invert scale.             |
// | 1     | BlackIsZero  | Grayscale, 0=black. Standard.                 |
// | 2     | RGB          | Direct R,G,B channels.                        |
// | 32803 | CFA          | Bayer mosaic. Demosaic then colour pipeline.   |
//
// ## Security Limits
//
// | Limit                  | Value        |
// |------------------------|--------------|
// | Max image width        | 32768 pixels |
// | Max image height       | 32768 pixels |
// | Max IFD chain length   | 256 (ifd.rs) |

use pixel_container::PixelContainer;

use crate::bayer;
use crate::color::{apply_color_pipeline, TiffDecodeOptions};
use crate::ifd;
use crate::strips;

// ─── Security limits ─────────────────────────────────────────────────────────

/// Maximum supported image width in pixels.
const MAX_WIDTH: u32 = 32768;
/// Maximum supported image height in pixels.
const MAX_HEIGHT: u32 = 32768;

// ─── Public decode functions ──────────────────────────────────────────────────

/// Decode the first image from a TIFF byte stream.
///
/// Returns a `PixelContainer` with RGBA8 pixels (A=255 always).
///
/// This is equivalent to `decode_tiff_with_opts(bytes, &TiffDecodeOptions::default())`.
pub fn decode_tiff(bytes: &[u8]) -> Result<PixelContainer, String> {
    decode_tiff_with_opts(bytes, &TiffDecodeOptions::default())
}

/// Decode a TIFF image with custom options.
///
/// # Options
///
/// `TiffDecodeOptions` allows callers (DNG, CR2, NEF codecs) to supply:
/// - Which IFD index to decode (`ifd_index`)
/// - White balance multipliers
/// - Camera-to-sRGB colour matrix
/// - Black level (pedestal)
/// - White level (saturation point)
///
/// # Error cases
///
/// - Invalid TIFF header or magic number
/// - `ifd_index` out of range
/// - Image dimensions exceed 32768 × 32768
/// - Unsupported compression scheme
/// - Truncated strip/tile data
pub fn decode_tiff_with_opts(
    bytes: &[u8],
    opts: &TiffDecodeOptions,
) -> Result<PixelContainer, String> {
    // ── Step 1: Parse IFD chain ────────────────────────────────────────────
    let ifds = ifd::parse_ifd_chain(bytes)?;

    if opts.ifd_index >= ifds.len() {
        return Err(format!(
            "TIFF: ifd_index {} is out of range (file has {} IFDs)",
            opts.ifd_index,
            ifds.len()
        ));
    }
    let ifd = &ifds[opts.ifd_index];

    // ── Step 2: Validate dimensions ────────────────────────────────────────
    if ifd.width == 0 || ifd.height == 0 {
        return Err(format!(
            "TIFF: image dimensions {}×{} — zero-size images are not supported",
            ifd.width, ifd.height
        ));
    }
    if ifd.width > MAX_WIDTH || ifd.height > MAX_HEIGHT {
        return Err(format!(
            "TIFF: image dimensions {}×{} exceed maximum {}×{}",
            ifd.width, ifd.height, MAX_WIDTH, MAX_HEIGHT
        ));
    }

    let width = ifd.width as usize;
    let height = ifd.height as usize;

    // ── Step 3: Assemble pixel bytes (decompress strips or tiles) ─────────
    let raw_bytes = strips::assemble(bytes, ifd)?;

    // ── Step 4: Decode pixel bytes into RGBA8 ─────────────────────────────
    let bits = ifd.bits_per_sample.first().copied().unwrap_or(8);
    let photometric = ifd.photometric;

    let rgba: Vec<u8> = match photometric {
        0 | 1 => {
            // Grayscale: PhotometricInterpretation 0 = WhiteIsZero,
            //            PhotometricInterpretation 1 = BlackIsZero.
            //
            // We support 8-bit and 16-bit grayscale here.
            decode_grayscale(&raw_bytes, width, height, bits, photometric)?
        }
        2 => {
            // RGB: standard 8-bit or 16-bit RGB.
            decode_rgb(&raw_bytes, width, height, bits)?
        }
        32803 => {
            // CFA: Bayer colour filter array (RAW camera data).
            decode_cfa(&raw_bytes, width, height, bits, ifd, opts)?
        }
        p => {
            return Err(format!(
                "TIFF: unsupported PhotometricInterpretation {} \
                 (supported: 0=WhiteIsZero, 1=BlackIsZero, 2=RGB, 32803=CFA)",
                p
            ));
        }
    };

    // ── Step 5: Build PixelContainer ──────────────────────────────────────
    let expected_len = width * height * 4;
    if rgba.len() != expected_len {
        return Err(format!(
            "TIFF: internal error — pixel buffer has {} bytes, expected {}",
            rgba.len(), expected_len
        ));
    }

    Ok(PixelContainer::from_data(ifd.width, ifd.height, rgba))
}

// ─── Grayscale decoder ────────────────────────────────────────────────────────

/// Decode a grayscale TIFF image (PhotometricInterpretation 0 or 1).
///
/// Converts to RGBA8 by triplicating the grayscale value into R=G=B and
/// setting A=255.
///
/// For 16-bit input, we scale by dividing by 256 (top 8 bits).
/// For WhiteIsZero (photometric=0), we invert: gray = 255 - value.
fn decode_grayscale(
    raw: &[u8],
    width: usize,
    height: usize,
    bits: u16,
    photometric: u16,
) -> Result<Vec<u8>, String> {
    let num_pixels = width * height;
    let mut rgba = Vec::with_capacity(num_pixels * 4);

    match bits {
        8 => {
            if raw.len() < num_pixels {
                return Err(format!(
                    "TIFF: grayscale 8-bit buffer too short: {} < {}",
                    raw.len(), num_pixels
                ));
            }
            for i in 0..num_pixels {
                let mut v = raw[i];
                if photometric == 0 {
                    v = 255 - v; // WhiteIsZero: invert
                }
                rgba.push(v);
                rgba.push(v);
                rgba.push(v);
                rgba.push(255);
            }
        }
        16 => {
            let needed = num_pixels * 2;
            if raw.len() < needed {
                return Err(format!(
                    "TIFF: grayscale 16-bit buffer too short: {} < {}",
                    raw.len(), needed
                ));
            }
            for i in 0..num_pixels {
                // Read 16-bit value as little-endian (most TIFF files are LE).
                let lo = raw[i * 2] as u16;
                let hi = raw[i * 2 + 1] as u16;
                let val16 = lo | (hi << 8);
                // Scale to 8-bit: take the top 8 bits.
                let mut v = (val16 >> 8) as u8;
                if photometric == 0 {
                    v = 255 - v;
                }
                rgba.push(v);
                rgba.push(v);
                rgba.push(v);
                rgba.push(255);
            }
        }
        b => {
            return Err(format!("TIFF: unsupported BitsPerSample {} for grayscale", b));
        }
    }

    Ok(rgba)
}

// ─── RGB decoder ─────────────────────────────────────────────────────────────

/// Decode an RGB TIFF image (PhotometricInterpretation 2).
///
/// Supports 8-bit and 16-bit per channel. For 16-bit, takes the top 8 bits.
fn decode_rgb(raw: &[u8], width: usize, height: usize, bits: u16) -> Result<Vec<u8>, String> {
    let num_pixels = width * height;
    let mut rgba = Vec::with_capacity(num_pixels * 4);

    match bits {
        8 => {
            let needed = num_pixels * 3;
            if raw.len() < needed {
                return Err(format!(
                    "TIFF: RGB 8-bit buffer too short: {} < {}",
                    raw.len(), needed
                ));
            }
            for i in 0..num_pixels {
                rgba.push(raw[i * 3]);
                rgba.push(raw[i * 3 + 1]);
                rgba.push(raw[i * 3 + 2]);
                rgba.push(255);
            }
        }
        16 => {
            let needed = num_pixels * 6; // 3 channels × 2 bytes each
            if raw.len() < needed {
                return Err(format!(
                    "TIFF: RGB 16-bit buffer too short: {} < {}",
                    raw.len(), needed
                ));
            }
            for i in 0..num_pixels {
                // Each channel is stored as u16 LE. Take top byte.
                let r = raw[i * 6 + 1]; // high byte of R
                let g = raw[i * 6 + 3]; // high byte of G
                let b = raw[i * 6 + 5]; // high byte of B
                rgba.push(r);
                rgba.push(g);
                rgba.push(b);
                rgba.push(255);
            }
        }
        b => {
            return Err(format!("TIFF: unsupported BitsPerSample {} for RGB", b));
        }
    }

    Ok(rgba)
}

// ─── CFA (Bayer) decoder ──────────────────────────────────────────────────────

/// Decode a CFA/Bayer RAW image (PhotometricInterpretation 32803).
///
/// This is the full RAW pipeline:
/// 1. Unpack raw sensor values (12/14/16-bit) into u16.
/// 2. Apply black-level subtraction and white-level normalization.
/// 3. Bilinear Bayer demosaicing → linear RGB u16.
/// 4. Apply colour pipeline (WB, colour matrix, gamma).
/// 5. Return RGBA8.
fn decode_cfa(
    raw: &[u8],
    width: usize,
    height: usize,
    bits: u16,
    ifd: &ifd::Ifd,
    opts: &TiffDecodeOptions,
) -> Result<Vec<u8>, String> {
    let num_pixels = width * height;

    // ── Unpack sensor values ───────────────────────────────────────────────
    //
    // Sensor data is typically 12-bit or 14-bit, stored as 16-bit little-endian
    // values with the top bits empty. We read them as u16 directly.
    //
    // Some sensors pack 12-bit values as 3 bytes per 2 pixels — this is the
    // "12-bit packed" format used in some DNG files. For now we support only
    // 16-bit container (bits=16) and treat 12/14 as stored in 16-bit LE words.
    let sensor_values: Vec<u16> = match bits {
        8 => {
            if raw.len() < num_pixels {
                return Err("TIFF: CFA 8-bit buffer too short".to_string());
            }
            raw[..num_pixels].iter().map(|&b| b as u16 * 257).collect()
        }
        12 | 14 | 16 => {
            let needed = num_pixels * 2;
            if raw.len() < needed {
                return Err(format!(
                    "TIFF: CFA {}-bit buffer too short: {} < {}",
                    bits, raw.len(), needed
                ));
            }
            (0..num_pixels)
                .map(|i| {
                    let lo = raw[i * 2] as u16;
                    let hi = raw[i * 2 + 1] as u16;
                    let v = lo | (hi << 8);
                    // Scale to 16-bit range.
                    // 12-bit: max 4095 → scale by 65535/4095 ≈ 16
                    // 14-bit: max 16383 → scale by 65535/16383 ≈ 4
                    // 16-bit: already full range
                    match bits {
                        12 => v << 4,  // ×16 to fill u16
                        14 => v << 2,  // ×4
                        _ => v,
                    }
                })
                .collect()
        }
        b => {
            return Err(format!("TIFF: unsupported BitsPerSample {} for CFA", b));
        }
    };

    // ── Apply black-level subtraction ─────────────────────────────────────
    //
    // The sensor has a non-zero "pedestal" (dark current even with no light).
    // We subtract it to get true linear values. The opts.black_level array
    // has one entry per CFA channel (indexed by cfa_pattern position).
    //
    // For simplicity, we use black_level[0] for all channels here.
    // A DNG codec would supply per-channel values.
    let black = opts.black_level[0] as u16;
    let sensor_values: Vec<u16> = sensor_values
        .into_iter()
        .map(|v| v.saturating_sub(black))
        .collect();

    // ── Determine CFA pattern ──────────────────────────────────────────────
    let pattern = ifd.cfa_pattern.unwrap_or([0, 1, 1, 2]); // default RGGB

    // ── Bilinear Bayer demosaicing ─────────────────────────────────────────
    let rgb_linear = bayer::demosaic_bilinear(&sensor_values, width, height, pattern);

    // ── Apply colour pipeline ──────────────────────────────────────────────
    let rgb_u8 = apply_color_pipeline(rgb_linear, opts);

    // ── Convert to RGBA8 ──────────────────────────────────────────────────
    let mut rgba = Vec::with_capacity(num_pixels * 4);
    for (r, g, b) in rgb_u8 {
        rgba.push(r);
        rgba.push(g);
        rgba.push(b);
        rgba.push(255);
    }

    Ok(rgba)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoder::encode_tiff;

    #[test]
    fn decode_round_trip_1x1() {
        let mut pc = PixelContainer::new(1, 1);
        pc.set_pixel(0, 0, 100, 150, 200, 255);
        let bytes = encode_tiff(&pc);
        let decoded = decode_tiff(&bytes).unwrap();
        assert_eq!(decoded.width, 1);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.pixel_at(0, 0), (100, 150, 200, 255));
    }

    #[test]
    fn decode_round_trip_2x2() {
        let mut pc = PixelContainer::new(2, 2);
        pc.set_pixel(0, 0, 255, 0, 0, 255);
        pc.set_pixel(1, 0, 0, 255, 0, 255);
        pc.set_pixel(0, 1, 0, 0, 255, 255);
        pc.set_pixel(1, 1, 128, 128, 128, 255);
        let bytes = encode_tiff(&pc);
        let decoded = decode_tiff(&bytes).unwrap();
        assert_eq!(decoded.pixel_at(0, 0), (255, 0, 0, 255));
        assert_eq!(decoded.pixel_at(1, 0), (0, 255, 0, 255));
        assert_eq!(decoded.pixel_at(0, 1), (0, 0, 255, 255));
        assert_eq!(decoded.pixel_at(1, 1), (128, 128, 128, 255));
    }

    #[test]
    fn decode_round_trip_4x4() {
        let mut pc = PixelContainer::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                pc.set_pixel(x, y, (x * 60) as u8, (y * 60) as u8, 128, 255);
            }
        }
        let bytes = encode_tiff(&pc);
        let decoded = decode_tiff(&bytes).unwrap();
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
        for y in 0..4u32 {
            for x in 0..4u32 {
                let expected = ((x * 60) as u8, (y * 60) as u8, 128u8, 255u8);
                assert_eq!(decoded.pixel_at(x, y), expected, "Mismatch at ({}, {})", x, y);
            }
        }
    }

    #[test]
    fn decode_bad_magic_returns_err() {
        let mut bytes = vec![0x49u8, 0x49, 0xFF, 0xFF, 8, 0, 0, 0]; // bad magic
        bytes.extend_from_slice(&[0u8; 6]); // minimal IFD
        assert!(decode_tiff(&bytes).is_err());
    }

    #[test]
    fn decode_truncated_returns_err() {
        let bytes = vec![0x49u8, 0x49, 0x2A, 0x00]; // just 4 bytes
        assert!(decode_tiff(&bytes).is_err());
    }

    #[test]
    fn decode_ifd_index_out_of_range() {
        let mut pc = PixelContainer::new(1, 1);
        pc.set_pixel(0, 0, 255, 0, 0, 255);
        let bytes = encode_tiff(&pc);
        let opts = TiffDecodeOptions { ifd_index: 5, ..Default::default() };
        assert!(decode_tiff_with_opts(&bytes, &opts).is_err());
    }
}
