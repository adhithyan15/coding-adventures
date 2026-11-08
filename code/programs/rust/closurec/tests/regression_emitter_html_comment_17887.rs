//! Regression test for #17887 (Bug 1): emitter writes out `<!--` HTML comment delimiter.
//!
//! ## The miscompile
//!
//! When minifying `a < !--b` (or `a < ! --b`), the emitter previously printed `a<!--b`
//! at WHITESPACE_ONLY, SIMPLE, and ADVANCED compilation levels.
//!
//! In ECMAScript Annex B.1.1 (script mode), `<!--` begins a SingleLineHTMLOpenComment.
//! Consequently, Node.js and browsers treat `<!--` and all subsequent text on that line
//! as a comment, silently discarding subsequent statements:
//!
//! ```js
//! var b=2,a=1; var x = a < !--b; console.log("ok", x);
//! ```
//!
//! printed `ok false` in Node.js before compilation, and nothing at all after `closurec`.
//!
//! ## The fix
//!
//! Both `closure-emitter` (for SIMPLE and ADVANCED) and `whitespace_only.rs` (for
//! WHITESPACE_ONLY) now preserve a separating space between `<` and `!` when immediately
//! followed by prefix `--`, producing `a< !--b;`, matching upstream Closure Compiler.
//!
//! When `node` is on `PATH`, the test executes the compiled output to verify that
//! code following `< !--` is executed and not swallowed as an HTML comment.

use std::fs;
use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn run_closurec(source: &str, level: &str) -> (String, String, i32) {
    let dir = std::env::temp_dir();
    let id = format!(
        "closurec_test_17887_{}_{}_{:?}",
        level,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let in_file = dir.join(format!("{id}.js"));
    fs::write(&in_file, source).unwrap();

    let output = Command::new(BINARY)
        .arg("--js")
        .arg(&in_file)
        .arg("--compilation_level")
        .arg(level)
        .output()
        .unwrap();

    let _ = fs::remove_file(&in_file);

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

fn node_eval(code: &str) -> Option<(String, bool)> {
    let output = Command::new("node").arg("-e").arg(code).output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Some((stdout, output.status.success()))
}

#[test]
fn test_html_comment_avoidance_whitespace_only() {
    let src = "var b=2,a=1; var x = a < !--b; console.log(\"ok\", x);";
    let (stdout, stderr, code) = run_closurec(src, "WHITESPACE_ONLY");
    assert_eq!(code, 0, "closurec failed: {stderr}");
    assert!(
        stdout.contains("< !--"),
        "expected '< !--' in output, got: {stdout}"
    );
    assert!(
        !stdout.contains("<!--"),
        "must not contain '<!--' HTML comment sequence: {stdout}"
    );

    if let Some((node_out, success)) = node_eval(&stdout) {
        assert!(success, "node execution failed for: {stdout}");
        assert_eq!(
            node_out.trim(),
            "ok false",
            "node did not execute expected statements: {node_out}"
        );
    }
}

#[test]
fn test_html_comment_avoidance_simple() {
    let src = "var b=2,a=1; var x = a < !--b; console.log(\"ok\", x);";
    let (stdout, stderr, code) = run_closurec(src, "SIMPLE");
    assert_eq!(code, 0, "closurec failed: {stderr}");
    assert!(
        stdout.contains("< !--"),
        "expected '< !--' in output, got: {stdout}"
    );
    assert!(
        !stdout.contains("<!--"),
        "must not contain '<!--' HTML comment sequence: {stdout}"
    );

    if let Some((node_out, success)) = node_eval(&stdout) {
        assert!(success, "node execution failed for: {stdout}");
        assert_eq!(
            node_out.trim(),
            "ok false",
            "node did not execute expected statements: {node_out}"
        );
    }
}

#[test]
fn test_html_comment_avoidance_advanced() {
    let src = "var b=2,a=1; var x = a < !--b; console.log(\"ok\", x);";
    let (stdout, stderr, code) = run_closurec(src, "ADVANCED");
    assert_eq!(code, 0, "closurec failed: {stderr}");
    assert!(
        stdout.contains("< !--"),
        "expected '< !--' in output, got: {stdout}"
    );
    assert!(
        !stdout.contains("<!--"),
        "must not contain '<!--' HTML comment sequence: {stdout}"
    );

    if let Some((node_out, success)) = node_eval(&stdout) {
        assert!(success, "node execution failed for: {stdout}");
        assert_eq!(
            node_out.trim(),
            "ok false",
            "node did not execute expected statements: {node_out}"
        );
    }
}

#[test]
fn test_spaced_source_tokens_avoid_html_comment() {
    // Even if source had spaces like `a < ! --b`, minifier must not collapse to `a<!--b`
    let src = "var b=2,a=1; var x = a < ! --b; console.log(\"ok\", x);";
    for level in ["WHITESPACE_ONLY", "SIMPLE", "ADVANCED"] {
        let (stdout, stderr, code) = run_closurec(src, level);
        assert_eq!(code, 0, "{level} compilation failed: {stderr}");
        assert!(
            stdout.contains("< !--"),
            "{level}: expected '< !--' in output, got: {stdout}"
        );
        assert!(
            !stdout.contains("<!--"),
            "{level}: must not contain '<!--': {stdout}"
        );

        if let Some((node_out, success)) = node_eval(&stdout) {
            assert!(success, "{level}: node execution failed for: {stdout}");
            assert_eq!(node_out.trim(), "ok false", "{level}: unexpected output: {node_out}");
        }
    }
}

#[test]
fn test_negative_cases_do_not_over_space() {
    // When not followed by `--`, `< !` does NOT need a separating space
    let cases = [
        ("a < !b;", "a<!b;"),
        ("a < !++b;", "a<!++b;"),
        ("a < ! - -b;", "a<!- -b;"),
        ("a << !--b;", "a<<!--b;"),
        ("a <= !--b;", "a<=!--b;"),
    ];

    for (src, expected) in cases {
        let (stdout, stderr, code) = run_closurec(src, "WHITESPACE_ONLY");
        assert_eq!(code, 0, "WHITESPACE_ONLY failed for {src}: {stderr}");
        assert_eq!(stdout.trim(), expected, "WHITESPACE_ONLY output mismatch for {src}");
    }
}
