// # strips.rs — Strip and Tile Assembly
//
// TIFF pixel data is divided into either strips (horizontal bands) or tiles
// (rectangular blocks). This module handles decompress + reassemble for both.
//
// ## Strip Layout
//
// ```text
// Strip 0: rows 0 .. RowsPerStrip-1
// Strip 1: rows RowsPerStrip .. 2*RowsPerStrip-1
// ...
// Last strip: may be partial if height % RowsPerStrip != 0
//
// strip_index = row / RowsPerStrip
// byte_offset = StripOffsets[strip_index]
// byte_count  = StripByteCounts[strip_index]
// row_within_strip = row % RowsPerStrip
// ```
//
// ## Tile Layout
//
// ```text
// tiles_across = ceil(width  / tile_width)
// tiles_down   = ceil(height / tile_height)
// tile_index   = tile_row * tiles_across + tile_col
// ```
//
// Each tile occupies `tile_width × tile_height` pixels. Tiles at the right and
// bottom edges may be padded — the decoder clips to actual image dimensions.
//
// ## Security
//
// - At most 65536 strips/tiles (see MAX_STRIP_COUNT).
// - Checked arithmetic for strip sizes.
// - All byte offsets validated against file length.

use crate::compression;
use crate::ifd::Ifd;

/// Maximum number of strips or tiles. Prevents excessive memory allocation.
const MAX_STRIP_COUNT: usize = 65536;

// ─── Main assembly entry point ────────────────────────────────────────────────

/// Assemble all decompressed pixel bytes for the given IFD.
///
/// Returns a flat byte array in row-major order. For chunky RGB, this is
/// `[R0,G0,B0, R1,G1,B1, ...]`. For grayscale, it's `[L0, L1, ...]`.
///
/// # Arguments
///
/// - `bytes`: the entire raw TIFF file buffer
/// - `ifd`: the parsed IFD describing where pixel data lives
///
/// # Returns
///
/// All pixel bytes, fully decompressed, in row-major order.
pub fn assemble(bytes: &[u8], ifd: &Ifd) -> Result<Vec<u8>, String> {
    // Decide: is this a tile layout or a strip layout?
    if ifd.tile_width.is_some() && ifd.tile_length.is_some() {
        assemble_tiles(bytes, ifd)
    } else {
        assemble_strips(bytes, ifd)
    }
}

// ─── Strip assembly ──────────────────────────────────────────────────────────

/// Assemble pixel data from TIFF strips.
///
/// Each strip holds `rows_per_strip` rows of compressed pixel data.
/// We decompress each strip and stitch the rows together in order.
fn assemble_strips(bytes: &[u8], ifd: &Ifd) -> Result<Vec<u8>, String> {
    let width = ifd.width as usize;
    let height = ifd.height as usize;
    let samples = ifd.samples_per_pixel as usize;
    let bits = ifd.bits_per_sample.first().copied().unwrap_or(8) as usize;

    // Compute bytes per sample (round up to nearest byte).
    let bytes_per_sample = bits.div_ceil(8);

    // Row stride in bytes (uncompressed, chunky layout).
    let row_stride = width
        .checked_mul(samples)
        .and_then(|n| n.checked_mul(bytes_per_sample))
        .ok_or("TIFF: strip row stride overflow")?;

    // Expected total decompressed size.
    let total_expected = height
        .checked_mul(row_stride)
        .ok_or("TIFF: total image size overflow")?;

    let num_strips = ifd.strip_offsets.len();
    if num_strips == 0 {
        return Err("TIFF: no strip offsets (missing StripOffsets tag)".into());
    }
    if num_strips > MAX_STRIP_COUNT {
        return Err(format!("TIFF: {} strips exceeds max {}", num_strips, MAX_STRIP_COUNT));
    }
    if num_strips != ifd.strip_byte_counts.len() {
        return Err(format!(
            "TIFF: StripOffsets has {} entries but StripByteCounts has {}",
            num_strips,
            ifd.strip_byte_counts.len()
        ));
    }

    let rows_per_strip = if ifd.rows_per_strip == 0 || ifd.rows_per_strip == u32::MAX {
        // "Entire image is one strip"
        height as u32
    } else {
        ifd.rows_per_strip
    };

    let mut output = Vec::with_capacity(total_expected);

    for strip_idx in 0..num_strips {
        let strip_offset = ifd.strip_offsets[strip_idx] as usize;
        let strip_byte_count = ifd.strip_byte_counts[strip_idx] as usize;

        // Security: validate offset and size.
        if strip_offset > bytes.len() {
            return Err(format!(
                "TIFF: strip {} offset {} beyond file end {}",
                strip_idx, strip_offset, bytes.len()
            ));
        }
        let strip_end = strip_offset.checked_add(strip_byte_count).ok_or_else(|| {
            format!("TIFF: strip {} offset+count overflow", strip_idx)
        })?;
        if strip_end > bytes.len() {
            return Err(format!(
                "TIFF: strip {} extends to byte {} beyond file end {}",
                strip_idx, strip_end, bytes.len()
            ));
        }

        let compressed = &bytes[strip_offset..strip_end];

        // Compute how many rows are in this strip.
        let first_row = (strip_idx as u32) * rows_per_strip;
        let last_row = (first_row + rows_per_strip).min(height as u32);
        let rows_in_strip = (last_row - first_row) as usize;
        let expected_strip_bytes = rows_in_strip
            .checked_mul(row_stride)
            .ok_or("TIFF: strip expected bytes overflow")?;

        // Decompress this strip.
        let decompressed = compression::decompress(
            compressed,
            ifd.compression,
            expected_strip_bytes,
            ifd.predictor,
            ifd.width,
            ifd.samples_per_pixel,
            ifd.bits_per_sample.first().copied().unwrap_or(8),
        )?;

        // Append only the expected number of bytes (decompressor may over-read).
        let to_take = decompressed.len().min(expected_strip_bytes);
        output.extend_from_slice(&decompressed[..to_take]);
    }

    Ok(output)
}

