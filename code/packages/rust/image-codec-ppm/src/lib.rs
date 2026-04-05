// # image-codec-ppm
//
// PPM (Portable Pixmap) P6 image encoder and decoder.
//
// ## File Format
//
// PPM P6 is deliberately minimal — a few lines of ASCII header, then raw RGB:
//
//   P6\n
//   <width> <height>\n
//   255\n
//   <width * height * 3 raw bytes: R G B per pixel, row-major>
//
// There is no compression, no metadata, and no padding. Three bytes per pixel.
//
// ## Alpha Handling
//
// PPM has no alpha channel. During encode, the alpha byte is dropped.
// During decode, every pixel is set to A = 255 (fully opaque).
//
// ## Interoperability
//
// Files produced by this encoder are accepted by ImageMagick, ffmpeg, and
// any Netpbm tool. Files produced by those tools can be decoded here.

use pixel_container::{ImageCodec, PixelContainer};

// ---------------------------------------------------------------------------
// PpmCodec
// ---------------------------------------------------------------------------

/// PPM P6 image encoder and decoder.
pub struct PpmCodec;

impl ImageCodec for PpmCodec {
    fn mime_type(&self) -> &'static str {
        "image/x-portable-pixmap"
    }

    fn encode(&self, container: &PixelContainer) -> Vec<u8> {
        encode_ppm_impl(container)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_ppm_impl(bytes)
    }
}

// ---------------------------------------------------------------------------
// Convenience functions
// ---------------------------------------------------------------------------

/// Encode a `PixelContainer` to PPM P6 bytes.
///
/// Alpha is dropped during encoding (PPM has no alpha channel).
///
/// # Examples
///
/// ```
/// use pixel_container::PixelContainer;
/// use image_codec_ppm::encode_ppm;
///
/// let mut buf = PixelContainer::new(2, 1);
/// buf.set_pixel(0, 0, 255, 0, 0, 255);   // red
/// buf.set_pixel(1, 0, 0, 0, 255, 255);   // blue
/// let ppm = encode_ppm(&buf);
/// assert!(ppm.starts_with(b"P6\n"));
/// ```
pub fn encode_ppm(container: &PixelContainer) -> Vec<u8> {
    encode_ppm_impl(container)
}

/// Decode PPM P6 bytes into a `PixelContainer`.
///
/// Decoded pixels have A = 255 (PPM has no alpha channel).
///
/// # Errors
///
/// Returns `Err` if the bytes are not valid PPM P6 format.
///
/// # Examples
///
/// ```
/// use pixel_container::PixelContainer;
/// use image_codec_ppm::{encode_ppm, decode_ppm};
///
/// let mut buf = PixelContainer::new(3, 2);
/// buf.fill(128, 64, 32, 255);
/// let encoded = encode_ppm(&buf);
/// let decoded = decode_ppm(&encoded).unwrap();
/// assert_eq!(decoded.width, 3);
/// assert_eq!(decoded.height, 2);
/// // RGB is preserved; alpha is always 255 after decode.
/// assert_eq!(decoded.pixel_at(1, 1), (128, 64, 32, 255));
/// ```
pub fn decode_ppm(bytes: &[u8]) -> Result<PixelContainer, String> {
    decode_ppm_impl(bytes)
}

// ---------------------------------------------------------------------------
// Encode implementation
// ---------------------------------------------------------------------------

