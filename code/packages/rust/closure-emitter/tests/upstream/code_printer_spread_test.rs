//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **spread** printing cases — the `SpreadElement`
//! `...arg`. This is the fifteenth CodePrinter port into `closure-emitter`
//! (after core / declarations / trailing-comma / numbers / string-escape /
//! ascii-escape / object-literal / function-expression / arrow-function /
//! template / update / new / sequence / tagged-template) and isolates
//! `emit_spread` + the `PREC_ASSIGNMENT` classification that landed with
//! `Expression::SpreadElement` (CLOC12.162).
//!
//! # How the emitter prints a spread (recap)
//!
//! ```text
//!   f(...a)          → f(...a)         spread a call argument, no `... a` space
//!   f(a, ...b, c)    → f(a,...b,c)     interleaved, arity preserved
//!   [1, ...a, 2]     → [1,...a,2]      spread an array element
//!   new F(...a)      → new F(...a)     spread a `new` argument
//! ```
//!
//! The `...` prefix has no interior space, and the argument prints at
//! `PREC_ASSIGNMENT`:
//!
//! ```text
//!   f(...a?b:c)      a conditional argument binds tighter than the floor → bare
//!   f(...(a,b))      a LOOSER sequence argument WRAPS, or `...a,b` would spread
//!                    only `a` and leave `,b` as a second list slot
//! ```
//!
//! The spread node itself tags at `PREC_ASSIGNMENT`, matching the
//! assignment-position argument/element slots it lives in, so it is never
//! spuriously parenthesised there.
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (upstream lex/parses
//! source). The emitter is the unit under test here — the bridge conversion of
//! the spread form (CLOC12.162 PR2, gap-163) is exercised separately in
//! `javascript-parser`.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    ArrayExpression, CallExpression, ConditionalExpression, Expression, ExpressionStatement,
    Identifier, MemberExpression, NewExpression, NumericLiteral, Program, ProgramItem,
    SequenceExpression, SourceType, SpreadElement, Statement,
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

fn spread(argument: Expression) -> Expression {
    Expression::SpreadElement(SpreadElement { cv: None, argument: Box::new(argument) })
}

fn call(callee: Expression, arguments: Vec<Expression>) -> Expression {
    Expression::CallExpression(CallExpression { cv: None, callee: Box::new(callee), arguments })
}

fn new_expr(callee: Expression, arguments: Vec<Expression>) -> Expression {
    Expression::NewExpression(NewExpression { cv: None, callee: Box::new(callee), arguments })
}

fn array(elements: Vec<Option<Expression>>) -> Expression {
    Expression::ArrayExpression(ArrayExpression { cv: None, elements })
}

fn member(object: Expression, property: &str) -> Expression {
    Expression::MemberExpression(MemberExpression {
        cv: None,
        object: Box::new(object),
        property: Box::new(ident(property)),
        computed: false,
    })
}

fn seq(operands: Vec<Expression>) -> Expression {
    Expression::SequenceExpression(SequenceExpression { cv: None, expressions: operands })
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
        "spread emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — spread in a call argument list
// =====================================================================

/// `f(...a)` — a spread as the sole call argument prints bare, with no space
/// between `...` and the argument.
#[test]
fn spread_sole_call_arg() {
    assert_emits(call(ident("f"), vec![spread(ident("a"))]), "f(...a);");
}

/// `f(a, ...b, c)` — a spread interleaved with plain arguments keeps position
/// and arity.
#[test]
fn spread_interleaved_call_args() {
    let e = call(ident("f"), vec![ident("a"), spread(ident("b")), ident("c")]);
    assert_emits(e, "f(a,...b,c);");
}

/// `f(...a, ...b)` — two adjacent spreads both print bare.
#[test]
fn spread_two_adjacent_call_args() {
    let e = call(ident("f"), vec![spread(ident("a")), spread(ident("b"))]);
    assert_emits(e, "f(...a,...b);");
}

/// `f(...a.b)` — the spread argument may be a member chain (member binds
/// tighter than assignment, so it prints bare).
#[test]
fn spread_member_argument() {
    let e = call(ident("f"), vec![spread(member(ident("a"), "b"))]);
    assert_emits(e, "f(...a.b);");
}

// =====================================================================
// Active — spread in an array literal
// =====================================================================

/// `[...a]` — a spread as the sole array element prints bare.
#[test]
fn spread_sole_array_element() {
    assert_emits(array(vec![Some(spread(ident("a")))]), "[...a];");
}

/// `[1, ...a, 2]` — a spread interleaved with literals keeps element count and
/// order.
#[test]
fn spread_interleaved_array_elements() {
    let e = array(vec![Some(num(1.0, "1")), Some(spread(ident("a"))), Some(num(2.0, "2"))]);
    assert_emits(e, "[1,...a,2];");
}

// =====================================================================
// Active — spread in a `new` argument list
// =====================================================================

/// `new F(...a)` — a spread flows into a `new` argument list exactly as a call
/// argument does.
#[test]
fn spread_new_argument() {
    assert_emits(new_expr(ident("F"), vec![spread(ident("a"))]), "new F(...a);");
}

/// `new F(a, ...b)` — interleaved with a plain `new` argument.
#[test]
fn spread_interleaved_new_arguments() {
    let e = new_expr(ident("F"), vec![ident("a"), spread(ident("b"))]);
    assert_emits(e, "new F(a,...b);");
}

// =====================================================================
// Active — precedence (spread argument prints at PREC_ASSIGNMENT)
// =====================================================================

/// `f(...(a,b))` — a **sequence** spread argument is the one form that must
/// wrap: a bare `...a,b` would spread only `a` and leave `,b` as a second list
/// slot.
#[test]
fn spread_sequence_argument_is_wrapped() {
    let e = call(ident("f"), vec![spread(seq(vec![ident("a"), ident("b")]))]);
    assert_emits(e, "f(...(a,b));");
}

/// `f(...a?b:c)` — a conditional argument binds tighter than the sequence
/// floor, so it prints bare (spread's operand grammar is an
/// `AssignmentExpression`, which subsumes the conditional): no over-wrap.
#[test]
fn spread_conditional_argument_is_bare() {
    let cond = Expression::ConditionalExpression(ConditionalExpression {
        cv: None,
        test: Box::new(ident("a")),
        consequent: Box::new(ident("b")),
        alternate: Box::new(ident("c")),
    });
    assert_emits(call(ident("f"), vec![spread(cond)]), "f(...a?b:c);");
}
