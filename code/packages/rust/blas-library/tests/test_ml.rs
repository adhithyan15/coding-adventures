//! Tests for ML BLAS extensions: activations, normalization, convolution, attention.

use blas_library::traits::MlBlasBackend;
use blas_library::{CpuBlas, Matrix, Vector};

fn blas() -> CpuBlas {
    CpuBlas
}

// =========================================================================
// ReLU tests
// =========================================================================

#[test]
fn test_relu_positive() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let result = blas().relu(&x);
    assert_eq!(result.data(), &[1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn test_relu_negative() {
    let x = Matrix::new(vec![-1.0, -2.0, -3.0, -4.0], 2, 2);
    let result = blas().relu(&x);
    assert_eq!(result.data(), &[0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn test_relu_mixed() {
    let x = Matrix::new(vec![-1.0, 2.0, -3.0, 4.0], 2, 2);
    let result = blas().relu(&x);
    assert_eq!(result.data(), &[0.0, 2.0, 0.0, 4.0]);
}

#[test]
fn test_relu_zero() {
    let x = Matrix::new(vec![0.0, 0.0], 1, 2);
    let result = blas().relu(&x);
    assert_eq!(result.data(), &[0.0, 0.0]);
}

#[test]
fn test_relu_preserves_shape() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let result = blas().relu(&x);
    assert_eq!(result.rows(), 2);
    assert_eq!(result.cols(), 3);
}

#[test]
fn test_relu_large_negative() {
    let x = Matrix::new(vec![-1000.0, 1000.0], 1, 2);
    let result = blas().relu(&x);
    assert_eq!(result.data(), &[0.0, 1000.0]);
}

// =========================================================================
// GELU tests
// =========================================================================

#[test]
fn test_gelu_zero() {
    let x = Matrix::new(vec![0.0], 1, 1);
    let result = blas().gelu(&x);
    assert!(result.data()[0].abs() < 1e-6);
}

#[test]
fn test_gelu_positive() {
    let x = Matrix::new(vec![1.0], 1, 1);
    let result = blas().gelu(&x);
    // GELU(1) ~= 0.8412
    assert!((result.data()[0] - 0.8412).abs() < 0.01);
}

#[test]
fn test_gelu_negative() {
    let x = Matrix::new(vec![-1.0], 1, 1);
    let result = blas().gelu(&x);
    // GELU(-1) ~= -0.1588
    assert!((result.data()[0] - (-0.1588)).abs() < 0.01);
}

#[test]
fn test_gelu_large_positive() {
    let x = Matrix::new(vec![5.0], 1, 1);
    let result = blas().gelu(&x);
    // For large positive, GELU(x) ~= x
    assert!((result.data()[0] - 5.0).abs() < 0.01);
}

#[test]
fn test_gelu_large_negative() {
    let x = Matrix::new(vec![-5.0], 1, 1);
    let result = blas().gelu(&x);
    // For large negative, GELU(x) ~= 0
    assert!(result.data()[0].abs() < 0.01);
}

#[test]
fn test_gelu_preserves_shape() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let result = blas().gelu(&x);
    assert_eq!(result.rows(), 2);
    assert_eq!(result.cols(), 3);
}

// =========================================================================
// Sigmoid tests
// =========================================================================

#[test]
fn test_sigmoid_zero() {
    let x = Matrix::new(vec![0.0], 1, 1);
    let result = blas().sigmoid(&x);
    assert!((result.data()[0] - 0.5).abs() < 1e-6);
}

#[test]
fn test_sigmoid_large_positive() {
    let x = Matrix::new(vec![100.0], 1, 1);
    let result = blas().sigmoid(&x);
    assert!((result.data()[0] - 1.0).abs() < 1e-6);
}

#[test]
fn test_sigmoid_large_negative() {
    let x = Matrix::new(vec![-100.0], 1, 1);
    let result = blas().sigmoid(&x);
    assert!(result.data()[0].abs() < 1e-6);
}

#[test]
fn test_sigmoid_range() {
    // All sigmoid outputs should be in (0, 1)
    let x = Matrix::new(vec![-5.0, -1.0, 0.0, 1.0, 5.0], 1, 5);
    let result = blas().sigmoid(&x);
    for &v in result.data() {
        assert!(v > 0.0 && v < 1.0);
    }
}

#[test]
fn test_sigmoid_symmetry() {
    // sigmoid(x) + sigmoid(-x) = 1
    let x = Matrix::new(vec![2.0], 1, 1);
    let neg_x = Matrix::new(vec![-2.0], 1, 1);
    let result_pos = blas().sigmoid(&x);
    let result_neg = blas().sigmoid(&neg_x);
    assert!((result_pos.data()[0] + result_neg.data()[0] - 1.0).abs() < 1e-5);
}

#[test]
fn test_sigmoid_preserves_shape() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3, 2);
    let result = blas().sigmoid(&x);
    assert_eq!(result.rows(), 3);
    assert_eq!(result.cols(), 2);
}

// =========================================================================
// Tanh activation tests
// =========================================================================

#[test]
fn test_tanh_zero() {
    let x = Matrix::new(vec![0.0], 1, 1);
    let result = blas().tanh_activation(&x);
    assert!(result.data()[0].abs() < 1e-6);
}

#[test]
fn test_tanh_positive() {
    let x = Matrix::new(vec![1.0], 1, 1);
    let result = blas().tanh_activation(&x);
    assert!((result.data()[0] - 1.0_f32.tanh()).abs() < 1e-6);
}

#[test]
fn test_tanh_range() {
    let x = Matrix::new(vec![-5.0, -1.0, 0.0, 1.0, 5.0], 1, 5);
    let result = blas().tanh_activation(&x);
    for &v in result.data() {
        assert!(v > -1.0 && v < 1.0);
    }
}

#[test]
fn test_tanh_antisymmetry() {
    // tanh(-x) = -tanh(x)
    let x = Matrix::new(vec![2.0], 1, 1);
    let neg_x = Matrix::new(vec![-2.0], 1, 1);
    let result_pos = blas().tanh_activation(&x);
    let result_neg = blas().tanh_activation(&neg_x);
    assert!((result_pos.data()[0] + result_neg.data()[0]).abs() < 1e-5);
}

// =========================================================================
// Softmax tests
// =========================================================================

#[test]
fn test_softmax_row() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0], 1, 3);
    let result = blas().softmax(&x, -1);
    let sum: f32 = result.data().iter().sum();
    assert!((sum - 1.0).abs() < 1e-5);
    // Values should be increasing
    assert!(result.data()[0] < result.data()[1]);
    assert!(result.data()[1] < result.data()[2]);
}

