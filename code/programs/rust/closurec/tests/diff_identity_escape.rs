//! Integration test for the `tests/diff/identity-escape/` fixture.
//!
//! Exercises the identity-escape backslash-drop in `javascript-parser` 0.62.0,
//! which generalizes the `\8`/`\9` fix (0.61.0) to the full ECMAScript
//! *IdentityEscape* set.
//!
//! ## Why the backslash is dropped
//!
//! A backslash before a character that is not a recognized escape is a
//! *NonEscapeCharacter*: the backslash is dropped and the character kept.
//! Previously the bridge's catch-all arm KEPT the backslash, so `"\q"` decoded
//! to the two-char value `\q` instead of `"q"` — a value miscompile. The
//! reference Closure Compiler drops the backslash uniformly.
//!
//! ## Fact — SIMPLE: `g(a, "\q\/")` -> `g(a,"q/")`
//!
//! Both escapes drop their backslash, yielding the two-char string `"q/"`.
//! Verified byte-identical to the real Closure jar.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/identity-escape/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn identity_escapes_drop_backslash() {
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
    let expected = std::fs::read_to_string("tests/diff/identity-escape/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // `\q\/` dropped both backslashes -> the string value is exactly `q/`.
    let flat = actual.replace([' ', '\n'], "");
    assert_eq!(flat, r#"g(a,"q/");"#, "expected the decoded value q/");
}
