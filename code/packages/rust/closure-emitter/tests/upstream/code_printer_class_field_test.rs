//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **class-field** printing cases — a
//! `PropertyDefinition` member (`ClassMember::Field`), the non-method class
//! member. This is the twenty-second CodePrinter port into `closure-emitter`
//! (companion to the class-*expression* port `code_printer_class_test.rs` and
//! the class-*declaration* port `code_printer_class_declaration_test.rs`) and
//! isolates `emit_class_field` + the shared `emit_class_tail` member loop that
//! grew a `Field` arm with `ClassMember::Field` (CLOC12.175 PR1).
//!
//! # How the emitter prints a class field (recap)
//!
//! Inside a class body a field prints `[static ]key[=value];`:
//!
//! ```text
//!   x = 1        → x=1;
//!   y            → y;
//!   static z = 2 → static z=2;
//!   [k] = v      → [k]=v;
//! ```
//!
//! Two differences from a *method* member: a field **ends with `;`** (a method's
//! `}` is self-terminating; a field has no brace so the `;` separates it from the
//! next member), and it has **no** parameter-list/body tail. The `static` prefix
//! and computed-key bracketing reuse the same helpers as a method; the
//! initializer, when present, is emitted at `PREC_ASSIGNMENT` (the RHS of `=`),
//! so a looser bare `a,b` sequence wraps while an ordinary expression prints bare.
//! A **bare** field (`y;`) has no initializer and emits just `key;`.
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (the emitter is the unit
//! under test). The bridge conversion of a field lands in CLOC12.175 PR2 and is
//! exercised separately in `javascript-parser`; here the emitter is driven from
//! hand-constructed AST so this port does not depend on the bridge. Building the
//! AST directly also lets the port exercise field shapes the grammar/bridge
//! cannot yet parse (computed / numeric / string keys, a static computed key, a
//! sequence initializer that must wrap — see `CLOC12-gaps.md`).

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    ClassDeclaration, ClassMember, Declaration, Expression, Identifier, MethodDefinition, MethodKind,
    NumericLiteral, Program, ProgramItem, PropertyDefinition, PropertyKey, SequenceExpression,
    SourceType, StringLiteral,
};
use coding_adventures_javascript_ast::{BlockStatement, FunctionExpression};
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

/// A `PropertyKey::Identifier` key.
fn ident_key(name: &str) -> PropertyKey {
    PropertyKey::Identifier(Identifier { cv: None, name: name.to_string() })
}

/// Build one class **field** member.
fn field(key: PropertyKey, value: Option<Expression>, computed: bool, is_static: bool) -> ClassMember {
    ClassMember::Field(PropertyDefinition { cv: None, key, value, computed, is_static })
}

/// A no-op method value `(){}` (no params, empty body).
fn plain_method_value() -> FunctionExpression {
    FunctionExpression {
        cv: None,
        id: None,
        params: vec![],
        body: BlockStatement { cv: None, body: vec![] },
        generator: false,
        is_async: false,
    }
}

/// Build one plain method member (used only to prove field/method interleave).
fn method(name: &str) -> ClassMember {
    ClassMember::Method(MethodDefinition {
        cv: None,
        key: ident_key(name),
        kind: MethodKind::Method,
        value: plain_method_value(),
        computed: false,
        is_static: false,
    })
}

/// Emit `class C{<members>}` as a top-level declaration, returning the code.
fn emit_body(body: Vec<ClassMember>) -> String {
    let decl = Declaration::ClassDeclaration(ClassDeclaration {
        cv: None,
        id: Identifier { cv: None, name: "C".to_string() },
        super_class: None,
        body,
    });
    let prog = Program::new_untraced(EsVersion::Es2025, SourceType::Module)
        .with_body(vec![ProgramItem::Declaration(decl)]);
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    emit(&prog, &sidecar, &mut cv, &EmitOptions::default())
        .expect("emit failed")
        .code
}

