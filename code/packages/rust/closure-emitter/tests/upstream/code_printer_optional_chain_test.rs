//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **optional chaining** `a?.b` / `a?.[k]` / `a?.()`
//! printing cases (ES2020) — the `OptionalMemberExpression` /
//! `OptionalCallExpression` links and the transparent `ChainExpression` wrapper
//! (`javascript-ast` 0.30.0, CLOC12.171 PR1). This is the twenty-fourth
//! CodePrinter port into `closure-emitter`, and it isolates `emit_optional_member`
//! / `emit_optional_call` / `emit_chain`.
//!
//! # How the emitter prints an optional chain (recap)
//!
//! An optional link spells its access operator `?.`: `?.` before a dot name,
//! `?.[` before a computed key, `?.(` before call arguments. The whole spine is
//! wrapped once in a `ChainExpression`, which is **transparent** — it prints
//! only its inner expression. Two facts drive the printing:
//!
//!   1. **Each link keeps its own optionality.** Only the `?.`-marked link
//!      prints `?.`; a plain link that follows an optional one prints an
//!      ordinary `.` / `(` — so `a?.b.c` prints `a?.b.c` (not `a?.b?.c`).
//!   2. **The object/callee binds at `PREC_PRIMARY`.** A looser object keeps its
//!      parens (`(a||b)?.c`), and a looser *sequence* call argument wraps
//!      (`a?.((b,c))`).
//!
//! ```text
//!   a?.b        → a?.b        optional dot member
//!   a?.[b]      → a?.[b]      optional computed member
//!   a?.()       → a?.()       optional call, no args
//!   a?.b.c      → a?.b.c      optional link then a PLAIN link
//!   a?.b()      → a?.b()      plain call on an optional member
//!   (a||b)?.c   → (a||b)?.c   object keeps its parens
//!   a?.((b,c))  → a?.((b,c))  a sequence argument wraps
//! ```
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (the emitter is the unit
//! under test). The bridge conversion of `a?.b` (gap-OptionalChain) lands in
//! CLOC12.171 PR2 and is exercised separately in `javascript-parser`; here the
//! emitter is driven from hand-constructed AST so this port does not depend on
//! the bridge.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    CallExpression, ChainExpression, Expression, ExpressionStatement, Identifier, LogicalExpression,
    LogicalOperator, MemberExpression, OptionalCallExpression, OptionalMemberExpression, Program,
    ProgramItem, SequenceExpression, SourceType, Statement,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn ident(name: &str) -> Expression {
    Expression::Identifier(Identifier { cv: None, name: name.to_string() })
}

/// `obj?.prop` (dot) or `obj?.[prop]` (computed) — an optional member access.
fn opt_member(object: Expression, prop: &str, computed: bool) -> Expression {
    Expression::OptionalMemberExpression(OptionalMemberExpression {
        cv: None,
        object: Box::new(object),
        property: Box::new(ident(prop)),
        computed,
    })
}

/// `callee?.(args)` — an optional call.
fn opt_call(callee: Expression, arguments: Vec<Expression>) -> Expression {
    Expression::OptionalCallExpression(OptionalCallExpression {
        cv: None,
        callee: Box::new(callee),
        arguments,
    })
}

/// A plain `object.prop` member access (used for the non-optional link that
/// follows an optional one).
fn member(object: Expression, prop: &str) -> Expression {
    Expression::MemberExpression(MemberExpression {
        cv: None,
        object: Box::new(object),
        property: Box::new(ident(prop)),
        computed: false,
    })
}

/// The transparent chain-boundary wrapper.
fn chain(inner: Expression) -> Expression {
    Expression::ChainExpression(ChainExpression { cv: None, expression: Box::new(inner) })
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

/// Upstream `assertPrint(input, expected)` reshaped: emit the (chain-wrapped)
/// expression as a single-statement program and assert the emitted code equals
/// `expected`.
fn assert_emits(expr: Expression, expected: &str) {
    let code = emit_default(expr);
    assert_eq!(
        code, expected,
        "optional-chain emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — the optional access operator prints `?.`
// =====================================================================

/// `a?.b` — an optional dot member.
#[test]
fn optional_member_dot() {
    assert_emits(chain(opt_member(ident("a"), "b", false)), "a?.b;");
}

/// `a?.[b]` — an optional computed member.
#[test]
fn optional_member_computed() {
    assert_emits(chain(opt_member(ident("a"), "b", true)), "a?.[b];");
}

/// `a?.()` — an optional call with no arguments; `a?.(b)` — with one.
#[test]
fn optional_call() {
    assert_emits(chain(opt_call(ident("a"), vec![])), "a?.();");
    assert_emits(chain(opt_call(ident("a"), vec![ident("b")])), "a?.(b);");
}

// =====================================================================
// Active — only the `?.`-marked link is optional; the wrapper is transparent
// =====================================================================

/// `a?.b.c` — the `.c` that follows the optional `?.b` prints as a PLAIN dot,
/// and the `ChainExpression` wrapper adds no syntax of its own.
#[test]
fn optional_then_plain_link_prints_plain_dot() {
    let inner = member(opt_member(ident("a"), "b", false), "c");
    assert_emits(chain(inner), "a?.b.c;");
}

/// `a?.b()` — a PLAIN call on an optional member.
#[test]
fn plain_call_on_optional_member() {
    let called = Expression::CallExpression(CallExpression {
        cv: None,
        callee: Box::new(opt_member(ident("a"), "b", false)),
        arguments: vec![],
    });
    assert_emits(chain(called), "a?.b();");
}

// =====================================================================
// Active — object/argument precedence
// =====================================================================

/// `(a||b)?.c` — the object binds at `PREC_PRIMARY`, so a looser logical object
/// keeps its parens.
#[test]
fn optional_member_object_below_primary_is_parenthesised() {
    let or = Expression::LogicalExpression(LogicalExpression {
        cv: None,
        operator: LogicalOperator::Or,
        left: Box::new(ident("a")),
        right: Box::new(ident("b")),
    });
    assert_emits(chain(opt_member(or, "c", false)), "(a||b)?.c;");
}

/// `a?.((b,c))` — a looser *sequence* argument must wrap, exactly as a plain
/// call argument does; a bare `a?.(b,c)` would be a two-argument call.
#[test]
fn optional_call_sequence_argument_wraps() {
    let seq = Expression::SequenceExpression(SequenceExpression {
        cv: None,
        expressions: vec![ident("b"), ident("c")],
    });
    assert_emits(chain(opt_call(ident("a"), vec![seq])), "a?.((b,c));");
}
