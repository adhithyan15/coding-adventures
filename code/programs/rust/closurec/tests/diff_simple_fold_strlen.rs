//! Integration test for the `tests/diff/simple-fold-strlen/` fixture.
//!
//! End-to-end oracle for string-literal `.length` folding in
//! `closure-pass-constant-fold`: `"hello".length` collapses to its UTF-16
//! code-unit count (JS `String#length` semantics).
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var n=5;report(n);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-strlen/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_strlen_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-strlen/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `"hello".length` must fold to the literal `5` — no `.length` left in output.
#[test]
fn simple_fold_strlen_folds_to_numeric_literal() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("n=5"), "\"hello\".length should fold to 5; got:\n{actual}");
    assert!(
        !actual.contains(".length"),
        "no `.length` should remain after folding; got:\n{actual}",
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// WHITESPACE_ONLY fallback (which would leave `"hello".length` intact).
#[test]
fn simple_fold_strlen_did_not_fall_back_to_whitespace_only() {
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
