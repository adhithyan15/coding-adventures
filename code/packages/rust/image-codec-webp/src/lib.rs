//! # image-codec-webp
//!
//! WebP image codec for the paint-instructions pixel pipeline.
//! Implements VP8L (lossless) and VP8 lossy encoding and decoding.
//!
//! ## Architecture
//!
//! WebP files use a RIFF container:
//!
//! ```text
//! RIFF <file_size> WEBP
//!   VP8L <chunk_size> <vp8l-bitstream>
//! ```
//!
//! The VP8L bitstream stores pixels using:
//!
//! 1. Optional transforms (subtract_green, color, predictor, color_index).
//!    This release always writes no transforms.
//! 2. LZ77 backward references with 2D distance mapping.
//!    This release uses literal-only mode (no back-references).
//! 3. Canonical Huffman prefix codes in 5 groups (G, R, B, A, Dist).
//!
//! ## Usage
//!
//! ```rust,ignore
//! use image_codec_webp::{encode_webp_lossless, decode_webp, WebPCodec};
//! use paint_instructions::ImageCodec;
//!
//! // Functional API:
//! let encoded = encode_webp_lossless(&pixels);
//! let decoded = decode_webp(&encoded).unwrap();
//!
//! // Trait API:
//! let codec = WebPCodec::new(90, true);
//! let bytes = codec.encode(&pixels);
//! let pixels2 = codec.decode(&bytes).unwrap();
//! ```
//!
//! ## VP8 lossy
//!
//! Lossy WebP (VP8) requires the `range-coder` crate (arithmetic coding) which
//! is being implemented in a parallel PR.  Calling `encode_webp` or constructing
//! `WebPCodec::new(q, false)` and calling `encode` will panic with a clear message.
//!
//! ## References
//!
//! - WebP lossless bitstream spec: https://developers.google.com/speed/webp/docs/webp_lossless_bitstream_specification
//! - WebP container spec: https://developers.google.com/speed/webp/docs/riff_container
//! - VP8 lossy spec: https://www.rfc-editor.org/rfc/rfc6386

pub const VERSION: &str = "0.3.3";

mod riff;
pub mod vp8;
pub mod vp8l;

use paint_instructions::{ImageCodec, PixelContainer};

// ---------------------------------------------------------------------------
// WebPCodec — implements the ImageCodec trait
// ---------------------------------------------------------------------------

/// A WebP image codec that implements [`ImageCodec`].
///
/// Supports lossless encoding (VP8L) and returns a descriptive error for
/// VP8 lossy encoding (requires the `range-coder` crate, coming in a future PR).
///
/// ## Example
///
/// ```rust,ignore
/// use image_codec_webp::WebPCodec;
/// use paint_instructions::ImageCodec;
///
/// let codec = WebPCodec::new(90, true); // 90% quality, lossless
/// let bytes = codec.encode(&pixels);
/// let decoded = codec.decode(&bytes).unwrap();
/// ```
pub struct WebPCodec {
    /// Quality hint for lossy encoding (0–100).  Ignored in lossless mode.
    pub quality: u8,
    /// If `true`, use VP8L lossless encoding.  If `false`, use VP8 lossy
    /// (currently unimplemented — panics).
    pub lossless: bool,
}

impl WebPCodec {
    /// Create a new `WebPCodec`.
    ///
    /// `quality` is a hint for the VP8 lossy encoder (0=worst, 100=best).
    /// In lossless mode (`lossless=true`) quality is ignored.
    pub fn new(quality: u8, lossless: bool) -> Self {
        Self { quality, lossless }
    }
}

impl ImageCodec for WebPCodec {
    /// Returns `"image/webp"`.
    fn mime_type(&self) -> &'static str {
        "image/webp"
    }

    /// Encode a pixel buffer as a WebP file.
    ///
    /// In lossless mode (`self.lossless = true`) this calls `encode_webp_lossless`.
    ///
    /// # Panics
    ///
    /// Panics if `self.lossless = false` (VP8 lossy not yet implemented).
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> {
        if self.lossless {
            encode_webp_lossless(pixels)
        } else {
            encode_webp(pixels, self.quality)
        }
    }

    /// Decode a WebP file into a pixel buffer.
    ///
    /// Supports VP8L (lossless) chunk type.
    /// Returns `Err` for VP8 lossy, VP8X extended, or unknown chunk types.
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_webp(bytes)
    }
}

