//! Property-collapse pass for the Closure Compiler clone.
//!
//! Per [CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
//! canonical pass set. Collapses repeated nested property-access
//! chains on stable namespace-style objects into shorter local
//! bindings:
//!
//! ```js
//! // before
//! ns.utils.format.currency(1);
//! ns.utils.format.percent(0.5);
//! ns.utils.format.date(now);
//!
//! // after collapse-properties
//! const $f = ns.utils.format;     // bound once
//! $f.currency(1);
//! $f.percent(0.5);
//! $f.date(now);
//! ```
//!
//! Two wins:
//!
//! 1. **Smaller output.** `ns.utils.format` is repeated thrice
//!    in the original; once after collapse.
//! 2. **Faster runtime.** Each `.foo.bar.baz` is a chain of
//!    property lookups. Caching the intermediate object in a
//!    local skips the lookups on the hot path.
//!
//! # Why "stable" matters
//!
//! Collapsing is only safe when the intermediate object is
//! genuinely stable — i.e., neither the property chain nor any
//! visible function can mutate it between accesses:
//!
//! ```js
//! // UNSAFE: mutate() could replace ns.utils.format mid-chain
//! ns.utils.format.currency(1);
//! mutate();
//! ns.utils.format.percent(0.5);    // might be a different object now
//! ```
//!
//! The pass reads the type sidecar's `stable` / `pure` / `frozen`
//! attributes plus a local mutation analysis. Without that
//! evidence it bails. (CLOC04 left the exact attribute name
//! open; the implementation will pick once the sidecar grows
//! the namespace-stability marker.)
//!
//! # Why depends_on = ["constant-fold"]?
//!
//! Folded constant expressions can resolve into recognisable
//! property-access shapes. Without fold, the pass would miss
//! collapsing in expressions like:
//!
//! ```js
//! const KEY = "utils";              // fold can prove this
//! ns[KEY].format.x(...)             // → ns.utils.format.x(...)
//! ns[KEY].format.y(...)             // collapse can now see the
//! ns[KEY].format.z(...)             //   shared `ns.utils.format`
//! ```
//!
//! # Why `FixedPoint`?
//!
//! Collapsing one chain can rewrite call sites in ways that
//! expose *new* chains:
//!
//! ```js
//! // before
//! lib.feature.config.theme.color;
//! lib.feature.config.theme.font;
//!
//! // first iteration collapses lib.feature.config:
//! const $c = lib.feature.config;
//! $c.theme.color;
//! $c.theme.font;
//!
//! // second iteration spots the new shared `$c.theme`:
//! const $c = lib.feature.config;
//! const $t = $c.theme;
//! $t.color;
//! $t.font;
//! ```
//!
//! Bounded in practice by the depth of the chains; we don't
//! pre-cap.
//!
//! # Scope (v1)
//!
//! `javascript-ast` ships only `Program` / `SourceType` today
//! (CLOC02 Phase 1). With no `MemberExpression` / `Identifier` /
//! `VariableDeclaration` nodes there is nothing to collapse;
//! [`CollapsePropertiesPass::run`] is identity in v1. Per CLOC03
//! §"When a pass keeps a node unchanged" no contributions are
//! emitted.
//!
//! What this PR locks down:
//!
//! 1. Pass metadata (`name`, `iteration_policy`, `cost`,
//!    `depends_on`) — what the scheduler keys on and what the
//!    future `closurec` CLI surfaces as
//!    `--disable=collapse-properties`.
//! 2. The `depends_on("constant-fold")` edge so the scheduler
//!    forces fold-before-collapse as soon as both passes are
//!    in one pipeline.
//! 3. The two-pass integration test that proves the ordering.

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use coding_adventures_closure_scope_analyzer::{analyze, BindingId, BindingKind};

/// `Pass::depends_on` value. Kept as a `const` so future tests
/// and sibling crates can reference the dependency name without
/// retyping it.
const DEPS: &[&str] = &["constant-fold"];

