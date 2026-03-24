//! # Integration tests for the `grammar-tools` CLI binary.
//!
//! These tests invoke the built binary via `std::process::Command` and check
//! its stdout output and exit code. They follow the pattern recommended in
//! "The Rust Programming Language" book for testing binary crates:
//! build the binary first, then drive it as a black box.
//!
//! # Why black-box tests for a CLI?
//!
//! Unit tests verify that individual functions return the right values.
//! But a CLI also has an interface contract with its users:
//!
//! - The exit code must be 0, 1, or 2 (not just "non-zero").
//! - The output format must match what CI scripts and humans expect.
//! - File-not-found errors must be caught and reported clearly.
//!
//! Black-box tests catch regressions in the CLI contract that unit tests
//! cannot see, because they exercise the full argument-parsing path.
//!
//! # How these tests find the binary
//!
//! Cargo sets the `CARGO_BIN_EXE_grammar-tools` environment variable to
//! the path of the built binary. We rely on this rather than guessing the
//! target directory, so the tests work in both `debug` and `release` modes.

use std::io::Write;
use std::process::Command;

// ===========================================================================
// Helper: get the path to the grammar-tools binary
// ===========================================================================

/// Return the path to the built `grammar-tools` binary.
///
/// Cargo populates `CARGO_BIN_EXE_<name>` for every `[[bin]]` in the
/// workspace when running integration tests. We use this instead of
/// hard-coding a `target/debug/` path.
fn bin_path() -> std::path::PathBuf {
    // The env variable name uses the binary name exactly as written in Cargo.toml.
    let path = env!("CARGO_BIN_EXE_grammar-tools");
    std::path::PathBuf::from(path)
}

// ===========================================================================
// Helper: write a temp file with given content
// ===========================================================================

/// Write `content` to a temporary file with the given suffix and return the
/// path. The caller is responsible for cleaning up (or letting the OS do it).
fn temp_file(suffix: &str, content: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    // Use a unique-enough name: thread id + suffix.
    let id = std::thread::current().id();
    path.push(format!("grammar_tools_test_{:?}{}", id, suffix));
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

// ===========================================================================
// --help / no args
// ===========================================================================

#[test]
fn test_help_flag_exits_zero() {
    let out = Command::new(bin_path())
        .arg("--help")
        .output()
        .expect("failed to run grammar-tools --help");
    assert_eq!(out.status.code(), Some(0), "help should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("grammar-tools"), "help should mention the binary name");
    assert!(stdout.contains("validate"), "help should mention the validate command");
}

#[test]
fn test_no_args_exits_zero_and_shows_usage() {
    let out = Command::new(bin_path())
        .output()
        .expect("failed to run grammar-tools with no args");
    // No args shows usage and exits 0 (mirrors Python implementation).
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("validate"));
}

// ===========================================================================
// validate — success paths
// ===========================================================================

#[test]
fn test_validate_ok_pair() {
    // A valid .tokens / .grammar pair should exit 0 and print "All checks passed."
    let tokens = temp_file(".tokens", "NUMBER = /[0-9]+/\nPLUS = \"+\"\n");
    let grammar = temp_file(".grammar", "expression = NUMBER { PLUS NUMBER } ;\n");

    let out = Command::new(bin_path())
        .arg("validate")
        .arg(&tokens)
        .arg(&grammar)
        .output()
        .expect("failed to run grammar-tools validate");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "stdout: {}", stdout);
    assert!(stdout.contains("All checks passed."), "stdout: {}", stdout);

    let _ = std::fs::remove_file(&tokens);
    let _ = std::fs::remove_file(&grammar);
}

#[test]
fn test_validate_ok_output_includes_counts() {
    // Output should include token count and rule count.
    let tokens = temp_file("_counts.tokens", "NUMBER = /[0-9]+/\nPLUS = \"+\"\n");
    let grammar = temp_file("_counts.grammar", "expression = NUMBER { PLUS NUMBER } ;\n");

    let out = Command::new(bin_path())
        .arg("validate")
        .arg(&tokens)
        .arg(&grammar)
        .output()
        .expect("failed to run validate");

    let stdout = String::from_utf8_lossy(&out.stdout);
    // Should say "2 tokens" and "1 rules".
    assert!(stdout.contains("2 tokens"), "stdout: {}", stdout);
    assert!(stdout.contains("1 rules"), "stdout: {}", stdout);

    let _ = std::fs::remove_file(&tokens);
    let _ = std::fs::remove_file(&grammar);
}

