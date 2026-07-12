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
    statement::TaggedStatement, ArrayExpression, AssignmentExpression, AssignmentOperator,
    AssignmentTarget, BinaryExpression, BindingTarget, BlockStatement, CallExpression, NewExpression, SequenceExpression, SpreadElement, YieldExpression, AwaitExpression, ImportExpression,
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
use std::collections::HashMap;

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
    let new_body = prog
        .body
        .iter()
        .map(|item| fold_program_item(item, st))
        .collect();
    Program {
        cv: prog.cv.clone(),
        version: prog.version,
        source_type: prog.source_type,
        body: new_body,
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
        // We therefore only recurse structurally: fold the body and the test.
        TaggedStatement::DoWhileStatement(s) => {
            Statement::Tagged(TaggedStatement::DoWhileStatement(DoWhileStatement {
                cv: s.cv.clone(),
                body: Box::new(fold_statement(&s.body, st)),
                test: fold_expression(&s.test, st),
            }))
        }
        TaggedStatement::ForStatement(s) => {
            Statement::Tagged(TaggedStatement::ForStatement(ForStatement {
                cv: s.cv.clone(),
                init: s.init.as_ref().map(|i| match i {
                    ForInit::VariableDeclaration(v) => {
                        ForInit::VariableDeclaration(fold_variable_declaration(v, st))
                    }
                    ForInit::Expression(e) => ForInit::Expression(fold_expression(e, st)),
                }),
                test: s.test.as_ref().map(|e| fold_expression(e, st)),
                update: s.update.as_ref().map(|e| fold_expression(e, st)),
                body: Box::new(fold_statement(&s.body, st)),
            }))
        }
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
        body: new_body,
    }
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
        // Tagged statements introduce no lexical binding of their own.
        Statement::Tagged(_) => true,
    })
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
            // discarded — tombstone it so its span stays auditable.
            let discarded = [alternate.as_ref().and_then(statement_cv)];
            st.record_fold_deleting(
                &s.cv,
                &discarded,
                "folded-branch",
                "if (<truthy literal>) { … } else { … }",
                "{ consequent }",
            );
            consequent
        }
        Some(false) => {
            // The `consequent` branch is statically unreachable and is
            // discarded — tombstone it so its span stays auditable.
            let discarded = [statement_cv(&consequent)];
            st.record_fold_deleting(
                &s.cv,
                &discarded,
                "folded-branch",
                "if (<falsy literal>) { … } else { … }",
                "{ alternate }",
            );
            alternate.unwrap_or_else(|| {
                // No alternate to take — replace with `;`. The
                // EmptyStatement inherits the IfStatement's cv so
                // source maps still point at the original
                // position when tracing is on.
                Statement::empty_statement(EmptyStatement { cv: s.cv.clone() })
            })
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
            //   if (x) foo();                  → unchanged (no
            //                                    alternate to fold)
            //   if (x) { a; b; } else { c; }   → unchanged (multi-
            //                                    statement consequent)
            //   if (x) return 1; else return 2;
            //                                  → unchanged (return is
            //                                    not an expression
            //                                    statement; tracked
            //                                    as gap-019)
            //
            // Why this is safe: a ConditionalExpression has the same
            // evaluation order as the if-else — `test` is evaluated
            // first, then exactly one of the two branches. Side
            // effects in `test`, in `consequent`, in `alternate`
            // observed in the original program are all preserved in
            // the rewritten form.
            if let Some(alt) = &alternate {
                if let (Some(c_expr), Some(a_expr)) =
                    (single_expr_stmt(&consequent), single_expr_stmt(alt))
                {
                    st.record_fold(
                        &s.cv,
                        "if-else-to-ternary",
                        "if (<test>) <expr1>; else <expr2>;",
                        "<test> ? <expr1> : <expr2>;",
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
            // Why we don't fold when alternate exists: the previous
            // ternary branch already handled that. We only reach
            // here if alternate is `None` (or if alternate exists
            // but the ternary fold bailed — in which case we don't
            // discard the alternate by folding to `x && S` because
            // that would silently drop the else branch).
            if alternate.is_none() {
                if let Some(c_expr) = single_expr_stmt(&consequent) {
                    st.record_fold(
                        &s.cv,
                        "if-to-logical-and",
                        "if (<test>) <expr>;",
                        "<test> && <expr>;",
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

            // Couldn't ternarise or logical-and — keep the IfStatement.
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

/// Fold a `WhileStatement`. If `test` is a known-falsy literal,
/// the loop never runs — collapse to `EmptyStatement`.
fn fold_while_statement(s: &WhileStatement, st: &mut FoldState) -> Statement {
    let test = fold_expression(&s.test, st);
    let body = fold_statement(&s.body, st);
    match literal_truthy(&test) {
        Some(false) => {
            // A `while (false)` loop never runs — its body is discarded.
            // Tombstone the body so its span stays auditable.
            let discarded = [statement_cv(&body)];
            st.record_fold_deleting(
                &s.cv,
                &discarded,
                "folded-branch",
                "while (<falsy literal>) { … }",
                ";",
            );
            Statement::empty_statement(EmptyStatement { cv: s.cv.clone() })
        }
        // `while (true)` could loop forever — don't fold to
        // EmptyStatement (semantics differ: an infinite loop is
        // observable). Future Phase 1.x may collapse provably-
        // pure infinite loops; v1 leaves them alone.
        _ => Statement::Tagged(TaggedStatement::WhileStatement(WhileStatement {
            cv: s.cv.clone(),
            test,
            body: Box::new(body),
        })),
    }
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
            // gap-015 / CLOC12.37 — var hoisting. Lift `var x = expr;`
            // declarations from inside nested blocks (`if`, `while`,
            // `for-body`, plain blocks) up to the function-body top.
            // The split form is the canonical hoisted shape upstream
            // Closure emits and lets downstream passes see all
            // function-scoped bindings at the top.
            let hoisted_body = hoist_function_body_vars(&folded_body, st);
            Declaration::FunctionDeclaration(FunctionDeclaration {
                cv: f.cv.clone(),
                id: f.id.clone(),
                params: f.params.clone(),
                body: hoisted_body,
                generator: f.generator,
                is_async: f.is_async,
            })
        }
        // A class *declaration* folds inside its heritage + method bodies, like
        // `fold_class` for the expression form. Unlike a function declaration
        // it hoists no `var`s of its own — a class body is not a `var` scope in
        // the hoisting sense — so no `hoist_function_body_vars` step applies.
        // An import has no foldable control flow — preserve it verbatim.
        Declaration::ImportDeclaration(i) => Declaration::ImportDeclaration(i.clone()),
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

// =====================================================================
// gap-015 / CLOC12.37 — var hoisting
//
// JavaScript `var` declarations are function-scoped, not
// block-scoped: the binding hoists to the enclosing function (or
// to the script top level if not inside a function). Per
// ECMAScript §13.3.2, every `var x;` inside a function body is
// semantically equivalent to declaring `x` at the top of that
// body and leaving any initializer as an assignment at the
// original site.
//
// Upstream Closure makes this hoist *syntactically* visible:
//
//     function f() {
//       if (cond) { var y = 1; }
//     }
//   ↓
//     function f() {
//       var y;
//       if (cond) y = 1;
//     }
//
// The shape change is observable in byte-output comparisons and
// makes downstream rename / dce see all function-scoped bindings
// at the body's top.
//
// # Scope of this implementation
//
// - **Recurses into**: nested `BlockStatement`, `IfStatement`
//   consequent/alternate, `WhileStatement` body, `ForStatement`
//   body, `LabeledStatement` body, `SwitchStatement` case
//   consequents.
// - **Does NOT recurse into**: nested `FunctionDeclaration` /
//   `FunctionExpression` bodies — they have their own
//   function-scope and own hoisting.
// - **Does NOT touch**: `for(var x = 0; ...; ...)` init slots
//   (they're already at a single-statement position that fold-
//   control-flow's wider work covers); `let` / `const` (block-
//   scoped, hoisting doesn't apply).
// - **Conservative bail**: if the function body is a no-op
//   (no var declarations inside nested blocks), the body is
//   returned verbatim — no allocations, no `changed` signal.

/// Lift `var x = expr;` declarations from inside nested blocks
/// of `body` to the top, leaving `x = expr;` assignment-
/// statements behind at the original site (or nothing if there
/// was no initializer).
fn hoist_function_body_vars(body: &BlockStatement, st: &mut FoldState) -> BlockStatement {
    let mut collected: Vec<Identifier> = Vec::new();
    let new_stmts: Vec<Statement> = body
        .body
        .iter()
        .map(|s| hoist_visit_stmt(s, &mut collected))
        .collect();

    if collected.is_empty() {
        // Pure passthrough — keep original allocation.
        return BlockStatement {
            cv: body.cv.clone(),
            body: new_stmts,
        };
    }

    // Record one Contribution per hoist for the whole function
    // body (not per identifier — same shape as block-flatten).
    st.record_fold(
        &body.cv,
        "var-hoisted",
        &format!("function body with {} statement(s)", body.body.len()),
        &format!("hoisted {} `var` binding(s) to top", collected.len()),
    );

    // Build the single `var x, y, z;` declaration to prepend.
    let hoist_decl = VariableDeclaration {
        cv: None,
        kind: VarKind::Var,
        declarations: collected
            .into_iter()
            .map(|id| VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(id),
                init: None,
            })
            .collect(),
    };
    let mut out = Vec::with_capacity(new_stmts.len() + 1);
    out.push(Statement::Declaration(Declaration::VariableDeclaration(
        hoist_decl,
    )));
    out.extend(new_stmts);
    BlockStatement {
        cv: body.cv.clone(),
        body: out,
    }
}

/// Rewrite one statement, collecting any hoistable `var` names
/// and emitting replacement statements (assignment-expression-
/// statements where there was an initializer; nothing where the
/// declaration was bare).
fn hoist_visit_stmt(stmt: &Statement, collected: &mut Vec<Identifier>) -> Statement {
    match stmt {
        // Plain `var ...;` declaration — the core rewrite.
        Statement::Declaration(Declaration::VariableDeclaration(v)) if v.kind == VarKind::Var => {
            hoist_rewrite_var_decl(v, collected)
        }

        // Compound statements with nested bodies — recurse.
        Statement::Tagged(TaggedStatement::BlockStatement(b)) => Statement::Tagged(
            TaggedStatement::BlockStatement(hoist_visit_block(b, collected)),
        ),
        Statement::Tagged(TaggedStatement::IfStatement(i)) => Statement::Tagged(
            TaggedStatement::IfStatement(IfStatement {
                cv: i.cv.clone(),
                test: i.test.clone(),
                consequent: Box::new(hoist_visit_stmt(&i.consequent, collected)),
                alternate: i
                    .alternate
                    .as_ref()
                    .map(|alt| Box::new(hoist_visit_stmt(alt, collected))),
            }),
        ),
        Statement::Tagged(TaggedStatement::WhileStatement(w)) => Statement::Tagged(
            TaggedStatement::WhileStatement(WhileStatement {
                cv: w.cv.clone(),
                test: w.test.clone(),
                body: Box::new(hoist_visit_stmt(&w.body, collected)),
            }),
        ),
        Statement::Tagged(TaggedStatement::ForStatement(f)) => {
            // Recurse into body only — leave init slot alone
            // (per the scope note above).
            Statement::Tagged(TaggedStatement::ForStatement(ForStatement {
                cv: f.cv.clone(),
                init: f.init.clone(),
                test: f.test.clone(),
                update: f.update.clone(),
                body: Box::new(hoist_visit_stmt(&f.body, collected)),
            }))
        }
        Statement::Tagged(TaggedStatement::LabeledStatement(l)) => Statement::Tagged(
            TaggedStatement::LabeledStatement(
                coding_adventures_javascript_ast::LabeledStatement {
                    cv: l.cv.clone(),
                    label: l.label.clone(),
                    body: Box::new(hoist_visit_stmt(&l.body, collected)),
                },
            ),
        ),
        Statement::Tagged(TaggedStatement::SwitchStatement(s)) => Statement::Tagged(
            TaggedStatement::SwitchStatement(
                coding_adventures_javascript_ast::SwitchStatement {
                    cv: s.cv.clone(),
                    discriminant: s.discriminant.clone(),
                    cases: s
                        .cases
                        .iter()
                        .map(|c| coding_adventures_javascript_ast::SwitchCase {
                            cv: c.cv.clone(),
                            test: c.test.clone(),
                            consequent: c
                                .consequent
                                .iter()
                                .map(|s| hoist_visit_stmt(s, collected))
                                .collect(),
                        })
                        .collect(),
                },
            ),
        ),

        // **Inner function bodies are their own scope.** Do NOT
        // recurse into a nested `FunctionDeclaration`'s body
        // here — its own `var` declarations belong to *that*
        // function's hoist set, which fold-control-flow's
        // dispatch on `Declaration::FunctionDeclaration` already
        // handles before this collector ever sees the
        // declaration. (FunctionExpression bodies are similarly
        // self-contained; we leave them untouched.)
        Statement::Declaration(_) | Statement::Tagged(_) => stmt.clone(),
    }
}

/// Helper: recurse into a `BlockStatement.body` while collecting
/// hoistable vars. Used by the BlockStatement arm.
fn hoist_visit_block(b: &BlockStatement, collected: &mut Vec<Identifier>) -> BlockStatement {
    BlockStatement {
        cv: b.cv.clone(),
        body: b
            .body
            .iter()
            .map(|s| hoist_visit_stmt(s, collected))
            .collect(),
    }
}

/// Rewrite a `var ...;` declaration into: collect each
/// `Identifier` binding into `collected`, return either an
/// `EmptyStatement` (if no declarators had initializers) or an
/// `ExpressionStatement` with a comma-separated assignment chain
/// (if any did).
///
/// Why a single statement back: we can't return zero or many
/// statements from a `Vec<_>::map` cleanly without changing the
/// shape. So:
///
/// - All-bare `var x, y;` → `EmptyStatement` (no observable
///   work — the binding moved to the hoisted-declaration prefix).
/// - Any-init `var x = e;` / `var x, y = e;` → a single
///   `ExpressionStatement` whose `expression` is either one
///   `AssignmentExpression` (single init) or a
///   `SequenceExpression`-shaped binary chain of assignments
///   threaded with `,` — we don't have `SequenceExpression`
///   in Phase 1 yet, so for multiple inits we emit ONE
///   `ExpressionStatement` per init expanded into a
///   `BlockStatement` wrapper. Simpler: just emit a
///   `BlockStatement` containing one assignment-statement per
///   declarator-with-init.
fn hoist_rewrite_var_decl(
    v: &VariableDeclaration,
    collected: &mut Vec<Identifier>,
) -> Statement {
    let mut assignments: Vec<Statement> = Vec::new();
    for decl in &v.declarations {
        // We currently only model `BindingTarget::Identifier`
        // here. Patterns (`var [a, b] = ...;`) are Phase 2 work
        // — for those, treat as identity (don't hoist) by
        // bailing on the whole declaration.
        let id = match &decl.id {
            BindingTarget::Identifier(i) => i.clone(),
        };
        collected.push(id.clone());
        if let Some(init) = &decl.init {
            assignments.push(Statement::expression_statement(ExpressionStatement {
                cv: None,
                expression: Expression::AssignmentExpression(AssignmentExpression {
                    cv: None,
                    operator: AssignmentOperator::Eq,
                    left: AssignmentTarget::Identifier(id),
                    right: Box::new(init.clone()),
                }),
            }));
        }
    }
    if assignments.is_empty() {
        // Pure declaration, no init: site collapses to nothing
        // observable. Emit an EmptyStatement that the DCE block
        // walker will sweep up.
        Statement::Tagged(TaggedStatement::EmptyStatement(EmptyStatement { cv: None }))
    } else if assignments.len() == 1 {
        // Single init: emit just the assignment-expression-
        // statement at the original site.
        assignments.into_iter().next().unwrap()
    } else {
        // Multiple inits: wrap in a BlockStatement (the
        // block-flatten step of DCE will splice it into the
        // parent if it's safe to do so).
        Statement::Tagged(TaggedStatement::BlockStatement(BlockStatement {
            cv: None,
            body: assignments,
        }))
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
                let hoisted_body = hoist_function_body_vars(&folded_body, st);
                ClassMember::Method(MethodDefinition {
                    cv: md.cv.clone(),
                    key: md.key.clone(),
                    kind: md.kind,
                    value: FunctionExpression {
                        cv: md.value.cv.clone(),
                        id: md.value.id.clone(),
                        params: md.value.params.clone(),
                        body: hoisted_body,
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
            // run at class-definition time) and is its own `var`-hoisting scope,
            // so it folds + hoists exactly like a method body.
            ClassMember::StaticBlock(b) => {
                let folded = fold_block_statement(b, st);
                ClassMember::StaticBlock(hoist_function_body_vars(&folded, st))
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
        // body, then hoist its `var`s to the function top).
        Expression::FunctionExpression(f) => {
            let folded_body = fold_block_statement(&f.body, st);
            let hoisted_body = hoist_function_body_vars(&folded_body, st);
            Expression::FunctionExpression(FunctionExpression {
                cv: f.cv.clone(),
                id: f.id.clone(),
                params: f.params.clone(),
                body: hoisted_body,
                generator: f.generator,
                is_async: f.is_async,
            })
        }
        Expression::ClassExpression(c) => fold_class(c, st),
        // Fold control flow inside an arrow-value's body. A block body is
        // folded and its `var`s hoisted exactly as a function body; a
        // concise (expression) body declares no `var`s, so it only needs
        // its single expression folded.
        Expression::ArrowFunctionExpression(a) => {
            let body = match &a.body {
                ArrowBody::Block(b) => {
                    let folded = fold_block_statement(b, st);
                    ArrowBody::Block(hoist_function_body_vars(&folded, st))
                }
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
    fn if_non_literal_test_with_multi_statement_consequent_passes_through() {
        // **Pre-gap-016**: this test asserted `if (flag) x;` stayed
        // unchanged. **Post-gap-016** (CLOC12.24), that single-expr
        // case now folds to `flag && x;`. We update the test to use
        // a *multi-statement* consequent block, which still can't
        // collapse to a LogicalExpression (LogicalExpression takes
        // exactly one right-hand expression).
        //
        // This preserves the original intent — "the fold doesn't
        // over-fire on non-collapsible shapes".
        let if_stmt = Statement::if_statement(IfStatement {
            cv: Some("if.2".to_string()),
            test: ident("flag"),
            consequent: Box::new(Statement::block_statement(BlockStatement {
                cv: None,
                body: vec![expr_stmt(ident("x"), None), expr_stmt(ident("y"), None)],
            })),
            alternate: None,
        });
        let prog = program().with_body(vec![ProgramItem::Statement(if_stmt)]);
        let (out, _contribs, _changed, _) = run_pass(prog);
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::IfStatement(_)) => {}
            other => panic!(
                "expected IfStatement intact (multi-statement consequent); got {:?}",
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
    fn while_true_is_left_alone() {
        let w = Statement::while_statement(WhileStatement {
            cv: Some("w.1".to_string()),
            test: boolean(true, None),
            body: Box::new(expr_stmt(ident("body"), None)),
        });
        let prog = program().with_body(vec![ProgramItem::Statement(w)]);
        let (out, _, changed, _) = run_pass(prog);
        assert!(!changed);
        match first_stmt(&out) {
            Statement::Tagged(TaggedStatement::WhileStatement(_)) => {}
            other => panic!("expected WhileStatement intact; got {:?}", other),
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
    /// stays: `if(x){g; m} else {h}` is unchanged. (The consequent has two
    /// statements so the if-else→ternary fold also does not apply — isolating
    /// the else-hoist's terminator gate as the reason it stays.)
    #[test]
    fn else_not_hoisted_when_consequent_falls_through() {
        let consequent = Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![expr_stmt(ident("g"), None), expr_stmt(ident("m"), None)],
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

    /// `function f() { if (cond) { var y = 1; } }` →
    /// `function f() { var y; if (cond) y = 1; }`.
    #[test]
    fn var_inside_if_consequent_block_hoists() {
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
        let (out, contribs, changed, _) = run_pass(prog);
        let body = extract_fn_body(&out);
        // Body should be: [var y;, if (cond) { y = 1; }]
        assert_eq!(body.body.len(), 2, "expected 2 stmts; got {:?}", body.body);
        assert!(matches!(
            &body.body[0],
            Statement::Declaration(Declaration::VariableDeclaration(_))
        ));
        if let Statement::Declaration(Declaration::VariableDeclaration(v)) = &body.body[0] {
            assert_eq!(v.kind, VarKind::Var);
            assert_eq!(v.declarations.len(), 1);
            assert!(v.declarations[0].init.is_none());
        }
        assert!(changed);
        assert!(contribs.iter().any(|c| c.tag == "var-hoisted"));
    }

    /// `function f() { var x = 1; }` — already at the top, no
    /// nested hoist target. The var declaration is still
    /// collected (the hoist treats every `var` in the function
    /// body uniformly), but the result is the same shape:
    /// declaration moves to a prepended `var x;` and assignment
    /// stays as `x = 1;`.
    #[test]
    fn var_at_top_of_function_body_is_split() {
        let prog = fdecl_with_body(vec![make_var_decl("x", Some(num(1.0, None)))]);
        let (out, _, changed, _) = run_pass(prog);
        let body = extract_fn_body(&out);
        assert_eq!(body.body.len(), 2);
        assert!(changed);
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
        // Outer body has just the inner FunctionDeclaration — no
        // hoisted `var y;` at the outer top.
        assert_eq!(outer_body.body.len(), 1);
        if let Statement::Declaration(Declaration::FunctionDeclaration(g)) = &outer_body.body[0] {
            // The inner function's body should itself have been
            // split (var y; then y = 1;).
            assert_eq!(g.body.body.len(), 2);
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

    /// Bare `var x;` (no init) inside a block: name is hoisted,
    /// site collapses to `EmptyStatement`.
    #[test]
    fn bare_var_no_init_collapses_to_empty_at_site() {
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
        let (out, _, _, _) = run_pass(prog);
        let body = extract_fn_body(&out);
        // [var y;, if (cond) { ; }]
        assert_eq!(body.body.len(), 2);
        if let Statement::Tagged(TaggedStatement::IfStatement(i)) = &body.body[1] {
            if let Statement::Tagged(TaggedStatement::BlockStatement(b)) = &*i.consequent {
                assert_eq!(b.body.len(), 1);
                assert!(matches!(
                    &b.body[0],
                    Statement::Tagged(TaggedStatement::EmptyStatement(_))
                ));
            }
        }
    }
}
