// # image-codec-dng
//
// Adobe DNG (Digital Negative) image codec — decodes DNG RAW camera files to
// RGBA8 pixels and encodes `PixelContainer` to minimal DNG-compatible TIFF files.
//
// ## What is DNG?
//
// DNG is an open RAW image format created by Adobe in 2004 and now at version
// 1.6. It is a strict superset of TIFF 6.0: every DNG file is a valid TIFF
// file, extended with private tags (IDs 50700–51100+) that carry the camera's
// colour calibration data.
//
// Many cameras output DNG directly (Google Pixel, Leica, Hasselblad, Pentax).
// Others can be converted using Adobe's free DNG Converter. The open spec means
// no reverse engineering is needed — unlike Canon CR2, Nikon NEF, or Sony ARW.
//
// ## Architecture
//
// This crate is a thin shim over `image-codec-tiff`. The TIFF decoder already
// handles IFD parsing, strip decompression, Bayer demosaicing, and the RAW
// colour pipeline. The DNG layer only needs to:
//
// 1. Find the right IFD (the raw image, not the thumbnail)
// 2. Extract DNG colour calibration tags (ForwardMatrix, ColorMatrix, AsShotNeutral)
// 3. Build `TiffDecodeOptions` and pass everything to `decode_tiff_with_opts`
//
// ## Module map
//
// ```text
// lib.rs       ← public API, DngCodec trait impl, VERSION
// tags.rs      ← DNG private tag ID constants (50706–50880)
// color.rs     ← WB from AsShotNeutral, matrix math, XYZ D50 → sRGB
// decoder.rs   ← find raw IFD, extract tags, call decode_tiff_with_opts
// encoder.rs   ← minimal synthetic DNG writer (encode as TIFF)
// ```
//
// ## Colour pipeline overview
//
// ```text
// Raw sensor values (12 or 14 bit)
//   ↓  subtract BlackLevel
//   ↓  divide by WhiteLevel
//   ↓  Bayer demosaicing → camera linear RGB [0, 1]
//   ↓  multiply by white balance [WB_R, 1.0, WB_B]  ← from AsShotNeutral
//   ↓  multiply by colour matrix (camera → sRGB)     ← ForwardMatrix or inv(ColorMatrix)
//   ↓  clamp to [0, 1]
//   ↓  apply sRGB gamma curve (done in image-codec-tiff)
//   ↓  scale to u8 [0, 255]
// Output RGBA8 PixelContainer
// ```

pub const VERSION: &str = "0.1.0";

// ─── Module declarations ──────────────────────────────────────────────────────

pub mod color;
pub mod tags;
mod decoder;
mod encoder;

// ─── Public API ───────────────────────────────────────────────────────────────

use paint_instructions::ImageCodec;
use pixel_container::PixelContainer;

/// Decode a DNG file to an RGBA8 `PixelContainer`.
///
/// Automatically extracts DNG colour calibration tags (AsShotNeutral,
/// ForwardMatrix1, ColorMatrix1, BlackLevel, WhiteLevel) and applies the
/// full colour pipeline.
///
/// # Errors
///
/// Returns `Err(String)` for:
/// - Invalid TIFF/DNG header
/// - Unsupported compression type
/// - Truncated pixel data
///
/// # Example
///
/// ```rust,ignore
/// let dng_bytes = std::fs::read("photo.dng").unwrap();
/// let pixels = image_codec_dng::decode_dng(&dng_bytes)?;
/// println!("{}×{} image", pixels.width, pixels.height);
/// ```
pub fn decode_dng(bytes: &[u8]) -> Result<PixelContainer, String> {
    decoder::decode_dng(bytes)
}

/// Encode a `PixelContainer` as a minimal DNG-compatible TIFF file.
///
/// The output is an uncompressed RGB TIFF — a valid minimal DNG since DNG is
/// a superset of TIFF. For round-trip testing only.
///
/// # Example
///
/// ```rust,ignore
/// let mut pc = PixelContainer::new(4, 4);
/// pc.fill(200, 150, 100, 255);
/// let bytes = image_codec_dng::encode_dng(&pc);
/// let decoded = image_codec_dng::decode_dng(&bytes).unwrap();
/// assert_eq!(decoded.width, 4);
/// ```
pub fn encode_dng(pixels: &PixelContainer) -> Vec<u8> {
    encoder::encode_dng(pixels)
}

// ─── DngCodec — ImageCodec trait implementation ───────────────────────────────

/// A DNG image codec that implements the `ImageCodec` trait.
///
/// Plug into any pipeline that accepts `ImageCodec` objects:
///
/// ```rust,ignore
/// use image_codec_dng::DngCodec;
/// use paint_instructions::ImageCodec;
///
/// let codec: &dyn ImageCodec = &DngCodec;
/// let pixels = codec.decode(&dng_bytes)?;
/// let re_encoded = codec.encode(&pixels);
/// ```
pub struct DngCodec;

