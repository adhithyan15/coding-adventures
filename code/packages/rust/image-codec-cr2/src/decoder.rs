// # decoder.rs — CR2 Top-Level Decoder
//
// The CR2 decoder pipeline:
//
// ```text
// CR2 bytes
//   ↓ validate CR2 signature (bytes 8–10 = "CR\x02")
//   ↓ parse TIFF IFD chain → find IFD3 (the full-resolution RAW IFD)
//   ↓ pass to image-codec-tiff with Canon-specific decode options
//   ↓ PixelContainer (RGBA8, A=255)
// ```
//
// ## Why Delegate to image-codec-tiff?
//
// CR2 is a TIFF container. The only things that make it "CR2" rather than
// a plain TIFF are:
//   1. The "CR\x02" signature at bytes 8–10.
//   2. The raw sensor data lives in IFD3 (the 4th IFD), not IFD0.
//   3. Canon-specific default black level, white level, and colour matrix.
//
// `image-codec-tiff` already implements IFD parsing, strip decompression
// (including lossless JPEG via Compression=6/7), Bayer demosaicing, and
// the full colour pipeline. Rather than re-implementing all of that here,
// we validate the CR2 signature, pick the right IFD index, set Canon-
// specific colour parameters, and hand off to `decode_tiff_with_opts`.

use crate::{CANON_BLACK_LEVEL, CANON_COLOR_MATRIX, CANON_WHITE_LEVEL};
use image_codec_tiff::TiffDecodeOptions;
use pixel_container::PixelContainer;

/// Decode a Canon CR2 file.
///
/// # What this function does
///
/// 1. Validates the CR2 file signature (bytes 0–3 = TIFF LE header,
///    bytes 8–10 = "CR\x02").
/// 2. Parses the TIFF IFD chain to count the available IFDs.
/// 3. Selects IFD3 (the full-resolution RAW IFD) — or the last IFD if
///    fewer than 4 IFDs are present.
/// 4. Builds a `TiffDecodeOptions` struct with Canon-specific colour
///    parameters (hardcoded EOS 5D-era matrix, 14-bit black/white levels).
/// 5. Delegates all strip decompression, demosaicing, and colour rendering
///    to `image_codec_tiff::decode_tiff_with_opts`.
///
/// # Errors
///
/// Returns `Err(String)` for:
/// - File shorter than 16 bytes.
/// - Not a little-endian TIFF (bytes 0–1 != "II").
/// - Bad TIFF magic (bytes 2–3 != 42 LE).
/// - Missing CR2 signature (bytes 8–10 != "CR\x02").
/// - Any TIFF decode failure (corrupt IFD, unsupported compression, etc.)
pub fn decode_cr2(bytes: &[u8]) -> Result<PixelContainer, String> {
    // ── Step 1: validate the CR2 signature ─────────────────────────────────
    //
    // CR2 files begin with the standard TIFF little-endian header:
    //   bytes[0..2] = "II"   → little-endian byte order marker
    //   bytes[2..4] = 42     → TIFF magic number (LE u16)
    //   bytes[4..8] = IFD0 offset
    //
    // Immediately after the standard 8-byte TIFF header, CR2 stores a 4-byte
    // signature at bytes[8..12]:
    //   bytes[8]  = 'C' (0x43)
    //   bytes[9]  = 'R' (0x52)
    //   bytes[10] = 2   (CR2 version 2)
    //   bytes[11] = 0   (minor version)

    if bytes.len() < 16 {
        return Err("CR2: file too short (need at least 16 bytes)".into());
    }
    if &bytes[0..2] != b"II" {
        return Err(format!(
            "CR2: expected little-endian TIFF marker 'II', got {:02X} {:02X}",
            bytes[0], bytes[1]
        ));
    }
    let magic = u16::from_le_bytes([bytes[2], bytes[3]]);
    if magic != 42 {
        return Err(format!(
            "CR2: bad TIFF magic — expected 42, got {}",
            magic
        ));
    }
    if &bytes[8..10] != b"CR" || bytes[10] != 2 {
        return Err(format!(
            "CR2: missing CR2 signature at offset 8 — expected CR\\x02, got {:02X} {:02X} {:02X}",
            bytes[8], bytes[9], bytes[10]
        ));
    }

    // ── Step 2: parse the IFD chain ────────────────────────────────────────
    //
    // A standard CR2 file has 4 IFDs linked in a chain:
    //   IFD0 — JPEG thumbnail
    //   IFD1 — reduced-size image or absent
    //   IFD2 — reduced-size RAW or absent
    //   IFD3 — full-resolution CFA sensor data ← we want this one
    //
    // We ask image-codec-tiff to parse the chain and count IFDs so we can
    // pick the right index.

    let ifds = image_codec_tiff::parse_ifd_chain(bytes)
        .map_err(|e| format!("CR2: IFD parse failed: {}", e))?;

    // Use IFD3 if available; fall back to the last IFD for synthetic test files
    // that only have 1–3 IFDs.
    let raw_ifd_index = if ifds.len() >= 4 {
        3
    } else {
        ifds.len().saturating_sub(1)
    };

    // ── Step 3: build decode options with Canon colour parameters ──────────
    //
    // These values are hardcoded for the EOS 5D-era Canon DSLR family.
    // A production decoder would look up the exact model (via CanonModelID in
    // the MakerNote) and pick the right matrix from a table.
    //
    // See spec §4.2 and color_matrices.rs for the matrix derivation.

    let opts = TiffDecodeOptions {
        ifd_index: raw_ifd_index,
        wb_multipliers: [1.0, 1.0, 1.0], // D65 flat WB (no correction)
        color_matrix: CANON_COLOR_MATRIX,
        black_level: [CANON_BLACK_LEVEL; 4],
        white_level: CANON_WHITE_LEVEL,
    };

    // ── Step 4: decode via the TIFF engine ─────────────────────────────────
    //
    // `decode_tiff_with_opts` handles:
    //   - Strip decompression (uncompressed, PackBits, lossless JPEG 6/7)
    //   - Bayer demosaicing (bilinear RGGB)
    //   - Black level subtraction
    //   - White balance × colour matrix × sRGB gamma
    //   - Output to RGBA8 PixelContainer

    image_codec_tiff::decode_tiff_with_opts(bytes, &opts)
        .map_err(|e| format!("CR2: TIFF decode failed: {}", e))
}
