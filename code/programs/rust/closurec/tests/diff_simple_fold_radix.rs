//! Integration test for the `tests/diff/simple-fold-radix/` fixture.
//!
//! End-to-end oracle for numeric `toString([radix])` folding in
//! `closure-pass-constant-fold`: `(255).toString(16)` collapses to the string
//! literal `"ff"` (JS `Number.prototype.toString`).
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var s="ff";report(s);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-radix/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_radix_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-radix/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `(255).toString(16)` must fold to the string literal `"ff"` — no method call
/// should remain in the output.
#[test]
fn simple_fold_radix_folds_to_string_literal() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("\"ff\""), "should fold to \"ff\"; got:\n{actual}");
    assert!(
        !actual.contains("toString"),
        "no `toString` call should remain after folding; got:\n{actual}",
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave `(255).toString(16)` intact).
#[test]
fn simple_fold_radix_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains("255"),
        "expected the numeric literal to be folded away by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
