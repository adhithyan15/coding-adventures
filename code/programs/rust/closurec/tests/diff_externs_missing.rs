//! Integration test for the `tests/diff/externs-missing/` fixture.
//!
//! Exercises CLOC11.05 — `--externs` glob resolution. When a user
//! passes an `--externs` path that doesn't match any file, CC
//! errors out with a `JSC_NO_JS_FILES_FOUND_FOR_PATTERN`-style
//! message; we mirror that by tagging the error with the flag
//! name (`--externs:`) so the user can tell which glob was bad
//! without re-reading the command line.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/externs-missing/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn missing_externs_pattern_errors_and_names_the_flag() {
    let flags = read_flags();
    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .expect("run closurec");

    // Non-zero exit — a missing externs file is a CC-level error,
    // not a successful compile with a warning.
    assert!(
        !out.status.success(),
        "expected non-zero exit, got success. stdout: {}",
        String::from_utf8_lossy(&out.stdout),
    );

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // The error must name the flag, so the user sees which glob
    // failed without having to re-read their command line.
    assert!(
        combined.contains("--externs:"),
        "error must be prefixed with --externs:. Got:\n{combined}"
    );

    // And the missing path must surface so the user can fix the
    // typo or commit the file.
    assert!(
        combined.contains("this-file-does-not-exist.js"),
        "error must name the missing path. Got:\n{combined}"
    );
}
