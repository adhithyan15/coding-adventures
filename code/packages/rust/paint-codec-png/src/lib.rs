//! # paint-codec-png
//!
//! PNG image codec for the paint-instructions pixel pipeline.
//!
//! This crate encodes a [`PixelContainer`] to PNG bytes and decodes PNG bytes
//! back to a [`PixelContainer`]. It knows nothing about Metal, SVG, or any
//! renderer — it just takes pixels in, produces PNG out (and vice versa).
//!
//! Uses the zero-dependency `png` crate from the workspace — no external
//! supply-chain dependencies.
//!
//! ## Architecture
//!
//! ```text
//! paint-metal (GPU renderer)
//!   │
//!   │  PixelContainer (RGBA8 bytes)
//!   ▼
//! paint-codec-png (this crate)
//!   │
//!   │  Vec<u8> (PNG file bytes)
//!   ▼
//! std::fs::write("output.png", png_bytes)
//! ```
//!
//! ## Why a separate crate?
//!
//! Each codec is its own crate so that consumers can depend on exactly the
//! formats they need. A terminal renderer doesn't need PNG. A Metal renderer
//! doesn't need WebP. The `ImageCodec` trait in `paint-instructions` is the
//! shared contract; each codec crate implements it independently.
//!
//! ## Pipeline example
//!
//! ```rust,ignore
//! use paint_instructions::PixelContainer;
//! use paint_codec_png::PngCodec;
//! use paint_instructions::ImageCodec;
//!
//! // Encode
//! let pixels = PixelContainer::new(100, 100);
//! let png_bytes = PngCodec.encode(&pixels);
//! std::fs::write("output.png", &png_bytes).unwrap();
//!
//! // Decode
//! let roundtrip = PngCodec.decode(&png_bytes).unwrap();
//! assert_eq!(roundtrip.width, 100);
//! assert_eq!(roundtrip.height, 100);
//! ```
//!
//! ## PNG file structure
//!
//! PNG stores image data in a series of named chunks:
//!
//! ```text
//! 8-byte magic: 89 50 4E 47 0D 0A 1A 0A
//!
//! IHDR chunk (13 bytes):
//!   width         u32 big-endian
//!   height        u32 big-endian
//!   bit depth     u8  (8 for RGBA8)
//!   colour type   u8  (6 = RGBA)
//!   compression   u8  (0 = deflate)
//!   filter method u8  (0 = adaptive)
//!   interlace     u8  (0 = none)
//!
//! IDAT chunk(s): deflate-compressed scanlines, each prefixed by a filter byte
//!
//! IEND chunk:    0-byte terminator
//! ```
//!
//! ## Colour type 6 (RGBA)
//!
//! We always use RGBA (colour type 6) because `PixelContainer` is always RGBA8.
//! There is no RGB or grayscale conversion path — callers who need those can
//! post-process the `PixelContainer` before encoding.

pub const VERSION: &str = "0.2.0";

use paint_instructions::{ImageCodec, PixelContainer};
use std::io;

// ---------------------------------------------------------------------------
// PngCodec — implements ImageCodec for PNG
// ---------------------------------------------------------------------------

/// PNG image codec.
///
/// Implements the [`ImageCodec`] trait from `paint-instructions`.
/// Zero-dependency: uses the workspace `png` crate (no `libpng`, no `image` crate).
///
/// ## Encoding
///
/// The `encode()` method calls `png::encode_png_rgba()`, which:
/// 1. Writes the 8-byte PNG magic signature
/// 2. Writes an IHDR chunk (width, height, bit depth 8, colour type RGBA)
/// 3. Runs deflate compression on the pixel data (one filter byte per row)
/// 4. Writes one or more IDAT chunks with the compressed data
/// 5. Writes the 12-byte IEND terminator chunk
///
/// ## Decoding
///
/// The `decode()` method calls `png::decode_png_rgba()`, which:
/// 1. Validates the magic signature
/// 2. Reads IHDR to get dimensions and bit depth
/// 3. Decompresses IDAT data with inflate
/// 4. Strips per-row filter bytes to recover raw pixels
/// 5. Returns the RGBA8 data as a `PixelContainer`
pub struct PngCodec;

