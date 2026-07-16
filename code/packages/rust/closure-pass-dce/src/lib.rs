//! Dead-code elimination pass for the Closure Compiler clone.
//!
//! Per [CLOC06](../../../specs/CLOC06-pass-interface-contract.md).
//! Final step in the autonomous-chain real-body rollout (after
//! constant-fold, fold-control-flow, and the closure-emitter).
//!
//! # What this pass does
//!
//! Two cleanup categories:
//!
//! 1. **Dead-after-terminator**: in any `BlockStatement.body`, drop
//!    everything after a `ReturnStatement` or a `ThrowStatement` — both
//!    unconditionally end the statement list's execution, in every block
//!    context, so what follows is unreachable. `BreakStatement` and
//!    `ContinueStatement` only qualify relative to their enclosing loop/switch
//!    scope, so they are handled only in switch-case consequents
//!    (`is_case_terminator`), not at the general block level (Phase 2 work).
//! 2. **Empty-statement removal**: drop `EmptyStatement` nodes
//!    from `BlockStatement.body`. They're semantically a no-op,
//!    just `;` noise.
//!
//! Recurses through every Phase 1 node so nested blocks (function
//! bodies, if-bodies, while-bodies, for-bodies) get cleaned too.
//!
//! # Why this overlaps with fold-control-flow's dead-after-return
//!
//! `closure-pass-fold-control-flow` also drops code after
//! `ReturnStatement` in blocks. The overlap is intentional:
//!
//! - **fold-control-flow** does the cleanup as part of its block
//!   rewrite when it observes the terminator while folding.
//! - **DCE** runs *after* fold-control-flow per CLOC06 canonical
//!   order, and catches anything fold-control-flow missed —
//!   especially blocks that *became* dead-after-terminator only
//!   after an earlier pass rewrote them (e.g. constant-fold
//!   collapsed `1 < 2 → true`, then fold-control-flow turned
//!   `if (true) {return x;} else {y;}` into a block with the
//!   return and the leftover `y;` from the alternate slot —
//!   fold-control-flow caught that case too, but DCE provides a
//!   safety net).
//!
//! DCE's responsibility is also to clean `EmptyStatement` noise
//! that fold-control-flow *produces* — when fold-control-flow
//! collapses `if (false) {…}` (no alternate) it leaves an
//! `EmptyStatement` behind. DCE removes those.
//!
//! # What this pass *doesn't* do (yet)
//!
//! - Unreferenced `VariableDeclaration` removal — that's
//!   `closure-pass-remove-unused-vars`'s job per CLOC06.
//! - Unreachable code inside `if` branches — that's
//!   fold-control-flow's job (when the branch is known dead).
//! - Empty `BlockStatement` collapse to `EmptyStatement` —
//!   tracked for Phase 1.x (preserves debugging-step shape for
//!   now).
//!
//! # CV tracing — both modes work per CLOC09
//!
//! - **Traced** (`cv: Some` on the block): a `Contribution` is
//!   appended to the block's CV per drop category, with `source =
//!   "dce"`, `tag = "removed-dead-code"` or
//!   `"removed-empty-statement"`, and `meta` describing how many
//!   nodes were dropped.
//! - **Untraced** (`cv: None`): drops silently, no
//!   `Contribution`s emitted. `changed: true` still set so the
//!   pipeline knows.

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use coding_adventures_correlation_vector::{CVLog, Contribution};
use coding_adventures_javascript_ast::{
    statement::TaggedStatement, ArrayExpression, AssignmentExpression, BigIntLiteral,
    BinaryExpression, BlockStatement, BooleanLiteral, CallExpression, ConditionalExpression, NewExpression, SequenceExpression, SpreadElement, YieldExpression, AwaitExpression, ImportExpression,
    Declaration, EmptyStatement, Expression, ExpressionStatement, ForInStatement, ForInit,
    ForOfStatement,
    ForStatement,
    ArrowBody, ArrowFunctionExpression, TaggedTemplateExpression, TemplateLiteral,
    ClassDeclaration, ClassExpression, ClassMember, MethodDefinition, PropertyDefinition,
    ChainExpression, FunctionDeclaration, FunctionExpression, IfStatement, LogicalExpression, MemberExpression, NullLiteral, OptionalCallExpression, OptionalMemberExpression,
    NumericLiteral, ObjectExpression, ObjectMember, Program, ProgramItem, Property, PropertyKey,
    ReturnStatement, Statement, StringLiteral, UnaryExpression, UnaryOperator, UndefinedLiteral, UpdateExpression, VarKind,
    DoWhileStatement, VariableDeclaration, VariableDeclarator, WhileStatement, WithStatement,
};
use serde_json::json;
use std::collections::HashMap;

/// `Pass::depends_on` value. Per CLOC06 canonical order:
/// `constant-fold → fold-control-flow → dce → ...`. We declare
/// only `constant-fold` here in v0.1.0; once
/// `fold-control-flow` is fully stable that joins too. The
/// scaffolding spec noted this; the real-body version keeps the
/// same edges so the scheduler doesn't churn.
const DEPS: &[&str] = &["constant-fold"];

#[derive(Debug, Default, Clone, Copy)]
pub struct DcePass;

impl DcePass {
    pub fn new() -> Self {
        Self
    }
}

impl Pass for DcePass {
    fn name(&self) -> &'static str {
        "dce"
    }

    fn depends_on(&self) -> &[&'static str] {
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // Per CLOC06: deletion can free further nodes. v1 still
        // runs the pipeline once (FixedPoint iteration support is
        // Phase 1.x in closure-pass-pipeline), but the policy
        // captures intent and a single bottom-up walk handles
        // the common cases anyway.
        IterationPolicy::FixedPoint
    }

    fn cost(&self) -> u32 {
        // Single tree walk + bounded work per block. Matches the
        // v0.1.0 scaffolding cost.
        3
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        let mut state = DceState {
            cv: ctx.cv,
            contributions: Vec::new(),
            changed: false,
            nodes_touched: 0,
        };
        let new_program = dce_program(ctx.program, &mut state);
        Ok(PassOutput {
            program: new_program,
            contributions: state.contributions,
            changed: state.changed,
            diagnostics: Vec::new(),
            stats: PassStats {
                nodes_touched: state.nodes_touched,
            },
        })
    }
}

// =====================================================================
// DceState — mirrors FoldState shape from constant-fold /
// fold-control-flow so the recursion pattern is the same.
// =====================================================================

struct DceState<'a> {
    cv: &'a mut CVLog,
    contributions: Vec<Contribution>,
    changed: bool,
    nodes_touched: u32,
}

impl DceState<'_> {
    fn record(&mut self, parent: &Option<String>, tag: &str, before: &str, after: &str) {
        self.changed = true;
        if let Some(parent_cv) = parent {
            let contribution = Contribution {
                source: "dce".to_string(),
                tag: tag.to_string(),
                meta: [
                    ("before".to_string(), json!(before)),
                    ("after".to_string(), json!(after)),
                    ("parent_cv".to_string(), json!(parent_cv)),
                ]
                .into_iter()
                .collect(),
            };
            self.contributions.push(contribution);
        }
    }

    /// Record a genuine *deletion* of one or more nodes.
    ///
    /// [`record`](Self::record) logs a summary [`Contribution`]
    /// against the *container* — "dce dropped N statements here." But
    /// the container is not what disappeared: the individual removed
    /// nodes are. This method additionally *tombstones* each removed
    /// node's own CV entry with a `DeletionRecord` (via
    /// [`CVLog::delete`]), so a `--correlation_vector` consumer that
    /// later asks "what happened to the span at 42:3-42:19?" gets a
    /// definite answer — *dce removed it, because `<reason>`* — instead
    /// of the span silently vanishing from the provenance graph. A
    /// minifier that drops code without a trace is unauditable; one
    /// that tombstones every removal can always be asked to justify
    /// itself. That audit trail is the entire reason the correlation
    /// vector exists.
    ///
    /// When the log is disabled (production default) `delete` is a
    /// no-op, so this costs nothing on the hot path — the tombstones
    /// only materialise under `--correlation_vector`.
    ///
    /// Note what does NOT call this: `block-flattened` *moves* a nested
    /// block's statements up one scope level, it does not delete them,
    /// so those nodes must stay live in the CV log and keep only their
    /// summary contribution.
    fn record_deletion(
        &mut self,
        removed: &[Option<String>],
        container: &Option<String>,
        reason: &str,
        before: &str,
        after: &str,
    ) {
        // Tombstone each removed node individually. `flatten()` skips
        // the `None`s (untraced nodes — nothing to attribute to).
        for cv_id in removed.iter().flatten() {
            let mut meta: HashMap<String, serde_json::Value> = HashMap::new();
            if let Some(container_cv) = container {
                meta.insert("container_cv".to_string(), json!(container_cv));
            }
            self.cv.delete(cv_id, "dce", reason, meta);
        }
        // Keep the container-level summary contribution so existing
        // history/stats/tests that look for this tag still observe the
        // removal at the enclosing node.
        self.record(container, reason, before, after);
    }

    fn visit(&mut self) {
        self.nodes_touched += 1;
    }
}

/// Is `expr` a leaf literal we can guarantee has no observable side
/// effects when evaluated?
///
/// Conservative: only the seven primitive-literal node types qualify.
/// Identifier reads can throw under TDZ (for unitialized `let` /
/// `const`), and we don't do scope analysis here. Member / call /
/// binary / unary / etc. can all have effects. So we bail.
///
/// Used by gap-014 step 2's empty-switch elimination: when the
/// switch has no observable side effects (pure discriminant +
/// pure tests + empty consequents), the entire switch can drop to
/// `;`. Anything non-literal: leave alone.
fn is_pure_leaf(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::NumericLiteral(NumericLiteral { .. })
            | Expression::StringLiteral(StringLiteral { .. })
            | Expression::BooleanLiteral(BooleanLiteral { .. })
            | Expression::NullLiteral(NullLiteral { .. })
            | Expression::UndefinedLiteral(UndefinedLiteral { .. })
            | Expression::BigIntLiteral(BigIntLiteral { .. })
    )
}

/// Is evaluating `expr` free of observable side effects? Used to decide whether
/// a statement whose only job is to evaluate `expr` (e.g. the test of an
/// otherwise-empty `if`) can be dropped entirely.
///
/// This is broader than [`is_pure_leaf`] (which is literals only): it mirrors
/// the reference Closure Compiler's notion, under which reading a binding or a
/// property, and combining pure sub-expressions with the pure operators, has no
/// side effect. So `if (x) {}`, `if (x.y) {}`, `if (x[k]) {}`, `if (a && b) {}`,
/// `if (typeof x) {}`, `if (!x) {}` all drop.
///
/// SAFE-BY-CONSTRUCTION: anything not positively known pure returns `false`
/// (never removed). The impure set — calls, `new`, assignment, `++`/`--`,
/// `delete`, `yield`, `await`, tagged templates, dynamic `import()`, and
/// (conservatively) the comma operator — is therefore handled by the catch-all.
///
/// - **Member access** is pure only when its object *and* (for a computed key)
///   its property are pure — `f().y` is NOT pure (the call runs).
/// - **`delete`** is excluded from the pure unary operators: it mutates.
/// - The comma operator (`SequenceExpression`) returns `false` even when both
///   operands are pure, because the reference compiler rewrites
///   `if (a, b) {}` to `a;` (a different transform), so declining here keeps us
///   byte-identical rather than removing it outright.
fn is_side_effect_free(expr: &Expression) -> bool {
    match expr {
        // Leaf values and bindings: no side effect to read.
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::UndefinedLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::Identifier(_)
        | Expression::ThisExpression(_) => true,
        // Property read: pure iff the object (and computed key) are pure.
        Expression::MemberExpression(m) => {
            is_side_effect_free(&m.object)
                && (!m.computed || is_side_effect_free(&m.property))
        }
        // Pure prefix operators over a pure operand. `delete` is NOT here — it
        // removes a property (a side effect).
        Expression::UnaryExpression(u) => {
            u.operator != UnaryOperator::Delete && is_side_effect_free(&u.argument)
        }
        Expression::BinaryExpression(b) => {
            is_side_effect_free(&b.left) && is_side_effect_free(&b.right)
        }
        Expression::LogicalExpression(l) => {
            is_side_effect_free(&l.left) && is_side_effect_free(&l.right)
        }
        Expression::ConditionalExpression(c) => {
            is_side_effect_free(&c.test)
                && is_side_effect_free(&c.consequent)
                && is_side_effect_free(&c.alternate)
        }
        // Anything else (Call / New / Assignment / Update / Sequence / Yield /
        // Await / TaggedTemplate / ImportExpression / …) is treated as possibly
        // side-effecting and is NOT removed.
        _ => false,
    }
}

/// Does this statement do nothing when executed — an `EmptyStatement` (`;`) or
/// an empty `BlockStatement` (`{}`)? Used to test whether an `if`'s branch is
/// empty. (An empty block IS observably a no-op: it declares no bindings.)
fn statement_is_empty(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Tagged(TaggedStatement::EmptyStatement(_))
    ) || matches!(
        stmt,
        Statement::Tagged(TaggedStatement::BlockStatement(b)) if b.body.is_empty()
    )
}

// =====================================================================
// Program / top-level
// =====================================================================

fn dce_program(prog: &Program, st: &mut DceState) -> Program {
    st.visit();
    let mut new_body: Vec<ProgramItem> = prog
        .body
        .iter()
        .map(|item| dce_program_item(item, st))
        .collect();

    // Strip top-level `debugger;` statements (CLOC24).
    //
    // The program body is a list of `ProgramItem`s, not a `BlockStatement`,
    // so it needs its own sweep separate from `dce_block_statement`'s. Same
    // rationale: a `debugger` statement is a development-only breakpoint with
    // no effect on a shipped program, so at SIMPLE/ADVANCED we remove it — and
    // because this pass never runs at WHITESPACE_ONLY, `debugger` survives
    // there, matching upstream Closure exactly. See `is_debugger_statement`.
    let before_debugger_drop = new_body.len();
    // Capture the removed items' CV ids *before* `retain` drops them,
    // so `record_deletion` can tombstone each vanished span.
    let removed_debugger_cvs: Vec<Option<String>> = new_body
        .iter()
        .filter(|item| is_debugger_program_item(item))
        .map(program_item_cv)
        .collect();
    new_body.retain(|item| !is_debugger_program_item(item));
    let dropped_debuggers = before_debugger_drop - new_body.len();
    if dropped_debuggers > 0 {
        st.record_deletion(
            &removed_debugger_cvs,
            &prog.cv,
            "removed-debugger",
            &format!("program with {} top-level items", before_debugger_drop),
            &format!("dropped {} top-level debugger statements", dropped_debuggers),
        );
    }

    // Strip stray top-level `EmptyStatement`s (`;`) — CLOC12.195.
    //
    // Like `debugger`, an empty statement at statement-list position is a pure
    // no-op, so the program body needs its own sweep separate from
    // `dce_block_statement`'s (which already does this for block bodies). These
    // arise from a hand-written `;`, from `constant-fold`/`fold-control-flow`
    // folding `if (false) …;` / `while (false) …;` to an `EmptyStatement`, and
    // from the trailing `;` a flattened block leaves behind
    // (`g(0);{g(1)};g(2)` → `g(0);g(1);;g(2)` → `g(0);g(1);g(2)`). Upstream
    // Closure removes them all; matching it here removes the last stray `;`.
    // A `for (…) ;` / `if (c) ;` empty *substatement* is a loop/if body, NOT a
    // statement-list member, so it never reaches this sweep — it stays intact,
    // exactly as Closure keeps it.
    let before_empty_drop = new_body.len();
    let removed_empty_cvs: Vec<Option<String>> = new_body
        .iter()
        .filter(|item| is_empty_program_item(item))
        .map(program_item_cv)
        .collect();
    new_body.retain(|item| !is_empty_program_item(item));
    let dropped_empties = before_empty_drop - new_body.len();
    if dropped_empties > 0 {
        st.record_deletion(
            &removed_empty_cvs,
            &prog.cv,
            "removed-empty-statement",
            &format!("program with {} top-level items", before_empty_drop),
            &format!("dropped {} top-level empty statements", dropped_empties),
        );
    }

    Program {
        cv: prog.cv.clone(),
        version: prog.version,
        source_type: prog.source_type,
        body: new_body,
    }
}

