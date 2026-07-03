//! Integration test for the `tests/diff/output-wrapper-error/` fixture.
//!
//! Exercises CLOC11.32 — `--output_wrapper` missing `%output%`
//! placeholder. Mirrors CC's `AbstractCommandLineRunner.checkFlags`:
//! a non-empty wrapper without the placeholder is a hard error
//! with the exact message
//!   `ERROR - No %output% placeholder in the output wrapper`
//!
//! The fixture pins that CC-compat message verbatim — tools that
//! grep CC's stderr for it must see the same string from closurec.
//! Exit code is non-zero (we use 2 for compile-time errors;
//! CC uses different non-zero codes, the exact value isn't part
//! of the compat contract).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/output-wrapper-error/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn missing_output_placeholder_emits_cc_error_and_exits_nonzero() {
    let flags = read_flags();
    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .expect("run closurec");

    // Non-zero exit: this is a compiler error, not success.
    assert!(
        !out.status.success(),
        "expected non-zero exit, got success. stdout: {}",
        String::from_utf8_lossy(&out.stdout),
    );

    // The CC-compat error message lands on stdout (closurec
    // prints all output to stdout today — that's the binary's
    // current convention, not part of the compat surface).
    // We assert presence rather than exact-match to leave room
    // for a future move to stderr; the *message string* is what
    // matters for compat.
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("ERROR - No %output% placeholder in the output wrapper"),
        "CC-compat message missing from output. Got:\n{combined}"
    );
}
