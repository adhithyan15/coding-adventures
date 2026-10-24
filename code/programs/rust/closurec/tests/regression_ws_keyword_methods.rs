//! Regression test for #18123: WHITESPACE_ONLY must not apply statement
//! body-slot rules to a *method* named `if`, `while`, `for` or `with`.
//!
//! `class C{if(){}}` printed as `class C{if();}` and `x={while(a){c=a;}}` as
//! `x={while(a)c=a}`. Neither parses. A keyword directly inside a class body
//! or object literal is a member name (`member_list_positions`), so it no
//! longer arms a control-flow head or the header-body flatten.
//!
//! Every `expected` string was captured from the pinned
//! `closure-compiler-v20260915` jar under Java 21.0.10 with
//! `--compilation_level WHITESPACE_ONLY`. The list also holds real statements
//! inside blocks, labels, `case` clauses, arrow and method bodies, which must
//! still flatten. When `node` is on `PATH`, each output must also parse.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// `(source, upstream WHITESPACE_ONLY output)`.
const CASES: &[(&str, &str)] = &[
    ("class C{with(a){return a}}", "class C{with(a){return a}};"),
    ("x={while(a){c=a;}}", "x={while(a){c=a}};"),
    ("class C{if(){}}", "class C{if(){}};"),
    ("x={get if(){}}", "x={get if(){}};"),
    ("class C{static for(){}}", "class C{static for(){}};"),
    ("x={a:1,if(b){c()}}", "x={a:1,if(b){c()}};"),
    ("class C{*while(){yield 1}}", "class C{*while(){yield 1}};"),
    ("x=class{if(){}}", "x=class{if(){}};"),
    ("class C{m(){}if(){}}", "class C{m(){}if(){}};"),
    ("x={a:{if(b){c()}}}", "x={a:{if(b){c()}}};"),
    (
        "class A extends f({}){if(){}}",
        "class A extends f({}){if(){}};",
    ),
    ("if(a){b()}", "if(a)b();"),
    ("function f(){if(a){b()}}", "function f(){if(a)b()};"),
    (
        "switch(x){case 1:{if(a){b()}}}",
        "switch(x){case 1:{if(a)b()}};",
    ),
    ("x=()=>{if(a){b()}}", "x=()=>{if(a)b()};"),
    ("{if(a){b()}}", "{if(a)b()};"),
    ("x={m(){if(a){b()}}}", "x={m(){if(a)b()}};"),
    ("class C{m(){while(a){b()}}}", "class C{m(){while(a)b()}};"),
    ("if(a);", "if(a);"),
    ("x={m(){if(a);}}", "x={m(){if(a);}};"),
    ("a?b:{}", "a?b:{};"),
];

fn compile_ws(source: &str) -> String {
    // Tests run in parallel in one process: a counter keeps scratch dirs apart.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-ws-keyword-methods-{}-{n}-{}",
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
fn whitespace_only_keyword_methods_match_upstream() {
    for (source, expected) in CASES {
        assert_eq!(&compile_ws(source), expected, "source: {source:?}");
    }
}

#[test]
fn whitespace_only_keyword_method_output_parses() {
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
