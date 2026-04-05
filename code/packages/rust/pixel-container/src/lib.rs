// # pixel-container
//
// A zero-dependency crate that owns the two types every image codec needs:
//
// - `PixelContainer` — a flat RGBA8 pixel buffer (the universal interchange
//   format between GPU renderers and image encoders)
// - `ImageCodec` — a trait for encode/decode operations over `PixelContainer`
//
// ## Why a Separate Crate?
//
// Image codecs (BMP, PPM, QOI, PNG, JPEG…) should not depend on the paint
// stack. A JPEG decoder has no business knowing what a `PaintScene` or
// `PaintInstruction` is. By putting `PixelContainer` here, every codec can
// depend only on this crate — zero rendering types in scope.
//
// `paint-instructions` re-exports both types from here, so all existing code
// that imports `paint_instructions::PixelContainer` continues to compile
// unchanged.
//
// ## Pixel Layout
//
// The buffer stores 4 bytes per pixel in RGBA order (red, green, blue, alpha),
// row-major from top-left:
//
//   offset = (y * width + x) * 4
//   data[offset + 0] = R
//   data[offset + 1] = G
//   data[offset + 2] = B
//   data[offset + 3] = A
//
// A fully opaque pixel has A = 255. A fully transparent pixel has A = 0; its
// RGB bytes are conventionally zero but are not meaningful when A = 0.

// ---------------------------------------------------------------------------
// PixelContainer
// ---------------------------------------------------------------------------

/// A flat, row-major RGBA8 pixel buffer.
///
/// This is the universal interchange type between renderers and image codecs.
/// A Metal renderer produces a `PixelContainer`. A JPEG decoder produces a
/// `PixelContainer`. Codecs receive one and serialise it — they never see
/// `PaintScene` or any rendering concept.
///
/// # Pixel Layout
///
/// ```text
/// offset = (y * width + x) * 4
///
/// data[offset + 0] = R
/// data[offset + 1] = G
/// data[offset + 2] = B
/// data[offset + 3] = A
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PixelContainer {
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
    /// Raw RGBA8 pixel data, row-major, top-left origin.
    /// Length is always `width * height * 4`.
    pub data: Vec<u8>,
}

impl PixelContainer {
    /// Create a blank (all-zero, fully transparent) buffer of the given size.
    ///
    /// Every pixel starts as (R=0, G=0, B=0, A=0). Use `set_pixel` to fill in
    /// values, or use `from_data` if you already have a pixel buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pixel_container::PixelContainer;
    ///
    /// let buf = PixelContainer::new(4, 4);
    /// assert_eq!(buf.width, 4);
    /// assert_eq!(buf.height, 4);
    /// assert_eq!(buf.data.len(), 4 * 4 * 4);
    /// ```
    pub fn new(width: u32, height: u32) -> Self {
        // Use usize arithmetic throughout to prevent u32 overflow in release mode.
        // width * height * 4 can exceed u32::MAX for large images (e.g. 46341×46341).
        let size = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .expect("PixelContainer dimensions overflow usize");
        Self {
            width,
            height,
            data: vec![0u8; size],
        }
    }

    /// Create a `PixelContainer` from an existing pixel buffer.
    ///
    /// # Panics
    ///
    /// Panics if `data.len() != width * height * 4`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pixel_container::PixelContainer;
    ///
    /// // Create a 1×1 fully opaque red pixel.
    /// let p = PixelContainer::from_data(1, 1, vec![255, 0, 0, 255]);
    /// assert_eq!(p.pixel_at(0, 0), (255, 0, 0, 255));
    /// ```
    pub fn from_data(width: u32, height: u32, data: Vec<u8>) -> Self {
        // Use usize arithmetic to prevent u32 overflow in release mode.
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .expect("PixelContainer dimensions overflow usize");
        assert_eq!(
            data.len(),
            expected,
            "PixelContainer::from_data: data.len()={} but width*height*4={}",
            data.len(),
            expected
        );
        Self { width, height, data }
    }

