//! Integration test for the `tests/diff/line-wrap/` fixture.
//!
//! Exercises `closure-emitter` 0.57.0: output is wrapped at a 500-column budget
//! instead of being emitted as one unbounded line.
//!
//! ## Facts (oracle-verified against the real Closure jar, SIMPLE)
//!
//! `expected.stdout` was produced BY THE ORACLE
//! (`closure-compiler-v20260712.jar`, `--compilation_level SIMPLE_OPTIMIZATIONS
//! --language_in ECMASCRIPT_2020 --language_out NO_TRANSPILE`), not by closurec,
//! so this pins real byte-identity rather than freezing current behaviour.
//!
//! The input is 70 call statements of exactly 10 chars each (`sink000();`). The
//! oracle emits TWO lines, 510 and 190 chars. 510 is the FIRST length exceeding
//! the 500 budget: 50 statements would sit at exactly 500 (allowed), so the 51st
//! is what tips it over, and the break lands AFTER that statement -- not before
//! it. Lines therefore run slightly over the budget by design.
//!
//! Call statements are used deliberately: a run of separate `var` declarations
//! would be collapsed into a single statement at SIMPLE and would not exercise
//! the statement-boundary wrap at all.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/line-wrap/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn run() -> String {
    let flags = read_flags();
    let out = Command::new(BINARY).args(&flags).output().expect("run closurec");
    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn line_wrap_matches_closure_byte_for_byte() {
    let actual = run();
    let expected =
        std::fs::read_to_string("tests/diff/line-wrap/expected.stdout").expect("read expected");
    assert_eq!(
        actual, expected,
        "mismatch.\nactual lines:   {:?}\nexpected lines: {:?}",
        actual.lines().map(str::len).collect::<Vec<_>>(),
        expected.lines().map(str::len).collect::<Vec<_>>(),
    );
}

#[test]
fn output_is_actually_wrapped_not_one_long_line() {
    // Guards the point of the fixture: before this change closurec emitted the
    // whole program on a single 700-char line.
    let actual = run();
    let lens: Vec<usize> = actual.lines().map(str::len).collect();
    assert_eq!(lens, vec![510, 190], "unexpected line shape: {lens:?}");
}
