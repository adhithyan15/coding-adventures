//! # Modular image coding — gradient predictor and residual transform
//!
//! JXL Modular mode achieves lossless compression by decorrelating pixel
//! values before entropy-coding them.  The key insight is that neighbouring
//! pixels are highly correlated: knowing the values to the left, above, and
//! diagonally adjacent gives a very good prediction of the current pixel.
//! The *residual* (actual − predicted) is small and concentrated near zero,
//! making it cheap to entropy-code.
//!
//! ## The gradient predictor
//!
//! Given a pixel at position (x, y) with neighbours:
//!
//! ```text
//!   NW  N  NE
//!    W  P
//! ```
//!
//! We compute:
//!
//! ```text
//! grad = W + N − NW               (gradient extrapolation)
//! pred = clamp(grad, min(W,N,NW,NE), max(W,N,NW,NE))
//! ```
//!
//! Clamping to the min/max of the four neighbours keeps the predictor from
//! wildly over- or under-shooting at edges and ramps.
//!
//! ## Edge conditions
//!
//! At the edges of the image we fall back to:
//!
//! | position         | W   | N   | NW  | NE  |
//! |------------------|-----|-----|-----|-----|
//! | x=0, y=0        | 0   | 0   | 0   | 0   |
//! | x=0, y>0        | N   | N   | N   | N¹  |
//! | x>0, y=0        | W   | W   | W   | W¹  |
//! | x>0, y>0, x=W-1 | ...  | ... | ... | N   |
//!
//! ¹ NE falls back to N if x = image_width − 1 *or* y = 0.
//!
//! ## Reconstruction
//!
//! Decoding is the exact inverse: scan in raster order, predict with the
//! already-reconstructed neighbours, add the residual, and store.

/// Predict the value of pixel (x, y) from already-decoded neighbours.
///
/// `channel` must be a flat row-major slice of length `width * height` where
/// only pixels before (x, y) in raster order have been filled in.
/// Pixels not yet decoded may be 0 — the predictor only reads earlier ones.
pub fn gradient_predict(x: u32, y: u32, width: u32, channel: &[i32]) -> i32 {
    // Helper closure: safely index into `channel`.
    // We never call this with coordinates that are out of the allocated slice,
    // but we *can* call it with x = width (conceptual NE for the last column),
    // which we guard against below.
    let get = |px: u32, py: u32| -> i32 { channel[(py * width + px) as usize] };

    // ── Fetch the four relevant neighbours ───────────────────────────────
    //
    // W  = pixel to the left  (x−1, y).   Boundary: 0 when x = 0.
    // N  = pixel above        (x, y−1).   Boundary: W when y = 0.
    // NW = diagonal           (x−1, y−1). Boundary: W when x=0 or y=0,
    //                                                N when x=0 and y>0.
    // NE = above-right        (x+1, y−1). Boundary: N when y=0 or x=W−1.

    let w = if x > 0 { get(x - 1, y) } else { 0 };

    let n = if y > 0 { get(x, y - 1) } else { w };

    let nw = if x > 0 && y > 0 {
        get(x - 1, y - 1)
    } else if y > 0 {
        // x == 0: use N (nothing to the left)
        n
    } else {
        // y == 0: use W (nothing above)
        w
    };

    let ne = if y > 0 && x + 1 < width {
        get(x + 1, y - 1)
    } else {
        // No pixel to the upper-right: use N
        n
    };

    // ── Gradient + clamp ─────────────────────────────────────────────────
    //
    // The gradient formula extrapolates linearly from the three cardinal
    // neighbours.  Clamping prevents overshooting at hard edges.
    let grad = w + n - nw;
    let lo = w.min(n).min(nw).min(ne);
    let hi = w.max(n).max(nw).max(ne);
    grad.clamp(lo, hi)
}

/// Compute residuals for a complete channel plane.
///
/// Scans in raster order (left-to-right, top-to-bottom).  `values` must have
/// length `width * height`.  Returns a parallel slice of residuals.
///
/// Residuals can be negative: `residual = pixel − prediction`.
pub fn compute_residuals(values: &[i32], width: u32, height: u32) -> Vec<i32> {
    let n = (width * height) as usize;
    debug_assert_eq!(values.len(), n, "channel length mismatch");

    let mut residuals = Vec::with_capacity(n);

    for y in 0..height {
        for x in 0..width {
            let pred = gradient_predict(x, y, width, values);
            residuals.push(values[(y * width + x) as usize] - pred);
        }
    }

    residuals
}

/// Reconstruct pixel values from residuals (the decode side).
///
/// Fills a buffer in raster order, using already-filled slots as neighbours for
/// the predictor.  This is the exact inverse of `compute_residuals`.
pub fn reconstruct_values(residuals: &[i32], width: u32, height: u32) -> Vec<i32> {
    let n = (width * height) as usize;
    debug_assert_eq!(residuals.len(), n, "residual slice length mismatch");

    // The buffer is filled left-to-right, top-to-bottom.  At each step the
    // predictor can already see all pixels decoded before (x, y) in raster
    // order, which is exactly what `gradient_predict` requires.
    let mut values = vec![0i32; n];

    for y in 0..height {
        for x in 0..width {
            let pred = gradient_predict(x, y, width, &values);
            values[(y * width + x) as usize] = pred + residuals[(y * width + x) as usize];
        }
    }

    values
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Verify that encode → decode is lossless for a small channel plane.
    fn round_trip_channel(values: Vec<i32>, w: u32, h: u32) {
        let residuals = compute_residuals(&values, w, h);
        let recovered = reconstruct_values(&residuals, w, h);
        assert_eq!(values, recovered, "round-trip failed for {}×{}", w, h);
    }

    #[test]
    fn single_pixel() {
        round_trip_channel(vec![42], 1, 1);
    }

    #[test]
    fn constant_value() {
        // A flat image: predictor nails every pixel after the first → tiny residuals.
        let values = vec![128i32; 16];
        round_trip_channel(values, 4, 4);
    }

    #[test]
    fn gradient_values() {
        let values: Vec<i32> = (0..16).map(|i| i * 10).collect();
        round_trip_channel(values, 4, 4);
    }

    #[test]
    fn random_8bpp_values() {
        let values: Vec<i32> = (0u8..=255).take(64).map(|v| v as i32).collect();
        round_trip_channel(values, 8, 8);
    }

    #[test]
    fn first_pixel_predicts_zero() {
        // With no neighbours the predictor should return 0.
        // For (0,0) we pass an empty-so-far buffer:
        let pred = gradient_predict(0, 0, 2, &[0, 0, 0, 0]);
        assert_eq!(pred, 0);
    }

    #[test]
    fn residuals_sum_property() {
        // For a ramp image the residuals should be almost all zero except (0,0).
        // (A linear horizontal ramp is predicted perfectly by the gradient predictor.)
        let values: Vec<i32> = (0..8).map(|x| x * 10).collect();
        let residuals = compute_residuals(&values, 8, 1);
        // First residual = pixel[0] - pred(0,0,w=8) = 0 - 0 = 0
        assert_eq!(residuals[0], 0);
        // For a perfect ramp, predictor = W for y=0, so residuals should all be 10.
        // Actually for y=0 with uniform step: N=W=0 so pred = W = values[x-1].
        // residual[x] = values[x] - values[x-1] = 10 for all x > 0.
        for &r in &residuals[1..] {
            assert_eq!(r, 10);
        }
    }
}
