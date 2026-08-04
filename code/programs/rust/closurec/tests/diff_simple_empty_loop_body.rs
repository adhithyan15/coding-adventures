//! Integration test for the `tests/diff/simple-empty-loop-body/` fixture.
//!
//! End-to-end oracle for empty-loop-body normalization in
//! `closure-pass-fold-control-flow`: a `for`/`while` loop whose body folds to an
//! empty block (`{}`, `{;;}`, `{{}}`) has that body normalized to an empty
//! statement (`;`), dropping the braces. `while` first lowers to `for` and then
//! re-folds through the same path. A non-empty body is unaffected.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! for(var i=0;i<n;i++);for(;cond;);for(;run();)step();
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-empty-loop-body/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_empty_loop_body_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-empty-loop-body/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
