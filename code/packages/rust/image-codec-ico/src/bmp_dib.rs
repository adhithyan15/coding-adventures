//! BMP Device-Independent Bitmap (DIB) decoder for ICO frames.
//!
//! An ICO file embeds BMP data *without* the 14-byte `BITMAPFILEHEADER` that
//! a standalone `.bmp` file carries.  The data starts directly with the
//! 40-byte `BITMAPINFOHEADER`.
//!
//! ## Row order
//!
//! Windows bitmaps are stored **bottom-up** (first byte is the bottom row).
//! We reverse the rows before returning the pixel buffer so callers always
//! receive top-down data.
//!
//! ## ICO height encoding
//!
//! Inside an ICO, `biHeight = 2 × pixel_height`.  The DIB actually stores
//! two layers of equal height:
//!
//! 1. **XOR mask** — color/pixel data (bottom-up, pixel_height rows)
//! 2. **AND mask** — 1bpp transparency mask (bottom-up, pixel_height rows)
//!    - bit 0 = opaque pixel, bit 1 = transparent pixel
//!
//! For 32bpp images the AND mask is conventionally all zeros and the BGRA
//! alpha byte is the true transparency source.

// ── BITMAPINFOHEADER constants ─────────────────────────────────────────────

/// Expected value of `biSize` in the BITMAPINFOHEADER.
const BITMAPINFOHEADER_SIZE: u32 = 40;
/// BI_RGB: no compression.
const BI_RGB: u32 = 0;

// ── Public entry point ──────────────────────────────────────────────────────

