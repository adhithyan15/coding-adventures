//! Integration test for the `tests/diff/simple-fold-repeat/` fixture.
//!
//! End-to-end oracle for string `repeat` folding in
//! `closure-pass-constant-fold`: `"ab".repeat(3)` collapses to the string
//! literal `"ababab"` (JS `String.prototype.repeat`).
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var s="ababab";report(s);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-repeat/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_repeat_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-repeat/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `"ab".repeat(3)` must fold to the string literal `"ababab"` — no method call
/// should remain in the output.
#[test]
fn simple_fold_repeat_folds_to_string_literal() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("\"ababab\""), "should fold to \"ababab\"; got:\n{actual}");
    assert!(
        !actual.contains("repeat"),
        "no `repeat` call should remain after folding; got:\n{actual}",
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave `"ab".repeat(3)` intact).
#[test]
fn simple_fold_repeat_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains(".repeat"),
        "expected the call to be folded away by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
