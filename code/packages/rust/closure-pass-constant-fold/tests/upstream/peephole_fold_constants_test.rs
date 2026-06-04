//! Ported from `PeepholeFoldConstantsTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! First port under CLOC12 — establishes the file layout and gap-tracking
//! pattern. Covers a small subset of upstream's
//! `PeepholeFoldConstantsTest` focused on cases that our `ConstantFoldPass`
//! can fold today without language features still to come (no `typeof`,
//! no `void 0`, no BigInt literal, no `NaN`/`Infinity` identifier).
//!
//! Tests that exercise features we don't fold yet are marked
//! `#[ignore = "blocked on gap-NNN"]` with the gap recorded in
//! `code/specs/CLOC12-gaps.md`. Running `cargo test -- --include-ignored`
//! exercises those too and lets us measure progress as gaps close.

use coding_adventures_closure_pass_constant_fold::ConstantFoldPass;
use coding_adventures_closure_pass_pipeline::{Pass, PassContext};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    statement::TaggedStatement, BinaryExpression, BinaryOperator, BooleanLiteral, Expression,
    ExpressionStatement, NullLiteral, NumericLiteral, Program, ProgramItem, SourceType,
    Statement, StringLiteral, UndefinedLiteral, UnaryExpression, UnaryOperator,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
//
// `b(...)` builds binary expressions, `n`/`s`/`bool_`/`null_` build the
// matching literal expressions. Each upstream `test("input", "expected")`
// becomes one call to `assert_fold(input_expr, expected_expr)`.
//
// These mirror the inline test helpers in `closure-pass-constant-fold`'s
// own unit tests so the byte output is constructed the same way the
// rest of the crate does it.
// =====================================================================

fn n(v: f64) -> Expression {
    Expression::NumericLiteral(NumericLiteral {
        cv: None,
        value: v,
        // Match the `raw` shape the inline tests use for integers and
        // decimals; the constant-fold pass reads `value`, not `raw`.
        raw: if v.fract() == 0.0 && v.is_finite() {
            format!("{}", v as i64)
        } else {
            v.to_string()
        },
    })
}

fn s(v: &str) -> Expression {
    Expression::StringLiteral(StringLiteral {
        cv: None,
        value: v.to_string(),
        raw: format!("\"{}\"", v),
    })
}

fn bool_(v: bool) -> Expression {
    Expression::BooleanLiteral(BooleanLiteral { cv: None, value: v })
}

fn null_() -> Expression {
    Expression::NullLiteral(NullLiteral { cv: None })
}

fn b(left: Expression, op: BinaryOperator, right: Expression) -> Expression {
    Expression::BinaryExpression(BinaryExpression {
        cv: None,
        operator: op,
        left: Box::new(left),
        right: Box::new(right),
    })
}

/// Wrap an expression as the only statement in a program, run
/// `ConstantFoldPass` over it, and pull the (possibly-folded) top-level
/// expression back out.
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
    let ctx = PassContext {
        program: &program,
        sidecar: &sidecar,
        cv: &mut cv,
    };
    let out = pass.run(ctx).expect("constant-fold pass run failed");
    let item = out.program.body.into_iter().next().expect("body empty");
    let ProgramItem::Statement(Statement::Tagged(TaggedStatement::ExpressionStatement(es))) = item
    else {
        panic!("unexpected program shape after fold");
    };
    es.expression
}

/// Upstream `test(input, expected)`. Run the fold and assert structural
/// equality against the expected expression.
fn assert_fold(input: Expression, expected: Expression) {
    let actual = fold_once(input);
    assert_eq!(
        actual, expected,
        "fold output did not match expected; actual = {:?}, expected = {:?}",
        actual, expected
    );
}

/// Upstream `testSame(input)`. Run the fold and assert the output is
/// structurally identical to the input.
fn assert_same(input: Expression) {
    let before = input.clone();
    let after = fold_once(input);
    assert_eq!(
        after, before,
        "expected pass to leave expression unchanged; got {:?}",
        after
    );
}

// =====================================================================
// Ported tests
//
// Each method below mirrors a `@Test public void <name>()` in upstream.
// Order and naming track the upstream file so a future port re-sync
// (CLOC12 §4) can diff cleanly.
// =====================================================================

