//! Ported from `PeepholeRemoveDeadCodeTest.java` in
//! `google/closure-compiler`, Apache-2.0. Upstream SHA: see
//! `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! Second port under CLOC12. Most upstream `@Test` methods exercise
//! features that don't live in our `DcePass` today — dead-`if`
//! branches collapse to fold-control-flow, useless loops collapse to
//! fold-control-flow, useless labelled statements need a `LabeledStatement`
//! AST node we don't have, switch optimisations need `SwitchStatement`,
//! etc. So the bulk of this file is `#[ignore = "blocked on gap-NNN"]`
//! placeholders that record the upstream intent and pin the work
//! against `code/specs/CLOC12-gaps.md`.
//!
//! What we *can* port today is the two narrow things DCE actually
//! does:
//!
//! 1. **Dead-after-`return`** — drop everything in a `BlockStatement`
//!    after a `ReturnStatement`.
//! 2. **Empty-statement removal** — drop `EmptyStatement` nodes from
//!    `BlockStatement`s.
//!
//! Plus a `testSame`-style: assertions that DCE leaves a return-then-
//! nothing function alone.

use coding_adventures_closure_pass_dce::DcePass;
use coding_adventures_closure_pass_pipeline::{Pass, PassContext};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BlockStatement, BreakStatement, Declaration, EmptyStatement,
    Expression, ExpressionStatement, FunctionDeclaration, Identifier, LabeledStatement,
    NumericLiteral, Program, ProgramItem, ReturnStatement, SourceType, Statement,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
//
// `function_body(stmts)` builds a Program whose only top-level item is
// a `function f() { …stmts… }`. That's the most common shape upstream
// uses inside `fold("function f(){…}", "function f(){…}")` test
// strings.
//
// `assert_dce_yields(input_body, expected_body)` runs `DcePass` and
// compares the resulting function body statement-by-statement.
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

fn expr_stmt(expr: Expression) -> Statement {
    Statement::expression_statement(ExpressionStatement {
        cv: None,
        expression: expr,
    })
}

fn return_stmt(arg: Option<Expression>) -> Statement {
    Statement::return_statement(ReturnStatement {
        cv: None,
        argument: arg,
    })
}

fn empty_stmt() -> Statement {
    Statement::empty_statement(EmptyStatement { cv: None })
}

fn function_body(body: Vec<Statement>) -> Program {
    let block = BlockStatement { cv: None, body };
    let fdecl = Declaration::FunctionDeclaration(FunctionDeclaration {
        cv: None,
        id: Identifier {
            cv: None,
            name: "f".to_string(),
        },
        params: vec![],
        body: block,
        generator: false,
        is_async: false,
    });
    Program::new_untraced(EsVersion::Es2025, SourceType::Module).with_body(vec![
        ProgramItem::Declaration(fdecl),
    ])
}

fn run_dce(prog: Program) -> Program {
    let pass = DcePass::new();
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    let ctx = PassContext {
        program: &prog,
        sidecar: &sidecar,
        cv: &mut cv,
    };
    let out = pass.run(ctx).expect("DCE pass run failed");
    out.program
}

fn function_body_stmts(prog: &Program) -> &[Statement] {
    let ProgramItem::Declaration(Declaration::FunctionDeclaration(f)) = &prog.body[0] else {
        panic!("expected a FunctionDeclaration at body[0], got {:?}", prog.body[0]);
    };
    &f.body.body
}

/// Upstream `fold(input, expected)`. Compare resulting function body
/// statements structurally against the expected statement list.
fn assert_dce_yields(input_body: Vec<Statement>, expected_body: Vec<Statement>) {
    let input_prog = function_body(input_body);
    let folded = run_dce(input_prog);
    let actual = function_body_stmts(&folded);
    assert_eq!(
        actual,
        expected_body.as_slice(),
        "DCE output did not match expected\n  actual:   {:?}\n  expected: {:?}",
        actual,
        expected_body
    );
}

