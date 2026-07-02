//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **template-literal** printing cases — the
//! backtick `` `…` `` form, both the no-substitution shape and the
//! `${…}`-substitution shape. This is the tenth CodePrinter port into
//! `closure-emitter` (after core / declarations / trailing-comma / numbers /
//! string-escape / ascii-escape / object-literal / function-expression /
//! arrow-function) and isolates `emit_template_literal` +
//! `emit_template_element` + the `PREC_PRIMARY` classification that landed
//! with `Expression::TemplateLiteral` (CLOC12.154).
//!
//! ## How the emitter prints a template literal (recap)
//!
//! A template is `quasis` (the fixed string parts) interleaved with
//! `expressions` (the `${…}` inserts), under the structural invariant
//! `quasis.len() == expressions.len() + 1` so a quasi both opens and closes
//! the run:
//!
//! ```text
//!   `q0${e0}q1${e1}…qN`
//! ```
//!
//! Each quasi is emitted from its **raw** text verbatim (escape sequences
//! intact) so the template round-trips byte-for-byte. (A quasi carrying a
//! *literal* embedded newline is the one exception the emitter can't print
//! yet — see the `raw_preserves_internal_newline` case and gap-158.) A
//! `${…}` insert is a self-delimiting full-expression context, so the inner
//! expression is emitted at the loosest precedence — the braces already fence
//! it, so even a low-precedence body like `a+b` needs no parens.
//!
//! ```text
//!   `hello`               no-substitution           → `hello`
//!   `${world}`            single insert, empty edges → `${world}`
//!   `hello ${world}`      text then insert           → `hello ${world}`
//!   `${a}${b}`            adjacent inserts           → `${a}${b}`
//!   `${a + b}`            low-prec insert body       → `${a+b}`
//! ```
//!
//! ## A template is a PRIMARY expression
//!
//! `Expression::TemplateLiteral` classifies as `PREC_PRIMARY` — the tightest
//! binding, a leaf token-run bounded by backticks. So it is **never**
//! parenthesised as an operand or as a member-access object:
//!
//! ```text
//!   `hello`.length        member object   → `hello`.length   (no wrap)
//!   `hello` + world       `+` left operand → `hello`+world    (no wrap)
//!   `hello` == world      `==` left operand → `hello`==world  (no wrap)
//! ```
//!
//! ## Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (upstream lex/parses
//! source). That lets the port exercise **`${…}`-substitution** templates —
//! multi-part `quasis` with `expressions` — which the emitter prints
//! correctly even though the current grammar tokenises only no-substitution
//! templates into the bridge (see CLOC12-gaps gap-157). The emitter is the
//! unit under test here, not the parser.
//!
//! ## Tagged templates
//!
//! Upstream also covers `` tag`${x} world` `` (tagged template expressions).
//! We have no `TaggedTemplateExpression` AST node yet, so those cases cannot
//! be hand-constructed and are intentionally not ported (see ATTRIBUTION.md
//! "Skipped").

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BinaryExpression, BinaryOperator, Expression, ExpressionStatement, Identifier, MemberExpression,
    Program, ProgramItem, SourceType, Statement, TemplateElement, TemplateLiteral,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn ident(name: &str) -> Expression {
    Expression::Identifier(Identifier { cv: None, name: name.to_string() })
}

/// One template quasi built from its raw text. `cooked` mirrors `raw` for
/// these ASCII cases; the emitter re-emits `raw` regardless.
fn quasi(raw: &str, tail: bool) -> TemplateElement {
    TemplateElement { cv: None, raw: raw.to_string(), cooked: Some(raw.to_string()), tail }
}

fn template(quasis: Vec<TemplateElement>, expressions: Vec<Expression>) -> Expression {
    Expression::TemplateLiteral(TemplateLiteral { cv: None, quasis, expressions })
}

