//! JV02 milestone M0 tests: literals + a synthesized `main` function.
//!
//! Every positive test also asserts the lowered [`Module`] passes
//! `semantic_ir::validate()` — not just that lowering itself didn't
//! panic/error, mirroring `matlab-to-semantic-ir`'s own
//! `tests/test_validator.rs` discipline (a module that lowers but fails
//! the shared SIR validator is not actually working, just runnable).

use java_to_semantic_ir::{compile, compile_source};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{Expr, Function, Module, Stmt};

fn compile_ok(src: &str) -> Module {
    let module =
        compile_source(src, "prog").unwrap_or_else(|e| panic!("expected lowering to succeed: {e:?}"));
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "SIR validation failed for `{src}`: {:?}",
        report.issues
    );
    module
}

fn main_fn(m: &Module) -> &Function {
    m.functions
        .iter()
        .find(|f| f.name == "main")
        .expect("expected a synthesized `main` function")
}

fn wrap(body: &str) -> String {
    format!("class Main {{ public static void main(String[] args) {{ {body} }} }}")
}

// ── literals ─────────────────────────────────────────────────────────────

#[test]
fn integer_literal() {
    let m = compile_ok(&wrap("42;"));
    let main = main_fn(&m);
    assert_eq!(main.body.stmts.len(), 1);
    match &main.body.stmts[0] {
        Stmt::ExprStmt { expr, .. } => {
            assert!(matches!(expr, Expr::IntLit { value: 42, .. }));
        }
        other => panic!("expected ExprStmt, got {other:?}"),
    }
}

// `3.14` below is an arbitrary float literal test value, not an
// approximation of PI.
#[allow(clippy::approx_constant)]
#[test]
fn float_literal() {
    let m = compile_ok(&wrap("3.14;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::ExprStmt { expr, .. } => {
            assert!(matches!(expr, Expr::FloatLit { value, .. } if (*value - 3.14).abs() < 1e-9));
        }
        other => panic!("expected ExprStmt, got {other:?}"),
    }
}

#[test]
fn float_literal_with_exponent() {
    let m = compile_ok(&wrap("1e3;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::ExprStmt { expr, .. } => {
            assert!(matches!(expr, Expr::FloatLit { value, .. } if (*value - 1000.0).abs() < 1e-9));
        }
        other => panic!("expected ExprStmt, got {other:?}"),
    }
}

// Java's `float` literal suffix (`3.14f`) -- M0 does not distinguish
// `float` from `double`, both lower to Expr::FloatLit (see lower.rs's own
// doc comment on `number_literal_expr`). `3.14` is an arbitrary test
// value, not an approximation of PI.
#[allow(clippy::approx_constant)]
#[test]
fn float_literal_with_f_suffix() {
    let m = compile_ok(&wrap("3.14f;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::ExprStmt { expr, .. } => {
            assert!(matches!(expr, Expr::FloatLit { value, .. } if (*value - 3.14).abs() < 1e-6));
        }
        other => panic!("expected ExprStmt, got {other:?}"),
    }
}

#[test]
fn large_integer_falls_back_to_float() {
    // Larger than i64::MAX -- must not silently truncate or panic.
    let m = compile_ok(&wrap("99999999999999999999;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::ExprStmt { expr, .. } => {
            assert!(matches!(expr, Expr::FloatLit { .. }));
        }
        other => panic!("expected ExprStmt, got {other:?}"),
    }
}

#[test]
fn boolean_literals() {
    let m = compile_ok(&wrap("true; false;"));
    let main = main_fn(&m);
    assert_eq!(main.body.stmts.len(), 2);
    assert!(matches!(
        &main.body.stmts[0],
        Stmt::ExprStmt {
            expr: Expr::BoolLit { value: true, .. },
            ..
        }
    ));
    assert!(matches!(
        &main.body.stmts[1],
        Stmt::ExprStmt {
            expr: Expr::BoolLit { value: false, .. },
            ..
        }
    ));
}

#[test]
fn null_literal() {
    let m = compile_ok(&wrap("null;"));
    assert!(matches!(
        &main_fn(&m).body.stmts[0],
        Stmt::ExprStmt {
            expr: Expr::NilLit { .. },
            ..
        }
    ));
}

