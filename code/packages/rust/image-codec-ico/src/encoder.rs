//! ICO encoder — writes a single-image 32bpp BGRA ICO file.
//!
//! ## Output structure
//!
//! ```text
//! ICO header  (6 bytes):
//!   reserved: u16 = 0
//!   type:     u16 = 1  (ICO)
//!   count:    u16 = 1  (one image)
//!
//! Directory entry  (16 bytes):
//!   width:         u8   (0 means 256)
//!   height:        u8   (0 means 256)
//!   color_count:   u8   = 0  (truecolor — no palette)
//!   reserved:      u8   = 0
//!   planes:        u16  = 1
//!   bit_count:     u16  = 32
//!   bytes_in_res:  u32  (total BMP DIB byte count)
//!   image_offset:  u32  = 22 (= 6 header + 16 dir entry)
//!
//! BMP DIB  (BITMAPINFOHEADER + XOR pixels + AND mask):
//!   BITMAPINFOHEADER  (40 bytes)
//!   XOR pixel data    (width × height × 4 bytes BGRA, bottom-up)
//!   AND mask          (row-padded to 4 bytes, all zeros)
//! ```
//!
//! ## Why 32bpp BGRA?
//!
//! 32bpp is the highest fidelity option — it embeds the full alpha channel
//! in the BGRA byte, so transparent pixels survive the encode/decode cycle
//! exactly.  All modern Windows, macOS, and Linux ICO renderers support it.

use pixel_container::PixelContainer;

// ── Constants ──────────────────────────────────────────────────────────────

/// Byte offset of the first (and only) image in the output file.
/// = 6 (header) + 16 (one directory entry).
const IMAGE_OFFSET: u32 = 22;

/// Bits per pixel for the encoded output.
const BITS_PER_PIXEL: u16 = 32;

// ── Public entry point ─────────────────────────────────────────────────────

/// Encode a `PixelContainer` as a single-image 32bpp ICO file.
///
/// Dimensions are clamped to 255×255 because the ICO directory byte wraps
/// 256→0 (a stored value of 0 means 256, but we stay below to avoid
/// ambiguity with the 256 convention).
///
/// The output is a complete, self-contained `.ico` file.
pub fn encode_ico(pixels: &PixelContainer) -> Vec<u8> {
    // Clamp to 255 — the directory byte can hold 0-255 where 0 means 256.
    // We emit explicit pixel dimensions (1-255) to keep things unambiguous.
    let width = (pixels.width as usize).min(255);
    let height = (pixels.height as usize).min(255);

    // Build the BMP DIB.
    let dib = build_bmp_dib(pixels, width, height);
    let dib_len = dib.len() as u32;

    let mut out = Vec::with_capacity(22 + dib.len());

    // ── ICO header (6 bytes) ──────────────────────────────────────────────
    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // type = 1 (ICO)
    out.extend_from_slice(&1u16.to_le_bytes()); // count = 1

    // ── Directory entry (16 bytes) ────────────────────────────────────────
    out.push(width as u8);                      // width (1-255)
    out.push(height as u8);                     // height (1-255)
    out.push(0);                                // color_count = 0 (truecolor)
    out.push(0);                                // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // planes = 1
    out.extend_from_slice(&BITS_PER_PIXEL.to_le_bytes()); // bit_count = 32
    out.extend_from_slice(&dib_len.to_le_bytes()); // bytes_in_res
    out.extend_from_slice(&IMAGE_OFFSET.to_le_bytes()); // image_offset = 22

    // ── BMP DIB ───────────────────────────────────────────────────────────
    out.extend_from_slice(&dib);

    out
}

// ── BMP DIB builder ────────────────────────────────────────────────────────

