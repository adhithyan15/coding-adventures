//! Integration test for the `tests/diff/simple-fold-split/` fixture.
//!
//! End-to-end oracle for `String.prototype.split(separator[, limit])` folding in
//! `closure-pass-constant-fold`: a `"…".split(sepLit[, limit])` call whose
//! receiver and separator are string literals collapses to the **array literal**
//! V8 would produce at runtime (ECMAScript §22.1.3.23) — the first fold that
//! emits an `ArrayExpression` rather than a scalar.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=["a","b","c"];var b=["a","b","c"];var c=["a","b"];var d=["abc"];report(a,b,c,d);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-split/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_split_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-split/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `split` on string literals folds to array literals — no `.split(` call
/// survives, and the limit and empty-separator cases are honoured.
#[test]
fn simple_fold_split_folds_to_array_literals() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    // "a,b,c".split(",") → ["a","b","c"], "abc".split("") → ["a","b","c"],
    // "a,b,c".split(",",2) → ["a","b"], "abc".split() → ["abc"].
    assert!(
        actual.contains(r#"a=["a","b","c"]"#),
        "\"a,b,c\".split(\",\") → [\"a\",\"b\",\"c\"]; got:\n{actual}"
    );
    assert!(
        actual.contains(r#"b=["a","b","c"]"#),
        "\"abc\".split(\"\") → [\"a\",\"b\",\"c\"]; got:\n{actual}"
    );
    assert!(
        actual.contains(r#"c=["a","b"]"#),
        "\"a,b,c\".split(\",\",2) → [\"a\",\"b\"]; got:\n{actual}"
    );
    assert!(
        actual.contains(r#"d=["abc"]"#),
        "\"abc\".split() → [\"abc\"]; got:\n{actual}"
    );
    assert!(
        !actual.contains(".split("),
        "no `.split(` call should remain after folding; got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave the calls intact).
#[test]
fn simple_fold_split_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains(".split("),
        "expected the calls to be folded away by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
