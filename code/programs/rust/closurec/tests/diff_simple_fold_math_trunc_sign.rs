//! Integration test for the `tests/diff/simple-fold-math-trunc-sign/` fixture.
//!
//! End-to-end oracle for static `Math.trunc(...)` / `Math.sign(...)` folding in
//! `closure-pass-constant-fold`: when the single argument is a numeric literal
//! the call collapses to the truncated / signed result as a numeric literal
//! (ECMAScript 21.3.2.38 / 21.3.2.34). A non-literal argument (`Math.trunc(x)`),
//! a transcendental method the reference never folds (`Math.sqrt(16)`), a
//! non-global receiver (`m.trunc(1.5)`), and any `-0` result are all declined.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=4,b=-4,c=1,d=-1,e=0,f=Math.trunc(x),g=Math.sqrt(16),h=m.trunc(1.5);report(a,b,c,d,e,f,g,h);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-math-trunc-sign/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_math_trunc_sign_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-math-trunc-sign/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
