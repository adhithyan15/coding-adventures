//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **this** printing cases — the `this` keyword
//! (`ThisExpression`). This is the eighteenth CodePrinter port into
//! `closure-emitter` (after core / declarations / trailing-comma / numbers /
//! string-escape / ascii-escape / object-literal / function-expression /
//! arrow-function / template / update / new / sequence / tagged-template /
//! spread / yield / await) and isolates `emit_this` + the `PREC_PRIMARY`
//! classification that landed with `Expression::ThisExpression`
//! (CLOC12.165).
//!
//! # How the emitter prints a `this` (recap)
//!
//! `this` is a *reserved-word primary* — a bare keyword that binds at the
//! tightest level, like an identifier. The emitter prints the four characters
//! `this` and never wraps it (in any parent) nor forces a paren around an
//! operand (it carries none).
//!
//! ```text
//!   this        → this        bare keyword
//!   this.x      → this.x      member object binds at primary → no parens
//!   this()      → this()      call callee binds at primary → no parens
//!   f(this)     → f(this)     plain primary argument
//!   this+1      → this+1      a binary parent leaves the primary bare
//!   this.a.b    → this.a.b    member chains compose without parens
//!   this.m()    → this.m()    method call composes without parens
//! ```
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (the emitter is the unit
//! under test). The bridge conversion of `this` (gap-166) lands in CLOC12.165
//! PR2 and is exercised separately in `javascript-parser`; here the emitter is
//! driven from hand-constructed AST so this port does not depend on the bridge.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BinaryExpression, BinaryOperator, CallExpression, Expression, ExpressionStatement, Identifier,
    MemberExpression, NumericLiteral, Program, ProgramItem, SourceType, Statement, ThisExpression,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn ident(name: &str) -> Expression {
    Expression::Identifier(Identifier { cv: None, name: name.to_string() })
}

fn num(value: f64, raw: &str) -> Expression {
    Expression::NumericLiteral(NumericLiteral { cv: None, value, raw: raw.to_string() })
}

fn member(object: Expression, property: &str) -> Expression {
    Expression::MemberExpression(MemberExpression {
        cv: None,
        object: Box::new(object),
        property: Box::new(ident(property)),
        computed: false,
    })
}

fn binary(op: BinaryOperator, left: Expression, right: Expression) -> Expression {
    Expression::BinaryExpression(BinaryExpression {
        cv: None,
        operator: op,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn call(callee: Expression, arguments: Vec<Expression>) -> Expression {
    Expression::CallExpression(CallExpression { cv: None, callee: Box::new(callee), arguments })
}

/// Build a `ThisExpression` — the `this` keyword.
fn this_expr() -> Expression {
    Expression::ThisExpression(ThisExpression { cv: None })
}

fn stmt(expr: Expression) -> ProgramItem {
    ProgramItem::Statement(Statement::expression_statement(ExpressionStatement {
        cv: None,
        expression: expr,
    }))
}

fn emit_default(expr: Expression) -> String {
    let prog =
        Program::new_untraced(EsVersion::Es2025, SourceType::Module).with_body(vec![stmt(expr)]);
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    emit(&prog, &sidecar, &mut cv, &EmitOptions::default())
        .expect("emit failed")
        .code
}

/// Upstream `assertPrint(input, expected)` reshaped: emit the expression as a
/// single-statement program and assert the emitted code equals `expected`.
fn assert_emits(expr: Expression, expected: &str) {
    let code = emit_default(expr);
    assert_eq!(
        code, expected,
        "this emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — the surface shape
// =====================================================================

/// `this` — the bare keyword.
#[test]
fn this_value_is_bare_keyword() {
    assert_emits(this_expr(), "this;");
}

// =====================================================================
// Active — `this` as a primary composes without parens
// =====================================================================

/// `this.x` — a member parent binds at primary strength; the `this` object
/// needs no parens.
#[test]
fn this_member_object_is_bare() {
    assert_emits(member(this_expr(), "x"), "this.x;");
}

/// `this()` — a call callee likewise leaves the primary `this` bare.
#[test]
fn this_call_callee_is_bare() {
    assert_emits(call(this_expr(), vec![]), "this();");
}

/// `f(this)` — `this` as a call argument is a plain primary operand.
#[test]
fn this_as_call_argument_is_bare() {
    assert_emits(call(ident("f"), vec![this_expr()]), "f(this);");
}

/// `this.a.b` — member chains off `this` compose without any parens.
#[test]
fn this_member_chain_is_bare() {
    assert_emits(member(member(this_expr(), "a"), "b"), "this.a.b;");
}

/// `this.m()` — a method call `this.m()` composes without parens (member then
/// call, both at primary strength).
#[test]
fn this_method_call_is_bare() {
    assert_emits(call(member(this_expr(), "m"), vec![]), "this.m();");
}

// =====================================================================
// Active — the whole node's precedence (this tags at PREC_PRIMARY)
// =====================================================================

/// `this+1` — even a binary parent leaves the primary `this` bare on the left.
#[test]
fn this_under_binary_parent_is_bare() {
    assert_emits(binary(BinaryOperator::Add, this_expr(), num(1.0, "1")), "this+1;");
}
