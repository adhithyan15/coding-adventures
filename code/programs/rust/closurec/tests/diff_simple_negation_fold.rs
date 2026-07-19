//! Integration test for the `tests/diff/simple-negation-fold/` fixture.
//!
//! End-to-end oracle for the **negation-push** optimization in
//! `closure-pass-constant-fold` (upstream Closure's
//! `PeepholeMinimizeConditions`): `!(a == b)` → `a!=b`,
//! `!(a === b)` → `a!==b`. Sound for the four (in)equality operators only —
//! relational operators are NOT inverted because `!(a<b)` ≠ `a >= b` when an
//! operand is `NaN`.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=first(),b=second(),dead=17;report(a!=b,a!==b,!(a<b));
//! ```
//!
//! The unused `dead = 8 + 9` binding is KEPT (open-world SIMPLE never deletes
//! a top-level `var`), but its initializer is still constant-folded to `17`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-negation-fold/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_negation_fold_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-negation-fold/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// The equality negations must be pushed in, and the relational negation must
/// be left intact (NaN-safety).
#[test]
fn simple_negation_fold_pushes_equality_but_not_relational() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        actual.contains("a!=b"),
        "`!(a == b)` should push to `a!=b`; got:\n{actual}",
    );
    assert!(
        actual.contains("a!==b"),
        "`!(a === b)` should push to `a!==b`; got:\n{actual}",
    );
    // Relational stays negated — never rewritten to `a >= b` (NaN-unsafe).
    assert!(
        actual.contains("!(a<b)"),
        "`!(a<b)` must survive verbatim (NaN-safety); got:\n{actual}",
    );
    assert!(
        !actual.contains("a >= b"),
        "`!(a<b)` must NOT become `a >= b`; got:\n{actual}",
    );
}

/// Regression guard: the output must NOT be the WHITESPACE_ONLY fallback. The
/// unused `var dead = 8 + 9;` binding is KEPT at open-world SIMPLE, but its
/// initializer is still constant-folded to `17` — a transform WHITESPACE_ONLY
/// never performs (it would leave `8 + 9` verbatim).
#[test]
fn simple_negation_fold_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        actual.contains("dead=17"),
        "expected the kept `dead` binding's `8 + 9` initializer to be \
         constant-folded to `17` (proving this is the SIMPLE optimizer, not \
         the whitespace fallback); got:\n{actual}",
    );
    assert!(
        !actual.contains("8 + 9") && !actual.contains("8+9"),
        "expected `8 + 9` to have been folded away; got:\n{actual}",
    );
}