#[test]
fn test_softmax_uniform() {
    let x = Matrix::new(vec![1.0, 1.0, 1.0], 1, 3);
    let result = blas().softmax(&x, -1);
    for &v in result.data() {
        assert!((v - 1.0 / 3.0).abs() < 1e-5);
    }
}

#[test]
fn test_softmax_sum_to_one() {
    let x = Matrix::new(vec![0.5, 1.5, 2.5, 3.5, 4.5, 5.5], 2, 3);
    let result = blas().softmax(&x, -1);
    // Each row should sum to 1
    let row1_sum: f32 = result.data()[0..3].iter().sum();
    let row2_sum: f32 = result.data()[3..6].iter().sum();
    assert!((row1_sum - 1.0).abs() < 1e-5);
    assert!((row2_sum - 1.0).abs() < 1e-5);
}

#[test]
fn test_softmax_axis_0() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let result = blas().softmax(&x, 0);
    // Each column should sum to 1
    let col1_sum = result.data()[0] + result.data()[2];
    let col2_sum = result.data()[1] + result.data()[3];
    assert!((col1_sum - 1.0).abs() < 1e-5);
    assert!((col2_sum - 1.0).abs() < 1e-5);
}

#[test]
fn test_softmax_numerical_stability() {
    // Large values should not cause overflow
    let x = Matrix::new(vec![1000.0, 1001.0, 1002.0], 1, 3);
    let result = blas().softmax(&x, -1);
    let sum: f32 = result.data().iter().sum();
    assert!((sum - 1.0).abs() < 1e-5);
    assert!(result.data().iter().all(|&v| v.is_finite()));
}

#[test]
fn test_softmax_all_positive() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 1, 4);
    let result = blas().softmax(&x, -1);
    for &v in result.data() {
        assert!(v > 0.0 && v < 1.0);
    }
}

