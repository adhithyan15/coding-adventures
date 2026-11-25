//! # Image filter design helpers (DSP04 Phase 5)
//!
//! The canonical image-processing kernels you reach for
//! every time you write `cv2.GaussianBlur`, `cv2.Sobel`, or
//! `scipy.ndimage.laplace`.
//!
//! ## Separable kernels (length-N 1-D)
//!
//! These return a 1-D Vec ready to pass to [`crate::sep_conv2d`]
//! with the same kernel along both axes for symmetric blurs:
//!
//! - [`gaussian_blur_kernel`] — Gaussian with given σ and size.
//! - [`box_blur_kernel`] — uniform box.
//!
//! ## Non-separable 3×3 kernels
//!
//! These return a 9-element row-major 3×3 Vec ready for
//! [`crate::conv2d`] (with `kernel_height = kernel_width = 3`):
//!
//! - [`sobel_x_kernel`] / [`sobel_y_kernel`] — directional edge
//!   detection.
//! - [`laplacian_kernel`] — second-derivative / edge magnitude.
//! - [`sharpen_kernel`] — identity + scaled negative Laplacian.

use crate::ConvError;
use std::f32::consts::PI;

// ─────────────────────────── Separable 1-D ───────────────────────────

/// Checked, panic-free 1-D Gaussian blur kernel with standard deviation
/// `sigma` and length `size`. Normalised so the sum is `1.0`.
///
/// Rejects even/zero `size`, `size > MAX_KERNEL_LEN`, non-finite /
/// non-positive `sigma`, and `sigma` values so small that `2 * sigma^2`
/// underflows or `1 / (sigma * sqrt(2 * pi))` overflows to non-finite (CWE-20).
pub fn try_gaussian_blur_kernel(sigma: f32, size: u32) -> Result<Vec<f32>, ConvError> {
    if size == 0 || size.is_multiple_of(2) {
        return Err(ConvError::InvalidInput(format!(
            "size must be odd and > 0; got {}",
            size
        )));
    }
    if (size as usize) > crate::MAX_KERNEL_LEN {
        return Err(ConvError::InvalidInput(format!(
            "size {} exceeds MAX_KERNEL_LEN {}",
            size,
            crate::MAX_KERNEL_LEN
        )));
    }
    if !sigma.is_finite() || sigma <= 0.0 {
        return Err(ConvError::InvalidInput(format!(
            "sigma must be positive and finite; got {}",
            sigma
        )));
    }

    let n = size as usize;
    let centre = (n - 1) as f32 / 2.0;
    let two_sigma_sq = 2.0 * sigma * sigma;
    if !two_sigma_sq.is_finite() || two_sigma_sq <= 0.0 {
        return Err(ConvError::InvalidInput(format!(
            "sigma {} underflows or overflows 2*sigma^2 (got {})",
            sigma, two_sigma_sq
        )));
    }
    let norm_pdf = 1.0 / (sigma * (2.0 * PI).sqrt());
    if !norm_pdf.is_finite() {
        return Err(ConvError::InvalidInput(format!(
            "sigma {} overflows Gaussian normalization factor",
            sigma
        )));
    }

    let mut k = Vec::with_capacity(n);
    for i in 0..n {
        let x = (i as f32) - centre;
        let v = norm_pdf * (-(x * x) / two_sigma_sq).exp();
        if !v.is_finite() {
            return Err(ConvError::InvalidInput(
                "Gaussian kernel tap became non-finite".into(),
            ));
        }
        k.push(v);
    }
    // Re-normalise so sum = 1 exactly (the discrete truncation
    // perturbs the analytic 1.0).
    let sum: f32 = k.iter().sum();
    if !sum.is_finite() || sum <= 0.0 {
        return Err(ConvError::InvalidInput(
            "Gaussian kernel sum is non-positive or non-finite".into(),
        ));
    }
    for v in &mut k {
        *v /= sum;
        if !v.is_finite() {
            return Err(ConvError::InvalidInput(
                "normalized Gaussian kernel tap became non-finite".into(),
            ));
        }
    }
    Ok(k)
}

