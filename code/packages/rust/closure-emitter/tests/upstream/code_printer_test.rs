//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! Fourth port under CLOC12 — first one targeting the emitter rather
//! than a transform pass. Most upstream `@Test` methods exercise
//! Phase 2+ AST node variants we don't have yet (BigInt,
//! OptionalCallExpression, TemplateLiteral, etc.) or use the upstream
//! `assertPrintSame` round-trip which expects unparenthesised
//! expression-statements like `2+3;` while our Phase 1 emitter
//! deliberately wraps every ExpressionStatement in parens
//! (`(2 + 3);`).
//!
//! So the bulk of this file is `#[ignore = "blocked on gap-NNN"]`
//! placeholders that pin the upstream intent and document the
//! divergence. The few non-ignored tests assert the byte-equal output
//! our emitter actually produces today, matching the inline-test
//! shapes in `closure-emitter`'s own `#[cfg(test)]` module.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BinaryExpression, BinaryOperator, BooleanLiteral, Expression, ExpressionStatement, Identifier,
    NumericLiteral, Program, ProgramItem, SourceType, Statement, StringLiteral, UnaryExpression,
    UnaryOperator,
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
        raw: if v.fract() == 0.0 && v.is_finite() {
            format!("{}", v as i64)
        } else {
            v.to_string()
        },
    })
}
fn string(v: &str) -> Expression {
    Expression::StringLiteral(StringLiteral {
        cv: None,
        value: v.to_string(),
        raw: format!("\"{}\"", v),
    })
}
fn boolean(v: bool) -> Expression {
    Expression::BooleanLiteral(BooleanLiteral { cv: None, value: v })
}

fn stmt(expr: Expression) -> ProgramItem {
    ProgramItem::Statement(Statement::expression_statement(ExpressionStatement {
        cv: None,
        expression: expr,
    }))
}

fn program_with(item: ProgramItem) -> Program {
    Program::new_untraced(EsVersion::Es2025, SourceType::Module).with_body(vec![item])
}

fn emit_default(prog: Program) -> String {
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    emit(&prog, &sidecar, &mut cv, &EmitOptions::default())
        .expect("emit failed")
        .code
}

/// Upstream `assertPrint(input, expected)` reshaped for our AST
/// surface: given an Expression that models the input, emit it as
/// the body of a single-statement program and assert the resulting
/// code equals `expected_emitted`.
fn assert_emits(expr: Expression, expected_emitted: &str) {
    let code = emit_default(program_with(stmt(expr)));
    assert_eq!(
        code, expected_emitted,
        "emit output did not match expected\n  actual:   {:?}\n  expected: {:?}",
        code, expected_emitted
    );
}

// =====================================================================
// Ported tests
// =====================================================================

/// Upstream `testBigInt`:
///
///   assertPrintSame("1n");
///   assertPrint("0b10n", "2n");
///   ...
#[test]
#[ignore = "blocked on gap-021: BigIntLiteral not in Phase 1 AST"]
fn test_big_int() {
    // Requires Expression::BigIntLiteral variant. Phase 1.x work.
}

/// Upstream `testTrailingCommaInArrayAndObjectWithPrettyPrint`:
///
///   languageMode = LanguageMode.ECMASCRIPT_2017;
///   assertPrettyPrint("var x = [1,];", "var x = [1];\n");
///   ...
///
/// **gap-022 routed in CLOC12.32** — the trailing-comma-focused
/// port file landed at
/// `closure-emitter/tests/upstream/code_printer_trailing_comma_test.rs`.
/// Both array and object cases live there, in compact and pretty
/// modes. See that file's module doc for the full reasoning on
/// why our AST doesn't need a `trailing_comma: bool` flag to
/// pass this upstream test family.
#[test]
#[ignore = "routed in CLOC12.32 to code_printer_trailing_comma_test.rs (gap-022)"]
fn test_trailing_comma_in_array_and_object_with_pretty_print() {
    // Routed. See doc comment above for the new home.
}

/// Upstream `testNoTrailingCommaInEmptyArrayLiteral`:
///
///   assertPrintSame("var x = [];");
///
/// **gap-023 routed in CLOC12.30** — the declarations-focused port file
/// landed at
/// `closure-emitter/tests/upstream/code_printer_declarations_test.rs`.
/// This specific upstream test is covered there by
/// `var_with_empty_array_init`.
#[test]
#[ignore = "routed in CLOC12.30 to code_printer_declarations_test.rs (gap-023)"]
fn test_no_trailing_comma_in_empty_array_literal() {
    // Routed. See doc comment above for the new home.
}