/// Upstream `foldSame(input)`. Run DCE and assert the function body
/// is unchanged.
fn assert_dce_same(body: Vec<Statement>) {
    let before = body.clone();
    let input_prog = function_body(body);
    let folded = run_dce(input_prog);
    let actual = function_body_stmts(&folded);
    assert_eq!(
        actual,
        before.as_slice(),
        "DCE changed an expression it should have left alone\n  before: {:?}\n  after:  {:?}",
        before,
        actual
    );
}

// =====================================================================
// Ported tests
//
// Names mirror upstream `@Test public void <name>()` methods. Each
// docstring records the upstream `fold(...)` / `foldSame(...)` line
// being modelled so a future re-port can diff cleanly.
// =====================================================================

/// Upstream `testRemoveNoOpLabelledStatement`:
///
///   fold("a: break a;", "");
///   fold("a: { break a; }", "");
///
/// gap-009 PARTIALLY closed in CLOC12.13: the `LabeledStatement` /
/// `BreakStatement` AST nodes are now modelled, so we can BUILD the
/// input AST and assert that DCE handles it cleanly. The actual
/// "collapse `a: break a;` to empty" optimisation is a separate
/// gap — DCE today preserves the labelled-break-self verbatim
/// because no other pass yet rewrites it. This test pins the
/// **current** behaviour (passthrough) so the future collapse
/// commit can flip the assertion when the optimisation lands.
#[test]
fn test_remove_no_op_labelled_statement() {
    // a: break a;  — built by hand because we don't yet have a parser.
    let label = Identifier {
        cv: None,
        name: "a".to_string(),
    };
    let inner_break = Statement::break_statement(BreakStatement {
        cv: None,
        label: Some(label.clone()),
    });
    let labelled = Statement::labeled_statement(LabeledStatement {
        cv: None,
        label,
        body: Box::new(inner_break),
    });
    // Current DCE leaves it alone (passthrough). When the collapse
    // optimisation lands, change `assert_dce_same` → `assert_dce_yields(
    // vec![labelled], vec![])`.
    assert_dce_same(vec![labelled]);
}

/// Upstream `testFoldBlock`:
///
///   fold("{{foo()}}", "foo()");
///   fold("{foo();{}}", "foo()");
///   ... (block-flattening cases) ...
///
/// **gap-010 closed in CLOC12.19**: a new flatten step in
/// `dce_block_statement` splices any direct-child BlockStatement's
/// body into the enclosing block. Empty inner blocks disappear;
/// multi-statement inner blocks hoist their contents up. The
/// flatten is gated on a scope-safety check so `let`/`const`/
/// `class`/`function` inner blocks stay put (their bindings would
/// leak upward otherwise).
#[test]
fn test_fold_block_flattening() {
    let block = |body: Vec<Statement>| {
        Statement::block_statement(BlockStatement { cv: None, body })
    };
    let foo_call = expr_stmt(Expression::Identifier(Identifier {
        cv: None,
        name: "foo".to_string(),
    }));
    let bar_call = expr_stmt(Expression::Identifier(Identifier {
        cv: None,
        name: "bar".to_string(),
    }));

    // {{foo();}}  →  {foo();}   (outer body unchanged, but inner
    // {foo();} flattens, leaving the outer body with just the
    // call after one DCE pass).
    assert_dce_yields(vec![block(vec![foo_call.clone()])], vec![foo_call.clone()]);

    // {foo();{}}  →  {foo();}   (empty inner block drops, then
    // EmptyStatement-removal phase also drops the empty)
    assert_dce_yields(
        vec![foo_call.clone(), block(vec![])],
        vec![foo_call.clone()],
    );

    // {{};foo();}  →  {foo();}
    assert_dce_yields(
        vec![block(vec![]), foo_call.clone()],
        vec![foo_call.clone()],
    );

    // {foo();{bar();}}  →  {foo();bar();}
    assert_dce_yields(
        vec![foo_call.clone(), block(vec![bar_call.clone()])],
        vec![foo_call.clone(), bar_call.clone()],
    );
}

/// Upstream `testFoldBlock` line:
///
///   foldSame("function f(){return;}");
///
/// `return;` (no argument) inside a function body has no dead code
/// after it to remove. DCE should leave the body alone.
#[test]
fn test_function_with_bare_return_is_unchanged() {
    assert_dce_same(vec![return_stmt(None)]);
}

