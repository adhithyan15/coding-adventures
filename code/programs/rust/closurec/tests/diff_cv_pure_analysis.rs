//! Integration test for the `tests/diff/cv-pure-analysis/` fixture.
//!
//! Exercises the full CV pure-analysis combo built up across
//! CLOC11.60 → 11.76:
//!
//!   --correlation_vector              (CLOC11.60)
//!   --correlation_vector_summary      (CLOC11.73)
//!   --correlation_vector_summary_only (CLOC11.76)
//!   --correlation_vector_format NONE  (CLOC11.69)
//!
//! Contract this test pins down:
//!
//!   1. With `--correlation_vector_summary_only`, no JS file
//!      lands on disk even though `--js` is supplied.
//!   2. With `--correlation_vector_format NONE`, no CV sidecar
//!      lands either — pure in-memory analysis, no writes.
//!   3. The CV summary line *still* makes it to stdout because
//!      `--correlation_vector_summary` is on and the
//!      `summary_stderr` flag is off (default).
//!   4. closurec exits 0 — the pure-analysis combo is a normal
//!      successful invocation, not an error path.
//!
//! Why this exists as its own integration test (not just unit
//! tests in run.rs): the combination touches CLI parsing, wire
//! reading, four config fields, three skip-gates in
//! run_compiler, and the summary serializer. A single
//! end-to-end test through the actual binary catches integration
//! drift that per-feature unit tests would miss — e.g. a future
//! refactor that splits SpecialModesConfig and forgets to thread
//! one of the four flags would fail here even if every isolated
//! test still passed.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/cv-pure-analysis/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn cv_pure_analysis_writes_no_files_and_prints_summary() {
    let flags = read_flags();
    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .expect("run closurec");

    // Successful exit — pure-analysis is a valid invocation.
    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );

    let stdout = String::from_utf8_lossy(&out.stdout);

    // (1) + (3): stdout contains the CLOC11.73 summary line.
    // We assert the marker substring rather than the exact
    // line because counts depend on the lexer's output for the
    // input — and that may evolve as later passes touch the
    // token stream. The structural marker stays stable.
    assert!(
        stdout.contains("cv sidecar:"),
        "expected `cv sidecar:` summary on stdout, got: {stdout:?}"
    );

    // (1) + (2): no files written anywhere relative to the
    // fixture. We check the conventional default sidecar
    // locations the absence of a `--js_output_file` would
    // otherwise place on disk.
    assert!(
        !std::path::Path::new("closurec-cv.json").exists(),
        "summary_only + format=NONE should not write the default \
         cwd sidecar (closurec-cv.json), found one"
    );
    assert!(
        !std::path::Path::new("tests/diff/cv-pure-analysis/input/a.js.cv.json")
            .exists(),
        "summary_only + format=NONE should not write any sidecar \
         alongside an input file"
    );

    // (3): no extra stderr output. stderr should be empty
    // because we did NOT set --correlation_vector_summary_stderr;
    // the summary is on stdout.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.is_empty() || !stderr.contains("cv sidecar:"),
        "expected NO summary on stderr by default, got: {stderr:?}"
    );
}
