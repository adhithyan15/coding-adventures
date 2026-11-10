//! Regression test for #17939: closurec WHITESPACE_ONLY drops the
//! restricted-production newline after yield.
//!
//! ## The miscompile / bug
//!
//! ECMA-262 §15.4 defines YieldExpression as:
//!
//! ```text
//! YieldExpression[In, Await] :
//!     yield
//!     yield [no LineTerminator here] AssignmentExpression[?In, +Yield, ?Await]
//!     yield [no LineTerminator here] * AssignmentExpression[?In, +Yield, ?Await]
//! ```
//!
//! `yield` is a restricted production: a line break immediately following `yield`
//! prevents parsing an assignment expression on the next line and forces statement
//! termination (yielding `undefined`).
//!
//! In:
//!
//! ```javascript
//! function* G(){ yield
//! { x; }
//! }
//! ```
//!
//! The source yields `undefined` and then executes block `{ x; }`.
//! Previously, the ECMAScript grammars (`es2015.grammar` .. `es2025.grammar`)
//! required an `assignment_expression` operand for `yield_expression`, making bare
//! `yield;` or `yield\n` a parse error. Consequently, ASI insertion points failed,
//! and WHITESPACE_ONLY dropped the restricted-production boundary or fell back to
//! keeping an un-minified newline.
//!
//! ## The fix
//!
//! 1. `yield_expression` in `es2015.grammar` through `es2025.grammar` accepts
//!    bare `yield [ assignment_expression ]`.
//! 2. The typed AST bridge in `javascript-parser/src/bridge.rs` produces
//!    `YieldExpression { argument: None, delegate: false }` when no operand is present.
//! 3. ASI insertion points correctly identify and insert semicolons after bare `yield`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

const CASES: &[&str] = &[
    // The exact repro from #17939:
    "function* G(){ yield\n{ x; }\n}",
    // Bare yield in a generator function:
    "function* gen(){ yield; }",
    // Multiple bare yields separated by newlines:
    "function* gen(){ yield\nyield\nreturn 1; }",
    // Bare yield followed by non-block statement on new line:
    "function* gen(){ yield\nvar x = 1; }",
];

fn scratch_dir() -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-yield-bare-{}-{n}-{}",
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
fn yield_bare_compiles_at_all_compilation_levels() {
    for source in CASES {
        for level in ["WHITESPACE_ONLY", "SIMPLE", "ADVANCED"] {
            let out = compile(source, level);
            assert!(
                !out.is_empty(),
                "{level} {source:?} output should not be empty"
            );
        }
    }
}

#[test]
fn yield_repro_17939_preserves_restricted_production_semantics() {
    let source = "function* G(){ var x = 42; yield\n{ x; } } console.log(G);";
    let node_check = "var g = G(); var first = g.next(); var second = g.next(); console.log(JSON.stringify({v1: first.value, d1: first.done, v2: second.value, d2: second.done}));";

    for level in ["WHITESPACE_ONLY", "SIMPLE", "ADVANCED"] {
        let compiled = compile(source, level);

        // Crucial check: in the output, `yield` must NOT have fused into `yield{x}` (yielding an object).
        // It must have a semicolon after yield: `yield;`
        assert!(
            compiled.contains("yield;"),
            "{level} output should end bare yield with a semicolon, got: {compiled}"
        );

        if let Some((src_ok, src_out, _)) = run_node(&format!("{source}\n{node_check}")) {
            if src_ok {
                let (comp_ok, comp_out, err) =
                    run_node(&format!("{compiled}\n{node_check}")).unwrap();
                assert!(
                    comp_ok,
                    "{level} compiled code failed to run in node: {err}\nCode was:\n{compiled}"
                );
                assert_eq!(
                    src_out, comp_out,
                    "{level} output produced divergent node execution"
                );
            }
        }
    }
}