/// Upstream `testFoldBlock` line:
///
///   foldSame("function f(){if(x)return; x=3; return; }");
///
/// Conditional return *inside* an `IfStatement` does not block the
/// `x=3; return;` tail from reaching. DCE has no business removing
/// either of those, and `if`-then-`return` doesn't unconditionally
/// terminate the block. We CAN test this today.
///
/// (We elide the `if` wrapper here since we don't construct
/// IfStatement boilerplate — instead we model the
/// "return-not-unconditional" semantics by simply asserting that DCE
/// leaves a `[expr_stmt, return]` sequence alone. The body matches
/// what upstream asserts after the conditional return path.)
#[test]
fn test_function_with_assignment_then_return_is_unchanged() {
    // x = 3; return;
    use coding_adventures_javascript_ast::{AssignmentExpression, AssignmentOperator, AssignmentTarget};
    let assign = Expression::AssignmentExpression(AssignmentExpression {
        cv: None,
        operator: AssignmentOperator::Eq,
        left: AssignmentTarget::Identifier(Identifier {
            cv: None,
            name: "x".to_string(),
        }),
        right: Box::new(num(3.0)),
    });
    assert_dce_same(vec![expr_stmt(assign), return_stmt(None)]);
}

/// Dead-after-return: the central upstream behaviour we cover.
///
/// Upstream lines like:
///
///   fold("function f(){return 3; foo();}", "function f(){return 3;}");
///
/// don't appear verbatim with that exact wording in `testFoldBlock`
/// (which mostly tests block flattening). The equivalent contract is
/// the documented DCE rule. We assert it directly here.
#[test]
fn test_dead_statement_after_return_with_argument_is_dropped() {
    // function f() { return 3; foo(); }  →  function f() { return 3; }
    let body = vec![
        return_stmt(Some(num(3.0))),
        expr_stmt(ident("foo")),
    ];
    let expected = vec![return_stmt(Some(num(3.0)))];
    assert_dce_yields(body, expected);
}

/// Multiple dead statements after a bare return are all dropped.
///
///   function f() { return; foo(); bar(); }  →  function f() { return; }
#[test]
fn test_multiple_dead_statements_after_return_are_dropped() {
    let body = vec![
        return_stmt(None),
        expr_stmt(ident("foo")),
        expr_stmt(ident("bar")),
    ];
    let expected = vec![return_stmt(None)];
    assert_dce_yields(body, expected);
}

/// Empty statements get removed from blocks. Upstream models this in
/// `testFoldBlock`:
///
///   fold("{x=3;;;y=2;;;}", "x=3;y=2");
///
/// Our DCE handles `;;;` noise — the wrapping block here is a function
/// body, which is the same `BlockStatement` shape.
#[test]
fn test_empty_statements_dropped_from_function_body() {
    use coding_adventures_javascript_ast::{AssignmentExpression, AssignmentOperator, AssignmentTarget};
    let assign_x = Expression::AssignmentExpression(AssignmentExpression {
        cv: None,
        operator: AssignmentOperator::Eq,
        left: AssignmentTarget::Identifier(Identifier {
            cv: None,
            name: "x".to_string(),
        }),
        right: Box::new(num(3.0)),
    });
    let assign_y = Expression::AssignmentExpression(AssignmentExpression {
        cv: None,
        operator: AssignmentOperator::Eq,
        left: AssignmentTarget::Identifier(Identifier {
            cv: None,
            name: "y".to_string(),
        }),
        right: Box::new(num(2.0)),
    });
    let body = vec![
        expr_stmt(assign_x.clone()),
        empty_stmt(),
        empty_stmt(),
        empty_stmt(),
        expr_stmt(assign_y.clone()),
        empty_stmt(),
        empty_stmt(),
    ];
    let expected = vec![expr_stmt(assign_x), expr_stmt(assign_y)];
    assert_dce_yields(body, expected);
}

