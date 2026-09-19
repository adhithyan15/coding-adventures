//! Differential contract for CCR-002: SIMPLE and ADVANCED must fail closed.
//!
//! The reference observations were captured with Google Closure Compiler
//! `v20260915` (jar SHA-256
//! `9C8AF06056AA06F968B5A457540A85869C7BA2861C211C56D8D4EF6C35DDF36D`),
//! Java 21.0.12, and `--language_out NO_TRANSPILE`:
//!
//! - malformed `var = ;` exits 1, writes a `JSC_PARSE_ERROR` diagnostic, and
//!   emits no JavaScript;
//! - object destructuring compiles successfully upstream. Until closurec can
//!   represent its binding pattern, reporting a local bridge error is honest;
//!   silently substituting WHITESPACE_ONLY output is not.
//!
//! These tests intentionally drive the built executable. Unit tests prove the
//! internal error variants; this file pins the user-visible status, stream,
//! and file-system contract that build tooling actually observes.

use std::path::{Path, PathBuf};
use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn temp_output(level: &str, case: &str) -> (PathBuf, PathBuf) {
    let id = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "closurec-ccr002-{id}-{nanos}-{}-{case}",
        level.to_ascii_lowercase(),
    ));
    let output = dir.join("result.js");
    (dir, output)
}

fn assert_fails_closed(level: &str, input: &Path, expected_stage: &str, needle: &str) {
    let (dir, output_path) = temp_output(level, expected_stage);

    let output = Command::new(BINARY)
        .args([
            "--compilation_level",
            level,
            "--language_out",
            "NO_TRANSPILE",
            "--js",
        ])
        .arg(input)
        .arg("--js_output_file")
        .arg(&output_path)
        .output()
        .expect("run closurec");

    assert_eq!(
        output.status.code(),
        Some(1),
        "{level} {expected_stage} failure must use the compilation-error exit code; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        output.stdout.is_empty(),
        "failed compilation must not emit JavaScript on stdout: {}",
        String::from_utf8_lossy(&output.stdout),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "{level} compilation failed at {expected_stage} stage"
        )),
        "missing stable stage diagnostic in stderr: {stderr}",
    );
    assert!(
        stderr.contains(needle),
        "missing stage-specific detail {needle:?} in stderr: {stderr}",
    );
    assert!(
        !output_path.exists(),
        "failed compilation must not create {}",
        output_path.display(),
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn simple_and_advanced_parse_failures_match_the_upstream_failure_contract() {
    let input = Path::new("tests/diff/typed-pipeline-failure/input/malformed.js");
    for level in ["SIMPLE", "ADVANCED"] {
        assert_fails_closed(level, input, "parse", "Expected NAME");
    }
}

#[test]
fn simple_and_advanced_bridge_declines_are_explicit_failures() {
    let input = Path::new("tests/diff/typed-pipeline-failure/input/destructuring.js");
    for level in ["SIMPLE", "ADVANCED"] {
        assert_fails_closed(level, input, "typed AST bridge", "binding_pattern");
    }
}
