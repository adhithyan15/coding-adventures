//! Tests for BLAS Level 3 operations: matrix-matrix operations.
//!
//! Level 3 operations are O(n^3) -- the workhorses of ML.

use blas_library::traits::BlasBackend;
use blas_library::{CpuBlas, Matrix, Side, Transpose};

fn blas() -> CpuBlas {
    CpuBlas
}

// =========================================================================
// SGEMM tests: C = alpha * op(A) * op(B) + beta * C
// =========================================================================

#[test]
fn test_sgemm_identity() {
    // I * B = B
    let a = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2);
    let c = Matrix::zeros(2, 2);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(result.data(), &[5.0, 6.0, 7.0, 8.0]);
}

#[test]
fn test_sgemm_basic_2x2() {
    // [[1, 2], [3, 4]] * [[5, 6], [7, 8]] = [[19, 22], [43, 50]]
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2);
    let c = Matrix::zeros(2, 2);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(result.data(), &[19.0, 22.0, 43.0, 50.0]);
}

#[test]
fn test_sgemm_with_alpha() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2);
    let c = Matrix::zeros(2, 2);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 2.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(result.data(), &[38.0, 44.0, 86.0, 100.0]);
}

#[test]
fn test_sgemm_with_beta() {
    let a = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let b = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let c = Matrix::new(vec![10.0, 20.0, 30.0, 40.0], 2, 2);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 1.0, &c)
        .unwrap();
    assert_eq!(result.data(), &[11.0, 20.0, 30.0, 41.0]);
}

#[test]
fn test_sgemm_trans_a() {
    // A = [[1, 3], [2, 4]], A^T = [[1, 2], [3, 4]]
    // A^T * B where B = [[5, 6], [7, 8]]
    // = [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]]
    // = [[19, 22], [43, 50]]
    let a = Matrix::new(vec![1.0, 3.0, 2.0, 4.0], 2, 2);
    let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2);
    let c = Matrix::zeros(2, 2);
    let result = blas()
        .sgemm(Transpose::Trans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(result.data(), &[19.0, 22.0, 43.0, 50.0]);
}

#[test]
fn test_sgemm_trans_b() {
    // A = [[1, 2], [3, 4]], B^T = [[5, 7], [6, 8]]
    // A * B^T = [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]] = [[19, 22], [43, 50]]
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let b = Matrix::new(vec![5.0, 7.0, 6.0, 8.0], 2, 2);
    let c = Matrix::zeros(2, 2);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::Trans, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(result.data(), &[19.0, 22.0, 43.0, 50.0]);
}

#[test]
fn test_sgemm_trans_both() {
    let a = Matrix::new(vec![1.0, 3.0, 2.0, 4.0], 2, 2);
    let b = Matrix::new(vec![5.0, 7.0, 6.0, 8.0], 2, 2);
    let c = Matrix::zeros(2, 2);
    let result = blas()
        .sgemm(Transpose::Trans, Transpose::Trans, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(result.data(), &[19.0, 22.0, 43.0, 50.0]);
}

#[test]
fn test_sgemm_rectangular_2x3_times_3x2() {
    // A = [[1, 2, 3], [4, 5, 6]], B = [[7, 8], [9, 10], [11, 12]]
    // C = A * B = [[58, 64], [139, 154]]
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let b = Matrix::new(vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0], 3, 2);
    let c = Matrix::zeros(2, 2);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(result.data(), &[58.0, 64.0, 139.0, 154.0]);
}

#[test]
fn test_sgemm_1x1() {
    let a = Matrix::new(vec![3.0], 1, 1);
    let b = Matrix::new(vec![4.0], 1, 1);
    let c = Matrix::zeros(1, 1);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(result.data(), &[12.0]);
}

#[test]
fn test_sgemm_dimension_mismatch_inner() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let b = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let c = Matrix::zeros(2, 2);
    assert!(blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .is_err());
}

#[test]
fn test_sgemm_dimension_mismatch_c() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let b = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let c = Matrix::zeros(3, 3);
    assert!(blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .is_err());
}

#[test]
fn test_sgemm_zero_alpha_preserves_c() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2);
    let c = Matrix::new(vec![10.0, 20.0, 30.0, 40.0], 2, 2);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 0.0, &a, &b, 1.0, &c)
        .unwrap();
    assert_eq!(result.data(), &[10.0, 20.0, 30.0, 40.0]);
}

#[test]
fn test_sgemm_3x3() {
    let a = Matrix::new(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0], 3, 3);
    let b = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let c = Matrix::zeros(3, 3);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(
        result.data(),
        &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]
    );
}

// =========================================================================
// SSYMM tests: C = alpha * A * B + beta * C (A symmetric)
// =========================================================================

