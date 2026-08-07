//! Integration test for the `tests/diff/simple-fold-math-fround/` fixture.
//!
//! End-to-end oracle for static `Math.fround(...)` folding in
//! `closure-pass-constant-fold`: when the single argument is a numeric literal
//! that is already an exact float32 (a fixed point, `fround(x) === x`) the call
//! collapses to that numeric literal, unchanged. A double that fround would
//! change (`Math.fround(1.1)`, `Math.fround(16777217)`) and a non-global
//! receiver (`m.fround(1.5)`) are declined.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=1.5,b=-2.5,c=.25,d=16777216,e=Math.fround(1.1),f=Math.fround(16777217),g=m.fround(1.5);report(a,b,c,d,e,f,g);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-math-fround/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_math_fround_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-math-fround/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
