//! End-to-end integration tests for the Axiom reserved-head handlers
//! (`__axiom_declare`/`__axiom_coerce`/`__axiom_has`) added to
//! `semantic-ir-to-javascript`'s SIR23 evaluator (MA13/Wave 7 close-out —
//! `code/specs/MA13-axiom-language.md`).
//!
//! Mirrors `tests/sir23_symbolic.rs`'s own hand-built-SIR-then-run-under-
//! `node` pattern: these tests exercise the three new heads IN ISOLATION,
//! with **no dependency on `axiom-to-semantic-ir` at all** (this crate has
//! no such dependency today and should not gain one just for a test) —
//! every `SymApply`/`SymSymbol` node below is hand-constructed to the exact
//! shape `axiom-to-semantic-ir::lower` documents it emits (confirmed
//! directly against that crate's own `src/lower.rs`: `__axiom_declare(List
//! (name, ...), typeExpr)`, `__axiom_coerce(value, typeExpr)`,
//! `__axiom_has(domainTypeExpr, categoryTypeExpr)`, and a bare/
//! parameterized `type_expr` lowering to a plain `SymSymbol`/`SymApply` —
//! the same shape an ordinary call already has).
//!
//! For the full oracle diff against `axiom-runtime` itself (native vs.
//! compiled, real Axiom SOURCE text on both sides), see
//! `axiom-to-semantic-ir/tests/oracle.rs` instead — this file's job is
//! narrower: prove the three handlers here behave correctly for hand-fed
//! SIR node shapes, independent of whichever frontend produces them.
//!
//! Node is optional at test time; when unavailable every test degrades to
//! a no-op rather than failing (mirroring `sir23_symbolic.rs`).

use std::path::PathBuf;
use std::process::Command;

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span, Stmt,
};
use semantic_ir_to_javascript::compile;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn sp() -> Span {
    Span::synthetic()
}

fn sym(name: &str) -> Expr {
    Expr::SymSymbol {
        name: name.into(),
        span: sp(),
    }
}

fn sym_apply(head: Expr, args: Vec<Expr>) -> Expr {
    Expr::SymApply {
        head: Box::new(head),
        args,
        span: sp(),
    }
}

fn int_lit(value: i64) -> Expr {
    Expr::IntLit { value, span: sp() }
}

fn bc(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall {
        name: name.into(),
        args,
        effects: EffectSet::PURE,
        span: sp(),
    }
}

/// A statement that prints `arg`'s EVALUATED value — the harness-only
/// observability pattern every sibling SIR23 oracle/integration test in
/// this crate uses (`emit.rs`'s `pick_print_of_sym23_root` arm).
fn print(arg: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: bc("print", vec![arg]),
        span: sp(),
    }
}

/// A statement that evaluates `expr` for its side effects and discards the
/// result — `emit.rs`'s `is_sym23_root_shape` arm wraps this in exactly
/// one `evalTerm` call, same as `print` above minus the `console.log`.
fn bare(expr: Expr) -> Stmt {
    Stmt::ExprStmt { expr, span: sp() }
}

/// A `type_expr`-shaped term: a bare `NAME` lowers to `SymSymbol`, a
/// parameterized `NAME(args...)` to `SymApply(SymSymbol(name), args)` —
/// mirrors `axiom-to-semantic-ir::lower::Lowerer::lower_type_expr` exactly
/// (see that function's own doc comment: "the exact same node shapes an
/// ordinary call already produces").
fn type_expr(name: &str, args: Vec<Expr>) -> Expr {
    if args.is_empty() {
        sym(name)
    } else {
        sym_apply(sym(name), args)
    }
}

fn names_list(names: &[&str]) -> Expr {
    sym_apply(sym("List"), names.iter().map(|n| sym(n)).collect())
}

fn declare(names: &[&str], ty: Expr) -> Expr {
    sym_apply(sym("__axiom_declare"), vec![names_list(names), ty])
}

fn coerce(value: Expr, ty: Expr) -> Expr {
    sym_apply(sym("__axiom_coerce"), vec![value, ty])
}

fn has(domain: Expr, category: Expr) -> Expr {
    sym_apply(sym("__axiom_has"), vec![domain, category])
}

fn assign(name: &str, value: Expr) -> Expr {
    sym_apply(sym("Assign"), vec![sym(name), value])
}

/// `source_language("axiom")` — so `SIR_DISPLAY_AXIOM_BOOLEAN` is exercised
/// too (the shared `True`/`False` symbols must render lowercase here,
/// unlike every other source language's own hand-built module in
/// `sir23_symbolic.rs`, which uses `"handbuilt"` and gets the generic,
/// capitalized spelling).
fn module_with_main(stmts: Vec<Stmt>, value: Expr) -> Module {
    Module {
        name: "sir23_axiom".into(),
        manifest: FeatureManifest::from_features(&[Feature::SymbolicExpr]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts,
                value,
                span: sp(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: sp(),
        }],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("axiom")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    }
}

/// Compile and run `module` under `node`, returning `(exit_success, stdout,
/// stderr)`. `None` when `node` is unavailable (skip, don't fail).
fn run_module(module: &Module, tag: &str) -> Option<(bool, String, String)> {
    let artifact = compile(module).expect("compile to javascript");
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_js_axiom_{}_{}.js", tag, std::process::id()));
    std::fs::write(&path, &artifact.source).expect("write temp js");
    let output = Command::new("node")
        .arg(&path)
        .output()
        .expect("spawn node");
    let _ = std::fs::remove_file(&path);
    let stdout = String::from_utf8_lossy(&output.stdout)
        .trim_end_matches(['\n', '\r'])
        .to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Some((output.status.success(), stdout, stderr))
}

