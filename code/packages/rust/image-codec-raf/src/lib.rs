// # image-codec-raf
//
// Fujifilm RAF (RAW image Format) encoder and decoder.
//
// ## What is RAF?
//
// RAF is Fujifilm's proprietary RAW image container format, used by every
// Fujifilm digital camera since the early 2000s.  Unlike most other RAW
// formats (Nikon NEF, Canon CR2, Sony ARW), RAF does NOT use a TIFF container
// — it has its own binary layout.
//
// RAF files contain three sections:
// - An embedded JPEG thumbnail (the preview you see in the camera menu)
// - A CFA metadata header (image size, CFA pattern, white balance, levels)
// - The raw sensor pixel data (12-bit packed, big-endian)
//
// ## What makes RAF special: X-Trans
//
// Fujifilm's flagship cameras (X-Pro, X-T, X-E, X100 series) use an X-Trans
// colour filter array — a 6×6 pseudo-random pattern rather than the classic
// 2×2 Bayer grid.  X-Trans reduces moiré without an optical low-pass filter,
// but requires a different demosaicing algorithm.
//
// This crate handles both:
// - **Bayer RAF** (FinePix compacts, older bodies): RGGB bilinear demosaicing
// - **X-Trans RAF** (X-Pro, X-T, X-E, X100 series): 6×6 simplified bilinear
//
// ## Decode pipeline
//
// ```text
// 1.  Magic check ("FUJIFILMCCD-RAW ")
// 2.  Outer header parse (JPEG / CFA / pixel offsets)
// 3.  CFA header tag scan (size, pattern, WB, black/white levels)
// 4.  12-bit big-endian pixel unpack
// 5.  Bayer or X-Trans bilinear demosaicing
// 6.  White balance normalisation + colour matrix + sRGB gamma
// 7.  PixelContainer assembly (RGBA, A=255)
// ```
//
// ## Crate modules
//
// - `header`     — outer RAF header (magic check, region offsets)
// - `cfa_header` — CFA metadata tag-block parser
// - `unpack`     — 12-bit big-endian packer/unpacker
// - `bayer`      — 2×2 Bayer bilinear demosaicing
// - `xtrans`     — 6×6 X-Trans bilinear demosaicing
// - `color`      — WB normalisation, colour matrix, sRGB gamma
// - `decoder`    — top-level `decode_raf` orchestrator
// - `encoder`    — minimal test encoder for round-trip tests

pub mod bayer;
pub mod cfa_header;
pub mod color;
pub mod decoder;
pub mod encoder;
pub mod header;
pub mod unpack;
pub mod xtrans;

use paint_instructions::ImageCodec;
use pixel_container::PixelContainer;

pub use decoder::decode_raf;
pub use encoder::encode_raf;

/// Crate version, kept in sync with Cargo.toml.
pub const VERSION: &str = "0.1.0";

// ---------------------------------------------------------------------------
// RafCodec — ImageCodec trait implementation
// ---------------------------------------------------------------------------

/// Fujifilm RAF image codec.
///
/// Implements the `ImageCodec` trait so RAF files can participate in the
/// general-purpose codec pipeline alongside BMP, JPEG, QOI, etc.
///
/// # Example
///
/// ```rust,ignore
/// use image_codec_raf::RafCodec;
/// use paint_instructions::ImageCodec;
///
/// let bytes = std::fs::read("photo.raf").unwrap();
/// let pixels = RafCodec.decode(&bytes).unwrap();
/// println!("Decoded {}×{} image", pixels.width, pixels.height);
/// ```
pub struct RafCodec;