impl ImageCodec for PngCodec {
    fn mime_type(&self) -> &'static str {
        "image/png"
    }

    /// Encode a [`PixelContainer`] to PNG bytes.
    ///
    /// Fully implemented — uses the workspace `png` crate (zero external deps).
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> {
        encode_png(pixels)
    }

    /// Decode PNG bytes back to a [`PixelContainer`].
    ///
    /// Returns `Err` until inflate support is available in the workspace.
    /// See [`decode_png`] for details.
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_png(bytes)
    }
}

// ---------------------------------------------------------------------------
// Public convenience functions
// ---------------------------------------------------------------------------

/// Encode a [`PixelContainer`] to PNG bytes.
///
/// Returns the complete PNG file — magic header, IHDR, IDAT, IEND.
/// This is identical to calling `PngCodec.encode(pixels)`.
///
/// ## Example
///
/// ```rust,ignore
/// let pixels = paint_metal::render(&scene);
/// let png_bytes = paint_codec_png::encode_png(&pixels);
/// std::fs::write("output.png", png_bytes).unwrap();
/// ```
pub fn encode_png(pixels: &PixelContainer) -> Vec<u8> {
    png::encode_png_rgba(pixels.width, pixels.height, &pixels.data)
}

/// Decode PNG bytes back to a [`PixelContainer`].
///
/// Delegates to `png::decode_png_rgba`, which handles:
/// 1. 8-byte magic validation
/// 2. IHDR parsing (bit depth 8, RGB or RGBA, no interlacing)
/// 3. Multi-IDAT reassembly and zlib decompression via `deflate::zlib_decompress`
/// 4. All 5 PNG filter types (None, Sub, Up, Average, Paeth)
/// 5. RGB→RGBA expansion (alpha=255 inserted for colour type 2)
///
/// Returns `Err` with a descriptive message on any format violation.
pub fn decode_png(bytes: &[u8]) -> Result<PixelContainer, String> {
    if bytes.len() < 8 {
        return Err("PNG decode: input too short to be a valid PNG file".to_string());
    }
    let (width, height, rgba) = png::decode_png_rgba(bytes)
        .map_err(|e| format!("PNG decode: {}", e))?;
    Ok(PixelContainer::from_data(width, height, rgba))
}

