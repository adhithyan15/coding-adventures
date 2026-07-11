//! ICO/CUR file parser.
//!
//! Reads the file header, scans directory entries, selects the best-resolution
//! image, then dispatches to either the BMP DIB decoder or the PNG decoder.
//!
//! ## Image selection strategy
//!
//! When an ICO contains multiple images (e.g., 16×16, 32×32, 48×48, 256×256)
//! we return the one with the largest pixel area.  Among ties we prefer higher
//! bit depth — 32bpp or PNG over 24/8/4/1bpp.

use crate::bmp_dib;
use pixel_container::PixelContainer;

// ── PNG magic bytes ────────────────────────────────────────────────────────

/// First 8 bytes of any PNG file.
const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];

// ── Directory entry ────────────────────────────────────────────────────────

/// One 16-byte entry in the ICO directory.
#[derive(Debug, Clone)]
struct DirEntry {
    /// Pixel width (0 → 256).
    width: usize,
    /// Pixel height (0 → 256).
    height: usize,
    /// Bits per pixel (0 for PNG frames).
    bit_count: u16,
    /// Byte length of the image data.
    bytes_in_res: usize,
    /// Byte offset from the start of the file.
    image_offset: usize,
}

impl DirEntry {
    /// Pixel area (for sorting).
    fn area(&self) -> usize {
        self.width * self.height
    }

    /// Is this frame a PNG (first 8 bytes are PNG magic)?
    fn is_png(&self, data: &[u8]) -> bool {
        let end = self.image_offset + 8;
        if end > data.len() {
            return false;
        }
        data[self.image_offset..self.image_offset + 8] == PNG_MAGIC
    }
}

// ── Public entry point ─────────────────────────────────────────────────────

/// Decode the best-resolution image from an ICO or CUR file.
///
/// Returns the largest image.  For equal-size candidates the 32bpp BMP or
/// PNG variant is preferred.
pub fn decode_ico(data: &[u8]) -> Result<PixelContainer, String> {
    // ── Header (6 bytes) ─────────────────────────────────────────────────
    if data.len() < 6 {
        return Err("ICO: file too short".into());
    }
    let reserved = u16_le(data, 0);
    let typ = u16_le(data, 2);
    let count = u16_le(data, 4) as usize;

    if reserved != 0 {
        return Err(format!(
            "ICO: invalid reserved field {} (expected 0)",
            reserved
        ));
    }
    if typ != 1 && typ != 2 {
        return Err(format!(
            "ICO: unknown type {} (expected 1=ICO or 2=CUR)",
            typ
        ));
    }
    if count == 0 {
        return Err("ICO: no images in file".into());
    }

    // Security cap: real ICO files contain a handful of images (typically
    // ≤12).  65535 entries would allocate ~2.6 MiB for the directory and
    // ~2.5 MiB for the Vec<DirEntry>, creating an allocation amplification
    // DoS for any caller that processes untrusted .ico files.
    const MAX_ICO_ENTRIES: usize = 256;
    if count > MAX_ICO_ENTRIES {
        return Err(format!(
            "ICO: too many directory entries {} (max {})",
            count, MAX_ICO_ENTRIES
        ));
    }

    // ── Directory entries (16 bytes each) ─────────────────────────────────
    // count ≤ 256, so count * 16 ≤ 4096 — no overflow.
    let dir_end = 6 + count * 16;
    if data.len() < dir_end {
        return Err("ICO: file truncated in directory".into());
    }

    let mut entries: Vec<DirEntry> = Vec::with_capacity(count);
    for i in 0..count {
        let base = 6 + i * 16;
        let w_byte = data[base]; // 0 means 256
        let h_byte = data[base + 1];
        let bit_count = u16_le(data, base + 6);
        let bytes_in_res = u32_le(data, base + 8) as usize;
        let image_offset = u32_le(data, base + 12) as usize;

        // The directory byte 0 means 256 (the maximum dimension).
        let width = if w_byte == 0 { 256 } else { w_byte as usize };
        let height = if h_byte == 0 { 256 } else { h_byte as usize };

        // Basic bounds check.
        if image_offset.saturating_add(bytes_in_res) > data.len() {
            return Err(format!(
                "ICO: entry {} image data (offset={}, size={}) extends past end of file ({}B)",
                i, image_offset, bytes_in_res, data.len()
            ));
        }

        entries.push(DirEntry {
            width,
            height,
            bit_count,
            bytes_in_res,
            image_offset,
        });
    }

    // ── Select best entry ────────────────────────────────────────────────
    let best = select_best(&entries, data);
    let entry = &entries[best];
    let image_data = &data[entry.image_offset..entry.image_offset + entry.bytes_in_res];

    // ── Dispatch: PNG or BMP DIB ─────────────────────────────────────────
    if entry.is_png(data) {
        decode_png_frame(image_data)
    } else {
        decode_bmp_frame(image_data)
    }
}