/// Upstream:
///
///   test("undefined == undefined", "true");
///   test("undefined === undefined", "true");
///   ... (and many more)
#[test]
#[ignore = "blocked on gap-001: no `undefined`/`NaN`/`Infinity` literal in typed AST"]
fn test_undefined_comparison_1() {
    // Requires modeling `undefined` (and `void 0`) as literal-equivalent
    // expressions in the typed AST + a fold rule recognising them.
}

/// Upstream:
///
///   test("\"123\" !== void 0", "true");
///   test("\"123\" === void 0", "false");
///   test("void 0 !== \"123\"", "true");
///   test("void 0 === \"123\"", "false");
/// **gap-002 RESOLVED in CLOC12.20** — `UnaryOperator::Void` over a
/// primitive literal now folds to `UndefinedLiteral`. With gap-001
/// (UndefinedLiteral variant) already resolved in CLOC12.16,
/// downstream binary-equality folds can then participate.
///
/// This test exercises the fold rule directly: `void 0` (i.e.
/// `UnaryExpression { op: Void, arg: NumericLiteral(0) }`)
/// becomes `UndefinedLiteral`.
#[test]
fn test_undefined_comparison_2() {
    // `void 0` → undefined.
    assert_fold(
        Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: UnaryOperator::Void,
            prefix: true,
            argument: Box::new(n(0.0)),
        }),
        Expression::UndefinedLiteral(UndefinedLiteral { cv: None }),
    );
    // `void 1` → undefined (same rule; any numeric primitive
    // argument).
    assert_fold(
        Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: UnaryOperator::Void,
            prefix: true,
            argument: Box::new(n(1.0)),
        }),
        Expression::UndefinedLiteral(UndefinedLiteral { cv: None }),
    );
    // `void "x"` → undefined (any primitive literal).
    assert_fold(
        Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: UnaryOperator::Void,
            prefix: true,
            argument: Box::new(s("x")),
        }),
        Expression::UndefinedLiteral(UndefinedLiteral { cv: None }),
    );
    // `void undefined` → undefined (folds to itself).
    assert_fold(
        Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: UnaryOperator::Void,
            prefix: true,
            argument: Box::new(Expression::UndefinedLiteral(UndefinedLiteral {
                cv: None,
            })),
        }),
        Expression::UndefinedLiteral(UndefinedLiteral { cv: None }),
    );
}

/// Upstream:
///
///   test("null == null", "true");
///   test("null === null", "true");
///   test("null != null", "false");
///   test("null !== null", "false");
///   test("null < null", "false");
///   test("null > null", "false");
///   test("null >= null", "true");
///   test("null <= null", "true");
///
/// (Subset of the full `testNullComparison1`. Lines that mention
/// `undefined`, `void 0`, `NaN`, `Infinity` are deferred to gap-001.)
///
/// **gap-007 closed in CLOC12.03** — `try_fold_binary_op` now has a
/// `NullLiteral`/`NullLiteral` branch that returns the JS-spec value
/// for each operator (`==`/`===`/`<=`/`>=` → `true`, the rest →
/// `false`).
#[test]
fn test_null_comparison_1_self_relations() {
    assert_fold(b(null_(), BinaryOperator::Eq, null_()), bool_(true));
    assert_fold(b(null_(), BinaryOperator::StrictEq, null_()), bool_(true));
    assert_fold(b(null_(), BinaryOperator::NotEq, null_()), bool_(false));
    assert_fold(
        b(null_(), BinaryOperator::StrictNotEq, null_()),
        bool_(false),
    );
    assert_fold(b(null_(), BinaryOperator::Lt, null_()), bool_(false));
    assert_fold(b(null_(), BinaryOperator::Gt, null_()), bool_(false));
    assert_fold(b(null_(), BinaryOperator::GtEq, null_()), bool_(true));
    assert_fold(b(null_(), BinaryOperator::LtEq, null_()), bool_(true));
}

