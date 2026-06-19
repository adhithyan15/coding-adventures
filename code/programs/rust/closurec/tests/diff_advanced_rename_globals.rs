//! Integration test for the `tests/diff/advanced-rename-globals/` fixture.
//!
//! Exercises CLOC13.I — `rename-globals` wired into ADVANCED. This is the
//! first point where ADVANCED produces SMALLER output than SIMPLE: the
//! private top-level `helper` (which survives `inline`/`treeshake`) is
//! shortened to `a` under ADVANCED but kept under SIMPLE (a top-level name
//! may be externally visible). The test runs BOTH levels on the same
//! input and asserts the divergence.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn run(level: &str) -> String {
    let out = Command::new(BINARY)
        .args([
            "--compilation_level",
            level,
            "--js",
            "tests/diff/advanced-rename-globals/input/a.js",
        ])
        .output()
        .expect("run closurec");
    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8_lossy(&out.stdout)
        .trim_end_matches('\n')
        .to_string()
}

#[test]
fn advanced_rename_globals_fixture_matches_expected_stdout() {
    let expected = std::fs::read_to_string("tests/diff/advanced-rename-globals/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(run("ADVANCED"), expected.trim_end_matches('\n'));
}

#[test]
fn advanced_renames_top_level_helper_that_simple_keeps() {
    let simple = run("SIMPLE");
    let advanced = run("ADVANCED");

    // SIMPLE keeps the top-level name; ADVANCED renames it away.
    assert!(
        simple.contains("function helper("),
        "SIMPLE should keep the top-level `helper`: {simple}"
    );
    assert!(
        !advanced.contains("helper"),
        "ADVANCED should rename `helper` away: {advanced}"
    );
    // The divergence is a net size win.
    assert!(
        advanced.len() < simple.len(),
        "ADVANCED ({advanced}) should be smaller than SIMPLE ({simple})"
    );
}
