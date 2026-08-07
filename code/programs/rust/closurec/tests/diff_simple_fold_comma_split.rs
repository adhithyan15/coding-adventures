//! Integration test for the `tests/diff/simple-fold-comma-split/` fixture.
//!
//! End-to-end oracle for comma-sequence statement splitting in
//! `closure-pass-fold-control-flow`: a comma sequence used as an expression
//! statement at a statement-LIST position (a function body or the program body)
//! splits into one statement per operand, because the comma operator discards
//! every value but the last and an expression statement already discards its
//! value. A comma sequence in a single-statement body (an `if`/`for` with no
//! braces) has no statement list to splice into, so it stays fused.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! function f(){a();b()}x();y();z();cond&&(p(),q());for(;run();)step(),tick();
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-comma-split/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_comma_split_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-comma-split/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
