//! Integration tests for `array-core` Phase 1. Each function is exercised
//! with at least: a happy path, an Excel-365 edge case (negative indices,
//! over-take/over-drop, etc.), an NA case where applicable, and one or two
//! error paths.

use array_core::filter::filter;
use array_core::generate::sequence;
use array_core::na_real;
use array_core::pick::{choose_cols, choose_rows};
use array_core::reshape::{to_col, to_row, wrap_cols, wrap_rows};
use array_core::shape::{drop, expand, take};
use array_core::sort::{sort, sort_by, Order, SortKey};
use array_core::stack::{hstack, vstack};
use array_core::unique::unique;
use array_core::{is_na_real, Array2D, ArrayError};

// ---- helpers ----

fn arr(rows: usize, cols: usize, data: Vec<f64>) -> Array2D<f64> {
    Array2D::new(rows, cols, data).unwrap()
}

/// Assert that `a` and `b` are bit-identical, treating NA cells as equal.
/// (We can't just compare with PartialEq for cell-wise equality because
/// `na_real()` is a NaN bit pattern and `NaN != NaN`.)
fn assert_arr_eq(a: &Array2D<f64>, b: &Array2D<f64>) {
    assert_eq!(a.rows, b.rows, "row count: left={:?} right={:?}", a, b);
    assert_eq!(a.cols, b.cols, "col count: left={:?} right={:?}", a, b);
    for (i, (x, y)) in a.data.iter().zip(b.data.iter()).enumerate() {
        let both_na = is_na_real(*x) && is_na_real(*y);
        assert!(
            both_na || x == y,
            "cell {i}: {x:?} vs {y:?} (rows={}, cols={})",
            a.rows,
            a.cols
        );
    }
}

// ---- Array2D ----

#[test]
fn array2d_new_rejects_shape_mismatch() {
    let err = Array2D::new(2, 3, vec![1.0, 2.0]).unwrap_err();
    matches!(err, ArrayError::ShapeMismatch { .. });
}

#[test]
fn array2d_from_vector_makes_column() {
    let a = Array2D::from_vector(vec![1.0, 2.0, 3.0]);
    assert_eq!(a.rows, 3);
    assert_eq!(a.cols, 1);
}

#[test]
fn array2d_row_and_col() {
    let a = arr(2, 3, vec![1., 2., 3., 4., 5., 6.]);
    assert_eq!(a.row(0), vec![1., 2., 3.]);
    assert_eq!(a.row(1), vec![4., 5., 6.]);
    assert_eq!(a.col(0), vec![1., 4.]);
    assert_eq!(a.col(2), vec![3., 6.]);
}

#[test]
fn array2d_is_na() {
    let a = arr(1, 2, vec![1.0, na_real()]);
    assert!(!a.is_na(0, 0));
    assert!(a.is_na(0, 1));
}

#[test]
fn array2d_empty_is_empty() {
    let a = Array2D::<f64>::new(0, 0, vec![]).unwrap();
    assert!(a.is_empty());
    let b = Array2D::<f64>::new(0, 3, vec![]).unwrap();
    assert!(b.is_empty());
}

// ---- SEQUENCE ----

#[test]
fn sequence_default_1d() {
    let out = sequence(3, None, None, None).unwrap();
    assert_arr_eq(&out, &arr(3, 1, vec![1., 2., 3.]));
}

#[test]
fn sequence_2d() {
    let out = sequence(2, Some(3), None, None).unwrap();
    assert_arr_eq(&out, &arr(2, 3, vec![1., 2., 3., 4., 5., 6.]));
}

#[test]
fn sequence_custom_start_step() {
    let out = sequence(3, None, Some(0.0), Some(0.5)).unwrap();
    assert_arr_eq(&out, &arr(3, 1, vec![0.0, 0.5, 1.0]));
}

#[test]
fn sequence_negative_step() {
    let out = sequence(3, None, Some(5.0), Some(-1.0)).unwrap();
    assert_arr_eq(&out, &arr(3, 1, vec![5.0, 4.0, 3.0]));
}

