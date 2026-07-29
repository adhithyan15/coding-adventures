//! Integration test for the `tests/diff/simple-trailing-continue/` fixture.
//!
//! End-to-end oracle for trailing-`continue` removal in
//! `closure-pass-fold-control-flow`: a bare (unlabeled) `continue` at the tail
//! of a for/while/do-while body is removed (it is a no-op); the shortened body
//! then unwraps or normalizes. Labeled continues and continues with dead code
//! after them are left intact.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! for(;c;)step();for(;d;)work();do tick();while(e);for(;f;);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-trailing-continue/flags.txt")
        .expect("read flags.txt");
    raw.lines().filter(|l| !l.is_empty()).map(|s| s.to_string()).collect()
}

#[test]
fn simple_trailing_continue_fixture_matches_expected_stdout() {
    let flags = read_flags();
    let out = Command::new(BINARY).args(&flags).output().expect("run closurec");
    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/simple-trailing-continue/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
