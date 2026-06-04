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
    statement::TaggedStatement, ArrayExpression, AssignmentExpression, BinaryExpression,
    BlockStatement, CallExpression, ConditionalExpression, Declaration, EmptyStatement,
    Expression, ExpressionStatement, ForInit, ForStatement, FunctionDeclaration, IfStatement,
    LogicalExpression, LogicalOperator, MemberExpression, ObjectExpression, Program, ProgramItem, Property,
    PropertyKey, ReturnStatement, Statement, UnaryExpression, UnaryOperator, VariableDeclaration,
    VariableDeclarator, WhileStatement,
};
use serde_json::json;

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
            // Keep the cv handle threaded for future invariants.
            let _ = self.cv;
            self.contributions.push(contribution);
        }
    }

    fn visit(&mut self) {
        self.nodes_touched += 1;
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
        TaggedStatement::BreakStatement(_)
        | TaggedStatement::ContinueStatement(_)
        | TaggedStatement::EmptyStatement(_) => Statement::Tagged(stmt.clone()),
    }
}

/// Folds `BlockStatement.body` by recursing into each statement
/// AND dropping everything after a `ReturnStatement` (dead code
/// after a definite terminator).
fn fold_block_statement(b: &BlockStatement, st: &mut FoldState) -> BlockStatement {
    let mut new_body = Vec::with_capacity(b.body.len());
    let mut hit_terminator = false;
    let mut dropped_count = 0usize;
    for s in &b.body {
        if hit_terminator {
            dropped_count += 1;
            continue;
        }
        let folded = fold_statement(s, st);
        let terminates = is_terminator(&folded);
        new_body.push(folded);
        if terminates {
            hit_terminator = true;
        }
    }
    if dropped_count > 0 {
        st.record_fold(
            &b.cv,
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
fn is_terminator(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Tagged(TaggedStatement::ReturnStatement(_))
    )
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
    let (test, consequent, alternate) = if alternate.is_some() {
        if let Expression::UnaryExpression(u) = test {
            if u.operator == UnaryOperator::Not {
                let inner = *u.argument;
                st.record_fold(
                    &s.cv,
                    "de-morgan-swap-not",
                    "if (!<inner>) <c>; else <a>;",
                    "if (<inner>) <a>; else <c>;",
                );
                let alt = alternate.expect("alternate.is_some() checked above");
                (inner, alt, Some(consequent))
            } else {
                (Expression::UnaryExpression(u), consequent, alternate)
            }
        } else {
            (test, consequent, alternate)
        }
    } else {
        (test, consequent, alternate)
    };

    match literal_truthy(&test) {
        Some(true) => {
            st.record_fold(
                &s.cv,
                "folded-branch",
                "if (<truthy literal>) { … } else { … }",
                "{ consequent }",
            );
            consequent
        }
        Some(false) => {
            st.record_fold(
                &s.cv,
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

/// Fold a `WhileStatement`. If `test` is a known-falsy literal,
/// the loop never runs — collapse to `EmptyStatement`.
fn fold_while_statement(s: &WhileStatement, st: &mut FoldState) -> Statement {
    let test = fold_expression(&s.test, st);
    let body = fold_statement(&s.body, st);
    match literal_truthy(&test) {
        Some(false) => {
            st.record_fold(
                &s.cv,
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
            Declaration::FunctionDeclaration(FunctionDeclaration {
                cv: f.cv.clone(),
                id: f.id.clone(),
                params: f.params.clone(),
                body: fold_block_statement(&f.body, st),
                generator: f.generator,
                is_async: f.is_async,
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

fn fold_expression(expr: &Expression, st: &mut FoldState) -> Expression {
    st.visit();
    match expr {
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
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
        Expression::MemberExpression(m) => Expression::MemberExpression(MemberExpression {
            cv: m.cv.clone(),
            object: Box::new(fold_expression(&m.object, st)),
            property: Box::new(fold_expression(&m.property, st)),
            computed: m.computed,
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
                .map(|p| Property {
                    cv: p.cv.clone(),
                    kind: p.kind,
                    key: match &p.key {
                        PropertyKey::Identifier(i) => PropertyKey::Identifier(i.clone()),
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
                })
                .collect(),
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
            st.record_fold(
                &c.cv,
                "folded-branch",
                "(<truthy literal>) ? … : …",
                "consequent",
            );
            consequent
        }
        Some(false) => {
            st.record_fold(
                &c.cv,
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
            panic!("expected Statement; got {:?}", &prog.body[0]);
        };
        s
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
        assert!(out
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
}
