//! Integration test for the `tests/diff/source-map-location-mapping-bad/` fixture.
//!
//! Exercises CLOC11.41 — `--source_map_location_mapping` malformed
//! value. Sibling to the CLOC11.40 fix for `--source_map_input`:
//! same silent-drop bug in the same `read_source_map` function.
//! Pre-CLOC11.41, a typo'd `--source_map_location_mapping src/`
//! silently vanished via `filter_map`, leaving the user wondering
//! why their map URLs didn't rewrite. Now errors with both the
//! flag name and the offending value.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string(
        "tests/diff/source-map-location-mapping-bad/flags.txt",
    )
    .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn malformed_location_mapping_errors_with_flag_and_value() {
    let flags = read_flags();
    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .expect("run closurec");

    // Non-zero exit — a malformed --source_map_location_mapping
    // is a config-level error, propagated through main.rs's
    // ParserOutput::Parse branch as ExitCode::from(1).
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

    assert!(
        combined.contains("--source_map_location_mapping"),
        "error must mention --source_map_location_mapping. Got:\n{combined}"
    );

    assert!(
        combined.contains("malformed-no-pipe-separator"),
        "error must echo the malformed value. Got:\n{combined}"
    );
}