impl ImageCodec for RafCodec {
    fn mime_type(&self) -> &'static str {
        "image/x-fuji-raf"
    }

    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> {
        encode_raf(pixels)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_raf(bytes)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bayer::demosaic_bayer_2x2;
    use crate::cfa_header::parse_cfa_header;
    use crate::color::normalise_wb;
    use crate::unpack::{pack_12bit_be, unpack_12bit_be as unpack_pixels};
    use crate::xtrans::demosaic_xtrans;

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Build a solid-colour PixelContainer for testing.
    fn solid(w: u32, h: u32, r: u8, g: u8, b: u8) -> PixelContainer {
        let mut buf = PixelContainer::new(w, h);
        buf.fill(r, g, b, 255);
        buf
    }

    // ── Test 1: magic_accepted ──────────────────────────────────────────────
    //
    // Encode a tiny image, decode it back, and verify we get a PixelContainer
    // with the right dimensions.
    #[test]
    fn magic_accepted() {
        let original = solid(2, 2, 100, 100, 100);
        let encoded = encode_raf(&original);

        // Outer header magic must be present at the start.
        assert_eq!(&encoded[0..16], b"FUJIFILMCCD-RAW ");

        let decoded = decode_raf(&encoded).expect("decode should succeed");
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 2);
    }

    // ── Test 2: magic_rejected ───────────────────────────────────────────────
    //
    // A buffer that doesn't start with the RAF magic should return Err.
    #[test]
    fn magic_rejected() {
        let mut bad = vec![0u8; 200];
        bad[0..3].copy_from_slice(b"BMP"); // wrong magic
        let result = decode_raf(&bad);
        assert!(result.is_err(), "non-RAF magic should be rejected");
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("magic") || err_msg.contains("RAF"),
            "error message should mention magic or RAF"
        );
    }

    // ── Test 3: header_too_short ─────────────────────────────────────────────
    //
    // Any buffer shorter than 116 bytes must return Err immediately.
    #[test]
    fn header_too_short() {
        // Even with the correct magic, fewer than 116 bytes is rejected.
        let mut buf = b"FUJIFILMCCD-RAW ".to_vec();
        buf.extend_from_slice(&[0u8; 10]); // only 26 bytes total
        let result = decode_raf(&buf);
        assert!(result.is_err(), "truncated header should be rejected");
        let err_msg = result.unwrap_err().to_lowercase();
        assert!(
            err_msg.contains("short") || err_msg.contains("header"),
            "error message should mention short/header"
        );
    }

    // ── Test 4: cfa_header_image_size ────────────────────────────────────────
    //
    // Directly parse a hand-crafted CFA header that contains only tag 0x0100
    // and verify that width and height are extracted correctly.
    #[test]
    fn cfa_header_image_size() {
        // Tag 0x0100: image size
        //   tag:        0x01, 0x00
        //   byte_count: 0x00, 0x04
        //   value:      width=0x0280 (640), height=0x01E0 (480)
        let cfa_bytes: Vec<u8> = vec![
            0x01, 0x00, // tag
            0x00, 0x04, // byte_count = 4
            0x02, 0x80, // width  = 640 (big-endian u16)
            0x01, 0xE0, // height = 480 (big-endian u16)
        ];
        let cfa = parse_cfa_header(&cfa_bytes).expect("parse should succeed");
        assert_eq!(cfa.width, 640);
        assert_eq!(cfa.height, 480);
    }

    // ── Test 5: unpack_roundtrip ──────────────────────────────────────────────
    //
    // Verify that round-tripping through pack → unpack produces the original
    // pixel values.
    #[test]
    fn unpack_roundtrip() {
        let original = vec![0u16, 1, 100, 255, 1000, 2048, 4095];
        let packed = pack_12bit_be(&original);
        let unpacked = unpack_pixels(&packed, original.len());
        assert_eq!(unpacked, original, "round-trip pack/unpack must be lossless");
    }

    // ── Test 6: unpack_12bit_be_two_pixels ───────────────────────────────────
    //
    // The byte sequence [0x12, 0x34, 0x56] must decode to pixels 0x123 and
    // 0x456.  This exercises the specific bit-manipulation formula.
    #[test]
    fn unpack_12bit_be_two_pixels() {
        // b0=0x12, b1=0x34, b2=0x56
        // p0 = (0x12 << 4) | (0x34 >> 4) = 0x120 | 0x03 = 0x123 = 291
        // p1 = ((0x34 & 0x0F) << 8) | 0x56 = (0x04 << 8) | 0x56 = 0x456 = 1110
        let bytes = [0x12u8, 0x34, 0x56];
        let pixels = unpack_pixels(&bytes, 2);
        assert_eq!(pixels.len(), 2);
        assert_eq!(pixels[0], 0x123, "first pixel should be 0x123 = 291");
        assert_eq!(pixels[1], 0x456, "second pixel should be 0x456 = 1110");
    }

    // ── Test 7: bayer_rggb_2x2 ───────────────────────────────────────────────
    //
    // Demosaic a hand-crafted 2×2 RGGB mosaic where each pixel has a known
    // channel value.  The demosaic result should have the expected dominant
    // channel at each corner.
    #[test]
    fn bayer_rggb_2x2() {
        // RGGB pattern: TL=R, TR=G, BL=G, BR=B
        // Raw grid (2×2):
        //   TL(R)=4000  TR(G)=2000
        //   BL(G)=2000  BR(B)=3000
        let raw = vec![4000u16, 2000, 2000, 3000];
        let pattern = [0u8, 1, 1, 2]; // RGGB

        let result = demosaic_bayer_2x2(&raw, 2, 2, pattern);
        assert_eq!(result.len(), 4);

        // Top-left (R pixel): R channel should be exactly 4000.
        let (r_tl, _, _) = result[0];
        assert_eq!(r_tl, 4000, "TL pixel: R channel should be exact raw value");

        // Top-right (G pixel): G channel should be exactly 2000.
        let (_, g_tr, _) = result[1];
        assert_eq!(g_tr, 2000, "TR pixel: G channel should be exact raw value");

        // Bottom-right (B pixel): B channel should be exactly 3000.
        let (_, _, b_br) = result[3];
        assert_eq!(b_br, 3000, "BR pixel: B channel should be exact raw value");
    }

    // ── Test 8: bayer_rggb_4x4 ───────────────────────────────────────────────
    //
    // Demosaic a 4×4 RGGB mosaic and verify no panic and correct output count.
    #[test]
    fn bayer_rggb_4x4() {
        // 4×4 RGGB mosaic with a gradient pattern.
        let raw: Vec<u16> = (0..16).map(|i| i * 256).collect();
        let pattern = [0u8, 1, 1, 2];
        let result = demosaic_bayer_2x2(&raw, 4, 4, pattern);
        assert_eq!(result.len(), 16, "demosaic output must have one entry per pixel");
        // All values must be in 12-bit range [0, 4095].
        for (r, g, b) in &result {
            assert!(*r <= 4095, "R channel must be <= 4095");
            assert!(*g <= 4095, "G channel must be <= 4095");
            assert!(*b <= 4095, "B channel must be <= 4095");
        }
    }

    // ── Test 9: xtrans_pattern_6x6 ───────────────────────────────────────────
    //
    // Parse a CFA header whose 0x0111 tag has 36 bytes (X-Trans), and verify
    // that the parsed pattern is stored as CfaPattern::XTrans.
    #[test]
    fn xtrans_pattern_6x6() {
        // Build a minimal CFA header with just the 0x0111 tag (36 bytes).
        // Standard X-Trans pattern (values 0=R, 1=G, 2=B):
        // Row 0: G B G G R G → 1 2 1 1 0 1
        // Row 1: R G R B G B → 0 1 0 2 1 2
        // Row 2: G B G G R G → 1 2 1 1 0 1
        // Row 3: G R G G B G → 1 0 1 1 2 1
        // Row 4: B G B R G R → 2 1 2 0 1 0
        // Row 5: G R G G B G → 1 0 1 1 2 1
        #[rustfmt::skip]
        let xtrans_36: [u8; 36] = [
            1, 2, 1, 1, 0, 1,
            0, 1, 0, 2, 1, 2,
            1, 2, 1, 1, 0, 1,
            1, 0, 1, 1, 2, 1,
            2, 1, 2, 0, 1, 0,
            1, 0, 1, 1, 2, 1,
        ];

        let mut cfa_bytes = Vec::new();
        cfa_bytes.extend_from_slice(&0x0111u16.to_be_bytes()); // tag
        cfa_bytes.extend_from_slice(&36u16.to_be_bytes());     // byte_count
        cfa_bytes.extend_from_slice(&xtrans_36);

        let cfa = parse_cfa_header(&cfa_bytes).expect("parse should succeed");
        match cfa.pattern {
            cfa_header::CfaPattern::XTrans(p) => {
                assert_eq!(p[0], 1, "row0,col0 should be G (1)");
                assert_eq!(p[4], 0, "row0,col4 should be R (0)");
                assert_eq!(p[5], 1, "row0,col5 should be G (1)");
            }
            cfa_header::CfaPattern::Bayer(_) => {
                panic!("expected XTrans pattern, got Bayer");
            }
        }
    }

    // ── Test 10: xtrans_demosaic_basic ───────────────────────────────────────
    //
    // Demosaic a 6×6 X-Trans image and verify no panic and correct output count.
    #[test]
    fn xtrans_demosaic_basic() {
        #[rustfmt::skip]
        let pattern: [u8; 36] = [
            1, 2, 1, 1, 0, 1,
            0, 1, 0, 2, 1, 2,
            1, 2, 1, 1, 0, 1,
            1, 0, 1, 1, 2, 1,
            2, 1, 2, 0, 1, 0,
            1, 0, 1, 1, 2, 1,
        ];
        // 6×6 raw grid filled with a checkerboard of 1000/3000.
        let raw: Vec<u16> = (0..36).map(|i| if i % 2 == 0 { 1000 } else { 3000 }).collect();
        let result = demosaic_xtrans(&raw, 6, 6, &pattern);
        assert_eq!(result.len(), 36, "demosaic output must have one entry per pixel");
        // All values must be in 12-bit range.
        for (r, g, b) in &result {
            assert!(*r <= 4095, "R out of range");
            assert!(*g <= 4095, "G out of range");
            assert!(*b <= 4095, "B out of range");
        }
    }

    // ── Test 11: wb_normalisation ─────────────────────────────────────────────
    //
    // Verify that the WB normalisation formula divides by G and keeps G=1.0.
    #[test]
    fn wb_normalisation() {
        // R=256, G=512, B=256 → normalised R=0.5, G=1.0, B=0.5
        let wb = normalise_wb([256, 512, 256]);
        assert!((wb[0] - 0.5).abs() < 1e-9, "normalised R should be 0.5");
        assert!((wb[1] - 1.0).abs() < 1e-9, "normalised G should be 1.0");
        assert!((wb[2] - 0.5).abs() < 1e-9, "normalised B should be 0.5");
    }

    // ── Test 12: color_pipeline_neutral ──────────────────────────────────────
    //
    // With neutral WB and the identity colour matrix, white pixels (all channels
    // at maximum) should produce near-white output (255, 255, 255).
    #[test]
    fn color_pipeline_neutral() {
        use crate::color::apply_color_pipeline;

        // One pixel: all channels at white level (4095).
        let rgb = vec![(4095u16, 4095, 4095)];
        let black_level = [0u32; 4];
        let white_level = 4095u32;
        let wb = [1024u32, 1024, 1024]; // neutral (R=G=B → no shift)
        // Identity colour matrix.
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

        let result = apply_color_pipeline(rgb, black_level, white_level, wb, identity);
        assert_eq!(result.len(), 1);
        let (r8, g8, b8) = result[0];
        // After sRGB gamma, a linear 1.0 input → display 255.
        assert_eq!(r8, 255, "full-white R should be 255");
        assert_eq!(g8, 255, "full-white G should be 255");
        assert_eq!(b8, 255, "full-white B should be 255");
    }

    // ── Test 13: round_trip_solid_red ────────────────────────────────────────
    //
    // Encode a 4×4 solid red image, decode it, and verify that the result is
    // "reddish" (R > G and R > B) at every pixel.  We don't assert an exact
    // colour because the encode→pack→unpack→demosaic→gamma chain introduces
    // mild rounding.
    #[test]
    fn round_trip_solid_red() {
        let original = solid(4, 4, 200, 30, 30);
        let encoded = encode_raf(&original);
        let decoded = decode_raf(&encoded).expect("decode should succeed");

        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);

        // Every pixel should be dominantly red.
        for y in 0..4u32 {
            for x in 0..4u32 {
                let (r, g, b, a) = decoded.pixel_at(x, y);
                assert_eq!(a, 255, "alpha must always be 255");
                assert!(
                    r > g && r > b,
                    "pixel ({x},{y}): R={r} G={g} B={b} — should be reddish"
                );
            }
        }
    }

    // ── Test 14: mime_type ────────────────────────────────────────────────────
    #[test]
    fn mime_type() {
        assert_eq!(RafCodec.mime_type(), "image/x-fuji-raf");
    }

    // ── Test 15: version_constant ────────────────────────────────────────────
    #[test]
    fn version_constant() {
        assert_eq!(VERSION, "0.1.0");
    }

    // ── Test 16: codec_trait_encode_decode ───────────────────────────────────
    //
    // Verify that the ImageCodec trait dispatch works correctly.
    #[test]
    fn codec_trait_encode_decode() {
        let original = solid(2, 2, 80, 80, 80);
        let encoded = RafCodec.encode(&original);
        let decoded = RafCodec.decode(&encoded).expect("codec trait decode should succeed");
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 2);
    }

    // ── Test 17: cfa_header_wb_tag ───────────────────────────────────────────
    //
    // Parse a CFA header that contains tag 0x0130 and verify the WB values.
    #[test]
    fn cfa_header_wb_tag() {
        // Tag 0x0130: WB multipliers (3× u32 LE: R=2048, G=1024, B=1536)
        let mut cfa_bytes = Vec::new();
        cfa_bytes.extend_from_slice(&0x0130u16.to_be_bytes());
        cfa_bytes.extend_from_slice(&12u16.to_be_bytes()); // 3 × 4 bytes
        cfa_bytes.extend_from_slice(&2048u32.to_le_bytes()); // R
        cfa_bytes.extend_from_slice(&1024u32.to_le_bytes()); // G
        cfa_bytes.extend_from_slice(&1536u32.to_le_bytes()); // B

        let cfa = parse_cfa_header(&cfa_bytes).expect("parse should succeed");
        assert_eq!(cfa.wb[0], 2048, "R WB");
        assert_eq!(cfa.wb[1], 1024, "G WB");
        assert_eq!(cfa.wb[2], 1536, "B WB");
    }

    // ── Test 18: decode_empty_bytes ──────────────────────────────────────────
    #[test]
    fn decode_empty_bytes() {
        let result = decode_raf(&[]);
        assert!(result.is_err(), "empty slice should return Err");
    }

    // ── Test 19: unpack_single_pixel ─────────────────────────────────────────
    //
    // A request for 1 pixel from a 2-byte buffer should work without panic.
    // With only 2 bytes the loop needs i+2 < data.len() which fails for len=2,
    // so a 3-byte buffer is used (the third byte is a don't-care padding byte).
    #[test]
    fn unpack_single_pixel() {
        // [0xAB, 0xC0, 0x00] → p0 = (0xAB << 4) | (0xC0 >> 4) = 0xAB0 | 0x0C = 0xABC = 2748
        let bytes = [0xABu8, 0xC0, 0x00];
        let pixels = unpack_pixels(&bytes, 1);
        assert_eq!(pixels.len(), 1);
        assert_eq!(pixels[0], 0xABC, "single-pixel unpack should produce 0xABC");
    }
}
