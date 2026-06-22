//! Integration test for the `tests/diff/simple-fold-charat/` fixture.
//!
//! End-to-end oracle for string-indexing folding in
//! `closure-pass-constant-fold`: `"hello".charCodeAt(0)` collapses to the
//! UTF-16 code unit at that index (JS `String#charCodeAt`).
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var c=104;report(c);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-charat/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_charat_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-charat/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `"hello".charCodeAt(0)` must fold to the literal `104` — no method call
/// should remain in the output.
#[test]
fn simple_fold_charat_folds_to_numeric_literal() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("c=104"), "should fold to 104; got:\n{actual}");
    assert!(
        !actual.contains("charCodeAt"),
        "no `charCodeAt` call should remain after folding; got:\n{actual}",
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// WHITESPACE_ONLY fallback (which would leave `"hello".charCodeAt(0)` intact).
#[test]
fn simple_fold_charat_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains("\"hello\""),
        "expected the string literal to be folded away by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
