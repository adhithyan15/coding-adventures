//! Integration test for the `tests/diff/octal-whitespace/` fixture.
//!
//! Exercises legacy octal string-escape decoding on the WHITESPACE_ONLY path —
//! the closurec `whitespace_only` fix (companion to the SIMPLE-path
//! `javascript-parser` 0.59.0 fix).
//!
//! ## Why this is a fix (not just a fold)
//!
//! The WHITESPACE_ONLY minifier's own string unescaper (`decode_js_string`)
//! left `\1`–`\7` and multi-digit `\NNN` undecoded — it dropped the backslash,
//! so `"\101"` became the wrong three-character value `"101"` instead of `"A"`
//! (octal 101 = 65 = `A`). The reference Closure Compiler decodes it — reading
//! UP TO THREE octal digits regardless of the leading digit (`\401` = U+0101,
//! not the Annex-B `\40`+`"1"`). closurec now matches at WHITESPACE_ONLY.
//!
//! ## Fact — WHITESPACE_ONLY: `x = "\101";` → `x="A";`
//!
//! The escape decodes to `A`. Before the fix the WHITESPACE_ONLY output was
//! `x="101";` (wrong value); now it is `x="A";`, byte-identical to the real
//! Closure jar.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/octal-whitespace/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn legacy_octal_escape_decodes_on_whitespace_only_path() {
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
    let expected = std::fs::read_to_string("tests/diff/octal-whitespace/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    assert!(
        flat.contains("\"A\""),
        "octal escape did not decode to `A` on the WHITESPACE_ONLY path: {actual}"
    );
    assert!(
        !flat.contains("101"),
        "the octal digits survived undecoded: {actual}"
    );
}