/// Upstream `assertPrint(input, expected)` reshaped: emit a single-field (or
/// multi-member) class `C` and assert the emitted code equals `expected`.
fn assert_emits(body: Vec<ClassMember>, expected: &str) {
    let code = emit_body(body);
    assert_eq!(
        code, expected,
        "class-field emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — an initialized field prints `key=value;`
// =====================================================================

/// `class C{x=1;}` — an identifier key with a numeric initializer. The `=` has
/// no surrounding spaces and the member terminates with `;`.
#[test]
fn field_with_initializer() {
    assert_emits(vec![field(ident_key("x"), Some(num(1.0, "1")), false, false)], "class C{x=1;}");
}

/// The initializer is an identifier reference — `class C{x=y;}`.
#[test]
fn field_with_identifier_initializer() {
    assert_emits(vec![field(ident_key("x"), Some(ident("y")), false, false)], "class C{x=y;}");
}

// =====================================================================
// Active — a bare field prints `key;` (no `=`, no value)
// =====================================================================

/// `class C{y;}` — a bare field (no initializer) prints just the key and the
/// terminating `;`.
#[test]
fn bare_field_has_no_equals() {
    assert_emits(vec![field(ident_key("y"), None, false, false)], "class C{y;}");
}

/// The bare shape is exactly the emitted string — a regression guard that a
/// value-less field emits no stray `=`.
#[test]
fn bare_field_emits_no_equals_char() {
    let code = emit_body(vec![field(ident_key("y"), None, false, false)]);
    assert_eq!(code, "class C{y;}");
    assert!(!code.contains('='), "a bare field must not emit `=`, got {code:?}");
}

// =====================================================================
// Active — `static` prefix
// =====================================================================

/// `class C{static z=2;}` — a static field prints the `static` keyword (with a
/// space) before the key.
#[test]
fn static_field() {
    assert_emits(vec![field(ident_key("z"), Some(num(2.0, "2")), false, true)], "class C{static z=2;}");
}

/// `class C{static z;}` — a static *bare* field.
#[test]
fn static_bare_field() {
    assert_emits(vec![field(ident_key("z"), None, false, true)], "class C{static z;}");
}

// =====================================================================
// Active — computed / literal keys
// =====================================================================

/// `class C{[k]=v;}` — a computed key is bracketed.
#[test]
fn computed_key_field() {
    assert_emits(
        vec![field(PropertyKey::Expression(Box::new(ident("k"))), Some(ident("v")), true, false)],
        "class C{[k]=v;}",
    );
}

/// `class C{static [k]=v;}` — `static` stacks before a computed key.
#[test]
fn static_computed_key_field() {
    assert_emits(
        vec![field(PropertyKey::Expression(Box::new(ident("k"))), Some(ident("v")), true, true)],
        "class C{static [k]=v;}",
    );
}

/// `class C{0=1;}` — a numeric-literal key prints the number bare.
#[test]
fn numeric_key_field() {
    assert_emits(
        vec![field(
            PropertyKey::NumericLiteral(NumericLiteral { cv: None, value: 0.0, raw: "0".to_string() }),
            Some(num(1.0, "1")),
            false,
            false,
        )],
        "class C{0=1;}",
    );
}

/// `class C{"a-b"=1;}` — a string key that is NOT a valid identifier stays
/// quoted (the emitter's quote-vs-bare choice, shared with object keys).
#[test]
fn string_key_field_stays_quoted() {
    assert_emits(
        vec![field(
            PropertyKey::StringLiteral(StringLiteral {
                cv: None,
                value: "a-b".to_string(),
                raw: "\"a-b\"".to_string(),
            }),
            Some(num(1.0, "1")),
            false,
            false,
        )],
        "class C{\"a-b\"=1;}",
    );
}

// =====================================================================
// Active — the initializer is emitted at PREC_ASSIGNMENT
// =====================================================================

/// `class C{x=(a,b);}` — a bare comma *sequence* initializer binds looser than
/// `PREC_ASSIGNMENT`, so it is wrapped to preserve the field-boundary (an
/// unwrapped `x=a,b` would parse the `,b` as a second field / syntax error).
#[test]
fn sequence_initializer_is_wrapped() {
    assert_emits(
        vec![field(
            ident_key("x"),
            Some(Expression::SequenceExpression(SequenceExpression {
                cv: None,
                expressions: vec![ident("a"), ident("b")],
            })),
            false,
            false,
        )],
        "class C{x=(a,b);}",
    );
}

// =====================================================================
// Active — fields interleave with methods and each other
// =====================================================================

/// `class C{x=1;m(){}}` — a field then a method, in source order, each printing
/// its own terminator (the field's `;`, the method's `}`).
#[test]
fn field_then_method() {
    assert_emits(
        vec![field(ident_key("x"), Some(num(1.0, "1")), false, false), method("m")],
        "class C{x=1;m(){}}",
    );
}

/// `class C{m(){}x=1;}` — a method then a field: the method's `}` abuts the
/// field key with no separator.
#[test]
fn method_then_field() {
    assert_emits(
        vec![method("m"), field(ident_key("x"), Some(num(1.0, "1")), false, false)],
        "class C{m(){}x=1;}",
    );
}

/// `class C{x=1;y;static z=2;}` — three fields back-to-back, each terminated by
/// its own `;`; the `static` prefix survives on the third.
#[test]
fn three_fields_back_to_back() {
    assert_emits(
        vec![
            field(ident_key("x"), Some(num(1.0, "1")), false, false),
            field(ident_key("y"), None, false, false),
            field(ident_key("z"), Some(num(2.0, "2")), false, true),
        ],
        "class C{x=1;y;static z=2;}",
    );
}
