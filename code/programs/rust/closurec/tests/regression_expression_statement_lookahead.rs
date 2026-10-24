//! Regression test for F12 phase 0: the ExpressionStatement lookahead.
//!
//! ## The miscompile
//!
//! ECMA-262 forbids an ExpressionStatement from starting with `{`,
//! `function`, `class`, `let [` or `async function`. The ES grammars did not
//! enforce it. A block whose last statement had no `;` fails to parse as a
//! Block until ASI runs, and ASI only runs after the whole parse fails. So
//! the parser fell through to `expression_statement` and read the block as an
//! object literal:
//!
//! ```text
//! { a }⏎/foo/g.exec(s)     SIMPLE → ({a})/foo/g.exec(s)     (a different program)
//! if (x) function f(){}    SIMPLE → x&&function f(){}       (Closure refuses it)
//! ```
//!
//! ## What this test checks
//!
//! The first form is now refused rather than miscompiled. (F12 phase 3 will
//! let the parser re-lex the `/` as a regex and compile it.) Closure refuses
//! the second, and so does closurec. Blocks, object literals in parentheses
//! and ordinary statements still compile, and when `node` is on `PATH` they
//! print what the source prints.
//!
//! WHITESPACE_ONLY still emits a program the grammar can't parse. Such a
//! program must keep the restricted-production `;`s (`return⏎42`), which the
//! lookahead's new refusals would otherwise have dropped (security review).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// Programs that must be refused, never compiled to something else.
const REFUSED: &[&str] = &[
    "var s = 'xfoox'; { s }\n/foo/g.exec(s)",
    "var a = 1; { a } / 2;",
    "var x = 1; if (x) function f(){}",
    "l: function f(){} f();",
];

/// Programs that must still compile and behave.
const CASES: &[&str] = &[
    "var x = {a: 1}\n{ x }\nconsole.log(x.a)",
    "console.log(({a: 4}).a / 2)",
    "{ var y = 2 }\nconsole.log(y)",
    "lbl: { console.log('in block') }",
    "do { console.log('once') } while (0)",
    "class A { m() { return 3 } } console.log(new A().m())",
    "(function () { console.log('iife') })()",
];

fn run(source: &str, level: &str) -> std::process::Output {
    // Tests run in parallel in one process: a counter keeps scratch dirs apart.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-exprstmt-{}-{n}-{}",
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
    out
}

fn compile(source: &str, level: &str) -> String {
    let out = run(source, level);
    assert!(
        out.status.success(),
        "{level} {source:?}: exit {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout).expect("UTF-8 output")
}

/// Run `program` under node: `(succeeded, stdout, stderr)`, or `None`
/// without node.
fn run_node(program: &str) -> Option<(bool, String, String)> {
    let run = Command::new("node").args(["-e", program]).output().ok()?;
    Some((
        run.status.success(),
        String::from_utf8_lossy(&run.stdout).into_owned(),
        String::from_utf8_lossy(&run.stderr).into_owned(),
    ))
}

/// WHITESPACE_ONLY input that does not parse (the trailing Annex B form is
/// refused) but must keep its restricted-production `;`s.
const UNPARSEABLE_WHITESPACE_ONLY: &[&str] = &[
    "function f(){ return\n42 }\nconsole.log(f()); if (1) function g(){}",
    "var n=0; o: for(var i=0;i<3;i++){ for(;;){ n++; break\no } }\nconsole.log(n); if (1) function g(){}",
    "function* g(){ yield\n7 }\nconsole.log(g().next().value); if (1) function h(){}",
    "function f(){ return\n42 }\nconsole.log(f()); l: function g(){}",
];

#[test]
fn whitespace_only_keeps_restricted_semicolons_when_the_parse_fails() {
    for source in UNPARSEABLE_WHITESPACE_ONLY {
        let out = compile(source, "WHITESPACE_ONLY");
        let Some((ok, expected, err)) = run_node(source) else {
            eprintln!("node not on PATH; skipping the runtime check");
            return;
        };
        assert!(ok, "source {source:?} must run: {err}");
        let (ok, got, err) = run_node(&out).expect("node ran once already");
        assert!(ok, "WHITESPACE_ONLY {source:?} -> {out:?} throws: {err}");
        assert_eq!(got, expected, "WHITESPACE_ONLY {source:?} -> {out:?}");
    }
}

#[test]
fn statements_starting_with_brace_or_function_are_never_expressions() {
    for source in REFUSED {
        for level in ["SIMPLE", "ADVANCED"] {
            let out = run(source, level);
            assert!(
                !out.status.success(),
                "{level} {source:?} must be refused, got {:?}",
                String::from_utf8_lossy(&out.stdout),
            );
        }
    }
    for source in CASES {
        let outputs: Vec<(&str, String)> =
            ["SIMPLE", "ADVANCED"].iter().map(|l| (*l, compile(source, l))).collect();
        let Some((ok, expected, err)) = run_node(source) else {
            eprintln!("node not on PATH; skipping the runtime check");
            continue;
        };
        assert!(ok, "source {source:?} must run: {err}");
        for (level, out) in outputs {
            let (ok, got, err) = run_node(&out).expect("node ran once already");
            assert!(ok, "{level} {source:?} -> {out:?} throws: {err}");
            assert_eq!(got, expected, "{level} {source:?} -> {out:?}");
        }
    }
}