/// Property-collapse pass. v1 is identity — see crate-level docs.
///
/// Zero-sized type: no per-instance state. Pass-internal state
/// (per-scope chain frequency tables, the "do-not-collapse" set
/// seeded from mutated namespaces) lives in pass-local maps
/// constructed inside [`Pass::run`] per CLOC06 §"Pass-internal
/// state."
#[derive(Debug, Default, Clone, Copy)]
pub struct CollapsePropertiesPass;

impl CollapsePropertiesPass {
    /// Zero-arg constructor for ergonomic
    /// `PassPipeline::add(Box::new(CollapsePropertiesPass::new()))`
    /// registration.
    pub fn new() -> Self {
        Self
    }
}

impl Pass for CollapsePropertiesPass {
    fn name(&self) -> &'static str {
        "collapse-properties"
    }

    fn depends_on(&self) -> &[&'static str] {
        // CLOC06 canonical order: constant-fold first so folded
        // constants resolve into recognisable property-access
        // shapes that collapse can spot.
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // Per CLOC06: collapsing one chain can expose new
        // shared prefixes in the now-rewritten call sites.
        // Bounded in practice by chain depth.
        IterationPolicy::FixedPoint
    }

    fn cost(&self) -> u32 {
        // Per scope:
        //   1. Walk the scope to gather property-chain frequencies.
        //   2. For each chain whose frequency × length passes the
        //      threshold, emit a binding + rewrite uses.
        // Same shape and weight as DCE's mark+sweep.
        3
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        // CLOC13.D wiring: consume the shared `ScopeAnalysis` from
        // `closure-scope-analyzer` to identify *alias candidates* —
        // bindings whose initializer is a stable member-expression
        // chain that can be safely flattened.
        //
        // The algorithm (steps 1 + 2 here; step 3 deferred):
        //
        //   1. Walk `analysis.bindings`. A binding is a *candidate*
        //      for collapsing when ALL of:
        //        a. kind == Const  (Var/Let could be reassigned;
        //           the alias would silently diverge from the
        //           target. Const guarantees the binding's value
        //           doesn't move.)
        //        b. The binding has at least one reference (we
        //           don't collapse a never-used alias — that's
        //           remove-unused-vars' job).
        //   2. Track candidates in a Vec<BindingId> for the
        //      observability path.
        //   3. Apply collapse — deferred to CLOC13.D.1 because
        //      cleanly rewriting member-access chains needs a
        //      binding → initializer-expression backreference
        //      that the analyzer doesn't yet ship.
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
        // so the candidate scan finds zero aliases, the candidates
        // vec is empty, `nodes_touched` is small, and the program
        // passes through unchanged. The wiring becomes *effective*
        // (real candidate-finding) the moment CLOC13.0 lands the
        // analyzer body — no churn here.

        let analysis = analyze(ctx.program);

        // Step 1: use-count per binding (for the "has at least one
        // reference" gate).
        let mut use_count: Vec<usize> = vec![0; analysis.bindings.len()];
        for reference in &analysis.references {
            if let Some(BindingId(idx)) = reference.binding {
                if let Some(slot) = use_count.get_mut(idx as usize) {
                    *slot += 1;
                }
            }
        }

        // Step 2: candidate scan.
        let mut alias_candidates: Vec<BindingId> = Vec::new();
        for (idx, binding) in analysis.bindings.iter().enumerate() {
            let id = BindingId(idx as u32);
            let uses = use_count.get(idx).copied().unwrap_or(0);
            if uses == 0 {
                continue; // not aliased anywhere — let remove-unused-vars handle it
            }
            // `#[non_exhaustive]` on BindingKind: future variants
            // conservatively passthrough via wildcard.
            match binding.kind {
                BindingKind::Const => alias_candidates.push(id),
                // Var/Let can be reassigned mid-program — collapsing
                // would create an alias that diverges from the
                // source. Function/Param/Class need additional
                // shape analysis the analyzer doesn't expose.
                _ => {}
            }
        }

        // Step 3 deferred — keep `changed = false`.
        let _alias_candidates = alias_candidates;

        Ok(PassOutput {
            program: ctx.program.clone(),
            contributions: Vec::new(),
            changed: false,
            diagnostics: Vec::new(),
            stats: PassStats {
                // Visited the program root + every binding + every
                // reference. Real cost numbers for the scheduler.
                nodes_touched: (1
                    + analysis.bindings.len()
                    + analysis.references.len()) as u32,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    //! Tests pin the public contract and lock the ordering
    //! integration with `PassPipeline`. v1's `run` is identity,
    //! but the metadata drives the scheduler and outlives the
    //! v1 body.
    use super::*;
    use coding_adventures_closure_pass_constant_fold::ConstantFoldPass;
    use coding_adventures_closure_pass_pipeline::{PassPipeline, PipelineOutput};
    use coding_adventures_correlation_vector::CVLog;
    use coding_adventures_javascript_ast::{Program, SourceType};
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }

    #[test]
    fn name_is_collapse_properties() {
        // `--disable=collapse-properties` and
        // `out.stats["collapse-properties"]` key on this.
        // Public contract.
        assert_eq!(
            CollapsePropertiesPass::new().name(),
            "collapse-properties"
        );
    }

    #[test]
    fn iteration_policy_is_fixed_point() {
        // Collapse one chain → expose new shared prefix →
        // collapse again. FixedPoint captures intent.
        assert_eq!(
            CollapsePropertiesPass::new().iteration_policy(),
            IterationPolicy::FixedPoint
        );
    }

    #[test]
    fn cost_is_three_pass_units() {
        // Gather + rewrite. Same shape as DCE.
        assert_eq!(CollapsePropertiesPass::new().cost(), 3);
    }

    #[test]
    fn depends_on_constant_fold() {
        let p = CollapsePropertiesPass::new();
        assert_eq!(p.depends_on(), &["constant-fold"]);
    }

    #[test]
    fn invalidates_empty_in_v1() {
        // CLOC06 Open Question 1: informational only in v0.1.0.
        assert!(CollapsePropertiesPass::new().invalidates().is_empty());
    }

    #[test]
    fn run_on_empty_program_returns_unchanged_identity() {
        // Identity: same CV, version, source_type; no
        // contributions, no diagnostics, changed=false,
        // nodes_touched=1.
        let pass = CollapsePropertiesPass::new();
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
    fn pipeline_orders_constant_fold_before_collapse_properties() {
        // Register CollapsePropertiesPass FIRST. If
        // `depends_on` were ignored, the pipeline would run them
        // in registration order: [collapse-properties,
        // constant-fold]. The scheduler must reorder them to
        // [constant-fold, collapse-properties] per CLOC06
        // canonical order.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(CollapsePropertiesPass::new()));
        pipeline.add(Box::new(ConstantFoldPass::new()));

        let mut cv = CVLog::new(true);
        let out: PipelineOutput = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        assert_eq!(
            out.execution_order,
            vec![
                "constant-fold".to_string(),
                "collapse-properties".to_string(),
            ],
            "collapse-properties must run after constant-fold per CLOC06 canonical order"
        );
        assert!(out.stats.contains_key("constant-fold"));
        assert!(out.stats.contains_key("collapse-properties"));
    }

    #[test]
    fn pipeline_runs_collapse_properties_as_solo_pass() {
        // CollapsePropertiesPass alone (without constant-fold)
        // still works: depends_on is "soft" in v1 — the v0.1.0
        // scheduler silently drops unknown dependencies.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(CollapsePropertiesPass::new()));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert_eq!(
            out.execution_order,
            vec!["collapse-properties".to_string()]
        );
        assert_eq!(out.stats["collapse-properties"].nodes_touched, 1);
        // The pipeline now iterates FixedPoint passes to a fixed point;
        // a non-changing solo pass converges in one sweep, so the old
        // "not-yet-iterated" limitation note is gone.
        assert!(
            !out.diagnostics
                .iter()
                .any(|d| d.group.0 == "pipeline.fixed-point-not-yet-iterated"),
            "the not-yet-iterated note must be gone now that the pipeline iterates; got {:?}",
            out.diagnostics
        );
    }

    #[test]
    fn pass_is_default_and_clone() {
        let _a: CollapsePropertiesPass = Default::default();
        let _b: CollapsePropertiesPass = CollapsePropertiesPass::new();
        let _c = _b;
        let _d = _c;
    }
}
