//! # image-codec-webp
//!
//! WebP image codec for the paint-instructions pixel pipeline.
//! Implements VP8L (lossless) and VP8 lossy encoding and decoding.
//!
//! ## Architecture
//!
//! WebP files use a RIFF container:
//!
//! ```text
//! RIFF <file_size> WEBP
//!   VP8L <chunk_size> <vp8l-bitstream>
//! ```
//!
//! The VP8L bitstream stores pixels using:
//!
//! 1. Optional transforms (subtract_green, color, predictor, color_index).
//!    This release always writes no transforms.
//! 2. LZ77 backward references with 2D distance mapping.
//!    This release uses literal-only mode (no back-references).
//! 3. Canonical Huffman prefix codes in 5 groups (G, R, B, A, Dist).
//!
//! ## Usage
//!
//! ```rust,ignore
//! use image_codec_webp::{encode_webp_lossless, decode_webp, WebPCodec};
//! use paint_instructions::ImageCodec;
//!
//! // Functional API:
//! let encoded = encode_webp_lossless(&pixels);
//! let decoded = decode_webp(&encoded).unwrap();
//!
//! // Trait API:
//! let codec = WebPCodec::new(90, true);
//! let bytes = codec.encode(&pixels);
//! let pixels2 = codec.decode(&bytes).unwrap();
//! ```
//!
//! ## VP8 lossy
//!
//! Lossy WebP (VP8) requires the `range-coder` crate (arithmetic coding) which
//! is being implemented in a parallel PR.  Calling `encode_webp` or constructing
//! `WebPCodec::new(q, false)` and calling `encode` will panic with a clear message.
//!
//! ## References
//!
//! - WebP lossless bitstream spec: https://developers.google.com/speed/webp/docs/webp_lossless_bitstream_specification
//! - WebP container spec: https://developers.google.com/speed/webp/docs/riff_container
//! - VP8 lossy spec: https://www.rfc-editor.org/rfc/rfc6386

pub const VERSION: &str = "0.3.8";

mod riff;
pub mod vp8;
pub mod vp8l;

use paint_instructions::{ImageCodec, PixelContainer};

// ---------------------------------------------------------------------------
// WebPCodec — implements the ImageCodec trait
// ---------------------------------------------------------------------------

/// A WebP image codec that implements [`ImageCodec`].
///
/// Supports lossless encoding (VP8L) and returns a descriptive error for
/// VP8 lossy encoding (requires the `range-coder` crate, coming in a future PR).
///
/// ## Example
///
/// ```rust,ignore
/// use image_codec_webp::WebPCodec;
/// use paint_instructions::ImageCodec;
///
/// let codec = WebPCodec::new(90, true); // 90% quality, lossless
/// let bytes = codec.encode(&pixels);
/// let decoded = codec.decode(&bytes).unwrap();
/// ```
pub struct WebPCodec {
    /// Quality hint for lossy encoding (0–100).  Ignored in lossless mode.
    pub quality: u8,
    /// If `true`, use VP8L lossless encoding.  If `false`, use VP8 lossy
    /// (currently unimplemented — panics).
    pub lossless: bool,
}

impl WebPCodec {
    /// Create a new `WebPCodec`.
    ///
    /// `quality` is a hint for the VP8 lossy encoder (0=worst, 100=best).
    /// In lossless mode (`lossless=true`) quality is ignored.
    pub fn new(quality: u8, lossless: bool) -> Self {
        Self { quality, lossless }
    }
}

impl ImageCodec for WebPCodec {
    /// Returns `"image/webp"`.
    fn mime_type(&self) -> &'static str {
        "image/webp"
    }

    /// Encode a pixel buffer as a WebP file.
    ///
    /// In lossless mode (`self.lossless = true`) this calls `encode_webp_lossless`.
    ///
    /// # Panics
    ///
    /// Panics if `self.lossless = false` (VP8 lossy not yet implemented).
    fn encode(&self, pixels: &PixelContainer) -> Vec<u8> {
        if self.lossless {
            encode_webp_lossless(pixels)
        } else {
            encode_webp(pixels, self.quality)
        }
    }

    /// Decode a WebP file into a pixel buffer.
    ///
    /// Supports VP8L (lossless) chunk type.
    /// Returns `Err` for VP8 lossy, VP8X extended, or unknown chunk types.
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_webp(bytes)
    }
}