#[test]
fn string_literal() {
    let m = compile_ok(&wrap(r#""hello";"#));
    match &main_fn(&m).body.stmts[0] {
        Stmt::ExprStmt { expr, .. } => {
            assert!(matches!(expr, Expr::StrLit { value, .. } if value == "hello"));
        }
        other => panic!("expected ExprStmt, got {other:?}"),
    }
}

#[test]
fn multiple_statements_lower_in_source_order() {
    let m = compile_ok(&wrap("1; 2; 3;"));
    let main = main_fn(&m);
    assert_eq!(main.body.stmts.len(), 3);
    for (i, stmt) in main.body.stmts.iter().enumerate() {
        match stmt {
            Stmt::ExprStmt { expr, .. } => {
                assert!(matches!(expr, Expr::IntLit { value, .. } if *value == (i as i64) + 1));
            }
            other => panic!("expected ExprStmt, got {other:?}"),
        }
    }
}

#[test]
fn empty_main_body_lowers_to_zero_statements() {
    let m = compile_ok(&wrap(""));
    assert_eq!(main_fn(&m).body.stmts.len(), 0);
}

#[test]
fn module_name_is_preserved() {
    let m = compile_source(&wrap("42;"), "my-module").unwrap();
    assert_eq!(m.name, "my-module");
}

#[test]
fn metadata_records_source_language_and_sir_version() {
    let m = compile_ok(&wrap("42;"));
    assert_eq!(m.metadata.source_language.as_deref(), Some("java"));
    assert_eq!(
        m.metadata.sir_version.as_deref(),
        Some(semantic_ir::CURRENT_SIR_VERSION)
    );
}

// ── error cases (M0 scope boundary) ─────────────────────────────────────

#[test]
fn missing_main_method_is_an_error() {
    let err = compile_source("class Main { void other() { } }", "prog").unwrap_err();
    assert!(err.message.contains("main"), "unexpected message: {}", err.message);
}

#[test]
fn no_class_declaration_is_an_error() {
    let err = compile_source("", "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn multiple_top_level_classes_is_an_error() {
    let src = "class A { public static void main(String[] args) { } } class B { }";
    let err = compile_source(src, "prog").unwrap_err();
    assert!(
        err.message.contains("exactly one"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn variable_reference_is_unsupported_in_m0() {
    let err = compile_source(&wrap("x;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn binary_operator_is_unsupported_in_m0() {
    let err = compile_source(&wrap("1 + 2;"), "prog").unwrap_err();
    assert!(
        err.message.contains("operator"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn unary_minus_is_unsupported_in_m0() {
    let err = compile_source(&wrap("-7;"), "prog").unwrap_err();
    assert!(
        err.message.contains("operator"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn method_call_is_unsupported_in_m0() {
    let err = compile_source(&wrap("System.out.println(1);"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

// ── depth-guard regression (CWE-674, found by /security-review) ────────

fn node(rule_name: &str, children: Vec<ASTNodeOrToken>) -> GrammarASTNode {
    GrammarASTNode {
        rule_name: rule_name.to_string(),
        children,
        start_line: Some(1),
        start_column: Some(1),
        end_line: Some(1),
        end_column: Some(1),
    }
}

/// `find_main_method`'s recursive class-body search must not overflow the
/// native stack on a pathologically deep tree handed directly to the
/// public `compile()` entry point (which accepts a raw `GrammarASTNode`,
/// not only one produced by `parse_java`'s own depth-capped parser).
/// Regression test for a real gap `/security-review` found before this
/// crate shipped: an earlier version of `find_main_method`'s `search`
/// helper had no depth cap of its own at all.
#[test]
fn deeply_nested_class_body_reports_depth_error_not_stack_overflow() {
    // Build `program -> class_declaration -> wrapper(wrapper(...(leaf)))`,
    // far deeper than MAX_TREE_DEPTH, with no `method_declaration`
    // anywhere -- the search must terminate with a depth error rather
    // than recursing forever (or, pre-fix, overflowing the stack).
    let mut inner = node("leaf", vec![]);
    for _ in 0..500 {
        inner = node("wrapper", vec![ASTNodeOrToken::Node(inner)]);
    }
    let class_decl = node("class_declaration", vec![ASTNodeOrToken::Node(inner)]);
    let program = node("program", vec![ASTNodeOrToken::Node(class_decl)]);

    let err = compile(&program, "prog").unwrap_err();
    assert!(
        err.message.contains("nesting exceeds"),
        "expected a depth-exceeded error, got: {}",
        err.message
    );
}

/// The top-level `class_declaration` search (in `lower_program`, before
/// `find_main_method` ever runs) must be depth-guarded too — a second,
/// earlier-executing instance of the same CWE-674 gap `/security-review`
/// found: `parser::grammar_parser::find_nodes` (originally used here) has
/// no depth cap of its own. This tree has NO `class_declaration` or
/// `method_declaration` anywhere, so a pre-fix version would recurse to
/// the bottom of the whole 500-level chain before ever producing any
/// error at all.
#[test]
fn deeply_nested_tree_with_no_class_declaration_reports_depth_error() {
    let mut inner = node("leaf", vec![]);
    for _ in 0..500 {
        inner = node("wrapper", vec![ASTNodeOrToken::Node(inner)]);
    }
    let program = node("program", vec![ASTNodeOrToken::Node(inner)]);

    let err = compile(&program, "prog").unwrap_err();
    assert!(
        err.message.contains("nesting exceeds"),
        "expected a depth-exceeded error, got: {}",
        err.message
    );
}
