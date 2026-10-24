//! Regression test for #18134: inlining must not let a nested function close
//! over a caller's shadowing binding.
//!
//! ```js
//! var y=1;
//! function t(){return function(){return y}}
//! function g(){var y=2;return t()()}
//! console.log(g())                         // 1
//! ```
//!
//! ADVANCED spliced `t` into `g`, so the returned function read `g`'s `y`,
//! and the program printed `2`. The inliner's capture guards never looked
//! inside a nested `function` or method body, so they missed the free `y`.
//!
//! When `node` is on `PATH`, each program below must print the same thing
//! after SIMPLE and ADVANCED as before.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

const PROGRAMS: &[&str] = &[
    // The issue's shape: the nested function reads a `y` that `g` shadows.
    "var y=1;function t(){return function(){return y}}function g(){var y=2;return t()()}console.log(g())",
    // The same through a class method body.
    "var y=1;function t(){return class{m(){return y}}}function g(){var y=2;return new(t())().m()}console.log(g())",
    // The nested function declares its own `y`: no capture either way.
    "var y=1;function t(){return function(){var y=3;return y}}function g(){var y=2;return t()()}console.log(g())",
    // A self-contained nested function is still inlined.
    "function t(){return function(){return 7}}console.log(t()())",
];

fn compile(source: &str, level: &str) -> String {
    // Tests run in parallel in one process: a counter keeps scratch dirs apart.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-inline-nested-{}-{n}-{}",
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

/// Run `node -e code` and return what it printed, or `None` when node isn't
/// installed.
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
fn the_capturing_helper_is_not_spliced_at_advanced() {
    // Without node, still pin the issue's shape: `t` stays a function.
    let out = compile(PROGRAMS[0], "ADVANCED");
    assert!(out.contains("function t()"), "t was spliced into g: {out}");
}

#[test]
fn compiled_output_prints_what_the_source_prints() {
    for source in PROGRAMS {
        let Some(source_says) = node_stdout(source) else {
            eprintln!("node not on PATH; skipping the runtime comparison");
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
