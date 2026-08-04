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
/// position. Our emitter applies the Closure-style boolean shorthand,
/// printing `true` as `!0` and `false` as `!1` (value-exact: `!0 === true`,
/// `!1 === false`). `!0` / `!1` start with `!`, so there is no
/// expression-statement / ASI pitfall at statement position.
#[test]
fn test_boolean_literal_at_statement_position() {
    assert_emits(boolean(true), "!0;");
    assert_emits(boolean(false), "!1;");
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

/// Upstream `testNumericKeys` / `testExponents`-style shortest-form
/// number printing: the emitter picks whichever of the decimal or the
/// uppercase-`E` exponential spelling is shorter (ties → decimal).
///
/// gap-025 RESOLVED — `format_js_number` in `closure-emitter` now does
/// this (see CLOC12-gaps.md CLOC12.138). This was previously an
/// `#[ignore]` placeholder; it is now an active conformance test.
#[test]
fn test_number_formatting_shortest_form() {
    // Exponential is strictly shorter → collapse (uppercase E, no `+`).
    assert_emits(num(1_000_000_000.0), "1E9;"); // decimal 10 vs "1E9" 3
    assert_emits(num(1_000_000.0), "1E6;"); //     decimal 7  vs "1E6" 3
    assert_emits(num(1e21), "1E21;"); //           beyond i64, expo wins
    // Tie or decimal-shorter → keep decimal.
    assert_emits(num(100.0), "100;"); //           "100" 3 vs "1E2" 3 → tie→decimal
    // Value position drops the leading fraction zero: `.5` (2) beats both
    // `0.5` (3) and `5E-1` (4) — matches the reference CodePrinter.
    assert_emits(num(0.5), ".5;");
    // Negative zero must keep its sign (`1/-0 === -Infinity`).
    assert_emits(num(-0.0), "-0;");
}

/// Upstream `testStringQuoteChoice`-style — pick the quote that
/// minimises escapes: when the value contains more `"` than `'`, the
/// single-quote spelling is shorter (no escaping of the inner `"`).
///
/// gap-026 RESOLVED — `emit_string` now does this (CLOC12-gaps.md
/// CLOC12.138). Previously an `#[ignore]` placeholder; now active.
#[test]
fn test_string_quote_choice_minimises_escapes() {
    // More `"` than `'` → single-quote form (inner `"` need no escape).
    assert_emits(string("she said \"hi\""), "'she said \"hi\"';");
    // `'` present, no `"` → default double-quote form (tie/dq-not-more).
    assert_emits(string("o'malley"), "\"o'malley\";");
    // No quotes at all → default double.
    assert_emits(string("plain"), "\"plain\";");
}

/// Upstream `testOperatorPrecedence`-style — when a higher-precedence
/// operator contains a lower-precedence subexpression, the
/// subexpression needs parens to preserve evaluation order.
///
/// gap-027 RESOLVED — the emitter now models operator precedence and
/// inserts exactly the parens needed (CLOC12-gaps.md CLOC12.138).
/// Previously an `#[ignore]` placeholder; now active.
#[test]
fn test_operator_precedence_inserts_inner_parens() {
    // `a * (b + c)` — `+` is lower precedence than `*`, so the right
    // operand needs parens to preserve evaluation order.
    let a_times_b_plus_c = Expression::BinaryExpression(BinaryExpression {
        cv: None,
        operator: BinaryOperator::Mul,
        left: Box::new(ident("a")),
        right: Box::new(Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: BinaryOperator::Add,
            left: Box::new(ident("b")),
            right: Box::new(ident("c")),
        })),
    });
    assert_emits(a_times_b_plus_c, "a*(b+c);");

    // `(a + b) * c` — the SAME associativity note: `a + b` (lower prec)
    // on the left of `*` also needs parens.
    let a_plus_b_times_c = Expression::BinaryExpression(BinaryExpression {
        cv: None,
        operator: BinaryOperator::Mul,
        left: Box::new(Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: BinaryOperator::Add,
            left: Box::new(ident("a")),
            right: Box::new(ident("b")),
        })),
        right: Box::new(ident("c")),
    });
    assert_emits(a_plus_b_times_c, "(a+b)*c;");

    // `a + b * c` — `*` binds tighter, so NO parens are needed.
    let a_plus_b_times_c_noparen = Expression::BinaryExpression(BinaryExpression {
        cv: None,
        operator: BinaryOperator::Add,
        left: Box::new(ident("a")),
        right: Box::new(Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: BinaryOperator::Mul,
            left: Box::new(ident("b")),
            right: Box::new(ident("c")),
        })),
    });
    assert_emits(a_plus_b_times_c_noparen, "a+b*c;");
}
