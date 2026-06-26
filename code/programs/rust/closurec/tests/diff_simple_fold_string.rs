//! Integration test for the `tests/diff/simple-fold-string/` fixture.
//!
//! End-to-end oracle for global `String(…)` folding in
//! `closure-pass-constant-fold`: a `String(lit)` call whose single argument is a
//! string or INTEGER number literal collapses to the string literal V8's
//! `ToString` would produce (ECMAScript §22.1.3.1 → §7.1.17). A fractional
//! number (e.g. `0.5`) is declined — Rust's and V8's shortest-decimal
//! formatters can break an exact binary tie in opposite directions, so we never
//! risk a mis-fold.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a="42";var b="x";var c="-3";var d="255";var e=String(0.5);report(a,b,c,d,e);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-string/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_string_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-string/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// The foldable `String(...)` calls collapse to string literals — an integer →
/// its decimal text, a string argument unchanged — while `String(0.5)` (a
/// fractional number) is left intact.
#[test]
fn simple_fold_string_folds_to_string_literals() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("a=\"42\""), "String(42) → \"42\"; got:\n{actual}");
    assert!(actual.contains("b=\"x\""), "String(\"x\") → \"x\"; got:\n{actual}");
    assert!(actual.contains("c=\"-3\""), "String(-3) → \"-3\"; got:\n{actual}");
    assert!(actual.contains("d=\"255\""), "String(255) → \"255\"; got:\n{actual}");
    // `String(0.5)` is fractional — declined to avoid a tie-break mis-fold.
    assert!(
        actual.contains("e=String("),
        "String(0.5) must NOT fold (fractional); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly one
/// `String(` call — the declined `String(0.5)` — may remain.
#[test]
fn simple_fold_string_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("String(").count(),
        1,
        "exactly one String( call (the fractional decline) should remain — \
         proving the typed SIMPLE optimizer ran, not the whitespace fallback; \
         got:\n{actual}",
    );
}
