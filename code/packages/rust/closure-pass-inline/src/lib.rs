//! Function-inlining pass for the Closure Compiler clone.
//!
//! Per [CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
//! canonical pass set. Substitutes a callee's body at the call
//! site when doing so is cheaper than the call:
//!
//! ```js
//! // before
//! function double(x) { return x * 2; }
//! const a = double(7);
//!
//! // after
//! const a = 7 * 2;        // (then constant-fold turns this into `14`)
//! ```
//!
//! # The two questions every inliner answers
//!
//! 1. **Is it safe?** A call can only be inlined if substituting
//!    the body doesn't change semantics. The hard cases:
//!    - `this` and `arguments` bindings: the inlined body sees
//!      different ones than it did when defined.
//!    - Captured variables from a different closure scope.
//!    - Recursive calls (inline once, sure — but where do you
//!      stop?).
//!    - Side-effecting argument expressions vs. parameters used
//!      multiple times in the body (you'd evaluate the arg twice).
//!    The sidecar's `no_side_effects` / `pure` attributes plus a
//!    use-count analysis on each parameter answer most of this.
//! 2. **Is it worth it?** Inlining a 1000-line function at 50
//!    call sites bloats output. Inlining a 3-line single-use
//!    helper shrinks it. CLOC06 leaves the exact heuristic open;
//!    common knobs are body size, call count, and whether the
//!    callee is itself the result of a fold (and thus probably
//!    going to fold further once inlined).
//!
//! # Why this enables downstream folding
//!
//! Once the body is substituted at the call site, the next
//! `constant-fold` iteration sees concrete arguments instead of
//! parameter references. `double(7)` → `7 * 2` → `14`. That's
//! why the canonical order has inline *after* constant-fold but
//! also why we'd want fold to run again afterward — which is
//! exactly what `IterationPolicy::FixedPoint` on the pipeline
//! gets us once the v0.1.0 scheduler grows iteration support.
//!
//! # Where this pass sits in the canonical order
//!
//! CLOC06 §"Canonical pass set" pins:
//!
//! ```text
//! constant-fold → fold-control-flow → dce → inline → rename → ...
//! ```
//!
//! Inline runs **after DCE** so it doesn't bother inlining
//! callees that are about to be deleted, and **before rename** so
//! the inliner's heuristics can use meaningful names rather than
//! `a`/`b`/`c`.
//!
//! # Why depends_on = ["constant-fold"]?
//!
//! Folding the *arguments* at a call site (`f(1+1)` → `f(2)`)
//! makes the inliner's substitution simpler — concrete literals
//! plug in cleanly. Without fold, the inliner would carry around
//! unfolded expression trees as argument bindings.
//!
//! We don't declare `depends_on = ["constant-fold", "dce"]` even
//! though the canonical order has DCE before inline. DCE is a
//! *preference* (don't waste inlining work on dead callees), not
//! a *correctness requirement*. Inlining is still correct on
//! un-DCE'd input; it just does redundant work.
//!
//! # Scope (v1)
//!
//! `javascript-ast` ships only `Program` / `SourceType` today
//! (CLOC02 Phase 1). With no `FunctionDeclaration` /
//! `CallExpression` / `Identifier` nodes there is nothing to
//! inline; [`InlinePass::run`] is identity in v1. Per CLOC03
//! §"When a pass keeps a node unchanged" no contributions are
//! emitted.
//!
//! What this PR locks down:
//!
//! 1. Pass metadata (`name`, `iteration_policy`, `cost`,
//!    `depends_on`) — what the scheduler keys on and what the
//!    future `closurec` CLI surfaces as `--disable=inline`.
//! 2. The `depends_on("constant-fold")` edge so the scheduler
//!    forces fold-before-inline as soon as both passes are in
//!    one pipeline.
//! 3. The two-pass integration test that proves the ordering.

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use coding_adventures_closure_scope_analyzer::{analyze, BindingId, BindingKind};

/// `Pass::depends_on` value. Kept as a `const` so future tests
/// and dependent crates can refer to it without retyping the
/// pass name.
const DEPS: &[&str] = &["constant-fold"];

