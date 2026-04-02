//! # draw-instructions-pixels
//!
//! Shared pixel buffer type that sits between GPU renderers and image encoders.
//!
//! ## The pixel pipeline
//!
//! Every GPU rendering API (Metal, Vulkan, Direct2D, OpenGL) can produce a
//! rectangular grid of RGBA pixels.  Every image encoder (PNG, JPEG, WebP)
//! consumes a rectangular grid of RGBA pixels.  This crate defines the
//! shared type — `PixelBuffer` — that connects them.
//!
//! ```text
//!   Metal renderer ──┐
//!   Vulkan renderer ─┤── PixelBuffer ──┬── PNG encoder
//!   Direct2D renderer┘                 ├── JPEG encoder
//!                                      └── WebP encoder
//! ```
//!
//! ## Pixel format
//!
//! RGBA8 — four bytes per pixel, each in the range 0–255:
//!
//! | Byte | Channel | Meaning                        |
//! |------|---------|--------------------------------|
//! | 0    | Red     | 0 = no red, 255 = full red     |
//! | 1    | Green   | 0 = no green, 255 = full green |
//! | 2    | Blue    | 0 = no blue, 255 = full blue   |
//! | 3    | Alpha   | 0 = transparent, 255 = opaque  |
//!
//! Pixels are stored in row-major order with a **top-left origin** — the
//! same coordinate system used by SVG, HTML Canvas, and the draw-instructions
//! IR.  The byte offset for pixel (x, y) is `(y * width + x) * 4`.
//!
//! ## Memory layout example
//!
//! For a 3×2 image:
//!
//! ```text
//! byte index:  0  1  2  3    4  5  6  7    8  9 10 11
//!              ├──R──G──B──A──┼──R──G──B──A──┼──R──G──B──A──┤  ← row 0
//! byte index: 12 13 14 15   16 17 18 19   20 21 22 23
//!              ├──R──G──B──A──┼──R──G──B──A──┼──R──G──B──A──┤  ← row 1
//! ```
//!
//! ## Why this format?
//!
//! This is the native output format of:
//! - Metal `getBytes()` with `MTLPixelFormat.rgba8Unorm`
//! - Vulkan `vkMapMemory` with `VK_FORMAT_R8G8B8A8_UNORM`
//! - OpenGL `glReadPixels` with `GL_RGBA` / `GL_UNSIGNED_BYTE`
//!
//! And the native input format for PNG, JPEG, and WebP encoders.
//! No conversion step is needed — the bytes flow straight from GPU to encoder.

pub const VERSION: &str = "0.1.0";

// ---------------------------------------------------------------------------
// PixelBuffer — the universal interchange type
// ---------------------------------------------------------------------------

/// An RGBA pixel buffer — the universal interchange format between
/// GPU renderers and image encoders.
///
/// The buffer stores pixels in row-major order with a top-left origin.
/// Each pixel is four bytes: red, green, blue, alpha (0–255 each).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PixelBuffer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

// The number of bytes per pixel.  RGBA = 4 channels × 1 byte each.
const BYTES_PER_PIXEL: usize = 4;

impl PixelBuffer {
    /// Create a new pixel buffer filled with transparent black (all zeros).
    ///
    /// Every pixel starts as (0, 0, 0, 0) — fully transparent black.
    /// This is the conventional "empty" state for an RGBA buffer.
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize)
            .checked_mul(height as usize)
            .and_then(|s| s.checked_mul(BYTES_PER_PIXEL))
            .expect("PixelBuffer dimensions overflow (width * height * 4 exceeds usize)");
        Self {
            width,
            height,
            data: vec![0u8; size],
        }
    }

    /// Create a pixel buffer from existing RGBA data.
    ///
    /// # Panics
    ///
    /// Panics if `data.len() != width * height * 4`.
    pub fn from_data(width: u32, height: u32, data: Vec<u8>) -> Self {
        let expected = (width as usize) * (height as usize) * BYTES_PER_PIXEL;
        assert_eq!(
            data.len(),
            expected,
            "PixelBuffer::from_data: expected {} bytes for {}×{} image, got {}",
            expected,
            width,
            height,
            data.len()
        );
        Self { width, height, data }
    }

    /// Read one pixel.  Returns (red, green, blue, alpha).
    ///
    /// # Panics
    ///
    /// Panics if `x >= width` or `y >= height`.
    pub fn pixel_at(&self, x: u32, y: u32) -> (u8, u8, u8, u8) {
        let offset = self.offset(x, y);
        (
            self.data[offset],
            self.data[offset + 1],
            self.data[offset + 2],
            self.data[offset + 3],
        )
    }

    /// Write one pixel.
    ///
    /// # Panics
    ///
    /// Panics if `x >= width` or `y >= height`.
    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        let offset = self.offset(x, y);
        self.data[offset] = r;
        self.data[offset + 1] = g;
        self.data[offset + 2] = b;
        self.data[offset + 3] = a;
    }

    /// Total number of pixels (width × height).
    pub fn pixel_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// Number of bytes in the data buffer (width × height × 4).
    pub fn byte_count(&self) -> usize {
        self.data.len()
    }

    /// Byte offset for pixel (x, y).
    ///
    /// The formula is `(y * width + x) * 4`.  This is the standard
    /// row-major packing: all pixels in row 0 come first, then row 1, etc.
    fn offset(&self, x: u32, y: u32) -> usize {
        assert!(x < self.width, "x={} out of bounds (width={})", x, self.width);
        assert!(y < self.height, "y={} out of bounds (height={})", y, self.height);
        ((y as usize) * (self.width as usize) + (x as usize)) * BYTES_PER_PIXEL
    }
}

