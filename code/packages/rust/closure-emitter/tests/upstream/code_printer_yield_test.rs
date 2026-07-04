//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **yield** printing cases — the generator
//! `YieldExpression` `yield` / `yield x` / `yield* xs`. This is the sixteenth
//! CodePrinter port into `closure-emitter` (after core / declarations /
//! trailing-comma / numbers / string-escape / ascii-escape / object-literal /
//! function-expression / arrow-function / template / update / new / sequence /
//! tagged-template / spread) and isolates `emit_yield` + the `PREC_ASSIGNMENT`
//! classification that landed with `Expression::YieldExpression` (CLOC12.163).
//!
//! # How the emitter prints a yield (recap)
//!
//! ```text
//!   yield            → yield          bare, keyword only
//!   yield a          → yield a        non-delegate: mandatory keyword↔arg space
//!   yield* xs        → yield*xs       delegate: the `*` self-terminates, no space
//! ```
//!
//! The argument prints at `PREC_ASSIGNMENT` (the yield operand grammar is an
//! `AssignmentExpression`):
//!
//! ```text
//!   yield a?b:c      a conditional argument binds tighter than the floor → bare
//!   yield a=b        an assignment argument is exactly at the floor       → bare
//!   yield (a,b)      a LOOSER sequence argument WRAPS
//! ```
//!
//! And the whole yield tags at `PREC_ASSIGNMENT`, so a tighter parent wraps it:
//!
//! ```text
//!   (yield a)+1      binary parent binds tighter → wrap
//!   (yield a).b      member parent binds tighter → wrap
//! ```
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (upstream lex/parses
//! source inside a generator function). The emitter is the unit under test
//! here — the bridge conversion of the yield form (CLOC12.163 PR2, gap-164) is
//! exercised separately in `javascript-parser` once generator bodies parse.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    AssignmentExpression, AssignmentOperator, AssignmentTarget, BinaryExpression, BinaryOperator,
    ConditionalExpression, Expression, ExpressionStatement, Identifier, MemberExpression,
    NumericLiteral, Program, ProgramItem, SequenceExpression, SourceType, Statement,
    YieldExpression,
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

fn seq(operands: Vec<Expression>) -> Expression {
    Expression::SequenceExpression(SequenceExpression { cv: None, expressions: operands })
}

fn binary(op: BinaryOperator, left: Expression, right: Expression) -> Expression {
    Expression::BinaryExpression(BinaryExpression {
        cv: None,
        operator: op,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn assign(name: &str, rhs: Expression) -> Expression {
    Expression::AssignmentExpression(AssignmentExpression {
        cv: None,
        operator: AssignmentOperator::Eq,
        left: AssignmentTarget::Identifier(Identifier { cv: None, name: name.to_string() }),
        right: Box::new(rhs),
    })
}

/// Build a `YieldExpression` from its two axes: `delegate` (the `*`) and the
/// optional operand.
fn yld(delegate: bool, argument: Option<Expression>) -> Expression {
    Expression::YieldExpression(YieldExpression {
        cv: None,
        delegate,
        argument: argument.map(Box::new),
    })
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
        "yield emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — the three surface shapes
// =====================================================================

/// `yield` — a bare yield with no operand prints just the keyword.
/// Models upstream `assertPrintSame("function*f(){yield}")` (yield in
/// statement position, operand-less).
#[test]
fn yield_bare_keyword_only() {
    assert_emits(yld(false, None), "yield;");
}

/// `yield a` — a non-delegating yield with an operand. The keyword and operand
/// are separated by a mandatory space (`yielda` would be a single identifier).
/// Models upstream `assertPrintSame("function*f(){yield a}")`.
#[test]
fn yield_value_requires_space() {
    assert_emits(yld(false, Some(ident("a"))), "yield a;");
}

/// `yield*xs` — a delegating yield. The `*` terminates the keyword token, so
/// no separator is needed before the operand. Models upstream
/// `assertPrintSame("function*f(){yield*xs}")`.
#[test]
fn yield_delegate_no_space() {
    assert_emits(yld(true, Some(ident("xs"))), "yield*xs;");
}

/// `yield*a.b` — a delegating yield whose operand is a member chain (member
/// binds tighter than assignment, so it prints bare after `yield*`).
#[test]
fn yield_delegate_member_operand() {
    assert_emits(yld(true, Some(member(ident("a"), "b"))), "yield*a.b;");
}

// =====================================================================
// Active — operand precedence (operand prints at PREC_ASSIGNMENT)
// =====================================================================

/// `yield a?b:c` — a conditional operand binds tighter than the sequence floor
/// (yield's operand grammar is an `AssignmentExpression`, which subsumes the
/// conditional), so it prints bare: no over-wrap.
#[test]
fn yield_conditional_operand_is_bare() {
    let cond = Expression::ConditionalExpression(ConditionalExpression {
        cv: None,
        test: Box::new(ident("a")),
        consequent: Box::new(ident("b")),
        alternate: Box::new(ident("c")),
    });
    assert_emits(yld(false, Some(cond)), "yield a?b:c;");
}

/// `yield a=b` — an assignment operand is exactly at the operand's assignment
/// precedence, so it prints bare. Models upstream `a=yield b` shapes where the
/// yield operand itself is an assignment.
#[test]
fn yield_assignment_operand_is_bare() {
    assert_emits(yld(false, Some(assign("a", ident("b")))), "yield a=b;");
}

/// `yield (a,b)` — a **sequence** operand is the one form that must wrap: it
/// binds looser than the assignment-precedence operand floor, so a bare
/// `yield a,b` would parse as `(yield a),b`.
#[test]
fn yield_sequence_operand_is_wrapped() {
    assert_emits(yld(false, Some(seq(vec![ident("a"), ident("b")]))), "yield (a,b);");
}

// =====================================================================
// Active — the whole node's precedence (yield tags at PREC_ASSIGNMENT)
// =====================================================================

/// `(yield a)+1` — the whole yield binds looser than `+`, so a binary parent
/// wraps it. Models upstream `assertPrint` cases where a yield is an operand of
/// a tighter operator.
#[test]
fn yield_wraps_as_binary_operand() {
    let e = binary(BinaryOperator::Add, yld(false, Some(ident("a"))), num(1.0, "1"));
    assert_emits(e, "(yield a)+1;");
}

/// `(yield a).b` — a member parent binds at primary strength and wraps the
/// looser yield object.
#[test]
fn yield_wraps_as_member_object() {
    assert_emits(member(yld(false, Some(ident("a"))), "b"), "(yield a).b;");
}
