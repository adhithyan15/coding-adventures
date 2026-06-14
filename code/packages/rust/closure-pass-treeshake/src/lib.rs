//! Tree-shaking pass for the Closure Compiler clone.
//!
//! Per [CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
//! canonical pass set. Removes `export` declarations and `import`
//! bindings that aren't reachable from any entry point — the
//! whole-program version of DCE.
//!
//! # Why "tree-shake"?
//!
//! Imagine the program as a tree of modules connected by
//! `import` edges. The roots are the entry-point modules. Hold
//! the tree by its roots, shake it — anything not connected to a
//! root falls off. That's tree-shaking.
//!
//! ```text
//! entry.js  imports  { a, b }       from utils.js
//! utils.js  exports  a, b, c, d
//!                            ▲
//!                            └─ c and d are unreached;
//!                               treeshake removes them
//! ```
//!
//! Real bundles often pull in entire modules just because one
//! function is needed, then bundle the other 47 functions
//! along for the ride. Tree-shake reads the actual `import`
//! statements and trims everything else away.
//!
//! # The difference from DCE
//!
//! - **DCE** operates **within** a module/function. It finds
//!   bindings that no in-module code uses. It can't tell that
//!   an `export` is unused because the export is, by definition,
//!   reachable from the module's outside.
//! - **Tree-shake** operates **across** modules. It looks at the
//!   import graph and decides which exports are *actually* used
//!   by something that's actually used. Once tree-shake decides
//!   an export is dead, *intra*-module DCE on the next iteration
//!   can finally delete the underlying definition.
//!
//! That's why CLOC06 pins tree-shake to depend on `dce`: DCE
//! shrinks the reachable use-sets first, which simplifies the
//! cross-module use-chain analysis tree-shake needs.
//!
//! # Why `FixedPoint`?
//!
//! Tree-shake → DCE → tree-shake again is a real cascade. If
//! tree-shake removes module `utils.js` entirely, then DCE
//! deletes the now-dead `import` statements in `entry.js`,
//! which can in turn make *more* modules' exports unreferenced.
//! Bounded by module count in practice; we don't pre-cap it.
//!
//! # Scope (v1)
//!
//! `javascript-ast` ships only `Program` / `SourceType` today
//! (CLOC02 Phase 1). With no `ImportDeclaration` /
//! `ExportDeclaration` / `Identifier` nodes there is nothing to
//! shake; [`TreeshakePass::run`] is identity in v1. Per CLOC03
//! §"When a pass keeps a node unchanged" no contributions are
//! emitted.
//!
//! What this PR locks down:
//!
//! 1. Pass metadata (`name`, `iteration_policy`, `cost`,
//!    `depends_on`) — what the scheduler keys on and what the
//!    future `closurec` CLI surfaces as `--disable=treeshake`.
//! 2. The `depends_on("dce")` edge so the scheduler forces
//!    intra-module DCE before cross-module shaking.
//! 3. The two-pass integration test that proves the ordering.

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use std::collections::HashSet;

use coding_adventures_closure_scope_analyzer::{analyze, BindingId, BindingKind, ScopeId};
use coding_adventures_javascript_ast::{Declaration, ProgramItem};

/// `Pass::depends_on` value. CLOC06 canonical order pins DCE
/// before tree-shake. Kept as a `const` so future tests / sibling
/// crates can reference the dependency name without retyping.
const DEPS: &[&str] = &["dce"];

/// Tree-shaking pass. v1 is identity — see crate-level docs.
///
/// Zero-sized type: no per-instance state. Pass-internal state
/// (cross-module import graph, root-set seeded from entry points
/// and `export` markers that the host declared reachable, the
/// per-module set of reached identifiers) lives in pass-local
/// maps constructed inside [`Pass::run`] per CLOC06
/// §"Pass-internal state."
#[derive(Debug, Default, Clone, Copy)]
pub struct TreeshakePass;

impl TreeshakePass {
    /// Zero-arg constructor for ergonomic
    /// `PassPipeline::add(Box::new(TreeshakePass::new()))`
    /// registration.
    pub fn new() -> Self {
        Self
    }
}

