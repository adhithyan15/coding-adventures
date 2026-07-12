// # image-codec-cr2
//
// Canon CR2 RAW image codec for Rust.
//
// ## What is CR2?
//
// CR2 (Canon RAW 2) is Canon's proprietary camera RAW format, introduced
// in 2004 with the EOS 20D and used in Canon DSLRs through approximately
// 2018. It was replaced by CR3 (ISO BMFF-based) in the mirrorless EOS R line.
//
// Every CR2 file is structurally a TIFF 6.0 file — it starts with the
// standard TIFF 8-byte header — with two Canon-specific extensions:
//
//   1. A 4-byte signature "CR\x02\x00" at bytes 8–11 (immediately after
//      the TIFF header's IFD0 offset field).
//   2. The full-resolution sensor data lives in **IFD3** (the 4th IFD),
//      rather than IFD0 which holds a JPEG thumbnail.
//
// ## Format at a Glance
//
// ```text
// ┌──────────────────────────────────────────────┐
// │  TIFF header (8 bytes)                        │
// │    "II" (LE) + 42 (magic) + IFD0 offset       │
// ├──────────────────────────────────────────────┤
// │  CR2 signature (4 bytes at offset 8)          │
// │    'C' 'R' 0x02 0x00                          │
// ├──────────────────────────────────────────────┤
// │  IFD0  — JPEG thumbnail + camera metadata    │
// │  IFD1  — Reduced-size image (optional)       │
// │  IFD2  — Reduced-size RAW (optional)         │
// │  IFD3  — Full-resolution CFA sensor data     │
// │    Compression = 6 or 7 (lossless JPEG)      │
// │    PhotometricInterpretation = 32803 (CFA)   │
// │    BitsPerSample = 14 (14-bit raw ADC)        │
// └──────────────────────────────────────────────┘
// ```
//
// ## Decode Pipeline (v0.1)
//
// ```text
// CR2 bytes
//   ↓ validate "II" + magic 42 + "CR\x02" signature
//   ↓ parse TIFF IFD chain → select IFD3
//   ↓ delegate to image-codec-tiff::decode_tiff_with_opts
//       with Canon black level (2047), white level (15383),
//       and hardcoded EOS-5D-era camera-to-sRGB colour matrix
//   ↓ PixelContainer (RGBA8, A=255)
// ```
//
// ## Modules
//
// - `decoder` — CR2 signature validation + TIFF delegation
// - `encoder` — minimal synthetic CR2 writer for round-trip tests
// - `lossless_jpeg` — SOF3 lossless JPEG decoder (exposed as `pub` for testing)
//
// ## Crate Dependencies
//
// - `pixel-container` — the `PixelContainer` RGBA8 buffer type
// - `paint-instructions` — the `ImageCodec` trait
// - `image-codec-tiff` — full TIFF decode/encode engine (strip, demosaic, colour)
//
// ## Limitations (v0.1)
//
// - Uses a single hardcoded colour matrix for all Canon DSLR models.
// - Canon MakerNote white balance and per-model colour data are not parsed.
// - Bayer pattern is determined by image-codec-tiff (defaults to RGGB).
// - lossless_jpeg::decode_sof3 is a v0.1 best-effort decoder; it handles the
//   common 2-component predictor-1 case. Complex Huffman tables or unusual
//   restart intervals may not decode correctly.

pub const VERSION: &str = "0.1.0";

// ─── Colour constants ──────────────────────────────────────────────────────────

