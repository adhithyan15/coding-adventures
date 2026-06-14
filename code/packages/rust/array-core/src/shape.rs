//! Shape-mutating helpers: TAKE, DROP, EXPAND.
//!
//! All three accept negative arguments meaning "from the end" (Excel 365
//! convention). The implementations factor through a small helper
//! `slice_range` that turns Excel's signed length into a 0-based half-open
//! range against an axis.

use crate::{na_real, Array2D, ArrayError};

/// Compute the 0-based half-open range that an Excel-style signed `n`
/// selects for TAKE on an axis of length `len`.
///
/// * `n > 0`: take the first `min(n, len)` items -> `0..min(n, len)`.
/// * `n < 0`: take the last `min(|n|, len)` items -> `len - min(|n|, len) .. len`.
/// * `n == 0`: take zero items -> `0..0`.
///
/// This matches `=TAKE({1,2,3,4,5}, 2)` -> `{1,2}` and
/// `=TAKE({1,2,3,4,5}, -2)` -> `{4,5}`. Over-taking is a no-op clamp: Excel
/// does not error on `=TAKE({1,2,3}, 99)`.
fn take_range(len: usize, n: i64) -> std::ops::Range<usize> {
    if n == 0 {
        return 0..0;
    }
    let take = std::cmp::min(n.unsigned_abs() as usize, len);
    if n > 0 {
        0..take
    } else {
        (len - take)..len
    }
}

/// Compute the 0-based half-open range that survives DROP of an Excel-style
/// signed `n` on an axis of length `len`.
///
/// * `n > 0`: drop the first `n` -> `min(n, len)..len`.
/// * `n < 0`: drop the last `|n|` -> `0..len - min(|n|, len)`.
/// * `n == 0`: drop nothing -> `0..len`.
///
/// Over-dropping is again a clamp: `=DROP({1,2,3}, 99)` returns an empty
/// array (Excel emits `#CALC!` in that case; for the host frontend we surface
/// the empty result and let the cell layer translate).
fn drop_range(len: usize, n: i64) -> std::ops::Range<usize> {
    if n == 0 {
        return 0..len;
    }
    let drop = std::cmp::min(n.unsigned_abs() as usize, len);
    if n > 0 {
        drop..len
    } else {
        0..(len - drop)
    }
}

/// `TAKE(array, rows, [cols])` — Excel 365.
///
/// Returns the first `rows` rows and first `cols` cols. Negative counts take
/// from the end. `cols = None` keeps all columns; passing `Some(0)` drops all
/// columns (producing a `(rows, 0)` array). Same for `rows = 0`.
pub fn take(
    array: &Array2D<f64>,
    rows: i64,
    cols: Option<i64>,
) -> Result<Array2D<f64>, ArrayError> {
    let row_range = take_range(array.rows, rows);
    let col_range = match cols {
        Some(c) => take_range(array.cols, c),
        None => 0..array.cols,
    };
    slice(array, row_range, col_range)
}

/// `DROP(array, rows, [cols])` — Excel 365.
///
/// Returns the array with the first `rows` rows and `cols` cols removed.
/// Negative counts drop from the end. Over-dropping returns an empty array.
pub fn drop(
    array: &Array2D<f64>,
    rows: i64,
    cols: Option<i64>,
) -> Result<Array2D<f64>, ArrayError> {
    let row_range = drop_range(array.rows, rows);
    let col_range = match cols {
        Some(c) => drop_range(array.cols, c),
        None => 0..array.cols,
    };
    slice(array, row_range, col_range)
}

/// Generic slice helper used by TAKE and DROP. Materializes a fresh
/// row-major buffer for the sub-rectangle described by the two ranges.
fn slice(
    array: &Array2D<f64>,
    row_range: std::ops::Range<usize>,
    col_range: std::ops::Range<usize>,
) -> Result<Array2D<f64>, ArrayError> {
    let out_rows = row_range.end.saturating_sub(row_range.start);
    let out_cols = col_range.end.saturating_sub(col_range.start);
    let mut data = Vec::with_capacity(out_rows * out_cols);
    for r in row_range {
        for c in col_range.clone() {
            data.push(*array.get(r, c));
        }
    }
    Array2D::new(out_rows, out_cols, data)
}

/// `EXPAND(array, rows, [cols], [pad_with])` — Excel 365.
///
/// Grows `array` to exactly `(rows, cols)`, padding the new region with
/// `pad_with` (default: NA — Excel emits `#N/A`). Shrinking is an error:
/// Excel emits `#VALUE!` if the requested dimensions are smaller than the
/// source. We translate that into `BadParameter`.
pub fn expand(
    array: &Array2D<f64>,
    rows: usize,
    cols: Option<usize>,
    pad_with: Option<f64>,
) -> Result<Array2D<f64>, ArrayError> {
    let cols = cols.unwrap_or(array.cols);
    if rows < array.rows {
        return Err(ArrayError::BadParameter {
            name: "rows",
            value: format!("{rows} (smaller than source {})", array.rows),
        });
    }
    if cols < array.cols {
        return Err(ArrayError::BadParameter {
            name: "cols",
            value: format!("{cols} (smaller than source {})", array.cols),
        });
    }
    let pad = pad_with.unwrap_or_else(na_real);
    let mut data = vec![pad; rows * cols];
    // Copy the source into the top-left corner. We intentionally do not
    // center or right-align — Excel always anchors at the top-left.
    for r in 0..array.rows {
        for c in 0..array.cols {
            data[r * cols + c] = *array.get(r, c);
        }
    }
    Array2D::new(rows, cols, data)
}
