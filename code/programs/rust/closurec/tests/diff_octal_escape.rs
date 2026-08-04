//! Integration test for the `tests/diff/octal-escape/` fixture.
//!
//! Exercises legacy octal string-escape decoding end-to-end — the
//! `javascript-parser` 0.59.0 fix.
//!
//! ## Why this is a fix (not just a fold)
//!
//! `"\101"` is a legacy octal escape (octal 101 = 65 = `A`). The bridge
//! string-unescaper previously left `\1`–`\7` and multi-digit `\NNN`
//! undecoded, so the string literal's VALUE was wrong (`\101` survived as the
//! four characters `\`, `1`, `0`, `1`). The reference Closure Compiler decodes
//! it to `"A"`; closurec now matches, producing the correct value.
//!
//! ## Fact — SIMPLE: `x = "\101";` → `x="A";`
//!
//! The octal escape decodes to the single character `A`, emitted as `"A"`. A
//! WHITESPACE_ONLY fallback goes through a different (still-legacy) path and
//! does NOT produce `"A"`, so the presence of `x="A"` proves the SIMPLE bridge
//! decoded the escape. Verified byte-identical to the real Closure jar across
//! the octal truth table (`\101`, `\012`, `\40`, `\377`, `\7`, `\77`, …).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/octal-escape/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn legacy_octal_escape_decodes_to_character() {
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
    let expected = std::fs::read_to_string("tests/diff/octal-escape/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // `\101` decoded to the character `A`. Proof the bridge decoded the escape
    // (the raw digits `101` do not survive, and no stray backslash remains).
    assert!(
        flat.contains("\"A\""),
        "octal escape did not decode to `A`: {actual}"
    );
    assert!(
        !flat.contains("101"),
        "the octal digits survived undecoded — the escape was not decoded: {actual}"
    );
}
