//! # image-codec-jxl — JPEG XL Modular lossless encoder/decoder (IC09)
//!
//! This crate implements a simplified but self-consistent JPEG XL Modular
//! encoder and decoder for teaching purposes.
//!
//! ## JPEG XL overview
//!
//! JPEG XL (JXL) is an open image format standardised by ISO/IEC 18181 and
//! designed as a successor to legacy JPEG.  It has two main coding modes:
//!
//! - **VarDCT** — a lossy path similar to JPEG, using DCT blocks and quantisation.
//! - **Modular** — a lossless (or visually lossless) path based on gradient
//!   prediction + entropy coding.  This is what we implement here.
//!
//! ## What we implement (Phase 1 scope)
//!
//! We emit a **naked codestream** (magic `FF 0A`) rather than an ISOBMFF
//! container.  The internal structure after the SizeHeader is our own
//! simplified binary format (not standard JXL metadata ANS coding), which
//! keeps the implementation short and learnable.
//!
//! The decoder handles both naked codestreams and ISOBMFF-wrapped files
//! (container detection only — the metadata inside would need the full JXL
//! spec to decode).
//!
//! ## Codec pipeline
//!
//! ```text
//! PixelContainer
//!       │
//!       ▼ encoder
//! For each channel (R, G, B, [A]):
//!   1. Flatten to i32 plane
//!   2. Gradient predictor  →  residuals ∈ [−255, 255]
//!   3. Sign/magnitude split →  two u8 symbol streams
//!   4. rANS entropy coding  →  compressed bytes
//!       │
//!       ▼ wire format
//! [FF 0A][SizeHeader bits][num_ch w h][sign rANS][mag rANS] × channels
//!       │
//!       ▼ decoder (reverse)
//! PixelContainer
//! ```
//!
//! ## Crate version
//!
//! The `VERSION` constant mirrors the `Cargo.toml` version string.

pub mod bitreader;
pub mod bitwriter;
pub mod container;
pub mod decoder;
pub mod encoder;
pub mod entropy;
pub mod modular;
pub mod rct;

use pixel_container::{ImageCodec, PixelContainer};

/// Crate version, matching `Cargo.toml`.
pub const VERSION: &str = "0.1.0";

// ── Public API ───────────────────────────────────────────────────────────────

/// Encode a [`PixelContainer`] into a simplified JXL Modular codestream.
///
/// The output starts with the two-byte naked codestream magic `FF 0A` and is
/// a complete, self-contained file that `decode_jxl` can round-trip exactly.
///
/// # Panics
///
/// Panics if `pixels` has zero width or zero height.
pub fn encode_jxl(pixels: &PixelContainer) -> Vec<u8> {
    encoder::encode(pixels)
}

/// Decode a JXL codestream or ISOBMFF container back into a [`PixelContainer`].
///
/// Only our simplified format (as emitted by `encode_jxl`) is guaranteed to
/// decode correctly.  Standard libjxl output would require the full JXL spec.
///
/// # Errors
///
/// Returns `Err` with a descriptive message if the data is not recognised as
/// a valid simplified JXL Modular stream.
pub fn decode_jxl(data: &[u8]) -> Result<PixelContainer, String> {
    decoder::decode(data)
}

// ── ImageCodec trait implementation ──────────────────────────────────────────

/// Unit struct that implements [`ImageCodec`] for JPEG XL Modular lossless.
///
/// # Example
///
/// ```
/// use image_codec_jxl::JxlCodec;
/// use pixel_container::{ImageCodec, PixelContainer};
///
/// let mut src = PixelContainer::new(4, 4);
/// src.fill(200, 100, 50, 255);
///
/// let bytes  = JxlCodec.encode(&src);
/// let dst    = JxlCodec.decode(&bytes).unwrap();
///
/// assert_eq!(dst.pixel_at(2, 2), (200, 100, 50, 255));
/// ```
pub struct JxlCodec;

impl ImageCodec for JxlCodec {
    fn mime_type(&self) -> &'static str {
        "image/jxl"
    }

    fn encode(&self, container: &PixelContainer) -> Vec<u8> {
        encode_jxl(container)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_jxl(bytes)
    }
}

