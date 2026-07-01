//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **object-literal** printing cases —
//! `testObjectLit*` and the key-quoting behaviors — in the DEFAULT
//! (minified) mode. This is the fifth CodePrinter port into
//! `closure-emitter` (alongside the core / declarations / trailing-comma
//! / numbers / string-escape / ascii-escape ports) and isolates the
//! `emit_object` / `emit_property` / `emit_property_key` surface.
//!
//! ## How the emitter prints an object literal (recap)
//!
//! ```text
//!   {}                      empty object            → {}
//!   { a: 1 }                identifier key          → {a:1}
//!   { a: 1, b: 2 }          comma-separated, no ws  → {a:1,b:2}
//!   { "abc": 1 }            string key, ident-valid → {abc:1}   (quotes dropped)
//!   { "a-b": 1 }            string key, non-ident   → {"a-b":1} (quotes kept)
//!   { "1": 1 }              string key "1"          → {"1":1}   (kept: numeric-looking)
//!   { "__proto__": 1 }      the proto exception     → {"__proto__":1} (kept)
//!   { 1: 1 }                numeric literal key     → {1:1}
//!   { [a]: 1 }              computed key            → {[a]:1}
//!   { a }                   shorthand               → {a}
//! ```
//!
//! A string key drops its quotes ONLY when its decoded value is a valid
//! ASCII identifier name AND is not `__proto__` (whose bare form is the
//! prototype setter, a semantic change). An object literal at the START
//! of an expression statement is parenthesized — `({a:1});` — so it is
//! not mistaken for a block; every `assert_emits` below shows that.
//!
//! Getters/setters (`{get a(){}}`) and method shorthand (`{m(){}}`) are
//! NOT covered here: their values are `FunctionExpression`s, which the
//! Phase-1 emitter does not yet print. They join when function-expression
//! emission lands; no `#[ignore]` placeholder is added because the AST
//! cannot even represent the function body today.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    Expression, ExpressionStatement, Identifier, NumericLiteral, ObjectExpression, Program,
    ProgramItem, Property, PropertyKey, PropertyKind, SourceType, Statement, StringLiteral,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn ident(name: &str) -> Expression {
    Expression::Identifier(Identifier {
        cv: None,
        name: name.to_string(),
    })
}

fn num(v: f64) -> Expression {
    Expression::NumericLiteral(NumericLiteral {
        cv: None,
        value: v,
        raw: format!("{}", v as i64),
    })
}

/// A non-shorthand `Init` property `key: value`. The `computed` and
/// `shorthand` flags default to `false`; the two tests that need them
/// set the field explicitly on the returned `Property`.
fn prop(key: PropertyKey, value: Expression) -> Property {
    Property {
        cv: None,
        kind: PropertyKind::Init,
        key,
        value: Box::new(value),
        computed: false,
        shorthand: false,
        method: false,
    }
}

fn ident_key(name: &str) -> PropertyKey {
    PropertyKey::Identifier(Identifier {
        cv: None,
        name: name.to_string(),
    })
}

fn string_key(v: &str) -> PropertyKey {
    PropertyKey::StringLiteral(StringLiteral {
        cv: None,
        value: v.to_string(),
        raw: String::new(),
    })
}

fn numeric_key(v: f64) -> PropertyKey {
    PropertyKey::NumericLiteral(NumericLiteral {
        cv: None,
        value: v,
        raw: format!("{}", v as i64),
    })
}

fn object(properties: Vec<Property>) -> Expression {
    Expression::ObjectExpression(ObjectExpression {
        cv: None,
        properties,
    })
}

fn stmt(expr: Expression) -> ProgramItem {
    ProgramItem::Statement(Statement::expression_statement(ExpressionStatement {
        cv: None,
        expression: expr,
    }))
}

