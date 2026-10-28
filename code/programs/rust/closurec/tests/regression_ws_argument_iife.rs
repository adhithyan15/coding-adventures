//! Regression test for #18114: WHITESPACE_ONLY must not move an IIFE's call
//! out of an enclosing call.
//!
//! gap-051 rewrote every `} ( ) )` token window to `} ) ( )`, which is right
//! only when the outer `)` closes a grouping paren around the function:
//! `(function(){}())` → `(function(){})()`. In `f(function(){return 1}())`
//! the outer `)` is `f`'s, so closurec printed `f(function(){return 1})()`,
//! which calls `f` with the function and then calls `f`'s result.
//! `console.log(function(){return 1}())` printed the function and then threw
//! `TypeError`.
//!
//! The rewrite now checks that the outer `(` is a grouping paren that holds
//! exactly the function expression.
//!
//! `CASES` expected strings were captured from the pinned
//! `closure-compiler-v20260915` jar under Java 21.0.10, using
//! `--compilation_level WHITESPACE_ONLY`. `KEPT` lists the two shapes where
//! closurec keeps a grouping paren that upstream drops; both are equivalent
//! to their source. When `node` is on `PATH`, `RUNTIME` programs must print
//! what their source prints.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// `(source, upstream WHITESPACE_ONLY output)`.
const CASES: &[(&str, &str)] = &[
    ("(function(){return 42;}());", "(function(){return 42})();"),
    ("f(function(){return 1}());", "f(function(){return 1}());"),
    (
        "console.log(function(){return 1}());",
        "console.log(function(){return 1}());",
    ),
    (
        "f(a,function(){return 1}());",
        "f(a,function(){return 1}());",
    ),
    (
        "x=(a||function(){return 1}());",
        "x=a||function(){return 1}();",
    ),
    (
        "if(function(){return 1}())g();",
        "if(function(){return 1}())g();",
    ),
    (
        "new(function(){return 1}());",
        "new (function(){return 1}());",
    ),
    (
        "a?.(function(){return 1}());",
        "a?.(function(){return 1}());",
    ),
    ("x=[function(){return 1}()];", "x=[function(){return 1}()];"),
    (
        "x=(function f(){return 1}());",
        "x=function f(){return 1}();",
    ),
    ("x=(async function(){}());", "x=async function(){}();"),
    ("x=(function*(){}());", "x=function*(){}();"),
    (
        "while(function(){return 0}());",
        "while(function(){return 0}());",
    ),
    (
        "x=(function(){return 1}()),y=2;",
        "x=function(){return 1}(),y=2;",
    ),
    ("(function(){}()),f();", "(function(){})(),f();"),
    ("!function(){}();", "!function(){}();"),
    (
        "x={a:function(){return 1}()};",
        "x={a:function(){return 1}()};",
    ),
    ("f(function(){}(),1);", "f(function(){}(),1);"),
    (
        "g(x)(function(){return 1}());",
        "g(x)(function(){return 1}());",
    ),
    ("f(\"}\"());", "f(\"}\"());"),
];

/// `(source, closurec WHITESPACE_ONLY output)`. Upstream prints
/// `var x=function(){return 1}();` and `f(function(){return 1}());`.
const KEPT: &[(&str, &str)] = &[
    (
        "var x=(function(){return 1;}());",
        "var x=(function(){return 1})();",
    ),
    (
        "f((function(){return 1}()));",
        "f((function(){return 1})());",
    ),
];

/// Programs whose output shows what the IIFE handed on.
const RUNTIME: &[&str] = &[
    "console.log(function(){return 1}());",
    "function f(v){return typeof v}console.log(f(function(){return 1}()));",
    "function f(v){return function(){return 'called '+typeof v}}console.log(f(function(){return 1}()));",
    "console.log((function(){return 2}()));",
    "var x=(function(){return 3}());console.log(x);",
    "(function(){console.log(4)}());",
    "var a=0;console.log(a||function(){return 5}());",
];

fn compile_ws(source: &str) -> String {
    // Tests run in parallel in one process: a counter keeps scratch dirs apart.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-ws-iife-{}-{n}-{}",
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
        .args(["--compilation_level", "WHITESPACE_ONLY", "--js"])
        .arg(&path)
        .output()
        .expect("run closurec");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{source:?}: exit {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout)
        .expect("UTF-8 output")
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
fn whitespace_only_iifes_match_upstream() {
    for (source, expected) in CASES {
        assert_eq!(&compile_ws(source), expected, "source: {source:?}");
    }
}

#[test]
fn whitespace_only_iifes_closurec_keeps_parens() {
    for (source, expected) in KEPT {
        assert_eq!(&compile_ws(source), expected, "source: {source:?}");
    }
}

#[test]
fn whitespace_only_iife_output_prints_what_the_source_prints() {
    for source in RUNTIME {
        let Some(source_says) = node_stdout(source) else {
            eprintln!("node not on PATH; skipping the runtime comparison");
            return;
        };
        let compiled = compile_ws(source);
        let compiled_says = node_stdout(&compiled).expect("node ran a moment ago");
        assert_eq!(compiled_says, source_says, "{source:?} -> {compiled:?}");
    }
}