fn dce_program_item(item: &ProgramItem, st: &mut DceState) -> ProgramItem {
    match item {
        ProgramItem::Statement(s) => ProgramItem::Statement(dce_statement(s, st)),
        ProgramItem::Declaration(d) => ProgramItem::Declaration(dce_declaration(d, st)),
    }
}

// =====================================================================
// Statements
// =====================================================================

fn dce_statement(stmt: &Statement, st: &mut DceState) -> Statement {
    st.visit();
    match stmt {
        Statement::Tagged(t) => Statement::Tagged(dce_tagged_statement(t, st)),
        Statement::Declaration(d) => Statement::Declaration(dce_declaration(d, st)),
    }
}

fn dce_tagged_statement(stmt: &TaggedStatement, st: &mut DceState) -> TaggedStatement {
    match stmt {
        TaggedStatement::ExpressionStatement(s) => {
            TaggedStatement::ExpressionStatement(ExpressionStatement {
                cv: s.cv.clone(),
                expression: dce_expression(&s.expression, st),
            })
        }
        TaggedStatement::BlockStatement(s) => {
            TaggedStatement::BlockStatement(dce_block_statement(s, st))
        }
        TaggedStatement::IfStatement(s) => {
            let test = dce_expression(&s.test, st);
            let consequent = Box::new(dce_statement(&s.consequent, st));
            let alternate = s.alternate.as_ref().map(|a| Box::new(dce_statement(a, st)));

            // Empty-`if` elimination. When both branches do nothing (consequent
            // empty AND alternate absent-or-empty) the whole statement's only
            // remaining effect is evaluating `test`; if `test` is side-effect-
            // free the entire `if` is dead and collapses to `;` (which the
            // block/program sweep then drops). `if(x){}`, `if(x.y){}else{}`, … →
            // removed.
            let cons_empty = statement_is_empty(&consequent);
            let alt_empty = alternate.as_ref().is_none_or(|a| statement_is_empty(a));
            if cons_empty && alt_empty && is_side_effect_free(&test) {
                st.record(
                    &s.cv,
                    "if_eliminated",
                    "IfStatement{ <empty>, test side-effect-free }",
                    "EmptyStatement",
                );
                return TaggedStatement::EmptyStatement(EmptyStatement { cv: s.cv.clone() });
            }

            // A side-effecting test with both branches empty still has to RUN
            // the test, so the `if` wrapper is dead but the test survives as an
            // expression statement: `if(f()){}` → `f();`, `if(a.b()){}` →
            // `a.b();`, `if(f(1,2)){}else{}` → `f(1,2);`. This is the impure
            // twin of the `is_side_effect_free` removal above (the two guards
            // are mutually exclusive — a call is never side-effect-free).
            //
            // Scoped to a plain `CallExpression`: as an expression statement a
            // bare call is already Closure's *final* form, so the rewrite is
            // byte-identical. Other impure tests get FURTHER simplifications
            // that are separate transforms, so we decline them here rather than
            // emit a non-canonical intermediate:
            //   - `!f()`   → Closure drops the discarded `!`   → `f();`
            //   - `a = b`  → dead-assignment removal may delete it entirely
            //   - `a, f()` → the sequence is split into statements `a;f();`
            //   - `new F()`→ emitted `new F` (no parens) in statement position
            // Non-empty branches are a different rewrite again (`if(f()){g()}`
            // → `f()&&g()`, the if→logical arc), so this only fires when BOTH
            // branches are empty.
            if cons_empty && alt_empty && matches!(test, Expression::CallExpression(_)) {
                st.record(
                    &s.cv,
                    "if_to_expression_statement",
                    "IfStatement{ <empty>, test = CallExpression }",
                    "ExpressionStatement",
                );
                return TaggedStatement::ExpressionStatement(ExpressionStatement {
                    cv: s.cv.clone(),
                    expression: test,
                });
            }

            TaggedStatement::IfStatement(IfStatement {
                cv: s.cv.clone(),
                test,
                consequent,
                alternate,
            })
        }
        TaggedStatement::WhileStatement(s) => TaggedStatement::WhileStatement(WhileStatement {
            cv: s.cv.clone(),
            test: dce_expression(&s.test, st),
            body: Box::new(dce_statement(&s.body, st)),
        }),
        // `with (object) body` (CLOC12.187) — DCE the object and body like
        // `while`. Not yet reachable (the bridge still declines `with`).
        TaggedStatement::WithStatement(s) => TaggedStatement::WithStatement(WithStatement {
            cv: s.cv.clone(),
            object: dce_expression(&s.object, st),
            body: Box::new(dce_statement(&s.body, st)),
        }),
        // Recurse DCE into the do-while body and test. Like `while`, a
        // `do`-`while` is NOT a terminator (control can exit the loop), so
        // code after it stays reachable — we do not add it to the
        // dead-after-terminator set.
        TaggedStatement::DoWhileStatement(s) => {
            TaggedStatement::DoWhileStatement(DoWhileStatement {
                cv: s.cv.clone(),
                body: Box::new(dce_statement(&s.body, st)),
                test: dce_expression(&s.test, st),
            })
        }
        TaggedStatement::ForStatement(s) => TaggedStatement::ForStatement(ForStatement {
            cv: s.cv.clone(),
            init: s.init.as_ref().map(|i| match i {
                ForInit::VariableDeclaration(v) => {
                    ForInit::VariableDeclaration(dce_variable_declaration(v, st))
                }
                ForInit::Expression(e) => ForInit::Expression(dce_expression(e, st)),
            }),
            test: s.test.as_ref().map(|e| dce_expression(e, st)),
            update: s.update.as_ref().map(|e| dce_expression(e, st)),
            body: Box::new(dce_statement(&s.body, st)),
        }),
        // Recurse DCE into the for-in left, right, and body. Like the other
        // loops, a for-in is NOT a terminator (the body may run zero times),
        // so code after it stays reachable.
        TaggedStatement::ForInStatement(s) => TaggedStatement::ForInStatement(ForInStatement {
            cv: s.cv.clone(),
            left: match &s.left {
                ForInit::VariableDeclaration(v) => {
                    ForInit::VariableDeclaration(dce_variable_declaration(v, st))
                }
                ForInit::Expression(e) => ForInit::Expression(dce_expression(e, st)),
            },
            right: dce_expression(&s.right, st),
            body: Box::new(dce_statement(&s.body, st)),
        }),
        TaggedStatement::ForOfStatement(s) => TaggedStatement::ForOfStatement(ForOfStatement {
            cv: s.cv.clone(),
            left: match &s.left {
                ForInit::VariableDeclaration(v) => {
                    ForInit::VariableDeclaration(dce_variable_declaration(v, st))
                }
                ForInit::Expression(e) => ForInit::Expression(dce_expression(e, st)),
            },
            right: dce_expression(&s.right, st),
            body: Box::new(dce_statement(&s.body, st)),
        }),
        TaggedStatement::ReturnStatement(s) => {
            TaggedStatement::ReturnStatement(ReturnStatement {
                cv: s.cv.clone(),
                argument: s.argument.as_ref().map(|e| dce_expression(e, st)),
            })
        }
        TaggedStatement::LabeledStatement(s) => {
            // DCE recurses into the body so dead-after-return inside
            // `a: { ... return; ...dead... }` still gets stripped.
            // The label itself is preserved verbatim — collapsing
            // `a: break a;` to empty is a separate optimisation
            // tracked under the gap-009 follow-up.
            TaggedStatement::LabeledStatement(
                coding_adventures_javascript_ast::LabeledStatement {
                    cv: s.cv.clone(),
                    label: s.label.clone(),
                    body: Box::new(dce_statement(&s.body, st)),
                },
            )
        }
        TaggedStatement::ThrowStatement(s) => {
            // Recurse into the argument so any DCE work inside the
            // expression (e.g. nested ConditionalExpression cleanup,
            // once those become DCE-tracked) lands here. The throw
            // is itself a definite terminator — the block-walker
            // (`dce_block_statement`) treats it the same as
            // `ReturnStatement` for dead-after-terminator purposes
            // and that wiring lives there. This arm just preserves
            // the node.
            TaggedStatement::ThrowStatement(coding_adventures_javascript_ast::ThrowStatement {
                cv: s.cv.clone(),
                argument: dce_expression(&s.argument, st),
            })
        }
        TaggedStatement::SwitchStatement(s) => {
            // Recurse into discriminant, each case's test, and each
            // statement in each consequent first — peephole rules
            // run on the folded shape so we catch switches whose
            // bodies *became* empty after constant-fold +
            // fold-control-flow ran earlier in the pipeline.
            let new_disc = dce_expression(&s.discriminant, st);
            let mut new_cases: Vec<_> = s
                .cases
                .iter()
                .map(|c| coding_adventures_javascript_ast::SwitchCase {
                    cv: c.cv.clone(),
                    test: c.test.as_ref().map(|e| dce_expression(e, st)),
                    consequent: c
                        .consequent
                        .iter()
                        .map(|s| dce_statement(s, st))
                        .collect(),
                })
                .collect();

            // gap-014 step 3 / CLOC12.35 — drop-after-break inside
            // case consequents.
            //
            // Inside a switch case, `break;` exits the switch
            // entirely; `return;` / `throw e;` exit the enclosing
            // function. Everything after such a terminator in the
            // SAME case's consequent is unreachable.
            //
            // We do NOT generalise into `is_terminator` (which
            // handles block-level `ReturnStatement`-only dropping)
            // because `BreakStatement` at function-body block
            // level is a SyntaxError — broadening the block walker
            // would mishandle that. Case consequents are a special
            // context where Break is legal and terminating.
            //
            // `Continue` is intentionally NOT a terminator here:
            // it could refer to an enclosing loop, in which case
            // it's a real cross-scope control-flow jump we don't
            // model statically. Conservative bail.
            for case in new_cases.iter_mut() {
                if let Some(term_idx) = case
                    .consequent
                    .iter()
                    .position(is_case_terminator)
                {
                    let original_len = case.consequent.len();
                    let dropped = original_len - (term_idx + 1);
                    if dropped > 0 {
                        let removed_cvs: Vec<Option<String>> = case.consequent[term_idx + 1..]
                            .iter()
                            .map(statement_cv)
                            .collect();
                        case.consequent.truncate(term_idx + 1);
                        st.record_deletion(
                            &removed_cvs,
                            &case.cv,
                            "removed-dead-code-in-case",
                            &format!("case with {} statements", original_len),
                            &format!(
                                "dropped {} statements after terminator at index {}",
                                dropped, term_idx
                            ),
                        );
                    }
                }
            }

            // gap-014 step 2 / CLOC12.34 — empty-switch elimination.
            //
            // If every case's consequent is empty (or there are no
            // cases at all) AND both the discriminant and every
            // case-test are leaf literals (no side-effect risk),
            // collapse the whole switch to `;`. The block-walker
            // (`dce_block_statement`) will drop the EmptyStatement
            // in its next sweep.
            //
            // Conservative bail: anything else (Identifier
            // discriminant, computed test, non-empty consequent)
            // keeps the switch intact. The "drop after pure
            // discriminant with side-effecting tests" rule is a
            // future slice that needs a proper effect analysis.
            let all_consequents_empty = new_cases.iter().all(|c| c.consequent.is_empty());
            let discriminant_pure = is_pure_leaf(&new_disc);
            let all_tests_pure_or_none = new_cases
                .iter()
                .all(|c| c.test.as_ref().is_none_or(is_pure_leaf));
            if all_consequents_empty && discriminant_pure && all_tests_pure_or_none {
                st.record(
                    &s.cv,
                    "switch_eliminated",
                    "SwitchStatement{<empty body>}",
                    "EmptyStatement",
                );
                return TaggedStatement::EmptyStatement(EmptyStatement { cv: s.cv.clone() });
            }

            // gap-014 step 4 / CLOC12.36 — constant-discriminant
            // collapse.
            //
            // When the discriminant is a pure leaf literal and every
            // case test is None or also a pure leaf literal, we can
            // compile-time evaluate which case runs:
            //
            // 1. Find the first case whose `test` is strict-equal
            //    to the discriminant.
            // 2. If no match, fall back to the `default:` case if
            //    one exists.
            // 3. Replace the entire switch with a `BlockStatement`
            //    holding the matched case's consequent — with any
            //    trailing `break;` stripped (it's spurious now
            //    that there's no switch to exit). `return;` /
            //    `throw e;` stay; they're terminators with their
            //    own semantics.
            // 4. No match AND no default → switch executes nothing;
            //    replace with `EmptyStatement`.
            //
            // Conservative bail (keep switch unchanged):
            // - Matched case's consequent doesn't end with a
            //   case-terminator (`break;` / `return ...;` /
            //   `throw ...;`). Without a terminator, control
            //   *would* fall through to the next case, and we
            //   don't model fall-through here. A future slice can
            //   concatenate consequents up to the next terminator.
            // - Discriminant is a `NumericLiteral` with value
            //   `NaN`. Per spec, `NaN !== NaN`, so NaN never
            //   matches anything — but rather than emit subtle
            //   no-match semantics on a literal that's already
            //   surprising, we bail. (Constant-fold normally
            //   produces this via `0/0`-style folds.)
            if discriminant_pure && all_tests_pure_or_none {
                if let Some(target) = pick_matching_case(&new_disc, &new_cases) {
                    let last = target.consequent.last();
                    let terminates = last.is_some_and(is_case_terminator);
                    // Empty consequent → fall-through to next case per
                    // ECMAScript §13.12. The classic "share body"
                    // pattern `case 1: case 2: body; break;` has
                    // `case 1: []` as the matched case and would
                    // wrongly drop `body` if collapsed to `{}`. We
                    // don't model fall-through across cases here,
                    // so bail. A future slice can concatenate
                    // consequents through the next terminator.
                    if terminates && !target.consequent.is_empty() {
                        let new_body = strip_trailing_break(&target.consequent);
                        st.record(
                            &s.cv,
                            "switch_collapsed_to_matched_case",
                            "SwitchStatement{<pure-discriminant>}",
                            &format!(
                                "BlockStatement with {} statements (from matched case)",
                                new_body.len()
                            ),
                        );
                        return TaggedStatement::BlockStatement(BlockStatement {
                            cv: s.cv.clone(),
                            body: new_body,
                        });
                    }
                } else {
                    // No matching case and no default → nothing
                    // runs. Discriminant is pure so safe to drop.
                    st.record(
                        &s.cv,
                        "switch_collapsed_no_match",
                        "SwitchStatement{<pure-discriminant>}",
                        "EmptyStatement",
                    );
                    return TaggedStatement::EmptyStatement(EmptyStatement { cv: s.cv.clone() });
                }
            }

            TaggedStatement::SwitchStatement(coding_adventures_javascript_ast::SwitchStatement {
                cv: s.cv.clone(),
                discriminant: new_disc,
                cases: new_cases,
            })
        }
        TaggedStatement::TryStatement(s) => {
            // Recurse DCE into the protected block, the catch body, and the
            // finalizer — each is an ordinary block, so dead-after-terminator
            // and empty-statement cleanup apply within them. The catch `param`
            // is preserved verbatim (it is a binding, not removable here). We
            // do NOT treat the `try` itself as a terminator: it can catch and
            // continue, so code after it is reachable.
            TaggedStatement::TryStatement(coding_adventures_javascript_ast::TryStatement {
                cv: s.cv.clone(),
                block: dce_block_statement(&s.block, st),
                handler: s.handler.as_ref().map(|h| {
                    coding_adventures_javascript_ast::CatchClause {
                        cv: h.cv.clone(),
                        param: h.param.clone(),
                        body: dce_block_statement(&h.body, st),
                    }
                }),
                finalizer: s.finalizer.as_ref().map(|f| dce_block_statement(f, st)),
            })
        }
        TaggedStatement::BreakStatement(_)
        | TaggedStatement::ContinueStatement(_)
        | TaggedStatement::EmptyStatement(_)
        // A `debugger;` reaching HERE is a non-list child (e.g. a brace-less
        // `if (c) debugger;` consequent), so it is preserved as-is. The
        // CLOC24 strip operates on statement *lists* — the block-body sweep in
        // `dce_block_statement` and the top-level sweep in `dce_program` —
        // where a `debugger` can be removed without leaving a dangling
        // single-statement slot. (A bare consequent could be stripped too, but
        // that is a rarer shape left for future work, consistent with how the
        // empty-statement sweep is also list-scoped.)
        | TaggedStatement::DebuggerStatement(_) => stmt.clone(),
    }
}

