//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **import.meta** printing cases — the
//! `import.meta` module meta-property (`ImportMeta`). This is the twenty-first
//! CodePrinter port into `closure-emitter` (after core / declarations /
//! trailing-comma / numbers / string-escape / ascii-escape / object-literal /
//! function-expression / arrow-function / template / update / new / sequence /
//! tagged-template / spread / yield / await / this / super / new.target) and
//! isolates `emit_import_meta` + the `PREC_PRIMARY` classification that landed
//! with `Expression::ImportMeta` (CLOC12.168).
//!
//! # How the emitter prints `import.meta` (recap)
//!
//! `import.meta` is a *reserved-word primary* — the sibling of `new.target`, a
//! fixed three-token-plus-dot spelling that binds at the tightest level. The
//! `.meta` is part of the spelling, NOT a member access, so the whole thing is
//! one atomic leaf. The emitter prints the eleven characters `import.meta` and
//! never wraps it (in any parent) nor forces a paren around an operand (it
//! carries none).
//!
//! ```text
//!   import.meta        → import.meta        the bare meta-property
//!   import.meta.url    → import.meta.url    member object binds at primary → no parens
//!   f(import.meta)     → f(import.meta)     plain primary argument
//!   import.meta.a.b    → import.meta.a.b    member chains compose without parens
//!   import.meta.m()    → import.meta.m()    method call composes without parens
//!   import.meta+1      → import.meta+1      a binary parent leaves the primary bare
//! ```
//!
//! `import.meta` is *syntactically* legal only inside a module, but that is the
//! parser's concern — the emitter is a pure printer and prints whatever AST it
//! is handed. The cases below are hand-constructed to isolate the emitter's
//! leaf-primary handling, not to assert JS validity.
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (the emitter is the unit
//! under test). The bridge conversion of `import.meta` (gap-169) lands in
//! CLOC12.168 PR2 and is exercised separately in `javascript-parser`; here the
//! emitter is driven from hand-constructed AST so this port does not depend on
//! the bridge.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BinaryExpression, BinaryOperator, CallExpression, Expression, ExpressionStatement, Identifier,
    ImportMeta, MemberExpression, NumericLiteral, Program, ProgramItem, SourceType, Statement,
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

/// Build an `ImportMeta` — the `import.meta` meta-property.
fn import_meta_expr() -> Expression {
    Expression::ImportMeta(ImportMeta { cv: None })
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
        "import.meta emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — the surface shape
// =====================================================================

/// `import.meta` — the bare meta-property, printed as its eleven-character
/// spelling.
#[test]
fn import_meta_value_is_bare_spelling() {
    assert_emits(import_meta_expr(), "import.meta;");
}

// =====================================================================
// Active — `import.meta` as a primary composes without parens
// =====================================================================

/// `import.meta.url` — a member parent binds at primary strength; the
/// `import.meta` object needs no parens (the outer `.url` is a genuine member
/// access, distinct from the `.meta` that is part of the spelling).
#[test]
fn import_meta_member_object_is_bare() {
    assert_emits(member(import_meta_expr(), "url"), "import.meta.url;");
}

/// `f(import.meta)` — `import.meta` as a call argument is a plain primary
/// operand.
#[test]
fn import_meta_as_call_argument_is_bare() {
    assert_emits(call(ident("f"), vec![import_meta_expr()]), "f(import.meta);");
}

/// `import.meta.a.b` — member chains off `import.meta` compose without any
/// parens.
#[test]
fn import_meta_member_chain_is_bare() {
    assert_emits(member(member(import_meta_expr(), "a"), "b"), "import.meta.a.b;");
}

/// `import.meta.m()` — a method call `import.meta.m()` composes without parens
/// (member then call, both at primary strength).
#[test]
fn import_meta_method_call_is_bare() {
    assert_emits(call(member(import_meta_expr(), "m"), vec![]), "import.meta.m();");
}

// =====================================================================
// Active — the whole node's precedence (import.meta tags at PREC_PRIMARY)
// =====================================================================

/// `import.meta+1` — even a binary parent leaves the primary `import.meta` bare
/// on the left.
#[test]
fn import_meta_under_binary_parent_is_bare() {
    assert_emits(
        binary(BinaryOperator::Add, import_meta_expr(), num(1.0, "1")),
        "import.meta+1;",
    );
}
