// Matches lib.rs's own allow: index loops here parallel matrix-math
// notation (row/col indices used directly in the arithmetic), which reads
// more naturally than an iterator-adapter rewrite for this kind of code.
#![allow(clippy::needless_range_loop)]

use matrix::Matrix;

#[test]
fn test_zeros() {
    let z = Matrix::zeros(2, 3);
    assert_eq!(z.rows, 2);
    assert_eq!(z.cols, 3);
    assert_eq!(z.data[1][2], 0.0);
}

#[test]
fn test_add_subtract() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    let b = Matrix::new_2d(vec![vec![5.0, 6.0], vec![7.0, 8.0]]);
    
    let c = a.add(&b).unwrap();
    assert_eq!(c.data, vec![vec![6.0, 8.0], vec![10.0, 12.0]]);
    
    let d = b.subtract(&a).unwrap();
    assert_eq!(d.data, vec![vec![4.0, 4.0], vec![4.0, 4.0]]);
    
    // Scalar addition execution
    let e = a.add_scalar(2.0);
    assert_eq!(e.data, vec![vec![3.0, 4.0], vec![5.0, 6.0]]);
    
    assert!(a.add(&Matrix::new_scalar(1.0)).is_err());
}

#[test]
fn test_scale() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    let c = a.scale(2.0);
    assert_eq!(c.data, vec![vec![2.0, 4.0], vec![6.0, 8.0]]);
}

#[test]
fn test_transpose() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
    let c = a.transpose();
    assert_eq!(c.data, vec![vec![1.0, 4.0], vec![2.0, 5.0], vec![3.0, 6.0]]);
}

#[test]
fn test_dot() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    let b = Matrix::new_2d(vec![vec![5.0, 6.0], vec![7.0, 8.0]]);
    
    let c = a.dot(&b).unwrap();
    assert_eq!(c.data, vec![vec![19.0, 22.0], vec![43.0, 50.0]]);
    
    let d = Matrix::new_1d(vec![1.0, 2.0, 3.0]);
    let e = Matrix::new_2d(vec![vec![4.0], vec![5.0], vec![6.0]]);
    let f = d.dot(&e).unwrap();
    assert_eq!(f.data, vec![vec![32.0]]);

    assert!(a.dot(&e).is_err());
}

// ─── Linear Solve (VIS01) ───────────────────────────────────────────

fn assert_close(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() <= tol, "{a} not close to {b} (tolerance {tol})");
}

#[test]
fn solve_identity_returns_b_unchanged() {
    let i = Matrix::identity(3);
    let b = vec![1.0, 2.0, 3.0];
    let x = i.solve(&b).unwrap();
    for k in 0..3 {
        assert_close(x[k], b[k], 1e-12);
    }
}

#[test]
fn solve_diagonal_divides_each_entry() {
    let d = Matrix::from_diagonal(&[2.0, 4.0, 5.0]);
    let x = d.solve(&[10.0, 20.0, 25.0]).unwrap();
    assert_close(x[0], 5.0, 1e-9);
    assert_close(x[1], 5.0, 1e-9);
    assert_close(x[2], 5.0, 1e-9);
}

#[test]
fn solve_hand_verified_3x3_system() {
    // x + y + z = 6, 2y + 5z = -4, 2x + 5y - z = 27 -> (x,y,z) = (5, 3, -2)
    let a = Matrix::new_2d(vec![
        vec![1.0, 1.0, 1.0],
        vec![0.0, 2.0, 5.0],
        vec![2.0, 5.0, -1.0],
    ]);
    let x = a.solve(&[6.0, -4.0, 27.0]).unwrap();
    assert_close(x[0], 5.0, 1e-9);
    assert_close(x[1], 3.0, 1e-9);
    assert_close(x[2], -2.0, 1e-9);
}

#[test]
fn solve_round_trip_a_dot_x_equals_b() {
    let a = Matrix::new_2d(vec![
        vec![4.0, 3.0, 0.0, 1.0],
        vec![2.0, -1.0, 5.0, 0.0],
        vec![1.0, 2.0, 3.0, 4.0],
        vec![0.0, 5.0, -2.0, 1.0],
    ]);
    let b = vec![10.0, -3.0, 22.0, 7.0];
    let x = a.solve(&b).unwrap();
    let x_col = Matrix::new_2d(x.iter().map(|&v| vec![v]).collect());
    let recovered = a.dot(&x_col).unwrap();
    for i in 0..4 {
        assert_close(recovered.data[i][0], b[i], 1e-6);
    }
}

#[test]
fn solve_rejects_non_square() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
    let err = a.solve(&[1.0, 2.0]).unwrap_err();
    assert!(err.contains("square"));
}

#[test]
fn solve_rejects_mismatched_b_length() {
    let a = Matrix::identity(3);
    let err = a.solve(&[1.0, 2.0]).unwrap_err();
    assert!(!err.contains("square")); // distinct from the non-square error class
}