/// Upstream:
///
///   test("null == 0", "false");
///   test("null == 1", "false");
///   test("null == 'hi'", "false");
///   test("null == true", "false");
///   test("null == false", "false");
///
/// Loose equality of `null` with anything non-null/non-undefined is
/// `false`. Our pass folds `==` only when both sides are the *same* JS
/// type (sound default — see crate-level docs); these are blocked
/// pending a richer fold rule.
///
/// **gap-003 closed in CLOC12.21** — the constant-fold pass now
/// implements the `null`-side branch of the ECMAScript abstract
/// equality algorithm for compile-time-known partners. `null == X` is
/// `true` iff `X` is `null` or `undefined`; every other literal partner
/// yields `false`. `!=` is the negation. See `try_fold_binary_op` for
/// the truth table and unsoundness guard.
#[test]
fn test_null_comparison_1_loose_against_other_types() {
    assert_fold(b(null_(), BinaryOperator::Eq, n(0.0)), bool_(false));
    assert_fold(b(null_(), BinaryOperator::Eq, n(1.0)), bool_(false));
    assert_fold(b(null_(), BinaryOperator::Eq, s("hi")), bool_(false));
    assert_fold(b(null_(), BinaryOperator::Eq, bool_(true)), bool_(false));
    assert_fold(b(null_(), BinaryOperator::Eq, bool_(false)), bool_(false));
}

/// Upstream `testNumberNumberComparison` lines that use only literals:
///
///   test("1 > 1", "false");
///   test("2 == 3", "false");
///   test("3.6 === 3.6", "true");
#[test]
fn test_number_number_comparison_literal_lines() {
    assert_fold(b(n(1.0), BinaryOperator::Gt, n(1.0)), bool_(false));
    assert_fold(b(n(2.0), BinaryOperator::Eq, n(3.0)), bool_(false));
    assert_fold(b(n(3.6), BinaryOperator::StrictEq, n(3.6)), bool_(true));
}

/// Upstream `testStringStringComparison` literal-only lines:
///
///   test("'a' < 'b'", "true");
///   test("'a' <= 'b'", "true");
///   test("'a' > 'b'", "false");
///   test("'a' >= 'b'", "false");
///   test("'a' == 'a'", "true");
///   test("'b' != 'a'", "true");
///   test("'a' === 'a'", "true");
///   test("'b' !== 'a'", "true");
#[test]
fn test_string_string_comparison_literal_lines() {
    assert_fold(b(s("a"), BinaryOperator::Lt, s("b")), bool_(true));
    assert_fold(b(s("a"), BinaryOperator::LtEq, s("b")), bool_(true));
    assert_fold(b(s("a"), BinaryOperator::Gt, s("b")), bool_(false));
    assert_fold(b(s("a"), BinaryOperator::GtEq, s("b")), bool_(false));
    assert_fold(b(s("a"), BinaryOperator::Eq, s("a")), bool_(true));
    assert_fold(b(s("b"), BinaryOperator::NotEq, s("a")), bool_(true));
    assert_fold(b(s("a"), BinaryOperator::StrictEq, s("a")), bool_(true));
    assert_fold(b(s("b"), BinaryOperator::StrictNotEq, s("a")), bool_(true));
}

/// Upstream `testNumberStringComparison` lines that use only literals:
///
///   test("1 < '2'", "true");
///   test("1 == '2'", "false");
///   test("1 === '1'", "false");
///   test("1 !== '1'", "true");
///
/// `1 == '2'` and friends are cross-type comparisons. Upstream folds
/// them via the JS abstract equality algorithm. Our pass returns
/// unchanged for mixed-type `==` (see crate doc); strict `===` between
/// number and string is `false` by definition though, which we *can*
/// fold today (literal types differ, strict equality short-circuits to
/// false).
///
/// **gap-004 closed in CLOC12.22** — `try_fold_binary_op` now coerces a
/// String operand against a Number operand via a conservative subset of
/// §StringToNumber (`js_string_to_number_strict`), then evaluates the
/// resulting Number-vs-Number comparison. See the helper's doc comment
/// for which string forms are recognised.
#[test]
fn test_number_string_comparison_literal_lines() {
    assert_fold(b(n(1.0), BinaryOperator::Lt, s("2")), bool_(true));
    assert_fold(b(n(1.0), BinaryOperator::Eq, s("2")), bool_(false));
    assert_fold(b(n(1.0), BinaryOperator::StrictEq, s("1")), bool_(false));
    assert_fold(b(n(1.0), BinaryOperator::StrictNotEq, s("1")), bool_(true));
}

/// Upstream specific lines:
///
///   test("1 === '1'", "false");
///   test("1 !== '1'", "true");
///
/// These hold by JS-spec (strict equality requires same JS type; a
/// Number and a String can never `===`). **gap-008 closed in
/// CLOC12.03** — `try_fold_binary_op` now has a cross-type
/// strict-equality branch that fires when both sides are literals of
/// different JS types. Stays orthogonal to gap-004 (which is about
/// loose equality + abstract relational comparison).
#[test]
fn test_number_string_strict_equality_lines() {
    assert_fold(b(n(1.0), BinaryOperator::StrictEq, s("1")), bool_(false));
    assert_fold(
        b(n(1.0), BinaryOperator::StrictNotEq, s("1")),
        bool_(true),
    );
}

