//! Integration tests for `lookup-core`.  These exercise public APIs end to
//! end and cross-check that the various lookup paths agree with each other
//! and with documented Excel behaviour.

use lookup_core::choose::choose;
use lookup_core::index_match::{index_1d, index_2d, r#match, IndexResult, MatchType};
use lookup_core::offset::offset;
use lookup_core::position::{column, columns, row, rows, Shape};
use lookup_core::vlookup::{hlookup, vlookup};
use lookup_core::xlookup::{xlookup, xmatch, XMatchMode, XSearchMode};
use lookup_core::{LookupError, LookupValue};

fn t(s: &str) -> LookupValue {
    LookupValue::Text(s.into())
}
fn n(x: f64) -> LookupValue {
    LookupValue::Number(x)
}

#[test]
fn vlookup_index_match_agree_on_simple_hit() {
    // A canonical Excel parity check: VLOOKUP and INDEX/MATCH should
    // return the same answer for the same problem.
    let table = vec![
        vec![t("apple"), n(1.0)],
        vec![t("banana"), n(2.0)],
        vec![t("cherry"), n(3.0)],
    ];
    let keys: Vec<LookupValue> = table.iter().map(|r| r[0].clone()).collect();
    let vals: Vec<LookupValue> = table.iter().map(|r| r[1].clone()).collect();

    let vlk = vlookup(&t("banana"), &table, 2, false).unwrap();
    let pos = r#match(&t("banana"), &keys, MatchType::Exact).unwrap();
    let idx = index_1d(&vals, pos).unwrap();
    assert_eq!(vlk, idx);
}

#[test]
fn xlookup_matches_vlookup_on_exact() {
    let table = vec![vec![t("a"), n(10.0)], vec![t("b"), n(20.0)]];
    let keys: Vec<LookupValue> = table.iter().map(|r| r[0].clone()).collect();
    let vals: Vec<LookupValue> = table.iter().map(|r| r[1].clone()).collect();
    let v = vlookup(&t("b"), &table, 2, false).unwrap();
    let x = xlookup(
        &t("b"),
        &keys,
        &vals,
        None,
        XMatchMode::Exact,
        XSearchMode::FirstToLast,
    )
    .unwrap();
    assert_eq!(v, x);
}

#[test]
fn full_grid_navigation_with_offset_and_index() {
    // 3x3 grid; OFFSET extracts middle cell, INDEX agrees.
    let grid = vec![
        vec![n(1.0), n(2.0), n(3.0)],
        vec![n(4.0), n(5.0), n(6.0)],
        vec![n(7.0), n(8.0), n(9.0)],
    ];
    let off = offset(&grid, 1, 1, 1, 1, None, None).unwrap();
    match index_2d(&grid, 2, 2).unwrap() {
        IndexResult::Scalar(v) => assert_eq!(off, vec![vec![v]]),
        _ => panic!(),
    }
}

#[test]
fn position_helpers_round_trip() {
    let shape = Shape::Matrix { rows: 4, cols: 6 };
    assert_eq!(rows(shape), 4);
    assert_eq!(columns(shape), 6);
    assert_eq!(row(shape).len(), 4);
    assert_eq!(column(shape).len(), 6);
}

#[test]
fn choose_dispatches_to_correct_argument() {
    let values = vec![t("alpha"), t("beta"), t("gamma")];
    assert_eq!(choose(2, &values).unwrap(), t("beta"));
}

#[test]
fn hlookup_and_vlookup_are_axis_symmetric() {
    let row_oriented = vec![
        vec![t("a"), t("b"), t("c")],
        vec![n(1.0), n(2.0), n(3.0)],
    ];
    let col_oriented = vec![
        vec![t("a"), n(1.0)],
        vec![t("b"), n(2.0)],
        vec![t("c"), n(3.0)],
    ];
    assert_eq!(
        hlookup(&t("c"), &row_oriented, 2, false).unwrap(),
        vlookup(&t("c"), &col_oriented, 2, false).unwrap()
    );
}

#[test]
fn xmatch_wildcard_then_index_resolves_to_value() {
    let keys = vec![t("alpha"), t("beta"), t("gamma")];
    let vals = vec![n(1.0), n(2.0), n(3.0)];
    let pos = xmatch(
        &t("be*"),
        &keys,
        XMatchMode::Wildcard,
        XSearchMode::FirstToLast,
    )
    .unwrap();
    let v = index_1d(&vals, pos).unwrap();
    assert_eq!(v, n(2.0));
}

#[test]
fn type_mismatch_during_approx_lookup() {
    let table = vec![vec![n(1.0), t("a")], vec![n(2.0), t("b")]];
    let err = vlookup(&t("x"), &table, 2, true).unwrap_err();
    assert!(matches!(err, LookupError::TypeMismatch { .. }));
}

#[test]
fn na_in_lookup_value_returns_na() {
    let table = vec![vec![n(1.0), t("a")]];
    let r = vlookup(&LookupValue::na(), &table, 2, false).unwrap();
    assert!(r.is_na());
}

#[test]
fn na_in_lookup_array_is_skipped() {
    let keys = vec![LookupValue::na(), t("b"), LookupValue::na(), t("c")];
    let pos = r#match(&t("c"), &keys, MatchType::Exact).unwrap();
    // Original index: 'c' is at position 4 (1-based).
    assert_eq!(pos, 4);
}

#[test]
fn mixed_types_lookup_column() {
    // VLOOKUP exact match across a column with both numbers and text.
    let table = vec![
        vec![t("apple"), t("fruit")],
        vec![n(42.0), t("answer")],
        vec![t("banana"), t("yellow")],
    ];
    assert_eq!(
        vlookup(&n(42.0), &table, 2, false).unwrap(),
        t("answer")
    );
    assert_eq!(
        vlookup(&t("banana"), &table, 2, false).unwrap(),
        t("yellow")
    );
}

#[test]
fn xlookup_if_not_found_fallback() {
    let keys = vec![t("a"), t("b")];
    let vals = vec![n(1.0), n(2.0)];
    let r = xlookup(
        &t("z"),
        &keys,
        &vals,
        Some(n(-1.0)),
        XMatchMode::Exact,
        XSearchMode::FirstToLast,
    )
    .unwrap();
    assert_eq!(r, n(-1.0));
}