/// Upstream `assertPrint("2 + 3", "2+3")` — `+` binary expression at
/// statement position. **gap-024 closed in CLOC12.10**: the emitter no
/// longer wraps `ExpressionStatement` bodies in parens except for the
/// leading-token-ambiguous `ObjectExpression` case.
///
/// Our (compact) output is now `2+3;` — byte-identical to upstream's
/// minified `assertPrint("2 + 3", "2+3")`. Symbolic binary operators are
/// emitted tight in compact mode; a space appears only in `pretty` mode.
#[test]
fn test_binary_addition_emits_without_outer_parens() {
    let e = Expression::BinaryExpression(BinaryExpression {
        cv: None,
        operator: BinaryOperator::Add,
        left: Box::new(num(2.0)),
        right: Box::new(num(3.0)),
    });
    assert_emits(e, "2+3;");
}

/// Same shape for string concatenation: `\"a\" + \"b\";` without outer
/// parens. **gap-024 closed in CLOC12.10**.
#[test]
fn test_string_concat_emits_without_outer_parens() {
    let e = Expression::BinaryExpression(BinaryExpression {
        cv: None,
        operator: BinaryOperator::Add,
        left: Box::new(string("a")),
        right: Box::new(string("b")),
    });
    assert_emits(e, "\"a\"+\"b\";");
}

/// Upstream `assertPrintSame("!x")` — unary-not on identifier, no
/// space between `!` and the argument. We emit `!x;` (no surrounding
/// parens for a bare unary statement).
#[test]
fn test_unary_not_emits_without_space() {
    let e = Expression::UnaryExpression(UnaryExpression {
        cv: None,
        operator: UnaryOperator::Not,
        prefix: true,
        argument: Box::new(ident("x")),
    });
    assert_emits(e, "!x;");
}

/// Upstream `assertPrintSame("true")` — boolean literal at statement
/// position. Our emitter produces `true;` — bare boolean literal, no
/// parens.
#[test]
fn test_boolean_literal_at_statement_position() {
    assert_emits(boolean(true), "true;");
    assert_emits(boolean(false), "false;");
}

/// Upstream `assertPrintSame("0")`, `assertPrintSame("42")`, etc. —
/// integer numeric literal at statement position. Our emitter
/// produces `0;`, `42;` etc.
#[test]
fn test_integer_literals_at_statement_position() {
    assert_emits(num(0.0), "0;");
    assert_emits(num(42.0), "42;");
    assert_emits(num(1.0), "1;");
}

/// Upstream `assertPrintSame("\"hello\"")` — string literal at
/// statement position. We emit `"hello";`.
#[test]
fn test_string_literal_at_statement_position() {
    assert_emits(string("hello"), "\"hello\";");
    assert_emits(string("a"), "\"a\";");
}

/// Upstream tests cover number formatting like
/// `assertPrint("1000000000", "1E9")` — using `1E9` exponential
/// notation as shorter. Our emitter formats `1000000000` as
/// `1000000000` (no exponent collapse).
#[test]
#[ignore = "blocked on gap-025: number-formatting (shortest-form / exponential) not implemented"]
fn test_number_formatting_shortest_form() {
    // assertPrint("1000000000", "1E9");
    // Belongs in a future emitter-numeric-formatter slice.
}

/// Upstream `testStringQuoteChoice`-style — pick the quote that
/// minimises escapes:
///
///   assertPrint("var x = \"single 'quote'\";", "var x='single \\'quote\\''");
///   assertPrint("var x = \"a\\nb\";", "var x=\"a\\nb\"");
///
/// Our emitter always uses double quotes and doesn't switch to
/// single. Gap-026.
#[test]
#[ignore = "blocked on gap-026: quote-choice optimisation not implemented"]
fn test_string_quote_choice_minimises_escapes() {
    // Belongs in a future emitter-string-escape slice.
}

/// Upstream `testOperatorPrecedence`-style — when a higher-precedence
/// operator contains a lower-precedence subexpression, the
/// subexpression needs parens to preserve evaluation order:
///
///   assertPrint("(a+b)*c", "(a+b)*c");
///   assertPrint("a*(b+c)", "a*(b+c)");
///
/// Our emitter doesn't model operator precedence — it doesn't emit
/// inner parens, and it wraps the outer expression in parens
/// unconditionally. Filed as gap-027.
#[test]
#[ignore = "blocked on gap-027: precedence-aware paren insertion not implemented"]
fn test_operator_precedence_inserts_inner_parens() {
    // Would assert that `a * (b + c)` emits with the inner parens.
}