/// Upstream:
///
///   test("1 < 2", "true"); test("2 < 1", "false");
///   test("1 == 1", "true"); test("2 == 1", "false");
///
/// Pure same-type comparisons — these are well within our current
/// fold rules (literal vs. literal, both `Number`).
#[test]
fn test_basic_number_comparisons() {
    assert_fold(b(n(1.0), BinaryOperator::Lt, n(2.0)), bool_(true));
    assert_fold(b(n(2.0), BinaryOperator::Lt, n(1.0)), bool_(false));
    assert_fold(b(n(1.0), BinaryOperator::Eq, n(1.0)), bool_(true));
    assert_fold(b(n(2.0), BinaryOperator::Eq, n(1.0)), bool_(false));
    assert_fold(b(n(1.0), BinaryOperator::StrictEq, n(1.0)), bool_(true));
    assert_fold(b(n(2.0), BinaryOperator::StrictEq, n(1.0)), bool_(false));
}

/// Upstream `testStringStringComparison` line:
///
///   test("typeof 3 > typeof 4", "false")
///
/// `typeof <NumericLiteral>` folds to `"number"`, and `"number" >
/// "number"` is `false` (string comparison via the existing
/// string-vs-string branch). **gap-005 closed in CLOC12.09** for the
/// primitive-literal-typeof cases.
#[test]
fn test_typeof_literal_comparison_folds() {
    use coding_adventures_javascript_ast::{UnaryExpression, UnaryOperator};
    let typeof_lit = |arg: Expression| {
        Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: UnaryOperator::TypeOf,
            prefix: true,
            argument: Box::new(arg),
        })
    };
    // typeof 3 > typeof 4  →  "number" > "number"  →  false
    assert_fold(
        b(typeof_lit(n(3.0)), BinaryOperator::Gt, typeof_lit(n(4.0))),
        bool_(false),
    );
}

/// Upstream `testStringStringComparison` lines:
///
///   testSame("typeof a < 'a'")
///   testSame("'a' >= typeof a")
///
/// `typeof <identifier>` is *not* foldable — the identifier may bind to
/// anything at runtime. **gap-005 closed in CLOC12.09**: our pass
/// leaves these alone, which is exactly what upstream `testSame`
/// asserts.
#[test]
fn test_typeof_identifier_is_left_alone() {
    use coding_adventures_javascript_ast::{Identifier, UnaryExpression, UnaryOperator};
    let ident_expr = |name: &str| {
        Expression::Identifier(Identifier {
            cv: None,
            name: name.to_string(),
        })
    };
    let typeof_a = Expression::UnaryExpression(UnaryExpression {
        cv: None,
        operator: UnaryOperator::TypeOf,
        prefix: true,
        argument: Box::new(ident_expr("a")),
    });
    // typeof a < "a"  →  leave alone (identifier means runtime-unknown)
    assert_same(b(typeof_a.clone(), BinaryOperator::Lt, s("a")));
    // "a" >= typeof a  →  same shape, flipped
    assert_same(b(s("a"), BinaryOperator::GtEq, typeof_a));
}

/// Upstream `testStringStringComparison` lines using
/// `typeof <identifier> === typeof <SAME identifier>`:
///
///   test("typeof a === typeof a", "true");
///   test("typeof a !== typeof a", "false");
///
/// **gap-029 closed in CLOC12.17**: a new structural-equality arm in
/// `try_fold_binary_op` recognises `typeof <Identifier> {===,!==} typeof
/// <same Identifier>` and folds to `true`/`false` respectively.
///
/// Safety: ECMAScript §UnaryTypeofExpression special-cases
/// `typeof <undeclared-identifier>` to return `"undefined"` instead of
/// throwing a ReferenceError, so even if `a` is never declared the two
/// evaluations produce the same string and the fold is sound.
#[test]
fn test_typeof_identifier_identity_fold() {
    use coding_adventures_javascript_ast::{Identifier, UnaryExpression, UnaryOperator};
    let typeof_ = |name: &str| {
        Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: UnaryOperator::TypeOf,
            prefix: true,
            argument: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: name.to_string(),
            })),
        })
    };
    //   typeof a === typeof a  →  true
    assert_fold(
        b(typeof_("a"), BinaryOperator::StrictEq, typeof_("a")),
        bool_(true),
    );
    //   typeof a !== typeof a  →  false
    assert_fold(
        b(typeof_("a"), BinaryOperator::StrictNotEq, typeof_("a")),
        bool_(false),
    );
    //   typeof a === typeof b  →  NOT folded (different identifiers)
    assert_same(b(typeof_("a"), BinaryOperator::StrictEq, typeof_("b")));
}