#[test]
fn sequence_zero_rows_rejected() {
    let err = sequence(0, None, None, None).unwrap_err();
    assert!(matches!(err, ArrayError::BadParameter { name: "rows", .. }));
}

#[test]
fn sequence_zero_cols_rejected() {
    let err = sequence(3, Some(0), None, None).unwrap_err();
    assert!(matches!(err, ArrayError::BadParameter { name: "cols", .. }));
}

// ---- TAKE ----

#[test]
fn take_positive_rows() {
    let a = arr(3, 2, vec![1., 2., 3., 4., 5., 6.]);
    let out = take(&a, 2, None).unwrap();
    assert_arr_eq(&out, &arr(2, 2, vec![1., 2., 3., 4.]));
}

#[test]
fn take_negative_rows() {
    let a = arr(3, 2, vec![1., 2., 3., 4., 5., 6.]);
    let out = take(&a, -2, None).unwrap();
    assert_arr_eq(&out, &arr(2, 2, vec![3., 4., 5., 6.]));
}

#[test]
fn take_rows_and_cols() {
    let a = arr(3, 3, vec![1., 2., 3., 4., 5., 6., 7., 8., 9.]);
    let out = take(&a, 2, Some(2)).unwrap();
    assert_arr_eq(&out, &arr(2, 2, vec![1., 2., 4., 5.]));
}

#[test]
fn take_negative_cols() {
    let a = arr(2, 3, vec![1., 2., 3., 4., 5., 6.]);
    let out = take(&a, 2, Some(-1)).unwrap();
    assert_arr_eq(&out, &arr(2, 1, vec![3., 6.]));
}

#[test]
fn take_exceeds_clamps() {
    let a = arr(2, 2, vec![1., 2., 3., 4.]);
    let out = take(&a, 99, Some(99)).unwrap();
    assert_arr_eq(&out, &a);
}

#[test]
fn take_zero_yields_empty() {
    let a = arr(2, 2, vec![1., 2., 3., 4.]);
    let out = take(&a, 0, None).unwrap();
    assert_eq!(out.rows, 0);
    assert_eq!(out.cols, 2);
}

// ---- DROP ----

#[test]
fn drop_positive() {
    let a = arr(3, 2, vec![1., 2., 3., 4., 5., 6.]);
    let out = drop(&a, 1, None).unwrap();
    assert_arr_eq(&out, &arr(2, 2, vec![3., 4., 5., 6.]));
}

#[test]
fn drop_negative() {
    let a = arr(3, 2, vec![1., 2., 3., 4., 5., 6.]);
    let out = drop(&a, -1, None).unwrap();
    assert_arr_eq(&out, &arr(2, 2, vec![1., 2., 3., 4.]));
}

#[test]
fn drop_cols() {
    let a = arr(2, 3, vec![1., 2., 3., 4., 5., 6.]);
    let out = drop(&a, 0, Some(1)).unwrap();
    assert_arr_eq(&out, &arr(2, 2, vec![2., 3., 5., 6.]));
}

#[test]
fn drop_over_yields_empty() {
    let a = arr(3, 2, vec![1., 2., 3., 4., 5., 6.]);
    let out = drop(&a, 99, None).unwrap();
    assert_eq!(out.rows, 0);
    assert_eq!(out.cols, 2);
}

// ---- EXPAND ----

#[test]
fn expand_pads_with_na_by_default() {
    let a = arr(1, 1, vec![7.0]);
    let out = expand(&a, 2, Some(2), None).unwrap();
    assert!(!is_na_real(*out.get(0, 0)));
    assert_eq!(*out.get(0, 0), 7.0);
    assert!(is_na_real(*out.get(0, 1)));
    assert!(is_na_real(*out.get(1, 0)));
    assert!(is_na_real(*out.get(1, 1)));
}

#[test]
fn expand_with_custom_pad() {
    let a = arr(1, 1, vec![7.0]);
    let out = expand(&a, 2, Some(2), Some(-1.0)).unwrap();
    assert_arr_eq(&out, &arr(2, 2, vec![7.0, -1.0, -1.0, -1.0]));
}

