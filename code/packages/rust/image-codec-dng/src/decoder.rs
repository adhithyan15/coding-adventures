// # decoder.rs — DNG Decoder
//
// This module implements the main DNG decoding path:
//
// 1. Parse the TIFF IFD chain (DNG is a strict superset of TIFF).
// 2. Identify the raw IFD: the one with `NewSubfileType == 0` and a RAW
//    photometric interpretation (CFA=32803 or LinearRaw=34892).
// 3. Extract DNG-specific tags from `extra_tags` (black level, white level,
//    white balance, colour calibration matrices).
// 4. Build a `TiffDecodeOptions` struct with the extracted parameters.
// 5. Delegate to `image_codec_tiff::decode_tiff_with_opts` to do the actual
//    pixel decoding.
//
// ## Why delegate to image-codec-tiff?
//
// DNG is a TIFF container, so all the IFD parsing, strip decompression,
// Bayer demosaicing, and colour pipeline code already lives in image-codec-tiff.
// The DNG layer only needs to extract the DNG-specific calibration tags and
// feed them to the TIFF decoder via `TiffDecodeOptions`.
//
// ## IfdValue — the typed tag variant
//
// `Ifd.extra_tags` stores `IfdValue` variants for each unrecognised tag.
// The `IfdValue` enum has typed variants:
//   - `Bytes(Vec<u8>)` — raw bytes (UNDEFINED, BYTE)
//   - `Shorts(Vec<u16>)` — unsigned 16-bit (SHORT)
//   - `Longs(Vec<u32>)` — unsigned 32-bit (LONG)
//   - `Rationals(Vec<(u32, u32)>)` — unsigned rational (RATIONAL)
//   - `SRationals(Vec<(i32, i32)>)` — signed rational (SRATIONAL)
//
// Each helper function below extracts values from these typed variants.
// The helpers also accept raw `Bytes` as a fallback (for loosely-encoded files
// where the type code was wrong but the bytes are still parseable).

use image_codec_tiff::{decode_tiff_with_opts, parse_ifd_chain, IfdValue, TiffDecodeOptions};
use pixel_container::PixelContainer;

// ─── Tag value extraction helpers ────────────────────────────────────────────

/// Read an array of SRATIONAL values from an `IfdValue`.
///
/// SRATIONAL (type code 10) is a pair of signed 32-bit integers representing
/// a fraction: value = numerator / denominator. Used in colour matrices
/// (ForwardMatrix, ColorMatrix) where negative values are common.
///
/// ## Fallback to raw bytes
///
/// Some DNG files store SRATIONAL tags as `Bytes` (UNDEFINED type) rather
/// than using the proper SRATIONAL type code. We handle both cases:
/// - `SRationals(vec)` → convert directly
/// - `Bytes(raw)` → parse as little-endian i32/i32 pairs (8 bytes each)
///
/// If the denominator is zero, returns 0.0 (safe fallback).
pub(crate) fn read_srationals(val: &IfdValue) -> Vec<f64> {
    match val {
        IfdValue::SRationals(pairs) => pairs
            .iter()
            .map(|(n, d)| {
                if *d == 0 {
                    0.0
                } else {
                    *n as f64 / *d as f64
                }
            })
            .collect(),
        IfdValue::Bytes(raw) => raw
            .chunks(8)
            .map(|c| {
                if c.len() < 8 {
                    return 0.0;
                }
                let num = i32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                let den = i32::from_le_bytes([c[4], c[5], c[6], c[7]]);
                if den == 0 {
                    0.0
                } else {
                    num as f64 / den as f64
                }
            })
            .collect(),
        _ => vec![],
    }
}