impl Pass for TreeshakePass {
    fn name(&self) -> &'static str {
        "treeshake"
    }

    fn depends_on(&self) -> &[&'static str] {
        // CLOC06 canonical order: DCE first so intra-module dead
        // code is gone before cross-module use-chains get
        // analyzed. Shrinks the search space tree-shake walks.
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // Per CLOC06: tree-shake → DCE → tree-shake is a real
        // cascade. Removing one export can leave its `import`
        // statement dead, which DCE deletes, which can render a
        // whole imported module unreached, which tree-shake can
        // then remove entirely.
        IterationPolicy::FixedPoint
    }

    fn cost(&self) -> u32 {
        // Two-phase per iteration:
        //   1. Mark — reachability walk from entry points across
        //      the import graph.
        //   2. Sweep — remove exports/imports not in the reached
        //      set.
        // Similar shape to DCE (also mark+sweep), so similar
        // cost. The cross-module walk is the main difference;
        // tree-shake's reachable-set is per-module instead of
        // per-function, which is comparable in size.
        3
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        // CLOC13.C wiring: consume the shared `ScopeAnalysis` from
        // `closure-scope-analyzer` to identify *module-shape
        // candidates* — bindings whose kind (`Function` / `Class`)
        // is the shape that becomes a module export once
        // `javascript-ast` grows `ImportDeclaration` /
        // `ExportDeclaration` variants.
        //
        // The algorithm (mark phase; sweep deferred):
        //
        //   1. Walk `analysis.bindings`. A binding is a
        //      *module-shape candidate* when:
        //        a. kind == Function OR kind == Class  (these are
        //           the only binding shapes ESM allows to be
        //           exported as a named export from the top
        //           level. `Var`/`Let`/`Const` *can* be exported
        //           but cross over to remove-unused-vars and
        //           collapse-properties; the cleanest split is
        //           kind-based.)
        //   2. Track candidates in a Vec<BindingId> for the
        //      observability path.
        //   3. Sweep — deferred to CLOC13.C.1 because *removing* a
        //      function/class binding cleanly requires the AST to
        //      have ImportDeclaration / ExportDeclaration nodes
        //      (otherwise treeshake can't tell an exported
        //      function from an internal one).
        //
        // **Critical (lesson from CLOC13.E security review):**
        // `changed` is hard-pinned to `false` until step 3 lands.
        // Reporting `changed = true` while returning an unchanged
        // program would cause the scheduler under
        // `IterationPolicy::FixedPoint` to re-run forever — each
        // iteration would find the same candidates, claim a
        // change, return the same program, repeat. Documented in
        // both the code and the CHANGELOG so the next contributor
        // doesn't reintroduce the bug.
        //
        // **Why this is safe with the v0.1.0 analyzer body.** The
        // current `analyze` returns empty bindings + references,
        // so the candidate scan finds zero shapes, the candidates
        // vec is empty, `nodes_touched` is small, and the program
        // passes through unchanged. The wiring becomes *effective*
        // (real shape-finding) the moment CLOC13.0 lands the
        // analyzer body — no churn here.

        let analysis = analyze(ctx.program);

        // Step 1: use-count per binding (same shape as
        // remove-unused-vars / collapse-properties / inline).
        // Unresolved references (binding = None) — free globals
        // like `console` — don't increment any count.
        let mut use_count: Vec<usize> = vec![0; analysis.bindings.len()];
        for reference in &analysis.references {
            if let Some(BindingId(idx)) = reference.binding {
                if let Some(slot) = use_count.get_mut(idx as usize) {
                    *slot += 1;
                }
            }
        }

        // Step 2: dead-shape scan. A binding is *dead* (removable
        // by treeshake) when ALL of:
        //   - kind ∈ { Function, Class } (the module-export-shaped
        //     kinds; Var/Let/Const cross over to remove-unused-vars
        //     and collapse-properties)
        //   - uses == 0 (no reference resolves to it)
        //   - scope == ScopeId::GLOBAL (the only scope CLOC13.0
        //     populates today; restricting here keeps the apply
        //     step correct when CLOC13.0.2 introduces nested
        //     scopes, since we only act on top-level names below)
        //
        // `#[non_exhaustive]` on BindingKind: future variants
        // conservatively passthrough via wildcard.
        let mut dead_names: HashSet<String> = HashSet::new();
        for (idx, binding) in analysis.bindings.iter().enumerate() {
            let uses = use_count.get(idx).copied().unwrap_or(0);
            if uses != 0 {
                continue;
            }
            if binding.scope != ScopeId::GLOBAL {
                continue;
            }
            match binding.kind {
                BindingKind::Function | BindingKind::Class => {
                    dead_names.insert(binding.name.clone());
                }
                _ => {}
            }
        }

        // Step 3 — APPLY. CLOC13.C.1 lifts the hard-pin on
        // `changed` and actually mutates the program.
        //
        // Walk program.body. For each item:
        //   - Declaration::FunctionDeclaration: drop if its name
        //     is in dead_names; passthrough otherwise.
        //   - Declaration::VariableDeclaration: passthrough
        //     unconditionally (Var/Let/Const are not treeshake's
        //     responsibility — remove-unused-vars handles them).
        //   - ProgramItem::Statement: passthrough. Statement
        //     walking lands as references arrive (CLOC13.0.1).
        //
        // **Pin lifted; safety preserved.** `changed = removed >
        // 0` is safe because we genuinely mutated when we say we
        // did. If `removed == 0` (no dead Function/Class bindings
        // in GLOBAL), the program passes through unchanged with
        // `changed = false`.
        //
        // **Why this stays safe under FixedPoint.** Each iteration
        // strictly reduces the binding set: a removed
        // FunctionDeclaration produces no new bindings, and the
        // dropped function body can't introduce *new* references
        // because those references were already inside the dead
        // function and resolved either (a) to other dead bindings
        // (removed in this same iteration) or (b) to live
        // bindings (whose use_count was incremented by those refs;
        // removing the refs decrements the count, possibly making
        // *those* bindings newly-dead in the next iteration). The
        // fixed point reaches when no Function/Class binding has
        // zero refs.

        let mut new_body: Vec<ProgramItem> = Vec::with_capacity(ctx.program.body.len());
        let mut removed_count: usize = 0;
        for item in &ctx.program.body {
            match item {
                ProgramItem::Declaration(Declaration::FunctionDeclaration(fd))
                    if dead_names.contains(&fd.id.name) =>
                {
                    removed_count += 1;
                    // Drop.
                }
                _ => new_body.push(item.clone()),
            }
        }

        let mut new_program = ctx.program.clone();
        new_program.body = new_body;
        let changed = removed_count > 0;

        Ok(PassOutput {
            program: new_program,
            contributions: Vec::new(),
            changed,
            diagnostics: Vec::new(),
            stats: PassStats {
                // Visited the program root + every binding +
                // every reference. Real cost numbers for the
                // scheduler.
                nodes_touched: (1
                    + analysis.bindings.len()
                    + analysis.references.len()) as u32,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    //! Tests pin the public contract and lock the DCE ordering
    //! integration with `PassPipeline`. v1's `run` is identity,
    //! but the metadata drives the scheduler and outlives the
    //! v1 body.
    use super::*;
    use coding_adventures_closure_pass_dce::DcePass;
    use coding_adventures_closure_pass_pipeline::{PassPipeline, PipelineOutput};
    use coding_adventures_correlation_vector::CVLog;
    use coding_adventures_javascript_ast::{Program, SourceType};
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }

    #[test]
    fn name_is_treeshake() {
        // The handle for `--disable=treeshake` and
        // `out.stats["treeshake"]`. Public contract.
        assert_eq!(TreeshakePass::new().name(), "treeshake");
    }

    #[test]
    fn iteration_policy_is_fixed_point() {
        // tree-shake → DCE → tree-shake cascade. FixedPoint
        // captures the intent even though v1 converges in one
        // no-op step.
        assert_eq!(
            TreeshakePass::new().iteration_policy(),
            IterationPolicy::FixedPoint
        );
    }

    #[test]
    fn cost_is_three_pass_units() {
        // Mark + sweep, cross-module. Same shape as DCE.
        assert_eq!(TreeshakePass::new().cost(), 3);
    }

    #[test]
    fn depends_on_dce() {
        let p = TreeshakePass::new();
        assert_eq!(p.depends_on(), &["dce"]);
    }

    #[test]
    fn invalidates_empty_in_v1() {
        // CLOC06 Open Question 1: informational only in v0.1.0.
        assert!(TreeshakePass::new().invalidates().is_empty());
    }

    #[test]
    fn run_on_empty_program_returns_unchanged_identity() {
        // Identity: same CV, version, source_type; no
        // contributions, no diagnostics, changed=false,
        // nodes_touched=1.
        let pass = TreeshakePass::new();
        let prog = program();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);

        let ctx = PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        };
        let out = pass.run(ctx).expect("pass should succeed");

        assert_eq!(out.program.cv, prog.cv);
        assert_eq!(out.program.version, prog.version);
        assert_eq!(out.program.source_type, prog.source_type);
        assert!(!out.changed);
        assert!(out.contributions.is_empty());
        assert!(out.diagnostics.is_empty());
        assert_eq!(out.stats.nodes_touched, 1);
    }

    #[test]
    fn pipeline_orders_dce_before_treeshake() {
        // Register TreeshakePass FIRST. If `depends_on` were
        // ignored, the pipeline would run them in registration
        // order: [treeshake, dce]. The scheduler must reorder
        // them to [dce, treeshake] per CLOC06 canonical order.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(TreeshakePass::new()));
        pipeline.add(Box::new(DcePass::new()));

        let mut cv = CVLog::new(true);
        let out: PipelineOutput = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        assert_eq!(
            out.execution_order,
            vec!["dce".to_string(), "treeshake".to_string()],
            "treeshake must run after dce per CLOC06 canonical order"
        );
        assert!(out.stats.contains_key("dce"));
        assert!(out.stats.contains_key("treeshake"));
    }

    #[test]
    fn pipeline_runs_treeshake_as_solo_pass() {
        // TreeshakePass alone (without DCE) still works:
        // depends_on is "soft" in v1 — the v0.1.0 scheduler
        // silently drops unknown dependencies.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(TreeshakePass::new()));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert_eq!(out.execution_order, vec!["treeshake".to_string()]);
        assert_eq!(out.stats["treeshake"].nodes_touched, 1);
        // FixedPoint policy → v0.1.0 pipeline emits the
        // "not yet iterated" informational note.
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.group.0 == "pipeline.fixed-point-not-yet-iterated"),
            "expected the pipeline's FixedPoint note; got {:?}",
            out.diagnostics
        );
    }

    #[test]
    fn pass_is_default_and_clone() {
        let _a: TreeshakePass = Default::default();
        let _b: TreeshakePass = TreeshakePass::new();
        let _c = _b;
        let _d = _c.clone();
    }

    // -----------------------------------------------------------------
    // CLOC13.C.1 — apply-step tests.
    //
    // Treeshake drops top-level FunctionDeclarations whose names are
    // in the dead set (Function/Class kind, scope=GLOBAL, uses=0).
    // These tests exercise the body-walk machinery directly via the
    // Pass trait, not through the pipeline.
    // -----------------------------------------------------------------

    use coding_adventures_javascript_ast::{
        BindingTarget, BlockStatement, Expression, ExpressionStatement, FunctionDeclaration,
        Identifier, NumericLiteral, Statement, VarKind, VariableDeclaration,
        VariableDeclarator,
    };

    fn ident(name: &str) -> Identifier {
        Identifier {
            cv: None,
            name: name.to_string(),
        }
    }

    fn fn_decl(name: &str) -> ProgramItem {
        ProgramItem::Declaration(Declaration::FunctionDeclaration(FunctionDeclaration {
            cv: None,
            id: ident(name),
            params: Vec::new(),
            body: BlockStatement {
                cv: None,
                body: Vec::new(),
            },
            generator: false,
            is_async: false,
        }))
    }

    fn var_decl_simple(name: &str) -> ProgramItem {
        ProgramItem::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind: VarKind::Let,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(ident(name)),
                init: None,
            }],
        }))
    }

    fn program_with(items: Vec<ProgramItem>) -> Program {
        let mut p = program();
        p.body = items;
        p
    }

    fn run_pass(prog: Program) -> coding_adventures_closure_pass_pipeline::PassOutput {
        let pass = TreeshakePass::new();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let ctx = coding_adventures_closure_pass_pipeline::PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        };
        pass.run(ctx).expect("pass ran")
    }

    #[test]
    fn apply_step_drops_unreferenced_function() {
        // `function f() {}` with no callers anywhere → f is dead
        // → apply step drops it. Works under both pre- and post-
        // CLOC13.0.1 analyzer (no `f()` call site means use_count=0
        // either way).
        let prog = program_with(vec![fn_decl("f")]);
        let out = run_pass(prog);
        assert!(out.changed, "unreferenced function should be removed");
        assert!(out.program.body.is_empty(), "no items survive");
    }

    #[test]
    fn apply_step_drops_multiple_unreferenced_functions() {
        // `function f() {} function g() {} function h() {}` — all
        // dropped. Pin that the loop processes every item.
        let prog = program_with(vec![fn_decl("f"), fn_decl("g"), fn_decl("h")]);
        let out = run_pass(prog);
        assert!(out.changed);
        assert!(out.program.body.is_empty());
    }

    #[test]
    fn apply_step_passes_var_declarations_through() {
        // `let x;` — treeshake does NOT remove Var/Let/Const,
        // even when use_count == 0. That's remove-unused-vars'
        // job. Passthrough should leave the item intact.
        let prog = program_with(vec![var_decl_simple("x")]);
        let out = run_pass(prog.clone());
        assert!(!out.changed, "Let-kind binding is not treeshake's target");
        assert_eq!(out.program.body, prog.body);
    }

    #[test]
    fn apply_step_passes_statements_through() {
        // Top-level ExpressionStatements are not declarations and
        // never enter the dead-set; passthrough unconditionally.
        let stmt = ProgramItem::Statement(Statement::expression_statement(
            ExpressionStatement {
                cv: None,
                expression: Expression::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 1.0,
                    raw: "1".to_string(),
                }),
            },
        ));
        let prog = program_with(vec![stmt]);
        let out = run_pass(prog.clone());
        assert!(!out.changed);
        assert_eq!(out.program.body, prog.body);
    }

    #[test]
    fn apply_step_mixed_program_drops_only_functions() {
        // `let x; function f() {} let y;` — only `f` is dropped;
        // the `let` declarations pass through. Pins that the apply
        // step's `match` arm correctly partitions Function decls
        // from other items.
        //
        // Under empty analyzer references (pre-CLOC13.0.1), use
        // counts are all 0; only `f` enters the dead set because of
        // the BindingKind::Function | Class filter at step 2.
        let prog = program_with(vec![
            var_decl_simple("x"),
            fn_decl("f"),
            var_decl_simple("y"),
        ]);
        let out = run_pass(prog);
        assert!(out.changed);
        assert_eq!(out.program.body.len(), 2);
        // Surviving items are the two var declarations, in order.
        match &out.program.body[0] {
            ProgramItem::Declaration(Declaration::VariableDeclaration(vd)) => {
                let BindingTarget::Identifier(id) = &vd.declarations[0].id;
                assert_eq!(id.name, "x");
            }
            _ => panic!("expected the let x declaration to survive at index 0"),
        }
        match &out.program.body[1] {
            ProgramItem::Declaration(Declaration::VariableDeclaration(vd)) => {
                let BindingTarget::Identifier(id) = &vd.declarations[0].id;
                assert_eq!(id.name, "y");
            }
            _ => panic!("expected the let y declaration to survive at index 1"),
        }
    }

    #[test]
    fn apply_step_empty_program_no_change() {
        // Pin the invariant: empty program → no work → changed=false.
        let prog = program();
        let out = run_pass(prog.clone());
        assert!(!out.changed);
        assert_eq!(out.program.body, prog.body);
    }

    #[test]
    fn apply_step_keeps_function_when_called() {
        // `function f() {} f();` — the function is referenced by
        // the top-level `f()` call site, so it must NOT be dropped.
        //
        // This is the retention test that was intentionally
        // deferred from CLOC13.C.1 (PR #4803). At the time, the
        // analyzer (CLOC13.0.1 / PR #4800) only collected
        // references inside top-level ExpressionStatements; the
        // `f()` call WAS in such a statement, so the reference
        // would have been emitted with `from_scope = GLOBAL` —
        // but the lookup would have correctly resolved `f` to
        // the function binding either way.
        //
        // With CLOC13.0.2 (PR #4825) on main, the analyzer now
        // walks function bodies under nested Function scopes too,
        // and reference resolution walks the parent chain. This
        // test exercises the simpler top-level-callee case, and
        // pins the retention contract: use_count[f] == 1 ⇒
        // treeshake skips ⇒ program unchanged ⇒ changed == false.
        use coding_adventures_javascript_ast::{
            CallExpression, Expression, ExpressionStatement, Identifier, Statement,
        };
        let call_f = ProgramItem::Statement(Statement::expression_statement(
            ExpressionStatement {
                cv: None,
                expression: Expression::CallExpression(CallExpression {
                    cv: None,
                    callee: Box::new(Expression::Identifier(Identifier {
                        cv: None,
                        name: "f".to_string(),
                    })),
                    arguments: Vec::new(),
                }),
            },
        ));
        let prog = program_with(vec![fn_decl("f"), call_f]);
        let out = run_pass(prog.clone());
        assert!(!out.changed, "f is referenced by the call site");
        assert_eq!(out.program.body.len(), 2);
        // The function declaration survives at index 0.
        match &out.program.body[0] {
            ProgramItem::Declaration(Declaration::FunctionDeclaration(fd)) => {
                assert_eq!(fd.id.name, "f");
            }
            _ => panic!("expected the function declaration to survive at index 0"),
        }
    }
}
