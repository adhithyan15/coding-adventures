//! Ported from `UnreachableCodeEliminationTest.java` in
//! `google/closure-compiler`, Apache-2.0. Upstream SHA: see
//! `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! The **second** CLOC12 port into `closure-pass-dce` (alongside the
//! `PeepholeRemoveDeadCode` port). Upstream `UnreachableCodeElimination`
//! walks the control-flow graph and deletes any statement that cannot be
//! reached — code after a `return`/`throw`, code after `break`/`continue`
//! inside a loop, code after an `if` whose every branch terminates, empty
//! statements, and so on — while preserving *hoisted* `var`/`function`
//! declarations (which are reachable regardless of textual position).
//!
//! closurec's `DcePass` implements the **provably-sound block-level core**
//! of that: inside any `BlockStatement.body` it drops everything after a
//! `ReturnStatement`/`ThrowStatement`, drops `EmptyStatement`s, and (for
//! hoisting soundness) *declines* to truncate a tail that contains a
//! `var`/`function` (or a compound statement that could wrap one) — a
//! decline is never a miscompile, and `remove-unused-vars` cleans a
//! genuinely-dead hoisted binding downstream. Break/continue are only
//! treated as terminators inside switch-case consequents, not at the
//! general block level, and an `if`-both-branches-terminate is not yet
//! recognised — those are the gaps.
//!
//! ## Harness
//!
//! `closure-pass-dce` has no source-string entry point in its test
//! harness, so — exactly as the sibling `PeepholeRemoveDeadCode` port
//! does — each case is built directly on the typed AST via small helpers
//! and compared statement-by-statement against the expected function
//! body. `assert_dce_yields` models upstream `fold(input, expected)`;
//! `assert_dce_same` models `foldSame(input)`.

use coding_adventures_closure_pass_dce::DcePass;
use coding_adventures_closure_pass_pipeline::{Pass, PassContext};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BlockStatement, BreakStatement, Declaration, EmptyStatement,
    Expression, ExpressionStatement, FunctionDeclaration, Identifier, IfStatement, NumericLiteral,
    Program, ProgramItem, ReturnStatement, SourceType, Statement, ThrowStatement, VarKind,
    VariableDeclaration, VariableDeclarator, WhileStatement,
};
use coding_adventures_javascript_ast::BindingTarget;
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers (mirroring the sibling PeepholeRemoveDeadCode port)
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

/// A bare `name;` expression statement — a reachable, side-effecting-ish
/// statement used as the "dead tail" the pass should drop.
fn expr_stmt(name: &str) -> Statement {
    Statement::expression_statement(ExpressionStatement {
        cv: None,
        expression: ident(name),
    })
}

fn return_stmt(arg: Option<Expression>) -> Statement {
    Statement::return_statement(ReturnStatement { cv: None, argument: arg })
}

fn throw_stmt(arg: Expression) -> Statement {
    Statement::throw_statement(ThrowStatement { cv: None, argument: arg })
}

fn empty_stmt() -> Statement {
    Statement::empty_statement(EmptyStatement { cv: None })
}

fn break_stmt() -> Statement {
    Statement::break_statement(BreakStatement { cv: None, label: None })
}

fn block(body: Vec<Statement>) -> Statement {
    Statement::block_statement(BlockStatement { cv: None, body })
}

/// A `var name = value;` declaration statement — carries a *hoisted*
/// binding, so a DCE tail containing it must NOT be truncated.
fn var_stmt(name: &str, value: Expression) -> Statement {
    Statement::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
        cv: None,
        kind: VarKind::Var,
        declarations: vec![VariableDeclarator {
            cv: None,
            id: BindingTarget::Identifier(Identifier { cv: None, name: name.to_string() }),
            init: Some(value),
        }],
    }))
}

fn if_stmt(test: Expression, consequent: Statement, alternate: Option<Statement>) -> Statement {
    Statement::if_statement(IfStatement {
        cv: None,
        test,
        consequent: Box::new(consequent),
        alternate: alternate.map(Box::new),
    })
}

fn while_stmt(test: Expression, body: Statement) -> Statement {
    Statement::while_statement(WhileStatement {
        cv: None,
        test,
        body: Box::new(body),
    })
}

fn function_body(body: Vec<Statement>) -> Program {
    let fdecl = Declaration::FunctionDeclaration(FunctionDeclaration {
        cv: None,
        id: Identifier { cv: None, name: "f".to_string() },
        params: vec![],
        body: BlockStatement { cv: None, body },
        generator: false,
        is_async: false,
    });
    Program::new_untraced(EsVersion::Es2025, SourceType::Module)
        .with_body(vec![ProgramItem::Declaration(fdecl)])
}

fn run_dce(prog: Program) -> Program {
    let pass = DcePass::new();
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    pass.run(PassContext { program: &prog, sidecar: &sidecar, cv: &mut cv })
        .expect("DCE pass run failed")
        .program
}

fn function_body_stmts(prog: &Program) -> Vec<Statement> {
    let ProgramItem::Declaration(Declaration::FunctionDeclaration(f)) = &prog.body[0] else {
        panic!("expected FunctionDeclaration at body[0]");
    };
    f.body.body.clone()
}

