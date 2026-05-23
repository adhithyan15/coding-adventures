//! Unreferenced-variable cleanup pass for the Closure Compiler
//! clone.
//!
//! Per [CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
//! canonical pass set. The final sweep before emission: deletes
//! variable bindings whose initializer is pure and whose
//! references-count, after every earlier pass has run, is zero.
//!
//! ```js
//! // After dce + inline + treeshake have done their work,
//! // the program might still contain:
//!
//! const TMP = pure_compute();   // 0 references — bindings only used by
//! const X = 1;                  // 0 references — code DCE just deleted
//!
//! // remove-unused-vars deletes both lines.
//! ```
//!
//! # Why a separate pass instead of folding into DCE?
//!
//! DCE catches *unreachable statements* — code paths nothing
//! flows through. It doesn't necessarily catch every
//! *unreferenced binding* — a `const X = pure_compute()` with
//! no references is technically reachable (the statement
//! "executes," even though its result is unused) but useless if
//! `pure_compute()` is pure.
//!
//! Splitting it out has three benefits:
//!
//! 1. **Different safety question.** DCE asks "is this code
//!    path reached?" This pass asks "is this binding's
//!    initializer pure?" The second question needs the sidecar's
//!    `pure` / `no_side_effects` attributes; the first doesn't.
//! 2. **Different ordering.** This pass must run *after* DCE
//!    and inline — both of those can leave behind newly-orphaned
//!    bindings that this pass catches. CLOC06 pins
//!    `depends_on = ["dce", "inline"]` accordingly.
//! 3. **Different CLI knob.** Users may want to keep some
//!    unreferenced bindings around for side effects they trust
//!    but haven't annotated. `--disable=remove-unused-vars`
//!    is a finer-grained knob than disabling all of DCE.
//!
//! Closure Compiler itself ships a `removeUnusedVars` pass for
//! exactly this reason.
//!
//! # Why `FixedPoint`?
//!
//! Removing one binding can unreference another:
//!
//! ```js
//! const A = pure_helper();      // referenced only by B below
//! const B = A + 1;              // 0 external references → removed
//! // first iteration removes B
//! // second iteration sees A is now also unreferenced → removes A
//! ```
//!
//! Bounded in practice by the length of the unused-binding
//! chain; we don't pre-cap.
//!
//! # Why depends_on = ["dce", "inline"]?
//!
//! Both predecessors shrink the reference graph:
//!
//! - **DCE** removes unreachable statements that might have
//!   referenced bindings, leaving those bindings unreferenced.
//! - **Inline** replaces `f(x)` calls with the substituted body
//!   of `f`. After inlining, the original function declaration
//!   may have zero remaining call sites, and this pass cleans
//!   up.
//!
//! Both are *correctness preconditions* for catching the right
//! set of unused bindings, not just preferences. Running this
//! pass without DCE or inline would still be correct (it'd
//! just catch fewer bindings), but the spec pins both edges to
//! let the scheduler force canonical ordering.
//!
//! # Scope (v1)
//!
//! `javascript-ast` ships only `Program` / `SourceType` today
//! (CLOC02 Phase 1). With no `VariableDeclaration` /
//! `Identifier` nodes there is nothing to remove;
//! [`RemoveUnusedVarsPass::run`] is identity in v1. Per CLOC03
//! §"When a pass keeps a node unchanged" no contributions are
//! emitted.
//!
//! What this PR locks down:
//!
//! 1. Pass metadata (`name`, `iteration_policy`, `cost`,
//!    `depends_on`) — what the scheduler keys on and what the
//!    future `closurec` CLI surfaces as
//!    `--disable=remove-unused-vars`.
//! 2. The `depends_on(["dce", "inline"])` edges so the scheduler
//!    forces DCE-before, inline-before-this as soon as all
//!    three are in one pipeline.
//! 3. Two- and three-pass integration tests that prove the
//!    ordering.

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};

/// `Pass::depends_on` value — both DCE and inline must run first
/// per CLOC06 canonical order. Kept as a `const` so future tests
/// and sibling crates can reference these names without retyping.
const DEPS: &[&str] = &["dce", "inline"];