/// Build a 32bpp BMP DIB (no BITMAPFILEHEADER) from `pixels`.
///
/// The DIB contains:
/// 1. `BITMAPINFOHEADER` (40 bytes)
/// 2. XOR pixel data: rows bottom-up, 4 bytes (BGRA) per pixel
/// 3. AND mask: rows bottom-up, 1 bpp, padded to 4-byte boundary, all zeros
///
/// `bi_height = 2 * pixel_height` because the DIB stores both XOR + AND masks.
fn build_bmp_dib(pixels: &PixelContainer, width: usize, height: usize) -> Vec<u8> {
    // Row stride for 32bpp: width × 4 bytes (always 4-byte aligned).
    let xor_stride = width * 4;
    let xor_size = xor_stride * height;

    // AND mask stride: ((width + 31) / 32) × 4 bytes.
    let and_stride = width.div_ceil(32) * 4;
    let and_size = and_stride * height;

    let mut dib = Vec::with_capacity(40 + xor_size + and_size);

    // ── BITMAPINFOHEADER (40 bytes) ─────────────────────────────────────────
    dib.extend_from_slice(&40u32.to_le_bytes());                   // biSize
    dib.extend_from_slice(&(width as i32).to_le_bytes());          // biWidth
    dib.extend_from_slice(&((height * 2) as i32).to_le_bytes());   // biHeight = 2*h
    dib.extend_from_slice(&1u16.to_le_bytes());                    // biPlanes
    dib.extend_from_slice(&BITS_PER_PIXEL.to_le_bytes());          // biBitCount
    dib.extend_from_slice(&0u32.to_le_bytes());                    // biCompression = BI_RGB
    dib.extend_from_slice(&0u32.to_le_bytes());                    // biSizeImage (0 for BI_RGB)
    dib.extend_from_slice(&0i32.to_le_bytes());                    // biXPelsPerMeter
    dib.extend_from_slice(&0i32.to_le_bytes());                    // biYPelsPerMeter
    dib.extend_from_slice(&0u32.to_le_bytes());                    // biClrUsed
    dib.extend_from_slice(&0u32.to_le_bytes());                    // biClrImportant

    // ── XOR pixel data: rows bottom-up (last pixel row first) ──────────────
    for row in (0..height).rev() {
        for col in 0..width {
            let (r, g, b, a) = pixels.pixel_at(col as u32, row as u32);
            // BGRA byte order.
            dib.push(b);
            dib.push(g);
            dib.push(r);
            dib.push(a);
        }
    }

    // ── AND mask: all zeros (alpha from BGRA channel) ─────────────────────
    // A zero AND mask bit = opaque; alpha=0 in BGRA handles transparency.
    dib.extend(std::iter::repeat_n(0u8, and_size));

    dib
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_header_is_correct() {
        let px = PixelContainer::new(2, 2);
        let bytes = encode_ico(&px);
        // ICO header
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0, "reserved");
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 1, "type=ICO");
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 1, "count=1");
    }

    #[test]
    fn encode_directory_entry_is_correct() {
        let px = PixelContainer::new(4, 8);
        let bytes = encode_ico(&px);
        assert_eq!(bytes[6], 4, "width in dir");
        assert_eq!(bytes[7], 8, "height in dir");
        assert_eq!(bytes[8], 0, "colorCount");
        assert_eq!(bytes[9], 0, "reserved");
        assert_eq!(u16::from_le_bytes([bytes[10], bytes[11]]), 1, "planes");
        assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 32, "bitCount");
        assert_eq!(u32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]), 22, "imageOffset");
    }

    #[test]
    fn encode_image_offset_is_22() {
        let px = PixelContainer::new(1, 1);
        let bytes = encode_ico(&px);
        let offset = u32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]);
        assert_eq!(offset, 22);
    }

    #[test]
    fn encode_bit_count_is_32() {
        let px = PixelContainer::new(1, 1);
        let bytes = encode_ico(&px);
        let bit_count = u16::from_le_bytes([bytes[12], bytes[13]]);
        assert_eq!(bit_count, 32);
    }

    #[test]
    fn encode_pixel_stored_as_bgra() {
        let mut px = PixelContainer::new(1, 1);
        px.set_pixel(0, 0, 10, 20, 30, 40); // R=10,G=20,B=30,A=40
        let bytes = encode_ico(&px);
        // XOR data starts at byte 22 (image_offset) + 40 (DIB header) = 62.
        let xor_start = 22 + 40;
        assert_eq!(bytes[xor_start], 30, "B");
        assert_eq!(bytes[xor_start + 1], 20, "G");
        assert_eq!(bytes[xor_start + 2], 10, "R");
        assert_eq!(bytes[xor_start + 3], 40, "A");
    }

    #[test]
    fn encode_and_mask_is_all_zeros() {
        let mut px = PixelContainer::new(2, 2);
        px.fill(0, 0, 0, 0); // fully transparent
        let bytes = encode_ico(&px);
        // AND mask follows XOR data.
        let xor_size = 2 * 2 * 4; // 2×2 pixels × 4 bytes BGRA
        let and_start = 22 + 40 + xor_size;
        // AND mask for width=2: stride=4, 2 rows = 8 bytes, all zero.
        for (offset, &byte) in bytes[and_start..and_start + 8].iter().enumerate() {
            assert_eq!(byte, 0, "AND mask byte {} should be 0", and_start + offset);
        }
    }
}
