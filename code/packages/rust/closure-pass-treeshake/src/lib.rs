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
use coding_adventures_closure_scope_analyzer::{analyze, BindingId, BindingKind};

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

        // Step 1 + 2: shape-candidate scan.
        let mut shape_candidates: Vec<BindingId> = Vec::new();
        for (idx, binding) in analysis.bindings.iter().enumerate() {
            let id = BindingId(idx as u32);
            // `#[non_exhaustive]` on BindingKind: future variants
            // conservatively passthrough via wildcard.
            match binding.kind {
                BindingKind::Function | BindingKind::Class => {
                    shape_candidates.push(id);
                }
                // Var/Let/Const/Param: these go through DCE +
                // remove-unused-vars + collapse-properties. Splitting
                // by kind keeps the passes from arguing over the
                // same binding.
                _ => {}
            }
        }

        // Step 3 deferred — keep `changed = false`.
        let _shape_candidates = shape_candidates;

        Ok(PassOutput {
            program: ctx.program.clone(),
            contributions: Vec::new(),
            changed: false,
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
}
