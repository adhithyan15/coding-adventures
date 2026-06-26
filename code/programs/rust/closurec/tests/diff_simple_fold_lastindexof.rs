//! Integration test for the `tests/diff/simple-fold-lastindexof/` fixture.
//!
//! End-to-end oracle for `String.prototype.lastIndexOf(needle)` folding in
//! `closure-pass-constant-fold` — the UTF-16 code-unit index of the *last*
//! occurrence (or -1), the mirror of the already-folded `indexOf`. The fixture
//! covers last-match, absent, empty-needle (→ string length), and the basic
//! case:
//!
//! ```text
//! var a=4;var b=-1;var c=3;var d=1;report(a,b,c,d);
//! ```
//!
//! - `"abcabc".lastIndexOf("bc")` → `4` — the last "bc" (indexOf would give 1);
//! - `"abcabc".lastIndexOf("z")`  → `-1` — absent;
//! - `"abc".lastIndexOf("")`      → `3` — empty needle yields the string length;
//! - `"ab".lastIndexOf("b")`      → `1`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-lastindexof/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_lastindexof_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-lastindexof/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `lastIndexOf` on string literals folds to numerics — no `.lastIndexOf(` call
/// survives — and the last-match / absent / empty-needle-is-length rules hold.
#[test]
fn simple_fold_lastindexof_folds_to_numbers() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    // "abcabc".lastIndexOf("bc") → 4 (the LAST "bc"; indexOf would give 1).
    assert!(
        actual.contains("a=4"),
        "\"abcabc\".lastIndexOf(\"bc\") → 4 (last match); got:\n{actual}"
    );
    // "abcabc".lastIndexOf("z") → -1 (absent).
    assert!(
        actual.contains("b=-1"),
        "\"abcabc\".lastIndexOf(\"z\") → -1; got:\n{actual}"
    );
    // "abc".lastIndexOf("") → 3 (empty needle yields the string length).
    assert!(
        actual.contains("c=3"),
        "\"abc\".lastIndexOf(\"\") → 3 (length); got:\n{actual}"
    );
    // "ab".lastIndexOf("b") → 1.
    assert!(
        actual.contains("d=1"),
        "\"ab\".lastIndexOf(\"b\") → 1; got:\n{actual}"
    );
    assert!(
        !actual.contains("lastIndexOf"),
        "no `lastIndexOf` call should remain after folding; got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave the calls intact).
#[test]
fn simple_fold_lastindexof_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains("lastIndexOf"),
        "expected the calls to be folded away by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
