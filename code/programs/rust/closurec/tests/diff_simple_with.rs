//! Integration test for the `tests/diff/simple-with/` fixture.
//!
//! The pinned reference compiler (`closure-compiler-v20260915`) refuses this
//! input with `JSC_USE_OF_WITH` ("The with statement cannot be used in strict mode."):
//! `with (o) { x(1 + 2); }` uses the legacy `with` statement. Since CLOC44 slice 4
//! (CCR-075, #15860), closurec refuses it the same way: `expected.stderr` and
//! `expected.exit` were captured from the jar, the stderr must match byte for
//! byte, and there must be no stdout. Before slice 4 closurec compiled this
//! input, and this fixture pinned that output.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");
const DIR: &str = "tests/diff/simple-with";

#[test]
fn simple_with_is_refused_like_upstream() {
    let flags: Vec<String> = std::fs::read_to_string(format!("{DIR}/flags.txt"))
        .expect("read flags.txt")
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    let out = Command::new(BINARY).args(&flags).output().expect("run closurec");

    let want_exit: i32 = std::fs::read_to_string(format!("{DIR}/expected.exit"))
        .expect("read expected.exit")
        .trim()
        .parse()
        .expect("expected.exit is a number");
    let want_stderr = std::fs::read(format!("{DIR}/expected.stderr")).expect("read expected.stderr");

    assert_eq!(out.status.code(), Some(want_exit), "exit status");
    assert!(out.stdout.is_empty(), "a refused input must produce no output");
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&want_stderr),
        "diagnostics must match upstream byte for byte"
    );
}

#[test]
fn advanced_with_is_also_refused() {
    let out = Command::new(BINARY)
        .args([
            "--js",
            "tests/diff/simple-with/input/adv.js",
            "--compilation_level",
            "ADVANCED",
        ])
        .output()
        .expect("run closurec");

    assert_eq!(out.status.code(), Some(1), "exit status");
    assert!(out.stdout.is_empty(), "a refused input must produce no output");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("JSC_USE_OF_WITH"), "must report JSC_USE_OF_WITH: {stderr}");
    assert!(
        stderr.contains("The with statement cannot be used in strict mode."),
        "must report strict mode message: {stderr}"
    );
}

#[test]
fn whitespace_only_allows_with() {
    let out = Command::new(BINARY)
        .args([
            "--js",
            "tests/diff/simple-with/input/a.js",
            "--compilation_level",
            "WHITESPACE_ONLY",
        ])
        .output()
        .expect("run closurec");

    assert!(out.status.success(), "whitespace only must exit 0: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("with(o)"), "must preserve with in whitespace-only: {stdout}");
}
