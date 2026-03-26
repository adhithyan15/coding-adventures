//! Tests for BLAS Level 1 operations: vector-vector operations.
//!
//! Level 1 operations are O(n) -- they process each element once.

use blas_library::traits::BlasBackend;
use blas_library::{CpuBlas, Vector};

fn blas() -> CpuBlas {
    CpuBlas
}

// =========================================================================
// SAXPY tests: y = alpha * x + y
// =========================================================================

#[test]
fn test_saxpy_basic() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::new(vec![4.0, 5.0, 6.0]);
    let result = blas().saxpy(2.0, &x, &y).unwrap();
    assert_eq!(result.data(), &[6.0, 9.0, 12.0]);
}

#[test]
fn test_saxpy_zero_alpha() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::new(vec![4.0, 5.0, 6.0]);
    let result = blas().saxpy(0.0, &x, &y).unwrap();
    assert_eq!(result.data(), &[4.0, 5.0, 6.0]);
}

#[test]
fn test_saxpy_negative_alpha() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::new(vec![4.0, 5.0, 6.0]);
    let result = blas().saxpy(-1.0, &x, &y).unwrap();
    assert_eq!(result.data(), &[3.0, 3.0, 3.0]);
}

#[test]
fn test_saxpy_alpha_one() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::new(vec![10.0, 20.0, 30.0]);
    let result = blas().saxpy(1.0, &x, &y).unwrap();
    assert_eq!(result.data(), &[11.0, 22.0, 33.0]);
}

#[test]
fn test_saxpy_single_element() {
    let x = Vector::new(vec![5.0]);
    let y = Vector::new(vec![3.0]);
    let result = blas().saxpy(2.0, &x, &y).unwrap();
    assert_eq!(result.data(), &[13.0]);
}

#[test]
fn test_saxpy_dimension_mismatch() {
    let x = Vector::new(vec![1.0, 2.0]);
    let y = Vector::new(vec![1.0, 2.0, 3.0]);
    let result = blas().saxpy(1.0, &x, &y);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("dimension mismatch"));
}

#[test]
fn test_saxpy_zeros() {
    let x = Vector::zeros(4);
    let y = Vector::zeros(4);
    let result = blas().saxpy(1.0, &x, &y).unwrap();
    assert_eq!(result.data(), &[0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn test_saxpy_large_alpha() {
    let x = Vector::new(vec![1.0, 1.0]);
    let y = Vector::new(vec![0.0, 0.0]);
    let result = blas().saxpy(1000.0, &x, &y).unwrap();
    assert_eq!(result.data(), &[1000.0, 1000.0]);
}

// =========================================================================
// SDOT tests: result = sum(x_i * y_i)
// =========================================================================

#[test]
fn test_sdot_basic() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::new(vec![4.0, 5.0, 6.0]);
    let result = blas().sdot(&x, &y).unwrap();
    assert_eq!(result, 32.0); // 1*4 + 2*5 + 3*6
}

#[test]
fn test_sdot_orthogonal() {
    let x = Vector::new(vec![1.0, 0.0]);
    let y = Vector::new(vec![0.0, 1.0]);
    let result = blas().sdot(&x, &y).unwrap();
    assert_eq!(result, 0.0);
}

#[test]
fn test_sdot_parallel() {
    let x = Vector::new(vec![1.0, 0.0]);
    let y = Vector::new(vec![3.0, 0.0]);
    let result = blas().sdot(&x, &y).unwrap();
    assert_eq!(result, 3.0);
}

#[test]
fn test_sdot_single() {
    let x = Vector::new(vec![3.0]);
    let y = Vector::new(vec![4.0]);
    assert_eq!(blas().sdot(&x, &y).unwrap(), 12.0);
}

#[test]
fn test_sdot_negative() {
    let x = Vector::new(vec![-1.0, 2.0]);
    let y = Vector::new(vec![3.0, -4.0]);
    assert_eq!(blas().sdot(&x, &y).unwrap(), -11.0);
}

#[test]
fn test_sdot_dimension_mismatch() {
    let x = Vector::new(vec![1.0]);
    let y = Vector::new(vec![1.0, 2.0]);
    assert!(blas().sdot(&x, &y).is_err());
}

#[test]
fn test_sdot_zeros() {
    let x = Vector::zeros(5);
    let y = Vector::new(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    assert_eq!(blas().sdot(&x, &y).unwrap(), 0.0);
}

// =========================================================================
// SNRM2 tests: ||x||_2 = sqrt(sum(x_i^2))
// =========================================================================

#[test]
fn test_snrm2_basic() {
    let x = Vector::new(vec![3.0, 4.0]);
    assert_eq!(blas().snrm2(&x), 5.0); // 3-4-5 triangle
}

#[test]
fn test_snrm2_unit() {
    let x = Vector::new(vec![1.0, 0.0, 0.0]);
    assert_eq!(blas().snrm2(&x), 1.0);
}

#[test]
fn test_snrm2_zero() {
    let x = Vector::zeros(3);
    assert_eq!(blas().snrm2(&x), 0.0);
}

#[test]
fn test_snrm2_single() {
    let x = Vector::new(vec![5.0]);
    assert_eq!(blas().snrm2(&x), 5.0);
}

#[test]
fn test_snrm2_negative() {
    let x = Vector::new(vec![-3.0, -4.0]);
    assert_eq!(blas().snrm2(&x), 5.0);
}

#[test]
fn test_snrm2_all_ones() {
    let x = Vector::new(vec![1.0, 1.0, 1.0, 1.0]);
    assert!((blas().snrm2(&x) - 2.0).abs() < 1e-6);
}

// =========================================================================
// SSCAL tests: result = alpha * x
// =========================================================================

#[test]
fn test_sscal_basic() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let result = blas().sscal(2.0, &x);
    assert_eq!(result.data(), &[2.0, 4.0, 6.0]);
}

#[test]
fn test_sscal_zero() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let result = blas().sscal(0.0, &x);
    assert_eq!(result.data(), &[0.0, 0.0, 0.0]);
}

#[test]
fn test_sscal_one() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let result = blas().sscal(1.0, &x);
    assert_eq!(result.data(), &[1.0, 2.0, 3.0]);
}

