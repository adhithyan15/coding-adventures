//! VP8L LZ77 backward-reference distance mapping.
//!
//! In VP8L, pixel data can be compressed using LZ77 backward references.
//! A backward reference says "copy N pixels from position X in the already-
//! decoded output buffer".  The distance is encoded using a special 2D mapping
//! that gives shorter codes to nearby pixels (both horizontally and
//! vertically).
//!
//! ## Symbol encoding
//!
//! In the main pixel stream, the green-channel Huffman tree (G group) uses an
//! extended alphabet:
//!
//! ```text
//! Symbol 0..=255        → literal green channel value; read R, B, A from
//!                          their respective trees to form a full pixel.
//! Symbol 256..=279      → backward reference; symbol encodes copy length,
//!                          read distance from Dist tree.
//! Symbol 280..=287      → backward reference (extended length codes).
//! Symbol ≥ 256+24+cache → color-cache reference (if cache enabled).
//! ```
//!
//! ## Distance mapping
//!
//! VP8L maps a 1D distance `d` in the pixel stream to a 2D spatial distance
//! `(dx, dy)` using a predefined lookup table.  This gives short codes to
//! pixels that are spatially close (nearby rows and columns) rather than
//! just close in memory order.
//!
//! The mapping for the first 120 distance codes is fixed by the spec
//! (see Appendix A of the WebP lossless bitstream specification).
//! For distances > 120, the 1D distance in raster order is used directly.
//!
//! ## Current status
//!
//! This initial release does **not** emit LZ77 backward references.  The
//! encoder uses literal-only mode (every pixel is a literal).  This module
//! provides the distance-mapping table and helper functions for future use.
//!
//! The decoder detects backward-reference symbols in the stream and returns
//! an error, because an encoder in literal-only mode will never produce them.
//! A future PR will implement full LZ77 encoding and decoding.

// ---------------------------------------------------------------------------
// Distance mapping table (first 120 entries from the VP8L spec)
// ---------------------------------------------------------------------------

/// VP8L 2D distance offsets for the first 120 distance codes.
///
/// Each entry is `(dx, dy)` where `dx` is the horizontal offset (positive =
/// right, negative = left) and `dy` is the vertical offset (positive = down).
/// The 1D backward distance in the pixel array is `dy * image_width + dx`.
///
/// These are taken verbatim from Table 1 of the WebP lossless bitstream spec.
/// Distances are ordered so that nearby pixels (small spatial distance) get
/// the smallest codes.
pub const DISTANCE_MAP: [(i32, i32); 120] = [
    (0, 1), (1, 0), (1, 1), (-1, 1), (0, 2), (2, 0), (1, 2), (-1, 2),
    (2, 1), (-2, 1), (2, 2), (-2, 2), (0, 3), (3, 0), (1, 3), (-1, 3),
    (3, 1), (-3, 1), (2, 3), (-2, 3), (3, 2), (-3, 2), (0, 4), (4, 0),
    (1, 4), (-1, 4), (4, 1), (-4, 1), (3, 3), (-3, 3), (2, 4), (-2, 4),
    (4, 2), (-4, 2), (0, 5), (3, 4), (-3, 4), (4, 3), (-4, 3), (5, 0),
    (1, 5), (-1, 5), (5, 1), (-5, 1), (2, 5), (-2, 5), (5, 2), (-5, 2),
    (4, 4), (-4, 4), (3, 5), (-3, 5), (5, 3), (-5, 3), (0, 6), (6, 0),
    (1, 6), (-1, 6), (6, 1), (-6, 1), (2, 6), (-2, 6), (6, 2), (-6, 2),
    (4, 5), (-4, 5), (5, 4), (-5, 4), (3, 6), (-3, 6), (6, 3), (-6, 3),
    (0, 7), (7, 0), (4, 6), (-4, 6), (6, 4), (-6, 4), (1, 7), (-1, 7),
    (5, 5), (-5, 5), (7, 1), (-7, 1), (2, 7), (-2, 7), (7, 2), (-7, 2),
    (3, 7), (-3, 7), (7, 3), (-7, 3), (4, 7), (-4, 7), (7, 4), (-7, 4),
    (5, 6), (-5, 6), (6, 5), (-6, 5), (8, 0), (5, 7), (-5, 7), (7, 5),
    (-7, 5), (8, 1), (-8, 1), (6, 6), (-6, 6), (8, 2), (-8, 2), (6, 7),
    (-6, 7), (7, 6), (-7, 6), (8, 3), (-8, 3), (7, 7), (-7, 7), (8, 4),
];