/// Encode a [`PixelContainer`] and write the PNG to a file.
///
/// Creates the file if it doesn't exist; truncates it if it does.
///
/// # Safety (path handling)
///
/// `path` is passed directly to `std::fs::File::create`. Do not pass
/// untrusted user input without validation.
pub fn write_png(pixels: &PixelContainer, path: &str) -> io::Result<()> {
    png::write_png_rgba(pixels.width, pixels.height, &pixels.data, path)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::{ImageCodec, PixelContainer};

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.2.0");
    }

    #[test]
    fn mime_type_is_png() {
        assert_eq!(PngCodec.mime_type(), "image/png");
    }

    /// Every valid PNG file begins with the 8-byte magic signature.
    ///
    /// The bytes are:
    ///   0x89  — high bit set, not valid ASCII (catches text-mode FTP transfers)
    ///   PNG   — human-readable identifier
    ///   0x0D 0x0A  — DOS line ending (catches CRLF-stripping)
    ///   0x1A  — Ctrl-Z (catches ^Z-stopping in DOS)
    ///   0x0A  — Unix line ending
    #[test]
    fn encoded_png_has_magic_bytes() {
        let pixels = PixelContainer::new(1, 1);
        let png_data = PngCodec.encode(&pixels);

        assert!(png_data.len() > 8, "PNG should be more than 8 bytes");
        assert_eq!(
            &png_data[0..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
            "PNG magic bytes should match"
        );
    }

    /// The IHDR chunk starts at byte 8 and is always 25 bytes:
    ///   4 bytes length (= 13)
    ///   4 bytes "IHDR"
    ///   13 bytes data (width, height, bit depth, colour type, compression, filter, interlace)
    ///   4 bytes CRC
    #[test]
    fn encoded_png_has_valid_ihdr() {
        let pixels = PixelContainer::new(10, 20);
        let png_data = PngCodec.encode(&pixels);

        // IHDR length field = 13
        assert_eq!(&png_data[8..12], &[0, 0, 0, 13]);
        // IHDR type tag
        assert_eq!(&png_data[12..16], b"IHDR");
        // Width = 10 as big-endian u32
        assert_eq!(&png_data[16..20], &10u32.to_be_bytes());
        // Height = 20 as big-endian u32
        assert_eq!(&png_data[20..24], &20u32.to_be_bytes());
    }

    /// `ImageCodec::encode` and the convenience function must produce the same output.
    #[test]
    fn trait_and_convenience_fn_agree() {
        let pixels = PixelContainer::new(4, 4);
        assert_eq!(PngCodec.encode(&pixels), encode_png(&pixels));
    }

    /// 1×1 image — the smallest valid PNG.
    #[test]
    fn minimum_image_encodes() {
        let pixels = PixelContainer::new(1, 1);
        let png_data = PngCodec.encode(&pixels);
        assert!(png_data.len() > 8, "1×1 PNG should be non-trivial");
    }

    /// A 100×100 image encodes without panicking.
    #[test]
    fn larger_image_encodes() {
        let mut pixels = PixelContainer::new(100, 100);
        for y in 0..100u32 {
            for x in 0..100u32 {
                pixels.set_pixel(x, y, (x * 2) as u8, (y * 2) as u8, 128, 255);
            }
        }
        let png_bytes = PngCodec.encode(&pixels);
        // Valid PNG must start with the magic bytes
        assert_eq!(&png_bytes[0..4], &[0x89, b'P', b'N', b'G']);
    }

    // ─── Decode — full round-trip now that inflate is in the workspace ────────

    /// Encode then decode a 2×2 image — every pixel must survive losslessly.
    #[test]
    fn decode_roundtrip_rgba() {
        let mut pixels = PixelContainer::new(2, 2);
        pixels.set_pixel(0, 0, 255,   0,   0, 255);
        pixels.set_pixel(1, 0,   0, 255,   0, 128);
        pixels.set_pixel(0, 1,   0,   0, 255,  64);
        pixels.set_pixel(1, 1, 128, 128, 128, 255);
        let png_bytes = PngCodec.encode(&pixels);
        let decoded = PngCodec.decode(&png_bytes).expect("round-trip decode failed");
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 2);
        assert_eq!(decoded.pixel_at(0, 0), (255,   0,   0, 255));
        assert_eq!(decoded.pixel_at(1, 0), (  0, 255,   0, 128));
        assert_eq!(decoded.pixel_at(0, 1), (  0,   0, 255,  64));
        assert_eq!(decoded.pixel_at(1, 1), (128, 128, 128, 255));
    }

    /// Round-trip convenience functions agree with trait methods.
    #[test]
    fn decode_convenience_fn_matches_trait() {
        let mut pixels = PixelContainer::new(3, 3);
        pixels.set_pixel(1, 1, 200, 100, 50, 255);
        let png_bytes = PngCodec.encode(&pixels);
        let via_trait = PngCodec.decode(&png_bytes).expect("trait decode failed");
        let via_fn    = decode_png(&png_bytes).expect("fn decode failed");
        assert_eq!(via_trait.width,  via_fn.width);
        assert_eq!(via_trait.height, via_fn.height);
        assert_eq!(via_trait.data,   via_fn.data);
    }

    /// Round-trip a 100×100 gradient image — checks no panics, correct size.
    #[test]
    fn decode_roundtrip_large() {
        let mut pixels = PixelContainer::new(100, 100);
        for y in 0..100u32 {
            for x in 0..100u32 {
                pixels.set_pixel(x, y, (x * 2) as u8, (y * 2) as u8, 128, 255);
            }
        }
        let decoded = decode_png(&encode_png(&pixels)).expect("large round-trip failed");
        assert_eq!(decoded.width, 100);
        assert_eq!(decoded.height, 100);
        assert_eq!(decoded.pixel_at(50, 50), (100, 100, 128, 255));
    }

    /// Garbage input returns a descriptive error, not a panic.
    #[test]
    fn decode_garbage_returns_err() {
        let result = PngCodec.decode(b"this is not a PNG");
        assert!(result.is_err());
    }

    /// Empty slice returns a descriptive error.
    #[test]
    fn decode_empty_returns_err() {
        let result = PngCodec.decode(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too short"));
    }
}
