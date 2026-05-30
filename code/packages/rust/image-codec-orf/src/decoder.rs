// # decoder.rs — top-level ORF decode orchestrator
//
// This module contains the main `decode_orf` function and its helper
// `normalize_orf_magic`.
//
// ## Decode flow
//
// ```text
// 1. Validate file length (≥ 8 bytes)
// 2. Validate byte-order marker (must be "II" — LE)
// 3. Normalize IIRO magic → standard TIFF magic (if needed)
// 4. Parse IFD chain via image_codec_tiff::parse_ifd_chain
// 5. Validate Make tag (reject non-Olympus if tag present)
// 6. Find the CFA IFD (PhotometricInterpretation = 32803)
// 7. Decode via image_codec_tiff::decode_tiff_with_opts
// ```
//
// ## IIRO magic
//
// Standard TIFF little-endian header: bytes[0..4] = b"II" + [0x2A, 0x00]
//
// Some Olympus cameras (older models, firmware variants) write a non-standard
// magic: bytes[0..4] = b"II" + [0x52, 0x4F] = b"IIRO"
//
// The Olympus extension was never formally standardised; different cameras use
// it inconsistently.  The safest approach (also used by dcraw and LibRaw) is to
// patch the two non-standard bytes to [0x2A, 0x00] before parsing.  Everything
// else in the file is standard little-endian TIFF, so this works correctly.
//
// ## Make tag (tag 271)
//
// Tag 271 in IFD0 identifies the camera manufacturer as an ASCII string.
// Olympus ORF files have:
//   "OLYMPUS IMAGING CORP."  — older Olympus bodies
//   "OLYMPUS CORPORATION"    — E-M series and PEN series
//   "OM Digital Solutions"   — recent OM SYSTEM bodies
//
// If the tag is absent (our encoder doesn't write it, nor do most minimal TIFFs),
// we pass through without complaint.  If the tag is present and doesn't contain
// "OLYMPUS" or "OM DIGITAL" (case-insensitive), we reject the file to prevent
// silently misinterpreting Canon, Nikon, or Sony RAW files as ORF.

use crate::{OLYMPUS_BLACK_LEVEL, OLYMPUS_COLOR_MATRIX, OLYMPUS_WHITE_LEVEL};
use image_codec_tiff::{IfdValue, TiffDecodeOptions};
use pixel_container::PixelContainer;

