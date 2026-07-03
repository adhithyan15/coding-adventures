//! Integration test for the `tests/diff/simple-fold-slice/` fixture.
//!
//! End-to-end oracle for string `slice` folding in
//! `closure-pass-constant-fold`: `"abcd".slice(1, 3)` collapses to the string
//! literal `"bc"` (JS `String.prototype.slice`).
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var s="bc";report(s);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-slice/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_slice_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-slice/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `"abcd".slice(1, 3)` must fold to the string literal `"bc"` — no method call
/// should remain in the output.
#[test]
fn simple_fold_slice_folds_to_string_literal() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("\"bc\""), "should fold to \"bc\"; got:\n{actual}");
    assert!(
        !actual.contains("slice"),
        "no `slice` call should remain after folding; got:\n{actual}",
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave `"abcd".slice(1, 3)` intact).
#[test]
fn simple_fold_slice_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains("abcd"),
        "expected the string literal to be folded away by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
