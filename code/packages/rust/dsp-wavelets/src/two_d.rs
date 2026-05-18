//! # 2-D Discrete Wavelet Transform (DSP06 Phase 4)
//!
//! Separable row-then-column 2-D DWT built on top of the
//! 1-D filter bank from Phases 1-3.  The canonical algorithm
//! for image wavelet decomposition (used by JPEG 2000, DICOM
//! medical imaging, and most image wavelet code).
//!
//! ## Algorithm
//!
//! One level of 2-D DWT factors as:
//!
//! ```text
//!    image ──row 1-D DWT──► [L_row | H_row]
//!                                    │
//!                              column 1-D DWT
//!                                    │
//!                                    ▼
//!                            ┌─────────────────┐
//!                            │  LL  │   HL    │
//!                            │      │         │
//!                            │ (cA) │  (cH)   │
//!                            ├──────┼─────────┤
//!                            │  LH  │   HH    │
//!                            │      │         │
//!                            │ (cV) │  (cD)   │
//!                            └─────────────────┘
//! ```
//!
//! The four sub-bands at level `j`:
//!
//! - **LL** — both row and column lowpass.  Smooth, downscaled
//!   approximation.  Recursed for level `j + 1`.
//! - **HL** — row lowpass, column highpass.  Captures vertical
//!   detail (= horizontal edges).
//! - **LH** — row highpass, column lowpass.  Captures horizontal
//!   detail (= vertical edges).
//! - **HH** — both highpass.  Captures diagonal detail.
//!
//! After `J` levels, the output is:
//!
//! ```text
//!   [LL_J | HL_J | LH_J | HH_J | HL_{J-1} | LH_{J-1} | HH_{J-1} | ... | HL_1 | LH_1 | HH_1]
//! ```
//!
//! (flat row-major, matching `pywt.wavedec2` flattened convention).
//!
//! ## V1 (Phase 4a) scope
//!
//! - Orthogonal wavelets only (Haar, Db2/4/6/8, Sym4, Coif1) —
//!   the verified set from Phases 1-3.
//! - Periodic boundary only (where 1-D round-trip is mathematically
//!   exact for orthogonal wavelets).
//! - Square or rectangular images, but each dimension must
//!   support `J` levels of halving.
//!
//! Phase 4b will add:
//! - Biorthogonal wavelets (Bior 5/3 and Bior 9/7 — the JPEG 2000
//!   pair) which require refactoring `synthesize_one_level` to
//!   accept separate analysis + synthesis filter pairs.
//! - Symmetric boundary for non-Haar wavelets.

use crate::{dwt_1d, idwt_1d, WaveletBoundary, WaveletError, WaveletType, MAX_LEVELS, MAX_SAMPLES};

/// Forward 2-D DWT via separable row-then-column 1-D DWT.
///
/// `image` is a row-major flattened `[n_rows, n_cols]` `f32`
/// matrix.  Output is a row-major flattened concatenation of
/// per-level sub-bands as documented at the module level.
///
/// Phase 4a supports orthogonal wavelets only (Haar, Db2/4/6/8,
/// Sym4, Coif1) under Periodic boundary.  Other combinations
/// return `WaveletError::InvalidParam`.
pub fn dwt_2d(
    image: &[f32],
    n_rows: u32,
    n_cols: u32,
    wavelet: WaveletType,
    levels: u32,
    boundary: WaveletBoundary,
) -> Result<Vec<f32>, WaveletError> {
    validate_2d_inputs(image, n_rows, n_cols, levels, boundary)?;
    check_2d_supported(wavelet, boundary)?;

    // Walk the pyramid.  At each level, start with `current_image`
    // of size `(cur_rows, cur_cols)`, produce four sub-bands of
    // size `(cur_rows/2, cur_cols/2)` each.  LL becomes the
    // input for the next level; the other three are pushed onto
    // the output buffer (kept in coarsest-to-finest order via a
    // reversed-at-the-end Vec<Vec<f32>>).
    let mut current = image.to_vec();
    let mut cur_rows = n_rows as usize;
    let mut cur_cols = n_cols as usize;
    // detail_levels_reversed[0] = level 1 details, etc.; we reverse
    // at the end so the output starts with the coarsest.
    let mut detail_levels_reversed: Vec<(Vec<f32>, Vec<f32>, Vec<f32>)> =
        Vec::with_capacity(levels as usize);

    for _ in 0..levels {
        let (ll, hl, lh, hh) = dwt_2d_one_level(
            &current, cur_rows, cur_cols, wavelet, boundary,
        )?;
        detail_levels_reversed.push((hl, lh, hh));
        cur_rows = cur_rows.div_ceil(2);
        cur_cols = cur_cols.div_ceil(2);
        current = ll;
    }

    // Assemble output: LL_J, then for each j = J..1, (HL_j, LH_j, HH_j).
    let mut total_len = current.len();
    for (hl, lh, hh) in &detail_levels_reversed {
        total_len += hl.len() + lh.len() + hh.len();
    }
    let mut out = Vec::with_capacity(total_len);
    out.extend_from_slice(&current); // LL_J (coarsest)
    for (hl, lh, hh) in detail_levels_reversed.iter().rev() {
        out.extend_from_slice(hl);
        out.extend_from_slice(lh);
        out.extend_from_slice(hh);
    }
    debug_assert_eq!(out.len(), total_len);
    Ok(out)
}

