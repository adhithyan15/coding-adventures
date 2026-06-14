//! UNIQUE — distinct rows (or columns) of an array.
//!
//! # NA handling
//!
//! Two NA values compare equal here (so duplicate NA rows collapse into
//! one). This matches Excel: `=UNIQUE({#N/A; #N/A; 1})` -> `{#N/A; 1}`.
//! For non-NA cells we compare bitwise via `f64::to_bits`, so `+0.0` and
//! `-0.0` are considered distinct only if their bit patterns differ. This
//! is a deliberate trade: bit-level equality is the only definition that
//! makes NaN deduplication well-behaved.
//!
//! # exactly_once
//!
//! With `exactly_once = true` we return only items that appear in the input
//! exactly once. Items appearing 2+ times are dropped entirely (not even one
//! representative). This matches Excel's documented behavior.

use crate::{is_na_real, Array2D, ArrayError};
use std::collections::HashMap;

/// Hash key for a row (or column): a Vec<u64> of bit patterns, with NA
/// normalized to a single canonical value so all NAs collide in the table.
fn key_of(values: &[f64]) -> Vec<u64> {
    const NA_CANON: u64 = u64::MAX; // sentinel distinct from any real f64
    values
        .iter()
        .map(|&v| if is_na_real(v) { NA_CANON } else { v.to_bits() })
        .collect()
}

/// `UNIQUE(array, [by_col], [exactly_once])` — Excel 365.
///
/// * `by_col = false` (default): return unique rows.
/// * `by_col = true`: return unique columns.
/// * `exactly_once = true`: only items appearing exactly once survive.
pub fn unique(
    array: &Array2D<f64>,
    by_col: Option<bool>,
    exactly_once: Option<bool>,
) -> Result<Array2D<f64>, ArrayError> {
    let by_col = by_col.unwrap_or(false);
    let exactly_once = exactly_once.unwrap_or(false);

    if array.is_empty() {
        return Ok(array.clone());
    }

    if !by_col {
        // Iterate rows, keep first occurrences, count to support
        // `exactly_once`.
        let mut order: Vec<usize> = Vec::new();
        let mut counts: HashMap<Vec<u64>, usize> = HashMap::new();
        let mut first_pos: HashMap<Vec<u64>, usize> = HashMap::new();
        for r in 0..array.rows {
            let row = array.row(r);
            let k = key_of(&row);
            *counts.entry(k.clone()).or_insert(0) += 1;
            first_pos.entry(k.clone()).or_insert_with(|| {
                order.push(r);
                r
            });
        }
        let kept_rows: Vec<usize> = order
            .into_iter()
            .filter(|r| {
                let k = key_of(&array.row(*r));
                if exactly_once {
                    counts.get(&k) == Some(&1)
                } else {
                    true
                }
            })
            .collect();
        if kept_rows.is_empty() {
            return Array2D::new(0, array.cols, vec![]);
        }
        let mut data = Vec::with_capacity(kept_rows.len() * array.cols);
        for r in &kept_rows {
            for c in 0..array.cols {
                data.push(*array.get(*r, c));
            }
        }
        Array2D::new(kept_rows.len(), array.cols, data)
    } else {
        // Same logic mirrored for columns.
        let mut order: Vec<usize> = Vec::new();
        let mut counts: HashMap<Vec<u64>, usize> = HashMap::new();
        let mut first_pos: HashMap<Vec<u64>, usize> = HashMap::new();
        for c in 0..array.cols {
            let col = array.col(c);
            let k = key_of(&col);
            *counts.entry(k.clone()).or_insert(0) += 1;
            first_pos.entry(k.clone()).or_insert_with(|| {
                order.push(c);
                c
            });
        }
        let kept_cols: Vec<usize> = order
            .into_iter()
            .filter(|c| {
                let k = key_of(&array.col(*c));
                if exactly_once {
                    counts.get(&k) == Some(&1)
                } else {
                    true
                }
            })
            .collect();
        if kept_cols.is_empty() {
            return Array2D::new(array.rows, 0, vec![]);
        }
        let out_cols = kept_cols.len();
        let mut data = vec![0.0; array.rows * out_cols];
        for (out_c, src_c) in kept_cols.iter().enumerate() {
            for r in 0..array.rows {
                data[r * out_cols + out_c] = *array.get(r, *src_c);
            }
        }
        Array2D::new(array.rows, out_cols, data)
    }
}
