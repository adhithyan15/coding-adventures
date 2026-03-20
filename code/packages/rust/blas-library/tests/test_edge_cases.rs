//! Edge case tests: boundary conditions, special values, numerical stability.

use blas_library::traits::{BlasBackend, MlBlasBackend};
use blas_library::{CpuBlas, Matrix, Side, Transpose, Vector};

fn blas() -> CpuBlas {
    CpuBlas
}

// =========================================================================
// Special float values
// =========================================================================

#[test]
fn test_saxpy_with_zeros() {
    let x = Vector::zeros(3);
    let y = Vector::zeros(3);
    let result = blas().saxpy(1.0, &x, &y).unwrap();
    assert_eq!(result.data(), &[0.0, 0.0, 0.0]);
}

#[test]
fn test_sdot_with_zeros() {
    let x = Vector::zeros(4);
    let y = Vector::new(vec![1.0, 2.0, 3.0, 4.0]);
    assert_eq!(blas().sdot(&x, &y).unwrap(), 0.0);
}

#[test]
fn test_snrm2_single_element() {
    let x = Vector::new(vec![-7.0]);
    assert_eq!(blas().snrm2(&x), 7.0);
}

#[test]
fn test_snrm2_empty() {
    let x = Vector::new(vec![]);
    assert_eq!(blas().snrm2(&x), 0.0);
}

#[test]
fn test_sasum_empty() {
    let x = Vector::new(vec![]);
    assert_eq!(blas().sasum(&x), 0.0);
}

#[test]
fn test_sscal_empty() {
    let x = Vector::new(vec![]);
    let result = blas().sscal(5.0, &x);
    assert_eq!(result.size(), 0);
}

#[test]
fn test_scopy_single() {
    let x = Vector::new(vec![42.0]);
    let copy = blas().scopy(&x);
    assert_eq!(copy.data(), &[42.0]);
}

// =========================================================================
// Large vector operations
// =========================================================================

#[test]
fn test_saxpy_large() {
    let n = 1000;
    let x = Vector::new(vec![1.0; n]);
    let y = Vector::new(vec![2.0; n]);
    let result = blas().saxpy(3.0, &x, &y).unwrap();
    assert!(result.data().iter().all(|&v| (v - 5.0).abs() < 1e-6));
}

#[test]
fn test_sdot_large() {
    let n = 1000;
    let x = Vector::new(vec![1.0; n]);
    let y = Vector::new(vec![1.0; n]);
    assert_eq!(blas().sdot(&x, &y).unwrap(), n as f32);
}

#[test]
fn test_sgemm_larger() {
    // 4x4 identity times 4x4 matrix = same matrix
    let mut id_data = vec![0.0_f32; 16];
    for i in 0..4 {
        id_data[i * 4 + i] = 1.0;
    }
    let identity = Matrix::new(id_data, 4, 4);
    let b = Matrix::new((1..=16).map(|i| i as f32).collect(), 4, 4);
    let c = Matrix::zeros(4, 4);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &identity, &b, 0.0, &c)
        .unwrap();
    assert_eq!(result.data(), b.data());
}

// =========================================================================
// Numerical precision tests
// =========================================================================

#[test]
fn test_sdot_precision() {
    // Many small additions can accumulate error
    let n = 100;
    let x = Vector::new(vec![0.1; n]);
    let y = Vector::new(vec![0.1; n]);
    let result = blas().sdot(&x, &y).unwrap();
    // 100 * 0.01 = 1.0, but floating point...
    assert!((result - 1.0).abs() < 0.01);
}

#[test]
fn test_snrm2_precision() {
    // [1, 1, 1, ..., 1] with n elements -> norm = sqrt(n)
    let n = 100;
    let x = Vector::new(vec![1.0; n]);
    let expected = (n as f32).sqrt();
    assert!((blas().snrm2(&x) - expected).abs() < 0.01);
}

// =========================================================================
// GEMM edge cases
// =========================================================================

#[test]
fn test_sgemm_zero_matrices() {
    let a = Matrix::zeros(2, 2);
    let b = Matrix::zeros(2, 2);
    let c = Matrix::zeros(2, 2);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert!(result.data().iter().all(|&v| v == 0.0));
}