#[test]
fn solve_rejects_zero_row_as_singular() {
    let a = Matrix::new_2d(vec![vec![0.0, 0.0], vec![1.0, 1.0]]);
    let err = a.solve(&[1.0, 2.0]).unwrap_err();
    assert!(err.contains("singular"));
}

#[test]
fn solve_rejects_identical_rows_as_singular() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0], vec![1.0, 2.0]]);
    assert!(a.solve(&[3.0, 3.0]).is_err());
}

#[test]
fn solve_rejects_proportional_rows_as_singular() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0], vec![2.0, 4.0]]);
    assert!(a.solve(&[3.0, 6.0]).is_err());
}

#[test]
fn solve_partial_pivoting_recovers_accurate_answer() {
    // Row 0 has a tiny pivot (1e-12); without swapping to bring row 1's
    // much larger value onto the diagonal first, eliminating x from row 1
    // using row 0 as the pivot row divides by that near-zero value,
    // amplifying floating-point rounding error into the result. Exact
    // answer (solve 1e-12*x + y = 1, x + y = 2): x = 1, y = 1.
    let a = Matrix::new_2d(vec![vec![1e-12, 1.0], vec![1.0, 1.0]]);
    let x = a.solve(&[1.0, 2.0]).unwrap();
    assert_close(x[0], 1.0, 1e-6);
    assert_close(x[1], 1.0, 1e-6);
}

#[test]
fn solve_does_not_panic_on_nan_or_inf() {
    let a = Matrix::new_2d(vec![vec![f64::NAN, 1.0], vec![1.0, 1.0]]);
    let _ = a.solve(&[1.0, 2.0]); // must not panic; Ok(NaN...) or Err both acceptable
    let b = Matrix::new_2d(vec![vec![f64::INFINITY, 1.0], vec![1.0, 1.0]]);
    let _ = b.solve(&[1.0, 2.0]);
}

#[test]
fn solve_zero_size_returns_empty_vector() {
    let empty = Matrix::new_2d(vec![]);
    let x = empty.solve(&[]).unwrap();
    assert!(x.is_empty());
}

#[test]
fn invert_2x2_round_trips() {
    let a = Matrix::new_2d(vec![vec![4.0, 7.0], vec![2.0, 6.0]]);
    let inv = a.invert().unwrap();
    let product = a.dot(&inv).unwrap();
    assert!(product.close(&Matrix::identity(2), 1e-9));
}

#[test]
fn invert_identity_is_identity() {
    let i = Matrix::identity(5);
    let inv = i.invert().unwrap();
    assert!(inv.close(&Matrix::identity(5), 1e-12));
}

#[test]
fn invert_8x8_round_trips() {
    // The size VIS03's homography solve actually needs.
    let mut data = vec![vec![0.0; 8]; 8];
    for i in 0..8 {
        for j in 0..8 {
            data[i][j] = ((i * 7 + j * 3 + 1) % 11) as f64 - 5.0;
        }
        data[i][i] += 25.0; // strengthen the diagonal so the matrix is well-conditioned
    }
    let a = Matrix::new_2d(data);
    let inv = a.invert().unwrap();
    let product = a.dot(&inv).unwrap();
    assert!(product.close(&Matrix::identity(8), 1e-6));
}

#[test]
fn invert_rejects_non_square() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
    assert!(a.invert().is_err());
}

#[test]
fn invert_rejects_singular() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0], vec![2.0, 4.0]]);
    assert!(a.invert().is_err());
}

#[test]
fn determinant_identity_is_one() {
    assert_close(Matrix::identity(4).determinant().unwrap(), 1.0, 1e-12);
}

#[test]
fn determinant_diagonal_is_product_of_diagonal() {
    let d = Matrix::from_diagonal(&[2.0, 3.0, 5.0]);
    assert_close(d.determinant().unwrap(), 30.0, 1e-9);
}

#[test]
fn determinant_identical_rows_is_zero() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0], vec![1.0, 2.0]]);
    assert_close(a.determinant().unwrap(), 0.0, 1e-9);
    // A zero determinant implies singularity: solve/invert on the same
    // matrix must agree, not silently return a plausible-looking answer.
    assert!(a.solve(&[1.0, 1.0]).is_err());
    assert!(a.invert().is_err());
}

#[test]
fn determinant_known_3x3_value() {
    let a = Matrix::new_2d(vec![
        vec![6.0, 1.0, 1.0],
        vec![4.0, -2.0, 5.0],
        vec![2.0, 8.0, 7.0],
    ]);
    assert_close(a.determinant().unwrap(), -306.0, 1e-6);
}

#[test]
fn determinant_rejects_non_square() {
    let a = Matrix::new_2d(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
    assert!(a.determinant().is_err());
}

#[test]
fn determinant_does_not_panic_on_nan_or_inf() {
    let a = Matrix::new_2d(vec![vec![f64::NAN, 1.0], vec![1.0, 1.0]]);
    let _ = a.determinant();
}
