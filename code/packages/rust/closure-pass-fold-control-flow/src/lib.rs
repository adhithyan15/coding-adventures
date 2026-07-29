//! Control-flow folding pass for the Closure Compiler clone.
//!
//! Sits between `constant-fold` and `dce` in the CLOC06 canonical
//! pass set. Where `constant-fold` collapses pure value-level
//! expressions (`2 + 2 → 4`, `"a" + "b" → "ab"`), `fold-control-flow`
//! does the same job for **control flow shapes** whose decision is
//! statically known:
//!
//! ```text
//! if (false) { A } else { B }                  →  B
//! if (true)  { A } else { B }                  →  A
//! if (false) { A }                             →  ;             (EmptyStatement)
//! while (false) { ... }                        →  ;
//! function f() { return 1; A; B; }             →  function f() { return 1; }
//! true  ? a : b                                →  a
//! ```
//!
//! These rewrites typically open *new* opportunities for DCE — a
//! `let foo = 1; if (false) { use(foo); }` becomes `let foo = 1;`,
//! and then `closure-pass-remove-unused-vars` can drop the binding
//! too. That's why CLOC06 pins
//! `constant-fold → fold-control-flow → dce`.
//!
//! # CV tracing — both modes work
//!
//! Per CLOC09's `cv: Option<CvId>` amendment, every node carries
//! optional CV identity. The pass mirrors the pattern
//! `closure-pass-constant-fold` established:
//!
//! - **Traced input** (`cv: Some(parent)`): the kept replacement
//!   uses its own pre-existing `cv` (it's the same node, just
//!   promoted from inside its parent). A `Contribution` is
//!   appended with `source = "fold-control-flow"`, `tag =
//!   "folded-branch"` or `"removed-dead-code"`, and `meta`
//!   describing the rewrite.
//! - **Untraced input** (`cv: None`): same fold happens, but no
//!   `Contribution` is emitted and no CV id derivation occurs.
//!   `changed: true` is still set so the pipeline knows.
//!
//! # What this pass *doesn't* do
//!
//! - It doesn't fold value-level expressions. `if (1 < 2) {A}`
//!   keeps its `if` here — `constant-fold`'s job is to first
//!   collapse the `1 < 2` to `true`, then `fold-control-flow`
//!   sees `if (true) {A}` and folds. The order in CLOC06 is what
//!   makes this combination work; running only this pass on
//!   `if (1 < 2) {A}` leaves it alone.
//! - It doesn't drop unreferenced bindings (`remove-unused-vars`).
//! - It doesn't propagate `BreakStatement` / `ContinueStatement`
//!   as terminators yet — Phase 2 will, once we have explicit
//!   labelled control-flow handling.

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use coding_adventures_correlation_vector::{CVLog, Contribution};
use coding_adventures_javascript_ast::{
    statement::TaggedStatement, ArrayExpression, AssignmentExpression,
    BinaryExpression, BindingTarget, BlockStatement, CallExpression, NewExpression, SequenceExpression, SpreadElement, YieldExpression, AwaitExpression, ImportExpression,
    ConditionalExpression, Declaration, EmptyStatement, Expression, ExpressionStatement, ForInit,
    ArrowBody, ArrowFunctionExpression, TaggedTemplateExpression, TemplateLiteral,
    ClassDeclaration, ClassExpression, ClassMember, MethodDefinition, PropertyDefinition,
    ForInStatement, ForOfStatement, ForStatement, FunctionDeclaration, FunctionExpression, Identifier, IfStatement,
    LogicalExpression,
    LogicalOperator,
    ChainExpression, MemberExpression, ObjectExpression, ObjectMember, OptionalCallExpression, OptionalMemberExpression, Program, ProgramItem, Property, PropertyKey,
    ReturnStatement, Statement, UnaryExpression, UnaryOperator, UpdateExpression, VarKind, VariableDeclaration,
    DoWhileStatement, VariableDeclarator, WhileStatement, WithStatement,
};
use serde_json::json;
use std::collections::{HashMap, HashSet};

/// `Pass::depends_on` value. CLOC06 canonical order pins
/// constant-fold first so we see folded literals as branch
/// conditions.
const DEPS: &[&str] = &["constant-fold"];

/// Control-flow folding pass — see crate-level docs.
#[derive(Debug, Default, Clone, Copy)]
pub struct FoldControlFlowPass;

impl FoldControlFlowPass {
    pub fn new() -> Self {
        Self
    }
}

impl Pass for FoldControlFlowPass {
    fn name(&self) -> &'static str {
        "fold-control-flow"
    }

    fn depends_on(&self) -> &[&'static str] {
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // Per CLOC06: eliminating one branch can expose another
        // statically-dead branch (nested ifs).
        IterationPolicy::FixedPoint
    }

    fn cost(&self) -> u32 {
        // Tree walk + per-branch condition evaluation. Comparable
        // to constant-fold.
        2
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        let mut state = FoldState {
            cv: ctx.cv,
            contributions: Vec::new(),
            changed: false,
            nodes_touched: 0,
        };
        let new_program = fold_program(ctx.program, &mut state);
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
// FoldState — mutable bookkeeping threaded through the walk
// =====================================================================

struct FoldState<'a> {
    cv: &'a mut CVLog,
    contributions: Vec<Contribution>,
    changed: bool,
    nodes_touched: u32,
}

impl FoldState<'_> {
    /// Record a fold-control-flow rewrite. When the input was
    /// traced (`parent` is `Some`), append a `Contribution`; in
    /// either mode set `changed = true`.
    fn record_fold(&mut self, parent: &Option<String>, tag: &str, before: &str, after: &str) {
        self.changed = true;
        if let Some(parent_cv) = parent {
            let contribution = Contribution {
                source: "fold-control-flow".to_string(),
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

    /// Record a fold that *eliminates* one or more branches — and
    /// tombstone each discarded node's own CV entry.
    ///
    /// [`record_fold`](Self::record_fold) logs a summary
    /// `Contribution` against the container (the `if` / `while` /
    /// ternary being folded). But when the pass folds
    /// `if (true) A else B` to `A`, the whole `B` branch *disappears* —
    /// and the container is not what vanished, `B` is. This method
    /// additionally marks each discarded node's CV entry with a
    /// `DeletionRecord` via [`CVLog::delete`], so a
    /// `--correlation_vector` consumer that later asks "what happened to
    /// the code that used to be here?" gets a definite answer —
    /// *fold-control-flow eliminated it, because `<reason>`* — instead
    /// of the branch silently vanishing from the provenance graph. This
    /// mirrors the deletion provenance the DCE pass records for the
    /// statements *it* drops.
    ///
    /// `delete` is a no-op when the log is disabled (production
    /// default), so this costs nothing off the `--correlation_vector`
    /// path. Rewrites that *preserve* both branches (`if→ternary`,
    /// De Morgan swaps, `if→&&`) must NOT call this — nothing was
    /// discarded there — and keep using plain `record_fold`.
    fn record_fold_deleting(
        &mut self,
        parent: &Option<String>,
        discarded: &[Option<String>],
        tag: &str,
        before: &str,
        after: &str,
    ) {
        for cv_id in discarded.iter().flatten() {
            let mut meta: HashMap<String, serde_json::Value> = HashMap::new();
            if let Some(parent_cv) = parent {
                meta.insert("container_cv".to_string(), json!(parent_cv));
            }
            self.cv.delete(cv_id, "fold-control-flow", tag, meta);
        }
        // Keep the container-level summary contribution so existing
        // history/stats/tests that look for this tag still observe the
        // fold at the enclosing node.
        self.record_fold(parent, tag, before, after);
    }

    fn visit(&mut self) {
        self.nodes_touched += 1;
    }
}

/// Fetch a statement's own correlation-vector id, if it carries one.
///
/// The branch-elimination sites need the *discarded* branch's CV id —
/// not just the enclosing `if` / `while`'s — so they can tombstone the
/// exact code that vanished (see [`FoldState::record_fold_deleting`]).
/// Every AST node struct carries an `Option<CvId>`; this unwraps the
/// `Statement` → `TaggedStatement` nesting to reach it. A
/// [`Statement::Declaration`] returns `None`: a declaration is never the
/// discarded arm of a folded conditional in the sites wired here.
fn statement_cv(stmt: &Statement) -> Option<String> {
    match stmt {
        Statement::Tagged(t) => tagged_statement_cv(t),
        Statement::Declaration(_) => None,
    }
}

/// The `TaggedStatement` arm of [`statement_cv`]. Exhaustive on purpose
/// (no `_` wildcard): a new statement kind added upstream fails to
/// compile here rather than silently losing provenance.
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

/// Fetch an expression's own correlation-vector id, if it carries one.
///
/// The ternary-collapse site (`fold_conditional`) discards one *arm* of a
/// `cond ? c : a` when `cond` is a literal — an `Expression`, not a
/// statement — so it needs this alongside [`statement_cv`] to tombstone
/// the exact expression that vanished. Exhaustive on purpose (no `_`
/// wildcard): a new expression kind added upstream fails to compile here
/// rather than silently losing provenance.
fn expression_cv(expr: &Expression) -> Option<String> {
    use Expression::*;
    match expr {
        Identifier(e) => e.cv.clone(),
        NumericLiteral(e) => e.cv.clone(),
        StringLiteral(e) => e.cv.clone(),
        BooleanLiteral(e) => e.cv.clone(),
        NullLiteral(e) => e.cv.clone(),
        BigIntLiteral(e) => e.cv.clone(),
        RegExpLiteral(e) => e.cv.clone(),
        UndefinedLiteral(e) => e.cv.clone(),
        BinaryExpression(e) => e.cv.clone(),
        LogicalExpression(e) => e.cv.clone(),
        UnaryExpression(e) => e.cv.clone(),
        UpdateExpression(e) => e.cv.clone(),
        AssignmentExpression(e) => e.cv.clone(),
        ConditionalExpression(e) => e.cv.clone(),
        CallExpression(e) => e.cv.clone(),
        NewExpression(e) => e.cv.clone(),
        SequenceExpression(e) => e.cv.clone(),
        TaggedTemplateExpression(e) => e.cv.clone(),
        SpreadElement(e) => e.cv.clone(),
        YieldExpression(y) => y.cv.clone(),
        AwaitExpression(a) => a.cv.clone(),
        ImportExpression(e) => e.cv.clone(),
        ThisExpression(t) => t.cv.clone(),
        Super(s) => s.cv.clone(),
        NewTarget(n) => n.cv.clone(),
        ImportMeta(n) => n.cv.clone(),
        MemberExpression(e) => e.cv.clone(),
        OptionalMemberExpression(e) => e.cv.clone(),
        OptionalCallExpression(e) => e.cv.clone(),
        ChainExpression(e) => e.cv.clone(),
        ArrayExpression(e) => e.cv.clone(),
        ObjectExpression(e) => e.cv.clone(),
        FunctionExpression(e) => e.cv.clone(),
        ClassExpression(e) => e.cv.clone(),
        ArrowFunctionExpression(e) => e.cv.clone(),
        TemplateLiteral(e) => e.cv.clone(),
    }
}

// =====================================================================
// Program / top-level
// =====================================================================

fn fold_program(prog: &Program, st: &mut FoldState) -> Program {
    st.visit();
    // Fold each top-level item, then flatten any redundant block it produced
    // (CLOC12.194). We build the list imperatively rather than `map`ping because
    // flattening turns one block item into *many* items (its body spliced in) —
    // or zero, for an empty block.
    let mut new_body: Vec<ProgramItem> = Vec::with_capacity(prog.body.len());
    for item in &prog.body {
        let folded = fold_program_item(item, st);
        if let ProgramItem::Statement(s) = &folded {
            if let Some((block_cv, body)) = redundant_block(s) {
                st.record_fold(
                    block_cv,
                    "flatten-redundant-block",
                    "{ <stmts> }",
                    "<stmts>",
                );
                new_body.extend(body.iter().cloned().map(ProgramItem::Statement));
                continue;
            }
            // Split a top-level comma-sequence statement (`a(), b();` → `a(); b();`).
            if let Some(split) = split_sequence_statement(s) {
                st.record_fold(
                    &statement_cv(s),
                    "split-comma-sequence",
                    "<a>, <b>;",
                    "<a>; <b>;",
                );
                new_body.extend(split.into_iter().map(ProgramItem::Statement));
                continue;
            }
        }
        new_body.push(folded);
    }
    Program {
        cv: prog.cv.clone(),
        version: prog.version,
        source_type: prog.source_type,
        // DIV#2 — as a post-step, merge any runs of adjacent same-kind variable
        // declarations this list now holds (see `coalesce_var_decls`).
        body: coalesce_var_decls(new_body, st),
    }
}

fn fold_program_item(item: &ProgramItem, st: &mut FoldState) -> ProgramItem {
    match item {
        ProgramItem::Statement(s) => ProgramItem::Statement(fold_statement(s, st)),
        ProgramItem::Declaration(d) => ProgramItem::Declaration(fold_declaration(d, st)),
    }
}

// =====================================================================
// Statements
// =====================================================================

fn fold_statement(stmt: &Statement, st: &mut FoldState) -> Statement {
    st.visit();
    match stmt {
        Statement::Tagged(t) => fold_tagged_statement(t, st),
        Statement::Declaration(d) => Statement::Declaration(fold_declaration(d, st)),
    }
}

/// Returns a `Statement` (not necessarily `TaggedStatement`) so
/// fold targets that lift outside the `Tagged` wrapper (rare in
/// Phase 1 but documented for consistency with future variants)
/// don't have to be wrapped back inside.
fn fold_tagged_statement(stmt: &TaggedStatement, st: &mut FoldState) -> Statement {
    match stmt {
        TaggedStatement::ExpressionStatement(s) => {
            Statement::Tagged(TaggedStatement::ExpressionStatement(ExpressionStatement {
                cv: s.cv.clone(),
                expression: fold_expression(&s.expression, st),
            }))
        }
        TaggedStatement::BlockStatement(s) => {
            Statement::Tagged(TaggedStatement::BlockStatement(fold_block_statement(s, st)))
        }
        TaggedStatement::IfStatement(s) => fold_if_statement(s, st),
        TaggedStatement::WhileStatement(s) => fold_while_statement(s, st),
        // `with (object) body` (CLOC12.187) — fold the object and body
        // structurally (a `with` is never eliminated). Not yet reachable.
        TaggedStatement::WithStatement(s) => Statement::Tagged(TaggedStatement::WithStatement(
            WithStatement {
                cv: s.cv.clone(),
                object: fold_expression(&s.object, st),
                body: Box::new(fold_statement(&s.body, st)),
            },
        )),
        // A `do … while(test)` runs its body at least once, so — unlike
        // `while` — it can NEVER be eliminated as a dead loop even when
        // `test` is statically falsy (the single body run is observable).
        // We therefore recurse structurally (fold the body and the test) and
        // then apply the same loop-body comma-fusion as `for`/`while`:
        //
        //   do { a(); b(); } while(c);   →   do a(), b(); while(c);
        //
        // A `BlockStatement` body whose statements are *all* plain expression
        // statements collapses to one (possibly comma-sequenced) expression
        // statement, dropping the braces — the comma operator runs them
        // left-to-right with identical side effects and the loop discards the
        // value. `stmts_as_sequence_expr` returns `None` (keeping the block) for
        // any body carrying a declaration (`var`/`let`/`const`), a
        // `break`/`continue`/`return`, or a nested statement — none of which can
        // join a comma-sequence. Unlike `while`, a `do`-loop is not rewritten to
        // a `for`, so the fusion is applied here rather than inherited from
        // `fold_for_statement`. It runs *after* the body's own inner folds, so an
        // `if (x) a();` that folded to `x && a()` participates:
        // `do { if (x) a(); b(); } while(c);` → `do x && a(), b(); while(c);`.
        TaggedStatement::DoWhileStatement(s) => {
            let body = fold_statement(&s.body, st);

            // Empty-bodied `do … while` ≡ `while`. `do {} while(test)` runs the
            // (empty) body once and then evaluates `test`; because the body is a
            // no-op, its test-evaluation sequence is IDENTICAL to `while(test){}`
            // (the leading empty run changes nothing observable). We rewrite to
            // the equivalent `while`, which lowers to `for` and — via the empty
            // loop-body normalization — collapses the braces:
            //
            //   do {} while(c);   →  while(c){}  →  for(; c;) {}  →  for(; c;) ;
            //   do {} while(0);   →  while(0){}  →  dead loop     →  ;  (dce drops)
            //
            // A NON-empty body keeps the `do` form (a `do` body runs before its
            // test, so it can't generally become a `while`); those fall through
            // to the loop-body comma-fusion below.
            if statement_is_empty(&body) {
                st.record_fold(
                    &s.cv,
                    "empty-do-while-to-while",
                    "do {} while(<test>)",
                    "while(<test>) {}",
                );
                return fold_while_statement(
                    &WhileStatement {
                        cv: s.cv.clone(),
                        test: s.test.clone(),
                        body: Box::new(body),
                    },
                    st,
                );
            }

            let body = match &body {
                Statement::Tagged(TaggedStatement::BlockStatement(_)) => {
                    match stmts_as_sequence_expr(&body) {
                        Some(expr) => {
                            let body_cv = statement_cv(&body);
                            st.record_fold(
                                &s.cv,
                                "loop-body-fuse",
                                "do { s1; s2; … } while(…)",
                                "do s1, s2, … while(…)",
                            );
                            Statement::expression_statement(ExpressionStatement {
                                cv: body_cv,
                                expression: expr,
                            })
                        }
                        None => body,
                    }
                }
                _ => body,
            };
            Statement::Tagged(TaggedStatement::DoWhileStatement(DoWhileStatement {
                cv: s.cv.clone(),
                body: Box::new(body),
                test: fold_expression(&s.test, st),
            }))
        }
        TaggedStatement::ForStatement(s) => fold_for_statement(s, st),
        TaggedStatement::ForInStatement(s) => {
            Statement::Tagged(TaggedStatement::ForInStatement(ForInStatement {
                cv: s.cv.clone(),
                left: match &s.left {
                    ForInit::VariableDeclaration(v) => {
                        ForInit::VariableDeclaration(fold_variable_declaration(v, st))
                    }
                    ForInit::Expression(e) => ForInit::Expression(fold_expression(e, st)),
                },
                right: fold_expression(&s.right, st),
                body: Box::new(fold_statement(&s.body, st)),
            }))
        }
        TaggedStatement::ForOfStatement(s) => {
            Statement::Tagged(TaggedStatement::ForOfStatement(ForOfStatement {
                cv: s.cv.clone(),
                left: match &s.left {
                    ForInit::VariableDeclaration(v) => {
                        ForInit::VariableDeclaration(fold_variable_declaration(v, st))
                    }
                    ForInit::Expression(e) => ForInit::Expression(fold_expression(e, st)),
                },
                right: fold_expression(&s.right, st),
                body: Box::new(fold_statement(&s.body, st)),
            }))
        }
        TaggedStatement::ReturnStatement(s) => {
            Statement::Tagged(TaggedStatement::ReturnStatement(ReturnStatement {
                cv: s.cv.clone(),
                argument: s.argument.as_ref().map(|e| fold_expression(e, st)),
            }))
        }
        TaggedStatement::LabeledStatement(s) => {
            // Fold inside the body. Don't try to peephole the label
            // away yet — `a: break a;` collapse is its own gap, handled
            // separately (it requires DCE/fold-control-flow to prove
            // there's no other reference to the label).
            Statement::Tagged(TaggedStatement::LabeledStatement(
                coding_adventures_javascript_ast::LabeledStatement {
                    cv: s.cv.clone(),
                    label: s.label.clone(),
                    body: Box::new(fold_statement(&s.body, st)),
                },
            ))
        }
        TaggedStatement::ThrowStatement(s) => {
            // `throw` terminates control flow like `return` does, so
            // future fold-control-flow rules (e.g. drop-dead-after-throw,
            // `if (x) foo() else throw e` → `if (!x) throw e; foo();`)
            // will live here. For CLOC12.14 the pass just walks through
            // and folds the argument expression.
            Statement::Tagged(TaggedStatement::ThrowStatement(
                coding_adventures_javascript_ast::ThrowStatement {
                    cv: s.cv.clone(),
                    argument: fold_expression(&s.argument, st),
                },
            ))
        }
        TaggedStatement::SwitchStatement(s) => {
            // Walk the discriminant + each case's test + consequent
            // through fold_expression / fold_statement. No peephole
            // rule yet — the "empty switch", "constant discriminant
            // → single case body" optimisations are gap-014
            // follow-ups; this PR ships the AST + recursive walk so
            // those rules have a place to land.
            Statement::Tagged(TaggedStatement::SwitchStatement(
                coding_adventures_javascript_ast::SwitchStatement {
                    cv: s.cv.clone(),
                    discriminant: fold_expression(&s.discriminant, st),
                    cases: s
                        .cases
                        .iter()
                        .map(|c| coding_adventures_javascript_ast::SwitchCase {
                            cv: c.cv.clone(),
                            test: c.test.as_ref().map(|e| fold_expression(e, st)),
                            consequent: c
                                .consequent
                                .iter()
                                .map(|s| fold_statement(s, st))
                                .collect(),
                        })
                        .collect(),
                },
            ))
        }
        TaggedStatement::TryStatement(s) => {
            // Recurse fold-control-flow into the protected block, catch body,
            // and finalizer (each an ordinary block). The catch `param` is
            // preserved. The `try` is not a terminator, so no control-flow
            // peephole applies to the statement itself here.
            Statement::Tagged(TaggedStatement::TryStatement(
                coding_adventures_javascript_ast::TryStatement {
                    cv: s.cv.clone(),
                    block: fold_block_statement(&s.block, st),
                    handler: s.handler.as_ref().map(|h| {
                        coding_adventures_javascript_ast::CatchClause {
                            cv: h.cv.clone(),
                            param: h.param.clone(),
                            body: fold_block_statement(&h.body, st),
                        }
                    }),
                    finalizer: s
                        .finalizer
                        .as_ref()
                        .map(|f| fold_block_statement(f, st)),
                },
            ))
        }
        TaggedStatement::BreakStatement(_)
        | TaggedStatement::ContinueStatement(_)
        | TaggedStatement::EmptyStatement(_)
        | TaggedStatement::DebuggerStatement(_) => Statement::Tagged(stmt.clone()),
    }
}

/// Folds `BlockStatement.body` by recursing into each statement
/// AND dropping everything after a `ReturnStatement` (dead code
/// after a definite terminator).
fn fold_block_statement(b: &BlockStatement, st: &mut FoldState) -> BlockStatement {
    let mut new_body = Vec::with_capacity(b.body.len());
    let mut hit_terminator = false;
    let mut dropped_count = 0usize;
    // Capture each dead-after-terminator statement's CV id so it can be
    // tombstoned (see the `record_fold_deleting` call below).
    let mut removed_cvs: Vec<Option<String>> = Vec::new();
    for s in &b.body {
        if hit_terminator {
            dropped_count += 1;
            removed_cvs.push(statement_cv(s));
            continue;
        }
        let folded = fold_statement(s, st);

        // CLOC25 — drop a redundant `else` after a terminating consequent.
        //
        // `if (C) <T-that-terminates> else <E>` is equivalent to
        // `if (C) <T>` followed by `E`: when `C` is true the consequent exits
        // (return/throw), so control reaches `E` only when `C` was false —
        // exactly the `else` semantics. Removing the `else` deletes a keyword
        // and (for a block) a pair of braces, and un-nests `else if` chains.
        // We only hoist when `E` is scope-safe to splice into this block (no
        // block-scoped declarations leaking out). See
        // `consequent_definitely_terminates` / `alternate_is_hoistable`.
        if let Statement::Tagged(TaggedStatement::IfStatement(if_s)) = &folded {
            if let Some(alt) = &if_s.alternate {
                if consequent_definitely_terminates(&if_s.consequent)
                    && alternate_is_hoistable(alt)
                {
                    // Push `if (C) T` with the `else` stripped.
                    new_body.push(Statement::if_statement(IfStatement {
                        cv: if_s.cv.clone(),
                        test: if_s.test.clone(),
                        consequent: if_s.consequent.clone(),
                        alternate: None,
                    }));
                    // Splice the former `else` body into this block.
                    match alt.as_ref() {
                        Statement::Tagged(TaggedStatement::BlockStatement(eb)) => {
                            for inner in &eb.body {
                                new_body.push(inner.clone());
                            }
                        }
                        other => new_body.push(other.clone()),
                    }
                    st.record_fold(
                        &if_s.cv,
                        "hoisted-else-after-terminator",
                        "if (C) <terminates> else <E>",
                        "if (C) <terminates> followed by <E>",
                    );
                    // The trimmed `if` is NOT itself a terminator (a false test
                    // falls through), but the hoisted tail might end in one —
                    // if so, mark it so the dead-code-after-terminator drop
                    // applies to any following statements.
                    if new_body.last().map(is_terminator).unwrap_or(false) {
                        hit_terminator = true;
                    }
                    continue;
                }
            }
        }

        // CLOC12.194 — flatten a redundant nested block into this block's
        // statement list. A bare `{ … }` with nothing block-scoped inside is
        // identical to its body run in place (e.g. the block left behind when
        // `if (true) { … }` folds to its consequent). Splicing may expose a
        // terminator as the new last statement, so re-check `hit_terminator`
        // for the dead-code-after-terminator drop, exactly as the `else`-hoist
        // above does.
        if let Some((block_cv, body)) = redundant_block(&folded) {
            st.record_fold(
                block_cv,
                "flatten-redundant-block",
                "{ <stmts> }",
                "<stmts>",
            );
            for inner in body {
                new_body.push(inner.clone());
            }
            if new_body.last().map(is_terminator).unwrap_or(false) {
                hit_terminator = true;
            }
            continue;
        }

        // Split a comma-sequence expression statement into separate statements
        // (`a(), b();` → `a(); b();`). A `SequenceExpression` never contains a
        // terminator (they are statements, not expressions), so no
        // `hit_terminator` re-check is needed.
        if let Some(split) = split_sequence_statement(&folded) {
            st.record_fold(
                &statement_cv(&folded),
                "split-comma-sequence",
                "<a>, <b>;",
                "<a>; <b>;",
            );
            new_body.extend(split);
            continue;
        }

        let terminates = is_terminator(&folded);
        new_body.push(folded);
        if terminates {
            hit_terminator = true;
        }
    }
    if dropped_count > 0 {
        // These statements are unreachable after a definite terminator
        // (`return`/`throw`) and are eliminated — tombstone each so its
        // span stays auditable, matching what DCE records for the same
        // dead-after-terminator drop it performs.
        st.record_fold_deleting(
            &b.cv,
            &removed_cvs,
            "removed-dead-code",
            &format!("block with {} statements", b.body.len()),
            &format!("dropped {} statements after terminator", dropped_count),
        );
    }
    BlockStatement {
        cv: b.cv.clone(),
        // DIV#2 — merge adjacent same-kind variable declarations in the block
        // body (including any the flatten/hoist above made adjacent).
        body: coalesce_var_decls(new_body, st),
    }
}

// =====================================================================
// DIV#2 — coalesce adjacent same-kind variable declarations
// =====================================================================
//
//   var a=1;var b=2;   ──▶   var a=1,b=2;      (one keyword + one `;` saved)
//   let a=1;let b=2;    ──▶   let a=1,b=2;
//   var a;var b;        ──▶   var a,b;
//
// The reference Closure Compiler merges a run of *strictly adjacent*
// `VariableDeclaration` statements of the *same kind* (`var`/`let`/`const`)
// into a single multi-declarator declaration. It is a pure size win with
// identical semantics: the declarators keep their original order — and so their
// initializer evaluation order (`var a=b++;var c=2;` → `var a=b++,c=2;` still
// runs `b++` first) — and merging same-scope same-kind bindings changes nothing
// about hoisting or the temporal dead zone.
//
// DECLINE conditions (each preserves byte-identity with Closure):
//   * different kinds don't merge — `var a=1;let b=2;` stays two statements;
//   * a non-declaration statement (or a different-kind decl) between two decls
//     breaks the run — only *strictly adjacent* decls merge; an EmptyStatement
//     (`;`) also breaks it here, but the pipeline's separate empty-statement
//     removal drops the `;` in an earlier fixed-point iteration so the decls
//     become adjacent and merge on a later pass;
//   * a run whose merged declarator list would REPEAT a binding name is left
//     untouched: `var a=1;var a=2;` must NOT become `var a=1,a=2;` — Closure
//     rewrites the redeclaration's second half to a bare assignment
//     (`var a=1;a=2;`), a *different* transform this pass does not perform, so
//     we decline rather than diverge. (For `let`/`const` a repeated name is a
//     syntax error that never reaches us.)
//
// Runs as a POST-STEP after this crate's other statement-list rewrites
// (block-flatten, else-hoist, dead-code drop) so decls a block-flatten made
// adjacent (`{var a=1}var b=2` → `var a=1;var b=2` → `var a=1,b=2`) also merge.

/// A statement-list element that *may* carry a bare variable declaration.
///
/// Implemented for both list-element types this pass assembles — [`ProgramItem`]
/// (top level) and [`Statement`] (block body) — so one generic coalescer serves
/// both. A top-level `var` is bridged as
/// `ProgramItem::Statement(Statement::Declaration(…))` (the parser routes
/// variable statements through `statement`), so the program-item view looks
/// through that wrapper as well as the bare `ProgramItem::Declaration` form, and
/// rebuilds into the wrapper form the bridge uses.
trait VarDeclCarrier: Clone {
    /// The variable declaration this element carries, if any.
    fn as_var_decl(&self) -> Option<&VariableDeclaration>;
    /// Wrap a (merged) variable declaration back into a list element.
    fn from_var_decl(decl: VariableDeclaration) -> Self;
}

impl VarDeclCarrier for ProgramItem {
    fn as_var_decl(&self) -> Option<&VariableDeclaration> {
        match self {
            ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(vd)))
            | ProgramItem::Declaration(Declaration::VariableDeclaration(vd)) => Some(vd),
            _ => None,
        }
    }
    fn from_var_decl(decl: VariableDeclaration) -> Self {
        // Match the bridge's program-level shape (a variable *statement*).
        ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(decl)))
    }
}