/// 1-D Gaussian blur kernel with standard deviation `sigma`
/// and length `size`.  Normalised so the sum is `1.0`.
///
/// `size` must be **odd** so the Gaussian's centre lands on
/// an integer index.  The function panics if either `size` is
/// even / zero, `sigma <= 0`, or `sigma` underflows/overflows.
///
/// For separable 2-D Gaussian blur, pass the returned kernel
/// as both `horizontal_kernel` and `vertical_kernel` to
/// [`crate::sep_conv2d`].
///
/// # Panics
///
/// Panics if [`try_gaussian_blur_kernel`] returns `Err`.
pub fn gaussian_blur_kernel(sigma: f32, size: u32) -> Vec<f32> {
    try_gaussian_blur_kernel(sigma, size).unwrap_or_else(|e| panic!("{e}"))
}

/// Checked, panic-free 1-D box blur (uniform) kernel of length `size`.
pub fn try_box_blur_kernel(size: u32) -> Result<Vec<f32>, ConvError> {
    if size == 0 {
        return Err(ConvError::InvalidInput("size must be > 0".into()));
    }
    if (size as usize) > crate::MAX_KERNEL_LEN {
        return Err(ConvError::InvalidInput(format!(
            "size {} exceeds MAX_KERNEL_LEN {}",
            size,
            crate::MAX_KERNEL_LEN
        )));
    }
    let n = size as usize;
    let v = 1.0 / (n as f32);
    Ok(vec![v; n])
}

/// 1-D box blur (uniform) kernel of length `size`.  Each tap
/// is `1.0 / size` so the kernel sums to `1.0`.
///
/// For separable 2-D box blur, pass the returned kernel as
/// both `horizontal_kernel` and `vertical_kernel` to
/// [`crate::sep_conv2d`].
///
/// # Panics
///
/// Panics if `size == 0` or `size > MAX_KERNEL_LEN`.
pub fn box_blur_kernel(size: u32) -> Vec<f32> {
    try_box_blur_kernel(size).unwrap_or_else(|e| panic!("{e}"))
}

// ─────────────────────────── Non-separable 3×3 ───────────────────────────

/// 3×3 Sobel kernel for horizontal-gradient (edge) detection.
///
/// Layout (row-major, length 9):
///
/// ```text
///     [-1,  0,  1,
///      -2,  0,  2,
///      -1,  0,  1]
/// ```
///
/// Pass to [`crate::conv2d`] with `kernel_height = 3, kernel_width = 3`.
/// Sums to 0 (rejects DC) — output is the rate of change in
/// the horizontal direction at each pixel.
pub fn sobel_x_kernel() -> Vec<f32> {
    vec![-1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0]
}

/// 3×3 Sobel kernel for vertical-gradient (edge) detection.
///
/// Layout (row-major):
///
/// ```text
///     [-1, -2, -1,
///       0,  0,  0,
///       1,  2,  1]
/// ```
///
/// Sums to 0.  Output is the vertical gradient — large
/// positive values where intensity increases downward.
pub fn sobel_y_kernel() -> Vec<f32> {
    vec![-1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0]
}

/// 3×3 Laplacian kernel — 4-connected discrete approximation
/// of `∇²`.
///
/// ```text
///     [0,  1, 0,
///      1, -4, 1,
///      0,  1, 0]
/// ```
///
/// Sums to 0.  Highlights second-derivative features: edges
/// and corners stand out.  Often used inside unsharp masking.
pub fn laplacian_kernel() -> Vec<f32> {
    vec![0.0, 1.0, 0.0, 1.0, -4.0, 1.0, 0.0, 1.0, 0.0]
}

