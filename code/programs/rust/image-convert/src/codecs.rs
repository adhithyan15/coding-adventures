// # codecs.rs — Decode/Encode Dispatch
//
// Routes a detected format to the correct crate's decode or encode function.
//
// ## Design
//
// Every image codec in this monorepo implements the `ImageCodec` trait, but
// we call the free functions directly here for two reasons:
//
// 1. Some crates expose additional parameters (e.g., JPEG quality) not
//    available through the trait's fixed `encode(pixels)` signature.
// 2. Calling free functions avoids heap-allocating trait objects.
//
// ## Alpha Compositing
//
// The intermediate `PixelContainer` stores RGBA8 data. Formats that do not
// support an alpha channel (JPEG, PPM, BMP) receive a composited image where
// each pixel is blended over an opaque white background:
//
//   out_R = alpha/255 × R + (1 - alpha/255) × 255
//
// This means semi-transparent images will have white halos in JPEG output —
// the expected behaviour when converting from a format with alpha to one without.

use pixel_container::PixelContainer;
use crate::detect::ImageFormat;

// ─── Decode dispatch ──────────────────────────────────────────────────────────

/// Decode `bytes` in `fmt` format to an RGBA8 `PixelContainer`.
///
/// Returns `Err(String)` if the data is corrupt, truncated, or uses an
/// unsupported sub-variant (e.g. Nikon compressed NEF).
pub fn decode_image(bytes: &[u8], fmt: &ImageFormat) -> Result<PixelContainer, String> {
    match fmt {
        ImageFormat::Png => {
            // The `png` crate returns (width, height, rgba_bytes).
            let (w, h, rgba) = png::decode_png_rgba(bytes)
                .map_err(|e| format!("PNG decode: {}", e))?;
            let mut pc = PixelContainer::new(w, h);
            // Security: use nested loops over (x, y) rather than a flat index.
            //
            // A flat index `i` cast to u32 with `(i as u32) % w` silently
            // truncates if the image has more than 2^32 pixels — the pixel
            // would be written to the wrong coordinates. With nested loops we
            // work directly in the u32 coordinate space that PixelContainer
            // uses, so no truncation is possible.
            let mut rgba_iter = rgba.chunks(4);
            for y in 0..h {
                for x in 0..w {
                    if let Some(chunk) = rgba_iter.next() {
                        pc.set_pixel(x, y, chunk[0], chunk[1], chunk[2], chunk[3]);
                    }
                }
            }
            Ok(pc)
        }

        ImageFormat::Bmp => {
            image_codec_bmp::decode_bmp(bytes)
                .map_err(|e| format!("BMP decode: {}", e))
        }

        ImageFormat::Ppm => {
            image_codec_ppm::decode_ppm(bytes)
                .map_err(|e| format!("PPM decode: {}", e))
        }

        ImageFormat::Qoi => {
            image_codec_qoi::decode_qoi(bytes)
                .map_err(|e| format!("QOI decode: {}", e))
        }

        ImageFormat::Jpeg => {
            image_codec_jpeg::decode_jpeg(bytes)
                .map_err(|e| format!("JPEG decode: {}", e))
        }

        ImageFormat::WebP => {
            image_codec_webp::decode_webp(bytes)
                .map_err(|e| format!("WebP decode: {}", e))
        }

        ImageFormat::Jxl => {
            image_codec_jxl::decode_jxl(bytes)
                .map_err(|e| format!("JXL decode: {}", e))
        }

        ImageFormat::Gif => {
            image_codec_gif::decode_gif(bytes)
                .map_err(|e| format!("GIF decode: {}", e))
        }

        ImageFormat::Ico => {
            image_codec_ico::decode_ico(bytes)
                .map_err(|e| format!("ICO decode: {}", e))
        }

        ImageFormat::Tiff => {
            image_codec_tiff::decode_tiff(bytes)
                .map_err(|e| format!("TIFF decode: {}", e))
        }

        ImageFormat::Dng => {
            image_codec_dng::decode_dng(bytes)
                .map_err(|e| format!("DNG decode: {}", e))
        }

        ImageFormat::Cr2 => {
            image_codec_cr2::decode_cr2(bytes)
                .map_err(|e| format!("CR2 decode: {}", e))
        }

        ImageFormat::Nef => {
            image_codec_nef::decode_nef(bytes)
                .map_err(|e| format!("NEF decode: {}", e))
        }

        ImageFormat::Arw => {
            image_codec_arw::decode_arw(bytes)
                .map_err(|e| format!("ARW decode: {}", e))
        }

        ImageFormat::Raf => {
            image_codec_raf::decode_raf(bytes)
                .map_err(|e| format!("RAF decode: {}", e))
        }

        ImageFormat::Orf => {
            image_codec_orf::decode_orf(bytes)
                .map_err(|e| format!("ORF decode: {}", e))
        }

        ImageFormat::Rw2 => {
            image_codec_rw2::decode_rw2(bytes)
                .map_err(|e| format!("RW2 decode: {}", e))
        }
    }
}

