//! Regression test for #18146: Concise arrow body leftmost object parenthesisation.
//!
//! When an arrow function's concise body has an object literal as its leftmost token
//! (e.g. `() => ({})`, `() => ({}).x`, `() => ({})[1]`, `() => ({})()`), the object
//! literal must be wrapped in parentheses (`arrow_concise_start`) so JavaScript engines
//! do not mis-parse the bare `{` as a block body.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

const PROGRAMS: &[&str] = &[
    "var b = () => ({}); console.log(typeof b());",
    "var b = () => ({a: 1}); console.log(b().a);",
    "var b = (() => ({}).x)(); console.log(b);",
    "var b = (() => ({a: 42}).a)(); console.log(b);",
    "var b = (() => ({})[1])(); console.log(b);",
    "var b = (() => ({1: 'ok'})[1])(); console.log(b);",
    "var b = () => ({a: 1 + 2}.a); console.log(b());",
];

fn compile(source: &str, level: &str) -> String {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-arrow-concise-object-18146-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir(&dir).expect("fresh scratch dir");
    let path = dir.join("input.js");
    std::fs::write(&path, format!("{source}\n")).expect("write input");
    let out = Command::new(BINARY)
        .args(["--compilation_level", level, "--js"])
        .arg(&path)
        .output()
        .expect("run closurec");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{source:?} at {level}: exit {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout)
        .expect("closurec output is UTF-8")
        .trim_end_matches('\n')
        .to_string()
}

fn node_stdout(code: &str) -> Option<String> {
    let out = Command::new("node").args(["-e", code]).output().ok()?;
    assert!(
        out.status.success(),
        "node -e {code:?} failed: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn compiled_output_preserves_parens_and_runtime_semantics() {
    for source in PROGRAMS {
        let Some(source_says) = node_stdout(source) else {
            eprintln!("node not on PATH; skipping runtime comparison");
            return;
        };
        for level in ["SIMPLE", "ADVANCED"] {
            let compiled = compile(source, level);
            let compiled_says = node_stdout(&compiled).expect("node ran a moment ago");
            assert_eq!(
                compiled_says, source_says,
                "{source:?} at {level}: {compiled}"
            );
        }
    }
}
