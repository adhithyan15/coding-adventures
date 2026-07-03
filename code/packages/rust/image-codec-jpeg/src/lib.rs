// # image-codec-jpeg — Baseline JPEG encoder and decoder
//
// IC04: A pure-Rust implementation of the JPEG (JFIF) image format.
//
// ## What is JPEG?
//
// JPEG (Joint Photographic Experts Group) is the world's most widely used
// image format. It's a *lossy* codec — unlike PNG or BMP, it doesn't preserve
// every pixel exactly. Instead, it discards visual information that human eyes
// are least likely to notice: high-frequency detail and colour variation.
//
// A JPEG file goes through these stages when encoding:
//
//   1. **Colour transform** (RGB → YCbCr)
//      Separate luminance (brightness, Y) from chrominance (colour, Cb/Cr).
//      Eyes are more sensitive to brightness than colour.
//
//   2. **Block splitting**
//      Divide the image into 8×8 pixel blocks. JPEG operates block-by-block.
//
//   3. **DCT — Discrete Cosine Transform**
//      Transform each block from spatial domain (pixel values) to frequency
//      domain (how much of each spatial frequency is present). Think of it
//      like a Fourier transform but for a finite block.
//
//   4. **Quantization**
//      Divide each DCT coefficient by a "step size" and round. This is the
//      lossy step. High-frequency coefficients (which carry fine texture
//      detail) get divided by large numbers, becoming zero. Low-frequency
//      coefficients (which carry broad tones and edges) get smaller divisors.
//
//   5. **Entropy coding** (Huffman coding)
//      Losslessly compress the quantized coefficients using Huffman codes.
//      Zeros are encoded efficiently with run-length coding.
//
// Decoding reverses these steps: Huffman decode → dequantize → IDCT → colour
// convert.
//
// ## Crate structure
//
//   lib.rs      — JpegCodec, encode_jpeg/decode_jpeg, module declarations
//   color.rs    — RGB↔YCbCr conversions (BT.601 / JFIF standard)
//   quantize.rs — Standard Annex K tables, quality scaling, quantize/dequantize
//   entropy.rs  — Huffman tables, bit I/O, DC/AC encode/decode
//   encoder.rs  — encode_jpeg_inner: assembles complete JFIF file
//   decoder.rs  — decode_jpeg_inner: parses JFIF and reconstructs pixels
//
// ## Quality parameter
//
// The `quality` parameter (1–100) controls the trade-off between file size and
// image fidelity. Higher quality → smaller quantization step sizes → more
// coefficients survive → better image → larger file.
//
// This is the same quality scale used by libjpeg-turbo, ImageMagick, and
// virtually every other JPEG tool. A value of 75 is a good default.

/// Crate version string.
pub const VERSION: &str = "0.1.0";

mod color;
mod decoder;
mod encoder;
mod entropy;
mod quantize;

use pixel_container::{ImageCodec, PixelContainer};

// Re-export the inner functions for users who want direct access.
pub use decoder::decode_jpeg_inner;
pub use encoder::encode_jpeg_inner;

// ---------------------------------------------------------------------------
// JpegCodec — implements the ImageCodec trait
// ---------------------------------------------------------------------------

/// JPEG image encoder and decoder.
///
/// Encodes and decodes baseline JPEG (JFIF SOF0) files. Uses:
/// - YCbCr colour space (BT.601 / JFIF coefficients)
/// - 8×8 block DCT (via `dsp-dct`)
/// - Standard Annex K quantization tables with quality factor 1–100
/// - Standard Annex K Huffman tables (DC/AC, luma/chroma)
/// - 4:4:4 chroma sampling (no downsampling)
///
/// # Examples
///
/// ```
/// use pixel_container::PixelContainer;
/// use image_codec_jpeg::JpegCodec;
/// use pixel_container::ImageCodec;
///
/// let mut image = PixelContainer::new(8, 8);
/// image.set_pixel(0, 0, 255, 128, 64, 255);
///
/// let codec = JpegCodec::new(75);
/// let jpeg_bytes = codec.encode(&image);
/// assert_eq!(&jpeg_bytes[0..2], &[0xFF, 0xD8]); // SOI marker
/// ```
pub struct JpegCodec {
    /// JPEG quality, 1–100 (higher = better quality, larger files).
    pub quality: u8,
}