/// Inverse 2-D DWT — reverses [`dwt_2d`].
pub fn idwt_2d(
    coeffs: &[f32],
    n_rows: u32,
    n_cols: u32,
    wavelet: WaveletType,
    levels: u32,
    boundary: WaveletBoundary,
) -> Result<Vec<f32>, WaveletError> {
    if coeffs.is_empty() {
        return Err(WaveletError::EmptySignal);
    }
    if n_rows == 0 || n_cols == 0 {
        return Err(WaveletError::InvalidParam(
            "n_rows and n_cols must be > 0".into(),
        ));
    }
    if (n_rows as usize) > MAX_SAMPLES as usize
        || (n_cols as usize) > MAX_SAMPLES as usize
    {
        return Err(WaveletError::InvalidParam(format!(
            "image dimension {}×{} exceeds MAX_SAMPLES {}",
            n_rows, n_cols, MAX_SAMPLES
        )));
    }
    if levels == 0 {
        return Err(WaveletError::InvalidParam("levels must be > 0".into()));
    }
    if levels > MAX_LEVELS {
        return Err(WaveletError::InvalidParam(format!(
            "levels {} exceeds MAX_LEVELS {}",
            levels, MAX_LEVELS
        )));
    }
    check_2d_supported(wavelet, boundary)?;

    // Compute per-level dimensions: (rows_0, cols_0), (rows_1,
    // cols_1), ..., (rows_J, cols_J) where rows_0 = n_rows etc.
    let level_dims = forward_level_dims(n_rows as usize, n_cols as usize, levels);

    // Coarsest LL block size.
    let (ll_rows, ll_cols) = level_dims[levels as usize];

    // Verify total coeff length matches.
    let mut expected_total = ll_rows * ll_cols;
    for j in 1..=(levels as usize) {
        let (rj, cj) = level_dims[j];
        // 3 detail sub-bands at level j, each (rj, cj).
        expected_total += 3 * rj * cj;
    }
    if coeffs.len() != expected_total {
        return Err(WaveletError::InvalidCoefficients(format!(
            "coeffs length {} != expected {} for n_rows={}, n_cols={}, levels={}",
            coeffs.len(),
            expected_total,
            n_rows,
            n_cols,
            levels
        )));
    }

    // Slice out LL_J + per-level details.
    let mut offset = 0;
    let mut ll = coeffs[offset..offset + ll_rows * ll_cols].to_vec();
    offset += ll_rows * ll_cols;
    let mut current_rows = ll_rows;
    let mut current_cols = ll_cols;

    for j in (1..=levels as usize).rev() {
        let (rj, cj) = level_dims[j];
        let band_len = rj * cj;
        let hl = &coeffs[offset..offset + band_len];
        offset += band_len;
        let lh = &coeffs[offset..offset + band_len];
        offset += band_len;
        let hh = &coeffs[offset..offset + band_len];
        offset += band_len;
        // Target dimensions = level_dims[j - 1].
        let (target_rows, target_cols) = level_dims[j - 1];
        ll = idwt_2d_one_level(
            &ll, hl, lh, hh, rj, cj, target_rows, target_cols,
            wavelet, boundary,
        )?;
        current_rows = target_rows;
        current_cols = target_cols;
    }
    debug_assert_eq!(offset, coeffs.len());
    debug_assert_eq!(current_rows, n_rows as usize);
    debug_assert_eq!(current_cols, n_cols as usize);
    Ok(ll)
}

// ───────────────── Internal helpers ─────────────────