#[test]
fn expand_noop_if_already_sized() {
    let a = arr(2, 2, vec![1., 2., 3., 4.]);
    let out = expand(&a, 2, Some(2), None).unwrap();
    assert_arr_eq(&out, &a);
}

#[test]
fn expand_shrink_rejected() {
    let a = arr(2, 2, vec![1., 2., 3., 4.]);
    let err = expand(&a, 1, Some(1), None).unwrap_err();
    assert!(matches!(err, ArrayError::BadParameter { .. }));
}

// ---- HSTACK / VSTACK ----

#[test]
fn hstack_equal_rows() {
    let a = arr(2, 1, vec![1., 2.]);
    let b = arr(2, 2, vec![10., 20., 30., 40.]);
    let out = hstack(&[&a, &b]).unwrap();
    assert_arr_eq(&out, &arr(2, 3, vec![1., 10., 20., 2., 30., 40.]));
}

#[test]
fn hstack_mismatched_rows_pads_na() {
    let a = arr(2, 1, vec![1., 2.]);
    let b = arr(3, 1, vec![10., 20., 30.]);
    let out = hstack(&[&a, &b]).unwrap();
    assert_eq!(out.rows, 3);
    assert_eq!(out.cols, 2);
    assert_eq!(*out.get(0, 0), 1.0);
    assert_eq!(*out.get(1, 0), 2.0);
    assert!(is_na_real(*out.get(2, 0)));
    assert_eq!(*out.get(2, 1), 30.0);
}

#[test]
fn hstack_empty_input_returns_empty() {
    let out = hstack(&[]).unwrap();
    assert!(out.is_empty());
}

#[test]
fn vstack_equal_cols() {
    let a = arr(1, 2, vec![1., 2.]);
    let b = arr(2, 2, vec![10., 20., 30., 40.]);
    let out = vstack(&[&a, &b]).unwrap();
    assert_arr_eq(&out, &arr(3, 2, vec![1., 2., 10., 20., 30., 40.]));
}

#[test]
fn vstack_mismatched_cols_pads_na() {
    let a = arr(1, 2, vec![1., 2.]);
    let b = arr(1, 3, vec![10., 20., 30.]);
    let out = vstack(&[&a, &b]).unwrap();
    assert_eq!(out.rows, 2);
    assert_eq!(out.cols, 3);
    assert!(is_na_real(*out.get(0, 2)));
    assert_eq!(*out.get(1, 2), 30.0);
}

// ---- TOROW / TOCOL ----

#[test]
fn to_row_flattens_row_major() {
    let a = arr(2, 3, vec![1., 2., 3., 4., 5., 6.]);
    let out = to_row(&a, None, None).unwrap();
    assert_arr_eq(&out, &arr(1, 6, vec![1., 2., 3., 4., 5., 6.]));
}

#[test]
fn to_row_scan_by_column() {
    let a = arr(2, 3, vec![1., 2., 3., 4., 5., 6.]);
    let out = to_row(&a, None, Some(true)).unwrap();
    assert_arr_eq(&out, &arr(1, 6, vec![1., 4., 2., 5., 3., 6.]));
}

#[test]
fn to_row_ignore_blanks_drops_na() {
    let a = arr(1, 3, vec![1.0, na_real(), 3.0]);
    let out = to_row(&a, Some(1), None).unwrap();
    assert_arr_eq(&out, &arr(1, 2, vec![1.0, 3.0]));
}

#[test]
fn to_col_flattens() {
    let a = arr(2, 2, vec![1., 2., 3., 4.]);
    let out = to_col(&a, None, None).unwrap();
    assert_arr_eq(&out, &arr(4, 1, vec![1., 2., 3., 4.]));
}

#[test]
fn to_col_ignore_skip_both() {
    let a = arr(2, 2, vec![1.0, na_real(), na_real(), 4.0]);
    let out = to_col(&a, Some(3), None).unwrap();
    assert_arr_eq(&out, &arr(2, 1, vec![1.0, 4.0]));
}

#[test]
fn to_row_invalid_ignore_code() {
    let a = arr(1, 1, vec![1.0]);
    let err = to_row(&a, Some(9), None).unwrap_err();
    assert!(matches!(err, ArrayError::BadParameter { name: "ignore", .. }));
}