impl VarDeclCarrier for Statement {
    fn as_var_decl(&self) -> Option<&VariableDeclaration> {
        match self {
            Statement::Declaration(Declaration::VariableDeclaration(vd)) => Some(vd),
            _ => None,
        }
    }
    fn from_var_decl(decl: VariableDeclaration) -> Self {
        Statement::Declaration(Declaration::VariableDeclaration(decl))
    }
}

/// Merge runs of strictly-adjacent same-kind variable declarations in one
/// statement list into single multi-declarator declarations. See the module
/// comment above for the exact rule and the decline conditions.
fn coalesce_var_decls<T: VarDeclCarrier>(items: Vec<T>, st: &mut FoldState) -> Vec<T> {
    let mut out: Vec<T> = Vec::with_capacity(items.len());
    let mut i = 0;
    while i < items.len() {
        // A run can only start at a variable declaration; anything else passes
        // through verbatim.
        let kind = match items[i].as_var_decl() {
            Some(vd) => vd.kind,
            None => {
                out.push(items[i].clone());
                i += 1;
                continue;
            }
        };
        // Extend the run over every immediately-following same-kind decl.
        let mut j = i + 1;
        while j < items.len() && items[j].as_var_decl().map(|vd| vd.kind) == Some(kind) {
            j += 1;
        }
        if j - i < 2 {
            // A lone declaration — nothing adjacent to merge into it.
            out.push(items[i].clone());
            i += 1;
            continue;
        }

        // Gather the run's declarators in order; a repeated binding name across
        // the run declines the *whole* run (redeclaration → separate transform).
        //
        // The seen-names check uses a `HashSet` (not a linear `Vec` scan): a run
        // of N strictly-adjacent single-declarator statements is trivially
        // authorable in the input JS (`var a0=1;var a1=1;…`), and a per-declarator
        // linear membership test would be Θ(N²) — a linear-input → quadratic-work
        // DoS on this hot pass path. The `&str` borrows live in `items[i..j]`,
        // which outlives this per-run gather, so no owned clone is needed.
        let mut declarations: Vec<VariableDeclarator> = Vec::new();
        let mut names: HashSet<&str> = HashSet::new();
        let mut duplicate = false;
        for item in &items[i..j] {
            let vd = item.as_var_decl().expect("run member is a var declaration");
            for d in &vd.declarations {
                let BindingTarget::Identifier(id) = &d.id;
                // `insert` returns false when the name was already present.
                if !names.insert(id.name.as_str()) {
                    duplicate = true;
                }
                declarations.push(d.clone());
            }
        }

        if duplicate {
            // Leave the run untouched — declining is never wrong.
            for item in &items[i..j] {
                out.push(item.clone());
            }
            i = j;
            continue;
        }

        // Merge: the first decl's CV becomes the container; the folded-away
        // declarations' CVs are tombstoned (their statement wrappers vanish).
        let container_cv = items[i].as_var_decl().unwrap().cv.clone();
        let discarded: Vec<Option<String>> = items[i + 1..j]
            .iter()
            .map(|item| item.as_var_decl().unwrap().cv.clone())
            .collect();
        st.record_fold_deleting(
            &container_cv,
            &discarded,
            "coalesce-var-declarations",
            "adjacent same-kind var declarations",
            "one multi-declarator declaration",
        );
        out.push(T::from_var_decl(VariableDeclaration {
            cv: container_cv,
            kind,
            declarations,
        }));
        i = j;
    }
    out
}

/// A statement that unconditionally transfers control out of the
/// enclosing block. In Phase 1, only `ReturnStatement` qualifies
/// — Phase 2 adds `ThrowStatement`, and `BreakStatement` /
/// `ContinueStatement` qualify in their enclosing loop scope.
///
/// NOTE: this gates the dead-code-after-terminator drop in
/// [`fold_block_statement`]; keeping it return-only preserves that pass's
/// long-standing behaviour. The `else`-hoist (CLOC25) uses the broader
/// [`consequent_definitely_terminates`] instead, which also accepts `throw`.
fn is_terminator(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Tagged(TaggedStatement::ReturnStatement(_))
    )
}

/// Does this statement, used as an `if` **consequent**, unconditionally leave
/// the enclosing block — so that the matching `else` branch only runs when the
/// consequent did NOT, and can therefore be hoisted out after the `if`?
/// (CLOC25, upstream Closure's `MinimizeExitPoints`.)
///
/// `return` and `throw` both qualify. A `BlockStatement` qualifies when its
/// LAST statement does. We deliberately do NOT look inside nested
/// `if`/loops/`try` (a conservative check — declining merely forgoes an
/// optimization and is never a miscompile).
///
/// ```text
///   if (c) { …; return x; } else { B }   →   if (c) { …; return x; } B
///   if (bad) throw e; else use(v);        →   if (bad) throw e; use(v);
/// ```
fn consequent_definitely_terminates(stmt: &Statement) -> bool {
    match stmt {
        Statement::Tagged(TaggedStatement::ReturnStatement(_))
        | Statement::Tagged(TaggedStatement::ThrowStatement(_)) => true,
        Statement::Tagged(TaggedStatement::BlockStatement(b)) => b
            .body
            .last()
            .map(consequent_definitely_terminates)
            .unwrap_or(false),
        _ => false,
    }
}

/// Can an `else` block's statements be spliced into the enclosing block
/// without changing any binding's scope? Block-scoped declarations
/// (`let`/`const`/`function`) would leak upward or collide if moved out, so
/// they make the block unsafe to hoist; a plain `var` is function-scoped and
/// hoists harmlessly. Mirrors `closure-pass-dce`'s
/// `block_is_scope_safe_to_flatten`.
fn block_is_scope_safe_to_hoist(b: &BlockStatement) -> bool {
    b.body.iter().all(|s| match s {
        Statement::Declaration(Declaration::VariableDeclaration(v)) => {
            matches!(v.kind, VarKind::Var)
        }
        Statement::Declaration(Declaration::FunctionDeclaration(_)) => false,
        // A `class` declaration is block-scoped, so hoisting it out of the
        // `else` block would leak or collide its binding — unsafe, like a
        // nested function declaration.
        Statement::Declaration(Declaration::ClassDeclaration(_)) => false,
        // An import declaration is module-top-level only; never legally inside
        // the block being hoisted, so treat it as unsafe to hoist.
        Statement::Declaration(Declaration::ImportDeclaration(_)) => false,
        Statement::Declaration(Declaration::ExportNamedDeclaration(_)) => false,
        Statement::Declaration(Declaration::ExportDefaultDeclaration(_)) => false,
        Statement::Declaration(Declaration::ExportAllDeclaration(_)) => false,
        // Tagged statements introduce no lexical binding of their own.
        Statement::Tagged(_) => true,
    })
}

/// CLOC12.194 — is `folded`, sitting at statement-list position, a **redundant
/// block** that should be replaced by its own statements spliced in place?
///
/// A bare `{ … }` block introduces a fresh lexical scope, but if nothing inside
/// is block-scoped (`let`/`const`/`class`/`function`) that scope is
/// unobservable: the braces can be removed and the inner statements run
/// directly in the enclosing list with identical semantics. `var` is
/// function-scoped, so it hoists out harmlessly; the empty block `{}` flattens
/// to *nothing* (removed). This mirrors Closure's `PeepholeRemoveDeadCode`
/// block normalization and fires on a hand-written `{ … }` as well as on the
/// block left behind when `if (true) { … }` collapses to its consequent.
///
/// Returns `Some((block_cv, body))` when safe to flatten — the caller splices
/// `body` into the statement list and records a fold against `block_cv` — or
/// `None` to keep the statement as-is (any non-block, or a block that declares
/// a block-scoped binding: reuses the [`block_is_scope_safe_to_hoist`] gate that
/// backs the CLOC25 `else`-hoist, so the soundness boundary is shared).
fn redundant_block(folded: &Statement) -> Option<(&Option<String>, &[Statement])> {
    match folded {
        Statement::Tagged(TaggedStatement::BlockStatement(b))
            if block_is_scope_safe_to_hoist(b) =>
        {
            Some((&b.cv, &b.body))
        }
        _ => None,
    }
}

/// A comma sequence used as an EXPRESSION STATEMENT at a statement-list position
/// splits into one statement per sub-expression, matching the reference Closure
/// Compiler's Normalize (the inverse of the loop-body comma-fusion): `a(), b();`
/// -> `a(); b();`, `1, a();` -> `1; a();`. The split is behaviour-preserving —
/// the comma operator evaluates its operands left-to-right and discards all but
/// the last, and an expression statement already discards its value, so running
/// each operand as its own statement is identical.
///
/// This is valid ONLY at a statement-LIST position (program / block body), which
/// is why it lives in [`fold_block_statement`] / [`fold_program`] rather than
/// [`fold_statement`]: a single-statement body (`if (x) a(), b();`,
/// `for (;;) a(), b();`) has no braces, so the sequence must stay fused there.
/// Returns `None` when `folded` is not an `ExpressionStatement` wrapping a
/// `SequenceExpression`.
fn split_sequence_statement(folded: &Statement) -> Option<Vec<Statement>> {
    if let Statement::Tagged(TaggedStatement::ExpressionStatement(es)) = folded {
        if let Expression::SequenceExpression(seq) = &es.expression {
            return Some(
                seq.expressions
                    .iter()
                    .map(|e| {
                        Statement::expression_statement(ExpressionStatement {
                            cv: None,
                            expression: e.clone(),
                        })
                    })
                    .collect(),
            );
        }
    }
    None
}

/// Is this `else` branch safe to hoist into the enclosing block (CLOC25)?
///
/// - A `BlockStatement` is gated on [`block_is_scope_safe_to_hoist`].
/// - A non-block single statement cannot be a bare lexical/`function`
///   declaration in valid JS (those require braces), so any single *tagged*
///   statement is safe to hoist as-is.
/// - A bare `Declaration` (e.g. `else var x;` / Annex-B `else function f(){}`)
///   is declined — moving it could change its scope.
///
/// INVARIANT (load-bearing for soundness): every binding-introducing form —
/// `var`/`let`/`const` and `function` declarations — is represented as
/// `Statement::Declaration(..)`, never wrapped in a `TaggedStatement`. The
/// `Tagged(_) => true` arm therefore admits only non-declaration statements. If
/// the parser ever wrapped a declaration in `TaggedStatement`, this arm would
/// start leaking a block-scoped binding into the outer scope — so that
/// representation invariant must hold.
fn alternate_is_hoistable(alt: &Statement) -> bool {
    match alt {
        Statement::Tagged(TaggedStatement::BlockStatement(b)) => block_is_scope_safe_to_hoist(b),
        Statement::Tagged(_) => true,
        Statement::Declaration(_) => false,
    }
}

