//! Ported from `PeepholeMinimizeConditionsTest.java` in
//! `google/closure-compiler`, Apache-2.0. Upstream SHA: see
//! `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! Third port under CLOC12 — first port against
//! `closure-pass-fold-control-flow`. Most upstream `@Test` methods
//! exercise behaviour our pass doesn't have yet — converting
//! `if (x) foo()` to `x && foo()`, hoisting ternaries out of
//! if-else, De Morgan rewrites, etc. So the bulk of this file is
//! `#[ignore = "blocked on gap-NNN"]` placeholders that pin the
//! upstream intent.
//!
//! What we *can* port today are literal-test if-folds:
//!
//!   if (true)  C; else A;   →  C;
//!   if (false) C; else A;   →  A;
//!   if (false) C;           →  ;        (EmptyStatement)
//!   if (1)     C; else A;   →  C;       (literal_truthy on numeric)
//!   if (0)     C; else A;   →  A;       (literal_falsy)
//!   if ("hi")  C; else A;   →  C;
//!   if ("")    C; else A;   →  A;
//!   if (null)  C; else A;   →  A;
//!
//! Plus `testSame` for non-literal tests:
//!
//!   if (x) C; else A;       →  unchanged

use coding_adventures_closure_pass_fold_control_flow::FoldControlFlowPass;
use coding_adventures_closure_pass_pipeline::{Pass, PassContext};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BooleanLiteral, EmptyStatement, Expression, ExpressionStatement,
    Identifier, IfStatement, NullLiteral, NumericLiteral, Program, ProgramItem, SourceType,
    Statement, StringLiteral, UnaryExpression, UnaryOperator,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
//
// Same pattern as the inline tests: build literals + helper to wrap an
// IfStatement into a single-statement Program, run the pass, and pull
// out the (possibly-folded) top-level statement for assertion.
// =====================================================================

fn ident(name: &str) -> Expression {
    Expression::Identifier(Identifier {
        cv: None,
        name: name.to_string(),
    })
}
fn boolean(v: bool) -> Expression {
    Expression::BooleanLiteral(BooleanLiteral { cv: None, value: v })
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
fn null_lit() -> Expression {
    Expression::NullLiteral(NullLiteral { cv: None })
}

fn expr_stmt(expr: Expression) -> Statement {
    Statement::expression_statement(ExpressionStatement {
        cv: None,
        expression: expr,
    })
}

fn if_stmt(test: Expression, consequent: Statement, alternate: Option<Statement>) -> Statement {
    Statement::if_statement(IfStatement {
        cv: None,
        test,
        consequent: Box::new(consequent),
        alternate: alternate.map(Box::new),
    })
}

fn program_with(stmt: Statement) -> Program {
    Program::new_untraced(EsVersion::Es2025, SourceType::Module)
        .with_body(vec![ProgramItem::Statement(stmt)])
}

fn run_fcf(prog: Program) -> Program {
    let pass = FoldControlFlowPass::new();
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    let ctx = PassContext {
        program: &prog,
        sidecar: &sidecar,
        cv: &mut cv,
    };
    let out = pass.run(ctx).expect("FCF pass run failed");
    out.program
}

fn first_stmt(prog: &Program) -> &Statement {
    let ProgramItem::Statement(s) = &prog.body[0] else {
        panic!("expected a Statement at body[0]");
    };
    s
}

/// Upstream `fold(input, expected)`. Wrap input in a program, run the
/// pass, and assert the resulting top-level statement equals `expected`.
fn assert_fold(input: Statement, expected: Statement) {
    let out = run_fcf(program_with(input));
    let actual = first_stmt(&out);
    assert_eq!(
        actual, &expected,
        "fold output did not match expected\n  actual:   {:?}\n  expected: {:?}",
        actual, expected
    );
}

fn assert_same(input: Statement) {
    let before = input.clone();
    let out = run_fcf(program_with(input));
    let actual = first_stmt(&out);
    assert_eq!(
        actual, &before,
        "pass changed a statement it should have left alone\n  before: {:?}\n  after:  {:?}",
        before, actual
    );
}

fn empty_stmt() -> Statement {
    Statement::empty_statement(EmptyStatement { cv: None })
}

// =====================================================================
// Ported tests
//
// Names mirror upstream `@Test public void <name>()` methods or a
// disambiguating suffix when we cover only a subset of the upstream
// method.
// =====================================================================

/// Upstream `testFoldOneChildBlocks`:
///
///   fold("function f(){if(x)a();x=3}", "function f(){x&&a();x=3}");
///   fold("function f(){if(x){a()}x=3}", "function f(){x&&a();x=3}");
///   ... (many more, all rewriting `if(x) S` to `x && S`)
///
/// **gap-016 closed in CLOC12.24**: when `test` is non-literal,
/// the consequent reduces to a single ExpressionStatement (directly
/// or via single-statement BlockStatement layers), and there is *no*
/// alternate, the IfStatement rewrites to an ExpressionStatement
/// wrapping a LogicalExpression with operator `And`. Side-effect
/// order is preserved because `&&` evaluates `test` first and the
/// right operand only when `test` is truthy — identical to `if (test) S`.
#[test]
fn test_fold_one_child_blocks_if_to_logical_and() {
    use coding_adventures_javascript_ast::{
        BlockStatement, CallExpression, LogicalExpression, LogicalOperator,
    };
    let call = |name: &str| {
        Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident(name)),
            arguments: vec![],
        })
    };
    let block = |s: Statement| {
        Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![s],
        })
    };
    let expect_logical_and = |t: Expression, c: Expression| -> Statement {
        Statement::expression_statement(ExpressionStatement {
            cv: None,
            expression: Expression::LogicalExpression(LogicalExpression {
                cv: None,
                operator: LogicalOperator::And,
                left: Box::new(t),
                right: Box::new(c),
            }),
        })
    };

    // 1. Bare consequent: `if (x) a();` → `x && a();`
    assert_fold(
        if_stmt(ident("x"), expr_stmt(call("a")), None),
        expect_logical_and(ident("x"), call("a")),
    );

    // 2. Single-statement block consequent: `if (x) { a(); }` → `x && a();`
    assert_fold(
        if_stmt(ident("x"), block(expr_stmt(call("a"))), None),
        expect_logical_and(ident("x"), call("a")),
    );

    // 3. Nested single-statement blocks: `if (x) {{ a(); }}` → `x && a();`
    // (single_expr_stmt recurses through BlockStatement layers.)
    assert_fold(
        if_stmt(ident("x"), block(block(expr_stmt(call("a")))), None),
        expect_logical_and(ident("x"), call("a")),
    );

    // 4. Bare assignment in consequent: `if (x) y;` → `x && y;`
    // (Any identifier is a valid expression statement.)
    assert_fold(
        if_stmt(ident("x"), expr_stmt(ident("y")), None),
        expect_logical_and(ident("x"), ident("y")),
    );
}

