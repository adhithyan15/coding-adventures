//! Regression tests for issue #17941:
//! `closurec parser rejects object method shorthand and async function expressions`
//!
//! Two ES2015+/ES2017 expression forms previously failed to parse in closurec
//! (SIMPLE), failing closed with parse errors:
//!
//! 1. Object-literal method shorthand: `var o = {f(){ return 1 }};` and `{valueOf(){…}}`
//!    failed with `Expected COLON or COMMA or RBRACE, got "("\n` because the grammar
//!    matched `NAME` before `method_definition`.
//! 2. `async function` expressions: `var p = async function () {};` and
//!    `var o = 1 ? 2 : async function () { return 1 };` failed with
//!    `Expected ARROW or NAME or LPAREN …, got "function"\n` because the grammar
//!    matched `NAME` for contextual keyword `async` before `async_function_expression`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn run_closurec(source: &str, level: &str) -> (bool, String, String) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-regression-17941-{}-{n}-{}",
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
fn test_object_method_shorthand_simple() {
    let (ok, stdout, stderr) = run_closurec("var o = {f(){ return 1 + 2; }};", "SIMPLE");
    assert!(ok, "compilation failed: {stderr}");
    assert_eq!(stdout.trim(), "var o={f(){return 3}};");
}

#[test]
fn test_object_valueof_method_shorthand_simple() {
    let (ok, stdout, stderr) = run_closurec("var o = {valueOf(){ return 1; }};", "SIMPLE");
    assert!(ok, "compilation failed: {stderr}");
    assert_eq!(stdout.trim(), "var o={valueOf(){return 1}};");
}

#[test]
fn test_async_function_expression_simple() {
    let (ok, stdout, stderr) = run_closurec("var p = async function () {};", "SIMPLE");
    assert!(ok, "compilation failed: {stderr}");
    assert_eq!(stdout.trim(), "var p=async function(){};");
}

#[test]
fn test_async_function_expression_ternary_simple() {
    let (ok, stdout, stderr) = run_closurec("var o = cond ? 2 : async function () { return 1 + 2; };", "SIMPLE");
    assert!(ok, "compilation failed: {stderr}");
    assert_eq!(stdout.trim(), "var o=cond?2:async function(){return 3};");
}

#[test]
fn test_async_function_expression_named_simple() {
    let (ok, stdout, stderr) = run_closurec("var p = async function foo() { return 1; };", "SIMPLE");
    assert!(ok, "compilation failed: {stderr}");
    assert_eq!(stdout.trim(), "var p=async function foo(){return 1};");
}

#[test]
fn test_async_generator_expression_simple() {
    let (ok, stdout, stderr) = run_closurec("var p = async function* () {};", "SIMPLE");
    assert!(ok, "compilation failed: {stderr}");
    assert_eq!(stdout.trim(), "var p=async function*(){};");
}
