// # matrix.rs — 3×3 Matrix Operations for Colour Science
//
// Colour pipelines in camera RAW development use 3×3 matrices to transform
// between different RGB colour spaces. Two primitives suffice for all RAW
// codec needs:
//
// - `mat3x3_mul`: apply a 3×3 matrix to a column vector (per pixel, hot path)
// - `invert_3x3`: invert a 3×3 matrix (per image, cold path)
//
// ## Why not the `matrix` crate?
//
// The repo's `matrix` crate stores data as `Vec<Vec<f64>>` — one heap
// allocation per matrix and per column vector. For per-pixel use (millions
// of calls per image), heap allocation per call is unacceptable. For the
// one-time-per-file inversion, it would be fine, but keeping zero external
// dependencies avoids coupling the RAW codec layer to general-purpose matrix
// algebra infrastructure.
//
// Instead we use:
// - Plain Rust arrays `[[f64;3];3]` and `[f64;3]` — stack-allocated, zero
//   indirection, trivially copy-able.
// - Cramer's rule for inversion — exactly 9 cofactors × 1 determinant, all
//   scalar ops, no loops over variable sizes.
//
// ## Row-major convention
//
// `m[row][col]`. Matrix-vector product: `out[i] = Σ_j m[i][j] * v[j]`.
//
// ## Cramer's rule for 3×3 inversion
//
// For M = [[a,b,c],[d,e,f],[g,h,k]], the inverse is:
//
//   det(M) = a(ek - fh) - b(dk - fg) + c(dh - eg)
//
//   inv(M) = (1/det) × C^T
//
// where C is the cofactor matrix:
//
//   C[0][0] =  (ek - fh),  C[0][1] = -(dk - fg),  C[0][2] =  (dh - eg)
//   C[1][0] = -(bk - ch),  C[1][1] =  (ak - cg),  C[1][2] = -(ah - bg)
//   C[2][0] =  (bf - ce),  C[2][1] = -(af - cd),  C[2][2] =  (ae - bd)
//
// The transpose C^T swaps row/col indices, so inv[i][j] = C[j][i] / det.

