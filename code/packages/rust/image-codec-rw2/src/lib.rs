// # image-codec-rw2
//
// Panasonic RW2 RAW image decoder (and minimal test encoder) for Rust.
//
// ## What is RW2?
//
// RW2 (RAW version 2) is Panasonic's proprietary camera RAW format, used in
// every Lumix body since the GH1 (2009). It replaced the older `.raw` format
// and is now the dominant RAW format on all Micro 4/3 and full-frame S-series
// bodies.
//
// ## Format at a Glance
//
//   ┌──────────────────────────────────────────────┐
//   │  8-byte RW2 header                           │
//   │    "II" (LE) + version 85 + IFD offset       │
//   ├──────────────────────────────────────────────┤
//   │  TIFF-like IFD (Panasonic private tags)       │
//   │    SensorWidth / SensorHeight                 │
//   │    Border crop tags (active area)             │
//   │    RedBalance / BlueBalance (WB)              │
//   │    RawDataOffset                              │
//   ├──────────────────────────────────────────────┤
//   │  12-bit LE packed raw Bayer data              │
//   │    RGGB pattern, stride = ⌈W×12/8⌉ bytes     │
//   └──────────────────────────────────────────────┘
//
// ## Decode Pipeline
//
// ```text
// RW2 bytes
//   → magic + IFD parse (header.rs)
//   → 12-bit LE unpack (unpack.rs)
//   → active-area crop
//   → bilinear Bayer demosaic, RGGB (bayer.rs)
//   → WB × colour matrix × sRGB gamma (color.rs)
//   → PixelContainer (RGBA8, A=255)
// ```
//
// ## Limitations (v0.1)
//
// - Only 12-bit packed (uncompressed) RW2 is decoded.
// - Panasonic lossless compression (GH5/S1/S5 v5+) returns `Err`.
// - 16-bit ImageDepth returns `Err`.
// - A single hardcoded colour matrix (Panasonic GH5 D65) is used for all models.

pub const VERSION: &str = "0.1.0";

// Re-export pixel_container types so callers don't need a separate dep.
pub use pixel_container::PixelContainer;

// Sub-modules.
pub mod bayer;
pub mod color;
mod decoder;
mod encoder;
mod header;
mod unpack;

// ── Public API ───────────────────────────────────────────────────────────────

/// Decode a Panasonic RW2 file from a byte slice into a [`PixelContainer`].
///
/// The returned container is RGBA8 with A=255 throughout.
///
/// # Errors
///
/// Returns `Err` with a descriptive message if:
/// - The bytes are not valid RW2 (wrong magic, truncated file, etc.)
/// - The file uses Panasonic lossless compression (not supported in v0.1)
/// - The sensor uses 16-bit depth (not supported in v0.1)
///
/// # Examples
///
/// ```rust,ignore
/// let bytes = std::fs::read("sample.RW2").unwrap();
/// match image_codec_rw2::decode_rw2(&bytes) {
///     Ok(pixels) => println!("{}×{}", pixels.width, pixels.height),
///     Err(e) => eprintln!("Error: {e}"),
/// }
/// ```
pub fn decode_rw2(bytes: &[u8]) -> Result<PixelContainer, String> {
    decoder::decode_rw2(bytes)
}

/// Encode a [`PixelContainer`] into a minimal synthetic RW2 file.
///
/// This is a test-only encoder — it produces structurally valid RW2 bytes that
/// `decode_rw2` can round-trip. It is NOT suitable for writing files to a
/// Panasonic camera; the format is proprietary and read-only in practice.
pub fn encode_rw2(pixels: &PixelContainer) -> Vec<u8> {
    encoder::encode_rw2(pixels)
}

// ── Codec trait implementation ────────────────────────────────────────────────

use paint_instructions::ImageCodec;

/// `ImageCodec` implementation for Panasonic RW2.
///
/// Plugs into the codec pipeline:
///
/// ```text
/// rw2_bytes → Rw2Codec::decode() → PixelContainer → PngCodec::encode() → png_bytes
/// ```
pub struct Rw2Codec;

