//! Regression test for CCR-086 (#15923): DCE drops the dead tail after a
//! top-level `throw`, as it already did inside blocks.
//!
//! Every `expected` string was captured from the pinned
//! `closure-compiler-v20260915` jar (sha256 verified) under Java 21.0.10, using
//! `--compilation_level SIMPLE_OPTIMIZATIONS`. A tail holding a hoisted
//! declaration is kept whole (`throw 1;function g(){}`). Upstream instead
//! hoists a `var` (`throw 1;var z=1;` → `var z;throw 1;`); that gap is shared
//! with CCR-082 and not pinned here.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// `(source, upstream SIMPLE output)`.
const CASES: &[(&str, &str)] = &[
    ("throw 1;console.log(1);", "throw 1;"),
    ("{throw 1;console.log(1);}", "throw 1;"),
    ("throw 1;debugger;", "throw 1;"),
    ("f();throw 1;g();h();", "f();throw 1;"),
    ("throw 1;function g(){}", "throw 1;function g(){};"),
];

fn compile_simple(source: &str) -> String {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-toplevel-throw-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir(&dir).expect("fresh scratch dir");
    let path = dir.join("a.js");
    std::fs::write(&path, format!("{source}\n")).expect("write input");
    let out = Command::new(BINARY)
        .args(["--compilation_level", "SIMPLE", "--js"])
        .arg(&path)
        .output()
        .expect("run closurec");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(out.status.success(), "{source:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).expect("UTF-8").trim_end_matches('\n').to_string()
}

#[test]
fn top_level_dead_tail_matches_upstream() {
    for (source, expected) in CASES {
        assert_eq!(&compile_simple(source), expected, "source: {source:?}");
    }
}