#[test]
fn test_sgemm_alpha_zero_beta_zero() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2);
    let c = Matrix::new(vec![100.0, 200.0, 300.0, 400.0], 2, 2);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 0.0, &a, &b, 0.0, &c)
        .unwrap();
    assert!(result.data().iter().all(|&v| v == 0.0));
}

#[test]
fn test_sgemm_alpha_zero_beta_one() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2);
    let c = Matrix::new(vec![100.0, 200.0, 300.0, 400.0], 2, 2);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 0.0, &a, &b, 1.0, &c)
        .unwrap();
    assert_eq!(result.data(), c.data());
}

#[test]
fn test_sgemm_rectangular_tall_times_wide() {
    // (3x1) * (1x3) = (3x3)
    let a = Matrix::new(vec![1.0, 2.0, 3.0], 3, 1);
    let b = Matrix::new(vec![4.0, 5.0, 6.0], 1, 3);
    let c = Matrix::zeros(3, 3);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(
        result.data(),
        &[4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 12.0, 15.0, 18.0]
    );
}

#[test]
fn test_sgemm_wide_times_tall() {
    // (1x3) * (3x1) = (1x1)
    let a = Matrix::new(vec![1.0, 2.0, 3.0], 1, 3);
    let b = Matrix::new(vec![4.0, 5.0, 6.0], 3, 1);
    let c = Matrix::zeros(1, 1);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, 1.0, &a, &b, 0.0, &c)
        .unwrap();
    assert_eq!(result.data(), &[32.0]); // 1*4 + 2*5 + 3*6
}

// =========================================================================
// SSYMM edge cases
// =========================================================================

#[test]
fn test_ssymm_identity() {
    let a = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let b = Matrix::new(vec![3.0, 4.0, 5.0, 6.0], 2, 2);
    let c = Matrix::zeros(2, 2);
    let result = blas().ssymm(Side::Left, 1.0, &a, &b, 0.0, &c).unwrap();
    assert_eq!(result.data(), b.data());
}

#[test]
fn test_ssymm_3x3() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 2.0, 4.0, 5.0, 3.0, 5.0, 6.0], 3, 3);
    let b = Matrix::new(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0], 3, 3);
    let c = Matrix::zeros(3, 3);
    let result = blas().ssymm(Side::Left, 1.0, &a, &b, 0.0, &c).unwrap();
    assert_eq!(result.data(), a.data());
}

// =========================================================================
// Batched GEMM edge cases
// =========================================================================

#[test]
fn test_sgemm_batched_three_items() {
    let as_vec: Vec<Matrix> = (0..3)
        .map(|_| Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2))
        .collect();
    let bs: Vec<Matrix> = vec![
        Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2),
        Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2),
        Matrix::new(vec![9.0, 10.0, 11.0, 12.0], 2, 2),
    ];
    let cs: Vec<Matrix> = (0..3).map(|_| Matrix::zeros(2, 2)).collect();
    let results = blas()
        .sgemm_batched(
            Transpose::NoTrans,
            Transpose::NoTrans,
            1.0,
            &as_vec,
            &bs,
            0.0,
            &cs,
        )
        .unwrap();
    assert_eq!(results.len(), 3);
    // Identity * B = B for each
    assert_eq!(results[0].data(), bs[0].data());
    assert_eq!(results[1].data(), bs[1].data());
    assert_eq!(results[2].data(), bs[2].data());
}

// =========================================================================
// ML extension edge cases
// =========================================================================

#[test]
fn test_relu_1x1() {
    let x = Matrix::new(vec![-5.0], 1, 1);
    assert_eq!(blas().relu(&x).data(), &[0.0]);
}

#[test]
fn test_relu_1x1_positive() {
    let x = Matrix::new(vec![5.0], 1, 1);
    assert_eq!(blas().relu(&x).data(), &[5.0]);
}