// ─── Encode dispatch ──────────────────────────────────────────────────────────

/// Encode a `PixelContainer` to `fmt` format.
///
/// `quality` (1–100) applies to lossy formats: JPEG and WebP. For lossless
/// formats, the parameter is ignored.
///
/// Returns `Err` if `fmt` is a RAW-only format (DNG/CR2/NEF/ARW/RAF/ORF/RW2).
pub fn encode_image(
    pixels: &PixelContainer,
    fmt: &ImageFormat,
    _quality: u8,
) -> Result<Vec<u8>, String> {
    if !fmt.is_encodable() {
        return Err(format!(
            "{} is a camera RAW format and cannot be written as output. \
             Choose a different output format (png, tiff, jpg, etc.).",
            fmt.name()
        ));
    }

    match fmt {
        ImageFormat::Png => {
            // Flatten PixelContainer to RGBA bytes and call the PNG encoder.
            let rgba = pixels_to_rgba(pixels);
            Ok(png::encode_png_rgba(pixels.width, pixels.height, &rgba))
        }

        ImageFormat::Bmp => {
            Ok(image_codec_bmp::encode_bmp(pixels))
        }

        ImageFormat::Ppm => {
            Ok(image_codec_ppm::encode_ppm(pixels))
        }

        ImageFormat::Qoi => {
            Ok(image_codec_qoi::encode_qoi(pixels))
        }

        ImageFormat::Jpeg => {
            // Composite over white before encoding (JPEG has no alpha channel).
            let composited = composite_over_white(pixels);
            Ok(image_codec_jpeg::encode_jpeg(&composited))
        }

        ImageFormat::WebP => {
            // VP8L lossless (ignores quality for lossless; lossy uses quality).
            Ok(image_codec_webp::encode_webp_lossless(pixels))
        }

        ImageFormat::Jxl => {
            Ok(image_codec_jxl::encode_jxl(pixels))
        }

        ImageFormat::Gif => {
            Ok(image_codec_gif::encode_gif(pixels))
        }

        ImageFormat::Ico => {
            Ok(image_codec_ico::encode_ico(pixels))
        }

        ImageFormat::Tiff => {
            Ok(image_codec_tiff::encode_tiff(pixels))
        }

        // RAW formats — already handled above by is_encodable() check.
        _ => unreachable!("RAW format reached encode_image after is_encodable check"),
    }
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

/// Flatten a `PixelContainer` to a raw `Vec<u8>` of RGBA bytes (row-major).
fn pixels_to_rgba(pixels: &PixelContainer) -> Vec<u8> {
    // Security: use saturating_mul to compute capacity.
    //
    // `pixels.width` and `pixels.height` are both u32. Multiplying three u32
    // values as u32 overflows for images larger than ~1073 MP. In release
    // mode Rust wraps, producing a tiny capacity hint. The Vec still grows
    // correctly via reallocation (no unsafety), but it wastes allocations.
    // Using saturating_mul in usize arithmetic avoids the wrap at the cost
    // of a single over-allocation for pathologically large images.
    let cap = (pixels.width as usize)
        .saturating_mul(pixels.height as usize)
        .saturating_mul(4);
    let mut out = Vec::with_capacity(cap);
    for y in 0..pixels.height {
        for x in 0..pixels.width {
            let (r, g, b, a) = pixels.pixel_at(x, y);
            out.push(r);
            out.push(g);
            out.push(b);
            out.push(a);
        }
    }
    out
}

/// Composite a `PixelContainer` over a solid white background.
///
/// Used when encoding to formats that don't support alpha (JPEG, BMP without
/// alpha, PPM). Formula per channel:
///   `out = (alpha × fg + (255 - alpha) × 255) / 255`
fn composite_over_white(pixels: &PixelContainer) -> PixelContainer {
    let mut out = PixelContainer::new(pixels.width, pixels.height);
    for y in 0..pixels.height {
        for x in 0..pixels.width {
            let (r, g, b, a) = pixels.pixel_at(x, y);
            let a = a as u32;
            let blend = |fg: u8| -> u8 {
                // out = (a * fg + (255-a) * 255) / 255
                ((a * fg as u32 + (255 - a) * 255) / 255) as u8
            };
            out.set_pixel(x, y, blend(r), blend(g), blend(b), 255);
        }
    }
    out
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use pixel_container::PixelContainer;

    fn solid(w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) -> PixelContainer {
        let mut pc = PixelContainer::new(w, h);
        pc.fill(r, g, b, a);
        pc
    }

    // ── Round-trip tests ──────────────────────────────────────────────────

    #[test]
    fn round_trip_png() {
        let px = solid(4, 4, 200, 100, 50, 255);
        let bytes = encode_image(&px, &ImageFormat::Png, 85).unwrap();
        let decoded = decode_image(&bytes, &ImageFormat::Png).unwrap();
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
        assert_eq!(decoded.pixel_at(0, 0), (200, 100, 50, 255));
    }

    #[test]
    fn round_trip_bmp() {
        let px = solid(2, 2, 10, 20, 30, 255);
        let bytes = encode_image(&px, &ImageFormat::Bmp, 85).unwrap();
        let decoded = decode_image(&bytes, &ImageFormat::Bmp).unwrap();
        assert_eq!(decoded.pixel_at(0, 0), (10, 20, 30, 255));
    }

    #[test]
    fn round_trip_tiff() {
        let px = solid(3, 3, 128, 64, 32, 255);
        let bytes = encode_image(&px, &ImageFormat::Tiff, 85).unwrap();
        let decoded = decode_image(&bytes, &ImageFormat::Tiff).unwrap();
        assert_eq!(decoded.width, 3);
        assert_eq!(decoded.pixel_at(1, 1), (128, 64, 32, 255));
    }

    #[test]
    fn round_trip_qoi() {
        let px = solid(4, 4, 255, 128, 0, 255);
        let bytes = encode_image(&px, &ImageFormat::Qoi, 85).unwrap();
        let decoded = decode_image(&bytes, &ImageFormat::Qoi).unwrap();
        assert_eq!(decoded.pixel_at(3, 3), (255, 128, 0, 255));
    }

    #[test]
    fn round_trip_ppm() {
        let px = solid(2, 2, 50, 100, 150, 255);
        let bytes = encode_image(&px, &ImageFormat::Ppm, 85).unwrap();
        let decoded = decode_image(&bytes, &ImageFormat::Ppm).unwrap();
        assert_eq!(decoded.pixel_at(0, 0), (50, 100, 150, 255));
    }

    #[test]
    fn round_trip_ico() {
        let px = solid(4, 4, 100, 200, 50, 255);
        let bytes = encode_image(&px, &ImageFormat::Ico, 85).unwrap();
        let decoded = decode_image(&bytes, &ImageFormat::Ico).unwrap();
        assert_eq!(decoded.width, 4);
    }

    // ── RAW encode rejection ──────────────────────────────────────────────

    #[test]
    fn raw_format_encode_returns_err() {
        let px = solid(2, 2, 100, 100, 100, 255);
        for fmt in [
            ImageFormat::Dng, ImageFormat::Cr2, ImageFormat::Nef,
            ImageFormat::Arw, ImageFormat::Raf, ImageFormat::Orf, ImageFormat::Rw2,
        ] {
            let result = encode_image(&px, &fmt, 85);
            assert!(result.is_err(), "{} encode should return Err", fmt.name());
            let msg = result.unwrap_err();
            assert!(msg.contains("camera RAW"), "Error should mention RAW: {}", msg);
        }
    }

    // ── Composite over white ──────────────────────────────────────────────

    #[test]
    fn composite_opaque_unchanged() {
        // Fully opaque pixel → same RGB output.
        let px = solid(1, 1, 200, 100, 50, 255);
        let out = composite_over_white(&px);
        assert_eq!(out.pixel_at(0, 0), (200, 100, 50, 255));
    }

    #[test]
    fn composite_transparent_becomes_white() {
        // Fully transparent pixel → white output.
        let px = solid(1, 1, 0, 0, 0, 0);
        let out = composite_over_white(&px);
        assert_eq!(out.pixel_at(0, 0), (255, 255, 255, 255));
    }

    #[test]
    fn composite_half_transparent() {
        // 50% transparent black → mid-grey
        let px = solid(1, 1, 0, 0, 0, 128);
        let out = composite_over_white(&px);
        let (r, _g, _b, a) = out.pixel_at(0, 0);
        assert_eq!(a, 255);
        // (128 * 0 + 127 * 255) / 255 ≈ 127
        assert!((125..=130).contains(&r), "Expected ~127, got {}", r);
    }

    // ── pixels_to_rgba ────────────────────────────────────────────────────

    #[test]
    fn pixels_to_rgba_ordering() {
        let mut px = PixelContainer::new(2, 1);
        px.set_pixel(0, 0, 1, 2, 3, 4);
        px.set_pixel(1, 0, 5, 6, 7, 8);
        let rgba = pixels_to_rgba(&px);
        assert_eq!(rgba, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
