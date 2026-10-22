//! Regression test for CCR-084 slice 2 (#15919): at WHITESPACE_ONLY a `throw`
//! keeps its `;`, even before a closing `}`.
//!
//! Upstream's `CodeGenerator` ends every `throw` with a forced semicolon,
//! `endStatement(/* needSemiColon= */ true, …)`, commented "otherwise safari
//! can't parse this". Every other statement lets a following `}` absorb its
//! `;`. Slice 1 (#17839) did this for the AST emitter. Here the token path
//! exempts a throw's terminator from gap-030 rule A (drop `;` before `}`).
//!
//! Every `expected` string was captured from the pinned
//! `closure-compiler-v20260915` jar (sha256 verified) under Java 21.0.10, using
//! `--compilation_level WHITESPACE_ONLY`. The list includes `throw` as a
//! method or property name (`class A{throw(){}}`, `{throw:1}`),
//! and the contrast rows that must keep dropping their `;`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// `(source, upstream WHITESPACE_ONLY output)`.
const CASES: &[(&str, &str)] = &[
    ("function f(){throw 1}", "function f(){throw 1;};"),
    ("function f(){throw 1;}", "function f(){throw 1;};"),
    ("{throw 1}", "{throw 1;};"),
    ("function f(){a();throw 1}", "function f(){a();throw 1;};"),
    ("try{throw 1}catch(e){}", "try{throw 1;}catch(e){};"),
    ("function f(){return}", "function f(){return};"),
    ("while(1){continue}", "while(1)continue;"),
    ("function f(){a()}", "function f(){a()};"),
    ("switch(a){case 1:throw 1}", "switch(a){case 1:throw 1;};"),
    ("f(function(){throw new Error(\"x\")})", "f(function(){throw new Error(\"x\");});"),
    ("class A{throw(){}}", "class A{throw(){}};"),
    // `a.throw(1)` (upstream `a.throw(1);`) is left out: closurec's grammar
    // rejects a reserved word as a property name, so ASI can't run on it and
    // its final `;` is missing for a reason unrelated to `throw` statements.
    ("x={throw:1}", "x={throw:1};"),
    ("function f(){throw(e)}", "function f(){throw e;};"),
    ("function f(){if(a)throw 1}", "function f(){if(a)throw 1;};"),
    ("function f(){throw function(){a()}}", "function f(){throw function(){a()};};"),
];

fn compile_ws(source: &str) -> String {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-ws-throw-{}-{n}-{}",
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
        .args(["--compilation_level", "WHITESPACE_ONLY", "--js"])
        .arg(&path)
        .output()
        .expect("run closurec");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(out.status.success(), "{source:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout)
        .expect("UTF-8 output")
        .trim_end_matches('\n')
        .to_string()
}

#[test]
fn whitespace_only_throw_terminator_matches_upstream() {
    for (source, expected) in CASES {
        assert_eq!(&compile_ws(source), expected, "source: {source:?}");
    }
}
