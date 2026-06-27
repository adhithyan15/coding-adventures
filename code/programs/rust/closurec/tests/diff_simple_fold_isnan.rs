//! Integration test for the `tests/diff/simple-fold-isnan/` fixture.
//!
//! End-to-end oracle for global `isNaN` / `isFinite` folding in
//! `closure-pass-constant-fold`: a call whose single argument is a string- or
//! number-literal collapses to the boolean literal V8 would produce (ECMAScript
//! §19.2.3 / §19.2.2). Both run `ToNumber` on the argument and classify the
//! result — `isNaN` is true iff `NaN`, `isFinite` is true iff neither `NaN` nor
//! `±Infinity`. Unlike `Number(...)`, no shape declines.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=true;var b=false;var c=false;var d=true;var e=false;var f=false;var g=true;report(a,b,c,d,e,f,g);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-isnan/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_isnan_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-isnan/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Each call collapses to the boolean its `ToNumber` classification implies —
/// including the surprising `isNaN(" ")` (whitespace coerces to `+0`, not NaN)
/// and `isFinite("Infinity")` (a number, but not finite).
#[test]
fn simple_fold_isnan_folds_to_booleans() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("a=true"), "isNaN(\"abc\") → true; got:\n{actual}");
    assert!(actual.contains("b=false"), "isNaN(\"42\") → false; got:\n{actual}");
    assert!(actual.contains("c=false"), "isNaN(\" \") → false (ToNumber(\" \")=+0); got:\n{actual}");
    assert!(actual.contains("d=true"), "isFinite(\"1e3\") → true; got:\n{actual}");
    assert!(actual.contains("e=false"), "isFinite(\"Infinity\") → false; got:\n{actual}");
    assert!(actual.contains("f=false"), "isFinite(\"abc\") → false; got:\n{actual}");
    assert!(actual.contains("g=true"), "isFinite(0) → true; got:\n{actual}");
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Every
/// `isNaN`/`isFinite` call folds, so none may remain.
#[test]
fn simple_fold_isnan_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("isNaN(").count(),
        0,
        "every isNaN call should fold; got:\n{actual}",
    );
    assert_eq!(
        actual.matches("isFinite(").count(),
        0,
        "every isFinite call should fold — proving the typed SIMPLE optimizer \
         ran, not the whitespace fallback; got:\n{actual}",
    );
}
