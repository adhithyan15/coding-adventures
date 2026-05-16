//! Reshape helpers: TOROW, TOCOL, WRAPROWS, WRAPCOLS.
//!
//! TOROW/TOCOL flatten an arbitrary 2-D array into a single row or column,
//! optionally skipping blanks or errors. WRAPROWS/WRAPCOLS go the other
//! direction, reshaping a 1-D vector into a 2-D grid with NA padding when
//! the count does not divide evenly.

use crate::{is_na_real, na_real, Array2D, ArrayError};

/// `ignore` modes for TOROW / TOCOL, matching Excel 365's documented
/// 4-value enum.
///
/// | Code | Meaning                              |
/// |------|--------------------------------------|
/// | 0    | Keep every cell (default).           |
/// | 1    | Skip blanks (here: NA cells).        |
/// | 2    | Skip errors. We currently have no    |
/// |      | distinct error encoding apart from   |
/// |      | NA, so this behaves like 0.          |
/// | 3    | Skip both. Behaves like 1.           |
///
/// # Divergence from Excel 365
///
/// Excel distinguishes "blank" from `#N/A` and other error sentinels. In our
/// `Array2D<f64>` storage there is only NA. When richer text/error arrays
/// land in Phase 2 we will revisit modes 2 and 3 so they treat
/// `#VALUE!`/`#DIV/0!`/etc. as the "error" bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ignore {
    KeepAll,
    SkipBlanks,
    SkipErrors,
    SkipBoth,
}

impl Ignore {
    pub fn from_code(code: u8) -> Result<Self, ArrayError> {
        match code {
            0 => Ok(Ignore::KeepAll),
            1 => Ok(Ignore::SkipBlanks),
            2 => Ok(Ignore::SkipErrors),
            3 => Ok(Ignore::SkipBoth),
            _ => Err(ArrayError::BadParameter {
                name: "ignore",
                value: code.to_string(),
            }),
        }
    }

    fn keeps(self, value: f64) -> bool {
        let is_na = is_na_real(value);
        match self {
            Ignore::KeepAll => true,
            // We treat NA as "blank" for now (see module docstring).
            Ignore::SkipBlanks => !is_na,
            // No distinct error type yet; nothing to skip.
            Ignore::SkipErrors => true,
            // Same NA-as-blank treatment.
            Ignore::SkipBoth => !is_na,
        }
    }
}

/// Walk an array, optionally row-major (default) or column-major, applying
/// `ignore`. Returns a single flat `Vec<f64>` of the kept cells in scan
/// order.
fn flatten_with_ignore(
    array: &Array2D<f64>,
    ignore: Ignore,
    scan_by_column: bool,
) -> Vec<f64> {
    let mut out = Vec::with_capacity(array.data.len());
    if scan_by_column {
        for c in 0..array.cols {
            for r in 0..array.rows {
                let value = *array.get(r, c);
                if ignore.keeps(value) {
                    out.push(value);
                }
            }
        }
    } else {
        for r in 0..array.rows {
            for c in 0..array.cols {
                let value = *array.get(r, c);
                if ignore.keeps(value) {
                    out.push(value);
                }
            }
        }
    }
    out
}

/// `TOROW(array, [ignore], [scan_by_column])` — Excel 365.
///
/// Flattens to a `(1, n)` array. If all cells are skipped, returns a
/// `(1, 0)` array (Excel emits `#CALC!`; we surface empty and let the cell
/// layer decide).
pub fn to_row(
    array: &Array2D<f64>,
    ignore: Option<u8>,
    scan_by_column: Option<bool>,
) -> Result<Array2D<f64>, ArrayError> {
    let ignore = Ignore::from_code(ignore.unwrap_or(0))?;
    let scan_by_column = scan_by_column.unwrap_or(false);
    let flat = flatten_with_ignore(array, ignore, scan_by_column);
    let len = flat.len();
    Array2D::new(1, len, flat)
}

/// `TOCOL(array, [ignore], [scan_by_column])` — Excel 365.
///
/// Flattens to an `(n, 1)` array. See `to_row` for skip semantics.
pub fn to_col(
    array: &Array2D<f64>,
    ignore: Option<u8>,
    scan_by_column: Option<bool>,
) -> Result<Array2D<f64>, ArrayError> {
    let ignore = Ignore::from_code(ignore.unwrap_or(0))?;
    let scan_by_column = scan_by_column.unwrap_or(false);
    let flat = flatten_with_ignore(array, ignore, scan_by_column);
    let len = flat.len();
    Array2D::new(len, 1, flat)
}

/// `WRAPROWS(vector, wrap_count, [pad_with])` — Excel 365.
///
/// Reshapes a 1-D input (any `(r, c)` is allowed, but Excel documents this as
/// a "vector") into a grid `wrap_count` columns wide. Reads input in
/// row-major order. The last row is padded with `pad_with` (default NA) if
/// the total cell count is not a multiple of `wrap_count`.
pub fn wrap_rows(
    vector: &Array2D<f64>,
    wrap_count: usize,
    pad_with: Option<f64>,
) -> Result<Array2D<f64>, ArrayError> {
    if wrap_count == 0 {
        return Err(ArrayError::BadParameter {
            name: "wrap_count",
            value: "0".into(),
        });
    }
    let pad = pad_with.unwrap_or_else(na_real);
    let n = vector.data.len();
    if n == 0 {
        return Array2D::new(0, wrap_count, vec![]);
    }
    let rows = n.div_ceil(wrap_count);
    let mut data = vec![pad; rows * wrap_count];
    for (i, v) in vector.data.iter().enumerate() {
        data[i] = *v;
    }
    Array2D::new(rows, wrap_count, data)
}

/// `WRAPCOLS(vector, wrap_count, [pad_with])` — Excel 365.
///
/// Like WRAPROWS but reshapes into a grid `wrap_count` rows tall. The output
/// fills column-by-column, padding the last column with `pad_with`.
pub fn wrap_cols(
    vector: &Array2D<f64>,
    wrap_count: usize,
    pad_with: Option<f64>,
) -> Result<Array2D<f64>, ArrayError> {
    if wrap_count == 0 {
        return Err(ArrayError::BadParameter {
            name: "wrap_count",
            value: "0".into(),
        });
    }
    let pad = pad_with.unwrap_or_else(na_real);
    let n = vector.data.len();
    if n == 0 {
        return Array2D::new(wrap_count, 0, vec![]);
    }
    let cols = n.div_ceil(wrap_count);
    let mut data = vec![pad; wrap_count * cols];
    // Fill column-by-column. Input is consumed in its row-major order.
    for (i, v) in vector.data.iter().enumerate() {
        let c = i / wrap_count;
        let r = i % wrap_count;
        data[r * cols + c] = *v;
    }
    Array2D::new(wrap_count, cols, data)
}