impl ImageCodec for DngCodec {
    fn mime_type(&self) -> &'static str {
        "image/x-adobe-dng"
    }

    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> {
        encode_dng(pixels)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_dng(bytes)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Constant and trait tests ──────────────────────────────────────────

    /// The crate's version string must match the Cargo.toml version.
    #[test]
    fn version_is_0_1_0() {
        assert_eq!(VERSION, "0.1.0");
    }

    /// The DNG codec must report the correct MIME type for DNG files.
    ///
    /// The IANA-registered MIME type for DNG is `image/x-adobe-dng`.
    #[test]
    fn mime_type() {
        assert_eq!(DngCodec.mime_type(), "image/x-adobe-dng");
    }

    // ── Round-trip tests ──────────────────────────────────────────────────

    /// Encode a solid red 4×4 image as DNG and decode it back.
    ///
    /// The encoder produces a plain TIFF (valid DNG superset). The decoder
    /// should return the same dimensions without panicking.
    ///
    /// Note: pixel values may differ slightly due to the colour pipeline
    /// (identity WB and matrix applied). We only verify dimensions.
    #[test]
    fn round_trip_solid_red() {
        let mut px = PixelContainer::new(4, 4);
        px.fill(200, 50, 30, 255);
        let bytes = encode_dng(&px);
        let decoded = decode_dng(&bytes).unwrap();
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
        // Values may differ due to colour pipeline; just check dims and no panic.
    }

    /// Round-trip a 2×2 grey image via the DngCodec trait interface.
    ///
    /// Tests that `DngCodec.encode` + `DngCodec.decode` work together without
    /// going through the free functions directly.
    #[test]
    fn round_trip_via_codec_trait() {
        let mut px = PixelContainer::new(2, 2);
        px.fill(128, 128, 128, 255);
        let bytes = DngCodec.encode(&px);
        let decoded = DngCodec.decode(&bytes).unwrap();
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 2);
    }

    /// Round-trip a 1×1 pixel image.
    ///
    /// The TIFF encoder can handle single-pixel images; the decoder must
    /// also handle them without an off-by-one or empty-strip error.
    #[test]
    fn round_trip_1x1() {
        let mut px = PixelContainer::new(1, 1);
        px.fill(100, 150, 200, 255);
        let bytes = encode_dng(&px);
        let decoded = decode_dng(&bytes).unwrap();
        assert_eq!(decoded.width, 1);
        assert_eq!(decoded.height, 1);
    }

    // ── White balance tests ───────────────────────────────────────────────

    /// Identity white balance: AsShotNeutral=[1,1,1] → WB=[1,1,1].
    ///
    /// When the camera sees a neutral grey as equal R=G=B, no correction is
    /// needed. The function must return [1.0, 1.0, 1.0].
    #[test]
    fn wb_from_as_shot_neutral_identity() {
        use crate::color::wb_from_as_shot_neutral;
        let wb = wb_from_as_shot_neutral(&[1.0, 1.0, 1.0]);
        assert!((wb[0] - 1.0).abs() < 1e-9, "R should be 1.0");
        assert!((wb[1] - 1.0).abs() < 1e-9, "G should be 1.0");
        assert!((wb[2] - 1.0).abs() < 1e-9, "B should be 1.0");
    }

    /// Normalised white balance: G channel must always be 1.0.
    ///
    /// Given AsShotNeutral=[0.5, 1.0, 0.5]:
    ///   WB = [1/0.5, 1/1.0, 1/0.5] = [2.0, 1.0, 2.0]
    ///   Normalise by G=1.0 → [2.0, 1.0, 2.0]
    ///
    /// G is always 1.0 after normalisation; R and B scale relative to G.
    #[test]
    fn wb_from_as_shot_neutral_normalised_green() {
        use crate::color::wb_from_as_shot_neutral;
        let wb = wb_from_as_shot_neutral(&[0.5, 1.0, 0.5]);
        assert!((wb[1] - 1.0).abs() < 1e-9, "G always 1.0 after normalise");
        assert!((wb[0] - 2.0).abs() < 1e-6, "R = 1/0.5 / 1.0 = 2.0");
        assert!((wb[2] - 2.0).abs() < 1e-6, "B = 1/0.5 / 1.0 = 2.0");
    }

    /// Empty neutrals slice must not panic, returns [1,1,1] identity.
    ///
    /// Defensive: if a DNG file is missing AsShotNeutral, the decoder
    /// defaults to [1,1,1]. This test checks the fallback directly.
    #[test]
    fn wb_empty_neutrals_returns_default() {
        use crate::color::wb_from_as_shot_neutral;
        let wb = wb_from_as_shot_neutral(&[]);
        assert_eq!(wb, [1.0, 1.0, 1.0]);
    }

    /// Typical daylight WB: R neutral lower than G (camera sees more red).
    ///
    /// AsShotNeutral=[0.5, 1.0, 0.7] (typical daylight on a bright sensor):
    ///   WB = [1/0.5, 1/1.0, 1/0.7] = [2.0, 1.0, ~1.43]
    ///   G normalised to 1.0, R > 1.0, B > 1.0
    #[test]
    fn wb_typical_daylight() {
        use crate::color::wb_from_as_shot_neutral;
        let wb = wb_from_as_shot_neutral(&[0.5, 1.0, 0.7]);
        assert!((wb[1] - 1.0).abs() < 1e-9, "G = 1.0");
        assert!((wb[0] - 2.0).abs() < 1e-6, "R = 2.0");
        let expected_b = 1.0 / 0.7; // ≈ 1.4286
        assert!((wb[2] - expected_b).abs() < 1e-5, "B ≈ 1.43");
    }

    // ── Matrix math tests ─────────────────────────────────────────────────

    /// Inverting the identity matrix should return the identity matrix.
    ///
    /// This is the simplest possible case for matrix inversion:
    ///   inv([[1,0,0],[0,1,0],[0,0,1]]) = [[1,0,0],[0,1,0],[0,0,1]]
    #[test]
    fn identity_matrix_inversion() {
        use crate::color::invert_3x3;
        let id = [[1.0f64, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let inv = invert_3x3(&id).unwrap();
        for (i, row) in inv.iter().enumerate() {
            for (j, &value) in row.iter().enumerate() {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!(
                    (value - expected).abs() < 1e-9,
                    "inv[{}][{}] = {} ≠ {}",
                    i,
                    j,
                    value,
                    expected
                );
            }
        }
    }

    /// A zero matrix (singular) must return None — not panic.
    ///
    /// The zero matrix has determinant 0 and no inverse. The function must
    /// detect this and return `None` rather than dividing by zero.
    #[test]
    fn singular_matrix_inversion_returns_none() {
        use crate::color::invert_3x3;
        let m = [[0.0f64, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
        assert!(invert_3x3(&m).is_none(), "Zero matrix has no inverse");
    }

    /// Multiplying a matrix by its inverse must give identity (up to float error).
    ///
    /// Uses a diagonal matrix [[2,0,0],[0,3,0],[0,0,4]] whose inverse is
    /// [[0.5,0,0],[0,0.333,0],[0,0,0.25]].
    #[test]
    fn matrix_times_its_inverse_is_identity() {
        use crate::color::{invert_3x3, matrix_multiply};
        let m = [[2.0f64, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 4.0]];
        let inv = invert_3x3(&m).unwrap();
        let product = matrix_multiply(&m, &inv);
        for (i, row) in product.iter().enumerate() {
            for (j, &value) in row.iter().enumerate() {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!(
                    (value - expected).abs() < 1e-9,
                    "M×inv(M)[{}][{}] = {} ≠ {}",
                    i,
                    j,
                    value,
                    expected
                );
            }
        }
    }

    /// ForwardMatrix=identity gives camera_to_srgb = XYZ_D50_TO_SRGB.
    ///
    /// When the forward matrix is identity (camera IS XYZ D50), the combined
    /// matrix must equal XYZ_D50_TO_SRGB exactly.
    #[test]
    fn forward_matrix_multiply_identity() {
        use crate::color::{camera_to_srgb_via_forward, XYZ_D50_TO_SRGB};
        let id = [[1.0f64, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let result = camera_to_srgb_via_forward(&id);
        // camera_to_srgb_via_forward(I) = XYZ_D50_TO_SRGB × I = XYZ_D50_TO_SRGB
        assert_eq!(result.len(), 3);
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (result[i][j] - XYZ_D50_TO_SRGB[i][j]).abs() < 1e-9,
                    "result[{}][{}] = {} ≠ {}",
                    i, j, result[i][j], XYZ_D50_TO_SRGB[i][j]
                );
            }
        }
    }

    // ── Tag byte reader tests ─────────────────────────────────────────────

    /// SRATIONAL reader: [1/2] → 0.5.
    ///
    /// A single SRATIONAL (i32/i32) pair: numerator=1, denominator=2.
    /// Expected value: 0.5.
    #[test]
    fn read_srationals_parses_correctly() {
        let bytes: Vec<u8> = vec![
            1, 0, 0, 0, // num = 1 (LE i32)
            2, 0, 0, 0, // den = 2 (LE i32)
        ];
        let vals = crate::decoder::read_srationals_bytes(&bytes);
        assert_eq!(vals.len(), 1);
        assert!((vals[0] - 0.5).abs() < 1e-9, "Expected 0.5, got {}", vals[0]);
    }

    /// SRATIONAL reader handles negative numerators.
    ///
    /// Negative values appear in colour matrices (e.g. XYZ_D50_TO_SRGB has
    /// several negative entries). Test that the signed i32 is read correctly.
    #[test]
    fn read_srationals_negative_numerator() {
        // -1 in i32 LE is 0xFF 0xFF 0xFF 0xFF; den = 4
        let bytes: Vec<u8> = vec![
            0xFF, 0xFF, 0xFF, 0xFF, // num = -1 (LE i32)
            4, 0, 0, 0,             // den = 4
        ];
        let vals = crate::decoder::read_srationals_bytes(&bytes);
        assert_eq!(vals.len(), 1);
        assert!((vals[0] - (-0.25)).abs() < 1e-9, "Expected -0.25, got {}", vals[0]);
    }

    /// RATIONAL reader: [3/4] → 0.75.
    ///
    /// A single RATIONAL (u32/u32) pair: numerator=3, denominator=4.
    /// Expected value: 0.75.
    #[test]
    fn read_rationals_parses_correctly() {
        let bytes: Vec<u8> = vec![
            3, 0, 0, 0, // num = 3 (LE u32)
            4, 0, 0, 0, // den = 4 (LE u32)
        ];
        let vals = crate::decoder::read_rationals_bytes(&bytes);
        assert_eq!(vals.len(), 1);
        assert!((vals[0] - 0.75).abs() < 1e-9, "Expected 0.75, got {}", vals[0]);
    }

    /// LONG reader: [5] → 5u32.
    ///
    /// A single LONG (u32) value: 5.
    #[test]
    fn read_longs_parses_correctly() {
        let bytes: Vec<u8> = vec![5, 0, 0, 0]; // 5 in LE u32
        let vals = crate::decoder::read_longs_bytes(&bytes);
        assert_eq!(vals, vec![5u32]);
    }

    /// LONG reader: multiple values in sequence.
    ///
    /// ActiveArea is LONG[4] = [top, left, bottom, right]. Test parsing
    /// a 4-element array.
    #[test]
    fn read_longs_multiple_values() {
        let bytes: Vec<u8> = vec![
            10, 0, 0, 0, // 10
            20, 0, 0, 0, // 20
            100, 0, 0, 0, // 100
            200, 0, 0, 0, // 200
        ];
        let vals = crate::decoder::read_longs_bytes(&bytes);
        assert_eq!(vals, vec![10u32, 20, 100, 200]);
    }

    // ── Error handling tests ──────────────────────────────────────────────

    /// Non-TIFF data must return a descriptive error, not panic.
    ///
    /// Bytes "not a tiff" are not a valid TIFF header. The decoder must
    /// return Err with a message mentioning the failure.
    #[test]
    fn error_on_bad_tiff() {
        let result = decode_dng(b"not a tiff");
        assert!(result.is_err(), "Bad input should return Err");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("DNG") || msg.contains("parse") || msg.contains("IFD"),
            "Error message should be descriptive: {}",
            msg
        );
    }

    /// Empty input must return an error, not panic.
    ///
    /// An empty byte slice has no TIFF header. The decoder must return
    /// Err rather than panicking on an out-of-bounds read.
    #[test]
    fn error_on_empty_input() {
        let result = decode_dng(&[]);
        assert!(result.is_err(), "Empty input should return Err");
    }

    // ── Tag constant tests ────────────────────────────────────────────────

    /// Verify the DNG tag constants match the DNG 1.6 specification values.
    ///
    /// These are Adobe-defined private tag IDs. Getting them wrong means
    /// we'd look for data at the wrong offset in the IFD.
    #[test]
    fn tag_constants_correct() {
        use crate::tags;
        assert_eq!(tags::DNG_VERSION, 50706);
        assert_eq!(tags::UNIQUE_CAMERA_MODEL, 50708);
        assert_eq!(tags::BLACK_LEVEL, 50714);
        assert_eq!(tags::WHITE_LEVEL, 50717);
        assert_eq!(tags::COLOR_MATRIX_1, 50721);
        assert_eq!(tags::COLOR_MATRIX_2, 50722);
        assert_eq!(tags::AS_SHOT_NEUTRAL, 50728);
        assert_eq!(tags::CALIBRATION_ILLUMINANT_1, 50778);
        assert_eq!(tags::ACTIVE_AREA, 50829);
        assert_eq!(tags::FORWARD_MATRIX_1, 50879);
        assert_eq!(tags::FORWARD_MATRIX_2, 50880);
    }
}