#[test]
fn test_gelu_batch() {
    let x = Matrix::new(vec![-2.0, -1.0, 0.0, 1.0, 2.0, 3.0], 2, 3);
    let result = blas().gelu(&x);
    assert_eq!(result.rows(), 2);
    assert_eq!(result.cols(), 3);
    // GELU(0) ~= 0
    assert!(result.data()[2].abs() < 1e-5);
}

#[test]
fn test_sigmoid_batch() {
    let x = Matrix::new(vec![-10.0, 0.0, 10.0, -10.0, 0.0, 10.0], 2, 3);
    let result = blas().sigmoid(&x);
    assert_eq!(result.rows(), 2);
    assert_eq!(result.cols(), 3);
    // sigmoid(0) = 0.5
    assert!((result.data()[1] - 0.5).abs() < 1e-5);
    assert!((result.data()[4] - 0.5).abs() < 1e-5);
}

#[test]
fn test_tanh_batch() {
    let x = Matrix::new(vec![-10.0, 0.0, 10.0, -10.0, 0.0, 10.0], 2, 3);
    let result = blas().tanh_activation(&x);
    assert!((result.data()[1]).abs() < 1e-5); // tanh(0) = 0
    assert!((result.data()[4]).abs() < 1e-5);
}

#[test]
fn test_softmax_single_element() {
    let x = Matrix::new(vec![42.0], 1, 1);
    let result = blas().softmax(&x, -1);
    assert!((result.data()[0] - 1.0).abs() < 1e-5);
}

#[test]
fn test_softmax_two_elements_equal() {
    let x = Matrix::new(vec![0.0, 0.0], 1, 2);
    let result = blas().softmax(&x, -1);
    assert!((result.data()[0] - 0.5).abs() < 1e-5);
    assert!((result.data()[1] - 0.5).abs() < 1e-5);
}

#[test]
fn test_softmax_dominated() {
    // One very large element should get ~1.0
    let x = Matrix::new(vec![0.0, 100.0], 1, 2);
    let result = blas().softmax(&x, -1);
    assert!(result.data()[1] > 0.99);
    assert!(result.data()[0] < 0.01);
}

#[test]
fn test_layer_norm_single_feature() {
    // Single feature per sample: normalized = 0, result = beta
    let x = Matrix::new(vec![5.0, 10.0], 2, 1);
    let gamma = Vector::new(vec![1.0]);
    let beta = Vector::new(vec![3.0]);
    let result = blas().layer_norm(&x, &gamma, &beta, 1e-5).unwrap();
    // With 1 feature, variance = 0, so x_hat ~= 0, result ~= beta
    for &v in result.data() {
        assert!((v - 3.0).abs() < 1e-2);
    }
}

#[test]
fn test_conv2d_same_padding() {
    // 3x3 input, 3x3 kernel, stride=1, padding=1 -> 3x3 output
    let input = Matrix::new(vec![1.0; 9], 3, 3);
    let weight = Matrix::new(vec![1.0; 9], 3, 3);
    let result = blas().conv2d(&input, &weight, None, 1, 1).unwrap();
    assert_eq!(result.rows(), 3);
    assert_eq!(result.cols(), 3);
}

#[test]
fn test_attention_identical_qkv() {
    // Q = K = V: self-attention with identical queries
    let m = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let result = blas().attention(&m, &m, &m, None, None).unwrap();
    assert_eq!(result.rows(), 2);
    assert_eq!(result.cols(), 2);
    assert!(result.data().iter().all(|v| v.is_finite()));
}

// =========================================================================
// SGEMV edge cases
// =========================================================================

#[test]
fn test_sgemv_1x1() {
    let a = Matrix::new(vec![3.0], 1, 1);
    let x = Vector::new(vec![4.0]);
    let y = Vector::new(vec![0.0]);
    let result = blas()
        .sgemv(Transpose::NoTrans, 1.0, &a, &x, 0.0, &y)
        .unwrap();
    assert_eq!(result.data(), &[12.0]);
}

#[test]
fn test_sgemv_tall_matrix() {
    // (3x1) * (1,) = (3,)
    let a = Matrix::new(vec![1.0, 2.0, 3.0], 3, 1);
    let x = Vector::new(vec![2.0]);
    let y = Vector::zeros(3);
    let result = blas()
        .sgemv(Transpose::NoTrans, 1.0, &a, &x, 0.0, &y)
        .unwrap();
    assert_eq!(result.data(), &[2.0, 4.0, 6.0]);
}