    /// Read the RGBA components of the pixel at `(x, y)`.
    ///
    /// Returns `(0, 0, 0, 0)` if the coordinates are out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use pixel_container::PixelContainer;
    ///
    /// let mut buf = PixelContainer::new(2, 2);
    /// buf.set_pixel(1, 0, 200, 100, 50, 255);
    /// assert_eq!(buf.pixel_at(1, 0), (200, 100, 50, 255));
    /// assert_eq!(buf.pixel_at(99, 99), (0, 0, 0, 0)); // out of bounds
    /// ```
    pub fn pixel_at(&self, x: u32, y: u32) -> (u8, u8, u8, u8) {
        if x >= self.width || y >= self.height {
            return (0, 0, 0, 0);
        }
        // Use usize arithmetic to avoid u32 overflow on large images in release mode.
        let i = (y as usize * self.width as usize + x as usize) * 4;
        (self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3])
    }

    /// The number of pixels in the buffer (`width * height`).
    ///
    /// # Examples
    ///
    /// ```
    /// use pixel_container::PixelContainer;
    ///
    /// let buf = PixelContainer::new(4, 3);
    /// assert_eq!(buf.pixel_count(), 12);
    /// ```
    pub fn pixel_count(&self) -> usize {
        self.width as usize * self.height as usize
    }

    /// The number of bytes in the backing buffer (`width * height * 4`).
    ///
    /// # Examples
    ///
    /// ```
    /// use pixel_container::PixelContainer;
    ///
    /// let buf = PixelContainer::new(4, 3);
    /// assert_eq!(buf.byte_count(), 48);
    /// ```
    pub fn byte_count(&self) -> usize {
        self.data.len()
    }

    /// Write the RGBA components of the pixel at `(x, y)`.
    ///
    /// No-op if the coordinates are out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use pixel_container::PixelContainer;
    ///
    /// let mut buf = PixelContainer::new(3, 3);
    /// buf.set_pixel(1, 1, 0, 128, 255, 200);
    /// assert_eq!(buf.pixel_at(1, 1), (0, 128, 255, 200));
    /// ```
    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        // Use usize arithmetic to avoid u32 overflow on large images in release mode.
        let i = (y as usize * self.width as usize + x as usize) * 4;
        self.data[i]     = r;
        self.data[i + 1] = g;
        self.data[i + 2] = b;
        self.data[i + 3] = a;
    }

    /// Fill the entire buffer with a single RGBA colour.
    ///
    /// Useful for clearing a canvas before rendering.
    ///
    /// # Examples
    ///
    /// ```
    /// use pixel_container::PixelContainer;
    ///
    /// let mut buf = PixelContainer::new(4, 4);
    /// buf.fill(255, 255, 255, 255); // white
    /// assert_eq!(buf.pixel_at(2, 2), (255, 255, 255, 255));
    /// ```
    pub fn fill(&mut self, r: u8, g: u8, b: u8, a: u8) {
        // Write the RGBA pattern into every four-byte group.
        for chunk in self.data.chunks_exact_mut(4) {
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = a;
        }
    }
}

// ---------------------------------------------------------------------------
// ImageCodec
// ---------------------------------------------------------------------------

/// A trait for image format encode/decode operations over `PixelContainer`.
///
/// Implementing this trait is all a codec needs to participate in the codec
/// pipeline. Codecs are composable:
///
/// ```text
/// jpg_bytes → JpegCodec::decode() → PixelContainer → PngCodec::encode() → png_bytes
/// ```
///
/// No rendering types are needed. A codec only speaks `PixelContainer`.
///
/// # Example Implementation Sketch
///
/// ```rust,ignore
/// use pixel_container::{ImageCodec, PixelContainer};
///
/// pub struct FooCodec;
///
/// impl ImageCodec for FooCodec {
///     fn mime_type(&self) -> &'static str { "image/x-foo" }
///
///     fn encode(&self, container: &PixelContainer) -> Vec<u8> {
///         // serialise pixels to foo format
///         todo!()
///     }
///
///     fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
///         // parse foo format into pixel buffer
///         todo!()
///     }
/// }
/// ```
pub trait ImageCodec {
    /// The IANA MIME type for this format, e.g. `"image/png"`.
    fn mime_type(&self) -> &'static str;

    /// Encode a pixel buffer into the bytes of this format.
    ///
    /// The output is a complete, self-contained file — ready to write to disk
    /// or send over the network.
    fn encode(&self, container: &PixelContainer) -> Vec<u8>;

    /// Decode bytes in this format into a pixel buffer.
    ///
    /// Returns `Err` with a human-readable message if the bytes are not valid
    /// for this format.
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- PixelContainer::new ---

    #[test]
    fn new_allocates_correct_size() {
        let buf = PixelContainer::new(10, 20);
        assert_eq!(buf.width, 10);
        assert_eq!(buf.height, 20);
        assert_eq!(buf.data.len(), 10 * 20 * 4);
    }

