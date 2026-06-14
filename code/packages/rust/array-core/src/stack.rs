//! HSTACK / VSTACK — concatenation with NA padding when shapes differ.
//!
//! Excel 365 lets you stack arrays of mismatched shape; the shorter axis is
//! padded with `#N/A` (we use `r-vector::na_real()`). HSTACK aligns rows
//! (vertical axis), VSTACK aligns columns (horizontal axis).

use crate::{na_real, Array2D, ArrayError};

/// `HSTACK(arr1, arr2, ...)` — horizontal concat.
///
/// All inputs are placed side-by-side; the result has
/// `cols = sum(arr_i.cols)` and `rows = max(arr_i.rows)`. Rows shorter than
/// the max are padded with NA at the bottom.
///
/// Empty inputs are skipped. If no inputs (or all empty), returns an empty
/// `(0, 0)` array.
pub fn hstack(arrays: &[&Array2D<f64>]) -> Result<Array2D<f64>, ArrayError> {
    if arrays.is_empty() {
        return Array2D::new(0, 0, vec![]);
    }
    let total_cols: usize = arrays.iter().map(|a| a.cols).sum();
    let max_rows: usize = arrays.iter().map(|a| a.rows).max().unwrap_or(0);

    if total_cols == 0 || max_rows == 0 {
        return Array2D::new(max_rows, total_cols, vec![na_real(); max_rows * total_cols]);
    }

    let mut data = vec![na_real(); max_rows * total_cols];
    let mut col_offset = 0usize;
    for a in arrays {
        for r in 0..a.rows {
            for c in 0..a.cols {
                data[r * total_cols + (col_offset + c)] = *a.get(r, c);
            }
        }
        col_offset += a.cols;
    }
    Array2D::new(max_rows, total_cols, data)
}

/// `VSTACK(arr1, arr2, ...)` — vertical concat.
///
/// All inputs are placed top-to-bottom; the result has
/// `rows = sum(arr_i.rows)` and `cols = max(arr_i.cols)`. Columns shorter
/// than the max are padded with NA on the right.
pub fn vstack(arrays: &[&Array2D<f64>]) -> Result<Array2D<f64>, ArrayError> {
    if arrays.is_empty() {
        return Array2D::new(0, 0, vec![]);
    }
    let total_rows: usize = arrays.iter().map(|a| a.rows).sum();
    let max_cols: usize = arrays.iter().map(|a| a.cols).max().unwrap_or(0);

    if total_rows == 0 || max_cols == 0 {
        return Array2D::new(total_rows, max_cols, vec![na_real(); total_rows * max_cols]);
    }

    let mut data = vec![na_real(); total_rows * max_cols];
    let mut row_offset = 0usize;
    for a in arrays {
        for r in 0..a.rows {
            for c in 0..a.cols {
                data[(row_offset + r) * max_cols + c] = *a.get(r, c);
            }
        }
        row_offset += a.rows;
    }
    Array2D::new(total_rows, max_cols, data)
}
