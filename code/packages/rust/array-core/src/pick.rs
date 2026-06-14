//! CHOOSEROWS / CHOOSECOLS — pick rows or columns by 1-based index.
//!
//! Indices are 1-based, possibly negative (count from end). Excel raises
//! `#VALUE!` for `0` and `#REF!` for out-of-range; we map both to
//! `ArrayError::OutOfRange` and let the cell layer decide which sentinel to
//! surface.

use crate::{resolve_index, Array2D, ArrayError};

/// `CHOOSEROWS(array, indices...)` — Excel 365.
///
/// Returns a new array containing the specified rows in the order given.
/// Repeating an index repeats the row. Negative indices count from the end.
pub fn choose_rows(array: &Array2D<f64>, indices: &[i64]) -> Result<Array2D<f64>, ArrayError> {
    if indices.is_empty() {
        return Err(ArrayError::BadParameter {
            name: "indices",
            value: "(none)".into(),
        });
    }
    let mut data = Vec::with_capacity(indices.len() * array.cols);
    for &idx in indices {
        let r = resolve_index("CHOOSEROWS", idx, array.rows)?;
        for c in 0..array.cols {
            data.push(*array.get(r, c));
        }
    }
    Array2D::new(indices.len(), array.cols, data)
}

/// `CHOOSECOLS(array, indices...)` — Excel 365.
pub fn choose_cols(array: &Array2D<f64>, indices: &[i64]) -> Result<Array2D<f64>, ArrayError> {
    if indices.is_empty() {
        return Err(ArrayError::BadParameter {
            name: "indices",
            value: "(none)".into(),
        });
    }
    let out_cols = indices.len();
    let mut data = vec![0.0; array.rows * out_cols];
    for (out_c, &idx) in indices.iter().enumerate() {
        let src_c = resolve_index("CHOOSECOLS", idx, array.cols)?;
        for r in 0..array.rows {
            data[r * out_cols + out_c] = *array.get(r, src_c);
        }
    }
    Array2D::new(array.rows, out_cols, data)
}
