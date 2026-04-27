// Integration tests for cas-matrix.
//
// Mirrors the Python reference tests in
// code/packages/python/cas-matrix/tests/.

use cas_matrix::{
    add_matrices, determinant, dimensions, dot, get_entry, identity_matrix, inverse, is_matrix,
    matrix, num_cols, num_rows, scalar_multiply, sub_matrices, trace, transpose, zero_matrix,
    MatrixError, MATRIX,
};
use symbolic_ir::{apply, int, sym, ADD, LIST, SUB};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a row of IRNode::Integer values.
fn irow(vals: &[i64]) -> Vec<symbolic_ir::IRNode> {
    vals.iter().map(|&v| int(v)).collect()
}

// ---------------------------------------------------------------------------
// Construction and shape
// ---------------------------------------------------------------------------

#[test]
fn matrix_2x2_is_matrix() {
    let m = matrix(vec![irow(&[1, 2]), irow(&[3, 4])]).unwrap();
    assert!(is_matrix(&m));
    // head should be Symbol("Matrix")
    if let symbolic_ir::IRNode::Apply(a) = &m {
        assert_eq!(a.head, sym(MATRIX));
        assert_eq!(a.args.len(), 2); // two rows
    } else {
        panic!("expected Apply");
    }
}

#[test]
fn matrix_rejects_jagged() {
    let err = matrix(vec![irow(&[1, 2]), irow(&[3])]);
    assert!(err.is_err());
    assert!(matches!(err, Err(MatrixError(_))));
}

#[test]
fn matrix_rejects_empty() {
    assert!(matrix(vec![]).is_err());
}

#[test]
fn dimensions_1x3() {
    let m = matrix(vec![irow(&[1, 2, 3])]).unwrap();
    let dims = dimensions(&m).unwrap();
    assert_eq!(dims, apply(sym(LIST), vec![int(1), int(3)]));
}

#[test]
fn num_rows_cols_2x3() {
    let m = matrix(vec![irow(&[1, 2, 3]), irow(&[4, 5, 6])]).unwrap();
    assert_eq!(num_rows(&m).unwrap(), 2);
    assert_eq!(num_cols(&m).unwrap(), 3);
}

#[test]
fn get_entry_one_based() {
    let m = matrix(vec![
        vec![sym("a"), sym("b")],
        vec![sym("c"), sym("d")],
    ])
    .unwrap();
    assert_eq!(get_entry(&m, 1, 1).unwrap(), sym("a"));
    assert_eq!(get_entry(&m, 2, 2).unwrap(), sym("d"));
    assert_eq!(get_entry(&m, 1, 2).unwrap(), sym("b"));
    assert_eq!(get_entry(&m, 2, 1).unwrap(), sym("c"));
}

#[test]
fn get_entry_out_of_range() {
    let m = matrix(vec![irow(&[1])]).unwrap();
    assert!(get_entry(&m, 2, 1).is_err());
    assert!(get_entry(&m, 1, 2).is_err());
}

#[test]
fn is_matrix_rejects_non_matrix() {
    assert!(!is_matrix(&int(5)));
    assert!(!is_matrix(&sym("x")));
    assert!(!is_matrix(&apply(sym(ADD), vec![int(1)])));
}

// ---------------------------------------------------------------------------
// identity_matrix / zero_matrix
// ---------------------------------------------------------------------------

#[test]
fn identity_3x3() {
    let eye = identity_matrix(3).unwrap();
    let expected = matrix(vec![
        irow(&[1, 0, 0]),
        irow(&[0, 1, 0]),
        irow(&[0, 0, 1]),
    ])
    .unwrap();
    assert_eq!(eye, expected);
}

#[test]
fn identity_1x1() {
    let eye = identity_matrix(1).unwrap();
    assert_eq!(get_entry(&eye, 1, 1).unwrap(), int(1));
}

#[test]
fn zero_matrix_shape() {
    let z = zero_matrix(2, 4).unwrap();
    assert_eq!(num_rows(&z).unwrap(), 2);
    assert_eq!(num_cols(&z).unwrap(), 4);
}