/// Upstream "and similar" lines that boil down to plain arithmetic on
/// literals show up across several `@Test` methods. Capture a sample
/// here so this first port crate has a meaningful pass count:
///
///   test("2 + 3", "5");
///   test("'a' + 'b'", "'ab'");
///   test("'x' + 1", "'x1'");
///   test("5 * 4", "20");
///   test("10 / 2", "5");
///   test("7 % 3", "1");
///   test("2 ** 8", "256");
///
/// These mirror the truth-table in `closure-pass-constant-fold`'s
/// crate-level docs.
#[test]
fn test_basic_arithmetic_folds() {
    assert_fold(b(n(2.0), BinaryOperator::Add, n(3.0)), n(5.0));
    assert_fold(b(s("a"), BinaryOperator::Add, s("b")), s("ab"));
    assert_fold(b(s("x"), BinaryOperator::Add, n(1.0)), s("x1"));
    assert_fold(b(n(5.0), BinaryOperator::Mul, n(4.0)), n(20.0));
    assert_fold(b(n(10.0), BinaryOperator::Div, n(2.0)), n(5.0));
    assert_fold(b(n(7.0), BinaryOperator::Mod, n(3.0)), n(1.0));
    assert_fold(b(n(2.0), BinaryOperator::Exp, n(8.0)), n(256.0));
}

// =====================================================================
// `testSame` checks — upstream lines that should NOT change
// =====================================================================

/// Upstream `testNumberNumberComparison`:
///
///   testSame("+x > +y");
///   testSame("+x == +y");
///
/// Identifier-bearing expressions stay put. We cover the plain
/// identifier shape here to lock in the "don't touch identifiers" rule.
///
/// **gap-006 closed in CLOC12.23** — the `+x > +y` / `-x > -y` / `+x ==
/// +y` shapes are now covered explicitly in
/// `test_same_unary_on_identifier_in_comparison` below. `fold_unary`
/// already declines to fold `+<identifier>` (or `-<identifier>`) because
/// the runtime value is unknown, so the wrapped UnaryExpression survives
/// into `try_fold_binary_op` — which then declines because neither side
/// is a recognised literal. The fix was purely test bookkeeping; no
/// production code changed.
#[test]
fn test_same_when_either_side_has_an_identifier_subset() {
    use coding_adventures_javascript_ast::Identifier;
    let ident = |name: &str| {
        Expression::Identifier(Identifier {
            cv: None,
            name: name.to_string(),
        })
    };
    assert_same(b(ident("x"), BinaryOperator::Gt, ident("y")));
    assert_same(b(ident("x"), BinaryOperator::Eq, ident("y")));
    assert_same(b(ident("x"), BinaryOperator::StrictEq, ident("y")));
    assert_same(b(ident("x"), BinaryOperator::Gt, ident("x")));
}

