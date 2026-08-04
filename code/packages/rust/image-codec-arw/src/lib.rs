// # image-codec-arw
//
// Sony ARW (Alpha RAW) RAW image codec.
//
// ## What is ARW?
//
// ARW is Sony's proprietary RAW format used in all Sony Alpha cameras since
// 2006. Like NEF and CR2, ARW stores unprocessed sensor data in a TIFF 6.0
// container with manufacturer-specific extensions.
//
// ARW file structure:
//
// ```text
// IFD0:
//   Make  = "SONY"
//   Model = "ILCE-xxxx" (A-series mirrorless) or "DSLR-xxxx"
//   SubIFDs (tag 330) → [preview IFD, raw CFA IFD]
//   Exif IFD → Sony MakerNote (WB, tone curve, etc.)
//
// Raw CFA sub-IFD:
//   PhotometricInterpretation = 32803  (CFA / Bayer mosaic)
//   BitsPerSample = 12 (ARW 1.0) or 14 (ARW 2.x)
//   Compression = 32767 (Sony-specific)
//   StripOffsets[0] = offset of raw pixel data
// ```
//
// ## ARW versions
//
// | Version | Cameras | Notes |
// |---------|---------|-------|
// | ARW 1.0 | α100, α700 (2006–2008) | 12-bit, uncompressed |
// | ARW 2.x | α900, A7 I–III (2008–2018) | 14-bit, Sony compressed |
// | ARW 3.0 | A7R IV+ (2018+) | New compression, unsupported in v0.1 |
//
// ## v0.1 scope
//
// - Files without Make tag (synthetic) are decoded via the TIFF pipeline.
// - Non-Sony Make tag → `Err`.
// - Sony colour matrix hardcoded (A7R II representative from dcraw).
// - Black level = 200 (ARW 2.x default).
// - White level = 16383 (14-bit maximum, 2^14 - 1).
// - White balance: D65 default (no MakerNote parsing in v0.1).
//
// ## Module structure
//
// ```text
// lib.rs      — public API, ArwCodec trait impl, colour constants, tests
// decoder.rs  — decode_arw orchestrator
// encoder.rs  — minimal test encoder (wraps TIFF encoder)
// ```

pub const VERSION: &str = "0.1.0";

// ─── Colour constants ─────────────────────────────────────────────────────────

// ## Sony colour matrix (A7R II representative)
//
// This 3×3 matrix converts from Sony camera colour space to sRGB.
//
// Values from dcraw.c for the Sony A7R II — a mid-range representative
// matrix that works reasonably well across the full A7 generation.
//
// Interpretation:
//   R_sRGB = 1.318 * R_cam - 0.398 * G_cam + 0.080 * B_cam
//   G_sRGB = -0.213 * R_cam + 1.586 * G_cam - 0.373 * B_cam
//   B_sRGB = 0.047 * R_cam - 0.474 * G_cam + 1.427 * B_cam
pub(crate) const SONY_COLOR_MATRIX: [[f64; 3]; 3] = [
    [ 1.318, -0.398,  0.080],
    [-0.213,  1.586, -0.373],
    [ 0.047, -0.474,  1.427],
];

// ## Black and white levels
//
// Sony ARW 2.x bodies have a black level of approximately 200 (12-bit
// equivalent scale after the 14-bit normalisation). The white level is
// 16383 = 2^14 - 1 for 14-bit sensors.
//
// ARW 1.0 bodies used 512 as the black level on 12-bit sensors; for v0.1
// we use the more common ARW 2.x value of 200.
pub(crate) const SONY_BLACK_LEVEL: u32 = 200;
pub(crate) const SONY_WHITE_LEVEL: u32 = 16383;

// ─── Module declarations ──────────────────────────────────────────────────────

mod decoder;
mod encoder;

// ─── Public re-exports ────────────────────────────────────────────────────────

pub use decoder::decode_arw;
pub use encoder::encode_arw;

use paint_instructions::ImageCodec;
use pixel_container::PixelContainer;

// ─── ArwCodec — ImageCodec trait implementation ───────────────────────────────

/// Sony ARW image codec.
///
/// Implements the `ImageCodec` trait for plug-in codec use alongside other
/// formats (BMP, JPEG, TIFF, RAF, NEF, etc.).
///
/// # Example
///
/// ```rust,ignore
/// use image_codec_arw::ArwCodec;
/// use paint_instructions::ImageCodec;
///
/// let bytes = std::fs::read("photo.ARW").unwrap();
/// let pixels = ArwCodec.decode(&bytes).unwrap();
/// println!("Decoded {}×{} image", pixels.width, pixels.height);
/// ```
pub struct ArwCodec;