fn validate_2d_inputs(
    image: &[f32],
    n_rows: u32,
    n_cols: u32,
    levels: u32,
    boundary: WaveletBoundary,
) -> Result<(), WaveletError> {
    if image.is_empty() {
        return Err(WaveletError::EmptySignal);
    }
    if n_rows == 0 || n_cols == 0 {
        return Err(WaveletError::InvalidParam(
            "n_rows and n_cols must be > 0".into(),
        ));
    }
    if (n_rows as usize) > MAX_SAMPLES as usize
        || (n_cols as usize) > MAX_SAMPLES as usize
    {
        return Err(WaveletError::InvalidParam(format!(
            "image dimension {}×{} exceeds MAX_SAMPLES {}",
            n_rows, n_cols, MAX_SAMPLES
        )));
    }
    if image.len() != (n_rows as usize) * (n_cols as usize) {
        return Err(WaveletError::InvalidParam(format!(
            "image length {} does not match n_rows × n_cols = {} × {} = {}",
            image.len(),
            n_rows,
            n_cols,
            (n_rows as usize) * (n_cols as usize)
        )));
    }
    if levels == 0 {
        return Err(WaveletError::InvalidParam("levels must be > 0".into()));
    }
    if levels > MAX_LEVELS {
        return Err(WaveletError::InvalidParam(format!(
            "levels {} exceeds MAX_LEVELS {}",
            levels, MAX_LEVELS
        )));
    }
    Ok(())
}

fn check_2d_supported(
    wavelet: WaveletType,
    boundary: WaveletBoundary,
) -> Result<(), WaveletError> {
    if boundary != WaveletBoundary::Periodic {
        return Err(WaveletError::InvalidParam(format!(
            "Phase 4a only supports Periodic boundary for 2-D DWT (got {:?})",
            boundary
        )));
    }
    // Defer wavelet validation to the 1-D paths — they'll reject
    // anything outside the Phase 3a/3b verified set.
    match wavelet {
        WaveletType::Haar
        | WaveletType::Daubechies(_)
        | WaveletType::Symlets(_)
        | WaveletType::Coiflets(_) => Ok(()),
        _ => Err(WaveletError::InvalidParam(format!(
            "Phase 4a only supports orthogonal wavelets for 2-D DWT (got {:?}); \
             Biorthogonal coming in Phase 4b",
            wavelet
        ))),
    }
}

/// Compute per-level (rows, cols) dimensions for the recursive
/// 2-D halving.  `level_dims[0] = (n_rows, n_cols)`,
/// `level_dims[1] = (⌈n_rows/2⌉, ⌈n_cols/2⌉)`, etc.
fn forward_level_dims(
    n_rows: usize,
    n_cols: usize,
    levels: u32,
) -> Vec<(usize, usize)> {
    let mut dims = Vec::with_capacity((levels as usize) + 1);
    dims.push((n_rows, n_cols));
    let mut r = n_rows;
    let mut c = n_cols;
    for _ in 0..levels {
        r = r.div_ceil(2);
        c = c.div_ceil(2);
        dims.push((r, c));
    }
    dims
}

/// One level of 2-D DWT: row-DWT each row, then column-DWT each
/// of the resulting two columns of (L, H) blocks.  Returns
/// (LL, HL, LH, HH), each of size (⌈rows/2⌉, ⌈cols/2⌉).
fn dwt_2d_one_level(
    image: &[f32],
    n_rows: usize,
    n_cols: usize,
    wavelet: WaveletType,
    boundary: WaveletBoundary,
) -> Result<(Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>), WaveletError> {
    debug_assert_eq!(image.len(), n_rows * n_cols);
    let half_cols = n_cols.div_ceil(2);
    let half_rows = n_rows.div_ceil(2);

    // ── Row pass.  For each row, compute 1-level 1-D DWT, producing
    //    [cA | cD] of total length n_cols (sample-count-preserving
    //    for Periodic boundary).  Stack L and H into two separate
    //    row-major buffers, each (n_rows, half_cols).
    let mut l_rows: Vec<f32> = Vec::with_capacity(n_rows * half_cols);
    let mut h_rows: Vec<f32> = Vec::with_capacity(n_rows * half_cols);
    for r in 0..n_rows {
        let row = &image[r * n_cols..(r + 1) * n_cols];
        let row_coeffs = dwt_1d(row, wavelet, 1, boundary)?;
        // row_coeffs layout: [cA (len half_cols) | cD (len half_cols)]
        l_rows.extend_from_slice(&row_coeffs[..half_cols]);
        h_rows.extend_from_slice(&row_coeffs[half_cols..2 * half_cols]);
    }

    // ── Column pass.  For each of L and H (each of size
    //    n_rows × half_cols), apply 1-D DWT down each column.
    //    Result per column: [cA (len half_rows) | cD (len half_rows)].
    //    Combine: cA of L-cols → LL, cD of L-cols → LH,
    //             cA of H-cols → HL, cD of H-cols → HH.
    let mut ll = vec![0.0_f32; half_rows * half_cols];
    let mut hl = vec![0.0_f32; half_rows * half_cols];
    let mut lh = vec![0.0_f32; half_rows * half_cols];
    let mut hh = vec![0.0_f32; half_rows * half_cols];

    let mut col_buf = vec![0.0_f32; n_rows];

    for c in 0..half_cols {
        // L sub-image, column c.
        for r in 0..n_rows {
            col_buf[r] = l_rows[r * half_cols + c];
        }
        let col_coeffs = dwt_1d(&col_buf, wavelet, 1, boundary)?;
        // col_coeffs: [cA (half_rows) | cD (half_rows)]
        for r in 0..half_rows {
            ll[r * half_cols + c] = col_coeffs[r];
            lh[r * half_cols + c] = col_coeffs[half_rows + r];
        }

        // H sub-image, column c.
        for r in 0..n_rows {
            col_buf[r] = h_rows[r * half_cols + c];
        }
        let col_coeffs = dwt_1d(&col_buf, wavelet, 1, boundary)?;
        for r in 0..half_rows {
            hl[r * half_cols + c] = col_coeffs[r];
            hh[r * half_cols + c] = col_coeffs[half_rows + r];
        }
    }

    Ok((ll, hl, lh, hh))
}