/// Fold an `IfStatement`. When `test` is a known-truthy/falsy
/// literal, collapse to the chosen branch (or `EmptyStatement` if
/// the chosen branch doesn't exist).
fn fold_if_statement(s: &IfStatement, st: &mut FoldState) -> Statement {
    // Recurse first so child folds happen before we decide.
    let test = fold_expression(&s.test, st);
    let consequent = fold_statement(&s.consequent, st);
    let alternate = s.alternate.as_ref().map(|a| fold_statement(a, st));

    // (CLOC12.25 / gap-018): De Morgan's negation-swap.
    //
    // When the test is exactly `!<inner>` AND an alternate exists,
    // strip the unary `!` and swap consequent ↔ alternate:
    //
    //   if (!x) C; else A;     →   if (x) A; else C;
    //   if (!flag) foo(); else bar();
    //                          →   if (flag) bar(); else foo();
    //
    // Why this is safe:
    //
    //   * `!x` and `x` evaluate `x` the same number of times (once
    //     each) and produce the same `ToBoolean(x)` decision flipped
    //     bit-wise. After the rewrite, the swapped branches make the
    //     overall control-flow observationally identical.
    //   * No additional evaluations of the operand are introduced.
    //   * No side effects in `x` are re-ordered relative to the
    //     consequent / alternate, because they originally ran before
    //     the branch was selected and they still do after the rewrite.
    //
    // Why we require alternate.is_some(): without an alternate, the
    // rewrite would have to synthesise an empty branch
    // (`if (x) ; else C;`) which actively adds an empty statement
    // node — the wrong shape for output minification, and the
    // gap-016 `if (!x) C;` → `!x && C;` rewrite already handles
    // that case better.
    //
    // Why this runs before literal_truthy: if the inner expression
    // (`<inner>` after stripping `!`) is itself a literal, the next
    // step's literal_truthy resolution will produce the correct
    // chosen branch. Equivalent to letting the pipeline converge:
    // doing the swap first means fewer iterations.
    //
    // We do NOT chain into multiple `!!...!<inner>` peels here —
    // a single peel per fixed-point iteration is enough; the
    // scheduler will re-call us until the expression stabilises.
    let (test, consequent, alternate) = match alternate {
        Some(alt) => {
            if let Expression::UnaryExpression(u) = test {
                if u.operator == UnaryOperator::Not {
                    let inner = *u.argument;
                    st.record_fold(
                        &s.cv,
                        "de-morgan-swap-not",
                        "if (!<inner>) <c>; else <a>;",
                        "if (<inner>) <a>; else <c>;",
                    );
                    (inner, alt, Some(consequent))
                } else {
                    (Expression::UnaryExpression(u), consequent, Some(alt))
                }
            } else {
                (test, consequent, Some(alt))
            }
        }
        None => (test, consequent, None),
    };

    match literal_truthy(&test) {
        Some(true) => {
            // The `alternate` branch is statically unreachable and is
            // discarded — tombstone it so its span stays auditable. Any
            // hoisted `var` inside it must SURVIVE the removal (dropping it is
            // a miscompile), so extract it before the taken `consequent`.
            let discarded = [alternate.as_ref().and_then(statement_cv)];
            st.record_fold_deleting(
                &s.cv,
                &discarded,
                "folded-branch",
                "if (<truthy literal>) { … } else { … }",
                "{ consequent }",
            );
            match alternate {
                Some(alt) => collapse_extracting_dead_vars(&alt, Some(consequent), &s.cv),
                None => consequent,
            }
        }
        Some(false) => {
            // The `consequent` branch is statically unreachable and is
            // discarded — tombstone it so its span stays auditable. Any
            // hoisted `var` inside it must SURVIVE the removal (dropping it is
            // a miscompile), so extract it before the taken `alternate` (or as
            // the sole survivor when there is no `else`). A no-var branch pick
            // yields the `alternate` / `;` unchanged, exactly as before.
            let discarded = [statement_cv(&consequent)];
            st.record_fold_deleting(
                &s.cv,
                &discarded,
                "folded-branch",
                "if (<falsy literal>) { … } else { … }",
                "{ alternate }",
            );
            collapse_extracting_dead_vars(&consequent, alternate, &s.cv)
        }
        None => {
            // Non-literal test. Try the if-else→ternary fold first
            // (CLOC12.18 / gap-017): when both branches reduce to a
            // single ExpressionStatement, rewrite the whole
            // IfStatement to an ExpressionStatement wrapping a
            // ConditionalExpression.
            //
            // Truth table (assuming `test` has no literal-truthy
            // resolution — those cases are handled above):
            //
            //   if (x) foo(); else bar();      → x ? foo() : bar();
            //   if (x) { foo(); } else { bar(); }
            //                                  → x ? foo() : bar();
            //                                    (single-statement
            //                                     blocks unwrap)
            //   if (x) a(); else { b(); c(); } → x ? a() : (b(), c());
            //   if (x) { m(); n(); } else a(); → x ? (m(), n()) : a();
            //   if (x) { a(); b(); } else { c(); d(); }
            //                                  → x ? (a(), b()) : (c(), d());
            //   if (x) foo();                  → unchanged (no
            //                                    alternate to fold)
            //   if (x) return 1; else return 2;
            //                                  → unchanged (return is
            //                                    not an expression
            //                                    statement; tracked
            //                                    as gap-019)
            //
            // A branch that is a **run of expression statements** is
            // reduced to a single comma-sequence expression by
            // `stmts_as_sequence_expr` (a single expression statement
            // stays as-is; two-or-more become `(s1, s2, …)`). The comma
            // operator evaluates its operands left-to-right and yields
            // the last, so a block's statement order and side effects
            // are preserved exactly. A branch that contains anything
            // other than expression statements (a `return`, a
            // declaration, a nested `if`, …) makes `stmts_as_sequence_expr`
            // return `None`, so this fold declines and the `if` is kept.
            //
            // Why this is safe: a ConditionalExpression has the same
            // evaluation order as the if-else — `test` is evaluated
            // first, then exactly one of the two branches. Side
            // effects in `test`, in `consequent`, in `alternate`
            // observed in the original program are all preserved in
            // the rewritten form; the wrapper ExpressionStatement
            // discards the ternary's value, so only the branches' side
            // effects matter and they fire under the same condition.
            if let Some(alt) = &alternate {
                if let (Some(c_expr), Some(a_expr)) =
                    (stmts_as_sequence_expr(&consequent), stmts_as_sequence_expr(alt))
                {
                    st.record_fold(
                        &s.cv,
                        "if-else-to-ternary",
                        "if (<test>) <then>; else <else>;",
                        "<test> ? <then> : <else>;",
                    );
                    let cond = Expression::ConditionalExpression(ConditionalExpression {
                        cv: None,
                        test: Box::new(test),
                        consequent: Box::new(c_expr),
                        alternate: Box::new(a_expr),
                    });
                    return Statement::expression_statement(ExpressionStatement {
                        cv: s.cv.clone(),
                        expression: cond,
                    });
                }
            }

            // (CLOC12.26 / gap-019): when both branches reduce to a
            // ReturnStatement-with-argument, hoist the return out and
            // wrap the argument expressions in a ternary:
            //
            //   if (x) return E1; else return E2;
            //                              → return x ? E1 : E2;
            //   if (x) { return E1; } else { return E2; }
            //                              → return x ? E1 : E2;
            //
            // Upstream Closure performs the same rewrite under
            // `PeepholeMinimizeConditions.tryFoldReturns`.
            //
            // Why this is safe:
            //
            //   * The if-else evaluates `x` once, then takes exactly
            //     one branch and runs `return E1` or `return E2`. The
            //     ternary form also evaluates `x` once, then takes
            //     exactly one of `E1` / `E2`, then returns. The set of
            //     values evaluated and the function's exit value match.
            //   * Side-effect ordering preserved: `x` runs first; then
            //     exactly the chosen branch's argument expression runs.
            //   * Control flow preserved: in both forms the function
            //     returns immediately after the chosen argument
            //     evaluates. No fall-through possible because both
            //     branches were terminal returns.
            //
            // Why we require both arguments to be `Some`: a bare
            // `return;` with no value is equivalent to `return undefined;`
            // but we can't synthesise an `undefined` expression on the
            // typed AST cleanly without `UndefinedLiteral` care that
            // belongs in its own follow-up. Conservative: bail when
            // either side is bare-return.
            //
            // Why we don't fire on `if (x) return E;` (no alternate
            // with a return): the fall-through case after a missing
            // alternate has implicit `return undefined` (or the next
            // statement's behaviour), which doesn't compose cleanly
            // with a ternary in statement position. The caller-supplied
            // gap-016 path also doesn't apply here (returns aren't
            // expression statements). Tracked separately.
            if let Some(alt) = &alternate {
                if let (Some(c_arg), Some(a_arg)) =
                    (single_return_with_arg(&consequent), single_return_with_arg(alt))
                {
                    st.record_fold(
                        &s.cv,
                        "if-else-returns-to-ternary-return",
                        "if (<test>) return <e1>; else return <e2>;",
                        "return <test> ? <e1> : <e2>;",
                    );
                    let cond = Expression::ConditionalExpression(ConditionalExpression {
                        cv: None,
                        test: Box::new(test),
                        consequent: Box::new(c_arg),
                        alternate: Box::new(a_arg),
                    });
                    return Statement::return_statement(ReturnStatement {
                        cv: s.cv.clone(),
                        argument: Some(cond),
                    });
                }
            }

            // (CLOC12.24 / gap-016): when there's *no* alternate and
            // the consequent reduces to a single ExpressionStatement,
            // rewrite the IfStatement to `<test> && <consequent>` as
            // a LogicalExpression. Upstream Closure performs the same
            // rewrite under `PeepholeMinimizeConditions::tryMinimizeIf`.
            //
            // Truth table (with `test` non-literal — literal cases
            // were handled above):
            //
            //   if (x) foo();              → x && foo();
            //   if (x) { foo(); }          → x && foo();   (single-expr
            //                                               block unwraps
            //                                               via single_expr_stmt)
            //   if (x) foo(); else {}      → x && foo();   (empty else is a
            //                                               no-op — same as no
            //                                               alternate at all)
            //   if (x) { foo(); } else ;   → x && foo();   (empty-stmt else)
            //   if (x) foo(); else bar();  → handled above (ternary)
            //   if (x) return 1;           → unchanged (return is not an
            //                                expression statement; this
            //                                is gap-019's territory)
            //   if (x) { a; b; }           → unchanged (multi-statement
            //                                consequent doesn't reduce
            //                                to a single Expression)
            //   if (x) ;                   → unchanged (empty consequent
            //                                isn't expressible as a
            //                                LogicalExpression without
            //                                synthesising `undefined`,
            //                                which we leave for later)
            //
            // Why this is safe:
            //
            //   * `x && consequent` evaluates `x` first (the
            //     short-circuit gate), exactly like `if (x) S` does.
            //   * If `x` is falsy: `&&` returns `x` *without*
            //     evaluating the right operand. In `if (x) S`, when
            //     `x` is falsy `S` is also not executed. Behaviour
            //     match.
            //   * If `x` is truthy: `&&` returns the right operand's
            //     value, which equals evaluating `consequent`. In
            //     `if (x) S`, when `x` is truthy `S` is executed for
            //     its side effects; the wrapper ExpressionStatement
            //     discards the result, so the *value* of `&&` is
            //     irrelevant. Behaviour match.
            //   * Order-of-evaluation: identical. No second `x`
            //     evaluation; `consequent`'s side effects fire when
            //     and only when `x` is truthy.
            //
            // An **empty** alternate (`else {}` / `else ;`) is a no-op,
            // so `if (x) S; else {}` is behaviourally identical to
            // `if (x) S;` — we accept it here and fold to `x && S` too.
            // (The ternary fold above already declined it: an empty
            // alternate has no expression to place in the ternary's
            // else arm, so `stmts_as_sequence_expr(alt)` returned
            // `None`.) A **non-empty** alternate that merely failed to
            // ternarise is NOT folded here — that would silently drop
            // the else branch — so the gate is "no alternate, or an
            // empty one".
            // A multi-statement consequent is comma-sequenced, mirroring
            // the ternary fold: `if (x) { a(); b(); }` → `x && (a(), b())`,
            // `if (x) { a(); b(); } else {}` → `x && (a(), b())`. Only a
            // run of expression statements qualifies (`stmts_as_sequence_expr`
            // returns `None` otherwise, e.g. a `return` or declaration
            // member), so the fold declines rather than misconvert.
            let alternate_is_absent_or_empty =
                alternate.as_ref().is_none_or(statement_is_empty);
            if alternate_is_absent_or_empty {
                if let Some(c_expr) = stmts_as_sequence_expr(&consequent) {
                    st.record_fold(
                        &s.cv,
                        "if-to-logical-and",
                        "if (<test>) <then>;",
                        "<test> && <then>;",
                    );
                    let and = Expression::LogicalExpression(LogicalExpression {
                        cv: None,
                        operator: LogicalOperator::And,
                        left: Box::new(test),
                        right: Box::new(c_expr),
                    });
                    return Statement::expression_statement(ExpressionStatement {
                        cv: s.cv.clone(),
                        expression: and,
                    });
                }
            }

            // Empty THEN branch + single-expression-statement ELSE →
            // `<test> || <expr>`. The mirror of the `if-to-logical-and` above:
            // when the consequent does nothing and the alternate is one
            // expression statement, `if (x) {} else S;` is `x || S;`.
            //
            //   if (x) {} else foo();        → x || foo();
            //   if (x) ; else foo();         → x || foo();     (empty-stmt then)
            //   if (x) {} else { foo(); }    → x || foo();     (single-expr
            //                                                   block unwraps)
            //   if (x) {} else b = 1;        → x || (b = 1);   (emitter parens
            //                                                   the lower-
            //                                                   precedence assign)
            //   if (a && b) {} else c();     → a && b || c();  (compound test)
            //   if (x) {} else { a(); b(); } → x || (a(), b());  (multi-statement
            //                                  else — sequenced; the `,` groups
            //                                  the effects under one `||` right
            //                                  operand, preserving order)
            //   if (x) {} else return E;     → unchanged (return isn't an
            //                                  expression statement, so it can't
            //                                  join a comma-sequence)
            //
            // Why this is safe (the dual of the `&&` case):
            //
            //   * `x || S` evaluates `x` first — exactly like `if (x) {} else S`.
            //   * If `x` is truthy: `||` short-circuits and does NOT evaluate
            //     `S`, matching the empty then-branch running nothing.
            //   * If `x` is falsy: `||` evaluates `S`, matching the else branch.
            //     When the else is several statements, `S` is `(s1, s2, …)` — the
            //     comma operator runs them left-to-right, same order as the block.
            //   * The wrapper ExpressionStatement discards the result, so the
            //     value `||` yields is irrelevant — only `S`'s side effects
            //     matter, and they fire exactly when `x` is falsy in both forms.
            //
            // A `!<inner>` test never reaches here with an empty consequent: the
            // De Morgan swap above already rewrote `if (!x) {} else S;` to
            // `if (x) S; else {}` (non-empty consequent), so we don't interfere
            // with that normalisation.
            if let Some(alt) = &alternate {
                if statement_is_empty(&consequent) {
                    if let Some(a_expr) = stmts_as_sequence_expr(alt) {
                        st.record_fold(
                            &s.cv,
                            "if-empty-then-to-logical-or",
                            "if (<test>) {} else <stmts>;",
                            "<test> || (<stmts>);",
                        );
                        let or = Expression::LogicalExpression(LogicalExpression {
                            cv: None,
                            operator: LogicalOperator::Or,
                            left: Box::new(test),
                            right: Box::new(a_expr),
                        });
                        return Statement::expression_statement(ExpressionStatement {
                            cv: s.cv.clone(),
                            expression: or,
                        });
                    }
                }
            }

            // Couldn't ternarise or logical-and/or — keep the IfStatement.
            Statement::if_statement(IfStatement {
                cv: s.cv.clone(),
                test,
                consequent: Box::new(consequent),
                alternate: alternate.map(Box::new),
            })
        }
    }
}

/// Helper for the if-else→ternary fold (gap-017). Returns the inner
/// expression when `stmt` is exactly one ExpressionStatement
/// (possibly wrapped in single-statement BlockStatement layers);
/// returns `None` for everything else.
///
/// We recurse through BlockStatement so source like `if (x) { foo(); }`
/// (with explicit braces around a single statement) folds the same
/// way as `if (x) foo();`. We do NOT recurse into anything that
/// changes statement count — multi-statement blocks bail out.
fn single_expr_stmt(stmt: &Statement) -> Option<Expression> {
    match stmt {
        Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
            Some(es.expression.clone())
        }
        Statement::Tagged(TaggedStatement::BlockStatement(b)) if b.body.len() == 1 => {
            single_expr_stmt(&b.body[0])
        }
        _ => None,
    }
}

/// Collapse a statement into a *single expression* by comma-sequencing, when
/// possible. Generalises [`single_expr_stmt`] from one expression statement to a
/// **run** of them:
///
///   { a(); }            → a()               (1-element: bare, no `,` wrapper)
///   { a(); b(); }       → (a(), b())         (2+ → SequenceExpression)
///   { a(); b; c(); }    → (a(), b, c())      (pure members kept — dropping them
///                                             is a *separate* DCE transform)
///
/// It is a strict superset of `single_expr_stmt`, so callers that used that
/// helper keep byte-identical output on the single-statement case and now also
/// fold a multi-statement block. It **declines** (returns `None`) whenever a
/// member isn't a plain expression statement — a `var`/`let`/`const`
/// declaration, a `return`/`break`/`if`/loop, or a nested block — because those
/// can't be expressed as one comma-sequence (Closure reaches for a different
/// rewrite there, e.g. the De Morgan `if (!x) { … }`).
///
/// The comma operator evaluates its operands left-to-right and yields the last,
/// so `(s1, s2, …)` runs the block's statements in the original order with the
/// same set of side effects — the wrapping context (`x || (…)`) discards the
/// value, so only that ordering matters.
fn stmts_as_sequence_expr(stmt: &Statement) -> Option<Expression> {
    // The 1-element case (a bare expression statement, or single-statement block
    // layers) is exactly `single_expr_stmt`; reuse it so behaviour is identical.
    if let Some(e) = single_expr_stmt(stmt) {
        return Some(e);
    }
    // Otherwise only a block of ≥2 statements can sequence, and only when EVERY
    // member is a plain expression statement.
    if let Statement::Tagged(TaggedStatement::BlockStatement(b)) = stmt {
        if b.body.len() >= 2 {
            let mut expressions = Vec::with_capacity(b.body.len());
            for member in &b.body {
                match member {
                    Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
                        expressions.push(es.expression.clone());
                    }
                    // A declaration / return / nested block / etc. can't join a
                    // comma-sequence — bail so the `if` is left intact.
                    _ => return None,
                }
            }
            return Some(Expression::SequenceExpression(SequenceExpression {
                cv: None,
                expressions,
            }));
        }
    }
    None
}

/// True when `stmt` does nothing observable — an `EmptyStatement` (`;`) or a
/// `BlockStatement` whose every member is itself empty (`{}`, `{;;}`, `{{}}`).
///
/// Used by the `if-empty-then-to-logical-or` fold to recognise a do-nothing
/// consequent. We recurse through block layers (mirroring `single_expr_stmt`)
/// so `if (x) {} else …` and `if (x) ; else …` fold the same way; anything that
/// does real work — a statement that isn't empty — makes the whole block
/// non-empty and bails the fold.
fn statement_is_empty(stmt: &Statement) -> bool {
    match stmt {
        Statement::Tagged(TaggedStatement::EmptyStatement(_)) => true,
        Statement::Tagged(TaggedStatement::BlockStatement(b)) => {
            b.body.iter().all(statement_is_empty)
        }
        _ => false,
    }
}

/// Helper for the return-then-return fold (gap-019). Returns the
/// `argument` expression when `stmt` is exactly one ReturnStatement
/// whose argument is `Some` (possibly wrapped in single-statement
/// BlockStatement layers); returns `None` for everything else,
/// including `return;` (bare return with no value).
///
/// Mirrors `single_expr_stmt`'s shape — both recurse through
/// single-statement BlockStatement layers but bail on anything that
/// changes statement count.
fn single_return_with_arg(stmt: &Statement) -> Option<Expression> {
    match stmt {
        Statement::Tagged(TaggedStatement::ReturnStatement(rs)) => rs.argument.clone(),
        Statement::Tagged(TaggedStatement::BlockStatement(b)) if b.body.len() == 1 => {
            single_return_with_arg(&b.body[0])
        }
        _ => None,
    }
}

/// Fold a `WhileStatement`.
///
/// Two things happen here:
///
/// * A **known-falsy** test (`while (false) …`) never runs, so the loop
///   collapses to `EmptyStatement`. This arm is unchanged by the while→for
///   work below and remains the minimal drop; preserving a `var` hoisted out
///   of the never-run body (which stays observable in the enclosing function
///   scope) is a separate, companion dead-loop `var`-extraction concern and is
///   not done here.
///
/// * Every **live** `while (cond) body` is canonicalised to
///   `for (; cond; ) body` — the shape the reference Closure Compiler always
///   emits. A `while` and a `for` with an empty init *and* empty update are
///   exactly equivalent: no init runs, and `continue` targets the test in
///   both forms (there is no update clause to fall through to). So this is a
///   pure spelling change, never a semantic one. A redundant always-truthy
///   test is then dropped to the canonical infinite `for (;;)`, mirroring
///   [`fold_for_statement`]'s truthy-test elision:
///
/// ```text
///   while (x) a();        →  for (; x; ) a();     (test kept)
///   while (1) a();        →  for (;;) a();        (truthy test dropped)
///   while (true) a();     →  for (;;) a();
/// ```
fn fold_while_statement(s: &WhileStatement, st: &mut FoldState) -> Statement {
    let test = fold_expression(&s.test, st);
    let body = fold_statement(&s.body, st);
    match literal_truthy(&test) {
        Some(false) => {
            // A `while (false)` loop never runs — its body is discarded. But a
            // hoisted body `var` still hoists to the enclosing function scope,
            // so it must be EXTRACTED, not dropped: `while(false){var x=1}` →
            // `var x;` (dropping `x` was a miscompile — `typeof x` would flip
            // from "undefined" to a ReferenceError). A body with no hoisted
            // `var` collapses to `;`. Same reversed-order extraction as the dead
            // `for`-loop; `while` has no header, so no init to append.
            let discarded = [statement_cv(&body)];
            st.record_fold_deleting(
                &s.cv,
                &discarded,
                "folded-branch",
                "while (<falsy literal>) { … }",
                "<hoisted var(s)>;",
            );
            extract_dead_loop_vars(&body, None, &s.cv)
        }
        // Live loop (`while (true)`, `while (<truthy>)`, or an unknown test):
        // rewrite to the equivalent `for`. A truthy literal test is redundant
        // in a `for` header (unlike `while`, whose test is mandatory), so it is
        // elided to `for (;;)`; any other test is carried across verbatim.
        truthy => {
            let for_test = if truthy == Some(true) {
                st.record_fold(
                    &s.cv,
                    "while-to-for-truthy",
                    "while (<truthy literal>) …",
                    "for (;;) …",
                );
                None
            } else {
                st.record_fold(
                    &s.cv,
                    "while-to-for",
                    "while (<test>) …",
                    "for (; <test>; ) …",
                );
                Some(test)
            };
            Statement::Tagged(TaggedStatement::ForStatement(ForStatement {
                cv: s.cv.clone(),
                init: None,
                test: for_test,
                update: None,
                body: Box::new(body),
            }))
        }
    }
}

/// Collect, in source (traversal) order, the names of every `var` binding that
/// hoists out of `stmt` — the dead body of a removed loop. The coverage mirrors
/// [`body_has_hoistable_var`] exactly (every binding-transparent construct a
/// `var` can hide in), but pushes each declarator's name instead of answering a
/// bool. This is what lets a dead loop with hoisted `var`s collapse to a bare
/// `var …;` declaration instead of being declined or — worse — dropped.
///
/// `BindingTarget` in this AST is only `Identifier` (destructuring patterns are
/// declined at the bridge and never reach a pass), so every declarator yields
/// exactly one name and collection never has to bail out.
fn collect_hoistable_vars(stmt: &Statement, out: &mut Vec<Identifier>) {
    match stmt {
        Statement::Declaration(Declaration::VariableDeclaration(v)) if v.kind == VarKind::Var => {
            collect_var_decl_names(v, out);
        }
        Statement::Declaration(_) => {}
        Statement::Tagged(t) => match t {
            TaggedStatement::BlockStatement(b) => {
                b.body.iter().for_each(|s| collect_hoistable_vars(s, out))
            }
            TaggedStatement::IfStatement(i) => {
                collect_hoistable_vars(&i.consequent, out);
                if let Some(a) = i.alternate.as_deref() {
                    collect_hoistable_vars(a, out);
                }
            }
            TaggedStatement::WhileStatement(w) => collect_hoistable_vars(&w.body, out),
            TaggedStatement::DoWhileStatement(d) => collect_hoistable_vars(&d.body, out),
            TaggedStatement::ForStatement(f) => {
                collect_for_init_names(f.init.as_ref(), out);
                collect_hoistable_vars(&f.body, out);
            }
            TaggedStatement::ForInStatement(f) => {
                collect_for_init_names(Some(&f.left), out);
                collect_hoistable_vars(&f.body, out);
            }
            TaggedStatement::ForOfStatement(f) => {
                collect_for_init_names(Some(&f.left), out);
                collect_hoistable_vars(&f.body, out);
            }
            TaggedStatement::LabeledStatement(l) => collect_hoistable_vars(&l.body, out),
            TaggedStatement::WithStatement(w) => collect_hoistable_vars(&w.body, out),
            TaggedStatement::SwitchStatement(s) => s
                .cases
                .iter()
                .for_each(|c| c.consequent.iter().for_each(|s| collect_hoistable_vars(s, out))),
            TaggedStatement::TryStatement(tr) => {
                tr.block.body.iter().for_each(|s| collect_hoistable_vars(s, out));
                if let Some(h) = &tr.handler {
                    h.body.body.iter().for_each(|s| collect_hoistable_vars(s, out));
                }
                if let Some(f) = &tr.finalizer {
                    f.body.iter().for_each(|s| collect_hoistable_vars(s, out));
                }
            }
            TaggedStatement::ExpressionStatement(_)
            | TaggedStatement::ReturnStatement(_)
            | TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::ThrowStatement(_)
            | TaggedStatement::EmptyStatement(_)
            | TaggedStatement::DebuggerStatement(_) => {}
        },
    }
}

/// Push each declarator name of a `var` declaration (in order).
fn collect_var_decl_names(v: &VariableDeclaration, out: &mut Vec<Identifier>) {
    for d in &v.declarations {
        let BindingTarget::Identifier(id) = &d.id;
        out.push(id.clone());
    }
}

/// A nested `for`/`for-in`/`for-of` header var (`for(var k in o)`) hoists too;
/// collect its names. `let`/`const` headers and expression heads bind nothing.
fn collect_for_init_names(init: Option<&ForInit>, out: &mut Vec<Identifier>) {
    if let Some(ForInit::VariableDeclaration(v)) = init {
        if v.kind == VarKind::Var {
            collect_var_decl_names(v, out);
        }
    }
}

/// Build the declaration that a dead loop leaves behind after its body and
/// header vars are hoisted out — matching the reference Closure Compiler.
///
/// `body` is the never-run loop body; `for_init` is the `for` header (its `var`
/// declarators run once at entry and are kept **with** their initializers). The
/// emitted `var` declares, in this exact order:
///
///   REVERSE(body-hoisted var names, initializers stripped) ++ (for-init `var`
///   declarators, original order, initializers kept)
///
/// e.g. `for(var i=0;false;){var y=2;var z=3}` → `var z,y,i=0;`. The reversal of
/// the body names, and the appended-in-order kept init declarators, are both
/// what Closure emits. Returns an `EmptyStatement` when nothing hoists (the
/// dead loop had no `var`s and no `var` header).
fn extract_dead_loop_vars(
    body: &Statement,
    for_init: Option<&ForInit>,
    cv: &Option<String>,
) -> Statement {
    let mut body_names: Vec<Identifier> = Vec::new();
    collect_hoistable_vars(body, &mut body_names);
    body_names.reverse();
    let mut declarations: Vec<VariableDeclarator> = body_names
        .into_iter()
        .map(|id| VariableDeclarator {
            cv: None,
            id: BindingTarget::Identifier(id),
            init: None,
        })
        .collect();
    if let Some(ForInit::VariableDeclaration(v)) = for_init {
        if v.kind == VarKind::Var {
            declarations.extend(v.declarations.iter().cloned());
        }
    }
    if declarations.is_empty() {
        Statement::empty_statement(EmptyStatement { cv: cv.clone() })
    } else {
        Statement::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
            cv: cv.clone(),
            kind: VarKind::Var,
            declarations,
        }))
    }
}

