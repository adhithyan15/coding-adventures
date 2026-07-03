// # decoder.rs — NEF top-level decode orchestrator
//
// This module wires together the IFD chain parser, Make-tag validation, CFA
// sub-IFD discovery, and the TIFF colour pipeline into one `decode_nef`
// function.
//
// ## Decode strategy
//
// NEF files are TIFF containers. The raw Bayer pixel data lives in a sub-IFD
// identified by `PhotometricInterpretation = 32803` (CFA). Our approach:
//
// 1. Call `image_codec_tiff::parse_ifd_chain` to get all IFDs.
// 2. Check the first IFD's Make tag (tag 271):
//    - Missing Make: allowed (synthetic / test files lack it).
//    - Make present but not containing "NIKON": reject with Err.
// 3. Scan the IFD list for the one with `photometric == 32803`.
//    If not found, fall back to index 0 (handles plain TIFF round-trip files).
// 4. Read `bits_per_sample` from that IFD to decide 12-bit vs 14-bit mode.
// 5. Call `decode_tiff_with_opts` with the Nikon colour parameters.
// 6. Propagate any error from the TIFF decoder (e.g., unsupported compression
//    34713) with a NEF-specific context prefix.

use image_codec_tiff::{IfdValue, TiffDecodeOptions};
use pixel_container::PixelContainer;

use crate::{
    NIKON_BLACK_LEVEL_12BIT, NIKON_BLACK_LEVEL_14BIT, NIKON_COLOR_MATRIX,
    NIKON_WHITE_LEVEL_12BIT, NIKON_WHITE_LEVEL_14BIT,
};

/// Decode a Nikon NEF file from a byte slice.
///
/// Returns a `PixelContainer` with RGBA8 pixels (alpha=255).
///
/// # Make tag validation
///
/// The Make tag (tag 271) in IFD0 identifies the camera manufacturer. If the
/// tag is *absent* (e.g., in synthetic test files), we proceed without
/// rejection. If the tag is *present* and does NOT contain "NIKON" (and is
/// longer than 2 bytes to filter out trivially empty tags), we return Err.
///
/// # Compression support
///
/// - Compression=1 (uncompressed): fully supported.
/// - Compression=34713 (Nikon compressed): the TIFF decoder returns Err,
///   which we propagate with a helpful message.
///
/// # Errors
///
/// Returns `Err(String)` for any of:
/// - File too short (< 8 bytes)
/// - Invalid TIFF header
/// - Make tag present but not Nikon
/// - No decodable IFD found
/// - Unsupported compression
/// - Truncated pixel data
pub fn decode_nef(bytes: &[u8]) -> Result<PixelContainer, String> {
    // ── Step 1: Minimum length check ──────────────────────────────────────────
    //
    // A valid TIFF needs at least 8 bytes (4-byte header + 4-byte IFD offset).
    // Failing early avoids confusing error messages from the TIFF crate.
    if bytes.len() < 8 {
        return Err("NEF: file too short".into());
    }

    // ── Step 2: Parse IFD chain ────────────────────────────────────────────────
    //
    // `parse_ifd_chain` walks the TIFF IFD linked list and returns all IFDs,
    // including the standard chain (IFD0 → IFD1 → …). Sub-IFDs pointed to
    // by SubIFDs tag (330) may or may not be followed depending on the TIFF
    // crate version. We search the returned list for photometric=32803.
    let ifds = image_codec_tiff::parse_ifd_chain(bytes)
        .map_err(|e| format!("NEF: {}", e))?;

    // ── Step 3: Validate Make tag ──────────────────────────────────────────────
    //
    // The Make tag (271) in IFD0 is an ASCII string that names the camera
    // manufacturer. We accept "NIKON", "NIKON CORPORATION", and variants.
    //
    // We only reject if:
    //   a. The Make tag IS present (extra_tags contains key 271), AND
    //   b. The decoded ASCII string does NOT contain "NIKON" (case-insensitive), AND
    //   c. The string is longer than 2 chars (filters empty/NUL-only tags).
    if let Some(ifd0) = ifds.first() {
        if let Some(make_value) = ifd0.extra_tags.get(&271) {
            let make_str = match make_value {
                IfdValue::Ascii(s) => s.clone(),
                IfdValue::Bytes(b) => String::from_utf8_lossy(b).to_string(),
                _ => String::new(),
            };
            let make_upper = make_str.to_uppercase();
            if make_str.len() > 2 && !make_upper.contains("NIKON") {
                return Err(format!(
                    "NEF: Make tag indicates this is not a Nikon file (Make='{}')",
                    make_str.trim_end_matches('\0')
                ));
            }
        }
    }

    // ── Step 4: Find the CFA IFD ──────────────────────────────────────────────
    //
    // The raw Bayer data IFD has PhotometricInterpretation=32803 (CFA).
    // If not found, fall back to index 0 (handles plain TIFF round-trip).
    let raw_ifd_index = ifds
        .iter()
        .position(|ifd| ifd.photometric == 32803)
        .unwrap_or(0);

    // ── Step 5: Determine bit depth ───────────────────────────────────────────
    //
    // Older Nikon bodies (D50/D70/D80/D90) use 12-bit depth.
    // Newer bodies (D300+, Z-series) use 14-bit depth.
    // We read from the IFD's bits_per_sample field; default to 12 if absent.
    let bps = ifds[raw_ifd_index]
        .bits_per_sample
        .first()
        .copied()
        .unwrap_or(12);

    let (black, white) = if bps >= 14 {
        (NIKON_BLACK_LEVEL_14BIT, NIKON_WHITE_LEVEL_14BIT)
    } else {
        (NIKON_BLACK_LEVEL_12BIT, NIKON_WHITE_LEVEL_12BIT)
    };

    // ── Step 6: Decode with Nikon colour parameters ───────────────────────────
    //
    // `TiffDecodeOptions` bundles all the RAW decode parameters:
    //
    // - `ifd_index`: which IFD to decode (the CFA one we found above).
    // - `wb_multipliers`: white balance [R, G, B] multipliers. Using [1,1,1]
    //   (D65 neutral) as default since we don't decrypt the MakerNote in v0.1.
    // - `color_matrix`: 3×3 camera-to-sRGB matrix.
    // - `black_level`: per-channel black offset to subtract.
    // - `white_level`: maximum sensor value (after black subtraction).
    let opts = TiffDecodeOptions {
        ifd_index: raw_ifd_index,
        wb_multipliers: [1.0, 1.0, 1.0],
        color_matrix: NIKON_COLOR_MATRIX,
        black_level: [black; 4],
        white_level: white,
    };

    image_codec_tiff::decode_tiff_with_opts(bytes, &opts)
        .map_err(|e| {
            // Provide a more helpful message for Nikon compressed format.
            if e.contains("34713") || e.contains("compression") {
                format!(
                    "NEF: Nikon compressed format (34713) not fully supported in v0.1; \
                     try converting to uncompressed NEF. (TIFF error: {})",
                    e
                )
            } else {
                format!("NEF: {}", e)
            }
        })
}
