//! Integration test for the `tests/diff/octal-3digit/` fixture.
//!
//! Exercises the three-octal-digit correction in `javascript-parser` 0.60.0.
//!
//! ## Why three digits
//!
//! 0.59.0 decoded legacy octal escapes but capped a leading `4`–`7` at two
//! octal digits (ECMAScript Annex B.1.2). The reference Closure Compiler does
//! NOT apply that cap — it reads up to THREE octal digits regardless of the
//! leading digit, so `"\401"` decodes to U+0101 (octal 401 = 257), not the
//! two-digit `"\40"` + `"1"` (a space + `"1"`). Byte-identity requires matching
//! Closure even where it is non-conformant.
//!
//! ## Fact — SIMPLE: `g(a, "\401")` → `g(a,"ā")`
//!
//! The escape decodes to U+0101, emitted as `ā`. A two-digit read would
//! have produced `" 1"` (space + `"1"`); the presence of `ā` proves the
//! three-digit read. Verified byte-identical to the real Closure jar.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/octal-3digit/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn leading_four_to_seven_octal_reads_three_digits() {
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
    let expected = std::fs::read_to_string("tests/diff/octal-3digit/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // `\401` read THREE digits -> U+0101 (`ā`). A two-digit read would give
    // `\40` (space) + "1".
    assert!(
        flat.contains("\\u0101"),
        "`\\401` did not read three octal digits to U+0101: {actual}"
    );
}
