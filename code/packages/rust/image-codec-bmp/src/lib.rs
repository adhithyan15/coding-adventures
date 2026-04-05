// # image-codec-bmp
//
// BMP (Bitmap) image encoder and decoder for 32-bit BGRA images.
//
// ## File Structure
//
// A BMP file is a fixed-size 54-byte header followed by raw pixel data:
//
//   ┌──────────────────────────────────────────┐
//   │  BITMAPFILEHEADER  (14 bytes, offset 0)  │
//   ├──────────────────────────────────────────┤
//   │  BITMAPINFOHEADER  (40 bytes, offset 14) │
//   ├──────────────────────────────────────────┤
//   │  Pixel data (width * height * 4 bytes)   │
//   └──────────────────────────────────────────┘
//
// All integers are little-endian.
//
// ## Pixel Format
//
// BMP stores pixels in BGRA order (blue first, then green, red, alpha).
// Our PixelContainer stores pixels in RGBA order. The only transform is
// swapping bytes 0 and 2 (R ↔ B) for each pixel.
//
// ## Top-Down vs Bottom-Up
//
// The original BMP format is bottom-up: the last row of pixels appears first
// in the file. A negative biHeight signals top-down layout, which matches our
// PixelContainer. We always write negative biHeight, so no row reversal is
// needed during encode. The decoder handles both variants.

use pixel_container::{ImageCodec, PixelContainer};

// ---------------------------------------------------------------------------
// BmpCodec
// ---------------------------------------------------------------------------

/// BMP image encoder and decoder.
///
/// Encodes and decodes 32-bit BGRA BMP files. The encoded format is:
/// - `BITMAPFILEHEADER` (14 bytes)
/// - `BITMAPINFOHEADER` (40 bytes)
/// - Pixel data: BGRA, top-down, `width * height * 4` bytes
pub struct BmpCodec;

impl ImageCodec for BmpCodec {
    fn mime_type(&self) -> &'static str {
        "image/bmp"
    }

    fn encode(&self, container: &PixelContainer) -> Vec<u8> {
        encode_bmp_impl(container)
    }

    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String> {
        decode_bmp_impl(bytes)
    }
}

// ---------------------------------------------------------------------------
// Convenience functions
// ---------------------------------------------------------------------------

/// Encode a `PixelContainer` to BMP bytes.
///
/// # Examples
///
/// ```
/// use pixel_container::PixelContainer;
/// use image_codec_bmp::encode_bmp;
///
/// let mut buf = PixelContainer::new(2, 2);
/// buf.set_pixel(0, 0, 255, 0, 0, 255); // red
/// let bmp_bytes = encode_bmp(&buf);
/// assert_eq!(&bmp_bytes[0..2], b"BM");
/// ```
pub fn encode_bmp(container: &PixelContainer) -> Vec<u8> {
    encode_bmp_impl(container)
}

/// Decode BMP bytes into a `PixelContainer`.
///
/// # Errors
///
/// Returns `Err` if the bytes are not a valid 32-bit BGRA BMP file.
///
/// # Examples
///
/// ```
/// use pixel_container::PixelContainer;
/// use image_codec_bmp::{encode_bmp, decode_bmp};
///
/// let mut buf = PixelContainer::new(2, 2);
/// buf.fill(100, 150, 200, 255);
/// let encoded = encode_bmp(&buf);
/// let decoded = decode_bmp(&encoded).unwrap();
/// assert_eq!(decoded.width, 2);
/// assert_eq!(decoded.height, 2);
/// assert_eq!(decoded.pixel_at(1, 1), (100, 150, 200, 255));
/// ```
pub fn decode_bmp(bytes: &[u8]) -> Result<PixelContainer, String> {
    decode_bmp_impl(bytes)
}

// ---------------------------------------------------------------------------
// Encode implementation
// ---------------------------------------------------------------------------

