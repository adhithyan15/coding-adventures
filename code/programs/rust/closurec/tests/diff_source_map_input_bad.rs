//! Integration test for the `tests/diff/source-map-input-bad/` fixture.
//!
//! Exercises CLOC11.40 — `--source_map_input` malformed value.
//! Previously the malformed entry was silently dropped via
//! `filter_map`, leaving the user wondering why their source-map
//! chain didn't apply. CC errors on this; we now match by
//! emitting a typed `ConfigError::InvalidSourceMapInput`.
//!
//! Exit must be non-zero, and the error must name both the flag
//! and the offending value so the user knows what to fix.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/source-map-input-bad/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn malformed_source_map_input_errors_with_flag_and_value() {
    let flags = read_flags();
    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .expect("run closurec");

    // Non-zero exit — a malformed --source_map_input is a
    // config-level error (exit code 1 from wire.rs's
    // config_from_parsed failure path).
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

    // The error must name the flag, so the user sees which
    // argument was malformed.
    assert!(
        combined.contains("--source_map_input"),
        "error must mention --source_map_input. Got:\n{combined}"
    );

    // And the malformed value must surface verbatim so the user
    // can locate it in their argv.
    assert!(
        combined.contains("this-is-malformed-no-pipe-separator"),
        "error must echo the malformed value. Got:\n{combined}"
    );
}
