//! Ported from `PeepholeReplaceKnownMethodsTest.java` in
//! `google/closure-compiler`, Apache-2.0. Upstream SHA: see
//! `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! Upstream `PeepholeReplaceKnownMethods` folds calls to well-known,
//! side-effect-free String / Array / Math methods on constant receivers
//! (`"abcdef".indexOf("cd")` → `2`). Our `ConstantFoldPass` implements a large
//! slice of the String methods and the numeric `Math.max`/`Math.min`; this port
//! pins those and records the upstream behaviors we do not fold yet as
//! `#[ignore = "blocked on gap-NNN"]` placeholders.
//!
//! Built on the same AST-builder surface as the sibling
//! `peephole_fold_constants_test.rs` port (closurec has no public
//! source-string → typed `Program` entry for pass crates with a minimal
//! dev-dependency set), so each upstream `test("input", "expected")` becomes an
//! `assert_*` over a hand-built call expression.
//!
//! Every active test that *disagrees* with our pass is a real closurec defect,
//! not a translation artifact — the whole point of the port.

use coding_adventures_closure_pass_constant_fold::ConstantFoldPass;
use coding_adventures_closure_pass_pipeline::{Pass, PassContext};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    statement::TaggedStatement, CallExpression, Expression, ExpressionStatement, Identifier,
    MemberExpression, NumericLiteral, Program, ProgramItem, SourceType, Statement, StringLiteral,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// ----- AST builders (mirror the crate's own unit-test helpers) -----

fn s(v: &str) -> Expression {
    Expression::StringLiteral(StringLiteral {
        cv: None,
        value: v.to_string(),
        raw: format!("\"{}\"", v),
    })
}

