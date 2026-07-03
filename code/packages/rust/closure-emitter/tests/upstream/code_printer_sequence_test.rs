//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **comma-operator** printing cases — the
//! `SequenceExpression` `a, b, c`. This is the thirteenth CodePrinter port into
//! `closure-emitter` (after core / declarations / trailing-comma / numbers /
//! string-escape / ascii-escape / object-literal / function-expression /
//! arrow-function / template / update / new) and isolates `emit_sequence` + the
//! `PREC_SEQUENCE` (lowest) classification and the four assignment-position
//! wrap sites that landed with `Expression::SequenceExpression` (CLOC12.160).
//!
//! # How the emitter prints a sequence (recap)
//!
//! ```text
//!   a, b, c            → a,b,c            operands comma-joined, no spaces
//! ```
//!
//! The comma operator is the **loosest** expression there is (below
//! assignment), so a sequence used as a *sub-operand* almost always needs
//! parentheses, or the surrounding operator captures only one arm:
//!
//! ```text
//!   f((a, b), c)   without parens `f(a, b, c)` is a THREE-argument call
//!   [(a, b), c]    without parens `[a, b, c]` is a THREE-element array
//!   x = (a, b)     without parens `x = a, b` parses as `(x = a), b`
//!   x ? (a, b) : c  a bare comma branch would be captured by the statement
//!   !(a, b)        without parens `!a, b` parses as `(!a), b`
//! ```
//!
//! The two contexts where a bare sequence is legal — and therefore printed
//! **without** parens — are a statement-position expression (`a, b, c;`) and a
//! computed-member key (`obj[a, b]`, which the surrounding `[ ]` delimits).
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (upstream lex/parses
//! source). The emitter is the unit under test here — the bridge conversion of
//! the comma operator (CLOC12.160 PR2, gap-161) is exercised separately in
//! `javascript-parser`.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    AssignmentExpression, AssignmentOperator, AssignmentTarget, CallExpression, ConditionalExpression,
    Expression, ExpressionStatement, Identifier, MemberExpression, Program, ProgramItem,
    SequenceExpression, SourceType, Statement, UnaryExpression, UnaryOperator,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn ident(name: &str) -> Expression {
    Expression::Identifier(Identifier { cv: None, name: name.to_string() })
}

fn seq(operands: Vec<Expression>) -> Expression {
    Expression::SequenceExpression(SequenceExpression { cv: None, expressions: operands })
}

fn call(callee: Expression, arguments: Vec<Expression>) -> Expression {
    Expression::CallExpression(CallExpression {
        cv: None,
        callee: Box::new(callee),
        arguments,
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
        "comma-operator emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — bare positions (statement, computed-member key)
// =====================================================================

/// `assertPrintSame("a,b,c")` — a sequence at statement position prints bare;
/// nothing captures it.
#[test]
fn sequence_at_statement_is_bare() {
    assert_emits(seq(vec![ident("a"), ident("b"), ident("c")]), "a,b,c;");
}

/// A two-operand sequence at statement position: `a,b`.
#[test]
fn sequence_two_operands_at_statement() {
    assert_emits(seq(vec![ident("a"), ident("b")]), "a,b;");
}

/// A sequence as a computed-member key needs NO parens — the `[ ]` already
/// delimits a full expression: `a[b,c]` (evaluates the key to `c`).
#[test]
fn sequence_as_computed_member_key_is_bare() {
    let e = Expression::MemberExpression(MemberExpression {
        cv: None,
        object: Box::new(ident("a")),
        property: Box::new(seq(vec![ident("b"), ident("c")])),
        computed: true,
    });
    assert_emits(e, "a[b,c];");
}

// =====================================================================
// Active — assignment-position wraps
// =====================================================================

/// `f((a, b))` — a sole sequence argument MUST wrap, or `f(a,b)` would be a
/// two-argument call instead of one sequence argument.
#[test]
fn sequence_as_sole_call_arg_is_wrapped() {
    assert_emits(call(ident("f"), vec![seq(vec![ident("a"), ident("b")])]), "f((a,b));");
}

/// `f((a, b), c)` — the arity is preserved; never the three-argument
/// `f(a,b,c)`.
#[test]
fn sequence_as_call_arg_preserves_arity() {
    let e = call(ident("f"), vec![seq(vec![ident("a"), ident("b")]), ident("c")]);
    assert_emits(e, "f((a,b),c);");
}

/// `[(a, b), c]` — a sequence array element wraps, or the element count
/// changes; never the three-element `[a,b,c]`.
#[test]
fn sequence_as_array_element_is_wrapped() {
    let e = Expression::ArrayExpression(coding_adventures_javascript_ast::ArrayExpression {
        cv: None,
        elements: vec![Some(seq(vec![ident("a"), ident("b")])), Some(ident("c"))],
    });
    assert_emits(e, "[(a,b),c];");
}

/// `x = (a, b)` — a sequence assignment RHS wraps; a bare `x=a,b` reparses as
/// `(x=a),b`.
#[test]
fn sequence_as_assignment_rhs_is_wrapped() {
    let e = Expression::AssignmentExpression(AssignmentExpression {
        cv: None,
        operator: AssignmentOperator::Eq,
        left: AssignmentTarget::Identifier(Identifier { cv: None, name: "x".to_string() }),
        right: Box::new(seq(vec![ident("a"), ident("b")])),
    });
    assert_emits(e, "x=(a,b);");
}

/// `x ? (a, b) : c` — a sequence conditional branch wraps (the branch is an
/// assignment-position expression).
#[test]
fn sequence_as_conditional_branch_is_wrapped() {
    let e = Expression::ConditionalExpression(ConditionalExpression {
        cv: None,
        test: Box::new(ident("x")),
        consequent: Box::new(seq(vec![ident("a"), ident("b")])),
        alternate: Box::new(ident("c")),
    });
    assert_emits(e, "x?(a,b):c;");
}

/// `!(a, b)` — a sequence unary operand wraps; a bare `!a,b` parses as
/// `(!a),b`.
#[test]
fn sequence_as_unary_operand_is_wrapped() {
    let e = Expression::UnaryExpression(UnaryExpression {
        cv: None,
        operator: UnaryOperator::Not,
        prefix: true,
        argument: Box::new(seq(vec![ident("a"), ident("b")])),
    });
    assert_emits(e, "!(a,b);");
}
