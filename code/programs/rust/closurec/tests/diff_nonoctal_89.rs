//! Integration test for the `tests/diff/nonoctal-89/` fixture.
//!
//! Exercises the `\8` / `\9` non-octal decimal escape fix in
//! `javascript-parser` 0.61.0.
//!
//! ## Why the backslash is dropped
//!
//! `\8` and `\9` (ECMAScript Annex B.1.2 *NonOctalDecimalEscapeSequence*) are
//! NOT octal escapes — 8 and 9 are not octal digits, so the legacy-octal decode
//! never matched them. Previously they fell through to the generic "unknown
//! escape" arm, which KEPT the backslash: `"\8"` decoded to the two-char value
//! `\8` instead of the one-char `"8"` — a value miscompile.
//!
//! In sloppy-mode string literals the backslash before `8`/`9` is simply
//! dropped and the decimal digit kept. The reference Closure Compiler decodes
//! them this way.
//!
//! ## Fact — SIMPLE: `g(a, "\8\9")` -> `g(a,"89")`
//!
//! Each backslash is dropped and the digit kept, yielding the two-char string
//! `"89"`. A backslash-preserving decode would have emitted `"\8\9"` (or its
//! re-escaped form) — a different value. Verified byte-identical to the real
//! Closure jar.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/nonoctal-89/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn nonoctal_decimal_escapes_drop_backslash() {
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
    let expected = std::fs::read_to_string("tests/diff/nonoctal-89/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // `\8\9` dropped both backslashes -> the string value is exactly `89`.
    // A backslash-preserving decode would leave a `\` in the output.
    let flat = actual.replace([' ', '\n'], "");
    assert_eq!(flat, r#"g(a,"89");"#, "expected the decoded value 89");
}