/// Collapse a dead control-flow construct: `surviving` is the part that is kept
/// and still runs (`None` when nothing survives — a dead `if` with no `else`),
/// and `dead` is the statically-unreachable part whose hoisted `var`s must be
/// rescued. Used for a dead `if` branch (survivor = the taken branch) and for a
/// dead `for` loop with an expression init (survivor = the once-run init `e;`,
/// dead = the never-run body).
///
/// A hoisted `var` inside the dead branch still hoists to the enclosing function
/// scope, so — exactly like a dead loop's body `var` (see
/// [`extract_dead_loop_vars`]) — it must SURVIVE the branch's removal rather than
/// be dropped. Dropping it is a miscompile: a later `z` / `typeof z` read flips
/// from reading a declared-`undefined` binding to a `ReferenceError` (or a
/// different outer `z`). We EXTRACT those bindings — initializers STRIPPED, since
/// the dead branch never runs — as a bare `var …;` placed BEFORE the surviving
/// branch, matching the reference Closure Compiler at SIMPLE byte-for-byte:
///
/// ```text
///   if (false) { var z = 1; }              →  var z;
///   if (false) { var a=1; var b=2; }       →  var b, a;      (reversed, as loops)
///   if (false) { var z = g(); } else h();  →  var z; h();    (init stripped)
///   if (true)  h(); else { var z = 1; }     →  var z; h();    (var before survivor)
/// ```
///
/// When the dead branch has no hoistable `var`, this is a plain branch pick and
/// the surviving branch (or `;`) is returned unchanged. When there IS a `var` and
/// a surviving branch, the two are wrapped in a `BlockStatement`, which
/// [`fold_program`] / [`fold_block_statement`] then splice into the enclosing
/// statement list ([`block_is_scope_safe_to_hoist`] admits a `var`-only block, so
/// the wrapper is redundant and flattens; adjacent `var`s then coalesce).
fn collapse_extracting_dead_vars(
    dead: &Statement,
    surviving: Option<Statement>,
    cv: &Option<String>,
) -> Statement {
    let mut names: Vec<Identifier> = Vec::new();
    collect_hoistable_vars(dead, &mut names);
    names.reverse();
    if names.is_empty() {
        return surviving
            .unwrap_or_else(|| Statement::empty_statement(EmptyStatement { cv: cv.clone() }));
    }
    let var_decl = Statement::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
        cv: cv.clone(),
        kind: VarKind::Var,
        declarations: names
            .into_iter()
            .map(|id| VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(id),
                init: None,
            })
            .collect(),
    }));
    match surviving {
        None => var_decl,
        Some(surv) => Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![var_decl, surv],
        }),
    }
}

/// Is a `let`/`const` `for`-header safe to *drop* when its loop is dead? Only
/// when every declarator's initializer is absent or a side-effect-free literal.
///
/// A `for` header's lexical bindings are initialized exactly once at loop entry,
/// **before** the (failing) test, per ECMAScript §14.7.4 (CreateForBindings).
/// The binding is scoped to the loop and cannot be lifted out, so an initializer
/// with a side effect (a call, a getter-bearing member access, a
/// possibly-undeclared identifier read) is observed once and must not be elided.
/// Closure keeps the whole loop in that case (`for(let i=f();0;)`); we mirror
/// that by declining the collapse. Literals never throw and have no side effect,
/// so a purely-literal header (`let i = 0`) is safe to drop with the loop.
fn lexical_header_is_droppable(v: &VariableDeclaration) -> bool {
    v.declarations
        .iter()
        .all(|d| d.init.as_ref().map(|e| literal_truthy(e).is_some()).unwrap_or(true))
}

/// Fold a `for (init; test; update) body`. When `test` is a known-falsy literal
/// the body and update never run, so the loop is dead — it collapses to just
/// its `init`, which runs once (before the first, failing, test):
///
/// ```text
///   for (; false; ) body           →  ;              (no init)
///   for (a(); false; ) body        →  a();           (expression init kept)
///   for (var i = 0; false; ) body  →  var i = 0;     (a `var` hoists — kept)
///   for (let i = 0; false; ) body  →  ;              (literal-init `let`/`const`
///                                                      is loop-scoped, so
///                                                      dropping the loop removes
///                                                      the binding unobservably)
/// ```
///
/// A hoisted body `var` still hoists to the enclosing function scope, so it is
/// EXTRACTED (initializer stripped, since the body never runs) rather than
/// dropped — including when an *expression* init means the result is two
/// statements, which are wrapped in a block that the enclosing list then
/// flattens (see [`collapse_extracting_dead_vars`]):
///
/// ```text
///   for (; false; ) { var y = 1; }    →  var y;
///   for (f(); false; ) { var x = 1; } →  var x; f();     (two statements)
/// ```
///
/// The collapse is **declined** (loop kept, test folded) only when a `let`/
/// `const` header runs a side-effecting initializer, which is loop-scoped and
/// cannot be lifted out (§14.7.4) — flagged by the security review:
///
/// ```text
///   for (let i = f(); false; ) body   →  loop kept   (lexical init runs once at
///                                                      entry; side effect can't
///                                                      be lifted out — §14.7.4)
/// ```
///
/// A **truthy** literal test is instead redundant — the loop runs forever, and
/// a `for` header can omit the test — so it is dropped: `for (; true; )` →
/// `for (;;)` (a `while (true)`, whose test is mandatory, is left alone by
/// `fold_while_statement`). A non-literal or absent test rebuilds the loop
/// unchanged. Mirrors [`fold_while_statement`]'s falsy-test removal.
fn fold_for_statement(s: &ForStatement, st: &mut FoldState) -> Statement {
    let init = s.init.as_ref().map(|i| match i {
        ForInit::VariableDeclaration(v) => {
            ForInit::VariableDeclaration(fold_variable_declaration(v, st))
        }
        ForInit::Expression(e) => ForInit::Expression(fold_expression(e, st)),
    });
    let test = s.test.as_ref().map(|e| fold_expression(e, st));
    let update = s.update.as_ref().map(|e| fold_expression(e, st));
    let body = fold_statement(&s.body, st);

    // Always-FALSE test → dead loop: it collapses to just its `init` (which
    // runs once, before the first, failing, test) — BUT only when that collapse
    // is observably equivalent. Two hazards force us to DECLINE and keep the
    // loop (both surfaced by the security review):
    //
    //   * A `let`/`const` header runs its initializer exactly once at loop
    //     entry, before the failing test. The binding is loop-scoped and can't
    //     be lifted, so a side-effecting initializer must not be dropped —
    //     `lexical_header_is_droppable` gates this (literal inits are safe).
    //   * A `var`/hoisted binding *inside* the never-run body still hoists to
    //     the function scope and stays observable — `body_has_hoistable_var`
    //     gates this. (Closure extracts such a `var`; this pass doesn't yet, so
    //     we keep the loop rather than silently drop the binding.)
    //
    // When declined we fall through and rebuild the loop unchanged (with the
    // folded falsy test) — sound, valid, and a no-op versus the pre-fold shape.
    if test.as_ref().and_then(literal_truthy) == Some(false) {
        let lexical_header_unsafe = matches!(
            &init,
            Some(ForInit::VariableDeclaration(v))
                if v.kind != VarKind::Var && !lexical_header_is_droppable(v)
        );
        if !lexical_header_unsafe {
            // A hoisted `var` inside the never-run body still hoists to the
            // enclosing function scope, so it must survive the loop's removal.
            // We EXTRACT those bindings (Closure does the same) rather than drop
            // them: `for(;false;){var x=1}` → `var x;`,
            // `for(var i=0;false;){var y=2}` → `var y,i=0;` (see
            // `extract_dead_loop_vars` for the exact reversed-body-then-init
            // order).
            //
            // An *expression* init combined with a hoisted body `var` is two
            // statements (`var x; e;`); we emit them wrapped in a block that the
            // enclosing list then flattens (`collapse_extracting_dead_vars`).
            // An expression init with no body `var` collapses to the init
            // expression (`for(a();false;) …` → `a();`), which runs once at
            // entry.
            let mut body_names = Vec::new();
            collect_hoistable_vars(&body, &mut body_names);
            let discarded = [statement_cv(&body)];
            st.record_fold_deleting(
                &s.cv,
                &discarded,
                "folded-branch",
                "for (…; <falsy literal>; …) { … }",
                "<hoisted var(s)> / <init>;",
            );
            return match &init {
                // Expression init WITH hoisted body `var`(s): two statements
                // (`var x; e;`). Extract the body `var`s BEFORE the once-run
                // init expression, wrapped in a block that `fold_program` /
                // `fold_block_statement` then splice into the enclosing list
                // (`for(f();false;){var x=1}` → `var x; f();`).
                Some(ForInit::Expression(e)) if !body_names.is_empty() => {
                    collapse_extracting_dead_vars(
                        &body,
                        Some(Statement::expression_statement(ExpressionStatement {
                            cv: s.cv.clone(),
                            expression: e.clone(),
                        })),
                        &s.cv,
                    )
                }
                // Expression init, no body vars: the init runs once, kept.
                Some(ForInit::Expression(e)) => {
                    Statement::expression_statement(ExpressionStatement {
                        cv: s.cv.clone(),
                        expression: e.clone(),
                    })
                }
                // `None` or `var`/`let`/`const` init: hoist the body `var`s
                // out and, for a `var` header, append its declarators (kept
                // with initializers). A `let`/`const` header is loop-scoped
                // and contributes nothing.
                _ => extract_dead_loop_vars(&body, init.as_ref(), &s.cv),
            };
        }
        // A side-effecting lexical header declined above — fall through to
        // rebuild the loop with the folded test.
    }

    // Always-TRUE test → the loop runs forever; the test is redundant, so drop
    // it to the canonical infinite `for (;;)` (`for (; true; )` → `for (;;)`).
    // Unlike `while (true)` — whose test is mandatory — a `for` header can omit
    // it. Init and update are preserved. A non-literal or absent test is kept.
    let test = if test.as_ref().and_then(literal_truthy) == Some(true) {
        st.record_fold(
            &s.cv,
            "for-truthy-test-dropped",
            "for (…; <truthy literal>; …)",
            "for (…;;…)",
        );
        None
    } else {
        test
    };

    // Loop-body comma-fusion: a BLOCK body whose statements are *all* plain
    // expression statements collapses to a single (possibly comma-sequenced)
    // expression statement, dropping the braces — matching Closure at SIMPLE:
    //
    //   for (…) { a(); }          →  for (…) a();          (single-stmt block unwrapped)
    //   for (…) { a(); b(); }     →  for (…) a(), b();      (comma-sequenced)
    //
    // The comma operator runs the statements left-to-right with the same side
    // effects, and a loop body discards the value, so only that left-to-right
    // ordering matters — the rewrite is behaviour-preserving. `stmts_as_sequence_expr`
    // returns `None` (leaving the block intact) for any body carrying a
    // declaration (`var`/`let`/`const`), a `break`/`continue`/`return`, a nested
    // `if`/loop, or a nested block — none of which can join a comma-sequence.
    // Only a `BlockStatement` body is a fusion candidate; a bare-statement body
    // is already brace-free. This runs *after* the body's own inner folds, so an
    // `if (x) a();` that folded to `x && a()` inside the block participates:
    // `for (…) { if (x) a(); b(); }` → `for (…) x && a(), b();`.
    let body = match &body {
        Statement::Tagged(TaggedStatement::BlockStatement(_)) => {
            if statement_is_empty(&body) {
                // Empty loop body: `for (…) {}` (or `{;;}`, `{{}}`) normalizes to
                // `for (…) ;` — the reference compiler drops the braces of a
                // do-nothing body. An empty block declares no bindings, so the
                // `;` form is behaviour-identical. This also catches a `while`
                // loop, since `while` is first rewritten to `for` (0.31.0) and
                // then re-folded here: `while (c) {}` → `for (; c;) {}` → `for
                // (; c;) ;`.
                let body_cv = statement_cv(&body);
                st.record_fold(
                    &s.cv,
                    "empty-loop-body",
                    "for (…) {}",
                    "for (…) ;",
                );
                Statement::empty_statement(EmptyStatement { cv: body_cv })
            } else {
                match stmts_as_sequence_expr(&body) {
                    Some(expr) => {
                        let body_cv = statement_cv(&body);
                        st.record_fold(
                            &s.cv,
                            "loop-body-fuse",
                            "for (…) { s1; s2; … }",
                            "for (…) s1, s2, …",
                        );
                        Statement::expression_statement(ExpressionStatement {
                            cv: body_cv,
                            expression: expr,
                        })
                    }
                    None => body,
                }
            }
        }
        _ => body,
    };

    Statement::Tagged(TaggedStatement::ForStatement(ForStatement {
        cv: s.cv.clone(),
        init,
        test,
        update,
        body: Box::new(body),
    }))
}

// =====================================================================
// Declarations
// =====================================================================

fn fold_declaration(decl: &Declaration, st: &mut FoldState) -> Declaration {
    st.visit();
    match decl {
        Declaration::VariableDeclaration(v) => {
            Declaration::VariableDeclaration(fold_variable_declaration(v, st))
        }
        Declaration::FunctionDeclaration(f) => {
            let folded_body = fold_block_statement(&f.body, st);
            // gap-015 removed (CLOC/#213): we used to split `var x = e;` inside
            // nested blocks into a hoisted `var x;` at the body top plus an
            // `x = e;` assignment at the site. Oracle probing proved upstream
            // Closure NEVER emits that split in SIMPLE output — a `var`'s
            // declaration stays exactly where it was written. The transform
            // only ever moved us away from byte-identity, so it is gone.
            Declaration::FunctionDeclaration(FunctionDeclaration {
                cv: f.cv.clone(),
                id: f.id.clone(),
                params: f.params.clone(),
                body: folded_body,
                generator: f.generator,
                is_async: f.is_async,
            })
        }
        // A class *declaration* folds inside its heritage + method bodies, like
        // `fold_class` for the expression form.
        // An import has no foldable control flow — preserve it verbatim.
        Declaration::ImportDeclaration(i) => Declaration::ImportDeclaration(i.clone()),
        Declaration::ExportNamedDeclaration(i) => Declaration::ExportNamedDeclaration(i.clone()),
        Declaration::ExportDefaultDeclaration(i) => Declaration::ExportDefaultDeclaration(i.clone()),
        Declaration::ExportAllDeclaration(i) => Declaration::ExportAllDeclaration(i.clone()),
        Declaration::ClassDeclaration(c) => {
            let (super_class, body) = fold_class_body(&c.super_class, &c.body, st);
            Declaration::ClassDeclaration(ClassDeclaration {
                cv: c.cv.clone(),
                id: c.id.clone(),
                super_class,
                body,
            })
        }
    }
}

fn fold_variable_declaration(
    v: &VariableDeclaration,
    st: &mut FoldState,
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
                init: d.init.as_ref().map(|e| fold_expression(e, st)),
            })
            .collect(),
    }
}

// =====================================================================
// Expressions — recurse only; fold-control-flow doesn't collapse
// value-level expressions (constant-fold's job). Conditional with
// a literal test IS folded for robustness when this pass runs solo.
// =====================================================================

/// Fold control flow inside the shared `[extends S] { members }` tail of a
/// class — the heritage operand (a value-position expression) and each method's
/// function *value* body (folded, then its `var`s hoisted, exactly like the
/// `FunctionExpression` arm). Reused by both the class *expression*
/// ([`fold_class`]) and the class *declaration* (the `fold_declaration` arm),
/// which share their body shape. `#[inline(never)]` so it does not inflate the
/// caller's frame (stack-overflow DoS guard).
#[inline(never)]
fn fold_class_body(
    super_class: &Option<Box<Expression>>,
    body: &[ClassMember],
    st: &mut FoldState,
) -> (Option<Box<Expression>>, Vec<ClassMember>) {
    let super_class = super_class
        .as_ref()
        .map(|s| Box::new(fold_expression(s, st)));
    let body = body
        .iter()
        .map(|m| match m {
            ClassMember::Method(md) => {
                let folded_body = fold_block_statement(&md.value.body, st);
                ClassMember::Method(MethodDefinition {
                    cv: md.cv.clone(),
                    key: md.key.clone(),
                    kind: md.kind,
                    value: FunctionExpression {
                        cv: md.value.cv.clone(),
                        id: md.value.id.clone(),
                        params: md.value.params.clone(),
                        body: folded_body,
                        generator: md.value.generator,
                        is_async: md.value.is_async,
                    },
                    computed: md.computed,
                    is_static: md.is_static,
                })
            }
            // A class field folds control flow inside its initializer (an
            // expression that runs at construction). The key is cloned; the
            // value is optional.
            ClassMember::Field(fd) => ClassMember::Field(PropertyDefinition {
                cv: fd.cv.clone(),
                key: fd.key.clone(),
                value: fd.value.as_ref().map(|v| fold_expression(v, st)),
                computed: fd.computed,
                is_static: fd.is_static,
            }),
            // A static-init block folds control flow inside its statements (they
            // run at class-definition time). Like a method body, its `var`s are
            // left where written (see the gap-015 removal note above).
            ClassMember::StaticBlock(b) => {
                ClassMember::StaticBlock(fold_block_statement(b, st))
            }
        })
        .collect();
    (super_class, body)
}

/// Fold control flow inside a class expression: delegates to
/// [`fold_class_body`] for the heritage + method bodies. `#[inline(never)]` so
/// it does not inflate `fold_expression`'s frame (stack-overflow DoS guard).
#[inline(never)]
fn fold_class(c: &ClassExpression, st: &mut FoldState) -> Expression {
    let (super_class, body) = fold_class_body(&c.super_class, &c.body, st);
    Expression::ClassExpression(ClassExpression {
        cv: c.cv.clone(),
        id: c.id.clone(),
        super_class,
        body,
    })
}

fn fold_expression(expr: &Expression, st: &mut FoldState) -> Expression {
    st.visit();
    match expr {
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        // `this` is a leaf keyword — nothing inside to fold, so it clones
        // through like the literals.
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        | Expression::UndefinedLiteral(_) => expr.clone(),

        Expression::BinaryExpression(b) => Expression::BinaryExpression(BinaryExpression {
            cv: b.cv.clone(),
            operator: b.operator,
            left: Box::new(fold_expression(&b.left, st)),
            right: Box::new(fold_expression(&b.right, st)),
        }),
        Expression::LogicalExpression(l) => Expression::LogicalExpression(LogicalExpression {
            cv: l.cv.clone(),
            operator: l.operator,
            left: Box::new(fold_expression(&l.left, st)),
            right: Box::new(fold_expression(&l.right, st)),
        }),
        Expression::UnaryExpression(u) => Expression::UnaryExpression(UnaryExpression {
            cv: u.cv.clone(),
            operator: u.operator,
            prefix: u.prefix,
            argument: Box::new(fold_expression(&u.argument, st)),
        }),
        // `++x` / `x++`: recurse into the argument; the read-modify-write is
        // preserved verbatim (it has a side effect and is not a constant).
        Expression::UpdateExpression(u) => Expression::UpdateExpression(UpdateExpression {
            cv: u.cv.clone(),
            operator: u.operator,
            prefix: u.prefix,
            argument: Box::new(fold_expression(&u.argument, st)),
        }),
        Expression::AssignmentExpression(a) => {
            Expression::AssignmentExpression(AssignmentExpression {
                cv: a.cv.clone(),
                operator: a.operator,
                left: a.left.clone(),
                right: Box::new(fold_expression(&a.right, st)),
            })
        }
        Expression::ConditionalExpression(c) => fold_conditional(c, st),
        Expression::CallExpression(c) => Expression::CallExpression(CallExpression {
            cv: c.cv.clone(),
            callee: Box::new(fold_expression(&c.callee, st)),
            arguments: c.arguments.iter().map(|a| fold_expression(a, st)).collect(),
        }),
        Expression::NewExpression(n) => Expression::NewExpression(NewExpression {
            cv: n.cv.clone(),
            callee: Box::new(fold_expression(&n.callee, st)),
            arguments: n.arguments.iter().map(|a| fold_expression(a, st)).collect(),
        }),
        Expression::SequenceExpression(s) => Expression::SequenceExpression(SequenceExpression {
            cv: s.cv.clone(),
            expressions: s.expressions.iter().map(|e| fold_expression(e, st)).collect(),
        }),
        // `` tag`a${x}b` `` — recurse into the tag callee and each `${…}`
        // substitution; the raw quasi strings are opaque and untouched.
        Expression::TaggedTemplateExpression(t) => {
            Expression::TaggedTemplateExpression(TaggedTemplateExpression {
                cv: t.cv.clone(),
                tag: Box::new(fold_expression(&t.tag, st)),
                quasi: TemplateLiteral {
                    cv: t.quasi.cv.clone(),
                    quasis: t.quasi.quasis.clone(),
                    expressions: t
                        .quasi
                        .expressions
                        .iter()
                        .map(|e| fold_expression(e, st))
                        .collect(),
                },
            })
        }
        // `...arg` — recurse into the spread argument; the spread is kept.
        Expression::SpreadElement(s) => Expression::SpreadElement(SpreadElement {
            cv: s.cv.clone(),
            argument: Box::new(fold_expression(&s.argument, st)),
        }),
        Expression::YieldExpression(y) => Expression::YieldExpression(YieldExpression {
            cv: y.cv.clone(),
            delegate: y.delegate,
            argument: y.argument.as_ref().map(|a| Box::new(fold_expression(a, st))),
        }),
        Expression::AwaitExpression(a) => Expression::AwaitExpression(AwaitExpression {
            cv: a.cv.clone(),
            argument: Box::new(fold_expression(&a.argument, st)),
        }),
        Expression::ImportExpression(e) => Expression::ImportExpression(ImportExpression {
            cv: e.cv.clone(),
            source: Box::new(fold_expression(&e.source, st)),
        }),
        Expression::MemberExpression(m) => Expression::MemberExpression(MemberExpression {
            cv: m.cv.clone(),
            object: Box::new(fold_expression(&m.object, st)),
            property: Box::new(fold_expression(&m.property, st)),
            computed: m.computed,
        }),
        // `a?.b` / `a?.[k]` — recurse into object and property exactly as a
        // plain member access; the optional-member node is kept verbatim.
        Expression::OptionalMemberExpression(m) => {
            Expression::OptionalMemberExpression(OptionalMemberExpression {
                cv: m.cv.clone(),
                object: Box::new(fold_expression(&m.object, st)),
                property: Box::new(fold_expression(&m.property, st)),
                computed: m.computed,
            })
        }
        // `a?.()` — recurse into callee and arguments as for an ordinary call.
        Expression::OptionalCallExpression(c) => {
            Expression::OptionalCallExpression(OptionalCallExpression {
                cv: c.cv.clone(),
                callee: Box::new(fold_expression(&c.callee, st)),
                arguments: c.arguments.iter().map(|a| fold_expression(a, st)).collect(),
            })
        }
        // A chain expression transparently wraps its optional-chain spine —
        // recurse into the inner expression and rewrap.
        Expression::ChainExpression(c) => Expression::ChainExpression(ChainExpression {
            cv: c.cv.clone(),
            expression: Box::new(fold_expression(&c.expression, st)),
        }),
        Expression::ArrayExpression(a) => Expression::ArrayExpression(ArrayExpression {
            cv: a.cv.clone(),
            elements: a
                .elements
                .iter()
                .map(|e| e.as_ref().map(|x| fold_expression(x, st)))
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
                                PropertyKey::Expression(Box::new(fold_expression(e, st)))
                            }
                        },
                        value: Box::new(fold_expression(&p.value, st)),
                        computed: p.computed,
                        shorthand: p.shorthand,
                        method: p.method,
                    }),
                    // Object spread `...expr` — recurse into the spread argument.
                    ObjectMember::Spread(s) => ObjectMember::Spread(SpreadElement {
                        cv: s.cv.clone(),
                        argument: Box::new(fold_expression(&s.argument, st)),
                    }),
                })
                .collect(),
        }),
        // Fold control flow inside a function *value*'s body, mirroring
        // the `FunctionDeclaration` arm in `fold_declaration` (fold the
        // body; `var`s stay where written — see the gap-015 removal note).
        Expression::FunctionExpression(f) => {
            let folded_body = fold_block_statement(&f.body, st);
            Expression::FunctionExpression(FunctionExpression {
                cv: f.cv.clone(),
                id: f.id.clone(),
                params: f.params.clone(),
                body: folded_body,
                generator: f.generator,
                is_async: f.is_async,
            })
        }
        Expression::ClassExpression(c) => fold_class(c, st),
        // Fold control flow inside an arrow-value's body. A block body is
        // folded, `var`s left where written (like a function body); a
        // concise (expression) body declares no `var`s, so it only needs
        // its single expression folded.
        Expression::ArrowFunctionExpression(a) => {
            let body = match &a.body {
                ArrowBody::Block(b) => ArrowBody::Block(fold_block_statement(b, st)),
                ArrowBody::Expression(e) => ArrowBody::Expression(Box::new(fold_expression(e, st))),
            };
            Expression::ArrowFunctionExpression(ArrowFunctionExpression {
                cv: a.cv.clone(),
                params: a.params.clone(),
                body,
                is_async: a.is_async,
            })
        }
        // Fold control flow inside a template literal's `${…}` expressions;
        // the `quasis` are fixed string segments.
        Expression::TemplateLiteral(t) => Expression::TemplateLiteral(TemplateLiteral {
            cv: t.cv.clone(),
            quasis: t.quasis.clone(),
            expressions: t.expressions.iter().map(|e| fold_expression(e, st)).collect(),
        }),
    }
}