    #[test]
    fn new_is_all_zeros() {
        let buf = PixelContainer::new(5, 5);
        assert!(buf.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn new_zero_dimensions() {
        // A 0×0 container is valid; just has no pixels.
        let buf = PixelContainer::new(0, 0);
        assert_eq!(buf.data.len(), 0);
    }

    // --- PixelContainer::from_data ---

    #[test]
    fn from_data_round_trip() {
        let data = vec![255u8, 128, 64, 32];
        let buf = PixelContainer::from_data(1, 1, data.clone());
        assert_eq!(buf.data, data);
    }

    #[test]
    #[should_panic]
    fn from_data_wrong_length_panics() {
        // data.len() = 3, expected = 1*1*4 = 4 → should panic
        PixelContainer::from_data(1, 1, vec![1, 2, 3]);
    }

    // --- pixel_at / set_pixel ---

    #[test]
    fn set_and_get_pixel() {
        let mut buf = PixelContainer::new(4, 4);
        buf.set_pixel(2, 3, 10, 20, 30, 40);
        assert_eq!(buf.pixel_at(2, 3), (10, 20, 30, 40));
    }

    #[test]
    fn pixel_at_out_of_bounds_returns_zero() {
        let buf = PixelContainer::new(4, 4);
        assert_eq!(buf.pixel_at(4, 0), (0, 0, 0, 0));
        assert_eq!(buf.pixel_at(0, 4), (0, 0, 0, 0));
        assert_eq!(buf.pixel_at(100, 100), (0, 0, 0, 0));
    }

    #[test]
    fn set_pixel_out_of_bounds_is_noop() {
        let mut buf = PixelContainer::new(4, 4);
        // This should not panic.
        buf.set_pixel(10, 10, 255, 255, 255, 255);
        // Original data is unchanged.
        assert!(buf.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn pixel_offset_is_correct() {
        // Verify offset = (y * width + x) * 4
        let mut buf = PixelContainer::new(5, 5);
        buf.set_pixel(3, 2, 1, 2, 3, 4); // offset = (2*5+3)*4 = 52
        assert_eq!(buf.data[52], 1);
        assert_eq!(buf.data[53], 2);
        assert_eq!(buf.data[54], 3);
        assert_eq!(buf.data[55], 4);
    }

    // --- fill ---

    #[test]
    fn fill_sets_all_pixels() {
        let mut buf = PixelContainer::new(3, 3);
        buf.fill(255, 128, 0, 255);
        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(buf.pixel_at(x, y), (255, 128, 0, 255));
            }
        }
    }

    // --- clone and equality ---

    #[test]
    fn clone_is_independent() {
        let mut original = PixelContainer::new(2, 2);
        original.set_pixel(0, 0, 1, 2, 3, 4);
        let mut clone = original.clone();
        clone.set_pixel(0, 0, 99, 99, 99, 99);
        // Original is unchanged.
        assert_eq!(original.pixel_at(0, 0), (1, 2, 3, 4));
    }

    #[test]
    fn equality_compares_all_fields() {
        let a = PixelContainer::from_data(1, 1, vec![1, 2, 3, 4]);
        let b = PixelContainer::from_data(1, 1, vec![1, 2, 3, 4]);
        let c = PixelContainer::from_data(1, 1, vec![1, 2, 3, 5]); // different alpha
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // --- ImageCodec trait usage ---

    // A minimal stub codec for testing the trait interface without any real
    // format logic. It encodes by prepending a 4-byte header [w, h, 0, 0] and
    // then storing raw RGBA bytes.
    struct StubCodec;
    impl ImageCodec for StubCodec {
        fn mime_type(&self) -> &'static str {
            "image/x-stub"
        }
        fn encode(&self, c: &PixelContainer) -> Vec<u8> {
            let mut out = vec![c.width as u8, c.height as u8, 0u8, 0u8];
            out.extend_from_slice(&c.data);
            out
        }
        fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
            if bytes.len() < 4 {
                return Err("stub: too short".into());
            }
            let w = bytes[0] as u32;
            let h = bytes[1] as u32;
            let data = bytes[4..].to_vec();
            Ok(PixelContainer::from_data(w, h, data))
        }
    }

    #[test]
    fn codec_mime_type() {
        assert_eq!(StubCodec.mime_type(), "image/x-stub");
    }

    #[test]
    fn codec_round_trip() {
        let mut original = PixelContainer::new(2, 1);
        original.set_pixel(0, 0, 10, 20, 30, 40);
        original.set_pixel(1, 0, 50, 60, 70, 80);

        let encoded = StubCodec.encode(&original);
        let decoded = StubCodec.decode(&encoded).unwrap();

        assert_eq!(decoded.width, original.width);
        assert_eq!(decoded.height, original.height);
        assert_eq!(decoded.data, original.data);
    }

    #[test]
    fn codec_decode_error() {
        let result = StubCodec.decode(&[]);
        assert!(result.is_err());
    }
}