impl JpegCodec {
    /// Create a new JpegCodec with the given quality setting.
    ///
    /// The quality value is clamped to [1, 100] if out of range.
    pub fn new(quality: u8) -> Self {
        Self { quality: quality.clamp(1, 100) }
    }
}

impl ImageCodec for JpegCodec {
    fn mime_type(&self) -> &'static str {
        "image/jpeg"
    }

    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> {
        encode_jpeg_inner(pixels.width, pixels.height, &pixels.data, self.quality)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        let (w, h, rgba) = decode_jpeg_inner(bytes)?;
        Ok(PixelContainer::from_data(w, h, rgba))
    }
}

// ---------------------------------------------------------------------------
// Convenience functions
// ---------------------------------------------------------------------------

/// Encode a `PixelContainer` to JPEG bytes at quality 75.
///
/// Quality 75 is the standard default for photographic JPEG content — a good
/// balance between quality and file size, indistinguishable from the original
/// to most viewers at typical viewing distances.
///
/// # Examples
///
/// ```
/// use pixel_container::PixelContainer;
/// use image_codec_jpeg::encode_jpeg;
///
/// let buf = PixelContainer::new(8, 8);
/// let jpeg = encode_jpeg(&buf);
/// assert_eq!(&jpeg[0..2], &[0xFF, 0xD8]);
/// ```
pub fn encode_jpeg(pixels: &PixelContainer) -> Vec<u8> {
    JpegCodec::new(75).encode(pixels)
}

