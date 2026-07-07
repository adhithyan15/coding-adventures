//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **new.target** printing cases — the
//! `new.target` meta-property (`NewTarget`). This is the twentieth CodePrinter
//! port into `closure-emitter` (after core / declarations / trailing-comma /
//! numbers / string-escape / ascii-escape / object-literal /
//! function-expression / arrow-function / template / update / new / sequence /
//! tagged-template / spread / yield / await / this / super) and isolates
//! `emit_new_target` + the `PREC_PRIMARY` classification that landed with
//! `Expression::NewTarget` (CLOC12.167).
//!
//! # How the emitter prints `new.target` (recap)
//!
//! `new.target` is a *reserved-word primary* — the sibling of `this` / `super`,
//! a fixed two-token-plus-dot spelling that binds at the tightest level. The
//! `.` is part of the spelling, NOT a member access, so the whole thing is one
//! atomic leaf. The emitter prints the ten characters `new.target` and never
//! wraps it (in any parent) nor forces a paren around an operand (it carries
//! none).
//!
//! ```text
//!   new.target       → new.target       the bare meta-property
//!   new.target.x     → new.target.x     member object binds at primary → no parens
//!   f(new.target)    → f(new.target)    plain primary argument
//!   new.target.a.b   → new.target.a.b   member chains compose without parens
//!   new.target.m()   → new.target.m()   method call composes without parens
//!   new.target+1     → new.target+1     a binary parent leaves the primary bare
//! ```
//!
//! `new.target` is *syntactically* legal only inside a function / constructor,
//! but that is the parser's concern — the emitter is a pure printer and prints
//! whatever AST it is handed. The cases below are hand-constructed to isolate
//! the emitter's leaf-primary handling, not to assert JS validity.
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (the emitter is the unit
//! under test). The bridge conversion of `new.target` (gap-168) lands in
//! CLOC12.167 PR2 and is exercised separately in `javascript-parser`; here the
//! emitter is driven from hand-constructed AST so this port does not depend on
//! the bridge.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BinaryExpression, BinaryOperator, CallExpression, Expression, ExpressionStatement, Identifier,
    MemberExpression, NewTarget, NumericLiteral, Program, ProgramItem, SourceType, Statement,
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

/// Build a `NewTarget` — the `new.target` meta-property.
fn new_target_expr() -> Expression {
    Expression::NewTarget(NewTarget { cv: None })
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
        "new.target emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — the surface shape
// =====================================================================

/// `new.target` — the bare meta-property, printed as its ten-character
/// spelling.
#[test]
fn new_target_value_is_bare_spelling() {
    assert_emits(new_target_expr(), "new.target;");
}

// =====================================================================
// Active — `new.target` as a primary composes without parens
// =====================================================================

/// `new.target.x` — a member parent binds at primary strength; the
/// `new.target` object needs no parens (the outer `.x` is a genuine member
/// access, distinct from the `.target` that is part of the spelling).
#[test]
fn new_target_member_object_is_bare() {
    assert_emits(member(new_target_expr(), "x"), "new.target.x;");
}

/// `f(new.target)` — `new.target` as a call argument is a plain primary
/// operand.
#[test]
fn new_target_as_call_argument_is_bare() {
    assert_emits(call(ident("f"), vec![new_target_expr()]), "f(new.target);");
}

/// `new.target.a.b` — member chains off `new.target` compose without any
/// parens.
#[test]
fn new_target_member_chain_is_bare() {
    assert_emits(member(member(new_target_expr(), "a"), "b"), "new.target.a.b;");
}

/// `new.target.m()` — a method call `new.target.m()` composes without parens
/// (member then call, both at primary strength).
#[test]
fn new_target_method_call_is_bare() {
    assert_emits(call(member(new_target_expr(), "m"), vec![]), "new.target.m();");
}

// =====================================================================
// Active — the whole node's precedence (new.target tags at PREC_PRIMARY)
// =====================================================================

/// `new.target+1` — even a binary parent leaves the primary `new.target` bare
/// on the left.
#[test]
fn new_target_under_binary_parent_is_bare() {
    assert_emits(
        binary(BinaryOperator::Add, new_target_expr(), num(1.0, "1")),
        "new.target+1;",
    );
}
