//! # 2-D convolution for image filtering (DSP04 Phase 3)
//!
//! Adds [`conv2d`] for same-size 2-D convolution on row-major
//! `[H, W]` real `f32` images.  Uses the same four boundary
//! modes as 1-D conv ([`crate::BoundaryMode`]).
//!
//! ## Algorithm
//!
//! Direct 2-D convolution.  For each output `(r, c)`:
//!
//! ```text
//!     out[r, c] = Σ_{kr, kc}  kernel[kr, kc]
//!                            · image_ext[r + ch - kr, c + cw - kc]
//! ```
//!
//! where `ch = KH / 2` and `cw = KW / 2` (integer division for
//! upper-centre on even kernel sizes), and `image_ext` extends
//! the image past `[0, H) × [0, W)` per the chosen boundary
//! mode along each axis independently.
//!
//! `O(H · W · KH · KW)` time, `O(H · W)` memory.  Phase 4 will
//! add `sep_conv2d` for the row-then-column fast path on
//! separable kernels (`O(H · W · (KH + KW))`).
//!
//! ## Use cases
//!
//! - Gaussian blur, box blur — separable kernels (faster via
//!   `sep_conv2d` in Phase 4, but `conv2d` works too).
//! - Sobel / Prewitt / Scharr edge detection — directional 3×3
//!   kernels.
//! - Laplacian, sharpen, emboss — short non-separable kernels
//!   where `conv2d` is the natural API.

use crate::{extend_index, BoundaryMode, ConvError};

