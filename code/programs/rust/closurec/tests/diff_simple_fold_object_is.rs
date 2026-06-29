//! Integration test for the `tests/diff/simple-fold-object-is/` fixture.
//!
//! End-to-end oracle for static `Object.is(a, b)` folding in
//! `closure-pass-constant-fold`: the SameValue comparison (ECMAScript
//! §20.1.2.13 / §7.2.11) folds to a boolean literal when both arguments are
//! primitive literals. SameValue differs from `===` in two cases —
//! `Object.is(NaN, NaN)` is `true` and `Object.is(+0, -0)` is `false`. A bare
//! global `NaN` is an identifier (not a literal), so `Object.is(NaN, NaN)` is
//! conservatively declined here.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=true;var b=false;var c=true;var d=false;var e=Object.is(NaN,NaN);report(a,b,c,d,e);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-object-is/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_object_is_fixture_matches_expected_stdout() {
    let flags = read_flags();
    let out = Command::new(BINARY).args(&flags).output().expect("run closurec");

    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );

    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/simple-fold-object-is/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Each literal `Object.is(...)` folds to the SameValue boolean; the ±0 case
/// (`false`, where `===` would be `true`) and the type mismatch are respected.
#[test]
fn simple_fold_object_is_folds_same_value() {
    let out = Command::new(BINARY).args(read_flags()).output().expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("a=true"), "Object.is(1,1) → true; got:\n{actual}");
    assert!(
        actual.contains("b=false"),
        "Object.is(0,-0) → false (±0 SameValue, NOT ===); got:\n{actual}"
    );
    assert!(actual.contains("c=true"), "Object.is(\"x\",\"x\") → true; got:\n{actual}");
    assert!(
        actual.contains("d=false"),
        "Object.is(1,\"1\") → false (different Type); got:\n{actual}"
    );
    // `NaN` is the global identifier, not a literal — conservatively declined.
    assert!(
        actual.contains("e=Object.is(NaN,NaN)"),
        "Object.is(NaN,NaN) must NOT fold (NaN is an identifier); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly one
/// `Object.is` call — the declined NaN-identifier comparison — may remain.
#[test]
fn simple_fold_object_is_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY).args(read_flags()).output().expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("Object.is").count(),
        1,
        "exactly one Object.is call (the declined NaN-identifier comparison) should \
         remain — proving the typed SIMPLE optimizer ran, not the whitespace \
         fallback; got:\n{actual}",
    );
}
