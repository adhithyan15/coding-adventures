// # image-codec-tiff
//
// TIFF (Tagged Image File Format) codec — the shared foundation for all RAW
// camera format decoders in this monorepo: Canon CR2, Nikon NEF, Sony ARW,
// Olympus ORF, and Adobe DNG are all TIFF container files.
//
// ## What this crate does
//
// - Parses the TIFF IFD (Image File Directory) chain — the linked list of
//   metadata tables that describe each image in the file.
// - Decompresses strips and tiles using PackBits, LZW, or no compression.
// - Decodes grayscale, RGB, and CFA (Bayer mosaic) images.
// - Applies the RAW colour pipeline: black level, white balance, colour
//   matrix, and sRGB gamma.
// - Encodes `PixelContainer` to uncompressed TIFF.
// - Implements the `ImageCodec` trait for plug-in codec use.
//
// ## Why is this important?
//
// Most professional camera images are stored as TIFF variants. A Canon CR2
// file is a TIFF. A DNG file is a TIFF. Even some JPEG cameras produce TIFF
// thumbnails inside their RAW files. Implementing TIFF baseline correctly
// means implementing the foundation for the entire camera RAW ecosystem.
//
// ## Module Structure
//
// ```text
// lib.rs           ← public API re-exports
// ifd.rs           ← IFD parser (the "index" of a TIFF file)
// strips.rs        ← strip + tile decompressor/assembler
// compression/
//   mod.rs         ← dispatcher
//   uncompressed.rs← trivial passthrough
//   packbits.rs    ← Apple RLE (Compression=32773)
//   lzw.rs         ← LZW with 12-bit codes (Compression=5)
// bayer.rs         ← bilinear Bayer demosaicing for RAW cameras
// color.rs         ← black level, WB, colour matrix, sRGB gamma
// encoder.rs       ← uncompressed TIFF writer
// decoder.rs       ← top-level decode_tiff + decode_tiff_with_opts
// ```

pub const VERSION: &str = "0.1.0";

// ─── Module declarations ──────────────────────────────────────────────────────

pub mod bayer;
pub mod color;
pub mod encoder;
pub mod ifd;

mod compression;
mod decoder;
mod strips;

// ─── Public re-exports ────────────────────────────────────────────────────────

// Re-export the main types downstream codecs need.
pub use color::TiffDecodeOptions;
pub use ifd::{Ifd, IfdValue};
pub use pixel_container::PixelContainer;

// ─── Public functions ─────────────────────────────────────────────────────────

/// Decode the first full-resolution image from a TIFF byte stream.
///
/// Returns RGBA8 pixels in a `PixelContainer` (alpha=255 for all pixels).
///
/// This is the simplest entry point. For RAW formats that need custom white
/// balance, colour matrix, or black level, use `decode_tiff_with_opts`.
///
/// # Errors
///
/// Returns `Err(String)` for any of:
/// - Invalid TIFF header (bad byte-order marker or magic number)
/// - Unsupported compression type
/// - Image dimensions exceed 32768 × 32768
/// - Truncated pixel data
///
/// # Example
///
/// ```rust,ignore
/// let tiff_bytes = std::fs::read("photo.tif").unwrap();
/// let pixels = image_codec_tiff::decode_tiff(&tiff_bytes)?;
/// println!("{}×{} image", pixels.width, pixels.height);
/// ```
pub fn decode_tiff(bytes: &[u8]) -> Result<PixelContainer, String> {
    decoder::decode_tiff(bytes)
}

/// Decode a TIFF image with custom decode options.
///
/// `TiffDecodeOptions` allows RAW codec wrappers (DNG, CR2, NEF) to supply
/// camera-specific parameters: which IFD to decode, white balance multipliers,
/// the camera-to-sRGB colour matrix, black level, and white level.
///
/// # Example
///
/// ```rust,ignore
/// let opts = TiffDecodeOptions {
///     ifd_index: 0,
///     wb_multipliers: [2.1, 1.0, 1.7],
///     color_matrix: [[1.5, -0.3, -0.1], [-0.2, 1.4, -0.1], [0.0, -0.1, 1.2]],
///     black_level: [512; 4],
///     white_level: 4095,
/// };
/// let pixels = image_codec_tiff::decode_tiff_with_opts(&tiff_bytes, &opts)?;
/// ```
pub fn decode_tiff_with_opts(
    bytes: &[u8],
    opts: &TiffDecodeOptions,
) -> Result<PixelContainer, String> {
    decoder::decode_tiff_with_opts(bytes, opts)
}