/// Decode a BMP DIB from `data` (starting at the BITMAPINFOHEADER, no file
/// header) into an RGBA pixel buffer.
///
/// Returns `(width, height, rgba_pixels)` on success.
pub fn decode_bmp_dib(data: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if data.len() < BITMAPINFOHEADER_SIZE as usize {
        return Err("ICO: BMP DIB too short for BITMAPINFOHEADER".into());
    }

    // Parse BITMAPINFOHEADER fields (all little-endian).
    let bi_size = read_u32le(data, 0);
    if bi_size != BITMAPINFOHEADER_SIZE {
        return Err(format!(
            "ICO: unsupported BITMAPINFOHEADER size {} (expected 40)",
            bi_size
        ));
    }
    let bi_width = read_i32le(data, 4);
    let bi_height_raw = read_i32le(data, 8);
    let bi_bit_count = read_u16le(data, 14);
    let bi_compression = read_u32le(data, 16);
    let bi_clr_used = read_u32le(data, 32);

    if bi_compression != BI_RGB {
        return Err(format!(
            "ICO: compressed BMP DIB not supported (biCompression={})",
            bi_compression
        ));
    }

    // biHeight in ICO encodes 2× the actual image height.
    // Negative biHeight (top-down) is technically allowed but extremely rare
    // in practice; we treat biHeight > 0 as bottom-up (the standard ICO case).
    let is_bottom_up = bi_height_raw > 0;
    let bi_height = bi_height_raw.unsigned_abs(); // strip sign

    // Actual pixel dimensions.
    let pixel_width = bi_width.unsigned_abs() as usize;
    // biHeight in an ICO = 2 × pixel_height (XOR + AND stacked).
    let pixel_height = (bi_height / 2) as usize;

    if pixel_width == 0 || pixel_height == 0 {
        return Err("ICO: zero-dimension BMP DIB".into());
    }

    // Safety cap: no sane ICO has dimensions above 256×256.
    if pixel_width > 256 || pixel_height > 256 {
        return Err(format!(
            "ICO: BMP DIB dimensions {}×{} exceed maximum 256×256",
            pixel_width, pixel_height
        ));
    }

    // Palette count.
    //
    // Security: `bi_clr_used` comes from untrusted input.  Without capping it
    // at the theoretical maximum for the bit depth (`1 << biBitCount`), a
    // malicious file could set `bi_clr_used = 0xFFFF_FFFF` and trigger either:
    //   - a ~16 GiB `Vec::with_capacity` allocation (OOM DoS on 64-bit), or
    //   - a `usize` overflow in `palette_end` (silent wrap past the guard on
    //     32-bit, leading to an out-of-bounds read).
    //
    // We cap at the theoretical maximum, which is at most 256 entries (8bpp).
    let max_palette = if bi_bit_count <= 8 { 1usize << bi_bit_count } else { 0 };
    let palette_count = if bi_bit_count <= 8 {
        if bi_clr_used > 0 {
            let requested = bi_clr_used as usize;
            if requested > max_palette {
                return Err(format!(
                    "ICO: biClrUsed {} exceeds the maximum {} for {}bpp",
                    bi_clr_used, max_palette, bi_bit_count
                ));
            }
            requested
        } else {
            max_palette
        }
    } else {
        0
    };

    // Palette starts after the header.
    let palette_start = BITMAPINFOHEADER_SIZE as usize;
    // palette_count is at most 256 (8bpp), so palette_count * 4 ≤ 1024 —
    // no overflow possible.
    let palette_end = palette_start + palette_count * 4; // RGBQUAD = 4 bytes

    if data.len() < palette_end {
        return Err("ICO: BMP DIB truncated before palette".into());
    }

    // Parse RGBQUAD palette: entries are (blue, green, red, reserved).
    let mut palette: Vec<(u8, u8, u8)> = Vec::with_capacity(palette_count);
    for i in 0..palette_count {
        let base = palette_start + i * 4;
        palette.push((data[base + 2], data[base + 1], data[base])); // R, G, B
    }

    // XOR (color) data: rows from bottom to top, each row padded to 4-byte boundary.
    //
    // Use checked arithmetic for the stride even though the dimension cap above
    // (≤256×256) makes overflow impossible in practice.  Defence-in-depth.
    let xor_row_stride = row_stride_checked(pixel_width, bi_bit_count as usize)
        .ok_or_else(|| format!(
            "ICO: row stride overflow for {}bpp width {} (unreachable under 256px cap)",
            bi_bit_count, pixel_width
        ))?;
    let xor_size = xor_row_stride.checked_mul(pixel_height)
        .ok_or("ICO: XOR data size overflow")?;
    let xor_start = palette_end;
    let xor_end = xor_start.checked_add(xor_size)
        .ok_or("ICO: XOR end-offset overflow")?;

    if data.len() < xor_end {
        return Err("ICO: BMP DIB truncated in XOR pixel data".into());
    }
    let xor_data = &data[xor_start..xor_end];

    // AND (alpha) mask: 1bpp rows from bottom to top, 4-byte padded.
    // AND mask stride: pixel_width ≤ 256 → (256+31)/32*4 = 32 bytes max; no overflow.
    let and_row_stride = row_stride_checked(pixel_width, 1)
        .ok_or("ICO: AND mask stride overflow")?;
    let and_size = and_row_stride.checked_mul(pixel_height)
        .ok_or("ICO: AND mask size overflow")?;
    let and_start = xor_end;
    let and_end = and_start + and_size;

    // The AND mask is optional — some encoders omit it for 32bpp images.
    let has_and_mask = data.len() >= and_end;
    let and_data: &[u8] = if has_and_mask {
        &data[and_start..and_end]
    } else {
        &[]
    };

    // Build the RGBA output: top-down order (flip if bottom-up).
    let mut rgba: Vec<u8> = vec![0u8; pixel_width * pixel_height * 4];

    for row_idx in 0..pixel_height {
        // Source row index in the bottom-up BMP (row 0 of BMP = bottom of image).
        let src_row = if is_bottom_up {
            pixel_height - 1 - row_idx // flip to top-down
        } else {
            row_idx
        };

        let dst_row = row_idx;
        let dst_base = dst_row * pixel_width * 4;

        match bi_bit_count {
            1 => decode_row_1bpp(
                xor_data,
                src_row,
                xor_row_stride,
                &palette,
                and_data,
                and_row_stride,
                has_and_mask,
                &mut rgba[dst_base..dst_base + pixel_width * 4],
            ),
            4 => decode_row_4bpp(
                xor_data,
                src_row,
                xor_row_stride,
                &palette,
                and_data,
                and_row_stride,
                has_and_mask,
                &mut rgba[dst_base..dst_base + pixel_width * 4],
            ),
            8 => decode_row_8bpp(
                xor_data,
                src_row,
                xor_row_stride,
                &palette,
                and_data,
                and_row_stride,
                has_and_mask,
                &mut rgba[dst_base..dst_base + pixel_width * 4],
            ),
            24 => decode_row_24bpp(
                xor_data,
                src_row,
                xor_row_stride,
                and_data,
                and_row_stride,
                has_and_mask,
                &mut rgba[dst_base..dst_base + pixel_width * 4],
            ),
            32 => decode_row_32bpp(
                xor_data,
                src_row,
                xor_row_stride,
                and_data,
                and_row_stride,
                has_and_mask,
                &mut rgba[dst_base..dst_base + pixel_width * 4],
            ),
            bpp => {
                return Err(format!("ICO: unsupported bit depth {}", bpp));
            }
        }
    }

    Ok((pixel_width as u32, pixel_height as u32, rgba))
}

