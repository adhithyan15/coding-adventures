//! GIF87a/GIF89a encoder.
//!
//! Converts an RGBA `PixelContainer` into a complete GIF byte stream.
//!
//! # Encoding decisions (Phase 1)
//!
//! - Output format: GIF87a for fully opaque images; GIF89a with a Graphic
//!   Control Extension for images with transparent pixels (alpha < 128).
//! - Palette: exact palette if ≤ 256 distinct RGB colours appear; otherwise
//!   **median-cut quantization** to 256 colours.
//! - Interlacing: never (always write progressive scan).
//! - Animation: not supported — single frame only.
//! - `lzw_minimum_code_size` = max(2, ceil(log2(palette_size))).

use pixel_container::PixelContainer;

use crate::lzw;

// ─── Public entry point ────────────────────────────────────────────────────────

/// Encode a `PixelContainer` as a GIF byte stream.
///
/// Pixels with alpha < 128 are treated as fully transparent.
/// All other pixels are treated as fully opaque (alpha is ignored for RGB).
///
/// Returns a complete GIF file (starting with "GIF87a" or "GIF89a").
pub fn encode_gif(pixels: &PixelContainer) -> Vec<u8> {
    let width = pixels.width as usize;
    let height = pixels.height as usize;

    // Collect RGBA pixels and determine if we need transparency.
    let total = width * height;
    let mut rgb_pixels: Vec<(u8, u8, u8)> = Vec::with_capacity(total);
    let mut has_transparency = false;
    let mut transparent_positions: Vec<bool> = Vec::with_capacity(total);

    for y in 0..height {
        for x in 0..width {
            let (r, g, b, a) = pixels.pixel_at(x as u32, y as u32);
            let is_transparent = a < 128;
            if is_transparent {
                has_transparency = true;
            }
            rgb_pixels.push((r, g, b));
            transparent_positions.push(is_transparent);
        }
    }

    // ── Build palette ──
    // Collect distinct RGB colours (ignoring transparent pixels for palette building).
    let opaque_colors: Vec<(u8, u8, u8)> = rgb_pixels
        .iter()
        .enumerate()
        .filter(|(i, _)| !transparent_positions[*i])
        .map(|(_, &c)| c)
        .collect();

    // For opaque images we can use all 256 palette slots.
    // For images with transparency we must reserve one slot for the transparent
    // colour, so the opaque palette can hold at most 255 distinct colours.
    let max_opaque = if has_transparency { 255 } else { 256 };
    let mut palette = build_palette(&opaque_colors, max_opaque);

    // If we need transparency, reserve the last palette slot for it.
    let transparent_index: u8 = if has_transparency {
        let ti = palette.len() as u8;
        palette.push((0, 0, 0)); // transparent color (value doesn't matter)
        ti
    } else {
        0
    };

    // Palette must have at least 2 entries; pad with black if needed.
    while palette.len() < 2 {
        palette.push((0, 0, 0));
    }

    // ── Determine GCT size ──
    // GCT size = 2^(n+1); find smallest n such that 2^(n+1) >= palette.len().
    let palette_size = palette.len();
    let gct_size_field = required_gct_field(palette_size);
    let gct_count = 1usize << (gct_size_field + 1);
    // Pad palette to gct_count entries.
    while palette.len() < gct_count {
        palette.push((0, 0, 0));
    }

    // ── Map pixels to palette indices ──
    let indices: Vec<u8> = rgb_pixels
        .iter()
        .enumerate()
        .map(|(i, &color)| {
            if transparent_positions[i] {
                transparent_index
            } else {
                nearest_palette_index(color, &palette[..palette_size])
            }
        })
        .collect();

    // ── Determine lzw_minimum_code_size ──
    let min_code_size = required_min_code_size(palette_size);

    // ── Build the GIF byte stream ──
    let mut out: Vec<u8> = Vec::new();

    // Header: GIF89a if transparency needed, else GIF87a.
    if has_transparency {
        out.extend_from_slice(b"GIF89a");
    } else {
        out.extend_from_slice(b"GIF87a");
    }

    // Logical Screen Descriptor.
    out.extend_from_slice(&(width as u16).to_le_bytes());
    out.extend_from_slice(&(height as u16).to_le_bytes());
    // Packed byte: global_ct_flag=1, color_resolution=1 (unused), sort=0, gct_size_field.
    let packed: u8 = 0b1000_0000 | (gct_size_field as u8 & 0x07);
    out.push(packed);
    out.push(0); // background color index
    out.push(0); // pixel aspect ratio (unspecified)

    // Global Color Table.
    for &(r, g, b) in &palette {
        out.push(r);
        out.push(g);
        out.push(b);
    }

    // Graphic Control Extension (GIF89a only, for transparency).
    if has_transparency {
        out.push(0x21); // Extension Introducer
        out.push(0xF9); // Graphic Control label
        out.push(0x04); // block size
        out.push(0x01); // packed: transparent_color_flag = 1
        out.extend_from_slice(&0u16.to_le_bytes()); // delay = 0
        out.push(transparent_index);
        out.push(0x00); // terminator
    }

    // Image Descriptor.
    out.push(0x2C); // Image Separator
    out.extend_from_slice(&0u16.to_le_bytes()); // left = 0
    out.extend_from_slice(&0u16.to_le_bytes()); // top = 0
    out.extend_from_slice(&(width as u16).to_le_bytes());
    out.extend_from_slice(&(height as u16).to_le_bytes());
    out.push(0x00); // packed: no local color table, no interlace

    // LZW-compressed image data.
    let lzw_data = lzw::encode(&indices, min_code_size);
    out.extend_from_slice(&lzw_data);

    // Trailer.
    out.push(0x3B);

    out
}