// ---- WRAPROWS / WRAPCOLS ----

#[test]
fn wrap_rows_even() {
    let v = arr(1, 6, vec![1., 2., 3., 4., 5., 6.]);
    let out = wrap_rows(&v, 3, None).unwrap();
    assert_arr_eq(&out, &arr(2, 3, vec![1., 2., 3., 4., 5., 6.]));
}

#[test]
fn wrap_rows_pads() {
    let v = arr(1, 5, vec![1., 2., 3., 4., 5.]);
    let out = wrap_rows(&v, 3, Some(0.0)).unwrap();
    assert_arr_eq(&out, &arr(2, 3, vec![1., 2., 3., 4., 5., 0.0]));
}

#[test]
fn wrap_rows_zero_count_rejected() {
    let v = arr(1, 3, vec![1., 2., 3.]);
    let err = wrap_rows(&v, 0, None).unwrap_err();
    assert!(matches!(err, ArrayError::BadParameter { name: "wrap_count", .. }));
}

#[test]
fn wrap_cols_even() {
    let v = arr(1, 6, vec![1., 2., 3., 4., 5., 6.]);
    let out = wrap_cols(&v, 3, None).unwrap();
    // 3 rows tall, 2 cols. Column-by-column fill: col0 = 1,2,3; col1 = 4,5,6.
    assert_arr_eq(&out, &arr(3, 2, vec![1., 4., 2., 5., 3., 6.]));
}

#[test]
fn wrap_cols_pads() {
    let v = arr(1, 5, vec![1., 2., 3., 4., 5.]);
    let out = wrap_cols(&v, 3, Some(0.0)).unwrap();
    // 3 rows, 2 cols. col0 = 1,2,3; col1 = 4,5,pad.
    assert_arr_eq(&out, &arr(3, 2, vec![1., 4., 2., 5., 3., 0.0]));
}

// ---- CHOOSEROWS / CHOOSECOLS ----

#[test]
fn choose_rows_positive_and_negative() {
    let a = arr(3, 2, vec![1., 2., 3., 4., 5., 6.]);
    let out = choose_rows(&a, &[1, -1]).unwrap();
    assert_arr_eq(&out, &arr(2, 2, vec![1., 2., 5., 6.]));
}

#[test]
fn choose_rows_repeats() {
    let a = arr(2, 1, vec![1., 2.]);
    let out = choose_rows(&a, &[1, 1, 2]).unwrap();
    assert_arr_eq(&out, &arr(3, 1, vec![1., 1., 2.]));
}

#[test]
fn choose_rows_zero_index_errors() {
    let a = arr(2, 1, vec![1., 2.]);
    let err = choose_rows(&a, &[0]).unwrap_err();
    assert!(matches!(err, ArrayError::OutOfRange { index: 0, .. }));
}

#[test]
fn choose_rows_out_of_range() {
    let a = arr(2, 1, vec![1., 2.]);
    let err = choose_rows(&a, &[3]).unwrap_err();
    assert!(matches!(err, ArrayError::OutOfRange { .. }));
}

#[test]
fn choose_cols_positive_and_negative() {
    let a = arr(2, 3, vec![1., 2., 3., 4., 5., 6.]);
    let out = choose_cols(&a, &[3, -3]).unwrap();
    assert_arr_eq(&out, &arr(2, 2, vec![3., 1., 6., 4.]));
}

#[test]
fn choose_cols_empty_indices_errors() {
    let a = arr(2, 1, vec![1., 2.]);
    let err = choose_cols(&a, &[]).unwrap_err();
    assert!(matches!(err, ArrayError::BadParameter { .. }));
}

// ---- FILTER ----

#[test]
fn filter_matching_rows() {
    let a = arr(3, 2, vec![1., 2., 3., 4., 5., 6.]);
    let mask = Array2D::from_vector(vec![1.0, 0.0, 1.0]);
    let out = filter(&a, &mask, None).unwrap();
    assert_arr_eq(&out, &arr(2, 2, vec![1., 2., 5., 6.]));
}