/// Upstream `testFoldOneChildBlocks` line:
///
///   fold("function f(){if(x){foo()}else{bar()}}",
///        "function f(){x?foo():bar()}");
///
/// **gap-017 closed in CLOC12.18**: when both `consequent` and
/// `alternate` reduce to a single `ExpressionStatement` (directly
/// or via single-statement BlockStatement layers), the IfStatement
/// rewrites to an `ExpressionStatement` wrapping a
/// `ConditionalExpression`. Side-effect order is preserved because
/// a ternary evaluates `test`, then exactly one branch — identical
/// to the if-else.
#[test]
fn test_fold_one_child_blocks_if_else_to_ternary() {
    use coding_adventures_javascript_ast::{
        BlockStatement, CallExpression, ConditionalExpression,
    };
    let call = |name: &str| {
        Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident(name)),
            arguments: vec![],
        })
    };
    let block = |s: Statement| {
        Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![s],
        })
    };
    let expect_ternary = |t: Expression, c: Expression, a: Expression| -> Statement {
        Statement::expression_statement(ExpressionStatement {
            cv: None,
            expression: Expression::ConditionalExpression(ConditionalExpression {
                cv: None,
                test: Box::new(t),
                consequent: Box::new(c),
                alternate: Box::new(a),
            }),
        })
    };

    // if (x) foo(); else bar();  →  x ? foo() : bar();
    let inp = if_stmt(
        ident("x"),
        expr_stmt(call("foo")),
        Some(expr_stmt(call("bar"))),
    );
    assert_fold(inp, expect_ternary(ident("x"), call("foo"), call("bar")));

    // if (x) { foo(); } else { bar(); }  →  same ternary (single-
    // statement block unwraps).
    let inp_blocks = if_stmt(
        ident("x"),
        block(expr_stmt(call("foo"))),
        Some(block(expr_stmt(call("bar")))),
    );
    assert_fold(
        inp_blocks,
        expect_ternary(ident("x"), call("foo"), call("bar")),
    );

    // **Pre-gap-016**: no alternate → stays an IfStatement (testSame).
    // **Post-gap-016** (CLOC12.24): now folds to `x && foo();`.
    //
    // This is the canonical gap-016 case; the dedicated
    // `test_fold_one_child_blocks_if_to_logical_and` test above
    // covers it (along with several other shapes), so we no longer
    // need a duplicate assertion here. Pre-gap-016 the assertion
    // here was identity; we leave the comment as a historical marker
    // so the gap-017 vs gap-016 split stays visible in the file.
}