// ─── Palette helpers ──────────────────────────────────────────────────────────

/// Build a palette with at most `max_colors` entries from the given colour list.
///
/// If the distinct colour count is ≤ `max_colors`, uses the exact set.
/// Otherwise, applies median-cut quantization.
fn build_palette(colors: &[(u8, u8, u8)], max_colors: usize) -> Vec<(u8, u8, u8)> {
    if colors.is_empty() {
        return vec![(0, 0, 0)];
    }

    // Collect distinct colours using a simple set.
    let mut distinct: Vec<(u8, u8, u8)> = colors.to_vec();
    distinct.sort_unstable();
    distinct.dedup();

    if distinct.len() <= max_colors {
        return distinct;
    }

    // Median-cut: partition the colour space into `max_colors` buckets.
    median_cut(colors, max_colors)
}

/// Median-cut colour quantization.
///
/// Recursively splits colour buckets along the axis with the greatest range,
/// until we have `target` buckets. Each bucket's representative is its
/// average RGB.
fn median_cut(colors: &[(u8, u8, u8)], target: usize) -> Vec<(u8, u8, u8)> {
    if target == 0 || colors.is_empty() {
        return Vec::new();
    }

    let mut buckets: Vec<Vec<(u8, u8, u8)>> = vec![colors.to_vec()];

    while buckets.len() < target {
        // Find the bucket with the largest range.
        let idx = largest_range_bucket(&buckets);
        let bucket = buckets.remove(idx);

        // Find the axis with the greatest range.
        let (r_min, r_max) = min_max(&bucket, |c| c.0);
        let (g_min, g_max) = min_max(&bucket, |c| c.1);
        let (b_min, b_max) = min_max(&bucket, |c| c.2);
        let r_range = r_max - r_min;
        let g_range = g_max - g_min;
        let b_range = b_max - b_min;

        // Sort by the widest axis and split at the median.
        let mut sorted = bucket;
        if r_range >= g_range && r_range >= b_range {
            sorted.sort_unstable_by_key(|c| c.0);
        } else if g_range >= b_range {
            sorted.sort_unstable_by_key(|c| c.1);
        } else {
            sorted.sort_unstable_by_key(|c| c.2);
        }

        let mid = sorted.len() / 2;
        let (lo, hi) = sorted.split_at(mid);
        if !lo.is_empty() {
            buckets.push(lo.to_vec());
        }
        if !hi.is_empty() {
            buckets.push(hi.to_vec());
        }

        if buckets.len() >= target {
            break;
        }
    }

    // Compute average colour for each bucket.
    buckets
        .iter()
        .map(|b| {
            let (rs, gs, bs) = b.iter().fold((0u64, 0u64, 0u64), |(r, g, b_), &(cr, cg, cb)| {
                (r + cr as u64, g + cg as u64, b_ + cb as u64)
            });
            let n = b.len() as u64;
            ((rs / n) as u8, (gs / n) as u8, (bs / n) as u8)
        })
        .collect()
}

