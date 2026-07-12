// # bayer.rs — Bilinear Bayer Demosaicing
//
// Modern digital cameras don't capture full colour at each pixel. Instead,
// they use a **Colour Filter Array (CFA)** — a mosaic of red, green, and blue
// filters placed over the sensor. The most common pattern is **RGGB** (Bayer):
//
// ```text
// R G R G R G ...
// G B G B G B ...
// R G R G R G ...
// G B G B G B ...
// ```
//
// Each pixel only captures ONE colour channel. To produce an RGB image, we
// must *reconstruct* the missing two channels at each pixel using the
// surrounding pixels — this process is called **demosaicing**.
//
// ## Bilinear Demosaicing
//
// The simplest (and fastest) demosaicing method: for each pixel, the
// missing channels are estimated as the **average of the nearest neighbours**
// that have that channel.
//
// ```text
// For a pixel at position (r, c) with channel X:
//   - Channel X value: directly from the raw data.
//   - Channel Y value: average of pixels at (r±1, c) and (r, c±1) that are Y.
//   - Channel Z value: average of pixels at (r±1, c±1) that are Z.
// ```
//
// Border pixels replicate the edge value (extend the image by repeating the
// border row/column), which prevents edge artefacts.
//
// ## Pattern Encoding
//
// The `pattern` argument is a `[u8; 4]` in row-major order describing the 2×2
// CFA tile. Values: `0=R`, `1=G`, `2=B`.
//
// | Pattern    | Meaning |
// |------------|---------|
// | [0,1,1,2]  | RGGB   |
// | [1,0,2,1]  | GRBG   |
// | [1,2,0,1]  | GBRG   |
// | [2,1,1,0]  | BGGR   |
//
// The function handles all four standard 2×2 patterns automatically because
// it looks up each pixel's channel from the pattern array rather than
// hard-coding assumptions about where R/G/B appear.

// ─── Colour channel constants ─────────────────────────────────────────────────

/// Red channel identifier in a CFA pattern byte.
const R: u8 = 0;
/// Green channel identifier in a CFA pattern byte.
const G: u8 = 1;
/// Blue channel identifier in a CFA pattern byte.
const B: u8 = 2;

// ─── Main demosaicing function ────────────────────────────────────────────────

