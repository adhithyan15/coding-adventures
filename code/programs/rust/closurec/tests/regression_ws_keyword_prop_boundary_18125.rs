//! Regression test for #18125: WHITESPACE_ONLY loses the newline boundary after
//! a keyword property name (e.g. `x=y.if\nl:;`).
//!
//! ## The miscompile
//!
//! When a member access property name is spelled like a keyword (such as
//! `x=y.if`, `x=y.while`, `x=y.for`, `x=y.default`), the lexer emits a `Keyword`
//! token for the property identifier. The grammar parser expects `DOT NAME`,
//! which caused `asi_insertion_points` to fail with a parse error and decline
//! ASI points across the program. Consequently, WHITESPACE_ONLY dropped the
//! newline statement boundary, merging `function f(){x=y.if\nl:;}` into
//! `function f(){x=y.if l:};` (SyntaxError).
//!
//! The fix:
//! 1. `javascript-parser/src/asi.rs`: normalizes member access keyword tokens
//!    after `.` or `?.` to `TokenType::Name` during Phase-1 ASI parsing, allowing
//!    the grammar parser to recognize keyword property accesses (§13.3.7).
//! 2. `closurec/src/whitespace_only.rs`: ensures `contextual_asi_points` and
//!    `is_statement_start_token` recognize keyword property terminations and all
//!    valid statement-starting tokens across line breaks.
//!
//! When `node` is on `PATH`, the test verifies that each minified output parses.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

const CASES: &[(&str, &str)] = &[
    (
        "function f(){\nx=y.if\nl:;\n}",
        "function f(){x=y.if;l:;};",
    ),
    (
        "function f(){x=y.if\nl:;}",
        "function f(){x=y.if;l:;};",
    ),
    (
        "x=y.if\nl:;",
        "x=y.if;l:;",
    ),
    (
        "function f(){x=y.while\nreturn 1;}",
        "function f(){x=y.while;return 1};",
    ),
    (
        "x=y.for\nif(a)b();",
        "x=y.for;if(a)b();",
    ),
    (
        "x=y.default\nvar z=1;",
        "x=y.default;var z=1;",
    ),
    (
        "async function f(){x=y.delete\nawait p;}",
        "async function f(){x=y.delete;await p};",
    ),
    (
        "function f(){x=y?.if\nl:;}",
        "function f(){x=y?.if;l:;};",
    ),
    (
        "x=y.class\n!foo();",
        "x=y.class;!foo();",
    ),
    (
        "x=y.catch\n{let b=2;}",
        "x=y.catch;{let b=2};",
    ),
    (
        "x=y.finally\ntrue;",
        "x=y.finally;true;",
    ),
    (
        "x=y.return\n~x;",
        "x=y.return;~x;",
    ),
    (
        "x=y.export\nconst c=3;",
        "x=y.export;const c=3;",
    ),
    (
        "x=y.function\nfunction g(){}",
        "x=y.function;function g(){};",
    ),
    (
        "x=y.switch\nswitch(a){}",
        "x=y.switch;switch(a){};",
    ),
    (
        "x=y.try\ntry{}catch(e){}",
        "x=y.try;try{}catch(e){};",
    ),
    (
        "x=y.throw\nthrow e;",
        "x=y.throw;throw e;",
    ),
    (
        "x=y.debugger\ndebugger;",
        "x=y.debugger;debugger;",
    ),
    (
        "x=y.do\ndo{}while(c);",
        "x=y.do;do; while(c);",
    ),
    (
        "x=y.this\nthis.foo();",
        "x=y.this;this.foo();",
    ),
    (
        "x=y.null\nnull;",
        "x=y.null;null;",
    ),
    (
        "x=y.false\nfalse;",
        "x=y.false;false;",
    ),
    (
        "x=y.void\nvoid 0;",
        "x=y.void;void 0;",
    ),
    (
        "x=y.typeof\ntypeof x;",
        "x=y.typeof;typeof x;",
    ),
    // Property access continuations across newline must not receive semicolons
    (
        "x=y.if\n.filter(fn);",
        "x=y.if.filter(fn);",
    ),
    (
        "x=y.if\n(arg);",
        "x=y.if(arg);",
    ),
    (
        "x=y.if\n[key];",
        "x=y.if[key];",
    ),
];

fn compile_ws(source: &str, tag: usize) -> String {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-ws-prop-boundary-{}-{n}-{}",
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
fn whitespace_only_matches_keyword_prop_boundary_expected() {
    for (i, (source, expected)) in CASES.iter().enumerate() {
        let actual = compile_ws(source, i);
        assert_eq!(&actual, expected, "source: {source:?}");
    }
}

#[test]
fn whitespace_only_keyword_prop_boundary_output_parses() {
    for (i, (source, _)) in CASES.iter().enumerate() {
        let out = compile_ws(source, i);
        // `new Function` parses without running script code; skips execution.
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
