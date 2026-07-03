//! Integration test for the `tests/diff/checks-only/` fixture.
//!
//! Exercises CLOC11.51 — `--checks_only` — end-to-end via the
//! built binary. Validates a small JS input and asserts the
//! emitted output is empty (CC's behavior: no JS written when
//! `--checks_only` is set, just validation).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/checks-only/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn checks_only_fixture_emits_empty_stdout_and_exits_clean() {
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
    // Both the fixture and our binary should produce empty stdout.
    let expected = std::fs::read_to_string("tests/diff/checks-only/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(actual.as_ref(), expected, "mismatch — checks_only should emit nothing");
    assert!(actual.is_empty(), "stdout must be empty under --checks_only");
}
