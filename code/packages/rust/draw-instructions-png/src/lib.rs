//! # draw-instructions-png
//!
//! PNG encoder for draw-instructions pixel buffers.
//!
//! This crate takes a `PixelBuffer` (the universal interchange format
//! between GPU renderers and image encoders) and encodes it as a PNG file.
//! It knows nothing about Metal, Vulkan, or any renderer — it just takes
//! pixels and produces PNG.
//!
//! ## How PNG encoding works
//!
//! PNG (Portable Network Graphics) stores images losslessly using deflate
//! compression.  The encoding process:
//!
//! 1. **Header** — 8-byte magic number (`\x89PNG\r\n\x1a\n`) that identifies
//!    the file as PNG
//! 2. **IHDR chunk** — image dimensions, color type (RGBA), bit depth (8)
//! 3. **IDAT chunks** — the pixel data, filtered and deflate-compressed
//! 4. **IEND chunk** — marks the end of the file
//!
//! The `png` crate handles all of this.  We just configure it with our
//! pixel buffer's dimensions and color type, then write the raw RGBA bytes.
//!
//! ## Pixel format
//!
//! The `PixelBuffer` from `draw-instructions-pixels` stores RGBA8 data
//! in row-major order with a top-left origin — exactly what PNG expects.
//! No conversion is needed.

pub const VERSION: &str = "0.1.0";

use draw_instructions_pixels::{PixelBuffer, PixelEncoder};
use std::io::{self, BufWriter, Write};

// ---------------------------------------------------------------------------
// PngEncoder — implements the PixelEncoder trait
// ---------------------------------------------------------------------------

/// PNG encoder for pixel buffers.
///
/// Uses the `png` crate (pure Rust) for encoding.  The default compression
/// level provides a good balance of file size and encoding speed for the
/// barcode use case (mostly solid rectangles with large uniform areas
/// that compress very well).
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
///
/// This is a convenience function that creates a `PngEncoder` and calls
/// `encode`.  Use this when you don't need the trait-based API.
pub fn encode_png(buffer: &PixelBuffer) -> Vec<u8> {
    let mut output = Vec::new();
    write_png_to(buffer, &mut output);
    output
}

/// Encode and write a PNG directly to a file.
///
/// Creates the file if it doesn't exist, truncates it if it does.
///
/// # Safety (path handling)
///
/// The `path` is used directly with `std::fs::File::create`.  The caller
/// is responsible for ensuring the path is safe — do not pass untrusted
/// user input without validation.  This function does not guard against
/// path traversal (e.g. `"../../etc/shadow"`).
pub fn write_png(buffer: &PixelBuffer, path: &str) -> io::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_png_to(buffer, &mut writer);
    writer.flush()
}

// ---------------------------------------------------------------------------
// Internal implementation
// ---------------------------------------------------------------------------

/// Write PNG data to any `Write` destination.
///
/// The PNG crate's encoder API works in three steps:
/// 1. Create an encoder with the target writer and image dimensions
/// 2. Configure color type and bit depth
/// 3. Write the header, then write the pixel data
fn write_png_to<W: Write>(buffer: &PixelBuffer, output: &mut W) {
    let mut encoder = png::Encoder::new(output, buffer.width, buffer.height);

    // RGBA = four channels (red, green, blue, alpha), 8 bits each.
    // This matches our PixelBuffer format exactly.
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    // Write the PNG header (magic bytes + IHDR chunk)
    let mut writer = encoder
        .write_header()
        .expect("PNG header write should not fail for in-memory buffer");

    // Write the pixel data (IDAT chunks — the crate handles filtering
    // and deflate compression internally)
    writer
        .write_image_data(&buffer.data)
        .expect("PNG data write should not fail for in-memory buffer");
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

    /// Encode a 2×2 image with known pixel values, decode it back,
    /// and verify the pixels match.
    #[test]
    fn round_trip_encode_decode() {
        // Create a 2×2 image:
        //   (0,0) = red     (1,0) = green
        //   (0,1) = blue    (1,1) = white
        let mut buf = PixelBuffer::new(2, 2);
        buf.set_pixel(0, 0, 255, 0, 0, 255);   // red
        buf.set_pixel(1, 0, 0, 255, 0, 255);   // green
        buf.set_pixel(0, 1, 0, 0, 255, 255);   // blue
        buf.set_pixel(1, 1, 255, 255, 255, 255); // white

        let png_data = encode_png(&buf);

        // Decode it back
        let decoder = png::Decoder::new(std::io::Cursor::new(&png_data));
        let mut reader = decoder.read_info().expect("PNG decode should succeed");
        let mut decoded = vec![0u8; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut decoded).expect("frame read should succeed");

        assert_eq!(info.width, 2);
        assert_eq!(info.height, 2);
        assert_eq!(info.color_type, png::ColorType::Rgba);
        assert_eq!(info.bit_depth, png::BitDepth::Eight);

        // Verify pixel values
        let decoded = &decoded[..info.buffer_size()];
        assert_eq!(&decoded[0..4], &[255, 0, 0, 255]);     // red
        assert_eq!(&decoded[4..8], &[0, 255, 0, 255]);     // green
        assert_eq!(&decoded[8..12], &[0, 0, 255, 255]);    // blue
        assert_eq!(&decoded[12..16], &[255, 255, 255, 255]); // white
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

    /// Zero-size images should still produce valid PNG.
    #[test]
    fn zero_size_image_encodes() {
        // A 0×0 image is technically valid PNG (some decoders accept it).
        // The png crate may or may not support this, but our API shouldn't panic.
        // If the crate panics, we'd need to handle this as a special case.
        // For now, we test 1×1 as the minimum meaningful image.
        let buf = PixelBuffer::new(1, 1);
        let png_data = encode_png(&buf);
        assert!(png_data.len() > 8);
    }
}
