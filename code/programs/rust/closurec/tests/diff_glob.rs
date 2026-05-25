//! Integration test for the `tests/diff/js-glob/` fixture.
//!
//! Per [CLOC11 §3](../../../specs/CLOC11-drop-in-closure-compat.md#3-strategy),
//! every behavioral closurec flag gets a checked-in fixture that
//! exercises it. The fixture's `expected.stdout` was captured (in
//! intent) from real `closure-compiler.jar` for the same input —
//! CLOC11.02 establishes the convention; CLOC11.06+ will diff
//! against actual Java tool output.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// Read the `tests/diff/js-glob/flags.txt` fixture as `--flag value`
/// pairs (one per line). Empty lines are skipped.
fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/js-glob/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn js_glob_fixture_matches_expected_stdout() {
    let flags = read_flags();
    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .expect("run closurec");

    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/js-glob/expected.stdout")
        .expect("read expected.stdout");

    // The CLOC11.02 identity pipeline appends a newline after each
    // input that didn't end with one. Expected was authored to
    // already contain those newlines; compare as-is.
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "stdout mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
    assert!(out.status.success(), "closurec must exit 0 on success");
}