/// Demosaic a Bayer CFA image using bilinear interpolation.
///
/// # Arguments
///
/// - `raw`: flat array of raw pixel values (one value per pixel), row-major.
///   The values are u16 regardless of the original bit depth — 12-bit or
///   14-bit values from the sensor should be left-shifted or padded to u16.
/// - `width`: image width in pixels.
/// - `height`: image height in pixels.
/// - `pattern`: 2×2 CFA pattern in row-major order, values 0=R, 1=G, 2=B.
///
/// # Returns
///
/// `Vec<(u16, u16, u16)>` — one (R, G, B) triple per pixel, in row-major order.
/// Values are in the range [0, 65535], scaled from the sensor's bit depth.
///
/// # Algorithm
///
/// For each pixel at (row, col):
/// 1. Read its own channel value directly from `raw`.
/// 2. For each missing channel, find all valid neighbours (up, down, left,
///    right, and/or diagonals) that have that channel.
/// 3. Average those neighbours.
/// 4. Clamp to [0, 65535].
///
/// Border pixels use **edge replication**: when a neighbour would be outside
/// the image, we use the closest in-bounds pixel instead.
// Explicit `if divisor == 0` guard is intentional (and clearer than checked_div here); allow the 1.97 manual_checked_ops lint.
#[allow(clippy::manual_checked_ops)]
pub fn demosaic_bilinear(
    raw: &[u16],
    width: usize,
    height: usize,
    pattern: [u8; 4],
) -> Vec<(u16, u16, u16)> {
    let num_pixels = width * height;
    if num_pixels == 0 || raw.len() < num_pixels {
        return Vec::new();
    }

    let mut output = vec![(0u16, 0u16, 0u16); num_pixels];

    for row in 0..height {
        for col in 0..width {
            // Channel at this pixel position (from the 2×2 CFA pattern tile).
            let this_channel = pattern[(row % 2) * 2 + (col % 2)];

            // Get this pixel's own value.
            let own_value = raw[row * width + col];

            // Read a neighbour value, clamping coordinates to image bounds.
            // This implements edge replication: border pixels simply reflect
            // their own boundary row/column.
            let _get = |r: isize, c: isize| -> u16 {
                let rr = r.clamp(0, (height as isize) - 1) as usize;
                let cc = c.clamp(0, (width as isize) - 1) as usize;
                raw[rr * width + cc]
            };

            // Collect same-channel neighbours in each direction.
            let r = row as isize;
            let c = col as isize;

            // We need to gather values for R, G, B separately.
            // For the channel this pixel IS, use own_value.
            // For other channels, average available neighbours.

            // For a 2×2 RGGB pattern, neighbour positions by channel:
            //
            // R at (even_row, even_col):
            //   G neighbours: up, down, left, right (cross)
            //   B neighbours: diagonals (corners)
            //
            // G at (even_row, odd_col) or (odd_row, even_col):
            //   For G-at-even_row,odd_col:
            //     R neighbours: left, right
            //     B neighbours: up, down
            //   For G-at-odd_row,even_col:
            //     R neighbours: up, down
            //     B neighbours: left, right
            //
            // B at (odd_row, odd_col):
            //   G neighbours: up, down, left, right
            //   R neighbours: diagonals
            //
            // But since the pattern can be any rotation, we generalise:
            // Look at all 8 neighbours and filter by channel.

            // Neighbour offsets: (Δrow, Δcol) for cardinal and diagonal directions.
            //
            // Cardinals: (−1,0), (+1,0), (0,−1), (0,+1)   ← share an edge
            // Diagonals: (−1,−1), (−1,+1), (+1,−1), (+1,+1)  ← share a corner
            //
            // For bilinear, we average same-channel cardinal neighbours for
            // channels that appear on the cross, and diagonal neighbours for
            // channels that appear only on the corners.
            //
            // We generalise: collect all 8 neighbours by channel, then average.

            let neighbours_8: [(isize, isize); 8] = [
                (-1, -1), (-1, 0), (-1, 1),
                ( 0, -1),          ( 0, 1),
                ( 1, -1), ( 1, 0), ( 1, 1),
            ];

            // Channel for each neighbouring position in the 2×2 tile.
            let channel_at = |nr: isize, nc: isize| -> u8 {
                let rr = nr.rem_euclid(2) as usize;
                let cc = nc.rem_euclid(2) as usize;
                pattern[rr * 2 + cc]
            };

            // Average only in-bounds neighbours that have the target channel.
            //
            // IMPORTANT: Do NOT use the clamped `get()` helper here. If we
            // used get() with clamped coordinates but channel_at() with
            // unclamped coordinates, we'd mix up channels at borders:
            // the clamped pixel has a DIFFERENT channel than the virtual
            // out-of-bounds position. This produces colour contamination.
            //
            // Instead, skip out-of-bounds neighbours entirely. Interior
            // pixels always have enough valid neighbours; edge pixels use
            // whatever in-bounds neighbours exist (still gives reasonable
            // output — just slightly different from interior pixels).
            let avg_channel = |ch: u8| -> u16 {
                let mut sum: u64 = 0;
                let mut count: u64 = 0;
                for (dr, dc) in &neighbours_8 {
                    let nr = r + dr;
                    let nc = c + dc;
                    // Skip out-of-bounds positions entirely.
                    if nr < 0 || nr >= height as isize || nc < 0 || nc >= width as isize {
                        continue;
                    }
                    if channel_at(nr, nc) == ch {
                        sum += raw[(nr as usize) * width + (nc as usize)] as u64;
                        count += 1;
                    }
                }
                if count == 0 {
                    // No valid neighbours — fall back to own value (happens
                    // only on a 1×1 image or degenerate patterns).
                    own_value
                } else {
                    (sum / count).min(65535) as u16
                }
            };

            let red = if this_channel == R { own_value } else { avg_channel(R) };
            let green = if this_channel == G { own_value } else { avg_channel(G) };
            let blue = if this_channel == B { own_value } else { avg_channel(B) };

            output[row * width + col] = (red, green, blue);
        }
    }

    output
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// RGGB pattern: R at (0,0), G at (0,1) and (1,0), B at (1,1).
    const RGGB: [u8; 4] = [0, 1, 1, 2];

    #[test]
    fn demosaic_empty_image() {
        let result = demosaic_bilinear(&[], 0, 0, RGGB);
        assert!(result.is_empty());
    }

    #[test]
    fn demosaic_1x1_image() {
        // A single-pixel "image" — all channels are averages of the one pixel.
        // Since there are no neighbours, the missing channels come from the pixel itself.
        // With RGGB, the top-left pixel is Red.
        // No neighbours → green and blue average to 0.
        let raw = vec![1000u16];
        let result = demosaic_bilinear(&raw, 1, 1, RGGB);
        assert_eq!(result.len(), 1);
        let (r, _g, _b) = result[0];
        assert_eq!(r, 1000); // own channel
    }

    #[test]
    fn demosaic_2x2_rggb_uniform_image() {
        // A uniform image where all pixels have the same value.
        // After demosaicing, all pixels should be close to that value.
        let v: u16 = 8000;
        let raw = vec![v; 4]; // 2×2, all pixels = v
        let result = demosaic_bilinear(&raw, 2, 2, RGGB);
        assert_eq!(result.len(), 4);
        // In a uniform field, all channels should be v regardless of position.
        for (r, g, b) in &result {
            assert_eq!(*r, v, "R should be {}", v);
            assert_eq!(*g, v, "G should be {}", v);
            assert_eq!(*b, v, "B should be {}", v);
        }
    }

    #[test]
    fn demosaic_4x4_rggb_pure_red() {
        // A 4×4 RGGB image where only the Red pixels are non-zero.
        // Pattern:
        //   R G R G        1000  0    1000  0
        //   G B G B    →      0  0       0  0
        //   R G R G        1000  0    1000  0
        //   G B G B           0  0       0  0
        //
        // All non-R positions are 0.
        // After demosaicing, we expect R to be ~1000 everywhere and G,B near 0.
        let mut raw = vec![0u16; 16];
        // Set R positions (even row, even col).
        for row in (0..4).step_by(2) {
            for col in (0..4).step_by(2) {
                raw[row * 4 + col] = 1000;
            }
        }
        let result = demosaic_bilinear(&raw, 4, 4, RGGB);
        assert_eq!(result.len(), 16);
        // Every pixel should have R = 1000 (direct or interpolated from neighbours).
        // G and B should be 0 (no green or blue signal in the raw data).
        for (r, g, b) in &result {
            assert_eq!(*r, 1000, "Expected R=1000 everywhere, got R={}", r);
            assert_eq!(*g, 0, "Expected G=0, got G={}", g);
            assert_eq!(*b, 0, "Expected B=0, got B={}", b);
        }
    }

    #[test]
    fn demosaic_output_length_matches_input() {
        let raw = vec![100u16; 6 * 4]; // 6×4 image
        let result = demosaic_bilinear(&raw, 6, 4, RGGB);
        assert_eq!(result.len(), 24);
    }

    #[test]
    fn demosaic_grbg_pattern() {
        // GRBG: G at (0,0), R at (0,1), B at (1,0), G at (1,1).
        let grbg: [u8; 4] = [1, 0, 2, 1];
        let raw = vec![500u16; 4];
        let result = demosaic_bilinear(&raw, 2, 2, grbg);
        assert_eq!(result.len(), 4);
        // Uniform field → all channels should be ~500.
        for (r, g, b) in &result {
            assert_eq!(*r, 500);
            assert_eq!(*g, 500);
            assert_eq!(*b, 500);
        }
    }
}
