//! # image-codec-ico
//!
//! ICO (Windows Icon) and CUR (cursor) image codec — IC08.
//!
//! Encodes a `PixelContainer` as a single-image 32bpp ICO file and decodes
//! the best-resolution image from any ICO/CUR file into an RGBA8 `PixelContainer`.
//!
//! ## Quick start
//!
//! ```
//! use image_codec_ico::{encode_ico, decode_ico};
//! use pixel_container::PixelContainer;
//!
//! // Encode a 2×2 red ICO.
//! let mut px = PixelContainer::new(2, 2);
//! px.fill(255, 0, 0, 255);
//! let bytes = encode_ico(&px);
//! assert_eq!(&bytes[2..4], &[1, 0]); // type = ICO
//!
//! // Decode it back.
//! let recovered = decode_ico(&bytes).unwrap();
//! assert_eq!(recovered.width, 2);
//! assert_eq!(recovered.height, 2);
//! ```

pub use decoder::decode_ico;
pub use encoder::encode_ico;
use paint_instructions::ImageCodec;
use pixel_container::PixelContainer;

mod bmp_dib;
mod decoder;
mod encoder;

/// Package version — kept in sync with CHANGELOG.md and Cargo.toml.
pub const VERSION: &str = "0.1.0";

/// ICO/CUR image codec implementing `paint_instructions::ImageCodec`.
///
/// Encodes as a single-image 32bpp ICO (lossless, full RGBA).
/// Decodes ICO and CUR files, returning the best-resolution frame.
pub struct IcoCodec;

impl ImageCodec for IcoCodec {
    fn mime_type(&self) -> &'static str {
        "image/x-icon"
    }

    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> {
        encode_ico(pixels)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_ico(bytes)
    }
}

// ── Integration tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: encode then decode, asserting pixel-exact round-trip.
    fn round_trip(pixels: &PixelContainer) {
        let bytes = encode_ico(pixels);
        let recovered = decode_ico(&bytes)
            .unwrap_or_else(|e| panic!("decode failed: {}", e));
        assert_eq!(recovered.width, pixels.width, "width mismatch");
        assert_eq!(recovered.height, pixels.height, "height mismatch");
        for y in 0..pixels.height {
            for x in 0..pixels.width {
                let orig = pixels.pixel_at(x, y);
                let got = recovered.pixel_at(x, y);
                assert_eq!(got, orig, "pixel ({x},{y}) mismatch: orig={:?} got={:?}", orig, got);
            }
        }
    }

    // ── Basic round-trips ──────────────────────────────────────────────────

    #[test]
    fn round_trip_solid_rgba() {
        let mut px = PixelContainer::new(4, 4);
        px.fill(200, 100, 50, 255);
        round_trip(&px);
    }

    #[test]
    fn round_trip_1x1() {
        let mut px = PixelContainer::new(1, 1);
        px.set_pixel(0, 0, 10, 20, 30, 200);
        round_trip(&px);
    }

    #[test]
    fn round_trip_transparent() {
        let mut px = PixelContainer::new(4, 4);
        px.fill(0, 0, 0, 0); // fully transparent
        round_trip(&px);
    }

    #[test]
    fn round_trip_mixed_alpha() {
        let mut px = PixelContainer::new(2, 2);
        px.set_pixel(0, 0, 255, 0, 0, 255); // opaque red
        px.set_pixel(1, 0, 0, 255, 0, 128); // half-transparent green
        px.set_pixel(0, 1, 0, 0, 255, 0);   // fully transparent blue
        px.set_pixel(1, 1, 128, 128, 0, 64); // quarter-transparent yellow
        round_trip(&px);
    }

    #[test]
    fn round_trip_16x16() {
        let mut px = PixelContainer::new(16, 16);
        for y in 0..16u32 {
            for x in 0..16u32 {
                px.set_pixel(x, y, (x * 16) as u8, (y * 16) as u8, 128, 255);
            }
        }
        round_trip(&px);
    }

    #[test]
    fn round_trip_32x32() {
        let mut px = PixelContainer::new(32, 32);
        for y in 0..32u32 {
            for x in 0..32u32 {
                px.set_pixel(x, y, x as u8, y as u8, 200, 255);
            }
        }
        round_trip(&px);
    }

    // ── File format structure ──────────────────────────────────────────────

    #[test]
    fn encode_produces_correct_header() {
        let px = PixelContainer::new(2, 2);
        let bytes = encode_ico(&px);
        assert!(bytes.len() >= 6, "file must have at least 6 bytes");
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0, "reserved");
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 1, "type=ICO");
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 1, "count=1");
    }

    #[test]
    fn encode_bit_count_is_32() {
        let px = PixelContainer::new(1, 1);
        let bytes = encode_ico(&px);
        let bit_count = u16::from_le_bytes([bytes[12], bytes[13]]);
        assert_eq!(bit_count, 32);
    }

    #[test]
    fn encode_image_offset_is_22() {
        let px = PixelContainer::new(1, 1);
        let bytes = encode_ico(&px);
        let offset = u32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]);
        assert_eq!(offset, 22);
    }

    // ── Error cases ────────────────────────────────────────────────────────

    #[test]
    fn decode_error_bad_magic() {
        let err = decode_ico(b"\x89PNG\r\n\x1a\n").unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn decode_error_too_short() {
        let err = decode_ico(b"ICO").unwrap_err();
        assert!(err.contains("too short"), "got: {}", err);
    }

    #[test]
    fn decode_error_bad_type() {
        let data: Vec<u8> = vec![0, 0, 3, 0, 1, 0]; // type=3
        let err = decode_ico(&data).unwrap_err();
        assert!(err.contains("unknown type") || err.contains("type"), "got: {}", err);
    }

    // ── MIME type ──────────────────────────────────────────────────────────

    #[test]
    fn codec_mime_type() {
        assert_eq!(IcoCodec.mime_type(), "image/x-icon");
    }

    #[test]
    fn codec_encode_decode_round_trip() {
        let mut px = PixelContainer::new(4, 4);
        px.fill(100, 150, 200, 255);
        let bytes = IcoCodec.encode(&px);
        let recovered = IcoCodec.decode(&bytes).unwrap();
        assert_eq!(recovered.width, 4);
        assert_eq!(recovered.height, 4);
    }
}
