//! Integration test for the `tests/diff/impure-ternary-comma/` fixture.
//!
//! Exercises the impure-test equal-branch ternary collapse in
//! `closure-pass-constant-fold` 0.104.0.
//!
//! ## Why the comma sequence
//!
//! When both arms of a ternary are the same expression `X` but the test `t` is
//! side-effectful, the value is `X` regardless of how `t` decides — but `t`'s
//! effect must be preserved. Closure rewrites this to `(t, X)`, which evaluates
//! `t` then `X`, left to right, yielding `X` — the same order as the ternary.
//! (The pure-test case `t ? X : X` collapses straight to `X`, shipped earlier.)
//!
//! ## Fact — SIMPLE
//!
//! - `w(f()?x:x)`   → `w((f(),x))`   (impure call preserved)
//! - `w((a=b)?c:c)` → `w((a=b,c))`   (assignment preserved)
//!
//! Verified byte-identical to the real Closure jar.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/impure-ternary-comma/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn impure_test_equal_branch_ternary_becomes_comma_sequence() {
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
    let expected = std::fs::read_to_string("tests/diff/impure-ternary-comma/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
