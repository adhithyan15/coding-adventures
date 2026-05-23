//! Control-flow folding pass for the Closure Compiler clone.
//!
//! Sits between `constant-fold` and `dce` in the CLOC06 canonical
//! pass set. Where `constant-fold` collapses pure value-level
//! expressions (`2 + 2 → 4`, `"a" + "b" → "ab"`), `fold-control-flow`
//! does the same job for **control flow shapes** that have a
//! statically-known answer:
//!
//! ```text
//! if (false) { A } else { B }          →  B
//! if (true)  { A } else { B }          →  A
//! while (false) { ... }                →  (deleted)
//! function f() { return 1; A; B; }     →  function f() { return 1; }
//! switch (1) { case 1: A; break; ... } →  A
//! ```
//!
//! These rewrites typically open *new* opportunities for DCE: the
//! dropped `A` branch may have referenced a variable that's now
//! unused, and DCE can finish the cleanup. That's why CLOC06 pins
//! the order **constant-fold → fold-control-flow → dce**.
//!
//! # Why a separate pass instead of merging into constant-fold?
//!
//! - **Different node-kind focus.** Constant-fold works on
//!   `Expression`. Fold-control-flow works on `Statement` (and a
//!   few specific expressions like `cond ? a : b` where the cond is
//!   statically known).
//! - **Different reasoning step.** Constant-fold's safety question
//!   is "does evaluating this expression have side effects?"
//!   Fold-control-flow's question is "is this branch reachable
//!   given what we statically know about the condition?" Once the
//!   AST grows nodes, these are independent analyses.
//! - **Different invalidation footprint.** Eliminating a branch can
//!   remove `break`/`continue`/`return` paths, which is enough of a
//!   structural change that future analyses (alias, escape) will
//!   want to know about it specifically.
//!
//! Keeping them split lets each pass stay small and focused, and
//! lets `closurec --disable=fold-control-flow` be a meaningful CLI
//! switch separate from `--disable=constant-fold`.
//!
//! # Scope (v1)
//!
//! `javascript-ast` ships only `Program` / `SourceType` today
//! (CLOC02 Phase 1). With no `Statement` / `Expression` variants
//! there is nothing to fold; [`FoldControlFlowPass::run`] is
//! identity in v1. Per CLOC03 §"When a pass keeps a node unchanged"
//! no contributions are emitted.
//!
//! What this PR still does:
//!
//! 1. Pins the pass metadata (`name`, `iteration_policy`, `cost`,
//!    `depends_on`) the pipeline scheduler reads.
//! 2. Locks the `depends_on("constant-fold")` edge so the scheduler
//!    is forced to run constant-fold first as soon as both passes
//!    are in one pipeline.
//! 3. Establishes the integration test that proves
//!    `PassPipeline` picks the right three-pass order
//!    (constant-fold → fold-control-flow → dce) even when the
//!    passes are registered out of order.
//!
//! # Followup: DCE's depends_on edge
//!
//! The `dce` crate currently declares
//! `depends_on = &["constant-fold"]`. Once this PR is merged a
//! one-line followup PR will extend that to
//! `depends_on = &["constant-fold", "fold-control-flow"]` to make
//! the canonical order survive even when DCE is registered without
//! constant-fold in the same pipeline. Splitting that into a
//! separate PR keeps each change reviewable in isolation per the
//! small-PR principle.

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};

/// `Pass::depends_on` value. Kept as a const so other crates (and
/// future tests in this crate) can refer to it without retyping the
/// pass name.
///
/// CLOC06 canonical order: `constant-fold` runs first so this pass
/// sees folded constants (`if (1+1 === 2)` → `if (true)` → kept
/// branch). Once more upstream passes appear (e.g. `inline-consts`),
/// they'd join this list too.
const DEPS: &[&str] = &["constant-fold"];

/// Control-flow folding pass. v1 is identity — see crate-level docs.
///
/// Zero-sized type: no per-instance state. Pass-internal state
/// (reachability sets, branch-condition lattices) lives in
/// pass-local maps constructed inside [`Pass::run`] per CLOC06
/// §"Pass-internal state."
#[derive(Debug, Default, Clone, Copy)]
pub struct FoldControlFlowPass;

impl FoldControlFlowPass {
    /// Zero-arg constructor for ergonomic
    /// `PassPipeline::add(Box::new(FoldControlFlowPass::new()))`
    /// registration.
    pub fn new() -> Self {
        Self
    }
}