/// Decode an ORF (Olympus RAW Format) byte stream into an RGBA8 PixelContainer.
///
/// # Arguments
///
/// * `bytes` — Raw file bytes.  May be a standard TIFF magic or IIRO variant.
///
/// # Errors
///
/// Returns `Err(String)` for:
/// - Fewer than 8 bytes
/// - Big-endian byte order marker (`MM`)
/// - Make tag present but not Olympus / OM Digital
/// - Any TIFF parse error from image-codec-tiff
pub fn decode_orf(bytes: &[u8]) -> Result<PixelContainer, String> {
    // ── Step 1: length guard ─────────────────────────────────────────────────
    //
    // A valid TIFF header is exactly 8 bytes (2 byte-order + 2 magic + 4 IFD0
    // offset).  Anything shorter cannot possibly be a valid ORF file.

    if bytes.len() < 8 {
        return Err("ORF: file too short".into());
    }

    // ── Step 2: byte-order check ─────────────────────────────────────────────
    //
    // ORF is always little-endian ("II").  Big-endian ("MM") is rejected.
    // The check is intentionally simple: only "II" is accepted.

    if &bytes[0..2] != b"II" {
        return Err("ORF: expected little-endian byte order".into());
    }

    // ── Step 3: IIRO magic normalisation ─────────────────────────────────────
    //
    // Uses Cow<[u8]> so that standard-magic files avoid the allocation.

    let normalized = normalize_orf_magic(bytes);
    let data = normalized.as_ref();

    // ── Step 4: parse IFD chain ───────────────────────────────────────────────
    //
    // image_codec_tiff::parse_ifd_chain returns a Vec<Ifd>, one entry per IFD
    // in the linked list (IFD0 → IFD1 → … → 0 terminator).

    let ifds = image_codec_tiff::parse_ifd_chain(data)
        .map_err(|e| format!("ORF: {}", e))?;

    // ── Step 5: Make tag validation ───────────────────────────────────────────
    //
    // Tag 271 (0x010F) stores the camera manufacturer as a null-terminated
    // ASCII string in the extra_tags map.
    //
    // Logic:
    //   - Tag absent: no rejection (our encoder doesn't emit it)
    //   - Tag present: check case-insensitively for "OLYMPUS" or "OM DIGITAL"
    //   - Tag present but wrong: reject

    let has_wrong_make = ifds.first().map(|ifd| {
        ifd.extra_tags
            .get(&271)
            .map(|tag_val| {
                // Make tag (271) is typically ASCII or raw bytes.
                // Extract the string from whichever IfdValue variant we got.
                let make_str = match tag_val {
                    IfdValue::Ascii(s) => s.clone(),
                    IfdValue::Bytes(b) => String::from_utf8_lossy(b).into_owned(),
                    _ => return false, // unexpected type — don't reject
                };
                let upper = make_str.to_uppercase();
                !upper.contains("OLYMPUS") && !upper.contains("OM DIGITAL") && upper.len() > 2
            })
            .unwrap_or(false)
    }).unwrap_or(false);

    if has_wrong_make {
        return Err("ORF: Make tag indicates this is not an Olympus file".into());
    }

    // ── Step 6: find the CFA IFD ──────────────────────────────────────────────
    //
    // Older Olympus bodies (E-1, E-500) store the raw CFA data directly in
    // IFD0.  Newer bodies (E-M series) use a Sub-IFD (tag 330).
    //
    // image_codec_tiff::parse_ifd_chain follows the linked-list chain AND
    // recurses into SubIFDs, so all IFDs appear in the Vec in traversal order.
    //
    // We search for the first IFD where PhotometricInterpretation = 32803 (CFA).
    // If none is found we fall back to IFD0 (index 0), which handles the case
    // where the file was produced by our own encoder (not a camera body).

    let raw_ifd_index = ifds
        .iter()
        .position(|ifd| ifd.photometric == 32803)
        .unwrap_or(0);

    // ── Step 7: decode with Olympus colour pipeline ───────────────────────────
    //
    // TiffDecodeOptions bundles all the parameters that vary by camera model:
    //   ifd_index    — which IFD holds the CFA data
    //   wb_multipliers — white balance (1.0 = no adjustment; real ORF parsers
    //                    would read these from MakerNote tag 0x1017/0x1018)
    //   color_matrix — camera-native → linear sRGB (E-M1 Mk II characterisation)
    //   black_level  — [256, 256, 256, 256] for all Bayer channels
    //   white_level  — 4095 (12-bit maximum)

    let opts = TiffDecodeOptions {
        ifd_index: raw_ifd_index,
        wb_multipliers: [1.0, 1.0, 1.0],
        color_matrix: OLYMPUS_COLOR_MATRIX,
        black_level: [OLYMPUS_BLACK_LEVEL; 4],
        white_level: OLYMPUS_WHITE_LEVEL,
    };

    image_codec_tiff::decode_tiff_with_opts(data, &opts)
        .map_err(|e| {
            // Surface a friendly message for the known unsupported compression.
            if e.contains("32767") || e.contains("compression") {
                "ORF: Olympus compressed format (32767) not supported; use uncompressed ORF"
                    .to_string()
            } else {
                format!("ORF: {}", e)
            }
        })
}

// ─── normalize_orf_magic ──────────────────────────────────────────────────────
//
// Patches the IIRO magic variant in-place (via a Vec clone) so that
// image_codec_tiff::parse_ifd_chain accepts the bytes.
//
// ## IIRO magic byte layout
//
// ```
// byte 0: 0x49  ('I') ┐
// byte 1: 0x49  ('I') ┘  byte-order marker "II"
// byte 2: 0x52  ('R') ← non-standard; standard = 0x2A (42)
// byte 3: 0x4F  ('O') ← non-standard; standard = 0x00
// ```
//
// Standard TIFF reads the 16-bit version field as LE u16: [0x2A, 0x00] = 42.
// IIRO reads it as: [0x52, 0x4F] = 0x4F52 = 20306.
//
// We replace bytes 2 and 3 with [42, 0] to produce a standard TIFF header.
// Everything else in the file is already standard LE TIFF, so no other changes
// are needed.
//
// Cow<[u8]> avoids an allocation for files that already have standard magic.

fn normalize_orf_magic(bytes: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    if bytes.len() >= 4
        && &bytes[0..2] == b"II"
        && bytes[2] == 0x52
        && bytes[3] == 0x4F
    {
        // IIRO variant: replace magic 0x4F52 with standard 42 (0x2A00)
        let mut patched = bytes.to_vec();
        patched[2] = 42;
        patched[3] = 0;
        std::borrow::Cow::Owned(patched)
    } else {
        std::borrow::Cow::Borrowed(bytes)
    }
}
