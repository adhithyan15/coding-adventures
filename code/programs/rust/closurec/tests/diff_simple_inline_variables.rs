//! Integration test for the `tests/diff/simple-inline-variables/` fixture.
//!
//! At SIMPLE a top-level `const` is kept at its ordinary uses (CCR-078,
//! #15863): the reference compiler is open-world there, so
//! `const RATE = 2; total(base * RATE); margin(RATE + 1);` stays
//! `const RATE=2;total(base*RATE);margin(RATE+1);`. The expected bytes were
//! re-captured from the pinned v20260915 jar. Until CCR-078 this fixture
//! pinned closurec's old output (`total(base*2);margin(3)`), which upstream
//! never produced. Under ADVANCED the const is propagated and dropped
//! (`total(base*2);margin(3);`; see the `run.rs` unit test).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-inline-variables/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_inline_variables_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-inline-variables/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
