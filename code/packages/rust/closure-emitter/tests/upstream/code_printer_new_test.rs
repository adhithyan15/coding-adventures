//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **`new`-operator** printing cases — the
//! `new Ctor(args)` construction form. This is the twelfth CodePrinter port
//! into `closure-emitter` (after core / declarations / trailing-comma /
//! numbers / string-escape / ascii-escape / object-literal /
//! function-expression / arrow-function / template / update) and isolates
//! `emit_new` + the `PREC_PRIMARY` classification, the `new`-keyword space, and
//! the callee-with-call wrapping that landed with `Expression::NewExpression`
//! (CLOC12.159).
//!
//! ## How the emitter prints a `new` expression (recap)
//!
//! ```text
//!   new X()        → new X()          identifier callee, empty args
//!   new X(a, b)    → new X(a,b)       argument list
//!   new a.b.c()    → new a.b.c()      member-chain callee (a valid target)
//! ```
//!
//! The argument parens are **always** printed, so a no-argument node is emitted
//! canonically as `new X()` (never bare `new X`). Every emitted `new` is
//! therefore the *argumented* form and binds at member/call strength
//! (`PREC_PRIMARY`), so `new X().y` needs no extra parens.
//!
//! ## Two seams
//!
//! ```text
//!   new X()         the `new` keyword needs a space before an identifier/member
//!                   callee, or `newX` fuses into one identifier.
//!   new (f())()     the `new` target is a MemberExpression per grammar and
//!   new (a.b().c)() cannot BE a call — a callee whose member spine bottoms out
//!                   in a call is wrapped, or the appended `(args)` binds to the
//!                   inner call (`new f()()` = `(new f())()`, a different program).
//! ```
//!
//! ## Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (upstream lex/parses
//! source). The emitter is the unit under test here — the bridge conversion of
//! `new` (CLOC12.159 PR2, gap-160) is exercised separately in
//! `javascript-parser`.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    CallExpression, Expression, ExpressionStatement, Identifier, MemberExpression, NewExpression,
    Program, ProgramItem, SourceType, Statement,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn ident(name: &str) -> Expression {
    Expression::Identifier(Identifier { cv: None, name: name.to_string() })
}

fn member(object: Expression, property: &str) -> Expression {
    Expression::MemberExpression(MemberExpression {
        cv: None,
        object: Box::new(object),
        property: Box::new(ident(property)),
        computed: false,
    })
}

fn call(callee: Expression, arguments: Vec<Expression>) -> Expression {
    Expression::CallExpression(CallExpression {
        cv: None,
        callee: Box::new(callee),
        arguments,
    })
}

fn new_expr(callee: Expression, arguments: Vec<Expression>) -> Expression {
    Expression::NewExpression(NewExpression {
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
        "new-operator emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — core shapes
// =====================================================================

/// `assertPrintSame("new X()")` — identifier callee, empty argument list. The
/// `new` keeps a separating space so it does not fuse into `newX`.
#[test]
fn new_identifier_no_args() {
    assert_emits(new_expr(ident("X"), vec![]), "new X();");
}

/// `assertPrintSame("new X(a,b)")` — the argument list prints comma-separated
/// with no minified inter-argument space.
#[test]
fn new_with_args() {
    assert_emits(
        new_expr(ident("X"), vec![ident("a"), ident("b")]),
        "new X(a,b);",
    );
}

/// `assertPrintSame("new a.b.c()")` — a pure member-chain callee is a valid
/// `new` target and stays paren-free.
#[test]
fn new_member_chain_callee_not_wrapped() {
    let callee = member(member(ident("a"), "b"), "c");
    assert_emits(new_expr(callee, vec![]), "new a.b.c();");
}

/// A member operand as a `new` argument round-trips: `new X(a.b)`.
#[test]
fn new_with_member_arg() {
    assert_emits(
        new_expr(ident("X"), vec![member(ident("a"), "b")]),
        "new X(a.b);",
    );
}

// =====================================================================
// Active — callee-with-call wrapping
// =====================================================================

/// `new (f())()` — a call in the callee spine MUST be parenthesised, or the
/// appended `()` would bind to the inner call (`new f()()` = `(new f())()`, a
/// different program). The wrapping paren also removes the `new`-keyword space.
#[test]
fn new_call_callee_is_wrapped() {
    assert_emits(new_expr(call(ident("f"), vec![]), vec![]), "new(f())();");
}

/// `new (a.b().c)()` — the callee's member spine bottoms out in a call
/// (`a.b()`), so the whole target is wrapped.
#[test]
fn new_callee_with_call_in_member_spine_is_wrapped() {
    let callee = member(call(member(ident("a"), "b"), vec![]), "c");
    assert_emits(new_expr(callee, vec![]), "new(a.b().c)();");
}

// =====================================================================
// Active — precedence (an argumented `new` is member/call strength)
// =====================================================================

/// `new X(a).y` — an argumented `new` binds at member strength, so a member
/// parent needs NO extra parens: `new X(a).y`.
#[test]
fn argumented_new_as_member_object_not_wrapped() {
    let m = member(new_expr(ident("X"), vec![ident("a")]), "y");
    assert_emits(m, "new X(a).y;");
}

/// A no-argument `new X` is emitted canonically as `new X()` (the parens are
/// always printed), so as a member object it needs no wrap: `new X().y`.
#[test]
fn no_arg_new_as_member_object_prints_argumented() {
    let m = member(new_expr(ident("X"), vec![]), "y");
    assert_emits(m, "new X().y;");
}

/// `new` nests: the inner `new X()` is a valid target (not a call), so no wrap
/// is forced and the outer `new` keeps its keyword space: `new new X()()`.
#[test]
fn nested_new_inner_not_wrapped() {
    let inner = new_expr(ident("X"), vec![]);
    assert_emits(new_expr(inner, vec![]), "new new X()();");
}

/// A `new` result called as a function: `new X().m()` — a call whose callee is
/// a member on the argumented `new`. All bind at `PREC_PRIMARY`, so no wraps.
#[test]
fn call_on_new_member_is_paren_free() {
    let e = call(member(new_expr(ident("X"), vec![]), "m"), vec![]);
    assert_emits(e, "new X().m();");
}