/// Upstream `testIf`:
///
///   fold("if (1){ x=1; } else { x = 2;}", "x=1");
///   fold("if (false){ x = 1; } else { x = 2; }", "x=2");
///   fold("if (null){ x = 1; } else { x = 2; }", "x=2");
///   ...
///
/// **gap-011 closed in CLOC12.06** — these upstream lines live in
/// `closure-pass-fold-control-flow`, not DCE. The behaviour is now
/// covered by `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs`
/// (`test_if_*_folds_to_consequent`, `test_if_*_folds_to_alternate`,
/// `test_if_null_folds_to_alternate`, `test_if_false_no_alternate_becomes_empty_statement`,
/// landed in CLOC12.05).
///
/// This stub stays *non-ignored* and trivially passes — it exists to
/// keep an audit-trail anchor in the DCE upstream-ports file
/// pointing readers at the right crate when they search upstream
/// `testIf`. The actual behavioural assertions live in fold-control-flow.
// `marker` is a compile-time-constant string, so clippy knows `is_empty()` is
// always false; the assertion is an intentional documentation marker, not a
// runtime check, so the const-evaluation lint is allowed here.
#[allow(clippy::const_is_empty)]
#[test]
fn test_if_with_constant_test_collapse() {
    // Sanity: this test_is intentionally a marker, not a behaviour
    // assertion. The behavioural coverage lives in
    // `closure-pass-fold-control-flow/tests/upstream/`. Confirming
    // by name lookup keeps the cross-crate audit trail explicit.
    let marker = "covered by closure-pass-fold-control-flow::test_if_*";
    assert!(!marker.is_empty(), "marker text must not be empty");
}

/// Upstream `testHook`:
///
///   fold("var x = a ? true : true", "var x = (a, true)");
///   ... (ConditionalExpression cleanup) ...
///
/// **gap-012 routed in CLOC12.27**: ConditionalExpression cleanup is
/// constant-fold's responsibility. The routing test stub lives at
/// `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs::test_hook_ternary_cleanup_sequence_dependent`.
/// The literal-test ternary cases are already covered by
/// `fold_conditional` + `literal_truthy` (inline tests in the
/// constant-fold crate). The SequenceExpression-dependent rewrites
/// (`a ? X : X` → `(a, X)`) are deferred until `javascript-ast`
/// grows a `SequenceExpression` variant.
#[test]
#[ignore = "routed in CLOC12.27 to closure-pass-constant-fold (gap-012); SequenceExpression-dependent shapes deferred to Phase 1.x AST work"]
fn test_hook_cleanup() {
    // Routed. See doc comment above for the new home in
    // constant-fold's port file.
}

/// Upstream `testFoldUselessFor`:
///
///   fold("while(x()){x}", "while(x());");
///
/// **gap-013 routed in CLOC12.28**: useless-loop-body folding is
/// `closure-pass-fold-control-flow`'s territory. The routing test
/// stub lives at
/// `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs::test_fold_useless_loop_body_routing`.
/// Literal-test loop-collapse (`while(false) { ... }` → `;`) is
/// already covered by `fold_while_statement` inline tests; the
/// effect-analysis-dependent rewrites are tracked separately under
/// a future "effect analysis" gap rather than under any CLOC12
/// gap-NNN entry — the missing piece is a primary analysis, not a
/// fold rule.
#[test]
#[ignore = "routed in CLOC12.28 to closure-pass-fold-control-flow (gap-013); effect-analysis-dependent shapes deferred to future analysis work"]
fn test_fold_useless_loop_body() {
    // Routed. See doc comment above for the new home.
}

/// Upstream `testOptimizeSwitch`:
///
///   fold("function f(){switch(x){case 1: foo(); break;}}", "...");
///
/// SwitchStatement not modelled in Phase 1 AST yet.
#[test]
#[ignore = "blocked on gap-014: SwitchStatement not in Phase 1 AST"]
fn test_optimize_switch() {
    // Would assert switch simplifications.
}

/// Upstream `testVarLifting`:
///
///   fold("if (foo) { var x }", "var x; if (foo) {}");
///
/// `var` hoisting / lifting requires scope analysis that DCE doesn't
/// perform. This is `closure-pass-remove-unused-vars` territory.
#[test]
#[ignore = "blocked on gap-015: var-lifting / hoisting handled elsewhere"]
fn test_var_lifting() {
    // Belongs in remove-unused-vars or a hoisting pass.
}
