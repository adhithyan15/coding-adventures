//! Integration test for the `tests/diff/simple-fold-boolean/` fixture.
//!
//! End-to-end oracle for global `Boolean(…)` folding in
//! `closure-pass-constant-fold`: a `Boolean(lit)` call whose single argument is
//! a string or number literal collapses to the boolean literal V8's `ToBoolean`
//! would produce (ECMAScript §7.1.2). A string is falsy only when empty; a
//! number is falsy only for `0`/`-0`.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=false;var b=true;var c=true;var d=false;var e=true;var f=Boolean(z);report(a,b,c,d,e,f);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-boolean/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_boolean_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-boolean/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// The foldable `Boolean(...)` calls collapse to boolean literals — empty string
/// and `0` are falsy, a non-empty string (even `"0"`) and a nonzero number are
/// truthy — while `Boolean(z)` (an identifier) is left intact.
#[test]
fn simple_fold_boolean_folds_to_boolean_literals() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("a=false"), "Boolean(\"\") → false; got:\n{actual}");
    assert!(actual.contains("b=true"), "Boolean(\"x\") → true; got:\n{actual}");
    assert!(actual.contains("c=true"), "Boolean(\"0\") → true (non-empty); got:\n{actual}");
    assert!(actual.contains("d=false"), "Boolean(0) → false; got:\n{actual}");
    assert!(actual.contains("e=true"), "Boolean(1) → true; got:\n{actual}");
    // `Boolean(z)` needs the runtime value of `z` — the call must survive.
    assert!(
        actual.contains("f=Boolean(z)"),
        "Boolean(z) must NOT fold (identifier); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly one
/// `Boolean(` call — the declined `Boolean(z)` — may remain.
#[test]
fn simple_fold_boolean_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("Boolean(").count(),
        1,
        "exactly one Boolean( call (the identifier decline) should remain — \
         proving the typed SIMPLE optimizer ran, not the whitespace fallback; \
         got:\n{actual}",
    );
}
