//! Regression test for #17940: function and class declarations in class
//! static blocks (`static { ... }`).
//!
//! ## The miscompile / parse error
//!
//! In ES2022–ES2025 grammars, `static_block` accepted only `{ statement }`.
//! When #17877 fixed `block`, `case_clause`, and `default_clause` to accept
//! declarations (`function_declaration`, `generator_declaration`,
//! `async_function_declaration`, `async_generator_declaration`,
//! `class_declaration`), `static_block` was missed.
//!
//! Consequently:
//!
//! ```text
//! class A { static { var s = 'ab'; function f() { return 1; }
//! /b/.test(s) && console.log('static block', f()); } }
//! ```
//!
//! failed at the parse stage because `function f() ...` was treated as an
//! expression statement needing statement termination, or when followed without
//! semicolons failed parsing entirely. In the typed AST bridge, `convert_static_block`
//! mapped only through `convert_statement`, which declined declarations.
//!
//! ## The fix
//!
//! In `es2022.grammar`, `es2023.grammar`, `es2024.grammar`, and `es2025.grammar`,
//! `static_block` takes the declaration alternatives in addition to `statement`.
//! `convert_static_block` in `bridge.rs` lowers each item via
//! `convert_statement_list_item`, matching `convert_block_statement`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

const CASES: &[&str] = &[
    // The exact repro from #17940:
    "class A { static { var s = 'ab'; function f() { return 1; }\n/b/.test(s) && console.log('static block', f()); } }",
    // Without newline before the next statement:
    "class A { static { function f() { return 42; } console.log(f()); } }",
    // Class declaration inside static block:
    "class A { static { class Inner { static val = 99; } console.log(Inner.val); } }",
    // Generator declaration inside static block:
    "class A { static { function* gen() { yield 1; yield 2; } console.log([...gen()].join(',')); } }",
    // Multiple static blocks with separate scoped declarations:
    "class A { static { function getVal() { return 10; } console.log(getVal()); } static { function getVal() { return 20; } console.log(getVal()); } }",
];

fn scratch_dir() -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-static-block-decl-{}-{n}-{}",
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
fn static_block_declarations_compile_at_simple_and_advanced() {
    for source in CASES {
        for level in ["SIMPLE", "ADVANCED"] {
            let out = compile(source, level);
            // Declarations inside static blocks should not be emitted as parenthesized expressions.
            assert!(
                !out.contains("(function f") && !out.contains("(class Inner"),
                "{level} {source:?} emitted a declaration as an expression: {out}"
            );
        }
    }
}

#[test]
fn static_block_declarations_print_what_the_source_prints() {
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