/// Same-size 2-D convolution on a row-major `[H, W]` real
/// image.
///
/// - `image` must have `image.len() == H · W`, row-major
///   (`image[r * W + c]` is the pixel at row `r`, col `c`).
/// - `kernel` must have `kernel.len() == KH · KW`, row-major.
/// - Output is a same-size `[H, W]` row-major `Vec<f32>`.
/// - Kernel is centred at `(KH / 2, KW / 2)` — matches
///   `scipy.ndimage.convolve`.
pub fn conv2d(
    image: &[f32],
    kernel: &[f32],
    image_height: u32,
    image_width: u32,
    kernel_height: u32,
    kernel_width: u32,
    mode: BoundaryMode,
) -> Result<Vec<f32>, ConvError> {
    if image_height == 0 || image_width == 0 {
        return Err(ConvError::ImageSizeMismatch(format!(
            "image dimensions must be non-zero; got {}×{}",
            image_height, image_width
        )));
    }
    if kernel_height == 0 || kernel_width == 0 {
        return Err(ConvError::EmptyKernel);
    }
    let h = image_height as usize;
    let w = image_width as usize;
    let kh = kernel_height as usize;
    let kw = kernel_width as usize;

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
    let expected_kernel_len = kh.checked_mul(kw).ok_or_else(|| {
        ConvError::ImageSizeMismatch(format!(
            "kernel dimensions overflow: {}×{}",
            kh, kw
        ))
    })?;
    if kernel.len() != expected_kernel_len {
        return Err(ConvError::ImageSizeMismatch(format!(
            "kernel length {} does not match {}×{} = {}",
            kernel.len(),
            kh,
            kw,
            expected_kernel_len
        )));
    }
    // V1 simplification: require kernel to fit inside the image.
    // Reflect mode's boundary extension formula assumes
    // `kernel_height ≤ image_height` (and similarly for width)
    // so it never reflects past the image's other edge.
    if kh > h || kw > w {
        return Err(ConvError::KernelTooLarge(format!(
            "kernel {}×{} exceeds image {}×{}",
            kh, kw, h, w
        )));
    }

    let ch = (kh / 2) as isize; // kernel centre row
    let cw = (kw / 2) as isize; // kernel centre col
    let h_isize = h as isize;
    let w_isize = w as isize;

    // ── Main convolution loop.
    //
    // For each output (r, c) in [0, H) × [0, W), sum over
    // kernel taps (kr, kc) in [0, KH) × [0, KW).  Source row is
    // `r + ch - kr`, source col is `c + cw - kc`.  Boundary
    // extension is applied independently along each axis via
    // the shared `extend_index` helper from lib.rs.
    let mut out = vec![0.0f32; expected_image_len];
    for r in 0..h {
        for c in 0..w {
            let mut acc = 0.0f32;
            for kr in 0..kh {
                let src_r = (r as isize) + ch - (kr as isize);
                let row = match extend_index(src_r, h_isize, mode) {
                    Some(rr) => rr,
                    None => continue, // Zero mode, row out of bounds
                };
                for kc in 0..kw {
                    let src_c = (c as isize) + cw - (kc as isize);
                    let col = match extend_index(src_c, w_isize, mode) {
                        Some(cc) => cc,
                        None => continue,
                    };
                    let img_v = image[row * w + col];
                    let ker_v = kernel[kr * kw + kc];
                    acc += img_v * ker_v;
                }
            }
            out[r * w + c] = acc;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conv1d;

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

    // ── error paths ────────────────────────────────────────────

    #[test]
    fn conv2d_rejects_zero_height() {
        let err = conv2d(&[], &[1.0], 0, 8, 1, 1, BoundaryMode::Zero).unwrap_err();
        assert!(matches!(err, ConvError::ImageSizeMismatch(_)));
    }

    #[test]
    fn conv2d_rejects_zero_width() {
        let err = conv2d(&[], &[1.0], 8, 0, 1, 1, BoundaryMode::Zero).unwrap_err();
        assert!(matches!(err, ConvError::ImageSizeMismatch(_)));
    }

    #[test]
    fn conv2d_rejects_image_size_mismatch() {
        // 4×4 image expects 16 floats; we pass 8.
        let img = vec![0.0f32; 8];
        let err =
            conv2d(&img, &[1.0], 4, 4, 1, 1, BoundaryMode::Zero).unwrap_err();
        assert!(matches!(err, ConvError::ImageSizeMismatch(_)));
    }

    #[test]
    fn conv2d_rejects_kernel_size_mismatch() {
        // 3×3 kernel expects 9 floats; we pass 4.
        let img = vec![0.0f32; 16];
        let err =
            conv2d(&img, &[1.0; 4], 4, 4, 3, 3, BoundaryMode::Zero).unwrap_err();
        assert!(matches!(err, ConvError::ImageSizeMismatch(_)));
    }

    #[test]
    fn conv2d_rejects_kernel_too_large() {
        // 5×5 kernel on a 3×3 image.
        let img = vec![0.0f32; 9];
        let kernel = vec![0.0f32; 25];
        let err =
            conv2d(&img, &kernel, 3, 3, 5, 5, BoundaryMode::Zero).unwrap_err();
        assert!(matches!(err, ConvError::KernelTooLarge(_)));
    }

    #[test]
    fn conv2d_rejects_empty_kernel() {
        let img = vec![0.0f32; 9];
        let err = conv2d(&img, &[], 3, 3, 0, 0, BoundaryMode::Zero).unwrap_err();
        assert!(matches!(err, ConvError::EmptyKernel));
    }

    // ── closed-form ────────────────────────────────────────────

    #[test]
    fn conv2d_identity_kernel_returns_image() {
        // 1×1 kernel = [1.0] is the convolutional identity.
        let img: Vec<f32> = (0..16).map(|i| i as f32).collect();
        for &mode in &[
            BoundaryMode::Zero,
            BoundaryMode::Replicate,
            BoundaryMode::Reflect,
            BoundaryMode::Wrap,
        ] {
            let out = conv2d(&img, &[1.0], 4, 4, 1, 1, mode).unwrap();
            assert_close(&out, &img, 1e-7);
        }
    }

    #[test]
    fn conv2d_centred_delta_preserves_image() {
        // 3×3 kernel with 1 at the centre, zeros elsewhere is
        // also the convolutional identity in conv terms.
        let img: Vec<f32> = (0..16).map(|i| (i as f32) * 0.5).collect();
        let mut kernel = vec![0.0f32; 9];
        kernel[4] = 1.0; // centre of 3×3
        for &mode in &[
            BoundaryMode::Zero,
            BoundaryMode::Replicate,
            BoundaryMode::Reflect,
            BoundaryMode::Wrap,
        ] {
            let out = conv2d(&img, &kernel, 4, 4, 3, 3, mode).unwrap();
            assert_close(&out, &img, 1e-7);
        }
    }

    #[test]
    fn conv2d_box_kernel_on_constant_image_passes_through() {
        // 3×3 box kernel (normalised to sum=1) on a constant 5×5
        // image should yield the same constant — under Replicate
        // there are no edge artefacts; under all modes the
        // interior is fine.
        let img = vec![3.5f32; 25];
        let kernel = vec![1.0 / 9.0; 9];
        let out =
            conv2d(&img, &kernel, 5, 5, 3, 3, BoundaryMode::Replicate).unwrap();
        for &v in &out {
            assert!(
                approx_eq(v, 3.5, 1e-5),
                "constant 2-D yielded {}, expected 3.5",
                v
            );
        }
        // Also check the interior under all four modes.
        for &mode in &[
            BoundaryMode::Zero,
            BoundaryMode::Replicate,
            BoundaryMode::Reflect,
            BoundaryMode::Wrap,
        ] {
            let out2 = conv2d(&img, &kernel, 5, 5, 3, 3, mode).unwrap();
            // Interior pixels: (r, c) in [1..4) × [1..4) — every
            // mode is identical here since no boundary samples
            // enter the sum.
            for r in 1..4 {
                for c in 1..4 {
                    let v = out2[r * 5 + c];
                    assert!(
                        approx_eq(v, 3.5, 1e-5),
                        "interior pixel ({},{}) under {:?} = {}",
                        r,
                        c,
                        mode,
                        v
                    );
                }
            }
        }
    }

    // ── separability cross-check ───────────────────────────────

    #[test]
    fn conv2d_outer_product_matches_sequential_conv1d() {
        // A separable 3×3 kernel:
        //   [[1, 2, 1],
        //    [2, 4, 2],     = [1, 2, 1] ⊗ [1, 2, 1]  (normalised)
        //    [1, 2, 1]]
        //
        // Applying it via conv2d should match applying
        // conv1d horizontally to each row, then conv1d
        // vertically to each column (same boundary mode).
        let h = 5;
        let w = 7;
        let img: Vec<f32> = (0..(h * w))
            .map(|i| ((i as f32) * 0.13).sin() + 0.4)
            .collect();
        let h1d = vec![1.0f32 / 4.0, 2.0 / 4.0, 1.0 / 4.0];
        let kernel_2d: Vec<f32> = {
            let mut k = vec![0.0f32; 9];
            for r in 0..3 {
                for c in 0..3 {
                    k[r * 3 + c] = h1d[r] * h1d[c];
                }
            }
            k
        };
        let mode = BoundaryMode::Replicate;
        let via_2d =
            conv2d(&img, &kernel_2d, h as u32, w as u32, 3, 3, mode).unwrap();

        // Row pass: conv1d along each row.
        let mut intermediate = vec![0.0f32; h * w];
        for r in 0..h {
            let row = &img[r * w..(r + 1) * w];
            let row_out = conv1d(row, &h1d, mode).unwrap();
            intermediate[r * w..(r + 1) * w].copy_from_slice(&row_out);
        }
        // Column pass: gather column, conv1d, scatter back.
        let mut via_separable = vec![0.0f32; h * w];
        let mut col_buf = vec![0.0f32; h];
        for c in 0..w {
            for r in 0..h {
                col_buf[r] = intermediate[r * w + c];
            }
            let col_out = conv1d(&col_buf, &h1d, mode).unwrap();
            for r in 0..h {
                via_separable[r * w + c] = col_out[r];
            }
        }
        assert_close(&via_2d, &via_separable, 1e-5);
    }

    // ── boundary mode spot-check ───────────────────────────────

    #[test]
    fn conv2d_boundary_modes_differ_at_corner() {
        // 3×3 image of [1, 2, 3; 4, 5, 6; 7, 8, 9] with a 3×3
        // uniform kernel.  Corner output (0, 0) depends on
        // boundary extension; verify each mode gives a distinct
        // value.
        let img = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let kernel = vec![1.0f32; 9];

        // Zero: corner (0,0) sees image at (-1,-1)=(0), (-1,0)=(0),
        // (-1,1)=(0), (0,-1)=(0), (0,0)=1, (0,1)=2, (1,-1)=(0),
        // (1,0)=4, (1,1)=5.  Sum = 1+2+4+5 = 12.
        let out_zero =
            conv2d(&img, &kernel, 3, 3, 3, 3, BoundaryMode::Zero).unwrap();
        assert!(approx_eq(out_zero[0], 12.0, 1e-5), "Zero corner = {}", out_zero[0]);

        // Replicate: corner (0,0) sees (-1,-1)=image[0,0]=1,
        // (-1,0)=image[0,0]=1, (-1,1)=image[0,1]=2,
        // (0,-1)=image[0,0]=1, (0,0)=1, (0,1)=2,
        // (1,-1)=image[1,0]=4, (1,0)=4, (1,1)=5.
        // Sum = 1+1+2+1+1+2+4+4+5 = 21.
        let out_rep =
            conv2d(&img, &kernel, 3, 3, 3, 3, BoundaryMode::Replicate).unwrap();
        assert!(approx_eq(out_rep[0], 21.0, 1e-5), "Replicate corner = {}", out_rep[0]);

        // All four should give different corner values.  We've
        // verified two; the other two are well-defined by the
        // boundary formulas — just check they're all distinct.
        let out_ref =
            conv2d(&img, &kernel, 3, 3, 3, 3, BoundaryMode::Reflect).unwrap();
        let out_wrap =
            conv2d(&img, &kernel, 3, 3, 3, 3, BoundaryMode::Wrap).unwrap();
        let corners = [out_zero[0], out_rep[0], out_ref[0], out_wrap[0]];
        for i in 0..corners.len() {
            for j in (i + 1)..corners.len() {
                assert!(
                    !approx_eq(corners[i], corners[j], 1e-4),
                    "modes {} and {} both gave corner = {}",
                    i,
                    j,
                    corners[i]
                );
            }
        }
    }
}
