//! # 2-D DCT for image / JPEG-style workloads (DSP02 Phase 4)
//!
//! Adds [`dct_2d`] and [`idct_2d`] on top of the Phase 1+2
//! 1-D `dct` / `idct`.  Operates on row-major `[H, W]` real
//! `f32` buffers (so `image.len() == H * W`).
//!
//! ## Algorithm
//!
//! The 2-D DCT is **separable**: applying the 1-D DCT to each
//! row, then the 1-D DCT to each column of the result, is
//! identical to the true 2-D DCT.  No special algorithm needed
//! — we just call the 1-D `dct` from this crate twice:
//!
//! 1. **Row pass.**  For each row `r in 0..H`, run
//!    `dct(image[r * W .. (r + 1) * W], dct_type, norm)`.
//!    Result fills a `[H, W]` intermediate buffer.
//! 2. **Column pass.**  For each column `c in 0..W`, gather
//!    `[intermediate[r * W + c] for r in 0..H]`, run `dct`
//!    on it, scatter back into the output.
//!
//! Total work: `O(H · W · log(W))` for the row pass +
//! `O(W · H · log(H))` for the column pass — i.e.
//! `O(H · W · (log H + log W))` for power-of-two H, W.
//!
//! Inverse direction uses the same pattern with `idct`.
//!
//! ## JPEG block sizes
//!
//! JPEG works on 8×8 blocks.  This module handles arbitrary
//! `(H, W)` and is the building block a future JPEG encoder
//! / decoder would call once per block (or once per whole
//! image, then quantise the coefficients).  Phase 5 will add
//! a specialised 8-point DCT-II for the 8×8 hot path.

use crate::{dct, idct, DctError, DctNorm, DctType};

/// Forward 2-D DCT on a row-major `[H, W]` real image.
///
/// `image` must satisfy `image.len() == (height as usize) *
/// (width as usize)`.  Returns a `Vec<f32>` of the same length
/// holding the 2-D DCT coefficients in the same row-major
/// `[H, W]` layout.
///
/// `dct_type` is the variant to use along **both** axes (you
/// can't currently mix DCT-II rows with DCT-III columns); pass
/// [`DctType::II`] for the standard forward 2-D DCT.
pub fn dct_2d(
    image: &[f32],
    height: u32,
    width: u32,
    dct_type: DctType,
    norm: DctNorm,
) -> Result<Vec<f32>, DctError> {
    apply_2d(image, height, width, |row| dct(row, dct_type, norm))
}

/// Inverse 2-D DCT.  Same shape contract as [`dct_2d`].
///
/// Pass [`DctType::III`] (with the same `norm` you used on the
/// forward call) to invert a [`dct_2d`] forward.
pub fn idct_2d(
    image: &[f32],
    height: u32,
    width: u32,
    dct_type: DctType,
    norm: DctNorm,
) -> Result<Vec<f32>, DctError> {
    apply_2d(image, height, width, |row| idct(row, dct_type, norm))
}