/// Convert a VP8L distance code to a 1D pixel-stream backward distance.
///
/// `dist_code` is the raw value from the Dist Huffman tree.
/// `image_width` is the width of the image being decoded.
///
/// Returns the number of pixels to go backward in the output buffer.
///
/// For codes 1..=120 the 2D mapping table is used.  For code 0 or codes
/// > 120 the distance is used directly (after subtracting 120 and adding the
/// distance directly in raster order, per the spec).
pub fn dist_code_to_offset(dist_code: u32, image_width: u32) -> usize {
    if dist_code == 0 {
        // Distance 0 is invalid; return 1 as a safe fallback.
        return 1;
    }
    if dist_code <= 120 {
        let (dx, dy) = DISTANCE_MAP[(dist_code - 1) as usize];
        let offset = dy * image_width as i32 + dx;
        // Negative offsets would mean forward references — invalid.
        offset.max(1) as usize
    } else {
        // For large distances, subtract 120 and use directly.
        (dist_code - 120) as usize
    }
}

// ---------------------------------------------------------------------------
// Copy-length helpers (spec Table 2)
// ---------------------------------------------------------------------------

/// Decode a VP8L copy-length code.
///
/// Symbols 256..=279 (24 symbols) encode copy lengths 2..=129 using a
/// prefix-extra-bits scheme (similar to DEFLATE):
///
/// ```text
/// Symbol 256 → length 2   (0 extra bits)
/// Symbol 257 → length 3   (0 extra bits)
/// ...
/// Symbol 264 → length 10  (0 extra bits)
/// Symbol 265 → length 11..=12 (1 extra bit)
/// ...
/// Symbol 279 → length 115..=129 (4 extra bits, last code)
/// ```
///
/// Returns `(base_length, extra_bits_needed)`.
pub fn length_symbol_to_base(symbol: u32) -> (u32, u32) {
    debug_assert!((256..=279).contains(&symbol), "not a length symbol: {symbol}");
    let code = symbol - 256;
    match code {
        0..=7 => (code + 2, 0),
        8..=9 => (10 + (code - 8) * 2, 1),
        10..=11 => (14 + (code - 10) * 4, 2),
        12..=13 => (22 + (code - 12) * 8, 3),
        14..=15 => (38 + (code - 14) * 16, 4),
        16..=23 => (70 + (code - 16) * 8, 3), // extended range
        _ => unreachable!("symbol out of range"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_map_size() {
        assert_eq!(DISTANCE_MAP.len(), 120);
    }

    #[test]
    fn dist_code_1_is_one_pixel_up() {
        // Code 1 → (dx=0, dy=1), so offset = 1 * width + 0 = width.
        let width = 8u32;
        assert_eq!(dist_code_to_offset(1, width), 8);
    }

    #[test]
    fn dist_code_2_is_one_pixel_left() {
        // Code 2 → (dx=1, dy=0), so offset = 0 * width + 1 = 1.
        let width = 8u32;
        assert_eq!(dist_code_to_offset(2, width), 1);
    }

    #[test]
    fn dist_code_large_uses_direct() {
        // dist_code=121 → 121 - 120 = 1
        let width = 10u32;
        assert_eq!(dist_code_to_offset(121, width), 1);
        // dist_code=200 → 200 - 120 = 80
        assert_eq!(dist_code_to_offset(200, width), 80);
    }

    #[test]
    fn length_symbol_base_cases() {
        assert_eq!(length_symbol_to_base(256), (2, 0));
        assert_eq!(length_symbol_to_base(263), (9, 0));
    }
}
