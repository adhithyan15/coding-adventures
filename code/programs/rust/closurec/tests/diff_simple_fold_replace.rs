//! Integration test for the `tests/diff/simple-fold-replace/` fixture.
//!
//! End-to-end oracle for string `replace` / `replaceAll` folding in
//! `closure-pass-constant-fold`: on string literals with literal search and
//! replacement arguments, `replace` substitutes the first match and
//! `replaceAll` every match, collapsing the call to a single string literal
//! (JS `String.prototype.replace` / `replaceAll`).
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a="a_b_c";var b="a-bXc";report(a,b);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-replace/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_replace_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-replace/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `replaceAll` folds every match and `replace` only the first — both collapse
/// to string literals with no method call left behind.
#[test]
fn simple_fold_replace_folds_to_string_literals() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    // replaceAll("-","_") on "a-b-c" → every dash replaced.
    assert!(actual.contains("a=\"a_b_c\""), "replaceAll should fold to \"a_b_c\"; got:\n{actual}");
    // replace("X","-") on "aXbXc" → only the FIRST X replaced.
    assert!(actual.contains("b=\"a-bXc\""), "replace should fold first match only to \"a-bXc\"; got:\n{actual}");
    for method in ["replace", "replaceAll"] {
        assert!(
            !actual.contains(method),
            "no `{method}` call should remain after folding; got:\n{actual}",
        );
    }
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave the calls intact).
#[test]
fn simple_fold_replace_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains(".replace"),
        "expected the calls to be folded away by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