fn encode_bmp_impl(c: &PixelContainer) -> Vec<u8> {
    // Total file size = 54-byte header + pixel data.
    // Use usize arithmetic throughout to prevent u32 overflow in release mode.
    let pixel_bytes = (c.width as usize)
        .checked_mul(c.height as usize)
        .and_then(|n| n.checked_mul(4))
        .expect("BMP encode: image dimensions overflow usize");
    let file_size   = 54usize.checked_add(pixel_bytes)
        .expect("BMP encode: file size overflow usize");
    let mut out     = Vec::with_capacity(file_size);

    // --- BITMAPFILEHEADER (14 bytes) ---
    //
    // bfType: 'BM' as little-endian u16 = [0x42, 0x4D]
    out.extend_from_slice(b"BM");
    // bfSize: total file size
    out.extend_from_slice(&(file_size as u32).to_le_bytes());
    // bfReserved1, bfReserved2: both zero
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    // bfOffBits: byte offset to pixel data (always 54 in our variant)
    out.extend_from_slice(&54u32.to_le_bytes());

    // --- BITMAPINFOHEADER (40 bytes) ---
    //
    // biSize: size of this struct
    out.extend_from_slice(&40u32.to_le_bytes());
    // biWidth: image width in pixels (positive)
    out.extend_from_slice(&(c.width as i32).to_le_bytes());
    // biHeight: NEGATIVE → top-down scanlines (first row = top of image)
    // A positive biHeight would mean bottom-up, requiring row reversal.
    out.extend_from_slice(&(-(c.height as i32)).to_le_bytes());
    // biPlanes: always 1
    out.extend_from_slice(&1u16.to_le_bytes());
    // biBitCount: 32 bits per pixel (BGRA8)
    out.extend_from_slice(&32u16.to_le_bytes());
    // biCompression: 0 = BI_RGB (no compression)
    out.extend_from_slice(&0u32.to_le_bytes());
    // biSizeImage: size of pixel data
    out.extend_from_slice(&(pixel_bytes as u32).to_le_bytes());
    // biXPelsPerMeter, biYPelsPerMeter: 0 (not used)
    out.extend_from_slice(&0i32.to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes());
    // biClrUsed, biClrImportant: 0 (no colour table)
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());

    // --- Pixel Data ---
    //
    // BMP pixel order is BGRA; PixelContainer is RGBA.
    // Swap bytes 0 and 2 (R ↔ B) per pixel.
    for y in 0..c.height {
        for x in 0..c.width {
            let (r, g, b, a) = c.pixel_at(x, y);
            out.push(b); // Blue first
            out.push(g);
            out.push(r); // Red third
            out.push(a);
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Decode implementation
// ---------------------------------------------------------------------------

fn decode_bmp_impl(bytes: &[u8]) -> Result<PixelContainer, String> {
    // Minimum: 54-byte header must be present.
    if bytes.len() < 54 {
        return Err("BMP: file too short".into());
    }

    // Verify magic 'BM' (0x42 0x4D).
    if &bytes[0..2] != b"BM" {
        return Err("BMP: invalid magic".into());
    }

    // Read pixel data offset from BITMAPFILEHEADER (bytes 10–13).
    let pixel_offset = read_u32_le(bytes, 10) as usize;

    // Pixel data must start after the standard 54-byte header.
    if pixel_offset < 54 {
        return Err("BMP: pixel offset is before end of header".into());
    }

    // Read dimensions from BITMAPINFOHEADER (bytes 18–25).
    let bi_width  = read_i32_le(bytes, 18);
    let bi_height = read_i32_le(bytes, 22);

    // Width must be positive. Height may be negative (top-down) or positive
    // (bottom-up). We take the absolute value and track direction separately.
    // Reject i32::MIN: unsigned_abs() would return 2^31, causing overflow.
    if bi_width <= 0 {
        return Err("BMP: invalid width".into());
    }
    if bi_height == i32::MIN {
        return Err("BMP: invalid height".into());
    }
    let width    = bi_width as u32;
    let height   = bi_height.unsigned_abs();
    let top_down = bi_height < 0; // negative biHeight → top-down

    if height == 0 {
        return Err("BMP: invalid height".into());
    }

    // Read biBitCount (bytes 28–29): only 32-bit BGRA is supported.
    let bit_count = read_u16_le(bytes, 28);
    if bit_count != 32 {
        return Err(format!(
            "BMP: unsupported bit depth {bit_count}, only 32-bit BGRA supported"
        ));
    }

    // Read biCompression (bytes 30–33): only BI_RGB (0) is supported.
    let compression = read_u32_le(bytes, 30);
    if compression != 0 {
        return Err(format!(
            "BMP: unsupported compression {compression}, only BI_RGB (0) supported"
        ));
    }

    // Verify pixel data fits in the file.
    // Use checked arithmetic to prevent overflow in the bounds check.
    let pixel_bytes = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or("BMP: image dimensions overflow")?;
    let pixel_end = pixel_offset
        .checked_add(pixel_bytes)
        .ok_or("BMP: pixel data range overflow")?;
    if bytes.len() < pixel_end {
        return Err("BMP: pixel data truncated".into());
    }

    // Read pixels: BGRA → RGBA. Handle scanline direction.
    let mut container = PixelContainer::new(width, height);
    for row in 0..height {
        // In top-down BMP, row 0 in the file is row 0 of the image (top).
        // In bottom-up BMP, row 0 in the file is the LAST row of the image.
        let dest_row = if top_down {
            row
        } else {
            height - 1 - row
        };
        for col in 0..width {
            // Use usize arithmetic to avoid u32 overflow in release mode.
            let file_idx = pixel_offset + (row as usize * width as usize + col as usize) * 4;
            let b = bytes[file_idx];
            let g = bytes[file_idx + 1];
            let r = bytes[file_idx + 2];
            let a = bytes[file_idx + 3];
            container.set_pixel(col, dest_row, r, g, b, a);
        }
    }

    Ok(container)
}

// ---------------------------------------------------------------------------
// Little-endian read helpers
// ---------------------------------------------------------------------------

fn read_u16_le(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_i32_le(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pixel_container::PixelContainer;

    // Helper: create a small solid-colour container.
    fn solid(w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) -> PixelContainer {
        let mut buf = PixelContainer::new(w, h);
        buf.fill(r, g, b, a);
        buf
    }

    // --- Header structure ---

    #[test]
    fn encoded_magic_is_bm() {
        let bmp = encode_bmp(&solid(4, 4, 0, 0, 0, 255));
        assert_eq!(&bmp[0..2], b"BM");
    }

    #[test]
    fn encoded_file_size_correct() {
        let bmp = encode_bmp(&solid(4, 4, 0, 0, 0, 255));
        let file_size = u32::from_le_bytes([bmp[2], bmp[3], bmp[4], bmp[5]]) as usize;
        assert_eq!(file_size, bmp.len());
        assert_eq!(file_size, 54 + 4 * 4 * 4); // 54 header + 64 pixels
    }

    #[test]
    fn encoded_pixel_offset_is_54() {
        let bmp = encode_bmp(&solid(2, 2, 0, 0, 0, 255));
        let off = u32::from_le_bytes([bmp[10], bmp[11], bmp[12], bmp[13]]);
        assert_eq!(off, 54);
    }

    #[test]
    fn encoded_biheight_is_negative() {
        // Negative biHeight = top-down layout.
        let bmp = encode_bmp(&solid(3, 5, 0, 0, 0, 255));
        let bi_h = i32::from_le_bytes([bmp[22], bmp[23], bmp[24], bmp[25]]);
        assert_eq!(bi_h, -5);
    }

    #[test]
    fn encoded_bit_count_is_32() {
        let bmp = encode_bmp(&solid(1, 1, 0, 0, 0, 255));
        let bc = u16::from_le_bytes([bmp[28], bmp[29]]);
        assert_eq!(bc, 32);
    }

    // --- Round-trip: encode then decode ---

    #[test]
    fn round_trip_solid_colour() {
        let original = solid(4, 4, 200, 100, 50, 255);
        let encoded  = encode_bmp(&original);
        let decoded  = decode_bmp(&encoded).unwrap();
        assert_eq!(decoded.width,  original.width);
        assert_eq!(decoded.height, original.height);
        assert_eq!(decoded.data,   original.data);
    }

    #[test]
    fn round_trip_checkerboard() {
        let mut original = PixelContainer::new(4, 4);
        for y in 0..4u32 {
            for x in 0..4u32 {
                if (x + y) % 2 == 0 {
                    original.set_pixel(x, y, 255, 255, 255, 255);
                } else {
                    original.set_pixel(x, y, 0, 0, 0, 255);
                }
            }
        }
        let decoded = decode_bmp(&encode_bmp(&original)).unwrap();
        assert_eq!(decoded.data, original.data);
    }

    #[test]
    fn round_trip_with_transparency() {
        let mut original = PixelContainer::new(2, 2);
        original.set_pixel(0, 0, 255, 0, 0, 128); // semi-transparent red
        original.set_pixel(1, 0, 0, 255, 0, 0);   // fully transparent green
        original.set_pixel(0, 1, 0, 0, 255, 200);
        original.set_pixel(1, 1, 100, 100, 100, 255);
        let decoded = decode_bmp(&encode_bmp(&original)).unwrap();
        assert_eq!(decoded.data, original.data);
    }

    // --- BGRA byte order in the file ---

    #[test]
    fn pixel_data_is_bgra_order() {
        // Single pixel: R=1, G=2, B=3, A=4
        let container = PixelContainer::from_data(1, 1, vec![1, 2, 3, 4]);
        let bmp = encode_bmp(&container);
        // Pixel data starts at offset 54.
        assert_eq!(bmp[54], 3); // B
        assert_eq!(bmp[55], 2); // G
        assert_eq!(bmp[56], 1); // R
        assert_eq!(bmp[57], 4); // A
    }

    // --- Decode errors ---

    #[test]
    fn decode_too_short_returns_error() {
        let result = decode_bmp(&[0x42, 0x4D, 0x00]); // only 3 bytes
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too short"));
    }

    #[test]
    fn decode_wrong_magic_returns_error() {
        let mut bmp = encode_bmp(&solid(2, 2, 0, 0, 0, 255));
        bmp[0] = 0xFF; // corrupt magic
        let result = decode_bmp(&bmp);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid magic"));
    }

    #[test]
    fn decode_unsupported_bit_depth_returns_error() {
        let mut bmp = encode_bmp(&solid(2, 2, 0, 0, 0, 255));
        // biBitCount is at offset 28–29; change to 24.
        bmp[28] = 24;
        bmp[29] = 0;
        let result = decode_bmp(&bmp);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported bit depth"));
    }

    // --- ImageCodec trait ---

    #[test]
    fn codec_mime_type() {
        assert_eq!(BmpCodec.mime_type(), "image/bmp");
    }

    #[test]
    fn codec_encode_decode_via_trait() {
        let original = solid(3, 3, 64, 128, 192, 255);
        let encoded  = BmpCodec.encode(&original);
        let decoded  = BmpCodec.decode(&encoded).unwrap();
        assert_eq!(decoded.data, original.data);
    }
}
