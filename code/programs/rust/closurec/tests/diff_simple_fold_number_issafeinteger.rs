//! Integration test for the `tests/diff/simple-fold-number-issafeinteger/` fixture.
//!
//! End-to-end oracle for static `Number.isSafeInteger(x)` folding in
//! `closure-pass-constant-fold`: the predicate (ECMAScript §21.1.2.5) folds to a
//! boolean literal — `true` iff the argument is a Number that is a finite integer
//! with magnitude ≤ 2^53−1 (`Number.MAX_SAFE_INTEGER` = 9007199254740991), with
//! NO coercion. An identifier argument is declined.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=!0;var b=!1;var c=!0;var d=!1;var e=Number.isSafeInteger(x);report(a,b,c,d,e);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/simple-fold-number-issafeinteger/flags.txt")
            .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_number_issafeinteger_fixture_matches_expected_stdout() {
    let flags = read_flags();
    let out = Command::new(BINARY).args(&flags).output().expect("run closurec");

    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );

    let actual = String::from_utf8_lossy(&out.stdout);
    let expected =
        std::fs::read_to_string("tests/diff/simple-fold-number-issafeinteger/expected.stdout")
            .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Each numeric-literal `Number.isSafeInteger(...)` folds to the boolean V8
/// computes; the safe/unsafe boundary at 2^53 is respected.
#[test]
fn simple_fold_number_issafeinteger_classifies_literals() {
    let out = Command::new(BINARY).args(read_flags()).output().expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("a=!0"), "isSafeInteger(7) → true; got:\n{actual}");
    assert!(actual.contains("b=!1"), "isSafeInteger(3.5) → false; got:\n{actual}");
    assert!(
        actual.contains("c=!0"),
        "isSafeInteger(9007199254740991) → true (MAX_SAFE_INTEGER); got:\n{actual}"
    );
    assert!(
        actual.contains("d=!1"),
        "isSafeInteger(9007199254740992) → false (2^53, past safe range); got:\n{actual}"
    );
    // The identifier argument is NOT folded — the call must remain.
    assert!(
        actual.contains("e=Number.isSafeInteger(x)"),
        "isSafeInteger(x) must NOT fold (type unknown); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly one
/// `Number.isSafeInteger` call — the declined identifier — may remain.
#[test]
fn simple_fold_number_issafeinteger_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY).args(read_flags()).output().expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("Number.isSafeInteger").count(),
        1,
        "exactly one Number.isSafeInteger call (the declined identifier) should \
         remain — proving the typed SIMPLE optimizer ran, not the whitespace \
         fallback; got:\n{actual}",
    );
}