/// Constant true test always selects the consequent. Mirrors upstream's
/// `fold("if (true) { f.onchange(); }", "if (1) f.onchange();")`-shape
/// behaviour from `testFoldOneChildBlocks` (line 797) — we don't
/// rewrite `true` to `1`, but we *do* fold the if entirely.
#[test]
fn test_if_true_folds_to_consequent() {
    // if (true) x; else y;  →  x;
    let inp = if_stmt(boolean(true), expr_stmt(ident("x")), Some(expr_stmt(ident("y"))));
    assert_fold(inp, expr_stmt(ident("x")));
}

/// Constant false test always selects the alternate.
#[test]
fn test_if_false_folds_to_alternate() {
    // if (false) x; else y;  →  y;
    let inp = if_stmt(boolean(false), expr_stmt(ident("x")), Some(expr_stmt(ident("y"))));
    assert_fold(inp, expr_stmt(ident("y")));
}

/// Constant false test with no alternate collapses to `;`
/// (EmptyStatement). DCE then strips it.
#[test]
fn test_if_false_no_alternate_becomes_empty_statement() {
    // if (false) x;  →  ;
    let inp = if_stmt(boolean(false), expr_stmt(ident("x")), None);
    assert_fold(inp, empty_stmt());
}

/// Numeric truthy: `if (1) x else y` → `x`.
///
/// Models the upstream line `fold("if (true) ...", "if (1) ...")` in
/// the limit where our pass commits to folding the whole `if`.
#[test]
fn test_if_numeric_one_folds_to_consequent() {
    let inp = if_stmt(num(1.0), expr_stmt(ident("x")), Some(expr_stmt(ident("y"))));
    assert_fold(inp, expr_stmt(ident("x")));
}

/// Numeric falsy: `if (0) x else y` → `y`.
#[test]
fn test_if_numeric_zero_folds_to_alternate() {
    let inp = if_stmt(num(0.0), expr_stmt(ident("x")), Some(expr_stmt(ident("y"))));
    assert_fold(inp, expr_stmt(ident("y")));
}

/// Non-empty string truthy: `if ("hi") x else y` → `x`.
#[test]
fn test_if_nonempty_string_folds_to_consequent() {
    let inp = if_stmt(string("hi"), expr_stmt(ident("x")), Some(expr_stmt(ident("y"))));
    assert_fold(inp, expr_stmt(ident("x")));
}

/// Empty string falsy: `if ("") x else y` → `y`.
#[test]
fn test_if_empty_string_folds_to_alternate() {
    let inp = if_stmt(string(""), expr_stmt(ident("x")), Some(expr_stmt(ident("y"))));
    assert_fold(inp, expr_stmt(ident("y")));
}

/// Null falsy: `if (null) x else y` → `y`.
///
/// Mirrors upstream `fold("if (null){ x = 1; } else { x = 2; }", "x=2")`
/// from `testIf` in PeepholeRemoveDeadCodeTest, which routes to this
/// pass per the gap-011 routing entry filed in CLOC12.04.
#[test]
fn test_if_null_folds_to_alternate() {
    let inp = if_stmt(null_lit(), expr_stmt(ident("x")), Some(expr_stmt(ident("y"))));
    assert_fold(inp, expr_stmt(ident("y")));
}

