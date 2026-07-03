//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **tagged-template** printing cases — the
//! `TaggedTemplateExpression` `` tag`...` ``. This is the fourteenth CodePrinter
//! port into `closure-emitter` (after core / declarations / trailing-comma /
//! numbers / string-escape / ascii-escape / object-literal / function-expression
//! / arrow-function / template / update / new / sequence) and isolates
//! `emit_tagged_template` + the `PREC_PRIMARY` classification that landed with
//! `Expression::TaggedTemplateExpression` (CLOC12.161).
//!
//! # How the emitter prints a tagged template (recap)
//!
//! ```text
//!   tag`abc`           → tag`abc`         the tag callee, then the template,
//!                                          with NO separator seam
//!   String.raw`a${x}b` → String.raw`a${x}b`
//! ```
//!
//! The tag is emitted at `PREC_PRIMARY`, and the node itself is `PREC_PRIMARY`,
//! so:
//!
//! ```text
//!   a`x`.length     the tagged template is a paren-free member object
//!   (a,b)`x`        a LOOSER tag (a sequence) wraps, or it would tag only `b`
//! ```
//!
//! The quasi is an ordinary `TemplateLiteral` (CLOC12.154) — a tagged template
//! is structurally "an expression applied to a template", so the `${…}`
//! substitutions round-trip through the same `emit_template_literal` the
//! untagged template uses.
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (upstream lex/parses
//! source). The emitter is the unit under test here — the bridge conversion of
//! the tagged-template form (CLOC12.161 PR2, gap-162) is exercised separately in
//! `javascript-parser`.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    Expression, ExpressionStatement, Identifier, MemberExpression, Program, ProgramItem,
    SequenceExpression, SourceType, Statement, TaggedTemplateExpression, TemplateElement,
    TemplateLiteral,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn ident(name: &str) -> Expression {
    Expression::Identifier(Identifier { cv: None, name: name.to_string() })
}

fn quasi(raw: &str, tail: bool) -> TemplateElement {
    TemplateElement { cv: None, raw: raw.to_string(), cooked: Some(raw.to_string()), tail }
}

/// A raw `TemplateLiteral` struct (not wrapped in `Expression`) for use as the
/// `quasi` of a tagged template.
fn raw_template(quasis: Vec<TemplateElement>, expressions: Vec<Expression>) -> TemplateLiteral {
    TemplateLiteral { cv: None, quasis, expressions }
}

fn tagged(tag: Expression, quasi: TemplateLiteral) -> Expression {
    Expression::TaggedTemplateExpression(TaggedTemplateExpression {
        cv: None,
        tag: Box::new(tag),
        quasi,
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

fn seq(operands: Vec<Expression>) -> Expression {
    Expression::SequenceExpression(SequenceExpression { cv: None, expressions: operands })
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
        "tagged-template emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — no-substitution tags
// =====================================================================

/// `` tag`abc` `` — an identifier tag directly precedes a no-substitution
/// template; the backtick abuts the tag with no separator.
#[test]
fn tagged_identifier_no_substitution() {
    assert_emits(tagged(ident("tag"), raw_template(vec![quasi("abc", true)], vec![])), "tag`abc`;");
}

/// An empty no-substitution template still round-trips: `` tag`` ``.
#[test]
fn tagged_empty_template() {
    assert_emits(tagged(ident("tag"), raw_template(vec![quasi("", true)], vec![])), "tag``;");
}

/// `` a.b`x` `` — a member-chain tag stays paren-free (member binds at
/// `PREC_PRIMARY`, the same as the tagged-template node).
#[test]
fn tagged_member_tag_not_wrapped() {
    let e = tagged(member(ident("a"), "b"), raw_template(vec![quasi("x", true)], vec![]));
    assert_emits(e, "a.b`x`;");
}

/// `` a.b.c`x` `` — a deeper member chain is still a paren-free tag.
#[test]
fn tagged_deep_member_tag_not_wrapped() {
    let tag = member(member(ident("a"), "b"), "c");
    assert_emits(tagged(tag, raw_template(vec![quasi("x", true)], vec![])), "a.b.c`x`;");
}

// =====================================================================
// Active — substitution tags
// =====================================================================

/// `` String.raw`a${x}b` `` — a substitution template as the quasi: the `${…}`
/// parts round-trip through the reused template emitter.
#[test]
fn tagged_with_substitution() {
    let tag = member(ident("String"), "raw");
    let quasi = raw_template(vec![quasi("a", false), quasi("b", true)], vec![ident("x")]);
    assert_emits(tagged(tag, quasi), "String.raw`a${x}b`;");
}

/// A leading substitution: `` tag`${x}b` ``.
#[test]
fn tagged_leading_substitution() {
    let quasi = raw_template(vec![quasi("", false), quasi("b", true)], vec![ident("x")]);
    assert_emits(tagged(ident("tag"), quasi), "tag`${x}b`;");
}

/// Two adjacent substitutions: `` tag`${x}${y}` ``.
#[test]
fn tagged_adjacent_substitutions() {
    let quasi = raw_template(
        vec![quasi("", false), quasi("", false), quasi("", true)],
        vec![ident("x"), ident("y")],
    );
    assert_emits(tagged(ident("tag"), quasi), "tag`${x}${y}`;");
}

// =====================================================================
// Active — precedence (tagged template is PREC_PRIMARY)
// =====================================================================

/// A member access on a tagged template needs no parens — the tagged template
/// is `PREC_PRIMARY`: `` a`x`.length ``.
#[test]
fn member_on_tagged_is_paren_free() {
    let inner = tagged(ident("a"), raw_template(vec![quasi("x", true)], vec![]));
    assert_emits(member(inner, "length"), "a`x`.length;");
}

/// A looser tag is parenthesised — a sequence tag would otherwise tag only its
/// last operand: `` (a,b)`x` ``.
#[test]
fn sequence_tag_is_wrapped() {
    let tag = seq(vec![ident("a"), ident("b")]);
    assert_emits(tagged(tag, raw_template(vec![quasi("x", true)], vec![])), "(a,b)`x`;");
}