/// Generic Canon DSLR (EOS 5D-era) approximate camera-to-sRGB colour matrix.
///
/// This 3×3 matrix converts from Canon camera-native linear RGB to linear sRGB
/// primaries (D65 adapted). Derived from dcraw / LibRaw camera profiles.
///
/// Rows: output sRGB channels [R, G, B].
/// Columns: input camera channels [R_cam, G_cam, B_cam].
///
/// ```text
/// sR = 1.901824·cR - 0.972035·cG + 0.070223·cB
/// sG = -0.229410·cR + 1.659384·cG - 0.429974·cB
/// sB = 0.042003·cR - 0.519400·cG + 1.477397·cB
/// ```
pub const CANON_COLOR_MATRIX: [[f64; 3]; 3] = [
    [1.901_824, -0.972_035, 0.070_223],
    [-0.229_410, 1.659_384, -0.429_974],
    [0.042_003, -0.519_400, 1.477_397],
];

/// Default black level for Canon CR2 (14-bit sensors).
///
/// Most Canon DSLRs report a black level (pedestal) around 2047–2048 counts
/// on a 14-bit (0–16383) scale. Pixels below this level represent the sensor's
/// noise floor, not real signal.
pub const CANON_BLACK_LEVEL: u32 = 2047;

/// Default white level (saturation point) for 14-bit Canon CR2.
///
/// The typical full-well capacity of EOS-generation Canon sensors saturates
/// around 15383 counts (14-bit). Values at or above this are clipped to 1.0.
pub const CANON_WHITE_LEVEL: u32 = 15383;

// ─── Modules ──────────────────────────────────────────────────────────────────

mod decoder;
mod encoder;
pub mod lossless_jpeg;

// ─── Public API ───────────────────────────────────────────────────────────────

/// Decode a Canon CR2 file from a byte slice into a [`PixelContainer`].
///
/// The returned container is RGBA8 with A=255 throughout.
///
/// # Errors
///
/// Returns `Err` with a descriptive message if:
/// - The file is shorter than 16 bytes.
/// - The file doesn't have a valid TIFF little-endian header ("II" + magic 42).
/// - The CR2 signature ("CR\x02") is absent at bytes 8–10.
/// - Any TIFF decode failure (corrupt IFD, unsupported compression, etc.).
///
/// # Example
///
/// ```rust,ignore
/// let bytes = std::fs::read("photo.CR2").unwrap();
/// match image_codec_cr2::decode_cr2(&bytes) {
///     Ok(pixels) => println!("{}×{} image", pixels.width, pixels.height),
///     Err(e) => eprintln!("Error: {e}"),
/// }
/// ```
pub fn decode_cr2(bytes: &[u8]) -> Result<pixel_container::PixelContainer, String> {
    decoder::decode_cr2(bytes)
}

/// Encode a [`PixelContainer`] into a minimal synthetic CR2 file.
///
/// This is a test-only encoder. It produces a standard uncompressed TIFF with
/// the 4-byte CR2 signature patched into bytes 8–11. The output is structurally
/// valid and can be decoded by `decode_cr2`, but is NOT a production CR2 file.
///
/// # Example
///
/// ```rust,ignore
/// let mut px = PixelContainer::new(4, 4);
/// px.fill(128, 64, 32, 255);
/// let cr2_bytes = image_codec_cr2::encode_cr2(&px);
/// let decoded = image_codec_cr2::decode_cr2(&cr2_bytes).unwrap();
/// assert_eq!(decoded.width, 4);
/// ```
pub fn encode_cr2(pixels: &pixel_container::PixelContainer) -> Vec<u8> {
    encoder::encode_cr2(pixels)
}

// ─── Cr2Codec — ImageCodec trait implementation ───────────────────────────────

/// A Canon CR2 image codec implementing the `ImageCodec` trait.
///
/// Plug this into any pipeline that accepts `ImageCodec` objects:
///
/// ```rust,ignore
/// use image_codec_cr2::Cr2Codec;
/// use paint_instructions::ImageCodec;
///
/// let codec: &dyn ImageCodec = &Cr2Codec;
/// let pixels = codec.decode(&cr2_bytes)?;
/// let re_encoded = codec.encode(&pixels);
/// ```
pub struct Cr2Codec;