// ── Integration tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Round-trip helper ────────────────────────────────────────────────

    /// Encode then decode and assert pixel-perfect identity.
    fn round_trip(px: &PixelContainer) {
        let bytes = encode_jxl(px);
        let recovered = decode_jxl(&bytes)
            .unwrap_or_else(|e| panic!("decode failed: {}", e));

        assert_eq!(recovered.width, px.width, "width mismatch");
        assert_eq!(recovered.height, px.height, "height mismatch");

        for y in 0..px.height {
            for x in 0..px.width {
                assert_eq!(
                    recovered.pixel_at(x, y),
                    px.pixel_at(x, y),
                    "pixel mismatch at ({}, {})",
                    x,
                    y
                );
            }
        }
    }

    // ── Round-trip tests ─────────────────────────────────────────────────

    #[test]
    fn round_trip_solid_rgb() {
        let mut p = PixelContainer::new(4, 4);
        p.fill(200, 100, 50, 255);
        round_trip(&p);
    }

    #[test]
    fn round_trip_1x1() {
        let mut p = PixelContainer::new(1, 1);
        p.set_pixel(0, 0, 10, 20, 30, 200);
        round_trip(&p);
    }

    #[test]
    fn round_trip_transparent_solid() {
        let mut p = PixelContainer::new(4, 4);
        p.fill(0, 0, 0, 0);
        round_trip(&p);
    }

    #[test]
    fn round_trip_mixed_alpha() {
        let mut p = PixelContainer::new(2, 2);
        p.set_pixel(0, 0, 255, 0, 0, 255);
        p.set_pixel(1, 0, 0, 255, 0, 128);
        p.set_pixel(0, 1, 0, 0, 255, 0);
        p.set_pixel(1, 1, 128, 128, 0, 64);
        round_trip(&p);
    }

    #[test]
    fn round_trip_gradient() {
        let mut p = PixelContainer::new(8, 8);
        for y in 0..8u32 {
            for x in 0..8u32 {
                p.set_pixel(x, y, (x * 32) as u8, (y * 32) as u8, 128, 255);
            }
        }
        round_trip(&p);
    }

    #[test]
    fn round_trip_large() {
        let mut p = PixelContainer::new(32, 32);
        for y in 0..32u32 {
            for x in 0..32u32 {
                p.set_pixel(x, y, x as u8, y as u8, 200, 255);
            }
        }
        round_trip(&p);
    }

    #[test]
    fn round_trip_max_channel_values() {
        let mut p = PixelContainer::new(4, 4);
        p.fill(255, 255, 255, 255);
        round_trip(&p);
    }

    #[test]
    fn round_trip_min_channel_values() {
        // All zeros = transparent black; has_alpha detects A=0 → 4-channel encode.
        let p = PixelContainer::new(4, 4);
        round_trip(&p);
    }

    #[test]
    fn round_trip_1x1_transparent() {
        let mut p = PixelContainer::new(1, 1);
        p.set_pixel(0, 0, 0, 0, 0, 0);
        round_trip(&p);
    }

    // ── Format / magic tests ─────────────────────────────────────────────

    #[test]
    fn naked_codestream_magic() {
        let p = PixelContainer::new(2, 2);
        let bytes = encode_jxl(&p);
        assert_eq!(&bytes[..2], &[0xFF, 0x0A], "first two bytes must be JXL naked magic");
    }

    // ── Error cases ──────────────────────────────────────────────────────

    #[test]
    fn decode_error_bad_magic() {
        let err = decode_jxl(b"\x89PNG\r\n\x1a\n").unwrap_err();
        assert!(!err.is_empty(), "error message must not be empty");
    }

    #[test]
    fn decode_error_too_short() {
        let err = decode_jxl(b"\xFF").unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn decode_error_empty() {
        let err = decode_jxl(b"").unwrap_err();
        assert!(!err.is_empty());
    }

    // ── ImageCodec trait ─────────────────────────────────────────────────

    #[test]
    fn codec_mime_type() {
        assert_eq!(JxlCodec.mime_type(), "image/jxl");
    }

    #[test]
    fn codec_encode_decode_round_trip() {
        let mut p = PixelContainer::new(4, 4);
        p.fill(100, 150, 200, 255);
        let bytes = JxlCodec.encode(&p);
        let recovered = JxlCodec.decode(&bytes).unwrap();
        assert_eq!(recovered.width, 4);
        assert_eq!(recovered.height, 4);
        assert_eq!(recovered.pixel_at(0, 0), (100, 150, 200, 255));
        assert_eq!(recovered.pixel_at(3, 3), (100, 150, 200, 255));
    }

    #[test]
    fn codec_version_is_semver() {
        // Must be parseable as X.Y.Z
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "VERSION must be X.Y.Z");
        for part in parts {
            assert!(part.parse::<u32>().is_ok(), "each VERSION part must be numeric");
        }
    }
}
