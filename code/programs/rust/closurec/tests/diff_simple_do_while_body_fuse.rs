//! Integration test for the `tests/diff/simple-do-while-body-fuse/` fixture.
//!
//! End-to-end oracle for do-while loop-body comma-fusion in
//! `closure-pass-fold-control-flow`: a `do … while` body that is a block of
//! all-plain-expression statements fuses to a single (possibly comma-sequenced)
//! expression statement, dropping the braces — the do-while counterpart of the
//! for/while fusion. It runs after the body's inner folds (a folded `if`
//! participates) and declines on any body carrying a declaration or a
//! control-flow statement.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! do a(),b();while(c);do x&&g(),h();while(d);do{var v=1;k(v)}while(e);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-do-while-body-fuse/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_do_while_body_fuse_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-do-while-body-fuse/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
