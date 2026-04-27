//! Determinant and inverse via cofactor expansion.
//!
//! ## Algorithm
//!
//! Both functions produce *symbolic* output: every arithmetic step is wrapped
//! as `IRApply(ADD/SUB/MUL/NEG/DIV, …)` rather than evaluated numerically.
//! Pass the results through `cas_simplify::simplify` to reduce numeric entries.
//!
//! ### Cofactor expansion
//!
//! The determinant is computed by expanding along the first row:
//!
//! ```text
//! det(M) = Σⱼ (-1)^(0+j) · M[0][j] · det(minor(M, 0, j))
//! ```
//!
//! Base cases:
//! - 0×0 matrix: `det = 1` (the empty product, by convention).
//! - 1×1 matrix: `det = M[0][0]`.
//! - 2×2 matrix: `det = Sub(Mul(a, d), Mul(b, c))`.
//!
//! ### Inverse
//!
//! Uses the classical adjugate formula:
//! ```text
//! M⁻¹ = adjugate(M) / det(M)
//! ```
//! where `adjugate(M)` is the transpose of the cofactor matrix.  Each entry
//! of the inverse is `Div(cofactor, det)` (unevaluated).
//!
//! ### Complexity
//!
//! O(n!) — practical only for n ≤ ~6.  For large numeric-only matrices, a
//! Bareiss-elimination fast path can be added later.

use symbolic_ir::{apply, int, sym, IRNode, ADD, DIV, MUL, NEG, SUB};

use crate::matrix::{matrix, rows_of, MatrixError, MatrixResult};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute the determinant of a square matrix.
///
/// Returns un-simplified IR. Pass through `cas_simplify::simplify` to reduce.
///
/// ```rust
/// use cas_matrix::{matrix, determinant};
/// use symbolic_ir::{int, IRNode};
///
/// let m = matrix(vec![
///     vec![int(1), int(2)],
///     vec![int(3), int(4)],
/// ]).unwrap();
///
/// let d = determinant(&m).unwrap();
/// // d = Sub(Mul(1, 4), Mul(2, 3))  →  simplify → -2
/// ```
pub fn determinant(m: &IRNode) -> MatrixResult<IRNode> {
    let rows = rows_of(m)?;
    let n = rows.len();
    let ncols = rows.first().map_or(0, |r| r.len());
    if n != ncols {
        return Err(MatrixError(format!(
            "determinant: matrix must be square, got {n}×{ncols}"
        )));
    }
    Ok(det(&rows))
}

/// Compute the symbolic inverse of a square matrix.
///
/// Returns a `Matrix` whose entries are `Div(cofactor, det)` expressions
/// (unevaluated).  Pass through `cas_simplify::simplify` to reduce numeric
/// entries.
///
/// ```rust
/// use cas_matrix::{matrix, inverse};
/// use symbolic_ir::int;
///
/// let m = matrix(vec![
///     vec![int(1), int(2)],
///     vec![int(3), int(4)],
/// ]).unwrap();
///
/// let inv = inverse(&m).unwrap();
/// // Each entry = Div(cofactor, det) — symbolic; run simplify to evaluate.
/// ```
pub fn inverse(m: &IRNode) -> MatrixResult<IRNode> {
    let rows = rows_of(m)?;
    let n = rows.len();
    let ncols = rows.first().map_or(0, |r| r.len());
    if n != ncols {
        return Err(MatrixError(format!(
            "inverse: matrix must be square, got {n}×{ncols}"
        )));
    }
    if n == 0 {
        return matrix(vec![vec![]]);  // 0×0 case — return empty matrix
    }
    let d = det(&rows);

    // Build cofactor matrix, then transpose to get adjugate.
    // cofactor[i][j] = (-1)^(i+j) · det(minor(rows, i, j))
    let cof_rows: Vec<Vec<IRNode>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let sub_det = det(&minor(&rows, i, j));
                    // Even (i+j) → positive, odd → negate.
                    if (i + j) % 2 == 0 {
                        sub_det
                    } else {
                        apply(sym(NEG), vec![sub_det])
                    }
                })
                .collect()
        })
        .collect();

    // Transpose cofactor matrix → adjugate.
    let adj_rows: Vec<Vec<IRNode>> = (0..n)
        .map(|c| (0..n).map(|r| cof_rows[r][c].clone()).collect())
        .collect();

    // Divide every adjugate entry by det.
    let inv_rows: Vec<Vec<IRNode>> = adj_rows
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|cell| apply(sym(DIV), vec![cell, d.clone()]))
                .collect()
        })
        .collect();

    matrix(inv_rows)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Recursive determinant via cofactor expansion along the first row.
///
/// `rows` is a fully-owned Vec-of-Vec of the current sub-matrix.
pub(crate) fn det(rows: &[Vec<IRNode>]) -> IRNode {
    let n = rows.len();
    match n {
        0 => int(1), // det of empty matrix is 1 by convention
        1 => rows[0][0].clone(),
        2 => {
            // det = a·d − b·c
            let (a, b) = (rows[0][0].clone(), rows[0][1].clone());
            let (c, d) = (rows[1][0].clone(), rows[1][1].clone());
            apply(
                sym(SUB),
                vec![
                    apply(sym(MUL), vec![a, d]),
                    apply(sym(MUL), vec![b, c]),
                ],
            )
        }
        _ => {
            // Expand along the first row.
            //
            // term_j = M[0][j] · det(minor(rows, 0, j))
            // sign:  j even → positive, j odd → negate the term
            let terms: Vec<IRNode> = rows[0]
                .iter()
                .enumerate()
                .map(|(j, entry)| {
                    let minor_rows = minor(rows, 0, j);
                    let sub_det = det(&minor_rows);
                    let product = apply(sym(MUL), vec![entry.clone(), sub_det]);
                    if j % 2 == 0 {
                        product
                    } else {
                        apply(sym(NEG), vec![product])
                    }
                })
                .collect();
            if terms.len() == 1 {
                terms.into_iter().next().unwrap()
            } else {
                apply(sym(ADD), terms)
            }
        }
    }
}

/// Return the (n-1)×(n-1) minor obtained by deleting row `skip_row` and
/// column `skip_col`.
fn minor(rows: &[Vec<IRNode>], skip_row: usize, skip_col: usize) -> Vec<Vec<IRNode>> {
    rows.iter()
        .enumerate()
        .filter(|(ri, _)| *ri != skip_row)
        .map(|(_, row)| {
            row.iter()
                .enumerate()
                .filter(|(ci, _)| *ci != skip_col)
                .map(|(_, cell)| cell.clone())
                .collect()
        })
        .collect()
}
