//! Integration test for the `tests/diff/simple-dead-branch-var/` fixture.
//!
//! End-to-end oracle for dead-branch hoisted-`var` extraction in
//! `closure-pass-fold-control-flow` (a miscompile fix): a dead `if` branch is
//! removed but its hoisted `var` is EXTRACTED (initializer stripped) before the
//! taken branch, so the binding survives — `if(false){var z=compute()}else use()`
//! optimizes to `var z;use();`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    std::fs::read_to_string("tests/diff/simple-dead-branch-var/flags.txt")
        .expect("read flags.txt")
        .lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_dead_branch_var_fixture_matches_expected_stdout() {
    let flags = read_flags();
    let out = Command::new(BINARY).args(&flags).output().expect("run closurec");
    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/simple-dead-branch-var/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
