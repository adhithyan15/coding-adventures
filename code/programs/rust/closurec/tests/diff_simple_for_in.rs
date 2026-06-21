//! Integration test for the `tests/diff/simple-for-in/` fixture.
//!
//! Exercises `--compilation_level SIMPLE` optimization THROUGH a `for`-`in`
//! loop (CLOC22). Before CLOC22, any program containing a for-in loop failed
//! the typed-AST parse and closurec fell back to WHITESPACE_ONLY (zero
//! optimization). This fixture is the end-to-end oracle proving every SIMPLE
//! pass now runs inside the for-in body and recurses into its right-hand
//! expression: `log(1)` is inlined to `report(1)`, `1 + 2` folds to `3`, the
//! loop is preserved, and `after()` stays reachable.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/simple-for-in/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_for_in_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-for-in/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Regression guard: the output must NOT be the WHITESPACE_ONLY fallback. If
/// `for`-`in` ever stops being representable in the typed AST, the fixture
/// above would still "pass" against a regenerated expected file, so we
/// additionally assert that an optimization that can ONLY come from the typed
/// pipeline (the `log` -> `report` inline) is present, the original
/// `function log` declaration is gone, and the statement after the loop
/// (`after()`) survives — proving a for-in is treated as non-terminating.
#[test]
fn simple_for_in_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        actual.contains("report(1)"),
        "expected the single-use `log` to be inlined into `report(1)` \
         (proving the typed pipeline ran, not the whitespace fallback); \
         got:\n{actual}",
    );
    assert!(
        !actual.contains("function log"),
        "expected the inlined `log` declaration to be removed; got:\n{actual}",
    );
    assert!(
        actual.contains("after()"),
        "expected the statement after the for-in loop to remain reachable; \
         got:\n{actual}",
    );
}