#[test]
fn filter_no_match_with_if_empty() {
    let a = arr(2, 1, vec![1., 2.]);
    let mask = Array2D::from_vector(vec![0.0, 0.0]);
    let out = filter(&a, &mask, Some(-1.0)).unwrap();
    assert_arr_eq(&out, &arr(1, 1, vec![-1.0]));
}

#[test]
fn filter_no_match_without_if_empty_errors() {
    let a = arr(2, 1, vec![1., 2.]);
    let mask = Array2D::from_vector(vec![0.0, 0.0]);
    let err = filter(&a, &mask, None).unwrap_err();
    assert!(matches!(err, ArrayError::EmptyResult { function: "FILTER" }));
}

#[test]
fn filter_na_in_mask_treated_as_false() {
    let a = arr(2, 1, vec![10., 20.]);
    let mask = Array2D::from_vector(vec![1.0, na_real()]);
    let out = filter(&a, &mask, None).unwrap();
    assert_arr_eq(&out, &arr(1, 1, vec![10.]));
}

#[test]
fn filter_mask_wrong_length() {
    let a = arr(2, 1, vec![1., 2.]);
    let mask = Array2D::from_vector(vec![1.0]);
    let err = filter(&a, &mask, None).unwrap_err();
    assert!(matches!(err, ArrayError::ShapeMismatch { .. }));
}

// ---- SORT ----

#[test]
fn sort_rows_ascending() {
    let a = arr(3, 2, vec![3., 1., 1., 2., 2., 3.]);
    let out = sort(&a, None, None, None).unwrap();
    // Sort by column 1 (default): keys = 3,1,2 -> order 1,2,3.
    assert_arr_eq(&out, &arr(3, 2, vec![1., 2., 2., 3., 3., 1.]));
}

#[test]
fn sort_rows_descending() {
    let a = arr(3, 2, vec![3., 1., 1., 2., 2., 3.]);
    let out = sort(&a, None, Some(-1), None).unwrap();
    assert_arr_eq(&out, &arr(3, 2, vec![3., 1., 2., 3., 1., 2.]));
}

#[test]
fn sort_by_col_axis() {
    let a = arr(2, 3, vec![3., 1., 2., 30., 10., 20.]);
    let out = sort(&a, Some(1), None, Some(true)).unwrap();
    // Sort columns by row 1: keys = 3,1,2 -> col order 1,2,0.
    assert_arr_eq(&out, &arr(2, 3, vec![1., 2., 3., 10., 20., 30.]));
}

#[test]
fn sort_na_sorts_to_end_asc() {
    let a = arr(3, 1, vec![2.0, na_real(), 1.0]);
    let out = sort(&a, None, None, None).unwrap();
    assert_eq!(*out.get(0, 0), 1.0);
    assert_eq!(*out.get(1, 0), 2.0);
    assert!(is_na_real(*out.get(2, 0)));
}

#[test]
fn sort_bad_order_rejected() {
    let a = arr(1, 1, vec![1.0]);
    let err = sort(&a, None, Some(5), None).unwrap_err();
    assert!(matches!(err, ArrayError::BadParameter { name: "sort_order", .. }));
}

#[test]
fn sort_empty_is_empty() {
    let a = Array2D::<f64>::new(0, 0, vec![]).unwrap();
    let out = sort(&a, None, None, None).unwrap();
    assert!(out.is_empty());
}

// ---- SORTBY ----

#[test]
fn sort_by_two_keys() {
    let a = arr(4, 1, vec![10., 20., 30., 40.]);
    // Primary: A,B,A,B -> after asc on letters, A's then B's.
    // We encode A=1, B=2.
    let primary = Array2D::from_vector(vec![1.0, 2.0, 1.0, 2.0]);
    // Secondary: 2,1,1,2 -> within group, ascending.
    let secondary = Array2D::from_vector(vec![2.0, 1.0, 1.0, 2.0]);
    let keys = vec![
        SortKey {
            by: &primary,
            order: Order::Ascending,
        },
        SortKey {
            by: &secondary,
            order: Order::Ascending,
        },
    ];
    let out = sort_by(&a, &keys).unwrap();
    // Groups: A={row0(2), row2(1)} -> sorted: row2(1), row0(2) -> 30, 10.
    //         B={row1(1), row3(2)} -> sorted: row1(1), row3(2) -> 20, 40.
    assert_arr_eq(&out, &arr(4, 1, vec![30., 10., 20., 40.]));
}

