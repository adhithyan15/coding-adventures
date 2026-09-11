//! # `vis-geometric-estimation` — VIS03
//!
//! Given a handful of point correspondences — where a landmark
//! actually sits, and where it should sit — solve for the geometric
//! transform between them. This is the piece that produces the 3x3
//! matrix `image_geometric_transforms::perspective_warp` already
//! *applies*; nothing in this workspace previously *produced* one from
//! real point measurements.
//!
//! ## Two transform families, each solved exactly
//!
//! - [`estimate_affine`] — rotation, scale, shear, translation; 6
//!   degrees of freedom, exactly determined by **3** correspondences.
//! - [`estimate_homography`] — a full projective transform (also
//!   corrects perspective/keystoning); 8 degrees of freedom, exactly
//!   determined by **4** correspondences, via the standard DLT
//!   (direct linear transform) construction.
//!
//! Both solve *exactly* from the minimum point count, not as a
//! least-squares fit from more (see `VIS03-geometric-estimation.md`
//! §2 for why that's deliberately out of scope here).
//!
//! ## Direction
//!
//! Both functions solve "the transform mapping `from[i]` to `to[i]`."
//! `perspective_warp` reads an **output-to-input** matrix ("given a
//! point in the destination image, where do I read from the source").
//! To get a `perspective_warp`-ready matrix directly, pass `from` =
//! points in the clean/destination space and `to` = the same points as
//! actually measured in the source image. This crate does not enforce
//! that direction — see the spec's §3.3 for why it can't be checked at
//! the type level, only documented.
//!
//! ## Usage
//!
//! ```
//! use vis_geometric_estimation::estimate_affine;
//!
//! // A translation by (10, 5): every point moves right 10, down 5.
//! let from = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
//! let to = [(10.0, 5.0), (11.0, 5.0), (10.0, 6.0)];
//! let m = estimate_affine(&from, &to).unwrap();
//! assert!((m[0][2] - 10.0).abs() < 1e-9); // c
//! assert!((m[1][2] - 5.0).abs() < 1e-9);  // f
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use matrix::Matrix;

/// Why `estimate_affine`/`estimate_homography` could not produce a
/// transform.
#[derive(Debug, Clone, PartialEq)]
pub enum EstimationError {
    /// `estimate_affine` needs exactly 3 correspondences,
    /// `estimate_homography` needs exactly 4. `got` is the larger of
    /// `from.len()`/`to.len()` when they disagree with each other, so
    /// it's always a concrete, informative number even when `from`
    /// and `to` were different lengths to begin with.
    WrongPointCount {
        /// The exact count this function requires.
        needed: usize,
        /// What was actually supplied.
        got: usize,
    },
    /// The underlying linear system (`matrix::solve`) was singular --
    /// degenerate input (collinear or coincident points) with no
    /// unique solution. Carries `matrix::solve`'s own message
    /// unchanged, not re-derived.
    Degenerate(String),
}

/// Estimate the 3x3 homogeneous affine transform mapping `from[i]` to
/// `to[i]`, for exactly 3 correspondences -- affine's 6 degrees of
/// freedom are exactly determined by 3 points, not fit as a
/// least-squares approximation from more.
///
/// # Errors
/// [`EstimationError::WrongPointCount`] if `from`/`to` don't both have
/// exactly 3 entries. [`EstimationError::Degenerate`] if the 3 `from`
/// points are collinear (no unique affine transform exists).
pub fn estimate_affine(
    from: &[(f64, f64)],
    to: &[(f64, f64)],
) -> Result<[[f64; 3]; 3], EstimationError> {
    check_point_count(from, to, 3)?;

    // x' = a*x + b*y + c, y' = d*x + e*y + f -- the same 3x3 matrix M
    // determines both (a,b,c) and (d,e,f), against two different
    // right-hand sides (VIS03-geometric-estimation.md §3.1).
    let m = Matrix::new_2d(vec![
        vec![from[0].0, from[0].1, 1.0],
        vec![from[1].0, from[1].1, 1.0],
        vec![from[2].0, from[2].1, 1.0],
    ]);
    let abc = m
        .solve(&[to[0].0, to[1].0, to[2].0])
        .map_err(EstimationError::Degenerate)?;
    let def = m
        .solve(&[to[0].1, to[1].1, to[2].1])
        .map_err(EstimationError::Degenerate)?;

    Ok([
        [abc[0], abc[1], abc[2]],
        [def[0], def[1], def[2]],
        [0.0, 0.0, 1.0],
    ])
}

