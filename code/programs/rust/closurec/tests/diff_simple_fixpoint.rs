//! Integration test for the `tests/diff/simple-fixpoint/` fixture.
//!
//! Exercises CLOC13.F — the pass pipeline runs to a FIXED POINT. The
//! input needs two sweeps to fully optimize: sweep 1 inlines the
//! single-use `double(7)` call into `log(7 * 2)` (and removes the dead
//! `double`); sweep 2 constant-folds `7 * 2` to `14`. Before fixed-point
//! iteration the pipeline ran each pass once and stopped at
//! `log(7 * 2);`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fixpoint/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fixpoint_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fixpoint/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