// ===========================================================================
// validate — error paths
// ===========================================================================

#[test]
fn test_validate_missing_tokens_file_exits_1() {
    let grammar = temp_file("_err.grammar", "expression = NUMBER ;\n");
    let out = Command::new(bin_path())
        .arg("validate")
        .arg("/nonexistent/path/file.tokens")
        .arg(&grammar)
        .output()
        .expect("failed to run validate");
    assert_eq!(out.status.code(), Some(1));
    let _ = std::fs::remove_file(&grammar);
}

#[test]
fn test_validate_wrong_arg_count_exits_2() {
    // 'validate' requires exactly two arguments.
    let out = Command::new(bin_path())
        .arg("validate")
        .arg("only_one_arg.tokens")
        .output()
        .expect("failed to run validate with wrong args");
    assert_eq!(out.status.code(), Some(2));
    // Error goes to stderr.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("two arguments") || stderr.contains("requires"), "stderr: {}", stderr);
}

// ===========================================================================
// validate-tokens — success
// ===========================================================================

#[test]
fn test_validate_tokens_ok() {
    let tokens = temp_file("_vt.tokens", "NUMBER = /[0-9]+/\nPLUS = \"+\"\n");
    let out = Command::new(bin_path())
        .arg("validate-tokens")
        .arg(&tokens)
        .output()
        .expect("failed to run validate-tokens");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "stdout: {}", stdout);
    assert!(stdout.contains("All checks passed."), "stdout: {}", stdout);
    let _ = std::fs::remove_file(&tokens);
}

#[test]
fn test_validate_tokens_missing_file_exits_1() {
    let out = Command::new(bin_path())
        .arg("validate-tokens")
        .arg("/nonexistent/no.tokens")
        .output()
        .expect("failed to run validate-tokens");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn test_validate_tokens_wrong_arg_count_exits_2() {
    let out = Command::new(bin_path())
        .arg("validate-tokens")
        .output()
        .expect("failed to run validate-tokens with no args");
    assert_eq!(out.status.code(), Some(2));
}

// ===========================================================================
// validate-grammar — success
// ===========================================================================

#[test]
fn test_validate_grammar_ok() {
    let grammar = temp_file("_vg.grammar", "expression = NUMBER { PLUS NUMBER } ;\n");
    let out = Command::new(bin_path())
        .arg("validate-grammar")
        .arg(&grammar)
        .output()
        .expect("failed to run validate-grammar");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "stdout: {}", stdout);
    assert!(stdout.contains("All checks passed."), "stdout: {}", stdout);
    let _ = std::fs::remove_file(&grammar);
}

#[test]
fn test_validate_grammar_missing_file_exits_1() {
    let out = Command::new(bin_path())
        .arg("validate-grammar")
        .arg("/nonexistent/no.grammar")
        .output()
        .expect("failed to run validate-grammar");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn test_validate_grammar_wrong_arg_count_exits_2() {
    let out = Command::new(bin_path())
        .arg("validate-grammar")
        .output()
        .expect("failed to run validate-grammar with no args");
    assert_eq!(out.status.code(), Some(2));
}

// ===========================================================================
// Unknown command
// ===========================================================================

#[test]
fn test_unknown_command_exits_2() {
    let out = Command::new(bin_path())
        .arg("frobnicate")
        .output()
        .expect("failed to run with unknown command");
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("frobnicate"), "stderr: {}", stderr);
}

// ===========================================================================
// Cross-validation warning (unused token)
// ===========================================================================

#[test]
fn test_validate_cross_warning_still_exits_0() {
    // An unused token is a warning, not an error. Exit code should be 0.
    let tokens = temp_file("_warn.tokens", "NUMBER = /[0-9]+/\nUNUSED = \"x\"\n");
    let grammar = temp_file("_warn.grammar", "expression = NUMBER ;\n");

    let out = Command::new(bin_path())
        .arg("validate")
        .arg(&tokens)
        .arg(&grammar)
        .output()
        .expect("failed to run validate");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "stdout: {}", stdout);
    // The "All checks passed." message should still appear.
    assert!(stdout.contains("All checks passed."), "stdout: {}", stdout);

    let _ = std::fs::remove_file(&tokens);
    let _ = std::fs::remove_file(&grammar);
}
