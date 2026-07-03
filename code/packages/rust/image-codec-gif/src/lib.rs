//! # image-codec-gif
//!
//! GIF (Graphics Interchange Format) image codec — IC07.
//!
//! Decodes GIF87a and GIF89a files into RGBA8 `PixelContainer` buffers and
//! encodes `PixelContainer` pixels as static GIF87a/GIF89a files.
//!
//! ## Key properties
//!
//! - **Color model**: indexed colour, ≤ 256 palette entries per image.
//! - **Compression**: GIF-variant LZW (minimum code size 2–8 bits, sub-block
//!   framing, LSB-first bit packing). Implemented inline (not the CMP03 crate)
//!   because GIF requires configurable code sizes.
//! - **Transparency**: one palette index designated transparent via Graphic
//!   Control Extension (GIF89a); decoded as alpha = 0.
//! - **Interlacing**: 4-pass de-interlacing on decode; never written on encode.
//! - **Animation**: first frame is returned; a second Image Descriptor triggers
//!   an error.
//!
//! ## Quick start
//!
//! ```
//! use image_codec_gif::{encode_gif, decode_gif};
//! use pixel_container::PixelContainer;
//!
//! // Encode a 2×2 red image.
//! let mut px = PixelContainer::new(2, 2);
//! px.fill(255, 0, 0, 255);
//! let bytes = encode_gif(&px);
//! assert!(bytes.starts_with(b"GIF"));
//!
//! // Decode it back.
//! let recovered = decode_gif(&bytes).unwrap();
//! assert_eq!(recovered.width, 2);
//! assert_eq!(recovered.height, 2);
//! ```

pub use decoder::decode_gif;
pub use encoder::encode_gif;
use paint_instructions::ImageCodec;
use pixel_container::PixelContainer;

mod decoder;
mod encoder;
mod lzw;

/// Package version — kept in sync with CHANGELOG.md and Cargo.toml.
pub const VERSION: &str = "0.1.0";

/// GIF image codec implementing `paint_instructions::ImageCodec`.
///
/// Encodes as GIF87a (opaque) or GIF89a (with transparency).
/// Decodes GIF87a and GIF89a — returns the first (only, for static) frame.
pub struct GifCodec;

impl ImageCodec for GifCodec {
    fn mime_type(&self) -> &'static str {
        "image/gif"
    }

    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> {
        encode_gif(pixels)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_gif(bytes)
    }
}

