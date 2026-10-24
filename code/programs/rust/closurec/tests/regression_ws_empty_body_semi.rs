//! Regression test for #17883: WHITESPACE_ONLY keeps the `;` that is the
//! empty-statement body of a `with(...)` or a label before `}`.
//!
//! gap-030 Rule A drops a `;` directly before `}`. It already kept the `;`
//! after an `if(...)`/`for(...)`/`while(...)` header, but not after
//! `with(...)` or a label's `:`, so `function f(){with(a);}` became
//! `function f(){with(a)}` and `{l2:;}` became `{l2:}`. Neither parses.
//!
//! The same change stops a keyword used as a property name (`{if:1}`) from
//! arming the next `(`. That used to turn `x={if:1};function g(){}` into the
//! invalid `x={if:1};function g();`.
//!
//! Every `expected` string below was captured from the pinned
//! `closure-compiler-v20260915` jar under Java 21.0.10, using
//! `--compilation_level WHITESPACE_ONLY`. When `node` is on `PATH`, the test
//! also checks that each output parses.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// `(source, upstream WHITESPACE_ONLY output)`.
const CASES: &[(&str, &str)] = &[
    ("function f(){with(a);}", "function f(){with(a);};"),
    ("{l2:;}", "{l2:;};"),
    ("function f(){l:{with(a);}}", "function f(){l:with(a);};"),
    ("switch(x){case 1:with(a);}", "switch(x){case 1:with(a);};"),
    ("with(o){}", "with(o);"),
    ("with(o){a()}", "with(o)a();"),
    ("function f(){with(o){a()}}", "function f(){with(o)a()};"),
    ("with(o)with(p);", "with(o)with(p);"),
    (
        "function f(){if(a)with(b);}",
        "function f(){if(a)with(b);};",
    ),
    ("do with(a);while(b)", "do with(a);while(b);"),
    ("o.with(a);if(c){d;}", "o.with(a);if(c)d;"),
    ("function f(){l:;}", "function f(){l:;};"),
    ("function f(){a:b:;}", "function f(){a:b:;};"),
    ("switch(x){case 1:l:;}", "switch(x){case 1:l:;};"),
    ("function f(){if(a)l:;}", "function f(){if(a)l:;};"),
    ("if(a){l:;}", "if(a)l:;"),
    (
        "function f(){if(a){}else l:;}",
        "function f(){if(a);else l:;};",
    ),
    ("switch(x){case 1:;}", "switch(x){case 1:};"),
    ("switch(x){case a.b:;}", "switch(x){case a.b:};"),
    ("switch(x){case f():;}", "switch(x){case f():};"),
    ("switch(x){default:;}", "switch(x){default:};"),
    ("x={if:1};function g(){}", "x={if:1};function g(){};"),
    ("x={with:1};function g(){}", "x={with:1};function g(){};"),
    (
        "x={while:1};function g(){a;}",
        "x={while:1};function g(){a};",
    ),
    (
        "var o={with:1},p=function(){};",
        "var o={with:1},p=function(){};",
    ),
    (
        "async function f(){for await(x of y){}}",
        "async function f(){for await(x of y);};",
    ),
];

fn compile_ws(source: &str) -> String {
    // Tests run in parallel in one process: a counter keeps scratch dirs apart.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-ws-empty-body-{}-{n}-{}",
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

#[test]
fn whitespace_only_empty_bodies_match_upstream() {
    for (source, expected) in CASES {
        assert_eq!(&compile_ws(source), expected, "source: {source:?}");
    }
}

#[test]
fn whitespace_only_empty_body_output_parses() {
    for (source, _) in CASES {
        let out = compile_ws(source);
        // `new Function` parses without running; skipped without node.
        let Ok(run) = Command::new("node")
            .args(["-e", "new Function(process.argv[1])", &out])
            .output()
        else {
            eprintln!("node not on PATH; skipping the parse check");
            return;
        };
        assert!(
            run.status.success(),
            "{source:?} -> {out:?} does not parse: {}",
            String::from_utf8_lossy(&run.stderr),
        );
    }
}