/// Internal helper: run a 1-D operation along rows then along
/// columns of a row-major `[H, W]` image.  Used by both
/// [`dct_2d`] and [`idct_2d`].
///
/// The closure takes a single 1-D row (length `W` for the row
/// pass, length `H` for the column pass — uniform so a single
/// closure can drive both) and returns its transformed copy.
fn apply_2d<F>(
    image: &[f32],
    height: u32,
    width: u32,
    op: F,
) -> Result<Vec<f32>, DctError>
where
    F: Fn(&[f32]) -> Result<Vec<f32>, DctError>,
{
    if height == 0 || width == 0 {
        return Err(DctError::InvalidInput(format!(
            "2-D DCT requires non-zero dimensions; got {}×{}",
            height, width
        )));
    }
    let h = height as usize;
    let w = width as usize;
    // `h * w` can overflow on 32-bit usize for huge dims, but the
    // checked_mul guard catches it.  On 64-bit, u32 × u32 fits.
    let expected_len = match h.checked_mul(w) {
        Some(n) => n,
        None => {
            return Err(DctError::InvalidInput(format!(
                "image dimensions overflow: {}×{}",
                h, w
            )))
        }
    };
    if image.len() != expected_len {
        return Err(DctError::InvalidInput(format!(
            "image length {} does not match {}×{} = {}",
            image.len(),
            h,
            w,
            expected_len
        )));
    }
    if image.is_empty() {
        // h == 0 || w == 0 is already handled above; this branch
        // is unreachable in practice but guards against weird
        // future edits.
        return Err(DctError::EmptyInput);
    }

    // ── Step 1: row pass.  Fill `intermediate[r*w + c]` with
    //   the 1-D DCT of row r.
    let mut intermediate = vec![0.0f32; expected_len];
    for r in 0..h {
        let row = &image[r * w..(r + 1) * w];
        let row_dct = op(row)?;
        intermediate[r * w..(r + 1) * w].copy_from_slice(&row_dct);
    }

    // ── Step 2: column pass.  Gather column c into a temp Vec
    //   of length h, run the 1-D op, scatter back.
    let mut out = vec![0.0f32; expected_len];
    let mut col_buf = vec![0.0f32; h];
    for c in 0..w {
        for r in 0..h {
            col_buf[r] = intermediate[r * w + c];
        }
        let col_dct = op(&col_buf)?;
        for r in 0..h {
            out[r * w + c] = col_dct[r];
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

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

    /// Naive O(H²W²) 2-D DCT-II oracle — direct double-sum, no
    /// FFT, no separability assumption.  Used to verify the
    /// row-then-column factorisation.
    fn naive_dct_2d_ii(
        image: &[f32],
        height: usize,
        width: usize,
        norm: DctNorm,
    ) -> Vec<f32> {
        let h_f = height as f32;
        let w_f = width as f32;
        let mut out = vec![0.0f32; height * width];
        for kr in 0..height {
            for kc in 0..width {
                let mut acc = 0.0f32;
                for nr in 0..height {
                    for nc in 0..width {
                        let theta_r = PI
                            * (kr as f32)
                            * (2.0 * (nr as f32) + 1.0)
                            / (2.0 * h_f);
                        let theta_c = PI
                            * (kc as f32)
                            * (2.0 * (nc as f32) + 1.0)
                            / (2.0 * w_f);
                        acc += image[nr * width + nc]
                            * theta_r.cos()
                            * theta_c.cos();
                    }
                }
                // Un-normalised 2-D DCT-II is 4 * Σ Σ.
                out[kr * width + kc] = 4.0 * acc;
            }
        }
        // Apply the separable Ortho rescaling on top of the
        // un-normalised result.
        if norm == DctNorm::Ortho {
            let s_r = |kr: usize| {
                if kr == 0 {
                    (1.0 / (4.0 * h_f)).sqrt()
                } else {
                    (1.0 / (2.0 * h_f)).sqrt()
                }
            };
            let s_c = |kc: usize| {
                if kc == 0 {
                    (1.0 / (4.0 * w_f)).sqrt()
                } else {
                    (1.0 / (2.0 * w_f)).sqrt()
                }
            };
            for kr in 0..height {
                for kc in 0..width {
                    out[kr * width + kc] *= s_r(kr) * s_c(kc);
                }
            }
        }
        out
    }

    // ── error paths ────────────────────────────────────────────

    #[test]
    fn rejects_zero_height() {
        let err = dct_2d(&[], 0, 8, DctType::II, DctNorm::None).unwrap_err();
        assert!(matches!(err, DctError::InvalidInput(_)));
    }

    #[test]
    fn rejects_zero_width() {
        let err = dct_2d(&[], 8, 0, DctType::II, DctNorm::None).unwrap_err();
        assert!(matches!(err, DctError::InvalidInput(_)));
    }

    #[test]
    fn rejects_length_mismatch() {
        // 8x8 expects length 64; we pass length 32.
        let img = vec![0.0f32; 32];
        let err = dct_2d(&img, 8, 8, DctType::II, DctNorm::None).unwrap_err();
        assert!(matches!(err, DctError::InvalidInput(_)));
    }

    #[test]
    fn idct_2d_rejects_length_mismatch() {
        let img = vec![0.0f32; 32];
        let err = idct_2d(&img, 8, 8, DctType::III, DctNorm::Ortho).unwrap_err();
        assert!(matches!(err, DctError::InvalidInput(_)));
    }

    // ── closed-form known vectors ──────────────────────────────

    #[test]
    fn dct_2d_of_dc_block_concentrates_at_origin() {
        // Constant 8x8 block — un-normalised 2-D DCT-II gives
        // `(2N) * (2M) * value` at bin (0, 0) (where the DC
        // factor for each axis is 2N) and zero elsewhere.  For
        // 8x8 with value=1: (2*8) * (2*8) = 256.
        let img = vec![1.0f32; 64];
        let coeffs = dct_2d(&img, 8, 8, DctType::II, DctNorm::None).unwrap();
        assert_eq!(coeffs.len(), 64);
        assert!(
            approx_eq(coeffs[0], 256.0, 1e-3),
            "DC coeff = {}, expected 256",
            coeffs[0]
        );
        for (i, &c) in coeffs.iter().enumerate().skip(1) {
            assert!(
                approx_eq(c, 0.0, 1e-3),
                "coeff[{}] = {}, expected ~0",
                i,
                c
            );
        }
    }

    #[test]
    fn dct_2d_of_impulse_block_matches_naive() {
        // An 8x8 impulse at (0, 0) is the canonical "spectral
        // basis function" check — the result is the outer product
        // of two cosine sequences.  We validate against the naive
        // O(H²W²) oracle.
        let mut img = vec![0.0f32; 64];
        img[0] = 1.0;
        let via_separable =
            dct_2d(&img, 8, 8, DctType::II, DctNorm::None).unwrap();
        let via_naive = naive_dct_2d_ii(&img, 8, 8, DctNorm::None);
        assert_close(&via_separable, &via_naive, 1e-3);
    }

    // ── naive cross-check ──────────────────────────────────────

    #[test]
    fn dct_2d_matches_naive_4x4() {
        let img: Vec<f32> = (0..16)
            .map(|i| ((i as f32) * 0.3 - 0.5).sin())
            .collect();
        let via_separable =
            dct_2d(&img, 4, 4, DctType::II, DctNorm::None).unwrap();
        let via_naive = naive_dct_2d_ii(&img, 4, 4, DctNorm::None);
        assert_close(&via_separable, &via_naive, 1e-4);
    }

    #[test]
    fn dct_2d_matches_naive_8x8_ortho() {
        let img: Vec<f32> = (0..64)
            .map(|i| ((i as f32) * 0.07).cos())
            .collect();
        let via_separable =
            dct_2d(&img, 8, 8, DctType::II, DctNorm::Ortho).unwrap();
        let via_naive = naive_dct_2d_ii(&img, 8, 8, DctNorm::Ortho);
        assert_close(&via_separable, &via_naive, 1e-4);
    }

    // ── round-trips under Ortho ────────────────────────────────

    fn round_trip_2d_ortho(h: u32, w: u32, tol: f32) {
        let n = (h as usize) * (w as usize);
        let img: Vec<f32> = (0..n)
            .map(|i| ((i as f32) * 0.1).sin() + 0.3)
            .collect();
        let coeffs = dct_2d(&img, h, w, DctType::II, DctNorm::Ortho).unwrap();
        let recovered =
            idct_2d(&coeffs, h, w, DctType::III, DctNorm::Ortho).unwrap();
        assert_close(&img, &recovered, tol);
    }

    #[test]
    fn round_trip_2d_ortho_8x8() {
        // The JPEG block size.
        round_trip_2d_ortho(8, 8, 1e-3);
    }

    #[test]
    fn round_trip_2d_ortho_16x16() {
        round_trip_2d_ortho(16, 16, 1e-3);
    }

    #[test]
    fn round_trip_2d_ortho_8x16_non_square() {
        round_trip_2d_ortho(8, 16, 1e-3);
    }

    #[test]
    fn round_trip_2d_ortho_3x5_non_pow2() {
        // Both dimensions non-power-of-two — exercises Bluestein
        // along both axes.
        round_trip_2d_ortho(3, 5, 1e-3);
    }
}
