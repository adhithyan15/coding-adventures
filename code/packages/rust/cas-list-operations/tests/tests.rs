// Integration tests for cas-list-operations.
//
// Mirrors the Python reference tests in
// code/packages/python/cas-list-operations/tests/.

use cas_list_operations::{
    append, apply_, first, flatten, join, last, length, make_list,
    map_, part, range_, rest, reverse, select, sort_, ListOperationError,
};
use symbolic_ir::{apply, int, sym, ADD};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a List node quickly from integer values.
fn li(vals: &[i64]) -> symbolic_ir::IRNode {
    make_list(vals.iter().map(|&v| int(v)).collect())
}

// ---------------------------------------------------------------------------
// length
// ---------------------------------------------------------------------------

#[test]
fn length_simple() {
    assert_eq!(length(&li(&[1, 2, 3])).unwrap(), int(3));
}

#[test]
fn length_empty() {
    assert_eq!(length(&make_list(vec![])).unwrap(), int(0));
}

#[test]
fn length_non_list_raises() {
    assert!(length(&int(5)).is_err());
}

// ---------------------------------------------------------------------------
// first
// ---------------------------------------------------------------------------

#[test]
fn first_basic() {
    assert_eq!(first(&li(&[7, 8])).unwrap(), int(7));
}

#[test]
fn first_empty_raises() {
    assert!(first(&make_list(vec![])).is_err());
}

#[test]
fn first_error_kind() {
    let err = first(&make_list(vec![])).unwrap_err();
    assert!(matches!(err, ListOperationError(_)));
}

// ---------------------------------------------------------------------------
// rest
// ---------------------------------------------------------------------------

#[test]
fn rest_basic() {
    assert_eq!(rest(&li(&[1, 2, 3])).unwrap(), li(&[2, 3]));
}

#[test]
fn rest_empty_raises() {
    assert!(rest(&make_list(vec![])).is_err());
}

// ---------------------------------------------------------------------------
// last
// ---------------------------------------------------------------------------

#[test]
fn last_basic() {
    assert_eq!(last(&li(&[1, 2, 3])).unwrap(), int(3));
}

#[test]
fn last_empty_raises() {
    assert!(last(&make_list(vec![])).is_err());
}

// ---------------------------------------------------------------------------
// reverse
// ---------------------------------------------------------------------------

#[test]
fn reverse_basic() {
    assert_eq!(reverse(&li(&[1, 2, 3])).unwrap(), li(&[3, 2, 1]));
}

#[test]
fn reverse_empty() {
    assert_eq!(reverse(&make_list(vec![])).unwrap(), make_list(vec![]));
}

// ---------------------------------------------------------------------------
// append / join
// ---------------------------------------------------------------------------

#[test]
fn append_two_lists() {
    let a = li(&[1]);
    let b = li(&[2]);
    assert_eq!(append(&[a, b]).unwrap(), li(&[1, 2]));
}

#[test]
fn append_many() {
    let a = li(&[1]);
    let b = li(&[2, 3]);
    let c = li(&[4]);
    assert_eq!(append(&[a, b, c]).unwrap(), li(&[1, 2, 3, 4]));
}

#[test]
fn append_empty_lists() {
    let a = make_list(vec![]);
    let b = li(&[1]);
    let c = make_list(vec![]);
    assert_eq!(append(&[a, b, c]).unwrap(), li(&[1]));
}

#[test]
fn join_is_alias_for_append() {
    let a = li(&[1]);
    let b = li(&[2]);
    assert_eq!(
        join(&[a.clone(), b.clone()]).unwrap(),
        append(&[a, b]).unwrap()
    );
}

// ---------------------------------------------------------------------------
// part
// ---------------------------------------------------------------------------

#[test]
fn part_first() {
    assert_eq!(part(&li(&[10, 20, 30]), 1).unwrap(), int(10));
}

#[test]
fn part_middle() {
    assert_eq!(part(&li(&[10, 20, 30]), 2).unwrap(), int(20));
}

#[test]
fn part_negative() {
    // part(lst, -1) → last element
    assert_eq!(part(&li(&[10, 20, 30]), -1).unwrap(), int(30));
}

#[test]
fn part_negative_second_to_last() {
    assert_eq!(part(&li(&[10, 20, 30]), -2).unwrap(), int(20));
}

#[test]
fn part_zero_invalid() {
    assert!(part(&li(&[10, 20, 30]), 0).is_err());
}

#[test]
fn part_out_of_range() {
    assert!(part(&li(&[10, 20, 30]), 5).is_err());
}

// ---------------------------------------------------------------------------
// range_
// ---------------------------------------------------------------------------

#[test]
fn range_one_arg() {
    // range_(5) → [1, 2, 3, 4, 5]  (MACSYMA convention)
    assert_eq!(range_(5, None, 1).unwrap(), li(&[1, 2, 3, 4, 5]));
}

#[test]
fn range_zero_arg() {
    // range_(0) → []
    assert_eq!(range_(0, None, 1).unwrap(), make_list(vec![]));
}

