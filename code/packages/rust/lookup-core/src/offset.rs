//! OFFSET — range arithmetic.
//!
//! # Simplification vs Excel
//!
//! In Excel, `OFFSET` returns a **reference** to a shifted range.  We don't
//! have a `Reference` / `RangeRef` type in this Layer-1 crate (those live in
//! `spreadsheet-core`, which doesn't exist yet), so we model OFFSET as a
//! function that:
//!
//! 1. Takes a 2-D table (the implicit workbook patch surrounding the
//!    anchor) plus an `anchor_row` / `anchor_col` (1-based) pointing at the
//!    top-left of the *reference*.
//! 2. Applies the row / column offsets and optional height / width to
//!    extract a sub-table from that 2-D table.
//!
//! Callers that have richer reference types can wrap this function easily.

use crate::{LookupError, LookupResult, LookupValue};

/// `OFFSET(reference, rows, cols, height, width)` — extract a rectangular
/// sub-table.
///
/// `anchor_row` / `anchor_col` are 1-based, identifying the top-left of the
/// original reference inside `table`.  `height` / `width` default to the
/// reference's own height/width when `None`; for our purposes the original
/// reference is exactly one cell, so the defaults are 1×1.
pub fn offset(
    table: &[Vec<LookupValue>],
    anchor_row: i64,
    anchor_col: i64,
    rows: i64,
    cols: i64,
    height: Option<i64>,
    width: Option<i64>,
) -> LookupResult<Vec<Vec<LookupValue>>> {
    let n_rows = table.len() as i64;
    let n_cols = if n_rows == 0 { 0 } else { table[0].len() as i64 };

    let h = height.unwrap_or(1);
    let w = width.unwrap_or(1);
    if h <= 0 {
        return Err(LookupError::BadParameter {
            name: "height",
            value: h.to_string(),
        });
    }
    if w <= 0 {
        return Err(LookupError::BadParameter {
            name: "width",
            value: w.to_string(),
        });
    }

    // New 1-based top-left after applying offsets.
    let new_top_1 = anchor_row + rows;
    let new_left_1 = anchor_col + cols;
    // 1-based inclusive bottom-right.
    let new_bot_1 = new_top_1 + h - 1;
    let new_right_1 = new_left_1 + w - 1;

    if new_top_1 < 1 || new_left_1 < 1 || new_bot_1 > n_rows || new_right_1 > n_cols {
        return Err(LookupError::OutOfRange {
            function: "OFFSET",
            index: new_top_1,
            max: n_rows.max(0) as usize,
        });
    }

    let mut out: Vec<Vec<LookupValue>> = Vec::with_capacity(h as usize);
    for r in new_top_1..=new_bot_1 {
        let row = &table[(r - 1) as usize];
        let slice: Vec<LookupValue> = row[(new_left_1 - 1) as usize..new_right_1 as usize]
            .to_vec();
        out.push(slice);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(x: f64) -> LookupValue {
        LookupValue::Number(x)
    }

    fn small_table() -> Vec<Vec<LookupValue>> {
        // 3x3 grid:
        //   1 2 3
        //   4 5 6
        //   7 8 9
        vec![
            vec![n(1.0), n(2.0), n(3.0)],
            vec![n(4.0), n(5.0), n(6.0)],
            vec![n(7.0), n(8.0), n(9.0)],
        ]
    }

    #[test]
    fn offset_zero_zero_returns_anchor_cell() {
        let t = small_table();
        let r = offset(&t, 2, 2, 0, 0, None, None).unwrap();
        assert_eq!(r, vec![vec![n(5.0)]]);
    }

    #[test]
    fn offset_positive_offsets_pick_new_cell() {
        let t = small_table();
        let r = offset(&t, 1, 1, 1, 1, None, None).unwrap();
        assert_eq!(r, vec![vec![n(5.0)]]);
    }

    #[test]
    fn offset_with_height_and_width_returns_block() {
        let t = small_table();
        // From anchor (1,1), shift 1 down 1 right, then take 2x2.
        let r = offset(&t, 1, 1, 1, 1, Some(2), Some(2)).unwrap();
        assert_eq!(r, vec![vec![n(5.0), n(6.0)], vec![n(8.0), n(9.0)]]);
    }

    #[test]
    fn offset_negative_offsets() {
        let t = small_table();
        // Anchor at (3,3), offset by (-2,-2) → (1,1).
        let r = offset(&t, 3, 3, -2, -2, None, None).unwrap();
        assert_eq!(r, vec![vec![n(1.0)]]);
    }

    #[test]
    fn offset_out_of_bounds_top() {
        let t = small_table();
        let err = offset(&t, 1, 1, -1, 0, None, None).unwrap_err();
        assert!(matches!(err, LookupError::OutOfRange { .. }));
    }

    #[test]
    fn offset_out_of_bounds_height() {
        let t = small_table();
        let err = offset(&t, 1, 1, 0, 0, Some(5), Some(1)).unwrap_err();
        assert!(matches!(err, LookupError::OutOfRange { .. }));
    }

    #[test]
    fn offset_zero_height_is_bad_parameter() {
        let t = small_table();
        let err = offset(&t, 1, 1, 0, 0, Some(0), Some(1)).unwrap_err();
        assert!(matches!(err, LookupError::BadParameter { name: "height", .. }));
    }

    #[test]
    fn offset_negative_width_is_bad_parameter() {
        let t = small_table();
        let err = offset(&t, 1, 1, 0, 0, Some(1), Some(-2)).unwrap_err();
        assert!(matches!(err, LookupError::BadParameter { name: "width", .. }));
    }

    #[test]
    fn offset_full_row_extraction() {
        let t = small_table();
        let r = offset(&t, 1, 1, 1, 0, Some(1), Some(3)).unwrap();
        assert_eq!(r, vec![vec![n(4.0), n(5.0), n(6.0)]]);
    }
}