/// Fold `test ? a : b` when the test is a known literal. Same
/// shape as constant-fold's handling; we redundantly do it here
/// so the pass produces correct results even when run solo
/// without constant-fold first.
fn fold_conditional(c: &ConditionalExpression, st: &mut FoldState) -> Expression {
    let test = fold_expression(&c.test, st);
    let consequent = fold_expression(&c.consequent, st);
    let alternate = fold_expression(&c.alternate, st);

    // (CLOC12.25 / gap-018): De Morgan negation-swap for ternary.
    //
    //   !x ? c : a    →    x ? a : c
    //
    // Mirrors the IfStatement case in `fold_if_statement` — same
    // semantic justification: `!x` and `x` make the same single
    // ToBoolean(x) decision, just flipped; swapping the branches
    // preserves observable behaviour. Unlike the IfStatement case,
    // a ConditionalExpression always has both arms (`alternate`
    // is not Optional), so no `is_some()` guard is needed.
    let (test, consequent, alternate) = if let Expression::UnaryExpression(u) = test {
        if u.operator == UnaryOperator::Not {
            let inner = *u.argument;
            st.record_fold(
                &c.cv,
                "de-morgan-swap-not-ternary",
                "!<inner> ? <c> : <a>",
                "<inner> ? <a> : <c>",
            );
            (inner, alternate, consequent)
        } else {
            (Expression::UnaryExpression(u), consequent, alternate)
        }
    } else {
        (test, consequent, alternate)
    };

    match literal_truthy(&test) {
        Some(true) => {
            // The `alternate` arm is statically unreachable and is
            // discarded — tombstone it so its span stays auditable.
            let discarded = [expression_cv(&alternate)];
            st.record_fold_deleting(
                &c.cv,
                &discarded,
                "folded-branch",
                "(<truthy literal>) ? … : …",
                "consequent",
            );
            consequent
        }
        Some(false) => {
            // The `consequent` arm is statically unreachable and is
            // discarded — tombstone it so its span stays auditable.
            let discarded = [expression_cv(&consequent)];
            st.record_fold_deleting(
                &c.cv,
                &discarded,
                "folded-branch",
                "(<falsy literal>) ? … : …",
                "alternate",
            );
            alternate
        }
        None => Expression::ConditionalExpression(ConditionalExpression {
            cv: c.cv.clone(),
            test: Box::new(test),
            consequent: Box::new(consequent),
            alternate: Box::new(alternate),
        }),
    }
}

// =====================================================================
// Helpers
// =====================================================================

