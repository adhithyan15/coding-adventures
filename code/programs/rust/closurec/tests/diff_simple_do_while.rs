//! Integration test for the `tests/diff/simple-do-while/` fixture.
//!
//! Exercises `--compilation_level SIMPLE` optimization THROUGH a
//! `do`/`while` loop (CLOC20). Before CLOC20, any program containing a
//! `do`-`while` failed the typed-AST parse and closurec fell back to
//! WHITESPACE_ONLY (zero optimization). This fixture is the end-to-end
//! oracle proving every SIMPLE pass now runs inside the do-while body and
//! recurses into its test:
//!
//! ```text
//! source ──parse──▶ grammar AST ──bridge──▶ typed Program (w/ DoWhileStatement)
//!        ──passes──▶ optimized Program ──emit──▶ JS text
//! ```
//!
//! Specifically: `log(1)` is inlined to `report(1)`, the foldable
//! arithmetic inside the do-while body (`1 + 2`) collapses, the loop is
//! preserved verbatim, and the `foo()` after the loop stays reachable (a
//! do-while is not a terminator).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/simple-do-while/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_do_while_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-do-while/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Regression guard: the output must NOT be the WHITESPACE_ONLY fallback.
/// If `do`-`while` ever stops being representable in the typed AST, the
/// fixture above would still "pass" against a regenerated expected file, so
/// we additionally assert that an optimization that can ONLY come from the
/// typed pipeline (the `log` -> `report` inline) is present, the original
/// `function log` declaration is gone, AND the statement after the loop
/// (`foo()`) survives — proving a do-while is treated as non-terminating.
#[test]
fn simple_do_while_did_not_fall_back_to_whitespace_only() {
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
        actual.contains("foo()"),
        "expected the statement after the do-while loop to remain reachable; \
         got:\n{actual}",
    );
}
