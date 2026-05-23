//! Dead-code elimination pass for the Closure Compiler clone.
//!
//! Second concrete optimization pass after constant-fold. Per
//! [CLOC06](../../../specs/CLOC06-pass-interface-contract.md), DCE
//! walks the program from entry / exported declarations, marks
//! reachable nodes, and deletes unmarked ones. Deletion can free
//! further nodes (an unreferenced variable whose initializer was
//! pure becomes deletable once the variable goes), so DCE is
//! `IterationPolicy::FixedPoint`.
//!
//! DCE depends on `constant-fold` (and, once it exists,
//! `fold-control-flow`): folds turn `if (false) { A } else { B }`
//! into `B`, exposing the `A` branch as unreachable. Running fold
//! first lets DCE find more dead code.
//!
//! # Scope (v1)
//!
//! `javascript-ast` ships only `Program` / `SourceType` today
//! (CLOC02 Phase 1). With no `Statement` / `Expression` variants
//! there is nothing to mark or delete; [`DcePass::run`] is identity
//! in v1. Per CLOC03 §"When a pass keeps a node unchanged," no
//! contributions get emitted. Once the AST grows expression and
//! declaration nodes, the reachability walk + deletion logic slots
//! into [`Pass::run`] here without changing the public surface.
//!
//! This PR still does real work:
//!
//! 1. Pins the pass metadata the scheduler reads — name, dependencies,
//!    policy, cost.
//! 2. Locks the `depends_on("constant-fold")` edge so the scheduler
//!    forces fold-before-DCE ordering as soon as both passes are in
//!    one pipeline.
//! 3. Establishes the integration test that proves
//!    `PassPipeline` picks the right order (constant-fold → dce).

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};

/// `Pass::depends_on` value — kept as a const so other crates (and
/// future tests in this crate) can refer to it without retyping the
/// pass name.
const DEPS: &[&str] = &["constant-fold"];

/// Dead-code elimination pass. v1 is identity — see crate-level docs.
///
/// Zero-sized type: no per-instance state. Pass-internal state
/// (reachability sets, escape analysis) lives in pass-local maps
/// constructed inside [`Pass::run`] per CLOC06 §"Pass-internal state."
#[derive(Debug, Default, Clone, Copy)]
pub struct DcePass;

impl DcePass {
    /// Zero-arg constructor for ergonomic
    /// `PassPipeline::add(Box::new(DcePass::new()))` registration.
    pub fn new() -> Self {
        Self
    }
}

impl Pass for DcePass {
    fn name(&self) -> &'static str {
        "dce"
    }

    fn depends_on(&self) -> &[&'static str] {
        // CLOC06 canonical order: constant-fold first so DCE sees
        // folded constants and can spot unreachable arms. Once
        // fold-control-flow exists it joins this list too.
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // Per CLOC06: "deletion can free further nodes." E.g.,
        // deleting a function declaration may make its captured
        // variables unreferenced too.
        IterationPolicy::FixedPoint
    }

    fn cost(&self) -> u32 {
        // Tree walk + reachability marking + post-walk deletion.
        // Slightly more than constant-fold's pure walk.
        3
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        // v1: no Expression / Declaration nodes in the AST yet, so
        // there's nothing to mark or delete. Pass through unchanged.
        // Real reachability walk + deletion lands once
        // javascript-ast grows the variants this pass needs.
        Ok(PassOutput {
            program: ctx.program.clone(),
            contributions: Vec::new(),
            changed: false,
            diagnostics: Vec::new(),
            stats: PassStats {
                // Visited the program root; no nodes deleted.
                nodes_touched: 1,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_closure_pass_constant_fold::ConstantFoldPass;
    use coding_adventures_closure_pass_pipeline::{PassPipeline, PipelineOutput};
    use coding_adventures_correlation_vector::CVLog;
    use coding_adventures_javascript_ast::{Program, SourceType};
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new(
            "prog.1".to_string(),
            EsVersion::Es2025,
            SourceType::Module,
        )
    }

    #[test]
    fn name_is_dce() {
        assert_eq!(DcePass::new().name(), "dce");
    }

    #[test]
    fn iteration_policy_is_fixed_point() {
        // Deletion can free further nodes — FixedPoint signals that
        // intent even though v1 only iterates once via the v0.1.0
        // pipeline.
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
        let p = DcePass::new();
        assert_eq!(p.depends_on(), &["constant-fold"]);
    }

    #[test]
    fn invalidates_empty_in_v1() {
        assert!(DcePass::new().invalidates().is_empty());
    }

    #[test]
    fn run_on_empty_program_returns_unchanged_identity() {
        let pass = DcePass::new();
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
    fn pipeline_orders_constant_fold_before_dce() {
        // The canonical order from CLOC06 §"Canonical pass set":
        // constant-fold → fold-control-flow → dce → ...
        //
        // Register DCE FIRST to verify that depends_on actually
        // forces reordering — without it, registration order would
        // give us [dce, constant-fold].
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(DcePass::new()));
        pipeline.add(Box::new(ConstantFoldPass::new()));

        let mut cv = CVLog::new(true);
        let out: PipelineOutput = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        assert_eq!(
            out.execution_order,
            vec!["constant-fold".to_string(), "dce".to_string()],
            "DCE must run after constant-fold per CLOC06 canonical order"
        );
        assert!(out.stats.contains_key("dce"));
        assert!(out.stats.contains_key("constant-fold"));
    }

    #[test]
    fn pipeline_runs_dce_as_solo_pass() {
        // DCE alone (without constant-fold) still works: depends_on
        // is "soft" in v1 — unknown dependencies are silently dropped
        // by the v0.1.0 scheduler (CLOC06 doesn't pin the behavior).
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(DcePass::new()));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert_eq!(out.execution_order, vec!["dce".to_string()]);
        assert_eq!(out.stats["dce"].nodes_touched, 1);
        // FixedPoint policy → pipeline emits the v0.1.0 "not yet
        // iterated" note.
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
        let _a: DcePass = Default::default();
        let _b: DcePass = DcePass::new();
        let _c = _b;
        let _d = _c.clone();
    }
}