/// Read an array of RATIONAL values from an `IfdValue`.
///
/// RATIONAL (type code 5) is a pair of unsigned 32-bit integers representing
/// a fraction: value = numerator / denominator. Used in AsShotNeutral and
/// BlackLevel.
///
/// ## Fallback to raw bytes
///
/// Some DNG files store RATIONAL tags as `Bytes`. We handle both:
/// - `Rationals(vec)` → convert directly
/// - `Bytes(raw)` → parse as little-endian u32/u32 pairs (8 bytes each)
///
/// If the denominator is zero, returns 0.0 (safe fallback).
pub(crate) fn read_rationals(val: &IfdValue) -> Vec<f64> {
    match val {
        IfdValue::Rationals(pairs) => pairs
            .iter()
            .map(|(n, d)| {
                if *d == 0 {
                    0.0
                } else {
                    *n as f64 / *d as f64
                }
            })
            .collect(),
        IfdValue::Bytes(raw) => raw
            .chunks(8)
            .map(|c| {
                if c.len() < 8 {
                    return 0.0;
                }
                let num = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                let den = u32::from_le_bytes([c[4], c[5], c[6], c[7]]);
                if den == 0 {
                    0.0
                } else {
                    num as f64 / den as f64
                }
            })
            .collect(),
        _ => vec![],
    }
}

/// Read an array of LONG (u32) values from an `IfdValue`.
#[allow(dead_code)]
///
/// LONG (type code 4) stores unsigned 32-bit integers. Used in WhiteLevel,
/// ActiveArea, and sometimes BlackLevel.
///
/// ## Fallback to raw bytes
///
/// - `Longs(vec)` → return directly
/// - `Shorts(vec)` → widen u16 to u32
/// - `Bytes(raw)` → parse as little-endian u32 (4 bytes each)
///
/// ## Example
///
/// ActiveArea = [top=0, left=0, bottom=4000, right=6000] → `[0, 0, 4000, 6000]`.
pub(crate) fn read_longs(val: &IfdValue) -> Vec<u32> {
    match val {
        IfdValue::Longs(v) => v.clone(),
        IfdValue::Shorts(v) => v.iter().map(|&x| x as u32).collect(),
        IfdValue::Bytes(raw) => raw
            .chunks(4)
            .map(|c| {
                if c.len() < 4 {
                    return 0;
                }
                u32::from_le_bytes([c[0], c[1], c[2], c[3]])
            })
            .collect(),
        _ => vec![],
    }
}

/// Read the first LONG or SHORT value from an `IfdValue`, returning u32.
///
/// Convenience helper for scalar tags like WhiteLevel (single value).
pub(crate) fn read_single_long(val: &IfdValue) -> Option<u32> {
    match val {
        IfdValue::Longs(v) => v.first().copied(),
        IfdValue::Shorts(v) => v.first().map(|&x| x as u32),
        IfdValue::Bytes(raw) if raw.len() >= 4 => {
            Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
        }
        IfdValue::Bytes(raw) if raw.len() >= 2 => {
            Some(u16::from_le_bytes([raw[0], raw[1]]) as u32)
        }
        _ => None,
    }
}

// ─── Raw byte helpers (used in tests via pub(crate)) ─────────────────────────

/// Read SRATIONAL values from raw bytes (LE i32/i32 pairs, 8 bytes each).
///
/// Exposed for tests that construct synthetic byte slices.
#[cfg(test)]
pub(crate) fn read_srationals_bytes(bytes: &[u8]) -> Vec<f64> {
    bytes
        .chunks(8)
        .map(|c| {
            if c.len() < 8 {
                return 0.0;
            }
            let num = i32::from_le_bytes([c[0], c[1], c[2], c[3]]);
            let den = i32::from_le_bytes([c[4], c[5], c[6], c[7]]);
            if den == 0 {
                0.0
            } else {
                num as f64 / den as f64
            }
        })
        .collect()
}

/// Read RATIONAL values from raw bytes (LE u32/u32 pairs, 8 bytes each).
///
/// Exposed for tests that construct synthetic byte slices.
#[cfg(test)]
pub(crate) fn read_rationals_bytes(bytes: &[u8]) -> Vec<f64> {
    bytes
        .chunks(8)
        .map(|c| {
            if c.len() < 8 {
                return 0.0;
            }
            let num = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
            let den = u32::from_le_bytes([c[4], c[5], c[6], c[7]]);
            if den == 0 {
                0.0
            } else {
                num as f64 / den as f64
            }
        })
        .collect()
}