/// Multiply a 3×3 row-major matrix `m` by column vector `v`.
///
/// Output: `out[i] = m[i][0]*v[0] + m[i][1]*v[1] + m[i][2]*v[2]`
///
/// Called once per pixel in `apply_color_pipeline` — no heap allocation.
///
/// # Example
///
/// ```
/// use image_raw_pipeline::mat3x3_mul;
///
/// // Identity matrix leaves the vector unchanged.
/// let id = [[1.0,0.0,0.0],[0.0,1.0,0.0],[0.0,0.0,1.0]];
/// let v  = [3.0, 5.0, 7.0];
/// let out = mat3x3_mul(&id, v);
/// assert_eq!(out, v);
/// ```
#[inline]
pub fn mat3x3_mul(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Analytically invert a 3×3 matrix using Cramer's rule.
///
/// Returns `None` if `|det(M)| < 1e-12` (singular or near-singular).
///
/// For a non-singular M: `M × inv(M) = I` (identity), up to floating-point
/// rounding error on the order of machine epsilon (~1e-15 for f64).
///
/// # Example
///
/// ```
/// use image_raw_pipeline::{mat3x3_mul, invert_3x3};
///
/// let m = [[2.0,0.0,0.0],[0.0,3.0,0.0],[0.0,0.0,4.0]];
/// let inv = invert_3x3(&m).unwrap();
///
/// // Verify M × inv(M) ≈ identity.
/// let e0 = mat3x3_mul(&m, mat3x3_mul(&inv, [1.0, 0.0, 0.0]));
/// assert!((e0[0] - 1.0).abs() < 1e-10);
/// assert!(e0[1].abs() < 1e-10);
/// assert!(e0[2].abs() < 1e-10);
/// ```
pub fn invert_3x3(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    // Unpack for readability (mirrors the Cramer's rule derivation above).
    let [a, b, c] = m[0];
    let [d, e, f] = m[1];
    let [g, h, k] = m[2];

    // 2×2 minors needed for cofactors (computed once, reused twice each).
    let ek_fh = e * k - f * h; // minor for a
    let dk_fg = d * k - f * g; // minor for b
    let dh_eg = d * h - e * g; // minor for c
    let bk_ch = b * k - c * h; // minor for d
    let ak_cg = a * k - c * g; // minor for e
    let ah_bg = a * h - b * g; // minor for f
    let bf_ce = b * f - c * e; // minor for g
    let af_cd = a * f - c * d; // minor for h
    let ae_bd = a * e - b * d; // minor for k

    // Determinant via expansion along the first row.
    let det = a * ek_fh - b * dk_fg + c * dh_eg;
    if det.abs() < 1e-12 {
        return None; // singular or near-singular
    }

    let inv_det = 1.0 / det;

    // Inverse = (1/det) × C^T  (cofactor matrix, transposed).
    //
    // Layout: inv[row][col] = C[col][row] / det
    //         (the transpose swaps the two indices)
    Some([
        [ ek_fh * inv_det, -bk_ch * inv_det,  bf_ce * inv_det],
        [-dk_fg * inv_det,  ak_cg * inv_det, -af_cd * inv_det],
        [ dh_eg * inv_det, -ah_bg * inv_det,  ae_bd * inv_det],
    ])
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // Fixed 3×3 loops index two dimensions and reuse the indices in assert
    // messages; explicit `r`/`c`/`i`/`j` ranges read clearer than iterators.
    #![allow(clippy::needless_range_loop)]
    use super::*;

    // ── mat3x3_mul ────────────────────────────────────────────────────────

    #[test]
    fn mul_identity_leaves_vector_unchanged() {
        let id = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let v = [3.0_f64, 5.0, 7.0];
        let out = mat3x3_mul(&id, v);
        assert_eq!(out, v);
    }

    #[test]
    fn mul_zero_matrix_gives_zero() {
        let z = [[0.0_f64; 3]; 3];
        let v = [1.0, 2.0, 3.0];
        assert_eq!(mat3x3_mul(&z, v), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn mul_channel_swap_rb() {
        // Swap R and B: [[0,0,1],[0,1,0],[1,0,0]]
        let swap = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
        let v = [1.0_f64, 2.0, 3.0]; // R=1, G=2, B=3
        let out = mat3x3_mul(&swap, v);
        assert_eq!(out, [3.0, 2.0, 1.0]); // R and B swapped
    }

    #[test]
    fn mul_known_example() {
        // [[1,2,3],[4,5,6],[7,8,9]] × [1,0,0] = [1,4,7]
        let m = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
        let out = mat3x3_mul(&m, [1.0, 0.0, 0.0]);
        assert!((out[0] - 1.0).abs() < 1e-12);
        assert!((out[1] - 4.0).abs() < 1e-12);
        assert!((out[2] - 7.0).abs() < 1e-12);
    }

    #[test]
    fn mul_scaling_matrix() {
        let scale = [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 4.0]];
        let v = [1.0_f64, 1.0, 1.0];
        let out = mat3x3_mul(&scale, v);
        assert!((out[0] - 2.0).abs() < 1e-12);
        assert!((out[1] - 3.0).abs() < 1e-12);
        assert!((out[2] - 4.0).abs() < 1e-12);
    }

    // ── invert_3x3 ────────────────────────────────────────────────────────

    #[test]
    fn invert_identity_gives_identity() {
        let id = [[1.0_f64, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let inv = invert_3x3(&id).unwrap();
        for r in 0..3 {
            for c in 0..3 {
                let expected = if r == c { 1.0 } else { 0.0 };
                assert!((inv[r][c] - expected).abs() < 1e-12,
                    "inv[{}][{}] = {}, expected {}", r, c, inv[r][c], expected);
            }
        }
    }

    #[test]
    fn invert_diagonal_matrix() {
        // Diagonal matrix [[2,0,0],[0,3,0],[0,0,4]] inverts to [[0.5,0,0],[0,1/3,0],[0,0,0.25]].
        let m = [[2.0_f64, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 4.0]];
        let inv = invert_3x3(&m).unwrap();
        assert!((inv[0][0] - 0.5).abs() < 1e-12);
        assert!((inv[1][1] - 1.0/3.0).abs() < 1e-12);
        assert!((inv[2][2] - 0.25).abs() < 1e-12);
        // Off-diagonal must be zero.
        for r in 0..3 {
            for c in 0..3 {
                if r != c {
                    assert!(inv[r][c].abs() < 1e-12, "off-diag [{}][{}] != 0", r, c);
                }
            }
        }
    }

    #[test]
    fn invert_channel_swap_is_self_inverse() {
        // The R↔B swap matrix is its own inverse (applying it twice = identity).
        let swap = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
        let inv = invert_3x3(&swap).unwrap();
        for r in 0..3 {
            for c in 0..3 {
                assert!((inv[r][c] - swap[r][c]).abs() < 1e-12,
                    "swap matrix not self-inverse at [{r}][{c}]");
            }
        }
    }

    #[test]
    fn invert_singular_returns_none() {
        // Zero matrix has det = 0 — must return None.
        let zero = [[0.0_f64; 3]; 3];
        assert!(invert_3x3(&zero).is_none());
    }

    #[test]
    fn invert_rank_deficient_returns_none() {
        // All rows the same → linearly dependent → det = 0.
        let m = [[1.0_f64, 2.0, 3.0], [1.0, 2.0, 3.0], [1.0, 2.0, 3.0]];
        assert!(invert_3x3(&m).is_none());
    }

    #[test]
    fn m_times_inv_m_equals_identity() {
        // M × inv(M) = I for a typical camera colour matrix.
        let m = [
            [ 1.392, -0.418,  0.026],
            [-0.254,  1.614, -0.360],
            [ 0.068, -0.584,  1.516],
        ];
        let inv = invert_3x3(&m).unwrap();
        // Check each basis vector: M × inv(M) × e_i ≈ e_i.
        for i in 0..3 {
            let mut e = [0.0_f64; 3];
            e[i] = 1.0;
            let result = mat3x3_mul(&m, mat3x3_mul(&inv, e));
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((result[j] - expected).abs() < 1e-8,
                    "M×inv(M) failed: [{}][{}] = {}, expected {}",
                    i, j, result[j], expected);
            }
        }
    }

    #[test]
    fn inv_m_times_m_equals_identity() {
        // inv(M) × M = I (check the other direction too).
        let m = [
            [1.318, -0.398, 0.080],
            [-0.213, 1.586, -0.373],
            [0.047, -0.474, 1.427],
        ];
        let inv = invert_3x3(&m).unwrap();
        for i in 0..3 {
            let mut e = [0.0_f64; 3];
            e[i] = 1.0;
            let result = mat3x3_mul(&inv, mat3x3_mul(&m, e));
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((result[j] - expected).abs() < 1e-8,
                    "inv(M)×M failed: [{}][{}] = {}, expected {}",
                    i, j, result[j], expected);
            }
        }
    }
}