/// Function-inlining pass. v1 is identity — see crate-level docs.
///
/// Zero-sized type: no per-instance state. Pass-internal state
/// (call graph, inlining-budget counter, per-call substitution
/// map, the "do-not-inline" set seeded from recursive callees
/// and exported function declarations) lives in pass-local maps
/// constructed inside [`Pass::run`] per CLOC06 §"Pass-internal
/// state."
#[derive(Debug, Default, Clone, Copy)]
pub struct InlinePass;

impl InlinePass {
    /// Zero-arg constructor for ergonomic
    /// `PassPipeline::add(Box::new(InlinePass::new()))` registration.
    pub fn new() -> Self {
        Self
    }
}

impl Pass for InlinePass {
    fn name(&self) -> &'static str {
        "inline"
    }

    fn depends_on(&self) -> &[&'static str] {
        // CLOC06 canonical order: constant-fold first so the
        // inliner sees folded arguments at call sites. Folded
        // literals plug into parameters cleanly; unfolded
        // expressions would require carrying around argument
        // expression trees.
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // Per CLOC06: inlining is canonically fixed-point.
        // Inlining `f(g(h(7)))` first inlines `f`, exposing the
        // call to `g` in the now-substituted body; the next
        // iteration can inline `g`, and so on. Bounded in
        // practice by the inlining-budget heuristic, not by the
        // policy.
        IterationPolicy::FixedPoint
    }

    fn cost(&self) -> u32 {
        // Heavier than the folds and DCE:
        //   - Build a call graph (per-function caller/callee
        //     edges) once per pipeline iteration.
        //   - For each call site, decide whether to inline
        //     (heuristic eval).
        //   - For each inlined call, clone the callee body and
        //     rewrite identifiers to the call-site bindings.
        // The clone-and-rewrite is the expensive step in
        // practice.
        4
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        // CLOC13.B wiring: consume the shared `ScopeAnalysis` from
        // `closure-scope-analyzer` to identify *inline candidates*
        // — function/class-shaped bindings that are called from
        // exactly one site, where substituting the body in place
        // saves the call overhead and lets downstream constant-fold
        // see the now-concrete arguments.
        //
        // The algorithm (mark phase; substitute deferred):
        //
        //   1. Per-binding use-count derived from
        //      `analysis.references`. The single-use property is
        //      the gate that makes inlining cheap (clone once vs.
        //      duplicating the body N times).
        //   2. Candidate scan. A binding is an inline candidate
        //      when ALL of:
        //        a. kind == Function OR kind == Class — these are
        //           the body-shaped kinds inlining substitutes
        //           into call/new sites. (`Var`/`Let`/`Const`
        //           bindings of *function expressions* lower to
        //           Function once the analyzer grows expression
        //           tracking; until then they're tracked as
        //           Const/Let and CLOC13.D handles their alias
        //           form.) `Param` is excluded — params aren't
        //           callable bodies. `#[non_exhaustive]` future
        //           variants are conservatively skipped (same
        //           default as treeshake / collapse-properties).
        //        b. uses == 1. Multi-use inlining is a budget
        //           decision (size threshold × call-site count);
        //           the single-use case is the unambiguous win
        //           and the cheapest substitution to land first.
        //   3. Substitute — deferred to CLOC13.B.1 because cleanly
        //      replacing a CallExpression with the callee's body
        //      requires both the AST to grow CallExpression /
        //      FunctionDeclaration variants AND the analyzer to
        //      surface a binding → defining-node backreference.
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
        // so the candidate scan finds zero call targets, the
        // candidates vec is empty, `nodes_touched` is small, and
        // the program passes through unchanged. The wiring
        // becomes *effective* (real candidate-finding) the moment
        // CLOC13.0 lands the analyzer body — no churn here.

        let analysis = analyze(ctx.program);

        // Step 1: use-count per binding.
        let mut use_count: Vec<usize> = vec![0; analysis.bindings.len()];
        for reference in &analysis.references {
            if let Some(BindingId(idx)) = reference.binding {
                if let Some(slot) = use_count.get_mut(idx as usize) {
                    *slot += 1;
                }
            }
        }

        // Step 2: single-use function/class scan.
        let mut inline_candidates: Vec<BindingId> = Vec::new();
        for (idx, binding) in analysis.bindings.iter().enumerate() {
            let id = BindingId(idx as u32);
            let uses = use_count.get(idx).copied().unwrap_or(0);
            if uses != 1 {
                continue; // multi-use is a budget decision deferred to a later PR
            }
            // `#[non_exhaustive]` on BindingKind: conservative
            // wildcard, future variants skipped.
            match binding.kind {
                BindingKind::Function | BindingKind::Class => {
                    inline_candidates.push(id);
                }
                // Var/Let/Const/Param: not function-body bindings;
                // var/let/const-of-function-expr lowering waits
                // for analyzer expression tracking.
                _ => {}
            }
        }

        // Step 3 deferred — keep `changed = false`.
        let _inline_candidates = inline_candidates;

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
    //! Tests pin the public contract (name, policy, cost,
    //! deps) and lock the ordering integration with
    //! `PassPipeline`. v1's `run` is identity, but the metadata
    //! drives the scheduler and outlives the v1 body.
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
    fn name_is_inline() {
        // `--disable=inline`, `out.stats["inline"]`, etc. all
        // key on this. Drift here is a breaking change.
        assert_eq!(InlinePass::new().name(), "inline");
    }

    #[test]
    fn iteration_policy_is_fixed_point() {
        // Inlining `f(g(...))` exposes a new call to `g` in the
        // substituted body. FixedPoint is the right intent even
        // though v1 converges in one no-op step.
        assert_eq!(
            InlinePass::new().iteration_policy(),
            IterationPolicy::FixedPoint
        );
    }

    #[test]
    fn cost_is_four_pass_units() {
        // Call graph + heuristic + clone-and-rewrite is the
        // heaviest of the v1 passes.
        assert_eq!(InlinePass::new().cost(), 4);
    }

    #[test]
    fn depends_on_constant_fold() {
        let p = InlinePass::new();
        assert_eq!(p.depends_on(), &["constant-fold"]);
    }

    #[test]
    fn invalidates_empty_in_v1() {
        // CLOC06 Open Question 1: informational only in v0.1.0.
        assert!(InlinePass::new().invalidates().is_empty());
    }

    #[test]
    fn run_on_empty_program_returns_unchanged_identity() {
        // Identity: same CV, version, source_type; no
        // contributions, no diagnostics, changed=false,
        // nodes_touched=1.
        let pass = InlinePass::new();
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
    fn pipeline_orders_constant_fold_before_inline() {
        // Register InlinePass FIRST. If `depends_on` were
        // ignored, the pipeline would run them in registration
        // order: [inline, constant-fold]. The scheduler must
        // reorder them to [constant-fold, inline].
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(InlinePass::new()));
        pipeline.add(Box::new(ConstantFoldPass::new()));

        let mut cv = CVLog::new(true);
        let out: PipelineOutput = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        assert_eq!(
            out.execution_order,
            vec!["constant-fold".to_string(), "inline".to_string()],
            "inline must run after constant-fold per CLOC06 canonical order"
        );
        assert!(out.stats.contains_key("constant-fold"));
        assert!(out.stats.contains_key("inline"));
    }

    #[test]
    fn pipeline_runs_inline_as_solo_pass() {
        // InlinePass alone (without constant-fold) still works:
        // depends_on is "soft" in v1 — the v0.1.0 scheduler
        // silently drops unknown dependencies.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(InlinePass::new()));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert_eq!(out.execution_order, vec!["inline".to_string()]);
        assert_eq!(out.stats["inline"].nodes_touched, 1);
        // FixedPoint policy → v0.1.0 pipeline emits the "not yet
        // iterated" informational note.
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
        let _a: InlinePass = Default::default();
        let _b: InlinePass = InlinePass::new();
        let _c = _b;
        let _d = _c.clone();
    }
}
