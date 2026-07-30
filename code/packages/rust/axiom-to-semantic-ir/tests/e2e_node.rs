//! End-to-end round-trip: Axiom → SIR → JavaScript → `node`.
//!
//! Mirrors `maple-to-semantic-ir`'s (and `reduce-to-semantic-ir`'s /
//! `derive-to-semantic-ir`'s) own `tests/e2e_node.rs` harnesses: lower an
//! Axiom program to SIR with this crate, validate the module, emit
//! JavaScript with the merged `semantic-ir-to-javascript` backend (a
//! dev-dependency), write it to a temp file, and **execute it with `node`**.
//!
//! # Why these tests check "runs cleanly", not a printed value
//!
//! See `wolfram-to-semantic-ir/tests/e2e_node.rs`'s own module doc comment
//! for the full explanation — the same "everything is symbolic data" design
//! applies here (`src/lower.rs`'s module doc comment): an Axiom program never
//! produces a host-language computed value through this pipeline, so there
//! is no `axiom-repl`-equivalent stdout to assert on. These tests prove
//! instead that every one of these programs compiles to JavaScript that
//! `node` actually executes without throwing.
//!
//! # `__axiom_declare`/`__axiom_coerce`/`__axiom_has` are now REALLY
//! evaluated, not just proven runnable
//!
//! UPDATE (Wave 7 close-out): `semantic-ir-to-javascript`'s shared JS
//! backend now has a real evaluation handler for all three reserved heads
//! (`axiomDeclareHandler`/`axiomCoerceHandler`/`axiomHasHandler`, a JS-side
//! port of `axiom-runtime::domains`'s fixed `AxiomDomain`/`AxiomCategory`
//! table — see that crate's `src/runtime.rs` for the full design). The
//! three tests below therefore now assert on the printed VALUE, not just
//! "node exited zero" — the genuine end-to-end oracle diff against
//! `axiom-runtime` itself lives in `tests/oracle.rs`, which additionally
//! covers the FAILING half of `:`/`::` (an invalid domain name, a
//! subdomain-predicate mismatch) that this file's own "runs cleanly, no
//! crash" framing does not attempt.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use coding_adventures_axiom_to_semantic_ir::compile_source;
use semantic_ir::{EffectSet, Expr, Module, Stmt};

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Compile `src`, validate, emit JS, write to a temp file, and run it under
/// `node`, asserting a clean exit.
fn run_via_node(name: &str, src: &str) {
    let module = compile_source(src, "prog").expect("lowering should succeed");
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "SIR validation failed for {name}: {:?}",
        report.issues
    );
    let artifact = semantic_ir_to_javascript::compile(&module).expect("backend emit should succeed");

    let mut path = std::env::temp_dir();
    path.push(format!("axiom_sir_e2e_{name}_{}.js", std::process::id()));
    // `create_new` (not `std::fs::write`) fails loudly on an existing path
    // (including a symlink) instead of silently following/truncating
    // through it -- mirrors `maple-to-semantic-ir`'s/`reduce-to-semantic-ir`'s
    // identical helper. Each test uses a unique name+PID, so this should
    // never legitimately collide.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create temp js (create_new, not following an existing symlink)");
    file.write_all(artifact.source.as_bytes()).expect("write temp js");
    drop(file);

    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "node failed for {name}: stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Wrap `src`'s ONE top-level expression in `print(...)` before compiling —
