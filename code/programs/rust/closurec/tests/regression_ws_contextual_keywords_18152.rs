//! Regression test for #18152: WHITESPACE_ONLY breaks let/using/await used
//! as identifiers (ASI and paren elision).
//!
//! ## The miscompiles
//!
//! 1. `let⏎new Foo`: `let` at end of line followed by a non-binding keyword
//!    miscompiled to `let new Foo;`, which is an invalid lexical declaration.
//!    In standard ECMAScript, `let` followed by a newline and `new` is an
//!    identifier expression statement `let;`.
//! 2. `let⏎++x`: `let` followed by `++x` on a new line miscompiled to
//!    `let++x;` or `let ++x;`. ECMA-262 §12.4 forbids line terminators before
//!    postfix `++`/`--`, so `++x` across a newline is strictly prefix `++`,
//!    demanding ASI after `let`.
//! 3. `await⏎using⏎x`: inside an async function, `using` followed by a line
//!    terminator cannot be a `using` declaration (ECMA-262 §14.3.1 forbids
//!    line terminators after `using`). It ended the statement as an await
//!    expression `await using;`, but miscompiled to `await using x;` (missing
//!    initializer in await using declaration).
//! 4. `await(x)` in scripts: in non-module scripts outside async functions,
//!    `await` is an ordinary identifier and `await(x)` is a function call.
//!    Paren elision stripped parens to top-level `await x;`, which is a
//!    SyntaxError in scripts.
//!
//! When `node` is on `PATH`, the test checks that each minified output parses.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

const CASES: &[(&str, &str)] = &[
    ("let\nnew Foo", "let;new Foo"),
    ("let\nnew Foo()", "let;new Foo"),
    ("let\nnew Foo();", "let;new Foo;"),
    ("let\n++x", "let;++x"),
    ("let\n++x;", "let;++x;"),
    (
        "async function f(){if(a){await\nusing\nx}}",
        "async function f(){if(a){await using;x}};",
    ),
    ("await(x)", "await(x);"),
    ("function f(){await(x)}", "function f(){await(x)};"),
    ("async function f(){await(x)}", "async function f(){await x};"),
    ("let f=async()=>{await(x)}", "let f=async()=>{await x};"),
    ("x=y.if\nl:;", "x=y.if;l:;"),
    ("obj.let\n(x);", "obj.let(x);"),
    ("stream.let\n.filter(fn);", "stream.let.filter(fn);"),
    (
        "target.element\ninstanceof HTMLInputElement;",
        "target.element instanceof HTMLInputElement;",
    ),
    ("target.element\nin obj;", "target.element in obj;"),
    ("window.export=1;\nawait(x);", "window.export=1;await(x);"),
    (
        "async function f(){class Foo{field=await(x)}}",
        "async function f(){class Foo{field=await(x);}};",
    ),
    ("let f=async()=>fn(a,await(b));", "let f=async()=>fn(a,await b);"),
];

fn compile_ws(source: &str, tag: usize) -> String {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-ws-contextual-{}-{n}-{}",
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
fn whitespace_only_matches_contextual_keywords_expected() {
    for (i, (source, expected)) in CASES.iter().enumerate() {
        let actual = compile_ws(source, i);
        assert_eq!(&actual, expected, "source: {source:?}");
    }
}

#[test]
fn whitespace_only_contextual_keywords_output_parses() {
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
