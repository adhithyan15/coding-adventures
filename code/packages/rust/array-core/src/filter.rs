//! FILTER — boolean-mask row filter.
//!
//! `FILTER(array, include, [if_empty])` returns the rows of `array` where
//! `include[i]` is truthy (non-zero, not NA). The mask must have one entry
//! per row of `array`. If nothing passes and `if_empty` is `None`, returns
//! `ArrayError::EmptyResult` — which the cell layer maps to `#CALC!`.
//!
//! # NA-in-mask behavior
//!
//! NA in the mask is treated as `FALSE`. This matches Excel: a blank or
//! `#N/A` in the include criteria excludes the corresponding row. (Modeling
//! that as "propagate NA into the included rows" would surprise spreadsheet
//! authors more than it would help; the included rows are a subset of the
//! original, not arithmetically derived from the mask.)

use crate::{is_na_real, Array2D, ArrayError};

/// Coerce an f64 mask entry to a boolean. Returns `false` for NA, `false`
/// for `0.0` (Excel convention), and `true` for everything else (including
/// non-NA NaN, matching Excel's quirk of `=IF(NaN, "y", "n")` being truthy).
fn mask_is_true(value: f64) -> bool {
    if is_na_real(value) {
        return false;
    }
    value != 0.0
}

/// `FILTER(array, include, [if_empty])` — Excel 365.
///
/// `include` must be an `Array2D<f64>` whose total cell count equals
/// `array.rows`. We accept any shape (row vector, column vector, or even a
/// single column with the right length) for ergonomic API surface. Excel
/// itself accepts both row and column vector masks.
///
/// Output: a new array containing only rows where the mask is truthy. If
/// the result is empty, returns `if_empty` (as a 1x1 array containing that
/// single value, matching Excel's spill behavior) or `EmptyResult`.
pub fn filter(
    array: &Array2D<f64>,
    include: &Array2D<f64>,
    if_empty: Option<f64>,
) -> Result<Array2D<f64>, ArrayError> {
    if include.data.len() != array.rows {
        return Err(ArrayError::ShapeMismatch {
            expected: format!("{} mask values (one per row)", array.rows),
            found: format!("{} mask values", include.data.len()),
        });
    }
    let kept_rows: Vec<usize> = include
        .data
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| if mask_is_true(v) { Some(i) } else { None })
        .collect();

    if kept_rows.is_empty() {
        return match if_empty {
            Some(v) => Array2D::new(1, 1, vec![v]),
            None => Err(ArrayError::EmptyResult { function: "FILTER" }),
        };
    }

    let mut data = Vec::with_capacity(kept_rows.len() * array.cols);
    for r in &kept_rows {
        for c in 0..array.cols {
            data.push(*array.get(*r, c));
        }
    }
    Array2D::new(kept_rows.len(), array.cols, data)
}
