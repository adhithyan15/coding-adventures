// Integration tests for cas-matrix.
//
// Mirrors the Python reference tests in
// code/packages/python/cas-matrix/tests/.

use cas_matrix::{
    add_matrices, columnspace, determinant, dimensions, dot, frobenius_norm, get_entry,
    identity_matrix, inverse, is_matrix, lu_decompose, matrix, norm, nullspace, num_cols, num_rows,
    rank, row_reduce, rowspace, scalar_multiply, sub_matrices, trace, transpose, zero_matrix,
    MatrixError, MATRIX,
};
use symbolic_ir::{apply, int, rat, sym, ADD, LIST, SQRT, SUB};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a row of IRNode::Integer values.
fn irow(vals: &[i64]) -> Vec<symbolic_ir::IRNode> {
    vals.iter().map(|&v| int(v)).collect()
}

fn matrix_entries(m: &symbolic_ir::IRNode) -> Vec<Vec<(i64, i64)>> {
    cas_matrix::rows_of(m)
        .unwrap()
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|entry| match entry {
                    symbolic_ir::IRNode::Integer(value) => (value, 1),
                    symbolic_ir::IRNode::Rational(numer, denom) => (numer, denom),
                    other => panic!("expected numeric entry, got {other:?}"),
                })
                .collect()
        })
        .collect()
}

