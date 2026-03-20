//! Tests for BLAS Level 2 operations: matrix-vector operations.
//!
//! Level 2 operations are O(n^2) -- they process each matrix element once.

use blas_library::traits::BlasBackend;
use blas_library::{CpuBlas, Matrix, Transpose, Vector};

fn blas() -> CpuBlas {
    CpuBlas
}

// =========================================================================
// SGEMV tests: y = alpha * op(A) * x + beta * y
// =========================================================================

#[test]
fn test_sgemv_basic_no_trans() {
    // A = [[1, 2], [3, 4]], x = [1, 1], y = [0, 0]
    // result = 1.0 * A * x + 0.0 * y = [3, 7]
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let x = Vector::new(vec![1.0, 1.0]);
    let y = Vector::zeros(2);
    let result = blas()
        .sgemv(Transpose::NoTrans, 1.0, &a, &x, 0.0, &y)
        .unwrap();
    assert_eq!(result.data(), &[3.0, 7.0]);
}

#[test]
fn test_sgemv_with_beta() {
    // A = [[1, 2], [3, 4]], x = [1, 0], y = [10, 20]
    // result = 1.0 * A * x + 1.0 * y = [1+10, 3+20] = [11, 23]
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let x = Vector::new(vec![1.0, 0.0]);
    let y = Vector::new(vec![10.0, 20.0]);
    let result = blas()
        .sgemv(Transpose::NoTrans, 1.0, &a, &x, 1.0, &y)
        .unwrap();
    assert_eq!(result.data(), &[11.0, 23.0]);
}

#[test]
fn test_sgemv_with_alpha() {
    // A = [[1, 2], [3, 4]], x = [1, 1], y = [0, 0]
    // result = 2.0 * A * x + 0.0 * y = [6, 14]
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let x = Vector::new(vec![1.0, 1.0]);
    let y = Vector::zeros(2);
    let result = blas()
        .sgemv(Transpose::NoTrans, 2.0, &a, &x, 0.0, &y)
        .unwrap();
    assert_eq!(result.data(), &[6.0, 14.0]);
}

#[test]
fn test_sgemv_transposed() {
    // A = [[1, 2], [3, 4]], A^T = [[1, 3], [2, 4]]
    // A^T * [1, 1] = [4, 6]
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let x = Vector::new(vec![1.0, 1.0]);
    let y = Vector::zeros(2);
    let result = blas()
        .sgemv(Transpose::Trans, 1.0, &a, &x, 0.0, &y)
        .unwrap();
    assert_eq!(result.data(), &[4.0, 6.0]);
}

#[test]
fn test_sgemv_rectangular() {
    // A = [[1, 2, 3], [4, 5, 6]], x = [1, 1, 1], y = [0, 0]
    // result = A * x = [6, 15]
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let x = Vector::new(vec![1.0, 1.0, 1.0]);
    let y = Vector::zeros(2);
    let result = blas()
        .sgemv(Transpose::NoTrans, 1.0, &a, &x, 0.0, &y)
        .unwrap();
    assert_eq!(result.data(), &[6.0, 15.0]);
}

#[test]
fn test_sgemv_rectangular_transposed() {
    // A = [[1, 2, 3], [4, 5, 6]] (2x3)
    // A^T = [[1, 4], [2, 5], [3, 6]] (3x2)
    // A^T * [1, 1] = [5, 7, 9]
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let x = Vector::new(vec![1.0, 1.0]);
    let y = Vector::zeros(3);
    let result = blas()
        .sgemv(Transpose::Trans, 1.0, &a, &x, 0.0, &y)
        .unwrap();
    assert_eq!(result.data(), &[5.0, 7.0, 9.0]);
}

#[test]
fn test_sgemv_identity() {
    // I * x = x
    let a = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let x = Vector::new(vec![3.0, 7.0]);
    let y = Vector::zeros(2);
    let result = blas()
        .sgemv(Transpose::NoTrans, 1.0, &a, &x, 0.0, &y)
        .unwrap();
    assert_eq!(result.data(), &[3.0, 7.0]);
}

#[test]
fn test_sgemv_x_dimension_mismatch() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::zeros(2);
    assert!(blas()
        .sgemv(Transpose::NoTrans, 1.0, &a, &x, 0.0, &y)
        .is_err());
}

