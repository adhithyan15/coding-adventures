//! Integration test for the `tests/diff/simple-fold-substring/` fixture.
//!
//! End-to-end oracle for `String.prototype.substring(start[, end])` folding in
//! `closure-pass-constant-fold`. Unlike `slice`, `substring` clamps each index
//! into `[0, len]` (a negative argument becomes 0 — it never counts from the
//! end) and SWAPS the endpoints when `start > end`. The fixture is chosen so
//! both behaviors are observable:
//!
//! ```text
//! var a="bc";var b="bc";var c="abcd";var d="";report(a,b,c,d);
//! ```
//!
//! - `"abcd".substring(1, 3)` → `"bc"` — the plain half-open range `[1, 3)`;
//! - `"abcd".substring(3, 1)` → `"bc"` — `start > end`, so endpoints swap;
//! - `"abcd".substring(-2)`   → `"abcd"` — a negative start clamps to 0;
//! - `"abcd".substring(10)`   → `""` — a start past the end clamps to `len`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-substring/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_substring_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-substring/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `substring` on string literals folds away — no `.substring(` call survives,
/// and the clamp (`-2` → whole string) and swap (`3,1` → `"bc"`) rules hold.
#[test]
fn simple_fold_substring_clamps_and_swaps() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    // "abcd".substring(1,3) → "bc"; "abcd".substring(3,1) → "bc" (swap).
    assert!(
        actual.contains(r#"a="bc""#),
        "\"abcd\".substring(1,3) → \"bc\"; got:\n{actual}"
    );
    assert!(
        actual.contains(r#"b="bc""#),
        "\"abcd\".substring(3,1) → \"bc\" (endpoints swap); got:\n{actual}"
    );
    // "abcd".substring(-2) → "abcd" (negative clamps to 0, NOT slice's "cd").
    assert!(
        actual.contains(r#"c="abcd""#),
        "\"abcd\".substring(-2) → \"abcd\" (clamp to 0); got:\n{actual}"
    );
    // "abcd".substring(10) → "" (start clamps to len).
    assert!(
        actual.contains(r#"d="""#),
        "\"abcd\".substring(10) → \"\"; got:\n{actual}"
    );
    assert!(
        !actual.contains(".substring("),
        "no `.substring(` call should remain after folding; got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave the calls intact).
#[test]
fn simple_fold_substring_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains(".substring("),
        "expected the calls to be folded away by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
