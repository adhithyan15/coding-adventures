//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **update-operator** printing cases — the
//! `++` / `--` increment/decrement forms, in both prefix (`++x`, `--x`) and
//! postfix (`x++`, `x--`) position. This is the eleventh CodePrinter port into
//! `closure-emitter` (after core / declarations / trailing-comma / numbers /
//! string-escape / ascii-escape / object-literal / function-expression /
//! arrow-function / template) and isolates `emit_update` + the `PREC_UNARY`
//! classification + the token-fusion seams that landed with
//! `Expression::UpdateExpression` (CLOC12.158).
//!
//! ## How the emitter prints an update expression (recap)
//!
//! ```text
//!   ++x     prefix increment    → ++x
//!   x++     postfix increment   → x++
//!   --x     prefix decrement    → --x
//!   x--     postfix decrement   → x--
//! ```
//!
//! An update is `PREC_UNARY`: loose enough to be parenthesised as an
//! exponentiation base (`(++x)**2` — a bare `++x**2` is a syntax error) and as
//! a member/call object (`(x++).y`, since `x++` is not a valid member base),
//! yet tight enough to print **bare** under a `!` / `typeof` parent (`!x++`,
//! `typeof x++`).
//!
//! ## Token-fusion seams
//!
//! `++`/`--` are maximal-munch tokens, so adjacency to a `+`/`-` must be
//! guarded or the pair mis-tokenises into a *different* program:
//!
//! ```text
//!   a - (--b)   must print   a- --b   (never a---b = (a--)-b)
//!   a + (++b)   must print   a+ ++b   (never a+++b = (a++)+b)
//!   (x++) + y   must print   x++ +y   (the trailing + of x++ meets the + op)
//! ```
//!
//! The binary/unary emitters handle these: `arg_starts_with_sign` reports a
//! prefix update's leading sign (right-seam), and the binary emitter's
//! output-tail check catches a postfix update ending in a sign (left-seam).
//!
//! ## Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (upstream lex/parses
//! source). The emitter is the unit under test here — the bridge conversion of
//! `++`/`--` (CLOC12.158 PR2) is exercised separately in `javascript-parser`.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BinaryExpression, BinaryOperator, Expression, ExpressionStatement, Identifier, MemberExpression,
    NumericLiteral, Program, ProgramItem, SourceType, Statement, UnaryExpression, UnaryOperator,
    UpdateExpression, UpdateOperator,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn ident(name: &str) -> Expression {
    Expression::Identifier(Identifier { cv: None, name: name.to_string() })
}

fn num(v: f64) -> Expression {
    Expression::NumericLiteral(NumericLiteral { cv: None, value: v, raw: format!("{}", v as i64) })
}