/// Estimate the 3x3 homography mapping `from[i]` to `to[i]`, for
/// exactly 4 correspondences, via the standard DLT construction
/// normalized so `h33 = 1` (VIS03-geometric-estimation.md §3.2). Same
/// `from`/`to` direction convention as [`estimate_affine`].
///
/// # Errors
/// [`EstimationError::WrongPointCount`] if `from`/`to` don't both have
/// exactly 4 entries. [`EstimationError::Degenerate`] if the 4 `from`
/// points don't determine a unique homography (collinear or
/// coincident points among them).
pub fn estimate_homography(
    from: &[(f64, f64)],
    to: &[(f64, f64)],
) -> Result<[[f64; 3]; 3], EstimationError> {
    check_point_count(from, to, 4)?;

    // Unknowns: [h11, h12, h13, h21, h22, h23, h31, h32], with h33
    // fixed to 1. Two rows per correspondence (VIS03-geometric-
    // estimation.md §3.2).
    let mut rows: Vec<Vec<f64>> = Vec::with_capacity(8);
    let mut b: Vec<f64> = Vec::with_capacity(8);
    for i in 0..4 {
        let (x, y) = from[i];
        let (xp, yp) = to[i];
        rows.push(vec![x, y, 1.0, 0.0, 0.0, 0.0, -x * xp, -y * xp]);
        b.push(xp);
        rows.push(vec![0.0, 0.0, 0.0, x, y, 1.0, -x * yp, -y * yp]);
        b.push(yp);
    }
    let a = Matrix::new_2d(rows);
    let h = a.solve(&b).map_err(EstimationError::Degenerate)?;

    Ok([
        [h[0], h[1], h[2]],
        [h[3], h[4], h[5]],
        [h[6], h[7], 1.0],
    ])
}