// ─── Tile assembly ────────────────────────────────────────────────────────────

/// Assemble pixel data from TIFF tiles.
///
/// Tiles are `tile_width × tile_height` pixels. Tiles along the right and bottom
/// edges may extend beyond the image dimensions — we clip those edge tiles.
///
/// The tiles are stored in row-major order across the image:
///
/// ```text
/// tiles_across = ceil(width  / tile_width)
/// tile[0]   = columns 0..tile_width-1,       rows 0..tile_height-1
/// tile[1]   = columns tile_width..2tw-1,     rows 0..tile_height-1
/// ...
/// tile[N-1] = last tile in last row
/// ```
fn assemble_tiles(bytes: &[u8], ifd: &Ifd) -> Result<Vec<u8>, String> {
    let width = ifd.width as usize;
    let height = ifd.height as usize;
    let samples = ifd.samples_per_pixel as usize;
    let bits = ifd.bits_per_sample.first().copied().unwrap_or(8) as usize;
    let bytes_per_sample = bits.div_ceil(8);
    let row_stride = width * samples * bytes_per_sample;

    let tile_w = ifd.tile_width.unwrap() as usize;
    let tile_h = ifd.tile_length.unwrap() as usize;
    if tile_w == 0 || tile_h == 0 {
        return Err("TIFF: tile dimensions must be > 0".into());
    }

    // Number of tiles across and down.
    let tiles_across = width.div_ceil(tile_w);
    let tiles_down = height.div_ceil(tile_h);
    let num_tiles = tiles_across
        .checked_mul(tiles_down)
        .ok_or("TIFF: tile count overflow")?;

    if num_tiles > MAX_STRIP_COUNT {
        return Err(format!("TIFF: {} tiles exceeds max {}", num_tiles, MAX_STRIP_COUNT));
    }
    if ifd.tile_offsets.len() < num_tiles {
        return Err(format!(
            "TIFF: need {} tile offsets, have {}",
            num_tiles,
            ifd.tile_offsets.len()
        ));
    }
    if ifd.tile_byte_counts.len() < num_tiles {
        return Err(format!(
            "TIFF: need {} tile byte counts, have {}",
            num_tiles,
            ifd.tile_byte_counts.len()
        ));
    }

    // Allocate the full image buffer (all zeros = transparent for edge tiles).
    let total = height * row_stride;
    let mut output = vec![0u8; total];

    // Tile row stride (full tile width, not clipped).
    let tile_row_stride = tile_w * samples * bytes_per_sample;

    for tile_row in 0..tiles_down {
        for tile_col in 0..tiles_across {
            let tile_idx = tile_row * tiles_across + tile_col;

            let tile_offset = ifd.tile_offsets[tile_idx] as usize;
            let tile_byte_count = ifd.tile_byte_counts[tile_idx] as usize;

            // Security: validate tile offset.
            if tile_offset > bytes.len() {
                return Err(format!(
                    "TIFF: tile {} offset {} beyond file end {}",
                    tile_idx, tile_offset, bytes.len()
                ));
            }
            let tile_end = tile_offset.checked_add(tile_byte_count).ok_or_else(|| {
                format!("TIFF: tile {} offset+count overflow", tile_idx)
            })?;
            if tile_end > bytes.len() {
                return Err(format!(
                    "TIFF: tile {} extends beyond file end", tile_idx
                ));
            }

            let compressed = &bytes[tile_offset..tile_end];
            let expected_tile_bytes = tile_w * tile_h * samples * bytes_per_sample;

            let decompressed = compression::decompress(
                compressed,
                ifd.compression,
                expected_tile_bytes,
                ifd.predictor,
                tile_w as u32,
                ifd.samples_per_pixel,
                ifd.bits_per_sample.first().copied().unwrap_or(8),
            )?;

            // Copy decompressed tile rows into the output buffer.
            // Clip to actual image boundaries.
            let img_x_start = tile_col * tile_w;
            let img_y_start = tile_row * tile_h;

            for tr in 0..tile_h {
                let img_y = img_y_start + tr;
                if img_y >= height {
                    break; // this row is outside the image
                }

                // How many pixels of this tile row are within the image?
                let pixels_in_row = (img_x_start + tile_w).min(width) - img_x_start;
                let src_start = tr * tile_row_stride;
                let src_end = src_start + pixels_in_row * samples * bytes_per_sample;

                if src_end > decompressed.len() {
                    break; // decompressed data too short
                }

                let dst_start = img_y * row_stride + img_x_start * samples * bytes_per_sample;
                let dst_end = dst_start + pixels_in_row * samples * bytes_per_sample;

                if dst_end > output.len() {
                    break;
                }
                output[dst_start..dst_end].copy_from_slice(&decompressed[src_start..src_end]);
            }
        }
    }

    Ok(output)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ifd::Ifd;

    fn make_simple_ifd(width: u32, height: u32) -> Ifd {
        Ifd {
            width,
            height,
            bits_per_sample: vec![8, 8, 8],
            compression: 1,
            samples_per_pixel: 3,
            rows_per_strip: height,
            strip_offsets: vec![8],
            strip_byte_counts: vec![(width * height * 3) as u64],
            ..Ifd::default()
        }
    }

    #[test]
    fn assemble_single_strip_rgb() {
        // 2×1 RGB image: [R0,G0,B0, R1,G1,B1]
        // The strip data starts at byte 8 in our fake "file".
        let pixel_data = vec![255u8, 0, 0, 0, 255, 0]; // red, green
        let mut fake_file = vec![0u8; 8]; // 8-byte "header" padding
        fake_file.extend_from_slice(&pixel_data);

        let ifd = make_simple_ifd(2, 1);
        let result = assemble(&fake_file, &ifd).unwrap();
        assert_eq!(result, pixel_data);
    }

    #[test]
    fn assemble_two_strips_rgb() {
        // 2×2 RGB image split into 2 strips of 1 row each.
        let row0 = vec![255u8, 0, 0, 0, 255, 0]; // top row: red, green
        let row1 = vec![0u8, 0, 255, 255, 255, 255]; // bottom row: blue, white

        let mut fake_file = vec![0u8; 8]; // header padding
        let offset0 = fake_file.len() as u64;
        fake_file.extend_from_slice(&row0);
        let offset1 = fake_file.len() as u64;
        fake_file.extend_from_slice(&row1);

        let ifd = Ifd {
            width: 2,
            height: 2,
            bits_per_sample: vec![8, 8, 8],
            compression: 1,
            samples_per_pixel: 3,
            rows_per_strip: 1,
            strip_offsets: vec![offset0, offset1],
            strip_byte_counts: vec![6, 6],
            ..Ifd::default()
        };

        let result = assemble(&fake_file, &ifd).unwrap();
        let mut expected = row0;
        expected.extend_from_slice(&row1);
        assert_eq!(result, expected);
    }

    #[test]
    fn assemble_no_strips_error() {
        let ifd = Ifd {
            width: 2,
            height: 2,
            ..Ifd::default()
        };
        assert!(assemble(&[], &ifd).is_err());
    }

    #[test]
    fn assemble_strip_offset_out_of_bounds_error() {
        let ifd = Ifd {
            width: 2,
            height: 1,
            bits_per_sample: vec![8],
            samples_per_pixel: 1,
            rows_per_strip: 1,
            strip_offsets: vec![9999],
            strip_byte_counts: vec![2],
            ..Ifd::default()
        };
        assert!(assemble(&[0u8; 100], &ifd).is_err());
    }
}