// ---------------------------------------------------------------------------
// Functional API
// ---------------------------------------------------------------------------

/// Encode a `PixelContainer` as a lossless WebP file (VP8L).
///
/// Returns a complete, self-contained WebP file (RIFF container + VP8L chunk).
/// Ready to write to disk or send over the network.
///
/// This uses literal-only VP8L encoding without transforms or LZ77
/// back-references.  Compression is valid but not optimal compared to a
/// full encoder with all transforms enabled.
pub fn encode_webp_lossless(pixels: &PixelContainer) -> Vec<u8> {
    let bitstream = vp8l::encode(pixels);
    riff::build_riff(b"VP8L", &bitstream)
}

/// Encode a `PixelContainer` as a lossy WebP file (VP8).
///
/// `quality` is in [0, 100]; higher = better quality / larger file.
/// Returns a complete RIFF/WEBP/VP8 container.
pub fn encode_webp(pixels: &PixelContainer, quality: u8) -> Vec<u8> {
    let vp8_data = vp8::encode(pixels, quality);
    riff::build_riff(b"VP8 ", &vp8_data)
}

/// Decode a WebP file (RIFF container) into a `PixelContainer`.
///
/// Supports the VP8L (lossless) chunk type.
/// Returns a descriptive error for:
/// - Files that are too short to parse.
/// - Files missing the RIFF magic or WEBP fourCC.
/// - VP8 lossy (`VP8 ` chunk) — not yet implemented.
/// - VP8X extended (`VP8X` chunk) — not yet implemented.
/// - Unknown chunk types.
pub fn decode_webp(bytes: &[u8]) -> Result<PixelContainer, String> {
    if bytes.len() < 12 {
        return Err("WebP: file too short (need at least 12 bytes for RIFF header)".to_string());
    }
    if &bytes[0..4] != b"RIFF" {
        return Err("WebP: missing RIFF magic bytes".to_string());
    }
    if bytes.len() < 20 {
        return Err("WebP: file too short to contain a WEBP chunk header".to_string());
    }
    if &bytes[8..12] != b"WEBP" {
        return Err("WebP: missing WEBP fourCC (bytes 8-11)".to_string());
    }

    // Parse the first chunk (immediately after the RIFF/WEBP header).
    let chunk_type = &bytes[12..16];
    let chunk_size = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;

    let chunk_data_start = 20usize;
    if bytes.len() < chunk_data_start + chunk_size {
        return Err(format!(
            "WebP: chunk truncated (need {} bytes after offset 20, have {})",
            chunk_size,
            bytes.len() - chunk_data_start
        ));
    }
    let chunk_data = &bytes[chunk_data_start..chunk_data_start + chunk_size];

    match chunk_type {
        b"VP8L" => vp8l::decode(chunk_data),
        b"VP8 " => vp8::decode(chunk_data),
        b"VP8X" => Err(
            "WebP: VP8X extended format not yet implemented".to_string()
        ),
        _ => Err(format!(
            "WebP: unknown chunk type {:?} (expected VP8L, VP8 , or VP8X)",
            std::str::from_utf8(chunk_type).unwrap_or("<non-UTF8>")
        )),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::PixelContainer;

    // ── Version ───────────────────────────────────────────────────────────────

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.3.3");
    }

    // ── WebPCodec ─────────────────────────────────────────────────────────────

    #[test]
    fn mime_type_is_webp() {
        assert_eq!(WebPCodec::new(75, true).mime_type(), "image/webp");
    }

    #[test]
    fn codec_encode_decode_roundtrip() {
        let mut pixels = PixelContainer::new(4, 4);
        pixels.fill(128, 64, 32, 200);
        let codec = WebPCodec::new(90, true);
        let bytes = codec.encode(&pixels);
        let decoded = codec.decode(&bytes).unwrap();
        assert_eq!(decoded.data, pixels.data);
    }

    // ── RIFF magic bytes ──────────────────────────────────────────────────────

    #[test]
    fn riff_magic_bytes() {
        let pixels = PixelContainer::new(4, 4);
        let bytes = encode_webp_lossless(&pixels);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WEBP");
    }

    // ── VP8L signature byte ───────────────────────────────────────────────────

    #[test]
    fn vp8l_signature_byte() {
        let pixels = PixelContainer::new(4, 4);
        let bytes = encode_webp_lossless(&pixels);
        // VP8L chunk: bytes[12..16] = b"VP8L", bytes[16..20] = chunk size, bytes[20] = 0x2F
        assert_eq!(&bytes[12..16], b"VP8L");
        assert_eq!(bytes[20], 0x2F);
    }

    // ── Round-trip tests ──────────────────────────────────────────────────────

    #[test]
    fn round_trip_solid_color() {
        let mut pixels = PixelContainer::new(4, 4);
        for y in 0..4u32 {
            for x in 0..4u32 {
                pixels.set_pixel(x, y, 200, 100, 50, 255);
            }
        }
        let encoded = encode_webp_lossless(&pixels);
        let decoded = decode_webp(&encoded).unwrap();
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
        assert_eq!(decoded.data, pixels.data);
    }

    #[test]
    fn round_trip_gradient() {
        let mut pixels = PixelContainer::new(8, 8);
        for y in 0..8u32 {
            for x in 0..8u32 {
                pixels.set_pixel(x, y, (x * 30) as u8, (y * 30) as u8, 128, 255);
            }
        }
        let encoded = encode_webp_lossless(&pixels);
        let decoded = decode_webp(&encoded).unwrap();
        assert_eq!(decoded.width, 8);
        assert_eq!(decoded.height, 8);
        assert_eq!(decoded.data, pixels.data);
    }

    #[test]
    fn round_trip_1x1() {
        let mut pixels = PixelContainer::new(1, 1);
        pixels.set_pixel(0, 0, 255, 128, 64, 200);
        let encoded = encode_webp_lossless(&pixels);
        let decoded = decode_webp(&encoded).unwrap();
        assert_eq!(decoded.pixel_at(0, 0), (255, 128, 64, 200));
    }

    #[test]
    fn round_trip_transparent() {
        let pixels = PixelContainer::new(4, 4); // all zeros (transparent black)
        let encoded = encode_webp_lossless(&pixels);
        let decoded = decode_webp(&encoded).unwrap();
        assert_eq!(decoded.data, pixels.data);
    }

    #[test]
    fn round_trip_all_channels_varied() {
        let mut pixels = PixelContainer::new(4, 1);
        pixels.set_pixel(0, 0, 10, 20, 30, 0);
        pixels.set_pixel(1, 0, 10, 20, 30, 85);
        pixels.set_pixel(2, 0, 10, 20, 30, 170);
        pixels.set_pixel(3, 0, 10, 20, 30, 255);
        let encoded = encode_webp_lossless(&pixels);
        let decoded = decode_webp(&encoded).unwrap();
        assert_eq!(decoded.data, pixels.data);
    }

    // ── Decode error cases ────────────────────────────────────────────────────

    #[test]
    fn decode_error_bad_magic() {
        let result = decode_webp(b"this is not a webp file at all!!");
        assert!(result.is_err());
    }

    #[test]
    fn decode_error_too_short() {
        let result = decode_webp(&[0u8; 8]);
        assert!(result.is_err());
    }

    // ── VP8 lossy ─────────────────────────────────────────────────────────────

    #[test]
    fn encode_webp_produces_riff_header() {
        let pixels = PixelContainer::new(4, 4);
        let bytes = encode_webp(&pixels, 75);
        assert_eq!(&bytes[0..4], b"RIFF", "must start with RIFF");
        assert_eq!(&bytes[8..12], b"WEBP", "must have WEBP fourCC");
    }

    #[test]
    fn encode_webp_produces_vp8_chunk() {
        let pixels = PixelContainer::new(4, 4);
        let bytes = encode_webp(&pixels, 75);
        assert_eq!(&bytes[12..16], b"VP8 ", "chunk type must be VP8 ");
    }

    #[test]
    fn round_trip_lossy_solid() {
        // Solid-colour image — DC prediction + skip residuals should be ±5
        let mut pixels = PixelContainer::new(16, 16);
        for y in 0..16u32 {
            for x in 0..16u32 {
                pixels.set_pixel(x, y, 180, 180, 180, 255);
            }
        }
        let bytes = encode_webp(&pixels, 75);
        let decoded = decode_webp(&bytes).expect("VP8 decode failed");
        assert_eq!(decoded.width, 16);
        assert_eq!(decoded.height, 16);
        for y in 0..16u32 {
            for x in 0..16u32 {
                let (r, _, _, _) = decoded.pixel_at(x, y);
                let orig_luma = 180i32;
                let dec_luma  = r as i32; // grey image: R≈G≈B
                assert!(
                    (dec_luma - orig_luma).abs() <= 5,
                    "pixel ({x},{y}): expected ~{orig_luma}, got {dec_luma}"
                );
            }
        }
    }

    #[test]
    fn round_trip_lossy_quality_100() {
        // quality=100 → qp=0 → step=4 → max error ≤ 2
        let mut pixels = PixelContainer::new(16, 16);
        for y in 0..16u32 {
            for x in 0..16u32 {
                pixels.set_pixel(x, y, 200, 200, 200, 255);
            }
        }
        let bytes = encode_webp(&pixels, 100);
        let decoded = decode_webp(&bytes).expect("VP8 decode failed");
        for y in 0..16u32 {
            for x in 0..16u32 {
                let (r, _, _, _) = decoded.pixel_at(x, y);
                assert!(
                    (r as i32 - 200).abs() <= 2,
                    "quality=100 round-trip error too large at ({x},{y}): got {r}"
                );
            }
        }
    }

    #[test]
    fn round_trip_lossy_color() {
        // Non-grey solid color: (R=200, G=80, B=40) → significant Cb and Cr residuals.
        // Tolerance: ±15 per channel (accounts for YCbCr quantization spread into RGB).
        let mut pixels = PixelContainer::new(16, 16);
        for y in 0..16u32 {
            for x in 0..16u32 {
                pixels.set_pixel(x, y, 200, 80, 40, 255);
            }
        }
        let bytes = encode_webp(&pixels, 75);
        let decoded = decode_webp(&bytes).expect("VP8 color decode failed");
        assert_eq!(decoded.width, 16);
        assert_eq!(decoded.height, 16);
        for y in 0..16u32 {
            for x in 0..16u32 {
                let (r, g, b, a) = decoded.pixel_at(x, y);
                assert_eq!(a, 255);
                assert!((r as i32 - 200).abs() <= 15, "R error at ({x},{y}): got {r}");
                assert!((g as i32 -  80).abs() <= 15, "G error at ({x},{y}): got {g}");
                assert!((b as i32 -  40).abs() <= 15, "B error at ({x},{y}): got {b}");
            }
        }
    }

    #[test]
    fn decode_error_truncated() {
        let mut fake = vec![0u8; 20];
        fake[0..4].copy_from_slice(b"RIFF");
        fake[4..8].copy_from_slice(&12u32.to_le_bytes());
        fake[8..12].copy_from_slice(b"WEBP");
        fake[12..16].copy_from_slice(b"VP8 ");
        // chunk_size = 100, but we only provide 4 bytes
        fake[16..20].copy_from_slice(&100u32.to_le_bytes());
        let result = decode_webp(&fake);
        assert!(result.is_err(), "truncated VP8 frame should return Err");
    }

    #[test]
    fn decode_unknown_chunk_returns_err() {
        let mut fake = vec![0u8; 24];
        fake[0..4].copy_from_slice(b"RIFF");
        fake[4..8].copy_from_slice(&16u32.to_le_bytes());
        fake[8..12].copy_from_slice(b"WEBP");
        fake[12..16].copy_from_slice(b"UNKN");
        fake[16..20].copy_from_slice(&4u32.to_le_bytes());
        let result = decode_webp(&fake);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown chunk"));
    }
}
