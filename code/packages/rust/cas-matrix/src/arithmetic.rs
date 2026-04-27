//! Matrix arithmetic: transpose, add, subtract, scalar multiply, trace, dot.
//!
//! ## Symbolic output
//!
//! Every arithmetic step is wrapped as unevaluated `IRApply(ADD/SUB/MUL/…)`
//! rather than collapsed to a numeric value.  Pass the result through
//! `cas_simplify::simplify` (or any downstream numeric-fold pass) to reduce
//! numeric entries.
//!
//! ## Constructors
//!
//! Also provides `identity_matrix(n)` and `zero_matrix(rows, cols)` — convenience
//! factories that build matrices of integer 0/1 entries.

use symbolic_ir::{apply, int, sym, IRNode, ADD, MUL, SUB};

use crate::matrix::{matrix, rows_of, MatrixError, MatrixResult};

// ---------------------------------------------------------------------------
// Constructors
// ---------------------------------------------------------------------------

/// Build an n×n identity matrix with integer 0/1 entries.
///
/// ```rust
/// use cas_matrix::identity_matrix;
/// use symbolic_ir::int;
///
/// let eye = identity_matrix(2).unwrap();
/// // Matrix([[1, 0], [0, 1]])
/// ```
pub fn identity_matrix(n: usize) -> MatrixResult<IRNode> {
    let rows: Vec<Vec<IRNode>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| int(if i == j { 1 } else { 0 }))
                .collect()
        })
        .collect();
    // edge case: 0×0 identity is a valid empty matrix — but our constructor
    // rejects empty vecs, so we special-case it.
    if n == 0 {
        return Err(MatrixError(
            "identity_matrix: n must be positive".into(),
        ));
    }
    matrix(rows)
}

/// Build a rows×cols matrix of integer zeros.
///
/// ```rust
/// use cas_matrix::zero_matrix;
/// use symbolic_ir::int;
///
/// let z = zero_matrix(2, 3).unwrap();
/// // Matrix([[0, 0, 0], [0, 0, 0]])
/// ```
pub fn zero_matrix(nrows: usize, ncols: usize) -> MatrixResult<IRNode> {
    if nrows == 0 || ncols == 0 {
        return Err(MatrixError(
            "zero_matrix: dims must be positive".into(),
        ));
    }
    let rows: Vec<Vec<IRNode>> = (0..nrows)
        .map(|_| (0..ncols).map(|_| int(0)).collect())
        .collect();
    matrix(rows)
}

// ---------------------------------------------------------------------------
// Transpose
// ---------------------------------------------------------------------------

/// Transpose a matrix.
///
/// ```rust
/// use cas_matrix::{matrix, transpose};
/// use symbolic_ir::int;
///
/// let m = matrix(vec![vec![int(1), int(2)], vec![int(3), int(4)]]).unwrap();
/// let t = transpose(&m).unwrap();
/// // t == [[1, 3], [2, 4]]
/// ```
pub fn transpose(m: &IRNode) -> MatrixResult<IRNode> {
    let rows = rows_of(m)?;
    if rows.is_empty() {
        return matrix(vec![vec![]]); // shouldn't happen given invariants
    }
    let nrows = rows.len();
    let ncols = rows[0].len();
    let new_rows: Vec<Vec<IRNode>> = (0..ncols)
        .map(|j| (0..nrows).map(|i| rows[i][j].clone()).collect())
        .collect();
    matrix(new_rows)
}

// ---------------------------------------------------------------------------
// Elementwise operations
// ---------------------------------------------------------------------------

/// Elementwise matrix addition A + B.
///
/// Each entry of the result is `Add(aᵢⱼ, bᵢⱼ)` (unevaluated).
///
/// ```rust
/// use cas_matrix::{matrix, add_matrices};
/// use symbolic_ir::{apply, int, sym, ADD};
///
/// let a = matrix(vec![vec![int(1), int(2)]]).unwrap();
/// let b = matrix(vec![vec![int(3), int(4)]]).unwrap();
/// let c = add_matrices(&a, &b).unwrap();
/// // c == [[Add(1,3), Add(2,4)]]
/// ```
pub fn add_matrices(a: &IRNode, b: &IRNode) -> MatrixResult<IRNode> {
    let a_rows = rows_of(a)?;
    let b_rows = rows_of(b)?;
    check_same_shape(&a_rows, &b_rows, "add")?;
    let new_rows = elementwise(a_rows, b_rows, |x, y| apply(sym(ADD), vec![x, y]));
    matrix(new_rows)
}

/// Elementwise matrix subtraction A − B.
///
/// Each entry is `Sub(aᵢⱼ, bᵢⱼ)`.
pub fn sub_matrices(a: &IRNode, b: &IRNode) -> MatrixResult<IRNode> {
    let a_rows = rows_of(a)?;
    let b_rows = rows_of(b)?;
    check_same_shape(&a_rows, &b_rows, "sub")?;
    let new_rows = elementwise(a_rows, b_rows, |x, y| apply(sym(SUB), vec![x, y]));
    matrix(new_rows)
}

