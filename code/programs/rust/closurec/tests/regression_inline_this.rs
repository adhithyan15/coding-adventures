//! Regression test for #18115: the inliner must not splice a function that
//! reads `this` (or `super`, `new.target`) into a caller.
//!
//! ```js
//! function t(){return this}
//! var o={m:function(){return t()}};
//! console.log(o.m()===o)                  // false
//! ```
//!
//! A plain call `t()` gives `t` its own `this`. ADVANCED inlined `t` into
//! `m`, which left `return this` reading `m`'s `this`, so the program printed
//! `true`. `this` is not an identifier in our AST, so the inliner's capture
//! guard, which rejects any name that isn't a parameter, never saw it. It
//! also never looked inside arrow bodies, which inherit `this`.
//!
//! When `node` is on `PATH`, each program below must print the same thing
//! after SIMPLE and ADVANCED as before.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

const PROGRAMS: &[&str] = &[
    // The issue's shape: the expression inliner.
    "function t(){return this}var o={m:function(){return t()}};console.log(o.m()===o)",
    // Through an arrow, which reads `t`'s `this`.
    "function t(){return ()=>this}var o={m:function(){return t()()}};console.log(o.m()===o)",
    // A statement-level (void) helper.
    "function t(){console.log(this===globalThis)}var o={m:function(){t()}};o.m()",
    // `new.target` is bound by the call too.
    "function t(){return new.target===undefined}function F(){console.log(t())}new F",
    // A helper that only returns a function is still inlined; that
    // function keeps its own `this`.
    "function t(){return function(){return this}}console.log(typeof t())",
];

fn compile(source: &str, level: &str) -> String {
    // Tests run in parallel in one process: a counter keeps scratch dirs apart.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-inline-this-{}-{n}-{}",
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
fn the_this_helper_is_not_spliced_at_advanced() {
    // Without node, still pin the issue's shape: `t` stays a function.
    let out = compile(PROGRAMS[0], "ADVANCED");
    assert!(
        !out.contains("m:function(){return this}"),
        "t was spliced into m: {out}"
    );
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
            assert_eq!(compiled_says, source_says, "{source:?} at {level}: {compiled}");
        }
    }
}