/// the harness-only observability pattern every sibling SIR23 oracle file
/// uses (`wrap_top_level_in_print`), needed here ONLY for the three tests
/// below that now assert on a printed VALUE rather than merely "ran
/// cleanly". Simpler than `tests/oracle.rs`'s own
/// `wrap_axiom_top_level_for_observation` (which additionally unrolls a
/// top-level `CompoundExpression` block) because none of these three
/// sources is a `;`-block.
fn run_via_node_expecting(name: &str, src: &str, expected_stdout: &str) {
    let mut module = compile_source(src, "prog").expect("lowering should succeed");
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "SIR validation failed for {name}: {:?}",
        report.issues
    );
    wrap_whole_statement_in_print(&mut module);
    let artifact = semantic_ir_to_javascript::compile(&module).expect("backend emit should succeed");

    let mut path = std::env::temp_dir();
    path.push(format!("axiom_sir_e2e_{name}_{}.js", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create temp js (create_new, not following an existing symlink)");
    file.write_all(artifact.source.as_bytes()).expect("write temp js");
    drop(file);

    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "node failed for {name}: stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end_matches(['\n', '\r']), expected_stdout, "for {name}");
}

fn wrap_whole_statement_in_print(module: &mut Module) {
    for f in &mut module.functions {
        if f.name != "main" {
            continue;
        }
        for stmt in &mut f.body.stmts {
            if let Stmt::ExprStmt { expr, span } = stmt {
                let inner = std::mem::replace(expr, Expr::NilLit { span: span.clone() });
                *expr = Expr::BuiltinCall {
                    name: "print".to_string(),
                    args: vec![inner],
                    effects: EffectSet::PURE,
                    span: span.clone(),
                };
            }
        }
    }
}

#[test]
fn plain_arithmetic_runs_in_node() {
    if !node_available() {
        eprintln!("skipping plain_arithmetic_runs_in_node: `node` not available");
        return;
    }
    run_via_node("arithmetic", "1 + 2 * 3");
}

#[test]
fn a_declared_function_definition_and_call_run_in_node() {
    if !node_available() {
        eprintln!("skipping a_declared_function_definition_and_call_run_in_node: `node` not available");
        return;
    }
    run_via_node(
        "declared_define_call",
        "(power(x: Integer, n: NonNegativeInteger): Integer == x ** n; power(2, 3))",
    );
}

#[test]
fn an_undeclared_function_definition_and_call_run_in_node() {
    if !node_available() {
        eprintln!("skipping an_undeclared_function_definition_and_call_run_in_node: `node` not available");
        return;
    }
    run_via_node("undeclared_define_call", "(f x == x * x; f(3))");
}

#[test]
fn assignment_runs_in_node() {
    if !node_available() {
        eprintln!("skipping assignment_runs_in_node: `node` not available");
        return;
    }
    run_via_node("assignment", "(x := 5; y := x + 1)");
}

#[test]
fn a_list_literal_runs_in_node() {
    if !node_available() {
        eprintln!("skipping a_list_literal_runs_in_node: `node` not available");
        return;
    }
    run_via_node("list", "[1, 2, 3]");
}

#[test]
fn if_then_else_runs_in_node() {
    if !node_available() {
        eprintln!("skipping if_then_else_runs_in_node: `node` not available");
        return;
    }
    run_via_node("if_then_else", "if 1 > 0 then 1 else -1");
}

#[test]
fn a_multi_statement_block_runs_in_node() {
    if !node_available() {
        eprintln!("skipping a_multi_statement_block_runs_in_node: `node` not available");
        return;
    }
    run_via_node("block", "(a := 1; b := 2; a + b)");
}

#[test]
fn a_declaration_evaluates_to_lowercase_true() {
    if !node_available() {
        eprintln!("skipping a_declaration_evaluates_to_lowercase_true: `node` not available");
        return;
    }
    run_via_node_expecting("declaration", "a : PositiveInteger", "true");
}

#[test]
fn a_coercion_evaluates_and_prints_the_coerced_value() {
    if !node_available() {
        eprintln!("skipping a_coercion_evaluates_and_prints_the_coerced_value: `node` not available");
        return;
    }
    run_via_node_expecting("coercion", "3 :: Fraction(Integer)", "3");
}

#[test]
fn a_has_query_evaluates_to_the_books_own_confirmed_answer() {
    if !node_available() {
        eprintln!("skipping a_has_query_evaluates_to_the_books_own_confirmed_answer: `node` not available");
        return;
    }
    run_via_node_expecting("has_query", "Polynomial(Integer) has Ring", "true");
}

#[test]
fn a_complex_multi_construct_program_runs_in_node() {
    if !node_available() {
        eprintln!("skipping a_complex_multi_construct_program_runs_in_node: `node` not available");
        return;
    }
    run_via_node(
        "complex",
        "(a : PositiveInteger; a := 5; f(x: Integer): Integer == if x > 0 then x else -x; f(a))",
    );
}
