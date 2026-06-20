//! Integration test for the `tests/diff/simple-try-catch/` fixture.
//!
//! Exercises `--compilation_level SIMPLE` optimization THROUGH a
//! `try`/`catch`/`finally` statement (CLOC19). Before CLOC19, any
//! program containing `try` failed the typed-AST parse and closurec
//! fell back to WHITESPACE_ONLY (zero optimization). This fixture is
//! the end-to-end oracle proving every SIMPLE pass now runs inside the
//! try block, the catch handler, and the finally block:
//!
//! ```text
//! source ──parse──▶ grammar AST ──bridge──▶ typed Program (w/ TryStatement)
//!        ──passes──▶ optimized Program ──emit──▶ JS text
//! ```
//!
//! Specifically: `log(1)` is inlined to `report(1)`, the foldable
//! arithmetic inside the try/catch blocks (`1 + 2`, `3 * 4`) collapses,
//! the unreachable `dead(99)` after the catch's `return` is dropped,
//! and `try`/`catch (e)`/`finally` (including the catch binding `e`)
//! survive verbatim.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/simple-try-catch/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_try_catch_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-try-catch/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Regression guard: the output must NOT be the WHITESPACE_ONLY
/// fallback. If try/catch ever stops being representable in the typed
/// AST, the fixture above would still "pass" against a regenerated
/// expected file, so we additionally assert that an optimization that
/// can ONLY come from the typed pipeline (the `log` -> `report` inline)
/// is present, and the original `function log` declaration is gone.
#[test]
fn simple_try_catch_did_not_fall_back_to_whitespace_only() {
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
}