/// Checked, panic-free 3×3 sharpen kernel: identity + `amount` × (-Laplacian).
///
/// Rejects non-finite `amount` and values where `1.0 + 4.0 * amount` overflows
/// `f32` to non-finite (e.g. `1e38_f32` / `f32::MAX`).
pub fn try_sharpen_kernel(amount: f32) -> Result<Vec<f32>, ConvError> {
    if !amount.is_finite() {
        return Err(ConvError::InvalidInput(format!(
            "amount must be finite; got {}",
            amount
        )));
    }
    let c = 1.0 + 4.0 * amount;
    if !c.is_finite() || !(-amount).is_finite() {
        return Err(ConvError::InvalidInput(format!(
            "amount {} overflows sharpen centre tap",
            amount
        )));
    }
    Ok(vec![
        0.0, -amount, 0.0,
        -amount, c, -amount,
        0.0, -amount, 0.0,
    ])
}

/// 3×3 sharpen kernel: identity + `amount` × (-Laplacian).
///
/// With `amount = 0.0` you get the identity (kernel does
/// nothing).  Increasing `amount` increasingly accentuates
/// edges / high-frequency detail:
///
/// ```text
///     [0,         -amount, 0,
///     -amount, 1+4·amount, -amount,
///      0,         -amount, 0]
/// ```
///
/// The kernel sums to `1.0` for any `amount`, so the average
/// brightness of the image is preserved.  Common values are
/// `amount ∈ [0.5, 2.0]`.
///
/// # Panics
///
/// Panics if [`try_sharpen_kernel`] returns `Err`.
pub fn sharpen_kernel(amount: f32) -> Vec<f32> {
    try_sharpen_kernel(amount).unwrap_or_else(|e| panic!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{conv2d, BoundaryMode};

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        let scale = a.abs().max(b.abs()).max(1.0);
        (a - b).abs() <= scale * tol
    }

    // ── Gaussian ───────────────────────────────────────────────

    #[test]
    fn gaussian_kernel_sums_to_one() {
        for &(sigma, size) in &[(0.5_f32, 3u32), (1.0, 5), (2.0, 9), (3.0, 15)] {
            let k = gaussian_blur_kernel(sigma, size);
            let sum: f32 = k.iter().sum();
            assert!(
                approx_eq(sum, 1.0, 1e-6),
                "Gaussian σ={} size={}: sum = {}",
                sigma,
                size,
                sum
            );
        }
    }

    #[test]
    fn gaussian_kernel_is_symmetric() {
        let k = gaussian_blur_kernel(1.5, 11);
        let n = k.len();
        for i in 0..(n / 2) {
            assert!(
                approx_eq(k[i], k[n - 1 - i], 1e-6),
                "asymmetry at i={}: {} vs {}",
                i,
                k[i],
                k[n - 1 - i]
            );
        }
    }

    #[test]
    fn gaussian_kernel_peak_at_centre() {
        let k = gaussian_blur_kernel(1.0, 7);
        let centre = k.len() / 2;
        let max = k.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(
            approx_eq(k[centre], max, 1e-7),
            "centre {} not max {}",
            k[centre],
            max
        );
    }

    // ── Box ────────────────────────────────────────────────────

    #[test]
    fn box_kernel_sums_to_one() {
        for size in [1u32, 3, 5, 9] {
            let k = box_blur_kernel(size);
            let sum: f32 = k.iter().sum();
            assert!(
                approx_eq(sum, 1.0, 1e-6),
                "Box size={}: sum = {}",
                size,
                sum
            );
        }
    }

    #[test]
    fn box_kernel_is_uniform() {
        let k = box_blur_kernel(7);
        let expected = 1.0 / 7.0;
        for &v in &k {
            assert!(
                approx_eq(v, expected, 1e-7),
                "non-uniform tap: {} vs {}",
                v,
                expected
            );
        }
    }

    // ── Sobel ──────────────────────────────────────────────────

    #[test]
    fn sobel_x_kernel_sums_to_zero() {
        let k = sobel_x_kernel();
        assert_eq!(k.len(), 9);
        let sum: f32 = k.iter().sum();
        assert!(approx_eq(sum, 0.0, 1e-7), "Sobel-X sum = {}", sum);
    }

    #[test]
    fn sobel_y_kernel_sums_to_zero() {
        let k = sobel_y_kernel();
        assert_eq!(k.len(), 9);
        let sum: f32 = k.iter().sum();
        assert!(approx_eq(sum, 0.0, 1e-7), "Sobel-Y sum = {}", sum);
    }

    #[test]
    fn sobel_x_responds_to_vertical_edge() {
        // 5×5 image with a vertical edge: left half = 0, right
        // half = 1.  Sobel-X should produce a strong response
        // along the edge column (column 2).
        let image: Vec<f32> = (0..25)
            .map(|i| {
                let c = i % 5;
                if c >= 3 {
                    1.0
                } else {
                    0.0
                }
            })
            .collect();
        let k = sobel_x_kernel();
        let out = conv2d(&image, &k, 5, 5, 3, 3, BoundaryMode::Replicate).unwrap();

        // Column 2 (the edge) should have a strong response (≈ ±4).
        // Verify the magnitude is much higher than at the edges
        // away from the discontinuity (column 0 or 4).
        let edge_resp = out[2 * 5 + 2].abs();
        let flat_resp = out[2 * 5].abs();
        assert!(
            edge_resp > 2.0,
            "edge response too small: {}",
            edge_resp
        );
        assert!(
            edge_resp > 3.0 * flat_resp.max(0.1),
            "edge {} not much bigger than flat {}",
            edge_resp,
            flat_resp
        );
    }

    // ── Laplacian ──────────────────────────────────────────────

    #[test]
    fn laplacian_kernel_sums_to_zero() {
        let k = laplacian_kernel();
        assert_eq!(k.len(), 9);
        let sum: f32 = k.iter().sum();
        assert!(approx_eq(sum, 0.0, 1e-7), "Laplacian sum = {}", sum);
    }

    #[test]
    fn laplacian_on_constant_image_is_zero() {
        // ∇² of a constant is 0.
        let image = vec![5.0f32; 25];
        let k = laplacian_kernel();
        let out = conv2d(&image, &k, 5, 5, 3, 3, BoundaryMode::Replicate).unwrap();
        for &v in &out {
            assert!(
                approx_eq(v, 0.0, 1e-5),
                "Laplacian on constant = {} (expected 0)",
                v
            );
        }
    }

    // ── Sharpen ────────────────────────────────────────────────

    #[test]
    fn sharpen_with_amount_zero_is_identity() {
        let k = sharpen_kernel(0.0);
        assert_eq!(k.len(), 9);
        let expected = vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        for (i, (a, b)) in k.iter().zip(expected.iter()).enumerate() {
            assert!(approx_eq(*a, *b, 1e-7), "tap {}: {} vs {}", i, a, b);
        }
    }

    #[test]
    fn sharpen_kernel_sums_to_one() {
        for &amount in &[0.0_f32, 0.5, 1.0, 2.0] {
            let k = sharpen_kernel(amount);
            let sum: f32 = k.iter().sum();
            assert!(
                approx_eq(sum, 1.0, 1e-6),
                "sharpen amount={}: sum = {}",
                amount,
                sum
            );
        }
    }

    #[test]
    fn sharpen_on_constant_image_passes_through() {
        // Sharpen kernel sums to 1, so constant input → same
        // constant output (no high-frequency content to amplify).
        let image = vec![3.0f32; 25];
        let k = sharpen_kernel(1.5);
        let out = conv2d(&image, &k, 5, 5, 3, 3, BoundaryMode::Replicate).unwrap();
        for &v in &out {
            assert!(approx_eq(v, 3.0, 1e-5), "got {}", v);
        }
    }
}
