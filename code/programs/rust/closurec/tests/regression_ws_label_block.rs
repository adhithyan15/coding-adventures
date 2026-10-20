//! CCR-087 (#15924): WHITESPACE_ONLY unwraps a labeled block that holds one
//! statement, as upstream does.
//!
//! Upstream prints a label's body through `addNonEmptyStatement`, which
//! drops the braces of a single-statement block and prints an empty one as
//! `;`. A block holding a lone `let`/`const`/`class`/`function` keeps its
//! braces, because the block is what scopes that declaration. closurec used
//! to unwrap only a body that started with `break`/`continue`/`return`/
//! `throw` (gap-067).
//!
//! Every `expected` string below was captured from the pinned
//! `closure-compiler-v20260915` jar (sha256 verified) under Java 21.0.10,
//! using `--compilation_level WHITESPACE_ONLY`. When `node` is on `PATH`,
//! the test also checks that each output parses.
//!
//! `KEPT` lists shapes upstream still unwraps but closurec deliberately
//! leaves braced: bodies with a nested `{`, and stacked labels. Whether a
//! `{` there is an object or a block can't be told from tokens alone, so
//! closurec keeps the braces. Their output is valid, only larger.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// `(source, upstream WHITESPACE_ONLY output)`.
const CASES: &[(&str, &str)] = &[
    // The four rows of #15924.
    ("lbl:{a()}", "lbl:a();"),
    ("lbl:{}", "lbl:;"),
    ("lbl:{a();b()}", "lbl:{a();b()};"),
    ("{a()}", "{a()};"),
    // More single statements.
    ("lbl:{a();}", "lbl:a();"),
    ("lbl:{;}", "lbl:;"),
    ("lbl:{var x=1}", "lbl:var x=1;"),
    ("lbl:{x=1}", "lbl:x=1;"),
    ("lbl:{if(a())break lbl}", "lbl:if(a())break lbl;"),
    ("lbl:{for(;;)break lbl}", "lbl:for(;;)break lbl;"),
    ("lbl:{while(a)b()}", "lbl:while(a)b();"),
    ("lbl:{for(var i=0;i<3;i++)a(i)}", "lbl:for(var i=0;i<3;i++)a(i);"),
    ("lbl:{debugger}", "lbl:debugger;"),
    ("lbl:{`t`}", "lbl:`t`;"),
    ("lbl:{\"use strict\"}", "lbl:\"use strict\";"),
    ("lbl:{new A}", "lbl:new A;"),
    ("lbl:{++i}", "lbl:++i;"),
    // Statement boundaries: after `;`, after a block, at a body's start.
    ("x;lbl:{a()}", "x;lbl:a();"),
    ("if(x){}lbl:{a()}", "if(x);lbl:a();"),
    ("function f(){lbl:{a()}}", "function f(){lbl:a()};"),
    ("function f(){lbl:{return 1}}", "function f(){lbl:return 1};"),
    ("(function(){lbl:{a()}})()", "(function(){lbl:a()})();"),
    ("if(a){y:{b()}}", "if(a)y:b();"),
    // An empty body's `;` survives before an enclosing `}`: a label needs a
    // statement (CCR-087 review; `l:}` would not parse).
    ("function f(){l:{}}", "function f(){l:;};"),
    ("function f(){l:{;}}", "function f(){l:;};"),
    ("function f(){x();l:{}}", "function f(){x();l:;};"),
    ("if(a){x();l:{}}", "if(a){x();l:;};"),
    ("{x;l:{}}", "{x;l:;};"),
    ("try{}catch(e){l:{}}", "try{}catch(e){l:;};"),
    ("switch(x){case 1:x;l:{}}", "switch(x){case 1:x;l:;};"),
    ("x={m(){l:{}}}", "x={m(){l:;}};"),
    ("class A{m(){l:{}}}", "class A{m(){l:;}};"),
    ("class A{static{x;l:{}}}", "class A{static{x;l:;}};"),
    ("x=function(){l:{}}", "x=function(){l:;};"),
    ("do{x();l:{}}while(a)", "do{x();l:;}while(a);"),
    ("if(a)b();else{x;l:{}}", "if(a)b();else{x;l:;};"),
    ("function f(){l:{}m:{}}", "function f(){l:;m:;};"),
    ("if(a){l:{}}", "if(a)l:;"),
    // An ordinary `;` before `}` is still dropped.
    ("switch(x){case 1:;}", "switch(x){case 1:};"),
    // A lone declaration keeps its block.
    ("lbl:{let x=1}", "lbl:{let x=1};"),
    ("lbl:{const x=1}", "lbl:{const x=1};"),
    ("lbl:{function g(){}}", "lbl:{function g(){}};"),
    ("lbl:{class C{}}", "lbl:{class C{}};"),
    ("lbl:{async function f(){}}", "lbl:{async function f(){}};"),
    // Two statements stay braced.
    ("lbl:{a()\nb()}", "lbl:{a();b()};"),
    ("lbl:{if(a){b()}c()}", "lbl:{if(a)b();c()};"),
    // Objects are never label bodies.
    ("x={y:{a}}", "x={y:{a}};"),
    ("x={y:{a:1}}", "x={y:{a:1}};"),
    ("f({y:{a}})", "f({y:{a}});"),
];

/// Shapes closurec keeps braced although upstream unwraps them. The output
/// must still parse.
const KEPT: &[&str] = &[
    "lbl:{a=function(){}}",
    "lbl:{a={}}",
    "lbl:{{a()}}",
    "a:b:{c()}",
    "lbl:{(function(){})()}",
    "lbl:{if(a)b();else c()}",
    "if(a){y:{if(b)c()}}else d()",
];

fn compile_ws(source: &str) -> String {
    // Tests run in parallel in one process: a counter keeps scratch dirs apart.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-ws-label-{}-{n}-{}",
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
fn whitespace_only_label_blocks_match_upstream() {
    for (source, expected) in CASES {
        assert_eq!(&compile_ws(source), expected, "source: {source:?}");
    }
}

#[test]
fn nested_labels_are_scanned_once() {
    // `l:{x;` × 8000: each label used to rescan to its closing brace,
    // O(depth × size). Brace facts are now computed in one pass.
    let source = format!("{}{}", "l:{x;".repeat(8000), "}".repeat(8000));
    let start = std::time::Instant::now();
    let out = compile_ws(&source);
    assert!(out.starts_with("l:{x;l:{x;"));
    assert!(
        start.elapsed() < std::time::Duration::from_secs(20),
        "took {:?}",
        start.elapsed()
    );
}

#[test]
fn whitespace_only_label_block_output_parses() {
    let all = CASES.iter().map(|(s, _)| *s).chain(KEPT.iter().copied());
    for source in all {
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
