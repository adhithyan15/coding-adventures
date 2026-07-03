//! Integration test for the `tests/diff/simple-fold-array-from/` fixture.
//!
//! End-to-end oracle for static `Array.from("…")` folding in
//! `closure-pass-constant-fold`: a single string-literal argument (no `mapFn`)
//! collapses to an array literal of single-code-point strings (ECMAScript
//! §23.1.2.1) — the string iterator yields one element per code point. A second
//! `mapFn` argument, a non-string-literal argument, and a non-global receiver
//! are all declined.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=["a","b","c"];var b=[];var c=Array.from("xy",f);var d=Array.from(s);var e=q.from("z");report(a,b,c,d,e);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-array-from/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_array_from_fixture_matches_expected_stdout() {
    let flags = read_flags();
    let out = Command::new(BINARY).args(&flags).output().expect("run closurec");

    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );

    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/simple-fold-array-from/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// A single string-literal `Array.from(...)` folds to the per-code-point array;
/// the mapFn / non-literal / non-global-receiver forms are declined.
#[test]
fn simple_fold_array_from_folds_string_literal() {
    let out = Command::new(BINARY).args(read_flags()).output().expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        actual.contains(r#"a=["a","b","c"]"#),
        "Array.from(\"abc\") → [\"a\",\"b\",\"c\"]; got:\n{actual}"
    );
    assert!(actual.contains("b=[]"), "Array.from(\"\") → []; got:\n{actual}");
    assert!(
        actual.contains(r#"c=Array.from("xy",f)"#),
        "Array.from(\"xy\", f) must NOT fold (mapFn); got:\n{actual}"
    );
    assert!(
        actual.contains("d=Array.from(s)"),
        "Array.from(s) must NOT fold (non-literal); got:\n{actual}"
    );
    assert!(
        actual.contains(r#"e=q.from("z")"#),
        "q.from(\"z\") must NOT fold (non-global receiver); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback. Exactly two `Array.from` calls — the declined
/// mapFn form and the declined non-literal form — may remain.
#[test]
fn simple_fold_array_from_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY).args(read_flags()).output().expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("Array.from").count(),
        2,
        "exactly two Array.from calls (mapFn + non-literal declines) should remain \
         — proving the typed SIMPLE optimizer ran, not the whitespace fallback; \
         got:\n{actual}",
    );
}