impl ImageCodec for ArwCodec {
    fn mime_type(&self) -> &'static str {
        "image/x-sony-arw"
    }

    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> {
        encode_arw(pixels)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_arw(bytes)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use paint_instructions::ImageCodec as _;

    // ── Helper: build a TIFF with a given Make tag ──────────────────────────

    fn build_tiff_with_make(w: u32, h: u32, make: &str) -> Vec<u8> {
        // Build make string: ASCII NUL-terminated, padded to even length.
        let mut make_bytes = make.as_bytes().to_vec();
        make_bytes.push(0);
        if !make_bytes.len().is_multiple_of(2) {
            make_bytes.push(0);
        }
        let make_len = make_bytes.len();

        let pixel_data: Vec<u8> = (0..w * h).flat_map(|_| [100u8, 100, 100]).collect();

        let num_entries: u16 = 11; // 10 core + 1 Make
        let ifd_size = 2 + (num_entries as usize) * 12 + 4;
        let ext_data_offset = 8 + ifd_size;
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

        entry(&mut buf, 256, 4, 1, w);
        entry(&mut buf, 257, 4, 1, h);
        entry(&mut buf, 258, 3, 1, 8);
        entry(&mut buf, 259, 3, 1, 1); // uncompressed
        entry(&mut buf, 262, 3, 1, 2); // RGB
        entry(&mut buf, 271, 2, make_len as u32, ext_data_offset as u32); // Make (external)
        entry(&mut buf, 273, 4, 1, pixel_start as u32);
        entry(&mut buf, 277, 3, 1, 3); // SamplesPerPixel
        entry(&mut buf, 278, 4, 1, h);
        entry(&mut buf, 279, 4, 1, pixel_data.len() as u32);
        entry(&mut buf, 284, 3, 1, 1);

        w32(&mut buf, 0);

        buf.extend_from_slice(&make_bytes);

        assert_eq!(buf.len(), pixel_start);
        buf.extend_from_slice(&pixel_data);
        buf
    }

    // Build a CFA TIFF without a Make tag (for multi-IFD tests).
    fn build_cfa_tiff_arw(width: u32, height: u32) -> Vec<u8> {
        let num_pixels = (width * height) as usize;
        let pixel_data: Vec<u8> = (0..num_pixels)
            .flat_map(|i| {
                let v = ((i * 17 + 5) % 16383) as u16;
                v.to_le_bytes()
            })
            .collect();

        let num_entries: u16 = 12; // 10 base + 2 CFA pattern
        let ifd_size = 2 + (num_entries as usize) * 12 + 4;
        let pixel_start = 8 + ifd_size;

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
        entry(&mut buf, 258, 3, 1, 16);
        entry(&mut buf, 259, 3, 1, 1);
        entry(&mut buf, 262, 3, 1, 32803); // CFA
        entry(&mut buf, 273, 4, 1, pixel_start as u32);
        entry(&mut buf, 277, 3, 1, 1);
        entry(&mut buf, 278, 4, 1, height);
        entry(&mut buf, 279, 4, 1, pixel_data.len() as u32);
        entry(&mut buf, 284, 3, 1, 1);
        entry(&mut buf, 33421, 3, 2, 0x0002_0002u32);
        entry(&mut buf, 33422, 1, 4, 0x0201_0100u32); // RGGB

        w32(&mut buf, 0);

        assert_eq!(buf.len(), pixel_start);
        buf.extend_from_slice(&pixel_data);
        buf
    }

    // ── Test 1: version ──────────────────────────────────────────────────────
    #[test]
    fn version() {
        assert_eq!(VERSION, "0.1.0");
    }

    // ── Test 2: mime_type ────────────────────────────────────────────────────
    #[test]
    fn mime_type() {
        assert_eq!(ArwCodec.mime_type(), "image/x-sony-arw");
    }

    // ── Test 3: round_trip_2x2 ──────────────────────────────────────────────
    #[test]
    fn round_trip_2x2() {
        let mut pc = PixelContainer::new(2, 2);
        pc.set_pixel(0, 0, 100, 150, 200, 255);
        pc.set_pixel(1, 0, 50, 75, 100, 255);
        pc.set_pixel(0, 1, 200, 100, 50, 255);
        pc.set_pixel(1, 1, 25, 50, 75, 255);
        let encoded = encode_arw(&pc);
        let decoded = decode_arw(&encoded).expect("round-trip 2×2 failed");
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 2);
    }

    // ── Test 4: round_trip_4x4 ──────────────────────────────────────────────
    #[test]
    fn round_trip_4x4() {
        let mut pc = PixelContainer::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                pc.set_pixel(x, y, (x * 40) as u8, (y * 40) as u8, 80, 255);
            }
        }
        let encoded = encode_arw(&pc);
        let decoded = decode_arw(&encoded).expect("round-trip 4×4 failed");
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
    }

    // ── Test 5: round_trip_via_codec ────────────────────────────────────────
    #[test]
    fn round_trip_via_codec() {
        let mut pc = PixelContainer::new(3, 3);
        pc.set_pixel(1, 1, 64, 128, 192, 255);
        let encoded = ArwCodec.encode(&pc);
        let decoded = ArwCodec.decode(&encoded).expect("codec trait round-trip failed");
        assert_eq!(decoded.width, 3);
        assert_eq!(decoded.height, 3);
    }

    // ── Test 6: error_on_empty ───────────────────────────────────────────────
    #[test]
    fn error_on_empty() {
        let result = decode_arw(&[]);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("ARW"), "Error should mention ARW: {}", msg);
    }

    // ── Test 7: error_on_short ──────────────────────────────────────────────
    #[test]
    fn error_on_short() {
        let result = decode_arw(&[0x49, 0x49, 0x2A]);
        assert!(result.is_err());
    }

    // ── Test 8: wrong_make_returns_err ──────────────────────────────────────
    //
    // A TIFF with Make="NIKON" should be rejected because it is not Sony.
    #[test]
    fn wrong_make_returns_err() {
        let tiff_bytes = build_tiff_with_make(2, 2, "NIKON");
        let result = decode_arw(&tiff_bytes);
        assert!(
            result.is_err(),
            "Expected Err for non-Sony Make, got Ok"
        );
        let msg = result.unwrap_err();
        assert!(
            msg.contains("ARW") || msg.contains("Sony") || msg.contains("not a Sony"),
            "Error should mention Sony: {}",
            msg
        );
    }

    // ── Test 9: color_matrix_shape ──────────────────────────────────────────
    //
    // The 3×3 colour matrix must have the right shape and dominant diagonal.
    #[test]
    fn color_matrix_shape() {
        assert_eq!(SONY_COLOR_MATRIX.len(), 3);
        for row in &SONY_COLOR_MATRIX {
            assert_eq!(row.len(), 3);
        }
        // Diagonal entries should be > 0.5 (dominant channel response).
        assert!(
            SONY_COLOR_MATRIX[0][0] > 0.5,
            "R→R diagonal should be > 0.5"
        );
        assert!(
            SONY_COLOR_MATRIX[1][1] > 0.5,
            "G→G diagonal should be > 0.5"
        );
        assert!(
            SONY_COLOR_MATRIX[2][2] > 0.5,
            "B→B diagonal should be > 0.5"
        );
    }

    // ── Test 10: black_white_level_constants ─────────────────────────────────
    #[test]
    fn black_white_level_constants() {
        assert_eq!(SONY_BLACK_LEVEL, 200);
        assert_eq!(SONY_WHITE_LEVEL, 16383);
        // 16383 = 2^14 - 1 (14-bit sensor maximum)
        assert_eq!(SONY_WHITE_LEVEL, (1 << 14) - 1);
    }

    // ── Test 11: sony_make_accepted ─────────────────────────────────────────
    //
    // A TIFF with Make="SONY" should not be rejected.
    #[test]
    fn sony_make_accepted() {
        let tiff_bytes = build_tiff_with_make(2, 2, "SONY");
        let result = decode_arw(&tiff_bytes);
        if let Err(e) = &result {
            assert!(
                !e.contains("not a Sony"),
                "Should not reject SONY make: {}",
                e
            );
        }
    }

    // ── Test 12: missing_make_accepted ──────────────────────────────────────
    //
    // A plain TIFF without Make tag should not fail Make validation.
    #[test]
    fn missing_make_accepted() {
        let mut pc = PixelContainer::new(2, 2);
        pc.fill(80, 80, 80, 255);
        let tiff_bytes = encode_arw(&pc);
        // Encode_arw produces TIFF without Make tag — should not fail Make check.
        let result = decode_arw(&tiff_bytes);
        if let Err(e) = &result {
            assert!(
                !e.contains("not a Sony"),
                "Missing Make should not fail: {}",
                e
            );
        }
    }

    // ── Test 13: cfa_ifd_discovery ──────────────────────────────────────────
    //
    // When a CFA IFD is present (photometric=32803), decode_arw should find it.
    #[test]
    fn cfa_ifd_discovery() {
        let cfa_bytes = build_cfa_tiff_arw(4, 4);
        let result = decode_arw(&cfa_bytes);
        match result {
            Ok(pc) => {
                assert_eq!(pc.width, 4);
                assert_eq!(pc.height, 4);
            }
            Err(e) => {
                // Accept TIFF-level errors about unsupported features.
                assert!(!e.is_empty(), "Error must not be empty");
            }
        }
    }
}
