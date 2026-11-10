//! Regression tests for issue #17852:
//! `[closurec] Grammar rejects two valid class/property shapes: adjacent class members, reserved-word property names`
//!
//! 1. Adjacent class members without semicolons between them:
//!    `class P { m() { a(); } n() { b(); } }`
//! 2. Reserved-word property names and member access:
//!    `a.throw(1); o.return(); x = { if: 1, default: 2 };`
//! 3. Reserved-word class method names:
//!    `class C { throw() {} return() {} }`

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn run_closurec(source: &str, level: &str) -> (bool, String, String) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-regression-17852-{}-{n}-{}",
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
fn test_adjacent_class_members_simple() {
    let (ok, stdout, stderr) = run_closurec(
        "class P { m() { a(); } n() { b(); } }",
        "SIMPLE",
    );
    assert!(ok, "compilation failed: {stderr}");
    assert_eq!(stdout.trim(), "class P{m(){a()}n(){b()}};");
}

#[test]
fn test_adjacent_class_members_advanced() {
    let (ok, stdout, stderr) = run_closurec(
        "class A{constructor(){this.x=1}get(){return this.x}}console.log((new A).get());",
        "ADVANCED",
    );
    assert!(ok, "compilation failed: {stderr}");
    assert!(stdout.contains("class A{constructor(){this.x=1}get(){return this.x}}"));
}

#[test]
fn test_reserved_words_member_access_and_object_lit_simple() {
    let (ok, stdout, stderr) = run_closurec(
        "a.throw(1); o.return(); var x = { if: 1, default: 2 };",
        "SIMPLE",
    );
    assert!(ok, "compilation failed: {stderr}");
    assert_eq!(stdout.trim(), "a.throw(1);o.return();var x={if:1,default:2};");
}

#[test]
fn test_reserved_words_class_methods_simple() {
    let (ok, stdout, stderr) = run_closurec(
        "class C { throw() {} return() {} }",
        "SIMPLE",
    );
    assert!(ok, "compilation failed: {stderr}");
    assert_eq!(stdout.trim(), "class C{throw(){}return(){}};");
}

#[test]
fn test_optional_chain_reserved_word_simple() {
    let (ok, stdout, stderr) = run_closurec(
        "a?.throw(); var y = o?.default;",
        "SIMPLE",
    );
    assert!(ok, "compilation failed: {stderr}");
    assert_eq!(stdout.trim(), "a?.throw();var y=o?.default;");
}
