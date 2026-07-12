//! Integration test for the CV NDJSON streaming sidecar
//! shape introduced in CLOC11.69.
//!
//! Exercises:
//!
//!   --correlation_vector             (CLOC11.60)
//!   --correlation_vector_format NDJSON (CLOC11.69)
//!   --js_output_file <tmp>/out.js
//!
//! Contract this test pins down:
//!
//!   1. The sidecar lands at `<js_output_file>.cv.json` (the
//!      CLOC11.67 default-path policy).
//!   2. The sidecar is **newline-delimited JSON**: every
//!      non-empty line parses as a standalone JSON value.
//!   3. There are at least 2 lines (at least one CV entry +
//!      the `_meta` footer).
//!   4. The final line is the `{"_meta": {...}}` footer
//!      object so streaming consumers (`tail -f`, jq) can
//!      reliably extract `pass_order` after the producer
//!      finishes.
//!   5. closurec exits 0.
//!
//! Why this exists as its own integration test (in addition
//! to the CLOC11.69 unit tests in run.rs): unit tests verify
//! `format_cv_log_ndjson` in isolation. This test exercises
//! the full path — CLI parse → wire → run_compiler →
//! formatter → disk write → consumer-style readback —
//! through the actual binary. Catches drift in any of those
//! layers, especially path resolution and the
//! `--js_output_file`-sibling sidecar convention.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

#[test]
fn cv_ndjson_sidecar_is_line_delimited_json_with_meta_footer() {
    // Per-test temp dir so the sidecar doesn't collide with
    // sibling test runs and we can clean up after.
    let dir = std::env::temp_dir().join("closurec_cloc11_78_ndjson_streaming");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");

    let out_path = dir.join("out.js");
    let sidecar_path = dir.join("out.js.cv.json");
    let input_path = "tests/diff/cv-ndjson-streaming/input/a.js";

    let res = Command::new(BINARY)
        .args([
            "--correlation_vector",
            "--correlation_vector_format",
            "NDJSON",
            "--js",
            input_path,
            "--js_output_file",
            out_path.to_str().expect("utf-8 out_path"),
        ])
        .output()
        .expect("run closurec");

    // (5): clean exit.
    assert!(
        res.status.success(),
        "exit: {:?}, stderr: {}",
        res.status.code(),
        String::from_utf8_lossy(&res.stderr),
    );

    // (1): sidecar exists at the CLOC11.67 default path.
    assert!(
        sidecar_path.exists(),
        "expected CV sidecar at {:?}",
        sidecar_path
    );

    let body = std::fs::read_to_string(&sidecar_path).expect("read sidecar");
    let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();

    // (3): at least entries + footer.
    assert!(
        lines.len() >= 2,
        "expected ≥2 NDJSON lines (entries + meta footer), got {}: {body}",
        lines.len()
    );

    // (2): every line parses as JSON on its own.
    for (i, line) in lines.iter().enumerate() {
        serde_json::from_str::<serde_json::Value>(line).unwrap_or_else(|err| {
            panic!("NDJSON line {i} did not parse as JSON ({err}): {line}")
        });
    }

    // (4): final line is the `_meta` footer.
    let last = lines.last().expect("at least one line");
    let last_val: serde_json::Value =
        serde_json::from_str(last).expect("last line parses");
    let meta = last_val
        .get("_meta")
        .expect("last line should have a _meta key");
    assert!(
        meta.is_object(),
        "_meta should be an object, got: {meta:?}"
    );
    // pass_order should be present in the footer for
    // streaming consumers that want it without re-parsing
    // every entry line.
    assert!(
        meta.get("pass_order").is_some(),
        "_meta should include pass_order, got: {meta:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