// ── Entry selection ────────────────────────────────────────────────────────

/// Choose the index of the best directory entry to decode.
///
/// Priority:
/// 1. Largest pixel area.
/// 2. Among ties: PNG > 32bpp > 24bpp > 8bpp > 4bpp > 1bpp.
fn select_best(entries: &[DirEntry], data: &[u8]) -> usize {
    let mut best = 0;
    for (i, e) in entries.iter().enumerate() {
        let b = &entries[best];
        if e.area() > b.area() {
            best = i;
        } else if e.area() == b.area() && quality_rank(e, data) > quality_rank(b, data) {
            best = i;
        }
    }
    best
}

/// Higher = better quality.  PNG = 100, 32bpp = 32, 24bpp = 24, etc.
fn quality_rank(e: &DirEntry, data: &[u8]) -> u32 {
    if e.is_png(data) {
        100
    } else {
        e.bit_count as u32
    }
}

// ── Frame decoders ─────────────────────────────────────────────────────────

/// Decode a full PNG file embedded in an ICO frame.
fn decode_png_frame(png_data: &[u8]) -> Result<PixelContainer, String> {
    let (width, height, rgba) =
        png::decode_png_rgba(png_data).map_err(|e| format!("ICO: PNG decode error: {}", e))?;
    let mut pc = PixelContainer::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let base = ((y * width + x) * 4) as usize;
            pc.set_pixel(x, y, rgba[base], rgba[base + 1], rgba[base + 2], rgba[base + 3]);
        }
    }
    Ok(pc)
}

/// Decode a BMP DIB frame (BITMAPINFOHEADER + pixel data + AND mask).
fn decode_bmp_frame(dib_data: &[u8]) -> Result<PixelContainer, String> {
    let (width, height, rgba) = bmp_dib::decode_bmp_dib(dib_data)?;
    let mut pc = PixelContainer::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let base = ((y * width + x) * 4) as usize;
            pc.set_pixel(x, y, rgba[base], rgba[base + 1], rgba[base + 2], rgba[base + 3]);
        }
    }
    Ok(pc)
}

// ── Little-endian helpers ──────────────────────────────────────────────────