// ── Row decoders ──────────────────────────────────────────────────────────────

/// 1bpp row: each byte holds 8 pixels, MSB = leftmost pixel.
fn decode_row_1bpp(
    xor: &[u8],
    src_row: usize,
    stride: usize,
    palette: &[(u8, u8, u8)],
    and: &[u8],
    and_stride: usize,
    has_and: bool,
    dst: &mut [u8],
) {
    let row_start = src_row * stride;
    let pixel_count = dst.len() / 4;
    for px in 0..pixel_count {
        let byte = xor[row_start + px / 8];
        let bit = (byte >> (7 - (px % 8))) & 1;
        let (r, g, b) = if (bit as usize) < palette.len() {
            palette[bit as usize]
        } else {
            (0, 0, 0)
        };
        let a = if has_and {
            let abyte = and[src_row * and_stride + px / 8];
            let abit = (abyte >> (7 - (px % 8))) & 1;
            if abit != 0 { 0u8 } else { 255u8 }
        } else {
            255
        };
        dst[px * 4] = r;
        dst[px * 4 + 1] = g;
        dst[px * 4 + 2] = b;
        dst[px * 4 + 3] = a;
    }
}

/// 4bpp row: high nibble = left pixel, low nibble = right pixel.
fn decode_row_4bpp(
    xor: &[u8],
    src_row: usize,
    stride: usize,
    palette: &[(u8, u8, u8)],
    and: &[u8],
    and_stride: usize,
    has_and: bool,
    dst: &mut [u8],
) {
    let row_start = src_row * stride;
    let pixel_count = dst.len() / 4;
    for px in 0..pixel_count {
        let byte = xor[row_start + px / 2];
        let nibble = if px % 2 == 0 { (byte >> 4) & 0xF } else { byte & 0xF };
        let (r, g, b) = if (nibble as usize) < palette.len() {
            palette[nibble as usize]
        } else {
            (0, 0, 0)
        };
        let a = and_alpha(and, src_row, and_stride, px, has_and);
        dst[px * 4] = r;
        dst[px * 4 + 1] = g;
        dst[px * 4 + 2] = b;
        dst[px * 4 + 3] = a;
    }
}

/// 8bpp row: one byte per pixel (palette index).
fn decode_row_8bpp(
    xor: &[u8],
    src_row: usize,
    stride: usize,
    palette: &[(u8, u8, u8)],
    and: &[u8],
    and_stride: usize,
    has_and: bool,
    dst: &mut [u8],
) {
    let row_start = src_row * stride;
    let pixel_count = dst.len() / 4;
    for px in 0..pixel_count {
        let idx = xor[row_start + px] as usize;
        let (r, g, b) = if idx < palette.len() {
            palette[idx]
        } else {
            (0, 0, 0)
        };
        let a = and_alpha(and, src_row, and_stride, px, has_and);
        dst[px * 4] = r;
        dst[px * 4 + 1] = g;
        dst[px * 4 + 2] = b;
        dst[px * 4 + 3] = a;
    }
}