#[test]
fn test_softmax_preserves_shape() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let result = blas().softmax(&x, -1);
    assert_eq!(result.rows(), 2);
    assert_eq!(result.cols(), 3);
}

// =========================================================================
// Layer normalization tests
// =========================================================================

#[test]
fn test_layer_norm_basic() {
    // With gamma=1 and beta=0, layer norm should produce zero-mean, unit-variance
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 1, 4);
    let gamma = Vector::new(vec![1.0, 1.0, 1.0, 1.0]);
    let beta = Vector::new(vec![0.0, 0.0, 0.0, 0.0]);
    let result = blas().layer_norm(&x, &gamma, &beta, 1e-5).unwrap();

    // Mean should be ~0
    let mean: f32 = result.data().iter().sum::<f32>() / 4.0;
    assert!(mean.abs() < 1e-4);
}

#[test]
fn test_layer_norm_with_gamma_beta() {
    let x = Matrix::new(vec![0.0, 0.0, 0.0, 0.0], 1, 4);
    let gamma = Vector::new(vec![2.0, 2.0, 2.0, 2.0]);
    let beta = Vector::new(vec![1.0, 1.0, 1.0, 1.0]);
    let result = blas().layer_norm(&x, &gamma, &beta, 1e-5).unwrap();
    // All zeros -> normalized = 0 -> gamma*0 + beta = beta
    for &v in result.data() {
        assert!((v - 1.0).abs() < 1e-3);
    }
}

#[test]
fn test_layer_norm_multi_row() {
    let x = Matrix::new(vec![1.0, 3.0, 5.0, 7.0], 2, 2);
    let gamma = Vector::new(vec![1.0, 1.0]);
    let beta = Vector::new(vec![0.0, 0.0]);
    let result = blas().layer_norm(&x, &gamma, &beta, 1e-5).unwrap();
    assert_eq!(result.rows(), 2);
    assert_eq!(result.cols(), 2);
    // Each row should have mean ~0
    let row1_mean = (result.data()[0] + result.data()[1]) / 2.0;
    let row2_mean = (result.data()[2] + result.data()[3]) / 2.0;
    assert!(row1_mean.abs() < 1e-4);
    assert!(row2_mean.abs() < 1e-4);
}

#[test]
fn test_layer_norm_gamma_mismatch() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let gamma = Vector::new(vec![1.0, 1.0, 1.0]);
    let beta = Vector::new(vec![0.0, 0.0]);
    assert!(blas().layer_norm(&x, &gamma, &beta, 1e-5).is_err());
}

#[test]
fn test_layer_norm_beta_mismatch() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let gamma = Vector::new(vec![1.0, 1.0]);
    let beta = Vector::new(vec![0.0, 0.0, 0.0]);
    assert!(blas().layer_norm(&x, &gamma, &beta, 1e-5).is_err());
}

#[test]
fn test_layer_norm_preserves_shape() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let gamma = Vector::new(vec![1.0, 1.0, 1.0]);
    let beta = Vector::new(vec![0.0, 0.0, 0.0]);
    let result = blas().layer_norm(&x, &gamma, &beta, 1e-5).unwrap();
    assert_eq!(result.rows(), 2);
    assert_eq!(result.cols(), 3);
}

// =========================================================================
// Batch normalization tests
// =========================================================================

#[test]
fn test_batch_norm_training() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let gamma = Vector::new(vec![1.0, 1.0]);
    let beta = Vector::new(vec![0.0, 0.0]);
    let running_mean = Vector::zeros(2);
    let running_var = Vector::new(vec![1.0, 1.0]);
    let result = blas()
        .batch_norm(&x, &gamma, &beta, &running_mean, &running_var, 1e-5, true)
        .unwrap();
    // Each column should have mean ~0
    let col1_mean = (result.data()[0] + result.data()[2]) / 2.0;
    let col2_mean = (result.data()[1] + result.data()[3]) / 2.0;
    assert!(col1_mean.abs() < 1e-4);
    assert!(col2_mean.abs() < 1e-4);
}