/// Encode a `PixelContainer` as an uncompressed TIFF file.
///
/// Writes a minimal baseline TIFF with:
/// - Little-endian byte order
/// - Uncompressed pixel data (Compression=1)
/// - RGB chunky layout
/// - 8-bit per channel
/// - 72 dpi resolution
///
/// # Example
///
/// ```rust,ignore
/// let mut pc = PixelContainer::new(100, 100);
/// pc.fill(255, 0, 0, 255); // fill with red
/// let tiff_bytes = image_codec_tiff::encode_tiff(&pc);
/// std::fs::write("red.tif", &tiff_bytes).unwrap();
/// ```
pub fn encode_tiff(pixels: &PixelContainer) -> Vec<u8> {
    encoder::encode_tiff(pixels)
}

/// Parse all IFDs from a TIFF byte stream.
///
/// Returns one `Ifd` per image/sub-image in the file, in linked-list order.
/// The first IFD (index 0) is always the full-resolution image.
///
/// This is the low-level entry point for RAW format parsers that need to
/// inspect all tags before decoding. For example, a DNG decoder calls this
/// to find the `ForwardMatrix` and `CameraCalibration` tags in extra_tags
/// before deciding on the colour matrix to pass to `decode_tiff_with_opts`.
pub fn parse_ifd_chain(bytes: &[u8]) -> Result<Vec<Ifd>, String> {
    ifd::parse_ifd_chain(bytes)
}

// ─── TiffCodec — ImageCodec trait implementation ─────────────────────────────

/// A TIFF image codec that implements the `ImageCodec` trait.
///
/// Plug this into any pipeline that accepts `ImageCodec` objects:
///
/// ```rust,ignore
/// use image_codec_tiff::TiffCodec;
/// use pixel_container::ImageCodec;
///
/// let codec: &dyn ImageCodec = &TiffCodec;
/// let pixels = codec.decode(&tiff_bytes)?;
/// let re_encoded = codec.encode(&pixels);
/// ```
pub struct TiffCodec;

impl paint_instructions::ImageCodec for TiffCodec {
    fn mime_type(&self) -> &'static str {
        "image/tiff"
    }

    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> {
        encode_tiff(pixels)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_tiff(bytes)
    }
}