// ---------------------------------------------------------------------------
// PixelEncoder — trait for image format encoders
// ---------------------------------------------------------------------------

/// Trait for image format encoders (PNG, JPEG, WebP, etc.).
///
/// Each encoder crate implements this trait.  The encoder takes a pixel
/// buffer and returns the encoded bytes in the target format.
///
/// This trait lives in `draw-instructions-pixels` so that encoder crates
/// only need one dependency and don't need to know about any renderer.
pub trait PixelEncoder {
    /// Encode a pixel buffer to the target image format.
    ///
    /// Returns the complete encoded file as a byte vector.  For PNG,
    /// this includes the PNG header, chunks, and IEND.  For JPEG,
    /// this includes the SOI marker through EOI.
    fn encode(&self, buffer: &PixelBuffer) -> Vec<u8>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn new_creates_zeroed_buffer() {
        let buf = PixelBuffer::new(3, 2);
        assert_eq!(buf.width, 3);
        assert_eq!(buf.height, 2);
        assert_eq!(buf.data.len(), 3 * 2 * 4);
        assert!(buf.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn from_data_accepts_correct_size() {
        let data = vec![0u8; 2 * 2 * 4];
        let buf = PixelBuffer::from_data(2, 2, data);
        assert_eq!(buf.pixel_count(), 4);
        assert_eq!(buf.byte_count(), 16);
    }

    #[test]
    #[should_panic(expected = "expected 16 bytes")]
    fn from_data_rejects_wrong_size() {
        let data = vec![0u8; 10];
        PixelBuffer::from_data(2, 2, data);
    }

    #[test]
    fn pixel_at_reads_correct_values() {
        //  2×1 image: pixel 0 is red, pixel 1 is blue
        let data = vec![
            255, 0, 0, 255, // (0,0) = red, opaque
            0, 0, 255, 255, // (1,0) = blue, opaque
        ];
        let buf = PixelBuffer::from_data(2, 1, data);
        assert_eq!(buf.pixel_at(0, 0), (255, 0, 0, 255));
        assert_eq!(buf.pixel_at(1, 0), (0, 0, 255, 255));
    }

    #[test]
    fn set_pixel_writes_correct_values() {
        let mut buf = PixelBuffer::new(2, 2);
        buf.set_pixel(1, 1, 128, 64, 32, 255);
        assert_eq!(buf.pixel_at(1, 1), (128, 64, 32, 255));
        // Other pixels should still be zero
        assert_eq!(buf.pixel_at(0, 0), (0, 0, 0, 0));
    }

    #[test]
    #[should_panic(expected = "x=2 out of bounds")]
    fn pixel_at_panics_on_x_out_of_bounds() {
        let buf = PixelBuffer::new(2, 2);
        buf.pixel_at(2, 0);
    }

    #[test]
    #[should_panic(expected = "y=2 out of bounds")]
    fn pixel_at_panics_on_y_out_of_bounds() {
        let buf = PixelBuffer::new(2, 2);
        buf.pixel_at(0, 2);
    }

    #[test]
    fn pixel_count_and_byte_count() {
        let buf = PixelBuffer::new(10, 20);
        assert_eq!(buf.pixel_count(), 200);
        assert_eq!(buf.byte_count(), 800);
    }

    #[test]
    fn zero_size_buffer() {
        let buf = PixelBuffer::new(0, 0);
        assert_eq!(buf.pixel_count(), 0);
        assert_eq!(buf.byte_count(), 0);
        assert!(buf.data.is_empty());
    }

    /// Verify row-major layout: pixel (1, 0) is at byte offset 4,
    /// pixel (0, 1) is at byte offset `width * 4`.
    #[test]
    fn row_major_layout() {
        let mut buf = PixelBuffer::new(3, 2);
        // Set pixel at (1, 0) — should be at byte offset 4
        buf.set_pixel(1, 0, 1, 2, 3, 4);
        assert_eq!(buf.data[4], 1);
        assert_eq!(buf.data[5], 2);
        assert_eq!(buf.data[6], 3);
        assert_eq!(buf.data[7], 4);

        // Set pixel at (0, 1) — should be at byte offset 3*4=12
        buf.set_pixel(0, 1, 10, 20, 30, 40);
        assert_eq!(buf.data[12], 10);
        assert_eq!(buf.data[13], 20);
        assert_eq!(buf.data[14], 30);
        assert_eq!(buf.data[15], 40);
    }
}