/// Upstream `fold(input, expected)`.
fn assert_dce_yields(input_body: Vec<Statement>, expected_body: Vec<Statement>) {
    let folded = run_dce(function_body(input_body));
    assert_eq!(
        function_body_stmts(&folded),
        expected_body,
        "DCE output did not match expected"
    );
}

/// Upstream `foldSame(input)`.
fn assert_dce_same(body: Vec<Statement>) {
    let before = body.clone();
    let folded = run_dce(function_body(body));
    assert_eq!(function_body_stmts(&folded), before, "DCE changed statements it should have kept");
}

// =====================================================================
// Active — reachability cleanup the pass performs today
// =====================================================================

/// Upstream `testRemoveUnreachableCode` (return subset): a single
/// statement after `return` is unreachable and dropped.
#[test]
fn drops_single_statement_after_return() {
    assert_dce_yields(
        vec![return_stmt(None), expr_stmt("x")],
        vec![return_stmt(None)],
    );
}

/// Multiple dead statements after `return` all go.
#[test]
fn drops_multiple_statements_after_return() {
    assert_dce_yields(
        vec![return_stmt(Some(num(1.0))), expr_stmt("x"), expr_stmt("y")],
        vec![return_stmt(Some(num(1.0)))],
    );
}

/// Code after a `throw` is unreachable and dropped.
#[test]
fn drops_statement_after_throw() {
    assert_dce_yields(
        vec![throw_stmt(ident("e")), expr_stmt("x")],
        vec![throw_stmt(ident("e"))],
    );
}

/// Reachable code BEFORE the terminator is kept.
#[test]
fn keeps_reachable_code_before_return() {
    assert_dce_same(vec![expr_stmt("x"), return_stmt(None)]);
}

/// A bare `return` with nothing after it is unchanged.
#[test]
fn bare_return_unchanged() {
    assert_dce_same(vec![return_stmt(None)]);
}

/// Upstream `testRemoveUselessNameStatements`-adjacent: empty statements
/// are dropped from a block.
#[test]
fn drops_empty_statements() {
    assert_dce_yields(
        vec![expr_stmt("x"), empty_stmt(), empty_stmt()],
        vec![expr_stmt("x")],
    );
}

/// Dead-after-return cleanup reaches into a NESTED block. The block is an
/// `if`-consequent (not subject to the top-level block-flattening that
/// would splice a bare `{ return }` into the function body), so it stays a
/// block and we can assert its interior was cleaned.
#[test]
fn drops_dead_code_in_nested_block() {
    assert_dce_yields(
        vec![if_stmt(
            ident("c"),
            block(vec![return_stmt(None), expr_stmt("x")]),
            None,
        )],
        vec![if_stmt(ident("c"), block(vec![return_stmt(None)]), None)],
    );
}

/// SOUNDNESS (hoisting): a tail after `return` that contains a hoisted
/// `var` is NOT truncated — the declaration is reachable regardless of
/// position. closurec conservatively keeps the whole tail (upstream would
/// keep the hoisted `var x` but drop its initializer and the dead call;
/// see gap-153). Declining to truncate is never a miscompile.
#[test]
fn declines_to_truncate_tail_with_hoisted_var() {
    assert_dce_same(vec![
        return_stmt(None),
        var_stmt("x", num(1.0)),
        expr_stmt("g"),
    ]);
}

// =====================================================================
// Ignored — upstream reachability analysis we do not do yet.
// =====================================================================

/// Upstream removes code after an `if` whose EVERY branch terminates
/// (`if (c) return; else return; y();` → the `y()` is unreachable). Ours
/// only truncates after a *direct* terminator statement; an `IfStatement`
/// is not `is_terminator`, so `y()` survives today.
#[test]
#[ignore = "blocked on gap-151: if-both-branches-terminate not recognised as making the tail unreachable"]
fn drops_code_after_if_both_branches_return() {
    assert_dce_yields(
        vec![
            if_stmt(
                ident("c"),
                block(vec![return_stmt(None)]),
                Some(block(vec![return_stmt(None)])),
            ),
            expr_stmt("y"),
        ],
        vec![if_stmt(
            ident("c"),
            block(vec![return_stmt(None)]),
            Some(block(vec![return_stmt(None)])),
        )],
    );
}

/// Upstream treats `break`/`continue` as terminating the enclosing loop
/// body, so code after a `break` inside a loop is unreachable
/// (`while (c) { break; y(); }` → `while (c) { break; }`). closurec only
/// treats break/continue as terminators inside switch-case consequents,
/// so the loop-body `y()` survives today.
#[test]
#[ignore = "blocked on gap-152: break/continue only terminate switch cases, not general loop blocks"]
fn drops_code_after_break_in_loop_block() {
    assert_dce_yields(
        vec![while_stmt(ident("c"), block(vec![break_stmt(), expr_stmt("y")]))],
        vec![while_stmt(ident("c"), block(vec![break_stmt()]))],
    );
}