#[test]
fn zero_matrix_all_zeros() {
    let z = zero_matrix(2, 2).unwrap();
    assert_eq!(get_entry(&z, 1, 1).unwrap(), int(0));
    assert_eq!(get_entry(&z, 2, 2).unwrap(), int(0));
}

// ---------------------------------------------------------------------------
// Transpose
// ---------------------------------------------------------------------------

#[test]
fn transpose_square() {
    let m = matrix(vec![irow(&[1, 2]), irow(&[3, 4])]).unwrap();
    let t = transpose(&m).unwrap();
    let expected = matrix(vec![irow(&[1, 3]), irow(&[2, 4])]).unwrap();
    assert_eq!(t, expected);
}

#[test]
fn transpose_rectangular() {
    // [[1,2,3],[4,5,6]] → [[1,4],[2,5],[3,6]]
    let m = matrix(vec![irow(&[1, 2, 3]), irow(&[4, 5, 6])]).unwrap();
    let t = transpose(&m).unwrap();
    let expected = matrix(vec![irow(&[1, 4]), irow(&[2, 5]), irow(&[3, 6])]).unwrap();
    assert_eq!(t, expected);
}

#[test]
fn transpose_double_is_identity() {
    let m = matrix(vec![
        vec![sym("a"), sym("b")],
        vec![sym("c"), sym("d")],
    ])
    .unwrap();
    let tt = transpose(&transpose(&m).unwrap()).unwrap();
    assert_eq!(tt, m);
}

// ---------------------------------------------------------------------------
// Elementwise operations
// ---------------------------------------------------------------------------

#[test]
fn add_matrices_shape() {
    let a = matrix(vec![irow(&[1, 2]), irow(&[3, 4])]).unwrap();
    let b = matrix(vec![irow(&[5, 6]), irow(&[7, 8])]).unwrap();
    let c = add_matrices(&a, &b).unwrap();
    assert_eq!(num_rows(&c).unwrap(), 2);
    assert_eq!(num_cols(&c).unwrap(), 2);
}

#[test]
fn add_matrices_entry_is_add_node() {
    let a = matrix(vec![irow(&[1, 2])]).unwrap();
    let b = matrix(vec![irow(&[3, 4])]).unwrap();
    let c = add_matrices(&a, &b).unwrap();
    // Entry (1,1) should be Add(1, 3)
    let e = get_entry(&c, 1, 1).unwrap();
    assert_eq!(e, apply(sym(ADD), vec![int(1), int(3)]));
}

#[test]
fn add_shape_mismatch() {
    let a = matrix(vec![irow(&[1, 2])]).unwrap();
    let b = matrix(vec![irow(&[1])]).unwrap();
    assert!(add_matrices(&a, &b).is_err());
}

#[test]
fn sub_matrices_shape() {
    let a = matrix(vec![irow(&[1, 2])]).unwrap();
    let b = matrix(vec![irow(&[3, 4])]).unwrap();
    let c = sub_matrices(&a, &b).unwrap();
    assert_eq!(num_rows(&c).unwrap(), 1);
    assert_eq!(num_cols(&c).unwrap(), 2);
}

#[test]
fn sub_matrices_entry_is_sub_node() {
    let a = matrix(vec![irow(&[1, 2])]).unwrap();
    let b = matrix(vec![irow(&[3, 4])]).unwrap();
    let c = sub_matrices(&a, &b).unwrap();
    let e = get_entry(&c, 1, 1).unwrap();
    assert_eq!(e, apply(sym(SUB), vec![int(1), int(3)]));
}

#[test]
fn scalar_multiply_shape() {
    let m = matrix(vec![irow(&[1, 2])]).unwrap();
    let out = scalar_multiply(&int(3), &m).unwrap();
    assert_eq!(num_rows(&out).unwrap(), 1);
    assert_eq!(num_cols(&out).unwrap(), 2);
}

// ---------------------------------------------------------------------------
// Dot product
// ---------------------------------------------------------------------------

#[test]
fn dot_1x2_times_2x1_gives_1x1() {
    let a = matrix(vec![irow(&[1, 2])]).unwrap(); // 1×2
    let b = matrix(vec![irow(&[3]), irow(&[4])]).unwrap(); // 2×1
    let c = dot(&a, &b).unwrap();
    assert_eq!(num_rows(&c).unwrap(), 1);
    assert_eq!(num_cols(&c).unwrap(), 1);
}