/// Upstream `testSame("if (x) C else A")` (when C and A are not single
/// ExpressionStatements) — non-literal test stays as an IfStatement.
///
/// **Updated in CLOC12.18**: the original `if (x) c; else a;` shape
/// (two single ExpressionStatement branches) is no longer a testSame —
/// the new gap-017 ternary fold rewrites it. So this test now uses a
/// multi-statement consequent block to keep the testSame behaviour
/// at this seam. The original single-expr case is covered by
/// `test_fold_one_child_blocks_if_else_to_ternary` above.
#[test]
fn test_if_non_literal_test_left_alone() {
    use coding_adventures_javascript_ast::BlockStatement;
    let multi_block = Statement::block_statement(BlockStatement {
        cv: None,
        body: vec![expr_stmt(ident("c1")), expr_stmt(ident("c2"))],
    });
    let inp = if_stmt(ident("x"), multi_block, Some(expr_stmt(ident("a"))));
    assert_same(inp);
}

/// Upstream `testFoldConditionalDeMorgan`:
///
///   fold("if (!a) { foo() } else { bar() }",
///        "if (a) { bar() } else { foo() }");
///
/// **gap-018 closed in CLOC12.25**: `fold_if_statement` now strips
/// a top-level `!` from the test and swaps consequent/alternate when
/// an alternate is present. The mirrored ternary case is in
/// `fold_conditional`. Both rules require that the operand is moved
/// (not cloned) — no second runtime evaluation of `<inner>` is
/// introduced.
#[test]
fn test_fold_conditional_de_morgan() {
    use coding_adventures_javascript_ast::{BlockStatement, CallExpression};
    let call = |name: &str| {
        Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident(name)),
            arguments: vec![],
        })
    };
    let block = |s: Statement| {
        Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![s],
        })
    };

    // Upstream literal-form: `if (!a) { foo() } else { bar() }` →
    // `if (a) { bar() } else { foo() }`. Our AST builder makes the
    // input slightly more granular; the semantic check is the same.
    let not_a = Expression::UnaryExpression(UnaryExpression {
        cv: None,
        operator: UnaryOperator::Not,
        argument: Box::new(ident("a")),
        prefix: true,
    });
    let inp = if_stmt(
        not_a,
        block(expr_stmt(call("foo"))),
        Some(block(expr_stmt(call("bar")))),
    );
    // After the swap, gap-017 ternary fires on the now-single-expr
    // arms, so the final shape is `a ? bar() : foo();` —
    // observationally equivalent to upstream's
    // `if (a) { bar() } else { foo() }`.
    let expected = Statement::expression_statement(ExpressionStatement {
        cv: None,
        expression: Expression::ConditionalExpression(
            coding_adventures_javascript_ast::ConditionalExpression {
                cv: None,
                test: Box::new(ident("a")),
                consequent: Box::new(call("bar")),
                alternate: Box::new(call("foo")),
            },
        ),
    });
    assert_fold(inp, expected);
}

/// Upstream `testFoldReturns`:
///
///   fold("function f(){if(x)return 1;else return 2}",
///        "function f(){return x?1:2}");
///
/// Hoisting two return statements through an if-else into a single
/// ternary-returning return needs the ternary rewrite from gap-017
/// plus return-statement-aware rewriting.
/// **gap-019 closed in CLOC12.26**: `fold_if_statement` now hoists
/// terminal `return E1; / return E2;` branches into a single
/// `return test ? E1 : E2;`. See `single_return_with_arg` for the
/// helper that recognises the single-ReturnStatement-with-argument
/// shape (recurses through single-statement BlockStatement layers).
#[test]
fn test_fold_returns_into_ternary() {
    use coding_adventures_javascript_ast::{
        BlockStatement, ConditionalExpression, ReturnStatement,
    };
    let block = |s: Statement| {
        Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![s],
        })
    };
    let ret = |arg: Expression| {
        Statement::return_statement(ReturnStatement {
            cv: None,
            argument: Some(arg),
        })
    };
    let num_lit = |v: f64| {
        Expression::NumericLiteral(NumericLiteral {
            cv: None,
            value: v,
            raw: format!("{}", v as i64),
        })
    };

    // Bare returns on both branches: `if (x) return 1; else return 2;`
    // → `return x ? 1 : 2;`.
    let inp = if_stmt(
        ident("x"),
        ret(num_lit(1.0)),
        Some(ret(num_lit(2.0))),
    );
    let expected = Statement::return_statement(ReturnStatement {
        cv: None,
        argument: Some(Expression::ConditionalExpression(ConditionalExpression {
            cv: None,
            test: Box::new(ident("x")),
            consequent: Box::new(num_lit(1.0)),
            alternate: Box::new(num_lit(2.0)),
        })),
    });
    assert_fold(inp, expected);

    // Block-wrapped returns: `if (x) { return 1; } else { return 2; }`
    // → `return x ? 1 : 2;`. Same shape after fold.
    let inp_blocks = if_stmt(
        ident("x"),
        block(ret(num_lit(1.0))),
        Some(block(ret(num_lit(2.0)))),
    );
    let expected_again = Statement::return_statement(ReturnStatement {
        cv: None,
        argument: Some(Expression::ConditionalExpression(ConditionalExpression {
            cv: None,
            test: Box::new(ident("x")),
            consequent: Box::new(num_lit(1.0)),
            alternate: Box::new(num_lit(2.0)),
        })),
    });
    assert_fold(inp_blocks, expected_again);

    // Bare-return on one side should NOT fold (the helper bails
    // when argument is None). Verifies the conservative guard.
    let inp_bare = if_stmt(
        ident("x"),
        Statement::return_statement(ReturnStatement {
            cv: None,
            argument: None,
        }),
        Some(ret(num_lit(2.0))),
    );
    let inp_bare_clone = inp_bare.clone();
    assert_fold(inp_bare, inp_bare_clone);
}