/// JS truthiness for literal expressions (matches the helper of
/// the same name in `closure-pass-constant-fold` — kept local to
/// avoid a cross-pass dependency).
fn literal_truthy(expr: &Expression) -> Option<bool> {
    match expr {
        Expression::BooleanLiteral(b) => Some(b.value),
        Expression::NumericLiteral(n) => Some(n.value != 0.0 && !n.value.is_nan()),
        Expression::StringLiteral(s) => Some(!s.value.is_empty()),
        Expression::NullLiteral(_) => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_closure_pass_constant_fold::ConstantFoldPass;
    use coding_adventures_closure_pass_pipeline::{PassPipeline, PipelineOutput};
    use coding_adventures_javascript_ast::{
        statement::TaggedStatement, BinaryOperator, BooleanLiteral, Identifier, NullLiteral,
        NumericLiteral, SourceType, StringLiteral,
    };
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }
    fn untraced_program() -> Program {
        Program::new_untraced(EsVersion::Es2025, SourceType::Module)
    }

    fn boolean(v: bool, cv: Option<&str>) -> Expression {
        Expression::BooleanLiteral(BooleanLiteral {
            cv: cv.map(|s| s.to_string()),
            value: v,
        })
    }
    fn num(v: f64, cv: Option<&str>) -> Expression {
        Expression::NumericLiteral(NumericLiteral {
            cv: cv.map(|s| s.to_string()),
            value: v,
            raw: v.to_string(),
        })
    }
    fn string(v: &str, cv: Option<&str>) -> Expression {
        Expression::StringLiteral(StringLiteral {
            cv: cv.map(|s| s.to_string()),
            value: v.to_string(),
            raw: format!("\"{}\"", v),
        })
    }
    fn null(cv: Option<&str>) -> Expression {
        Expression::NullLiteral(NullLiteral {
            cv: cv.map(|s| s.to_string()),
        })
    }
    fn ident(name: &str) -> Expression {
        Expression::Identifier(Identifier {
            cv: None,
            name: name.to_string(),
        })
    }
    fn expr_stmt(expr: Expression, cv: Option<&str>) -> Statement {
        Statement::expression_statement(ExpressionStatement {
            cv: cv.map(|s| s.to_string()),
            expression: expr,
        })
    }

    fn run_pass(prog: Program) -> (Program, Vec<Contribution>, bool, u32) {
        let pass = FoldControlFlowPass::new();
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

    fn first_stmt(prog: &Program) -> &Statement {
        let ProgramItem::Statement(s) = &prog.body[0] else {
            panic!("expected Statement; got {:?}", prog.body[0]);
        };
        s
    }

    // ---------------- CV deletion provenance (#89) -------------
    //
    // Mirror the production pipeline: the lexer/parser `create` a CV
    // entry per node and stamp its id onto the AST. So here we `create`
    // the discarded branch's entry FIRST, stamp its id onto the node,
    // then run the pass — otherwise `cv.delete` has no entry to
    // tombstone and the assertion would be vacuous. Property under test:
    // when fold-control-flow *eliminates* a branch, that branch's CV
    // entry survives in the log with `DeletionRecord{source:
    // "fold-control-flow", reason:"folded-branch"}`, so "what happened
    // to this code?" stays answerable.

    /// Like [`run_pass`] but threads the caller's CV log through so its
    /// `DeletionRecord`s can be inspected after the pass returns.
    fn run_capturing_cv(prog: &Program, cv: &mut CVLog) -> Program {
        let sidecar = Sidecar::new();
        let ctx = PassContext {
            program: prog,
            sidecar: &sidecar,
            cv,
        };
        FoldControlFlowPass::new()
            .run(ctx)
            .expect("pass should succeed")
            .program
    }

    #[test]
    fn if_true_tombstones_discarded_alternate() {
        // `if (true) kept; else dead;` → `kept;` — the `dead` alternate
        // is eliminated and must be tombstoned.
        let mut log = CVLog::new(true);
        let alt_id = log.create(None);
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: boolean(true, None),
            consequent: Box::new(expr_stmt(ident("kept"), None)),
            alternate: Some(Box::new(expr_stmt(ident("dead"), Some(alt_id.as_str())))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);

        let _out = run_capturing_cv(&prog, &mut log);

        let del = log
            .get(&alt_id)
            .unwrap()
            .deleted
            .as_ref()
            .expect("the discarded `else` branch must be tombstoned");
        assert_eq!(del.source, "fold-control-flow");
        assert_eq!(del.reason, "folded-branch");
        assert_eq!(
            del.meta.get("container_cv").and_then(|v| v.as_str()),
            Some("if.1"),
            "tombstone should record the enclosing `if`'s cv"
        );
    }

    #[test]
    fn if_false_tombstones_discarded_consequent() {
        // `if (false) dead; else kept;` → `kept;` — the `dead`
        // consequent is eliminated and must be tombstoned.
        let mut log = CVLog::new(true);
        let cons_id = log.create(None);
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.2".to_string()),
            test: boolean(false, None),
            consequent: Box::new(expr_stmt(ident("dead"), Some(cons_id.as_str()))),
            alternate: Some(Box::new(expr_stmt(ident("kept"), None))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);

        let _out = run_capturing_cv(&prog, &mut log);

        let del = log
            .get(&cons_id)
            .unwrap()
            .deleted
            .as_ref()
            .expect("the discarded `then` branch must be tombstoned");
        assert_eq!(del.reason, "folded-branch");
    }

    #[test]
    fn while_false_tombstones_discarded_body() {
        // `while (false) body;` → `;` — the loop never runs, so its body
        // is eliminated and must be tombstoned.
        let mut log = CVLog::new(true);
        let body_id = log.create(None);
        let w = Statement::while_statement(WhileStatement {
            cv: Some("w.1".to_string()),
            test: boolean(false, None),
            body: Box::new(expr_stmt(ident("body"), Some(body_id.as_str()))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(w)]);

        let _out = run_capturing_cv(&prog, &mut log);

        let del = log
            .get(&body_id)
            .unwrap()
            .deleted
            .as_ref()
            .expect("the eliminated `while (false)` body must be tombstoned");
        assert_eq!(del.reason, "folded-branch");
    }

    // ---------------- for(;false;) removal ------------------------------

    fn for_stmt(init: Option<ForInit>, test: Option<Expression>, body: Statement) -> Statement {
        Statement::for_statement(ForStatement {
            cv: Some("for.1".to_string()),
            init,
            test,
            update: None,
            body: Box::new(body),
        })
    }
    fn var_init(kind: VarKind, name: &str) -> ForInit {
        use coding_adventures_javascript_ast::{BindingTarget, VariableDeclaration, VariableDeclarator};
        ForInit::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(Identifier { cv: None, name: name.to_string() }),
                init: Some(num(0.0, None)),
            }],
        })
    }
    fn call_expr(name: &str) -> Expression {
        Expression::CallExpression(coding_adventures_javascript_ast::CallExpression {
            cv: None,
            callee: Box::new(ident(name)),
            arguments: vec![],
        })
    }

    /// `for (; false; ) body` — no init, dead loop → `;`.
    #[test]
    fn for_false_no_init_removed() {
        let f = for_stmt(None, Some(boolean(false, None)), expr_stmt(ident("body"), None));
        let (out, _, changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(changed);
        assert!(
            matches!(first_stmt(&out), Statement::Tagged(TaggedStatement::EmptyStatement(_))),
            "for(;false;) must collapse to EmptyStatement; got {:?}",
            first_stmt(&out)
        );
    }

    /// `for (;;) { a(); b(); }` → `for (;;) a(), b();` — a block body of plain
    /// expression statements fuses to a comma-sequenced single statement.
    #[test]
    fn for_body_multi_expr_fuses_to_sequence() {
        let body = block(
            Some("blk.1"),
            vec![
                expr_stmt(call_expr("a"), None),
                expr_stmt(call_expr("b"), None),
            ],
        );
        let f = for_stmt(None, None, body);
        let (out, contribs, changed, _) =
            run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(changed);
        assert!(contribs.iter().any(|c| c.tag == "loop-body-fuse"));
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ForStatement(f)) => match f.body.as_ref() {
                Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
                    match &es.expression {
                        Expression::SequenceExpression(s) => assert_eq!(s.expressions.len(), 2),
                        other => panic!("expected a 2-element sequence; got {other:?}"),
                    }
                }
                other => panic!("expected a fused expression-statement body; got {other:?}"),
            },
            other => panic!("expected ForStatement; got {other:?}"),
        }
    }

    /// `do { a(); b(); } while(c);` → `do a(), b(); while(c);` — a do-while
    /// block body of plain expression statements fuses to a comma-sequenced
    /// single statement, exactly like `for`/`while`.
    #[test]
    fn do_while_body_multi_expr_fuses_to_sequence() {
        let body = block(
            Some("blk.1"),
            vec![
                expr_stmt(call_expr("a"), None),
                expr_stmt(call_expr("b"), None),
            ],
        );
        let dw = Statement::Tagged(TaggedStatement::DoWhileStatement(DoWhileStatement {
            cv: Some("dw.1".to_string()),
            body: Box::new(body),
            test: ident("c"),
        }));
        let (out, contribs, changed, _) =
            run_pass(program().with_body(vec![ProgramItem::Statement(dw)]));
        assert!(changed);
        assert!(contribs.iter().any(|c| c.tag == "loop-body-fuse"));
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::DoWhileStatement(d)) => match d.body.as_ref() {
                Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
                    match &es.expression {
                        Expression::SequenceExpression(s) => assert_eq!(s.expressions.len(), 2),
                        other => panic!("expected a 2-element sequence; got {other:?}"),
                    }
                }
                other => panic!("expected a fused expression-statement body; got {other:?}"),
            },
            other => panic!("expected DoWhileStatement; got {other:?}"),
        }
    }

    /// `do {} while(c);` → an empty-bodied do-while lowers to the equivalent
    /// `while`, which the pass rewrites to `for(; c;)` (with the empty body
    /// normalized to `;`). The result is a `ForStatement`, not a `DoWhile`.
    #[test]
    fn empty_do_while_lowers_to_for() {
        let dw = Statement::Tagged(TaggedStatement::DoWhileStatement(DoWhileStatement {
            cv: Some("dw.1".to_string()),
            body: Box::new(block(Some("blk.1"), vec![])),
            test: ident("c"),
        }));
        let (out, contribs, changed, _) =
            run_pass(program().with_body(vec![ProgramItem::Statement(dw)]));
        assert!(changed);
        assert!(contribs.iter().any(|c| c.tag == "empty-do-while-to-while"));
        // One pass lowers `do{}while(c)` to `for(;c;){}`; the empty-block -> `;`
        // normalization runs on the next sweep (covered by
        // `for_empty_block_body_normalizes_to_empty_statement`). Either way the
        // body is empty, and it is now a `for`, not a `do`.
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ForStatement(f)) => assert!(
                statement_is_empty(f.body.as_ref()),
                "empty do-while must lower to a for with an empty body; got {:?}",
                f.body
            ),
            other => panic!("expected ForStatement; got {other:?}"),
        }
    }

    /// `do { var x = 1; a(); } while(c);` — a do-while body carrying a `var`
    /// declaration is NOT fused (a declaration can't join a comma-sequence);
    /// the block is kept intact.
    #[test]
    fn do_while_body_with_var_decl_not_fused() {
        use coding_adventures_javascript_ast::{
            BindingTarget, VariableDeclaration, VariableDeclarator,
        };
        let var_x = Statement::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind: VarKind::Var,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(Identifier { cv: None, name: "x".to_string() }),
                init: Some(num(1.0, None)),
            }],
        }));
        let body = block(Some("blk.1"), vec![var_x, expr_stmt(call_expr("a"), None)]);
        let dw = Statement::Tagged(TaggedStatement::DoWhileStatement(DoWhileStatement {
            cv: None,
            body: Box::new(body),
            test: ident("c"),
        }));
        let (out, _, _, _) = run_pass(program().with_body(vec![ProgramItem::Statement(dw)]));
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::DoWhileStatement(d)) => assert!(
                matches!(d.body.as_ref(), Statement::Tagged(TaggedStatement::BlockStatement(_))),
                "var-carrying do-while body must stay a block; got {:?}",
                d.body
            ),
            other => panic!("expected DoWhileStatement; got {other:?}"),
        }
    }

    /// `for (;;) { a(); }` → `for (;;) a();` — a single-statement block is
    /// unwrapped to the bare expression statement (no comma wrapper).
    #[test]
    fn for_body_single_expr_unwraps_block() {
        let body = block(Some("blk.1"), vec![expr_stmt(call_expr("a"), None)]);
        let f = for_stmt(None, None, body);
        let (out, _, changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(changed);
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ForStatement(f)) => match f.body.as_ref() {
                Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
                    assert!(matches!(&es.expression, Expression::CallExpression(_)));
                }
                other => panic!("expected a bare expression-statement body; got {other:?}"),
            },
            other => panic!("expected ForStatement; got {other:?}"),
        }
    }

    /// `for (;;) {}` → `for (;;) ;` — an empty block body normalizes to an
    /// empty statement (dropping the braces). Also covers `while (c) {}`, which
    /// first lowers to `for` and then re-folds through this same path.
    #[test]
    fn for_empty_block_body_normalizes_to_empty_statement() {
        let body = block(Some("blk.1"), vec![]);
        let f = for_stmt(None, None, body);
        let (out, contribs, changed, _) =
            run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(changed);
        assert!(contribs.iter().any(|c| c.tag == "empty-loop-body"));
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ForStatement(f)) => assert!(
                matches!(f.body.as_ref(), Statement::Tagged(TaggedStatement::EmptyStatement(_))),
                "empty for body must normalize to `;`; got {:?}",
                f.body
            ),
            other => panic!("expected ForStatement; got {other:?}"),
        }
    }

    /// `for (;;) { var x = 0; a(); }` — a body carrying a declaration is NOT
    /// fused (a `var` can't join a comma-sequence): the block is kept intact.
    #[test]
    fn for_body_with_var_decl_not_fused() {
        use coding_adventures_javascript_ast::{
            BindingTarget, VariableDeclaration, VariableDeclarator,
        };
        let var_x = Statement::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind: VarKind::Var,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(Identifier { cv: None, name: "x".to_string() }),
                init: Some(num(0.0, None)),
            }],
        }));
        let body = block(Some("blk.1"), vec![var_x, expr_stmt(call_expr("a"), None)]);
        let f = for_stmt(None, None, body);
        let (out, _, _, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ForStatement(f)) => {
                assert!(
                    matches!(f.body.as_ref(), Statement::Tagged(TaggedStatement::BlockStatement(_))),
                    "a var-bearing body must stay a block; got {:?}",
                    f.body
                );
            }
            other => panic!("expected ForStatement; got {other:?}"),
        }
    }

    /// `for (a(); false; ) body` → `a();` — the expression init runs once.
    #[test]
    fn for_false_expression_init_kept() {
        let f = for_stmt(
            Some(ForInit::Expression(call_expr("a"))),
            Some(boolean(false, None)),
            expr_stmt(ident("body"), None),
        );
        let (out, _, changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(changed);
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
                assert!(matches!(&es.expression, Expression::CallExpression(_)), "init call kept");
            }
            other => panic!("expected the init call as an ExpressionStatement; got {:?}", other),
        }
    }

    /// `for (var i = 0; false; ) body` → `var i = 0;` — a `var` hoists, so kept.
    #[test]
    fn for_false_var_init_kept_as_declaration() {
        let f = for_stmt(Some(var_init(VarKind::Var, "i")), Some(boolean(false, None)), expr_stmt(ident("body"), None));
        let (out, _, changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(changed);
        assert!(
            matches!(
                first_stmt(&out),
                Statement::Declaration(Declaration::VariableDeclaration(_))
            ),
            "for(var i=0;false;) must keep the var declaration; got {:?}",
            first_stmt(&out)
        );
    }

    /// `for (let i = 0; false; ) body` → `;` — a `let` is loop-scoped, so dropping
    /// the whole loop removes the binding with no observable effect.
    #[test]
    fn for_false_let_init_removed() {
        let f = for_stmt(Some(var_init(VarKind::Let, "i")), Some(boolean(false, None)), expr_stmt(ident("body"), None));
        let (out, _, changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(changed);
        assert!(
            matches!(first_stmt(&out), Statement::Tagged(TaggedStatement::EmptyStatement(_))),
            "for(let i=0;false;) must collapse to EmptyStatement; got {:?}",
            first_stmt(&out)
        );
    }

    /// A non-literal test keeps the loop: `for (; x; ) body` is unchanged.
    #[test]
    fn for_non_literal_test_kept() {
        let f = for_stmt(None, Some(ident("x")), expr_stmt(ident("body"), None));
        let (out, _, _changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(
            matches!(first_stmt(&out), Statement::Tagged(TaggedStatement::ForStatement(_))),
            "for(;x;) must be kept; got {:?}",
            first_stmt(&out)
        );
    }

    /// An always-true test is dropped: `for (; true; ) body` → `for (;;) body`
    /// (the loop stays live, but the redundant test is removed).
    #[test]
    fn for_truthy_test_dropped() {
        let f = for_stmt(None, Some(boolean(true, None)), expr_stmt(ident("body"), None));
        let (out, _, changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(changed);
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ForStatement(fs)) => {
                assert!(fs.test.is_none(), "the truthy test must be dropped; got {:?}", fs.test);
            }
            other => panic!("for(;true;) must stay a (test-less) for loop; got {:?}", other),
        }
    }

    /// `for (let i = f(); false; ) body` — a `let`/`const` header initializes
    /// its bindings exactly once at loop entry, so a side-effecting initializer
    /// can't be elided. Closure keeps the whole loop; we DECLINE the collapse
    /// (security-review Finding 1).
    #[test]
    fn for_false_let_init_side_effect_keeps_loop() {
        use coding_adventures_javascript_ast::{
            BindingTarget, VariableDeclaration, VariableDeclarator,
        };
        let init = ForInit::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind: VarKind::Let,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(Identifier {
                    cv: None,
                    name: "i".to_string(),
                }),
                init: Some(call_expr("f")),
            }],
        });
        let f = for_stmt(Some(init), Some(boolean(false, None)), expr_stmt(ident("body"), None));
        let (out, _, _changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(
            matches!(first_stmt(&out), Statement::Tagged(TaggedStatement::ForStatement(_))),
            "for(let i=f();false;) must keep the loop (lexical init side effect); got {:?}",
            first_stmt(&out)
        );
    }

    /// Assert the folded statement is a `var` declaration binding exactly
    /// `names` (in order), each with its initializer STRIPPED (a hoisted body
    /// var extracted from a dead loop). Panics with the actual shape otherwise.
    fn assert_hoisted_var_names(stmt: &Statement, names: &[&str]) {
        match stmt {
            Statement::Declaration(Declaration::VariableDeclaration(v)) => {
                assert_eq!(v.kind, VarKind::Var, "expected a `var` declaration; got {:?}", v.kind);
                let got: Vec<(&str, bool)> = v
                    .declarations
                    .iter()
                    .map(|d| {
                        let coding_adventures_javascript_ast::BindingTarget::Identifier(id) = &d.id;
                        (id.name.as_str(), d.init.is_none())
                    })
                    .collect();
                let want: Vec<(&str, bool)> = names.iter().map(|n| (*n, true)).collect();
                assert_eq!(got, want, "hoisted var names/stripped mismatch");
            }
            other => panic!("expected an extracted `var` declaration for {names:?}; got {other:?}"),
        }
    }

    /// `for (; false; ) { var x = 1; }` — a body `var` hoists to the enclosing
    /// function scope, so the dead loop EXTRACTS it (initializer stripped, since
    /// the body never runs): `var x;`. Dropping it would be a miscompile.
    #[test]
    fn for_false_body_hoisted_var_extracted() {
        use coding_adventures_javascript_ast::{
            BindingTarget, VariableDeclaration, VariableDeclarator,
        };
        let var_x = Statement::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind: VarKind::Var,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(Identifier {
                    cv: None,
                    name: "x".to_string(),
                }),
                init: Some(num(1.0, None)),
            }],
        }));
        let body = Statement::Tagged(TaggedStatement::BlockStatement(BlockStatement {
            cv: None,
            body: vec![var_x],
        }));
        let f = for_stmt(None, Some(boolean(false, None)), body);
        let (out, _, changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(changed);
        assert_hoisted_var_names(first_stmt(&out), &["x"]);
    }

    /// `for (; false; ) { function g(){} }` — a block-scoped `function` is *not*
    /// a hoisting hazard (Closure drops it with the dead loop), so the collapse
    /// still fires and the loop becomes `;`. Guards against over-declining.
    #[test]
    fn for_false_body_function_decl_collapses() {
        let g = Statement::Declaration(Declaration::FunctionDeclaration(FunctionDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "g".to_string(),
            },
            params: vec![],
            body: BlockStatement {
                cv: None,
                body: vec![],
            },
            generator: false,
            is_async: false,
        }));
        let body = Statement::Tagged(TaggedStatement::BlockStatement(BlockStatement {
            cv: None,
            body: vec![g],
        }));
        let f = for_stmt(None, Some(boolean(false, None)), body);
        let (out, _, changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(changed);
        assert!(
            matches!(first_stmt(&out), Statement::Tagged(TaggedStatement::EmptyStatement(_))),
            "for(;false;){{function g(){{}}}} must collapse to EmptyStatement; got {:?}",
            first_stmt(&out)
        );
    }

    /// Build a bare `var <name> = 1;` statement (a hoistable binding).
    fn var_decl_stmt(name: &str) -> Statement {
        use coding_adventures_javascript_ast::{
            BindingTarget, VariableDeclaration, VariableDeclarator,
        };
        Statement::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind: VarKind::Var,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(Identifier {
                    cv: None,
                    name: name.to_string(),
                }),
                init: Some(num(1.0, None)),
            }],
        }))
    }
    fn blk(stmts: Vec<Statement>) -> BlockStatement {
        BlockStatement {
            cv: None,
            body: stmts,
        }
    }

    /// `for (; false; ) { try {} finally { var x = 1; } }` — the hoistable `var`
    /// hides in a `finally` block; the exhaustive collector descends into `try`
    /// bodies, so it is EXTRACTED: `var x;`.
    #[test]
    fn for_false_var_in_finally_extracted() {
        use coding_adventures_javascript_ast::TryStatement;
        let try_stmt = Statement::Tagged(TaggedStatement::TryStatement(TryStatement {
            cv: None,
            block: blk(vec![]),
            handler: None,
            finalizer: Some(blk(vec![var_decl_stmt("x")])),
        }));
        let body = Statement::Tagged(TaggedStatement::BlockStatement(blk(vec![try_stmt])));
        let f = for_stmt(None, Some(boolean(false, None)), body);
        let (out, _, changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(changed);
        assert_hoisted_var_names(first_stmt(&out), &["x"]);
    }

    /// `for (; false; ) for (var i = 0; c; ) {}` — the hoistable `var i` hides in
    /// a nested `for`-header; the collector inspects nested loop headers, so it
    /// is EXTRACTED (initializer stripped): `var i;`.
    #[test]
    fn for_false_var_in_nested_for_header_extracted() {
        let inner = Statement::for_statement(ForStatement {
            cv: None,
            init: Some(var_init(VarKind::Var, "i")),
            test: Some(ident("c")),
            update: None,
            body: Box::new(Statement::empty_statement(EmptyStatement { cv: None })),
        });
        let f = for_stmt(None, Some(boolean(false, None)), inner);
        let (out, _, changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(changed);
        assert_hoisted_var_names(first_stmt(&out), &["i"]);
    }

    /// `for (; false; ) { var x = 1; var y = 2; }` — body vars are extracted in
    /// REVERSED source order (Closure's emission): `var y,x;`.
    #[test]
    fn for_false_body_vars_extracted_reversed() {
        let body = Statement::Tagged(TaggedStatement::BlockStatement(blk(vec![
            var_decl_stmt("x"),
            var_decl_stmt("y"),
        ])));
        let f = for_stmt(None, Some(boolean(false, None)), body);
        let (out, _, changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(changed);
        assert_hoisted_var_names(first_stmt(&out), &["y", "x"]);
    }

    /// `for (var i = 0; false; ) { var y = 2; }` — reversed body vars first, then
    /// the for-init `var` declarators appended in order WITH initializers:
    /// `var y, i = 0;` (only the `i=0` keeps its initializer).
    #[test]
    fn for_false_body_var_plus_init_var_extracted() {
        let body =
            Statement::Tagged(TaggedStatement::BlockStatement(blk(vec![var_decl_stmt("y")])));
        let f = for_stmt(Some(var_init(VarKind::Var, "i")), Some(boolean(false, None)), body);
        let (out, _, changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(f)]));
        assert!(changed);
        match first_stmt(&out) {
            Statement::Declaration(Declaration::VariableDeclaration(v)) => {
                assert_eq!(v.declarations.len(), 2);
                let coding_adventures_javascript_ast::BindingTarget::Identifier(id0) =
                    &v.declarations[0].id;
                assert_eq!(id0.name, "y");
                assert!(v.declarations[0].init.is_none(), "body var y stripped");
                let coding_adventures_javascript_ast::BindingTarget::Identifier(id1) =
                    &v.declarations[1].id;
                assert_eq!(id1.name, "i");
                assert!(v.declarations[1].init.is_some(), "init var i keeps its initializer");
            }
            other => panic!("expected `var y,i=0;`; got {other:?}"),
        }
    }

    /// `while (false) { var x = 1; }` — a hoisted body `var` is EXTRACTED, not
    /// dropped: `var x;`. Dropping it (the prior behavior) was a miscompile.
    #[test]
    fn while_false_hoisted_var_extracted() {
        use coding_adventures_javascript_ast::WhileStatement;
        let body =
            Statement::Tagged(TaggedStatement::BlockStatement(blk(vec![var_decl_stmt("x")])));
        let w = Statement::Tagged(TaggedStatement::WhileStatement(WhileStatement {
            cv: None,
            test: boolean(false, None),
            body: Box::new(body),
        }));
        let (out, _, changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(w)]));
        assert!(changed);
        assert_hoisted_var_names(first_stmt(&out), &["x"]);
    }

    /// `while (false) { f(); }` — no hoisted `var`, so the dead loop collapses to
    /// `;` (EmptyStatement), as before.
    #[test]
    fn while_false_no_var_collapses_to_empty() {
        use coding_adventures_javascript_ast::WhileStatement;
        let body = Statement::Tagged(TaggedStatement::BlockStatement(blk(vec![expr_stmt(
            call_expr("f"),
            None,
        )])));
        let w = Statement::Tagged(TaggedStatement::WhileStatement(WhileStatement {
            cv: None,
            test: boolean(false, None),
            body: Box::new(body),
        }));
        let (out, _, changed, _) = run_pass(program().with_body(vec![ProgramItem::Statement(w)]));
        assert!(changed);
        assert!(
            matches!(first_stmt(&out), Statement::Tagged(TaggedStatement::EmptyStatement(_))),
            "while(false){{f()}} must collapse to `;`; got {:?}",
            first_stmt(&out)
        );
    }

    #[test]
    fn dead_code_after_terminator_is_tombstoned() {
        // `{ return; dead; }` — `dead` is unreachable after the `return`
        // and is eliminated by the block dead-code drop, so its span must
        // be tombstoned (matching what DCE records for the same drop).
        let mut log = CVLog::new(true);
        let dead_id = log.create(None);
        let block = Statement::block_statement(BlockStatement {
            cv: Some("blk.1".to_string()),
            body: vec![
                Statement::return_statement(ReturnStatement {
                    cv: None,
                    argument: None,
                }),
                expr_stmt(ident("dead"), Some(dead_id.as_str())),
            ],
        });
        let prog = program().with_body(vec![ProgramItem::Statement(block)]);

        let _out = run_capturing_cv(&prog, &mut log);

        let del = log
            .get(&dead_id)
            .unwrap()
            .deleted
            .as_ref()
            .expect("a statement after a terminator must be tombstoned");
        assert_eq!(del.source, "fold-control-flow");
        assert_eq!(del.reason, "removed-dead-code");
        assert_eq!(
            del.meta.get("container_cv").and_then(|v| v.as_str()),
            Some("blk.1"),
            "tombstone should record the enclosing block's cv"
        );
    }

    #[test]
    fn ternary_rewrite_does_not_tombstone_preserved_arms() {
        // `if (x) foo(); else bar();` → `x ? foo() : bar();` — both arms
        // are PRESERVED inside the ternary (a rewrite, not a deletion),
        // so neither is tombstoned. This pins `record_fold_deleting` to
        // genuine eliminations and off the rewrite paths.
        let mut log = CVLog::new(true);
        let c_id = log.create(None);
        let a_id = log.create(None);
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.3".to_string()),
            test: ident("x"), // non-literal → the if→ternary rewrite path
            consequent: Box::new(expr_stmt(ident("foo"), Some(c_id.as_str()))),
            alternate: Some(Box::new(expr_stmt(ident("bar"), Some(a_id.as_str())))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);

        let _out = run_capturing_cv(&prog, &mut log);

        assert!(
            log.get(&c_id).unwrap().deleted.is_none(),
            "the consequent is preserved in the ternary, not discarded"
        );
        assert!(
            log.get(&a_id).unwrap().deleted.is_none(),
            "the alternate is preserved in the ternary, not discarded"
        );
    }

    #[test]
    fn ternary_true_tombstones_discarded_alternate_arm() {
        // `true ? kept : dead` → `kept` — the `dead` alternate arm is
        // statically unreachable and eliminated, so it must be tombstoned.
        let mut log = CVLog::new(true);
        let dead_id = log.create(None);
        let ternary = Expression::ConditionalExpression(ConditionalExpression {
            cv: Some("tern.1".to_string()),
            test: Box::new(boolean(true, None)),
            consequent: Box::new(ident("kept")),
            alternate: Box::new(Expression::Identifier(Identifier {
                cv: Some(dead_id.clone()),
                name: "dead".to_string(),
            })),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(expr_stmt(ternary, None))]);

        let _out = run_capturing_cv(&prog, &mut log);

        let del = log
            .get(&dead_id)
            .unwrap()
            .deleted
            .as_ref()
            .expect("the discarded ternary `alternate` arm must be tombstoned");
        assert_eq!(del.source, "fold-control-flow");
        assert_eq!(del.reason, "folded-branch");
    }

    #[test]
    fn ternary_false_tombstones_discarded_consequent_arm() {
        // `false ? dead : kept` → `kept` — the `dead` consequent arm is
        // statically unreachable and eliminated, so it must be tombstoned.
        let mut log = CVLog::new(true);
        let dead_id = log.create(None);
        let ternary = Expression::ConditionalExpression(ConditionalExpression {
            cv: Some("tern.2".to_string()),
            test: Box::new(boolean(false, None)),
            consequent: Box::new(Expression::Identifier(Identifier {
                cv: Some(dead_id.clone()),
                name: "dead".to_string(),
            })),
            alternate: Box::new(ident("kept")),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(expr_stmt(ternary, None))]);

        let _out = run_capturing_cv(&prog, &mut log);

        let del = log
            .get(&dead_id)
            .unwrap()
            .deleted
            .as_ref()
            .expect("the discarded ternary `consequent` arm must be tombstoned");
        assert_eq!(del.reason, "folded-branch");
    }

    // ---------------- metadata + identity ---------------------

    #[test]
    fn name_is_fold_control_flow() {
        assert_eq!(FoldControlFlowPass::new().name(), "fold-control-flow");
    }

    #[test]
    fn iteration_policy_is_fixed_point() {
        assert_eq!(
            FoldControlFlowPass::new().iteration_policy(),
            IterationPolicy::FixedPoint
        );
    }

    #[test]
    fn cost_is_two_pass_units() {
        assert_eq!(FoldControlFlowPass::new().cost(), 2);
    }

    #[test]
    fn depends_on_constant_fold() {
        assert_eq!(FoldControlFlowPass::new().depends_on(), &["constant-fold"]);
    }

    #[test]
    fn empty_program_is_identity() {
        let (_out, contribs, changed, _) = run_pass(program());
        assert!(!changed);
        assert!(contribs.is_empty());
    }

    // ---------------- IfStatement -----------------------------

    #[test]
    fn if_true_folds_to_consequent() {
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: boolean(true, Some("b.t")),
            consequent: Box::new(expr_stmt(ident("x"), Some("es.c"))),
            alternate: Some(Box::new(expr_stmt(ident("y"), Some("es.a")))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        assert_eq!(contribs.len(), 1);
        assert_eq!(contribs[0].tag, "folded-branch");
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => match &es.expression {
                Expression::Identifier(i) => assert_eq!(i.name, "x"),
                other => panic!("expected ident(x); got {:?}", other),
            },
            other => panic!("expected ExpressionStatement; got {:?}", other),
        }
    }

    #[test]
    fn if_false_folds_to_alternate() {
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: boolean(false, None),
            consequent: Box::new(expr_stmt(ident("x"), None)),
            alternate: Some(Box::new(expr_stmt(ident("y"), None))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _, _, _) = run_pass(prog);
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => match &es.expression {
                Expression::Identifier(i) => assert_eq!(i.name, "y"),
                other => panic!("expected ident(y); got {:?}", other),
            },
            other => panic!("expected ExpressionStatement; got {:?}", other),
        }
    }

    #[test]
    fn if_false_no_alternate_becomes_empty_statement() {
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: boolean(false, None),
            consequent: Box::new(expr_stmt(ident("x"), None)),
            alternate: None,
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _, changed, _) = run_pass(prog);
        assert!(changed);
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::EmptyStatement(_)) => {}
            other => panic!("expected EmptyStatement; got {:?}", other),
        }
    }

    /// Build a block `{ var <name> = 1; }` for the dead-branch extraction tests.
    fn block_with_var(name: &str) -> Statement {
        use coding_adventures_javascript_ast::{
            BindingTarget, VariableDeclaration, VariableDeclarator,
        };
        Statement::Tagged(TaggedStatement::BlockStatement(BlockStatement {
            cv: None,
            body: vec![Statement::Declaration(Declaration::VariableDeclaration(
                VariableDeclaration {
                    cv: None,
                    kind: VarKind::Var,
                    declarations: vec![VariableDeclarator {
                        cv: None,
                        id: BindingTarget::Identifier(Identifier {
                            cv: None,
                            name: name.to_string(),
                        }),
                        init: Some(num(1.0, None)),
                    }],
                },
            ))],
        }))
    }

    /// `if (false) { var z = 1; }` (no `else`) must EXTRACT the dead branch's
    /// hoisted `var` as `var z;` — dropping it is a miscompile (a later `z` read
    /// flips from a declared `undefined` binding to a `ReferenceError`).
    #[test]
    fn if_false_no_else_extracts_dead_var() {
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: boolean(false, None),
            consequent: Box::new(block_with_var("z")),
            alternate: None,
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _, changed, _) = run_pass(prog);
        assert!(changed);
        assert_hoisted_var_names(first_stmt(&out), &["z"]);
    }

    /// `if (false) { var z = 1; } else h;` — extract `var z;` BEFORE the taken
    /// `else` body; the wrapper block flattens into the list: `var z; h;`.
    #[test]
    fn if_false_with_else_extracts_dead_var_first() {
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: boolean(false, None),
            consequent: Box::new(block_with_var("z")),
            alternate: Some(Box::new(expr_stmt(ident("h"), None))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _, changed, _) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 2, "expected [var z; h;]; got {:?}", out.body);
        assert_hoisted_var_names(first_stmt(&out), &["z"]);
    }

    /// `if (true) h; else { var z = 1; }` — the DEAD alternate's `var z` is
    /// extracted before the taken consequent: `var z; h;`.
    #[test]
    fn if_true_extracts_dead_alternate_var() {
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: boolean(true, None),
            consequent: Box::new(expr_stmt(ident("h"), None)),
            alternate: Some(Box::new(block_with_var("z"))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _, changed, _) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 2, "expected [var z; h;]; got {:?}", out.body);
        assert_hoisted_var_names(first_stmt(&out), &["z"]);
    }

    /// `for (h; false; ) { var x = 1; }` — an expression init combined with a
    /// hoisted body `var` is two statements; extract `var x;` before the once-run
    /// init: `var x; h;`.
    #[test]
    fn for_false_expr_init_plus_body_var_extracted() {
        let f = for_stmt(
            Some(ForInit::Expression(ident("h"))),
            Some(boolean(false, None)),
            block_with_var("x"),
        );
        let prog = program().with_body(vec![ProgramItem::Statement(f)]);
        let (out, _, changed, _) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 2, "expected [var x; h;]; got {:?}", out.body);
        assert_hoisted_var_names(first_stmt(&out), &["x"]);
    }

    #[test]
    fn if_truthy_zero_string_or_null_works() {
        let cases = vec![
            (string("non-empty", None), "x"),
            (num(1.0, None), "x"),
            (num(0.0, None), "y"),
            (string("", None), "y"),
            (null(None), "y"),
        ];
        for (test, expected_name) in cases {
            let if_stmt = Statement::if_statement(IfStatement {
                cv: Some("if.1".to_string()),
                test: test.clone(),
                consequent: Box::new(expr_stmt(ident("x"), None)),
                alternate: Some(Box::new(expr_stmt(ident("y"), None))),
            });
            let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
            let (out, _, _, _) = run_pass(prog);
            match first_stmt(&out) {
                Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
                    match &es.expression {
                        Expression::Identifier(i) => assert_eq!(
                            i.name, expected_name,
                            "test value {:?} should select {}",
                            test, expected_name
                        ),
                        other => panic!("got {:?}", other),
                    }
                }
                other => panic!("got {:?}", other),
            }
        }
    }

    #[test]
    fn if_non_literal_test_with_single_expr_branches_folds_to_ternary() {
        // CLOC12.18 / gap-017: `if (flag) x; else y;` now ternarises
        // to `flag ? x : y;` even when `flag` isn't a known literal.
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: ident("flag"),
            consequent: Box::new(expr_stmt(ident("x"), None)),
            alternate: Some(Box::new(expr_stmt(ident("y"), None))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        assert!(!contribs.is_empty());
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
                match &es.expression {
                    Expression::ConditionalExpression(_) => {}
                    other => panic!("expected ConditionalExpression; got {:?}", other),
                }
            }
            other => panic!("expected ExpressionStatement; got {:?}", other),
        }
    }

    #[test]
    fn if_single_then_multi_else_folds_to_ternary_of_sequence() {
        // `if (x) a(); else { b(); c(); }` → `x ? a() : (b(), c());`
        // The single-expression consequent stays a bare expression; the
        // multi-statement else collapses to a comma-sequence in the
        // ternary's else arm.
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.t1".to_string()),
            test: ident("x"),
            consequent: Box::new(expr_stmt(ident("a"), None)),
            alternate: Some(Box::new(Statement::block_statement(BlockStatement {
                cv: None,
                body: vec![expr_stmt(ident("b"), None), expr_stmt(ident("c"), None)],
            }))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _c, changed, _) = run_pass(prog);
        assert!(changed);
        let Statement::Tagged(TaggedStatement::ExpressionStatement(es)) = first_stmt(&out) else {
            panic!("expected ExpressionStatement; got {:?}", first_stmt(&out))
        };
        let Expression::ConditionalExpression(cond) = &es.expression else {
            panic!("expected ConditionalExpression; got {:?}", es.expression)
        };
        assert!(
            matches!(&*cond.consequent, Expression::Identifier(_)),
            "then arm stays a bare expression; got {:?}",
            cond.consequent
        );
        let Expression::SequenceExpression(seq) = &*cond.alternate else {
            panic!("else arm must be a SequenceExpression; got {:?}", cond.alternate)
        };
        assert_eq!(seq.expressions.len(), 2, "the two else statements sequence");
    }

    #[test]
    fn if_multi_then_and_multi_else_folds_to_ternary_of_two_sequences() {
        // `if (x) { m(); n(); } else { a(); b(); }`
        //                        → `x ? (m(), n()) : (a(), b());`
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.t2".to_string()),
            test: ident("x"),
            consequent: Box::new(Statement::block_statement(BlockStatement {
                cv: None,
                body: vec![expr_stmt(ident("m"), None), expr_stmt(ident("n"), None)],
            })),
            alternate: Some(Box::new(Statement::block_statement(BlockStatement {
                cv: None,
                body: vec![expr_stmt(ident("a"), None), expr_stmt(ident("b"), None)],
            }))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _c, changed, _) = run_pass(prog);
        assert!(changed);
        let Statement::Tagged(TaggedStatement::ExpressionStatement(es)) = first_stmt(&out) else {
            panic!("expected ExpressionStatement; got {:?}", first_stmt(&out))
        };
        let Expression::ConditionalExpression(cond) = &es.expression else {
            panic!("expected ConditionalExpression; got {:?}", es.expression)
        };
        let (Expression::SequenceExpression(t), Expression::SequenceExpression(e)) =
            (&*cond.consequent, &*cond.alternate)
        else {
            panic!("both arms must be SequenceExpressions; got {:?} / {:?}", cond.consequent, cond.alternate)
        };
        assert_eq!(t.expressions.len(), 2);
        assert_eq!(e.expressions.len(), 2);
    }

    #[test]
    fn if_ternary_declines_when_a_branch_is_not_all_expressions() {
        // `if (x) a(); else { b(); return; }` — the else contains a `return`,
        // which cannot join a comma-sequence, so `stmts_as_sequence_expr`
        // returns None and the ternary fold declines. The `if` is kept
        // (and, since the alternate is non-empty, the && fold also declines).
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.keep".to_string()),
            test: ident("x"),
            consequent: Box::new(expr_stmt(ident("a"), None)),
            alternate: Some(Box::new(Statement::block_statement(BlockStatement {
                cv: None,
                body: vec![
                    expr_stmt(ident("b"), None),
                    Statement::return_statement(ReturnStatement { cv: None, argument: None }),
                ],
            }))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _c, _changed, _) = run_pass(prog);
        assert!(
            matches!(first_stmt(&out), Statement::Tagged(TaggedStatement::IfStatement(_))),
            "if with a return-containing branch must be kept; got {:?}",
            first_stmt(&out)
        );
    }

    #[test]
    fn if_multi_then_no_else_folds_to_logical_and_of_sequence() {
        // `if (x) { a(); b(); }` → `x && (a(), b());` — the multi-statement
        // consequent is comma-sequenced under the `&&` right operand.
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.and1".to_string()),
            test: ident("x"),
            consequent: Box::new(Statement::block_statement(BlockStatement {
                cv: None,
                body: vec![expr_stmt(ident("a"), None), expr_stmt(ident("b"), None)],
            })),
            alternate: None,
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _c, changed, _) = run_pass(prog);
        assert!(changed);
        let Statement::Tagged(TaggedStatement::ExpressionStatement(es)) = first_stmt(&out) else {
            panic!("expected ExpressionStatement; got {:?}", first_stmt(&out))
        };
        let Expression::LogicalExpression(l) = &es.expression else {
            panic!("expected LogicalExpression(And); got {:?}", es.expression)
        };
        assert_eq!(l.operator, LogicalOperator::And, "operator must be &&");
        let Expression::SequenceExpression(seq) = &*l.right else {
            panic!("&& right operand must be a SequenceExpression; got {:?}", l.right)
        };
        assert_eq!(seq.expressions.len(), 2);
    }

    #[test]
    fn if_single_then_empty_else_folds_to_logical_and() {
        // `if (x) S; else {}` and `if (x) S; else ;` — an empty alternate is a
        // no-op, so both fold to `x && S`, exactly like the no-alternate form.
        for alt in [
            Statement::block_statement(BlockStatement { cv: None, body: vec![] }),
            Statement::empty_statement(EmptyStatement { cv: None }),
        ] {
            let if_stmt = Statement::if_statement(IfStatement {
                cv: Some("if.and2".to_string()),
                test: ident("x"),
                consequent: Box::new(expr_stmt(ident("s"), None)),
                alternate: Some(Box::new(alt)),
            });
            let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
            let (out, _c, changed, _) = run_pass(prog);
            assert!(changed, "empty else should not block the && fold");
            let Statement::Tagged(TaggedStatement::ExpressionStatement(es)) = first_stmt(&out) else {
                panic!("expected ExpressionStatement; got {:?}", first_stmt(&out))
            };
            let Expression::LogicalExpression(l) = &es.expression else {
                panic!("expected LogicalExpression(And); got {:?}", es.expression)
            };
            assert_eq!(l.operator, LogicalOperator::And, "operator must be &&");
            assert!(
                matches!(&*l.right, Expression::Identifier(_)),
                "&& right operand is the single consequent expr; got {:?}",
                l.right
            );
        }
    }

    #[test]
    fn if_empty_then_with_single_expr_else_folds_to_logical_or() {
        // Empty consequent + single-expression-statement alternate →
        // `test || expr`, the mirror of the if-to-logical-and fold:
        // `if (x) {} else y;` and `if (x) ; else y;` → `x || y;`.
        for consequent in [
            Statement::block_statement(BlockStatement { cv: None, body: vec![] }),
            Statement::empty_statement(EmptyStatement { cv: None }),
        ] {
            let if_stmt = Statement::if_statement(IfStatement {
                cv: Some("if.or".to_string()),
                test: ident("x"),
                consequent: Box::new(consequent),
                alternate: Some(Box::new(expr_stmt(ident("y"), None))),
            });
            let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
            let (out, contribs, changed, _) = run_pass(prog);
            assert!(changed, "empty-then + expr-else should fold to ||");
            assert!(!contribs.is_empty());
            match first_stmt(&out) {
                Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
                    match &es.expression {
                        Expression::LogicalExpression(l) => {
                            assert_eq!(l.operator, LogicalOperator::Or, "operator must be ||");
                        }
                        other => panic!("expected LogicalExpression(Or); got {:?}", other),
                    }
                }
                other => panic!("expected ExpressionStatement; got {:?}", other),
            }
        }
    }

    #[test]
    fn if_empty_then_with_multi_statement_else_folds_to_or_of_sequence() {
        // `if (x) {} else { a; b; }` — the multi-statement else now collapses to
        // a comma-sequence under the `||` right operand: `x || (a, b)`.
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.or2".to_string()),
            test: ident("x"),
            consequent: Box::new(Statement::block_statement(BlockStatement {
                cv: None,
                body: vec![],
            })),
            alternate: Some(Box::new(Statement::block_statement(BlockStatement {
                cv: None,
                body: vec![expr_stmt(ident("a"), None), expr_stmt(ident("b"), None)],
            }))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _contribs, changed, _) = run_pass(prog);
        assert!(changed, "multi-statement else should fold to || of a sequence");
        let Statement::Tagged(TaggedStatement::ExpressionStatement(es)) = first_stmt(&out) else {
            panic!("expected an ExpressionStatement; got {:?}", first_stmt(&out))
        };
        let Expression::LogicalExpression(l) = &es.expression else {
            panic!("expected a LogicalExpression(Or); got {:?}", es.expression)
        };
        assert_eq!(l.operator, LogicalOperator::Or, "operator must be ||");
        let Expression::SequenceExpression(seq) = &*l.right else {
            panic!("|| right operand must be a SequenceExpression; got {:?}", l.right)
        };
        assert_eq!(seq.expressions.len(), 2, "the two else statements sequence");
    }

    #[test]
    fn if_empty_then_with_declaration_else_passes_through() {
        // `if (x) {} else { var y; a(); }` — a `var` declaration can't join a
        // comma-sequence, so the fold declines and the `if` is kept intact
        // (Closure reaches for a De Morgan `if (!x) { … }` there — a separate arc).
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.or3".to_string()),
            test: ident("x"),
            consequent: Box::new(Statement::block_statement(BlockStatement {
                cv: None,
                body: vec![],
            })),
            alternate: Some(Box::new(Statement::block_statement(BlockStatement {
                cv: None,
                body: vec![
                    Statement::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
                        cv: None,
                        kind: VarKind::Var,
                        declarations: vec![VariableDeclarator {
                            cv: None,
                            id: BindingTarget::Identifier(Identifier {
                                cv: None,
                                name: "y".to_string(),
                            }),
                            init: None,
                        }],
                    })),
                    expr_stmt(ident("a"), None),
                ],
            }))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _c, _changed, _) = run_pass(prog);
        assert!(
            matches!(
                first_stmt(&out),
                Statement::Tagged(TaggedStatement::IfStatement(_))
            ),
            "declaration in the else must keep the if intact; got {:?}",
            first_stmt(&out)
        );
    }

    #[test]
    fn if_declaration_in_consequent_passes_through() {
        // The if→logical/ternary folds only fire when a branch is a run of
        // **expression** statements. A `var` declaration in the consequent
        // makes `stmts_as_sequence_expr` return `None` (a declaration can't
        // join a comma-sequence), so `if (flag) { var x; y(); }` is kept
        // intact — the fold does not over-fire on non-collapsible shapes.
        //
        // (History: this test used to assert that a plain multi-statement
        // consequent `{ x; y; }` passed through. That case now legitimately
        // folds to `flag && (x, y)` — see
        // `if_multi_then_no_else_folds_to_logical_and_of_sequence` — so the
        // decline is re-anchored on a declaration member instead.)
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.2".to_string()),
            test: ident("flag"),
            consequent: Box::new(Statement::block_statement(BlockStatement {
                cv: None,
                body: vec![
                    Statement::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
                        cv: None,
                        kind: VarKind::Var,
                        declarations: vec![VariableDeclarator {
                            cv: None,
                            id: BindingTarget::Identifier(Identifier {
                                cv: None,
                                name: "x".to_string(),
                            }),
                            init: None,
                        }],
                    })),
                    expr_stmt(ident("y"), None),
                ],
            })),
            alternate: None,
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _contribs, _changed, _) = run_pass(prog);
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::IfStatement(_)) => {}
            other => panic!(
                "expected IfStatement intact (declaration in consequent); got {:?}",
                other
            ),
        }
    }

    #[test]
    fn if_with_unresolved_comparison_folds_via_gap016() {
        // **Pre-gap-016**: this test asserted `if (1<2) A;` stayed
        // unchanged because fold-control-flow alone doesn't fold the
        // `<` comparison (that's constant-fold's job) and there was
        // no alternate to ternarise. **Post-gap-016** (CLOC12.24),
        // even with `1<2` left as a BinaryExpression, the surrounding
        // `if (test) S;` (no alternate, single-expr consequent) now
        // folds to `(1<2) && A;` via the new logical-and rewrite.
        //
        // The original intent — "fold-control-flow alone doesn't have
        // access to constant-fold's binary-expression folding" — is
        // still pinned: the inner `1 < 2` survives as a
        // BinaryExpression rather than being folded to `true`.
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: Expression::BinaryExpression(BinaryExpression {
                cv: None,
                operator: BinaryOperator::Lt,
                left: Box::new(num(1.0, None)),
                right: Box::new(num(2.0, None)),
            }),
            consequent: Box::new(expr_stmt(ident("A"), None)),
            alternate: None,
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _, _changed, _) = run_pass(prog);
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
                if let Expression::LogicalExpression(le) = &es.expression {
                    assert_eq!(le.operator, LogicalOperator::And);
                    // The left operand must still be the unfolded
                    // BinaryExpression — fold-control-flow does NOT
                    // touch `<` comparisons (that's constant-fold).
                    assert!(matches!(*le.left, Expression::BinaryExpression(_)),
                        "left operand should still be a BinaryExpression (not folded by fold-control-flow); got {:?}", le.left);
                    assert!(matches!(*le.right, Expression::Identifier(_)),
                        "right operand should be the Identifier `A`; got {:?}", le.right);
                } else {
                    panic!("expected LogicalExpression; got {:?}", es.expression);
                }
            }
            other => panic!(
                "expected ExpressionStatement wrapping LogicalExpression; got {:?}",
                other
            ),
        }
    }

    // ---------------- WhileStatement --------------------------

    #[test]
    fn while_false_becomes_empty_statement() {
        let w = Statement::while_statement(WhileStatement {
            cv: Some("w.1".to_string()),
            test: boolean(false, None),
            body: Box::new(expr_stmt(ident("body"), None)),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(w)]);
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        assert_eq!(contribs.len(), 1);
        assert_eq!(contribs[0].tag, "folded-branch");
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::EmptyStatement(_)) => {}
            other => panic!("expected EmptyStatement; got {:?}", other),
        }
    }

    #[test]
    fn while_true_becomes_infinite_for() {
        // `while (true) body` → `for (;;) body`: the loop is canonicalised to a
        // `for`, and the redundant truthy test is elided (init/test/update all
        // absent). Matches Closure's `for (;;)`.
        let w = Statement::while_statement(WhileStatement {
            cv: Some("w.1".to_string()),
            test: boolean(true, None),
            body: Box::new(expr_stmt(ident("body"), None)),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(w)]);
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        assert_eq!(contribs.len(), 1);
        assert_eq!(contribs[0].tag, "while-to-for-truthy");
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ForStatement(f)) => {
                assert!(f.init.is_none(), "init should be absent");
                assert!(f.test.is_none(), "truthy test should be elided");
                assert!(f.update.is_none(), "update should be absent");
            }
            other => panic!("expected ForStatement; got {:?}", other),
        }
    }

    #[test]
    fn while_unknown_test_becomes_for_keeping_test() {
        // `while (x) body` → `for (; x; ) body`: an unknown (non-literal) test
        // is carried across verbatim; only the loop keyword changes.
        let w = Statement::while_statement(WhileStatement {
            cv: Some("w.1".to_string()),
            test: ident("x"),
            body: Box::new(expr_stmt(ident("body"), None)),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(w)]);
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        assert_eq!(contribs.len(), 1);
        assert_eq!(contribs[0].tag, "while-to-for");
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ForStatement(f)) => {
                assert!(f.init.is_none(), "init should be absent");
                assert!(f.update.is_none(), "update should be absent");
                match f.test.as_ref() {
                    Some(Expression::Identifier(id)) => assert_eq!(id.name, "x"),
                    other => panic!("expected test `x`; got {:?}", other),
                }
            }
            other => panic!("expected ForStatement; got {:?}", other),
        }
    }

    // ---------------- dead-code-after-return ------------------

    #[test]
    fn dead_code_after_return_is_dropped() {
        // function f() { x; return; y; z; } — drop y and z.
        let block = BlockStatement {
            cv: Some("block.1".to_string()),
            body: vec![
                expr_stmt(ident("x"), None),
                Statement::return_statement(ReturnStatement { cv: None, argument: None }),
                expr_stmt(ident("y"), None),
                expr_stmt(ident("z"), None),
            ],
        };
        let fdecl = Declaration::FunctionDeclaration(FunctionDeclaration {
            cv: Some("fn.1".to_string()),
            id: Identifier {
                cv: None,
                name: "f".to_string(),
            },
            params: vec![],
            body: block,
            generator: false,
            is_async: false,
        });
        let prog = program().with_body(vec![ProgramItem::Declaration(fdecl)]);
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        assert!(
            contribs.iter().any(|c| c.tag == "removed-dead-code"),
            "expected removed-dead-code contribution; got {:?}",
            contribs
        );
        let ProgramItem::Declaration(Declaration::FunctionDeclaration(out_fn)) = &out.body[0]
        else {
            panic!("expected FunctionDeclaration");
        };
        assert_eq!(
            out_fn.body.body.len(),
            2,
            "expected 2 statements after fold (x + return); got {:?}",
            out_fn.body.body
        );
    }

    #[test]
    fn block_without_return_unchanged() {
        let block = BlockStatement {
            cv: Some("block.1".to_string()),
            body: vec![expr_stmt(ident("x"), None), expr_stmt(ident("y"), None)],
        };
        let fdecl = Declaration::FunctionDeclaration(FunctionDeclaration {
            cv: Some("fn.1".to_string()),
            id: Identifier {
                cv: None,
                name: "f".to_string(),
            },
            params: vec![],
            body: block,
            generator: false,
            is_async: false,
        });
        let prog = program().with_body(vec![ProgramItem::Declaration(fdecl)]);
        let (_, contribs, changed, _) = run_pass(prog);
        assert!(!changed);
        assert!(contribs.is_empty());
    }

    // ---------------- ConditionalExpression -------------------

    #[test]
    fn ternary_with_truthy_test_folds_to_consequent() {
        let e = Expression::ConditionalExpression(ConditionalExpression {
            cv: Some("cond.1".to_string()),
            test: Box::new(boolean(true, None)),
            consequent: Box::new(ident("a")),
            alternate: Box::new(ident("b")),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(expr_stmt(
            e,
            Some("es.1"),
        ))]);
        let (out, _, changed, _) = run_pass(prog);
        assert!(changed);
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => match &es.expression {
                Expression::Identifier(i) => assert_eq!(i.name, "a"),
                other => panic!("got {:?}", other),
            },
            other => panic!("got {:?}", other),
        }
    }

    // ---------------- untraced (cv: None) ---------------------

    #[test]
    fn untraced_mode_folds_silently() {
        let if_stmt = Statement::if_statement(IfStatement {
            cv: None,
            test: boolean(true, None),
            consequent: Box::new(expr_stmt(ident("x"), None)),
            alternate: Some(Box::new(expr_stmt(ident("y"), None))),
        });
        let prog = untraced_program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        assert!(
            contribs.is_empty(),
            "untraced fold emits no contributions; got {:?}",
            contribs
        );
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => match &es.expression {
                Expression::Identifier(i) => assert_eq!(i.name, "x"),
                other => panic!("got {:?}", other),
            },
            other => panic!("got {:?}", other),
        }
    }

    // ---------------- pipeline integration --------------------

    #[test]
    fn pipeline_solo_runs_cleanly() {
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(FoldControlFlowPass::new()));
        let mut cv = CVLog::new(true);
        let out: PipelineOutput = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");
        assert_eq!(out.execution_order, vec!["fold-control-flow".to_string()]);
        // The pipeline now iterates to a fixed point; a non-changing
        // solo pass converges in one sweep, so the old "not-yet-iterated"
        // note is gone.
        assert!(!out
            .diagnostics
            .iter()
            .any(|d| d.group.0 == "pipeline.fixed-point-not-yet-iterated"));
    }

    #[test]
    fn pipeline_with_constant_fold_collapses_if_one_lt_two() {
        // Register both. constant-fold collapses `1 < 2 → true`,
        // then fold-control-flow collapses `if (true) {A} → A`.
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: Expression::BinaryExpression(BinaryExpression {
                cv: Some("cmp.1".to_string()),
                operator: BinaryOperator::Lt,
                left: Box::new(num(1.0, None)),
                right: Box::new(num(2.0, None)),
            }),
            consequent: Box::new(expr_stmt(ident("A"), Some("es.c"))),
            alternate: None,
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);

        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(ConstantFoldPass::new()));
        pipeline.add(Box::new(FoldControlFlowPass::new()));
        let mut cv = CVLog::new(true);
        let out = pipeline
            .run(prog, &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        assert_eq!(
            out.execution_order,
            vec!["constant-fold".to_string(), "fold-control-flow".to_string()]
        );
        match first_stmt(&out.program) {
            Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => match &es.expression {
                Expression::Identifier(i) => assert_eq!(i.name, "A"),
                other => panic!("expected ident(A); got {:?}", other),
            },
            other => panic!("expected ExpressionStatement holding A; got {:?}", other),
        }
    }

    // =====================================================================
    // gap-015 / CLOC12.37 — var-hoisting tests
    // =====================================================================

    use coding_adventures_javascript_ast::{BindingTarget, VarKind, VariableDeclarator};

    fn fdecl_with_body(body: Vec<Statement>) -> Program {
        let block = BlockStatement {
            cv: Some("fn.body".to_string()),
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

    fn extract_fn_body(prog: &Program) -> &BlockStatement {
        match &prog.body[0] {
            ProgramItem::Declaration(Declaration::FunctionDeclaration(f)) => &f.body,
            other => panic!("expected FunctionDeclaration; got {:?}", other),
        }
    }

    fn make_var_decl(name: &str, init: Option<Expression>) -> Statement {
        Statement::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind: VarKind::Var,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(Identifier {
                    cv: None,
                    name: name.to_string(),
                }),
                init,
            }],
        }))
    }

    fn make_let_decl(name: &str, init: Option<Expression>) -> Statement {
        Statement::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind: VarKind::Let,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(Identifier {
                    cv: None,
                    name: name.to_string(),
                }),
                init,
            }],
        }))
    }

    // ---------------- CLOC25: else-hoist after terminating consequent ----

    /// `function f(x){ if(x){return 1;} else { g; } }`
    /// → `function f(x){ if(x){return 1;} g; }` — the `else` is hoisted.
    #[test]
    fn else_block_hoisted_after_returning_consequent() {
        let consequent = Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![Statement::return_statement(ReturnStatement {
                cv: None,
                argument: Some(num(1.0, None)),
            })],
        });
        let alternate = Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![expr_stmt(ident("g"), None)],
        });
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: ident("x"),
            consequent: Box::new(consequent),
            alternate: Some(Box::new(alternate)),
        });
        let prog = fdecl_with_body(vec![if_stmt]);
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        assert!(
            contribs
                .iter()
                .any(|c| c.tag == "hoisted-else-after-terminator"),
            "expected hoisted-else-after-terminator; got {:?}",
            contribs
        );
        let body = extract_fn_body(&out);
        assert_eq!(
            body.body.len(),
            2,
            "expected [if(x){{return 1}}, g]; got {:?}",
            body.body
        );
        let Statement::Tagged(TaggedStatement::IfStatement(if_out)) = &body.body[0] else {
            panic!("expected if; got {:?}", body.body[0]);
        };
        assert!(if_out.alternate.is_none(), "the else must be removed");
        assert!(matches!(
            &body.body[1],
            Statement::Tagged(TaggedStatement::ExpressionStatement(_))
        ));
    }

    /// A bare `throw` consequent with a bare `else` body also hoists:
    /// `if(bad) throw e; else use;` → `if(bad) throw e; use;`.
    #[test]
    fn else_hoisted_after_throwing_consequent() {
        let consequent =
            Statement::throw_statement(coding_adventures_javascript_ast::ThrowStatement {
                cv: None,
                argument: ident("e"),
            });
        let alternate = expr_stmt(ident("use"), None);
        let if_stmt = Statement::if_statement(IfStatement {
            cv: None,
            test: ident("bad"),
            consequent: Box::new(consequent),
            alternate: Some(Box::new(alternate)),
        });
        let prog = fdecl_with_body(vec![if_stmt]);
        let (out, _contribs, changed, _) = run_pass(prog);
        assert!(changed);
        let body = extract_fn_body(&out);
        assert_eq!(body.body.len(), 2, "got {:?}", body.body);
        let Statement::Tagged(TaggedStatement::IfStatement(if_out)) = &body.body[0] else {
            panic!("expected if; got {:?}", body.body[0]);
        };
        assert!(if_out.alternate.is_none());
        assert!(matches!(
            &body.body[1],
            Statement::Tagged(TaggedStatement::ExpressionStatement(_))
        ));
    }

    /// An `else` block that declares a `let` is NOT hoisted — moving the
    /// binding out of its block would leak it / risk a TDZ collision.
    #[test]
    fn else_block_with_let_is_not_hoisted() {
        let consequent = Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![Statement::return_statement(ReturnStatement {
                cv: None,
                argument: None,
            })],
        });
        let alternate = Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![make_let_decl("y", Some(num(1.0, None))), expr_stmt(ident("g"), None)],
        });
        let if_stmt = Statement::if_statement(IfStatement {
            cv: None,
            test: ident("x"),
            consequent: Box::new(consequent),
            alternate: Some(Box::new(alternate)),
        });
        let prog = fdecl_with_body(vec![if_stmt]);
        let (out, contribs, _changed, _) = run_pass(prog);
        assert!(
            !contribs
                .iter()
                .any(|c| c.tag == "hoisted-else-after-terminator"),
            "a let in the else block must block the hoist; got {:?}",
            contribs
        );
        let body = extract_fn_body(&out);
        assert_eq!(body.body.len(), 1, "if-else stays a single statement; got {:?}", body.body);
        let Statement::Tagged(TaggedStatement::IfStatement(if_out)) = &body.body[0] else {
            panic!("expected if; got {:?}", body.body[0]);
        };
        assert!(if_out.alternate.is_some(), "the else must be preserved");
    }

    /// When the consequent does NOT unconditionally terminate, the `else`
    /// stays: `if(x){debugger} else {h}` is unchanged. (The consequent is a
    /// `debugger` statement — not an expression statement — so the
    /// if-else→ternary fold does not apply either, isolating the else-hoist's
    /// terminator gate as the reason it stays. A plain expression consequent
    /// like `{g()}` would now ternarise to `x ? g() : h`, so it can no longer
    /// serve to isolate the gate. `debugger` also does not var-hoist or
    /// terminate, so nothing else perturbs the statement.)
    #[test]
    fn else_not_hoisted_when_consequent_falls_through() {
        let consequent = Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![Statement::debugger_statement(
                coding_adventures_javascript_ast::DebuggerStatement { cv: None },
            )],
        });
        let alternate = Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![expr_stmt(ident("h"), None)],
        });
        let if_stmt = Statement::if_statement(IfStatement {
            cv: None,
            test: ident("x"),
            consequent: Box::new(consequent),
            alternate: Some(Box::new(alternate)),
        });
        let prog = fdecl_with_body(vec![if_stmt]);
        let (out, contribs, _changed, _) = run_pass(prog);
        assert!(
            !contribs
                .iter()
                .any(|c| c.tag == "hoisted-else-after-terminator"),
            "a non-terminating consequent must NOT hoist; got {:?}",
            contribs
        );
        let body = extract_fn_body(&out);
        assert_eq!(body.body.len(), 1);
        let Statement::Tagged(TaggedStatement::IfStatement(if_out)) = &body.body[0] else {
            panic!("expected if; got {:?}", body.body[0]);
        };
        assert!(if_out.alternate.is_some());
    }

    /// After hoisting an `else` whose body itself ends in a terminator, code
    /// following the original `if` becomes dead and is dropped in the same
    /// pass: `if(x){return 1} else {cleanup; return 2} after;`
    /// → `if(x){return 1} cleanup; return 2;` (`after` gone). (The else has
    /// two statements, so the if-else→ternary fold does not apply — the
    /// else-hoist does.)
    #[test]
    fn hoisted_returning_else_drops_following_dead_code() {
        let consequent = Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![Statement::return_statement(ReturnStatement {
                cv: None,
                argument: Some(num(1.0, None)),
            })],
        });
        let alternate = Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![
                expr_stmt(ident("cleanup"), None),
                Statement::return_statement(ReturnStatement {
                    cv: None,
                    argument: Some(num(2.0, None)),
                }),
            ],
        });
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: ident("x"),
            consequent: Box::new(consequent),
            alternate: Some(Box::new(alternate)),
        });
        let prog = fdecl_with_body(vec![if_stmt, expr_stmt(ident("after"), None)]);
        let (out, contribs, changed, _) = run_pass(prog);
        assert!(changed);
        assert!(
            contribs
                .iter()
                .any(|c| c.tag == "hoisted-else-after-terminator"),
            "got {:?}",
            contribs
        );
        let body = extract_fn_body(&out);
        // [if(x){return 1}, cleanup, return 2] — `after` dropped as dead code.
        assert_eq!(
            body.body.len(),
            3,
            "expected `after` to be dropped; got {:?}",
            body.body
        );
        assert!(matches!(
            &body.body[2],
            Statement::Tagged(TaggedStatement::ReturnStatement(_))
        ));
    }

    /// gap-015 removed (#213): a `var` inside an `if` block stays
    /// exactly where it is written. Oracle probing proved upstream
    /// Closure never hoists+splits `var y = 1;` into a top-of-body
    /// `var y;` plus an `y = 1;` assignment in SIMPLE output — the
    /// declaration is left nested, so `function f(){if(cond){var y=1}}`
    /// is unchanged by this pass and emits no `var-hoisted`
    /// contribution.
    #[test]
    fn var_inside_if_consequent_block_stays_nested() {
        let inner_block = Statement::Tagged(TaggedStatement::BlockStatement(BlockStatement {
            cv: None,
            body: vec![make_var_decl("y", Some(num(1.0, None)))],
        }));
        let if_stmt = Statement::if_statement(IfStatement {
            cv: None,
            test: ident("cond"),
            consequent: Box::new(inner_block),
            alternate: None,
        });
        let prog = fdecl_with_body(vec![if_stmt]);
        let (out, contribs, _, _) = run_pass(prog);
        let body = extract_fn_body(&out);
        // Body is still just the `if` — NO prepended `var y;`.
        assert_eq!(body.body.len(), 1, "expected 1 stmt; got {:?}", body.body);
        assert!(matches!(
            &body.body[0],
            Statement::Tagged(TaggedStatement::IfStatement(_))
        ));
        // The `var y = 1;` keeps its initializer where it sits — no split.
        if let Statement::Tagged(TaggedStatement::IfStatement(i)) = &body.body[0] {
            if let Statement::Tagged(TaggedStatement::BlockStatement(b)) = &*i.consequent {
                if let Statement::Declaration(Declaration::VariableDeclaration(v)) = &b.body[0] {
                    assert_eq!(v.kind, VarKind::Var);
                    assert!(v.declarations[0].init.is_some(), "init must survive intact");
                } else {
                    panic!("expected `var y = 1;` still nested; got {:?}", b.body[0]);
                }
            }
        }
        assert!(!contribs.iter().any(|c| c.tag == "var-hoisted"));
    }

    /// gap-015 removed (#213): `function f() { var x = 1; }` is left
    /// verbatim — no split into `var x; x = 1;`.
    #[test]
    fn var_at_top_of_function_body_stays_intact() {
        let prog = fdecl_with_body(vec![make_var_decl("x", Some(num(1.0, None)))]);
        let (out, contribs, _, _) = run_pass(prog);
        let body = extract_fn_body(&out);
        assert_eq!(body.body.len(), 1, "no split; got {:?}", body.body);
        if let Statement::Declaration(Declaration::VariableDeclaration(v)) = &body.body[0] {
            assert_eq!(v.declarations.len(), 1);
            assert!(v.declarations[0].init.is_some(), "init must survive intact");
        } else {
            panic!("expected `var x = 1;` intact; got {:?}", body.body[0]);
        }
        assert!(!contribs.iter().any(|c| c.tag == "var-hoisted"));
    }

    /// `let` is block-scoped — must NOT be hoisted.
    #[test]
    fn let_declaration_is_not_hoisted() {
        let inner = Statement::Tagged(TaggedStatement::BlockStatement(BlockStatement {
            cv: None,
            body: vec![make_let_decl("y", Some(num(1.0, None)))],
        }));
        let if_stmt = Statement::if_statement(IfStatement {
            cv: None,
            test: ident("cond"),
            consequent: Box::new(inner),
            alternate: None,
        });
        let prog = fdecl_with_body(vec![if_stmt]);
        let (out, contribs, _, _) = run_pass(prog);
        let body = extract_fn_body(&out);
        // Body still has just the if-statement.
        assert_eq!(body.body.len(), 1);
        assert!(matches!(
            &body.body[0],
            Statement::Tagged(TaggedStatement::IfStatement(_))
        ));
        assert!(!contribs.iter().any(|c| c.tag == "var-hoisted"));
    }

    /// `function f() { function g() { var y = 1; } }` — outer
    /// hoister must NOT touch the inner function's body. `g`'s
    /// var hoist runs at *its own* FunctionDeclaration dispatch.
    #[test]
    fn nested_function_body_vars_are_isolated() {
        let inner_fn = Declaration::FunctionDeclaration(FunctionDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "g".to_string(),
            },
            params: vec![],
            body: BlockStatement {
                cv: None,
                body: vec![make_var_decl("y", Some(num(1.0, None)))],
            },
            generator: false,
            is_async: false,
        });
        let prog = fdecl_with_body(vec![Statement::Declaration(inner_fn)]);
        let (out, _, _, _) = run_pass(prog);
        let outer_body = extract_fn_body(&out);
        // Outer body has just the inner FunctionDeclaration.
        assert_eq!(outer_body.body.len(), 1);
        if let Statement::Declaration(Declaration::FunctionDeclaration(g)) = &outer_body.body[0] {
            // gap-015 removed (#213): the inner body keeps `var y = 1;`
            // intact — no split into `var y; y = 1;`.
            assert_eq!(g.body.body.len(), 1, "no split; got {:?}", g.body.body);
            if let Statement::Declaration(Declaration::VariableDeclaration(v)) = &g.body.body[0] {
                assert!(v.declarations[0].init.is_some(), "init must survive intact");
            } else {
                panic!("expected `var y = 1;` intact; got {:?}", g.body.body[0]);
            }
        } else {
            panic!("expected nested FunctionDeclaration; got {:?}", outer_body.body[0]);
        }
    }

    /// `function f() {}` — empty body, no hoist work, no
    /// `var-hoisted` contribution.
    #[test]
    fn empty_function_body_does_nothing() {
        let prog = fdecl_with_body(vec![]);
        let (out, contribs, _, _) = run_pass(prog);
        let body = extract_fn_body(&out);
        assert!(body.body.is_empty());
        assert!(!contribs.iter().any(|c| c.tag == "var-hoisted"));
    }

    /// gap-015 removed (#213): a bare `var y;` (no init) inside a block
    /// also stays where written — it is NOT lifted to the body top nor
    /// collapsed to an `EmptyStatement`.
    #[test]
    fn bare_var_no_init_stays_nested() {
        let inner_block = Statement::Tagged(TaggedStatement::BlockStatement(BlockStatement {
            cv: None,
            body: vec![make_var_decl("y", None)],
        }));
        let if_stmt = Statement::if_statement(IfStatement {
            cv: None,
            test: ident("cond"),
            consequent: Box::new(inner_block),
            alternate: None,
        });
        let prog = fdecl_with_body(vec![if_stmt]);
        let (out, contribs, _, _) = run_pass(prog);
        let body = extract_fn_body(&out);
        // Body is still just the `if` — no prepended `var y;`.
        assert_eq!(body.body.len(), 1, "no hoist; got {:?}", body.body);
        if let Statement::Tagged(TaggedStatement::IfStatement(i)) = &body.body[0] {
            if let Statement::Tagged(TaggedStatement::BlockStatement(b)) = &*i.consequent {
                assert_eq!(b.body.len(), 1);
                assert!(matches!(
                    &b.body[0],
                    Statement::Declaration(Declaration::VariableDeclaration(_))
                ), "the bare `var y;` stays nested; got {:?}", b.body[0]);
            }
        }
        assert!(!contribs.iter().any(|c| c.tag == "var-hoisted"));
    }

    // =====================================================================
    // CLOC12.194 — redundant BlockStatement flattening (oracle divergence #4)
    //
    // A bare `{ … }` at statement-list position is a lexical scope with no
    // observable effect UNLESS it declares a block-scoped binding. Closure
    // removes such braces (`PeepholeRemoveDeadCode`); these tests pin the fold
    // and its soundness boundary — verified byte-identical to the real Closure
    // jar (`{a}`→`a;`, `{a;b}`→`a;b;`, `{var x=1;a}`→`var x=1;a;`, `{}`→removed;
    // `{let…}`/`{const…}`/`{function…}` kept).
    // =====================================================================

    /// Wrap statements in a `{ … }` block statement (test helper).
    fn block(cv: Option<&str>, body: Vec<Statement>) -> Statement {
        Statement::block_statement(BlockStatement {
            cv: cv.map(|s| s.to_string()),
            body,
        })
    }

    #[test]
    fn flatten_bare_block_at_program_level() {
        // `{ a; }` → `a;`
        let prog = program().with_body(vec![ProgramItem::Statement(block(
            Some("blk.1"),
            vec![expr_stmt(ident("a"), None)],
        ))]);
        let (out, _c, changed, _n) = run_pass(prog);
        assert!(changed, "bare block should flatten");
        assert_eq!(out.body.len(), 1, "block braces gone, one statement left");
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => assert!(
                matches!(&es.expression, Expression::Identifier(i) if i.name == "a")
            ),
            other => panic!("expected `a;`, got {other:?}"),
        }
    }

    #[test]
    fn flatten_multi_statement_block() {
        // `{ a; b; }` → `a; b;`
        let prog = program().with_body(vec![ProgramItem::Statement(block(
            None,
            vec![expr_stmt(ident("a"), None), expr_stmt(ident("b"), None)],
        ))]);
        let (out, _c, changed, _n) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 2, "both inner statements spliced in");
    }

    #[test]
    fn split_comma_sequence_at_program_level() {
        // `x, y, z;` → `x; y; z;` — a comma-sequence expression STATEMENT at a
        // statement-list position splits into one statement per sub-expression.
        let seq = Expression::SequenceExpression(SequenceExpression {
            cv: None,
            expressions: vec![ident("x"), ident("y"), ident("z")],
        });
        let prog = program().with_body(vec![ProgramItem::Statement(expr_stmt(seq, None))]);
        let (out, _c, changed, _n) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 3, "expected 3 split statements; got {:?}", out.body);
        for (i, name) in ["x", "y", "z"].iter().enumerate() {
            match &out.body[i] {
                ProgramItem::Statement(Statement::Tagged(
                    TaggedStatement::ExpressionStatement(es),
                )) => assert!(
                    matches!(&es.expression, Expression::Identifier(id) if &id.name == name),
                    "stmt {i} should be `{name};`; got {:?}",
                    es.expression
                ),
                other => panic!("expected ExpressionStatement for `{name}`; got {other:?}"),
            }
        }
    }

    #[test]
    fn split_comma_sequence_in_block() {
        // `{ a, b; }` → the block flattens and the sequence splits: `a; b;`.
        let seq = Expression::SequenceExpression(SequenceExpression {
            cv: None,
            expressions: vec![ident("a"), ident("b")],
        });
        let prog =
            program().with_body(vec![ProgramItem::Statement(block(None, vec![expr_stmt(seq, None)]))]);
        let (out, _c, changed, _n) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 2, "expected [a; b;]; got {:?}", out.body);
    }

    #[test]
    fn flatten_empty_block_removes_it() {
        // `a; {}` → `a;` — the empty block flattens to nothing.
        let prog = program().with_body(vec![
            ProgramItem::Statement(expr_stmt(ident("a"), None)),
            ProgramItem::Statement(block(None, vec![])),
        ]);
        let (out, _c, changed, _n) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 1, "empty block removed entirely");
    }

    #[test]
    fn flatten_block_with_only_var_is_safe() {
        // `{ var x = 1; a; }` → `var x = 1; a;` — `var` is function-scoped and
        // hoists, so the block boundary is unobservable.
        let prog = program().with_body(vec![ProgramItem::Statement(block(
            None,
            vec![
                make_var_decl("x", Some(num(1.0, None))),
                expr_stmt(ident("a"), None),
            ],
        ))]);
        let (out, _c, changed, _n) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 2);
        assert!(
            matches!(
                &out.body[0],
                ProgramItem::Statement(Statement::Declaration(
                    Declaration::VariableDeclaration(_)
                ))
            ),
            "the `var` declaration hoisted out of the block"
        );
    }

    #[test]
    fn block_with_let_is_not_flattened() {
        // `{ let x = 1; a; }` — `let` is block-scoped; flattening would leak the
        // binding into the enclosing scope. Must be kept.
        let prog = program().with_body(vec![ProgramItem::Statement(block(
            None,
            vec![
                make_let_decl("x", Some(num(1.0, None))),
                expr_stmt(ident("a"), None),
            ],
        ))]);
        let (out, _c, _changed, _n) = run_pass(prog);
        assert_eq!(out.body.len(), 1);
        assert!(
            matches!(
                first_stmt(&out),
                Statement::Tagged(TaggedStatement::BlockStatement(_))
            ),
            "a `let` block must NOT be flattened"
        );
    }

    #[test]
    fn block_with_const_is_not_flattened() {
        // `{ const x = 1; }` — `const` is block-scoped: keep the block.
        let const_decl =
            Statement::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
                cv: None,
                kind: VarKind::Const,
                declarations: vec![VariableDeclarator {
                    cv: None,
                    id: BindingTarget::Identifier(Identifier {
                        cv: None,
                        name: "x".to_string(),
                    }),
                    init: Some(num(1.0, None)),
                }],
            }));
        let prog = program().with_body(vec![ProgramItem::Statement(block(None, vec![const_decl]))]);
        let (out, _c, _changed, _n) = run_pass(prog);
        assert_eq!(out.body.len(), 1);
        assert!(matches!(
            first_stmt(&out),
            Statement::Tagged(TaggedStatement::BlockStatement(_))
        ));
    }

    #[test]
    fn block_with_function_declaration_is_not_flattened() {
        // `{ function f(){} }` — a nested function declaration is block-scoped
        // (strict mode) / Annex-B special; hoisting it out could change scope or
        // collide. Keep the block.
        let fdecl = Statement::Declaration(Declaration::FunctionDeclaration(FunctionDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "f".to_string(),
            },
            params: vec![],
            body: BlockStatement {
                cv: None,
                body: vec![],
            },
            generator: false,
            is_async: false,
        }));
        let prog = program().with_body(vec![ProgramItem::Statement(block(None, vec![fdecl]))]);
        let (out, _c, _changed, _n) = run_pass(prog);
        assert_eq!(out.body.len(), 1);
        assert!(matches!(
            first_stmt(&out),
            Statement::Tagged(TaggedStatement::BlockStatement(_))
        ));
    }

    #[test]
    fn if_true_block_consequent_flattens_to_statements() {
        // `if (true) { a; } else { b; }` → `a;` — the branch folds to its
        // consequent block, then that redundant block flattens. This is the
        // exact divergence #4 case (closurec previously emitted `{a}`).
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.1".to_string()),
            test: boolean(true, None),
            consequent: Box::new(block(None, vec![expr_stmt(ident("a"), None)])),
            alternate: Some(Box::new(block(None, vec![expr_stmt(ident("b"), None)]))),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _c, changed, _n) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 1);
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => assert!(
                matches!(&es.expression, Expression::Identifier(i) if i.name == "a"),
                "expected the consequent `a;` un-blocked"
            ),
            other => panic!("expected `a;` after if-fold + flatten, got {other:?}"),
        }
    }

    #[test]
    fn flatten_block_inside_function_body() {
        // `function f(){ { a; } }` → `function f(){ a; }` — exercises the
        // nested (`fold_block_statement`) flatten path, not just program level.
        let prog = fdecl_with_body(vec![block(None, vec![expr_stmt(ident("a"), None)])]);
        let (out, _c, changed, _n) = run_pass(prog);
        assert!(changed);
        let body = extract_fn_body(&out);
        assert_eq!(body.body.len(), 1);
        assert!(
            matches!(
                &body.body[0],
                Statement::Tagged(TaggedStatement::ExpressionStatement(_))
            ),
            "the inner block should flatten inside the function body"
        );
    }

    // ---------------- DIV#2: coalesce adjacent same-kind var decls --------

    /// A program-level variable declaration item (a `var`/`let`/`const`
    /// statement) with one declarator, optionally carrying a CV id.
    fn prog_var(name: &str, init: Option<Expression>, kind: VarKind, cv: Option<&str>) -> ProgramItem {
        ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(
            VariableDeclaration {
                cv: cv.map(|s| s.to_string()),
                kind,
                declarations: vec![VariableDeclarator {
                    cv: None,
                    id: BindingTarget::Identifier(Identifier {
                        cv: None,
                        name: name.to_string(),
                    }),
                    init,
                }],
            },
        )))
    }

    /// The variable declaration at program-body index `idx` (panics otherwise).
    fn prog_decl_at(prog: &Program, idx: usize) -> &VariableDeclaration {
        match &prog.body[idx] {
            ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(vd)))
            | ProgramItem::Declaration(Declaration::VariableDeclaration(vd)) => vd,
            other => panic!("expected a variable declaration at {idx}; got {other:?}"),
        }
    }

    /// The bound names of a declaration, in order (`var a=1,b=2` → `["a","b"]`).
    fn declarator_names(vd: &VariableDeclaration) -> Vec<String> {
        vd.declarations
            .iter()
            .map(|d| {
                let BindingTarget::Identifier(id) = &d.id;
                id.name.clone()
            })
            .collect()
    }

    #[test]
    fn two_adjacent_var_decls_coalesce() {
        // `var a=1;var b=2;` → `var a=1,b=2;`
        let prog = program().with_body(vec![
            prog_var("a", Some(num(1.0, None)), VarKind::Var, None),
            prog_var("b", Some(num(2.0, None)), VarKind::Var, None),
        ]);
        let (out, _c, changed, _) = run_pass(prog);
        assert!(changed, "coalescing should mark the program changed");
        assert_eq!(out.body.len(), 1, "the two decls should merge into one");
        let vd = prog_decl_at(&out, 0);
        assert_eq!(vd.kind, VarKind::Var);
        assert_eq!(declarator_names(vd), vec!["a", "b"]);
        // Inits are preserved in order.
        assert!(matches!(&vd.declarations[0].init, Some(Expression::NumericLiteral(n)) if n.value == 1.0));
        assert!(matches!(&vd.declarations[1].init, Some(Expression::NumericLiteral(n)) if n.value == 2.0));
    }

    #[test]
    fn three_adjacent_var_decls_coalesce_into_one() {
        // `var a=1;var b=2;var c=3;` → `var a=1,b=2,c=3;`
        let prog = program().with_body(vec![
            prog_var("a", Some(num(1.0, None)), VarKind::Var, None),
            prog_var("b", Some(num(2.0, None)), VarKind::Var, None),
            prog_var("c", Some(num(3.0, None)), VarKind::Var, None),
        ]);
        let (out, _c, changed, _) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 1);
        assert_eq!(declarator_names(prog_decl_at(&out, 0)), vec!["a", "b", "c"]);
    }

    #[test]
    fn adjacent_let_decls_coalesce() {
        // `let a=1;let b=2;` → `let a=1,b=2;` — same rule, `let` kind preserved.
        let prog = program().with_body(vec![
            prog_var("a", Some(num(1.0, None)), VarKind::Let, None),
            prog_var("b", Some(num(2.0, None)), VarKind::Let, None),
        ]);
        let (out, _c, changed, _) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 1);
        let vd = prog_decl_at(&out, 0);
        assert_eq!(vd.kind, VarKind::Let);
        assert_eq!(declarator_names(vd), vec!["a", "b"]);
    }

    #[test]
    fn declarations_without_initializers_coalesce() {
        // `var a;var b;` → `var a,b;`
        let prog = program().with_body(vec![
            prog_var("a", None, VarKind::Var, None),
            prog_var("b", None, VarKind::Var, None),
        ]);
        let (out, _c, changed, _) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 1);
        let vd = prog_decl_at(&out, 0);
        assert_eq!(declarator_names(vd), vec!["a", "b"]);
        assert!(vd.declarations.iter().all(|d| d.init.is_none()));
    }

    #[test]
    fn different_kind_decls_do_not_coalesce() {
        // `var a=1;let b=2;` — different kinds stay two separate statements.
        let prog = program().with_body(vec![
            prog_var("a", Some(num(1.0, None)), VarKind::Var, None),
            prog_var("b", Some(num(2.0, None)), VarKind::Let, None),
        ]);
        let (out, _c, changed, _) = run_pass(prog);
        assert!(!changed, "different-kind decls must not merge");
        assert_eq!(out.body.len(), 2);
        assert_eq!(prog_decl_at(&out, 0).kind, VarKind::Var);
        assert_eq!(prog_decl_at(&out, 1).kind, VarKind::Let);
    }

    #[test]
    fn non_adjacent_decls_do_not_coalesce() {
        // `var a=1;f();var b=2;` — the call between them breaks the run.
        let prog = program().with_body(vec![
            prog_var("a", Some(num(1.0, None)), VarKind::Var, None),
            ProgramItem::Statement(expr_stmt(ident("f"), None)),
            prog_var("b", Some(num(2.0, None)), VarKind::Var, None),
        ]);
        let (out, _c, changed, _) = run_pass(prog);
        assert!(!changed, "a statement between the decls must break the run");
        assert_eq!(out.body.len(), 3);
    }

    #[test]
    fn redeclaration_declines_to_preserve_byte_identity() {
        // `var a=1;var a=2;` must NOT become `var a=1,a=2;` — the repeated name
        // is a redeclaration the reference compiler rewrites differently
        // (`var a=1;a=2;`), so this pass leaves the run untouched.
        let prog = program().with_body(vec![
            prog_var("a", Some(num(1.0, None)), VarKind::Var, None),
            prog_var("a", Some(num(2.0, None)), VarKind::Var, None),
        ]);
        let (out, _c, changed, _) = run_pass(prog);
        assert!(!changed, "a repeated binding name must decline the merge");
        assert_eq!(out.body.len(), 2, "the two decls stay separate");
    }

    #[test]
    fn lone_declaration_is_left_alone() {
        let prog =
            program().with_body(vec![prog_var("a", Some(num(1.0, None)), VarKind::Var, None)]);
        let (out, _c, changed, _) = run_pass(prog);
        assert!(!changed);
        assert_eq!(out.body.len(), 1);
        assert_eq!(declarator_names(prog_decl_at(&out, 0)), vec!["a"]);
    }

    #[test]
    fn flattened_block_decls_then_coalesce() {
        // `{var a=1;var b=2;}` at program level: the scope-safe block flattens
        // into the program body (CLOC12.194), and the now-adjacent decls then
        // coalesce — the two rewrites compose to `var a=1,b=2;`.
        let block = Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![
                make_var_decl("a", Some(num(1.0, None))),
                make_var_decl("b", Some(num(2.0, None))),
            ],
        });
        let prog = program().with_body(vec![ProgramItem::Statement(block)]);
        let (out, _c, changed, _) = run_pass(prog);
        assert!(changed);
        assert_eq!(out.body.len(), 1, "flatten + coalesce compose to one decl");
        assert_eq!(declarator_names(prog_decl_at(&out, 0)), vec!["a", "b"]);
    }

    #[test]
    fn coalesce_tombstones_the_folded_away_declaration_cv() {
        // Merging `var a=1;var b=2;` deletes the second decl's statement
        // wrapper; its CV id must be tombstoned with the container recorded.
        let mut log = CVLog::new(true);
        let second_id = log.create(None);
        let prog = program().with_body(vec![
            prog_var("a", Some(num(1.0, None)), VarKind::Var, Some("v.first")),
            prog_var("b", Some(num(2.0, None)), VarKind::Var, Some(second_id.as_str())),
        ]);
        let out = run_capturing_cv(&prog, &mut log);
        assert_eq!(out.body.len(), 1, "the decls should have merged");
        let del = log
            .get(&second_id)
            .unwrap()
            .deleted
            .as_ref()
            .expect("the folded-away declaration must be tombstoned");
        assert_eq!(del.source, "fold-control-flow");
        assert_eq!(del.reason, "coalesce-var-declarations");
        assert_eq!(
            del.meta.get("container_cv").and_then(|v| v.as_str()),
            Some("v.first"),
            "tombstone should record the surviving container decl's cv"
        );
    }
}
