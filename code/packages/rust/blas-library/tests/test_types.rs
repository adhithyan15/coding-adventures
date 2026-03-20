//! Tests for BLAS data types: Vector, Matrix, and enumerations.

use blas_library::{Matrix, StorageOrder, Vector};

// =========================================================================
// Vector tests
// =========================================================================

#[test]
fn test_vector_new() {
    let v = Vector::new(vec![1.0, 2.0, 3.0]);
    assert_eq!(v.size(), 3);
    assert_eq!(v.data(), &[1.0, 2.0, 3.0]);
}

#[test]
fn test_vector_zeros() {
    let v = Vector::zeros(5);
    assert_eq!(v.size(), 5);
    assert_eq!(v.data(), &[0.0, 0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn test_vector_empty() {
    let v = Vector::new(vec![]);
    assert_eq!(v.size(), 0);
    assert!(v.data().is_empty());
}

#[test]
fn test_vector_single_element() {
    let v = Vector::new(vec![42.0]);
    assert_eq!(v.size(), 1);
    assert_eq!(v.data()[0], 42.0);
}

#[test]
fn test_vector_clone() {
    let v1 = Vector::new(vec![1.0, 2.0]);
    let v2 = v1.clone();
    assert_eq!(v1.data(), v2.data());
}

#[test]
fn test_vector_equality() {
    let v1 = Vector::new(vec![1.0, 2.0, 3.0]);
    let v2 = Vector::new(vec![1.0, 2.0, 3.0]);
    assert_eq!(v1, v2);
}

#[test]
fn test_vector_inequality() {
    let v1 = Vector::new(vec![1.0, 2.0, 3.0]);
    let v2 = Vector::new(vec![1.0, 2.0, 4.0]);
    assert_ne!(v1, v2);
}

#[test]
fn test_vector_debug_format() {
    let v = Vector::new(vec![1.0]);
    let dbg = format!("{:?}", v);
    assert!(dbg.contains("Vector"));
}

#[test]
fn test_vector_negative_values() {
    let v = Vector::new(vec![-1.0, -2.5, 3.14]);
    assert_eq!(v.data()[0], -1.0);
    assert_eq!(v.data()[1], -2.5);
}

#[test]
fn test_vector_large() {
    let data: Vec<f32> = (0..1000).map(|i| i as f32).collect();
    let v = Vector::new(data.clone());
    assert_eq!(v.size(), 1000);
    assert_eq!(v.data()[999], 999.0);
}

// =========================================================================
// Matrix tests
// =========================================================================

#[test]
fn test_matrix_new() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    assert_eq!(m.rows(), 2);
    assert_eq!(m.cols(), 3);
    assert_eq!(m.data(), &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn test_matrix_zeros() {
    let m = Matrix::zeros(3, 4);
    assert_eq!(m.rows(), 3);
    assert_eq!(m.cols(), 4);
    assert!(m.data().iter().all(|&x| x == 0.0));
}

#[test]
fn test_matrix_default_order() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    assert_eq!(m.order(), StorageOrder::RowMajor);
}

#[test]
fn test_matrix_with_order_column_major() {
    let m = Matrix::with_order(
        vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0],
        2,
        3,
        StorageOrder::ColumnMajor,
    );
    assert_eq!(m.order(), StorageOrder::ColumnMajor);
    assert_eq!(m.rows(), 2);
    assert_eq!(m.cols(), 3);
}

#[test]
#[should_panic(expected = "Matrix data has")]
fn test_matrix_dimension_mismatch() {
    Matrix::new(vec![1.0, 2.0, 3.0], 2, 2);
}

#[test]
fn test_matrix_1x1() {
    let m = Matrix::new(vec![42.0], 1, 1);
    assert_eq!(m.rows(), 1);
    assert_eq!(m.cols(), 1);
    assert_eq!(m.data()[0], 42.0);
}

#[test]
fn test_matrix_clone() {
    let m1 = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let m2 = m1.clone();
    assert_eq!(m1, m2);
}

#[test]
fn test_matrix_equality() {
    let m1 = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let m2 = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    assert_eq!(m1, m2);
}

#[test]
fn test_matrix_inequality() {
    let m1 = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let m2 = Matrix::new(vec![1.0, 2.0, 3.0, 5.0], 2, 2);
    assert_ne!(m1, m2);
}

#[test]
fn test_matrix_debug_format() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
    let dbg = format!("{:?}", m);
    assert!(dbg.contains("Matrix"));
}

#[test]
fn test_matrix_row_vector() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0], 1, 3);
    assert_eq!(m.rows(), 1);
    assert_eq!(m.cols(), 3);
}

#[test]
fn test_matrix_column_vector() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0], 3, 1);
    assert_eq!(m.rows(), 3);
    assert_eq!(m.cols(), 1);
}

// =========================================================================
// Enumeration tests
// =========================================================================

#[test]
fn test_storage_order_default() {
    let order = StorageOrder::default();
    assert_eq!(order, StorageOrder::RowMajor);
}

#[test]
fn test_storage_order_variants() {
    assert_ne!(StorageOrder::RowMajor, StorageOrder::ColumnMajor);
}

#[test]
fn test_transpose_variants() {
    use blas_library::Transpose;
    assert_ne!(Transpose::NoTrans, Transpose::Trans);
}

#[test]
fn test_side_variants() {
    use blas_library::Side;
    assert_ne!(Side::Left, Side::Right);
}

#[test]
fn test_storage_order_clone() {
    let order = StorageOrder::RowMajor;
    let cloned = order;
    assert_eq!(order, cloned);
}

#[test]
fn test_transpose_clone() {
    use blas_library::Transpose;
    let t = Transpose::Trans;
    let cloned = t;
    assert_eq!(t, cloned);
}

#[test]
fn test_side_clone() {
    use blas_library::Side;
    let s = Side::Left;
    let cloned = s;
    assert_eq!(s, cloned);
}

#[test]
fn test_storage_order_debug() {
    let dbg = format!("{:?}", StorageOrder::RowMajor);
    assert_eq!(dbg, "RowMajor");
}
