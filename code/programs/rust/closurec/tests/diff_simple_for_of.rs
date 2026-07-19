//! Integration test for the `tests/diff/simple-for-of/` fixture.
//!
//! Exercises `--compilation_level SIMPLE` optimization THROUGH a `for`-`of`
//! loop (CLOC23). Before CLOC23, any program containing a for-of loop failed
//! the typed-AST parse and closurec fell back to WHITESPACE_ONLY (zero
//! optimization). This fixture is the end-to-end oracle proving every SIMPLE
//! pass now runs inside the for-of body and recurses into its iterable
//! expression: `1 + 2` folds to `3`, the loop is preserved, and `after()`
//! stays reachable. `function log` is KEPT and `log(1)` stays a call — SIMPLE
//! is open-world and never inlines or deletes an observable top-level name;
//! that inline runs only at ADVANCED.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/simple-for-of/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_for_of_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-for-of/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Regression guard: the output must NOT be the WHITESPACE_ONLY fallback. If
/// `for`-`of` ever stops being representable in the typed AST, the fixture
/// above would still "pass" against a regenerated expected file, so we
/// additionally assert an optimization that can ONLY come from the typed
/// pipeline: the `1 + 2` inside the loop body is constant-folded to `3`
/// (WHITESPACE_ONLY leaves it verbatim). We also assert the statement after
/// the loop (`after()`) survives — proving a for-of is treated as
/// non-terminating — and that `function log` is KEPT, since open-world SIMPLE
/// must not inline or delete an observable top-level name.
#[test]
fn simple_for_of_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        actual.contains("x=3"),
        "expected `1 + 2` in the loop body to be constant-folded to `3` \
         (proving the typed pipeline ran, not the whitespace fallback); \
         got:\n{actual}",
    );
    assert!(
        !actual.contains("1 + 2") && !actual.contains("1+2"),
        "expected `1 + 2` to have been folded away; got:\n{actual}",
    );
    assert!(
        actual.contains("after()"),
        "expected the statement after the for-of loop to remain reachable; \
         got:\n{actual}",
    );
    assert!(
        actual.contains("function log"),
        "expected the top-level `log` declaration to be KEPT at open-world \
         SIMPLE (never inlined/deleted); got:\n{actual}",
    );
}