/// Decode JPEG bytes into a `PixelContainer`.
///
/// Returns `Err` with a human-readable message if the bytes are not valid
/// baseline JFIF JPEG.
///
/// # Examples
///
/// ```
/// use image_codec_jpeg::decode_jpeg;
///
/// let result = decode_jpeg(b"not a jpeg");
/// assert!(result.is_err());
/// ```
pub fn decode_jpeg(bytes: &[u8]) -> Result<PixelContainer, String> {
    JpegCodec::new(75).decode(bytes)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pixel_container::PixelContainer;

    // ── Metadata ─────────────────────────────────────────────────────────────

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn mime_type_is_jpeg() {
        assert_eq!(JpegCodec::new(75).mime_type(), "image/jpeg");
    }

    // ── Quality clamping ─────────────────────────────────────────────────────

    #[test]
    fn quality_clamps_to_valid_range() {
        let c = JpegCodec::new(0);   // below minimum
        assert_eq!(c.quality, 1, "quality 0 should clamp to 1");
        let c = JpegCodec::new(200); // above maximum
        assert_eq!(c.quality, 100, "quality 200 should clamp to 100");
        let c = JpegCodec::new(75);  // in range
        assert_eq!(c.quality, 75);
    }

    // ── SOI / EOI markers ────────────────────────────────────────────────────

    #[test]
    fn encode_produces_soi_marker() {
        let p = PixelContainer::new(8, 8);
        let jpeg = encode_jpeg(&p);
        assert_eq!(
            &jpeg[0..2], &[0xFF, 0xD8],
            "JPEG must start with SOI (FF D8)"
        );
    }

    #[test]
    fn encode_ends_with_eoi() {
        let p = PixelContainer::new(8, 8);
        let jpeg = encode_jpeg(&p);
        let n = jpeg.len();
        assert_eq!(
            &jpeg[n - 2..], &[0xFF, 0xD9],
            "JPEG must end with EOI (FF D9)"
        );
    }

    // ── Error paths ──────────────────────────────────────────────────────────

    #[test]
    fn decode_garbage_returns_err() {
        assert!(
            decode_jpeg(b"not a jpeg").is_err(),
            "garbage input should return Err"
        );
    }

    #[test]
    fn decode_empty_returns_err() {
        assert!(
            decode_jpeg(&[]).is_err(),
            "empty input should return Err"
        );
    }

    #[test]
    fn decode_truncated_soi_returns_err() {
        assert!(decode_jpeg(&[0xFF]).is_err());
    }

    // ── Basic structural checks ───────────────────────────────────────────────

    #[test]
    fn encode_non_trivial_size() {
        // A 16×16 image should produce at least a minimal JPEG structure.
        let mut p = PixelContainer::new(16, 16);
        for y in 0..16 {
            for x in 0..16 {
                p.set_pixel(x, y, (x * 10) as u8, (y * 10) as u8, 100, 255);
            }
        }
        let jpeg = JpegCodec::new(75).encode(&p);
        // Must be long enough to contain SOI + basic segments + EOI.
        assert!(jpeg.len() > 100, "encoded JPEG suspiciously short: {} bytes", jpeg.len());
        assert_eq!(&jpeg[0..2], &[0xFF, 0xD8]);
        let n = jpeg.len();
        assert_eq!(&jpeg[n - 2..], &[0xFF, 0xD9]);
    }

    // ── Round-trip tests ─────────────────────────────────────────────────────

    /// Helper: check that a round-tripped pixel is within `tol` of the original.
    fn check_pixel(original: (u8, u8, u8), decoded: (u8, u8, u8), tol: u8, label: &str) {
        let (ri, gi, bi) = original;
        let (ro, go, bo) = decoded;
        let dr = (ri as i16 - ro as i16).unsigned_abs();
        let dg = (gi as i16 - go as i16).unsigned_abs();
        let db = (bi as i16 - bo as i16).unsigned_abs();
        assert!(dr <= tol as u16, "{label} R: expected {ri}, got {ro} (diff {dr}, tol {tol})");
        assert!(dg <= tol as u16, "{label} G: expected {gi}, got {go} (diff {dg}, tol {tol})");
        assert!(db <= tol as u16, "{label} B: expected {bi}, got {bo} (diff {db}, tol {tol})");
    }

    #[test]
    fn roundtrip_solid_red() {
        // A solid 8×8 red image should decode back to roughly red within ±5.
        let mut p = PixelContainer::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                p.set_pixel(x, y, 200, 50, 50, 255);
            }
        }
        let jpeg = encode_jpeg(&p);
        let decoded = decode_jpeg(&jpeg).expect("roundtrip_solid_red: decode failed");
        assert_eq!(decoded.width, 8);
        assert_eq!(decoded.height, 8);
        for y in 0..8u32 {
            for x in 0..8u32 {
                let (r, g, b, a) = decoded.pixel_at(x, y);
                assert_eq!(a, 255, "alpha must be 255");
                check_pixel((200, 50, 50), (r, g, b), 5, &format!("solid red ({x},{y})"));
            }
        }
    }

    #[test]
    fn roundtrip_solid_green() {
        let mut p = PixelContainer::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                p.set_pixel(x, y, 50, 180, 50, 255);
            }
        }
        let jpeg = encode_jpeg(&p);
        let decoded = decode_jpeg(&jpeg).expect("roundtrip_solid_green: decode failed");
        for y in 0..8u32 {
            for x in 0..8u32 {
                let (r, g, b, _) = decoded.pixel_at(x, y);
                check_pixel((50, 180, 50), (r, g, b), 5, &format!("solid green ({x},{y})"));
            }
        }
    }

    #[test]
    fn roundtrip_solid_blue() {
        let mut p = PixelContainer::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                p.set_pixel(x, y, 50, 50, 200, 255);
            }
        }
        let jpeg = encode_jpeg(&p);
        let decoded = decode_jpeg(&jpeg).expect("roundtrip_solid_blue: decode failed");
        for y in 0..8u32 {
            for x in 0..8u32 {
                let (r, g, b, _) = decoded.pixel_at(x, y);
                check_pixel((50, 50, 200), (r, g, b), 5, &format!("solid blue ({x},{y})"));
            }
        }
    }

    #[test]
    fn roundtrip_quality_75() {
        // Generic solid colour round-trip at the default quality.
        let mut p = PixelContainer::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                p.set_pixel(x, y, 200, 50, 50, 255);
            }
        }
        let jpeg = JpegCodec::new(75).encode(&p);
        let decoded = JpegCodec::new(75).decode(&jpeg).expect("decode failed");
        for y in 0..8u32 {
            for x in 0..8u32 {
                let (r, g, b, _) = decoded.pixel_at(x, y);
                check_pixel((200, 50, 50), (r, g, b), 5, "quality 75");
            }
        }
    }

    #[test]
    fn roundtrip_quality_100() {
        // At quality 100, all quantization step sizes are 1, so the only loss
        // comes from the f32 DCT and the colour space round-trip. Tolerance ±3.
        let mut p = PixelContainer::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                p.set_pixel(x, y, 128, 200, 64, 255);
            }
        }
        let codec = JpegCodec::new(100);
        let jpeg = codec.encode(&p);
        let decoded = codec.decode(&jpeg).expect("roundtrip_quality_100: decode failed");
        for y in 0..8u32 {
            for x in 0..8u32 {
                let (r, g, b, _) = decoded.pixel_at(x, y);
                check_pixel((128, 200, 64), (r, g, b), 3, "quality 100");
            }
        }
    }

    #[test]
    fn roundtrip_white() {
        // White (255, 255, 255) is a degenerate case: Y=255, Cb=Cr=128.
        let mut p = PixelContainer::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                p.set_pixel(x, y, 255, 255, 255, 255);
            }
        }
        let jpeg = encode_jpeg(&p);
        let decoded = decode_jpeg(&jpeg).expect("white roundtrip failed");
        for y in 0..8u32 {
            for x in 0..8u32 {
                let (r, g, b, _) = decoded.pixel_at(x, y);
                check_pixel((255, 255, 255), (r, g, b), 5, "white");
            }
        }
    }

    #[test]
    fn roundtrip_black() {
        // Black (0, 0, 0): Y=0, Cb=Cr=128.
        let mut p = PixelContainer::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                p.set_pixel(x, y, 0, 0, 0, 255);
            }
        }
        let jpeg = encode_jpeg(&p);
        let decoded = decode_jpeg(&jpeg).expect("black roundtrip failed");
        for y in 0..8u32 {
            for x in 0..8u32 {
                let (r, g, b, _) = decoded.pixel_at(x, y);
                check_pixel((0, 0, 0), (r, g, b), 5, "black");
            }
        }
    }

    #[test]
    fn roundtrip_grey() {
        // Mid-grey (128, 128, 128): pure Y with Cb=Cr=128.
        let mut p = PixelContainer::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                p.set_pixel(x, y, 128, 128, 128, 255);
            }
        }
        let jpeg = encode_jpeg(&p);
        let decoded = decode_jpeg(&jpeg).expect("grey roundtrip failed");
        for y in 0..8u32 {
            for x in 0..8u32 {
                let (r, g, b, _) = decoded.pixel_at(x, y);
                check_pixel((128, 128, 128), (r, g, b), 5, "grey");
            }
        }
    }

    #[test]
    fn encode_16x16() {
        // Non-trivial 2-block-wide image; tests multi-block encoding.
        let mut p = PixelContainer::new(16, 16);
        for y in 0..16 {
            for x in 0..16 {
                p.set_pixel(x, y, (x * 10) as u8, (y * 10) as u8, 100, 255);
            }
        }
        let jpeg = JpegCodec::new(75).encode(&p);
        assert!(jpeg.len() > 100, "16x16 JPEG too short");
        assert_eq!(&jpeg[0..2], &[0xFF, 0xD8]);
        let n = jpeg.len();
        assert_eq!(&jpeg[n - 2..], &[0xFF, 0xD9]);
    }

    #[test]
    fn roundtrip_16x16() {
        // A solid 16×16 image — two blocks wide and two blocks tall.
        // Using a solid colour ensures every pixel in the block has the same value,
        // so each 8×8 block is a constant block (DC-only, all AC = 0). The round-trip
        // should recover the original colour within the ±5 tolerance of quality 80.
        let mut p = PixelContainer::new(16, 16);
        for y in 0..16 {
            for x in 0..16 {
                p.set_pixel(x, y, 180, 80, 60, 255);
            }
        }
        let codec = JpegCodec::new(80);
        let jpeg = codec.encode(&p);
        let decoded = codec.decode(&jpeg).expect("16x16 roundtrip decode failed");
        assert_eq!(decoded.width, 16);
        assert_eq!(decoded.height, 16);
        // All pixels should round-trip within ±5.
        for y in 0..16u32 {
            for x in 0..16u32 {
                let (r, g, b, _) = decoded.pixel_at(x, y);
                check_pixel((180, 80, 60), (r, g, b), 5, &format!("16x16 ({x},{y})"));
            }
        }
    }

    #[test]
    fn roundtrip_non_multiple_of_8() {
        // 10×10 image (padded to 16×16 internally, output cropped to 10×10).
        let mut p = PixelContainer::new(10, 10);
        for y in 0..10 {
            for x in 0..10 {
                p.set_pixel(x, y, 100, 150, 200, 255);
            }
        }
        let jpeg = encode_jpeg(&p);
        let decoded = decode_jpeg(&jpeg).expect("10x10 roundtrip decode failed");
        // Dimensions must match the original.
        assert_eq!(decoded.width, 10, "width must be 10 after round-trip");
        assert_eq!(decoded.height, 10, "height must be 10 after round-trip");
        // Interior pixels should be close.
        for y in 0..10u32 {
            for x in 0..10u32 {
                let (r, g, b, _) = decoded.pixel_at(x, y);
                check_pixel((100, 150, 200), (r, g, b), 8, &format!("10x10 ({x},{y})"));
            }
        }
    }

    #[test]
    fn roundtrip_1x1() {
        // Edge case: a single pixel (padded to 8×8 internally).
        let mut p = PixelContainer::new(1, 1);
        p.set_pixel(0, 0, 180, 90, 45, 255);
        let jpeg = encode_jpeg(&p);
        let decoded = decode_jpeg(&jpeg).expect("1x1 roundtrip decode failed");
        assert_eq!(decoded.width, 1);
        assert_eq!(decoded.height, 1);
        let (r, g, b, _) = decoded.pixel_at(0, 0);
        check_pixel((180, 90, 45), (r, g, b), 8, "1x1 pixel");
    }

    // ── Quality range tests ───────────────────────────────────────────────────

    #[test]
    fn quality_1_encodes_without_panic() {
        // Quality 1 produces the coarsest quantization — just ensure no panic.
        let mut p = PixelContainer::new(8, 8);
        for y in 0..8 { for x in 0..8 { p.set_pixel(x, y, 128, 128, 128, 255); } }
        let jpeg = JpegCodec::new(1).encode(&p);
        assert_eq!(&jpeg[0..2], &[0xFF, 0xD8]);
        // Quality 1 output should still decode without error.
        let decoded = JpegCodec::new(1).decode(&jpeg).expect("quality-1 decode failed");
        assert_eq!(decoded.width, 8);
    }

    #[test]
    fn quality_50_encodes_without_panic() {
        let p = PixelContainer::new(8, 8);
        let jpeg = JpegCodec::new(50).encode(&p);
        assert_eq!(&jpeg[0..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn higher_quality_produces_larger_file() {
        // At higher quality, the quantization step sizes are smaller, so more
        // AC coefficients survive (aren't rounded to zero). The file should be
        // larger or at least no smaller.
        let mut p = PixelContainer::new(8, 8);
        // Use a non-trivial (non-constant) image so quality actually matters.
        for y in 0..8u32 {
            for x in 0..8u32 {
                p.set_pixel(x, y, (x * 30) as u8, (y * 25) as u8, 100, 255);
            }
        }
        let jpeg_low  = JpegCodec::new(10).encode(&p);
        let jpeg_high = JpegCodec::new(90).encode(&p);
        assert!(
            jpeg_high.len() >= jpeg_low.len(),
            "quality-90 ({} bytes) should be >= quality-10 ({} bytes)",
            jpeg_high.len(), jpeg_low.len()
        );
    }

    // ── Alpha handling ────────────────────────────────────────────────────────

    #[test]
    fn decoded_alpha_is_always_255() {
        // JPEG has no transparency. Every decoded pixel should have alpha = 255.
        let mut p = PixelContainer::new(8, 8);
        for y in 0..8 { for x in 0..8 { p.set_pixel(x, y, 100, 100, 100, 128); } }
        let jpeg = encode_jpeg(&p);
        let decoded = decode_jpeg(&jpeg).expect("alpha test decode failed");
        for y in 0..8u32 {
            for x in 0..8u32 {
                let (_, _, _, a) = decoded.pixel_at(x, y);
                assert_eq!(a, 255, "decoded alpha at ({x},{y}) = {a}, expected 255");
            }
        }
    }
}
