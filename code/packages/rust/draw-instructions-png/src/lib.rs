//! # draw-instructions-png
//!
//! PNG encoder for draw-instructions pixel buffers.
//!
//! This crate takes a `PixelBuffer` (the universal interchange format
//! between GPU renderers and image encoders) and encodes it as a PNG file.
//! It knows nothing about Metal, Vulkan, or any renderer — it just takes
//! pixels and produces PNG.
//!
//! Uses our zero-dependency `png` and `deflate` crates — no external
//! supply chain dependencies.
//!
//! ## Pixel format
//!
//! The `PixelBuffer` from `draw-instructions-pixels` stores RGBA8 data
//! in row-major order with a top-left origin — exactly what PNG expects.
//! No conversion is needed.

pub const VERSION: &str = "0.1.0";

use draw_instructions_pixels::{PixelBuffer, PixelEncoder};
use std::io;

// ---------------------------------------------------------------------------
// PngEncoder — implements the PixelEncoder trait
// ---------------------------------------------------------------------------

/// PNG encoder for pixel buffers.
///
/// Uses our zero-dependency `png` crate for encoding.
pub struct PngEncoder;

impl PixelEncoder for PngEncoder {
    /// Encode a pixel buffer to PNG bytes.
    ///
    /// Returns the complete PNG file as a byte vector, including the
    /// 8-byte magic header, IHDR, IDAT, and IEND chunks.
    fn encode(&self, buffer: &PixelBuffer) -> Vec<u8> {
        encode_png(buffer)
    }
}

// ---------------------------------------------------------------------------
// Public API — convenience functions
// ---------------------------------------------------------------------------

/// Encode a pixel buffer to PNG bytes.
pub fn encode_png(buffer: &PixelBuffer) -> Vec<u8> {
    png::encode_png_rgba(buffer.width, buffer.height, &buffer.data)
}

/// Encode and write a PNG directly to a file.
///
/// Creates the file if it doesn't exist, truncates it if it does.
///
/// # Safety (path handling)
///
/// The `path` is used directly with `std::fs::File::create`.  The caller
/// is responsible for ensuring the path is safe — do not pass untrusted
/// user input without validation.
pub fn write_png(buffer: &PixelBuffer, path: &str) -> io::Result<()> {
    png::write_png_rgba(buffer.width, buffer.height, &buffer.data, path)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use draw_instructions_pixels::PixelBuffer;

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    /// The PNG magic bytes are `\x89PNG\r\n\x1a\n` — 8 bytes that
    /// identify a file as PNG.  Every valid PNG starts with these.
    #[test]
    fn encoded_png_has_magic_bytes() {
        let buf = PixelBuffer::new(1, 1);
        let png_data = encode_png(&buf);

        assert!(png_data.len() > 8, "PNG should be more than 8 bytes");
        assert_eq!(
            &png_data[0..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
            "PNG magic bytes should match"
        );
    }

    /// The PixelEncoder trait implementation should produce the same
    /// output as the convenience function.
    #[test]
    fn pixel_encoder_trait_matches_convenience_fn() {
        let buf = PixelBuffer::new(4, 4);
        let via_trait = PngEncoder.encode(&buf);
        let via_fn = encode_png(&buf);
        assert_eq!(via_trait, via_fn);
    }

    /// Encode a 2×2 image with known pixel values and verify the
    /// PNG structure is valid.
    #[test]
    fn encode_2x2_image() {
        let mut buf = PixelBuffer::new(2, 2);
        buf.set_pixel(0, 0, 255, 0, 0, 255);   // red
        buf.set_pixel(1, 0, 0, 255, 0, 255);   // green
        buf.set_pixel(0, 1, 0, 0, 255, 255);   // blue
        buf.set_pixel(1, 1, 255, 255, 255, 255); // white

        let png_data = encode_png(&buf);

        // Valid PNG structure: magic + IHDR + IDAT + IEND
        assert!(png_data.len() > 8 + 25 + 12 + 12);

        // Check IHDR chunk
        assert_eq!(&png_data[8..12], &[0, 0, 0, 13]); // length = 13
        assert_eq!(&png_data[12..16], b"IHDR");
        assert_eq!(&png_data[16..20], &2u32.to_be_bytes()); // width
        assert_eq!(&png_data[20..24], &2u32.to_be_bytes()); // height
    }

    #[test]
    fn minimum_image_encodes() {
        let buf = PixelBuffer::new(1, 1);
        let png_data = encode_png(&buf);
        assert!(png_data.len() > 8);
    }
}