// ── Integration tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: encode and decode, asserting pixel-exact round-trip.
    fn round_trip_exact(pixels: &PixelContainer) {
        let bytes = encode_gif(pixels);
        let recovered = decode_gif(&bytes)
            .unwrap_or_else(|e| panic!("decode failed: {}", e));
        assert_eq!(recovered.width, pixels.width, "width mismatch");
        assert_eq!(recovered.height, pixels.height, "height mismatch");
        for y in 0..pixels.height {
            for x in 0..pixels.width {
                let orig = pixels.pixel_at(x, y);
                let got = recovered.pixel_at(x, y);
                // For opaque pixels: R/G/B must match, A may be 255 after palette encoding.
                // Transparent pixels: A = 0.
                if orig.3 < 128 {
                    assert_eq!(
                        got.3, 0,
                        "pixel ({x},{y}) should be transparent, got {:?}",
                        got
                    );
                } else {
                    assert_eq!(
                        (got.0, got.1, got.2),
                        (orig.0, orig.1, orig.2),
                        "pixel ({x},{y}) RGB mismatch: orig {:?}, got {:?}",
                        orig,
                        got
                    );
                    assert_eq!(got.3, 255, "pixel ({x},{y}) alpha should be 255");
                }
            }
        }
    }

    // ── Basic round-trips ──────────────────────────────────────────────────

    #[test]
    fn round_trip_solid_red() {
        let mut px = PixelContainer::new(4, 4);
        px.fill(255, 0, 0, 255);
        round_trip_exact(&px);
    }

    #[test]
    fn round_trip_solid_blue() {
        let mut px = PixelContainer::new(8, 8);
        px.fill(0, 0, 255, 255);
        round_trip_exact(&px);
    }

    #[test]
    fn round_trip_1x1() {
        let mut px = PixelContainer::new(1, 1);
        px.set_pixel(0, 0, 100, 150, 200, 255);
        round_trip_exact(&px);
    }

    #[test]
    fn round_trip_gradient_256_colors() {
        // 256 distinct colours along the grey ramp — no quantization needed.
        let mut px = PixelContainer::new(256, 1);
        for i in 0u32..256 {
            let v = i as u8;
            px.set_pixel(i, 0, v, v, v, 255);
        }
        round_trip_exact(&px);
    }

    #[test]
    fn round_trip_4_colors() {
        let mut px = PixelContainer::new(2, 2);
        px.set_pixel(0, 0, 255, 0, 0, 255); // red
        px.set_pixel(1, 0, 0, 255, 0, 255); // green
        px.set_pixel(0, 1, 0, 0, 255, 255); // blue
        px.set_pixel(1, 1, 255, 255, 0, 255); // yellow
        round_trip_exact(&px);
    }

    #[test]
    fn round_trip_black_image() {
        let mut px = PixelContainer::new(16, 16);
        px.fill(0, 0, 0, 255);
        round_trip_exact(&px);
    }

    // ── Transparency ───────────────────────────────────────────────────────

    #[test]
    fn round_trip_fully_transparent() {
        let mut px = PixelContainer::new(4, 4);
        px.fill(0, 0, 0, 0); // fully transparent
        let bytes = encode_gif(&px);
        assert!(bytes.starts_with(b"GIF89a"), "transparent GIF must be GIF89a");
        let recovered = decode_gif(&bytes).unwrap();
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(recovered.pixel_at(x, y).3, 0, "pixel ({x},{y}) should be transparent");
            }
        }
    }

    #[test]
    fn round_trip_mixed_transparency() {
        let mut px = PixelContainer::new(2, 2);
        px.set_pixel(0, 0, 255, 0, 0, 255); // opaque red
        px.set_pixel(1, 0, 0, 0, 0, 0);     // transparent
        px.set_pixel(0, 1, 0, 255, 0, 255); // opaque green
        px.set_pixel(1, 1, 0, 0, 0, 0);     // transparent
        let bytes = encode_gif(&px);
        assert!(bytes.starts_with(b"GIF89a"), "transparent GIF must be GIF89a");
        let recovered = decode_gif(&bytes).unwrap();
        // Opaque pixels should have alpha=255.
        assert_eq!(recovered.pixel_at(0, 0).3, 255);
        assert_eq!(recovered.pixel_at(0, 1).3, 255);
        // Transparent pixels should have alpha=0.
        assert_eq!(recovered.pixel_at(1, 0).3, 0);
        assert_eq!(recovered.pixel_at(1, 1).3, 0);
    }

    // ── File format structure ──────────────────────────────────────────────

    #[test]
    fn encode_produces_gif87a_for_opaque() {
        let mut px = PixelContainer::new(2, 2);
        px.fill(100, 100, 100, 255);
        let bytes = encode_gif(&px);
        assert!(bytes.starts_with(b"GIF87a"), "expected GIF87a, got {:?}", &bytes[..6]);
    }

    #[test]
    fn encode_ends_with_trailer() {
        let mut px = PixelContainer::new(2, 2);
        px.fill(50, 100, 150, 255);
        let bytes = encode_gif(&px);
        assert_eq!(*bytes.last().unwrap(), 0x3B, "GIF must end with trailer 0x3B");
    }

    #[test]
    fn encode_canvas_size_in_lsd() {
        let mut px = PixelContainer::new(100, 80);
        px.fill(0, 128, 0, 255);
        let bytes = encode_gif(&px);
        // Bytes 6-7: width (LE), bytes 8-9: height (LE)
        let w = u16::from_le_bytes([bytes[6], bytes[7]]);
        let h = u16::from_le_bytes([bytes[8], bytes[9]]);
        assert_eq!(w, 100);
        assert_eq!(h, 80);
    }

    // ── Error cases ────────────────────────────────────────────────────────

    #[test]
    fn decode_error_bad_magic() {
        let err = decode_gif(b"\x89PNG\r\n\x1a\n").unwrap_err();
        assert!(err.contains("not a GIF"), "got: {}", err);
    }

    #[test]
    fn decode_error_too_short() {
        let err = decode_gif(b"GIF").unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn decode_error_unknown_version() {
        let mut data = b"GIF99a".to_vec();
        data.extend(vec![0u8; 20]);
        let err = decode_gif(&data).unwrap_err();
        assert!(err.contains("unknown version") || err.contains("version"), "got: {}", err);
    }

    // ── MIME type ──────────────────────────────────────────────────────────

    #[test]
    fn codec_mime_type() {
        assert_eq!(GifCodec.mime_type(), "image/gif");
    }

    #[test]
    fn codec_encode_decode_round_trip() {
        let mut px = PixelContainer::new(4, 4);
        px.fill(200, 100, 50, 255);
        let bytes = GifCodec.encode(&px);
        let recovered = GifCodec.decode(&bytes).unwrap();
        assert_eq!(recovered.width, 4);
        assert_eq!(recovered.height, 4);
    }

    // ── Large image ────────────────────────────────────────────────────────

    #[test]
    fn round_trip_64x64_random_pattern() {
        let mut px = PixelContainer::new(64, 64);
        // Fill with a deterministic pseudo-random pattern using only 16 colours.
        let palette_16 = [
            (0u8, 0u8, 0u8), (255, 0, 0), (0, 255, 0), (0, 0, 255),
            (255, 255, 0), (255, 0, 255), (0, 255, 255), (128, 128, 128),
            (200, 100, 50), (50, 200, 100), (100, 50, 200), (255, 128, 0),
            (0, 128, 255), (128, 0, 255), (64, 192, 64), (192, 64, 192),
        ];
        for y in 0u32..64 {
            for x in 0u32..64 {
                let idx = ((x * 7 + y * 13) % 16) as usize;
                let (r, g, b) = palette_16[idx];
                px.set_pixel(x, y, r, g, b, 255);
            }
        }
        round_trip_exact(&px);
    }
}