fn largest_range_bucket(buckets: &[Vec<(u8, u8, u8)>]) -> usize {
    let mut best = 0;
    let mut best_range = 0u32;
    for (i, b) in buckets.iter().enumerate() {
        let (r_min, r_max) = min_max(b, |c| c.0);
        let (g_min, g_max) = min_max(b, |c| c.1);
        let (b_min, b_max) = min_max(b, |c| c.2);
        let range = (r_max - r_min) as u32
            + (g_max - g_min) as u32
            + (b_max - b_min) as u32;
        if range > best_range {
            best_range = range;
            best = i;
        }
    }
    best
}

fn min_max<F: Fn(&(u8, u8, u8)) -> u8>(v: &[(u8, u8, u8)], f: F) -> (u8, u8) {
    let mut lo = 255u8;
    let mut hi = 0u8;
    for item in v {
        let val = f(item);
        if val < lo {
            lo = val;
        }
        if val > hi {
            hi = val;
        }
    }
    (lo, hi)
}

/// Find the nearest palette index for a given RGB colour using squared Euclidean distance.
fn nearest_palette_index(color: (u8, u8, u8), palette: &[(u8, u8, u8)]) -> u8 {
    let mut best_idx = 0usize;
    let mut best_dist = u32::MAX;
    let (r, g, b) = color;
    for (i, &(pr, pg, pb)) in palette.iter().enumerate() {
        let dr = (r as i32 - pr as i32).pow(2) as u32;
        let dg = (g as i32 - pg as i32).pow(2) as u32;
        let db = (b as i32 - pb as i32).pow(2) as u32;
        let dist = dr + dg + db;
        if dist < best_dist {
            best_dist = dist;
            best_idx = i;
            if dist == 0 {
                break; // exact match
            }
        }
    }
    best_idx as u8
}

/// Compute `size_of_gct` field: smallest n such that 2^(n+1) >= count.
fn required_gct_field(count: usize) -> usize {
    let mut n = 0usize;
    while (1 << (n + 1)) < count {
        n += 1;
        if n == 7 {
            break;
        }
    }
    n
}

/// Compute `lzw_minimum_code_size`: smallest n ≥ 2 such that 2^n >= palette_size.
fn required_min_code_size(palette_size: usize) -> u8 {
    let mut n = 2u8;
    while (1usize << n) < palette_size {
        n += 1;
        if n == 8 {
            break;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_min_code_size_values() {
        assert_eq!(required_min_code_size(2), 2);
        assert_eq!(required_min_code_size(4), 2);
        assert_eq!(required_min_code_size(5), 3);
        assert_eq!(required_min_code_size(16), 4);
        assert_eq!(required_min_code_size(17), 5);
        assert_eq!(required_min_code_size(256), 8);
    }

    #[test]
    fn required_gct_field_values() {
        assert_eq!(required_gct_field(2), 0); // 2^1 = 2
        assert_eq!(required_gct_field(4), 1); // 2^2 = 4
        assert_eq!(required_gct_field(5), 2); // 2^3 = 8
        assert_eq!(required_gct_field(256), 7); // 2^8 = 256
    }

    #[test]
    fn nearest_index_exact_match() {
        let palette = vec![(255, 0, 0), (0, 255, 0), (0, 0, 255)];
        assert_eq!(nearest_palette_index((255, 0, 0), &palette), 0);
        assert_eq!(nearest_palette_index((0, 255, 0), &palette), 1);
        assert_eq!(nearest_palette_index((0, 0, 255), &palette), 2);
    }

    #[test]
    fn build_palette_exact_under_limit() {
        let colors: Vec<(u8, u8, u8)> = vec![(0, 0, 0), (255, 0, 0), (0, 255, 0)];
        let p = build_palette(&colors, 255);
        assert_eq!(p.len(), 3);
    }

    #[test]
    fn encode_starts_with_gif87a() {
        let mut pc = PixelContainer::new(1, 1);
        pc.set_pixel(0, 0, 255, 0, 0, 255);
        let bytes = encode_gif(&pc);
        assert_eq!(&bytes[..6], b"GIF87a");
    }

    #[test]
    fn encode_transparent_starts_with_gif89a() {
        let mut pc = PixelContainer::new(1, 1);
        pc.set_pixel(0, 0, 0, 0, 0, 0); // fully transparent
        let bytes = encode_gif(&pc);
        assert_eq!(&bytes[..6], b"GIF89a");
    }

    #[test]
    fn encode_ends_with_trailer() {
        let mut pc = PixelContainer::new(2, 2);
        pc.fill(100, 150, 200, 255);
        let bytes = encode_gif(&pc);
        assert_eq!(*bytes.last().unwrap(), 0x3B);
    }
}
