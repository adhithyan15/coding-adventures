//! Constant-folding pass for the Closure Compiler clone.
//!
//! First concrete optimization pass plugged into the
//! [`coding_adventures_closure_pass_pipeline::Pass`] trait per
//! [CLOC06's canonical pass set](../../../specs/CLOC06-pass-interface-contract.md).
//!
//! # Scope (v1)
//!
//! Once we have an expression AST, this pass folds compile-time-
//! evaluable expressions: `2 + 3 → 5`, `"foo" + "bar" → "foobar"`,
//! `true && x → x`, `typeof "s" → "string"`. It's `IterationPolicy::FixedPoint`
//! so it runs until no further folds are possible (the v1 pipeline
//! caps the iteration count at 1 with a diagnostic — see
//! `closure-pass-pipeline` v0.1.0).
//!
//! **v1 is identity.** `javascript-ast` ships only `Program` /
//! `SourceType` today (per CLOC02 Phase 1); there are no expressions
//! to fold, so [`ConstantFoldPass::run`] clones the input `Program`
//! unchanged and returns `changed = false`. Per CLOC03 §"When a pass
//! keeps a node unchanged," no [`Contribution`]s get appended — the
//! pass is silent until it actually changes something.
//!
//! Even so, this PR does real work:
//!
//! 1. Establishes the crate layout for future `closure-pass-*` crates
//!    to mirror (deps, BUILD layout, README/CHANGELOG shape).
//! 2. Pins the pass metadata — name, iteration policy, cost — that the
//!    scheduler reads.
//! 3. Wires up CLOC03 plumbing so once the pass actually folds, the
//!    contribution-emission path is already in place.
//!
//! [`Contribution`]: coding_adventures_correlation_vector::Contribution

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};

/// Constant-folding pass. v1 is identity — see crate-level docs.
///
/// Zero-sized type: no per-instance state. Pass-internal state
/// (reaching-defs, escape analysis, etc.) lives in pass-local maps
/// constructed inside [`Pass::run`] per CLOC06 §"Pass-internal state."
#[derive(Debug, Default, Clone, Copy)]
pub struct ConstantFoldPass;

impl ConstantFoldPass {
    /// Convenient zero-arg constructor — matches the
    /// `PassPipeline::add(Box::new(ConstantFoldPass::new()))`
    /// registration idiom.
    pub fn new() -> Self {
        Self
    }
}

impl Pass for ConstantFoldPass {
    fn name(&self) -> &'static str {
        "constant-fold"
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // Folds expose further folds — `2 + 3 + 4` becomes `5 + 4`
        // becomes `9` over two iterations. FixedPoint signals that
        // intent; v1 still only runs once because the AST has no
        // foldable nodes yet.
        IterationPolicy::FixedPoint
    }

    fn cost(&self) -> u32 {
        // Tree walk + small constant work per visit. ~2 pass-units.
        2
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        // v1 has nothing to fold. Real folding lands once
        // javascript-ast grows `Statement` / `Expression` variants.
        Ok(PassOutput {
            program: ctx.program.clone(),
            contributions: Vec::new(),
            changed: false,
            diagnostics: Vec::new(),
            stats: PassStats {
                // We "touched" the program root in the sense that we
                // visited it — nothing else exists yet.
                nodes_touched: 1,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn name_is_constant_fold() {
        assert_eq!(ConstantFoldPass::new().name(), "constant-fold");
    }

    #[test]
    fn iteration_policy_is_fixed_point() {
        assert_eq!(
            ConstantFoldPass::new().iteration_policy(),
            IterationPolicy::FixedPoint
        );
    }

    #[test]
    fn cost_is_two_pass_units() {
        assert_eq!(ConstantFoldPass::new().cost(), 2);
    }

    #[test]
    fn no_depends_on_or_invalidates_in_v1() {
        let p = ConstantFoldPass::new();
        assert!(p.depends_on().is_empty());
        assert!(p.invalidates().is_empty());
    }

    #[test]
    fn run_on_empty_program_returns_unchanged_identity() {
        let pass = ConstantFoldPass::new();
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
        assert!(
            out.contributions.is_empty(),
            "v1 emits no contributions per CLOC03 §\"unchanged nodes\""
        );
        assert!(out.diagnostics.is_empty());
        assert_eq!(out.stats.nodes_touched, 1);
    }

    #[test]
    fn integrates_with_pass_pipeline_as_solo_pass() {
        // Smoke-test the full PassPipeline registration path: build a
        // pipeline, add the pass, run end-to-end.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(ConstantFoldPass::new()));

        let mut cv = CVLog::new(true);
        let out: PipelineOutput = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        assert_eq!(out.execution_order, vec!["constant-fold".to_string()]);
        assert!(out.stats.contains_key("constant-fold"));
        assert_eq!(out.stats["constant-fold"].nodes_touched, 1);

        // v1 FixedPoint policy => pipeline emits the
        // "not yet iterated" note diagnostic.
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.group.0 == "pipeline.fixed-point-not-yet-iterated"),
            "expected the pipeline's FixedPoint note diagnostic; got {:?}",
            out.diagnostics
        );
    }

    #[test]
    fn integrates_with_pass_pipeline_alongside_other_passes() {
        // ConstantFoldPass should play nicely with a tagging upstream
        // pass that has no depends_on relation — they should run in
        // registration order.
        struct UpstreamTag;
        impl Pass for UpstreamTag {
            fn name(&self) -> &'static str {
                "upstream-tag"
            }
            fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
                Ok(PassOutput {
                    program: ctx.program.clone(),
                    contributions: Vec::new(),
                    changed: false,
                    diagnostics: Vec::new(),
                    stats: PassStats { nodes_touched: 0 },
                })
            }
        }

        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(UpstreamTag));
        pipeline.add(Box::new(ConstantFoldPass::new()));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();

        assert_eq!(
            out.execution_order,
            vec!["upstream-tag".to_string(), "constant-fold".to_string()]
        );
    }

    #[test]
    fn pass_is_default_and_clone() {
        // Bookkeeping: the zero-sized type derives Default + Clone so
        // callers can construct it in ergonomic ways.
        let _a: ConstantFoldPass = Default::default();
        let _b: ConstantFoldPass = ConstantFoldPass::new();
        let _c = _b;
        let _d = _c.clone();
    }
}