/// The heart of the pass: process `BlockStatement.body` in three
/// passes — recurse into each child, drop dead-after-return,
/// drop empty statements. Records one Contribution per category
/// per block where drops happened (not one per dropped statement
/// — that'd be noisy).
fn dce_block_statement(b: &BlockStatement, st: &mut DceState) -> BlockStatement {
    // First, recurse into children (so nested blocks fold first).
    // While we're here, count empty statements so we can drop
    // them in a single sweep below.
    let mut working: Vec<Statement> = b
        .body
        .iter()
        .map(|s| dce_statement(s, st))
        .collect();

    // Block flattening (closes CLOC12 gap-010).
    //
    // Splice any direct-child `BlockStatement`'s body into our own
    // body. After the recurse-into-children step above, the inner
    // block has already had its own DCE applied, so what we splice
    // in is the already-cleaned version. Truth table:
    //
    //   {{foo();}}            → {foo();}           (1-stmt inner)
    //   {foo();{}}            → {foo();}           (empty inner)
    //   {{};foo();}           → {foo();}           (empty inner first)
    //   {{a();b();};}         → {a();b();}         (multi-stmt inner)
    //   {foo();{bar();baz();}} → {foo();bar();baz();}
    //   {let x=1;{let x=2;}}  → unchanged          (scope-safety)
    //
    // Scope safety: ECMAScript block scope means `let`, `const`,
    // `class`, and inner `function` declarations are bound to their
    // enclosing block. Hoisting their bodies into our scope would
    // either leak a binding upward (changing semantics) or trigger
    // a redeclaration TDZ error if a same-named binding already
    // exists. So we ONLY flatten an inner block when its body
    // contains no scope-bound declarations. Plain `var` is fine
    // — it's function-scoped, so hoisting it out of an inner
    // block doesn't change the binding's containing scope.
    //
    // Why DCE owns this fold: block flattening is a pure structural
    // simplification with no semantic prerequisites — it can run
    // safely without scope/symbol info beyond the per-block scan
    // we already do here, and it makes downstream emitter output
    // tighter. Per gap-010 it lives here for v1.
    let pre_flatten_len = working.len();
    let mut flattened: Vec<Statement> = Vec::with_capacity(pre_flatten_len);
    let mut flattening_happened = false;
    for stmt in working.drain(..) {
        match &stmt {
            Statement::Tagged(TaggedStatement::BlockStatement(inner))
                if block_is_scope_safe_to_flatten(inner) =>
            {
                flattening_happened = true;
                for inner_stmt in inner.body.iter() {
                    flattened.push(inner_stmt.clone());
                }
            }
            _ => flattened.push(stmt),
        }
    }
    working = flattened;
    if flattening_happened {
        st.record(
            &b.cv,
            "block-flattened",
            &format!("block with {} statements", pre_flatten_len),
            &format!("flattened to {} statements", working.len()),
        );
    }

    // Drop dead-after-terminator.
    //
    // SOUNDNESS — hoisting: `var` and `function` declarations hoist to the top
    // of the enclosing function regardless of their textual position, so one in
    // the otherwise-unreachable tail after a terminator still creates a binding
    // that code BEFORE the terminator (or in a sibling scope) can observe.
    // Example: `function f(){ h(); throw e; function h(){} }` — `h` is callable
    // at `h()` because the declaration hoists past the `throw`; dropping
    // `function h(){}` would make that a ReferenceError. We therefore truncate
    // ONLY when every statement in the tail is provably free of a hoisted
    // binding (`tail_is_safe_to_truncate` — a whitelist that excludes `var`,
    // `function`, and every compound statement that could wrap a `var`).
    // Declining to drop dead statements is never a miscompile; a genuinely
    // unused hoisted declaration is still removed downstream by
    // `remove-unused-vars`.
    let original_len = working.len();
    if let Some(terminator_idx) = working.iter().position(is_terminator) {
        let dropped = original_len - (terminator_idx + 1);
        if dropped > 0 && tail_is_safe_to_truncate(&working[terminator_idx + 1..]) {
            let removed_cvs: Vec<Option<String>> = working[terminator_idx + 1..]
                .iter()
                .map(statement_cv)
                .collect();
            working.truncate(terminator_idx + 1);
            st.record_deletion(
                &removed_cvs,
                &b.cv,
                "removed-dead-code",
                &format!("block with {} statements", original_len),
                &format!(
                    "dropped {} statements after terminator at index {}",
                    dropped, terminator_idx
                ),
            );
        }
    }

    // Drop EmptyStatements.
    let before_empty_drop = working.len();
    let removed_empty_cvs: Vec<Option<String>> = working
        .iter()
        .filter(|s| is_empty_statement(s))
        .map(statement_cv)
        .collect();
    working.retain(|s| !is_empty_statement(s));
    let dropped_empties = before_empty_drop - working.len();
    if dropped_empties > 0 {
        st.record_deletion(
            &removed_empty_cvs,
            &b.cv,
            "removed-empty-statement",
            &format!("block with {} statements", before_empty_drop),
            &format!("dropped {} empty statements", dropped_empties),
        );
    }

    // Strip `debugger;` statements (CLOC24).
    //
    // A `debugger` statement is a development-only breakpoint: it pauses
    // execution ONLY when a debugger is attached and is a no-op otherwise, so
    // removing it from a shipped program preserves the program's observable
    // behaviour. At SIMPLE/ADVANCED upstream Closure strips it; we do the same
    // here (and only here — this pass never runs at WHITESPACE_ONLY, where the
    // statement is preserved). We sweep it from the statement list exactly like
    // empty statements; a `debugger` reaching a non-list position (e.g. a
    // brace-less `if (c) debugger;` consequent) is left intact — see
    // `dce_tagged_statement`'s leaf arm.
    let before_debugger_drop = working.len();
    let removed_debugger_cvs: Vec<Option<String>> = working
        .iter()
        .filter(|s| is_debugger_statement(s))
        .map(statement_cv)
        .collect();
    working.retain(|s| !is_debugger_statement(s));
    let dropped_debuggers = before_debugger_drop - working.len();
    if dropped_debuggers > 0 {
        st.record_deletion(
            &removed_debugger_cvs,
            &b.cv,
            "removed-debugger",
            &format!("block with {} statements", before_debugger_drop),
            &format!("dropped {} debugger statements", dropped_debuggers),
        );
    }

    BlockStatement {
        cv: b.cv.clone(),
        body: working,
    }
}

/// A statement that unconditionally ends the execution of its enclosing
/// statement list, making everything after it in the same `BlockStatement.body`
/// (or function/program body) unreachable:
///
/// - `return …;` exits the enclosing function.
/// - `throw …;` raises out of the function (propagating up through any
///   `try`/loop/block), so nothing after it in the same list runs — exactly
///   like `return` for dead-code purposes, in EVERY block context. (This is
///   the same reasoning `is_case_terminator` already applies to switch cases.)
///
/// `break` / `continue` are deliberately excluded here: they only terminate
/// flow relative to an *enclosing* loop/switch, so whether the statements after
/// them in an arbitrary block are unreachable depends on context this
/// block-level check doesn't have (and a bare `break`/`continue` in a
/// function-body block is a SyntaxError a faithful parser never produces). The
/// switch-case context handles `break` separately via `is_case_terminator`.
fn is_terminator(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Tagged(TaggedStatement::ReturnStatement(_))
            | Statement::Tagged(TaggedStatement::ThrowStatement(_))
    )
}

/// Is every statement in `stmts` (a dead-after-terminator tail) provably free
/// of a **hoisted binding**, so the whole tail can be dropped soundly?
///
/// `var` and `function` declarations hoist to the top of the enclosing
/// function regardless of textual position, so one in an otherwise-unreachable
/// tail still creates a binding observable *before* the terminator — dropping
/// it would miscompile. Crucially a `var` can hoist while wrapped in a compound
/// statement (`for (var i …)`, `if (c) var y;`, `while (c) var z;`, a nested
/// `{ var x }`, a `switch` case…), so it is NOT enough to look for top-level
/// `Declaration` nodes. We instead use a **whitelist**: a tail is safe to drop
/// only when every statement is one of the forms that can carry no hoisted
/// binding into this function scope —
///
/// - `ExpressionStatement` (a statement-position expression declares nothing;
///   a function/class *expression* owns its own scope),
/// - `EmptyStatement`, `break`, `continue`, `return`, `throw`,
/// - `let` / `const` declarations (block-scoped, TDZ — not hoisted; a
///   reference before the declaration throws either way).
///
/// Every other form — a `function` declaration, a `var` declaration, or ANY
/// compound statement (`if` / `while` / `for` / block / `switch` / labeled)
/// that might transitively contain a hoisted `var` — is treated as unsafe, so
/// the tail is preserved. Conservative: declining to drop dead statements is
/// never a miscompile, and any genuinely-unused hoisted declaration is still
/// removed downstream by `remove-unused-vars`. New statement variants default
/// to unsafe (preserved) until explicitly vetted.
fn tail_is_safe_to_truncate(stmts: &[Statement]) -> bool {
    stmts.iter().all(|s| match s {
        Statement::Tagged(t) => matches!(
            t,
            TaggedStatement::ExpressionStatement(_)
                | TaggedStatement::EmptyStatement(_)
                | TaggedStatement::BreakStatement(_)
                | TaggedStatement::ContinueStatement(_)
                | TaggedStatement::ReturnStatement(_)
                | TaggedStatement::ThrowStatement(_)
        ),
        // `let` / `const` are block-scoped (safe to drop); `var` hoists (unsafe).
        Statement::Declaration(Declaration::VariableDeclaration(vd)) => vd.kind != VarKind::Var,
        // A `function` declaration is itself a hoisted binding — unsafe.
        Statement::Declaration(Declaration::FunctionDeclaration(_)) => false,
        // A `class` declaration binds a name too — treated as unsafe like a
        // function declaration (conservative: preserving it is never a
        // miscompile, and a genuinely-unused one is removed downstream).
        Statement::Declaration(Declaration::ClassDeclaration(_)) => false,
        // An import declaration is module-top-level only; it never legally
        // appears inside a block, so flattening/truncating it away is unsafe.
        Statement::Declaration(Declaration::ImportDeclaration(_)) => false,
        Statement::Declaration(Declaration::ExportNamedDeclaration(_)) => false,
        Statement::Declaration(Declaration::ExportDefaultDeclaration(_)) => false,
        Statement::Declaration(Declaration::ExportAllDeclaration(_)) => false,
    })
}

/// Inside a switch case's consequent, these statements end the
/// case's execution and make everything that follows unreachable:
///
/// - `break;` exits the switch
/// - `return ...;` exits the enclosing function
/// - `throw ...;` raises out of the function (and switch)
///
/// `continue` is excluded because it refers to an *enclosing
/// loop* (`switch` is not a loop), not the switch itself. Whether
/// the surrounding loop's body continues or terminates depends on
/// outer-context analysis we don't do here, so we bail.
///
/// Used by gap-014 step 3 / CLOC12.35: per-case dead-after-break
/// dropping. Distinct from `is_terminator` (block-level
/// ReturnStatement only) because `BreakStatement` at function-body
/// block level would be a SyntaxError — broadening
/// `is_terminator` would mishandle that. Case consequents are the
/// one statement context where bare `break` is legal AND
/// terminates flow.
fn is_case_terminator(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Tagged(TaggedStatement::ReturnStatement(_))
            | Statement::Tagged(TaggedStatement::BreakStatement(_))
            | Statement::Tagged(TaggedStatement::ThrowStatement(_))
    )
}

/// Compile-time `===` between two `is_pure_leaf` expressions.
///
/// Implements just enough of ECMAScript §IsStrictlyEqual to handle
/// the literal types `is_pure_leaf` recognises:
///
/// - `NumericLiteral` vs `NumericLiteral`: floats equal AND neither
///   is `NaN`. (`NaN !== NaN` per spec — but constant-fold doesn't
///   normally produce a NaN-typed literal; bail conservatively
///   if it ever does.)
/// - `StringLiteral` vs `StringLiteral`: `value` field equal.
/// - `BooleanLiteral` vs `BooleanLiteral`: `value` field equal.
/// - `NullLiteral` vs `NullLiteral`: always true (one canonical
///   null).
/// - `UndefinedLiteral` vs `UndefinedLiteral`: always true.
/// - `BigIntLiteral` vs `BigIntLiteral`: `value` field equal
///   (decimal-string comparison).
/// - Anything else (cross-type, non-literal): false. Per spec
///   strict-equality is false across primitive types.
///
/// Used by gap-014 step 4 / CLOC12.36's constant-discriminant
/// collapse to pick the matching case at compile time.
fn strict_equal_leaves(a: &Expression, b: &Expression) -> bool {
    match (a, b) {
        (Expression::NumericLiteral(x), Expression::NumericLiteral(y)) => {
            !x.value.is_nan() && !y.value.is_nan() && x.value == y.value
        }
        (Expression::StringLiteral(x), Expression::StringLiteral(y)) => x.value == y.value,
        (Expression::BooleanLiteral(x), Expression::BooleanLiteral(y)) => x.value == y.value,
        (Expression::NullLiteral(_), Expression::NullLiteral(_)) => true,
        (Expression::UndefinedLiteral(_), Expression::UndefinedLiteral(_)) => true,
        (Expression::BigIntLiteral(x), Expression::BigIntLiteral(y)) => x.value == y.value,
        _ => false,
    }
}

/// Pick the case that runs at compile time given a known
/// `discriminant` and the switch's `cases`.
///
/// Walks cases in source order:
/// 1. Return the first case whose `test` is `Some(t)` and
///    `strict_equal_leaves(t, discriminant)`.
/// 2. If no case matches, return the first case with
///    `test: None` (the `default:` clause) if one exists.
/// 3. Return `None` when no case matches and there's no default
///    — the switch produces nothing observable.
///
/// Caller is responsible for pre-checking that `discriminant` and
/// every case's `test` is `is_pure_leaf`. Otherwise the strict-
/// equality result is unsound.
fn pick_matching_case<'a>(
    discriminant: &Expression,
    cases: &'a [coding_adventures_javascript_ast::SwitchCase],
) -> Option<&'a coding_adventures_javascript_ast::SwitchCase> {
    for case in cases {
        if let Some(test) = &case.test {
            if strict_equal_leaves(test, discriminant) {
                return Some(case);
            }
        }
    }
    cases.iter().find(|c| c.test.is_none())
}

