//! Integration test for the `tests/diff/simple-fold-strpred/` fixture.
//!
//! End-to-end oracle for string-literal substring-predicate folding in
//! `closure-pass-constant-fold`: the single-argument `String#startsWith`,
//! `endsWith`, and `includes` collapse to a boolean literal when both the
//! receiver and the search string are string literals.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=true;var b=false;var c=true;report(a,b,c);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-strpred/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_strpred_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-strpred/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Each predicate must fold to its boolean literal — no method call should
/// remain in the output.
#[test]
fn simple_fold_strpred_folds_to_boolean_literals() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("a=true"), "startsWith should fold to true; got:\n{actual}");
    assert!(actual.contains("b=false"), "endsWith should fold to false; got:\n{actual}");
    assert!(actual.contains("c=true"), "includes should fold to true; got:\n{actual}");
    for method in ["startsWith", "endsWith", "includes"] {
        assert!(
            !actual.contains(method),
            "no `{method}` call should remain after folding; got:\n{actual}",
        );
    }
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave the calls intact).
#[test]
fn simple_fold_strpred_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains("\"hello\""),
        "expected the string literals to be folded away by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
