//! Integration test for the `tests/diff/whitespace-only/` fixture.
//!
//! Exercises the CLOC11.06 \--compilation_level WHITESPACE_ONLY path
//! end-to-end: spawn the built binary against an input file that
//! contains comments + lots of inter-token whitespace, assert the
//! emitted output is byte-equal to the canned `expected.stdout`.
//!
//! Per [CLOC11 §3], the fixture is a behavioral oracle — when we
//! verify against real `closure-compiler.jar` later, this expected
//! file is the diff target.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/whitespace-only/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn whitespace_only_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/whitespace-only/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
