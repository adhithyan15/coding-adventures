//! Integration test for the `tests/diff/define/` fixture.
//!
//! Exercises CLOC11.19 — `--define NAME=value` token-level
//! substitution — end-to-end through the built binary. Drives
//! `--define DEBUG=false` against a small input that references
//! `DEBUG` in two places (initializer + `if` condition).
//!
//! This test pins `--compilation_level WHITESPACE_ONLY` on purpose.
//! `--define` is level-independent (a token-level pass), so running it
//! at WHITESPACE_ONLY isolates the substitution from the SIMPLE
//! optimizer — which now keeps growing passes (constant-fold,
//! fold-control-flow, …) that would otherwise churn this expected file
//! on every SIMPLE PR. The SIMPLE level's own behavior is covered by
//! the dedicated `simple-*` fixtures.
//!
//! A subtlety worth recording: the compilation level runs *before* the
//! `--define` substitution (pass order: lex → compilation_level →
//! defines). Under SIMPLE that means `if (DEBUG) {…}` would be rewritten
//! to `DEBUG && …` while `DEBUG` is still a variable, and only then
//! substituted — a correct but surprising interaction. WHITESPACE_ONLY
//! does no such rewrite, so the fixture stays a clean substitution oracle.
//!
//! Per CLOC11 §3 this is "behavioral equality" not byte-equal to CC; the
//! expected file is checked in from our own output and the diff target
//! is the *meaning* of the substitution (`DEBUG` → `false` in both the
//! initializer and the `if` condition).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/define/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn define_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/define/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
