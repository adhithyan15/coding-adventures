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
//!    everything after a `ReturnStatement`. Phase 1 doesn't have
//!    `ThrowStatement` yet; `BreakStatement` and `ContinueStatement`
//!    only qualify in their enclosing loop scope (Phase 2 work).
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
    statement::TaggedStatement, ArrayExpression, AssignmentExpression, BinaryExpression,
    BlockStatement, CallExpression, ConditionalExpression, Declaration, Expression,
    ExpressionStatement, ForInit, ForStatement, FunctionDeclaration, IfStatement,
    LogicalExpression, MemberExpression, ObjectExpression, Program, ProgramItem, Property,
    PropertyKey, ReturnStatement, Statement, UnaryExpression, VariableDeclaration,
    VariableDeclarator, WhileStatement,
};
use serde_json::json;

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
            // Threaded `cv` reserved for future CV-invariant
            // bookkeeping the same way fold-control-flow does it.
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

fn dce_program(prog: &Program, st: &mut DceState) -> Program {
    st.visit();
    let new_body = prog
        .body
        .iter()
        .map(|item| dce_program_item(item, st))
        .collect();
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
        TaggedStatement::IfStatement(s) => TaggedStatement::IfStatement(IfStatement {
            cv: s.cv.clone(),
            test: dce_expression(&s.test, st),
            consequent: Box::new(dce_statement(&s.consequent, st)),
            alternate: s.alternate.as_ref().map(|a| Box::new(dce_statement(a, st))),
        }),
        TaggedStatement::WhileStatement(s) => TaggedStatement::WhileStatement(WhileStatement {
            cv: s.cv.clone(),
            test: dce_expression(&s.test, st),
            body: Box::new(dce_statement(&s.body, st)),
        }),
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
        TaggedStatement::ReturnStatement(s) => {
            TaggedStatement::ReturnStatement(ReturnStatement {
                cv: s.cv.clone(),
                argument: s.argument.as_ref().map(|e| dce_expression(e, st)),
            })
        }
        TaggedStatement::BreakStatement(_)
        | TaggedStatement::ContinueStatement(_)
        | TaggedStatement::EmptyStatement(_) => stmt.clone(),
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

    // Drop dead-after-terminator.
    let original_len = working.len();
    if let Some(terminator_idx) = working
        .iter()
        .position(|s| is_terminator(s))
    {
        let dropped = original_len - (terminator_idx + 1);
        if dropped > 0 {
            working.truncate(terminator_idx + 1);
            st.record(
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
    working.retain(|s| !is_empty_statement(s));
    let dropped_empties = before_empty_drop - working.len();
    if dropped_empties > 0 {
        st.record(
            &b.cv,
            "removed-empty-statement",
            &format!("block with {} statements", before_empty_drop),
            &format!("dropped {} empty statements", dropped_empties),
        );
    }

    BlockStatement {
        cv: b.cv.clone(),
        body: working,
    }
}

fn is_terminator(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Tagged(TaggedStatement::ReturnStatement(_))
    )
}

fn is_empty_statement(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Tagged(TaggedStatement::EmptyStatement(_))
    )
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

fn dce_expression(expr: &Expression, st: &mut DceState) -> Expression {
    st.visit();
    match expr {
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => expr.clone(),

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
        Expression::MemberExpression(m) => Expression::MemberExpression(MemberExpression {
            cv: m.cv.clone(),
            object: Box::new(dce_expression(&m.object, st)),
            property: Box::new(dce_expression(&m.property, st)),
            computed: m.computed,
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
                            PropertyKey::Expression(Box::new(dce_expression(e, st)))
                        }
                    },
                    value: Box::new(dce_expression(&p.value, st)),
                    computed: p.computed,
                    shorthand: p.shorthand,
                    method: p.method,
                })
                .collect(),
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
        statement::TaggedStatement, BinaryOperator, BooleanLiteral, EmptyStatement, Identifier,
        NumericLiteral, SourceType,
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
    fn recurses_into_nested_blocks() {
        // { x; { return; y; } z; }
        // Outer block: kept (no top-level terminator at outer
        // scope; the `return` is inside the inner block).
        // Inner block: { return; y; } → { return; } (drop y).
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
        // Outer block still has 3 statements (x, block, z).
        assert_eq!(outer.body.len(), 3);
        // Inner block: extract and check.
        match &outer.body[1] {
            Statement::Tagged(TaggedStatement::BlockStatement(inner)) => {
                assert_eq!(
                    inner.body.len(),
                    1,
                    "inner block should have just `return`; got {:?}",
                    inner.body
                );
            }
            other => panic!("expected nested BlockStatement; got {:?}", other),
        }
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
        assert!(out
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
}
