// # bayer.rs — Standard 2×2 Bayer demosaicing
//
// A colour camera sensor has one photodiode per pixel.  A colour filter array
// (CFA) in front of the sensor means each photodiode captures only one colour
// (R, G, or B).  For the classic RGGB Bayer pattern:
//
// ```text
// R  G  R  G  ...
// G  B  G  B  ...
// R  G  R  G  ...
// G  B  G  B  ...
// ```
//
// To reconstruct full-colour pixels, the missing two channels must be
// *interpolated* from neighbouring pixels of the right colour.  This process
// is called "demosaicing".
//
// ## Bilinear interpolation
//
// The simplest algorithm: for each missing channel, take the average of the
// nearest available samples of that channel.  For example, the G value at an
// R pixel is the average of its four cardinal G neighbours; the B value at an
// R pixel is the average of its four diagonal B neighbours.
//
// Bilinear demosaicing produces visible colour fringing at sharp edges (the
// "zipper" artifact).  More advanced algorithms (AHD, VNG, DHT) reduce this,
// but bilinear is sufficient for v0.1.
//
// ## Border handling
//
// Pixels on the image border are missing some neighbours.  We use edge
// replication: if a neighbour coordinate is out of bounds, we clamp it to the
// nearest in-bounds coordinate.  This keeps the algorithm simple and avoids
// introducing new colour values at the boundary.
//
// ## Pattern encoding
//
// The `pattern` array is row-major 2×2:
//   pattern[0] = top-left
//   pattern[1] = top-right
//   pattern[2] = bottom-left
//   pattern[3] = bottom-right
// Values: 0=R, 1=G, 2=B.
//
// The RGGB pattern → [0, 1, 1, 2].

/// Perform bilinear Bayer demosaicing on a 2D raw sensor grid.
///
/// # Arguments
///
/// * `raw`     — flat row-major array of 12-bit raw pixel values, length = `width * height`
/// * `width`   — number of columns
/// * `height`  — number of rows
/// * `pattern` — 2×2 CFA pattern, row-major [tl, tr, bl, br]; values 0=R, 1=G, 2=B
///
/// # Returns
///
/// A `Vec<(u16, u16, u16)>` with one `(R, G, B)` triple per pixel, in the
/// same row-major order as the input.
pub fn demosaic_bayer_2x2(
    raw: &[u16],
    width: usize,
    height: usize,
    pattern: [u8; 4],
) -> Vec<(u16, u16, u16)> {
    let n = width * height;
    let mut out = vec![(0u16, 0u16, 0u16); n];

    for row in 0..height {
        for col in 0..width {
            // What channel does this pixel contain?
            let ch = channel_at(row, col, &pattern);

            // Gather the three channels by averaging nearby same-channel pixels.
            let r = average_channel(raw, width, height, row, col, 0, &pattern);
            let g = average_channel(raw, width, height, row, col, 1, &pattern);
            let b = average_channel(raw, width, height, row, col, 2, &pattern);

            // The pixel's own channel value is exact; the others are averaged.
            // Overwrite whatever `average_channel` computed for the actual
            // channel with the exact sensor reading.
            let idx = row * width + col;
            out[idx] = match ch {
                0 => (raw[idx], g, b),
                1 => (r, raw[idx], b),
                2 => (r, g, raw[idx]),
                _ => (r, g, b),
            };
        }
    }

    out
}

// ── channel lookup ────────────────────────────────────────────────────────────

/// Return the CFA channel (0=R, 1=G, 2=B) for a pixel at `(row, col)`.
#[inline]
fn channel_at(row: usize, col: usize, pattern: &[u8; 4]) -> u8 {
    // The 2×2 pattern tiles across the whole image.
    let pr = row % 2; // 0 or 1
    let pc = col % 2; // 0 or 1
    pattern[pr * 2 + pc]
}

// ── bilinear averaging ────────────────────────────────────────────────────────

/// Compute the average value of pixels of channel `target_ch` that are
/// neighbours of `(row, col)`, using the 2×2 Bayer pattern `pattern`.
///
/// If `(row, col)` itself is `target_ch`, we return `raw[row * width + col]`
/// directly (no averaging needed).
///
/// For missing channels, we search a 3×3 neighbourhood (cardinal + diagonal)
/// for all pixels of `target_ch`, clamp out-of-bounds coordinates to the
/// nearest valid position (edge replication), and return the integer average.
// Explicit `if divisor == 0` guard is intentional (and clearer than checked_div here); allow the 1.97 manual_checked_ops lint.
#[allow(clippy::manual_checked_ops)]
fn average_channel(
    raw: &[u16],
    width: usize,
    height: usize,
    row: usize,
    col: usize,
    target_ch: u8,
    pattern: &[u8; 4],
) -> u16 {
    // If the centre pixel is already the target channel, return it directly.
    if channel_at(row, col, pattern) == target_ch {
        return raw[row * width + col];
    }

    // Collect same-channel neighbours within a 3×3 window.
    // "Clamped" row/col: isize arithmetic, then saturate at [0, dim-1].
    let mut sum = 0u32;
    let mut count = 0u32;

    for dr in -1i32..=1 {
        for dc in -1i32..=1 {
            if dr == 0 && dc == 0 {
                continue; // skip self; it's a different channel
            }
            // Clamp the neighbour coordinate to the image boundary.
            let nr = (row as i32 + dr).clamp(0, height as i32 - 1) as usize;
            let nc = (col as i32 + dc).clamp(0, width as i32 - 1) as usize;

            if channel_at(nr, nc, pattern) == target_ch {
                sum   += raw[nr * width + nc] as u32;
                count += 1;
            }
        }
    }

    if count == 0 {
        // No same-channel neighbour found within the 3×3 window.  This can
        // only happen with exotic / non-RGGB patterns; fall back to zero.
        0
    } else {
        // Integer division rounds toward zero; adding count/2 rounds to nearest.
        ((sum + count / 2) / count) as u16
    }
}
