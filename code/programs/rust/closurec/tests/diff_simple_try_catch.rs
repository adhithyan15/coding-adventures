//! Integration test for the `tests/diff/simple-try-catch/` fixture.
//!
//! The pinned reference compiler (`closure-compiler-v20260915`) refuses this
//! input with `JSC_PARSE_ERROR` ("Parse error. return must be inside function"): the `return` in its top-level `catch` handler is outside any function. Since CLOC44 slice 2
//! (CCR-075, #15860), closurec refuses it the same way: `expected.stderr` and
//! `expected.exit` were captured from the jar, the stderr must match byte for
//! byte, and there must be no stdout. Before slice 2 closurec compiled this
//! input, and this fixture pinned that output.
//!
//! The fixture was written (CLOC19) to show SIMPLE optimising through
//! `try`/`catch`/`finally`. That coverage lives on in the ladder's
//! `try_catch` rungs and `advanced-try-catch-rename`, whose inputs are valid.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");
const DIR: &str = "tests/diff/simple-try-catch";

#[test]
fn simple_try_catch_is_refused_like_upstream() {
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
