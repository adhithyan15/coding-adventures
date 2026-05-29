// # bayer.rs — Standard 2×2 bilinear Bayer demosaicing (RGGB)
//
// ## What is a Bayer mosaic?
//
// A digital camera sensor typically has one photosite per pixel. To record
// colour, each photosite is covered by a colour filter. Panasonic uses the
// RGGB pattern — a 2×2 repeating tile:
//
//   ┌─────┬─────┐
//   │  R  │  G  │  ← even row
//   ├─────┼─────┤
//   │  G  │  B  │  ← odd row
//   └─────┴─────┘
//    even   odd
//    col    col
//
// Every pixel on the sensor records only one colour channel; the missing two
// channels must be *interpolated* from neighbouring pixels of the same colour.
// This process is called demosaicing (or debayering).
//
// ## Bilinear Interpolation
//
// We use the simplest correct algorithm: bilinear interpolation. For each
// pixel, we average the nearest same-colour neighbours:
//
//   R pixel  → G from 4-connected (N, S, E, W) average
//              B from 4-diagonal (NE, NW, SE, SW) average
//
//   G pixel (even-row, odd-col = Gr)
//           → R from left/right average
//              B from up/down average
//
//   G pixel (odd-row, even-col = Gb)
//           → R from up/down average
//              B from left/right average
//
//   B pixel  → G from 4-connected average
//              R from 4-diagonal average
//
// Border pixels clamp their neighbour indices to the image bounds rather than
// wrapping (replication padding). This is the standard choice for RAW decoders.
//
// ## Output
//
// Returns a Vec of `(r, g, b)` tuples in u16, row-major, same dimensions as
// the input Bayer grid. The values stay in the 12-bit range [0, 4095] of the
// unpacked raw pixels.

/// Demosaic an RGGB Bayer grid using bilinear interpolation.
///
/// `raw`    — flat array of 12-bit pixel values, row-major, top-left origin.
/// `width`  — image width in pixels.
/// `height` — image height in pixels.
///
/// Returns a `Vec<(u16, u16, u16)>` of (R, G, B) triplets, one per pixel,
/// in row-major order. Values are in the u16 range [0, 4095].
pub fn demosaic_rggb(raw: &[u16], width: usize, height: usize) -> Vec<(u16, u16, u16)> {
    // A helper that reads a raw pixel, clamping out-of-bounds coordinates to
    // the nearest edge (replication padding). This avoids border special-casing
    // at every call site and is numerically equivalent to mirror padding for
    // the bilinear case.
    let get = |row: i64, col: i64| -> u16 {
        let r = row.max(0).min(height as i64 - 1) as usize;
        let c = col.max(0).min(width as i64 - 1) as usize;
        raw[r * width + c]
    };

    // Average up to four u16 values, discarding zero-weight slots (unused near
    // borders where fewer neighbours exist). The divisor is always the actual
    // number of terms, never zero.
    let avg2 = |a: u16, b: u16| -> u16 { ((a as u32 + b as u32) / 2) as u16 };
    let avg4 = |a: u16, b: u16, c: u16, d: u16| -> u16 {
        ((a as u32 + b as u32 + c as u32 + d as u32) / 4) as u16
    };

    let mut out = Vec::with_capacity(width * height);

    for row in 0..height {
        for col in 0..width {
            let r = row as i64;
            let c = col as i64;

            // Determine Bayer colour at this position.
            // RGGB tile:
            //   (even_row, even_col) → R
            //   (even_row, odd_col)  → Gr  (green in red row)
            //   (odd_row,  even_col) → Gb  (green in blue row)
            //   (odd_row,  odd_col)  → B
            let (red, green, blue) = match (row % 2, col % 2) {
                // ── Red site ──────────────────────────────────────────────
                (0, 0) => {
                    let red = get(r, c);
                    // Green: average of 4-connected same-row/same-col neighbours
                    let green = avg4(
                        get(r - 1, c), // north
                        get(r + 1, c), // south
                        get(r, c - 1), // west
                        get(r, c + 1), // east
                    );
                    // Blue: average of 4 diagonal neighbours
                    let blue = avg4(
                        get(r - 1, c - 1),
                        get(r - 1, c + 1),
                        get(r + 1, c - 1),
                        get(r + 1, c + 1),
                    );
                    (red, green, blue)
                }
                // ── Green-in-red-row (Gr) ─────────────────────────────────
                (0, 1) => {
                    let green = get(r, c);
                    // Red: left and right neighbours (same even row)
                    let red = avg2(get(r, c - 1), get(r, c + 1));
                    // Blue: up and down neighbours (odd rows)
                    let blue = avg2(get(r - 1, c), get(r + 1, c));
                    (red, green, blue)
                }
                // ── Green-in-blue-row (Gb) ────────────────────────────────
                (1, 0) => {
                    let green = get(r, c);
                    // Red: up and down (even rows)
                    let red = avg2(get(r - 1, c), get(r + 1, c));
                    // Blue: left and right (same odd row)
                    let blue = avg2(get(r, c - 1), get(r, c + 1));
                    (red, green, blue)
                }
                // ── Blue site ─────────────────────────────────────────────
                _ => {
                    let blue = get(r, c);
                    // Green: 4-connected
                    let green = avg4(
                        get(r - 1, c),
                        get(r + 1, c),
                        get(r, c - 1),
                        get(r, c + 1),
                    );
                    // Red: 4 diagonals
                    let red = avg4(
                        get(r - 1, c - 1),
                        get(r - 1, c + 1),
                        get(r + 1, c - 1),
                        get(r + 1, c + 1),
                    );
                    (red, green, blue)
                }
            };

            out.push((red, green, blue));
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demosaic_2x2_all_zeros() {
        // A 2×2 all-black Bayer grid should produce (0, 0, 0) for every pixel.
        let raw = vec![0u16; 4];
        let out = demosaic_rggb(&raw, 2, 2);
        assert_eq!(out.len(), 4);
        for (r, g, b) in &out {
            assert_eq!((*r, *g, *b), (0, 0, 0));
        }
    }

    #[test]
    fn demosaic_2x2_red_only() {
        // Only R site (0,0) is non-zero. Green and Blue sites are 0.
        // After demosaicing, the R pixel should keep its value; others derive via
        // interpolation from neighbours (all zero on a 2×2 grid at the boundary).
        //
        // Bayer grid (2×2):
        //   R=1000  G=0
        //   G=0     B=0
        let raw = vec![1000u16, 0, 0, 0];
        let out = demosaic_rggb(&raw, 2, 2);
        // (0,0) is the R site — its R value must be 1000.
        assert_eq!(out[0].0, 1000);
    }

    #[test]
    fn demosaic_uniform_produces_uniform() {
        // A uniform 4×4 Bayer grid (all pixels = 2048) should demosaic to
        // approximately (2048, 2048, 2048) at every site because all neighbour
        // averages of 2048 stay at 2048.
        let raw = vec![2048u16; 16];
        let out = demosaic_rggb(&raw, 4, 4);
        assert_eq!(out.len(), 16);
        for (r, g, b) in &out {
            assert_eq!(*r, 2048);
            assert_eq!(*g, 2048);
            assert_eq!(*b, 2048);
        }
    }

    #[test]
    fn demosaic_output_length() {
        let raw = vec![0u16; 6 * 4];
        let out = demosaic_rggb(&raw, 6, 4);
        assert_eq!(out.len(), 24);
    }
}
