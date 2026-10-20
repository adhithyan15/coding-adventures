//! Regression test for #17840 and #15867 (CCR-080): WHITESPACE_ONLY keeps the
//! statement boundaries that newlines provided.
//!
//! ## The miscompile
//!
//! The token path drops every newline with the rest of the trivia. Nothing
//! put the statement boundary back, so `a()⏎b()` became `a()b()`, which is a
//! SyntaxError. Any semicolon-light program compiled to invalid output.
//!
//! ## The fix
//!
//! `javascript_parser::asi::asi_insertion_points` runs the parser's ASI and
//! reports where it inserts a `;`. That covers:
//! - the line-terminator rule;
//! - `}` and end of input;
//! - the restricted productions `return`/`throw`/`break`/`continue`/`yield`
//!   and postfix `++`/`--`.
//!
//! The WHITESPACE_ONLY path splices a `;` in at each point. The end-of-input
//! point is also the program-final terminator CCR-080 asked for after a
//! `do…while`.
//!
//! ## Expected bytes
//!
//! Every `expected` string below was captured from the pinned
//! `closure-compiler-v20260915` jar (sha256 verified) under Java 21.0.10, using
//! `--compilation_level WHITESPACE_ONLY`. These are upstream's bytes, kept here
//! instead of under `tests/diff/`, which would also need oracle-manifest
//! entries. When `node` is on `PATH`, the test also checks that each output
//! parses.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// `(source, upstream WHITESPACE_ONLY output)`.
const CASES: &[(&str, &str)] = &[
    ("a()\nb()", "a();b();"),
    (
        "var a = function(){ console.log(1) }\nvar b = 2\nconsole.log(b)\nb++\nconsole.log(b)",
        "var a=function(){console.log(1)};var b=2;console.log(b);b++;console.log(b);",
    ),
    ("throw 1\nb()", "throw 1;b();"),
    ("function f(){return\na}", "function f(){return;a};"),
    ("x = 1\n++y", "x=1;++y;"),
    ("a()\n--b", "a();--b;"),
    ("x =\n++y", "x=++y;"),
    ("let a = 1\nlet b = 2", "let a=1;let b=2;"),
    ("if (a) b()\nelse c()", "if(a)b();else c();"),
    ("do a()\nwhile (b)", "do a();while(b);"),
    ("for (;;) {\nbreak\n}", "for(;;)break;"),
    ("var s = \"x\"\n;(function(){})()", "var s=\"x\";(function(){})();"),
    ("a = b\n(c)", "a=b(c);"),
    ("const T = 1\nif (T) a()", "const T=1;if(T)a();"),
    ("class A {}\nnew A()", "class A{}new A;"),
    ("var a = [1,2]\n[0].forEach(f)", "var a=[1,2][0].forEach(f);"),
    ("switch(a){case 1:b()\ncase 2:c()}", "switch(a){case 1:b();case 2:c()};"),
    ("try{a()}catch(e){b()}\nfinally{c()}", "try{a()}catch(e){b()}finally{c()};"),
    ("class P{m(){a()}\nn(){b()}}", "class P{m(){a()}n(){b()}};"),
    ("with(o){a()}", "with(o)a();"),
    ("if(a){if(b)c()}else d()", "if(a){if(b)c()}else d();"),
    // A statement header's `)` does not end an expression: the newline-led
    // `++`/`--` is the body (security review of #17840).
    ("if (x)\n++y", "if(x)++y;"),
    ("while (x)\n--y", "while(x)--y;"),
    ("for (;;)\n++i", "for(;;)++i;"),
    ("with (o)\n++a", "with(o)++a;"),
    ("if (a) b(); else if (c)\n++d", "if(a)b();else if(c)++d;"),
    ("class A{m(){}\nstatic x=1\nn(){}}", "class A{m(){}static x=1;n(){}};"),
    ("class A{static {a()}\nm(){}}", "class A{static{a()}m(){}};"),
    // `for await (…)` is a header too (review round 2).
    (
        "async function f(y){for await (const x of y)\ng(x)}",
        "async function f(y){for await(const x of y)g(x)};",
    ),
    ("x = {}\ny()", "x={};y();"),
    ("class A{*g(){}\nh(){}}", "class A{*g(){}h(){}};"),
    ("class A{async m(){}\nn(){}}", "class A{async m(){}n(){}};"),
    // CCR-080 (#15867): the program-final `;` after a `do…while`.
    ("do{console.log(1)}while(0)", "do console.log(1);while(0);"),
    ("x=1;do{a()}while(b)", "x=1;do a();while(b);"),
];

/// Inputs whose output must parse but whose bytes still differ from upstream
/// in class-body punctuation. Upstream keeps a field's final `;` before `}`,
/// which is CCR-077's scope (#15862). Before #17840 these glued fields into
/// invalid output (`class A{x=1 y=2}`).
const PARSE_ONLY: &[&str] = &[
    "class A{x=1\ny=2}",
    "class A{x\ny}",
    "class A{x = function(){}\ny = 1}",
    "class A{x=function*g(){}\ny=2}",
];

fn compile_ws(source: &str, tag: usize) -> String {
    // Tests run in parallel in one process: a counter keeps scratch dirs apart.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-ws-asi-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir(&dir).expect("fresh scratch dir");
    let path = dir.join(format!("case{tag}.js"));
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
fn whitespace_only_matches_upstream_on_semicolon_light_input() {
    for (i, (source, expected)) in CASES.iter().enumerate() {
        assert_eq!(&compile_ws(source, i), expected, "source: {source:?}");
    }
}

#[test]
fn whitespace_only_output_parses() {
    let all = CASES.iter().map(|(s, _)| *s).chain(PARSE_ONLY.iter().copied());
    for (i, source) in all.enumerate() {
        let out = compile_ws(source, i);
        // `new Function` parses without running; it is only reachable when
        // node is installed.
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
