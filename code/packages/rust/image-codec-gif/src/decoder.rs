//! GIF file parser: reads GIF87a and GIF89a streams into a pixel buffer.
//!
//! The parser follows the block structure defined in the IC07 spec:
//!
//! ```text
//! Header (6 bytes) → LSD (7 bytes) → [GCT] → {Block}* → Trailer (0x3B)
//!
//! Block introducers:
//!   0x2C → Image Descriptor (+ optional LCT + image data)
//!   0x21 → Extension (followed by label byte)
//!   0x3B → Trailer (stop)
//! ```
//!
//! This module focuses on decoding the *first* image frame.  Subsequent
//! Image Descriptors (animation frames) are detected and trigger an error.

use pixel_container::PixelContainer;

use crate::lzw;

// ─── Public entry point ────────────────────────────────────────────────────────

/// Decode a GIF byte stream into an RGBA8 `PixelContainer`.
///
/// Returns the first (only, for static GIFs) frame.
/// Transparent pixels (via Graphic Control Extension) have alpha = 0;
/// all other pixels have alpha = 255.
///
/// # Errors
///
/// Returns `Err(String)` for:
/// - Non-GIF data
/// - Animated GIFs (multiple images)
/// - Malformed or truncated data
/// - Invalid LZW streams
pub fn decode_gif(bytes: &[u8]) -> Result<PixelContainer, String> {
    let mut p = Parser::new(bytes);
    p.parse()
}

// ─── Internal parser ──────────────────────────────────────────────────────────