/// A no-substitution template: a single tail quasi, no inserts.
fn template_no_sub(raw: &str) -> Expression {
    template(vec![quasi(raw, true)], vec![])
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
        "template-literal emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — no-substitution templates
// =====================================================================

/// `assertPrintSame("`hello`")` — a plain template prints its raw text
/// between backticks. A backtick is an unambiguous expression-statement
/// start, so no leading paren wrap is added.
#[test]
fn no_substitution() {
    assert_emits(template_no_sub("hello"), "`hello`;");
}

/// An empty template is a single empty quasi.
#[test]
fn empty_template() {
    assert_emits(template_no_sub(""), "``;");
}

/// `assertPrintSame("`hel\\`lo`")` — an escaped backtick inside the literal
/// is preserved verbatim by the raw round-trip (the emitter never re-escapes
/// quasi text).
#[test]
fn raw_preserves_escaped_backtick() {
    // The raw source between the delimiters is `hel\`lo` (a backslash then a
    // backtick); emitted back unchanged.
    assert_emits(template_no_sub("hel\\`lo"), "`hel\\`lo`;");
}

/// `assertPrintSame` for a template with a `${` that is escaped (`\\${`) — a
/// literal dollar-brace, NOT a substitution — survives verbatim in the raw.
#[test]
fn raw_preserves_escaped_dollar_brace() {
    assert_emits(template_no_sub("price: \\${x}"), "`price: \\${x}`;");
}

/// Upstream `testMultilineTemplateLiteralPreservesInternalWhitespace`: an
/// internal newline (and its following indentation) is part of the raw text
/// and *should* print exactly, never collapsed.
///
/// **Ignored — gap-158.** The emitter routes quasi text through `write_str`,
/// which `debug_assert!`s the run contains no `'\n'` (newlines must go through
/// `newline()` so the source-map line/col bookkeeping stays correct). A
/// template quasi is the one primary token that can legitimately carry a raw
/// newline, so `emit_template_element` needs a newline-aware write path before
/// this case can pass. Tracked as gap-158; unignore when the emitter learns to
/// split quasi raw on embedded newlines.
#[test]
#[ignore = "gap-158: emitter write_str forbids embedded newlines in template quasi raw"]
fn raw_preserves_internal_newline() {
    assert_emits(template_no_sub("hello\n  world"), "`hello\n  world`;");
}

// =====================================================================
// Active — a template is a PRIMARY expression (never wrapped)
// =====================================================================

/// `assertPrintSame("`hello`.length")` — a template as a member-access
/// object is a primary expression and needs no parens.
#[test]
fn member_object_no_wrap() {
    assert_emits(member(template_no_sub("hello"), "length"), "`hello`.length;");
}

/// `assertPrintSame("`hello`.length.foo.bar")` — a member chain rooted on a
/// template stays unwrapped the whole way down.
#[test]
fn member_chain_no_wrap() {
    let m = member(member(member(template_no_sub("hello"), "length"), "foo"), "bar");
    assert_emits(m, "`hello`.length.foo.bar;");
}

/// `assertPrintSame("`hello` + world")` — a template as the left operand of
/// `+` needs no parens; the operator prints tight in compact mode.
#[test]
fn binary_add_left_operand() {
    assert_emits(binary(BinaryOperator::Add, template_no_sub("hello"), ident("world")),
        "`hello`+world;");
}

/// `assertPrintSame("`hello` == world")` — same for a comparison operator.
#[test]
fn binary_eq_left_operand() {
    assert_emits(binary(BinaryOperator::Eq, template_no_sub("hello"), ident("world")),
        "`hello`==world;");
}

/// `assertPrintSame("`hello` + `world`")` — two templates concatenated; each
/// is primary so neither is wrapped.
#[test]
fn concat_two_templates() {
    assert_emits(
        binary(BinaryOperator::Add, template_no_sub("hello"), template_no_sub("world")),
        "`hello`+`world`;",
    );
}

// =====================================================================
// Active — ${…} substitution templates
// =====================================================================

/// `assertPrintSame("`${world}`")` — a lone substitution with empty quasis on
/// both edges.
#[test]
fn single_substitution_only() {
    let t = template(vec![quasi("", false), quasi("", true)], vec![ident("world")]);
    assert_emits(t, "`${world}`;");
}

/// `assertPrintSame("`hello ${world}`")` — fixed text then an insert; the
/// leading quasi carries the literal `"hello "`.
#[test]
fn text_then_substitution() {
    let t = template(vec![quasi("hello ", false), quasi("", true)], vec![ident("world")]);
    assert_emits(t, "`hello ${world}`;");
}

/// `assertPrintSame("`${hello} world`")` — an insert then fixed text; the
/// tail quasi carries `" world"`.
#[test]
fn substitution_then_text() {
    let t = template(vec![quasi("", false), quasi(" world", true)], vec![ident("hello")]);
    assert_emits(t, "`${hello} world`;");
}

/// `assertPrintSame("`${hello}${world}`")` — adjacent inserts have an empty
/// quasi between them; the run still opens and closes with a (possibly empty)
/// quasi.
#[test]
fn adjacent_substitutions() {
    let t = template(
        vec![quasi("", false), quasi("", false), quasi("", true)],
        vec![ident("hello"), ident("world")],
    );
    assert_emits(t, "`${hello}${world}`;");
}

/// `assertPrintSame("`${a + b}`")` — a `${…}` context is the loosest, so a
/// low-precedence additive body needs no inner parens; the braces fence it.
#[test]
fn substitution_body_low_precedence_no_parens() {
    let sum = binary(BinaryOperator::Add, ident("a"), ident("b"));
    let t = template(vec![quasi("", false), quasi("", true)], vec![sum]);
    assert_emits(t, "`${a+b}`;");
}

/// `assertPrintSame("`${hello.length}`")` — a member-access substitution body
/// prints normally inside the braces.
#[test]
fn substitution_body_member_access() {
    let t = template(
        vec![quasi("", false), quasi("", true)],
        vec![member(ident("hello"), "length")],
    );
    assert_emits(t, "`${hello.length}`;");
}

/// `assertPrintSame("`${hello.length} ${world}`")` — two inserts separated by
/// a fixed-text quasi.
#[test]
fn two_substitutions_with_text_between() {
    let t = template(
        vec![quasi("", false), quasi(" ", false), quasi("", true)],
        vec![member(ident("hello"), "length"), ident("world")],
    );
    assert_emits(t, "`${hello.length} ${world}`;");
}

/// A substitution template as a member-access object is still primary — the
/// whole `` `${x}` `` is unwrapped under `.length`.
#[test]
fn substitution_template_as_member_object() {
    let t = template(vec![quasi("", false), quasi("", true)], vec![ident("x")]);
    assert_emits(member(t, "length"), "`${x}`.length;");
}
