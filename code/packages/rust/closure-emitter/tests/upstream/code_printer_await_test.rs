//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **await** printing cases — the async-suspend
//! `AwaitExpression` `await x`. This is the seventeenth CodePrinter port into
//! `closure-emitter` (after core / declarations / trailing-comma / numbers /
//! string-escape / ascii-escape / object-literal / function-expression /
//! arrow-function / template / update / new / sequence / tagged-template /
//! spread / yield) and isolates `emit_await` + the `PREC_UNARY` classification
//! that landed with `Expression::AwaitExpression` (CLOC12.164).
//!
//! # How the emitter prints an await (recap)
//!
//! `await` is a *word-shaped* unary operator, printed exactly like the
//! word-unaries `typeof` / `void` / `delete`: the keyword, a mandatory
//! separator, then the operand at `PREC_UNARY`.
//!
//! ```text
//!   await p          → await p        mandatory keyword↔operand space
//!   await a.b        → await a.b      member operand binds tighter → bare
//!   await f()        → await f()      call operand binds tighter → bare
//!   await (a+b)      → await (a+b)    binary operand binds LOOSER → wraps
//!   await p+1        → await p+1      the whole await binds tighter than + → (await p)+1
//!   (await p).x      → (await p).x    member parent wraps the await
//!   (await f)()      → (await f)()    call callee wraps the await
//!   (await p)**2     → (await p)**2   exponentiation base must wrap (bare `await p**2` is a syntax error)
//! ```
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly. The emitter is the unit
//! under test here — the bridge conversion of `await` (gap-165) is deferred:
//! the current grammar treats `await` inside an async body as a plain
//! identifier, so `async function f(){await p}` does not yet parse. Until that
//! grammar work lands, PR1's node + this port exercise the emitter via
//! hand-constructed AST, the same staging used for the substitution-template
//! slice (gap-157).

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    AwaitExpression, BinaryExpression, BinaryOperator, CallExpression, Expression,
    ExpressionStatement, Identifier, MemberExpression, NumericLiteral, Program, ProgramItem,
    SourceType, Statement,
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

/// Build an `AwaitExpression` (named `aw` — `await` is a Rust keyword).
fn aw(argument: Expression) -> Expression {
    Expression::AwaitExpression(AwaitExpression { cv: None, argument: Box::new(argument) })
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
        "await emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — the surface shape + operand precedence
// =====================================================================

/// `await p` — the keyword and operand are separated by a mandatory space.
#[test]
fn await_value_requires_space() {
    assert_emits(aw(ident("p")), "await p;");
}

/// `await a.b` — a member operand binds tighter than unary → bare.
#[test]
fn await_member_operand_is_bare() {
    assert_emits(aw(member(ident("a"), "b")), "await a.b;");
}

/// `await f()` — a call operand binds tighter than unary → bare.
#[test]
fn await_call_operand_is_bare() {
    assert_emits(aw(call(ident("f"), vec![])), "await f();");
}

/// `await (a+b)` — a **binary** operand binds looser than unary, so it wraps:
/// a bare `await a+b` would parse as `(await a)+b`.
#[test]
fn await_binary_operand_is_wrapped() {
    let e = aw(binary(BinaryOperator::Add, ident("a"), ident("b")));
    assert_emits(e, "await (a+b);");
}

// =====================================================================
// Active — the whole node's precedence (await tags at PREC_UNARY)
// =====================================================================

/// `await p+1` — the whole await binds tighter than `+`, so a binary parent
/// leaves it bare on the left: `(await p)+1`.
#[test]
fn await_binds_tighter_than_binary_parent() {
    assert_emits(binary(BinaryOperator::Add, aw(ident("p")), num(1.0, "1")), "await p+1;");
}

/// `(await p).x` — a member parent binds at primary strength and wraps the
/// looser await object.
#[test]
fn await_wrapped_as_member_object() {
    assert_emits(member(aw(ident("p")), "x"), "(await p).x;");
}

/// `(await f)()` — a call callee likewise wraps the await.
#[test]
fn await_wrapped_as_call_callee() {
    assert_emits(call(aw(ident("f")), vec![]), "(await f)();");
}

/// `(await p)**2` — the exponentiation **base** must wrap: a bare
/// `await p**2` is a JS syntax error (the `**` base may not be an unqualified
/// unary expression). The emitter requires `PREC_UNARY + 1` on the `**` base,
/// which parenthesises the unary-strength await.
#[test]
fn await_exponentiation_base_is_wrapped() {
    let e = binary(BinaryOperator::Exp, aw(ident("p")), num(2.0, "2"));
    assert_emits(e, "(await p)**2;");
}

/// `await await p` — a nested await operand prints bare (await is exactly at
/// the unary operand floor).
#[test]
fn await_nested_is_bare() {
    assert_emits(aw(aw(ident("p"))), "await await p;");
}
