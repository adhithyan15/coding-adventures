//! Integration test for the `tests/diff/simple-fold-array-of/` fixture.
//!
//! End-to-end oracle for static `Array.of(...)` folding in
//! `closure-pass-constant-fold`: `Array.of(v0, v1, …)` collapses to the array
//! literal `[v0, v1, …]` (ECMAScript §23.1.2.3) — exactly its arguments, in
//! order. Crucially `Array.of(7)` is the one-element array `[7]`, NOT
//! `Array(7)`'s length-7 hole array. A non-global receiver (`q.of(1)`) is
//! declined.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=[];var b=[7];var c=[1,2,3];var d=[x,y];var e=q.of(1);report(a,b,c,d,e);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-array-of/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_array_of_fixture_matches_expected_stdout() {
    let flags = read_flags();
    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .expect("run closurec");

    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );

    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/simple-fold-array-of/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Each bare-global `Array.of(...)` folds to the array literal of its arguments;
/// the single-numeric case is the one-element `[7]`, NOT `Array(7)`'s length.
#[test]
fn simple_fold_array_of_folds_to_array_literal() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("a=[]"), "Array.of() → []; got:\n{actual}");
    assert!(
        actual.contains("b=[7]"),
        "Array.of(7) → [7] (one element, NOT Array(7)'s length-7); got:\n{actual}"
    );
    assert!(
        actual.contains("c=[1,2,3]"),
        "Array.of(1,2,3) → [1,2,3]; got:\n{actual}"
    );
    assert!(
        actual.contains("d=[x,y]"),
        "Array.of(x,y) → [x,y] (identifier args preserved); got:\n{actual}"
    );
    // The non-global receiver is NOT folded — the call must remain.
    assert!(
        actual.contains("e=q.of(1)"),
        "q.of(1) must NOT fold (only the bare global Array.of); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly one
/// `.of(` call — the declined non-global receiver — may remain.
#[test]
fn simple_fold_array_of_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches(".of(").count(),
        1,
        "exactly one .of( call (the declined non-global receiver) should remain \
         — proving the typed SIMPLE optimizer ran, not the whitespace fallback; \
         got:\n{actual}",
    );
}
