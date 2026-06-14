//! LU decomposition with partial pivoting.
//!
//! Returns `List(L, U, P)` where `P * A = L * U`, matching the Python
//! cas-matrix API shape.

use symbolic_ir::{apply, sym, IRNode, LIST};

use crate::matrix::{num_cols, num_rows, MatrixError, MatrixResult};
use crate::rowreduce::{fracs_to_matrix, matrix_to_fracs, Frac};

/// Compute the LU decomposition of a square numeric matrix.
///
/// The implementation uses exact rational arithmetic and partial pivoting.
/// Singular matrices return `MatrixError`.
pub fn lu_decompose(m: &IRNode) -> MatrixResult<IRNode> {
    let n = num_rows(m)?;
    let ncols = num_cols(m)?;
    if n != ncols {
        return Err(MatrixError(format!(
            "lu_decompose: matrix must be square, got {n}×{ncols}"
        )));
    }

    let mut u = matrix_to_fracs(m)?;
    let mut l = identity_fracs(n);
    let mut p = identity_fracs(n);

    for k in 0..n {
        let mut best_row = k;
        let mut best_value = u[k][k].abs();
        for (row, entries) in u.iter().enumerate().skip(k + 1) {
            let candidate = entries[k].abs();
            if greater_frac(candidate, best_value) {
                best_value = candidate;
                best_row = row;
            }
        }

        if best_row != k {
            u.swap(k, best_row);
            p.swap(k, best_row);
            for col in 0..k {
                let tmp = l[k][col];
                l[k][col] = l[best_row][col];
                l[best_row][col] = tmp;
            }
        }

        let pivot = u[k][k];
        if pivot.is_zero() {
            return Err(MatrixError(format!(
                "lu_decompose: singular matrix (zero pivot at column {k})"
            )));
        }

        for row in (k + 1)..n {
            let factor = u[row][k] / pivot;
            l[row][k] = factor;
            for col in k..n {
                u[row][col] = u[row][col] - factor * u[k][col];
            }
        }
    }

    Ok(apply(
        sym(LIST),
        vec![
            fracs_to_matrix(l)?,
            fracs_to_matrix(u)?,
            fracs_to_matrix(p)?,
        ],
    ))
}

fn identity_fracs(n: usize) -> Vec<Vec<Frac>> {
    (0..n)
        .map(|row| {
            (0..n)
                .map(|col| {
                    if row == col {
                        Frac::from_i64(1)
                    } else {
                        Frac::zero()
                    }
                })
                .collect()
        })
        .collect()
}

fn greater_frac(left: Frac, right: Frac) -> bool {
    left.numer * right.denom > right.numer * left.denom
}