/// Read LONG values from raw bytes (LE u32, 4 bytes each).
///
/// Exposed for tests that construct synthetic byte slices.
#[cfg(test)]
pub(crate) fn read_longs_bytes(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks(4)
        .map(|c| {
            if c.len() < 4 {
                return 0;
            }
            u32::from_le_bytes([c[0], c[1], c[2], c[3]])
        })
        .collect()
}

// ─── Main decoder ─────────────────────────────────────────────────────────────

/// Decode a DNG file to RGBA8 pixels.
///
/// ## Algorithm
///
/// 1. Parse the TIFF IFD chain — a linked list of image descriptors.
/// 2. Find the raw IFD: `NewSubfileType == 0` AND photometric ∈ {32803 (CFA),
///    34892 (LinearRaw)}. If none found, use IFD 0.
/// 3. Extract DNG calibration tags from `extra_tags`:
///    - AsShotNeutral (50728): RATIONAL[3] → white balance multipliers
///    - ForwardMatrix1 (50879): SRATIONAL[9] → camera→XYZ D50 matrix (preferred)
///    - ColorMatrix1 (50721): SRATIONAL[9] → XYZ D50→camera (fallback, needs inversion)
///    - BlackLevel (50714): RATIONAL or LONG → sensor black level
///    - WhiteLevel (50717): SHORT or LONG → sensor saturation
/// 4. Build `TiffDecodeOptions` with computed WB and colour matrix.
/// 5. Call `decode_tiff_with_opts` for all the actual decoding work.
///
/// ## Errors
///
/// Returns `Err(String)` for:
/// - IFD parse failure (not a valid TIFF/DNG)
/// - Pixel decode failure (unsupported compression, truncated data, etc.)
pub fn decode_dng(bytes: &[u8]) -> Result<PixelContainer, String> {
    // Step 1: Parse IFD chain.
    //
    // DNG files are TIFF files, so parsing starts with the TIFF header.
    // `parse_ifd_chain` returns one `Ifd` struct per image directory in the file.
    let ifds = parse_ifd_chain(bytes).map_err(|e| format!("DNG: IFD parse failed: {}", e))?;

    // Step 2: Find the raw IFD.
    //
    // DNG files typically contain multiple IFDs:
    //   - IFD0: full-resolution RAW (NewSubfileType=0)
    //   - IFD1: thumbnail or preview JPEG (NewSubfileType=1)
    //   - Sometimes additional preview resolutions
    //
    // We want the RAW IFD: NewSubfileType=0 with a RAW photometric.
    //   - 32803 = CFA (Colour Filter Array, i.e., Bayer pattern data)
    //   - 34892 = LinearRaw (demosaiced, but still linear camera RGB)
    //
    // NewSubfileType is tag 254. If absent, assume 0.
    let ifd_index = ifds
        .iter()
        .position(|ifd| {
            let subfile_type = ifd
                .extra_tags
                .get(&254)
                .and_then(read_single_long)
                .unwrap_or(0);
            subfile_type == 0 && (ifd.photometric == 32803 || ifd.photometric == 34892)
        })
        .unwrap_or(0);

    let ifd = &ifds[ifd_index];

    // Step 3: Extract DNG-specific tags from extra_tags.
    //
    // The TIFF IFD parser stores typed `IfdValue` for each tag in `extra_tags`.
    // We extract the DNG calibration data using the helpers above.

    // AsShotNeutral (50728): RATIONAL[3] = [R_neutral, G_neutral, B_neutral]
    //
    // Encodes the white balance as the raw sensor response to a neutral grey
    // under the shot illuminant.
    let as_shot_neutral = ifd
        .extra_tags
        .get(&50728)
        .map(read_rationals)
        .unwrap_or_else(|| vec![1.0, 1.0, 1.0]);

    // ForwardMatrix1 (50879): SRATIONAL[9] = 3×3 matrix, camera RGB → XYZ D50.
    let forward_matrix_raw = ifd.extra_tags.get(&50879).map(read_srationals);

    // ColorMatrix1 (50721): SRATIONAL[9] = 3×3 matrix, XYZ D50 → camera RGB.
    let color_matrix_raw = ifd.extra_tags.get(&50721).map(read_srationals);

    // BlackLevel (50714): RATIONAL or LONG — sensor black level.
    //
    // We take the first value as a scalar black level.
    let black_level_val = ifd
        .extra_tags
        .get(&50714)
        .and_then(|v| match v {
            IfdValue::Rationals(pairs) => pairs.first().map(|(n, d)| {
                if *d == 0 {
                    0u32
                } else {
                    *n / d.max(&1)
                }
            }),
            IfdValue::Longs(lv) => lv.first().copied(),
            IfdValue::Shorts(sv) => sv.first().map(|&x| x as u32),
            IfdValue::Bytes(raw) if raw.len() >= 4 => {
                Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
            }
            _ => None,
        })
        .unwrap_or(0);

    // WhiteLevel (50717): SHORT or LONG — sensor saturation point.
    let white_level = ifd
        .extra_tags
        .get(&50717)
        .and_then(read_single_long)
        .unwrap_or_else(|| {
            // Cap bps at 31 to prevent u32 shift overflow on malformed files.
            // A DNG with BitsPerSample=32 or higher would cause 1u32 << 32 to
            // panic in debug mode (and wrap in release) — neither is correct.
            // Real sensors are 8, 12, 14, or 16 bit; 31 is a safe upper bound.
            let bps = (ifd.bits_per_sample.first().copied().unwrap_or(12) as u32).min(31);
            (1u32 << bps) - 1
        });

    // Step 4: Compute white-balance multipliers from AsShotNeutral.
    let wb = crate::color::wb_from_as_shot_neutral(&as_shot_neutral);

    // Step 5: Build the camera → sRGB colour matrix.
    //
    // Priority order:
    //   a) ForwardMatrix1 (camera → XYZ D50) × XYZ_D50_TO_SRGB
    //   b) inv(ColorMatrix1) × XYZ_D50_TO_SRGB
    //   c) Identity (no colour correction)
    let color_matrix = if let Some(fwd_raw) = forward_matrix_raw {
        if fwd_raw.len() >= 9 {
            let fwd = [
                [fwd_raw[0], fwd_raw[1], fwd_raw[2]],
                [fwd_raw[3], fwd_raw[4], fwd_raw[5]],
                [fwd_raw[6], fwd_raw[7], fwd_raw[8]],
            ];
            crate::color::camera_to_srgb_via_forward(&fwd)
        } else {
            default_identity()
        }
    } else if let Some(cm_raw) = color_matrix_raw {
        if cm_raw.len() >= 9 {
            let cm = [
                [cm_raw[0], cm_raw[1], cm_raw[2]],
                [cm_raw[3], cm_raw[4], cm_raw[5]],
                [cm_raw[6], cm_raw[7], cm_raw[8]],
            ];
            if let Some(inv) = crate::color::invert_3x3(&cm) {
                crate::color::matrix_multiply(&crate::color::XYZ_D50_TO_SRGB, &inv)
            } else {
                default_identity()
            }
        } else {
            default_identity()
        }
    } else {
        default_identity()
    };

    // Step 6: Decode using image-codec-tiff with DNG-derived options.
    let opts = TiffDecodeOptions {
        ifd_index,
        wb_multipliers: wb,
        color_matrix,
        black_level: [black_level_val; 4],
        white_level,
    };

    decode_tiff_with_opts(bytes, &opts).map_err(|e| format!("DNG: decode failed: {}", e))
}

/// Return the 3×3 identity matrix as the fallback colour matrix.
fn default_identity() -> [[f64; 3]; 3] {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}