#[test]
fn test_batch_norm_inference() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let gamma = Vector::new(vec![1.0, 1.0]);
    let beta = Vector::new(vec![0.0, 0.0]);
    let running_mean = Vector::new(vec![2.0, 3.0]);
    let running_var = Vector::new(vec![1.0, 1.0]);
    let result = blas()
        .batch_norm(&x, &gamma, &beta, &running_mean, &running_var, 1e-5, false)
        .unwrap();
    // Inference uses running stats: (x - mean) / sqrt(var + eps)
    assert!((result.data()[0] - (-1.0)).abs() < 1e-3); // (1-2)/sqrt(1)
    assert!((result.data()[1] - (-1.0)).abs() < 1e-3); // (2-3)/sqrt(1)
}

#[test]
fn test_batch_norm_gamma_mismatch() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let gamma = Vector::new(vec![1.0, 1.0, 1.0]);
    let beta = Vector::new(vec![0.0, 0.0]);
    let rm = Vector::zeros(2);
    let rv = Vector::new(vec![1.0, 1.0]);
    assert!(blas()
        .batch_norm(&x, &gamma, &beta, &rm, &rv, 1e-5, true)
        .is_err());
}

#[test]
fn test_batch_norm_beta_mismatch() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let gamma = Vector::new(vec![1.0, 1.0]);
    let beta = Vector::new(vec![0.0]);
    let rm = Vector::zeros(2);
    let rv = Vector::new(vec![1.0, 1.0]);
    assert!(blas()
        .batch_norm(&x, &gamma, &beta, &rm, &rv, 1e-5, true)
        .is_err());
}

#[test]
fn test_batch_norm_preserves_shape() {
    let x = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3, 2);
    let gamma = Vector::new(vec![1.0, 1.0]);
    let beta = Vector::new(vec![0.0, 0.0]);
    let rm = Vector::zeros(2);
    let rv = Vector::new(vec![1.0, 1.0]);
    let result = blas()
        .batch_norm(&x, &gamma, &beta, &rm, &rv, 1e-5, true)
        .unwrap();
    assert_eq!(result.rows(), 3);
    assert_eq!(result.cols(), 2);
}

#[test]
fn test_batch_norm_with_gamma_beta() {
    // With constant input, batch norm in training gives 0, then gamma*0 + beta = beta
    let x = Matrix::new(vec![5.0, 5.0, 5.0, 5.0], 2, 2);
    let gamma = Vector::new(vec![2.0, 3.0]);
    let beta = Vector::new(vec![1.0, 2.0]);
    let rm = Vector::zeros(2);
    let rv = Vector::new(vec![1.0, 1.0]);
    let result = blas()
        .batch_norm(&x, &gamma, &beta, &rm, &rv, 1e-5, true)
        .unwrap();
    // All values in each column are the same, so normalized = 0
    // result = gamma * 0 + beta = beta
    for i in 0..2 {
        assert!((result.data()[i * 2] - 1.0).abs() < 1e-3);
        assert!((result.data()[i * 2 + 1] - 2.0).abs() < 1e-3);
    }
}

// =========================================================================
// Conv2D tests
// =========================================================================

#[test]
fn test_conv2d_basic() {
    // 3x3 input, 2x2 kernel, stride=1, padding=0
    let input = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let weight = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let result = blas().conv2d(&input, &weight, None, 1, 0).unwrap();
    assert_eq!(result.rows(), 2);
    assert_eq!(result.cols(), 2);
    // [1*1+2*0+4*0+5*1, 2*1+3*0+5*0+6*1, ...] = [6, 8, 12, 14]
    assert_eq!(result.data(), &[6.0, 8.0, 12.0, 14.0]);
}

#[test]
fn test_conv2d_with_bias() {
    let input = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let weight = Matrix::new(vec![1.0], 1, 1);
    let bias = Vector::new(vec![10.0]);
    let result = blas()
        .conv2d(&input, &weight, Some(&bias), 1, 0)
        .unwrap();
    assert_eq!(result.data(), &[11.0, 12.0, 13.0, 14.0]);
}

#[test]
fn test_conv2d_with_padding() {
    let input = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let weight = Matrix::new(vec![1.0, 0.0, 0.0, 0.0], 2, 2);
    let result = blas().conv2d(&input, &weight, None, 1, 1).unwrap();
    // With padding=1, padded input is 4x4, output is 3x3
    assert_eq!(result.rows(), 3);
    assert_eq!(result.cols(), 3);
}

