//! JV02 milestone M0/M1/M2a tests: literals, a synthesized `main`
//! function (M0); local variable declarations/re-assignment/operators
//! (M1); if/while/do-while, and compound-assignment/increment/decrement
//! as bare statements (M2a).
//!
//! Every positive test also asserts the lowered [`Module`] passes
//! `semantic_ir::validate()` — not just that lowering itself didn't
//! panic/error, mirroring `matlab-to-semantic-ir`'s own
//! `tests/test_validator.rs` discipline (a module that lowers but fails
//! the shared SIR validator is not actually working, just runnable).

use java_to_semantic_ir::{compile, compile_source};
use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{Expr, Feature, Function, Module, ParamKind, Scope, Stmt};

fn compile_ok(src: &str) -> Module {
    let module = compile_source(src, "prog")
        .unwrap_or_else(|e| panic!("expected lowering to succeed: {e:?}"));
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

/// M3a: a full class body (multiple `method_declaration`s), unlike
/// [`wrap`] which only ever wraps `main`'s own body.
fn class_src(class_body: &str) -> String {
    format!("class Main {{ {class_body} }}")
}

fn find_fn<'a>(m: &'a Module, name: &str) -> &'a Function {
    m.functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| {
            panic!(
                "expected a `{name}` function, found {:?}",
                m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
            )
        })
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
    assert!(
        err.message.contains("main"),
        "unexpected message: {}",
        err.message
    );
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
fn undeclared_variable_reference_is_an_error() {
    let err = compile_source(&wrap("x;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn method_call_is_unsupported_in_m1() {
    let err = compile_source(&wrap("System.out.println(1);"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

// ── M1: local variable declarations ─────────────────────────────────────

#[test]
fn int_declaration_with_explicit_type() {
    let m = compile_ok(&wrap("int x = 1;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding { name, value, .. } => {
            assert_eq!(name, "x");
            assert!(matches!(value, Expr::IntLit { value: 1, .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn every_integral_primitive_type_declares() {
    for prim in ["byte", "short", "int", "long", "char"] {
        let m = compile_ok(&wrap(&format!("{prim} x = 1;")));
        assert!(
            matches!(&main_fn(&m).body.stmts[0], Stmt::LetStarBinding { .. }),
            "expected LetStarBinding for `{prim}`"
        );
    }
}

#[test]
fn every_floating_primitive_type_declares() {
    for prim in ["float", "double"] {
        let m = compile_ok(&wrap(&format!("{prim} x = 1.5;")));
        assert!(
            matches!(&main_fn(&m).body.stmts[0], Stmt::LetStarBinding { .. }),
            "expected LetStarBinding for `{prim}`"
        );
    }
}

#[test]
fn boolean_declaration_with_explicit_type() {
    let m = compile_ok(&wrap("boolean b = true;"));
    assert!(matches!(
        &main_fn(&m).body.stmts[0],
        Stmt::LetStarBinding { .. }
    ));
}

#[test]
fn string_declaration_with_explicit_type() {
    let m = compile_ok(&wrap(r#"String s = "hi";"#));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::StrLit { value, .. } if value == "hi"));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn string_declaration_accepts_null_initializer() {
    // Declared type wins over the initializer's own (transient) `Null`
    // kind -- see `lower_local_var_decl`'s handling of `declared_kind`.
    let m = compile_ok(&wrap(r#"String s = null;"#));
    assert!(matches!(
        &main_fn(&m).body.stmts[0],
        Stmt::LetStarBinding { .. }
    ));
}

#[test]
fn var_infers_int_from_initializer() {
    let m = compile_ok(&wrap("var x = 1;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::IntLit { value: 1, .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn var_infers_float_from_initializer() {
    let m = compile_ok(&wrap("var x = 1.5; double y = x + 1.0;"));
    assert_eq!(main_fn(&m).body.stmts.len(), 2);
}

#[test]
fn var_infers_string_from_initializer_and_supports_reassignment() {
    let m = compile_ok(&wrap(r#"var s = "hi"; s = "bye";"#));
    let main = main_fn(&m);
    assert_eq!(main.body.stmts.len(), 2);
    assert!(matches!(&main.body.stmts[0], Stmt::LetStarBinding { .. }));
    match &main.body.stmts[1] {
        Stmt::Assign { name, value, .. } => {
            assert_eq!(name, "s");
            assert!(matches!(value, Expr::StrLit { value, .. } if value == "bye"));
        }
        other => panic!("expected Assign, got {other:?}"),
    }
}

#[test]
fn var_cannot_infer_type_from_null_initializer() {
    let err = compile_source(&wrap("var x = null;"), "prog").unwrap_err();
    assert!(
        err.message.contains("null"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn uninitialized_declaration_is_an_error() {
    let err = compile_source(&wrap("int x;"), "prog").unwrap_err();
    assert!(
        err.message.contains("initializer"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn multiple_declarators_in_one_statement_is_an_error() {
    let err = compile_source(&wrap("int x = 1, y = 2;"), "prog").unwrap_err();
    assert!(
        err.message.contains("declarator"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn c_style_array_declarator_is_an_error() {
    let err = compile_source(&wrap("int x[] = null;"), "prog").unwrap_err();
    assert!(
        err.message.contains("array"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn unsupported_reference_type_is_an_error() {
    let err = compile_source(&wrap("Object x = null;"), "prog").unwrap_err();
    assert!(
        err.message.contains("reference type"),
        "unexpected message: {}",
        err.message
    );
}

// ── M1: re-assignment ────────────────────────────────────────────────────

#[test]
fn reassignment_of_a_declared_variable() {
    let m = compile_ok(&wrap("int x = 1; x = 2;"));
    let main = main_fn(&m);
    assert_eq!(main.body.stmts.len(), 2);
    match &main.body.stmts[1] {
        Stmt::Assign {
            name, scope, value, ..
        } => {
            assert_eq!(name, "x");
            assert_eq!(*scope, semantic_ir::Scope::Local);
            assert!(matches!(value, Expr::IntLit { value: 2, .. }));
        }
        other => panic!("expected Assign, got {other:?}"),
    }
}

#[test]
fn assignment_to_undeclared_variable_is_an_error() {
    let err = compile_source(&wrap("x = 1;"), "prog").unwrap_err();
    assert!(
        err.message.contains("undeclared"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn compound_assignment_as_a_statement_desugars_to_assign() {
    let m = compile_ok(&wrap("int x = 1; x += 2;"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::Assign {
            name,
            value: Expr::BuiltinCall { name: op, args, .. },
            ..
        } => {
            assert_eq!(name, "x");
            assert_eq!(op, "+");
            assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "x"));
            assert!(matches!(&args[1], Expr::IntLit { value: 2, .. }));
        }
        other => panic!("expected Assign(BuiltinCall(\"+\")), got {other:?}"),
    }
}

#[test]
fn every_compound_assignment_operator_desugars_to_its_builtin_name() {
    for (src_op, builtin_name) in [("-=", "-"), ("*=", "*"), ("%=", "%")] {
        let src = format!("int x = 5; x {src_op} 2;");
        let m = compile_ok(&wrap(&src));
        match &main_fn(&m).body.stmts[1] {
            Stmt::Assign {
                value: Expr::BuiltinCall { name, .. },
                ..
            } => {
                assert_eq!(name, builtin_name, "for source operator `{src_op}`");
            }
            other => panic!("expected Assign(BuiltinCall), got {other:?}"),
        }
    }
}

#[test]
fn compound_divide_assignment_selects_div_trunc_or_div_true() {
    let m = compile_ok(&wrap("int x = 5; x /= 2;"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::Assign {
            value: Expr::BuiltinCall { name, .. },
            ..
        } => assert_eq!(name, "div_trunc"),
        other => panic!("expected Assign(BuiltinCall(\"div_trunc\")), got {other:?}"),
    }
    let m = compile_ok(&wrap("double x = 5.0; x /= 2.0;"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::Assign {
            value: Expr::BuiltinCall { name, .. },
            ..
        } => assert_eq!(name, "div_true"),
        other => panic!("expected Assign(BuiltinCall(\"div_true\")), got {other:?}"),
    }
}

#[test]
fn compound_plus_assignment_on_a_string_concatenates() {
    let m = compile_ok(&wrap(r#"String s = "a"; s += "b";"#));
    match &main_fn(&m).body.stmts[1] {
        Stmt::Assign {
            value: Expr::StrConcat { parts, .. },
            ..
        } => assert_eq!(parts.len(), 2),
        other => panic!("expected Assign(StrConcat), got {other:?}"),
    }
}

#[test]
fn bitwise_compound_assignment_is_unsupported() {
    let err = compile_source(&wrap("int x = 1; x &= 1;"), "prog").unwrap_err();
    assert!(
        err.message.contains("unsupported assignment operator"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn nested_assignment_expression_is_unsupported() {
    let err = compile_source(&wrap("int x = 1; int y = 1; y = (x = 2);"), "prog").unwrap_err();
    assert!(
        err.message.contains("nested assignment"),
        "unexpected message: {}",
        err.message
    );
}

// ── M1: arithmetic operators ─────────────────────────────────────────────

#[test]
fn integer_addition() {
    let m = compile_ok(&wrap("int x = 1 + 2;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::BuiltinCall { name, args, .. },
            ..
        } => {
            assert_eq!(name, "+");
            assert_eq!(args.len(), 2);
        }
        other => panic!("expected LetStarBinding(BuiltinCall(\"+\")), got {other:?}"),
    }
}

#[test]
fn every_arithmetic_operator_lowers_to_its_builtin_name() {
    for (src_op, builtin_name) in [("-", "-"), ("*", "*"), ("%", "%")] {
        let src = format!("int x = 5 {src_op} 2;");
        let m = compile_ok(&wrap(&src));
        match &main_fn(&m).body.stmts[0] {
            Stmt::LetStarBinding {
                value: Expr::BuiltinCall { name, .. },
                ..
            } => {
                assert_eq!(name, builtin_name, "for source operator `{src_op}`");
            }
            other => panic!("expected LetStarBinding(BuiltinCall), got {other:?}"),
        }
    }
}

#[test]
fn integer_division_lowers_to_div_trunc() {
    let m = compile_ok(&wrap("int x = 5 / 2;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::BuiltinCall { name, .. },
            ..
        } => {
            assert_eq!(name, "div_trunc");
        }
        other => panic!("expected LetStarBinding(BuiltinCall(\"div_trunc\")), got {other:?}"),
    }
}

#[test]
fn division_involving_a_float_lowers_to_div_true() {
    let m = compile_ok(&wrap("double x = 5 / 2.0;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::BuiltinCall { name, .. },
            ..
        } => {
            assert_eq!(name, "div_true");
        }
        other => panic!("expected LetStarBinding(BuiltinCall(\"div_true\")), got {other:?}"),
    }
}

#[test]
fn arithmetic_on_non_numeric_operands_is_an_error() {
    let err = compile_source(&wrap(r#"boolean b = true; int x = b - 1;"#), "prog").unwrap_err();
    assert!(
        err.message.contains("numeric"),
        "unexpected message: {}",
        err.message
    );
}

// ── M1: comparison operators ─────────────────────────────────────────────

#[test]
fn every_relational_operator_lowers_to_its_builtin_name() {
    for op in ["<", ">", "<=", ">="] {
        let src = format!("boolean b = 1 {op} 2;");
        let m = compile_ok(&wrap(&src));
        match &main_fn(&m).body.stmts[0] {
            Stmt::LetStarBinding {
                value: Expr::BuiltinCall { name, .. },
                ..
            } => {
                assert_eq!(name, op);
            }
            other => panic!("expected LetStarBinding(BuiltinCall), got {other:?}"),
        }
    }
}

#[test]
fn equals_equals_lowers_to_bare_equals_builtin() {
    let m = compile_ok(&wrap("boolean b = 1 == 2;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::BuiltinCall { name, .. },
            ..
        } => {
            assert_eq!(name, "=");
        }
        other => panic!("expected LetStarBinding(BuiltinCall(\"=\")), got {other:?}"),
    }
}

#[test]
fn not_equals_lowers_to_bang_equals_builtin() {
    let m = compile_ok(&wrap("boolean b = 1 != 2;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::BuiltinCall { name, .. },
            ..
        } => {
            assert_eq!(name, "!=");
        }
        other => panic!("expected LetStarBinding(BuiltinCall(\"!=\")), got {other:?}"),
    }
}

#[test]
fn string_reference_equality_is_unsupported() {
    let err = compile_source(
        &wrap(r#"String a = "x"; String b = "x"; boolean c = a == b;"#),
        "prog",
    )
    .unwrap_err();
    assert!(
        err.message.contains("reference equality"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn instanceof_is_unsupported() {
    let err = compile_source(
        &wrap("Object o = null; boolean b = o instanceof String;"),
        "prog",
    );
    // `Object` itself is already rejected before `instanceof` is reached
    // (M1 supports only `String` reference types), so assert generically
    // that this is a clean, non-empty error rather than a panic.
    assert!(err.is_err());
}

// ── M1: logical operators ────────────────────────────────────────────────

#[test]
fn logical_and_and_or() {
    let m = compile_ok(&wrap(
        "boolean a = true; boolean b = false; boolean c = a && b; boolean d = a || b;",
    ));
    let main = main_fn(&m);
    assert!(matches!(
        &main.body.stmts[2],
        Stmt::LetStarBinding {
            value: Expr::LogicalAnd { .. },
            ..
        }
    ));
    assert!(matches!(
        &main.body.stmts[3],
        Stmt::LetStarBinding {
            value: Expr::LogicalOr { .. },
            ..
        }
    ));
}

#[test]
fn logical_and_requires_boolean_operands() {
    let err = compile_source(&wrap("int x = 1 && 2;"), "prog").unwrap_err();
    assert!(
        err.message.contains("boolean"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn logical_not() {
    let m = compile_ok(&wrap("boolean b = !true;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::BuiltinCall { name, .. },
            ..
        } => {
            assert_eq!(name, "not");
        }
        other => panic!("expected LetStarBinding(BuiltinCall(\"not\")), got {other:?}"),
    }
}

// ── M1: unary +/- ─────────────────────────────────────────────────────────

#[test]
fn unary_minus_on_a_literal_constant_folds() {
    let m = compile_ok(&wrap("int x = -7;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::IntLit { value: -7, .. },
            ..
        } => {}
        other => panic!("expected LetStarBinding(IntLit(-7)), got {other:?}"),
    }
}

#[test]
fn unary_minus_on_a_variable_lowers_to_neg_builtin() {
    let m = compile_ok(&wrap("int x = 1; int y = -x;"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::LetStarBinding {
            value: Expr::BuiltinCall { name, .. },
            ..
        } => {
            assert_eq!(name, "neg");
        }
        other => panic!("expected LetStarBinding(BuiltinCall(\"neg\")), got {other:?}"),
    }
}

#[test]
fn unary_plus_is_a_no_op() {
    let m = compile_ok(&wrap("int x = +7;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::IntLit { value: 7, .. },
            ..
        } => {}
        other => panic!("expected LetStarBinding(IntLit(7)), got {other:?}"),
    }
}

#[test]
fn prefix_increment_is_unsupported() {
    let err = compile_source(&wrap("int x = 1; int y = ++x;"), "prog").unwrap_err();
    assert!(
        err.message.contains("increment"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn postfix_increment_as_a_statement_desugars_to_assign() {
    let m = compile_ok(&wrap("int x = 1; x++;"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::Assign {
            name,
            value: Expr::BuiltinCall { name: op, args, .. },
            ..
        } => {
            assert_eq!(name, "x");
            assert_eq!(op, "+");
            assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "x"));
            assert!(matches!(&args[1], Expr::IntLit { value: 1, .. }));
        }
        other => panic!("expected Assign(BuiltinCall(\"+\")), got {other:?}"),
    }
}

#[test]
fn prefix_decrement_as_a_statement_desugars_to_assign() {
    let m = compile_ok(&wrap("int x = 1; --x;"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::Assign {
            name,
            value: Expr::BuiltinCall { name: op, args, .. },
            ..
        } => {
            assert_eq!(name, "x");
            assert_eq!(op, "-");
            assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "x"));
            assert!(matches!(&args[1], Expr::IntLit { value: 1, .. }));
        }
        other => panic!("expected Assign(BuiltinCall(\"-\")), got {other:?}"),
    }
}

#[test]
fn increment_on_a_float_variable_uses_a_float_one() {
    let m = compile_ok(&wrap("double x = 1.0; x++;"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::Assign {
            value: Expr::BuiltinCall { args, .. },
            ..
        } => {
            assert!(
                matches!(&args[1], Expr::FloatLit { value, .. } if (*value - 1.0).abs() < 1e-9)
            );
        }
        other => panic!("expected Assign(BuiltinCall), got {other:?}"),
    }
}

#[test]
fn increment_on_undeclared_variable_is_an_error() {
    let err = compile_source(&wrap("x++;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn increment_on_a_boolean_variable_is_an_error() {
    let err = compile_source(&wrap("boolean b = true; b++;"), "prog").unwrap_err();
    assert!(
        err.message.contains("numeric"),
        "unexpected message: {}",
        err.message
    );
}

// ── M1: string concatenation ─────────────────────────────────────────────

#[test]
fn string_concatenation_of_two_strings() {
    let m = compile_ok(&wrap(r#"String s = "a" + "b";"#));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::StrConcat { parts, .. },
            ..
        } => {
            assert_eq!(parts.len(), 2);
        }
        other => panic!("expected LetStarBinding(StrConcat), got {other:?}"),
    }
}

#[test]
fn string_concatenation_auto_stringifies_non_string_operands() {
    // Mirrors Java's own `+` semantics for mixed-type concatenation
    // (`"n=" + 5` -> `"n=5"`), lowered via the shared `Expr::StrConcat`
    // node (see that node's own doc comment on auto-stringification).
    let m = compile_ok(&wrap(r#"String s = "n=" + 5;"#));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::StrConcat { parts, .. },
            ..
        } => {
            assert_eq!(parts.len(), 2);
            assert!(matches!(&parts[0], Expr::StrLit { value, .. } if value == "n="));
            assert!(matches!(&parts[1], Expr::IntLit { value: 5, .. }));
        }
        other => panic!("expected LetStarBinding(StrConcat), got {other:?}"),
    }
}

#[test]
fn chained_string_concatenation_flattens_into_one_strconcat() {
    let m = compile_ok(&wrap(r#"String s = "a" + 1 + "b" + 2;"#));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::StrConcat { parts, .. },
            ..
        } => {
            assert_eq!(parts.len(), 4);
        }
        other => panic!("expected LetStarBinding(StrConcat), got {other:?}"),
    }
}

// ── M1: parenthesized expressions ────────────────────────────────────────

#[test]
fn parenthesized_expression_changes_grouping() {
    let m = compile_ok(&wrap("int x = (1 + 2) * 3;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::BuiltinCall { name, args, .. },
            ..
        } => {
            assert_eq!(name, "*");
            assert!(matches!(&args[0], Expr::BuiltinCall { name, .. } if name == "+"));
        }
        other => panic!("expected LetStarBinding(BuiltinCall(\"*\")), got {other:?}"),
    }
}

// ── M1: deferred constructs (clean errors, not mis-lowering) ────────────

#[test]
fn ternary_conditional_is_unsupported() {
    let err = compile_source(&wrap("int x = true ? 1 : 2;"), "prog").unwrap_err();
    assert!(
        err.message.contains("ternary"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn bitwise_and_is_unsupported() {
    let err = compile_source(&wrap("int x = 1 & 2;"), "prog").unwrap_err();
    assert!(
        err.message.contains("bitwise"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn shift_operator_is_unsupported() {
    let err = compile_source(&wrap("int x = 1 << 2;"), "prog").unwrap_err();
    assert!(
        err.message.contains("shift"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn cast_expression_is_unsupported() {
    let err = compile_source(&wrap("double x = 1.5; int y = (int) x;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn lambda_expression_is_unsupported() {
    let err = compile_source(&wrap("Runnable r = () -> {};"), "prog");
    // Rejected at the type-resolution step already (`Runnable` isn't
    // `String` or a primitive) -- assert generically.
    assert!(err.is_err());
}

// ── M2a: if/else ─────────────────────────────────────────────────────────

#[test]
fn if_with_else_lowers_to_expr_if() {
    let m = compile_ok(&wrap("if (true) { int x = 1; } else { int x = 2; }"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::ExprStmt {
            expr:
                Expr::If {
                    then_branch,
                    else_branch,
                    ..
                },
            ..
        } => {
            assert_eq!(then_branch.stmts.len(), 1);
            assert_eq!(else_branch.stmts.len(), 1);
        }
        other => panic!("expected ExprStmt(If), got {other:?}"),
    }
}

#[test]
fn if_without_else_gets_a_synthetic_empty_block() {
    let m = compile_ok(&wrap("if (true) { int x = 1; }"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::ExprStmt {
            expr:
                Expr::If {
                    then_branch,
                    else_branch,
                    ..
                },
            ..
        } => {
            assert_eq!(then_branch.stmts.len(), 1);
            assert_eq!(else_branch.stmts.len(), 0);
            assert!(matches!(else_branch.value, Expr::NilLit { .. }));
        }
        other => panic!("expected ExprStmt(If), got {other:?}"),
    }
}

#[test]
fn if_with_a_brace_less_single_statement_body() {
    let m = compile_ok(&wrap("int x = 1; if (true) x = 2;"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::ExprStmt {
            expr: Expr::If { then_branch, .. },
            ..
        } => {
            assert_eq!(then_branch.stmts.len(), 1);
            assert!(matches!(&then_branch.stmts[0], Stmt::Assign { .. }));
        }
        other => panic!("expected ExprStmt(If), got {other:?}"),
    }
}

#[test]
fn if_condition_must_be_boolean() {
    let err = compile_source(&wrap("if (1) {}"), "prog").unwrap_err();
    assert!(
        err.message.contains("boolean"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn a_local_declared_inside_an_if_body_does_not_leak_past_it() {
    // `x` inside the `if` body is a *different* declaration from the
    // outer `x` -- this must lower and validate cleanly (real block
    // scoping, not a flat namespace), and the outer `x` must still be
    // the one referenced afterward.
    let m = compile_ok(&wrap("int x = 1; if (true) { int x = 2; } int y = x;"));
    match &main_fn(&m).body.stmts[2] {
        Stmt::LetStarBinding {
            value: Expr::VarRef { name, .. },
            ..
        } => assert_eq!(name, "x"),
        other => panic!("expected LetStarBinding(VarRef), got {other:?}"),
    }
}

#[test]
fn referencing_an_if_body_local_after_the_if_is_an_error() {
    let err = compile_source(&wrap("if (true) { int x = 1; } int y = x;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

// ── M2a: while / do-while ────────────────────────────────────────────────

#[test]
fn while_loop_lowers_to_stmt_while() {
    let m = compile_ok(&wrap("int x = 0; while (x < 10) { x = x + 1; }"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::While { cond, body, .. } => {
            assert!(matches!(cond, Expr::BuiltinCall { name, .. } if name == "<"));
            assert_eq!(body.stmts.len(), 1);
        }
        other => panic!("expected While, got {other:?}"),
    }
}

#[test]
fn while_condition_must_be_boolean() {
    let err = compile_source(&wrap("while (1) {}"), "prog").unwrap_err();
    assert!(
        err.message.contains("boolean"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn do_while_desugars_to_a_flag_guarded_while_not_a_body_clone() {
    // See lower.rs's own `lower_do_while_statement` doc comment: an
    // earlier version cloned the already-lowered body for a literal
    // "run once, then while" duplication, which /security-review caught
    // as an exponential-blowup DoS on nested do-while (O(2^N) emitted
    // nodes for O(N) source bytes). The fix lowers the body exactly
    // once, wrapping it in a synthetic flag-guarded pretest loop
    // instead: `boolean __do_while_N = true; while (__do_while_N || C)
    // { S; __do_while_N = false; }` -- this test locks in that shape so
    // a future change can't silently reintroduce the clone.
    let m = compile_ok(&wrap("int x = 0; do { x = x + 1; } while (x < 10);"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::ExprStmt {
            expr: Expr::Block(block),
            ..
        } => {
            assert_eq!(
                block.stmts.len(),
                2,
                "expected [flag declaration, Stmt::While]"
            );
            match &block.stmts[0] {
                Stmt::LetStarBinding {
                    name,
                    value: Expr::BoolLit { value: true, .. },
                    ..
                } => {
                    assert!(name.starts_with("__do_while_"));
                }
                other => {
                    panic!("expected LetStarBinding(BoolLit(true)) flag declaration, got {other:?}")
                }
            }
            match &block.stmts[1] {
                Stmt::While {
                    cond: Expr::LogicalOr { .. },
                    body,
                    ..
                } => {
                    // original body statement (`x = x + 1;`) plus the
                    // appended `__do_while_N = false;` -- exactly one
                    // lowered copy of the source body, never two.
                    assert_eq!(body.stmts.len(), 2);
                    assert!(matches!(
                        &body.stmts[1],
                        Stmt::Assign {
                            value: Expr::BoolLit { value: false, .. },
                            ..
                        }
                    ));
                }
                other => panic!("expected While(LogicalOr(...)), got {other:?}"),
            }
        }
        other => panic!("expected ExprStmt(Block), got {other:?}"),
    }
}

#[test]
fn nested_do_while_lowers_without_cloning_the_inner_body() {
    // A direct regression test for the exponential-blowup bug itself
    // (see CHANGELOG under 0.3.0): the pre-fix desugaring cloned the
    // already-lowered body, so each additional nesting level roughly
    // *doubled* the emitted node count -- O(2^N) for N levels. This
    // compares the module's own `Debug`-formatted size (a proxy for
    // total emitted-node count -- deliberately not `main.body.stmts.len()`
    // at the top level, since nested do-while bodies live several
    // `Expr::Block`/`Stmt::While` layers deep and a shallow top-level
    // count would stay constant regardless of what's cloned underneath)
    // at two different nesting depths and asserts *linear*, not
    // exponential, growth -- a check that would fail against the old
    // code (2 -> 6 levels is a 2^4 = 16x blowup there, vs. the expected
    // ~3x for genuinely linear growth) rather than passing vacuously
    // either way. Levels are capped at 6: the pre-existing, unrelated
    // `collect_bounded`/`MAX_TREE_DEPTH` guard (bounding raw CST depth
    // from `program` down, not statement nesting specifically -- see
    // that guard's own doc comment) trips first past that on this
    // particular construct's per-level raw-node footprint, independent
    // of whatever this test is actually trying to measure.
    fn nested_do_while_source(levels: usize) -> String {
        let mut body = "y = y + 1;".to_string();
        for _ in 0..levels {
            body = format!("do {{ {body} }} while (y < 100);");
        }
        wrap(&format!("int y = 0; {body}"))
    }

    let small = compile_ok(&nested_do_while_source(2));
    let large = compile_ok(&nested_do_while_source(6));
    let small_size = format!("{small:?}").len();
    let large_size = format!("{large:?}").len();
    assert!(
        large_size < small_size * 8,
        "module size grew from {small_size} to {large_size} bytes across 2->6 nesting levels of do-while -- looks exponential (~16x expected), not linear (~3x expected)"
    );
}

#[test]
fn a_local_declared_inside_a_do_while_body_does_not_leak_past_it() {
    let m = compile_ok(&wrap(
        "int i = 0; do { int x = i; i = i + 1; } while (i < 3); int y = i;",
    ));
    assert_eq!(main_fn(&m).body.stmts.len(), 3);
}

#[test]
fn do_while_flag_name_does_not_collide_with_a_same_named_user_variable() {
    // Caught by /security-review: an earlier version generated the
    // do-while desugaring's synthetic flag as a bare `__do_while_N`
    // with no collision check against real Java locals already in
    // scope. `__do_while_0` is a legal Java identifier, so a program
    // that happens to declare a variable by that exact name is a real,
    // reachable case -- the old version silently shadowed and corrupted
    // it (through the Python backend, `x` came back `42` instead of the
    // correct `43`) rather than picking a different synthetic name.
    // `do_while_counter` starts at 0, so the very first synthetic name
    // it would try is exactly `__do_while_0` -- this test collides on
    // the first possible attempt, not an edge case reached only after
    // several unrelated do-while statements.
    let m = compile_ok(&wrap(concat!(
        "int __do_while_0 = 1; ",
        "do { __do_while_0 = __do_while_0 + 1; } while (false); ",
        "__do_while_0;"
    )));
    // `__do_while_0` (the user's own variable) must have been mutated by
    // the loop body exactly like any other local -- the fix's own
    // collision check must not have skipped lowering the mutation, and
    // the synthetic flag it generated instead must be a different name.
    match &main_fn(&m).body.stmts[2] {
        Stmt::ExprStmt {
            expr: Expr::VarRef { name, .. },
            ..
        } => assert_eq!(name, "__do_while_0"),
        other => panic!("expected ExprStmt(VarRef(\"__do_while_0\")), got {other:?}"),
    }
}

#[test]
fn do_while_flag_name_does_not_collide_with_a_local_the_body_itself_declares() {
    // Caught by a THIRD round of /security-review, on the fix for the
    // second round's finding above: the collision check there only
    // consulted `lookup_local` (the *ambient* scope active before the
    // body is lowered), which can never see a name the do-while body
    // itself declares -- by the time the check runs, `lower_body`'s own
    // scope for the body has already been pushed *and popped* (the
    // correct, real Java scope boundary). The flag-clear assignment this
    // desugaring appends lives *inside* that body's own top level,
    // though, so a same-named local declared there is exactly the case
    // that reaches it. Under any backend with real block scoping (unlike
    // this crate's own Python execution-proof harness, whose flat
    // function-level scoping doesn't reproduce the bug), the appended
    // flag-clear would resolve to the body's own shadowing local instead
    // of the outer flag, so the outer flag would never actually clear --
    // an infinite loop (`flag || C` staying `true` forever), not just a
    // corrupted value. `do_while_counter` starts at 0 and this test's
    // body declares exactly `__do_while_0`, so the fix must pick
    // `__do_while_1` (or later) for the actual flag; `body_declares_name`
    // is the check added to catch this specific case.
    let m = compile_ok(&wrap(
        "int y = 0; do { boolean __do_while_0 = true; y = y + 1; } while (y < 3); y;",
    ));
    match &main_fn(&m).body.stmts[1] {
        Stmt::ExprStmt {
            expr: Expr::Block(block),
            ..
        } => {
            let flag_name = match &block.stmts[0] {
                Stmt::LetStarBinding { name, .. } => name.clone(),
                other => panic!("expected LetStarBinding flag declaration, got {other:?}"),
            };
            assert_ne!(
                flag_name, "__do_while_0",
                "flag must not reuse the name the body itself declares"
            );
            match &block.stmts[1] {
                Stmt::While { body, .. } => {
                    // The body's own `__do_while_0` declaration survives
                    // completely unchanged -- untouched value, untouched
                    // name -- and the appended flag-clear at the end
                    // targets the *different* synthetic name instead.
                    match &body.stmts[0] {
                        Stmt::LetStarBinding { name, value: Expr::BoolLit { value: true, .. }, .. } => {
                            assert_eq!(name, "__do_while_0");
                        }
                        other => panic!("expected the body's own LetStarBinding(BoolLit(true)) untouched, got {other:?}"),
                    }
                    match body.stmts.last() {
                        Some(Stmt::Assign { name, value: Expr::BoolLit { value: false, .. }, .. }) => {
                            assert_eq!(name, &flag_name);
                        }
                        other => panic!("expected the appended flag-clear Assign to target the flag name, got {other:?}"),
                    }
                }
                other => panic!("expected While, got {other:?}"),
            }
        }
        other => panic!("expected ExprStmt(Block), got {other:?}"),
    }
}

// ── M2b: classic for-loop ────────────────────────────────────────────────

#[test]
fn classic_for_loop_desugars_to_init_then_while() {
    let m = compile_ok(&wrap(
        "int sum = 0; for (int i = 0; i < 5; i++) { sum = sum + i; } sum;",
    ));
    match &main_fn(&m).body.stmts[1] {
        Stmt::ExprStmt {
            expr: Expr::Block(block),
            ..
        } => {
            assert_eq!(block.stmts.len(), 2, "expected [init, Stmt::While]");
            match &block.stmts[0] {
                Stmt::LetStarBinding {
                    name,
                    value: Expr::IntLit { value: 0, .. },
                    ..
                } => {
                    assert_eq!(name, "i");
                }
                other => panic!("expected LetStarBinding(\"i\", IntLit(0)), got {other:?}"),
            }
            match &block.stmts[1] {
                Stmt::While {
                    cond: Expr::BuiltinCall { name, .. },
                    body,
                    ..
                } => {
                    assert_eq!(name, "<");
                    // the source body statement plus the appended update
                    // clause (`i++` desugared to an Assign).
                    assert_eq!(body.stmts.len(), 2);
                    assert!(matches!(&body.stmts[1], Stmt::Assign { name, .. } if name == "i"));
                }
                other => panic!("expected While(BuiltinCall(\"<\")), got {other:?}"),
            }
        }
        other => panic!("expected ExprStmt(Block), got {other:?}"),
    }
}

#[test]
fn classic_for_loop_init_variable_does_not_leak_past_the_loop() {
    let err =
        compile_source(&wrap("for (int i = 0; i < 5; i++) { } int y = i;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn classic_for_loop_without_a_declaration_reuses_an_existing_variable() {
    let m = compile_ok(&wrap("int i = -1; for (i = 0; i < 5; i++) { } i;"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::ExprStmt {
            expr: Expr::Block(block),
            ..
        } => {
            assert!(matches!(&block.stmts[0], Stmt::Assign { name, .. } if name == "i"));
        }
        other => panic!("expected ExprStmt(Block), got {other:?}"),
    }
}

#[test]
fn classic_for_loop_with_all_clauses_empty_is_an_unconditional_loop() {
    let m = compile_ok(&wrap("int c = 0; for (;;) { c = c + 1; c = c + 1; }"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::ExprStmt {
            expr: Expr::Block(block),
            ..
        } => {
            // no init statement -- just the While.
            assert_eq!(block.stmts.len(), 1);
            match &block.stmts[0] {
                Stmt::While {
                    cond: Expr::BoolLit { value: true, .. },
                    ..
                } => {}
                other => panic!("expected While(BoolLit(true)), got {other:?}"),
            }
        }
        other => panic!("expected ExprStmt(Block), got {other:?}"),
    }
}

#[test]
fn classic_for_loop_condition_must_be_boolean() {
    let err = compile_source(&wrap("for (int i = 0; i; i++) { }"), "prog").unwrap_err();
    assert!(
        err.message.contains("boolean"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn classic_for_loop_with_multiple_init_declarators_is_unsupported() {
    let err = compile_source(&wrap("for (int i = 0, j = 0; i < 5; i++) { }"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn classic_for_loop_with_multiple_update_expressions_is_unsupported() {
    let err = compile_source(&wrap("for (int i = 0; i < 5; i++, i++) { }"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn classic_for_loop_update_target_shadowed_by_a_body_local_is_an_error() {
    // Caught by /security-review: `update` is spliced onto the *end* of
    // `body.stmts`, sharing one flat scope with whatever `body` itself
    // already declared at its own top level -- by that point,
    // `lower_body` has already pushed and popped `body`'s own scope, so
    // there was no check that would notice `body` redeclaring the exact
    // name `update` assigns to. Under any backend with real block
    // scoping, that redeclaration would shadow the real loop control
    // variable for the appended update, silently mutating the body's own
    // local instead -- leaving the real loop variable permanently
    // unincremented (an infinite loop), the exact non-termination DoS
    // class `lower_do_while_statement` already needed two rounds of
    // fixes for elsewhere in this same file. Real `javac` rejects this
    // source outright (`variable i is already defined`), so rejecting it
    // here loses no real program's ability to compile.
    let err = compile_source(
        &wrap("int sum = 0; for (int i = 0; i < 3; i++) { int i = 999; sum = sum + 1; }"),
        "prog",
    )
    .unwrap_err();
    assert!(
        err.message.contains("shadowed"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn classic_for_loop_update_target_not_shadowed_by_a_sibling_variable_is_fine() {
    // A body-declared variable with a *different* name from the loop
    // control variable must not trip the collision check above -- this
    // guards against an overly-broad fix that rejects every declaration
    // inside a `for` body, not just an actual name collision.
    let m = compile_ok(&wrap(
        "int sum = 0; for (int i = 0; i < 3; i++) { int j = i * 2; sum = sum + j; } sum;",
    ));
    assert!(!main_fn(&m).body.stmts.is_empty());
}

// ── M2b: enhanced for-loop ───────────────────────────────────────────────

#[test]
fn enhanced_for_loop_lowers_to_stmt_foreach() {
    let m = compile_ok(&wrap(
        "int xs = 0; String s = \"\"; for (String x : xs) { s = x; }",
    ));
    match &main_fn(&m).body.stmts[2] {
        Stmt::ForEach {
            var,
            iter: Expr::VarRef { name, .. },
            body,
            ..
        } => {
            assert_eq!(var, "x");
            assert_eq!(name, "xs");
            assert_eq!(body.stmts.len(), 1);
        }
        other => panic!("expected ForEach, got {other:?}"),
    }
}

#[test]
fn enhanced_for_loop_variable_does_not_leak_past_the_loop() {
    let err = compile_source(
        &wrap("int xs = 0; for (String x : xs) { } String y = x;"),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn enhanced_for_loop_with_var_is_unsupported() {
    let err = compile_source(&wrap("int xs = 0; for (var x : xs) { }"), "prog").unwrap_err();
    assert!(
        err.message.contains("`var`"),
        "unexpected message: {}",
        err.message
    );
}

// ── M2a: switch / break / continue have no SIR IR yet ───────────────────

#[test]
fn switch_statement_is_unsupported() {
    let err =
        compile_source(&wrap("int x = 1; switch (x) { default: break; }"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn break_inside_a_while_loop_is_an_error() {
    let err = compile_source(&wrap("while (true) { break; }"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn continue_inside_a_while_loop_is_an_error() {
    let err = compile_source(&wrap("while (true) { continue; }"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

// ── M3a: method declarations + calls ─────────────────────────────────────

#[test]
fn every_method_becomes_its_own_function_in_the_module() {
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { } \
         static int add(int a, int b) { return a + b; } \
         static int square(int x) { return x * x; }",
    ));
    let mut names: Vec<&str> = m.functions.iter().map(|f| f.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["add", "main", "square"]);
}

#[test]
fn method_params_are_typed_and_declared_in_order() {
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { } \
         static int add(int a, int b) { return a + b; }",
    ));
    let add = find_fn(&m, "add");
    assert_eq!(add.params.len(), 2);
    assert_eq!(add.params[0].name, "a");
    assert_eq!(add.params[1].name, "b");
    assert!(add
        .params
        .iter()
        .all(|p| p.kind == ParamKind::Required && p.default.is_none()));
}

#[test]
fn call_to_a_method_declared_earlier_lowers_to_direct_call() {
    let m = compile_ok(&class_src(
        "static int add(int a, int b) { return a + b; } \
         public static void main(String[] args) { int r = add(1, 2); }",
    ));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding { name, value, .. } => {
            assert_eq!(name, "r");
            match value {
                Expr::DirectCall { fn_name, args, .. } => {
                    assert_eq!(fn_name, "add");
                    assert_eq!(args.len(), 2);
                }
                other => panic!("expected DirectCall, got {other:?}"),
            }
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn call_to_a_method_declared_later_still_resolves_forward_reference() {
    // Pass 1 registers every method's name+signature before any body is
    // lowered, so `main` (declared first) can call `helper` (declared
    // after it) exactly as real Java allows.
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { int r = helper(5); } \
         static int helper(int x) { return x; }",
    ));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::DirectCall { fn_name, .. },
            ..
        } => {
            assert_eq!(fn_name, "helper");
        }
        other => panic!("expected a DirectCall-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn plain_self_recursion_lowers_without_error_and_is_not_mutual_recursion() {
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { } \
         static int loop(int n) { return loop(n); }",
    ));
    assert!(!m.manifest.contains(Feature::MutualRecursion));
}

#[test]
fn mutual_recursion_between_two_methods_sets_the_manifest_feature() {
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { } \
         static boolean isEven(int n) { return isOdd(n); } \
         static boolean isOdd(int n) { return isEven(n); }",
    ));
    assert!(m.manifest.contains(Feature::MutualRecursion));
}

#[test]
fn mutual_recursion_through_a_three_method_cycle_sets_the_manifest_feature() {
    // a -> b -> c -> a: a cycle of length 3, not directly adjacent pairs
    // calling each other -- exercises `has_mutual_recursion`'s DFS
    // finding a back edge several frames deep, not just a 2-node cycle.
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { } \
         static int a(int n) { return b(n); } \
         static int b(int n) { return c(n); } \
         static int c(int n) { return a(n); }",
    ));
    assert!(m.manifest.contains(Feature::MutualRecursion));
}

#[test]
fn independent_self_recursive_methods_with_no_cross_calls_are_not_mutual_recursion() {
    // Two unrelated self-recursive methods (no edge between them at all)
    // must not be flagged -- each is its own singleton DFS component
    // with only a self-loop, which `has_mutual_recursion` explicitly
    // skips.
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { } \
         static int f(int n) { return f(n); } \
         static int g(int n) { return g(n); }",
    ));
    assert!(!m.manifest.contains(Feature::MutualRecursion));
}

#[test]
fn a_call_chain_with_no_cycle_is_not_mutual_recursion() {
    // a -> b -> c, no edge back to a or b anywhere -- a plain DAG, not a
    // cycle -- must not be flagged.
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { } \
         static int a(int n) { return b(n); } \
         static int b(int n) { return c(n); } \
         static int c(int n) { return n; }",
    ));
    assert!(!m.manifest.contains(Feature::MutualRecursion));
}

#[test]
fn void_method_call_as_a_bare_statement_lowers_to_expr_stmt() {
    let m = compile_ok(&class_src(
        "static void noop(int x) { } \
         public static void main(String[] args) { noop(1); }",
    ));
    match &main_fn(&m).body.stmts[0] {
        Stmt::ExprStmt {
            expr: Expr::DirectCall { fn_name, .. },
            ..
        } => {
            assert_eq!(fn_name, "noop");
        }
        other => panic!("expected an ExprStmt-wrapped DirectCall, got {other:?}"),
    }
}

#[test]
fn return_expression_as_the_last_statement_becomes_the_block_value() {
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { } \
         static int square(int x) { return x * x; }",
    ));
    let square = find_fn(&m, "square");
    assert!(square.body.stmts.is_empty());
    assert!(!matches!(square.body.value, Expr::NilLit { .. }));
}

#[test]
fn void_method_with_bare_return_has_a_nil_block_value() {
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { } \
         static void greet() { return; }",
    ));
    let greet = find_fn(&m, "greet");
    assert!(matches!(greet.body.value, Expr::NilLit { .. }));
}

#[test]
fn void_method_falling_off_the_end_has_a_nil_block_value() {
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { } \
         static void greet() { int x = 1; }",
    ));
    let greet = find_fn(&m, "greet");
    assert_eq!(greet.body.stmts.len(), 1);
    assert!(matches!(greet.body.value, Expr::NilLit { .. }));
}

#[test]
fn call_to_unknown_method_is_an_error() {
    let err = compile_source(&wrap("mystery(1);"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn call_with_wrong_argument_count_is_an_error() {
    let err = compile_source(
        &class_src(
            "static int add(int a, int b) { return a + b; } \
             public static void main(String[] args) { add(1); }",
        ),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn call_with_wrong_argument_kind_is_an_error() {
    let err = compile_source(
        &class_src(
            "static int add(int a, int b) { return a + b; } \
             public static void main(String[] args) { add(true, 2); }",
        ),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn duplicate_method_name_is_an_error() {
    let err = compile_source(
        &class_src(
            "public static void main(String[] args) { } \
             static int add(int a, int b) { return a + b; } \
             static int add(int a) { return a; }",
        ),
        "prog",
    )
    .unwrap_err();
    assert!(
        err.message.contains("duplicate"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn return_not_as_the_last_statement_is_an_error() {
    let err = compile_source(
        &class_src(
            "public static void main(String[] args) { } \
             static int f() { return 1; int y = 2; }",
        ),
        "prog",
    )
    .unwrap_err();
    assert!(
        err.message.contains("return"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn return_nested_inside_an_if_body_is_rejected_not_mis_lowered() {
    // The literal-last-top-level-statement rule means a branched/early
    // return (a `return` inside an `if`'s own body) is not recognized as
    // the method-body-level return at all -- it falls through to
    // `lower_statement`'s ordinary "unsupported statement kind"
    // rejection, since `return_statement` is not one of that
    // dispatcher's alternatives. A real deferred limitation, not a bug:
    // see this crate's own module doc comment.
    let err = compile_source(
        &class_src(
            "public static void main(String[] args) { } \
             static int f(int x) { if (x > 0) { return 1; } return 2; }",
        ),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn return_expression_kind_must_match_the_declared_return_type() {
    let err = compile_source(
        &class_src(
            "public static void main(String[] args) { } \
             static int f() { return true; }",
        ),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn return_with_expression_in_a_void_method_is_an_error() {
    let err = compile_source(
        &class_src(
            "public static void main(String[] args) { } \
             static void f() { return 1; }",
        ),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn bare_return_in_a_non_void_method_is_an_error() {
    let err = compile_source(
        &class_src(
            "public static void main(String[] args) { } \
             static int f() { return; }",
        ),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn array_parameter_type_is_now_supported_since_m4a() {
    // `kind_of_type_node` (shared by method-parameter and local-
    // declaration type resolution) gained array-type support in M4a; an
    // array-typed method parameter now lowers cleanly as a natural
    // consequence, not something M4a specifically built -- this is the
    // positive-test replacement for M3a's own `array_parameter_type_is_
    // still_unsupported`, which this milestone's own scope change made
    // stale.
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { } \
         static void f(int[] x) { }",
    ));
    let f = find_fn(&m, "f");
    assert_eq!(f.params.len(), 1);
    assert_eq!(f.params[0].name, "x");
}

#[test]
fn c_style_array_parameter_bracket_is_unsupported() {
    let err = compile_source(
        &class_src(
            "public static void main(String[] args) { } \
             static void f(int x[]) { }",
        ),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn varargs_parameter_is_unsupported() {
    let err = compile_source(
        &class_src(
            "public static void main(String[] args) { } \
             static void f(int... xs) { }",
        ),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn field_declaration_in_class_body_is_an_error() {
    let err = compile_source(
        &class_src("int x; public static void main(String[] args) { }"),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn qualified_call_remains_unsupported() {
    let err = compile_source(
        &class_src(
            "static int add(int a, int b) { return a + b; } \
             public static void main(String[] args) { Main.add(1, 2); }",
        ),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

// ── M3b: lambda expressions ──────────────────────────────────────────────

fn make_closure_fn_name(value: &Expr) -> &str {
    match value {
        Expr::MakeClosure { fn_name, .. } => fn_name,
        other => panic!("expected MakeClosure, got {other:?}"),
    }
}

#[test]
fn lambda_with_expression_body_lowers_to_make_closure() {
    let m = compile_ok(&wrap("var f = (int x) -> x + 1;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding { name, value, .. } => {
            assert_eq!(name, "f");
            let lambda = find_fn(&m, make_closure_fn_name(value));
            assert_eq!(lambda.params.len(), 1);
            assert_eq!(lambda.params[0].name, "x");
            assert!(lambda.captures.is_empty());
            assert!(!matches!(lambda.body.value, Expr::NilLit { .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn zero_parameter_lambda_lowers_correctly() {
    let m = compile_ok(&wrap("var f = () -> 42;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            let lambda = find_fn(&m, make_closure_fn_name(value));
            assert!(lambda.params.is_empty());
            assert!(matches!(lambda.body.value, Expr::IntLit { value: 42, .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn multi_parameter_typed_lambda_preserves_parameter_order() {
    let m = compile_ok(&wrap("var f = (int a, int b) -> a + b;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            let lambda = find_fn(&m, make_closure_fn_name(value));
            let names: Vec<&str> = lambda.params.iter().map(|p| p.name.as_str()).collect();
            assert_eq!(names, ["a", "b"]);
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn lambda_captures_an_enclosing_local() {
    let m = compile_ok(&wrap("int n = 10; var f = (int x) -> x + n;"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::MakeClosure {
                fn_name, captures, ..
            } => {
                assert_eq!(captures.len(), 1);
                assert_eq!(captures[0].name, "n");
                match &captures[0].value {
                    Expr::VarRef { name, scope, .. } => {
                        assert_eq!(name, "n");
                        assert_eq!(*scope, Scope::Local);
                    }
                    other => panic!("expected VarRef, got {other:?}"),
                }
                let lambda = find_fn(&m, fn_name);
                assert_eq!(lambda.captures.len(), 1);
                assert_eq!(lambda.captures[0].name, "n");
            }
            other => panic!("expected MakeClosure, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn lambda_capturing_a_method_parameter_uses_param_scope() {
    // Proves `resolve_name` correctly threads a `Scope::Param`-declared
    // enclosing name (not just `Scope::Local`) through a capture.
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { } \
         static int adder(int n) { var f = (int x) -> x + n; return n; }",
    ));
    let adder = find_fn(&m, "adder");
    match &adder.body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::MakeClosure { captures, .. },
            ..
        } => {
            assert_eq!(captures[0].name, "n");
            match &captures[0].value {
                Expr::VarRef { scope, .. } => assert_eq!(*scope, Scope::Param),
                other => panic!("expected VarRef, got {other:?}"),
            }
        }
        other => panic!("expected a MakeClosure-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn nested_lambda_captures_transitively_across_both_boundaries() {
    let m = compile_ok(&wrap(
        "int n = 10; var f = (int x) -> (int y) -> x + y + n;",
    ));
    match &main_fn(&m).body.stmts[1] {
        Stmt::LetStarBinding {
            value:
                Expr::MakeClosure {
                    fn_name: outer_name,
                    captures: outer_captures,
                    ..
                },
            ..
        } => {
            // Outer lambda captures `n` from `main`, as an ordinary Local.
            assert_eq!(outer_captures.len(), 1);
            assert_eq!(outer_captures[0].name, "n");
            assert_eq!(
                match &outer_captures[0].value {
                    Expr::VarRef { scope, .. } => *scope,
                    other => panic!("expected VarRef, got {other:?}"),
                },
                Scope::Local
            );
            let outer_lambda = find_fn(&m, outer_name);
            // Outer lambda's own body is itself a MakeClosure (the inner
            // lambda), which must capture BOTH `x` (the outer lambda's
            // own param) and `n` (re-threaded through as the outer
            // lambda's own capture) -- crossing two boundaries.
            match &outer_lambda.body.value {
                Expr::MakeClosure {
                    fn_name: inner_name,
                    captures: inner_captures,
                    ..
                } => {
                    let mut names: Vec<&str> =
                        inner_captures.iter().map(|c| c.name.as_str()).collect();
                    names.sort_unstable();
                    assert_eq!(names, ["n", "x"]);
                    let x_capture = inner_captures.iter().find(|c| c.name == "x").unwrap();
                    assert_eq!(
                        match &x_capture.value {
                            Expr::VarRef { scope, .. } => *scope,
                            other => panic!("expected VarRef, got {other:?}"),
                        },
                        Scope::Param,
                        "x should be read from the outer lambda's own param scope"
                    );
                    let n_capture = inner_captures.iter().find(|c| c.name == "n").unwrap();
                    assert_eq!(
                        match &n_capture.value {
                            Expr::VarRef { scope, .. } => *scope,
                            other => panic!("expected VarRef, got {other:?}"),
                        },
                        Scope::Capture,
                        "n should be re-threaded through the outer lambda's own capture, not read from main directly"
                    );
                    let inner_lambda = find_fn(&m, inner_name);
                    assert_eq!(inner_lambda.captures.len(), 2);
                }
                other => {
                    panic!("expected the outer lambda's body to be a MakeClosure, got {other:?}")
                }
            }
        }
        other => panic!("expected a MakeClosure-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn block_bodied_lambda_with_tail_return_lowers_correctly() {
    let m = compile_ok(&wrap("var f = (int x) -> { int y = x * 2; return y; };"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            let lambda = find_fn(&m, make_closure_fn_name(value));
            assert_eq!(lambda.body.stmts.len(), 1);
            assert!(!matches!(lambda.body.value, Expr::NilLit { .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn block_bodied_lambda_with_bare_return_has_a_nil_block_value() {
    let m = compile_ok(&wrap("int n = 0; var f = (int x) -> { return; };"));
    let value = match &main_fn(&m).body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => value,
        other => panic!("expected LetStarBinding, got {other:?}"),
    };
    let lambda = find_fn(&m, make_closure_fn_name(value));
    assert!(matches!(lambda.body.value, Expr::NilLit { .. }));
}

#[test]
fn block_bodied_lambda_falling_off_the_end_has_a_nil_block_value() {
    // A legal "statement lambda" shape (e.g. `Runnable`-like) -- no
    // `return` at all.
    let m = compile_ok(&wrap("var f = (int x) -> { int y = x + 1; };"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            let lambda = find_fn(&m, make_closure_fn_name(value));
            assert_eq!(lambda.body.stmts.len(), 1);
            assert!(matches!(lambda.body.value, Expr::NilLit { .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn lambda_as_a_bare_statement_lowers_to_expr_stmt() {
    let m = compile_ok(&wrap("(int x) -> x;"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::ExprStmt {
            expr: Expr::MakeClosure { .. },
            ..
        } => {}
        other => panic!("expected an ExprStmt-wrapped MakeClosure, got {other:?}"),
    }
}

#[test]
fn feature_closures_is_declared_when_a_lambda_is_lowered() {
    let m = compile_ok(&wrap("var f = (int x) -> x;"));
    assert!(m.manifest.contains(Feature::Closures));
}

#[test]
fn synthesized_lambda_name_does_not_collide_with_a_real_method_named_lambda_0() {
    // `__lambda_0` is a legal Java identifier, so a class declaring a
    // real method by that exact name -- and containing a lambda, which
    // would otherwise be the very first one synthesized as `__lambda_0`
    // too -- is a real, reachable case. Regression test for a finding
    // from `/security-review`: the synthetic name must be checked
    // against every real method name before being committed to, the
    // same discipline `lower_do_while_statement`'s own `__do_while_N`
    // flag-name collision probe already uses.
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { var f = (int x) -> x + 1; } \
         static int __lambda_0() { return 42; }",
    ));
    let mut names: Vec<&str> = m.functions.iter().map(|f| f.name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(
        m.functions.len(),
        names.len(),
        "expected every function name to be unique, got {:?}",
        m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    // The real method must still be named exactly `__lambda_0`, and the
    // synthesized closure must have been forced to a different name.
    assert!(m.functions.iter().any(|f| f.name == "__lambda_0"
        && f.params.is_empty()
        && matches!(f.body.value, Expr::IntLit { value: 42, .. })));
}

#[test]
fn assignment_to_a_captured_variable_is_an_error() {
    let err = compile_source(
        &wrap("int n = 0; var f = (int x) -> { n = n + x; return n; };"),
        "prog",
    )
    .unwrap_err();
    assert!(
        err.message.contains("effectively final"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn increment_of_a_captured_variable_is_an_error() {
    let err = compile_source(
        &wrap("int n = 0; var f = (int x) -> { n++; return n; };"),
        "prog",
    )
    .unwrap_err();
    assert!(
        err.message.contains("effectively final"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn bare_unparenthesized_lambda_parameter_is_unsupported() {
    let err = compile_source(&wrap("var f = x -> x + 1;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn untyped_parenthesized_lambda_parameter_is_unsupported() {
    let err = compile_source(&wrap("var f = (x) -> x + 1;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn var_lambda_parameter_is_unsupported() {
    let err = compile_source(&wrap("var f = (var x) -> x + 1;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn return_not_as_the_last_statement_in_a_lambda_body_is_an_error() {
    let err = compile_source(
        &wrap("var f = (int x) -> { return x; int y = 1; };"),
        "prog",
    )
    .unwrap_err();
    assert!(
        err.message.contains("return"),
        "unexpected message: {}",
        err.message
    );
}

// ── IndirectCall: invoking a lambda-valued local (task #54) ─────────────

#[test]
fn calling_a_lambda_valued_local_lowers_to_indirect_call() {
    let m = compile_ok(&wrap("var f = (int x) -> x + 1; int y = f(5);"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::LetStarBinding {
            value: Expr::IndirectCall { target, args, .. },
            ..
        } => {
            assert!(matches!(**target, Expr::VarRef { .. }));
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0], Expr::IntLit { value: 5, .. }));
        }
        other => panic!("expected an IndirectCall-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn calling_a_lambda_valued_local_result_kind_is_the_lambda_s_own_return_kind() {
    // The call's own result must be usable in an `int`-typed position --
    // proves the `Kind::Closure` interned signature correctly reports
    // the lambda's own return kind, not some placeholder.
    let m = compile_ok(&wrap(
        "var f = (int x) -> x + 1; int y = f(5); int z = y + 1;",
    ));
    assert_eq!(main_fn(&m).body.stmts.len(), 3);
}

#[test]
fn zero_argument_lambda_call_lowers_correctly() {
    let m = compile_ok(&wrap("var f = () -> 42; int y = f();"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::LetStarBinding {
            value: Expr::IndirectCall { args, .. },
            ..
        } => assert!(args.is_empty()),
        other => panic!("expected an IndirectCall-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn multi_argument_lambda_call_preserves_argument_order() {
    let m = compile_ok(&wrap("var f = (int a, int b) -> a - b; int y = f(10, 3);"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::LetStarBinding {
            value: Expr::IndirectCall { args, .. },
            ..
        } => {
            assert!(matches!(args[0], Expr::IntLit { value: 10, .. }));
            assert!(matches!(args[1], Expr::IntLit { value: 3, .. }));
        }
        other => panic!("expected an IndirectCall-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn calling_a_lambda_with_the_wrong_argument_count_is_an_error() {
    let err = compile_source(&wrap("var f = (int x) -> x + 1; f();"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn calling_a_lambda_with_the_wrong_argument_kind_is_an_error() {
    let err = compile_source(&wrap("var f = (int x) -> x + 1; f(true);"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn calling_a_non_closure_local_is_an_error() {
    let err = compile_source(&wrap("int x = 1; x();"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn calling_a_lambda_local_inside_a_non_main_method_lowers_correctly() {
    // Confirms this isn't somehow special-cased to `main`'s own body --
    // an ordinary method declaring and then invoking its own local
    // lambda, ending in a tail-position `return` of the call's result.
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { } \
         static int apply(int x) { var f = (int y) -> y * 2; return f(x); }",
    ));
    let apply = find_fn(&m, "apply");
    match &apply.body.value {
        Expr::IndirectCall { .. } => {}
        other => panic!("expected apply's body to end in an IndirectCall, got {other:?}"),
    }
}

#[test]
fn calling_a_captured_lambda_from_within_a_nested_lambda_lowers_correctly() {
    // The captured closure's own signature must still be recoverable
    // through `resolve_name`'s capture-threading, not just for a bare,
    // uncaptured local.
    let m = compile_ok(&wrap(
        "var f = (int x) -> x + 1; var g = (int y) -> f(y); int z = g(3);",
    ));
    assert_eq!(main_fn(&m).body.stmts.len(), 3);
}

#[test]
fn feature_closures_is_declared_when_a_lambda_is_invoked() {
    let m = compile_ok(&wrap("var f = (int x) -> x + 1; f(1);"));
    assert!(m.manifest.contains(Feature::Closures));
}

#[test]
fn reassigning_a_lambda_valued_local_to_a_different_signature_is_rejected() {
    // Caught by `/security-review`: without this rejection, a later call
    // site would type-check `f(...)` against `f`'s *original* signature,
    // not the closure it was actually reassigned to (`Kind::Closure`'s
    // own interned-signature index goes stale on reassignment, since
    // this crate only tracks a local's `Kind` at declaration time).
    let err = compile_source(
        &wrap("var f = (int x) -> x + 1; var g = () -> 42; f = g; int z = f(5);"),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn reassigning_a_lambda_valued_local_to_a_non_lambda_value_is_rejected() {
    let err = compile_source(&wrap("var f = (int x) -> x + 1; f = 5;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

// ── M4a: array declarations, indexing reads, .length ─────────────────────

#[test]
fn array_literal_declaration_lowers_to_seqlit() {
    let m = compile_ok(&wrap("int[] xs = {1, 2, 3};"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            name,
            value: Expr::SeqLit { items, .. },
            ..
        } => {
            assert_eq!(name, "xs");
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0], Expr::IntLit { value: 1, .. }));
            assert!(matches!(items[1], Expr::IntLit { value: 2, .. }));
            assert!(matches!(items[2], Expr::IntLit { value: 3, .. }));
        }
        other => panic!("expected a SeqLit-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn empty_array_literal_with_explicit_type_lowers_correctly() {
    let m = compile_ok(&wrap("int[] xs = {};"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::SeqLit { items, .. },
            ..
        } => assert!(items.is_empty()),
        other => panic!("expected a SeqLit-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn var_infers_array_kind_from_a_non_empty_literal() {
    let m = compile_ok(&wrap("var xs = {1, 2, 3}; int y = xs[0];"));
    assert!(matches!(
        &main_fn(&m).body.stmts[0],
        Stmt::LetStarBinding {
            value: Expr::SeqLit { .. },
            ..
        }
    ));
}

#[test]
fn empty_array_literal_with_var_cannot_infer_element_type() {
    let err = compile_source(&wrap("var xs = {};"), "prog").unwrap_err();
    assert!(
        err.message.contains("infer"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn array_element_kind_mismatch_is_an_error() {
    let err = compile_source(&wrap("int[] xs = {1, true, 3};"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn array_initializer_on_a_non_array_declared_type_is_an_error() {
    let err = compile_source(&wrap("int x = {1, 2, 3};"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn string_array_literal_lowers_correctly() {
    let m = compile_ok(&wrap("String[] names = {\"a\", \"b\"};"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::SeqLit { items, .. },
            ..
        } => assert_eq!(items.len(), 2),
        other => panic!("expected a SeqLit-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn multi_dimensional_array_type_is_now_supported_since_m4d() {
    // `kind_of_type_node` gained multi-dimensional array-type support in
    // M4d; an empty 2-D array literal (`{}`, zero outer elements) now
    // lowers cleanly as a natural consequence of that shared function --
    // this is the positive-test replacement for M4a's own
    // `multi_dimensional_array_type_is_unsupported`, which this
    // milestone's own scope change made stale.
    let m = compile_ok(&wrap("int[][] grid = {};"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::SeqLit { items, .. },
            ..
        } => assert!(items.is_empty()),
        other => panic!("expected a SeqLit-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn nested_array_initializer_is_unsupported() {
    let err = compile_source(&wrap("int[] xs = {{1, 2}};"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn array_index_read_lowers_to_seqindex() {
    let m = compile_ok(&wrap("int[] xs = {1, 2, 3}; int y = xs[0];"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::LetStarBinding {
            value: Expr::SeqIndex { seq, index, .. },
            ..
        } => {
            assert!(matches!(**seq, Expr::VarRef { .. }));
            assert!(matches!(**index, Expr::IntLit { value: 0, .. }));
        }
        other => panic!("expected a SeqIndex-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn array_index_read_kind_is_the_element_kind() {
    // A `String[]`'s own indexed reads must be usable in a String-typed
    // position (`String s = names[0];`) -- proves `Expr::SeqIndex`'s own
    // result kind is correctly the *element* kind, not the array kind.
    let m = compile_ok(&wrap(
        "String[] names = {\"a\", \"b\"}; String s = names[0];",
    ));
    assert_eq!(main_fn(&m).body.stmts.len(), 2);
}

#[test]
fn indexing_a_non_array_value_is_an_error() {
    let err = compile_source(&wrap("int x = 1; int y = x[0];"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn array_index_must_be_an_int() {
    let err = compile_source(&wrap("int[] xs = {1}; int y = xs[true];"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn array_length_lowers_to_seqlen() {
    let m = compile_ok(&wrap("int[] xs = {1, 2, 3}; int n = xs.length;"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::LetStarBinding {
            value: Expr::SeqLen { seq, .. },
            ..
        } => assert!(matches!(**seq, Expr::VarRef { .. })),
        other => panic!("expected a SeqLen-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn length_on_a_non_array_value_is_an_error() {
    let err = compile_source(&wrap("int x = 1; int y = x.length;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn field_access_other_than_length_remains_unsupported() {
    let err =
        compile_source(&wrap("int[] xs = {1}; int y = xs.somethingElse;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn array_typed_method_parameter_is_declared_correctly() {
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { } \
         static int first(int[] xs) { return xs[0]; }",
    ));
    let first = find_fn(&m, "first");
    assert_eq!(first.params.len(), 1);
    assert_eq!(first.params[0].name, "xs");
    assert!(matches!(first.body.value, Expr::SeqIndex { .. }));
}

#[test]
fn array_call_argument_and_return_kind_check() {
    let m = compile_ok(&class_src(
        "public static void main(String[] args) { int[] xs = {1, 2, 3}; int[] ys = copyOf(xs); } \
         static int[] copyOf(int[] xs) { return xs; }",
    ));
    match &main_fn(&m).body.stmts[1] {
        Stmt::LetStarBinding {
            value: Expr::DirectCall { fn_name, .. },
            ..
        } => assert_eq!(fn_name, "copyOf"),
        other => panic!("expected a DirectCall-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn array_call_argument_kind_mismatch_is_an_error() {
    let err = compile_source(
        &class_src(
            "public static void main(String[] args) { int[] xs = {1}; useStrings(xs); } \
             static void useStrings(String[] ss) { }",
        ),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn feature_sequences_is_declared_when_an_array_is_lowered() {
    let m = compile_ok(&wrap(
        "int[] xs = {1, 2, 3}; int y = xs[0]; int n = xs.length;",
    ));
    assert!(m.manifest.contains(Feature::Sequences));
}

#[test]
fn c_style_array_declarator_still_rejected() {
    // `int xs[] = {1, 2, 3};` -- the C-style declarator-suffix form
    // remains out of scope regardless of M4a (only the `int[] xs`
    // type-prefix form is supported) -- unchanged from M1's own
    // pre-existing rejection, re-verified here since M4a touched the
    // surrounding code.
    let err = compile_source(&wrap("int xs[] = {1, 2, 3};"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

// ── M4b: indexed array assignment ───────────────────────────────────────

#[test]
fn indexed_assignment_lowers_to_seqset() {
    let m = compile_ok(&wrap("int[] xs = {1, 2, 3}; xs[0] = 5;"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::SeqSet {
            seq, index, value, ..
        } => {
            assert!(matches!(seq, Expr::VarRef { .. }));
            assert!(matches!(index, Expr::IntLit { value: 0, .. }));
            assert!(matches!(value, Expr::IntLit { value: 5, .. }));
        }
        other => panic!("expected Stmt::SeqSet, got {other:?}"),
    }
}

#[test]
fn indexed_assignment_with_a_non_constant_index_lowers_correctly() {
    let m = compile_ok(&wrap("int[] xs = {1, 2, 3}; int i = 1; xs[i] = 9;"));
    assert!(matches!(&main_fn(&m).body.stmts[2], Stmt::SeqSet { .. }));
}

#[test]
fn indexed_assignment_in_a_classic_for_loop_update_clause_lowers_correctly() {
    // `for_update` reuses this same expression-statement desugaring path
    // (see M2b's own doc comment) -- proves indexed assignment works
    // there too, not just as a standalone statement.
    let m = compile_ok(&wrap(
        "int[] xs = {0, 0, 0}; for (int i = 0; i < 3; xs[i] = i) { i = i + 1; }",
    ));
    assert_eq!(main_fn(&m).body.stmts.len(), 2);
}

#[test]
fn indexed_assignment_on_a_string_array_lowers_correctly() {
    let m = compile_ok(&wrap("String[] names = {\"a\", \"b\"}; names[0] = \"c\";"));
    assert!(matches!(&main_fn(&m).body.stmts[1], Stmt::SeqSet { .. }));
}

#[test]
fn feature_sequences_is_declared_when_indexed_assignment_is_lowered() {
    let m = compile_ok(&wrap("int[] xs = {1}; xs[0] = 2;"));
    assert!(m.manifest.contains(Feature::Sequences));
}

#[test]
fn indexed_assignment_on_a_non_array_value_is_an_error() {
    let err = compile_source(&wrap("int x = 1; x[0] = 5;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn plain_assignment_to_a_simple_local_still_works_alongside_indexed_assignment() {
    // Regression check: adding `indexed_assign_target`'s new detour ahead
    // of `extract_bare_name` in `lower_expr_statement` must not disturb
    // the ordinary bare-name assignment path it still falls through to.
    let m = compile_ok(&wrap("int x = 1; x = 2;"));
    assert!(matches!(main_fn(&m).body.stmts[1], Stmt::Assign { .. }));
}

#[test]
fn indexed_assignment_target_index_must_be_an_int() {
    let err = compile_source(&wrap("int[] xs = {1, 2, 3}; xs[true] = 5;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn indexed_assignment_value_kind_must_match_element_kind() {
    let err = compile_source(&wrap("int[] xs = {1, 2, 3}; xs[0] = \"nope\";"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn field_target_assignment_remains_unsupported() {
    // `extract_bare_name`'s own remaining rejection surface, re-verified
    // after M4b's `indexed_assign_target` detour was inserted ahead of
    // it: `xs.length = 5;` still isn't a valid assignment target -- it's
    // both syntactically not a simple name and not an indexed target.
    let err = compile_source(&wrap("int[] xs = {1}; xs.length = 5;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn compound_assignment_on_an_indexed_target_is_still_deferred() {
    let err = compile_source(&wrap("int[] xs = {1, 2, 3}; xs[0] += 1;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn increment_of_an_indexed_target_is_still_deferred() {
    let err = compile_source(&wrap("int[] xs = {1, 2, 3}; xs[0]++;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

// ── M4c: new-based array-creation expressions ───────────────────────────

#[test]
fn new_sized_int_array_lowers_to_zero_filled_seqlit() {
    let m = compile_ok(&wrap("int[] xs = new int[3];"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::SeqLit { items, .. },
            ..
        } => {
            assert_eq!(items.len(), 3);
            for item in items {
                assert!(matches!(item, Expr::IntLit { value: 0, .. }));
            }
        }
        other => panic!("expected a SeqLit-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn new_sized_float_array_fills_with_zero_point_zero() {
    let m = compile_ok(&wrap("double[] xs = new double[2];"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::SeqLit { items, .. },
            ..
        } => {
            assert_eq!(items.len(), 2);
            for item in items {
                assert!(matches!(item, Expr::FloatLit { value, .. } if *value == 0.0));
            }
        }
        other => panic!("expected a SeqLit-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn new_sized_boolean_array_fills_with_false() {
    let m = compile_ok(&wrap("boolean[] flags = new boolean[2];"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::SeqLit { items, .. },
            ..
        } => {
            assert_eq!(items.len(), 2);
            for item in items {
                assert!(matches!(item, Expr::BoolLit { value: false, .. }));
            }
        }
        other => panic!("expected a SeqLit-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn new_sized_array_of_zero_length_lowers_to_an_empty_seqlit() {
    let m = compile_ok(&wrap("int[] xs = new int[0];"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::SeqLit { items, .. },
            ..
        } => assert!(items.is_empty()),
        other => panic!("expected a SeqLit-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn new_sized_array_used_alongside_a_real_index_write_loop() {
    // The realistic pattern M4b (indexed assignment) and M4c (sized
    // creation) together exist to enable: allocate, then fill by index.
    let m = compile_ok(&wrap(
        "int[] xs = new int[3]; for (int i = 0; i < 3; i++) { xs[i] = i; }",
    ));
    assert_eq!(main_fn(&m).body.stmts.len(), 2);
}

#[test]
fn new_sized_array_with_negative_size_is_an_error() {
    let err = compile_source(&wrap("int[] xs = new int[-1];"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn new_sized_array_exceeding_the_size_cap_is_an_error() {
    let err = compile_source(&wrap("int[] xs = new int[100000];"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn new_sized_array_with_a_non_constant_size_is_deferred() {
    let err = compile_source(&wrap("int n = 3; int[] xs = new int[n];"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn new_sized_array_of_strings_is_deferred() {
    let err = compile_source(&wrap("String[] xs = new String[3];"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn new_sized_multi_dimensional_array_is_deferred() {
    let err = compile_source(&wrap("int[][] grid = new int[2][3];"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn new_array_with_initializer_lowers_to_seqlit() {
    let m = compile_ok(&wrap("int[] xs = new int[]{1, 2, 3};"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::SeqLit { items, .. },
            ..
        } => assert_eq!(items.len(), 3),
        other => panic!("expected a SeqLit-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn new_array_with_initializer_of_strings_lowers_correctly() {
    let m = compile_ok(&wrap("String[] xs = new String[]{\"a\", \"b\"};"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::SeqLit { items, .. },
            ..
        } => assert_eq!(items.len(), 2),
        other => panic!("expected a SeqLit-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn new_array_with_initializer_element_kind_mismatch_is_an_error() {
    let err = compile_source(&wrap("int[] xs = new int[]{1, true, 3};"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn new_array_with_empty_initializer_lowers_to_empty_seqlit() {
    let m = compile_ok(&wrap("int[] xs = new int[]{};"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::SeqLit { items, .. },
            ..
        } => assert!(items.is_empty()),
        other => panic!("expected a SeqLit-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn new_multi_dimensional_array_with_initializer_is_deferred() {
    let err = compile_source(&wrap("int[][] grid = new int[][]{{1, 2}};"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn feature_sequences_is_declared_when_a_sized_array_is_created() {
    let m = compile_ok(&wrap("int[] xs = new int[2];"));
    assert!(m.manifest.contains(Feature::Sequences));
}

#[test]
fn new_object_construction_remains_unsupported() {
    // `new ClassName(...)` -- a different `primary` alternative entirely
    // (its second child is `class_type`, not `array_creation_type`), and
    // must remain rejected exactly as before M4c's own new match arms
    // were added.
    let err = compile_source(
        &class_src("public static void main(String[] args) { Object o = new Object(); }"),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

// ── M4d: multi-dimensional arrays ───────────────────────────────────────

#[test]
fn two_dimensional_array_literal_lowers_to_nested_seqlit() {
    let m = compile_ok(&wrap("int[][] grid = {{1, 2}, {3, 4}};"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::SeqLit { items, .. },
            ..
        } => {
            assert_eq!(items.len(), 2);
            for row in items {
                assert!(matches!(row, Expr::SeqLit { .. }));
            }
            match &items[0] {
                Expr::SeqLit { items: row0, .. } => {
                    assert!(matches!(row0[0], Expr::IntLit { value: 1, .. }));
                    assert!(matches!(row0[1], Expr::IntLit { value: 2, .. }));
                }
                other => panic!("expected a nested SeqLit, got {other:?}"),
            }
        }
        other => panic!("expected a SeqLit-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn three_dimensional_array_literal_lowers_correctly() {
    let m = compile_ok(&wrap("int[][][] cube = {{{1}}};"));
    assert!(matches!(
        &main_fn(&m).body.stmts[0],
        Stmt::LetStarBinding {
            value: Expr::SeqLit { .. },
            ..
        }
    ));
}

#[test]
fn ragged_two_dimensional_array_literal_lowers_correctly() {
    // Java arrays are genuinely ragged (each inner array is its own
    // independent object) -- rows of differing length are legal.
    let m = compile_ok(&wrap("int[][] grid = {{1, 2, 3}, {4}};"));
    match &main_fn(&m).body.stmts[0] {
        Stmt::LetStarBinding {
            value: Expr::SeqLit { items, .. },
            ..
        } => {
            let (Expr::SeqLit { items: row0, .. }, Expr::SeqLit { items: row1, .. }) =
                (&items[0], &items[1])
            else {
                panic!("expected two nested SeqLit rows");
            };
            assert_eq!(row0.len(), 3);
            assert_eq!(row1.len(), 1);
        }
        other => panic!("expected a SeqLit-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn two_dimensional_string_array_literal_lowers_correctly() {
    let m = compile_ok(&wrap("String[][] grid = {{\"a\", \"b\"}};"));
    assert!(matches!(
        &main_fn(&m).body.stmts[0],
        Stmt::LetStarBinding {
            value: Expr::SeqLit { .. },
            ..
        }
    ));
}

#[test]
fn two_dimensional_array_element_kind_mismatch_is_an_error() {
    let err = compile_source(&wrap("int[][] grid = {{1, 2}, {true}};"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn scalar_element_where_a_nested_array_is_expected_is_an_error() {
    let err = compile_source(&wrap("int[][] grid = {1, 2};"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn var_inferred_two_dimensional_array_literal_remains_deferred() {
    // `var`-inferred multi-dimensional array literals are deferred this
    // milestone (see `lower_array_initializer`'s own doc comment) --
    // only an explicitly-typed declared array type infers nested dims.
    let err = compile_source(&wrap("var grid = {{1, 2}, {3, 4}};"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn array_type_exceeding_the_dimension_cap_is_an_error() {
    // `MAX_ARRAY_DIMS` is 8 (private to `src/lower.rs`, not visible from
    // this integration test) -- 9 dimensions is one past the cap.
    let ty = "int".to_string() + &"[]".repeat(9);
    let err = compile_source(&wrap(&format!("{ty} xs = null;")), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn chained_index_read_on_a_two_dimensional_array_lowers_correctly() {
    let m = compile_ok(&wrap(
        "int[][] grid = {{1, 2}, {3, 4}}; int y = grid[1][0];",
    ));
    match &main_fn(&m).body.stmts[1] {
        Stmt::LetStarBinding {
            value: Expr::SeqIndex { seq, index, .. },
            ..
        } => {
            assert!(matches!(**seq, Expr::SeqIndex { .. }));
            assert!(matches!(**index, Expr::IntLit { value: 0, .. }));
        }
        other => panic!("expected a SeqIndex-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn chained_index_read_result_kind_is_the_fully_peeled_element_kind() {
    // `grid[i][j]` on an `int[][]` must be usable in an `int`-typed
    // position -- proves the chained fold's own result kind is the
    // scalar element kind, not still an array kind.
    let m = compile_ok(&wrap(
        "int[][] grid = {{1, 2}}; int y = grid[0][0]; int z = y + 1;",
    ));
    assert_eq!(main_fn(&m).body.stmts.len(), 3);
}

#[test]
fn single_index_read_on_a_two_dimensional_array_gives_an_array_valued_result() {
    // `grid[i]` alone (a single suffix, `lower_index_get`'s own case)
    // must peel exactly one dimension, leaving a still-indexable
    // one-dimensional array -- not the fully scalar element kind.
    let m = compile_ok(&wrap(
        "int[][] grid = {{1, 2}, {3, 4}}; int[] row = grid[0]; int y = row[1];",
    ));
    assert_eq!(main_fn(&m).body.stmts.len(), 3);
}

#[test]
fn three_dimensional_chained_index_read_lowers_correctly() {
    let m = compile_ok(&wrap("int[][][] cube = {{{1, 2}}}; int y = cube[0][0][1];"));
    match &main_fn(&m).body.stmts[1] {
        Stmt::LetStarBinding {
            value: Expr::SeqIndex { seq, .. },
            ..
        } => {
            // Two levels of nested SeqIndex inside the outermost one.
            match &**seq {
                Expr::SeqIndex { seq: inner, .. } => {
                    assert!(matches!(**inner, Expr::SeqIndex { .. }))
                }
                other => panic!("expected a nested SeqIndex, got {other:?}"),
            }
        }
        other => panic!("expected a SeqIndex-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn chained_index_beyond_the_array_s_own_dimension_count_is_an_error() {
    let err = compile_source(&wrap("int[] xs = {1}; int y = xs[0][0];"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn mixed_index_then_dot_suffix_chain_remains_unsupported() {
    // `grid[0].length` chains a `[` suffix then a `.` suffix -- since
    // `lower_chained_index` is reached only when *every* suffix in a
    // 2+-suffix chain is `[...]`-shaped (`is_index_only_suffix`'s own
    // guard), a mixed chain still falls through to the pre-existing
    // multi-suffix catch-all rejection, exactly as before M4d. The same
    // sub-array's `.length` remains reachable via an intermediate local
    // (see `single_index_read_on_a_two_dimensional_array_gives_an_
    // array_valued_result`) -- a real, disclosed scope boundary, not an
    // oversight.
    let err = compile_source(
        &wrap("int[][] grid = {{1}}; int y = grid[0].length;"),
        "prog",
    )
    .unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn dot_length_on_a_two_dimensional_array_returns_the_outer_length() {
    let m = compile_ok(&wrap(
        "int[][] grid = {{1, 2}, {3, 4}, {5, 6}}; int n = grid.length;",
    ));
    match &main_fn(&m).body.stmts[1] {
        Stmt::LetStarBinding {
            value: Expr::SeqLen { .. },
            ..
        } => {}
        other => panic!("expected a SeqLen-valued LetStarBinding, got {other:?}"),
    }
}

#[test]
fn feature_sequences_is_declared_when_a_multi_dimensional_array_is_lowered() {
    let m = compile_ok(&wrap("int[][] grid = {{1, 2}};"));
    assert!(m.manifest.contains(Feature::Sequences));
}

#[test]
fn multi_dimensional_indexed_assignment_remains_deferred() {
    // `grid[i][j] = v;` -- a chained assignment target -- remains
    // deferred alongside compound-assignment/increment-decrement on an
    // indexed target (see `lower_indexed_assignment`'s own doc comment).
    let err =
        compile_source(&wrap("int[][] grid = {{1, 2}}; grid[0][0] = 9;"), "prog").unwrap_err();
    assert!(!err.message.is_empty());
}

#[test]
fn single_index_assignment_on_a_two_dimensional_array_lowers_correctly() {
    // `grid[i] = v;` (a whole sub-array assignment, not chained) IS
    // supported -- `indexed_assign_target`'s own single-suffix match arm
    // already handles it; `v` must itself be an array value.
    let m = compile_ok(&wrap(
        "int[][] grid = {{1, 2}, {3, 4}}; grid[0] = new int[]{9, 9};",
    ));
    assert!(matches!(main_fn(&m).body.stmts[1], Stmt::SeqSet { .. }));
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

/// A minimal, hand-built `expression -> primary -> literal -> TOKEN
/// "true"` chain -- `lower_expr`'s dispatch is purely by grammar
/// `rule_name`, not by requiring every "official" precedence level in
/// between, so this three-node chain lowers exactly like a full
/// `assignment_expression -> ... -> literal` chain would, just without
/// the boilerplate. Reused below to keep a pathologically deep
/// hand-built `if`-nesting tree's *condition* subtrees cheap to build.
fn bool_true_expr() -> GrammarASTNode {
    let tok = Token {
        type_: TokenType::Keyword,
        value: "true".to_string(),
        line: 1,
        column: 1,
        type_name: None,
        flags: None,
        cv: None,
    };
    node(
        "expression",
        vec![ASTNodeOrToken::Node(node(
            "primary",
            vec![ASTNodeOrToken::Node(node(
                "literal",
                vec![ASTNodeOrToken::Token(tok)],
            ))],
        ))],
    )
}

/// `lower_statement`/`lower_if_statement`/`lower_body`/`lower_block_node`'s
/// mutual recursion must not overflow the native stack on pathologically
/// deep `if`-nesting handed directly to the public `compile()` entry
/// point, mirroring `find_main_method`'s own identically-reasoned guard.
/// In practice, real *parsed* Java source nesting this deep would already
/// be rejected by `collect_bounded`'s own blanket per-raw-node
/// `MAX_TREE_DEPTH` cap first (it walks every grammar node from `program`
/// down, not just statement boundaries, so it grows much faster per
/// source-level nesting than this statement-specific guard does) — this
/// hand-built tree keeps every level's raw node count minimal (one
/// `block` + one `block_statement` + one `if_statement` + `bool_true_expr`'s
/// fixed three nodes + one `statement` wrapping the next level) precisely
/// so `MAX_STMT_DEPTH` is the guard that actually fires here, proving it
/// is not dead code even though `collect_bounded` dominates for realistic
/// deeply-nested *parsed* input.
#[test]
fn deeply_nested_if_statements_report_depth_error_not_stack_overflow() {
    // innermost: an empty block (no further statement -- just needs to be
    // a valid `statement` alternative to stop the chain; a "block" is the
    // simplest since it needs no expression).
    let mut stmt = node(
        "statement",
        vec![ASTNodeOrToken::Node(node("block", vec![]))],
    );
    for _ in 0..40 {
        let if_stmt = node(
            "if_statement",
            vec![
                ASTNodeOrToken::Node(bool_true_expr()),
                ASTNodeOrToken::Node(stmt),
            ],
        );
        let inner_stmt = node("statement", vec![ASTNodeOrToken::Node(if_stmt)]);
        let block_stmt = node("block_statement", vec![ASTNodeOrToken::Node(inner_stmt)]);
        let block = node("block", vec![ASTNodeOrToken::Node(block_stmt)]);
        stmt = node("statement", vec![ASTNodeOrToken::Node(block)]);
    }
    let block_stmt = node("block_statement", vec![ASTNodeOrToken::Node(stmt)]);
    let block = node("block", vec![ASTNodeOrToken::Node(block_stmt)]);
    let method_body = node("method_body", vec![ASTNodeOrToken::Node(block)]);
    let method_declarator = node(
        "method_declarator",
        vec![ASTNodeOrToken::Token(Token {
            type_: TokenType::Name,
            value: "main".to_string(),
            line: 1,
            column: 1,
            type_name: None,
            flags: None,
            cv: None,
        })],
    );
    let method_decl = node(
        "method_declaration",
        vec![
            ASTNodeOrToken::Node(method_declarator),
            ASTNodeOrToken::Node(method_body),
        ],
    );
    let class_decl = node("class_declaration", vec![ASTNodeOrToken::Node(method_decl)]);
    let program = node("program", vec![ASTNodeOrToken::Node(class_decl)]);

    let err = compile(&program, "prog").unwrap_err();
    assert!(
        err.message.contains("nesting exceeds"),
        "expected a depth-exceeded error, got: {}",
        err.message
    );
}

/// Deeply nested lambda expressions (`x -> (y -> (z -> ...)))`) handed
/// directly to the public `compile()` entry point must not overflow the
/// native stack. This is specifically the property `lower_lambda_
/// expression`'s own doc comment argues for: it deliberately threads the
/// ambient `depth` counter through every recursive call it makes,
/// instead of resetting to a fresh budget at its own boundary the way
/// `lower_method_declaration`'s method-body lowering safely does (safe
/// there only because a `method_declaration` can never nest inside
/// another one at the source level — lambdas, unlike methods, can nest
/// arbitrarily via ordinary expression syntax, so resetting the depth
/// counter per lambda would let nested lambdas bypass `MAX_EXPR_DEPTH`/
/// `MAX_STMT_DEPTH` entirely).
///
/// In practice, for this hand-built tree `collect_bounded`'s own blanket
/// per-raw-node `MAX_TREE_DEPTH` cap is the guard that actually fires
/// (each lambda level costs it 4 raw nodes — `lambda_expression` +
/// `lambda_parameters` + `lambda_body` + the `expression` wrapper — so
/// it reaches its own 64-level cap well before this lambda chain could
/// reach `MAX_EXPR_DEPTH` on its own terms) — the same situation the
/// `if`-statement depth-guard test right above already discloses for
/// itself. Regardless of *which* guard fires first, the property this
/// test actually verifies is the same one: a clean, typed error instead
/// of a native stack overflow.
#[test]
fn deeply_nested_lambda_expressions_report_depth_error_not_stack_overflow() {
    fn zero_param_lambda(body: GrammarASTNode) -> GrammarASTNode {
        node(
            "lambda_expression",
            vec![
                ASTNodeOrToken::Node(node("lambda_parameters", vec![])),
                ASTNodeOrToken::Node(node("lambda_body", vec![ASTNodeOrToken::Node(body)])),
            ],
        )
    }
    let mut inner = bool_true_expr();
    for _ in 0..70 {
        let lam = zero_param_lambda(inner);
        inner = node("expression", vec![ASTNodeOrToken::Node(lam)]);
    }
    let expr_stmt = node("expression_statement", vec![ASTNodeOrToken::Node(inner)]);
    let stmt = node("statement", vec![ASTNodeOrToken::Node(expr_stmt)]);
    let block_stmt = node("block_statement", vec![ASTNodeOrToken::Node(stmt)]);
    let block = node("block", vec![ASTNodeOrToken::Node(block_stmt)]);
    let method_body = node("method_body", vec![ASTNodeOrToken::Node(block)]);
    let method_declarator = node(
        "method_declarator",
        vec![ASTNodeOrToken::Token(Token {
            type_: TokenType::Name,
            value: "main".to_string(),
            line: 1,
            column: 1,
            type_name: None,
            flags: None,
            cv: None,
        })],
    );
    let method_decl = node(
        "method_declaration",
        vec![
            ASTNodeOrToken::Node(method_declarator),
            ASTNodeOrToken::Node(method_body),
        ],
    );
    let class_body = node("class_body", vec![ASTNodeOrToken::Node(method_decl)]);
    let class_decl = node("class_declaration", vec![ASTNodeOrToken::Node(class_body)]);
    let program = node("program", vec![ASTNodeOrToken::Node(class_decl)]);

    let err = compile(&program, "prog").unwrap_err();
    assert!(
        err.message.contains("nesting exceeds"),
        "expected a depth-exceeded error, got: {}",
        err.message
    );
}
