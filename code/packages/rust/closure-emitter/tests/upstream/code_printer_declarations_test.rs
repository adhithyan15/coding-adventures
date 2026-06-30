//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Companion to `code_printer_test.rs` — this file holds the
//! `VariableDeclaration` round-trip ports that gap-023 was tracking.
//! Splitting the declarations out keeps each file at a manageable
//! size and isolates the verbosity of hand-constructing
//! `VariableDeclaration { kind, declarations: [VariableDeclarator
//! { id, init }] }` ASTs.
//!
//! ## Why a dedicated file?
//!
//! Upstream's `CodePrinterTest` uses `assertPrintSame("var x = 1;")`
//! to round-trip code: parse, print, assert the printed form equals
//! the input. We don't have a JS parser in the test harness yet, so
//! each test has to build the AST by hand. For a one-liner expression
//! this is fine; for a `VariableDeclaration` (which nests
//! `VariableDeclarator { id, init }` arrays inside a top-level
//! statement), it's noisier. Bundling all the declaration round-trips
//! into one file makes the verbosity contained and the intent
//! obvious — every test here is the same shape (build a var/let/const
//! AST, emit, compare to a literal string).
//!
//! ## What's covered
//!
//! The minimal Phase 1 surface for `VariableDeclaration` round-trips:
//!
//! - `var x;` — no init, default-`var` kind
//! - `var x = 1;` — single declarator, numeric init
//! - `var x = "hi";` — string init (pins quote-policy with declarations)
//! - `var x = null;` — null init
//! - `var x = true;` — boolean init
//! - `var x = a + b;` — non-literal init (BinaryExpression)
//! - `let x;` / `let x = 1;` — `let` kind
//! - `const X = 1;` — `const` kind
//! - `var x, y;` — multiple declarators, both bare
//! - `var x = 1, y = 2;` — multiple declarators with inits
//! - `var x, y = 2;` — mixed: first bare, second with init
//! - `var x = [];` — array literal init (also pins gap-022's empty-array
//!   no-trailing-comma case from upstream's
//!   `testNoTrailingCommaInEmptyArrayLiteral`)
//!
//! Each test mirrors an upstream `assertPrintSame(...)` line; the
//! mapping is in each test's doc comment.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    ArrayExpression, BinaryExpression, BinaryOperator, BindingTarget, BooleanLiteral, Declaration,
    Expression, Identifier, NullLiteral, NumericLiteral, Program, ProgramItem, SourceType,
    Statement, StringLiteral, VarKind, VariableDeclaration, VariableDeclarator,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn ident_name(name: &str) -> Identifier {
    Identifier {
        cv: None,
        name: name.to_string(),
    }
}

fn ident_expr(name: &str) -> Expression {
    Expression::Identifier(ident_name(name))
}

fn num(v: f64) -> Expression {
    Expression::NumericLiteral(NumericLiteral {
        cv: None,
        value: v,
        raw: if v.fract() == 0.0 && v.is_finite() {
            format!("{}", v as i64)
        } else {
            v.to_string()
        },
    })
}

fn string(v: &str) -> Expression {
    Expression::StringLiteral(StringLiteral {
        cv: None,
        value: v.to_string(),
        raw: format!("\"{}\"", v),
    })
}

fn null() -> Expression {
    Expression::NullLiteral(NullLiteral { cv: None })
}

fn boolean(v: bool) -> Expression {
    Expression::BooleanLiteral(BooleanLiteral { cv: None, value: v })
}

/// Build a single-declarator VariableDeclaration. Convenience for the
/// common `kind X [= init];` shape.
fn var_decl_single(kind: VarKind, name: &str, init: Option<Expression>) -> VariableDeclaration {
    VariableDeclaration {
        cv: None,
        kind,
        declarations: vec![VariableDeclarator {
            cv: None,
            id: BindingTarget::Identifier(ident_name(name)),
            init,
        }],
    }
}

