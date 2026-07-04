//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **super** printing cases — the `super` keyword
//! (`Super`). This is the nineteenth CodePrinter port into `closure-emitter`
//! (after core / declarations / trailing-comma / numbers / string-escape /
//! ascii-escape / object-literal / function-expression / arrow-function /
//! template / update / new / sequence / tagged-template / spread / yield /
//! await / this) and isolates `emit_super` + the `PREC_PRIMARY` classification
//! that landed with `Expression::Super` (CLOC12.166).
//!
//! # How the emitter prints a `super` (recap)
//!
//! `super` is a *reserved-word primary* — the sibling of `this`, a bare
//! keyword that binds at the tightest level. The emitter prints the five
//! characters `super` and never wraps it (in any parent) nor forces a paren
//! around an operand (it carries none).
//!
//! ```text
//!   super       → super       bare keyword
//!   super.x     → super.x     member object binds at primary → no parens
//!   super()     → super()     call callee binds at primary → no parens
//!   f(super)    → f(super)     plain primary argument
//!   super+1     → super+1     a binary parent leaves the primary bare
//!   super.a.b   → super.a.b   member chains compose without parens
//!   super.m()   → super.m()   method call composes without parens
//! ```
//!
//! `super` is *syntactically* legal only as a member object / call callee
//! inside a method or derived constructor, but that is the parser's concern —
//! the emitter is a pure printer and prints whatever AST it is handed. The
//! bare `super;` / `f(super)` / `super+1` cases below are hand-constructed to
//! isolate the emitter's leaf-primary handling, not to assert JS validity.
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (the emitter is the unit
//! under test). The bridge conversion of `super` (gap-167) lands in CLOC12.166
//! PR2 and is exercised separately in `javascript-parser`; here the emitter is
//! driven from hand-constructed AST so this port does not depend on the bridge.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BinaryExpression, BinaryOperator, CallExpression, Expression, ExpressionStatement, Identifier,
    MemberExpression, NumericLiteral, Program, ProgramItem, SourceType, Statement, Super,
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

/// Build a `Super` — the `super` keyword.
fn super_expr() -> Expression {
    Expression::Super(Super { cv: None })
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
        "super emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — the surface shape
// =====================================================================

/// `super` — the bare keyword.
#[test]
fn super_value_is_bare_keyword() {
    assert_emits(super_expr(), "super;");
}

// =====================================================================
// Active — `super` as a primary composes without parens
// =====================================================================

/// `super.x` — a member parent binds at primary strength; the `super` object
/// needs no parens.
#[test]
fn super_member_object_is_bare() {
    assert_emits(member(super_expr(), "x"), "super.x;");
}

/// `super()` — a call callee likewise leaves the primary `super` bare.
#[test]
fn super_call_callee_is_bare() {
    assert_emits(call(super_expr(), vec![]), "super();");
}

/// `f(super)` — `super` as a call argument is a plain primary operand.
#[test]
fn super_as_call_argument_is_bare() {
    assert_emits(call(ident("f"), vec![super_expr()]), "f(super);");
}

/// `super.a.b` — member chains off `super` compose without any parens.
#[test]
fn super_member_chain_is_bare() {
    assert_emits(member(member(super_expr(), "a"), "b"), "super.a.b;");
}

/// `super.m()` — a method call `super.m()` composes without parens (member
/// then call, both at primary strength).
#[test]
fn super_method_call_is_bare() {
    assert_emits(call(member(super_expr(), "m"), vec![]), "super.m();");
}

// =====================================================================
// Active — the whole node's precedence (super tags at PREC_PRIMARY)
// =====================================================================

/// `super+1` — even a binary parent leaves the primary `super` bare on the
/// left.
#[test]
fn super_under_binary_parent_is_bare() {
    assert_emits(binary(BinaryOperator::Add, super_expr(), num(1.0, "1")), "super+1;");
}
