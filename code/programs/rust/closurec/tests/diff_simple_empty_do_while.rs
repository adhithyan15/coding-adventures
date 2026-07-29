//! Integration test for the `tests/diff/simple-empty-do-while/` fixture.
//!
//! End-to-end oracle for empty-bodied do-while lowering in
//! `closure-pass-fold-control-flow`: `do {} while(test)` is equivalent to
//! `while(test){}` (the leading empty body is a no-op), so it lowers to the
//! equivalent loop, rewritten to `for` with the empty body normalized to `;`.
//! A statically-falsy test makes it a dead loop (removed); a non-empty do-while
//! keeps the `do` form.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! for(;cond;);for(;run(););do work();while(again);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-empty-do-while/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_empty_do_while_fixture_matches_expected_stdout() {
    let flags = read_flags();
    let out = Command::new(BINARY).args(&flags).output().expect("run closurec");
    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/simple-empty-do-while/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