#[test]
fn dot_incompatible_shapes() {
    let a = matrix(vec![irow(&[1, 2])]).unwrap(); // 1×2
    let b = matrix(vec![irow(&[3, 4])]).unwrap(); // 1×2 — incompatible
    assert!(dot(&a, &b).is_err());
}

#[test]
fn dot_3x3_with_identity_shape() {
    let a = matrix(vec![irow(&[1, 2, 3]), irow(&[4, 5, 6]), irow(&[7, 8, 9])]).unwrap();
    let eye = identity_matrix(3).unwrap();
    let c = dot(&a, &eye).unwrap();
    assert_eq!(num_rows(&c).unwrap(), 3);
    assert_eq!(num_cols(&c).unwrap(), 3);
}

// ---------------------------------------------------------------------------
// Trace
// ---------------------------------------------------------------------------

#[test]
fn trace_square() {
    let m = matrix(vec![irow(&[1, 2]), irow(&[3, 4])]).unwrap();
    let t = trace(&m).unwrap();
    // symbolic Add(1, 4)
    assert_eq!(t, apply(sym(ADD), vec![int(1), int(4)]));
}

#[test]
fn trace_1x1_returns_entry() {
    let m = matrix(vec![vec![sym("a")]]).unwrap();
    assert_eq!(trace(&m).unwrap(), sym("a"));
}

#[test]
fn trace_non_square_raises() {
    let m = matrix(vec![irow(&[1, 2, 3])]).unwrap();
    assert!(trace(&m).is_err());
}

// ---------------------------------------------------------------------------
// Determinant
// ---------------------------------------------------------------------------

#[test]
fn det_1x1() {
    let m = matrix(vec![vec![sym("a")]]).unwrap();
    assert_eq!(determinant(&m).unwrap(), sym("a"));
}

#[test]
fn det_2x2_returns_sub_expr() {
    let m = matrix(vec![
        vec![sym("a"), sym("b")],
        vec![sym("c"), sym("d")],
    ])
    .unwrap();
    let d = determinant(&m).unwrap();
    // Sub(Mul(a, d), Mul(b, c))
    if let symbolic_ir::IRNode::Apply(a) = &d {
        assert_eq!(a.head, sym(SUB));
    } else {
        panic!("expected Apply with Sub head, got {d:?}");
    }
}

#[test]
fn det_3x3_is_add_of_three_terms() {
    let m = matrix(vec![irow(&[1, 2, 3]), irow(&[4, 5, 6]), irow(&[7, 8, 9])]).unwrap();
    let d = determinant(&m).unwrap();
    if let symbolic_ir::IRNode::Apply(a) = &d {
        assert_eq!(a.head, sym(ADD));
        assert_eq!(a.args.len(), 3);
    } else {
        panic!("expected Add of 3 terms, got {d:?}");
    }
}

#[test]
fn det_non_square_raises() {
    let m = matrix(vec![irow(&[1, 2, 3])]).unwrap();
    assert!(determinant(&m).is_err());
}

// ---------------------------------------------------------------------------
// Inverse
// ---------------------------------------------------------------------------

#[test]
fn inverse_2x2_shape() {
    let m = matrix(vec![
        vec![sym("a"), sym("b")],
        vec![sym("c"), sym("d")],
    ])
    .unwrap();
    let inv = inverse(&m).unwrap();
    assert_eq!(num_rows(&inv).unwrap(), 2);
    assert_eq!(num_cols(&inv).unwrap(), 2);
}

#[test]
fn inverse_1x1_shape() {
    let m = matrix(vec![vec![sym("a")]]).unwrap();
    let inv = inverse(&m).unwrap();
    assert_eq!(num_rows(&inv).unwrap(), 1);
    assert_eq!(num_cols(&inv).unwrap(), 1);
}

#[test]
fn inverse_non_square_raises() {
    let m = matrix(vec![irow(&[1, 2, 3])]).unwrap();
    assert!(inverse(&m).is_err());
}