fn emit_default(expr: Expression) -> String {
    let prog = Program::new_untraced(EsVersion::Es2025, SourceType::Module).with_body(vec![stmt(expr)]);
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    emit(&prog, &sidecar, &mut cv, &EmitOptions::default())
        .expect("emit failed")
        .code
}

/// Upstream `assertPrint(input, expected)` reshaped: emit the object as a
/// single-statement program (parenthesized at statement start) and assert
/// the emitted code equals `expected`.
fn assert_emits(expr: Expression, expected: &str) {
    let code = emit_default(expr);
    assert_eq!(
        code, expected,
        "object emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — object-literal printing (minified default mode)
// =====================================================================

/// Upstream `testObjectLit`: the empty object.
#[test]
fn empty_object() {
    assert_emits(object(vec![]), "({});");
}

/// A single identifier-keyed property, no interior whitespace.
#[test]
fn single_identifier_key() {
    assert_emits(object(vec![prop(ident_key("a"), num(1.0))]), "({a:1});");
}

/// Multiple properties are comma-separated with no whitespace.
#[test]
fn multiple_properties() {
    assert_emits(
        object(vec![
            prop(ident_key("a"), num(1.0)),
            prop(ident_key("b"), num(2.0)),
        ]),
        "({a:1,b:2});",
    );
}

/// A string key whose value is a valid identifier drops its quotes.
#[test]
fn string_key_valid_identifier_drops_quotes() {
    assert_emits(object(vec![prop(string_key("abc"), num(1.0))]), "({abc:1});");
}

/// A reserved word is a valid identifier *name* — legal bare as a key.
#[test]
fn reserved_word_string_key_drops_quotes() {
    assert_emits(object(vec![prop(string_key("if"), num(1.0))]), "({if:1});");
}

/// A string key that is NOT a valid identifier keeps its quotes (bare
/// would be a `SyntaxError`).
#[test]
fn string_key_non_identifier_stays_quoted() {
    assert_emits(object(vec![prop(string_key("a-b"), num(1.0))]), "({\"a-b\":1});");
}

/// A numeric-looking string key (`"1"`) keeps its quotes — bare `1` would
/// be a numeric key, a different property.
#[test]
fn numeric_string_key_stays_quoted() {
    assert_emits(object(vec![prop(string_key("1"), num(1.0))]), "({\"1\":1});");
}

/// `"__proto__"` keeps its quotes even though it is a valid identifier:
/// the bare form is the prototype setter (B.3.1), a semantic change.
#[test]
fn proto_string_key_stays_quoted() {
    assert_emits(
        object(vec![prop(string_key("__proto__"), num(1.0))]),
        "({\"__proto__\":1});",
    );
}

/// A numeric *literal* key prints as the number.
#[test]
fn numeric_literal_key() {
    assert_emits(object(vec![prop(numeric_key(1.0), num(1.0))]), "({1:1});");
}

/// A computed key `[a]: 1` prints with brackets.
#[test]
fn computed_key() {
    let mut p = prop(PropertyKey::Expression(Box::new(ident("a"))), num(1.0));
    p.computed = true;
    assert_emits(object(vec![p]), "({[a]:1});");
}

/// A shorthand property prints only the key.
#[test]
fn shorthand_property() {
    let mut p = prop(ident_key("a"), ident("a"));
    p.shorthand = true;
    assert_emits(object(vec![p]), "({a});");
}

/// A nested object value prints recursively.
#[test]
fn nested_object() {
    assert_emits(
        object(vec![prop(
            ident_key("a"),
            object(vec![prop(ident_key("b"), num(1.0))]),
        )]),
        "({a:{b:1}});",
    );
}

/// String value keeps normal string emission inside a property.
#[test]
fn string_value_property() {
    assert_emits(
        object(vec![prop(
            ident_key("k"),
            Expression::StringLiteral(StringLiteral {
                cv: None,
                value: "v".to_string(),
                raw: "\"v\"".to_string(),
            }),
        )]),
        "({k:\"v\"});",
    );
}