// ---------------------------------------------------------------------------
// Functional API
// ---------------------------------------------------------------------------

/// Encode a `PixelContainer` as a lossless WebP file (VP8L).
///
/// Returns a complete, self-contained WebP file (RIFF container + VP8L chunk).
/// Ready to write to disk or send over the network.
///
/// This uses literal-only VP8L encoding without transforms or LZ77
/// back-references.  Compression is valid but not optimal compared to a
/// full encoder with all transforms enabled.
pub fn encode_webp_lossless(pixels: &PixelContainer) -> Vec<u8> {
    let bitstream = vp8l::encode(pixels);
    riff::build_riff(b"VP8L", &bitstream)
}

/// Encode a `PixelContainer` as a lossy WebP file (VP8).
///
/// `quality` is in [0, 100]; higher = better quality / larger file.
/// Returns a complete RIFF/WEBP/VP8 container.
pub fn encode_webp(pixels: &PixelContainer, quality: u8) -> Vec<u8> {
    let vp8_data = vp8::encode(pixels, quality);
    riff::build_riff(b"VP8 ", &vp8_data)
}

/// Decode a WebP file (RIFF container) into a `PixelContainer`.
///
/// Supports the VP8L (lossless) chunk type.
/// Decode a WebP file from raw bytes.
///
/// Supported container formats:
/// - `VP8L` — VP8L lossless (single chunk, no VP8X wrapper).
/// - `VP8 ` — VP8 lossy (single chunk, no VP8X wrapper).
/// - `VP8X` — Extended WebP: scans sub-chunks and dispatches `VP8L` or
///   `VP8 ` for image data; skips `ICCP`, `EXIF`, `XMP ` metadata chunks;
///   decodes `ALPH` lossless alpha and merges it with lossy `VP8 ` color.
///
/// Returns an error for animated WebP (`ANIM`/`ANMF` chunks) and unknown
/// chunk types.
pub fn decode_webp(bytes: &[u8]) -> Result<PixelContainer, String> {
    if bytes.len() < 12 {
        return Err("WebP: file too short (need at least 12 bytes for RIFF header)".to_string());
    }
    if &bytes[0..4] != b"RIFF" {
        return Err("WebP: missing RIFF magic bytes".to_string());
    }
    if bytes.len() < 20 {
        return Err("WebP: file too short to contain a WEBP chunk header".to_string());
    }
    if &bytes[8..12] != b"WEBP" {
        return Err("WebP: missing WEBP fourCC (bytes 8-11)".to_string());
    }

    // Parse the first chunk (immediately after the RIFF/WEBP header).
    let chunk_type = &bytes[12..16];
    let chunk_size = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;

    let chunk_data_start = 20usize;
    if bytes.len() < chunk_data_start + chunk_size {
        return Err(format!(
            "WebP: chunk truncated (need {} bytes after offset 20, have {})",
            chunk_size,
            bytes.len() - chunk_data_start
        ));
    }
    let chunk_data = &bytes[chunk_data_start..chunk_data_start + chunk_size];

    match chunk_type {
        b"VP8L" => vp8l::decode(chunk_data),
        b"VP8 " => vp8::decode(chunk_data),
        b"VP8X" => decode_vp8x(bytes),
        _ => Err(format!(
            "WebP: unknown chunk type {:?} (expected VP8L, VP8 , or VP8X)",
            std::str::from_utf8(chunk_type).unwrap_or("<non-UTF8>")
        )),
    }
}

// ---------------------------------------------------------------------------
// VP8X extended-format decoder
// ---------------------------------------------------------------------------