/// 24bpp row: 3 bytes per pixel in BGR order.
fn decode_row_24bpp(
    xor: &[u8],
    src_row: usize,
    stride: usize,
    and: &[u8],
    and_stride: usize,
    has_and: bool,
    dst: &mut [u8],
) {
    let row_start = src_row * stride;
    let pixel_count = dst.len() / 4;
    // Safety invariant: for pixel_count pixels at 24bpp, the XOR slice must
    // hold at least `row_start + pixel_count * 3` bytes.  The caller sets
    // dst.len() = pixel_width * 4 and stride = row_stride(pixel_width, 24),
    // so stride ≥ pixel_count * 3 and xor.len() = stride * pixel_height.
    debug_assert!(
        xor.len() >= row_start + pixel_count * 3,
        "decode_row_24bpp: xor slice too short (row_start={}, pixel_count={}, xor.len={})",
        row_start, pixel_count, xor.len()
    );
    for px in 0..pixel_count {
        let b = xor[row_start + px * 3];
        let g = xor[row_start + px * 3 + 1];
        let r = xor[row_start + px * 3 + 2];
        let a = and_alpha(and, src_row, and_stride, px, has_and);
        dst[px * 4] = r;
        dst[px * 4 + 1] = g;
        dst[px * 4 + 2] = b;
        dst[px * 4 + 3] = a;
    }
}

/// 32bpp row: 4 bytes per pixel in BGRA order (alpha embedded).
///
/// For 32bpp, the AND mask is an additional transparency source.
/// A pixel is transparent if the AND mask bit is set OR if the BGRA
/// alpha byte is 0.  The most common convention is AND mask = all zeros
/// and alpha in the BGRA byte.
fn decode_row_32bpp(
    xor: &[u8],
    src_row: usize,
    stride: usize,
    and: &[u8],
    and_stride: usize,
    has_and: bool,
    dst: &mut [u8],
) {
    let row_start = src_row * stride;
    let pixel_count = dst.len() / 4;
    // Safety invariant: for pixel_count pixels at 32bpp, the XOR slice must
    // hold at least `row_start + pixel_count * 4` bytes.
    debug_assert!(
        xor.len() >= row_start + pixel_count * 4,
        "decode_row_32bpp: xor slice too short (row_start={}, pixel_count={}, xor.len={})",
        row_start, pixel_count, xor.len()
    );
    for px in 0..pixel_count {
        let b = xor[row_start + px * 4];
        let g = xor[row_start + px * 4 + 1];
        let r = xor[row_start + px * 4 + 2];
        let a_bgra = xor[row_start + px * 4 + 3];
        // If AND mask bit is set, pixel is transparent regardless of alpha.
        let and_transparent = has_and && {
            let abyte = and[src_row * and_stride + px / 8];
            (abyte >> (7 - (px % 8))) & 1 != 0
        };
        let a = if and_transparent { 0 } else { a_bgra };
        dst[px * 4] = r;
        dst[px * 4 + 1] = g;
        dst[px * 4 + 2] = b;
        dst[px * 4 + 3] = a;
    }
}

// ── AND mask helper ──────────────────────────────────────────────────────────

/// Read the AND mask bit for pixel `px` in row `row` and return 0 (transparent)
/// or 255 (opaque).  Returns 255 when no AND mask is present.
#[inline]
fn and_alpha(and: &[u8], row: usize, stride: usize, px: usize, has_and: bool) -> u8 {
    if !has_and {
        return 255;
    }
    let byte = and[row * stride + px / 8];
    let bit = (byte >> (7 - (px % 8))) & 1;
    if bit != 0 { 0 } else { 255 }
}

// ── Geometry helpers ─────────────────────────────────────────────────────────

/// Row stride in bytes for an image of `width` pixels at `bpp` bits per pixel,
/// padded to 4-byte (DWORD) boundaries.
///
/// Formula: ((width × bpp + 31) / 32) × 4
///
/// Returns `None` on arithmetic overflow (defence-in-depth; in practice the
/// caller enforces `width ≤ 256` and `bpp ∈ {1,4,8,24,32}`, so overflow is
/// impossible, but checked arithmetic makes the invariant explicit).
pub fn row_stride_checked(width: usize, bpp: usize) -> Option<usize> {
    let bits = width.checked_mul(bpp)?.checked_add(31)?;
    (bits / 32).checked_mul(4)
}

/// Infallible row stride — panics on overflow.  Only call after validating
/// `width ≤ 256` and `bpp ∈ {1,4,8,24,32}`.
#[cfg(test)]
pub fn row_stride(width: usize, bpp: usize) -> usize {
    row_stride_checked(width, bpp).expect("row_stride overflow (width or bpp out of bounds)")
}

// ── Little-endian readers ────────────────────────────────────────────────────

fn read_u16le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_u32le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
}