fn check_point_count(
    from: &[(f64, f64)],
    to: &[(f64, f64)],
    needed: usize,
) -> Result<(), EstimationError> {
    if from.len() != needed || to.len() != needed {
        return Err(EstimationError::WrongPointCount {
            needed,
            got: from.len().max(to.len()),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} not close to {b} (tolerance {tol})");
    }

    /// Apply a 3x3 homogeneous matrix to a point, dividing through by
    /// the homogeneous (third) component -- the general application
    /// rule, which is a no-op division for an affine matrix (whose
    /// third row is always `[0, 0, 1]`) and does real perspective
    /// division for a true homography.
    fn apply(m: &[[f64; 3]; 3], p: (f64, f64)) -> (f64, f64) {
        let (x, y) = p;
        let u = m[0][0] * x + m[0][1] * y + m[0][2];
        let v = m[1][0] * x + m[1][1] * y + m[1][2];
        let w = m[2][0] * x + m[2][1] * y + m[2][2];
        (u / w, v / w)
    }

    // --- estimate_affine ----------------------------------------------

    #[test]
    fn affine_identity() {
        let pts = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let m = estimate_affine(&pts, &pts).unwrap();
        let held_out = apply(&m, (3.7, -2.1));
        assert_close(held_out.0, 3.7, 1e-9);
        assert_close(held_out.1, -2.1, 1e-9);
    }

    #[test]
    fn affine_pure_translation() {
        let from = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let to = [(10.0, 5.0), (11.0, 5.0), (10.0, 6.0)];
        let m = estimate_affine(&from, &to).unwrap();
        let held_out = apply(&m, (2.0, 3.0));
        assert_close(held_out.0, 12.0, 1e-9);
        assert_close(held_out.1, 8.0, 1e-9);
    }

    #[test]
    fn affine_rotation_scale_translation_round_trip() {
        // Known transform: rotate 30 degrees, scale 2x, translate (5, -3).
        let theta = std::f64::consts::PI / 6.0;
        let (cos_t, sin_t) = (theta.cos(), theta.sin());
        let known = [
            [2.0 * cos_t, -2.0 * sin_t, 5.0],
            [2.0 * sin_t, 2.0 * cos_t, -3.0],
            [0.0, 0.0, 1.0],
        ];
        let from = [(1.0, 0.0), (0.0, 1.0), (2.0, 3.0)];
        let to: Vec<(f64, f64)> = from.iter().map(|&p| apply(&known, p)).collect();

        let recovered = estimate_affine(&from, &to).unwrap();
        // A held-out point not used to estimate the transform.
        let held_out_src = (-4.0, 7.5);
        let expected = apply(&known, held_out_src);
        let actual = apply(&recovered, held_out_src);
        assert_close(actual.0, expected.0, 1e-6);
        assert_close(actual.1, expected.1, 1e-6);
    }

    #[test]
    fn affine_rejects_collinear_points() {
        let from = [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]; // all on y=0
        let to = [(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)];
        let err = estimate_affine(&from, &to).unwrap_err();
        assert!(matches!(err, EstimationError::Degenerate(_)));
    }

    #[test]
    fn affine_rejects_wrong_point_count() {
        let two = [(0.0, 0.0), (1.0, 0.0)];
        let three = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let four = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        assert_eq!(
            estimate_affine(&two, &two).unwrap_err(),
            EstimationError::WrongPointCount { needed: 3, got: 2 }
        );
        assert_eq!(
            estimate_affine(&four, &four).unwrap_err(),
            EstimationError::WrongPointCount { needed: 3, got: 4 }
        );
        // Length mismatch between from and to.
        assert_eq!(
            estimate_affine(&three, &two).unwrap_err(),
            EstimationError::WrongPointCount { needed: 3, got: 3 }
        );
    }

    // --- estimate_homography -------------------------------------------

    #[test]
    fn homography_identity() {
        let pts = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        let m = estimate_homography(&pts, &pts).unwrap();
        let held_out = apply(&m, (0.25, 0.75));
        assert_close(held_out.0, 0.25, 1e-6);
        assert_close(held_out.1, 0.75, 1e-6);
    }

    #[test]
    fn homography_with_real_perspective_term_round_trips() {
        // A known homography with a genuine perspective component
        // (h31, h32 both nonzero) -- not reducible to an affine map.
        let known = [[1.2, 0.1, 0.5], [-0.2, 0.9, 0.3], [0.0007, -0.0003, 1.0]];
        let from = [(0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (10.0, 10.0)];
        let to: Vec<(f64, f64)> = from.iter().map(|&p| apply(&known, p)).collect();

        let recovered = estimate_homography(&from, &to).unwrap();
        let held_out_src = (4.0, 6.0);
        let expected = apply(&known, held_out_src);
        let actual = apply(&recovered, held_out_src);
        assert_close(actual.0, expected.0, 1e-6);
        assert_close(actual.1, expected.1, 1e-6);
    }

    #[test]
    fn homography_rejects_collinear_points() {
        let from = [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)]; // all on y=0
        let to = [(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0)];
        let err = estimate_homography(&from, &to).unwrap_err();
        assert!(matches!(err, EstimationError::Degenerate(_)));
    }

    #[test]
    fn homography_rejects_coincident_points() {
        let from = [(0.0, 0.0), (0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]; // duplicate
        let to = [(0.0, 0.0), (0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let err = estimate_homography(&from, &to).unwrap_err();
        assert!(matches!(err, EstimationError::Degenerate(_)));
    }

    #[test]
    fn homography_rejects_wrong_point_count() {
        let three = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let five = [
            (0.0, 0.0),
            (1.0, 0.0),
            (0.0, 1.0),
            (1.0, 1.0),
            (2.0, 2.0),
        ];
        assert_eq!(
            estimate_homography(&three, &three).unwrap_err(),
            EstimationError::WrongPointCount { needed: 4, got: 3 }
        );
        assert_eq!(
            estimate_homography(&five, &five).unwrap_err(),
            EstimationError::WrongPointCount { needed: 4, got: 5 }
        );
    }

    // --- cross-validation ------------------------------------------------

    #[test]
    fn affine_is_a_special_case_of_homography() {
        // A known affine transform (h31 = h32 = 0 implicitly).
        let known_affine = [[1.5, 0.3, 2.0], [-0.4, 1.1, -1.0], [0.0, 0.0, 1.0]];
        let three = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let four = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];

        let to_three: Vec<(f64, f64)> = three.iter().map(|&p| apply(&known_affine, p)).collect();
        let to_four: Vec<(f64, f64)> = four.iter().map(|&p| apply(&known_affine, p)).collect();

        let via_affine = estimate_affine(&three, &to_three).unwrap();
        let via_homography = estimate_homography(&four, &to_four).unwrap();

        let held_out_src = (5.0, -2.0);
        let a = apply(&via_affine, held_out_src);
        let b = apply(&via_homography, held_out_src);
        assert_close(a.0, b.0, 1e-6);
        assert_close(a.1, b.1, 1e-6);
    }

    #[test]
    fn does_not_panic_on_empty_slices() {
        let empty: [(f64, f64); 0] = [];
        assert!(estimate_affine(&empty, &empty).is_err());
        assert!(estimate_homography(&empty, &empty).is_err());
    }
}
