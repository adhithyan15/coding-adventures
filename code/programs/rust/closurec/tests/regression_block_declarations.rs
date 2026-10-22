//! Regression test for #17877: a `function` or `class` declaration inside a
//! block stays a declaration.
//!
//! ## The miscompile
//!
//! The ECMAScript grammars gave `block`, `case_clause` and `default_clause`
//! only `{ statement }`. The declaration alternatives lived in
//! `source_element`, which only `program` and `function_body` use. Inside a
//! block, `function g(){}` therefore matched `expression_statement` as a
//! function *expression*. The output wrapped it in parentheses, so it bound
//! nothing:
//!
//! ```text
//! if(true){function g(){return 7}⏎console.log(g())}
//!   -> (function g(){return 7});console.log(g());   ReferenceError: g is not defined
//! ```
//!
//! Without a newline (`{function g(){} g()}`), the program failed to parse
//! at all.
//!
//! ## The fix
//!
//! In ES2015–ES2025 the three rules take an inline group: the same
//! declarations `source_element` lists, minus import/export, plus
//! `statement`. The bridge lowers a declaration child to
//! `Statement::Declaration`, the node a function body already holds.
//!
//! ## What this test checks
//!
//! Every case is compiled at SIMPLE and ADVANCED. It must succeed, and when
//! `node` is on `PATH`, the output must print exactly what the source
//! prints. These cases do not depend on sloppy-mode Annex B hoisting, so the
//! source's output is also upstream's (`closure-compiler-v20260915`, checked
//! with Java 21.0.10). Upstream's *bytes* are smaller, because it also
//! inlines or unwraps the function; that is an optimization gap, not this
//! bug.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

const CASES: &[&str] = &[
    "{class C{}\nconsole.log(new C().constructor.name)}",
    "if(true){function g(){return 7}\nconsole.log(g())}",
    // No newline: these used to fail at the parse stage.
    "if(true){function g(){return 7} console.log(g())}",
    "{class C{} console.log(new C().constructor.name)}",
    "{function* gen(){yield 1} console.log([...gen()].length)}",
    // Case and default clauses take the same items.
    "switch(1){case 1:function h(){return 3} console.log(h())}",
    "switch(1){default:class K{} console.log(typeof K)}",
    // Loop bodies, nested functions, and closures over outer state.
    "for(var i=0;i<1;i++){function k(){return i} console.log(k())}",
    "function f(){if(true){function g(){return 2}return g()}} console.log(f())",
    "function o(){var n=0;{function inc(){n++} inc(); inc()} return n} console.log(o())",
    // Two sibling blocks declaring the same name stay separate.
    "function f(){ {function g(){return 1} var a=g()} {function g(){return 2} var b=g()} return a+b } console.log(f())",
    // A block-scoped class and an outer `let` of the same name do not collide.
    "function f(){ let r=[]; {function g(){return 1} r.push(g())} {class g{} r.push(typeof g)} return r.join() } console.log(f())",
];

fn scratch_dir() -> std::path::PathBuf {
    // Tests run in parallel in one process: a counter keeps scratch dirs apart.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-block-decl-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir(&dir).expect("fresh scratch dir");
    dir
}

fn compile(source: &str, level: &str) -> String {
    let dir = scratch_dir();
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
        "{level} {source:?}: exit {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout).expect("UTF-8 output")
}

/// Run `program` under node and return its stdout, or `None` without node.
fn run_node(program: &str) -> Option<(bool, String, String)> {
    let run = Command::new("node").args(["-e", program]).output().ok()?;
    Some((
        run.status.success(),
        String::from_utf8_lossy(&run.stdout).into_owned(),
        String::from_utf8_lossy(&run.stderr).into_owned(),
    ))
}

#[test]
fn block_declarations_compile_at_simple_and_advanced() {
    for source in CASES {
        for level in ["SIMPLE", "ADVANCED"] {
            let out = compile(source, level);
            // A declaration is never re-emitted as a parenthesised expression.
            assert!(
                !out.contains("(function g") && !out.contains("(class C"),
                "{level} {source:?} emitted a declaration as an expression: {out}"
            );
        }
    }
}

#[test]
fn block_declarations_print_what_the_source_prints() {
    for source in CASES {
        let Some((ok, expected, err)) = run_node(source) else {
            eprintln!("node not on PATH; skipping the runtime check");
            return;
        };
        assert!(ok, "source {source:?} must run: {err}");
        for level in ["SIMPLE", "ADVANCED"] {
            let out = compile(source, level);
            let (ok, got, err) = run_node(&out).expect("node ran once already");
            assert!(ok, "{level} {source:?} -> {out:?} throws: {err}");
            assert_eq!(got, expected, "{level} {source:?} -> {out:?}");
        }
    }
}
