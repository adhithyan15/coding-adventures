//! Integration test for the `tests/diff/simple-fold-bitnot/` fixture.
//!
//! End-to-end oracle for unary bitwise-NOT folding in
//! `closure-pass-constant-fold`: `~<numeric literal>` collapses under ES
//! `ToInt32` semantics (the same `to_int32` coercion the binary `&`/`|`/`^`
//! operators already use, so the two stay bit-for-bit consistent).
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=-6;var b=0;var c=-6;var d=9;report(a,b,c,d);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-bitnot/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_bitnot_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-bitnot/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Each `~<literal>` must fold to its ES `ToInt32`-complement value, including
/// the double-complement `~~9 → 9` (which folds bottom-up in one walk).
#[test]
fn simple_fold_bitnot_folds_each_literal() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("a=-6"), "~5 should fold to -6; got:\n{actual}");
    assert!(actual.contains("b=0"), "~-1 should fold to 0; got:\n{actual}");
    assert!(actual.contains("c=-6"), "~5.9 should fold to -6; got:\n{actual}");
    assert!(actual.contains("d=9"), "~~9 should fold to 9; got:\n{actual}");
    // The unfolded form must not survive the typed pipeline.
    assert!(
        !actual.contains('~'),
        "no `~` should remain after folding; got:\n{actual}",
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// WHITESPACE_ONLY fallback (which would leave `~5` etc. intact).
#[test]
fn simple_fold_bitnot_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains("~5"),
        "expected `~5` to be folded by the typed pipeline (proving this is the \
         SIMPLE optimizer, not the whitespace fallback); got:\n{actual}",
    );
}