/// Inverse of [`dwt_2d_one_level`].
fn idwt_2d_one_level(
    ll: &[f32],
    hl: &[f32],
    lh: &[f32],
    hh: &[f32],
    band_rows: usize,
    band_cols: usize,
    target_rows: usize,
    target_cols: usize,
    wavelet: WaveletType,
    boundary: WaveletBoundary,
) -> Result<Vec<f32>, WaveletError> {
    debug_assert_eq!(ll.len(), band_rows * band_cols);

    // ── Inverse column pass.  Recover L and H rows (each
    //    target_rows × band_cols) from their cA/cD pairs.
    let mut l_rows = vec![0.0_f32; target_rows * band_cols];
    let mut h_rows = vec![0.0_f32; target_rows * band_cols];
    let mut col_coeffs = vec![0.0_f32; 2 * band_rows];

    for c in 0..band_cols {
        // L column: cA from LL[..][c], cD from LH[..][c].
        for r in 0..band_rows {
            col_coeffs[r] = ll[r * band_cols + c];
            col_coeffs[band_rows + r] = lh[r * band_cols + c];
        }
        let col = idwt_1d(
            &col_coeffs,
            wavelet,
            1,
            boundary,
            target_rows as u32,
        )?;
        for r in 0..target_rows {
            l_rows[r * band_cols + c] = col[r];
        }

        // H column: cA from HL[..][c], cD from HH[..][c].
        for r in 0..band_rows {
            col_coeffs[r] = hl[r * band_cols + c];
            col_coeffs[band_rows + r] = hh[r * band_cols + c];
        }
        let col = idwt_1d(
            &col_coeffs,
            wavelet,
            1,
            boundary,
            target_rows as u32,
        )?;
        for r in 0..target_rows {
            h_rows[r * band_cols + c] = col[r];
        }
    }

    // ── Inverse row pass.  For each row, recombine L and H halves
    //    via 1-level idwt_1d.
    let mut out = vec![0.0_f32; target_rows * target_cols];
    let mut row_coeffs = vec![0.0_f32; 2 * band_cols];
    for r in 0..target_rows {
        // cA = L row r, cD = H row r.
        for c in 0..band_cols {
            row_coeffs[c] = l_rows[r * band_cols + c];
            row_coeffs[band_cols + c] = h_rows[r * band_cols + c];
        }
        let row = idwt_1d(
            &row_coeffs,
            wavelet,
            1,
            boundary,
            target_cols as u32,
        )?;
        out[r * target_cols..(r + 1) * target_cols].copy_from_slice(&row);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        let scale = a.abs().max(b.abs()).max(1.0);
        (a - b).abs() <= scale * tol
    }

    #[test]
    fn dwt_2d_output_shape_matches_image_size() {
        // For Periodic boundary with power-of-2 dimensions, the
        // total coefficient count equals n_rows × n_cols.
        for &(rows, cols, j) in &[
            (8u32, 8u32, 1u32),
            (8, 8, 2),
            (16, 16, 3),
            (32, 16, 2),
        ] {
            let image = vec![0.5_f32; (rows * cols) as usize];
            let coeffs = dwt_2d(
                &image,
                rows,
                cols,
                WaveletType::Haar,
                j,
                WaveletBoundary::Periodic,
            )
            .unwrap();
            assert_eq!(
                coeffs.len(),
                (rows as usize) * (cols as usize),
                "rows={}, cols={}, j={}",
                rows,
                cols,
                j
            );
        }
    }

    fn round_trip_2d_check(
        image: &[f32],
        n_rows: u32,
        n_cols: u32,
        wavelet: WaveletType,
        levels: u32,
        tol: f32,
    ) {
        let coeffs = dwt_2d(
            image,
            n_rows,
            n_cols,
            wavelet,
            levels,
            WaveletBoundary::Periodic,
        )
        .unwrap();
        let recon = idwt_2d(
            &coeffs,
            n_rows,
            n_cols,
            wavelet,
            levels,
            WaveletBoundary::Periodic,
        )
        .unwrap();
        assert_eq!(recon.len(), image.len());
        for (i, (&a, &b)) in image.iter().zip(recon.iter()).enumerate() {
            assert!(
                approx_eq(a, b, tol),
                "{:?} 2-D round-trip: idx {}, image={}, recon={}",
                wavelet,
                i,
                a,
                b
            );
        }
    }

    #[test]
    fn dwt_2d_idwt_2d_round_trip_haar() {
        let image: Vec<f32> = (0..(16 * 16))
            .map(|i| ((i as f32) * 0.1).sin())
            .collect();
        round_trip_2d_check(&image, 16, 16, WaveletType::Haar, 2, 1e-4);
    }

    #[test]
    fn dwt_2d_idwt_2d_round_trip_db4() {
        let image: Vec<f32> = (0..(16 * 16))
            .map(|i| ((i as f32) * 0.07).cos())
            .collect();
        round_trip_2d_check(&image, 16, 16, WaveletType::Daubechies(4), 2, 1e-3);
    }

    #[test]
    fn dwt_2d_idwt_2d_round_trip_rectangular() {
        let image: Vec<f32> = (0..(32 * 16))
            .map(|i| ((i as f32) * 0.05).sin())
            .collect();
        round_trip_2d_check(&image, 32, 16, WaveletType::Haar, 2, 1e-4);
    }

    #[test]
    fn dwt_2d_of_constant_image_has_zero_detail() {
        // Constant image under Haar: all detail (HL, LH, HH) bands
        // are exactly 0.  LL contains the scaled DC component.
        let image = vec![2.5_f32; 16 * 16];
        let coeffs = dwt_2d(
            &image,
            16,
            16,
            WaveletType::Haar,
            2,
            WaveletBoundary::Periodic,
        )
        .unwrap();
        // Level 2: LL_2 size = 4*4 = 16.  Everything else (detail) = 240.
        let ll_len = 4 * 4;
        for (i, &v) in coeffs[ll_len..].iter().enumerate() {
            assert!(
                v.abs() <= 1e-6,
                "detail idx {} (raw {}) = {}, expected ~0",
                i,
                ll_len + i,
                v
            );
        }
    }

    #[test]
    fn dwt_2d_rejects_empty_image() {
        let err = dwt_2d(
            &[],
            0,
            0,
            WaveletType::Haar,
            1,
            WaveletBoundary::Periodic,
        )
        .unwrap_err();
        assert_eq!(err, WaveletError::EmptySignal);
    }

    #[test]
    fn dwt_2d_rejects_dimension_mismatch() {
        let image = vec![0.0_f32; 100];
        let err = dwt_2d(
            &image,
            8,
            8,
            WaveletType::Haar,
            1,
            WaveletBoundary::Periodic,
        )
        .unwrap_err();
        assert!(matches!(err, WaveletError::InvalidParam(_)));
    }

    #[test]
    fn dwt_2d_rejects_unsupported_wavelet_for_2d() {
        // Phase 4a explicitly defers Biorthogonal to Phase 4b.
        let image = vec![0.0_f32; 16 * 16];
        let err = dwt_2d(
            &image,
            16,
            16,
            WaveletType::Biorthogonal { vm_decomp: 5, vm_recon: 3 },
            1,
            WaveletBoundary::Periodic,
        )
        .unwrap_err();
        assert!(matches!(err, WaveletError::InvalidParam(_)));
    }

    #[test]
    fn dwt_2d_rejects_non_periodic_boundary() {
        // Phase 4a Periodic-only for 2-D.
        let image = vec![0.0_f32; 16 * 16];
        let err = dwt_2d(
            &image,
            16,
            16,
            WaveletType::Haar,
            1,
            WaveletBoundary::Symmetric,
        )
        .unwrap_err();
        assert!(matches!(err, WaveletError::InvalidParam(_)));
    }
}