struct Parser<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(data: &'a [u8]) -> Self {
        Parser { data, pos: 0 }
    }

    // ── Byte-level helpers ────────────────────────────────────────────────────

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn read_byte(&mut self) -> Result<u8, String> {
        if self.pos >= self.data.len() {
            return Err("GIF: unexpected end of data".into());
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn read_u16_le(&mut self) -> Result<u16, String> {
        let lo = self.read_byte()? as u16;
        let hi = self.read_byte()? as u16;
        Ok(lo | (hi << 8))
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.pos + n > self.data.len() {
            return Err("GIF: unexpected end of data".into());
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    /// Skip all sub-blocks at the current position (used to discard extensions).
    fn skip_sub_blocks(&mut self) -> Result<(), String> {
        loop {
            let len = self.read_byte()? as usize;
            if len == 0 {
                break;
            }
            if self.pos + len > self.data.len() {
                return Err("GIF: sub-block extends past end of data".into());
            }
            self.pos += len;
        }
        Ok(())
    }

    /// Read `3 * count` bytes as an RGB palette.
    /// Returns a Vec of (R, G, B) triples.
    fn read_color_table(&mut self, count: usize) -> Result<Vec<(u8, u8, u8)>, String> {
        let bytes = self.read_bytes(3 * count)?;
        Ok(bytes
            .chunks(3)
            .map(|c| (c[0], c[1], c[2]))
            .collect())
    }

    // ── Main parse loop ───────────────────────────────────────────────────────

    fn parse(&mut self) -> Result<PixelContainer, String> {
        // ── Header ──
        if self.remaining() < 6 {
            return Err("GIF: file too short to be a GIF".into());
        }
        let sig = self.read_bytes(3)?;
        if sig != b"GIF" {
            return Err("GIF: not a GIF file".into());
        }
        let ver = self.read_bytes(3)?;
        if ver != b"87a" && ver != b"89a" {
            return Err(format!(
                "GIF: unknown version '{}' (expected 87a or 89a)",
                String::from_utf8_lossy(ver)
            ));
        }

        // ── Logical Screen Descriptor ──
        if self.remaining() < 7 {
            return Err("GIF: truncated header (LSD missing)".into());
        }
        let _canvas_w = self.read_u16_le()?;
        let _canvas_h = self.read_u16_le()?;
        let packed = self.read_byte()?;
        let _bg_color_index = self.read_byte()?;
        let _pixel_aspect = self.read_byte()?;

        let global_ct_flag = (packed >> 7) & 1;
        let gct_size_field = (packed & 0x07) as u32;
        let gct_count = 1usize << (gct_size_field + 1);

        // ── Global Color Table ──
        let global_ct: Vec<(u8, u8, u8)> = if global_ct_flag == 1 {
            self.read_color_table(gct_count)?
        } else {
            Vec::new()
        };

        // ── State for pending Graphic Control Extension ──
        let mut transparent_color_flag = false;
        let mut transparent_color_index: u8 = 0;

        // ── Block scanning loop ──
        let mut frame_count = 0u32;
        let mut result: Option<PixelContainer> = None;

        loop {
            let introducer = self.read_byte()?;
            match introducer {
                0x3B => {
                    // Trailer — end of file.
                    break;
                }
                0x21 => {
                    // Extension block.
                    let label = self.read_byte()?;
                    match label {
                        0xF9 => {
                            // Graphic Control Extension — transparency / animation.
                            let block_size = self.read_byte()?;
                            if block_size < 4 {
                                return Err(
                                    "GIF: Graphic Control Extension block size < 4".into()
                                );
                            }
                            let gce_packed = self.read_byte()?;
                            let _delay = self.read_u16_le()?;
                            let tci = self.read_byte()?;
                            // Skip any extra bytes in this sub-block.
                            for _ in 4..block_size {
                                self.read_byte()?;
                            }
                            // Read the terminator (sub-block length = 0).
                            let term = self.read_byte()?;
                            if term != 0 {
                                // Extra sub-blocks in GCE — skip them.
                                self.pos -= 1;
                                self.skip_sub_blocks()?;
                            }
                            transparent_color_flag = (gce_packed & 0x01) != 0;
                            transparent_color_index = tci;
                        }
                        _ => {
                            // All other extensions (Application, Comment, Plain Text,
                            // and any future extensions): skip all sub-blocks.
                            self.skip_sub_blocks()?;
                        }
                    }
                }
                0x2C => {
                    // Image Descriptor.
                    frame_count += 1;
                    if frame_count > 1 {
                        return Err("GIF: animated GIF not supported (multiple frames detected)".into());
                    }

                    let _left = self.read_u16_le()? as usize;
                    let _top = self.read_u16_le()? as usize;
                    let width = self.read_u16_le()? as usize;
                    let height = self.read_u16_le()? as usize;

                    // Sanity-check declared dimensions before any allocation.
                    // 16 MP (4096×4096) is far above any realistic static GIF
                    // and still allows comfortable headroom for the GIF maximum
                    // of 65535×65535 to be rejected early.
                    const MAX_PIXELS: usize = 4096 * 4096;
                    let pixel_count = width.saturating_mul(height);
                    if pixel_count > MAX_PIXELS {
                        return Err(format!(
                            "GIF: image dimensions {}×{} = {} pixels exceed \
                             the {} pixel safety limit",
                            width, height, pixel_count, MAX_PIXELS
                        ));
                    }

                    let img_packed = self.read_byte()?;

                    let local_ct_flag = (img_packed >> 7) & 1;
                    let interlace_flag = (img_packed >> 6) & 1;
                    let lct_size_field = (img_packed & 0x07) as u32;
                    let lct_count = 1usize << (lct_size_field + 1);

                    // Choose color table (LCT overrides GCT).
                    let lct: Vec<(u8, u8, u8)> = if local_ct_flag == 1 {
                        self.read_color_table(lct_count)?
                    } else {
                        Vec::new()
                    };
                    let color_table: &Vec<(u8, u8, u8)> = if local_ct_flag == 1 {
                        &lct
                    } else {
                        &global_ct
                    };

                    if color_table.is_empty() {
                        return Err("GIF: no color table available for image".into());
                    }

                    // Read image data (starts with lzw_minimum_code_size byte,
                    // followed by sub-blocks).
                    let img_data_start = self.pos;

                    // Find the end of sub-blocks to collect image data.
                    // We collect a contiguous slice including the min_code_size byte.
                    // Skip min_code_size byte to find sub-blocks.
                    if self.pos >= self.data.len() {
                        return Err("GIF: truncated image data".into());
                    }
                    let _min_code_size_byte = self.data[self.pos];
                    self.pos += 1;
                    // Skip sub-blocks.
                    self.skip_sub_blocks()?;
                    let img_data = &self.data[img_data_start..self.pos];

                    // Decode LZW, capping output at pixel_count to prevent
                    // decompression-bomb attacks from crafted LZW streams.
                    let indices = lzw::decode(img_data, pixel_count)
                        .map_err(|e| format!("GIF: LZW error: {}", e))?;
                    if indices.len() < pixel_count {
                        return Err(format!(
                            "GIF: decoded {} indices but image needs {} ({}×{})",
                            indices.len(),
                            pixel_count,
                            width,
                            height
                        ));
                    }

                    // De-interlace if needed.
                    let ordered_indices: Vec<u8> = if interlace_flag == 1 {
                        de_interlace(&indices, width, height)
                    } else {
                        indices[..pixel_count].to_vec()
                    };

                    // Convert palette indices to RGBA.
                    let mut pixel_data = PixelContainer::new(width as u32, height as u32);

                    for (i, &idx) in ordered_indices[..pixel_count].iter().enumerate() {
                        let alpha = if transparent_color_flag && idx == transparent_color_index {
                            0u8
                        } else {
                            255u8
                        };
                        let (r, g, b) = if (idx as usize) < color_table.len() {
                            color_table[idx as usize]
                        } else {
                            (0, 0, 0)
                        };
                        let px = (i % width) as u32;
                        let py = (i / width) as u32;
                        pixel_data.set_pixel(px, py, r, g, b, alpha);
                    }

                    result = Some(pixel_data);

                    // Reset GCE state after consuming it.
                    transparent_color_flag = false;
                    transparent_color_index = 0;
                }
                other => {
                    return Err(format!(
                        "GIF: unknown block introducer 0x{:02X} at offset {}",
                        other,
                        self.pos - 1
                    ));
                }
            }
        }

        result.ok_or_else(|| "GIF: no image found".into())
    }
}

// ─── De-interlacing ───────────────────────────────────────────────────────────

/// Convert interlaced pixel order to progressive (top-to-bottom) order.
///
/// GIF interlacing uses 4 passes:
///   Pass 1: rows 0, 8, 16, 24, …
///   Pass 2: rows 4, 12, 20, 28, …
///   Pass 3: rows 2, 6, 10, 14, …
///   Pass 4: rows 1, 3, 5, 7, …
///
/// The LZW-decoded data streams rows in pass order; we rearrange to
/// top-to-bottom order.
fn de_interlace(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    // Each pass: (starting_row, row_step)
    let passes: [(usize, usize); 4] = [(0, 8), (4, 8), (2, 4), (1, 2)];

    let mut result = vec![0u8; width * height];
    let mut src = 0usize;

    for (start, step) in &passes {
        let mut row = *start;
        while row < height {
            let src_end = (src + width).min(data.len());
            let count = src_end - src;
            let dst_start = row * width;
            result[dst_start..dst_start + count]
                .copy_from_slice(&data[src..src + count]);
            src += width;
            row += step;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_bad_magic() {
        let err = decode_gif(b"PNG\x89\x0D\x0A").unwrap_err();
        assert!(err.contains("not a GIF"), "got: {}", err);
    }

    #[test]
    fn decode_bad_version() {
        let mut data = b"GIF99a".to_vec();
        data.extend_from_slice(&[0; 100]); // pad
        let err = decode_gif(&data).unwrap_err();
        assert!(err.contains("unknown version"), "got: {}", err);
    }

    #[test]
    fn decode_too_short() {
        let err = decode_gif(b"GIF").unwrap_err();
        assert!(err.contains("too short") || err.contains("truncated") || err.contains("end of data"), "got: {}", err);
    }

    #[test]
    fn de_interlace_2x4() {
        // 2 wide, 4 tall.
        // Interlaced order: pass1=row0, pass2=row2(skipped for step=8, height=4 has no row 4),
        // actually: pass1 (start=0,step=8): rows 0; pass2 (start=4,step=8): none (4>=4);
        // pass3 (start=2,step=4): row 2; pass4 (start=1,step=2): rows 1,3.
        // So interlaced data = [row0, row2, row1, row3].
        let data: Vec<u8> = vec![
            10, 11, // row 0 (pass 1)
            20, 21, // row 2 (pass 3)
            30, 31, // row 1 (pass 4)
            40, 41, // row 3 (pass 4)
        ];
        let out = de_interlace(&data, 2, 4);
        // Expected: [row0, row1, row2, row3]
        assert_eq!(out, vec![10, 11, 30, 31, 20, 21, 40, 41]);
    }
}