/// Build a new `BlockStatement.body` from a case consequent, with
/// the trailing **unlabeled** `BreakStatement` (if any) stripped.
///
/// Why only unlabeled: a bare `break;` inside a switch case exits
/// just that switch. Once we collapse the switch away, the
/// bare break has nothing to exit and is dead weight — strip it.
///
/// A `break label;` inside a switch case exits the switch AND
/// transfers control to after the named labeled-statement (typically
/// an outer loop or block). That escape is still semantically
/// required after the switch is collapsed; the labeled break must
/// stay in the resulting block so the outer-label exit still
/// happens. Stripping it would silently keep an enclosing loop
/// running — an observable behaviour change.
///
/// `return` and `throw` at the end always stay — they have
/// observable behaviour beyond just terminating the switch.
///
/// Used by gap-014 step 4 / CLOC12.36 when collapsing a switch
/// down to its single matching case body.
fn strip_trailing_break(consequent: &[Statement]) -> Vec<Statement> {
    if consequent.is_empty() {
        return Vec::new();
    }
    let last_idx = consequent.len() - 1;
    let last_is_unlabeled_break = matches!(
        &consequent[last_idx],
        Statement::Tagged(TaggedStatement::BreakStatement(b)) if b.label.is_none()
    );
    if last_is_unlabeled_break {
        consequent[..last_idx].to_vec()
    } else {
        consequent.to_vec()
    }
}

/// Returns `true` when an inner BlockStatement's body can be
/// hoisted into the enclosing block without changing ECMAScript
/// scoping semantics. Used by the block-flattening fold
/// (CLOC12.19 / gap-010).
///
/// Block-scoped declarations (`let`, `const`, `class`, inner
/// function declarations) are bound to their enclosing block.
/// Hoisting them upward either leaks the binding to a wider
/// scope or causes a redeclaration error against a same-named
/// binding in the outer block — either way, observably different.
///
/// `var` is function-scoped, so hoisting `{var x = 1;}` out of
/// an inner block to the function-body level produces the same
/// effective binding (a `var` declaration was already implicitly
/// hoisted to the function scope by the spec). Hence `Var`-kind
/// declarations are safe to flatten.
fn block_is_scope_safe_to_flatten(b: &BlockStatement) -> bool {
    b.body.iter().all(|s| match s {
        Statement::Declaration(Declaration::VariableDeclaration(v)) => {
            matches!(v.kind, VarKind::Var)
        }
        Statement::Declaration(Declaration::FunctionDeclaration(_)) => false,
        // A `class` declaration is block-scoped (per the doc above) — hoisting
        // it out of an inner block would leak the binding, so it is never safe
        // to flatten, exactly like a nested function declaration.
        Statement::Declaration(Declaration::ClassDeclaration(_)) => false,
        // An import declaration is module-top-level only; it never legally
        // appears inside a block, so flattening/truncating it away is unsafe.
        Statement::Declaration(Declaration::ImportDeclaration(_)) => false,
        Statement::Declaration(Declaration::ExportNamedDeclaration(_)) => false,
        Statement::Declaration(Declaration::ExportDefaultDeclaration(_)) => false,
        Statement::Declaration(Declaration::ExportAllDeclaration(_)) => false,
        // Tagged statements never introduce a new lexical binding
        // by themselves. `ExpressionStatement`, control flow,
        // `EmptyStatement`, etc. are all safe.
        Statement::Tagged(_) => true,
    })
}

fn is_empty_statement(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Tagged(TaggedStatement::EmptyStatement(_))
    )
}

/// Is this a `debugger;` statement?
///
/// Used by the block-body and top-level sweeps (CLOC24) to strip `debugger`
/// statements at SIMPLE/ADVANCED — a development-only breakpoint with no effect
/// on a shipped program, so removing it is a sound size win (this matches
/// upstream Closure). Because the dce pass runs only inside the typed pipeline,
/// `debugger` is preserved at WHITESPACE_ONLY, which never reaches here.
fn is_debugger_statement(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Tagged(TaggedStatement::DebuggerStatement(_))
    )
}

/// Is this top-level program item a `debugger;` statement? The program body is
/// a list of `ProgramItem`s rather than `Statement`s, so the top-level sweep in
/// `dce_program` needs this thin wrapper over [`is_debugger_statement`].
fn is_debugger_program_item(item: &ProgramItem) -> bool {
    matches!(item, ProgramItem::Statement(s) if is_debugger_statement(s))
}

/// Is this top-level item a bare `EmptyStatement` (`;`)? Mirrors
/// [`is_empty_statement`] for the `ProgramItem` list, so [`dce_program`] can
/// sweep stray semicolons out of the program body the same way
/// [`dce_block_statement`] sweeps them out of a block body. An `EmptyStatement`
/// only ever appears as a `ProgramItem::Statement`, never a `Declaration`.
fn is_empty_program_item(item: &ProgramItem) -> bool {
    matches!(item, ProgramItem::Statement(s) if is_empty_statement(s))
}

/// Fetch a statement's own correlation-vector id, if it carries one.
///
/// DCE's deletion sites need the *removed* node's CV id — not just the
/// enclosing container's — so they can tombstone the exact span that
/// vanished (see [`DceState::record_deletion`]). Every AST node struct
/// carries an `Option<CvId>`; this unwraps the `Statement` →
/// `TaggedStatement` nesting to reach it.
///
/// A [`Statement::Declaration`] returns `None`, and that is correct
/// rather than lossy: DCE's removal sites never drop a `var` / `function`
/// declaration. The dead-tail truncate is gated on
/// `tail_is_safe_to_truncate`, whose whitelist excludes both (they
/// hoist, so removing them could break earlier code), and the empty /
/// debugger sweeps only ever match those two leaf kinds. So there is no
/// declaration deletion here to attribute in the first place.
fn statement_cv(stmt: &Statement) -> Option<String> {
    match stmt {
        Statement::Tagged(t) => tagged_statement_cv(t),
        Statement::Declaration(_) => None,
    }
}

/// The `TaggedStatement` arm of [`statement_cv`]. Every variant carries a
/// `cv: Option<CvId>`; we clone it out. Kept as an exhaustive match (no
/// `_` wildcard) on purpose: if a new statement kind is added upstream,
/// this fails to compile and forces a conscious decision here rather than
/// silently dropping the new kind's provenance.
fn tagged_statement_cv(t: &TaggedStatement) -> Option<String> {
    use TaggedStatement::*;
    match t {
        ExpressionStatement(s) => s.cv.clone(),
        BlockStatement(s) => s.cv.clone(),
        IfStatement(s) => s.cv.clone(),
        WhileStatement(s) => s.cv.clone(),
        WithStatement(s) => s.cv.clone(),
        DoWhileStatement(s) => s.cv.clone(),
        ForStatement(s) => s.cv.clone(),
        ForInStatement(s) => s.cv.clone(),
        ForOfStatement(s) => s.cv.clone(),
        ReturnStatement(s) => s.cv.clone(),
        BreakStatement(s) => s.cv.clone(),
        ContinueStatement(s) => s.cv.clone(),
        LabeledStatement(s) => s.cv.clone(),
        ThrowStatement(s) => s.cv.clone(),
        SwitchStatement(s) => s.cv.clone(),
        TryStatement(s) => s.cv.clone(),
        EmptyStatement(s) => s.cv.clone(),
        DebuggerStatement(s) => s.cv.clone(),
    }
}

/// Top-level analogue of [`statement_cv`] for a `ProgramItem`. Only the
/// `Statement` arm can be a `debugger;` (the sole top-level deletion), so
/// a `Declaration` item falls through to `None`.
fn program_item_cv(item: &ProgramItem) -> Option<String> {
    match item {
        ProgramItem::Statement(s) => statement_cv(s),
        ProgramItem::Declaration(_) => None,
    }
}

// =====================================================================
// Declarations
// =====================================================================

fn dce_declaration(decl: &Declaration, st: &mut DceState) -> Declaration {
    st.visit();
    match decl {
        Declaration::VariableDeclaration(v) => {
            Declaration::VariableDeclaration(dce_variable_declaration(v, st))
        }
        Declaration::FunctionDeclaration(f) => {
            Declaration::FunctionDeclaration(FunctionDeclaration {
                cv: f.cv.clone(),
                id: f.id.clone(),
                params: f.params.clone(),
                body: dce_block_statement(&f.body, st),
                generator: f.generator,
                is_async: f.is_async,
            })
        }
        // A class *declaration* runs DCE inside its heritage operand and each
        // method body, exactly as `dce_class` does for a class *expression* —
        // only the outer node type and the required `id` differ.
        Declaration::ImportDeclaration(i) => Declaration::ImportDeclaration(i.clone()),
        Declaration::ExportNamedDeclaration(i) => Declaration::ExportNamedDeclaration(i.clone()),
        Declaration::ExportDefaultDeclaration(i) => Declaration::ExportDefaultDeclaration(i.clone()),
        Declaration::ExportAllDeclaration(i) => Declaration::ExportAllDeclaration(i.clone()),
        Declaration::ClassDeclaration(c) => {
            Declaration::ClassDeclaration(dce_class_declaration(c, st))
        }
    }
}

/// DCE inside a class *declaration*: the `extends` operand and each method
/// body. Mirrors `dce_class` (the expression form).
fn dce_class_declaration(c: &ClassDeclaration, st: &mut DceState) -> ClassDeclaration {
    ClassDeclaration {
        cv: c.cv.clone(),
        id: c.id.clone(),
        super_class: c.super_class.as_ref().map(|s| Box::new(dce_expression(s, st))),
        body: c
            .body
            .iter()
            .map(|m| match m {
                ClassMember::Method(md) => ClassMember::Method(MethodDefinition {
                    cv: md.cv.clone(),
                    key: md.key.clone(),
                    kind: md.kind,
                    value: FunctionExpression {
                        cv: md.value.cv.clone(),
                        id: md.value.id.clone(),
                        params: md.value.params.clone(),
                        body: dce_block_statement(&md.value.body, st),
                        generator: md.value.generator,
                        is_async: md.value.is_async,
                    },
                    computed: md.computed,
                    is_static: md.is_static,
                }),
                // A class field runs DCE inside its initializer (an expression
                // that runs at construction). The key is cloned; the value is
                // optional.
                ClassMember::Field(fd) => ClassMember::Field(PropertyDefinition {
                    cv: fd.cv.clone(),
                    key: fd.key.clone(),
                    value: fd.value.as_ref().map(|v| dce_expression(v, st)),
                    computed: fd.computed,
                    is_static: fd.is_static,
                }),
                // A static-init block runs DCE inside each of its statements
                // (they run at class-definition time) — mirroring the method
                // body. No key, no binding name; only the statement list recurses.
                ClassMember::StaticBlock(b) => ClassMember::StaticBlock(dce_block_statement(b, st)),
            })
            .collect(),
    }
}

fn dce_variable_declaration(
    v: &VariableDeclaration,
    st: &mut DceState,
) -> VariableDeclaration {
    VariableDeclaration {
        cv: v.cv.clone(),
        kind: v.kind,
        declarations: v
            .declarations
            .iter()
            .map(|d| VariableDeclarator {
                cv: d.cv.clone(),
                id: d.id.clone(),
                init: d.init.as_ref().map(|e| dce_expression(e, st)),
            })
            .collect(),
    }
}

// =====================================================================
// Expressions — recurse only (DCE doesn't collapse expressions)
// =====================================================================

/// DCE inside a class expression: the `extends` operand and each method body.
/// `#[inline(never)]` so it does not inflate `dce_expression`'s frame.
#[inline(never)]
fn dce_class(c: &ClassExpression, st: &mut DceState) -> Expression {
    Expression::ClassExpression(ClassExpression {
        cv: c.cv.clone(),
        id: c.id.clone(),
        super_class: c.super_class.as_ref().map(|s| Box::new(dce_expression(s, st))),
        body: c
            .body
            .iter()
            .map(|m| match m {
                ClassMember::Method(md) => ClassMember::Method(MethodDefinition {
                    cv: md.cv.clone(),
                    key: md.key.clone(),
                    kind: md.kind,
                    value: FunctionExpression {
                        cv: md.value.cv.clone(),
                        id: md.value.id.clone(),
                        params: md.value.params.clone(),
                        body: dce_block_statement(&md.value.body, st),
                        generator: md.value.generator,
                        is_async: md.value.is_async,
                    },
                    computed: md.computed,
                    is_static: md.is_static,
                }),
                // A class field runs DCE inside its initializer (an expression
                // that runs at construction). The key is cloned; the value is
                // optional.
                ClassMember::Field(fd) => ClassMember::Field(PropertyDefinition {
                    cv: fd.cv.clone(),
                    key: fd.key.clone(),
                    value: fd.value.as_ref().map(|v| dce_expression(v, st)),
                    computed: fd.computed,
                    is_static: fd.is_static,
                }),
                // A static-init block runs DCE inside each of its statements
                // (they run at class-definition time) — mirroring the method
                // body. No key, no binding name; only the statement list recurses.
                ClassMember::StaticBlock(b) => ClassMember::StaticBlock(dce_block_statement(b, st)),
            })
            .collect(),
    })
}

