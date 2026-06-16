//! Integration test for the `tests/diff/define/` fixture.
//!
//! Exercises CLOC11.19 — `--define NAME=value` token-level
//! substitution — end-to-end through the built binary. Drives
//! `--define DEBUG=false` against a small input that references
//! `DEBUG` in two places (initializer + `if` condition).
//!
//! Note this test does NOT pass `--compilation_level` explicitly,
//! so it runs at the default level (SIMPLE). As of closurec 0.138.0
//! SIMPLE routes through the typed-AST pipeline (bridge → passes →
//! emit), so the output is the emitter's form — e.g. the `if`
//! keeps its block braces (`if(false){…}`) where the older
//! whitespace-only path stripped them. `--define` runs as a
//! token-level pre-pass *before* the compilation level, so the
//! substitution's meaning (`DEBUG` → `false` in both the
//! initializer and the `if` condition) is identical across levels.
//! Per CLOC11 §3 this is "behavioral equality" not byte-equal to
//! CC; the expected file is checked in from our own output and the
//! diff target is the *meaning* of the substitution.

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
