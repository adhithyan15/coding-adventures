//! Nullspace, columnspace, and rowspace via exact RREF.
//!
//! Each public function returns a `List(...)` IR node containing matrix-vector
//! basis elements.  Empty lists represent trivial subspaces.

use symbolic_ir::{apply, sym, IRNode, LIST};

use crate::matrix::{matrix, rows_of, MatrixResult};
use crate::rowreduce::{matrix_to_fracs, Frac};

/// Return a basis for the null space of `m`.
///
/// Basis vectors are returned as `n×1` column-vector matrices inside a
/// `List(...)` node.
pub fn nullspace(m: &IRNode) -> MatrixResult<IRNode> {
    let frows = matrix_to_fracs(m)?;
    let ncols = frows.first().map_or(0, |row| row.len());
    let (pivot_cols, rref) = rref_pivot_info(&frows);
    let free_cols: Vec<usize> = (0..ncols).filter(|col| !pivot_cols.contains(col)).collect();

    let mut basis = Vec::new();
    for free_col in free_cols {
        let mut vector = vec![Frac::zero(); ncols];
        vector[free_col] = Frac::from_i64(1);

        for (pivot_row, pivot_col) in pivot_cols.iter().enumerate() {
            vector[*pivot_col] = -rref[pivot_row][free_col];
        }

        let rows = vector
            .into_iter()
            .map(|entry| Ok(vec![entry.to_irnode()?]))
            .collect::<MatrixResult<Vec<Vec<IRNode>>>>()?;
        basis.push(matrix(rows)?);
    }

    Ok(apply(sym(LIST), basis))
}

/// Return a basis for the column space of `m`.
///
/// The basis uses pivot columns from the original input matrix.
pub fn columnspace(m: &IRNode) -> MatrixResult<IRNode> {
    let frows = matrix_to_fracs(m)?;
    let (pivot_cols, _) = rref_pivot_info(&frows);
    let orig_rows = rows_of(m)?;
    let nrows = orig_rows.len();

    let basis = pivot_cols
        .into_iter()
        .map(|col| {
            let rows = (0..nrows)
                .map(|row| vec![orig_rows[row][col].clone()])
                .collect();
            matrix(rows)
        })
        .collect::<MatrixResult<Vec<IRNode>>>()?;

    Ok(apply(sym(LIST), basis))
}

/// Return a basis for the row space of `m`.
///
/// The basis consists of non-zero rows of the RREF as `1×n` row-vector
/// matrices.
pub fn rowspace(m: &IRNode) -> MatrixResult<IRNode> {
    let frows = matrix_to_fracs(m)?;
    let (_, rref) = rref_pivot_info(&frows);

    let mut basis = Vec::new();
    for row in rref {
        if row.iter().any(|entry| !entry.is_zero()) {
            let ir_row = row
                .into_iter()
                .map(Frac::to_irnode)
                .collect::<MatrixResult<Vec<IRNode>>>()?;
            basis.push(matrix(vec![ir_row])?);
        }
    }

    Ok(apply(sym(LIST), basis))
}

fn rref_pivot_info(frows: &[Vec<Frac>]) -> (Vec<usize>, Vec<Vec<Frac>>) {
    if frows.is_empty() || frows[0].is_empty() {
        return (Vec::new(), frows.to_vec());
    }

    let nrows = frows.len();
    let ncols = frows[0].len();
    let mut rref = frows.to_vec();
    let mut pivot_cols = Vec::new();
    let mut pivot_row = 0;

    for col in 0..ncols {
        let pivot_pos = (pivot_row..nrows).find(|&row| !rref[row][col].is_zero());
        let Some(pivot_pos) = pivot_pos else {
            continue;
        };

        if pivot_pos != pivot_row {
            rref.swap(pivot_row, pivot_pos);
        }

        let pivot = rref[pivot_row][col];
        rref[pivot_row] = rref[pivot_row].iter().map(|entry| *entry / pivot).collect();

        for row in 0..nrows {
            if row == pivot_row {
                continue;
            }
            let factor = rref[row][col];
            if factor.is_zero() {
                continue;
            }
            rref[row] = (0..ncols)
                .map(|c| rref[row][c] - factor * rref[pivot_row][c])
                .collect();
        }

        pivot_cols.push(col);
        pivot_row += 1;
        if pivot_row == nrows {
            break;
        }
    }

    (pivot_cols, rref)
}