fn read_i32le(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_stride_values() {
        assert_eq!(row_stride(1, 1), 4);   // 1 bit → pad to 32 bits = 4 bytes
        assert_eq!(row_stride(32, 1), 4);  // 32 bits → exactly 4 bytes
        assert_eq!(row_stride(33, 1), 8);  // 33 bits → next 32-bit boundary
        assert_eq!(row_stride(4, 8), 4);   // 4 pixels × 8 bpp = 32 bits
        assert_eq!(row_stride(5, 8), 8);   // 5 × 8 = 40 bits → 8 bytes
        assert_eq!(row_stride(4, 24), 12); // 4 × 24 = 96 bits = 12 bytes
        assert_eq!(row_stride(4, 32), 16); // 4 × 32 = 128 bits = 16 bytes
    }

    /// Build a minimal 2×2 32bpp BMP DIB manually and decode it.
    ///
    /// The DIB structure (no BITMAPFILEHEADER):
    ///   BITMAPINFOHEADER (40 bytes): width=2, height=4 (= 2*pixel_height), bpp=32
    ///   XOR data: 2 rows × 2 pixels × 4 bytes = 16 bytes (bottom-up)
    ///   AND data: 2 rows × 4 bytes (stride) = 8 bytes
    #[test]
    fn decode_32bpp_2x2() {
        let mut dib: Vec<u8> = Vec::new();
        // BITMAPINFOHEADER
        dib.extend_from_slice(&40u32.to_le_bytes()); // biSize
        dib.extend_from_slice(&2i32.to_le_bytes());  // biWidth
        dib.extend_from_slice(&4i32.to_le_bytes());  // biHeight = 2*pixel_height
        dib.extend_from_slice(&1u16.to_le_bytes());  // biPlanes
        dib.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
        dib.extend_from_slice(&0u32.to_le_bytes());  // biCompression
        dib.extend_from_slice(&0u32.to_le_bytes());  // biSizeImage
        dib.extend_from_slice(&0i32.to_le_bytes());  // biXPelsPerMeter
        dib.extend_from_slice(&0i32.to_le_bytes());  // biYPelsPerMeter
        dib.extend_from_slice(&0u32.to_le_bytes());  // biClrUsed
        dib.extend_from_slice(&0u32.to_le_bytes());  // biClrImportant

        // XOR data: bottom row first.
        // Row 1 (bottom = y=1 in output): pixel0=(B=0,G=255,R=0,A=255) green
        //                                 pixel1=(B=0,G=0,R=255,A=255)  red
        dib.extend_from_slice(&[0, 255, 0, 255]); // green BGRA
        dib.extend_from_slice(&[0, 0, 255, 255]); // red   BGRA
        // Row 0 (top = y=0 in output):   pixel0=(B=255,G=0,R=0,A=255)  blue
        //                                pixel1=(B=0,G=0,R=0,A=0)      transparent
        dib.extend_from_slice(&[255, 0, 0, 255]); // blue BGRA
        dib.extend_from_slice(&[0, 0, 0, 0]);     // transparent BGRA

        // AND mask: 2 rows × 4 bytes (stride=4 for width=2).
        // All zeros → no AND-mask transparency (alpha from BGRA).
        dib.extend_from_slice(&[0u8; 8]);

        let (w, h, rgba) = decode_bmp_dib(&dib).unwrap();
        assert_eq!(w, 2);
        assert_eq!(h, 2);

        // Top-left (y=0, x=0) = blue (255,0,0,255).
        assert_eq!(&rgba[0..4], &[0, 0, 255, 255]); // R=0,G=0,B=255,A=255
        // Top-right (y=0, x=1) = transparent (0,0,0,0).
        assert_eq!(&rgba[4..8], &[0, 0, 0, 0]);
        // Bottom-left (y=1, x=0) = green.
        assert_eq!(&rgba[8..12], &[0, 255, 0, 255]); // R=0,G=255,B=0,A=255
        // Bottom-right (y=1, x=1) = red.
        assert_eq!(&rgba[12..16], &[255, 0, 0, 255]);
    }

    /// AND mask transparency: pixel set in AND mask → alpha = 0.
    #[test]
    fn and_mask_overrides_alpha() {
        let mut dib: Vec<u8> = Vec::new();
        // 1×1 32bpp DIB.
        dib.extend_from_slice(&40u32.to_le_bytes()); // biSize
        dib.extend_from_slice(&1i32.to_le_bytes());  // biWidth
        dib.extend_from_slice(&2i32.to_le_bytes());  // biHeight = 2*1
        dib.extend_from_slice(&1u16.to_le_bytes());  // biPlanes
        dib.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
        dib.extend_from_slice(&[0u8; 24]);           // remaining 6 header fields (24 bytes)
        // XOR: opaque red (A=255 in BGRA).
        dib.extend_from_slice(&[0, 0, 255, 255]); // B=0,G=0,R=255,A=255
        // AND mask: 4-byte row (stride=4), bit 7 of byte 0 = set = transparent.
        dib.extend_from_slice(&[0x80, 0, 0, 0]);

        let (_, _, rgba) = decode_bmp_dib(&dib).unwrap();
        // Despite BGRA alpha=255, AND mask forces alpha=0.
        assert_eq!(rgba[3], 0, "AND mask should override alpha to 0");
    }

    #[test]
    fn decode_24bpp_1x1() {
        let mut dib: Vec<u8> = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes()); // biSize
        dib.extend_from_slice(&1i32.to_le_bytes());  // biWidth
        dib.extend_from_slice(&2i32.to_le_bytes());  // biHeight = 2*1
        dib.extend_from_slice(&1u16.to_le_bytes());  // biPlanes
        dib.extend_from_slice(&24u16.to_le_bytes()); // biBitCount
        // Remaining 6 BITMAPINFOHEADER fields:
        //   biCompression (4) + biSizeImage (4) + biXPelsPerMeter (4) +
        //   biYPelsPerMeter (4) + biClrUsed (4) + biClrImportant (4) = 24 bytes.
        dib.extend_from_slice(&[0u8; 24]);           // remaining 6 header fields
        // XOR: 1 row, 3 bytes BGR + 1 byte padding to reach 4-byte stride.
        dib.extend_from_slice(&[100, 150, 200, 0]); // B=100,G=150,R=200, pad
        // AND mask: 4 bytes, bit 7 clear (opaque).
        dib.extend_from_slice(&[0u8; 4]);

        let (_, _, rgba) = decode_bmp_dib(&dib).unwrap();
        assert_eq!(rgba[0], 200); // R
        assert_eq!(rgba[1], 150); // G
        assert_eq!(rgba[2], 100); // B
        assert_eq!(rgba[3], 255); // A = fully opaque
    }

    #[test]
    fn decode_8bpp_palette() {
        // 2×2 8bpp image using a 2-entry palette.
        let mut dib: Vec<u8> = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes()); // biSize
        dib.extend_from_slice(&2i32.to_le_bytes());  // biWidth
        dib.extend_from_slice(&4i32.to_le_bytes());  // biHeight = 2*2
        dib.extend_from_slice(&1u16.to_le_bytes());  // biPlanes
        dib.extend_from_slice(&8u16.to_le_bytes());  // biBitCount
        dib.extend_from_slice(&0u32.to_le_bytes());  // biCompression
        dib.extend_from_slice(&0u32.to_le_bytes());  // biSizeImage
        dib.extend_from_slice(&0i32.to_le_bytes());  // biXPelsPerMeter
        dib.extend_from_slice(&0i32.to_le_bytes());  // biYPelsPerMeter
        dib.extend_from_slice(&2u32.to_le_bytes());  // biClrUsed = 2
        dib.extend_from_slice(&0u32.to_le_bytes());  // biClrImportant

        // Palette: entry 0 = red (RGBQUAD = B,G,R,0), entry 1 = blue.
        dib.extend_from_slice(&[0, 0, 255, 0]); // entry 0: BGR = 0,0,255 → R
        dib.extend_from_slice(&[255, 0, 0, 0]); // entry 1: BGR = 255,0,0 → B

        // XOR rows (bottom-up, stride=4 for 2 pixels):
        // Bottom row: [1, 0, 0, 0] → blue, red
        // Top row:    [0, 1, 0, 0] → red, blue
        dib.extend_from_slice(&[1, 0, 0, 0]); // bottom
        dib.extend_from_slice(&[0, 1, 0, 0]); // top

        // AND mask: 2 rows × 4 bytes, all zeros (opaque).
        dib.extend_from_slice(&[0u8; 8]);

        let (w, h, rgba) = decode_bmp_dib(&dib).unwrap();
        assert_eq!(w, 2);
        assert_eq!(h, 2);
        // Top-left (y=0, x=0): palette[0] = red
        assert_eq!(&rgba[0..4], &[255, 0, 0, 255]);
        // Top-right (y=0, x=1): palette[1] = blue
        assert_eq!(&rgba[4..8], &[0, 0, 255, 255]);
        // Bottom-left (y=1, x=0): palette[1] = blue
        assert_eq!(&rgba[8..12], &[0, 0, 255, 255]);
        // Bottom-right (y=1, x=1): palette[0] = red
        assert_eq!(&rgba[12..16], &[255, 0, 0, 255]);
    }

    #[test]
    fn rejects_compression() {
        let mut dib = vec![0u8; 40];
        dib[0..4].copy_from_slice(&40u32.to_le_bytes()); // biSize
        dib[16..20].copy_from_slice(&1u32.to_le_bytes()); // biCompression = 1 (RLE)
        assert!(decode_bmp_dib(&dib).is_err());
    }

    #[test]
    fn rejects_bad_header_size() {
        let mut dib = vec![0u8; 40];
        dib[0..4].copy_from_slice(&12u32.to_le_bytes()); // biSize = 12 (BITMAPCOREHEADER)
        assert!(decode_bmp_dib(&dib).is_err());
    }

    // ── Security tests ─────────────────────────────────────────────────────

    /// biClrUsed > 2^biBitCount should be rejected, not cause OOM.
    ///
    /// A malicious file could set biClrUsed = 0xFFFF_FFFF to trigger a
    /// ~16 GiB Vec allocation on a 64-bit decoder.  Our fix caps it at the
    /// theoretical maximum for the bit depth.
    #[test]
    fn rejects_oversized_bi_clr_used() {
        let mut dib = vec![0u8; 40];
        dib[0..4].copy_from_slice(&40u32.to_le_bytes());    // biSize
        dib[4..8].copy_from_slice(&1i32.to_le_bytes());     // biWidth
        dib[8..12].copy_from_slice(&2i32.to_le_bytes());    // biHeight = 2*1
        dib[12..14].copy_from_slice(&1u16.to_le_bytes());   // biPlanes
        dib[14..16].copy_from_slice(&8u16.to_le_bytes());   // biBitCount = 8
        // biClrUsed = 0xFFFF_FFFF — 4B palette entries, impossibly large.
        dib[32..36].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        let err = decode_bmp_dib(&dib).unwrap_err();
        assert!(
            err.contains("biClrUsed") || err.contains("exceeds"),
            "expected biClrUsed rejection, got: {}",
            err
        );
    }

    /// biClrUsed = 2^biBitCount should be accepted (exact maximum).
    #[test]
    fn accepts_exact_max_bi_clr_used() {
        let mut dib: Vec<u8> = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes()); // biSize
        dib.extend_from_slice(&1i32.to_le_bytes());  // biWidth
        dib.extend_from_slice(&2i32.to_le_bytes());  // biHeight = 2*1
        dib.extend_from_slice(&1u16.to_le_bytes());  // biPlanes
        dib.extend_from_slice(&8u16.to_le_bytes());  // biBitCount = 8
        dib.extend_from_slice(&0u32.to_le_bytes());  // biCompression
        dib.extend_from_slice(&0u32.to_le_bytes());  // biSizeImage
        dib.extend_from_slice(&0i32.to_le_bytes());  // biXPelsPerMeter
        dib.extend_from_slice(&0i32.to_le_bytes());  // biYPelsPerMeter
        dib.extend_from_slice(&256u32.to_le_bytes()); // biClrUsed = 256 (max for 8bpp)
        dib.extend_from_slice(&0u32.to_le_bytes());  // biClrImportant
        // 256 palette entries × 4 bytes = 1024 bytes
        dib.extend(std::iter::repeat_n(0u8, 256 * 4));
        // 1 XOR pixel (8bpp, stride=4): palette index 0
        dib.extend_from_slice(&[0u8; 4]);
        // AND mask: 4 bytes
        dib.extend_from_slice(&[0u8; 4]);

        // Should decode successfully (palette[0] = black, opaque).
        let result = decode_bmp_dib(&dib);
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    }
}
