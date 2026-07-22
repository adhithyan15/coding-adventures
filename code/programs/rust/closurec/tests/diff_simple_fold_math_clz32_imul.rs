//! Integration test for the `tests/diff/simple-fold-math-clz32-imul/` fixture.
//!
//! End-to-end oracle for static `Math.clz32(...)` / `Math.imul(...)` folding in
//! `closure-pass-constant-fold`: `clz32` -> leading zero bits of ToUint32 (0..32),
//! `imul` -> 32-bit signed product of the ToUint32 operands. Both are exact
//! modular integer operations. A non-literal argument and a non-global receiver
//! are declined.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    std::fs::read_to_string("tests/diff/simple-fold-math-clz32-imul/flags.txt")
        .expect("read flags.txt")
        .lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_math_clz32_imul_fixture_matches_expected_stdout() {
    let flags = read_flags();
    let out = Command::new(BINARY).args(&flags).output().expect("run closurec");
    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/simple-fold-math-clz32-imul/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