fn encode_ppm_impl(c: &PixelContainer) -> Vec<u8> {
    // Build the ASCII header: "P6\n<width> <height>\n255\n"
    let header = format!("P6\n{} {}\n255\n", c.width, c.height);
    let pixel_bytes = (c.width * c.height * 3) as usize;

    let mut out = Vec::with_capacity(header.len() + pixel_bytes);
    out.extend_from_slice(header.as_bytes());

    // Write three bytes per pixel (RGB, drop alpha).
    for y in 0..c.height {
        for x in 0..c.width {
            let (r, g, b, _a) = c.pixel_at(x, y);
            out.push(r);
            out.push(g);
            out.push(b);
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Decode implementation
// ---------------------------------------------------------------------------

fn decode_ppm_impl(bytes: &[u8]) -> Result<PixelContainer, String> {
    let mut pos = 0usize;

    // Read the magic token: must be "P6".
    let magic = read_token(bytes, &mut pos);
    if magic.as_deref() != Some("P6") {
        return Err("PPM: invalid magic, expected P6".into());
    }

    skip_whitespace_and_comments(bytes, &mut pos);
    let width = read_int(bytes, &mut pos).ok_or("PPM: invalid dimensions")?;
    skip_whitespace_and_comments(bytes, &mut pos);
    let height = read_int(bytes, &mut pos).ok_or("PPM: invalid dimensions")?;
    skip_whitespace_and_comments(bytes, &mut pos);
    let maxval = read_int(bytes, &mut pos).ok_or("PPM: invalid max value")?;

    if maxval != 255 {
        return Err(format!(
            "PPM: unsupported max value {maxval}, only 255 supported"
        ));
    }

    // Skip exactly one whitespace byte after the max value (the spec requires
    // exactly one byte of whitespace before the binary pixel data starts).
    if pos >= bytes.len() {
        return Err("PPM: pixel data truncated".into());
    }
    pos += 1; // skip the single whitespace byte

    // Verify we have enough pixel data.
    // Use checked_mul to prevent overflow (width and height are usize from user input).
    let pixel_count = width.checked_mul(height)
        .ok_or("PPM: image dimensions overflow")?;
    let needed = pixel_count.checked_mul(3)
        .ok_or("PPM: pixel data size overflow")?;
    // Guard against pos > bytes.len() before subtraction (unsigned underflow).
    if pos > bytes.len() || bytes.len() - pos < needed {
        return Err("PPM: pixel data truncated".into());
    }

    // Read RGB triples → RGBA with A=255.
    let data_cap = pixel_count.checked_mul(4)
        .ok_or("PPM: pixel data size overflow")?;
    let mut data = Vec::with_capacity(data_cap);
    for _ in 0..pixel_count {
        let r = bytes[pos];
        let g = bytes[pos + 1];
        let b = bytes[pos + 2];
        pos += 3;
        data.push(r);
        data.push(g);
        data.push(b);
        data.push(255); // alpha
    }

    Ok(PixelContainer::from_data(width as u32, height as u32, data))
}

// ---------------------------------------------------------------------------
// Parser helpers
// ---------------------------------------------------------------------------

/// Skip ASCII whitespace and '#'-prefixed comment lines.
fn skip_whitespace_and_comments(bytes: &[u8], pos: &mut usize) {
    loop {
        // Skip whitespace bytes (space, tab, CR, LF).
        while *pos < bytes.len() && bytes[*pos].is_ascii_whitespace() {
            *pos += 1;
        }
        // If the next character is '#', skip until end of line.
        if *pos < bytes.len() && bytes[*pos] == b'#' {
            while *pos < bytes.len() && bytes[*pos] != b'\n' {
                *pos += 1;
            }
        } else {
            break;
        }
    }
}

/// Read a whitespace-delimited ASCII token. Returns None if at end of data.
fn read_token(bytes: &[u8], pos: &mut usize) -> Option<String> {
    skip_whitespace_and_comments(bytes, pos);
    if *pos >= bytes.len() {
        return None;
    }
    let start = *pos;
    while *pos < bytes.len() && !bytes[*pos].is_ascii_whitespace() {
        *pos += 1;
    }
    String::from_utf8(bytes[start..*pos].to_vec()).ok()
}

/// Read a decimal integer token. Returns None if not a valid integer.
fn read_int(bytes: &[u8], pos: &mut usize) -> Option<usize> {
    read_token(bytes, pos)?.parse().ok()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pixel_container::PixelContainer;

    // --- Header format ---

    #[test]
    fn header_starts_with_p6() {
        let buf = PixelContainer::new(2, 2);
        let ppm = encode_ppm(&buf);
        assert!(ppm.starts_with(b"P6\n"));
    }

    #[test]
    fn header_contains_dimensions() {
        let buf = PixelContainer::new(640, 480);
        let ppm = encode_ppm(&buf);
        let header = std::str::from_utf8(&ppm[0..20]).unwrap_or("");
        assert!(header.contains("640"));
        assert!(header.contains("480"));
    }

    #[test]
    fn encoded_size_correct() {
        let buf = PixelContainer::new(3, 2);
        let ppm = encode_ppm(&buf);
        // header = "P6\n3 2\n255\n" = 11 bytes, pixel data = 3*2*3 = 18 bytes
        let header = b"P6\n3 2\n255\n";
        assert_eq!(ppm.len(), header.len() + 3 * 2 * 3);
    }

    // --- Alpha is dropped on encode ---

    #[test]
    fn alpha_channel_not_in_file() {
        let mut buf = PixelContainer::new(1, 1);
        buf.set_pixel(0, 0, 10, 20, 30, 128); // semi-transparent
        let ppm = encode_ppm(&buf);
        // After the header, we should have exactly 3 bytes.
        let header = b"P6\n1 1\n255\n";
        assert_eq!(ppm.len(), header.len() + 3);
        assert_eq!(ppm[header.len()], 10); // R
        assert_eq!(ppm[header.len() + 1], 20); // G
        assert_eq!(ppm[header.len() + 2], 30); // B
    }

    // --- Round-trip (opaque images only) ---

    #[test]
    fn round_trip_solid_colour() {
        let mut original = PixelContainer::new(5, 3);
        original.fill(100, 150, 200, 255);
        let encoded = encode_ppm(&original);
        let decoded = decode_ppm(&encoded).unwrap();
        assert_eq!(decoded.width, 5);
        assert_eq!(decoded.height, 3);
        for y in 0..3u32 {
            for x in 0..5u32 {
                let (r, g, b, a) = decoded.pixel_at(x, y);
                assert_eq!((r, g, b), (100, 150, 200));
                assert_eq!(a, 255);
            }
        }
    }

    #[test]
    fn round_trip_rgb_preserved() {
        let mut original = PixelContainer::new(2, 2);
        original.set_pixel(0, 0, 255, 0, 0, 255);   // red
        original.set_pixel(1, 0, 0, 255, 0, 255);   // green
        original.set_pixel(0, 1, 0, 0, 255, 255);   // blue
        original.set_pixel(1, 1, 128, 128, 128, 255); // grey
        let decoded = decode_ppm(&encode_ppm(&original)).unwrap();
        for y in 0..2u32 {
            for x in 0..2u32 {
                let (r1, g1, b1, _) = original.pixel_at(x, y);
                let (r2, g2, b2, a) = decoded.pixel_at(x, y);
                assert_eq!((r1, g1, b1), (r2, g2, b2));
                assert_eq!(a, 255);
            }
        }
    }

    // --- Decode handles comments ---

    #[test]
    fn decode_with_comment_in_header() {
        let ppm = b"P6\n# this is a comment\n4 4\n255\n";
        let pixels = vec![0u8; 4 * 4 * 3];
        let mut data = ppm.to_vec();
        data.extend_from_slice(&pixels);
        let result = decode_ppm(&data);
        assert!(result.is_ok());
        let c = result.unwrap();
        assert_eq!(c.width, 4);
        assert_eq!(c.height, 4);
    }

    // --- Decode errors ---

    #[test]
    fn decode_wrong_magic_returns_error() {
        let data = b"P3\n1 1\n255\n\x00\x00\x00";
        let result = decode_ppm(data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid magic"));
    }

    #[test]
    fn decode_unsupported_maxval_returns_error() {
        let data = b"P6\n1 1\n65535\n\x00\x00\x00\x00\x00\x00";
        let result = decode_ppm(data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported max value"));
    }

    #[test]
    fn decode_truncated_pixel_data_returns_error() {
        // Header says 4×4 = 16 pixels = 48 bytes, but we only give 3.
        let data = b"P6\n4 4\n255\n\x00\x00\x00";
        let result = decode_ppm(data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("truncated"));
    }

    // --- ImageCodec trait ---

    #[test]
    fn codec_mime_type() {
        assert_eq!(PpmCodec.mime_type(), "image/x-portable-pixmap");
    }

    #[test]
    fn codec_encode_decode_via_trait() {
        let mut original = PixelContainer::new(3, 3);
        original.fill(60, 120, 180, 255);
        let encoded = PpmCodec.encode(&original);
        let decoded  = PpmCodec.decode(&encoded).unwrap();
        assert_eq!(decoded.pixel_at(1, 1), (60, 120, 180, 255));
    }
}