impl ImageCodec for Rw2Codec {
    fn mime_type(&self) -> &'static str {
        "image/x-panasonic-rw2"
    }

    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> {
        encode_rw2(pixels)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_rw2(bytes)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unpack::unpack_12bit_le;

    // ── Helper: create a solid-colour PixelContainer ─────────────────────────

    fn solid(w: u32, h: u32, r: u8, g: u8, b: u8) -> PixelContainer {
        let mut buf = PixelContainer::new(w, h);
        buf.fill(r, g, b, 255);
        buf
    }

    // ── Test 1: magic accepted ────────────────────────────────────────────────
    //
    // A 2×2 image encoded by our encoder must be successfully decoded back.

    #[test]
    fn magic_accepted() {
        let original = solid(2, 2, 100, 150, 200);
        let rw2 = encode_rw2(&original);
        let result = decode_rw2(&rw2);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
        let decoded = result.unwrap();
        assert_eq!(decoded.width,  2);
        assert_eq!(decoded.height, 2);
    }

    // ── Test 2: wrong magic — standard TIFF (II + 42) ─────────────────────────
    //
    // Standard TIFF uses version byte 42 (0x002A). RW2 uses 85 (0x0055).
    // We must reject standard TIFF files.

    #[test]
    fn wrong_magic_tiff() {
        let mut tiff_header = vec![0u8; 8];
        tiff_header[0] = 0x49; tiff_header[1] = 0x49; // "II"
        tiff_header[2] = 0x2A; tiff_header[3] = 0x00; // version 42 LE = standard TIFF
        tiff_header[4] = 0x08; // IFD at 8
        let result = decode_rw2(&tiff_header);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("version byte is 42") || msg.contains("not an RW2"),
            "Unexpected error: {msg}"
        );
    }

    // ── Test 3: wrong magic — random bytes ────────────────────────────────────

    #[test]
    fn wrong_magic_random() {
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x00, 0x00, 0x00];
        let result = decode_rw2(&data);
        assert!(result.is_err());
    }

    // ── Test 4: header too short ───────────────────────────────────────────────

    #[test]
    fn header_too_short() {
        let result = decode_rw2(&[0x49, 0x49, 0x55]);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("too short"),
            "Expected 'too short' error"
        );
    }

    // ── Test 5: 12-bit LE unpack pair ─────────────────────────────────────────
    //
    // Verify the unpacking arithmetic against a hand-computed reference.
    // p0 = 0x123, p1 = 0x456:
    //   byte0 = 0x23
    //   byte1 = 0x01 | (0x06 << 4) = 0x61
    //   byte2 = 0x45

    #[test]
    fn unpack_12bit_le_pair() {
        let data = [0x23u8, 0x61, 0x45];
        let pixels = unpack_12bit_le(&data, 2);
        assert_eq!(pixels[0], 0x123);
        assert_eq!(pixels[1], 0x456);
    }

    // ── Test 6: sensor border crop ────────────────────────────────────────────
    //
    // We build a synthetic 4×4 sensor but with borders that crop it to 2×2.
    // The decoder should produce a 2×2 output, not 4×4.

    #[test]
    fn sensor_border_crop() {
        // Encode a 4×4 image and patch the IFD borders to crop to center 2×2.
        // Our encoder sets borders = [0, 0, height, width] (full sensor). We
        // need to manipulate the raw bytes to set borders [1, 1, 3, 3] instead.
        //
        // The IFD starts at offset 8. Entry count is 10 entries × 12 bytes = 120
        // bytes of entries at offset 10. Entries are written in this order:
        //
        //   entry 0 (offset 10): tag 0x0002 SensorWidth
        //   entry 1 (offset 22): tag 0x0003 SensorHeight
        //   entry 2 (offset 34): tag 0x0004 SensorTopBorder   ← patch to 1
        //   entry 3 (offset 46): tag 0x0005 SensorLeftBorder  ← patch to 1
        //   entry 4 (offset 58): tag 0x0006 SensorBottomBorder← patch to 3
        //   entry 5 (offset 70): tag 0x0007 SensorRightBorder ← patch to 3
        //   ...
        //
        // value_or_offset is at bytes [8..12] within each 12-byte entry, i.e.
        // at IFD_start + 2 + (entry_index * 12) + 8.

        let base = solid(4, 4, 80, 120, 160);
        let mut rw2 = encode_rw2(&base);

        // Patch entries 2..5 to set borders [1, 1, 3, 3].
        // Each entry's value field starts at: 8 (header) + 2 (count) + idx*12 + 8
        let base_entry = 8 + 2; // 10
        for (idx, border_val) in [(2usize, 1u16), (3, 1), (4, 3), (5, 3)] {
            let val_offset = base_entry + idx * 12 + 8;
            rw2[val_offset]     = (border_val & 0xFF) as u8;
            rw2[val_offset + 1] = (border_val >> 8) as u8;
        }

        let decoded = decode_rw2(&rw2).expect("decode should succeed");
        assert_eq!(decoded.width,  2, "Expected crop width 2, got {}", decoded.width);
        assert_eq!(decoded.height, 2, "Expected crop height 2, got {}", decoded.height);
    }

    // ── Test 7: white balance extraction ─────────────────────────────────────
    //
    // Tag 0x0011 RedBalance = 512 → wb_r = 2.0
    // Tag 0x0012 BlueBalance = 256 → wb_b = 1.0

    #[test]
    fn wb_extraction() {
        use crate::color::white_balance_from_tags;
        let wb = white_balance_from_tags(Some(512), Some(256));
        assert!((wb[0] - 2.0).abs() < 1e-9, "wb_r expected 2.0, got {}", wb[0]);
        assert!((wb[1] - 1.0).abs() < 1e-9, "wb_g expected 1.0, got {}", wb[1]);
        assert!((wb[2] - 1.0).abs() < 1e-9, "wb_b expected 1.0, got {}", wb[2]);
    }

    // ── Test 8: colour pipeline neutral ───────────────────────────────────────
    //
    // With an identity 3×3 colour matrix, neutral WB, and a mid-grey input,
    // all three output channels should be equal (same grey value).

    #[test]
    fn color_pipeline_neutral() {
        use crate::color::apply_color_pipeline;
        let identity = [[1.0f64, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let pixels = vec![(2048u16, 2048, 2048)];
        let out = apply_color_pipeline(pixels, 240, 4095, [1.0, 1.0, 1.0], identity);
        let (r, g, b) = out[0];
        assert_eq!(r, g, "R and G should be equal for mid-grey");
        assert_eq!(g, b, "G and B should be equal for mid-grey");
    }

    // ── Test 9: round-trip solid colour ───────────────────────────────────────
    //
    // A 4×4 solid-green image should round-trip through encode→decode with
    // the green channel dominant in the output. We can't expect exact values
    // because the colour pipeline applies gamma and a colour matrix, but we
    // can check that g > r and g > b for a solid-green input.

    #[test]
    fn round_trip_solid_color() {
        let original = solid(4, 4, 0, 200, 0);
        let rw2 = encode_rw2(&original);
        let decoded = decode_rw2(&rw2).expect("decode should succeed");

        // The output should be 4×4.
        assert_eq!(decoded.width,  4);
        assert_eq!(decoded.height, 4);

        // Check a few pixels: green channel should dominate.
        let (r, g, b, _a) = decoded.pixel_at(1, 1);
        assert!(
            g >= r && g >= b,
            "Expected green to dominate: ({r},{g},{b})"
        );
    }

    // ── Test 10: lossless returns Err ─────────────────────────────────────────
    //
    // If the file claims a sensor size but the available raw bytes are less than
    // 80% of the expected uncompressed size, the decoder must return Err.

    #[test]
    fn lossless_returns_err() {
        // Build a valid 8×8 RW2 file, then truncate the raw data section to
        // force the "lossless compression detected" code path.
        let base = solid(8, 8, 50, 100, 150);
        let rw2 = encode_rw2(&base);

        // The raw data starts at offset 134 (8 header + 126 IFD).
        // For an 8×8 sensor at 12 bpp: stride = ceil(8*12/8) = 12 bytes,
        // expected total = 12 * 8 = 96 bytes. Keep only 10 bytes of raw data
        // (way under 80% of 96 = 76.8 bytes), which should trigger the error.
        let truncated = rw2[..134 + 10].to_vec();
        let result = decode_rw2(&truncated);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("lossless") || msg.contains("compression"),
            "Expected lossless error, got: {msg}"
        );
    }

    // ── Test 11: MIME type ────────────────────────────────────────────────────

    #[test]
    fn mime_type() {
        assert_eq!(Rw2Codec.mime_type(), "image/x-panasonic-rw2");
    }

    // ── Test 12: VERSION constant ─────────────────────────────────────────────

    #[test]
    fn version_constant() {
        assert_eq!(VERSION, "0.1.0");
    }

    // ── Test 13: codec trait round-trip ───────────────────────────────────────

    #[test]
    fn codec_trait_round_trip() {
        use paint_instructions::ImageCodec;
        let original = solid(4, 4, 120, 80, 40);
        let rw2 = Rw2Codec.encode(&original);
        let decoded = Rw2Codec.decode(&rw2).expect("decode should succeed");
        assert_eq!(decoded.width,  4);
        assert_eq!(decoded.height, 4);
    }
}