fn update(op: UpdateOperator, prefix: bool, arg: Expression) -> Expression {
    Expression::UpdateExpression(UpdateExpression {
        cv: None,
        operator: op,
        prefix,
        argument: Box::new(arg),
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

fn member(object: Expression, property: &str) -> Expression {
    Expression::MemberExpression(MemberExpression {
        cv: None,
        object: Box::new(object),
        property: Box::new(ident(property)),
        computed: false,
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
        "update-operator emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — the four core shapes
// =====================================================================

/// `assertPrintSame("++x")` — prefix increment prints operator-then-operand.
#[test]
fn prefix_increment() {
    assert_emits(update(UpdateOperator::Increment, true, ident("x")), "++x;");
}

/// `assertPrintSame("x++")` — postfix increment prints operand-then-operator.
#[test]
fn postfix_increment() {
    assert_emits(update(UpdateOperator::Increment, false, ident("x")), "x++;");
}

/// `assertPrintSame("--x")` — prefix decrement.
#[test]
fn prefix_decrement() {
    assert_emits(update(UpdateOperator::Decrement, true, ident("x")), "--x;");
}

/// `assertPrintSame("x--")` — postfix decrement.
#[test]
fn postfix_decrement() {
    assert_emits(update(UpdateOperator::Decrement, false, ident("x")), "x--;");
}

/// A member operand: `a.b++` — the postfix operator follows the whole member.
#[test]
fn postfix_increment_on_member() {
    assert_emits(update(UpdateOperator::Increment, false, member(ident("a"), "b")), "a.b++;");
}

// =====================================================================
// Active — precedence: an update is PREC_UNARY
// =====================================================================

/// `assertPrintSame("!x++")` — a `!` (unary) parent prints a postfix update
/// bare (`!(x++)` needs no parens).
#[test]
fn not_of_postfix_increment_is_bare() {
    let e = Expression::UnaryExpression(UnaryExpression {
        cv: None,
        operator: UnaryOperator::Not,
        prefix: true,
        argument: Box::new(update(UpdateOperator::Increment, false, ident("x"))),
    });
    assert_emits(e, "!x++;");
}

/// `assertPrintSame("typeof x++")` — a `typeof` parent keeps its space and
/// prints the update bare.
#[test]
fn typeof_of_postfix_increment_is_bare() {
    let e = Expression::UnaryExpression(UnaryExpression {
        cv: None,
        operator: UnaryOperator::TypeOf,
        prefix: true,
        argument: Box::new(update(UpdateOperator::Increment, false, ident("x"))),
    });
    assert_emits(e, "typeof x++;");
}

/// A postfix update as a member-access object is parenthesised — `x++` is not
/// a valid `MemberExpression` object, so `(x++).y`.
#[test]
fn postfix_update_as_member_object_is_wrapped() {
    assert_emits(
        member(update(UpdateOperator::Increment, false, ident("x")), "y"),
        "(x++).y;",
    );
}

/// A prefix update as an exponentiation base is parenthesised — a bare
/// `++x**2` is a syntax error, so `(++x)**2`.
#[test]
fn prefix_update_as_exponent_base_is_wrapped() {
    let e = binary(BinaryOperator::Exp, update(UpdateOperator::Increment, true, ident("x")), num(2.0));
    assert_emits(e, "(++x)**2;");
}

// =====================================================================
// Active — token-fusion seams
// =====================================================================

/// `a - (--b)` must print `a- --b`, never `a---b` (which JS reparses as
/// `(a--)-b`).
#[test]
fn prefix_decrement_after_minus_needs_space() {
    let e = binary(BinaryOperator::Sub, ident("a"), update(UpdateOperator::Decrement, true, ident("b")));
    assert_emits(e, "a- --b;");
}

/// `a + (++b)` must print `a+ ++b`, never `a+++b` (which JS reparses as
/// `(a++)+b`).
#[test]
fn prefix_increment_after_plus_needs_space() {
    let e = binary(BinaryOperator::Add, ident("a"), update(UpdateOperator::Increment, true, ident("b")));
    assert_emits(e, "a+ ++b;");
}

/// `(x++) + y` — the postfix `++` leaves the output ending in `+`, so the
/// following binary `+` needs a left-seam space (`x++ +y`).
#[test]
fn postfix_increment_before_plus_needs_space() {
    let e = binary(BinaryOperator::Add, update(UpdateOperator::Increment, false, ident("x")), ident("y"));
    assert_emits(e, "x++ +y;");
}

/// A postfix update as a plain `+` left operand with a non-sign right operand
/// still needs the left-seam space (`x++ *y` does NOT, but `x++ +y` does) —
/// pin the `-` variant too: `x-- -y`.
#[test]
fn postfix_decrement_before_minus_needs_space() {
    let e = binary(BinaryOperator::Sub, update(UpdateOperator::Decrement, false, ident("x")), ident("y"));
    assert_emits(e, "x-- -y;");
}

/// A `*` seam does NOT fuse (`x++*y` is unambiguous), so no space is added —
/// confirms the guard is scoped to `+`/`-` and does not over-space.
#[test]
fn postfix_increment_before_star_no_space() {
    let e = binary(BinaryOperator::Mul, update(UpdateOperator::Increment, false, ident("x")), ident("y"));
    assert_emits(e, "x++*y;");
}
