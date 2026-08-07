//! Integration test for the CV KV summary format
//! introduced in CLOC11.74.
//!
//! Exercises:
//!
//!   --correlation_vector              (CLOC11.60)
//!   --correlation_vector_summary      (CLOC11.73)
//!   --correlation_vector_summary_format KV (CLOC11.74)
//!   --js_output_file <tmp>/out.js
//!
//! Contract this test pins down:
//!
//!   1. stdout contains the CV summary in space-separated
//!      `key=value` form, every key prefixed with
//!      `cv_sidecar.`.
//!   2. The path RHS is quoted (`cv_sidecar.path="..."`) so
//!      shell tooling that splits on whitespace can recover
//!      the (possibly-spaced) path safely.
//!   3. Numeric and bool RHS values are bare (no quotes) —
//!      `cv_sidecar.entries=N`, `cv_sidecar.skipped=false`
//!      — so awk/cut consumers don't have to strip quotes.
//!   4. `pass_order` is also quoted (the comma-joined list
//!      would otherwise look like multiple keys to a naive
//!      consumer).
//!   5. closurec exits 0 and writes the JS output normally
//!      (summary_only is NOT set — this confirms KV summary
//!      coexists with a real compile).
//!
//! Why a separate integration test (in addition to the
//! CLOC11.74 unit tests in run.rs): unit tests verify the
//! KV serializer in isolation. This test drives it through
//! the full binary path — CLI parse -> wire -> run_compiler
//! -> summary_line -> stdout — catching layer drift that
//! per-feature unit tests would miss.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

#[test]
fn cv_kv_summary_emits_quoted_path_and_bare_numerics() {
    // Per-test temp dir; avoids collision with sibling test
    // runs and makes cleanup trivial.
    let dir = std::env::temp_dir().join("closurec_cloc11_79_kv_summary");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");

    let out_path = dir.join("out.js");
    let input_path = "tests/diff/cv-kv-summary/input/a.js";

    let res = Command::new(BINARY)
        .args([
            "--correlation_vector",
            "--correlation_vector_summary",
            "--correlation_vector_summary_format",
            "KV",
            "--js",
            input_path,
            "--js_output_file",
            out_path.to_str().expect("utf-8 out_path"),
        ])
        .output()
        .expect("run closurec");

    // (5): clean exit, JS file produced.
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

    // (1) + (2): KV-prefixed key with quoted path on the RHS.
    assert!(
        stdout.contains("cv_sidecar.path=\""),
        "expected quoted cv_sidecar.path=\"...\" in KV stdout, got: {stdout:?}"
    );

    // (3): bool RHS is bare.
    assert!(
        stdout.contains("cv_sidecar.skipped=false"),
        "expected bare cv_sidecar.skipped=false, got: {stdout:?}"
    );

    // (3): numeric RHS is bare. We don't pin a count; just
    // assert the `key=` prefix is followed by a digit.
    let entries_idx = stdout
        .find("cv_sidecar.entries=")
        .unwrap_or_else(|| panic!("missing cv_sidecar.entries=, got: {stdout:?}"));
    let after = &stdout[entries_idx + "cv_sidecar.entries=".len()..];
    let next_char = after
        .chars()
        .next()
        .expect("entries= followed by something");
    assert!(
        next_char.is_ascii_digit(),
        "expected digit after cv_sidecar.entries=, got {:?}: {stdout:?}",
        next_char
    );

    // (4): pass_order is quoted.
    assert!(
        stdout.contains("cv_sidecar.pass_order=\""),
        "expected quoted cv_sidecar.pass_order=\"...\", got: {stdout:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
