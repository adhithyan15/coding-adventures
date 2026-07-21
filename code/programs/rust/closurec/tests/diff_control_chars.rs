//! Integration test for the `tests/diff/control-chars/` fixture.
//!
//! Exercises `closure-emitter` 0.56.0: control characters inside string
//! literals render exactly as the reference Closure Compiler renders them.
//!
//! ## Facts (oracle-verified against the real Closure jar, SIMPLE)
//!
//! `expected.stdout` in this fixture was produced BY THE ORACLE
//! (`closure-compiler-v20260712.jar`, `--compilation_level
//! SIMPLE_OPTIMIZATIONS --language_in ECMASCRIPT_2020 --language_out
//! NO_TRANSPILE`), not by closurec -- so this test pins real byte-identity
//! rather than merely freezing whatever closurec happens to emit.
//!
//! The four rules it covers:
//!
//! - `U+0000` -> `\x00`  -- NUL is the ONLY code point Closure renders with
//!   the `\x` form. `\x01` would be shorter than `\u0001`, yet Closure still
//!   emits `\u0001`, so the shortening must NOT be generalised.
//! - `U+001B` -> `\u001b` -- LOWERCASE hex digits.
//! - `U+007F` -> `\u007f` -- DEL sits ABOVE the C0 block, so a naive
//!   `< 0x20` guard misses it and leaks a raw DEL byte into the output.
//! - `U+0008` / `U+000B` / `U+000C` -> `\b` / `\v` / `\f` -- the short escapes.
//!
//! The fixture is deliberately kept well under Closure's ~500-char output
//! line-length budget, so it does not also depend on the line-wrapping gap
//! tracked separately.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/control-chars/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn control_chars_match_closure_byte_for_byte() {
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
    let expected = std::fs::read_to_string("tests/diff/control-chars/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}expected:\n{expected}",
    );
}

#[test]
fn no_raw_control_byte_survives_into_the_output() {
    // The regression that motivated this fixture: DEL (0x7F) was not escaped
    // at all, so a raw control byte could reach the emitted JavaScript.
    let flags = read_flags();
    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .expect("run closurec");
    let bytes = &out.stdout;
    let leaked: Vec<u8> = bytes
        .iter()
        .copied()
        // Only LF is legitimate here: compact output has no indentation, so a
        // raw TAB would mean a control byte leaked out of a string literal.
        .filter(|b| (*b < 0x20 && *b != b'\n') || *b == 0x7f)
        .collect();
    assert!(
        leaked.is_empty(),
        "raw control bytes leaked into the output: {leaked:?}"
    );
}
