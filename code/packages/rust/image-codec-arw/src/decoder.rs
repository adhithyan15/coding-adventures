// # decoder.rs — ARW top-level decode orchestrator
//
// This module wires together the IFD chain parser, Make-tag validation, CFA
// sub-IFD discovery, and the TIFF colour pipeline into one `decode_arw`
// function.
//
// ## Decode strategy
//
// ARW files are TIFF containers. The raw Bayer pixel data lives in a sub-IFD
// identified by `PhotometricInterpretation = 32803` (CFA). Our approach:
//
// 1. Call `image_codec_tiff::parse_ifd_chain` to get all IFDs.
// 2. Check the first IFD's Make tag (tag 271):
//    - Missing Make: allowed (synthetic / test files).
//    - Make present but not containing "SONY": reject with Err.
// 3. Scan the IFD list for the one with `photometric == 32803`.
//    If not found, fall back to index 0.
// 4. Call `decode_tiff_with_opts` with the Sony colour parameters.
// 5. Propagate any error from the TIFF decoder with an ARW context prefix.
//
// ## Sony ARW versions
//
// - ARW 1.0 (Compression=32767, 12-bit): uncompressed 12-bit packed data.
// - ARW 2.x (Compression=32767, 14-bit): Sony variable-length compressed.
// - ARW 3.0 (A7R IV+): new compression scheme, flagged as unsupported in v0.1.
//
// In v0.1 we delegate entirely to `image_codec_tiff::decode_tiff_with_opts`.
// If the TIFF decoder cannot handle Sony compression 32767, it returns Err
// which we propagate.

use image_codec_tiff::{IfdValue, TiffDecodeOptions};
use pixel_container::PixelContainer;

use crate::{SONY_BLACK_LEVEL, SONY_COLOR_MATRIX, SONY_WHITE_LEVEL};

/// Decode a Sony ARW file from a byte slice.
///
/// Returns a `PixelContainer` with RGBA8 pixels (alpha=255).
///
/// # Make tag validation
///
/// If the Make tag (271) is absent, we proceed (synthetic files).
/// If present and does NOT contain "SONY" (and is > 2 chars), we return Err.
///
/// # Compression support
///
/// - Compression=1 (standard TIFF uncompressed): fully supported.
/// - Compression=32767 (Sony): the TIFF decoder propagates any decode error.
///
/// # Errors
///
/// - File too short (< 8 bytes)
/// - Invalid TIFF header
/// - Make tag present but not Sony
/// - No decodable IFD found
/// - Unsupported compression
/// - Truncated pixel data
pub fn decode_arw(bytes: &[u8]) -> Result<PixelContainer, String> {
    // ── Step 1: Minimum length check ──────────────────────────────────────────
    if bytes.len() < 8 {
        return Err("ARW: file too short".into());
    }

    // ── Step 2: Parse IFD chain ────────────────────────────────────────────────
    let ifds = image_codec_tiff::parse_ifd_chain(bytes)
        .map_err(|e| format!("ARW: {}", e))?;

    // ── Step 3: Validate Make tag ──────────────────────────────────────────────
    //
    // Sony ARW files carry Make = "SONY" in IFD0 tag 271.
    // We accept any spelling that contains "SONY" (case-insensitive).
    // An absent Make is fine (synthetic test files). A present non-Sony Make
    // is rejected.
    if let Some(ifd0) = ifds.first() {
        if let Some(make_value) = ifd0.extra_tags.get(&271) {
            let make_str = match make_value {
                IfdValue::Ascii(s) => s.clone(),
                IfdValue::Bytes(b) => String::from_utf8_lossy(b).to_string(),
                _ => String::new(),
            };
            let make_upper = make_str.to_uppercase();
            if make_str.len() > 2 && !make_upper.contains("SONY") {
                return Err(format!(
                    "ARW: Make tag indicates this is not a Sony file (Make='{}')",
                    make_str.trim_end_matches('\0')
                ));
            }
        }
    }

    // ── Step 4: Find the CFA IFD ──────────────────────────────────────────────
    //
    // The raw Bayer data IFD has PhotometricInterpretation=32803 (CFA).
    // Fall back to index 0 if not found (plain TIFF round-trip).
    let raw_ifd_index = ifds
        .iter()
        .position(|ifd| ifd.photometric == 32803)
        .unwrap_or(0);

    // ── Step 5: Decode with Sony colour parameters ─────────────────────────────
    //
    // Sony ARW 2.x typically uses 14-bit pixels, so SONY_WHITE_LEVEL = 16383.
    // Black level is 200 for ARW 2.x (vs 512 for ARW 1.0 — we use the more
    // common 2.x value in v0.1).
    let opts = TiffDecodeOptions {
        ifd_index: raw_ifd_index,
        wb_multipliers: [1.0, 1.0, 1.0],   // D65 default
        color_matrix: SONY_COLOR_MATRIX,
        black_level: [SONY_BLACK_LEVEL; 4],
        white_level: SONY_WHITE_LEVEL,
    };

    image_codec_tiff::decode_tiff_with_opts(bytes, &opts)
        .map_err(|e| format!("ARW: {}", e))
}