/// Scalar multiplication: every entry becomes `Mul(scalar, entry)`.
///
/// ```rust
/// use cas_matrix::{matrix, scalar_multiply};
/// use symbolic_ir::{apply, int, sym, MUL};
///
/// let m = matrix(vec![vec![int(1), int(2)]]).unwrap();
/// let s = scalar_multiply(&int(3), &m).unwrap();
/// // s == [[Mul(3,1), Mul(3,2)]]
/// ```
pub fn scalar_multiply(scalar: &IRNode, m: &IRNode) -> MatrixResult<IRNode> {
    let rows = rows_of(m)?;
    let new_rows: Vec<Vec<IRNode>> = rows
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|cell| apply(sym(MUL), vec![scalar.clone(), cell]))
                .collect()
        })
        .collect();
    matrix(new_rows)
}

/// Sum of the main diagonal.  Square matrices only.
///
/// Returns `Add(diag₀, diag₁, …)` for n>1, the single diagonal entry for n=1,
/// or `IRInteger(0)` for a 0×0 matrix (by convention).
///
/// ```rust
/// use cas_matrix::{matrix, trace};
/// use symbolic_ir::{apply, int, sym, ADD};
///
/// let m = matrix(vec![
///     vec![int(1), int(2)],
///     vec![int(3), int(4)],
/// ]).unwrap();
/// // trace == Add(1, 4)
/// ```
pub fn trace(m: &IRNode) -> MatrixResult<IRNode> {
    let rows = rows_of(m)?;
    let nrows = rows.len();
    let ncols = rows.first().map_or(0, |r| r.len());
    if nrows != ncols {
        return Err(MatrixError(format!(
            "trace: matrix must be square, got {nrows}×{ncols}"
        )));
    }
    if nrows == 0 {
        return Ok(int(0));
    }
    let diag: Vec<IRNode> = (0..nrows).map(|i| rows[i][i].clone()).collect();
    if diag.len() == 1 {
        return Ok(diag.into_iter().next().unwrap());
    }
    Ok(apply(sym(ADD), diag))
}

// ---------------------------------------------------------------------------
// Dot product (matrix multiplication)
// ---------------------------------------------------------------------------

/// Matrix product A · B.
///
/// `cols(A)` must equal `rows(B)`.  Each entry of the result is an
/// `Add(Mul(…), Mul(…), …)` expression (unevaluated).
///
/// ```rust
/// use cas_matrix::{matrix, dot};
/// use symbolic_ir::int;
///
/// let a = matrix(vec![vec![int(1), int(0)], vec![int(0), int(1)]]).unwrap(); // 2×2 identity
/// let b = matrix(vec![vec![int(5), int(6)], vec![int(7), int(8)]]).unwrap();
/// let c = dot(&a, &b).unwrap(); // A·B = B (symbolically, not simplified yet)
/// ```
pub fn dot(a: &IRNode, b: &IRNode) -> MatrixResult<IRNode> {
    let a_rows = rows_of(a)?;
    let b_rows = rows_of(b)?;
    if a_rows.is_empty() || b_rows.is_empty() {
        return Err(MatrixError(
            "dot: both operands must have at least one row".into(),
        ));
    }
    let a_cols = a_rows[0].len();
    let b_row_count = b_rows.len();
    if a_cols != b_row_count {
        return Err(MatrixError(format!(
            "dot: cols(A)={a_cols} != rows(B)={b_row_count}"
        )));
    }
    let b_cols = b_rows[0].len();
    let new_rows: Vec<Vec<IRNode>> = (0..a_rows.len())
        .map(|i| {
            (0..b_cols)
                .map(|j| {
                    let terms: Vec<IRNode> = (0..a_cols)
                        .map(|k| {
                            apply(
                                sym(MUL),
                                vec![a_rows[i][k].clone(), b_rows[k][j].clone()],
                            )
                        })
                        .collect();
                    if terms.len() == 1 {
                        terms.into_iter().next().unwrap()
                    } else {
                        apply(sym(ADD), terms)
                    }
                })
                .collect()
        })
        .collect();
    matrix(new_rows)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Verify that two row-of-rows have the same shape.
fn check_same_shape(
    a: &[Vec<IRNode>],
    b: &[Vec<IRNode>],
    op: &str,
) -> MatrixResult<()> {
    let (ar, ac) = (a.len(), a.first().map_or(0, |r| r.len()));
    let (br, bc) = (b.len(), b.first().map_or(0, |r| r.len()));
    if ar != br || ac != bc {
        return Err(MatrixError(format!(
            "{op}: shape mismatch ({ar}×{ac} vs {br}×{bc})"
        )));
    }
    Ok(())
}

/// Apply a binary combiner elementwise to two same-shape row matrices.
fn elementwise(
    a_rows: Vec<Vec<IRNode>>,
    b_rows: Vec<Vec<IRNode>>,
    f: impl Fn(IRNode, IRNode) -> IRNode,
) -> Vec<Vec<IRNode>> {
    a_rows
        .into_iter()
        .zip(b_rows)
        .map(|(ra, rb)| {
            ra.into_iter()
                .zip(rb)
                .map(|(x, y)| f(x, y))
                .collect()
        })
        .collect()
}
