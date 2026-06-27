//! Integration test for the `tests/diff/simple-fold-number-static/` fixture.
//!
//! End-to-end oracle for static `Number.isInteger` / `Number.isFinite` /
//! `Number.isNaN` folding in `closure-pass-constant-fold`: a call whose single
//! argument is a literal collapses to the boolean V8 would produce (ECMAScript
//! §21.1.2.2/.3/.4). Unlike the *global* `isNaN`/`isFinite`, these do NO
//! coercion — a non-Number argument is always `false`.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=true;var b=false;var c=true;var d=true;var e=false;var f=false;var g=false;report(a,b,c,d,e,f,g);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-number-static/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_number_static_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-number-static/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Each call collapses to the boolean its class implies — including the
/// no-coercion `Number.isInteger("42")` → false (a string is not a Number) and
/// the large-integer `Number.isInteger(1e21)` → true.
#[test]
fn simple_fold_number_static_folds_to_booleans() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("a=true"), "Number.isInteger(42) → true; got:\n{actual}");
    assert!(actual.contains("b=false"), "Number.isInteger(3.5) → false; got:\n{actual}");
    assert!(actual.contains("c=true"), "Number.isInteger(1e21) → true (integer-valued); got:\n{actual}");
    assert!(actual.contains("d=true"), "Number.isFinite(42) → true; got:\n{actual}");
    assert!(actual.contains("e=false"), "Number.isNaN(42) → false; got:\n{actual}");
    assert!(actual.contains("f=false"), "Number.isInteger(\"42\") → false (no coercion); got:\n{actual}");
    assert!(actual.contains("g=false"), "Number.isFinite(null) → false; got:\n{actual}");
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Every
/// `Number.isX` call folds, so none may remain.
#[test]
fn simple_fold_number_static_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("Number.is").count(),
        0,
        "every Number.isInteger/isFinite/isNaN call should fold — proving the \
         typed SIMPLE optimizer ran, not the whitespace fallback; got:\n{actual}",
    );
}
