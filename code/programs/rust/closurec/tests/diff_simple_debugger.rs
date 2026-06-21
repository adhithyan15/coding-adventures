//! Integration test for the `tests/diff/simple-debugger/` fixture.
//!
//! Exercises `--compilation_level SIMPLE` across a `debugger;` statement
//! (CLOC21 made it representable; CLOC24 strips it). Before CLOC21, any
//! program containing a `debugger` statement failed the typed-AST parse and
//! closurec fell back to WHITESPACE_ONLY (zero optimization). This fixture is
//! the end-to-end oracle proving the SIMPLE pipeline runs across a `debugger`
//! statement: `log(1)` is inlined to `report(1)`, `1 + 2` folds to `3`, and
//! the `debugger;` statement is STRIPPED (matching upstream Closure).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/simple-debugger/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_debugger_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-debugger/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Regression guard: the output must NOT be the WHITESPACE_ONLY fallback. If
/// `debugger` ever stops being representable in the typed AST, the fixture
/// above would still "pass" against a regenerated expected file, so we
/// additionally assert that an optimization that can ONLY come from the typed
/// pipeline (the `log` -> `report` inline) is present and the original
/// `function log` declaration is gone. CLOC24 additionally STRIPS the
/// `debugger;` statement at SIMPLE — proving the dce pass removed it (a
/// WHITESPACE_ONLY fallback would have kept `debugger;` verbatim, so its
/// absence here doubly confirms the typed pipeline ran).
#[test]
fn simple_debugger_did_not_fall_back_to_whitespace_only() {
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
        !actual.contains("debugger"),
        "expected the `debugger;` statement to be STRIPPED at SIMPLE (CLOC24); \
         got:\n{actual}",
    );
}