fn dce_expression(expr: &Expression, st: &mut DceState) -> Expression {
    st.visit();
    match expr {
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        // `this` is a leaf keyword — no sub-expression to walk, never dead on
        // its own, so it clones through like the literals.
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        | Expression::UndefinedLiteral(_) => expr.clone(),

        Expression::BinaryExpression(b) => Expression::BinaryExpression(BinaryExpression {
            cv: b.cv.clone(),
            operator: b.operator,
            left: Box::new(dce_expression(&b.left, st)),
            right: Box::new(dce_expression(&b.right, st)),
        }),
        Expression::LogicalExpression(l) => Expression::LogicalExpression(LogicalExpression {
            cv: l.cv.clone(),
            operator: l.operator,
            left: Box::new(dce_expression(&l.left, st)),
            right: Box::new(dce_expression(&l.right, st)),
        }),
        Expression::UnaryExpression(u) => Expression::UnaryExpression(UnaryExpression {
            cv: u.cv.clone(),
            operator: u.operator,
            prefix: u.prefix,
            argument: Box::new(dce_expression(&u.argument, st)),
        }),
        // `++x` / `x++` has a side effect (mutates its operand), so it is NOT
        // dead code even in value-discarded position — preserve it, recursing
        // only into the argument.
        Expression::UpdateExpression(u) => Expression::UpdateExpression(UpdateExpression {
            cv: u.cv.clone(),
            operator: u.operator,
            prefix: u.prefix,
            argument: Box::new(dce_expression(&u.argument, st)),
        }),
        Expression::AssignmentExpression(a) => {
            Expression::AssignmentExpression(AssignmentExpression {
                cv: a.cv.clone(),
                operator: a.operator,
                left: a.left.clone(),
                right: Box::new(dce_expression(&a.right, st)),
            })
        }
        Expression::ConditionalExpression(c) => {
            Expression::ConditionalExpression(ConditionalExpression {
                cv: c.cv.clone(),
                test: Box::new(dce_expression(&c.test, st)),
                consequent: Box::new(dce_expression(&c.consequent, st)),
                alternate: Box::new(dce_expression(&c.alternate, st)),
            })
        }
        Expression::CallExpression(c) => Expression::CallExpression(CallExpression {
            cv: c.cv.clone(),
            callee: Box::new(dce_expression(&c.callee, st)),
            arguments: c.arguments.iter().map(|a| dce_expression(a, st)).collect(),
        }),
        // `new X(args)` has construction side effects — never elided; recurse
        // into callee and arguments to reach any dead code nested within.
        Expression::NewExpression(n) => Expression::NewExpression(NewExpression {
            cv: n.cv.clone(),
            callee: Box::new(dce_expression(&n.callee, st)),
            arguments: n.arguments.iter().map(|a| dce_expression(a, st)).collect(),
        }),
        // `a, b, c` — recurse into each operand; the sequence itself is kept
        // (its operands may have side effects).
        Expression::SequenceExpression(s) => Expression::SequenceExpression(SequenceExpression {
            cv: s.cv.clone(),
            expressions: s.expressions.iter().map(|e| dce_expression(e, st)).collect(),
        }),
        // `` tag`a${x}b` `` — a tagged template is kept (the tag call may have
        // side effects); recurse into the tag and each `${…}` substitution.
        Expression::TaggedTemplateExpression(t) => {
            Expression::TaggedTemplateExpression(TaggedTemplateExpression {
                cv: t.cv.clone(),
                tag: Box::new(dce_expression(&t.tag, st)),
                quasi: TemplateLiteral {
                    cv: t.quasi.cv.clone(),
                    quasis: t.quasi.quasis.clone(),
                    expressions: t
                        .quasi
                        .expressions
                        .iter()
                        .map(|e| dce_expression(e, st))
                        .collect(),
                },
            })
        }
        // `...arg` — kept (the spread's iterable may have side effects);
        // recurse into the argument.
        Expression::SpreadElement(s) => Expression::SpreadElement(SpreadElement {
            cv: s.cv.clone(),
            argument: Box::new(dce_expression(&s.argument, st)),
        }),
        Expression::YieldExpression(y) => Expression::YieldExpression(YieldExpression {
            cv: y.cv.clone(),
            delegate: y.delegate,
            argument: y.argument.as_ref().map(|a| Box::new(dce_expression(a, st))),
        }),
        Expression::AwaitExpression(a) => Expression::AwaitExpression(AwaitExpression {
            cv: a.cv.clone(),
            argument: Box::new(dce_expression(&a.argument, st)),
        }),
        Expression::ImportExpression(e) => Expression::ImportExpression(ImportExpression {
            cv: e.cv.clone(),
            source: Box::new(dce_expression(&e.source, st)),
        }),
        Expression::MemberExpression(m) => Expression::MemberExpression(MemberExpression {
            cv: m.cv.clone(),
            object: Box::new(dce_expression(&m.object, st)),
            property: Box::new(dce_expression(&m.property, st)),
            computed: m.computed,
        }),
        // `a?.b` / `a?.[k]` — structurally identical to a member access; rebuild
        // it recursing into object and property exactly as `MemberExpression`.
        Expression::OptionalMemberExpression(m) => {
            Expression::OptionalMemberExpression(OptionalMemberExpression {
                cv: m.cv.clone(),
                object: Box::new(dce_expression(&m.object, st)),
                property: Box::new(dce_expression(&m.property, st)),
                computed: m.computed,
            })
        }
        // `a?.()` — an optional call has the same side-effect profile as an
        // ordinary call; recurse into callee and arguments and keep it.
        Expression::OptionalCallExpression(c) => {
            Expression::OptionalCallExpression(OptionalCallExpression {
                cv: c.cv.clone(),
                callee: Box::new(dce_expression(&c.callee, st)),
                arguments: c.arguments.iter().map(|a| dce_expression(a, st)).collect(),
            })
        }
        // A chain expression is a transparent optional-chain wrapper — recurse
        // into its inner expression and rewrap.
        Expression::ChainExpression(c) => Expression::ChainExpression(ChainExpression {
            cv: c.cv.clone(),
            expression: Box::new(dce_expression(&c.expression, st)),
        }),
        Expression::ArrayExpression(a) => Expression::ArrayExpression(ArrayExpression {
            cv: a.cv.clone(),
            elements: a
                .elements
                .iter()
                .map(|e| e.as_ref().map(|x| dce_expression(x, st)))
                .collect(),
        }),
        Expression::ObjectExpression(o) => Expression::ObjectExpression(ObjectExpression {
            cv: o.cv.clone(),
            properties: o
                .properties
                .iter()
                .map(|member| match member {
                    ObjectMember::Property(p) => ObjectMember::Property(Property {
                        cv: p.cv.clone(),
                        kind: p.kind,
                        key: match &p.key {
                            PropertyKey::Identifier(i) => PropertyKey::Identifier(i.clone()),
                            // A private name (`#x`) never occurs in an object
                            // literal, but the match must stay exhaustive.
                            PropertyKey::PrivateName(p) => PropertyKey::PrivateName(p.clone()),
                            PropertyKey::StringLiteral(s) => {
                                PropertyKey::StringLiteral(s.clone())
                            }
                            PropertyKey::NumericLiteral(n) => {
                                PropertyKey::NumericLiteral(n.clone())
                            }
                            PropertyKey::Expression(e) => {
                                PropertyKey::Expression(Box::new(dce_expression(e, st)))
                            }
                        },
                        value: Box::new(dce_expression(&p.value, st)),
                        computed: p.computed,
                        shorthand: p.shorthand,
                        method: p.method,
                    }),
                    // Object spread `...expr` — recurse into the spread argument.
                    ObjectMember::Spread(s) => ObjectMember::Spread(SpreadElement {
                        cv: s.cv.clone(),
                        argument: Box::new(dce_expression(&s.argument, st)),
                    }),
                })
                .collect(),
        }),
        // Recurse into a function-value's body exactly as `dce_declaration`
        // does for a `FunctionDeclaration` — dead code after a
        // `return`/`throw` inside `var f = function(){ return; g(); }`
        // is just as dead as in a named function.
        Expression::FunctionExpression(f) => Expression::FunctionExpression(FunctionExpression {
            cv: f.cv.clone(),
            id: f.id.clone(),
            params: f.params.clone(),
            body: dce_block_statement(&f.body, st),
            generator: f.generator,
            is_async: f.is_async,
        }),
        // A class expression: DCE the `extends` operand and each method body
        // exactly as a function body. Delegated to an `#[inline(never)]` helper
        // so this arm does not enlarge `dce_expression`'s frame on the hot
        // recursive path (deep-nesting stack-overflow DoS lesson).
        Expression::ClassExpression(c) => dce_class(c, st),
        // Recurse into an arrow-value's body. A block body eliminates
        // dead code after `return`/`throw` exactly as a function body
        // does; a concise (expression) body has no statements — a single
        // expression can't contain dead code — so we simply recurse into
        // it to reach any nested functions/arrows.
        Expression::ArrowFunctionExpression(a) => {
            Expression::ArrowFunctionExpression(ArrowFunctionExpression {
                cv: a.cv.clone(),
                params: a.params.clone(),
                body: match &a.body {
                    ArrowBody::Block(b) => ArrowBody::Block(dce_block_statement(b, st)),
                    ArrowBody::Expression(e) => {
                        ArrowBody::Expression(Box::new(dce_expression(e, st)))
                    }
                },
                is_async: a.is_async,
            })
        }
        // Recurse into a template literal's `${…}` expressions (a single
        // expression can't contain dead code, but nested functions/arrows
        // inside it can). The `quasis` are leaf strings.
        Expression::TemplateLiteral(t) => Expression::TemplateLiteral(TemplateLiteral {
            cv: t.cv.clone(),
            quasis: t.quasis.clone(),
            expressions: t.expressions.iter().map(|e| dce_expression(e, st)).collect(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_closure_pass_constant_fold::ConstantFoldPass;
    use coding_adventures_closure_pass_fold_control_flow::FoldControlFlowPass;
    use coding_adventures_closure_pass_pipeline::{PassPipeline, PipelineOutput};
    use coding_adventures_javascript_ast::{
        statement::TaggedStatement, BinaryOperator, BooleanLiteral, CatchClause, EmptyStatement,
        Identifier, NumericLiteral, SourceType, SwitchCase, SwitchStatement, TryStatement,
    };
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }
    fn untraced_program() -> Program {
        Program::new_untraced(EsVersion::Es2025, SourceType::Module)
    }

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
            raw: v.to_string(),
        })
    }
    fn boolean(v: bool) -> Expression {
        Expression::BooleanLiteral(BooleanLiteral { cv: None, value: v })
    }
    fn expr_stmt(expr: Expression) -> Statement {
        Statement::expression_statement(ExpressionStatement {
            cv: None,
            expression: expr,
        })
    }
    fn return_stmt() -> Statement {
        Statement::return_statement(ReturnStatement {
            cv: None,
            argument: None,
        })
    }
    fn empty_stmt() -> Statement {
        Statement::empty_statement(EmptyStatement { cv: None })
    }
    fn debugger_stmt() -> Statement {
        Statement::debugger_statement(
            coding_adventures_javascript_ast::DebuggerStatement { cv: None },
        )
    }
    /// `let <name>;` as a block statement — a non-hoisted (block-scoped)
    /// declaration, which the truncation whitelist treats as safe to drop.
    fn let_decl_stmt(name: &str) -> Statement {
        Statement::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind: VarKind::Let,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: coding_adventures_javascript_ast::BindingTarget::Identifier(Identifier {
                    cv: None,
                    name: name.to_string(),
                }),
                init: None,
            }],
        }))
    }
    /// `if (<test>) <consequent>;` — a compound statement. The truncation
    /// whitelist treats every compound statement as unsafe to drop (it could
    /// transitively wrap a hoisted `var`, e.g. `if (c) var y;`), so a tail
    /// containing one is preserved.
    fn if_stmt(test: Expression, consequent: Statement) -> Statement {
        Statement::if_statement(coding_adventures_javascript_ast::IfStatement {
            cv: None,
            test,
            consequent: Box::new(consequent),
            alternate: None,
        })
    }
    /// A `function <name>() {}` declaration as a block statement. Used to test
    /// the hoisting guard: a function declaration is hoisted, so it must not be
    /// dropped from a dead-after-terminator tail.
    fn func_decl_stmt(name: &str) -> Statement {
        Statement::Declaration(Declaration::FunctionDeclaration(FunctionDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: name.to_string(),
            },
            params: vec![],
            body: BlockStatement {
                cv: None,
                body: vec![],
            },
            generator: false,
            is_async: false,
        }))
    }

    fn run_pass(prog: Program) -> (Program, Vec<Contribution>, bool, u32) {
        let pass = DcePass::new();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let ctx = PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        };
        let out = pass.run(ctx).expect("pass should succeed");
        (out.program, out.contributions, out.changed, out.stats.nodes_touched)
    }

    /// Wrap `body` in a FunctionDeclaration so the block is
    /// reachable from a Program body. Returns the Program.
    fn program_with_function(body: Vec<Statement>, cv: Option<&str>) -> Program {
        let block = BlockStatement {
            cv: cv.map(|s| s.to_string()),
            body,
        };
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
        program().with_body(vec![ProgramItem::Declaration(fdecl)])
    }

    fn extract_function_body(prog: &Program) -> &BlockStatement {
        let ProgramItem::Declaration(Declaration::FunctionDeclaration(f)) = &prog.body[0]
        else {
            panic!("expected a FunctionDeclaration at body[0]");
        };
        &f.body
    }

    // ---------------- CV deletion provenance (#89) -------------
    //
    // These mirror the production pipeline: the lexer/parser `create`
    // a CV entry per node and stamp its id onto the AST. So here we
    // `create` the entry FIRST, stamp the returned id onto the node,
    // then run the pass — otherwise `cv.delete` has no entry to
    // tombstone and the assertion would be vacuous. The property under
    // test: when DCE removes a node, that node's CV entry survives in
    // the log carrying a `DeletionRecord{source:"dce", reason:<tag>}`,
    // so "what happened to this span?" is always answerable.

    /// Like [`run_pass`] but threads the caller's CV log through so its
    /// `DeletionRecord`s can be inspected after the pass returns.
    fn run_pass_capturing_cv(prog: &Program, cv: &mut CVLog) -> Program {
        let sidecar = Sidecar::new();
        let ctx = PassContext {
            program: prog,
            sidecar: &sidecar,
            cv,
        };
        DcePass::new()
            .run(ctx)
            .expect("pass should succeed")
            .program
    }

    /// A traced `ExpressionStatement` whose CV id is freshly created in
    /// `log` (so an entry exists for a later `delete` to tombstone).
    fn traced_expr_stmt(log: &mut CVLog, name: &str) -> (Statement, String) {
        let id = log.create(None);
        let stmt = Statement::expression_statement(ExpressionStatement {
            cv: Some(id.clone()),
            expression: ident(name),
        });
        (stmt, id)
    }

    /// A traced `debugger;` whose CV id is freshly created in `log`.
    fn traced_debugger(log: &mut CVLog) -> (Statement, String) {
        let id = log.create(None);
        let stmt = Statement::debugger_statement(
            coding_adventures_javascript_ast::DebuggerStatement {
                cv: Some(id.clone()),
            },
        );
        (stmt, id)
    }

    #[test]
    fn dead_code_removal_tombstones_each_removed_node() {
        // { x; return; y; z; } — `y` and `z` are unreachable. Each must
        // be tombstoned by dce, not silently dropped.
        let mut log = CVLog::new(true);
        let (dead_y, y_id) = traced_expr_stmt(&mut log, "y");
        let (dead_z, z_id) = traced_expr_stmt(&mut log, "z");
        let body = vec![expr_stmt(ident("x")), return_stmt(), dead_y, dead_z];
        let prog = program_with_function(body, Some("block.1"));

        let _out = run_pass_capturing_cv(&prog, &mut log);

        for (id, label) in [(&y_id, "y"), (&z_id, "z")] {
            let entry = log
                .get(id)
                .expect("a removed node must remain in the CV log as a tombstone");
            let del = entry
                .deleted
                .as_ref()
                .unwrap_or_else(|| panic!("dead statement `{label}` must be tombstoned"));
            assert_eq!(del.source, "dce");
            assert_eq!(del.reason, "removed-dead-code");
            assert_eq!(
                del.meta.get("container_cv").and_then(|v| v.as_str()),
                Some("block.1"),
                "tombstone should record the enclosing container's cv"
            );
        }
    }

    #[test]
    fn block_debugger_removal_tombstones_the_statement() {
        let mut log = CVLog::new(true);
        let (dbg, dbg_id) = traced_debugger(&mut log);
        let prog = program_with_function(vec![expr_stmt(ident("keep")), dbg], Some("block.7"));

        let _out = run_pass_capturing_cv(&prog, &mut log);

        let del = log
            .get(&dbg_id)
            .unwrap()
            .deleted
            .as_ref()
            .expect("a stripped debugger must be tombstoned");
        assert_eq!(del.source, "dce");
        assert_eq!(del.reason, "removed-debugger");
    }

    #[test]
    fn empty_statement_removal_tombstones_the_statement() {
        let mut log = CVLog::new(true);
        let empty_id = log.create(None);
        let empty = Statement::empty_statement(EmptyStatement {
            cv: Some(empty_id.clone()),
        });
        let prog = program_with_function(vec![expr_stmt(ident("keep")), empty], Some("block.9"));

        let _out = run_pass_capturing_cv(&prog, &mut log);

        let del = log
            .get(&empty_id)
            .unwrap()
            .deleted
            .as_ref()
            .expect("a swept empty statement must be tombstoned");
        assert_eq!(del.reason, "removed-empty-statement");
    }

    #[test]
    fn top_level_debugger_removal_tombstones_the_statement() {
        // The program-body sweep is a separate code path from the
        // block-body sweep, so it gets its own tombstone test.
        let mut log = CVLog::new(true);
        let (dbg, dbg_id) = traced_debugger(&mut log);
        let prog = program().with_body(vec![ProgramItem::Statement(dbg)]);

        let _out = run_pass_capturing_cv(&prog, &mut log);

        let del = log
            .get(&dbg_id)
            .unwrap()
            .deleted
            .as_ref()
            .expect("a stripped top-level debugger must be tombstoned");
        assert_eq!(del.reason, "removed-debugger");
    }

    #[test]
    fn top_level_empty_statement_is_removed() {
        // `keep(); ;` at PROGRAM level → `keep();` — CLOC12.195. Before this the
        // program-body sweep only stripped `debugger`, leaving stray top-level
        // `;` behind (block bodies were already cleaned by `dce_block_statement`).
        let prog = program().with_body(vec![
            ProgramItem::Statement(expr_stmt(ident("keep"))),
            ProgramItem::Statement(empty_stmt()),
        ]);
        let (out, _c, changed, _n) = run_pass(prog);
        assert!(changed, "a stray top-level `;` should be removed");
        assert_eq!(out.body.len(), 1, "only the kept statement survives");
        assert!(matches!(
            &out.body[0],
            ProgramItem::Statement(Statement::Tagged(TaggedStatement::ExpressionStatement(_)))
        ));
    }

    #[test]
    fn multiple_top_level_empty_statements_all_removed() {
        // `; ; keep(); ;` → `keep();` — leading, interior, and trailing empties.
        let prog = program().with_body(vec![
            ProgramItem::Statement(empty_stmt()),
            ProgramItem::Statement(empty_stmt()),
            ProgramItem::Statement(expr_stmt(ident("keep"))),
            ProgramItem::Statement(empty_stmt()),
        ]);
        let (out, _c, changed, _n) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 1, "all three empties swept, `keep` remains");
    }

    #[test]
    fn top_level_empty_statement_removal_tombstones_the_statement() {
        // The program-body empty sweep is a separate code path from the
        // block-body sweep, so it gets its own tombstone test (mirrors the
        // top-level debugger tombstone test).
        let mut log = CVLog::new(true);
        let empty_id = log.create(None);
        let empty = Statement::empty_statement(EmptyStatement {
            cv: Some(empty_id.clone()),
        });
        let prog = program().with_body(vec![
            ProgramItem::Statement(expr_stmt(ident("keep"))),
            ProgramItem::Statement(empty),
        ]);

        let _out = run_pass_capturing_cv(&prog, &mut log);

        let del = log
            .get(&empty_id)
            .unwrap()
            .deleted
            .as_ref()
            .expect("a stripped top-level empty statement must be tombstoned");
        assert_eq!(del.source, "dce");
        assert_eq!(del.reason, "removed-empty-statement");
    }

    #[test]
    fn block_flatten_does_not_tombstone_moved_statements() {
        // { { a; } keep; } → { a; keep; }. Flattening MOVES `a` up one
        // scope level — it is not deleted — so `a` must stay live in the
        // CV log with no `DeletionRecord`. This is the invariant that
        // keeps `block-flattened` off the `record_deletion` path.
        let mut log = CVLog::new(true);
        let (moved_a, a_id) = traced_expr_stmt(&mut log, "a");
        let inner = Statement::block_statement(BlockStatement {
            cv: Some("inner".to_string()),
            body: vec![moved_a],
        });
        let body = vec![inner, expr_stmt(ident("keep"))];
        let prog = program_with_function(body, Some("outer"));

        let _out = run_pass_capturing_cv(&prog, &mut log);

        assert!(
            log.get(&a_id)
                .expect("moved node must remain in the CV log")
                .deleted
                .is_none(),
            "a flattened (moved) statement must NOT be tombstoned"
        );
    }

    #[test]
    fn disabled_log_still_removes_code_without_panicking() {
        // With CV disabled, `delete` is a no-op; the pass must still
        // strip the debugger and never panic on the missing entry.
        let mut log = CVLog::new(false);
        let (dbg, _dbg_id) = traced_debugger(&mut log);
        let prog = program_with_function(vec![expr_stmt(ident("keep")), dbg], Some("b"));

        let out = run_pass_capturing_cv(&prog, &mut log);

        let block = extract_function_body(&out);
        assert_eq!(
            block.body.len(),
            1,
            "debugger must still be stripped under a disabled CV log"
        );
    }

    // ---------------- metadata + identity ---------------------

    #[test]
    fn name_is_dce() {
        assert_eq!(DcePass::new().name(), "dce");
    }

    #[test]
    fn iteration_policy_is_fixed_point() {
        assert_eq!(
            DcePass::new().iteration_policy(),
            IterationPolicy::FixedPoint
        );
    }

    #[test]
    fn cost_is_three_pass_units() {
        assert_eq!(DcePass::new().cost(), 3);
    }

    #[test]
    fn depends_on_constant_fold() {
        assert_eq!(DcePass::new().depends_on(), &["constant-fold"]);
    }

    #[test]
    fn empty_program_is_identity() {
        let (_out, contribs, changed, _) = run_pass(program());
        assert!(!changed);
        assert!(contribs.is_empty());
    }

    // ---------------- dead-after-return -----------------------

    #[test]
    fn drops_statements_after_return() {
        // { x; return; y; z; } → { x; return; }
        let body = vec![
            expr_stmt(ident("x")),
            return_stmt(),
            expr_stmt(ident("y")),
            expr_stmt(ident("z")),
        ];
        let prog = program_with_function(body, Some("block.1"));
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        assert!(
            contribs.iter().any(|c| c.tag == "removed-dead-code"),
            "expected removed-dead-code contribution; got {:?}",
            contribs
        );
        let new_block = extract_function_body(&out);
        assert_eq!(new_block.body.len(), 2, "expected 2 statements; got {:?}", new_block.body);
    }

    #[test]
    fn drops_statements_after_throw() {
        // { x; throw e; y; z; } → { x; throw e; }
        // `throw` unconditionally exits the block, so `y` and `z` are
        // unreachable — dropped exactly like dead-after-return.
        let body = vec![
            expr_stmt(ident("x")),
            throw_stmt(ident("e")),
            expr_stmt(ident("y")),
            expr_stmt(ident("z")),
        ];
        let prog = program_with_function(body, Some("block.1"));
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        assert!(
            contribs.iter().any(|c| c.tag == "removed-dead-code"),
            "expected removed-dead-code contribution; got {:?}",
            contribs
        );
        let new_block = extract_function_body(&out);
        assert_eq!(
            new_block.body.len(),
            2,
            "expected 2 statements (x; throw e;); got {:?}",
            new_block.body
        );
    }

    #[test]
    fn keeps_throw_with_nothing_after_it() {
        // { x; throw e; } is already minimal — the terminator is the last
        // statement, so nothing is dropped.
        let body = vec![expr_stmt(ident("x")), throw_stmt(ident("e"))];
        let prog = program_with_function(body, Some("block.1"));
        let (out, _contribs, changed, _) = run_pass(prog);
        assert!(!changed);
        let new_block = extract_function_body(&out);
        assert_eq!(new_block.body.len(), 2);
    }

    #[test]
    fn does_not_drop_hoisted_function_after_throw() {
        // SOUNDNESS: `{ h; throw e; function h(){} }` — `h` is hoisted, so the
        // reference `h` BEFORE the throw resolves to the declaration AFTER it.
        // Dropping `function h(){}` would make `h` a ReferenceError. The tail
        // contains a hoisted declaration, so truncation is declined: all three
        // statements survive.
        let body = vec![
            expr_stmt(ident("h")),
            throw_stmt(ident("e")),
            func_decl_stmt("h"),
        ];
        let prog = program_with_function(body, Some("block.1"));
        let (out, _contribs, changed, _) = run_pass(prog);
        assert!(!changed, "must not truncate past a hoisted function decl");
        let new_block = extract_function_body(&out);
        assert_eq!(
            new_block.body.len(),
            3,
            "hoisted `function h` must be preserved; got {:?}",
            new_block.body
        );
    }

    #[test]
    fn does_not_drop_hoisted_function_after_return() {
        // Same guard for `return` (the path the bug pre-existed on): a hoisted
        // function declaration in the unreachable tail is preserved.
        let body = vec![return_stmt(), func_decl_stmt("h")];
        let prog = program_with_function(body, Some("block.1"));
        let (out, _contribs, changed, _) = run_pass(prog);
        assert!(!changed, "must not truncate past a hoisted function decl");
        let new_block = extract_function_body(&out);
        assert_eq!(new_block.body.len(), 2);
    }

    #[test]
    fn does_not_drop_compound_statement_tail() {
        // The whitelist is conservative about COMPOUND statements: an `if`
        // (like `for`/`while`/block/`switch`) can transitively wrap a hoisted
        // `var` (`if (c) var y;`), which a top-level-`Declaration`-only check
        // would miss. So a tail containing any compound statement is preserved,
        // even when the compound here holds no var — declining to drop dead
        // code is never a miscompile.
        let body = vec![
            return_stmt(),
            if_stmt(ident("c"), expr_stmt(ident("foo"))),
        ];
        let prog = program_with_function(body, Some("block.1"));
        let (out, _contribs, changed, _) = run_pass(prog);
        assert!(!changed, "must not truncate past a compound statement");
        let new_block = extract_function_body(&out);
        assert_eq!(new_block.body.len(), 2);
    }

    #[test]
    fn drops_tail_of_let_declarations_after_terminator() {
        // `let`/`const` are block-scoped (not hoisted), so a tail of only
        // `let`/`const` declarations (here a `let`) IS safe to drop. Uses a
        // `var`-free declaration to confirm the whitelist admits non-`var`
        // declarations.
        let body = vec![return_stmt(), let_decl_stmt("k")];
        let prog = program_with_function(body, Some("block.1"));
        let (out, _contribs, changed, _) = run_pass(prog);
        assert!(changed, "a let-only tail after a terminator is safe to drop");
        let new_block = extract_function_body(&out);
        assert_eq!(new_block.body.len(), 1, "only the terminator should remain");
    }

    #[test]
    fn drops_no_statements_when_no_return() {
        let body = vec![expr_stmt(ident("x")), expr_stmt(ident("y"))];
        let prog = program_with_function(body, Some("block.1"));
        let (out, _contribs, changed, _) = run_pass(prog);
        assert!(!changed);
        let new_block = extract_function_body(&out);
        assert_eq!(new_block.body.len(), 2);
    }

    // ---------------- empty-statement removal ------------------

    #[test]
    fn drops_empty_statements_from_block() {
        // { x; ; y; ; ; } → { x; y; }
        let body = vec![
            expr_stmt(ident("x")),
            empty_stmt(),
            expr_stmt(ident("y")),
            empty_stmt(),
            empty_stmt(),
        ];
        let prog = program_with_function(body, Some("block.1"));
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        assert!(
            contribs.iter().any(|c| c.tag == "removed-empty-statement"),
            "expected removed-empty-statement contribution; got {:?}",
            contribs
        );
        let new_block = extract_function_body(&out);
        assert_eq!(new_block.body.len(), 2, "expected 2 statements; got {:?}", new_block.body);
    }

    // --- CLOC24: `debugger` stripping -----------------------------------

    #[test]
    fn strips_debugger_statement_from_block() {
        // { x; debugger; y; } → { x; y; }
        let body = vec![
            expr_stmt(ident("x")),
            debugger_stmt(),
            expr_stmt(ident("y")),
        ];
        let prog = program_with_function(body, Some("block.1"));
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed, "stripping a debugger must mark the program changed");
        assert!(
            contribs.iter().any(|c| c.tag == "removed-debugger"),
            "expected a removed-debugger contribution; got {:?}",
            contribs
        );
        let new_block = extract_function_body(&out);
        assert_eq!(
            new_block.body.len(),
            2,
            "the debugger should be gone, leaving x; y;; got {:?}",
            new_block.body
        );
        assert!(
            !new_block.body.iter().any(is_debugger_statement),
            "no debugger statement should remain; got {:?}",
            new_block.body
        );
    }

    #[test]
    fn strips_top_level_debugger_statement() {
        // top-level: x; debugger; → x;
        let prog = program().with_body(vec![
            ProgramItem::Statement(expr_stmt(ident("x"))),
            ProgramItem::Statement(debugger_stmt()),
        ]);
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed, "stripping a top-level debugger must mark changed");
        assert!(
            contribs.iter().any(|c| c.tag == "removed-debugger"),
            "expected a removed-debugger contribution; got {:?}",
            contribs
        );
        assert_eq!(
            out.body.len(),
            1,
            "only the top-level debugger should be removed; got {:?}",
            out.body
        );
        assert!(
            !out.body.iter().any(is_debugger_program_item),
            "no top-level debugger should remain; got {:?}",
            out.body
        );
    }

    #[test]
    fn block_of_only_debuggers_becomes_empty() {
        // { debugger; debugger; } → { }
        let body = vec![debugger_stmt(), debugger_stmt()];
        let prog = program_with_function(body, Some("block.1"));
        let (out, _contribs, changed, _) = run_pass(prog);
        assert!(changed);
        let new_block = extract_function_body(&out);
        assert!(
            new_block.body.is_empty(),
            "both debuggers should be gone, leaving an empty block; got {:?}",
            new_block.body
        );
    }

    #[test]
    fn preserves_braceless_if_consequent_debugger() {
        // `if (x) debugger;` — the debugger is the consequent, NOT in a
        // statement list, so the list-scoped sweep leaves it intact. This pins
        // the documented limitation (see `dce_tagged_statement`'s leaf arm).
        let body = vec![if_stmt(ident("x"), debugger_stmt())];
        let prog = program_with_function(body, Some("block.1"));
        let (out, contribs, _changed, _) = run_pass(prog);
        assert!(
            !contribs.iter().any(|c| c.tag == "removed-debugger"),
            "a brace-less consequent debugger must NOT be swept; got {:?}",
            contribs
        );
        let new_block = extract_function_body(&out);
        let Statement::Tagged(TaggedStatement::IfStatement(if_s)) = &new_block.body[0] else {
            panic!("expected the if statement to survive; got {:?}", new_block.body);
        };
        assert!(
            is_debugger_statement(&if_s.consequent),
            "the consequent debugger should be preserved; got {:?}",
            if_s.consequent
        );
    }

    #[test]
    fn handles_both_categories_in_one_block() {
        // { x; ;  return; y; ; }
        // → drop dead-after-return (`y;` and the trailing `;`)
        // → drop empties (`;` between x and return)
        // → final: { x; return; }
        let body = vec![
            expr_stmt(ident("x")),
            empty_stmt(),
            return_stmt(),
            expr_stmt(ident("y")),
            empty_stmt(),
        ];
        let prog = program_with_function(body, Some("block.1"));
        let (out, contribs, _, _) = run_pass(prog);
        // Two categories of drops → two contributions.
        assert!(
            contribs.iter().any(|c| c.tag == "removed-dead-code"),
            "expected removed-dead-code; got {:?}",
            contribs
        );
        assert!(
            contribs.iter().any(|c| c.tag == "removed-empty-statement"),
            "expected removed-empty-statement; got {:?}",
            contribs
        );
        let new_block = extract_function_body(&out);
        assert_eq!(new_block.body.len(), 2);
    }

    // ---------------- nested blocks ---------------------------

    #[test]
    fn recurses_into_nested_blocks_and_flattens() {
        // { x; { return; y; } z; }
        //
        // Step 1 (recurse): inner `{ return; y; }` → `{ return; }`
        //                   (drop dead-after-return)
        // Step 2 (flatten, CLOC12.19 / gap-010): inner block has no
        //         scope-bound decls, splice its body into outer:
        //         `{ x; return; z; }`
        // Step 3 (dead-after-return on outer): drop `z`:
        //         `{ x; return; }`
        //
        // So the outer body ends with 2 statements (was 3 pre-fold).
        let inner_block = BlockStatement {
            cv: Some("inner.1".to_string()),
            body: vec![return_stmt(), expr_stmt(ident("y"))],
        };
        let body = vec![
            expr_stmt(ident("x")),
            Statement::block_statement(inner_block),
            expr_stmt(ident("z")),
        ];
        let prog = program_with_function(body, Some("outer.1"));
        let (out, _contribs, changed, _) = run_pass(prog);
        assert!(changed);
        let outer = extract_function_body(&out);
        assert_eq!(
            outer.body.len(),
            2,
            "expected outer body = [x; return;] after flatten+drop; got {:?}",
            outer.body
        );
        // Second statement is `return;`.
        assert!(
            matches!(
                &outer.body[1],
                Statement::Tagged(TaggedStatement::ReturnStatement(_))
            ),
            "expected ReturnStatement at outer.body[1]; got {:?}",
            outer.body[1]
        );
    }

    // ---------------- untraced ---------------------------------

    #[test]
    fn untraced_mode_drops_silently() {
        // Same as drops_statements_after_return but with cv: None.
        let body = vec![
            expr_stmt(ident("x")),
            return_stmt(),
            expr_stmt(ident("y")),
        ];
        let block = BlockStatement {
            cv: None,
            body,
        };
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
        let prog = untraced_program().with_body(vec![ProgramItem::Declaration(fdecl)]);
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        assert!(
            contribs.is_empty(),
            "untraced should emit no contributions; got {:?}",
            contribs
        );
        let new_block = extract_function_body(&out);
        assert_eq!(new_block.body.len(), 2);
    }

    // ---------------- pipeline integration --------------------

    #[test]
    fn pipeline_solo_runs_cleanly() {
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(DcePass::new()));
        let mut cv = CVLog::new(true);
        let out: PipelineOutput = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");
        assert_eq!(out.execution_order, vec!["dce".to_string()]);
        // The pipeline now iterates to a fixed point; a non-changing
        // solo pass converges in one sweep, so the old "not-yet-iterated"
        // note is gone.
        assert!(!out
            .diagnostics
            .iter()
            .any(|d| d.group.0 == "pipeline.fixed-point-not-yet-iterated"));
    }

    #[test]
    fn full_canonical_pipeline_constant_fold_then_fcf_then_dce() {
        // `if (1 < 2) { z; }` after the full canonical chain:
        // constant-fold collapses `1 < 2 → true`;
        // fold-control-flow collapses `if (true) {z;}` → block `{z;}`
        // (or just the consequent statement, depending on which
        // body wrapper fold-control-flow chose).
        // dce cleans up.
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: Expression::BinaryExpression(BinaryExpression {
                cv: Some("cmp.1".to_string()),
                operator: BinaryOperator::Lt,
                left: Box::new(num(1.0)),
                right: Box::new(num(2.0)),
            }),
            consequent: Box::new(expr_stmt(ident("z"))),
            alternate: None,
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);

        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(ConstantFoldPass::new()));
        pipeline.add(Box::new(FoldControlFlowPass::new()));
        pipeline.add(Box::new(DcePass::new()));
        let mut cv = CVLog::new(true);
        let out = pipeline
            .run(prog, &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        assert_eq!(
            out.execution_order,
            vec![
                "constant-fold".to_string(),
                "fold-control-flow".to_string(),
                "dce".to_string(),
            ]
        );
        // After fold-control-flow's collapse of if(true){z;} the
        // result is just the consequent statement holding `z`.
        let item = &out.program.body[0];
        let ProgramItem::Statement(s) = item else {
            panic!("expected Statement at body[0]; got {:?}", item);
        };
        match s {
            Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
                match &es.expression {
                    Expression::Identifier(i) => assert_eq!(i.name, "z"),
                    other => panic!("expected ident(z); got {:?}", other),
                }
            }
            other => panic!("expected ExpressionStatement(z); got {:?}", other),
        }
    }

    #[test]
    fn pipeline_with_if_false_then_dce_cleans_empty_statement() {
        // `function f() { if (false) {x;} return; y; }`
        // Step 1: constant-fold — no value-level work needed.
        // Step 2: fold-control-flow — collapses `if (false) {x;}`
        //         (no alternate) to EmptyStatement. Also drops
        //         `y;` after `return;` inside the block (fold-
        //         control-flow's own dead-after-return logic).
        //         End result of step 2: { ;  return; }
        // Step 3: dce — removes the EmptyStatement.
        //         End result: { return; }
        let func_body = vec![
            Statement::if_statement(IfStatement {
                cv: Some("if.1".to_string()),
                test: boolean(false),
                consequent: Box::new(expr_stmt(ident("x"))),
                alternate: None,
            }),
            return_stmt(),
            expr_stmt(ident("y")),
        ];
        let prog = program_with_function(func_body, Some("fn.body"));

        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(ConstantFoldPass::new()));
        pipeline.add(Box::new(FoldControlFlowPass::new()));
        pipeline.add(Box::new(DcePass::new()));
        let mut cv = CVLog::new(true);
        let out = pipeline
            .run(prog, &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        let final_block = extract_function_body(&out.program);
        // The block should just hold `return;` after the full
        // canonical chain.
        assert_eq!(
            final_block.body.len(),
            1,
            "expected just `return;` after fold/fcf/dce; got {:?}",
            final_block.body
        );
        assert!(matches!(
            &final_block.body[0],
            Statement::Tagged(TaggedStatement::ReturnStatement(_))
        ));
    }

    // =====================================================================
    // gap-014 step 2 / CLOC12.34 — empty-switch elimination
    // =====================================================================

    fn switch_stmt(disc: Expression, cases: Vec<SwitchCase>) -> Statement {
        Statement::switch_statement(SwitchStatement {
            cv: Some("sw.1".to_string()),
            discriminant: disc,
            cases,
        })
    }

    fn case_empty(test: Option<Expression>) -> SwitchCase {
        SwitchCase {
            cv: None,
            test,
            consequent: vec![],
        }
    }

    /// `function f() { switch (1) {} }` → `function f() {}`.
    /// Empty switch with literal discriminant collapses; the
    /// resulting EmptyStatement is then dropped by the block
    /// walker, so the function body ends up empty.
    #[test]
    fn empty_switch_with_literal_discriminant_drops_entirely() {
        let body = vec![switch_stmt(num(1.0), vec![])];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, contribs, changed, _) = run_pass(prog);
        let block = extract_function_body(&out);
        assert!(block.body.is_empty(), "expected empty body; got {:?}", block.body);
        assert!(changed);
        assert!(
            contribs.iter().any(|c| c.tag == "switch_eliminated"),
            "expected switch_eliminated contribution"
        );
    }

    /// `function f() { switch (1) { case 2: ; default: ; } }` →
    /// `function f() {}`. All cases empty, literal tests, literal
    /// discriminant — drops.
    #[test]
    fn empty_switch_with_pure_cases_drops_entirely() {
        let cases = vec![
            case_empty(Some(num(2.0))),
            case_empty(None), // default
        ];
        let body = vec![switch_stmt(num(1.0), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, contribs, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        assert!(block.body.is_empty(), "expected empty body; got {:?}", block.body);
        assert!(contribs.iter().any(|c| c.tag == "switch_eliminated"));
    }

    /// Conservative bail: Identifier discriminant might TDZ-throw
    /// for an uninitialised `let` / `const`. Keep the switch.
    #[test]
    fn empty_switch_with_identifier_discriminant_keeps_switch() {
        let body = vec![switch_stmt(ident("x"), vec![])];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, contribs, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        // SwitchStatement preserved.
        assert!(matches!(
            &block.body[0],
            Statement::Tagged(TaggedStatement::SwitchStatement(_))
        ));
        assert!(!contribs.iter().any(|c| c.tag == "switch_eliminated"));
    }

    /// Non-empty consequent keeps the switch even with a pure
    /// discriminant — the consequent's statements have effect
    /// potential we don't analyse.
    #[test]
    fn switch_with_non_empty_consequent_keeps_switch() {
        let cases = vec![SwitchCase {
            cv: None,
            test: Some(num(1.0)),
            consequent: vec![expr_stmt(ident("y"))],
        }];
        let body = vec![switch_stmt(num(1.0), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        assert!(matches!(
            &block.body[0],
            Statement::Tagged(TaggedStatement::SwitchStatement(_))
        ));
    }

    /// Identifier case-test (not pure under TDZ) keeps the
    /// switch even though discriminant is pure and consequents
    /// are empty.
    #[test]
    fn empty_switch_with_identifier_case_test_keeps_switch() {
        let cases = vec![case_empty(Some(ident("k")))];
        let body = vec![switch_stmt(num(1.0), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        assert!(matches!(
            &block.body[0],
            Statement::Tagged(TaggedStatement::SwitchStatement(_))
        ));
    }

    /// Boolean / Null discriminants are pure too.
    #[test]
    fn empty_switch_with_boolean_discriminant_drops() {
        let body = vec![switch_stmt(boolean(true), vec![])];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        assert!(block.body.is_empty());
    }

    // =====================================================================
    // gap-014 step 3 / CLOC12.35 — drop-after-break in case consequents
    // =====================================================================

    fn break_stmt() -> Statement {
        Statement::break_statement(
            coding_adventures_javascript_ast::BreakStatement { cv: None, label: None },
        )
    }

    fn throw_stmt(arg: Expression) -> Statement {
        Statement::throw_statement(
            coding_adventures_javascript_ast::ThrowStatement { cv: None, argument: arg },
        )
    }

    /// Helper — extract the unique SwitchStatement from a function
    /// body so we can pattern-match on it.
    fn extract_switch(prog: &Program) -> &SwitchStatement {
        let block = extract_function_body(prog);
        match &block.body[0] {
            Statement::Tagged(TaggedStatement::SwitchStatement(s)) => s,
            other => panic!("expected SwitchStatement; got {:?}", other),
        }
    }

    /// `switch (x) { case 1: a; break; dead; }` →
    /// `switch (x) { case 1: a; break; }`. The case keeps `a` and
    /// `break;`; the trailing `dead;` is dropped.
    #[test]
    fn drop_after_break_in_case_consequent() {
        let cases = vec![SwitchCase {
            cv: Some("c.1".to_string()),
            test: Some(num(1.0)),
            consequent: vec![
                expr_stmt(ident("a")),
                break_stmt(),
                expr_stmt(ident("dead")),
            ],
        }];
        let body = vec![switch_stmt(ident("x"), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, contribs, changed, _) = run_pass(prog);
        let sw = extract_switch(&out);
        let cs = &sw.cases[0];
        assert_eq!(cs.consequent.len(), 2, "expected 2 stmts; got {:?}", cs.consequent);
        assert!(matches!(
            &cs.consequent[1],
            Statement::Tagged(TaggedStatement::BreakStatement(_))
        ));
        assert!(changed);
        assert!(contribs.iter().any(|c| c.tag == "removed-dead-code-in-case"));
    }

    /// `return` inside a case body is also a terminator.
    #[test]
    fn drop_after_return_in_case_consequent() {
        let cases = vec![SwitchCase {
            cv: None,
            test: Some(num(1.0)),
            consequent: vec![return_stmt(), expr_stmt(ident("dead"))],
        }];
        let body = vec![switch_stmt(ident("x"), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let sw = extract_switch(&out);
        assert_eq!(sw.cases[0].consequent.len(), 1);
    }

    /// `throw e;` inside a case body is also a terminator.
    #[test]
    fn drop_after_throw_in_case_consequent() {
        let cases = vec![SwitchCase {
            cv: None,
            test: Some(num(1.0)),
            consequent: vec![throw_stmt(num(1.0)), expr_stmt(ident("dead"))],
        }];
        let body = vec![switch_stmt(ident("x"), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let sw = extract_switch(&out);
        assert_eq!(sw.cases[0].consequent.len(), 1);
    }

    /// Default case also gets the truncate treatment.
    #[test]
    fn drop_after_break_in_default_consequent() {
        let cases = vec![SwitchCase {
            cv: None,
            test: None,
            consequent: vec![
                expr_stmt(ident("a")),
                break_stmt(),
                expr_stmt(ident("dead")),
            ],
        }];
        let body = vec![switch_stmt(ident("x"), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let sw = extract_switch(&out);
        assert_eq!(sw.cases[0].consequent.len(), 2);
    }

    /// Per-case truncation is independent — one case has dead code,
    /// the other doesn't.
    #[test]
    fn drop_after_break_applies_per_case() {
        let cases = vec![
            SwitchCase {
                cv: None,
                test: Some(num(1.0)),
                consequent: vec![
                    expr_stmt(ident("a")),
                    break_stmt(),
                    expr_stmt(ident("dead")),
                ],
            },
            SwitchCase {
                cv: None,
                test: Some(num(2.0)),
                consequent: vec![expr_stmt(ident("b"))],
            },
        ];
        let body = vec![switch_stmt(ident("x"), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let sw = extract_switch(&out);
        assert_eq!(sw.cases[0].consequent.len(), 2); // truncated
        assert_eq!(sw.cases[1].consequent.len(), 1); // untouched
    }

    /// `continue` is NOT a case terminator — it refers to an
    /// enclosing loop, not the switch. Conservative bail.
    #[test]
    fn continue_in_case_consequent_keeps_following_statements() {
        let cont = Statement::continue_statement(
            coding_adventures_javascript_ast::ContinueStatement { cv: None, label: None },
        );
        let cases = vec![SwitchCase {
            cv: None,
            test: Some(num(1.0)),
            consequent: vec![cont, expr_stmt(ident("y"))],
        }];
        let body = vec![switch_stmt(ident("x"), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let sw = extract_switch(&out);
        assert_eq!(sw.cases[0].consequent.len(), 2);
    }

    /// Case with no terminator is left alone.
    #[test]
    fn case_with_no_terminator_unchanged() {
        let cases = vec![SwitchCase {
            cv: None,
            test: Some(num(1.0)),
            consequent: vec![expr_stmt(ident("a")), expr_stmt(ident("b"))],
        }];
        let body = vec![switch_stmt(ident("x"), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let sw = extract_switch(&out);
        assert_eq!(sw.cases[0].consequent.len(), 2);
    }

    // =====================================================================
    // gap-014 step 4 / CLOC12.36 — constant-discriminant collapse
    // =====================================================================

    /// `function f() { switch (1) { case 1: a; break; } }` →
    /// `function f() { a; }`. The matched case body keeps `a`,
    /// the trailing `break;` is stripped, the rest of the switch
    /// disappears. After block-flattening, the resulting body
    /// becomes a single-statement function.
    #[test]
    fn switch_with_literal_disc_matching_case_collapses() {
        let cases = vec![SwitchCase {
            cv: None,
            test: Some(num(1.0)),
            consequent: vec![expr_stmt(ident("a")), break_stmt()],
        }];
        let body = vec![switch_stmt(num(1.0), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, contribs, changed, _) = run_pass(prog);
        let block = extract_function_body(&out);
        // After collapse + block flatten: just `a;`.
        assert_eq!(block.body.len(), 1, "expected 1 stmt; got {:?}", block.body);
        assert!(matches!(
            &block.body[0],
            Statement::Tagged(TaggedStatement::ExpressionStatement(_))
        ));
        assert!(changed);
        assert!(
            contribs
                .iter()
                .any(|c| c.tag == "switch_collapsed_to_matched_case")
        );
    }

    /// `function f() { switch (1) { case 2: a; break; default: b; break; } }`
    /// → `function f() { b; }`. No case matches `1`, so the
    /// `default:` runs; trailing break stripped.
    #[test]
    fn switch_with_literal_disc_no_match_uses_default() {
        let cases = vec![
            SwitchCase {
                cv: None,
                test: Some(num(2.0)),
                consequent: vec![expr_stmt(ident("a")), break_stmt()],
            },
            SwitchCase {
                cv: None,
                test: None, // default
                consequent: vec![expr_stmt(ident("b")), break_stmt()],
            },
        ];
        let body = vec![switch_stmt(num(1.0), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        assert_eq!(block.body.len(), 1);
    }

    /// `function f() { switch (1) { case 2: a; break; } }` →
    /// `function f() {}`. No match, no default; nothing runs.
    #[test]
    fn switch_with_literal_disc_no_match_no_default_drops() {
        let cases = vec![SwitchCase {
            cv: None,
            test: Some(num(2.0)),
            consequent: vec![expr_stmt(ident("a")), break_stmt()],
        }];
        let body = vec![switch_stmt(num(1.0), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, contribs, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        assert!(block.body.is_empty());
        assert!(contribs.iter().any(|c| c.tag == "switch_collapsed_no_match"));
    }

    /// Matched case ending with `return;` (not break) → return
    /// stays in the collapsed body. The function would return
    /// early; that's preserved.
    #[test]
    fn switch_collapse_with_return_terminator_preserves_return() {
        let cases = vec![SwitchCase {
            cv: None,
            test: Some(num(1.0)),
            consequent: vec![expr_stmt(ident("a")), return_stmt()],
        }];
        let body = vec![switch_stmt(num(1.0), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        // Two statements: `a;` then `return;`. Block flattening
        // hoisted them out of the inner switch's collapsed block.
        // DCE's own dead-after-return drop applies *within* the
        // outer block, so it stays as 2.
        assert_eq!(block.body.len(), 2, "expected 2 stmts; got {:?}", block.body);
        assert!(matches!(
            &block.body[1],
            Statement::Tagged(TaggedStatement::ReturnStatement(_))
        ));
    }

    /// String discriminant matching case test.
    #[test]
    fn switch_collapse_string_discriminant() {
        let s_lit = |v: &str| Expression::StringLiteral(
            coding_adventures_javascript_ast::StringLiteral {
                cv: None,
                value: v.to_string(),
                raw: format!("\"{}\"", v),
            },
        );
        let cases = vec![
            SwitchCase {
                cv: None,
                test: Some(s_lit("a")),
                consequent: vec![expr_stmt(ident("ax")), break_stmt()],
            },
            SwitchCase {
                cv: None,
                test: Some(s_lit("b")),
                consequent: vec![expr_stmt(ident("bx")), break_stmt()],
            },
        ];
        let body = vec![switch_stmt(s_lit("b"), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        assert_eq!(block.body.len(), 1);
        if let Statement::Tagged(TaggedStatement::ExpressionStatement(es)) = &block.body[0] {
            if let Expression::Identifier(i) = &es.expression {
                assert_eq!(i.name, "bx");
            } else {
                panic!("expected Identifier(bx) inside expr-stmt");
            }
        } else {
            panic!("expected ExpressionStatement; got {:?}", block.body[0]);
        }
    }

    /// Bail when matched-case consequent doesn't terminate —
    /// fall-through would happen and we don't model it.
    #[test]
    fn switch_with_literal_disc_no_terminator_keeps_switch() {
        let cases = vec![
            SwitchCase {
                cv: None,
                test: Some(num(1.0)),
                consequent: vec![expr_stmt(ident("a"))], // no break/return/throw
            },
            SwitchCase {
                cv: None,
                test: Some(num(2.0)),
                consequent: vec![expr_stmt(ident("b")), break_stmt()],
            },
        ];
        let body = vec![switch_stmt(num(1.0), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        assert!(matches!(
            &block.body[0],
            Statement::Tagged(TaggedStatement::SwitchStatement(_))
        ));
    }

    /// Cross-type test mismatch — discriminant `1` doesn't strict-
    /// equal `"1"`. The `case "1":` is skipped, default runs.
    #[test]
    fn switch_collapse_cross_type_test_does_not_match() {
        let s_lit = |v: &str| Expression::StringLiteral(
            coding_adventures_javascript_ast::StringLiteral {
                cv: None,
                value: v.to_string(),
                raw: format!("\"{}\"", v),
            },
        );
        let cases = vec![
            SwitchCase {
                cv: None,
                test: Some(s_lit("1")),
                consequent: vec![expr_stmt(ident("string_match")), break_stmt()],
            },
            SwitchCase {
                cv: None,
                test: None,
                consequent: vec![expr_stmt(ident("default_ran")), break_stmt()],
            },
        ];
        let body = vec![switch_stmt(num(1.0), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        assert_eq!(block.body.len(), 1);
        if let Statement::Tagged(TaggedStatement::ExpressionStatement(es)) = &block.body[0] {
            if let Expression::Identifier(i) = &es.expression {
                assert_eq!(i.name, "default_ran");
            } else {
                panic!("expected Identifier inside expr-stmt");
            }
        }
    }

    /// Identifier discriminant: bail. Conservative.
    #[test]
    fn switch_collapse_identifier_discriminant_keeps_switch() {
        let cases = vec![SwitchCase {
            cv: None,
            test: Some(num(1.0)),
            consequent: vec![expr_stmt(ident("a")), break_stmt()],
        }];
        let body = vec![switch_stmt(ident("x"), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        assert!(matches!(
            &block.body[0],
            Statement::Tagged(TaggedStatement::SwitchStatement(_))
        ));
    }

    /// **Bug fix from security review.** The classic "share body"
    /// pattern `case 1: case 2: body; break;` has `case 1` with
    /// an EMPTY consequent — control falls through to `case 2`'s
    /// `body; break;`. We don't model fall-through, so we must
    /// bail and keep the switch intact rather than wrongly
    /// collapsing to `{}` (which would drop `body`).
    #[test]
    fn switch_collapse_empty_matched_case_keeps_switch_due_to_fallthrough() {
        let cases = vec![
            SwitchCase {
                cv: None,
                test: Some(num(1.0)),
                consequent: vec![], // falls through to case 2
            },
            SwitchCase {
                cv: None,
                test: Some(num(2.0)),
                consequent: vec![expr_stmt(ident("body")), break_stmt()],
            },
        ];
        let body = vec![switch_stmt(num(1.0), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        // Switch preserved; we don't drop `body`.
        assert!(matches!(
            &block.body[0],
            Statement::Tagged(TaggedStatement::SwitchStatement(_))
        ));
    }

    /// **Bug fix from security review.** A trailing `break label;`
    /// (labeled break) exits the switch AND transfers control to
    /// after the labeled statement (typically an outer loop).
    /// Stripping it would silently keep the outer loop running.
    /// `strip_trailing_break` must only strip UNLABELED breaks.
    #[test]
    fn switch_collapse_preserves_trailing_labeled_break() {
        let labeled_break = Statement::break_statement(
            coding_adventures_javascript_ast::BreakStatement {
                cv: None,
                label: Some(coding_adventures_javascript_ast::Identifier {
                    cv: None,
                    name: "outer".to_string(),
                }),
            },
        );
        let cases = vec![SwitchCase {
            cv: None,
            test: Some(num(1.0)),
            consequent: vec![expr_stmt(ident("a")), labeled_break],
        }];
        let body = vec![switch_stmt(num(1.0), cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        // After collapse + block-flatten, body is `a; break outer;`
        // — the labeled break must still be there.
        assert_eq!(block.body.len(), 2, "expected 2 stmts; got {:?}", block.body);
        if let Statement::Tagged(TaggedStatement::BreakStatement(b)) = &block.body[1] {
            assert!(b.label.is_some(), "labeled break must be preserved");
            assert_eq!(b.label.as_ref().unwrap().name, "outer");
        } else {
            panic!("expected BreakStatement at body[1]; got {:?}", block.body[1]);
        }
    }

    /// NaN never matches per ECMAScript §IsStrictlyEqual.
    /// Conservative bail: discriminant `NaN` keeps the switch.
    #[test]
    fn switch_collapse_nan_discriminant_keeps_switch() {
        let nan = Expression::NumericLiteral(NumericLiteral {
            cv: None,
            value: f64::NAN,
            raw: "NaN".to_string(),
        });
        let cases = vec![SwitchCase {
            cv: None,
            test: Some(num(1.0)),
            consequent: vec![expr_stmt(ident("a")), break_stmt()],
        }];
        let body = vec![switch_stmt(nan, cases)];
        let prog = program_with_function(body, Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        // NaN doesn't strict-equal anything → no match. Without a
        // default case, the step 4 logic returns EmptyStatement
        // (no observable behaviour). With NaN we don't reach there
        // because strict_equal_leaves bails on NaN — but
        // pick_matching_case still returns None (no test
        // matches), and no default exists, so → EmptyStatement
        // path runs.
        //
        // Net result: empty function body. The earlier "bail on
        // NaN" doc is a soundness note for cases where a
        // theoretical case test could be NaN (which can't happen
        // through is_pure_leaf because we'd already have flagged
        // it).
        assert!(block.body.is_empty());
    }

    // ---- try / catch / finally (CLOC19) -----------------------

    /// Helper: pull the single `TryStatement` out of a function body.
    fn extract_try(block: &BlockStatement) -> &TryStatement {
        let Statement::Tagged(TaggedStatement::TryStatement(t)) = &block.body[0] else {
            panic!("expected a TryStatement at body[0], got {:?}", block.body[0]);
        };
        t
    }

    #[test]
    fn dce_recurses_into_catch_body_and_drops_dead_after_return() {
        // try { } catch (e) { return; foo(); }  — the `foo()` after the
        // `return` inside the catch body is unreachable. DCE must recurse
        // into the handler block and truncate it, while leaving the catch
        // param `e` untouched.
        let try_stmt = Statement::try_statement(TryStatement {
            cv: None,
            block: BlockStatement {
                cv: None,
                body: vec![],
            },
            handler: Some(CatchClause {
                cv: None,
                param: Some(Identifier {
                    cv: None,
                    name: "e".to_string(),
                }),
                body: BlockStatement {
                    cv: None,
                    body: vec![return_stmt(), expr_stmt(ident("foo"))],
                },
            }),
            finalizer: None,
        });
        let prog = program_with_function(vec![try_stmt], Some("fn.1"));
        let (out, _, changed, _) = run_pass(prog);
        assert!(changed, "DCE should report a change (dropped dead foo())");

        let block = extract_function_body(&out);
        let t = extract_try(block);
        let handler = t.handler.as_ref().expect("handler preserved");
        assert_eq!(
            handler.param.as_ref().map(|p| p.name.as_str()),
            Some("e"),
            "catch param must be preserved verbatim",
        );
        assert_eq!(
            handler.body.body.len(),
            1,
            "dead-after-return inside catch body must be truncated to just the return",
        );
        assert!(
            matches!(
                &handler.body.body[0],
                Statement::Tagged(TaggedStatement::ReturnStatement(_))
            ),
            "the surviving statement must be the return",
        );
    }

    #[test]
    fn dce_does_not_treat_try_as_a_terminator() {
        // try { } finally { }  followed by a reachable statement: the
        // `after()` call must survive, because a `try` can catch and
        // continue — it is NOT an unconditional terminator.
        let try_stmt = Statement::try_statement(TryStatement {
            cv: None,
            block: BlockStatement {
                cv: None,
                body: vec![],
            },
            handler: None,
            finalizer: Some(BlockStatement {
                cv: None,
                body: vec![],
            }),
        });
        let prog = program_with_function(vec![try_stmt, expr_stmt(ident("after"))], Some("fn.1"));
        let (out, _, _, _) = run_pass(prog);
        let block = extract_function_body(&out);
        assert_eq!(
            block.body.len(),
            2,
            "the statement after a try/finally must remain reachable; got {:?}",
            block.body,
        );
    }

    // ---------------- empty-`if` elimination ------------------------------

    fn empty_block_stmt() -> Statement {
        Statement::block_statement(BlockStatement { cv: None, body: vec![] })
    }
    fn if_full(test: Expression, consequent: Statement, alternate: Option<Statement>) -> Statement {
        Statement::if_statement(IfStatement {
            cv: None,
            test,
            consequent: Box::new(consequent),
            alternate: alternate.map(Box::new),
        })
    }
    fn member(obj: Expression, prop: &str) -> Expression {
        Expression::MemberExpression(MemberExpression {
            cv: None,
            object: Box::new(obj),
            property: Box::new(ident(prop)),
            computed: false,
        })
    }
    fn call0(name: &str) -> Expression {
        Expression::CallExpression(coding_adventures_javascript_ast::CallExpression {
            cv: None,
            callee: Box::new(ident(name)),
            arguments: vec![],
        })
    }
    fn not(arg: Expression) -> Expression {
        Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: UnaryOperator::Not,
            prefix: true,
            argument: Box::new(arg),
        })
    }

    #[test]
    fn empty_if_with_side_effect_free_test_is_removed() {
        // `if(x){}`, `if(x.y){}`, `if(true){}`, `if(!x){}` — the test is
        // side-effect-free and both branches empty, so the whole `if` is dead
        // and drops (collapses to `;`, then the program sweep removes it).
        for test in [ident("x"), member(ident("x"), "y"), boolean(true), not(ident("x"))] {
            let prog = program()
                .with_body(vec![ProgramItem::Statement(if_full(test.clone(), empty_block_stmt(), None))]);
            let (out, _c, changed, _) = run_pass(prog);
            assert!(changed, "empty if with pure test should mark changed: {test:?}");
            assert!(out.body.is_empty(), "the empty if should be removed; got {:?}", out.body);
        }
    }

    #[test]
    fn empty_if_else_both_empty_is_removed() {
        // `if(x){}else{}` — both branches empty, pure test → removed.
        let prog = program().with_body(vec![ProgramItem::Statement(if_full(
            ident("x"),
            empty_block_stmt(),
            Some(empty_block_stmt()),
        ))]);
        let (out, _c, changed, _) = run_pass(prog);
        assert!(changed);
        assert!(out.body.is_empty(), "if/else with both branches empty should drop");
    }

    #[test]
    fn empty_if_removed_but_neighbours_kept() {
        // The removal must not disturb sibling statements.
        let prog = program().with_body(vec![
            ProgramItem::Statement(expr_stmt(call0("before"))),
            ProgramItem::Statement(if_full(ident("x"), empty_block_stmt(), None)),
            ProgramItem::Statement(expr_stmt(call0("after"))),
        ]);
        let (out, _c, changed, _) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 2, "only the empty if should be removed; got {:?}", out.body);
    }

    #[test]
    fn empty_if_with_call_test_becomes_expression_statement() {
        // `if(f()){}` and `if(f()){}else{}` — the call may have side effects, so
        // the `if` wrapper is dead but the call must still RUN: it survives as
        // an expression statement `f();`.
        for alt in [None, Some(empty_block_stmt())] {
            let prog = program().with_body(vec![ProgramItem::Statement(if_full(
                call0("f"),
                empty_block_stmt(),
                alt,
            ))]);
            let (out, _c, changed, _) = run_pass(prog);
            assert!(changed, "impure-call empty if should mark changed");
            assert_eq!(out.body.len(), 1, "the call must survive as one statement");
            let ProgramItem::Statement(Statement::Tagged(TaggedStatement::ExpressionStatement(
                es,
            ))) = &out.body[0]
            else {
                panic!("expected an ExpressionStatement; got {:?}", out.body[0])
            };
            assert!(
                matches!(&es.expression, Expression::CallExpression(_)),
                "the expression statement should wrap the call test"
            );
        }
    }

    #[test]
    fn empty_if_with_non_call_impure_test_is_kept() {
        // `if(!f()){}` — the test is impure (the inner call runs) but is NOT a
        // bare `CallExpression`. Closure would drop the discarded `!` (→ `f();`),
        // a separate transform, so we DECLINE and keep the `if` intact rather
        // than emit a non-canonical `!f();`.
        let prog = program().with_body(vec![ProgramItem::Statement(if_full(
            not(call0("f")),
            empty_block_stmt(),
            None,
        ))]);
        let (out, _c, _changed, _) = run_pass(prog);
        assert_eq!(out.body.len(), 1, "non-call impure-test empty if must be kept");
        assert!(matches!(
            &out.body[0],
            ProgramItem::Statement(Statement::Tagged(TaggedStatement::IfStatement(_)))
        ));
    }

    #[test]
    fn if_with_non_empty_consequent_is_kept() {
        // `if(x)g();` — the consequent does work, so nothing is removed.
        let prog = program()
            .with_body(vec![ProgramItem::Statement(if_full(ident("x"), expr_stmt(call0("g")), None))]);
        let (out, _c, _changed, _) = run_pass(prog);
        assert_eq!(out.body.len(), 1, "non-empty-consequent if must be kept");
    }
}
