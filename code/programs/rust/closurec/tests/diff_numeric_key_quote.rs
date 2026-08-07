//! Integration test for the `tests/diff/numeric-key-quote/` fixture.
//!
//! Exercises the numeric object-key quoting rule in
//! `closure-pass-constant-fold` (0.106.0), which now matches Closure across
//! the full range thanks to the JS-exact `format_js_number` (0.105.0).
//!
//! ## The rule (oracle-verified against the real Closure jar)
//!
//! A numeric object key stays BARE (numeric) only when it is a non-negative
//! integer strictly below `2^53` (the safe-integer bound). Every other numeric
//! key is QUOTED with its JS `ToString`:
//!
//! - `100`               -> `100`                       (bare, safe integer)
//! - `4294967296`        -> `4294967296`                (bare, safe integer)
//! - `1e20`              -> `"100000000000000000000"`   (>= 2^53, quoted)
//! - `9007199254740992`  -> `"9007199254740992"`        (== 2^53, quoted)
//! - `1.5`               -> `"1.5"`                      (non-integer, quoted)
//! - `1e-7`              -> `"1e-7"`                     (non-integer, quoted)
//! - `1e21`              -> `"1e+21"`                    (exponential, quoted)
//!
//! Verified byte-identical to the real Closure jar.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/numeric-key-quote/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn numeric_object_keys_quote_per_closure_safe_integer_rule() {
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
    let expected = std::fs::read_to_string("tests/diff/numeric-key-quote/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