/// Upstream `testMinimizeIfWithThrow`:
///
///   fold("if (x) { foo(); } else { throw 1; }", "if (!x) throw 1; foo();");
///
/// ThrowStatement isn't in the Phase 1 typed AST yet.
#[test]
#[ignore = "blocked on gap-020: ThrowStatement not in Phase 1 AST"]
fn test_minimize_if_with_throw() {
    // Needs ThrowStatement AST variant + the rearrangement rule.
}

// =====================================================================
// Re-ports from `PeepholeRemoveDeadCodeTest` (CLOC12 gap-013 routing)
// =====================================================================
//
// Upstream's `PeepholeRemoveDeadCodeTest::testFoldUselessFor`,
// `testFoldUselessDo`, `testFoldEmptyDo`, and `testMinimizeLoop_*`
// exercise body-pruning for `while` / `do-while` / `for` loops:
// when the loop body provably has no observable effects, replace it
// with an empty statement (or fold the surrounding loop). This is
// `closure-pass-fold-control-flow`'s territory — DCE only handles
// post-`return`/`throw` unreachability and block-flattening, not
// effect analysis on loop bodies.
//
// What we can already cover today:
//
//   * `while(<falsy literal>) { ... }` → `;`. Handled by
//     `fold_while_statement::literal_truthy(Some(false))` and pinned
//     by inline tests in `closure-pass-fold-control-flow/src/lib.rs`.
//
// What requires future work before we can land:
//
//   * `while(x) {}` → `while(x);` (body-already-empty canonicalisation).
//     Mostly a cosmetic emit-side difference; the AST is already
//     `WhileStatement { body: BlockStatement { body: [] } }` and the
//     emitter could be taught to render the empty-body form as `;`.
//   * `do {} while(x)` → `x;` (drop the do-while when body is empty
//     and discard the loop's iteration count, since the condition
//     can never re-execute). Requires the rule plus emitter cooperation.
//   * `while(x) { S }` where `S` is provably pure → `while(x);`.
//     **Requires effect analysis** that we don't have. Same blocker
//     as several other gap-NNN entries that want to know "does this
//     expression / statement have observable effects?". Tracked
//     separately under a future "effect analysis" gap, not under any
//     CLOC12 gap-NNN entry — the missing piece is a primary
//     analysis, not a fold rule.

/// Routing marker for CLOC12 gap-013. The upstream
/// `PeepholeRemoveDeadCodeTest::testFoldUselessFor` /
/// `testFoldUselessDo` / `testFoldEmptyDo` / `testMinimizeLoop_*`
/// tests live here logically. The fold rules they exercise depend on
/// effect analysis we don't yet have (see the module-level comment
/// above). The literal-test loop-collapse cases are covered by
/// `fold_while_statement`'s inline tests in the crate's `src/lib.rs`.
#[test]
#[ignore = "blocked on effect-analysis machinery (separate future gap); literal-test cases covered by fold_while_statement inline tests"]
fn test_fold_useless_loop_body_routing() {
    // Would assert (when effect analysis lands):
    //   fold("while(x()){x}", "while(x());");
    //   fold("do{}while(x)", "x;");
    //   fold("for(;;){pure(...)}", "for(;;);");
    //
    // Each requires proving the body has no observable side effects
    // (no calls into unknown functions, no assignments, no
    // identifier evaluations that could throw ReferenceError, no
    // getter access).
}