#[test]
fn test_sscal_negative() {
    let x = Vector::new(vec![1.0, -2.0, 3.0]);
    let result = blas().sscal(-1.0, &x);
    assert_eq!(result.data(), &[-1.0, 2.0, -3.0]);
}

#[test]
fn test_sscal_fractional() {
    let x = Vector::new(vec![2.0, 4.0]);
    let result = blas().sscal(0.5, &x);
    assert_eq!(result.data(), &[1.0, 2.0]);
}

// =========================================================================
// SASUM tests: sum(|x_i|)
// =========================================================================

#[test]
fn test_sasum_basic() {
    let x = Vector::new(vec![1.0, -2.0, 3.0, -4.0]);
    assert_eq!(blas().sasum(&x), 10.0);
}

#[test]
fn test_sasum_all_positive() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    assert_eq!(blas().sasum(&x), 6.0);
}

#[test]
fn test_sasum_all_negative() {
    let x = Vector::new(vec![-1.0, -2.0, -3.0]);
    assert_eq!(blas().sasum(&x), 6.0);
}

#[test]
fn test_sasum_zeros() {
    let x = Vector::zeros(5);
    assert_eq!(blas().sasum(&x), 0.0);
}

#[test]
fn test_sasum_single() {
    let x = Vector::new(vec![-42.0]);
    assert_eq!(blas().sasum(&x), 42.0);
}

// =========================================================================
// ISAMAX tests: argmax(|x_i|)
// =========================================================================

#[test]
fn test_isamax_basic() {
    let x = Vector::new(vec![1.0, -5.0, 3.0, -2.0]);
    assert_eq!(blas().isamax(&x), 1); // |-5| is largest
}

#[test]
fn test_isamax_first_element() {
    let x = Vector::new(vec![10.0, 1.0, 2.0]);
    assert_eq!(blas().isamax(&x), 0);
}

#[test]
fn test_isamax_last_element() {
    let x = Vector::new(vec![1.0, 2.0, 10.0]);
    assert_eq!(blas().isamax(&x), 2);
}

#[test]
fn test_isamax_negative_max() {
    let x = Vector::new(vec![1.0, -10.0, 5.0]);
    assert_eq!(blas().isamax(&x), 1);
}

#[test]
fn test_isamax_single() {
    let x = Vector::new(vec![42.0]);
    assert_eq!(blas().isamax(&x), 0);
}

#[test]
fn test_isamax_empty() {
    let x = Vector::new(vec![]);
    assert_eq!(blas().isamax(&x), 0);
}

#[test]
fn test_isamax_all_equal() {
    let x = Vector::new(vec![5.0, 5.0, 5.0]);
    assert_eq!(blas().isamax(&x), 0); // First occurrence
}

// =========================================================================
// SCOPY tests: deep copy
// =========================================================================

#[test]
fn test_scopy_basic() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let result = blas().scopy(&x);
    assert_eq!(result.data(), x.data());
}

#[test]
fn test_scopy_empty() {
    let x = Vector::new(vec![]);
    let result = blas().scopy(&x);
    assert_eq!(result.size(), 0);
}

#[test]
fn test_scopy_preserves_values() {
    let x = Vector::new(vec![-1.0, 0.0, 1.0, 3.14]);
    let copy = blas().scopy(&x);
    assert_eq!(x, copy);
}

// =========================================================================
// SSWAP tests: exchange x and y
// =========================================================================

#[test]
fn test_sswap_basic() {
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::new(vec![4.0, 5.0, 6.0]);
    let (new_x, new_y) = blas().sswap(&x, &y).unwrap();
    assert_eq!(new_x.data(), &[4.0, 5.0, 6.0]);
    assert_eq!(new_y.data(), &[1.0, 2.0, 3.0]);
}

#[test]
fn test_sswap_same_values() {
    let x = Vector::new(vec![1.0, 1.0]);
    let y = Vector::new(vec![1.0, 1.0]);
    let (new_x, new_y) = blas().sswap(&x, &y).unwrap();
    assert_eq!(new_x.data(), &[1.0, 1.0]);
    assert_eq!(new_y.data(), &[1.0, 1.0]);
}

#[test]
fn test_sswap_dimension_mismatch() {
    let x = Vector::new(vec![1.0]);
    let y = Vector::new(vec![1.0, 2.0]);
    assert!(blas().sswap(&x, &y).is_err());
}

#[test]
fn test_sswap_single() {
    let x = Vector::new(vec![10.0]);
    let y = Vector::new(vec![20.0]);
    let (new_x, new_y) = blas().sswap(&x, &y).unwrap();
    assert_eq!(new_x.data(), &[20.0]);
    assert_eq!(new_y.data(), &[10.0]);
}

// =========================================================================
// Backend identity tests
// =========================================================================

#[test]
fn test_cpu_name() {
    assert_eq!(blas().name(), "cpu");
}

#[test]
fn test_cpu_device_name() {
    assert_eq!(blas().device_name(), "CPU (pure Rust)");
}