impl paint_instructions::ImageCodec for Cr2Codec {
    fn mime_type(&self) -> &'static str {
        "image/x-canon-cr2"
    }

    fn encode(&self, pixels: &pixel_container::PixelContainer) -> Vec<u8> {
        encode_cr2(pixels)
    }

    fn decode(&self, bytes: &[u8]) -> Result<pixel_container::PixelContainer, String> {
        decode_cr2(bytes)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::ImageCodec as _;
    use pixel_container::PixelContainer;

    // ── Helper ────────────────────────────────────────────────────────────

    /// Build a solid-colour PixelContainer, encode to CR2, return the bytes.
    fn make_cr2(w: u32, h: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
        let mut px = PixelContainer::new(w, h);
        px.fill(r, g, b, 255);
        encode_cr2(&px)
    }

    // ── Basic trait and constant tests ─────────────────────────────────────

    /// Verify the VERSION constant matches Cargo.toml.
    #[test]
    fn version() {
        assert_eq!(VERSION, "0.1.0");
    }

    /// Verify the MIME type registered by Cr2Codec.
    #[test]
    fn mime_type() {
        assert_eq!(Cr2Codec.mime_type(), "image/x-canon-cr2");
    }

    /// Verify the colour matrix dimensions.
    #[test]
    fn color_matrix_has_correct_shape() {
        assert_eq!(CANON_COLOR_MATRIX.len(), 3);
        assert_eq!(CANON_COLOR_MATRIX[0].len(), 3);
        assert_eq!(CANON_COLOR_MATRIX[1].len(), 3);
        assert_eq!(CANON_COLOR_MATRIX[2].len(), 3);
    }

    // ── Encoder: CR2 signature ─────────────────────────────────────────────

    /// The encoded bytes must have the CR2 signature at bytes 8–10.
    #[test]
    fn cr2_signature_present() {
        let bytes = make_cr2(2, 2, 100, 100, 100);
        assert_eq!(
            &bytes[8..10],
            b"CR",
            "Expected CR at bytes 8..10, got {:?}",
            &bytes[8..10]
        );
        assert_eq!(
            bytes[10], 2,
            "Expected CR2 version byte 2 at offset 10, got {}",
            bytes[10]
        );
    }

    /// The encoded bytes must have the TIFF LE marker at bytes 0–1.
    #[test]
    fn tiff_le_marker_present() {
        let bytes = make_cr2(2, 2, 50, 50, 50);
        assert_eq!(&bytes[0..2], b"II", "TIFF byte order must be LE ('II')");
        let magic = u16::from_le_bytes([bytes[2], bytes[3]]);
        assert_eq!(magic, 42, "TIFF magic must be 42");
    }

    // ── Round-trip tests ───────────────────────────────────────────────────

    /// A 2×2 image should survive encode → decode with the right dimensions.
    #[test]
    fn round_trip_2x2() {
        let mut px = PixelContainer::new(2, 2);
        px.fill(150, 100, 80, 255);
        let bytes = encode_cr2(&px);
        let decoded = decode_cr2(&bytes).unwrap();
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 2);
    }

    /// Same round-trip through the Cr2Codec trait methods.
    #[test]
    fn round_trip_via_codec() {
        let mut px = PixelContainer::new(4, 4);
        px.fill(200, 100, 50, 255);
        let bytes = Cr2Codec.encode(&px);
        let decoded = Cr2Codec.decode(&bytes).unwrap();
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
    }

    /// A 4×4 gradient image should decode to the right dimensions.
    #[test]
    fn decode_4x4_image() {
        let mut px = PixelContainer::new(4, 4);
        for y in 0..4u32 {
            for x in 0..4u32 {
                px.set_pixel(x, y, (x * 60) as u8, (y * 60) as u8, 100, 255);
            }
        }
        let bytes = encode_cr2(&px);
        let decoded = decode_cr2(&bytes).unwrap();
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
    }

    /// A 1×1 image round-trip.
    #[test]
    fn round_trip_1x1() {
        let mut px = PixelContainer::new(1, 1);
        px.set_pixel(0, 0, 255, 128, 64, 255);
        let bytes = encode_cr2(&px);
        let decoded = decode_cr2(&bytes).unwrap();
        assert_eq!(decoded.width, 1);
        assert_eq!(decoded.height, 1);
    }

    // ── Error cases ────────────────────────────────────────────────────────

    /// Empty slice must return Err.
    #[test]
    fn error_on_empty() {
        let result = decode_cr2(&[]);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("CR2"), "Error should be prefixed with CR2: {}", msg);
    }

    /// A 4-byte file is too short.
    #[test]
    fn error_on_short_file() {
        let result = decode_cr2(&[0x49, 0x49, 0x2A, 0x00]);
        assert!(result.is_err());
    }

    /// A valid TIFF header with no CR2 signature should fail.
    #[test]
    fn error_on_missing_cr2_sig() {
        let mut bytes = make_cr2(2, 2, 100, 100, 100);
        bytes[8] = 0x00; // corrupt "C" of CR2 signature
        let result = decode_cr2(&bytes);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("CR2"), "{}", msg);
    }

    /// Corrupting the TIFF magic should fail.
    #[test]
    fn error_on_bad_magic() {
        let mut bytes = make_cr2(2, 2, 100, 100, 100);
        bytes[2] = 0xFF; // corrupt TIFF magic
        let result = decode_cr2(&bytes);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("CR2") || msg.contains("magic"), "{}", msg);
    }

    /// Big-endian ("MM") marker should fail (CR2 is always LE).
    #[test]
    fn error_on_big_endian_marker() {
        let mut bytes = make_cr2(2, 2, 100, 100, 100);
        bytes[0] = b'M';
        bytes[1] = b'M';
        let result = decode_cr2(&bytes);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("CR2"), "{}", msg);
    }

    // ── lossless_jpeg module tests ─────────────────────────────────────────

    /// decode_sof3 on empty data returns Err.
    #[test]
    fn lossless_jpeg_empty_returns_err() {
        assert!(lossless_jpeg::decode_sof3(&[]).is_err());
    }

    /// decode_sof3 on data without SOI marker returns Err.
    #[test]
    fn lossless_jpeg_bad_soi_returns_err() {
        assert!(lossless_jpeg::decode_sof3(&[0x00, 0x00, 0x00]).is_err());
    }

    // ── Colour constants sanity checks ─────────────────────────────────────

    /// Black level should be less than white level.
    #[test]
    // Intentionally asserts a relationship between compile-time constants with a
    // descriptive runtime message; a `const` block assert cannot format the note.
    #[allow(clippy::assertions_on_constants)]
    fn black_level_less_than_white_level() {
        assert!(
            CANON_BLACK_LEVEL < CANON_WHITE_LEVEL,
            "Black level {} must be < white level {}",
            CANON_BLACK_LEVEL, CANON_WHITE_LEVEL
        );
    }

    /// White level should be representable in 14 bits (< 16384).
    #[test]
    // Intentionally asserts a property of a compile-time constant with a
    // descriptive runtime message; a `const` block assert cannot format the note.
    #[allow(clippy::assertions_on_constants)]
    fn white_level_within_14bit_range() {
        assert!(
            CANON_WHITE_LEVEL < (1 << 14),
            "White level {} should fit in 14 bits",
            CANON_WHITE_LEVEL
        );
    }

    /// Colour matrix diagonal should be positive (well-formed matrix heuristic).
    #[test]
    fn color_matrix_diagonal_positive() {
        for (i, row) in CANON_COLOR_MATRIX.iter().enumerate() {
            assert!(
                row[i] > 0.0,
                "Diagonal element [{i}][{i}] = {} should be positive",
                row[i]
            );
        }
    }
}
