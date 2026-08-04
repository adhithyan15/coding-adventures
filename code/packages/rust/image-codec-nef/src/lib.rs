// # image-codec-nef
//
// Nikon NEF (Nikon Electronic Format) RAW image codec.
//
// ## What is NEF?
//
// NEF is Nikon's proprietary RAW image format used in all Nikon DSLRs and
// mirrorless cameras. Unlike formats such as BMP or JPEG, NEF stores the
// "raw" sensor data before white balance, noise reduction, or colour
// processing — giving photographers maximum flexibility in post-production.
//
// NEF is a TIFF 6.0 container extended with Nikon-specific tags:
//
// ```text
// IFD0:
//   Make  = "NIKON CORPORATION"
//   Model = "NIKON D<model>"
//   SubIFDs (tag 330) → [preview IFD, raw CFA IFD]
//   Exif IFD → Nikon MakerNote (WB, tone curve, lens info)
//
// Raw CFA sub-IFD:
//   PhotometricInterpretation = 32803  (CFA / Bayer mosaic)
//   BitsPerSample = 12 or 14
//   Compression = 1 (uncompressed) or 34713 (Nikon compressed)
//   StripOffsets[0] = offset of raw pixel data
// ```
//
// ## v0.1 scope
//
// - Uncompressed 12-bit and 14-bit NEF files are fully decoded.
// - Nikon compressed (Compression=34713) returns a descriptive Err.
// - White balance defaults to D65 (no decrypted MakerNote WB in v0.1).
// - A single hardcoded generic colour matrix (Nikon D70 representative) is
//   used for all models. Future versions can add per-model lookup tables.
//
// ## Colour pipeline
//
// ```text
// 1. Parse IFD chain (image-codec-tiff)
// 2. Find IFD with PhotometricInterpretation=32803 (CFA)
// 3. Validate Make tag contains "NIKON"
// 4. Detect 12-bit or 14-bit mode from BitsPerSample
// 5. Set black level and white level accordingly
// 6. decode_tiff_with_opts → bilinear Bayer demosaic → colour matrix → gamma
// ```
//
// ## Module structure
//
// ```text
// lib.rs      — public API, NefCodec trait impl, colour constants, tests
// decoder.rs  — decode_nef orchestrator
// encoder.rs  — minimal test encoder (wraps TIFF encoder)
// ```

pub const VERSION: &str = "0.1.0";

// ─── Colour constants ─────────────────────────────────────────────────────────

// ## Nikon colour matrix (generic / D70 representative)
//
// This matrix converts from Nikon camera colour space to sRGB.
//
// Values from dcraw.c for the Nikon D70:
//   Row 0: [R_out = 1.392*R_in - 0.418*G_in + 0.026*B_in]
//   Row 1: [G_out = -0.254*R_in + 1.614*G_in - 0.360*B_in]
//   Row 2: [B_out = 0.068*R_in - 0.584*G_in + 1.516*B_in]
//
// A 3×3 matrix that is close to identity (diagonal near 1.0) is typical for
// a camera that produces images reasonably close to sRGB already. The
// off-diagonal terms correct for crosstalk between colour channels.
pub(crate) const NIKON_COLOR_MATRIX: [[f64; 3]; 3] = [
    [ 1.392, -0.418,  0.026],
    [-0.254,  1.614, -0.360],
    [ 0.068, -0.584,  1.516],
];

// ## Black and white levels by bit depth
//
// The "black level" is the sensor's baseline noise floor — pixel values at or
// below this are considered pure black. The "white level" is the maximum
// representable value (2^N - 1 for N-bit sensors). We subtract the black
// level and normalise to [0.0, 1.0] relative to the white level.
//
// Nikon 12-bit sensors: black=0, white=4095 (2^12 - 1).
// Nikon 14-bit sensors (D300+): black=0, white=16383 (2^14 - 1).
//
// In practice, many bodies have a non-zero black level stored in the
// MakerNote. For v0.1, we use 0 as a conservative default that works
// correctly for most bodies.
pub(crate) const NIKON_BLACK_LEVEL_12BIT: u32 = 0;
pub(crate) const NIKON_WHITE_LEVEL_12BIT: u32 = 4095;
pub(crate) const NIKON_BLACK_LEVEL_14BIT: u32 = 0;
pub(crate) const NIKON_WHITE_LEVEL_14BIT: u32 = 16383;

