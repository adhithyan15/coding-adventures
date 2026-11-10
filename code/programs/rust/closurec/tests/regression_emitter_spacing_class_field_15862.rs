//! Regression tests for issue #15862 (CCR-077):
//! `[closurec] CCR-077 emitter spacing and class-body punctuation differ from upstream`
//!
//! 1. A space after `function*`:
//!    Upstream emits `function*g()`, closurec previously emitted `function* g()`.
//! 2. A missing terminator after a class field at WHITESPACE_ONLY:
//!    Upstream emits `class A{x=1;}`, closurec previously dropped the `;` before `}`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn run_closurec(source: &str, level: &str) -> (bool, String, String) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-regression-15862-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).expect("fresh scratch dir");
    let path = dir.join("input.js");
    std::fs::write(&path, source).expect("write tempfile");

    let out = Command::new(BINARY)
        .arg("-O")
        .arg(level)
        .arg("--js")
        .arg(&path)
        .output()
        .expect("run closurec");

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let _ = std::fs::remove_dir_all(&dir);

    (out.status.success(), stdout, stderr)
}

#[test]
fn test_generator_no_space_simple() {
    let (ok, stdout, stderr) = run_closurec(
        "function* g(){yield 1}console.log([...g()]);",
        "SIMPLE",
    );
    assert!(ok, "compilation failed: {stderr}");
    assert_eq!(stdout, "function*g(){yield 1}console.log([...g()]);\n");
}

#[test]
fn test_class_field_terminator_whitespace_only() {
    let (ok, stdout, stderr) = run_closurec(
        "class A{x=1;}console.log((new A).x);",
        "WHITESPACE_ONLY",
    );
    assert!(ok, "compilation failed: {stderr}");
    assert_eq!(stdout, "class A{x=1;}console.log((new A).x);\n");
}

#[test]
fn test_class_field_asi_terminator_whitespace_only() {
    let (ok, stdout, stderr) = run_closurec(
        "class A{\n  x = 1\n}\nconsole.log((new A).x);",
        "WHITESPACE_ONLY",
    );
    assert!(ok, "compilation failed: {stderr}");
    assert_eq!(stdout, "class A{x=1;}console.log((new A).x);\n");
}
