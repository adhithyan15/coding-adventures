//! Integration test for the `tests/diff/simple-spread-array-flatten/` fixture.
//!
//! End-to-end oracle for array-literal spread flattening in
//! `closure-pass-constant-fold`: a `...[…]` whose argument is a hole-free array
//! literal is inlined into the enclosing array literal (`[...[1,2],3]` ->
//! `[1,2,3]`). Non-literal spreads (`[...y,4]`) and hole-carrying inner literals
//! are left intact.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=[1,2,3],b=[0,1,2,3],c=[1,2,3],d=[...y,4];
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-spread-array-flatten/flags.txt")
        .expect("read flags.txt");
    raw.lines().filter(|l| !l.is_empty()).map(|s| s.to_string()).collect()
}

#[test]
fn simple_spread_array_flatten_fixture_matches_expected_stdout() {
    let flags = read_flags();
    let out = Command::new(BINARY).args(&flags).output().expect("run closurec");
    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/simple-spread-array-flatten/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