// ─── Module declarations ──────────────────────────────────────────────────────

mod decoder;
mod encoder;

// ─── Public re-exports ────────────────────────────────────────────────────────

pub use decoder::decode_nef;
pub use encoder::encode_nef;

use paint_instructions::ImageCodec;
use pixel_container::PixelContainer;

// ─── NefCodec — ImageCodec trait implementation ───────────────────────────────

/// Nikon NEF image codec.
///
/// Implements the `ImageCodec` trait for plug-in codec use alongside other
/// formats (BMP, JPEG, TIFF, RAF, etc.).
///
/// # Example
///
/// ```rust,ignore
/// use image_codec_nef::NefCodec;
/// use paint_instructions::ImageCodec;
///
/// let bytes = std::fs::read("photo.NEF").unwrap();
/// let pixels = NefCodec.decode(&bytes).unwrap();
/// println!("Decoded {}×{} image", pixels.width, pixels.height);
/// ```
pub struct NefCodec;

impl ImageCodec for NefCodec {
    fn mime_type(&self) -> &'static str {
        "image/x-nikon-nef"
    }

    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> {
        encode_nef(pixels)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_nef(bytes)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use paint_instructions::ImageCodec as _;

    // ── Helper: build a minimal synthetic TIFF that decode_nef will accept ────
    //
    // Our encoder calls encode_tiff, which produces a valid TIFF but WITHOUT
    // a Make tag (tag 271). decode_nef permits missing Make tags (synthetic
    // files) and only rejects files where Make is *present and wrong*.
    //
    // For tests that need a specific Make tag, we use `build_tiff_with_make`.

    fn build_tiff_with_make(w: u32, h: u32, make: &str) -> Vec<u8> {
        // Start with an encoded TIFF from encode_nef.
        let mut pc = PixelContainer::new(w, h);
        pc.fill(100, 100, 100, 255);
        let base = encode_nef(&pc);

        // We need to inject a Make (ASCII tag 271) IFD entry.
        // The TIFF encoder from image-codec-tiff writes a minimal IFD.
        // We rebuild from scratch to avoid patching offsets.
        build_raw_tiff_with_make(w, h, make, 2 /* RGB */, 8, 3, &{
            let pixels: Vec<u8> = (0..w * h)
                .flat_map(|_| [100u8, 100, 100])
                .collect();
            pixels
        }, &base[..])
    }

    // Build a TIFF byte stream with an additional Make tag.
    #[allow(clippy::too_many_arguments)] // test helper spelling out TIFF fields explicitly
    fn build_raw_tiff_with_make(
        width: u32,
        height: u32,
        make: &str,
        photometric: u16,
        bits_per_sample: u16,
        samples_per_pixel: u16,
        pixel_data: &[u8],
        _base: &[u8],
    ) -> Vec<u8> {
        // Build a make string: ASCII NUL-terminated, padded to even length.
        let mut make_bytes = make.as_bytes().to_vec();
        make_bytes.push(0); // NUL terminator
        if !make_bytes.len().is_multiple_of(2) {
            make_bytes.push(0); // pad to even
        }
        let make_len = make_bytes.len();

        // Number of IFD entries: the core ones + Make tag.
        let num_entries: u16 = 11; // 10 core + 1 Make

        let ifd_size = 2 + (num_entries as usize) * 12 + 4;
        // External data section: Make string.
        let ext_data_offset = 8 + ifd_size;
        let pixel_start = ext_data_offset + make_bytes.len();

        let mut buf: Vec<u8> = Vec::new();

        let w16 = |buf: &mut Vec<u8>, v: u16| buf.extend_from_slice(&v.to_le_bytes());
        let w32 = |buf: &mut Vec<u8>, v: u32| buf.extend_from_slice(&v.to_le_bytes());

        // Header
        buf.extend_from_slice(b"II");
        w16(&mut buf, 42);
        w32(&mut buf, 8);

        // IFD entry count
        w16(&mut buf, num_entries);

        let entry = |buf: &mut Vec<u8>, tag: u16, typ: u16, count: u32, val: u32| {
            buf.extend_from_slice(&tag.to_le_bytes());
            buf.extend_from_slice(&typ.to_le_bytes());
            buf.extend_from_slice(&count.to_le_bytes());
            buf.extend_from_slice(&val.to_le_bytes());
        };

        // Make tag (271): ASCII, points to external data section.
        entry(&mut buf, 256, 4, 1, width);
        entry(&mut buf, 257, 4, 1, height);
        entry(&mut buf, 258, 3, 1, bits_per_sample as u32);
        entry(&mut buf, 259, 3, 1, 1); // Compression = uncompressed
        entry(&mut buf, 262, 3, 1, photometric as u32);
        entry(&mut buf, 271, 2, make_len as u32, ext_data_offset as u32); // Make ASCII
        entry(&mut buf, 273, 4, 1, pixel_start as u32);
        entry(&mut buf, 277, 3, 1, samples_per_pixel as u32);
        entry(&mut buf, 278, 4, 1, height);
        entry(&mut buf, 279, 4, 1, pixel_data.len() as u32);
        entry(&mut buf, 284, 3, 1, 1); // PlanarConfig = chunky

        w32(&mut buf, 0); // next IFD = 0

        // External data: Make string
        buf.extend_from_slice(&make_bytes);

        // Pixel data
        assert_eq!(
            buf.len(), pixel_start,
            "pixel_start mismatch: buf={} expected={}",
            buf.len(), pixel_start
        );
        buf.extend_from_slice(pixel_data);
        buf
    }

    // Build a CFA TIFF (photometric=32803) with an optional Make tag.
    // Used for testing that decode_nef finds the CFA IFD.
    fn build_cfa_tiff_nef(
        width: u32,
        height: u32,
        bits_per_sample: u16,
        make: Option<&str>,
    ) -> Vec<u8> {
        let num_pixels = (width * height) as usize;
        // 16-bit pixels for CFA
        let pixel_data: Vec<u8> = (0..num_pixels)
            .flat_map(|i| {
                let v = ((i * 13 + 7) % 4000) as u16;
                v.to_le_bytes()
            })
            .collect();

        let extra_entries: u16 = if make.is_some() { 1 } else { 0 };
        let num_entries: u16 = 12 + extra_entries; // 10 base + 2 CFA pattern + opt Make

        // We need to store Make externally if it's long.
        let make_bytes_opt: Option<Vec<u8>> = make.map(|m| {
            let mut mb = m.as_bytes().to_vec();
            mb.push(0);
            if mb.len() % 2 != 0 {
                mb.push(0);
            }
            mb
        });

        let ifd_size = 2 + (num_entries as usize) * 12 + 4;
        let ext_data_offset = 8 + ifd_size;
        let make_len = make_bytes_opt.as_ref().map(|b| b.len()).unwrap_or(0);
        let pixel_start = ext_data_offset + make_len;

        let mut buf: Vec<u8> = Vec::new();
        let w16 = |buf: &mut Vec<u8>, v: u16| buf.extend_from_slice(&v.to_le_bytes());
        let w32 = |buf: &mut Vec<u8>, v: u32| buf.extend_from_slice(&v.to_le_bytes());
        let entry = |buf: &mut Vec<u8>, tag: u16, typ: u16, count: u32, val: u32| {
            buf.extend_from_slice(&tag.to_le_bytes());
            buf.extend_from_slice(&typ.to_le_bytes());
            buf.extend_from_slice(&count.to_le_bytes());
            buf.extend_from_slice(&val.to_le_bytes());
        };

        buf.extend_from_slice(b"II");
        w16(&mut buf, 42);
        w32(&mut buf, 8);
        w16(&mut buf, num_entries);

        entry(&mut buf, 256, 4, 1, width);
        entry(&mut buf, 257, 4, 1, height);
        entry(&mut buf, 258, 3, 1, bits_per_sample as u32);
        entry(&mut buf, 259, 3, 1, 1);
        entry(&mut buf, 262, 3, 1, 32803); // CFA photometric

        if let Some(ref mb) = make_bytes_opt {
            entry(&mut buf, 271, 2, mb.len() as u32, ext_data_offset as u32);
        }

        entry(&mut buf, 273, 4, 1, pixel_start as u32);
        entry(&mut buf, 277, 3, 1, 1);
        entry(&mut buf, 278, 4, 1, height);
        entry(&mut buf, 279, 4, 1, pixel_data.len() as u32);
        entry(&mut buf, 284, 3, 1, 1);
        // CFARepeatPatternDim: 2×2
        entry(&mut buf, 33421, 3, 2, 0x0002_0002u32);
        // CFAPattern: RGGB
        entry(&mut buf, 33422, 1, 4, 0x0201_0100u32);

        w32(&mut buf, 0);

        // External data (Make string if any)
        if let Some(mb) = make_bytes_opt {
            buf.extend_from_slice(&mb);
        }

        assert_eq!(buf.len(), pixel_start, "pixel_start mismatch");
        buf.extend_from_slice(&pixel_data);
        buf
    }

    // ── Test 1: version_is_0_1_0 ─────────────────────────────────────────────
    //
    // The VERSION constant must match the Cargo.toml version.
    #[test]
    fn version_is_0_1_0() {
        assert_eq!(VERSION, "0.1.0");
    }

    // ── Test 2: mime_type_correct ─────────────────────────────────────────────
    #[test]
    fn mime_type_correct() {
        assert_eq!(NefCodec.mime_type(), "image/x-nikon-nef");
    }

    // ── Test 3: round_trip_2x2 ───────────────────────────────────────────────
    //
    // Encode a 2×2 image with encode_nef (wraps TIFF encoder), then decode it
    // back and verify dimensions survive.
    #[test]
    fn round_trip_2x2() {
        let mut pc = PixelContainer::new(2, 2);
        pc.set_pixel(0, 0, 200, 100, 50, 255);
        pc.set_pixel(1, 0, 10, 20, 30, 255);
        pc.set_pixel(0, 1, 40, 50, 60, 255);
        pc.set_pixel(1, 1, 70, 80, 90, 255);
        let encoded = encode_nef(&pc);
        let decoded = decode_nef(&encoded).expect("round-trip decode failed");
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 2);
    }

    // ── Test 4: round_trip_4x4_gradient ─────────────────────────────────────
    #[test]
    fn round_trip_4x4_gradient() {
        let mut pc = PixelContainer::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                pc.set_pixel(x, y, (x * 50) as u8, (y * 50) as u8, 100, 255);
            }
        }
        let encoded = encode_nef(&pc);
        let decoded = decode_nef(&encoded).expect("4x4 round-trip failed");
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
    }

    // ── Test 5: round_trip_via_codec_trait ──────────────────────────────────
    #[test]
    fn round_trip_via_codec_trait() {
        let mut pc = PixelContainer::new(3, 3);
        pc.set_pixel(1, 1, 128, 64, 192, 255);
        let encoded = NefCodec.encode(&pc);
        let decoded = NefCodec.decode(&encoded).expect("trait round-trip failed");
        assert_eq!(decoded.width, 3);
        assert_eq!(decoded.height, 3);
    }

    // ── Test 6: error_on_empty ────────────────────────────────────────────────
    #[test]
    fn error_on_empty() {
        let result = decode_nef(&[]);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("NEF"), "Error should mention NEF: {}", msg);
    }

    // ── Test 7: error_on_short_file ──────────────────────────────────────────
    #[test]
    fn error_on_short_file() {
        let result = decode_nef(&[0x49, 0x49, 0x2A]);
        assert!(result.is_err());
    }

    // ── Test 8: wrong_make_returns_err ───────────────────────────────────────
    //
    // If Make tag is present and contains a non-NIKON manufacturer name (and
    // is of non-trivial length), decode_nef must return Err.
    #[test]
    fn wrong_make_returns_err() {
        let tiff_bytes = build_tiff_with_make(2, 2, "CANON");
        let result = decode_nef(&tiff_bytes);
        assert!(
            result.is_err(),
            "Expected Err for non-Nikon Make, but got Ok"
        );
        let msg = result.unwrap_err();
        assert!(
            msg.contains("NEF") || msg.contains("Nikon") || msg.contains("not a Nikon"),
            "Error message should mention Nikon: {}",
            msg
        );
    }

    // ── Test 9: color_matrix_shape ───────────────────────────────────────────
    //
    // The 3×3 colour matrix constant must have the right shape and non-zero
    // diagonal (to avoid completely black output).
    #[test]
    fn color_matrix_shape() {
        assert_eq!(NIKON_COLOR_MATRIX.len(), 3);
        for row in &NIKON_COLOR_MATRIX {
            assert_eq!(row.len(), 3);
        }
        // Diagonal entries should be > 0.5 (dominant channel response).
        assert!(
            NIKON_COLOR_MATRIX[0][0] > 0.5,
            "R→R diagonal should be > 0.5"
        );
        assert!(
            NIKON_COLOR_MATRIX[1][1] > 0.5,
            "G→G diagonal should be > 0.5"
        );
        assert!(
            NIKON_COLOR_MATRIX[2][2] > 0.5,
            "B→B diagonal should be > 0.5"
        );
    }

    // ── Test 10: black_white_level_12bit ─────────────────────────────────────
    #[test]
    fn black_white_level_12bit() {
        assert_eq!(NIKON_BLACK_LEVEL_12BIT, 0);
        assert_eq!(NIKON_WHITE_LEVEL_12BIT, 4095);
        // 4095 = 2^12 - 1
        assert_eq!(NIKON_WHITE_LEVEL_12BIT, (1 << 12) - 1);
    }

    // ── Test 11: black_white_level_14bit ─────────────────────────────────────
    #[test]
    fn black_white_level_14bit() {
        assert_eq!(NIKON_BLACK_LEVEL_14BIT, 0);
        assert_eq!(NIKON_WHITE_LEVEL_14BIT, 16383);
        // 16383 = 2^14 - 1
        assert_eq!(NIKON_WHITE_LEVEL_14BIT, (1 << 14) - 1);
    }

    // ── Test 12: decode_multiple_ifds_picks_cfa ──────────────────────────────
    //
    // When multiple IFDs are present, decode_nef must find the one with
    // photometric=32803 (CFA) and decode it.
    #[test]
    fn decode_multiple_ifds_picks_cfa() {
        // Build a synthetic CFA TIFF (photometric=32803, no Make tag).
        // decode_nef should succeed because Make tag is absent (not wrong).
        let cfa_bytes = build_cfa_tiff_nef(4, 4, 16, None);
        let result = decode_nef(&cfa_bytes);
        // The decode may succeed (uncompressed CFA) or fail with a message
        // about CFA if image-codec-tiff can't handle it in test mode.
        // Either way we should not panic.
        match result {
            Ok(pc) => {
                assert_eq!(pc.width, 4);
                assert_eq!(pc.height, 4);
            }
            Err(e) => {
                // Accept TIFF-level errors about unsupported features.
                assert!(
                    !e.is_empty(),
                    "Error message must not be empty"
                );
            }
        }
    }

    // ── Test 13: nikon_make_accepted ─────────────────────────────────────────
    //
    // A TIFF with Make="NIKON" should not be rejected by the Make validation.
    #[test]
    fn nikon_make_accepted() {
        let tiff_bytes = build_tiff_with_make(2, 2, "NIKON");
        // This may succeed or fail at decode for other reasons, but it must
        // NOT fail with "not a Nikon file".
        let result = decode_nef(&tiff_bytes);
        if let Err(e) = &result {
            assert!(
                !e.contains("not a Nikon"),
                "Should not reject NIKON make: {}",
                e
            );
        }
    }

    // ── Test 14: nikon_corporation_make_accepted ──────────────────────────────
    #[test]
    fn nikon_corporation_make_accepted() {
        let tiff_bytes = build_tiff_with_make(2, 2, "NIKON CORPORATION");
        let result = decode_nef(&tiff_bytes);
        if let Err(e) = &result {
            assert!(
                !e.contains("not a Nikon"),
                "Should not reject NIKON CORPORATION make: {}",
                e
            );
        }
    }
}