fn list_args(list: &symbolic_ir::IRNode) -> Vec<symbolic_ir::IRNode> {
    match list {
        symbolic_ir::IRNode::Apply(apply) if apply.head == sym(LIST) => apply.args.clone(),
        other => panic!("expected List(...), got {other:?}"),
    }
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
    let m = matrix(vec![vec![sym("a"), sym("b")], vec![sym("c"), sym("d")]]).unwrap();
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
    let expected = matrix(vec![irow(&[1, 0, 0]), irow(&[0, 1, 0]), irow(&[0, 0, 1])]).unwrap();
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
    let m = matrix(vec![vec![sym("a"), sym("b")], vec![sym("c"), sym("d")]]).unwrap();
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
    let m = matrix(vec![vec![sym("a"), sym("b")], vec![sym("c"), sym("d")]]).unwrap();
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
    let m = matrix(vec![vec![sym("a"), sym("b")], vec![sym("c"), sym("d")]]).unwrap();
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

// ---------------------------------------------------------------------------
// Row reduction and rank
// ---------------------------------------------------------------------------

#[test]
fn row_reduce_identity_2x2_unchanged() {
    let eye = identity_matrix(2).unwrap();
    let reduced = row_reduce(&eye).unwrap();
    assert_eq!(
        matrix_entries(&reduced),
        vec![vec![(1, 1), (0, 1)], vec![(0, 1), (1, 1)]]
    );
}

#[test]
fn row_reduce_zero_matrix_3x3() {
    let zero = zero_matrix(3, 3).unwrap();
    let reduced = row_reduce(&zero).unwrap();
    assert!(matrix_entries(&reduced)
        .into_iter()
        .flatten()
        .all(|entry| entry == (0, 1)));
}

#[test]
fn row_reduce_full_rank_2x2_to_identity() {
    let m = matrix(vec![irow(&[2, 4]), irow(&[1, 3])]).unwrap();
    let reduced = row_reduce(&m).unwrap();
    assert_eq!(
        matrix_entries(&reduced),
        vec![vec![(1, 1), (0, 1)], vec![(0, 1), (1, 1)]]
    );
}

#[test]
fn row_reduce_singular_3x3() {
    let m = matrix(vec![irow(&[1, 2, 3]), irow(&[4, 5, 6]), irow(&[7, 8, 9])]).unwrap();
    let reduced = row_reduce(&m).unwrap();
    assert_eq!(
        matrix_entries(&reduced),
        vec![
            vec![(1, 1), (0, 1), (-1, 1)],
            vec![(0, 1), (1, 1), (2, 1)],
            vec![(0, 1), (0, 1), (0, 1)]
        ]
    );
}

#[test]
fn row_reduce_rational_dependent_rows() {
    let m = matrix(vec![vec![rat(1, 2), int(1)], vec![int(1), int(2)]]).unwrap();
    let reduced = row_reduce(&m).unwrap();
    assert_eq!(
        matrix_entries(&reduced),
        vec![vec![(1, 1), (2, 1)], vec![(0, 1), (0, 1)]]
    );
}

#[test]
fn rank_identity_and_zero() {
    assert_eq!(rank(&identity_matrix(3).unwrap()).unwrap(), int(3));
    assert_eq!(rank(&zero_matrix(3, 3).unwrap()).unwrap(), int(0));
}

#[test]
fn rank_full_and_singular_matrices() {
    let full = matrix(vec![irow(&[1, 2]), irow(&[3, 4])]).unwrap();
    let singular = matrix(vec![irow(&[1, 2, 3]), irow(&[4, 5, 6]), irow(&[7, 8, 9])]).unwrap();
    assert_eq!(rank(&full).unwrap(), int(2));
    assert_eq!(rank(&singular).unwrap(), int(2));
}

#[test]
fn rank_rational_dependent_rows() {
    let m = matrix(vec![vec![int(1), rat(1, 2)], vec![int(2), int(1)]]).unwrap();
    assert_eq!(rank(&m).unwrap(), int(1));
}

#[test]
fn rank_wide_and_tall_matrices() {
    let wide = matrix(vec![irow(&[1, 0, 2, 1]), irow(&[0, 1, 3, -1])]).unwrap();
    let tall = matrix(vec![
        irow(&[1, 0]),
        irow(&[0, 1]),
        irow(&[1, 1]),
        irow(&[2, 3]),
    ])
    .unwrap();
    assert_eq!(rank(&wide).unwrap(), int(2));
    assert_eq!(rank(&tall).unwrap(), int(2));
}

#[test]
fn row_reduce_and_rank_reject_symbolic_entries() {
    let m = matrix(vec![vec![sym("a"), int(1)], vec![int(0), int(1)]]).unwrap();
    assert!(row_reduce(&m).is_err());
    assert!(rank(&m).is_err());
}

// ---------------------------------------------------------------------------
// Norms
// ---------------------------------------------------------------------------

#[test]
fn norm_exact_rational_vector() {
    let v = matrix(vec![vec![rat(3, 5)], vec![rat(4, 5)]]).unwrap();
    assert_eq!(norm(&v).unwrap(), int(1));
}

#[test]
fn norm_non_square_sum_returns_sqrt() {
    let v = matrix(vec![irow(&[1]), irow(&[1])]).unwrap();
    assert_eq!(norm(&v).unwrap(), apply(sym(SQRT), vec![int(2)]));
}

#[test]
fn frobenius_norm_exact_matrix() {
    let m = matrix(vec![irow(&[1, 1]), irow(&[1, 1])]).unwrap();
    assert_eq!(frobenius_norm(&m).unwrap(), int(2));
}

#[test]
fn norm_rejects_non_vector() {
    let m = matrix(vec![irow(&[1, 0]), irow(&[0, 1])]).unwrap();
    assert!(norm(&m).is_err());
}

// ---------------------------------------------------------------------------
// LU decomposition
// ---------------------------------------------------------------------------

#[test]
fn lu_decompose_requires_pivoting() {
    let m = matrix(vec![irow(&[0, 1]), irow(&[1, 0])]).unwrap();
    let parts = list_args(&lu_decompose(&m).unwrap());
    assert_eq!(parts.len(), 3);

    let l = &parts[0];
    let u = &parts[1];
    let p = &parts[2];

    assert_eq!(
        matrix_entries(l),
        vec![vec![(1, 1), (0, 1)], vec![(0, 1), (1, 1)]]
    );
    assert_eq!(
        matrix_entries(u),
        vec![vec![(1, 1), (0, 1)], vec![(0, 1), (1, 1)]]
    );
    assert_eq!(
        matrix_entries(p),
        vec![vec![(0, 1), (1, 1)], vec![(1, 1), (0, 1)]]
    );
}

#[test]
fn lu_decompose_rejects_singular_matrix() {
    let m = matrix(vec![irow(&[1, 2]), irow(&[2, 4])]).unwrap();
    assert!(lu_decompose(&m).is_err());
}

// ---------------------------------------------------------------------------
// Subspaces
// ---------------------------------------------------------------------------

#[test]
fn nullspace_returns_column_vector_basis() {
    let m = matrix(vec![irow(&[1, 2, 3]), irow(&[4, 5, 6])]).unwrap();
    let basis = list_args(&nullspace(&m).unwrap());
    assert_eq!(basis.len(), 1);
    assert_eq!(
        matrix_entries(&basis[0]),
        vec![vec![(1, 1)], vec![(-2, 1)], vec![(1, 1)]]
    );
}

#[test]
fn columnspace_uses_original_pivot_columns() {
    let m = matrix(vec![irow(&[1, 2]), irow(&[2, 4])]).unwrap();
    let basis = list_args(&columnspace(&m).unwrap());
    assert_eq!(basis.len(), 1);
    assert_eq!(matrix_entries(&basis[0]), vec![vec![(1, 1)], vec![(2, 1)]]);
}

#[test]
fn rowspace_uses_nonzero_rref_rows() {
    let m = matrix(vec![irow(&[1, 2, 3]), irow(&[4, 5, 6])]).unwrap();
    let basis = list_args(&rowspace(&m).unwrap());
    assert_eq!(basis.len(), 2);
    assert_eq!(
        matrix_entries(&basis[0]),
        vec![vec![(1, 1), (0, 1), (-1, 1)]]
    );
    assert_eq!(
        matrix_entries(&basis[1]),
        vec![vec![(0, 1), (1, 1), (2, 1)]]
    );
}