#[test]
fn test_conv2d_stride_2() {
    let input = Matrix::new(
        vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
            16.0,
        ],
        4,
        4,
    );
    let weight = Matrix::new(vec![1.0, 0.0, 0.0, 0.0], 2, 2);
    let result = blas().conv2d(&input, &weight, None, 2, 0).unwrap();
    assert_eq!(result.rows(), 2);
    assert_eq!(result.cols(), 2);
}

#[test]
fn test_conv2d_identity_kernel() {
    // 1x1 kernel = identity
    let input = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let weight = Matrix::new(vec![1.0], 1, 1);
    let result = blas().conv2d(&input, &weight, None, 1, 0).unwrap();
    assert_eq!(result.data(), input.data());
}

#[test]
fn test_conv2d_preserves_correct_shape() {
    // 5x5 input, 3x3 kernel, stride 1, padding 0 -> 3x3 output
    let input = Matrix::zeros(5, 5);
    let weight = Matrix::zeros(3, 3);
    let result = blas().conv2d(&input, &weight, None, 1, 0).unwrap();
    assert_eq!(result.rows(), 3);
    assert_eq!(result.cols(), 3);
}

// =========================================================================
// Attention tests
// =========================================================================

#[test]
fn test_attention_basic() {
    // Simple 2x2 Q, K, V
    let q = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let k = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let v = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let result = blas().attention(&q, &k, &v, None, None).unwrap();
    assert_eq!(result.rows(), 2);
    assert_eq!(result.cols(), 2);
}

#[test]
fn test_attention_output_shape() {
    // Q: 3x4, K: 3x4, V: 3x4 -> output: 3x4
    let q = Matrix::zeros(3, 4);
    let k = Matrix::zeros(3, 4);
    let v = Matrix::zeros(3, 4);
    let result = blas().attention(&q, &k, &v, None, None).unwrap();
    assert_eq!(result.rows(), 3);
    assert_eq!(result.cols(), 4);
}

#[test]
fn test_attention_with_scale() {
    let q = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let k = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let v = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let result = blas().attention(&q, &k, &v, None, Some(1.0)).unwrap();
    assert_eq!(result.rows(), 2);
    assert_eq!(result.cols(), 2);
    // All values should be finite
    assert!(result.data().iter().all(|v| v.is_finite()));
}

#[test]
fn test_attention_with_mask() {
    let q = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let k = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let v = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    // Causal mask: -inf for upper triangle
    let mask = Matrix::new(vec![0.0, -1e9, 0.0, 0.0], 2, 2);
    let result = blas().attention(&q, &k, &v, Some(&mask), None).unwrap();
    assert_eq!(result.rows(), 2);
    assert!(result.data().iter().all(|v| v.is_finite()));
}

#[test]
fn test_attention_weights_sum_to_one() {
    // The internal softmax should produce weights that sum to 1
    // We can verify the output is a weighted combination of V rows
    let q = Matrix::new(vec![1.0, 0.0], 1, 2);
    let k = Matrix::new(vec![1.0, 0.0, 0.0, 1.0], 2, 2);
    let v = Matrix::new(vec![10.0, 20.0, 30.0, 40.0], 2, 2);
    let result = blas().attention(&q, &k, &v, None, None).unwrap();
    assert_eq!(result.rows(), 1);
    assert_eq!(result.cols(), 2);
    // Output should be between min and max of V values
    for &val in result.data() {
        assert!(val >= 10.0 && val <= 40.0);
    }
}

#[test]
fn test_attention_single_token() {
    let q = Matrix::new(vec![1.0, 2.0], 1, 2);
    let k = Matrix::new(vec![1.0, 2.0], 1, 2);
    let v = Matrix::new(vec![3.0, 4.0], 1, 2);
    let result = blas().attention(&q, &k, &v, None, None).unwrap();
    // With single token, softmax([score]) = [1.0], so output = v
    assert!((result.data()[0] - 3.0).abs() < 1e-4);
    assert!((result.data()[1] - 4.0).abs() < 1e-4);
}
