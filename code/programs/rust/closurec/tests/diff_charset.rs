//! Integration tests for the `tests/diff/charset-*/` fixtures.
//!
//! Exercises CLOC11.16 — `--charset` output normalization. Two
//! companion fixtures:
//!
//! - `charset-us-ascii/`: no `--charset` flag → CC's documented
//!   default applies (US-ASCII out). Non-ASCII chars in the
//!   input (©, em-dash, CJK) should appear in the output as
//!   `\uXXXX` escapes.
//!
//! - `charset-utf8/`: `--charset UTF-8` opts out of escaping;
//!   non-ASCII passes through verbatim.
//!
//! Pinning both ends of the toggle catches accidental swaps
//! (e.g. if a refactor inverts the default).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags(fixture: &str) -> Vec<String> {
    let path = format!("tests/diff/{fixture}/flags.txt");
    let raw = std::fs::read_to_string(&path).expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn run_fixture(fixture: &str) -> String {
    let flags = read_flags(fixture);
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
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn charset_default_is_us_ascii_escaping_non_ascii() {
    let actual = run_fixture("charset-us-ascii");
    let expected = std::fs::read_to_string("tests/diff/charset-us-ascii/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(actual, expected);

    // Sanity: the output must NOT contain raw non-ASCII bytes.
    // If it does, our default isn't actually escaping.
    assert!(
        actual.is_ascii(),
        "default --charset must produce pure-ASCII output: {actual}"
    );
}

#[test]
fn charset_utf8_passes_through_non_ascii_verbatim() {
    let actual = run_fixture("charset-utf8");
    let expected = std::fs::read_to_string("tests/diff/charset-utf8/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(actual, expected);

    // Sanity: with --charset UTF-8 the output should contain
    // the original non-ASCII characters, not escape sequences.
    assert!(
        !actual.is_ascii(),
        "--charset UTF-8 should preserve non-ASCII: {actual}"
    );
    assert!(actual.contains("©"));
    assert!(actual.contains("日本語"));
}