/// Unreferenced-variable cleanup pass. v1 is identity — see
/// crate-level docs.
///
/// Zero-sized type: no per-instance state. Pass-internal state
/// (the per-scope binding → reference-count map, the
/// pure-initializer set seeded from sidecar attributes, the
/// "do-not-remove" set seeded from `export`s) lives in pass-local
/// maps constructed inside [`Pass::run`] per CLOC06
/// §"Pass-internal state."
#[derive(Debug, Default, Clone, Copy)]
pub struct RemoveUnusedVarsPass;

impl RemoveUnusedVarsPass {
    /// Zero-arg constructor for ergonomic
    /// `PassPipeline::add(Box::new(RemoveUnusedVarsPass::new()))`
    /// registration.
    pub fn new() -> Self {
        Self
    }
}

impl Pass for RemoveUnusedVarsPass {
    fn name(&self) -> &'static str {
        "remove-unused-vars"
    }

    fn depends_on(&self) -> &[&'static str] {
        // CLOC06 canonical order: DCE prunes unreachable code
        // that might have referenced bindings; inline replaces
        // call sites and can leave function declarations
        // unreferenced. Both must run before this final cleanup
        // pass to catch the maximum set of orphans.
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // Per CLOC06: removing one binding can unreference
        // another that was only used by the just-removed
        // binding's initializer. Cascade is bounded by the
        // length of the unused-binding chain.
        IterationPolicy::FixedPoint
    }

    fn cost(&self) -> u32 {
        // Per scope:
        //   1. Walk the scope, building a binding → uses table
        //      (skipping bindings that the sidecar marks
        //      side-effecting and bindings that are exports).
        //   2. Delete the entries with use-count zero and pure
        //      initializers.
        // Same shape as DCE; identical cost.
        3
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        // v1: no VariableDeclaration / Identifier nodes in the
        // AST yet, so there's nothing to remove. Pass through
        // unchanged. The real per-scope walk + deletion slots
        // in here once javascript-ast grows the variants.
        Ok(PassOutput {
            program: ctx.program.clone(),
            contributions: Vec::new(),
            changed: false,
            diagnostics: Vec::new(),
            stats: PassStats {
                // Visited the program root; nothing removed.
                nodes_touched: 1,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    //! Tests pin the public contract and lock the two- and
    //! three-pass ordering integrations with `PassPipeline`.
    //! v1's `run` is identity, but the metadata drives the
    //! scheduler and outlives the v1 body.
    use super::*;
    use coding_adventures_closure_pass_dce::DcePass;
    use coding_adventures_closure_pass_inline::InlinePass;
    use coding_adventures_closure_pass_pipeline::{PassPipeline, PipelineOutput};
    use coding_adventures_correlation_vector::CVLog;
    use coding_adventures_javascript_ast::{Program, SourceType};
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }

    #[test]
    fn name_is_remove_unused_vars() {
        // `--disable=remove-unused-vars` and
        // `out.stats["remove-unused-vars"]` key on this.
        // Public contract.
        assert_eq!(
            RemoveUnusedVarsPass::new().name(),
            "remove-unused-vars"
        );
    }

    #[test]
    fn iteration_policy_is_fixed_point() {
        // Remove one binding → unreference another → remove
        // that one too. FixedPoint captures intent.
        assert_eq!(
            RemoveUnusedVarsPass::new().iteration_policy(),
            IterationPolicy::FixedPoint
        );
    }

    #[test]
    fn cost_is_three_pass_units() {
        // Per-scope binding-table build + delete. Same shape
        // as DCE.
        assert_eq!(RemoveUnusedVarsPass::new().cost(), 3);
    }

    #[test]
    fn depends_on_dce_and_inline() {
        let p = RemoveUnusedVarsPass::new();
        assert_eq!(p.depends_on(), &["dce", "inline"]);
    }

    #[test]
    fn invalidates_empty_in_v1() {
        // CLOC06 Open Question 1: informational only in v0.1.0.
        assert!(RemoveUnusedVarsPass::new().invalidates().is_empty());
    }

    #[test]
    fn run_on_empty_program_returns_unchanged_identity() {
        // Identity: same CV, version, source_type; no
        // contributions, no diagnostics, changed=false,
        // nodes_touched=1.
        let pass = RemoveUnusedVarsPass::new();
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
    fn pipeline_orders_dce_before_remove_unused_vars() {
        // Register RemoveUnusedVarsPass FIRST. If `depends_on`
        // were ignored, the pipeline would run them in
        // registration order: [remove-unused-vars, dce]. The
        // scheduler must reorder to [dce, remove-unused-vars]
        // per CLOC06 canonical order.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(RemoveUnusedVarsPass::new()));
        pipeline.add(Box::new(DcePass::new()));

        let mut cv = CVLog::new(true);
        let out: PipelineOutput = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        assert_eq!(
            out.execution_order,
            vec!["dce".to_string(), "remove-unused-vars".to_string()],
            "remove-unused-vars must run after dce per CLOC06 canonical order"
        );
        assert!(out.stats.contains_key("dce"));
        assert!(out.stats.contains_key("remove-unused-vars"));
    }

    #[test]
    fn pipeline_orders_three_passes_canonically() {
        // The big integration test: register all three passes
        // (DCE, inline, remove-unused-vars) in the *wrong*
        // order and verify the scheduler produces the canonical
        // order — both DCE and inline before
        // remove-unused-vars.
        //
        // Note: inline depends on constant-fold, which is NOT
        // in this pipeline. The v0.1.0 scheduler treats unknown
        // dependencies as soft, so it silently drops them. So:
        //   - dce has no deps in *this* pipeline (we don't
        //     register constant-fold) → it can run first.
        //   - inline's only listed dep is constant-fold, which
        //     is unknown → it has no effective dep → can run
        //     any time.
        //   - remove-unused-vars depends on both dce and inline
        //     → must run last.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(RemoveUnusedVarsPass::new()));
        pipeline.add(Box::new(InlinePass::new()));
        pipeline.add(Box::new(DcePass::new()));

        let mut cv = CVLog::new(true);
        let out = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        // remove-unused-vars must come last — it depends on
        // both dce and inline.
        let last = out
            .execution_order
            .last()
            .expect("pipeline must have run at least one pass");
        assert_eq!(
            last, "remove-unused-vars",
            "remove-unused-vars must run last; got {:?}",
            out.execution_order
        );
        // dce and inline both must come before
        // remove-unused-vars (their order relative to each
        // other is unspecified — neither depends on the other).
        let dce_idx = out
            .execution_order
            .iter()
            .position(|n| n == "dce")
            .expect("dce should be scheduled");
        let inline_idx = out
            .execution_order
            .iter()
            .position(|n| n == "inline")
            .expect("inline should be scheduled");
        let ruv_idx = out
            .execution_order
            .iter()
            .position(|n| n == "remove-unused-vars")
            .expect("remove-unused-vars should be scheduled");
        assert!(
            dce_idx < ruv_idx,
            "dce must precede remove-unused-vars; got {:?}",
            out.execution_order
        );
        assert!(
            inline_idx < ruv_idx,
            "inline must precede remove-unused-vars; got {:?}",
            out.execution_order
        );
        assert_eq!(out.execution_order.len(), 3);
        assert!(out.stats.contains_key("dce"));
        assert!(out.stats.contains_key("inline"));
        assert!(out.stats.contains_key("remove-unused-vars"));
    }

    #[test]
    fn pipeline_runs_remove_unused_vars_as_solo_pass() {
        // RemoveUnusedVarsPass alone (without dce or inline)
        // still works: depends_on is "soft" in v1 — the v0.1.0
        // scheduler silently drops unknown dependencies.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(RemoveUnusedVarsPass::new()));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert_eq!(
            out.execution_order,
            vec!["remove-unused-vars".to_string()]
        );
        assert_eq!(out.stats["remove-unused-vars"].nodes_touched, 1);
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
        let _a: RemoveUnusedVarsPass = Default::default();
        let _b: RemoveUnusedVarsPass = RemoveUnusedVarsPass::new();
        let _c = _b;
        let _d = _c.clone();
    }
}