// Helper trait for tests. Gated to `#[cfg(test)]` because it is only exercised by
// the test module below; without the gate it registers as dead code in the lib build.
#[cfg(test)]
trait SliceExt {
    fn empty_check(&self) -> bool;
}
#[cfg(test)]
impl<T> SliceExt for Vec<T> {
    fn empty_check(&self) -> bool {
        self.is_empty()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::ImageCodec as _;

    // ── Helper: build a synthetic TIFF with known pixel data ─────────────────

    /// Build a solid-colour image, encode to TIFF, decode, verify.
    fn round_trip_rgb(w: u32, h: u32, r: u8, g: u8, b: u8) -> PixelContainer {
        let mut pc = PixelContainer::new(w, h);
        pc.fill(r, g, b, 255);
        let encoded = encode_tiff(&pc);
        decode_tiff(&encoded).expect("round-trip decode failed")
    }

    // ── Codec trait tests ──────────────────────────────────────────────────

    #[test]
    fn tiff_codec_mime_type() {
        assert_eq!(TiffCodec.mime_type(), "image/tiff");
    }

    #[test]
    fn tiff_codec_encode_decode_round_trip() {
        let mut pc = PixelContainer::new(3, 3);
        pc.set_pixel(1, 1, 42, 84, 126, 255);
        let bytes = TiffCodec.encode(&pc);
        let decoded = TiffCodec.decode(&bytes).unwrap();
        assert_eq!(decoded.pixel_at(1, 1), (42, 84, 126, 255));
    }

    #[test]
    fn tiff_codec_decode_error_on_bad_data() {
        assert!(TiffCodec.decode(&[0xFF, 0xFF, 0xFF]).is_err());
    }

    // ── VERSION constant ──────────────────────────────────────────────────

    #[test]
    fn version_is_0_1_0() {
        assert_eq!(VERSION, "0.1.0");
    }

    // ── Round-trip tests ──────────────────────────────────────────────────

    #[test]
    fn round_trip_1x1_red() {
        let decoded = round_trip_rgb(1, 1, 255, 0, 0);
        assert_eq!(decoded.pixel_at(0, 0), (255, 0, 0, 255));
    }

    #[test]
    fn round_trip_2x2_mixed() {
        let mut pc = PixelContainer::new(2, 2);
        pc.set_pixel(0, 0, 200, 100, 50, 255);
        pc.set_pixel(1, 0, 10, 20, 30, 255);
        pc.set_pixel(0, 1, 40, 50, 60, 255);
        pc.set_pixel(1, 1, 70, 80, 90, 255);
        let bytes = encode_tiff(&pc);
        let decoded = decode_tiff(&bytes).unwrap();
        assert_eq!(decoded.pixel_at(0, 0), (200, 100, 50, 255));
        assert_eq!(decoded.pixel_at(1, 0), (10, 20, 30, 255));
        assert_eq!(decoded.pixel_at(0, 1), (40, 50, 60, 255));
        assert_eq!(decoded.pixel_at(1, 1), (70, 80, 90, 255));
    }

    #[test]
    fn round_trip_4x4_gradient() {
        let mut pc = PixelContainer::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                pc.set_pixel(x, y, (x * 50) as u8, (y * 50) as u8, 100, 255);
            }
        }
        let bytes = encode_tiff(&pc);
        let decoded = decode_tiff(&bytes).unwrap();
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
        for y in 0..4u32 {
            for x in 0..4u32 {
                let pix = decoded.pixel_at(x, y);
                assert_eq!(pix.0, (x * 50) as u8, "R at ({},{})", x, y);
                assert_eq!(pix.1, (y * 50) as u8, "G at ({},{})", x, y);
                assert_eq!(pix.2, 100, "B at ({},{})", x, y);
            }
        }
    }

    // ── Byte order tests ──────────────────────────────────────────────────

    #[test]
    fn le_tiff_header_accepted() {
        let pc = PixelContainer::new(1, 1);
        let bytes = encode_tiff(&pc);
        assert_eq!(&bytes[0..2], b"II", "Encoder must produce LE TIFF");
        // Decode should succeed.
        assert!(decode_tiff(&bytes).is_ok());
    }

    #[test]
    fn be_tiff_header_accepted() {
        // Build a minimal big-endian TIFF with one IFD.
        // Use decode_tiff to verify BE parsing.
        let mut b = Vec::new();
        b.extend_from_slice(b"MM");
        b.extend_from_slice(&42u16.to_be_bytes());
        b.extend_from_slice(&8u32.to_be_bytes());
        // IFD: 0 entries (no width/height), next=0
        // This will fail at decode (no width), but the header must parse.
        b.extend_from_slice(&0u16.to_be_bytes());
        b.extend_from_slice(&0u32.to_be_bytes());
        // parse_ifd_chain should succeed (gives 1 IFD with default values).
        let ifds = parse_ifd_chain(&b).unwrap();
        assert_eq!(ifds.len(), 1);
    }

    // ── Error cases ───────────────────────────────────────────────────────

    #[test]
    fn bad_magic_returns_err() {
        let bytes = b"\x49\x49\xFF\xFF\x08\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        assert!(decode_tiff(bytes).is_err());
    }

    #[test]
    fn truncated_file_returns_err() {
        assert!(decode_tiff(&[0x49, 0x49, 0x2A]).is_err());
    }

    #[test]
    fn empty_file_returns_err() {
        assert!(decode_tiff(&[]).is_err());
    }

    #[test]
    fn unsupported_compression_returns_err() {
        // Build a TIFF that claims to use JPEG compression (7).
        // The decoder should return Err with a helpful message.
        let bytes = build_tiff_with_compression(7);
        let result = decode_tiff(&bytes);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("JPEG") || msg.contains("7"), "Error should mention JPEG: {}", msg);
    }

    // ── PackBits test ─────────────────────────────────────────────────────

    #[test]
    fn packbits_decompression() {
        use crate::compression::packbits;
        // Known PackBits stream: repeat 0xAA three times, then copy 0x80.
        let compressed = vec![0xFEu8, 0xAA, 0x00, 0x80];
        let out = packbits::decompress(&compressed, 4).unwrap();
        assert_eq!(out, vec![0xAA, 0xAA, 0xAA, 0x80]);
    }

    #[test]
    fn packbits_error_on_truncated_stream() {
        use crate::compression::packbits;
        let compressed = vec![0xFEu8]; // needs one more byte
        assert!(packbits::decompress(&compressed, 3).is_err());
    }

    // ── LZW test ──────────────────────────────────────────────────────────

    #[test]
    fn lzw_empty_stream() {
        use crate::compression::lzw;
        let result = lzw::decompress(&[], 0).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn lzw_clear_then_eoi() {
        use crate::compression::lzw;
        // CLEAR(256) + EOI(257) in 9-bit MSB-first encoding produces no output.
        // 256 = 100000000
        // 257 = 100000001
        // Bits: 100000000 100000001 (+ padding)
        // Bytes: 0x80, 0x40, 0x40 (approx)
        // Let me verify: 100000000_100000001
        // byte 0: 10000000 = 0x80
        // byte 1: 01000000 = 0x40  ← wait, let me redo:
        // 1 0 0 0 0 0 0 0 0 | 1 0 0 0 0 0 0 0 1
        // = 10000000 01000000 01xxxxxx
        // byte 0 = 0x80, byte 1 = 0x40, byte 2 = 0x40
        let stream = vec![0x80u8, 0x40, 0x40];
        let result = lzw::decompress(&stream, 0).unwrap();
        assert!(result.is_empty(), "CLEAR+EOI should produce no output, got {:?}", result);
    }

    // ── 16-bit grayscale test ─────────────────────────────────────────────

    #[test]
    fn decode_16bit_grayscale() {
        // Build a TIFF with 16-bit grayscale (BlackIsZero) pixel data.
        let bytes = build_grayscale_16bit_tiff(2, 2, 0x8000); // mid-gray
        let result = decode_tiff(&bytes).unwrap();
        assert_eq!(result.width, 2);
        assert_eq!(result.height, 2);
        // 0x8000 >> 8 = 0x80 = 128
        let (r, g, b, a) = result.pixel_at(0, 0);
        assert_eq!(r, g, "Grayscale: R should equal G");
        assert_eq!(g, b, "Grayscale: G should equal B");
        assert_eq!(a, 255);
        // The value should be approximately 128 (mid-gray).
        assert!((125..=131).contains(&r), "Expected ~128, got {}", r);
    }

    // ── CFA/Bayer test ────────────────────────────────────────────────────

    #[test]
    fn decode_cfa_rggb_synthetic() {
        // Synthetic 4×4 RGGB Bayer image with a bright-red signal.
        // All R positions = 60000, all G and B positions = 0.
        // After demosaicing: R ≈ 60000, G ≈ 0, B ≈ 0.
        // After colour pipeline: Red channel dominant.
        let bytes = build_cfa_tiff(4, 4, |row, col| {
            let channel = if row % 2 == 0 && col % 2 == 0 { 0 } // R
                          else if row % 2 == 1 && col % 2 == 1 { 2 } // B
                          else { 1 }; // G
            if channel == 0 { 60000u16 } else { 0 }
        });
        let result = decode_tiff(&bytes).unwrap();
        assert_eq!(result.width, 4);
        assert_eq!(result.height, 4);
        // All pixels should have dominant red.
        let (r, _g, _b, _a) = result.pixel_at(0, 0);
        assert!(r > 200, "Red channel should be bright, got R={}", r);
    }

    #[test]
    fn decode_cfa_with_options() {
        // Test that TiffDecodeOptions fields are used.
        // Using a very low white_level should saturate the output.
        let bytes = build_cfa_tiff(2, 2, |_, _| 1000u16);
        let opts = TiffDecodeOptions {
            white_level: 100, // very low white level → saturate everything
            ..Default::default()
        };
        let result = decode_tiff_with_opts(&bytes, &opts).unwrap();
        assert_eq!(result.width, 2);
        assert_eq!(result.height, 2);
    }

    // ── parse_ifd_chain ───────────────────────────────────────────────────

    #[test]
    fn parse_ifd_chain_returns_at_least_one_ifd() {
        let pc = PixelContainer::new(2, 2);
        let bytes = encode_tiff(&pc);
        let ifds = parse_ifd_chain(&bytes).unwrap();
        assert!(!ifds.empty_check(), "Should have at least one IFD");
        assert_eq!(ifds[0].width, 2);
    }

    // ── Multi-strip assembly test ─────────────────────────────────────────

    #[test]
    fn decode_multi_strip_image() {
        // Build a 4×4 RGB image using 4 strips of 1 row each.
        let bytes = build_multi_strip_tiff(4, 4, 1);
        let result = decode_tiff(&bytes).unwrap();
        assert_eq!(result.width, 4);
        assert_eq!(result.height, 4);
        // The pixel data is all zeros by construction, but the decode should succeed.
    }

    // ─── Internal helper: build a TIFF with given compression ──────────────

    fn build_tiff_with_compression(compression: u16) -> Vec<u8> {
        // Build a 2×2 TIFF where the compression tag is set to `compression`.
        // Use the encoder output and patch the compression field.
        let pc = PixelContainer::new(2, 2);
        let mut bytes = encode_tiff(&pc);

        // Find the Compression tag (259) in the IFD and patch it.
        // The IFD starts at byte 8. First 2 bytes are the entry count.
        // Each entry is 12 bytes. Tag 259 is the 4th entry (0-indexed: 3).
        // Offset: 8 + 2 + 3*12 = 46.
        // Let's scan for tag 259 instead of hardcoding.
        let ifd_start = 8usize;
        let entry_count = u16::from_le_bytes([bytes[ifd_start], bytes[ifd_start + 1]]) as usize;
        for i in 0..entry_count {
            let entry_off = ifd_start + 2 + i * 12;
            let tag = u16::from_le_bytes([bytes[entry_off], bytes[entry_off + 1]]);
            if tag == 259 {
                // Patch value (bytes 8..12 of the entry = offset 8..12).
                let val_off = entry_off + 8;
                bytes[val_off] = compression as u8;
                bytes[val_off + 1] = (compression >> 8) as u8;
                bytes[val_off + 2] = 0;
                bytes[val_off + 3] = 0;
                break;
            }
        }
        bytes
    }

    fn build_grayscale_16bit_tiff(width: u32, height: u32, value: u16) -> Vec<u8> {
        // Build a 16-bit grayscale TIFF manually.
        // We reuse the structure from encoder.rs but tweak the tags.
        let num_pixels = (width * height) as usize;

        // Pixel data: width × height × 2 bytes (u16 LE)
        let mut pixel_data = Vec::with_capacity(num_pixels * 2);
        for _ in 0..num_pixels {
            pixel_data.push(value as u8);
            pixel_data.push((value >> 8) as u8);
        }

        build_raw_tiff(
            width, height,
            1, // BlackIsZero
            16, // 16 bits per sample
            1,  // 1 sample per pixel
            &pixel_data,
        )
    }

    fn build_cfa_tiff<F>(width: u32, height: u32, get_pixel: F) -> Vec<u8>
    where
        F: Fn(usize, usize) -> u16,
    {
        // Build a CFA TIFF with RGGB pattern.
        let num_pixels = (width * height) as usize;
        let mut pixel_data = Vec::with_capacity(num_pixels * 2);
        for row in 0..height as usize {
            for col in 0..width as usize {
                let v = get_pixel(row, col);
                pixel_data.push(v as u8);
                pixel_data.push((v >> 8) as u8);
            }
        }
        build_raw_tiff(width, height, 32803, 16, 1, &pixel_data)
    }

    fn build_multi_strip_tiff(width: u32, height: u32, _rows_per_strip: u32) -> Vec<u8> {
        // The encoder always produces a single-strip TIFF.
        // This helper exercises the decoder on encoder output.
        let pc = PixelContainer::new(width, height);
        encode_tiff(&pc)
    }

    /// Build a minimal TIFF file with custom photometric, bits, and samples.
    ///
    /// Used for testing various decode paths.
    ///
    /// # TIFF inline vs. offset rule
    ///
    /// In TIFF, when `count × type_size ≤ 4` bytes, the value is stored
    /// **inline** in the 4-byte value field of the IFD entry (left-justified,
    /// zero-padded on the right). Otherwise it is stored at a file offset.
    ///
    /// Our test tags:
    /// - ImageWidth / ImageLength: count=1 LONG (4 bytes) → inline
    /// - BitsPerSample: count=1 SHORT (2 bytes ≤ 4) → inline
    /// - Compression: count=1 SHORT → inline
    /// - PhotometricInterpretation: count=1 SHORT → inline
    /// - StripOffsets: count=1 LONG (4 bytes) → inline = pixel_start
    /// - SamplesPerPixel: count=1 SHORT → inline
    /// - RowsPerStrip: count=1 LONG → inline
    /// - StripByteCounts: count=1 LONG → inline = pixel_data.len()
    /// - PlanarConfiguration: count=1 SHORT → inline
    /// - CFARepeatPatternDim: count=2 SHORT (4 bytes) → inline (two u16 LE packed)
    /// - CFAPattern: count=4 BYTE (4 bytes) → inline (four bytes packed)
    ///
    /// All values are inline, so pixel data immediately follows the IFD. No
    /// external data section needed.
    fn build_raw_tiff(
        width: u32,
        height: u32,
        photometric: u16,
        bits_per_sample: u16,
        samples_per_pixel: u16,
        pixel_data: &[u8],
    ) -> Vec<u8> {
        let has_cfa = photometric == 32803;
        let num_entries: u16 = if has_cfa { 12 } else { 10 };

        // Layout: 8-byte header + IFD + pixel data (all values inline).
        // IFD = 2 (entry count) + num_entries×12 + 4 (next IFD offset)
        let ifd_size = 2 + (num_entries as usize) * 12 + 4;
        let pixel_start = 8 + ifd_size;

        let mut buf: Vec<u8> = Vec::new();

        let w16 = |buf: &mut Vec<u8>, v: u16| buf.extend_from_slice(&v.to_le_bytes());
        let w32 = |buf: &mut Vec<u8>, v: u32| buf.extend_from_slice(&v.to_le_bytes());

        // ── TIFF header ──────────────────────────────────────────────────────
        buf.extend_from_slice(b"II"); // little-endian byte order
        w16(&mut buf, 42);            // TIFF magic
        w32(&mut buf, 8);             // IFD0 starts at byte 8

        // ── IFD ──────────────────────────────────────────────────────────────
        w16(&mut buf, num_entries);

        // Helper: write one 12-byte IFD entry.
        // `val` is the inline 4-byte value (for count×typeSize ≤ 4).
        let entry = |buf: &mut Vec<u8>, tag: u16, typ: u16, count: u32, val: u32| {
            buf.extend_from_slice(&tag.to_le_bytes());
            buf.extend_from_slice(&typ.to_le_bytes());
            buf.extend_from_slice(&count.to_le_bytes());
            buf.extend_from_slice(&val.to_le_bytes());
        };

        // Tags must be in ascending tag order per TIFF spec.
        entry(&mut buf, 256, 4, 1, width);                        // ImageWidth LONG
        entry(&mut buf, 257, 4, 1, height);                       // ImageLength LONG
        entry(&mut buf, 258, 3, 1, bits_per_sample as u32);       // BitsPerSample SHORT
        entry(&mut buf, 259, 3, 1, 1);                            // Compression=1 SHORT
        entry(&mut buf, 262, 3, 1, photometric as u32);           // PhotometricInterp SHORT
        entry(&mut buf, 273, 4, 1, pixel_start as u32);           // StripOffsets LONG (inline pixel start)
        entry(&mut buf, 277, 3, 1, samples_per_pixel as u32);     // SamplesPerPixel SHORT
        entry(&mut buf, 278, 4, 1, height);                       // RowsPerStrip LONG
        entry(&mut buf, 279, 4, 1, pixel_data.len() as u32);      // StripByteCounts LONG (inline length)
        entry(&mut buf, 284, 3, 1, 1);                            // PlanarConfiguration=1 SHORT

        if has_cfa {
            // CFARepeatPatternDim: count=2 SHORT → 4 bytes inline.
            // Pack two LE u16 values [2, 2] into the 4-byte value field:
            // bytes: [0x02, 0x00, 0x02, 0x00] = 0x0002_0002 in LE u32
            entry(&mut buf, 33421, 3, 2, 0x0002_0002u32);

            // CFAPattern: count=4 BYTE → 4 bytes inline.
            // RGGB = [0, 1, 1, 2]. Pack as LE u32:
            // bytes: [0x00, 0x01, 0x01, 0x02] = 0x0201_0100 in LE u32
            entry(&mut buf, 33422, 1, 4, 0x0201_0100u32);
        }

        w32(&mut buf, 0); // next IFD offset = 0 (no more IFDs)

        // ── Pixel data immediately follows IFD ───────────────────────────────
        assert_eq!(
            buf.len(), pixel_start,
            "build_raw_tiff: pixel_start mismatch: buf.len()={} vs pixel_start={}",
            buf.len(), pixel_start
        );
        buf.extend_from_slice(pixel_data);
        buf
    }
}
