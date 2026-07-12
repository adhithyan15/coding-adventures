// # xtrans.rs — Fujifilm X-Trans 6×6 demosaicing
//
// X-Trans is Fujifilm's alternative colour filter array, introduced with the
// X-Pro1 in 2012.  Instead of the classic 2×2 Bayer tile, X-Trans uses a
// non-repeating 6×6 tile with a pseudo-random green arrangement:
//
// ```text
// Standard Fujifilm X-Trans pattern (rows 0–5, cols 0–5):
//
//   col:  0  1  2  3  4  5
// row 0:  G  B  G  G  R  G
// row 1:  R  G  R  B  G  B
// row 2:  G  B  G  G  R  G
// row 3:  G  R  G  G  B  G
// row 4:  B  G  B  R  G  R
// row 5:  G  R  G  G  B  G
// ```
//
// ## Why X-Trans?
//
// The Bayer pattern's strict regularity produces visible moiré patterns on
// fine repetitive textures (e.g., fabric weave, brick walls) because the
// sensor is in effect sampling a regular frequency grid.  X-Trans breaks
// the regularity: no colour repeats at a 1-, 2-, or 3-pixel period along any
// axis, so moiré is suppressed without an optical low-pass filter.
//
// ## Simplified bilinear demosaicing (v0.1)
//
// Full X-Trans demosaicing (as used in darktable, Rawtherapee, and Capture
// One) requires colour-specific edge detection and multi-pass refinement.
// For v0.1 we implement a simplified bilinear approach:
//
// For each output pixel `(r, c)`:
//   1. Look up its channel from the 6×6 pattern table.
//   2. For each missing channel, scan a 5×5 window centred on `(r, c)` for
//      all pixels of that channel, then average their values.
//   3. Border coordinates are clamped to `[0, dim-1]` (edge replication).
//
// The 5×5 window (rather than 3×3 as in Bayer) is needed because X-Trans
// has fewer red and blue samples per unit area, and some `(r, c)` positions
// are more than 1 pixel away from the nearest red or blue sample.
//
// Known limitation: this produces colour fringing at sharp edges.  Full AHD
// demosaicing for X-Trans is tracked as a future improvement.

/// Perform simplified bilinear demosaicing on X-Trans raw data.
///
/// # Arguments
///
/// * `raw`     — flat row-major array of 12-bit raw pixel values, length = `width * height`
/// * `width`   — number of columns
/// * `height`  — number of rows
/// * `pattern` — 36-byte row-major 6×6 X-Trans pattern; values 0=R, 1=G, 2=B
///
/// # Returns
///
/// A `Vec<(u16, u16, u16)>` with one `(R, G, B)` triple per pixel, in the
/// same row-major order as the input.
pub fn demosaic_xtrans(
    raw: &[u16],
    width: usize,
    height: usize,
    pattern: &[u8; 36],
) -> Vec<(u16, u16, u16)> {
    let n = width * height;
    let mut out = vec![(0u16, 0u16, 0u16); n];

    for row in 0..height {
        for col in 0..width {
            let idx = row * width + col;
            let ch  = xtrans_channel(row, col, pattern);

            // Each of the three channels is computed independently.
            let r = if ch == 0 {
                raw[idx] // this pixel *is* red — exact value
            } else {
                average_xtrans(raw, width, height, row, col, 0, pattern)
            };

            let g = if ch == 1 {
                raw[idx]
            } else {
                average_xtrans(raw, width, height, row, col, 1, pattern)
            };

            let b = if ch == 2 {
                raw[idx]
            } else {
                average_xtrans(raw, width, height, row, col, 2, pattern)
            };

            out[idx] = (r, g, b);
        }
    }

    out
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Return the CFA channel (0=R, 1=G, 2=B) for pixel `(row, col)` under
/// the given 6×6 X-Trans `pattern`.
#[inline]
pub fn xtrans_channel(row: usize, col: usize, pattern: &[u8; 36]) -> u8 {
    pattern[(row % 6) * 6 + (col % 6)]
}

/// Average all pixels of channel `target_ch` within a 5×5 window centred
/// on `(row, col)`, clamping border coordinates to the image bounds.
///
/// The 5×5 window gives a radius of 2.  Choosing radius 2 rather than 1
/// ensures we always capture at least one sample of each colour, even at
/// positions where the nearest same-channel pixel is 2 hops away.
// Explicit `if divisor == 0` guard is intentional (and clearer than checked_div here); allow the 1.97 manual_checked_ops lint.
#[allow(clippy::manual_checked_ops)]
fn average_xtrans(
    raw: &[u16],
    width: usize,
    height: usize,
    row: usize,
    col: usize,
    target_ch: u8,
    pattern: &[u8; 36],
) -> u16 {
    let mut sum   = 0u32;
    let mut count = 0u32;

    // Scan a 5×5 neighbourhood (offsets −2 … +2 on each axis).
    for dr in -2i32..=2 {
        for dc in -2i32..=2 {
            let nr = (row as i32 + dr).clamp(0, height as i32 - 1) as usize;
            let nc = (col as i32 + dc).clamp(0, width  as i32 - 1) as usize;

            if xtrans_channel(nr, nc, pattern) == target_ch {
                sum   += raw[nr * width + nc] as u32;
                count += 1;
            }
        }
    }

    if count == 0 {
        0
    } else {
        ((sum + count / 2) / count) as u16
    }
}
