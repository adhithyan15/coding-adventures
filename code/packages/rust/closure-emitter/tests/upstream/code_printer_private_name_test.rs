//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **private class-member name** printing cases — a
//! `#x` key on a class field or a `#m()` key on a class method
//! (`PropertyKey::PrivateName`, ESTree's `PrivateIdentifier`). This is the
//! twenty-fourth CodePrinter port into `closure-emitter` (companion to the
//! class-*field* port `code_printer_class_field_test.rs`) and isolates the
//! `PrivateName` arm of `emit_property_key` that landed with
//! `PropertyKey::PrivateName` (CLOC12.177 PR1).
//!
//! # How the emitter prints a private name (recap)
//!
//! A private name prints `#` followed by the stored bare name:
//!
//! ```text
//!   #x = 1        → #x=1;
//!   #x            → #x;
//!   static #x = 1 → static #x=1;
//!   #m(){}        → #m(){}
//! ```
//!
//! The stored [`PrivateName::name`] omits the leading `#` (mirroring
//! [`Identifier`]), so the emitter prepends it. No quote/shorten logic applies —
//! a private name is a hard token boundary, unlike a string key. As a **field**
//! key it terminates with `;`; as a **method** key it is brace-terminated. The
//! `static` prefix stacks before the `#` exactly as for a public key.
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (the emitter is the unit
//! under test). The bridge conversion of a private *field* landed in CLOC12.177
//! PR2 and is exercised separately in `javascript-parser` + a `closurec` e2e
//! fixture; a private *method* bridge is a later slice. Building the AST directly
//! lets this port exercise the private-method key shape today regardless.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BlockStatement, ClassDeclaration, ClassMember, Declaration, Expression, FunctionExpression,
    Identifier, MethodDefinition, MethodKind, NumericLiteral, PrivateName, Program, ProgramItem,
    PropertyDefinition, PropertyKey, SourceType,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn num(value: f64, raw: &str) -> Expression {
    Expression::NumericLiteral(NumericLiteral { cv: None, value, raw: raw.to_string() })
}

/// A `PropertyKey::PrivateName` key (`#name`), name stored WITHOUT the `#`.
fn private_key(name: &str) -> PropertyKey {
    PropertyKey::PrivateName(PrivateName { cv: None, name: name.to_string() })
}

/// A `PropertyKey::Identifier` key.
fn ident_key(name: &str) -> PropertyKey {
    PropertyKey::Identifier(Identifier { cv: None, name: name.to_string() })
}

/// Build one class **field** member with the given key.
fn field(key: PropertyKey, value: Option<Expression>, is_static: bool) -> ClassMember {
    ClassMember::Field(PropertyDefinition { cv: None, key, value, computed: false, is_static })
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

/// Build one method member with the given key.
fn method(key: PropertyKey) -> ClassMember {
    ClassMember::Method(MethodDefinition {
        cv: None,
        key,
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

/// Upstream `assertPrint(input, expected)` reshaped: emit a class `C` with the
/// given members and assert the emitted code equals `expected`.
fn assert_emits(body: Vec<ClassMember>, expected: &str) {
    let code = emit_body(body);
    assert_eq!(
        code, expected,
        "private-name emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — a private field prints `#name=value;`
// =====================================================================

/// `class C{#x=1;}` — a private key prepends `#` to the stored bare name.
#[test]
fn private_field_with_initializer() {
    assert_emits(vec![field(private_key("x"), Some(num(1.0, "1")), false)], "class C{#x=1;}");
}

/// `class C{#x;}` — a bare private field: just `#name` and the terminator.
#[test]
fn bare_private_field() {
    assert_emits(vec![field(private_key("x"), None, false)], "class C{#x;}");
}

/// The stored name omits the `#` but the emitted form has exactly one — a
/// regression guard against a doubled `##` or a missing `#`.
#[test]
fn private_field_emits_single_hash() {
    let code = emit_body(vec![field(private_key("x"), Some(num(1.0, "1")), false)]);
    assert!(code.contains("#x"), "expected `#x`, got {code:?}");
    assert!(!code.contains("##"), "private name doubled the `#`: {code:?}");
}

// =====================================================================
// Active — `static` stacks before the `#`
// =====================================================================

/// `class C{static #x=1;}` — the `static` keyword (with a space) precedes the
/// `#` private key.
#[test]
fn static_private_field() {
    assert_emits(vec![field(private_key("x"), Some(num(1.0, "1")), true)], "class C{static #x=1;}");
}

// =====================================================================
// Active — a private method key prints `#name(){}`
// =====================================================================

/// `class C{#m(){}}` — a private method key also prints `#`; the member is
/// brace-terminated (no `;`).
#[test]
fn private_method() {
    assert_emits(vec![method(private_key("m"))], "class C{#m(){}}");
}

// =====================================================================
// Active — private and public members interleave
// =====================================================================

/// `class C{#x=1;m(){}}` — a private field then a public method, each printing
/// its own terminator (the field's `;`, the method's `}`).
#[test]
fn private_field_then_public_method() {
    assert_emits(
        vec![field(private_key("x"), Some(num(1.0, "1")), false), method(ident_key("m"))],
        "class C{#x=1;m(){}}",
    );
}

/// `class C{x=1;#y=2;}` — a public field then a private field: each key prints
/// with the right form, `;`-separated.
#[test]
fn public_field_then_private_field() {
    assert_emits(
        vec![
            field(ident_key("x"), Some(num(1.0, "1")), false),
            field(private_key("y"), Some(num(2.0, "2")), false),
        ],
        "class C{x=1;#y=2;}",
    );
}