/// Decode a VP8X extended WebP file.
///
/// ## VP8X chunk layout (10 bytes of data)
///
/// ```text
/// Byte 0-3  flags (u32 LE):
///   bit 1 = ICC profile present
///   bit 2 = animation (ANIM/ANMF chunks follow; we return an error)
///   bit 3 = Exif metadata present
///   bit 4 = XMP metadata present
///   bit 5 = alpha channel present
/// Byte 4-6  reserved (3 bytes)
/// Byte 7-9  canvas_width_minus_1  (3 bytes LE)
/// Byte 10-12 canvas_height_minus_1 (3 bytes LE)
/// ```
///
/// After the VP8X chunk, sub-chunks appear in any order.  We scan them all:
///
/// - `ICCP` / `EXIF` / `XMP ` — metadata we skip silently.
/// - `ALPH` — lossless alpha plane; decoded and merged with VP8 color.
/// - `VP8L` — lossless image (full ARGB); return immediately.
/// - `VP8 ` — lossy image (RGB only); merge with ALPH if present.
/// - `ANIM` / `ANMF` — animated WebP; return an error.
fn decode_vp8x(bytes: &[u8]) -> Result<PixelContainer, String> {
    // VP8X chunk is at offset 12; size field at 16; data starts at 20.
    // Minimum VP8X data size = 10 bytes.
    if bytes.len() < 30 {
        return Err("WebP: VP8X chunk too short".to_string());
    }
    let vp8x_size = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;
    if vp8x_size < 10 {
        return Err(format!("WebP: VP8X chunk data too small ({vp8x_size} bytes, need 10)"));
    }
    let flags = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
    let has_animation = (flags >> 1) & 1 != 0; // bit 1 = ICC, bit 2 = anim in spec
    // Spec bit layout (zero-indexed from LSB):
    //   0 = reserved, 1 = ICC, 2 = alpha, 3 = Exif, 4 = XMP, 5 = animation
    // libwebp uses: bit 1=ICC, bit 2=animation, bit 3=Exif, bit 4=XMP, bit 5=alpha
    // We check any animation bit to be safe.
    let has_animation_safe = (flags & 0b0000_0010) != 0 || (flags & 0b0010_0000) != 0;
    let _ = has_animation;
    if has_animation_safe {
        return Err("WebP: animated WebP (ANIM/ANMF) is not supported".to_string());
    }

    // Scan sub-chunks starting after the VP8X chunk.
    // VP8X chunk ends at: 12 (chunk header offset) + 8 (header size) + vp8x_size (padded).
    let vp8x_padded = vp8x_size + (vp8x_size & 1); // pad to even byte boundary
    let mut pos = 12usize + 8 + vp8x_padded;

    let mut alph_data: Option<Vec<u8>> = None;
    let mut pixels: Option<PixelContainer> = None;

    while pos + 8 <= bytes.len() {
        let ctype = &bytes[pos..pos + 4];
        let csize = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let cdata_start = pos + 8;
        let cdata_end = cdata_start + csize;
        if cdata_end > bytes.len() {
            return Err(format!(
                "WebP VP8X: sub-chunk {:?} truncated (need {} bytes, have {})",
                std::str::from_utf8(ctype).unwrap_or("<?>"),
                csize,
                bytes.len() - cdata_start
            ));
        }
        let cdata = &bytes[cdata_start..cdata_end];

        match ctype {
            // Metadata chunks we don't need for pixel decoding — skip silently.
            b"ICCP" | b"EXIF" | b"XMP " => {}

            b"ANIM" | b"ANMF" => {
                return Err("WebP: animated WebP (ANIM/ANMF) is not supported".to_string());
            }

            b"ALPH" => {
                // ALPH chunk: 1-byte header followed by compressed alpha data.
                // Header bits [1:0]: compression method (0=uncompressed, 1=VP8L)
                if cdata.is_empty() {
                    return Err("WebP: ALPH chunk is empty".to_string());
                }
                alph_data = Some(cdata.to_vec());
            }

            b"VP8L" => {
                // Full lossless image — includes alpha; ignore ALPH if present.
                pixels = Some(vp8l::decode(cdata)?);
            }

            b"VP8 " => {
                // Lossy image — RGB only; alpha will be patched in from ALPH.
                pixels = Some(vp8::decode(cdata)?);
            }

            _ => {
                // Unknown chunk — VP8X files can legally have extra chunks.
                // Silently skip to stay forward-compatible.
            }
        }

        // Advance past this chunk (chunks are even-aligned).
        let padded = csize + (csize & 1);
        pos = cdata_start + padded;
    }

    let mut px = pixels.ok_or_else(|| {
        "WebP VP8X: no VP8L or VP8 image chunk found".to_string()
    })?;

    // If we have a separate alpha plane (ALPH chunk) and a VP8 (lossy) image,
    // decode the alpha and write it into the A channel of every pixel.
    if let Some(alph) = alph_data {
        // Only merge alpha when image came from VP8 (lossy); VP8L already has alpha.
        // (We can tell because VP8L sets alpha from bitstream; VP8 always outputs A=255.)
        // We unconditionally try to apply alpha if an ALPH chunk was present.
        apply_alph_chunk(&mut px, &alph)?;
    }

    Ok(px)
}