#[test]
fn test_sgemv_y_dimension_mismatch() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let x = Vector::new(vec![1.0, 2.0]);
    let y = Vector::new(vec![0.0, 0.0, 0.0]);
    assert!(blas()
        .sgemv(Transpose::NoTrans, 1.0, &a, &x, 0.0, &y)
        .is_err());
}

#[test]
fn test_sgemv_zero_alpha_zero_beta() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let x = Vector::new(vec![1.0, 1.0]);
    let y = Vector::new(vec![100.0, 200.0]);
    let result = blas()
        .sgemv(Transpose::NoTrans, 0.0, &a, &x, 0.0, &y)
        .unwrap();
    assert_eq!(result.data(), &[0.0, 0.0]);
}

#[test]
fn test_sgemv_zero_alpha_unit_beta() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let x = Vector::new(vec![1.0, 1.0]);
    let y = Vector::new(vec![100.0, 200.0]);
    let result = blas()
        .sgemv(Transpose::NoTrans, 0.0, &a, &x, 1.0, &y)
        .unwrap();
    assert_eq!(result.data(), &[100.0, 200.0]);
}

// =========================================================================
// SGER tests: A = alpha * x * y^T + A
// =========================================================================

#[test]
fn test_sger_basic() {
    // x = [1, 2], y = [3, 4], A = zeros(2, 2)
    // result = 1.0 * [1,2] * [3,4]^T = [[3, 4], [6, 8]]
    let x = Vector::new(vec![1.0, 2.0]);
    let y = Vector::new(vec![3.0, 4.0]);
    let a = Matrix::zeros(2, 2);
    let result = blas().sger(1.0, &x, &y, &a).unwrap();
    assert_eq!(result.data(), &[3.0, 4.0, 6.0, 8.0]);
}

#[test]
fn test_sger_with_existing_a() {
    // x = [1, 1], y = [1, 1], A = [[10, 20], [30, 40]]
    // result = 1.0 * outer(x, y) + A = [[11, 21], [31, 41]]
    let x = Vector::new(vec![1.0, 1.0]);
    let y = Vector::new(vec![1.0, 1.0]);
    let a = Matrix::new(vec![10.0, 20.0, 30.0, 40.0], 2, 2);
    let result = blas().sger(1.0, &x, &y, &a).unwrap();
    assert_eq!(result.data(), &[11.0, 21.0, 31.0, 41.0]);
}

#[test]
fn test_sger_with_alpha() {
    let x = Vector::new(vec![1.0, 2.0]);
    let y = Vector::new(vec![3.0, 4.0]);
    let a = Matrix::zeros(2, 2);
    let result = blas().sger(2.0, &x, &y, &a).unwrap();
    assert_eq!(result.data(), &[6.0, 8.0, 12.0, 16.0]);
}

#[test]
fn test_sger_rectangular() {
    // x = [1, 2], y = [1, 2, 3], A = zeros(2, 3)
    let x = Vector::new(vec![1.0, 2.0]);
    let y = Vector::new(vec![1.0, 2.0, 3.0]);
    let a = Matrix::zeros(2, 3);
    let result = blas().sger(1.0, &x, &y, &a).unwrap();
    assert_eq!(result.data(), &[1.0, 2.0, 3.0, 2.0, 4.0, 6.0]);
}

#[test]
fn test_sger_dimension_mismatch_rows() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::new(vec![1.0, 2.0]);
    let a = Matrix::zeros(2, 2);
    assert!(blas().sger(1.0, &x, &y, &a).is_err());
}

#[test]
fn test_sger_dimension_mismatch_cols() {
    let x = Vector::new(vec![1.0, 2.0]);
    let y = Vector::new(vec![1.0, 2.0, 3.0]);
    let a = Matrix::zeros(2, 2);
    assert!(blas().sger(1.0, &x, &y, &a).is_err());
}

#[test]
fn test_sger_zero_alpha() {
    let x = Vector::new(vec![1.0, 2.0]);
    let y = Vector::new(vec![3.0, 4.0]);
    let a = Matrix::new(vec![10.0, 20.0, 30.0, 40.0], 2, 2);
    let result = blas().sger(0.0, &x, &y, &a).unwrap();
    assert_eq!(result.data(), &[10.0, 20.0, 30.0, 40.0]);
}
