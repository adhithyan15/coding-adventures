//! Integration test for the CV JSON summary format
//! introduced in CLOC11.74.
//!
//! Exercises:
//!
//!   --correlation_vector              (CLOC11.60)
//!   --correlation_vector_summary      (CLOC11.73)
//!   --correlation_vector_summary_format JSON (CLOC11.74)
//!   --js_output_file <tmp>/out.js
//!
//! Contract this test pins down:
//!
//!   1. stdout contains a single-line JSON object that
//!      parses cleanly via `serde_json::from_str`.
//!   2. The top-level key is `cv_sidecar`; its value is an
//!      object.
//!   3. The object has all six expected fields with the
//!      right JSON types:
//!        - `path`: string (when a sidecar was written)
//!        - `skipped`: bool (false when written)
//!        - `entries`: integer
//!        - `contributions`: integer
//!        - `tombstones`: integer
//!        - `pass_order`: array of strings
//!   4. closurec exits 0 and writes the JS output normally.
//!
//! Why a separate integration test (in addition to CLOC11.74
//! unit tests in run.rs): unit tests verify the JSON
//! serializer in isolation. This test drives it through the
//! full binary path — CLI parse -> wire -> run_compiler ->
//! summary_line -> stdout — and parses the result back with
//! serde_json, so any drift that breaks JSON well-formedness
//! (e.g. an unescaped path containing a quote) shows up here
//! immediately. Completes the trio (TEXT covered by 11.77,
//! KV by 11.79, JSON by 11.80).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

#[test]
fn cv_json_summary_emits_parseable_object_with_expected_fields() {
    // Per-test temp dir to avoid sibling-test collisions.
    let dir = std::env::temp_dir().join("closurec_cloc11_80_json_summary");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");

    let out_path = dir.join("out.js");
    let input_path = "tests/diff/cv-json-summary/input/a.js";

    let res = Command::new(BINARY)
        .args([
            "--correlation_vector",
            "--correlation_vector_summary",
            "--correlation_vector_summary_format",
            "JSON",
            "--js",
            input_path,
            "--js_output_file",
            out_path.to_str().expect("utf-8 out_path"),
        ])
        .output()
        .expect("run closurec");

    // (4): clean exit, JS file produced.
    assert!(
        res.status.success(),
        "exit: {:?}, stderr: {}",
        res.status.code(),
        String::from_utf8_lossy(&res.stderr),
    );
    assert!(
        out_path.exists(),
        "expected JS output at {:?}",
        out_path
    );

    let stdout = String::from_utf8_lossy(&res.stdout);
    let line = stdout.trim();

    // (1): the trimmed line parses as JSON.
    let parsed: serde_json::Value = serde_json::from_str(line)
        .unwrap_or_else(|err| {
            panic!("JSON summary did not parse ({err}): {line}")
        });

    // (2): top-level key is `cv_sidecar` and its value is an
    // object.
    let cv = parsed
        .get("cv_sidecar")
        .and_then(|v| v.as_object())
        .unwrap_or_else(|| panic!("missing cv_sidecar object, got: {line}"));

    // (3): each field present with the right type.
    let path = cv
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("cv_sidecar.path missing or non-string: {line}"));
    assert!(
        !path.is_empty(),
        "cv_sidecar.path should be non-empty when a sidecar is written"
    );
    assert_eq!(
        cv.get("skipped"),
        Some(&serde_json::Value::Bool(false)),
        "cv_sidecar.skipped should be false when a sidecar is written"
    );
    let _entries = cv
        .get("entries")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| panic!("cv_sidecar.entries missing or non-integer: {line}"));
    let _contributions = cv
        .get("contributions")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| {
            panic!("cv_sidecar.contributions missing or non-integer: {line}")
        });
    let _tombstones = cv
        .get("tombstones")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| panic!("cv_sidecar.tombstones missing or non-integer: {line}"));
    let pass_order = cv
        .get("pass_order")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("cv_sidecar.pass_order missing or non-array: {line}"));
    for (i, entry) in pass_order.iter().enumerate() {
        assert!(
            entry.as_str().is_some(),
            "cv_sidecar.pass_order[{i}] should be a string, got: {entry:?}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}
