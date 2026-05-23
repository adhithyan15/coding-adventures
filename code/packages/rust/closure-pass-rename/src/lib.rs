//! Variable renaming pass for the Closure Compiler clone.
//!
//! Per [CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
//! canonical pass set. Replaces non-exported binding names (local
//! variables, internal function names, private class members) with
//! short identifiers — typically `a`, `b`, `c`, ... — to reduce
//! output size.
//!
//! # The two kinds of renaming (and why this pass handles both)
//!
//! 1. **Local renaming.** Within a single scope, rename
//!    `let user_name = ...` → `let a = ...`. Safe because the name
//!    is only visible inside the scope; no external code can refer
//!    to it.
//! 2. **Module-scoped renaming.** Across a module, rename
//!    non-exported top-level bindings the same way. Safe because
//!    nothing outside the module imports them.
//!
//! Externally-visible names (`export`s, public class methods,
//! property keys on objects passed to external code) **must not be
//! renamed** — that would break the public contract. The pass
//! reads the type sidecar's `external` attribute and the AST's
//! export markers to decide what's off-limits.
//!
//! # Where this pass sits in the canonical order
//!
//! CLOC06 §"Canonical pass set" pins:
//!
//! ```text
//! constant-fold → fold-control-flow → dce → inline → rename → ...
//! ```
//!
//! Rename runs **late** — after dead code is gone, after inlining
//! has decided which functions stay and which fold into call
//! sites. Renaming before DCE would waste work renaming bindings
//! that will get deleted; renaming before inline would make the
//! inlining heuristic harder (names tell the heuristic something
//! about user intent).
//!
//! In v1 `depends_on` is left empty rather than declaring
//! `["dce", "inline"]`. Reasoning: rename is *correct* with or
//! without those earlier passes — it just produces less
//! compression when they don't run. The scheduler shouldn't reject
//! a pipeline that only contains `rename`. Once we add a hard
//! dependency (e.g., a `freeze-externals` pass that rename
//! genuinely cannot run without), it goes here.
//!
//! # Why `OneShot` and not `FixedPoint`?
//!
//! Unlike constant-fold or DCE, rename doesn't open new
//! opportunities for itself. After one walk, every renameable
//! binding has been renamed; running again would just be a no-op.
//! `OneShot` tells the scheduler exactly that.
//!
//! # Scope (v1)
//!
//! `javascript-ast` ships only `Program` / `SourceType` today
//! (CLOC02 Phase 1). With no `Identifier` / `VariableDeclarator` /
//! `FunctionDeclaration` nodes there is nothing to rename;
//! [`RenamePass::run`] is identity in v1. Per CLOC03 §"When a pass
//! keeps a node unchanged" no contributions are emitted.
//!
//! What this PR locks down:
//!
//! 1. Pass metadata (`name`, `iteration_policy`, `cost`,
//!    `depends_on`) — what the scheduler keys on and what the
//!    future `closurec` CLI surfaces as `--disable=rename`.
//! 2. Establishes the pass as a registerable unit in the pipeline
//!    so subsequent PRs can wire in real renaming logic without
//!    touching the public surface.

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};

/// `Pass::depends_on` value. Empty in v1 — see crate-level docs
/// for why. Kept as a `const` so future tests/crates can refer to
/// it by reference rather than retyping.
const DEPS: &[&str] = &[];

/// Variable renaming pass. v1 is identity — see crate-level docs.
///
/// Zero-sized type: no per-instance state. Pass-internal state
/// (the binding → short-name map, the next-id counter, the
/// "do-not-rename" set seeded from `export`s and sidecar
/// `external` attributes) lives in pass-local maps constructed
/// inside [`Pass::run`] per CLOC06 §"Pass-internal state."
#[derive(Debug, Default, Clone, Copy)]
pub struct RenamePass;

impl RenamePass {
    /// Zero-arg constructor for ergonomic
    /// `PassPipeline::add(Box::new(RenamePass::new()))` registration.
    pub fn new() -> Self {
        Self
    }
}