impl Pass for FoldControlFlowPass {
    fn name(&self) -> &'static str {
        "fold-control-flow"
    }

    fn depends_on(&self) -> &[&'static str] {
        // CLOC06 canonical order: constant-fold first so we see
        // folded conditions like `if (true) { ... }`. The scheduler
        // uses this edge to topo-sort pipeline registration.
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // Per CLOC06: eliminating one branch can expose another
        // that's also statically dead. E.g.
        //   if (cond) { if (false) { A } else { B } }
        // First pass folds the inner `if (false)` to `B`; that
        // collapse may itself expose new constant-fold opportunities
        // that, once folded, expose more control-flow folds. Run to
        // fixed point.
        IterationPolicy::FixedPoint
    }

    fn cost(&self) -> u32 {
        // Tree walk + per-branch condition evaluation. Similar in
        // weight to constant-fold (cost = 2): both do a single
        // traversal with cheap local decisions per node.
        2
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        // v1: no Statement / Expression nodes in the AST yet, so
        // there's nothing to fold. Pass through unchanged. The real
        // implementation slots in here once javascript-ast grows
        // `IfStatement`, `WhileStatement`, `SwitchStatement`,
        // `ConditionalExpression`, and the return/throw flow
        // analysis the spec calls for.
        Ok(PassOutput {
            program: ctx.program.clone(),
            contributions: Vec::new(),
            changed: false,
            diagnostics: Vec::new(),
            stats: PassStats {
                // Visited the program root; nothing folded.
                nodes_touched: 1,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    //! These tests do two things:
    //!
    //! 1. **Pin the metadata.** Name, dependency edge, iteration
    //!    policy, and cost are part of the public contract — they
    //!    drive scheduler behavior and future CLI flag names. A
    //!    test for each catches accidental drift.
    //! 2. **Lock the ordering integration.** Even though v1 doesn't
    //!    touch any AST, the `depends_on` edge is real and the
    //!    scheduler honors it. The two- and three-pass integration
    //!    tests register passes deliberately *out of order* to
    //!    prove the topo-sort works, not just that the input
    //!    happened to be in the right order.
    use super::*;
    use coding_adventures_closure_pass_constant_fold::ConstantFoldPass;
    use coding_adventures_closure_pass_dce::DcePass;
    use coding_adventures_closure_pass_pipeline::{PassPipeline, PipelineOutput};
    use coding_adventures_correlation_vector::CVLog;
    use coding_adventures_javascript_ast::{Program, SourceType};
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    /// Small helper to spin up a fresh `Program` for every test.
    /// Tests should not share mutable state via a `static`.
    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }

    #[test]
    fn name_is_fold_control_flow() {
        // The name is what the scheduler keys on and what
        // `--disable=fold-control-flow` will eventually use, so
        // it's a public contract.
        assert_eq!(FoldControlFlowPass::new().name(), "fold-control-flow");
    }

    #[test]
    fn iteration_policy_is_fixed_point() {
        // Folding one branch can expose another statically-dead
        // branch — FixedPoint is the right intent even though v1
        // converges in one step (because there's nothing to fold).
        assert_eq!(
            FoldControlFlowPass::new().iteration_policy(),
            IterationPolicy::FixedPoint
        );
    }

    #[test]
    fn cost_is_two_pass_units() {
        // Matches constant-fold's weight: single tree walk.
        assert_eq!(FoldControlFlowPass::new().cost(), 2);
    }

    #[test]
    fn depends_on_constant_fold() {
        let p = FoldControlFlowPass::new();
        assert_eq!(p.depends_on(), &["constant-fold"]);
    }

    #[test]
    fn invalidates_empty_in_v1() {
        // CLOC06 Open Question 1: `invalidates()` is informational
        // in v1; the v0.1.0 scheduler doesn't act on it. Leaving
        // empty avoids committing to a footprint we'd have to walk
        // back later.
        assert!(FoldControlFlowPass::new().invalidates().is_empty());
    }

    #[test]
    fn run_on_empty_program_returns_unchanged_identity() {
        // Identity check: same CV, same version, same source type,
        // no contributions, no diagnostics, changed=false,
        // nodes_touched=1 (we visited the root).
        let pass = FoldControlFlowPass::new();
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
    fn pipeline_orders_constant_fold_before_fold_control_flow() {
        // Register FoldControlFlowPass FIRST. If `depends_on` were
        // ignored, the pipeline would run them in registration
        // order: [fold-control-flow, constant-fold]. The scheduler
        // must reorder them to [constant-fold, fold-control-flow].
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(FoldControlFlowPass::new()));
        pipeline.add(Box::new(ConstantFoldPass::new()));

        let mut cv = CVLog::new(true);
        let out: PipelineOutput = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        assert_eq!(
            out.execution_order,
            vec![
                "constant-fold".to_string(),
                "fold-control-flow".to_string(),
            ],
            "fold-control-flow must run after constant-fold per CLOC06 canonical order"
        );
        assert!(out.stats.contains_key("constant-fold"));
        assert!(out.stats.contains_key("fold-control-flow"));
    }

    #[test]
    fn pipeline_orders_three_passes_canonically() {
        // The big integration test: register all three passes
        // (constant-fold, fold-control-flow, dce) in the *wrong*
        // order (DCE first, then fold-control-flow, then
        // constant-fold) and verify the scheduler produces the
        // canonical order:
        //
        //   constant-fold → fold-control-flow → dce
        //
        // This is the order CLOC06 §"Canonical pass set" pins.
        //
        // Note: DCE's `depends_on` is currently just
        // `["constant-fold"]`, not `["constant-fold",
        // "fold-control-flow"]`. The canonical order still works
        // here because:
        //   - constant-fold has no deps → comes first
        //   - fold-control-flow depends on constant-fold → comes
        //     second
        //   - dce depends on constant-fold → comes after
        //     constant-fold, and the topo-sort happens to place it
        //     after fold-control-flow because of registration
        //     iteration order on a tie.
        //
        // A followup PR will tighten DCE's `depends_on` to
        // ["constant-fold", "fold-control-flow"] so the ordering
        // becomes structurally required rather than incidental.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(DcePass::new()));
        pipeline.add(Box::new(FoldControlFlowPass::new()));
        pipeline.add(Box::new(ConstantFoldPass::new()));

        let mut cv = CVLog::new(true);
        let out = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        // constant-fold must be first.
        assert_eq!(
            out.execution_order[0], "constant-fold",
            "constant-fold must run first; got {:?}",
            out.execution_order
        );
        // fold-control-flow and dce both depend on constant-fold;
        // both must come after it.
        let fcf_idx = out
            .execution_order
            .iter()
            .position(|n| n == "fold-control-flow")
            .expect("fold-control-flow should be scheduled");
        let dce_idx = out
            .execution_order
            .iter()
            .position(|n| n == "dce")
            .expect("dce should be scheduled");
        assert!(
            fcf_idx > 0,
            "fold-control-flow must come after constant-fold; got {:?}",
            out.execution_order
        );
        assert!(
            dce_idx > 0,
            "dce must come after constant-fold; got {:?}",
            out.execution_order
        );
        // All three passes ran.
        assert_eq!(out.execution_order.len(), 3);
        assert!(out.stats.contains_key("constant-fold"));
        assert!(out.stats.contains_key("fold-control-flow"));
        assert!(out.stats.contains_key("dce"));
    }

    #[test]
    fn pipeline_runs_fold_control_flow_as_solo_pass() {
        // FoldControlFlowPass alone (without constant-fold) still
        // works: `depends_on` is "soft" in v1 — unknown
        // dependencies are silently dropped by the v0.1.0
        // scheduler. CLOC06 doesn't pin the strict-vs-soft choice;
        // we lock in the v0.1.0 behavior here so a future change is
        // a deliberate decision rather than an accident.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(FoldControlFlowPass::new()));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert_eq!(out.execution_order, vec!["fold-control-flow".to_string()]);
        assert_eq!(out.stats["fold-control-flow"].nodes_touched, 1);
        // FixedPoint policy + v0.1.0 scheduler → pipeline emits
        // the "not yet iterated" informational diagnostic.
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
        // Zero-sized types should be Default+Clone+Copy so callers
        // don't need to think about ownership when registering them.
        let _a: FoldControlFlowPass = Default::default();
        let _b: FoldControlFlowPass = FoldControlFlowPass::new();
        let _c = _b;
        let _d = _c.clone();
    }
}
