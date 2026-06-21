//! Integration test for the `tests/diff/simple-else-hoist/` fixture.
//!
//! Exercises CLOC25 — dropping a redundant `else` after an `if` consequent that
//! unconditionally terminates (upstream Closure's `MinimizeExitPoints`),
//! implemented in the `fold-control-flow` pass. `classify`'s consequent ends in
//! `return`, so the `else` body is hoisted out after the `if` and the `else`
//! keyword + braces are deleted:
//!
//! ```text
//! function classify(n){if(n < 0){return negative(n)}record(n);return positive(n)}
//! ```
//!
//! Under WHITESPACE_ONLY the `else` survives verbatim, so the absence of `else`
//! here is also a proof that the typed (SIMPLE) pipeline ran.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-else-hoist/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_else_hoist_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-else-hoist/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Regression guard: the output must NOT be the WHITESPACE_ONLY fallback. Under
/// WHITESPACE_ONLY the `else` is preserved verbatim, so an optimized SIMPLE run
/// that hoisted the `else` must contain no `else` at all — while still keeping
/// the hoisted statements (`record(n)` / `positive(n)`) and the trailing
/// `report(...)` call reachable.
#[test]
fn simple_else_hoist_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains("else"),
        "expected the `else` to be hoisted away (CLOC25); a WHITESPACE_ONLY \
         fallback would have kept it. got:\n{actual}",
    );
    assert!(
        actual.contains("record(n)") && actual.contains("positive(n)"),
        "expected the hoisted else-body statements to survive; got:\n{actual}",
    );
    assert!(
        actual.contains("report(classify(5))"),
        "expected the trailing call keeping the function reachable; got:\n{actual}",
    );
}