#[test]
fn test_ssymm_left() {
    // A = [[1, 2], [2, 3]] (symmetric), B = [[1, 0], [0, 1]]
    // C = A * B = A
    let a = Matrix::new(vec![1.0, 2.0, 2.0, 3.0], 2, 2);
    let b = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let c = Matrix::zeros(2, 2);
    let result = blas()
        .ssymm(Side::Left, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(result.data(), &[1.0, 2.0, 2.0, 3.0]);
}

#[test]
fn test_ssymm_right() {
    // A = [[1, 2], [2, 3]] (symmetric), B = [[1, 0], [0, 1]]
    // C = B * A = A
    let a = Matrix::new(vec![1.0, 2.0, 2.0, 3.0], 2, 2);
    let b = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let c = Matrix::zeros(2, 2);
    let result = blas()
        .ssymm(Side::Right, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(result.data(), &[1.0, 2.0, 2.0, 3.0]);
}

#[test]
fn test_ssymm_not_square() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let b = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let c = Matrix::zeros(2, 2);
    assert!(blas().ssymm(Side::Left, 1.0, &a, &b, 0.0, &c).is_err());
}

#[test]
fn test_ssymm_left_with_alpha_beta() {
    let a = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let b = Matrix::new(vec![2.0, 3.0, 4.0, 5.0], 2, 2);
    let c = Matrix::new(vec![10.0, 10.0, 10.0, 10.0], 2, 2);
    let result = blas()
        .ssymm(Side::Left, 2.0, &a, &b, 1.0, &c)
        .unwrap();
    // 2 * I * B + C = 2*B + C
    assert_eq!(result.data(), &[14.0, 16.0, 18.0, 20.0]);
}

#[test]
fn test_ssymm_left_dimension_mismatch() {
    let a = Matrix::new(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0], 3, 3);
    let b = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let c = Matrix::zeros(3, 2);
    assert!(blas().ssymm(Side::Left, 1.0, &a, &b, 0.0, &c).is_err());
}

#[test]
fn test_ssymm_right_dimension_mismatch() {
    let a = Matrix::new(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0], 3, 3);
    let b = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let c = Matrix::zeros(2, 3);
    assert!(blas().ssymm(Side::Right, 1.0, &a, &b, 0.0, &c).is_err());
}

#[test]
fn test_ssymm_c_dimension_mismatch() {
    let a = Matrix::new(vec![1.0, 2.0, 2.0, 3.0], 2, 2);
    let b = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let c = Matrix::zeros(3, 3);
    assert!(blas().ssymm(Side::Left, 1.0, &a, &b, 0.0, &c).is_err());
}

// =========================================================================
// SGEMM_BATCHED tests: multiple independent GEMMs
// =========================================================================

#[test]
fn test_sgemm_batched_basic() {
    let a1 = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let b1 = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2);
    let c1 = Matrix::zeros(2, 2);

    let a2 = Matrix::new(vec![2.0, 0.0, 0.0, 2.0], 2, 2);
    let b2 = Matrix::new(vec![1.0, 1.0, 1.0, 1.0], 2, 2);
    let c2 = Matrix::zeros(2, 2);

    let results = blas()
        .sgemm_batched(
            Transpose::NoTrans,
            Transpose::NoTrans,
            1.0,
            &[a1, a2],
            &[b1, b2],
            0.0,
            &[c1, c2],
        )
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].data(), &[5.0, 6.0, 7.0, 8.0]);
    assert_eq!(results[1].data(), &[2.0, 2.0, 2.0, 2.0]);
}

#[test]
fn test_sgemm_batched_size_mismatch() {
    let a = vec![Matrix::zeros(2, 2)];
    let b = vec![Matrix::zeros(2, 2), Matrix::zeros(2, 2)];
    let c = vec![Matrix::zeros(2, 2)];
    assert!(blas()
        .sgemm_batched(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .is_err());
}

#[test]
fn test_sgemm_batched_single() {
    let a = vec![Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2)];
    let b = vec![Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2)];
    let c = vec![Matrix::zeros(2, 2)];
    let results = blas()
        .sgemm_batched(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].data(), &[19.0, 22.0, 43.0, 50.0]);
}

#[test]
fn test_sgemm_batched_empty() {
    let results = blas()
        .sgemm_batched(
            Transpose::NoTrans,
            Transpose::NoTrans,
            1.0,
            &[],
            &[],
            0.0,
            &[],
        )
        .unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_sgemm_batched_with_transpose() {
    let a = vec![Matrix::new(vec![1.0, 3.0, 2.0, 4.0], 2, 2)];
    let b = vec![Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2)];
    let c = vec![Matrix::zeros(2, 2)];
    let results = blas()
        .sgemm_batched(Transpose::Trans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(results[0].data(), &[19.0, 22.0, 43.0, 50.0]);
}