fn u16_le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal 1×1 32bpp ICO by hand and verify decoding.
    #[test]
    fn decode_minimal_1x1_32bpp() {
        let mut ico = Vec::<u8>::new();
        // Header
        ico.extend_from_slice(&0u16.to_le_bytes()); // reserved
        ico.extend_from_slice(&1u16.to_le_bytes()); // type = ICO
        ico.extend_from_slice(&1u16.to_le_bytes()); // count = 1

        // BMP DIB for 1×1 32bpp.
        // BITMAPINFOHEADER: biWidth=1, biHeight=2 (=2*pixel_height), biBitCount=32
        let mut dib = Vec::<u8>::new();
        dib.extend_from_slice(&40u32.to_le_bytes()); // biSize
        dib.extend_from_slice(&1i32.to_le_bytes());  // biWidth
        dib.extend_from_slice(&2i32.to_le_bytes());  // biHeight
        dib.extend_from_slice(&1u16.to_le_bytes());  // biPlanes
        dib.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
        dib.extend_from_slice(&[0u8; 24]); // remaining 6 header fields (24 bytes)           // remaining header fields
        // XOR: BGRA = (50, 100, 200, 255)
        dib.extend_from_slice(&[50, 100, 200, 255]);
        // AND mask: 4 bytes, all zero
        dib.extend_from_slice(&[0u8; 4]);

        let dib_len = dib.len() as u32;

        // Directory entry
        ico.push(1); // width
        ico.push(1); // height
        ico.push(0); // colorCount
        ico.push(0); // reserved
        ico.extend_from_slice(&1u16.to_le_bytes()); // planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bitCount
        ico.extend_from_slice(&dib_len.to_le_bytes());
        ico.extend_from_slice(&22u32.to_le_bytes()); // imageOffset

        ico.extend_from_slice(&dib);

        let pc = decode_ico(&ico).unwrap();
        assert_eq!(pc.width, 1);
        assert_eq!(pc.height, 1);
        let (r, g, b, a) = pc.pixel_at(0, 0);
        assert_eq!(r, 200); // R from BGRA B=50,G=100,R=200
        assert_eq!(g, 100);
        assert_eq!(b, 50);
        assert_eq!(a, 255);
    }

    #[test]
    fn decode_cur_type_2_accepted() {
        // Same as decode_minimal_1x1_32bpp but with type=2 (CUR).
        let mut ico = Vec::<u8>::new();
        ico.extend_from_slice(&0u16.to_le_bytes()); // reserved
        ico.extend_from_slice(&2u16.to_le_bytes()); // type = CUR
        ico.extend_from_slice(&1u16.to_le_bytes()); // count = 1

        let mut dib = Vec::<u8>::new();
        dib.extend_from_slice(&40u32.to_le_bytes()); // biSize
        dib.extend_from_slice(&1i32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&32u16.to_le_bytes());
        dib.extend_from_slice(&[0u8; 24]); // remaining 6 header fields (24 bytes)
        dib.extend_from_slice(&[0, 0, 255, 255]); // BGRA = blue opaque
        dib.extend_from_slice(&[0u8; 4]);          // AND mask

        let dib_len = dib.len() as u32;
        ico.push(1); ico.push(1); ico.push(0); ico.push(0);
        ico.extend_from_slice(&0u16.to_le_bytes()); // hotspot_x
        ico.extend_from_slice(&0u16.to_le_bytes()); // hotspot_y
        ico.extend_from_slice(&dib_len.to_le_bytes());
        ico.extend_from_slice(&22u32.to_le_bytes());
        ico.extend_from_slice(&dib);

        let pc = decode_ico(&ico).unwrap();
        assert_eq!(pc.width, 1);
        assert_eq!(pc.height, 1);
    }

    #[test]
    fn decode_selects_largest() {
        // Build a 2-image ICO: 1×1 and 2×2.  The 2×2 should be returned.
        let mut ico: Vec<u8> = Vec::new();
        ico.extend_from_slice(&0u16.to_le_bytes());
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&2u16.to_le_bytes()); // 2 images

        // Image data placeholders — we'll fill in offsets after computing sizes.
        // DIB for 1×1.
        let dib1 = make_solid_32bpp_dib(1, 1, (255, 0, 0, 255));
        // DIB for 2×2.
        let dib2 = make_solid_32bpp_dib(2, 2, (0, 255, 0, 255));

        // Total header = 6 + 2*16 = 38 bytes.
        let offset1 = 38u32;
        let offset2 = offset1 + dib1.len() as u32;

        // Dir entry for 1×1.
        ico.push(1); ico.push(1); ico.push(0); ico.push(0);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&(dib1.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset1.to_le_bytes());

        // Dir entry for 2×2.
        ico.push(2); ico.push(2); ico.push(0); ico.push(0);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&(dib2.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset2.to_le_bytes());

        ico.extend_from_slice(&dib1);
        ico.extend_from_slice(&dib2);

        let pc = decode_ico(&ico).unwrap();
        assert_eq!(pc.width, 2);
        assert_eq!(pc.height, 2);
        // Top-left should be green (from the 2×2 DIB).
        let (r, g, _b, _a) = pc.pixel_at(0, 0);
        assert_eq!(r, 0);
        assert_eq!(g, 255);
    }

    #[test]
    fn decode_error_bad_reserved() {
        let mut ico = vec![0u8; 6];
        ico[0] = 1; // reserved != 0
        ico[2] = 1; // type = ICO
        ico[4] = 1; // count = 1
        assert!(decode_ico(&ico).is_err());
    }

    #[test]
    fn decode_error_bad_type() {
        let mut ico = vec![0u8; 6];
        ico[2] = 3; // type = 3 (invalid)
        ico[4] = 1;
        assert!(decode_ico(&ico).is_err());
    }

    #[test]
    fn decode_error_zero_count() {
        let ico = vec![0u8, 0, 1, 0, 0, 0]; // reserved=0, type=1, count=0
        let err = decode_ico(&ico).unwrap_err();
        assert!(err.contains("no images"), "got: {}", err);
    }

    #[test]
    fn decode_error_too_short() {
        let err = decode_ico(b"ICO").unwrap_err();
        assert!(err.contains("too short"), "got: {}", err);
    }

    // ── Helper ───────────────────────────────────────────────────────────────

    /// Build a minimal 32bpp BMP DIB for a solid-color `width × height` image.
    fn make_solid_32bpp_dib(width: usize, height: usize, rgba: (u8, u8, u8, u8)) -> Vec<u8> {
        let (r, g, b, a) = rgba;
        let and_stride = width.div_ceil(32) * 4;
        let and_size = and_stride * height;

        let mut dib = Vec::<u8>::new();
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&(width as i32).to_le_bytes());
        dib.extend_from_slice(&((height * 2) as i32).to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&32u16.to_le_bytes());
        dib.extend_from_slice(&[0u8; 24]); // remaining 6 header fields (24 bytes) // remaining header

        for _ in 0..height {
            for _ in 0..width {
                dib.push(b);
                dib.push(g);
                dib.push(r);
                dib.push(a);
            }
        }
        dib.extend(std::iter::repeat_n(0u8, and_size));
        dib
    }
}
