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
//! Specifically: the foldable arithmetic inside the try/catch blocks
//! (`1 + 2`, `3 * 4`) collapses to `3`/`12`, the unreachable `dead(99)` after
//! the catch's `return` is dropped by DCE, and `try`/`catch (e)`/`finally`
//! (including the catch binding `e`) survive verbatim. `function log` is KEPT
//! and `log(1)` stays a call — SIMPLE is open-world and never inlines or
//! deletes an observable top-level name; that inline runs only at ADVANCED.

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
/// expected file, so we additionally assert an optimization that can ONLY
/// come from the typed pipeline: the unreachable `dead(99)` after the
/// catch's `return` is dropped by DCE (WHITESPACE_ONLY keeps it verbatim).
/// We also assert `function log` is KEPT, since open-world SIMPLE must not
/// inline or delete an observable top-level name.
#[test]
fn simple_try_catch_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains("dead"),
        "expected the unreachable `dead(99)` after `return` to be dropped by \
         DCE (proving the typed pipeline ran, not the whitespace fallback); \
         got:\n{actual}",
    );
    assert!(
        actual.contains("function log"),
        "expected the top-level `log` declaration to be KEPT at open-world \
         SIMPLE (never inlined/deleted); got:\n{actual}",
    );
}
