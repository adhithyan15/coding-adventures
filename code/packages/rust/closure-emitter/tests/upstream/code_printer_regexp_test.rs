//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **regular-expression literal** printing cases —
//! the `Token.REGEXP` node (`RegExpLiteral`). This is the nineteenth CodePrinter
//! port into `closure-emitter` (after core / declarations / trailing-comma /
//! numbers / string-escape / ascii-escape / object-literal / function-expression
//! / arrow-function / template / update / new / sequence / tagged-template /
//! spread / yield / await / this) and isolates `emit_regexp` + the
//! `PREC_PRIMARY` classification that landed with `Expression::RegExpLiteral`
//! (CLOC12.172 PR1).
//!
//! # How the emitter prints a regex (recap)
//!
//! A regex literal has exactly **one spelling** — unlike a string there is no
//! quote-choice and no re-escaping pass. Upstream Closure's `CodeGenerator`
//! writes the delimiters plus the raw pattern and flags verbatim
//! (`node.getString()` for each half). The pattern body — groups, character
//! classes, quantifiers, anchors, backreferences, an escaped `\/`, a `/` inside
//! a `[...]` class — is opaque text the printer never touches. Our
//! `emit_regexp` reconstructs `/{pattern}/{flags}` with the same verbatim
//! policy.
//!
//! ```text
//!   /ab+c/gi    → /ab+c/gi     pattern + flags round-trip
//!   /a.b/       → /a.b/        no flags → bare `/…/`
//!   /(?:a|b)/   → /(?:a|b)/    groups/alternation are opaque
//!   /[/]/       → /[/]/        a `/` inside a class is opaque
//!   /re/.test(a)→ /re/.test(a) a regex is PREC_PRIMARY → member needs no parens
//!   f(/x/g,1)   → f(/x/g,1)    a regex flows through an argument list bare
//! ```
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (the emitter is the unit
//! under test). The bridge conversion of a REGEX token (gap-RegExpAsIdentifier)
//! lands in CLOC12.172 PR2 and is exercised separately in `javascript-parser`;
//! here the emitter is driven from hand-constructed AST so this port does not
//! depend on the bridge.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    CallExpression, Expression, ExpressionStatement, Identifier, MemberExpression, NumericLiteral,
    Program, ProgramItem, RegExpLiteral, SourceType, Statement,
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

fn call(callee: Expression, arguments: Vec<Expression>) -> Expression {
    Expression::CallExpression(CallExpression { cv: None, callee: Box::new(callee), arguments })
}

/// Build a `RegExpLiteral` from its split `pattern` / `flags` halves — the same
/// shape the bridge produces from a `/pattern/flags` token.
fn regexp(pattern: &str, flags: &str) -> Expression {
    Expression::RegExpLiteral(RegExpLiteral {
        cv: None,
        pattern: pattern.to_string(),
        flags: flags.to_string(),
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
        "regexp emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — the surface shape
// =====================================================================

/// `/ab+c/gi` — pattern and both flags round-trip verbatim.
#[test]
fn regexp_pattern_and_flags_round_trip() {
    assert_emits(regexp("ab+c", "gi"), "/ab+c/gi;");
}

/// `/a.b/` — no flags produces a bare `/…/`.
#[test]
fn regexp_without_flags_is_bare_slashes() {
    assert_emits(regexp("a.b", ""), "/a.b/;");
}

// =====================================================================
// Active — the pattern body is opaque text
// =====================================================================

/// `/(?:a|b)/` — a non-capturing group with alternation is printed verbatim.
#[test]
fn regexp_group_and_alternation_are_opaque() {
    assert_emits(regexp("(?:a|b)", ""), "/(?:a|b)/;");
}

/// `/[a-z]/` — a character class is opaque text.
#[test]
fn regexp_character_class_is_opaque() {
    assert_emits(regexp("[a-z]", ""), "/[a-z]/;");
}

/// `/^\d+$/` — anchors, metacharacters and quantifiers are all opaque.
#[test]
fn regexp_anchors_and_quantifiers_are_opaque() {
    assert_emits(regexp("^\\d+$", ""), "/^\\d+$/;");
}

/// `/(a)\1/` — a capture group and a backreference are printed verbatim.
#[test]
fn regexp_backreference_is_opaque() {
    assert_emits(regexp("(a)\\1", ""), "/(a)\\1/;");
}

/// `/[/]/` — a `/` inside a character class is part of the opaque body; the
/// printer neither escapes it nor treats it as a delimiter.
#[test]
fn regexp_slash_inside_class_is_opaque() {
    assert_emits(regexp("[/]", ""), "/[/]/;");
}

/// `/\//` — an escaped closing delimiter in the body stays exactly as given.
#[test]
fn regexp_escaped_delimiter_is_preserved() {
    assert_emits(regexp("\\/", ""), "/\\//;");
}

// =====================================================================
// Active — flags are echoed verbatim (the printer is not a validator)
// =====================================================================

/// `/x/dgimsuy` — the full ES flag set is preserved in the given order.
#[test]
fn regexp_full_flag_set_preserved() {
    assert_emits(regexp("x", "dgimsuy"), "/x/dgimsuy;");
}

/// `/x/ig` — a non-canonical flag order is echoed as-is; the printer does not
/// reorder or normalise flags.
#[test]
fn regexp_flag_order_is_echoed_verbatim() {
    assert_emits(regexp("x", "ig"), "/x/ig;");
}

// =====================================================================
// Active — the whole node's precedence (regex tags at PREC_PRIMARY)
// =====================================================================

/// `/re/.test(a)` — a regex is a primary expression, so a member access on it
/// (and the enclosing call) needs no parentheses.
#[test]
fn regexp_as_member_base_is_paren_free() {
    assert_emits(call(member(regexp("re", ""), "test"), vec![ident("a")]), "/re/.test(a);");
}

/// `/re/g.source` — a flag on the regex does not change the paren-free member
/// access; the whole `/re/g` still binds at primary strength.
#[test]
fn regexp_with_flag_as_member_base_is_paren_free() {
    assert_emits(member(regexp("re", "g"), "source"), "/re/g.source;");
}

/// `f(/ab+c/gi,1)` — a regex flows through an argument list verbatim, bare,
/// beside other arguments.
#[test]
fn regexp_as_call_argument_is_bare() {
    assert_emits(call(ident("f"), vec![regexp("ab+c", "gi"), num(1.0, "1")]), "f(/ab+c/gi,1);");
}