impl Pass for RenamePass {
    fn name(&self) -> &'static str {
        "rename"
    }

    fn depends_on(&self) -> &[&'static str] {
        // Empty in v1: rename is correct with or without earlier
        // passes — it just produces less compression without them.
        // Future hard dependencies (e.g., a `freeze-externals`
        // pass) would go here.
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // After one walk, every renameable binding has been
        // renamed; a second walk would do nothing. Unlike
        // constant-fold and DCE which can cascade, rename doesn't
        // open new opportunities for itself.
        IterationPolicy::OneShot
    }

    fn cost(&self) -> u32 {
        // Two passes over the tree:
        //   1. Collect all bindings + figure out which are
        //      external (skip them).
        //   2. Walk again and substitute references.
        // Plus the name-allocator. Heavier than constant-fold's
        // single walk; comparable to DCE.
        3
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        // v1: no Identifier / VariableDeclarator / Function*
        // nodes in the AST yet, so there's nothing to rename. Pass
        // through unchanged. The real two-pass walk (collect →
        // substitute) slots in here once javascript-ast grows the
        // variants.
        Ok(PassOutput {
            program: ctx.program.clone(),
            contributions: Vec::new(),
            changed: false,
            diagnostics: Vec::new(),
            stats: PassStats {
                // Visited the program root; no identifiers renamed.
                nodes_touched: 1,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    //! These tests pin the public contract (name, policy, cost,
    //! deps) and lock the integration with `PassPipeline`. Even
    //! though v1's `run` is identity, the metadata is what the
    //! scheduler keys on, and it'll outlive the v1 body.
    use super::*;
    use coding_adventures_closure_pass_pipeline::{PassPipeline, PipelineOutput};
    use coding_adventures_correlation_vector::CVLog;
    use coding_adventures_javascript_ast::{Program, SourceType};
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }

    #[test]
    fn name_is_rename() {
        // The name is the public handle: `--disable=rename`,
        // `out.stats["rename"]`, etc. Drift here is a breaking
        // change.
        assert_eq!(RenamePass::new().name(), "rename");
    }

    #[test]
    fn iteration_policy_is_one_shot() {
        // Unlike fold/DCE, one pass converges. OneShot tells the
        // scheduler not to bother re-running.
        assert_eq!(
            RenamePass::new().iteration_policy(),
            IterationPolicy::OneShot
        );
    }

    #[test]
    fn cost_is_three_pass_units() {
        // Two-pass walk + name allocator. Heavier than constant-
        // fold's single walk.
        assert_eq!(RenamePass::new().cost(), 3);
    }

    #[test]
    fn depends_on_is_empty_in_v1() {
        // Empty in v1: rename is correct standalone. See
        // crate-level docs.
        let p = RenamePass::new();
        assert!(p.depends_on().is_empty());
    }

    #[test]
    fn invalidates_empty_in_v1() {
        // CLOC06 Open Question 1: invalidates() is informational
        // only in v0.1.0. Empty avoids over-committing.
        assert!(RenamePass::new().invalidates().is_empty());
    }

    #[test]
    fn run_on_empty_program_returns_unchanged_identity() {
        // Identity check: same CV, version, source_type; no
        // contributions, no diagnostics, changed=false,
        // nodes_touched=1.
        let pass = RenamePass::new();
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
    fn pipeline_runs_rename_as_solo_pass() {
        // Rename in a pipeline alone: should produce
        // execution_order=["rename"], stats["rename"], and — since
        // rename is OneShot, NOT FixedPoint — no "not yet iterated"
        // diagnostic from the scheduler.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(RenamePass::new()));

        let mut cv = CVLog::new(true);
        let out: PipelineOutput = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        assert_eq!(out.execution_order, vec!["rename".to_string()]);
        assert_eq!(out.stats["rename"].nodes_touched, 1);
        // OneShot ≠ FixedPoint: no fixed-point-deferred note.
        assert!(
            !out.diagnostics
                .iter()
                .any(|d| d.group.0 == "pipeline.fixed-point-not-yet-iterated"),
            "OneShot should NOT trigger the FixedPoint note; got {:?}",
            out.diagnostics
        );
    }

    #[test]
    fn pass_is_default_and_clone() {
        // ZST + Default + Copy + Clone keeps registration
        // ergonomic and avoids ownership thinking at call sites.
        let _a: RenamePass = Default::default();
        let _b: RenamePass = RenamePass::new();
        let _c = _b;
        let _d = _c.clone();
    }
}