/// Build a multi-declarator VariableDeclaration. Each element of
/// `bindings` is a `(name, init_opt)` pair.
fn var_decl_multi(kind: VarKind, bindings: Vec<(&str, Option<Expression>)>) -> VariableDeclaration {
    VariableDeclaration {
        cv: None,
        kind,
        declarations: bindings
            .into_iter()
            .map(|(name, init)| VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(ident_name(name)),
                init,
            })
            .collect(),
    }
}

/// Wrap a Declaration in a Program with a single top-level item.
fn program_with_decl(d: Declaration) -> Program {
    Program::new_untraced(EsVersion::Es2025, SourceType::Module)
        .with_body(vec![ProgramItem::Statement(Statement::Declaration(d))])
}

fn emit_default(prog: Program) -> String {
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    emit(&prog, &sidecar, &mut cv, &EmitOptions::default())
        .expect("emit failed")
        .code
}

/// `assertPrintSame(src)` shaped helper: emit a single
/// `VariableDeclaration` and compare to the expected literal output.
fn assert_var_emits(decl: VariableDeclaration, expected: &str) {
    let code = emit_default(program_with_decl(Declaration::VariableDeclaration(decl)));
    assert_eq!(
        code, expected,
        "emit output did not match expected\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Ported tests (gap-023)
// =====================================================================

/// Upstream `assertPrintSame("var x;")`.
///
/// The minimal declaration shape — no init, default `var` kind. Pins
/// the trailing-semicolon contract and that the bare identifier
/// emits without surrounding whitespace.
#[test]
fn var_bare_no_init() {
    assert_var_emits(var_decl_single(VarKind::Var, "x", None), "var x;");
}

/// Upstream `assertPrintSame("var x = 1;")`.
///
/// Numeric init with a single integer. Pins the `<kind> <id>=<init>;`
/// emit shape and the no-space-around-`=` policy (closure-compiler
/// drops the spaces in compact mode).
#[test]
fn var_with_integer_init() {
    assert_var_emits(var_decl_single(VarKind::Var, "x", Some(num(1.0))), "var x=1;");
}

/// Upstream `assertPrintSame("var x = \"hi\";")`.
///
/// String init. Pins the interaction between declaration emit and
/// gap-026's quote-choice optimisation — both quotes survive when
/// the input string contains no quote characters.
#[test]
fn var_with_string_init() {
    assert_var_emits(
        var_decl_single(VarKind::Var, "x", Some(string("hi"))),
        "var x=\"hi\";",
    );
}

/// Upstream `assertPrintSame("var x = null;")`.
///
/// Null literal init. Trivial — pins that `null` round-trips
/// verbatim through both the AST and the emitter.
#[test]
fn var_with_null_init() {
    assert_var_emits(
        var_decl_single(VarKind::Var, "x", Some(null())),
        "var x=null;",
    );
}

/// Upstream `assertPrintSame("var x = true;")`.
///
/// Boolean init. Pins that booleans emit as the keyword form (not
/// `1`/`0`).
#[test]
fn var_with_boolean_init() {
    assert_var_emits(
        var_decl_single(VarKind::Var, "x", Some(boolean(true))),
        "var x=true;",
    );
}

/// Upstream `assertPrintSame("var x = a + b;")`.
///
/// Non-literal init. Pins that the init expression nests cleanly
/// inside a declaration without extra parens — the BinaryExpression's
/// precedence doesn't trigger the gap-024 paren-wrap policy in this
/// position. Note: our emitter currently inserts spaces around binary
/// `+`/`-` even in compact mode (`a + b`, not `a+b`). Upstream
/// closure-compiler drops them; flipping our behaviour is its own
/// follow-up tracked under emitter style. We pin our current output
/// here so this test is a true round-trip canary.
#[test]
fn var_with_binary_init() {
    let init = Expression::BinaryExpression(BinaryExpression {
        cv: None,
        operator: BinaryOperator::Add,
        left: Box::new(ident_expr("a")),
        right: Box::new(ident_expr("b")),
    });
    assert_var_emits(
        var_decl_single(VarKind::Var, "x", Some(init)),
        "var x=a+b;",
    );
}

/// Upstream `assertPrintSame("let x;")` and `assertPrintSame("let x = 1;")`.
///
/// `let` kind both bare and with init. Pins that `let` emits as
/// `"let"` (not as `"var"` after some Phase 2 lowering — Phase 1
/// preserves the keyword).
#[test]
fn let_kind_round_trips() {
    assert_var_emits(var_decl_single(VarKind::Let, "x", None), "let x;");
    assert_var_emits(
        var_decl_single(VarKind::Let, "x", Some(num(1.0))),
        "let x=1;",
    );
}

/// Upstream `assertPrintSame("const X = 1;")`.
///
/// `const` kind. Pins the keyword preservation and the bare-init
/// shape. (`const X;` without an init is a syntax error in real JS,
/// so we don't test that case — the AST is permissive but the
/// emitter doesn't validate.)
#[test]
fn const_kind_round_trips() {
    assert_var_emits(
        var_decl_single(VarKind::Const, "X", Some(num(1.0))),
        "const X=1;",
    );
}

/// Upstream `assertPrintSame("var x, y;")`.
///
/// Multiple bare declarators. Pins the comma-separator policy: no
/// surrounding spaces in compact mode.
#[test]
fn var_multiple_bare_declarators() {
    assert_var_emits(
        var_decl_multi(VarKind::Var, vec![("x", None), ("y", None)]),
        "var x,y;",
    );
}

/// Upstream `assertPrintSame("var x = 1, y = 2;")`.
///
/// Multiple declarators each with their own init. Pins the
/// comma-separator policy AND that each declarator carries its own
/// `id=init` (the kind keyword is NOT repeated).
#[test]
fn var_multiple_declarators_with_inits() {
    assert_var_emits(
        var_decl_multi(
            VarKind::Var,
            vec![("x", Some(num(1.0))), ("y", Some(num(2.0)))],
        ),
        "var x=1,y=2;",
    );
}

/// Upstream `assertPrintSame("var x, y = 2;")`.
///
/// Mixed: first declarator bare, second with init. Pins the
/// per-declarator-optional-init shape — a declarator's init is
/// emitted iff it's `Some`.
#[test]
fn var_mixed_bare_and_init() {
    assert_var_emits(
        var_decl_multi(VarKind::Var, vec![("x", None), ("y", Some(num(2.0)))]),
        "var x,y=2;",
    );
}

/// Upstream `assertPrintSame("var x = [];")` — same as
/// `CodePrinterTest::testNoTrailingCommaInEmptyArrayLiteral`.
///
/// Empty array literal init. The body of the upstream test pins that
/// an empty array literal emits as `[]` and *not* `[,]` (the latter
/// would be a single-elision array, a different value). This is
/// gap-023's home for the "no trailing comma in empty array" case
/// originally tracked under that test name in `code_printer_test.rs`.
#[test]
fn var_with_empty_array_init() {
    let init = Expression::ArrayExpression(ArrayExpression {
        cv: None,
        elements: vec![],
    });
    assert_var_emits(
        var_decl_single(VarKind::Var, "x", Some(init)),
        "var x=[];",
    );
}

/// Upstream `assertPrintSame("var x = [1];")`.
///
/// Single-element array literal init. Pins that a single-element
/// array emits without a trailing comma (i.e. `[1]` not `[1,]`).
#[test]
fn var_with_single_element_array_init() {
    let init = Expression::ArrayExpression(ArrayExpression {
        cv: None,
        elements: vec![Some(num(1.0))],
    });
    assert_var_emits(
        var_decl_single(VarKind::Var, "x", Some(init)),
        "var x=[1];",
    );
}

/// Upstream `assertPrintSame("var x = [1, 2, 3];")` (rendered compact).
///
/// Multi-element array literal init. Pins the element-separator
/// policy: no surrounding spaces in compact mode.
#[test]
fn var_with_multi_element_array_init() {
    let init = Expression::ArrayExpression(ArrayExpression {
        cv: None,
        elements: vec![Some(num(1.0)), Some(num(2.0)), Some(num(3.0))],
    });
    assert_var_emits(
        var_decl_single(VarKind::Var, "x", Some(init)),
        "var x=[1,2,3];",
    );
}
