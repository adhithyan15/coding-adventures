//! # Separable 2-D convolution (DSP04 Phase 4)
//!
//! Most useful image filter kernels — Gaussian, box blur,
//! Sobel, anything built from `[1, 2, 1] ⊗ [1, 2, 1]` style
//! constructions — are **separable**: the 2-D kernel can be
//! written as the outer product of two 1-D kernels.  When
//! that's true, computing the 2-D convolution as one row-pass
//! followed by one column-pass cuts the work from
//! `O(H · W · KH · KW)` down to `O(H · W · (KH + KW))` —
//! a big win for typical 3×3, 5×5, 7×7, … kernels.
//!
//! ## Algorithm
//!
//! ```text
//!   intermediate[r, c] = Σ_k  horizontal_kernel[k]
//!                            · image_ext[r, c + cw - k]
//!
//!   out[r, c]          = Σ_k  vertical_kernel[k]
//!                            · intermediate_ext[r + ch - k, c]
//! ```
//!
//! Each pass is a call to [`crate::conv1d`] under the hood
//! (row pass takes a length-`W` slice and outputs a length-`W`
//! slice; column pass gathers a length-`H` column, runs
//! `conv1d`, scatters back).
//!
//! The boundary mode is applied along the corresponding axis
//! in each pass.

use crate::{conv1d, BoundaryMode, ConvError};