// ---------------------------------------------------------------------------
// __axiom_declare
// ---------------------------------------------------------------------------

#[test]
fn declare_resolves_and_returns_lowercase_true() {
    let module = module_with_main(
        vec![print(declare(&["a"], type_expr("PositiveInteger", vec![])))],
        int_lit(0),
    );
    if let Some((ok, stdout, stderr)) = run_module(&module, "declare_true") {
        assert!(ok, "node failed: {stderr}");
        assert_eq!(stdout, "true");
    }
}

#[test]
fn declare_rejects_an_unknown_domain_name_with_the_books_error_shape() {
    let module = module_with_main(
        vec![print(declare(&["a"], type_expr("Matrix", vec![])))],
        int_lit(0),
    );
    if let Some((ok, _stdout, stderr)) = run_module(&module, "declare_unknown_domain") {
        assert!(
            !ok,
            "expected node to exit non-zero for an unresolvable domain name"
        );
        assert!(
            stderr.contains("is not one of this cut's fixed built-in domains"),
            "stderr: {stderr}"
        );
    }
}

// ---------------------------------------------------------------------------
// __axiom_coerce
// ---------------------------------------------------------------------------

#[test]
fn coerce_integer_to_fraction_integer_succeeds_unchanged() {
    let module = module_with_main(
        vec![print(coerce(
            int_lit(3),
            type_expr("Fraction", vec![type_expr("Integer", vec![])]),
        ))],
        int_lit(0),
    );
    if let Some((ok, stdout, stderr)) = run_module(&module, "coerce_fraction") {
        assert!(ok, "node failed: {stderr}");
        assert_eq!(stdout, "3");
    }
}

#[test]
fn coerce_a_negative_integer_to_positive_integer_fails_with_the_books_error_shape() {
    let module = module_with_main(
        vec![print(coerce(int_lit(-1), type_expr("PositiveInteger", vec![])))],
        int_lit(0),
    );
    if let Some((ok, _stdout, stderr)) = run_module(&module, "coerce_fail") {
        assert!(!ok, "expected node to exit non-zero for a failed coercion");
        assert!(
            stderr.contains("Cannot convert -1 to an object of the type PositiveInteger."),
            "stderr: {stderr}"
        );
    }
}

// ---------------------------------------------------------------------------
// __axiom_has
// ---------------------------------------------------------------------------

#[test]
fn has_query_polynomial_integer_ring_is_true_the_books_own_confirmed_example() {
    let module = module_with_main(
        vec![print(has(
            type_expr("Polynomial", vec![type_expr("Integer", vec![])]),
            type_expr("Ring", vec![]),
        ))],
        int_lit(0),
    );
    if let Some((ok, stdout, stderr)) = run_module(&module, "has_true") {
        assert!(ok, "node failed: {stderr}");
        assert_eq!(stdout, "true");
    }
}

#[test]
fn has_query_list_integer_ring_is_false_the_books_own_confirmed_example() {
    let module = module_with_main(
        vec![print(has(
            type_expr("List", vec![type_expr("Integer", vec![])]),
            type_expr("Ring", vec![]),
        ))],
        int_lit(0),
    );
    if let Some((ok, stdout, stderr)) = run_module(&module, "has_false") {
        assert!(ok, "node failed: {stderr}");
        assert_eq!(stdout, "false");
    }
}

// ---------------------------------------------------------------------------
// Declare + Assign interaction (`assignHandler`'s own small, disclosed
// addition — see `runtime.rs`'s comment on that function).
// ---------------------------------------------------------------------------

#[test]
fn declare_then_matching_assignment_succeeds_and_binds() {
    let module = module_with_main(
        vec![
            bare(declare(&["a"], type_expr("PositiveInteger", vec![]))),
            bare(assign("a", int_lit(5))),
            print(sym_apply(sym("Add"), vec![sym("a"), int_lit(1)])),
        ],
        int_lit(0),
    );
    if let Some((ok, stdout, stderr)) = run_module(&module, "declare_then_assign_ok") {
        assert!(ok, "node failed: {stderr}");
        assert_eq!(stdout, "6");
    }
}

#[test]
fn declare_then_mismatched_assignment_fails_with_the_books_error_shape() {
    let module = module_with_main(
        vec![
            bare(declare(&["a"], type_expr("PositiveInteger", vec![]))),
            print(assign("a", int_lit(-1))),
        ],
        int_lit(0),
    );
    if let Some((ok, _stdout, stderr)) = run_module(&module, "declare_then_assign_fail") {
        assert!(
            !ok,
            "expected node to exit non-zero for a declared-domain mismatch"
        );
        assert!(
            stderr.contains(
                "Cannot convert right-hand side of assignment -1 to an object of the type \
                 PositiveInteger of the left-hand side."
            ),
            "stderr: {stderr}"
        );
    }
}

#[test]
fn assign_without_a_prior_declaration_is_unrestricted() {
    // Confirms `axiomDeclaredDomains`'s lookup is a genuine no-op absent a
    // `__axiom_declare` call for this name -- `Assign`'s ordinary behaviour
    // is completely unaffected (regression guard for the narrowness of the
    // `assignHandler` addition every OTHER SIR23 frontend's own `:=`/`=`/
    // `:` also routes through).
    let module = module_with_main(vec![print(assign("z", int_lit(-5)))], int_lit(0));
    if let Some((ok, stdout, stderr)) = run_module(&module, "assign_unrestricted") {
        assert!(ok, "node failed: {stderr}");
        assert_eq!(stdout, "-5");
    }
}
