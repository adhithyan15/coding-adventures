//! Integration test for the `tests/diff/simple-constant-fold/` fixture.
//!
//! Exercises the CLOC12.155 `--compilation_level SIMPLE` path
//! end-to-end. Unlike WHITESPACE_ONLY (which only strips
//! inter-token whitespace), SIMPLE routes the source through the
//! typed-AST pipeline:
//!
//! ```text
//! source ──parse──▶ grammar AST ──bridge──▶ typed Program
//!        ──passes──▶ optimized Program ──emit──▶ JS text
//! ```
//!
//! In this PR the pass pipeline holds a single pass —
//! `constant-fold` — so the fixture's constant initializers
//! (`1 + 2`, `3 * 4`, `2 + 3 * 4`) are evaluated at compile time and
//! emitted as their values (`3`, `12`, `14`). The companion
//! `simple_level_whitespace_only_leaves_arithmetic_unfolded` unit
//! test pins that the SAME input under WHITESPACE_ONLY keeps `1+2`,
//! proving the fold is the SIMPLE pipeline's doing.
//!
//! This is the behavioral oracle for the SIMPLE level: when we
//! later diff against the real `closure-compiler.jar`, this expected
//! file is the diff target.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-constant-fold/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_constant_fold_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-constant-fold/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
