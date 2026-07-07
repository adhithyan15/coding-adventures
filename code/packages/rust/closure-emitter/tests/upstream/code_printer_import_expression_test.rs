//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **dynamic `import()`** printing cases — the
//! dynamic-import call expression (`ImportExpression`). This is the twenty-second
//! CodePrinter port into `closure-emitter` (after core / declarations /
//! trailing-comma / numbers / string-escape / ascii-escape / object-literal /
//! function-expression / arrow-function / template / update / new / sequence /
//! tagged-template / spread / yield / await / this / super / new.target /
//! import.meta) and isolates `emit_import_expression` + the `PREC_PRIMARY`
//! classification that landed with `Expression::ImportExpression` (CLOC12.169).
//!
//! # How the emitter prints `import(x)` (recap)
//!
//! `import(x)` is the `import` keyword immediately followed by a *literal*
//! parenthesised argument — syntactically a call-like primary. Two facts drive
//! the printing:
//!
//!   1. **The specifier sits inside literal parens.** It is emitted at
//!      `PREC_ASSIGNMENT` — the same level as a call argument. Everything binding
//!      tighter than a comma prints bare; only a looser *sequence* (comma)
//!      specifier must wrap so its commas are not mistaken for an argument list.
//!   2. **The whole node is a `PREC_PRIMARY` leaf.** It is already atomic, so a
//!      tighter parent never wraps it: `import(x).then(f)` (member/call off the
//!      import) composes without extra parens.
//!
//! ```text
//!   import("m")        → import("m")        string specifier, bare
//!   import(x)          → import(x)          identifier specifier, bare
//!   import(a+b)        → import(a+b)        binary binds tighter than comma → bare
//!   import((a,b))      → import((a,b))      a sequence specifier must wrap
//!   import(x).then(f)  → import(x).then(f)  primary; member/call composes bare
//! ```
//!
//! Unlike `await` (a word-shaped unary that needs a separator before an operand),
//! no separator follows the keyword: the `(` abuts `import` directly (`import(x)`,
//! never `import (x)`).
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (the emitter is the unit
//! under test). The bridge conversion of `import(x)` (gap-170) lands in
//! CLOC12.169 PR2 and is exercised separately in `javascript-parser`; here the
//! emitter is driven from hand-constructed AST so this port does not depend on
//! the bridge.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BinaryExpression, BinaryOperator, CallExpression, Expression, ExpressionStatement, Identifier,
    ImportExpression, MemberExpression, Program, ProgramItem, SequenceExpression, SourceType,
    Statement, StringLiteral,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn ident(name: &str) -> Expression {
    Expression::Identifier(Identifier { cv: None, name: name.to_string() })
}

fn string_lit(value: &str) -> Expression {
    Expression::StringLiteral(StringLiteral {
        cv: None,
        value: value.to_string(),
        raw: format!("\"{value}\""),
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

fn sequence(expressions: Vec<Expression>) -> Expression {
    Expression::SequenceExpression(SequenceExpression { cv: None, expressions })
}

/// Build an `ImportExpression` — a dynamic `import(specifier)`.
fn import_expr(source: Expression) -> Expression {
    Expression::ImportExpression(ImportExpression { cv: None, source: Box::new(source) })
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
        "import() emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — the specifier prints at PREC_ASSIGNMENT inside the parens
// =====================================================================

/// `import("m")` — a string specifier prints bare inside the literal parens.
#[test]
fn import_string_specifier_is_bare() {
    assert_emits(import_expr(string_lit("m")), "import(\"m\");");
}

/// `import(x)` — an identifier specifier prints bare.
#[test]
fn import_identifier_specifier_is_bare() {
    assert_emits(import_expr(ident("x")), "import(x);");
}

/// `import(a+b)` — a binary specifier binds tighter than the comma, so it needs
/// no wrap inside the argument parens.
#[test]
fn import_binary_specifier_is_bare() {
    assert_emits(
        import_expr(binary(BinaryOperator::Add, ident("a"), ident("b"))),
        "import(a+b);",
    );
}

/// `import((a,b))` — a *sequence* specifier is looser than the argument comma,
/// so it must wrap to keep its commas from reading as an argument list.
#[test]
fn import_sequence_specifier_wraps() {
    assert_emits(
        import_expr(sequence(vec![ident("a"), ident("b")])),
        "import((a,b));",
    );
}

// =====================================================================
// Active — the whole node is a PREC_PRIMARY leaf (a tighter parent
// never wraps it)
// =====================================================================

/// `import(x).then(f)` — a member access + call off the import composes without
/// extra parens: `import(x)` is a `PREC_PRIMARY` node, already atomic.
#[test]
fn import_expression_member_call_composes_bare() {
    assert_emits(
        call(member(import_expr(ident("x")), "then"), vec![ident("f")]),
        "import(x).then(f);",
    );
}