#[test]
fn test_sgemv_wide_matrix() {
    // (1x3) * (3,) = (1,)
    let a = Matrix::new(vec![1.0, 2.0, 3.0], 1, 3);
    let x = Vector::new(vec![1.0, 1.0, 1.0]);
    let y = Vector::zeros(1);
    let result = blas()
        .sgemv(Transpose::NoTrans, 1.0, &a, &x, 0.0, &y)
        .unwrap();
    assert_eq!(result.data(), &[6.0]);
}

// =========================================================================
// SGER edge cases
// =========================================================================

#[test]
fn test_sger_1x1() {
    let x = Vector::new(vec![3.0]);
    let y = Vector::new(vec![4.0]);
    let a = Matrix::zeros(1, 1);
    let result = blas().sger(1.0, &x, &y, &a).unwrap();
    assert_eq!(result.data(), &[12.0]);
}

#[test]
fn test_sger_negative_alpha() {
    let x = Vector::new(vec![1.0, 2.0]);
    let y = Vector::new(vec![3.0, 4.0]);
    let a = Matrix::new(vec![10.0, 10.0, 10.0, 10.0], 2, 2);
    let result = blas().sger(-1.0, &x, &y, &a).unwrap();
    assert_eq!(result.data(), &[7.0, 6.0, 4.0, 2.0]);
}

// =========================================================================
// Additional activation tests
// =========================================================================

#[test]
fn test_sigmoid_one() {
    let x = Matrix::new(vec![1.0], 1, 1);
    let result = blas().sigmoid(&x);
    let expected = 1.0 / (1.0 + (-1.0_f32).exp());
    assert!((result.data()[0] - expected).abs() < 1e-5);
}

#[test]
fn test_tanh_one() {
    let x = Matrix::new(vec![1.0], 1, 1);
    let result = blas().tanh_activation(&x);
    assert!((result.data()[0] - 1.0_f32.tanh()).abs() < 1e-5);
}

#[test]
fn test_softmax_negative_values() {
    let x = Matrix::new(vec![-1.0, -2.0, -3.0], 1, 3);
    let result = blas().softmax(&x, -1);
    let sum: f32 = result.data().iter().sum();
    assert!((sum - 1.0).abs() < 1e-5);
    // All should be positive
    assert!(result.data().iter().all(|&v| v > 0.0));
}

#[test]
fn test_conv2d_no_bias() {
    let input = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let weight = Matrix::new(vec![1.0], 1, 1);
    let result = blas().conv2d(&input, &weight, None, 1, 0).unwrap();
    assert_eq!(result.data(), &[1.0, 2.0, 3.0, 4.0]);
}

#[test]
#[test]
fn test_sgemm_negative_alpha_negative_beta() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2);
    let c = Matrix::new(vec![10.0, 10.0, 10.0, 10.0], 2, 2);
    let result = blas()
        .sgemm(Transpose::NoTrans, Transpose::NoTrans, -1.0, &a, &b, -1.0, &c)
        .unwrap();
    // -1 * A*B + -1 * C = -[19,22,43,50] + -[10,10,10,10] = [-29,-32,-53,-60]
    assert_eq!(result.data(), &[-29.0, -32.0, -53.0, -60.0]);
}

#[test]
fn test_batch_norm_single_sample() {
    // With 1 sample, training batch norm has var=0
    // Result should still be finite (eps prevents divide by zero)
    let x = Matrix::new(vec![1.0, 2.0, 3.0], 1, 3);
    let gamma = Vector::new(vec![1.0, 1.0, 1.0]);
    let beta = Vector::new(vec![0.0, 0.0, 0.0]);
    let rm = Vector::zeros(3);
    let rv = Vector::new(vec![1.0, 1.0, 1.0]);
    let result = blas()
        .batch_norm(&x, &gamma, &beta, &rm, &rv, 1e-5, true)
        .unwrap();
    assert!(result.data().iter().all(|v| v.is_finite()));
}