/// Decode the ALPH chunk and write alpha values into `pixels`.
///
/// ## ALPH chunk format
///
/// ```text
/// Byte 0 (bitfield):
///   bits [1:0]: compression  0 = no compression  1 = VP8L lossless
///   bits [3:2]: filter       (ignored for decoding correctness)
///   bits [5:4]: pre-processing (ignored)
/// Remaining bytes: compressed alpha data
/// ```
///
/// When compression = 1 (VP8L): the data is a VP8L bitstream encoding a
/// grayscale `width × height` image.  The spec stores alpha in the **green**
/// channel of that image.
///
/// When compression = 0: the data is a raw `width × height` byte array.
fn apply_alph_chunk(pixels: &mut PixelContainer, alph: &[u8]) -> Result<(), String> {
    if alph.is_empty() {
        return Err("WebP: ALPH chunk has no header byte".to_string());
    }
    let method = alph[0] & 0x03;
    let alpha_data = &alph[1..];

    let alpha_values: Vec<u8> = match method {
        0 => {
            // Uncompressed: raw bytes, one per pixel, row-major.
            let expected = pixels.width as usize * pixels.height as usize;
            if alpha_data.len() < expected {
                return Err(format!(
                    "WebP ALPH: uncompressed alpha too short ({} bytes, need {expected})",
                    alpha_data.len()
                ));
            }
            alpha_data[..expected].to_vec()
        }
        1 => {
            // VP8L-compressed alpha: decode lossless image, extract G channel.
            
            vp8l::decode_as_alpha(alpha_data, pixels.width, pixels.height)?
        }
        _ => {
            return Err(format!("WebP ALPH: unknown compression method {method}"));
        }
    };

    // Write the decoded alpha values into every pixel's A channel.
    let total = pixels.width as usize * pixels.height as usize;
    for i in 0..total.min(alpha_values.len()) {
        pixels.data[i * 4 + 3] = alpha_values[i];
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::PixelContainer;

    // ── Version ───────────────────────────────────────────────────────────────

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.3.8");
    }

    // ── WebPCodec ─────────────────────────────────────────────────────────────

    #[test]
    fn mime_type_is_webp() {
        assert_eq!(WebPCodec::new(75, true).mime_type(), "image/webp");
    }

    #[test]
    fn codec_encode_decode_roundtrip() {
        let mut pixels = PixelContainer::new(4, 4);
        pixels.fill(128, 64, 32, 200);
        let codec = WebPCodec::new(90, true);
        let bytes = codec.encode(&pixels);
        let decoded = codec.decode(&bytes).unwrap();
        assert_eq!(decoded.data, pixels.data);
    }

    // ── RIFF magic bytes ──────────────────────────────────────────────────────

    #[test]
    fn riff_magic_bytes() {
        let pixels = PixelContainer::new(4, 4);
        let bytes = encode_webp_lossless(&pixels);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WEBP");
    }

    // ── VP8L signature byte ───────────────────────────────────────────────────

    #[test]
    fn vp8l_signature_byte() {
        let pixels = PixelContainer::new(4, 4);
        let bytes = encode_webp_lossless(&pixels);
        // VP8L chunk: bytes[12..16] = b"VP8L", bytes[16..20] = chunk size, bytes[20] = 0x2F
        assert_eq!(&bytes[12..16], b"VP8L");
        assert_eq!(bytes[20], 0x2F);
    }

    // ── Round-trip tests ──────────────────────────────────────────────────────

    #[test]
    fn round_trip_solid_color() {
        let mut pixels = PixelContainer::new(4, 4);
        for y in 0..4u32 {
            for x in 0..4u32 {
                pixels.set_pixel(x, y, 200, 100, 50, 255);
            }
        }
        let encoded = encode_webp_lossless(&pixels);
        let decoded = decode_webp(&encoded).unwrap();
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
        assert_eq!(decoded.data, pixels.data);
    }

    #[test]
    fn round_trip_gradient() {
        let mut pixels = PixelContainer::new(8, 8);
        for y in 0..8u32 {
            for x in 0..8u32 {
                pixels.set_pixel(x, y, (x * 30) as u8, (y * 30) as u8, 128, 255);
            }
        }
        let encoded = encode_webp_lossless(&pixels);
        let decoded = decode_webp(&encoded).unwrap();
        assert_eq!(decoded.width, 8);
        assert_eq!(decoded.height, 8);
        assert_eq!(decoded.data, pixels.data);
    }

    #[test]
    fn round_trip_1x1() {
        let mut pixels = PixelContainer::new(1, 1);
        pixels.set_pixel(0, 0, 255, 128, 64, 200);
        let encoded = encode_webp_lossless(&pixels);
        let decoded = decode_webp(&encoded).unwrap();
        assert_eq!(decoded.pixel_at(0, 0), (255, 128, 64, 200));
    }

    #[test]
    fn round_trip_transparent() {
        let pixels = PixelContainer::new(4, 4); // all zeros (transparent black)
        let encoded = encode_webp_lossless(&pixels);
        let decoded = decode_webp(&encoded).unwrap();
        assert_eq!(decoded.data, pixels.data);
    }

    #[test]
    fn round_trip_all_channels_varied() {
        let mut pixels = PixelContainer::new(4, 1);
        pixels.set_pixel(0, 0, 10, 20, 30, 0);
        pixels.set_pixel(1, 0, 10, 20, 30, 85);
        pixels.set_pixel(2, 0, 10, 20, 30, 170);
        pixels.set_pixel(3, 0, 10, 20, 30, 255);
        let encoded = encode_webp_lossless(&pixels);
        let decoded = decode_webp(&encoded).unwrap();
        assert_eq!(decoded.data, pixels.data);
    }

    // ── Decode error cases ────────────────────────────────────────────────────

    #[test]
    fn decode_error_bad_magic() {
        let result = decode_webp(b"this is not a webp file at all!!");
        assert!(result.is_err());
    }

    #[test]
    fn decode_error_too_short() {
        let result = decode_webp(&[0u8; 8]);
        assert!(result.is_err());
    }

    // ── VP8X extended container ───────────────────────────────────────────────

    /// Build a minimal valid VP8X container that wraps a VP8L payload.
    ///
    /// Layout:
    /// ```
    /// RIFF <file_size> WEBP
    ///   VP8X <10> <flags=0> <reserved=0x000000> <width-1 LE 3B> <height-1 LE 3B>
    ///   VP8L <n>  <vp8l_bytes>
    /// ```
    fn build_vp8x_with_vp8l(vp8l_bytes: &[u8], width: u32, height: u32) -> Vec<u8> {
        let mut out = Vec::new();

        // VP8X data (10 bytes): flags=0, reserved=0, canvas w-1, canvas h-1
        let w_m1 = width - 1;
        let h_m1 = height - 1;
        let vp8x_data: Vec<u8> = {
            let mut v = vec![0u8; 10];
            // flags (4 bytes LE) = 0
            // reserved (3 bytes) = 0
            // canvas width-1 (3 bytes LE)
            v[4] = (w_m1 & 0xFF) as u8;
            v[5] = ((w_m1 >> 8) & 0xFF) as u8;
            v[6] = ((w_m1 >> 16) & 0xFF) as u8;
            // canvas height-1 (3 bytes LE)
            v[7] = (h_m1 & 0xFF) as u8;
            v[8] = ((h_m1 >> 8) & 0xFF) as u8;
            v[9] = ((h_m1 >> 16) & 0xFF) as u8;
            v
        };

        // VP8X chunk
        out.extend_from_slice(b"VP8X");
        out.extend_from_slice(&(vp8x_data.len() as u32).to_le_bytes());
        out.extend_from_slice(&vp8x_data);

        // VP8L chunk
        out.extend_from_slice(b"VP8L");
        out.extend_from_slice(&(vp8l_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(vp8l_bytes);
        if !vp8l_bytes.len().is_multiple_of(2) {
            out.push(0); // padding
        }

        // Build RIFF wrapper
        let riff_size = (4 + out.len()) as u32; // WEBP fourCC + all chunks
        let mut riff = Vec::new();
        riff.extend_from_slice(b"RIFF");
        riff.extend_from_slice(&riff_size.to_le_bytes());
        riff.extend_from_slice(b"WEBP");
        riff.extend_from_slice(&out);
        riff
    }

    #[test]
    fn vp8x_wrapping_vp8l_round_trips() {
        // Encode a solid 4×4 image as VP8L, then wrap it in a VP8X container.
        let mut pixels = PixelContainer::new(4, 4);
        pixels.fill(100, 150, 200, 255);
        let raw_vp8l = encode_webp_lossless(&pixels);
        // The VP8L chunk payload is bytes[20..] of the VP8L-only file.
        let vp8l_payload = &raw_vp8l[20..];

        let vp8x_file = build_vp8x_with_vp8l(vp8l_payload, 4, 4);
        let decoded = decode_webp(&vp8x_file).unwrap();
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
        assert_eq!(decoded.data, pixels.data, "VP8X+VP8L round-trip must be pixel-exact");
    }

    #[test]
    fn vp8x_with_metadata_chunks_skipped() {
        // VP8X container with ICCP + EXIF chunks before the VP8L data.
        let mut pixels = PixelContainer::new(2, 2);
        pixels.fill(50, 100, 150, 200);
        let raw_vp8l = encode_webp_lossless(&pixels);
        let vp8l_payload = &raw_vp8l[20..];

        // VP8X data: exactly 10 bytes per spec
        //   4 bytes flags (LE), 3 bytes canvas_width-1 (LE), 3 bytes canvas_height-1 (LE)
        let vp8x_data = vec![
            0u8, 0, 0, 0,  // flags = 0 (no ICC, no anim, no alpha, etc.)
            1, 0, 0,        // canvas width-1 = 1  (LE 24-bit)
            1, 0, 0,        // canvas height-1 = 1 (LE 24-bit)
        ];
        let iccp_data = b"fakeiccdata";
        let exif_data = b"fakeexifdata";

        let mut chunks: Vec<u8> = Vec::new();
        // VP8X
        chunks.extend_from_slice(b"VP8X");
        chunks.extend_from_slice(&(vp8x_data.len() as u32).to_le_bytes());
        chunks.extend_from_slice(&vp8x_data);
        // ICCP
        chunks.extend_from_slice(b"ICCP");
        chunks.extend_from_slice(&(iccp_data.len() as u32).to_le_bytes());
        chunks.extend_from_slice(iccp_data);
        if !iccp_data.len().is_multiple_of(2) { chunks.push(0); }
        // EXIF
        chunks.extend_from_slice(b"EXIF");
        chunks.extend_from_slice(&(exif_data.len() as u32).to_le_bytes());
        chunks.extend_from_slice(exif_data);
        if !exif_data.len().is_multiple_of(2) { chunks.push(0); }
        // VP8L
        chunks.extend_from_slice(b"VP8L");
        chunks.extend_from_slice(&(vp8l_payload.len() as u32).to_le_bytes());
        chunks.extend_from_slice(vp8l_payload);
        if !vp8l_payload.len().is_multiple_of(2) { chunks.push(0); }

        let mut riff = Vec::new();
        riff.extend_from_slice(b"RIFF");
        riff.extend_from_slice(&((4 + chunks.len()) as u32).to_le_bytes());
        riff.extend_from_slice(b"WEBP");
        riff.extend_from_slice(&chunks);

        let decoded = decode_webp(&riff).unwrap();
        assert_eq!(decoded.data, pixels.data, "metadata chunks must be silently skipped");
    }

    #[test]
    fn vp8x_anim_returns_error() {
        // Build a VP8X container with animation flag set — must return Err.
        let mut vp8x_data = vec![0u8; 10];
        vp8x_data[0] = 0b0000_0010; // animation bit (bit 1 in flags byte 0)

        let mut chunks: Vec<u8> = Vec::new();
        chunks.extend_from_slice(b"VP8X");
        chunks.extend_from_slice(&(vp8x_data.len() as u32).to_le_bytes());
        chunks.extend_from_slice(&vp8x_data);

        let mut riff = Vec::new();
        riff.extend_from_slice(b"RIFF");
        riff.extend_from_slice(&((4 + chunks.len()) as u32).to_le_bytes());
        riff.extend_from_slice(b"WEBP");
        riff.extend_from_slice(&chunks);

        let result = decode_webp(&riff);
        assert!(result.is_err(), "animated WebP must return an error");
        assert!(result.unwrap_err().contains("anim"), "error should mention animation");
    }

    // ── VP8 lossy ─────────────────────────────────────────────────────────────

    #[test]
    fn encode_webp_produces_riff_header() {
        let pixels = PixelContainer::new(4, 4);
        let bytes = encode_webp(&pixels, 75);
        assert_eq!(&bytes[0..4], b"RIFF", "must start with RIFF");
        assert_eq!(&bytes[8..12], b"WEBP", "must have WEBP fourCC");
    }

    #[test]
    fn encode_webp_produces_vp8_chunk() {
        let pixels = PixelContainer::new(4, 4);
        let bytes = encode_webp(&pixels, 75);
        assert_eq!(&bytes[12..16], b"VP8 ", "chunk type must be VP8 ");
    }

    #[test]
    fn round_trip_lossy_solid() {
        // Solid-colour image — DC prediction + skip residuals should be ±5
        let mut pixels = PixelContainer::new(16, 16);
        for y in 0..16u32 {
            for x in 0..16u32 {
                pixels.set_pixel(x, y, 180, 180, 180, 255);
            }
        }
        let bytes = encode_webp(&pixels, 75);
        let decoded = decode_webp(&bytes).expect("VP8 decode failed");
        assert_eq!(decoded.width, 16);
        assert_eq!(decoded.height, 16);
        for y in 0..16u32 {
            for x in 0..16u32 {
                let (r, _, _, _) = decoded.pixel_at(x, y);
                let orig_luma = 180i32;
                let dec_luma  = r as i32; // grey image: R≈G≈B
                assert!(
                    (dec_luma - orig_luma).abs() <= 5,
                    "pixel ({x},{y}): expected ~{orig_luma}, got {dec_luma}"
                );
            }
        }
    }

    #[test]
    fn round_trip_lossy_quality_100() {
        // quality=100 → qp=0 → step=4 → max error ≤ 2
        let mut pixels = PixelContainer::new(16, 16);
        for y in 0..16u32 {
            for x in 0..16u32 {
                pixels.set_pixel(x, y, 200, 200, 200, 255);
            }
        }
        let bytes = encode_webp(&pixels, 100);
        let decoded = decode_webp(&bytes).expect("VP8 decode failed");
        for y in 0..16u32 {
            for x in 0..16u32 {
                let (r, _, _, _) = decoded.pixel_at(x, y);
                assert!(
                    (r as i32 - 200).abs() <= 2,
                    "quality=100 round-trip error too large at ({x},{y}): got {r}"
                );
            }
        }
    }

    #[test]
    fn round_trip_lossy_color() {
        // Non-grey solid color: (R=200, G=80, B=40) → significant Cb and Cr residuals.
        // Tolerance: ±15 per channel (accounts for YCbCr quantization spread into RGB).
        let mut pixels = PixelContainer::new(16, 16);
        for y in 0..16u32 {
            for x in 0..16u32 {
                pixels.set_pixel(x, y, 200, 80, 40, 255);
            }
        }
        let bytes = encode_webp(&pixels, 75);
        let decoded = decode_webp(&bytes).expect("VP8 color decode failed");
        assert_eq!(decoded.width, 16);
        assert_eq!(decoded.height, 16);
        for y in 0..16u32 {
            for x in 0..16u32 {
                let (r, g, b, a) = decoded.pixel_at(x, y);
                assert_eq!(a, 255);
                assert!((r as i32 - 200).abs() <= 15, "R error at ({x},{y}): got {r}");
                assert!((g as i32 -  80).abs() <= 15, "G error at ({x},{y}): got {g}");
                assert!((b as i32 -  40).abs() <= 15, "B error at ({x},{y}): got {b}");
            }
        }
    }

    #[test]
    fn decode_error_truncated() {
        let mut fake = vec![0u8; 20];
        fake[0..4].copy_from_slice(b"RIFF");
        fake[4..8].copy_from_slice(&12u32.to_le_bytes());
        fake[8..12].copy_from_slice(b"WEBP");
        fake[12..16].copy_from_slice(b"VP8 ");
        // chunk_size = 100, but we only provide 4 bytes
        fake[16..20].copy_from_slice(&100u32.to_le_bytes());
        let result = decode_webp(&fake);
        assert!(result.is_err(), "truncated VP8 frame should return Err");
    }

    #[test]
    fn decode_unknown_chunk_returns_err() {
        let mut fake = vec![0u8; 24];
        fake[0..4].copy_from_slice(b"RIFF");
        fake[4..8].copy_from_slice(&16u32.to_le_bytes());
        fake[8..12].copy_from_slice(b"WEBP");
        fake[12..16].copy_from_slice(b"UNKN");
        fake[16..20].copy_from_slice(&4u32.to_le_bytes());
        let result = decode_webp(&fake);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown chunk"));
    }
}
