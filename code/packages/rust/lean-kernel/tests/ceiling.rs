//! Tests at the bound-variable index ceiling: arithmetic that would overflow `u32` must
//! fail loudly, never wrap into a different term.

use lean_kernel::{BVarIndexTooLarge, ExprStore, Name, MAX_BVAR_INDEX};

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}

#[test]
fn bvar_ceiling_is_enforced() {
    let mut st = ExprStore::new();
    let top = st.try_bvar(MAX_BVAR_INDEX - 1).unwrap();
    assert_eq!(st.loose_bvar_range(top), MAX_BVAR_INDEX);
    assert_eq!(
        st.try_bvar(MAX_BVAR_INDEX),
        Err(BVarIndexTooLarge(MAX_BVAR_INDEX))
    );
    assert_eq!(st.try_bvar(u32::MAX), Err(BVarIndexTooLarge(u32::MAX)));
    // operations at the ceiling still work where the result stays inside it
    let c = st.constant(&n("c"), &[]);
    let below = st.try_bvar(MAX_BVAR_INDEX - 2).unwrap();
    assert_eq!(st.instantiate1(top, c), below);
    assert_eq!(st.lower_loose_bvars(top, 1, 1), below);
    assert_eq!(st.lift_loose_bvars(below, 0, 1), top);
    let app = st.app(top, c);
    assert_eq!(st.loose_bvar_range(app), MAX_BVAR_INDEX);
}

#[test]
#[should_panic(expected = "MAX_BVAR_INDEX")]
fn bvar_at_ceiling_panics() {
    let mut st = ExprStore::new();
    let _ = st.bvar(MAX_BVAR_INDEX);
}

#[test]
#[should_panic(expected = "MAX_BVAR_INDEX")]
fn lift_past_the_ceiling_panics_instead_of_wrapping() {
    let mut st = ExprStore::new();
    let top = st.bvar(MAX_BVAR_INDEX - 1);
    let _ = st.lift_loose_bvars(top, 0, 1);
}

#[test]
#[should_panic(expected = "overflowed u32")]
fn lift_by_huge_amount_panics_instead_of_wrapping() {
    let mut st = ExprStore::new();
    let b = st.bvar(5);
    let _ = st.lift_loose_bvars(b, 0, u32::MAX);
}

// ---------------------------------------------------------------------------------
// VER11a section 4.2: the line budgets of the trusted kernel, enforced in CI.
//
// A line counts when, after trimming, it is non-empty and does not start with `//`
// (so doc comments `///` and `//!` are free: the source is meant to be literate).
// Files are listed per budget row, so adding a module without listing it here is
// caught by `every_source_file_is_counted`.
// ---------------------------------------------------------------------------------

use std::path::Path;

fn counted_lines(file: &str) -> usize {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .count()
}

/// Row "Expressions and levels": at most 1,200 lines.
const TERM_LAYER: &[&str] = &["name.rs", "level.rs", "expr.rs"];
/// Row "Type inference and checking": at most 1,500 lines (this slice also puts the
/// environment, the local context, the error type and the checker state here).
const INFERENCE: &[&str] = &["error.rs", "lctx.rs", "env.rs", "checker.rs", "infer.rs"];
/// Row "Reduction and definitional equality": at most 2,200 lines.
const REDUCTION: &[&str] = &["whnf.rs", "defeq.rs"];
/// Row "Inductive types": at most 1,800 lines (slice 3a: single, non-nested inductives,
/// positivity, universe checks, recursor generation).
const INDUCTIVE: &[&str] = &["inductive.rs"];
/// Row "Export reader": at most 600 lines (VER11a 4.2, VER32 5.1: the PIF reader, with
/// every check of VER32 5.3). SHA-256, the import resolver and `nat` parsing are counted
/// separately, in the trusted base but outside this row.
const READER: &[&str] = &["pif.rs"];
/// Files that belong to no budget row.
const UNBUDGETED: &[&str] = &["lib.rs", "name_tests.rs"];

#[test]
fn line_budgets_hold() {
    let rows: [(&str, &[&str], usize); 5] = [
        ("expressions and levels", TERM_LAYER, 1_200),
        ("type inference and checking", INFERENCE, 1_500),
        ("reduction and definitional equality", REDUCTION, 2_200),
        ("inductive types", INDUCTIVE, 1_800),
        ("export reader (PIF)", READER, 600),
    ];
    for (row, files, limit) in rows {
        let total: usize = files.iter().map(|f| counted_lines(f)).sum();
        assert!(
            total <= limit,
            "{row}: {total} lines exceeds the {limit} budget"
        );
    }
}

#[test]
fn every_source_file_is_counted() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let listed: Vec<&str> = TERM_LAYER
        .iter()
        .chain(INFERENCE)
        .chain(REDUCTION)
        .chain(INDUCTIVE)
        .chain(READER)
        .chain(UNBUDGETED)
        .copied()
        .collect();
    for entry in std::fs::read_dir(src).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if name.ends_with(".rs") {
            assert!(
                listed.contains(&name.as_str()),
                "{name} is in no budget row"
            );
        }
    }
}