/// Same-size separable 2-D convolution.  Applies
/// `horizontal_kernel` along each row of `image`, then
/// `vertical_kernel` down each column of the result.  Output
/// is the same `[H, W]` row-major buffer as the input.
///
/// Asymptotically faster than [`crate::conv2d`] when the
/// 2-D kernel factors as `vertical_kernel ⊗ horizontal_kernel`:
///
/// - Direct 2-D: `O(H · W · KH · KW)`
/// - Separable:  `O(H · W · (KH + KW))`
///
/// For a 5×5 Gaussian on a 1080p image that's the difference
/// between 50M and 20M multiplies — 2.5× faster.
///
/// Both kernels are independently subject to the empty-kernel
/// check; either being empty returns `ConvError::EmptyKernel`.
/// Image-dimension checks match [`crate::conv2d`].
///
/// The boundary mode is applied along the horizontal axis
/// during the row pass and along the vertical axis during
/// the column pass.
pub fn sep_conv2d(
    image: &[f32],
    horizontal_kernel: &[f32],
    vertical_kernel: &[f32],
    image_height: u32,
    image_width: u32,
    mode: BoundaryMode,
) -> Result<Vec<f32>, ConvError> {
    if image_height == 0 || image_width == 0 {
        return Err(ConvError::ImageSizeMismatch(format!(
            "image dimensions must be non-zero; got {}×{}",
            image_height, image_width
        )));
    }
    if horizontal_kernel.is_empty() || vertical_kernel.is_empty() {
        return Err(ConvError::EmptyKernel);
    }
    let h = image_height as usize;
    let w = image_width as usize;
    let expected_image_len = h.checked_mul(w).ok_or_else(|| {
        ConvError::ImageSizeMismatch(format!(
            "image dimensions overflow: {}×{}",
            h, w
        ))
    })?;
    if image.len() != expected_image_len {
        return Err(ConvError::ImageSizeMismatch(format!(
            "image length {} does not match {}×{} = {}",
            image.len(),
            h,
            w,
            expected_image_len
        )));
    }
    if horizontal_kernel.len() > w {
        return Err(ConvError::KernelTooLarge(format!(
            "horizontal kernel length {} > image width {}",
            horizontal_kernel.len(),
            w
        )));
    }
    if vertical_kernel.len() > h {
        return Err(ConvError::KernelTooLarge(format!(
            "vertical kernel length {} > image height {}",
            vertical_kernel.len(),
            h
        )));
    }

    // ── Step 1: row pass.  For each row r ∈ [0, H), run
    //   conv1d(image[r * W..(r+1) * W], horizontal_kernel)
    //   and store into intermediate[r * W..(r+1) * W].
    let mut intermediate = vec![0.0f32; expected_image_len];
    for r in 0..h {
        let row = &image[r * w..(r + 1) * w];
        let row_out = conv1d(row, horizontal_kernel, mode)?;
        intermediate[r * w..(r + 1) * w].copy_from_slice(&row_out);
    }

    // ── Step 2: column pass.  For each column c ∈ [0, W),
    //   gather intermediate[:, c] into a temp vec, run
    //   conv1d(.., vertical_kernel), scatter back.
    let mut out = vec![0.0f32; expected_image_len];
    let mut col_buf = vec![0.0f32; h];
    for c in 0..w {
        for r in 0..h {
            col_buf[r] = intermediate[r * w + c];
        }
        let col_out = conv1d(&col_buf, vertical_kernel, mode)?;
        for r in 0..h {
            out[r * w + c] = col_out[r];
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conv2d;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        let scale = a.abs().max(b.abs()).max(1.0);
        (a - b).abs() <= scale * tol
    }

    fn assert_close(a: &[f32], b: &[f32], tol: f32) {
        assert_eq!(a.len(), b.len(), "length mismatch");
        for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
            assert!(
                approx_eq(*x, *y, tol),
                "mismatch at {}: {} vs {} (tol {})",
                i,
                x,
                y,
                tol
            );
        }
    }

    /// Build the outer product of `v_kernel` (length KH) and
    /// `h_kernel` (length KW) as a row-major flattened
    /// `[KH, KW]` buffer of length KH * KW.
    ///
    /// `outer[r * KW + c] = v_kernel[r] * h_kernel[c]`.
    fn outer_product(v_kernel: &[f32], h_kernel: &[f32]) -> Vec<f32> {
        let kh = v_kernel.len();
        let kw = h_kernel.len();
        let mut k = vec![0.0f32; kh * kw];
        for r in 0..kh {
            for c in 0..kw {
                k[r * kw + c] = v_kernel[r] * h_kernel[c];
            }
        }
        k
    }

    // ── error paths ────────────────────────────────────────────

    #[test]
    fn sep_conv2d_rejects_zero_dims() {
        let err =
            sep_conv2d(&[], &[1.0], &[1.0], 0, 8, BoundaryMode::Zero).unwrap_err();
        assert!(matches!(err, ConvError::ImageSizeMismatch(_)));
    }

    #[test]
    fn sep_conv2d_rejects_empty_horizontal_kernel() {
        let img = vec![0.0f32; 16];
        let err = sep_conv2d(&img, &[], &[1.0], 4, 4, BoundaryMode::Zero)
            .unwrap_err();
        assert!(matches!(err, ConvError::EmptyKernel));
    }

    #[test]
    fn sep_conv2d_rejects_empty_vertical_kernel() {
        let img = vec![0.0f32; 16];
        let err = sep_conv2d(&img, &[1.0], &[], 4, 4, BoundaryMode::Zero)
            .unwrap_err();
        assert!(matches!(err, ConvError::EmptyKernel));
    }

    #[test]
    fn sep_conv2d_rejects_image_size_mismatch() {
        let img = vec![0.0f32; 8];
        let err =
            sep_conv2d(&img, &[1.0], &[1.0], 4, 4, BoundaryMode::Zero).unwrap_err();
        assert!(matches!(err, ConvError::ImageSizeMismatch(_)));
    }

    // ── matches conv2d with outer-product kernel ───────────────

    #[test]
    fn sep_conv2d_matches_outer_product_for_3x3() {
        // Separable kernel: v = [1, 2, 1] / 4, h = [1, 2, 1] / 4.
        // The 2-D kernel is the outer product.
        let v = vec![0.25f32, 0.5, 0.25];
        let h = vec![0.25f32, 0.5, 0.25];
        let kernel_2d = outer_product(&v, &h);

        let height = 7;
        let width = 5;
        let image: Vec<f32> = (0..(height * width))
            .map(|i| ((i as f32) * 0.13).sin() + 0.4)
            .collect();

        for &mode in &[
            BoundaryMode::Zero,
            BoundaryMode::Replicate,
            BoundaryMode::Reflect,
            BoundaryMode::Wrap,
        ] {
            let via_sep = sep_conv2d(
                &image,
                &h,
                &v,
                height as u32,
                width as u32,
                mode,
            )
            .unwrap();
            let via_2d = conv2d(
                &image,
                &kernel_2d,
                height as u32,
                width as u32,
                3,
                3,
                mode,
            )
            .unwrap();
            assert_close(&via_sep, &via_2d, 1e-5);
        }
    }

    #[test]
    fn sep_conv2d_matches_outer_product_5x5_gaussian_under_replicate() {
        // 5×5 Gaussian-like kernel via outer product of
        // [1, 4, 6, 4, 1] / 16 with itself.  This is the
        // binomial approximation to a Gaussian and is
        // exactly separable.
        let g = vec![
            1.0f32 / 16.0,
            4.0 / 16.0,
            6.0 / 16.0,
            4.0 / 16.0,
            1.0 / 16.0,
        ];
        let kernel_2d = outer_product(&g, &g);

        let height = 9;
        let width = 11;
        let image: Vec<f32> = (0..(height * width))
            .map(|i| ((i as f32) * 0.07 + 0.3).sin())
            .collect();

        let via_sep = sep_conv2d(
            &image,
            &g,
            &g,
            height as u32,
            width as u32,
            BoundaryMode::Replicate,
        )
        .unwrap();
        let via_2d = conv2d(
            &image,
            &kernel_2d,
            height as u32,
            width as u32,
            5,
            5,
            BoundaryMode::Replicate,
        )
        .unwrap();
        assert_close(&via_sep, &via_2d, 1e-5);
    }

    #[test]
    fn sep_conv2d_identity_kernels_pass_through() {
        // [1.0] × [1.0] is the convolutional identity (1×1
        // kernel).  Image should pass through unchanged.
        let image: Vec<f32> = (0..30).map(|i| (i as f32) * 0.1).collect();
        let out = sep_conv2d(
            &image,
            &[1.0],
            &[1.0],
            5,
            6,
            BoundaryMode::Replicate,
        )
        .unwrap();
        assert_close(&out, &image, 1e-7);
    }

    #[test]
    fn sep_conv2d_constant_image_passes_through() {
        // Constant image + normalised separable kernel +
        // Replicate should give the same constant.
        let h = vec![1.0f32 / 3.0; 3];
        let v = vec![1.0f32 / 3.0; 3];
        let image = vec![3.5f32; 25];
        let out = sep_conv2d(&image, &h, &v, 5, 5, BoundaryMode::Replicate)
            .unwrap();
        for &val in &out {
            assert!(
                approx_eq(val, 3.5, 1e-5),
                "constant 2-D yielded {}",
                val
            );
        }
    }
}