fn n(v: f64) -> Expression {
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

fn ident(name: &str) -> Expression {
    Expression::Identifier(Identifier {
        cv: None,
        name: name.to_string(),
    })
}

fn member(object: Expression, name: &str) -> Expression {
    Expression::MemberExpression(MemberExpression {
        cv: None,
        object: Box::new(object),
        property: Box::new(ident(name)),
        computed: false,
    })
}

/// `<object>.<name>(<args...>)`.
fn call(object: Expression, name: &str, args: Vec<Expression>) -> Expression {
    Expression::CallExpression(CallExpression {
        cv: None,
        callee: Box::new(member(object, name)),
        arguments: args,
    })
}

/// Wrap `input` as the only statement, run `ConstantFoldPass`, pull the
/// (possibly-folded) top-level expression back out.
fn fold_once(input: Expression) -> Expression {
    let program = Program::new_untraced(EsVersion::Es2025, SourceType::Module).with_body(vec![
        ProgramItem::Statement(Statement::expression_statement(ExpressionStatement {
            cv: None,
            expression: input,
        })),
    ]);
    let pass = ConstantFoldPass::new();
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    let out = pass
        .run(PassContext {
            program: &program,
            sidecar: &sidecar,
            cv: &mut cv,
        })
        .expect("constant-fold pass run failed");
    let item = out.program.body.into_iter().next().expect("body empty");
    let ProgramItem::Statement(Statement::Tagged(TaggedStatement::ExpressionStatement(es))) = item
    else {
        panic!("unexpected program shape after fold");
    };
    es.expression
}

fn assert_str(input: Expression, expected: &str) {
    match fold_once(input) {
        Expression::StringLiteral(sl) => assert_eq!(sl.value, expected),
        other => panic!("expected string {expected:?}; got {other:?}"),
    }
}

fn assert_num(input: Expression, expected: f64) {
    match fold_once(input) {
        Expression::NumericLiteral(nl) => assert_eq!(nl.value, expected),
        other => panic!("expected number {expected}; got {other:?}"),
    }
}

fn assert_bool(input: Expression, expected: bool) {
    match fold_once(input) {
        Expression::BooleanLiteral(bl) => assert_eq!(bl.value, expected),
        other => panic!("expected bool {expected}; got {other:?}"),
    }
}

/// Upstream `testSame(input)`: the fold leaves the expression unchanged.
fn assert_same(input: Expression) {
    let before = input.clone();
    assert_eq!(fold_once(input), before, "expected pass to leave unchanged");
}

// ===================================================================
// Active ports — folds our ConstantFoldPass performs today.
// ===================================================================

/// Upstream `testStringIndexOf`.
#[test]
fn folds_string_index_of() {
    assert_num(call(s("abcdef"), "indexOf", vec![s("cd")]), 2.0);
    assert_num(call(s("abcdef"), "indexOf", vec![s("xy")]), -1.0);
}

/// Upstream `testStringLastIndexOf`.
#[test]
fn folds_string_last_index_of() {
    assert_num(call(s("abcabc"), "lastIndexOf", vec![s("bc")]), 4.0);
}

/// Upstream casing folds.
#[test]
fn folds_case_conversion() {
    assert_str(call(s("abc"), "toUpperCase", vec![]), "ABC");
    assert_str(call(s("ABC"), "toLowerCase", vec![]), "abc");
}

/// Upstream `testStringSlice` (subset our slice supports).
#[test]
fn folds_string_slice() {
    assert_str(call(s("hello"), "slice", vec![n(1.0), n(3.0)]), "el");
}

/// Upstream `testStringSubstring` / `testStringSubstr`.
#[test]
fn folds_string_substring_and_substr() {
    assert_str(call(s("abcd"), "substring", vec![n(1.0), n(3.0)]), "bc");
    assert_str(call(s("abcde"), "substr", vec![n(1.0), n(2.0)]), "bc");
}

/// Upstream `testStringCharAt` / `testStringCharCodeAt`.
#[test]
fn folds_char_at_and_char_code_at() {
    assert_str(call(s("abc"), "charAt", vec![n(1.0)]), "b");
    assert_num(call(s("abc"), "charCodeAt", vec![n(0.0)]), 97.0);
}

/// Upstream `testStringRepeat` (ES2015).
#[test]
fn folds_string_repeat() {
    assert_str(call(s("ab"), "repeat", vec![n(3.0)]), "ababab");
}

/// Upstream `testStringTrim`.
#[test]
fn folds_string_trim() {
    assert_str(call(s("  hi  "), "trim", vec![]), "hi");
}

/// Upstream `testStringIncludesStartsWithEndsWith` → boolean folds.
#[test]
fn folds_includes_startswith_endswith() {
    assert_bool(call(s("hello"), "includes", vec![s("ell")]), true);
    assert_bool(call(s("hello"), "startsWith", vec![s("he")]), true);
    assert_bool(call(s("hello"), "endsWith", vec![s("lo")]), true);
    assert_bool(call(s("hello"), "startsWith", vec![s("xx")]), false);
}

/// A method call on a non-constant receiver is left untouched — the value of
/// `s` is unknown at compile time.
#[test]
fn does_not_fold_on_non_constant_receiver() {
    assert_same(call(ident("s"), "toUpperCase", vec![]));
}

// ===================================================================
// Ignored ports — upstream folds our pass does not perform yet.
// Each is pinned to a gap in code/specs/CLOC12-gaps.md.
// ===================================================================

/// Upstream folds `Math.abs`/`floor`/`ceil`/`round` on numeric literals. Our
/// pass folds only `Math.max`/`Math.min` today.
#[test]
#[ignore = "blocked on gap-141: constant-fold does not fold Math.abs/floor/ceil/round"]
fn folds_math_unary_methods() {
    assert_num(call(ident("Math"), "abs", vec![n(-5.0)]), 5.0);
    assert_num(call(ident("Math"), "floor", vec![n(4.7)]), 4.0);
}

/// Upstream folds `[a,b,c].join(sep)` on an array literal of constants. Our
/// pass folds String methods but not Array#join.
#[test]
#[ignore = "blocked on gap-142: constant-fold does not fold Array.prototype.join on array literals"]
fn folds_array_join() {
    use coding_adventures_javascript_ast::ArrayExpression;
    let arr = Expression::ArrayExpression(ArrayExpression {
        cv: None,
        elements: vec![Some(s("a")), Some(s("b")), Some(s("c"))],
    });
    assert_str(call(arr, "join", vec![s("-")]), "a-b-c");
}

/// Upstream folds `"a".concat("b","c")` — covered by our pass for strings, but
/// the *number*-coercing form `"x".concat(1, 2)` is an upstream case we do not
/// fold (coercion of non-string args).
#[test]
#[ignore = "blocked on gap-143: constant-fold does not fold String#concat with non-string (coerced) args"]
fn folds_string_concat_with_coerced_args() {
    assert_str(call(s("x"), "concat", vec![n(1.0), n(2.0)]), "x12");
}
