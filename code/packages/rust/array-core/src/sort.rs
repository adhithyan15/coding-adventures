//! SORT / SORTBY — order rows or columns by one or more keys.
//!
//! Both functions are stable: equal keys preserve original order. NA values
//! sort to the end in ascending order and to the beginning in descending
//! order — equivalent to "errors always last in ascending direction", which
//! matches Excel's documented behavior.

use crate::{is_na_real, resolve_index, Array2D, ArrayError};

/// Sort direction. `+1` = ascending (default), `-1` = descending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Ascending,
    Descending,
}

impl Order {
    pub fn from_code(code: i32) -> Result<Self, ArrayError> {
        match code {
            1 => Ok(Order::Ascending),
            -1 => Ok(Order::Descending),
            _ => Err(ArrayError::BadParameter {
                name: "sort_order",
                value: code.to_string(),
            }),
        }
    }
}

/// Comparison that puts NA at the "end" of ascending order. We use
/// `f64::total_cmp` for the non-NA case (a total order over all f64s
/// including NaN), so subnormals and signed zeros behave deterministically.
fn cmp_key(a: f64, b: f64, order: Order) -> std::cmp::Ordering {
    let a_na = is_na_real(a);
    let b_na = is_na_real(b);
    // NA always sorts last in ascending; flip for descending so that NA
    // appears first there. This matches Excel's "errors last" convention
    // for ascending, mirrored.
    let cmp = match (a_na, b_na) {
        (true, true) => std::cmp::Ordering::Equal,
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        (false, false) => a.total_cmp(&b),
    };
    match order {
        Order::Ascending => cmp,
        Order::Descending => cmp.reverse(),
    }
}

/// `SORT(array, [sort_index], [sort_order], [by_col])` — Excel 365.
///
/// Default sorts rows ascending by the first column. `by_col = true` flips
/// the axis: sort columns by a row's values.
///
/// `sort_index` is 1-based and refers to a column (when sorting rows) or a
/// row (when sorting columns). Out-of-range values raise `OutOfRange`.
pub fn sort(
    array: &Array2D<f64>,
    sort_index: Option<i64>,
    sort_order: Option<i32>,
    by_col: Option<bool>,
) -> Result<Array2D<f64>, ArrayError> {
    let by_col = by_col.unwrap_or(false);
    let order = Order::from_code(sort_order.unwrap_or(1))?;
    let sort_index_1b = sort_index.unwrap_or(1);

    if array.is_empty() {
        return Ok(array.clone());
    }

    if !by_col {
        // Sort rows by `array[*, sort_index_1b - 1]`.
        let key_col = resolve_index("SORT", sort_index_1b, array.cols)?;
        let mut row_order: Vec<usize> = (0..array.rows).collect();
        // Stable sort preserves input order among equal keys.
        row_order.sort_by(|&i, &j| {
            cmp_key(*array.get(i, key_col), *array.get(j, key_col), order)
        });
        let mut data = Vec::with_capacity(array.rows * array.cols);
        for r in row_order {
            for c in 0..array.cols {
                data.push(*array.get(r, c));
            }
        }
        Array2D::new(array.rows, array.cols, data)
    } else {
        // Sort columns by `array[sort_index_1b - 1, *]`.
        let key_row = resolve_index("SORT", sort_index_1b, array.rows)?;
        let mut col_order: Vec<usize> = (0..array.cols).collect();
        col_order.sort_by(|&i, &j| {
            cmp_key(*array.get(key_row, i), *array.get(key_row, j), order)
        });
        let mut data = vec![0.0; array.rows * array.cols];
        for (out_c, src_c) in col_order.iter().enumerate() {
            for r in 0..array.rows {
                data[r * array.cols + out_c] = *array.get(r, *src_c);
            }
        }
        Array2D::new(array.rows, array.cols, data)
    }
}

/// A single (key array, order) pair for `sort_by`.
pub struct SortKey<'a> {
    pub by: &'a Array2D<f64>,
    pub order: Order,
}

/// `SORTBY(array, by_array1, [order1], by_array2, [order2], ...)` — Excel 365.
///
/// Sorts rows of `array` by one or more separate key arrays. Each key array
/// must have exactly `array.rows` cells. Sort is stable across keys, so
/// earlier keys are dominant and later keys break ties (lexicographic order).
///
/// We cap at 6 keys for Phase 1 (Excel allows up to 128); spreadsheet
/// frontends almost never use more than a handful.
pub fn sort_by(array: &Array2D<f64>, keys: &[SortKey<'_>]) -> Result<Array2D<f64>, ArrayError> {
    if keys.is_empty() {
        return Err(ArrayError::BadParameter {
            name: "keys",
            value: "(none)".into(),
        });
    }
    if keys.len() > 6 {
        return Err(ArrayError::BadParameter {
            name: "keys",
            value: format!("{} (max 6 for Phase 1)", keys.len()),
        });
    }
    for (i, k) in keys.iter().enumerate() {
        if k.by.data.len() != array.rows {
            return Err(ArrayError::ShapeMismatch {
                expected: format!("key {i} length = {}", array.rows),
                found: format!("{}", k.by.data.len()),
            });
        }
    }
    if array.is_empty() {
        return Ok(array.clone());
    }

    let mut row_order: Vec<usize> = (0..array.rows).collect();
    row_order.sort_by(|&i, &j| {
        for key in keys {
            let cmp = cmp_key(key.by.data[i], key.by.data[j], key.order);
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
        }
        // All keys equal — stable, so report Equal and let sort preserve
        // the original index order.
        std::cmp::Ordering::Equal
    });
    let mut data = Vec::with_capacity(array.rows * array.cols);
    for r in row_order {
        for c in 0..array.cols {
            data.push(*array.get(r, c));
        }
    }
    Array2D::new(array.rows, array.cols, data)
}
