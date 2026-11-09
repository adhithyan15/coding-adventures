//! Regression test for #17887 (Bug 3): WHITESPACE_ONLY drops a statement break
//! after `yield` used as an identifier in sloppy mode.
//!
//! In sloppy mode outside a generator, `yield` is a valid identifier. When followed
//! by a newline and a division operator (e.g. `var x = yield\n/ 2;`), WHITESPACE_ONLY
//! must preserve the statement break between the preceding statement and `var x`,
//! and must not insert a semicolon before the division operator `/`.
//!
//! Prior to this fix:
//! `var yield = 4\nvar x = yield\n/ 2;`
//! was minified to:
//! `var yield=4 var x=yield\n/2;`
//! which dropped the semicolon between `4` and `var`, causing a syntax error in Node.js.

use std::fs;
use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn run_closurec(source: &str, level: &str) -> (String, String, i32) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-ws-yield-17887-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&dir).expect("fresh scratch dir");
    let in_file = dir.join("input.js");
    fs::write(&in_file, source).unwrap();

    let output = Command::new(BINARY)
        .arg("--js")
        .arg(&in_file)
        .arg("--compilation_level")
        .arg(level)
        .output()
        .unwrap();

    let _ = fs::remove_dir_all(&dir);

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
fn test_whitespace_only_yield_as_identifier_break() {
    let src = "var yield = 4\nvar x = yield\n/ 2;\nconsole.log(\"ok\", x);";
    let (stdout, stderr, code) = run_closurec(src, "WHITESPACE_ONLY");
    assert_eq!(code, 0, "closurec failed: {stderr}");

    // Semicolon after `4` before `var` must be preserved:
    assert!(
        stdout.contains("4;var") || stdout.contains("4;\nvar"),
        "expected statement terminator between statements: {stdout}"
    );

    // If node is available, execute the minified code to verify runtime semantics:
    if let Some((node_out, success)) = node_eval(&stdout) {
        assert!(success, "node execution failed for minified code: {stdout}");
        assert_eq!(node_out.trim(), "ok 2", "unexpected output: {node_out}");
    }
}

#[test]
fn test_whitespace_only_yield_assignment_and_binary_continuation() {
    let src = "var a = 10\nyield = 6\nvar x = yield\n/ 2;\nconsole.log(\"val\", x);";
    let (stdout, stderr, code) = run_closurec(src, "WHITESPACE_ONLY");
    assert_eq!(code, 0, "closurec failed: {stderr}");

    if let Some((node_out, success)) = node_eval(&stdout) {
        assert!(success, "node execution failed for minified code: {stdout}");
        assert_eq!(node_out.trim(), "val 3", "unexpected output: {node_out}");
    }
}