#[test]
fn range_two_args() {
    // range_(3, Some(7), 1) → [3, 4, 5, 6, 7]
    assert_eq!(range_(3, Some(7), 1).unwrap(), li(&[3, 4, 5, 6, 7]));
}

#[test]
fn range_with_step() {
    // range_(1, Some(10), 2) → [1, 3, 5, 7, 9]
    assert_eq!(range_(1, Some(10), 2).unwrap(), li(&[1, 3, 5, 7, 9]));
}

#[test]
fn range_negative_step() {
    // range_(10, Some(1), -2) → [10, 8, 6, 4, 2]
    assert_eq!(range_(10, Some(1), -2).unwrap(), li(&[10, 8, 6, 4, 2]));
}

#[test]
fn range_step_zero_raises() {
    assert!(range_(1, Some(5), 0).is_err());
}

#[test]
fn range_empty_two_arg_ascending_impossible() {
    // start > stop with positive step → empty list
    assert_eq!(range_(5, Some(3), 1).unwrap(), make_list(vec![]));
}

// ---------------------------------------------------------------------------
// map_
// ---------------------------------------------------------------------------

#[test]
fn map_with_symbol_head() {
    // map_(f, [1, 2, 3]) → [f(1), f(2), f(3)]
    let f = sym("f");
    let lst = li(&[1, 2, 3]);
    let out = map_(f.clone(), &lst).unwrap();
    let expected = make_list(vec![
        apply(f.clone(), vec![int(1)]),
        apply(f.clone(), vec![int(2)]),
        apply(f.clone(), vec![int(3)]),
    ]);
    assert_eq!(out, expected);
}

#[test]
fn map_empty_list() {
    let f = sym("f");
    let lst = make_list(vec![]);
    assert_eq!(map_(f, &lst).unwrap(), make_list(vec![]));
}

// ---------------------------------------------------------------------------
// apply_
// ---------------------------------------------------------------------------

#[test]
fn apply_replaces_head() {
    // apply_(Add, [1, 2, 3]) → Add(1, 2, 3)
    let lst = li(&[1, 2, 3]);
    let out = apply_(sym(ADD), &lst).unwrap();
    let expected = apply(sym(ADD), vec![int(1), int(2), int(3)]);
    assert_eq!(out, expected);
}

// ---------------------------------------------------------------------------
// select
// ---------------------------------------------------------------------------

#[test]
fn select_filters() {
    // select([1,2,3,4], even?) → [2, 4]
    let lst = li(&[1, 2, 3, 4]);
    let evens = select(&lst, |n| {
        matches!(n, symbolic_ir::IRNode::Integer(v) if v % 2 == 0)
    })
    .unwrap();
    assert_eq!(evens, li(&[2, 4]));
}

#[test]
fn select_drops_all() {
    let lst = li(&[1, 2]);
    assert_eq!(select(&lst, |_| false).unwrap(), make_list(vec![]));
}

#[test]
fn select_keeps_all() {
    let lst = li(&[1, 2, 3]);
    assert_eq!(select(&lst, |_| true).unwrap(), lst);
}

// ---------------------------------------------------------------------------
// sort_
// ---------------------------------------------------------------------------

#[test]
fn sort_integers() {
    // sort_([3, 1, 2]) → [1, 2, 3]
    let lst = li(&[3, 1, 2]);
    assert_eq!(sort_(&lst).unwrap(), li(&[1, 2, 3]));
}

#[test]
fn sort_already_sorted() {
    let lst = li(&[1, 2, 3]);
    assert_eq!(sort_(&lst).unwrap(), lst);
}

#[test]
fn sort_empty() {
    let lst = make_list(vec![]);
    assert_eq!(sort_(&lst).unwrap(), lst);
}

// ---------------------------------------------------------------------------
// flatten
// ---------------------------------------------------------------------------

#[test]
fn flatten_one_level() {
    // [1, [2, 3], 4]  depth=1  →  [1, 2, 3, 4]
    let nested = make_list(vec![int(1), li(&[2, 3]), int(4)]);
    assert_eq!(flatten(&nested, 1).unwrap(), li(&[1, 2, 3, 4]));
}

#[test]
fn flatten_partial() {
    // [1, [2, [3, 4]]]  depth=1  →  [1, 2, [3, 4]]
    let inner = li(&[3, 4]);
    let nested = make_list(vec![int(1), make_list(vec![int(2), inner.clone()])]);
    let out = flatten(&nested, 1).unwrap();
    assert_eq!(out, make_list(vec![int(1), int(2), inner]));
}

#[test]
fn flatten_unlimited() {
    // flatten(.., -1) → fully flat
    let inner = li(&[3, 4]);
    let nested = make_list(vec![int(1), make_list(vec![int(2), inner])]);
    assert_eq!(flatten(&nested, -1).unwrap(), li(&[1, 2, 3, 4]));
}

#[test]
fn flatten_zero_depth_unchanged() {
    let nested = make_list(vec![int(1), li(&[2])]);
    assert_eq!(flatten(&nested, 0).unwrap(), nested);
}

#[test]
fn flatten_already_flat() {
    let lst = li(&[1, 2, 3]);
    assert_eq!(flatten(&lst, 1).unwrap(), lst);
}
