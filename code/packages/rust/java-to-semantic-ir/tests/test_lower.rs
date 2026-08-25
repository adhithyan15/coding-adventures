//! JV02 milestone M0/M1 tests: literals, a synthesized `main` function
//! (M0), and local variable declarations/re-assignment/operators (M1).
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
fn compound_assignment_is_unsupported() {
    let err = compile_source(&wrap("int x = 1; x += 1;"), "prog").unwrap_err();
    assert!(
        err.message.contains("compound assignment"),
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
fn postfix_increment_is_unsupported() {
    let err = compile_source(&wrap("int x = 1; x++;"), "prog").unwrap_err();
    assert!(
        err.message.contains("increment"),
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