#[test]
fn sort_by_rejects_empty_keys() {
    let a = arr(1, 1, vec![1.0]);
    let err = sort_by(&a, &[]).unwrap_err();
    assert!(matches!(err, ArrayError::BadParameter { name: "keys", .. }));
}

#[test]
fn sort_by_wrong_key_length() {
    let a = arr(2, 1, vec![1., 2.]);
    let bad_key = Array2D::from_vector(vec![1.0]);
    let keys = vec![SortKey {
        by: &bad_key,
        order: Order::Ascending,
    }];
    let err = sort_by(&a, &keys).unwrap_err();
    assert!(matches!(err, ArrayError::ShapeMismatch { .. }));
}

#[test]
fn sort_by_caps_at_six_keys() {
    let a = arr(1, 1, vec![1.0]);
    let k = Array2D::from_vector(vec![1.0]);
    let keys: Vec<SortKey> = (0..7)
        .map(|_| SortKey {
            by: &k,
            order: Order::Ascending,
        })
        .collect();
    let err = sort_by(&a, &keys).unwrap_err();
    assert!(matches!(err, ArrayError::BadParameter { name: "keys", .. }));
}

// ---- UNIQUE ----

#[test]
fn unique_rows_dedupe() {
    let a = arr(4, 2, vec![1., 2., 3., 4., 1., 2., 5., 6.]);
    let out = unique(&a, None, None).unwrap();
    assert_arr_eq(&out, &arr(3, 2, vec![1., 2., 3., 4., 5., 6.]));
}

#[test]
fn unique_na_collapses() {
    let a = arr(3, 1, vec![na_real(), 1.0, na_real()]);
    let out = unique(&a, None, None).unwrap();
    // NA appears once (first), then 1.0.
    assert_eq!(out.rows, 2);
    assert!(is_na_real(*out.get(0, 0)));
    assert_eq!(*out.get(1, 0), 1.0);
}

#[test]
fn unique_exactly_once() {
    let a = arr(4, 1, vec![1., 2., 1., 3.]);
    let out = unique(&a, None, Some(true)).unwrap();
    // 1 appears twice -> dropped. 2 and 3 appear once.
    assert_arr_eq(&out, &arr(2, 1, vec![2., 3.]));
}

#[test]
fn unique_by_col() {
    let a = arr(2, 3, vec![1., 2., 1., 3., 4., 3.]);
    let out = unique(&a, Some(true), None).unwrap();
    // Cols: [1;3], [2;4], [1;3] -> unique: col0, col1.
    assert_arr_eq(&out, &arr(2, 2, vec![1., 2., 3., 4.]));
}

#[test]
fn unique_exactly_once_all_dupes() {
    let a = arr(2, 1, vec![1., 1.]);
    let out = unique(&a, None, Some(true)).unwrap();
    assert_eq!(out.rows, 0);
    assert_eq!(out.cols, 1);
}

// ---- Edge cases ----

#[test]
fn all_na_array_sorts_unchanged() {
    let a = arr(2, 1, vec![na_real(), na_real()]);
    let out = sort(&a, None, None, None).unwrap();
    assert!(is_na_real(*out.get(0, 0)));
    assert!(is_na_real(*out.get(1, 0)));
}

#[test]
fn single_element_round_trip() {
    let a = arr(1, 1, vec![42.0]);
    let out = take(&a, 1, Some(1)).unwrap();
    assert_arr_eq(&out, &a);
}

#[test]
fn error_display_is_informative() {
    let e = ArrayError::OutOfRange {
        function: "X",
        index: -5,
        max: 2,
    };
    let s = format!("{e}");
    assert!(s.contains("X"));
    assert!(s.contains("-5"));
    assert!(s.contains("2"));
}
