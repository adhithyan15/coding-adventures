// # decoder.rs — Top-level RAF decode pipeline
//
// This module orchestrates all the sub-modules into a single public function:
//
// ```text
// decode_raf(bytes: &[u8]) -> Result<PixelContainer, String>
// ```
//
// The decode pipeline follows the colour processing order from the spec:
//
// ```
// 1.  Check magic ("FUJIFILMCCD-RAW ")
// 2.  Parse outer header (JPEG + CFA header + CFA pixel offsets/lengths)
// 3.  Parse CFA header (image size, pattern, WB, black/white levels)
// 4.  Validate raw pixel region (security: offset+length ≤ file size)
// 5.  Unpack 12-bit big-endian packed pixels
// 6.  Subtract black level, clip, and normalise implicitly via color pipeline
// 7.  Demosaic (bilinear Bayer or simplified bilinear X-Trans)
// 8.  Apply colour pipeline (WB + matrix + sRGB gamma)
// 9.  Build PixelContainer (RGBA, A=255)
// ```

use pixel_container::PixelContainer;

use crate::bayer::demosaic_bayer_2x2;
use crate::cfa_header::{parse_cfa_header, CfaPattern};
use crate::color::{apply_color_pipeline, FUJI_COLOR_MATRIX};
use crate::header::parse_header;
use crate::unpack::unpack_12bit_be;
use crate::xtrans::demosaic_xtrans;

/// Decode a Fujifilm RAF file from raw bytes into a `PixelContainer`.
///
/// # Errors
///
/// Returns `Err` with a human-readable message for:
/// - Wrong/missing magic (not a RAF file)
/// - Truncated outer header (< 116 bytes)
/// - Corrupt region offsets (point outside the file)
/// - Oversized image dimensions (> 4096 on any axis)
/// - Invalid image geometry (zero raw_width or raw_height)
pub fn decode_raf(bytes: &[u8]) -> Result<PixelContainer, String> {
    // ── Step 1 & 2: parse outer header ──────────────────────────────────────
    // This also checks the magic and validates that all three data regions
    // (JPEG, CFA header, CFA pixels) lie within the file buffer.
    let header = parse_header(bytes)?;

    // ── Step 3: parse CFA header ─────────────────────────────────────────────
    let cfa_data = &bytes[header.cfa_header_offset
        ..header.cfa_header_offset + header.cfa_header_length];
    let cfa = parse_cfa_header(cfa_data)?;

    // ── Step 4: determine pixel grid size ────────────────────────────────────
    // Use `raw_width` / `raw_height` from tag 0x0110 if available; fall back
    // to the displayed size from tag 0x0100.
    let (grid_w, grid_h) = if cfa.raw_width > 0 && cfa.raw_height > 0 {
        (cfa.raw_width as usize, cfa.raw_height as usize)
    } else if cfa.width > 0 && cfa.height > 0 {
        (cfa.width as usize, cfa.height as usize)
    } else {
        return Err("RAF: could not determine image dimensions".into());
    };

    // ── Step 5: checked pixel count arithmetic ───────────────────────────────
    // Use checked arithmetic so that a corrupt header with giant dimensions
    // does not silently overflow on 32-bit hosts.
    let num_pixels = grid_w
        .checked_mul(grid_h)
        .ok_or_else(|| "RAF: image dimensions overflow usize".to_string())?;

    // ── Step 6: unpack 12-bit BE pixels ─────────────────────────────────────
    let raw_data = &bytes[header.cfa_offset..header.cfa_offset + header.cfa_length];
    let raw_pixels = unpack_12bit_be(raw_data, num_pixels);

    // Guard: if the packed data was shorter than expected, pad with zeros so
    // we don't panic on index access during demosaicing.
    let raw_pixels = if raw_pixels.len() < num_pixels {
        let mut padded = raw_pixels;
        padded.resize(num_pixels, 0u16);
        padded
    } else {
        raw_pixels
    };

    // ── Step 7: demosaic ─────────────────────────────────────────────────────
    let demosaiced: Vec<(u16, u16, u16)> = match &cfa.pattern {
        CfaPattern::Bayer(pat) => {
            demosaic_bayer_2x2(&raw_pixels, grid_w, grid_h, *pat)
        }
        CfaPattern::XTrans(pat) => {
            demosaic_xtrans(&raw_pixels, grid_w, grid_h, pat)
        }
    };

    // ── Step 8: colour pipeline (WB + matrix + gamma) ───────────────────────
    let srgb = apply_color_pipeline(
        demosaiced,
        cfa.black_level,
        cfa.white_level,
        cfa.wb,
        FUJI_COLOR_MATRIX,
    );

    // ── Step 9: build PixelContainer ─────────────────────────────────────────
    // Displayed dimensions come from tag 0x0100 (or fall back to the grid).
    // We produce a container sized to the raw grid, which the caller can crop.
    let out_w = grid_w as u32;
    let out_h = grid_h as u32;
    let mut container = PixelContainer::new(out_w, out_h);

    for (idx, (r, g, b)) in srgb.into_iter().enumerate() {
        let x = (idx % grid_w) as u32;
        let y = (idx / grid_w) as u32;
        container.set_pixel(x, y, r, g, b, 255); // A = 255 (fully opaque)
    }

    Ok(container)
}