/// Upstream `testNumberNumberComparison` (closes gap-006):
///
///   testSame("+x > +y");
///   testSame("+x == +y");
///
/// Pins that unary plus / minus over an identifier survives, both
/// individually and on both sides of a comparison. The desired
/// production behaviour is that the pass leaves the whole expression
/// alone — `+x` cannot be folded because `x`'s runtime value is
/// unknown, and the surrounding comparison can't be folded because
/// neither side is a recognised literal.
///
/// We verify each operand-level UnaryExpression survives (no
/// accidental coercion to NumericLiteral(0) or similar), then assert
/// the parent BinaryExpression survives under several operators.
#[test]
fn test_same_unary_on_identifier_in_comparison() {
    use coding_adventures_javascript_ast::Identifier;
    let ident = |name: &str| {
        Expression::Identifier(Identifier {
            cv: None,
            name: name.to_string(),
        })
    };
    let unary = |op: UnaryOperator, inner: Expression| {
        Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: op,
            argument: Box::new(inner),
            prefix: true,
        })
    };

    // Both sides wrapped in unary plus.
    assert_same(b(
        unary(UnaryOperator::Plus, ident("x")),
        BinaryOperator::Gt,
        unary(UnaryOperator::Plus, ident("y")),
    ));
    assert_same(b(
        unary(UnaryOperator::Plus, ident("x")),
        BinaryOperator::Eq,
        unary(UnaryOperator::Plus, ident("y")),
    ));
    assert_same(b(
        unary(UnaryOperator::Plus, ident("x")),
        BinaryOperator::StrictEq,
        unary(UnaryOperator::Plus, ident("y")),
    ));

    // Unary negate variant — same reasoning.
    assert_same(b(
        unary(UnaryOperator::Negate, ident("x")),
        BinaryOperator::Lt,
        unary(UnaryOperator::Negate, ident("y")),
    ));

    // Asymmetric: literal on one side, unary-of-identifier on the
    // other. The fold must still bail because the identifier side
    // can't be resolved.
    assert_same(b(
        Expression::NumericLiteral(NumericLiteral {
            cv: None,
            value: 0.0,
            raw: "0".to_string(),
        }),
        BinaryOperator::Lt,
        unary(UnaryOperator::Plus, ident("x")),
    ));

    // Same identifier on both sides (the structural form `+x == +x`).
    // Even though x is the same identifier, we still don't fold —
    // `x` could be NaN at runtime, and `NaN == NaN` is `false`.
    assert_same(b(
        unary(UnaryOperator::Plus, ident("x")),
        BinaryOperator::Eq,
        unary(UnaryOperator::Plus, ident("x")),
    ));
}

// =====================================================================
// Re-ports from `PeepholeRemoveDeadCodeTest` (CLOC12 gap-012 routing)
// =====================================================================
//
// Upstream's `PeepholeRemoveDeadCodeTest::testHook` exercises
// `ConditionalExpression` (ternary) cleanups. Those cleanups belong
// in *this* crate (constant-fold), not in `closure-pass-dce`. CLOC12
// gap-012 tracks the routing of those tests across crates.
//
// What we can already cover today:
//
//   * Literal-test ternary collapse: `true ? c : a` → `c`,
//     `false ? c : a` → `a`. Handled in `fold_conditional` via
//     `literal_truthy`; pinned by existing inline tests in
//     `closure-pass-constant-fold/src/lib.rs::tests::fold_*` and
//     by upstream `testHook` lines like `assertFoldSameTo(...)`.
//
// What requires Phase 1.x AST extensions before we can land:
//
//   * `var x = a ? true : true;` → `var x = (a, true);` — needs the
//     `SequenceExpression` AST node (the comma operator).
//     `a` could have observable side effects (call, getter, even an
//     undeclared-identifier ReferenceError), so the cleanup is *not*
//     `var x = true;` — it must preserve evaluating `a` for effect.
//     Without `SequenceExpression`, the upstream rewrite shape is
//     unrepresentable.
//
// Resolution: gap-012 is RESOLVED for the lines we can model today
// (via existing same-arm-folding code paths) and the
// SequenceExpression-dependent rewrites are tracked as a separate
// Phase 1.x AST gap (not a missing fold rule). The placeholder below
// makes the routing visible in the constant-fold port file's test
// listing.

/// Routing marker for CLOC12 gap-012. The upstream
/// `PeepholeRemoveDeadCodeTest::testHook` lines that depend on
/// SequenceExpression (`a ? X : X` → `(a, X)`) remain deferred until
/// `javascript-ast` grows the `SequenceExpression` variant. The
/// literal-test cases are covered by `fold_conditional` and the
/// inline tests `fold_conditional_*` in the crate's `src/lib.rs`.
#[test]
#[ignore = "blocked on SequenceExpression AST variant (Phase 1.x); literal-test cases covered by fold_conditional inline tests"]
fn test_hook_ternary_cleanup_sequence_dependent() {
    // Would assert (when SequenceExpression lands):
    //   fold("var x = a ? true : true", "var x = (a, true)");
    //   fold("var x = a ? false : false", "var x = (a, false)");
    //   fold("var x = a ? 1 : 1", "var x = (a, 1)");
    //
    // These shapes collapse the two-equal-arms case while still
    // evaluating `a` once for its observable side effects (call,
    // getter, ReferenceError from undeclared identifier).
}
