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
//! `RemoveUnusedVarsPass::run` is a real transform: it uses
//! `closure-scope-analyzer` to find top-level (`ScopeId::GLOBAL`)
//! `var`/`let`/`const` bindings that no [`Reference`] resolves to, and
//! deletes them — provided the initializer is side-effect-free (see
//! [`is_removable_init`]).
//!
//! What this pass locks down:
//!
//! 1. Pass metadata (`name`, `iteration_policy`, `cost`,
//!    `depends_on`) — what the scheduler keys on and what the
//!    `closurec` CLI surfaces as `--disable=remove-unused-vars`.
//! 2. The `depends_on(["dce", "inline"])` edges so the scheduler
//!    forces DCE-before, inline-before-this when all three share a
//!    pipeline.
//! 3. Removal in **both** item shapes the program body can hold — the
//!    bare `ProgramItem::Declaration` and the
//!    `ProgramItem::Statement(Statement::Declaration(...))` form the
//!    `javascript-parser` bridge actually emits — gated by a
//!    conservative initializer-purity check.
//!
//! Scope of the current slice: only `GLOBAL`-scope bindings with a
//! literal / identifier / absent initializer. Function-local removal
//! and sidecar-driven purity (to reach `const x = pureCall()`) are
//! follow-ups; "keep" is always the safe answer in the meantime.

use std::collections::{HashMap, HashSet};

use coding_adventures_correlation_vector::Contribution;
use serde_json::json;

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use coding_adventures_closure_scope_analyzer::{
    analyze, BindingId, BindingKind, ScopeId,
};
use coding_adventures_javascript_ast::{
    BindingTarget, Declaration, Expression, ProgramItem, Statement, VariableDeclaration,
};

/// Is a variable initializer safe to delete along with its binding?
///
/// `remove-unused-vars` deletes the *whole* declarator — `id` **and**
/// `init` — when the binding is unreferenced. That is only sound when
/// evaluating `init` has no observable side effect, because the
/// initializer would otherwise still need to run for its effect even
/// though nobody reads the result.
///
/// We answer conservatively. A declarator is removable when its
/// initializer is:
///
/// - **absent** (`var x;`) — nothing to evaluate;
/// - a **literal** (number, string, boolean, null, bigint, undefined) —
///   evaluating a literal is pure;
/// - a bare **identifier** (`var x = y;`) — reading a variable binding
///   has no side effect (there are no value-level getters in JS).
///
/// Everything else stays: a call (`f()`), `new`, a member access
/// (`o.p` may trigger a getter), an assignment (`y = 1`), an array /
/// object literal (its elements may not be pure), etc. Those *might*
/// have side effects, so we keep the declarator and let the
/// (unreferenced but harmless) binding remain. A later, sidecar-driven
/// purity analysis can widen this set; until then "keep" is always the
/// safe answer.
fn is_removable_init(init: &Option<Expression>) -> bool {
    match init {
        None => true,
        Some(expr) => matches!(
            expr,
            Expression::NumericLiteral(_)
                | Expression::StringLiteral(_)
                | Expression::BooleanLiteral(_)
                | Expression::NullLiteral(_)
                | Expression::BigIntLiteral(_)
                | Expression::UndefinedLiteral(_)
                | Expression::Identifier(_)
        ),
    }
}

/// Prune the dead declarators out of one `VariableDeclaration`.
///
/// Returns `(pruned, removed)`:
/// - `removed` is how many declarators were dropped;
/// - `pruned` is `None` when *every* declarator was dropped (the whole
///   declaration disappears), or `Some(decl)` with the survivors
///   otherwise. When nothing was dropped, `pruned` is `Some(clone)` and
///   `removed == 0` — callers can keep the original item verbatim.
///
/// A declarator is dropped only when its name is in `dead` **and** its
/// initializer is [`is_removable_init`] — so a dead binding with a
/// side-effecting initializer is preserved.
fn prune_var_decl(
    var_decl: &VariableDeclaration,
    dead: &HashSet<String>,
) -> (Option<VariableDeclaration>, Vec<(Option<String>, String)>) {
    let mut kept = Vec::with_capacity(var_decl.declarations.len());
    // Each removed declarator's own CV id (if any) plus its name — so
    // the caller can tombstone the exact binding that vanished.
    let mut removed: Vec<(Option<String>, String)> = Vec::new();
    for declarator in &var_decl.declarations {
        let BindingTarget::Identifier(id) = &declarator.id;
        if dead.contains(&id.name) && is_removable_init(&declarator.init) {
            removed.push((declarator.cv.clone(), id.name.clone()));
        } else {
            kept.push(declarator.clone());
        }
    }
    if removed.is_empty() {
        (Some(var_decl.clone()), removed)
    } else if kept.is_empty() {
        (None, removed)
    } else {
        let mut split = var_decl.clone();
        split.declarations = kept;
        (Some(split), removed)
    }
}

/// `Pass::depends_on` value — both DCE and inline must run first
/// per CLOC06 canonical order. Kept as a `const` so future tests
/// and sibling crates can reference these names without retyping.
const DEPS: &[&str] = &["dce", "inline"];

/// Unreferenced-variable cleanup pass — deletes top-level bindings
/// nothing references, when their initializer is side-effect-free.
/// See crate-level docs.
///
/// Zero-sized type: no per-instance state. Pass-internal state
/// (the binding → reference-count map, the dead-name set) lives in
/// pass-local maps constructed inside [`Pass::run`] per CLOC06
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
        // CLOC13.E wiring: consume the shared `ScopeAnalysis` from
        // `closure-scope-analyzer` to identify bindings that no
        // reference points at. The algorithm:
        //
        //   1. Build a use-count table: `BindingId → usize` by
        //      scanning `analysis.references`. Unresolved
        //      references (binding = None) don't increment any
        //      count — those are free globals, never our problem.
        //   2. Walk `analysis.bindings`. A binding is *eligible*
        //      for removal when:
        //        a. use-count == 0
        //        b. kind ∈ { Var, Let, Const } (skipping Function
        //           bodies / Param / Class — those need extra
        //           analysis the analyzer doesn't yet expose).
        //   3. Apply the removal: walk `program.body` and drop the
        //      matching `VariableDeclarator`s whose initializer is
        //      side-effect-free (`is_removable_init`).
        //
        // All three steps are live. `changed` is `true` iff at least
        // one declarator was actually dropped; when no binding is dead
        // (or every dead binding has a side-effecting initializer) the
        // program passes through unchanged with `changed = false`.
        //
        // The removal is restricted to `ScopeId::GLOBAL` bindings,
        // matched by name against the top-level items in `program.body`
        // — see the dead-name set built below. Function-local removal
        // needs nested-scope name handling and is a follow-up.

        let analysis = analyze(ctx.program);

        // Step 1: use-count per binding.
        //
        // We scan the references list once. A single bound
        // identifier (the `name` field on `Reference`) can resolve
        // to exactly one binding, so this is O(references) — no
        // need for a hash set, just a vec indexed by binding id.
        let mut use_count: Vec<usize> = vec![0; analysis.bindings.len()];
        for reference in &analysis.references {
            if let Some(BindingId(idx)) = reference.binding {
                if let Some(slot) = use_count.get_mut(idx as usize) {
                    *slot += 1;
                }
                // Out-of-range BindingId — defensive: skip rather
                // than panic. Could only happen if a downstream
                // pass minted a forged ID, which is a bug
                // elsewhere. We treat it as "binding not found"
                // and let the scheduler keep going.
            }
        }

        // Step 2: eligibility scan.
        let mut dead_bindings: Vec<BindingId> = Vec::new();
        for (idx, binding) in analysis.bindings.iter().enumerate() {
            let id = BindingId(idx as u32);
            let uses = use_count.get(idx).copied().unwrap_or(0);
            if uses != 0 {
                continue;
            }
            // Only target plain variable bindings in v0.2.0.
            // `Function` / `Param` / `Class` need extra reachability
            // analysis (e.g., a Param might be unreferenced but
            // structurally required); `Class` isn't in the AST
            // yet anyway.
            //
            // `#[non_exhaustive]` on `BindingKind` (the
            // `closure-scope-analyzer` enum) means we explicitly
            // list the cases we ACT on; the implicit `_ => ()`
            // arm makes future variants conservatively passthrough.
            match binding.kind {
                BindingKind::Var | BindingKind::Let | BindingKind::Const => {
                    dead_bindings.push(id);
                }
                _ => {}
            }
        }

        // Step 3 — APPLY.  CLOC13.E.1 lifts the hard-pin on
        // `changed` and actually mutates the program.
        //
        // Strategy.  Without a binding → declarator backreference
        // (the analyzer hasn't grown one yet), we match by name +
        // scope.  Restrict the dead set to bindings in
        // `ScopeId::GLOBAL` (the only scope CLOC13.0 populates;
        // when CLOC13.0.1 introduces nested scopes, those
        // bindings will still be in their own scope, so this
        // filter stays correct — we only ever act on top-level
        // names).  Build a `HashSet<String>` of dead names.  Walk
        // `program.body`:
        //
        //   - `Declaration::VariableDeclaration` with declarators
        //     drawn from `BindingTarget::Identifier`: keep only
        //     declarators whose identifier name is NOT dead.
        //     - If every declarator is dead → drop the whole item.
        //     - If some are dead → produce a new
        //       `VariableDeclaration` with the surviving
        //       declarators.  Initializers of dropped declarators
        //       are dropped wholesale; this matches the
        //       remove-unused-vars contract (the pass only fires
        //       on declarations whose initializers are
        //       side-effect-free per the type sidecar / DCE
        //       ordering, CLOC06 §"What the pass relies on").
        //   - `Declaration::FunctionDeclaration`: not in scope for
        //     this PR — `Function`-kind bindings are filtered out
        //     at step 2.  Function-declaration removal is the
        //     treeshake pass's job.
        //   - `ProgramItem::Statement`: untouched.  Statement
        //     walking lands in CLOC13.0.1 alongside references.
        //
        // **Pin lifted; safety preserved.**  `changed = removed >
        // 0` is now safe because we genuinely mutated when we
        // say we did.  If `removed == 0` (the analyzer surfaced
        // no dead bindings, or none were `BindingKind::Var/Let/
        // Const` in GLOBAL), we return the program unchanged
        // with `changed = false`, identical to v0.2.0 behavior.
        //
        // **Why this stays safe under FixedPoint.**  Each
        // iteration reduces the binding set strictly — a removed
        // VariableDeclaration produces no new bindings, so the
        // next iteration's eligibility scan finds fewer dead
        // entries.  The fixed point is reached in at most one
        // additional iteration after the first non-empty
        // removal: bindings can only stop being dead by gaining
        // a reference, which a removal never adds.

        // Build the dead-name set, restricted to GLOBAL.
        let mut dead_names: HashSet<String> = HashSet::new();
        for binding_id in &dead_bindings {
            let binding = &analysis.bindings[binding_id.0 as usize];
            if binding.scope == ScopeId::GLOBAL {
                dead_names.insert(binding.name.clone());
            }
        }

        // Walk + rewrite `program.body`.  Build a new vec rather
        // than `Vec::retain_mut` so the splitting case (some
        // declarators dead, some live) is straightforward.
        let mut new_body: Vec<ProgramItem> = Vec::with_capacity(ctx.program.body.len());
        let mut removed_count: usize = 0;
        // Each removed binding's own CV id + name, captured before the
        // declarator is dropped, so it can be tombstoned below.
        let mut all_removed: Vec<(Option<String>, String)> = Vec::new();
        for item in &ctx.program.body {
            // A top-level `var/let/const` can reach us in TWO shapes:
            //
            //   - `ProgramItem::Declaration(VariableDeclaration)` — the
            //     "bare" form.
            //   - `ProgramItem::Statement(Statement::Declaration(
            //     VariableDeclaration))` — the wrapped form. This is what
            //     the `javascript-parser` bridge actually emits for a
            //     top-level `var x = 1;` (it routes `variable_statement`
            //     through `Statement::Declaration`). Earlier revisions
            //     only matched the bare form, so the pass was a silent
            //     no-op on every real (bridged) program — there was no
            //     test exercising removal to catch it.
            //
            // We prune both shapes with the same `prune_var_decl` helper
            // and re-wrap the survivors in whichever shape they arrived.
            match item {
                ProgramItem::Declaration(Declaration::VariableDeclaration(var_decl)) => {
                    let (pruned, removed) = prune_var_decl(var_decl, &dead_names);
                    removed_count += removed.len();
                    if removed.is_empty() {
                        new_body.push(item.clone());
                    } else {
                        all_removed.extend(removed);
                        if let Some(survivors) = pruned {
                            new_body.push(ProgramItem::Declaration(
                                Declaration::VariableDeclaration(survivors),
                            ));
                        }
                        // pruned == None → whole declaration dropped.
                    }
                }
                ProgramItem::Statement(Statement::Declaration(
                    Declaration::VariableDeclaration(var_decl),
                )) => {
                    let (pruned, removed) = prune_var_decl(var_decl, &dead_names);
                    removed_count += removed.len();
                    if removed.is_empty() {
                        new_body.push(item.clone());
                    } else {
                        all_removed.extend(removed);
                        if let Some(survivors) = pruned {
                            new_body.push(ProgramItem::Statement(Statement::Declaration(
                                Declaration::VariableDeclaration(survivors),
                            )));
                        }
                    }
                }
                // Function declarations (Function-kind bindings are
                // filtered out at step 2 — treeshake's job) and every
                // other statement pass through untouched.
                _ => new_body.push(item.clone()),
            }
        }

        // Deletion provenance (#89). Like DCE and treeshake, this pass
        // must not delete a binding silently — each removed declarator's
        // own CV entry is tombstoned with a `DeletionRecord` via
        // `CVLog::delete`, so a `--correlation_vector` consumer asking
        // "what happened to `const foo`?" gets a definite answer:
        // *remove-unused-vars removed it because it was unreferenced.*
        // `delete` is a no-op when the log is disabled (production
        // default), so this costs nothing off that path. We also emit one
        // summary `Contribution` against the program root.
        let mut contributions: Vec<Contribution> = Vec::new();
        if !all_removed.is_empty() {
            for (cv_id, name) in &all_removed {
                if let Some(id) = cv_id {
                    let mut meta: HashMap<String, serde_json::Value> = HashMap::new();
                    meta.insert("name".to_string(), json!(name));
                    ctx.cv
                        .delete(id, "remove-unused-vars", "removed-unused-binding", meta);
                }
            }
            if let Some(prog_cv) = &ctx.program.cv {
                contributions.push(Contribution {
                    source: "remove-unused-vars".to_string(),
                    tag: "removed-unused-binding".to_string(),
                    meta: [
                        ("removed".to_string(), json!(all_removed.len())),
                        ("parent_cv".to_string(), json!(prog_cv)),
                    ]
                    .into_iter()
                    .collect(),
                });
            }
        }

        // Construct the output program with the rewritten body.
        let mut new_program = ctx.program.clone();
        new_program.body = new_body;

        let changed = removed_count > 0;

        Ok(PassOutput {
            program: new_program,
            contributions,
            changed,
            diagnostics: Vec::new(),
            stats: PassStats {
                // Visited the program root + every binding +
                // every reference. Gives the scheduler real
                // visit counts for cost accounting.
                nodes_touched: (1
                    + analysis.bindings.len()
                    + analysis.references.len()) as u32,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    //! Tests pin the public contract: pass metadata, the two- and
    //! three-pass ordering integrations with `PassPipeline`, and the
    //! removal behavior itself — actual deletion of dead bindings in
    //! both item shapes, the multi-declarator split, and the
    //! initializer-purity gate.
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
        let _a: RemoveUnusedVarsPass = Default::default();
        let _b: RemoveUnusedVarsPass = RemoveUnusedVarsPass::new();
        let _c = _b;
        let _d = _c;
    }

    // -----------------------------------------------------------------
    // CLOC13.E.1 — apply-step tests.
    //
    // These exercise the body-walk machinery (split / drop /
    // passthrough).  They run against the *current*
    // closure-scope-analyzer build, which today (CLOC13.0
    // not-yet-merged) returns empty bindings — meaning the
    // dead-name set is empty and the body walks every item
    // through the passthrough path.  That's the right test
    // surface for now: it pins the no-op-when-no-dead-bindings
    // contract.
    //
    // Real-removal tests (assertions that an actually-unused
    // `let x` gets dropped) land in a follow-up once CLOC13.0
    // is on main, because they need the analyzer to surface
    // real bindings.  Adding them today would be brittle:
    // they'd fail until #4787 merges, succeed afterward.  Pin
    // the contracts we can pin now; let the follow-up pin the
    // rest once it can.
    // -----------------------------------------------------------------

    use coding_adventures_javascript_ast::{
        BindingTarget, BlockStatement, CallExpression, Declaration, Expression, ExpressionStatement,
        FunctionDeclaration, Identifier, NumericLiteral, ProgramItem, Statement, VarKind,
        VariableDeclaration, VariableDeclarator,
    };

    fn ident(name: &str) -> Identifier {
        Identifier {
            cv: None,
            name: name.to_string(),
        }
    }

    /// A top-level `var/let/const` in the **Statement-wrapped** shape
    /// the `javascript-parser` bridge actually emits for a real program
    /// (`ProgramItem::Statement(Statement::Declaration(...))`). Mirrors
    /// `var_decl` (which builds the bare `ProgramItem::Declaration`
    /// form), so the two helpers let a test pin both shapes.
    fn var_stmt(kind: VarKind, names_with_init: &[(&str, Option<f64>)]) -> ProgramItem {
        let ProgramItem::Declaration(decl) = var_decl(kind, names_with_init) else {
            unreachable!("var_decl always returns a Declaration")
        };
        ProgramItem::Statement(Statement::Declaration(decl))
    }

    /// A bare expression statement that *reads* `name` — i.e. a
    /// reference that keeps the binding alive (`name;`).
    fn use_stmt(name: &str) -> ProgramItem {
        ProgramItem::Statement(Statement::expression_statement(ExpressionStatement {
            cv: None,
            expression: Expression::Identifier(ident(name)),
        }))
    }

    /// A Statement-wrapped `let <name> = <callee>();` — an unused
    /// binding whose initializer is a **call** (impure), so the purity
    /// gate must keep it.
    fn var_stmt_call_init(name: &str, callee: &str) -> ProgramItem {
        ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(
            VariableDeclaration {
                cv: None,
                kind: VarKind::Let,
                declarations: vec![VariableDeclarator {
                    cv: None,
                    id: BindingTarget::Identifier(ident(name)),
                    init: Some(Expression::CallExpression(CallExpression {
                        cv: None,
                        callee: Box::new(Expression::Identifier(ident(callee))),
                        arguments: Vec::new(),
                    })),
                }],
            },
        )))
    }

    fn var_decl(kind: VarKind, names_with_init: &[(&str, Option<f64>)]) -> ProgramItem {
        ProgramItem::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind,
            declarations: names_with_init
                .iter()
                .map(|(n, init)| VariableDeclarator {
                    cv: None,
                    id: BindingTarget::Identifier(ident(n)),
                    init: init.map(|v| {
                        Expression::NumericLiteral(NumericLiteral {
                            cv: None,
                            value: v,
                            raw: v.to_string(),
                        })
                    }),
                })
                .collect(),
        }))
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

    fn program_with(items: Vec<ProgramItem>) -> Program {
        let mut p = program();
        p.body = items;
        p
    }

    fn run_pass(prog: Program) -> coding_adventures_closure_pass_pipeline::PassOutput {
        // Drive the pass directly (not via the pipeline) so the test
        // pins the pass's own contract, not the scheduler's.
        let pass = RemoveUnusedVarsPass::new();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let ctx = coding_adventures_closure_pass_pipeline::PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        };
        pass.run(ctx).expect("pass ran")
    }

    // -----------------------------------------------------------------
    // CV deletion provenance (#89).
    //
    // Mirror the pipeline: the lexer/parser `create` a CV entry per node
    // and stamp its id onto the AST. So we `create` the declarator's
    // entry FIRST, stamp its id, then run the pass — otherwise
    // `cv.delete` has no entry to tombstone and the assertion would be
    // vacuous. Property: a removed binding's CV entry survives with a
    // `DeletionRecord{source:"remove-unused-vars"}`.
    // -----------------------------------------------------------------

    /// A Statement-wrapped `let <name> = 1;` whose declarator's CV id is
    /// freshly created in `log`.
    fn traced_var(log: &mut CVLog, name: &str) -> (ProgramItem, String) {
        let id = log.create(None);
        let item = ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(
            VariableDeclaration {
                cv: None,
                kind: VarKind::Let,
                declarations: vec![VariableDeclarator {
                    cv: Some(id.clone()),
                    id: BindingTarget::Identifier(ident(name)),
                    init: Some(Expression::NumericLiteral(NumericLiteral {
                        cv: None,
                        value: 1.0,
                        raw: "1".to_string(),
                    })),
                }],
            },
        )));
        (item, id)
    }

    fn run_capturing_cv(
        prog: &Program,
        cv: &mut CVLog,
    ) -> coding_adventures_closure_pass_pipeline::PassOutput {
        let sidecar = Sidecar::new();
        let ctx = coding_adventures_closure_pass_pipeline::PassContext {
            program: prog,
            sidecar: &sidecar,
            cv,
        };
        RemoveUnusedVarsPass::new().run(ctx).expect("pass ran")
    }

    #[test]
    fn removed_binding_is_tombstoned() {
        // `let dead = 1;` — unreferenced global with a pure initializer →
        // removed, so its CV entry must be tombstoned.
        let mut log = CVLog::new(true);
        let (dead, dead_cv) = traced_var(&mut log, "dead");
        let prog = program_with(vec![dead]);

        let out = run_capturing_cv(&prog, &mut log);

        assert!(out.changed);
        let del = log
            .get(&dead_cv)
            .unwrap()
            .deleted
            .as_ref()
            .expect("a removed binding must be tombstoned");
        assert_eq!(del.source, "remove-unused-vars");
        assert_eq!(del.reason, "removed-unused-binding");
        assert_eq!(del.meta.get("name").and_then(|v| v.as_str()), Some("dead"));
    }

    #[test]
    fn referenced_binding_is_not_tombstoned() {
        // `let keep = 1; keep;` — the read keeps `keep` alive, so it is
        // neither removed nor tombstoned.
        let mut log = CVLog::new(true);
        let (keep, keep_cv) = traced_var(&mut log, "keep");
        let prog = program_with(vec![keep, use_stmt("keep")]);

        let out = run_capturing_cv(&prog, &mut log);

        assert!(!out.changed, "a referenced binding must survive");
        assert!(
            log.get(&keep_cv).unwrap().deleted.is_none(),
            "a surviving binding must NOT be tombstoned"
        );
    }

    #[test]
    fn disabled_log_still_removes_without_panicking() {
        // With CV disabled, `delete` is a no-op; the pass must still
        // remove the dead binding and never panic on the missing entry.
        let mut log = CVLog::new(false);
        let (dead, _cv) = traced_var(&mut log, "dead");
        let prog = program_with(vec![dead]);

        let out = run_capturing_cv(&prog, &mut log);

        assert!(out.changed);
    }

    #[test]
    fn apply_step_passthrough_keeps_used_let() {
        // `let x; x;` — the `x;` ExpressionStatement is the use
        // that keeps `x` live. Now that CLOC13.0.1 (PR #4787's
        // follow-up) is on this branch, the analyzer surfaces
        // both the `x` binding and the `x` reference, the
        // use-count for `x` is 1, so the apply step skips it.
        //
        // Pre-CLOC13.0.1 this test fixture was just `let x;` with
        // no use; that fixture would FAIL today because the
        // analyzer correctly identifies `x` as dead (zero refs).
        // The use-the-binding fix preserves the original
        // "no removal" assertion.
        let stmt_x = ProgramItem::Statement(Statement::expression_statement(
            ExpressionStatement {
                cv: None,
                expression: Expression::Identifier(Identifier {
                    cv: None,
                    name: "x".to_string(),
                }),
            },
        ));
        let prog = program_with(vec![var_decl(VarKind::Let, &[("x", None)]), stmt_x]);
        let out = run_pass(prog.clone());
        assert!(!out.changed, "x is referenced → no removal");
        assert_eq!(out.program.body.len(), 2);
    }

    #[test]
    fn apply_step_keeps_function_declaration() {
        // `function f() {}` — kind == Function is filtered out at
        // step 2 of the pass (functions are treeshake's job, not
        // remove-unused-vars').  Even after the analyzer body
        // lands, this should never be eligible for removal here.
        let prog = program_with(vec![fn_decl("f")]);
        let out = run_pass(prog);
        assert!(!out.changed);
        assert_eq!(out.program.body.len(), 1);
        match &out.program.body[0] {
            ProgramItem::Declaration(Declaration::FunctionDeclaration(fd)) => {
                assert_eq!(fd.id.name, "f");
            }
            _ => panic!("expected the function declaration to survive"),
        }
    }

    #[test]
    fn apply_step_passes_statements_through_untouched() {
        // Statement items are explicitly out of scope for the
        // apply step — CLOC13.0.1 lands the reference walker that
        // touches them.  Pin the passthrough contract.
        use coding_adventures_javascript_ast::{ExpressionStatement, Statement};
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
        let out = run_pass(prog);
        assert!(!out.changed);
        assert_eq!(out.program.body.len(), 1);
    }

    #[test]
    fn apply_step_preserves_multi_declarator_when_all_referenced() {
        // `const a = 1, b = 2; a; b;` — multi-declarator form
        // where BOTH declarators are referenced. Now that the
        // analyzer collects references (CLOC13.0.1), both
        // bindings have use_count = 1 and the apply step's
        // passthrough path keeps the whole VariableDeclaration
        // verbatim.
        let stmt_a = ProgramItem::Statement(Statement::expression_statement(
            ExpressionStatement {
                cv: None,
                expression: Expression::Identifier(Identifier {
                    cv: None,
                    name: "a".to_string(),
                }),
            },
        ));
        let stmt_b = ProgramItem::Statement(Statement::expression_statement(
            ExpressionStatement {
                cv: None,
                expression: Expression::Identifier(Identifier {
                    cv: None,
                    name: "b".to_string(),
                }),
            },
        ));
        let prog = program_with(vec![
            var_decl(VarKind::Const, &[("a", Some(1.0)), ("b", Some(2.0))]),
            stmt_a,
            stmt_b,
        ]);
        let out = run_pass(prog.clone());
        assert!(!out.changed);
        assert_eq!(out.program.body, prog.body);
    }

    #[test]
    fn apply_step_changed_is_false_when_all_used() {
        // Pin the invariant: `changed` is true iff at least one
        // declarator was removed. With every binding referenced
        // (via top-level ExpressionStatements naming each), the
        // apply step's eligibility scan finds zero dead, no
        // removal happens, and `changed == false`.
        //
        // `f` is a Function-kind binding; it's filtered out at
        // step 2 regardless (functions are treeshake's job).
        // We still need to reference `x` and `k` to keep them
        // alive under the now-active analyzer.
        let stmt_x = ProgramItem::Statement(Statement::expression_statement(
            ExpressionStatement {
                cv: None,
                expression: Expression::Identifier(Identifier {
                    cv: None,
                    name: "x".to_string(),
                }),
            },
        ));
        let stmt_k = ProgramItem::Statement(Statement::expression_statement(
            ExpressionStatement {
                cv: None,
                expression: Expression::Identifier(Identifier {
                    cv: None,
                    name: "k".to_string(),
                }),
            },
        ));
        let prog = program_with(vec![
            var_decl(VarKind::Let, &[("x", None)]),
            fn_decl("f"),
            var_decl(VarKind::Const, &[("k", Some(7.0))]),
            stmt_x,
            stmt_k,
        ]);
        let out = run_pass(prog.clone());
        assert!(!out.changed);
        assert_eq!(out.program.body, prog.body);
    }

    // ===================================================================
    // Removal — the apply step actually deletes dead bindings.
    //
    // Until CLOC13.E.2 these paths had ZERO coverage: every prior test
    // asserted `!out.changed`, and the lone removal path only matched
    // the bare `ProgramItem::Declaration` form — which the bridge never
    // emits — so the pass was a silent no-op on real programs. These
    // tests pin actual removal in BOTH item shapes plus the purity gate.
    // ===================================================================

    #[test]
    fn removes_unused_top_level_var_statement_form() {
        // `var unused = 1;` with no references, in the Statement-wrapped
        // shape the bridge emits. The whole declaration must disappear.
        let prog = program_with(vec![var_stmt(VarKind::Var, &[("unused", Some(1.0))])]);
        let out = run_pass(prog);
        assert!(out.changed, "an unused top-level var must be removed");
        assert!(
            out.program.body.is_empty(),
            "the dead declaration should be gone, got: {:?}",
            out.program.body
        );
    }

    #[test]
    fn removes_unused_top_level_var_bare_declaration_form() {
        // Same, but the bare `ProgramItem::Declaration` shape. Both
        // shapes route through `prune_var_decl`.
        let prog = program_with(vec![var_decl(VarKind::Const, &[("dead", Some(2.0))])]);
        let out = run_pass(prog);
        assert!(out.changed);
        assert!(out.program.body.is_empty());
    }

    #[test]
    fn keeps_used_var_statement_form() {
        // `var keep = 1; keep;` — referenced, so nothing is removed.
        let prog = program_with(vec![
            var_stmt(VarKind::Var, &[("keep", Some(1.0))]),
            use_stmt("keep"),
        ]);
        let out = run_pass(prog.clone());
        assert!(!out.changed, "a referenced var must be kept");
        assert_eq!(out.program.body, prog.body);
    }

    #[test]
    fn splits_multi_declarator_dropping_only_dead() {
        // `var a = 1, unused = 2; a;` — `a` is referenced, `unused` is
        // not. The surviving declaration keeps only `a`.
        let prog = program_with(vec![
            var_stmt(VarKind::Var, &[("a", Some(1.0)), ("unused", Some(2.0))]),
            use_stmt("a"),
        ]);
        let out = run_pass(prog);
        assert!(out.changed, "the dead declarator must be dropped");
        // First item is the pruned declaration with one survivor.
        let ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(vd))) =
            &out.program.body[0]
        else {
            panic!("expected a surviving var statement, got: {:?}", out.program.body[0])
        };
        assert_eq!(vd.declarations.len(), 1, "only `a` should survive");
        let BindingTarget::Identifier(id) = &vd.declarations[0].id;
        assert_eq!(id.name, "a");
    }

    #[test]
    fn keeps_unused_var_with_impure_initializer() {
        // `let unused = sideEffect();` — unreferenced, BUT the
        // initializer is a call, which may have side effects. The
        // purity gate must keep the declarator so the call still runs.
        // (`sideEffect` itself is a free global — an unresolved
        // reference, never our binding — so it doesn't keep `unused`
        // alive; only the purity gate does.)
        let prog = program_with(vec![var_stmt_call_init("unused", "sideEffect")]);
        let out = run_pass(prog.clone());
        assert!(
            !out.changed,
            "a dead binding with a side-effecting initializer must be kept"
        );
        assert_eq!(out.program.body, prog.body);
    }

    #[test]
    fn is_removable_init_classifies_purity() {
        // Direct unit coverage of the purity gate.
        assert!(is_removable_init(&None), "absent init is removable");
        assert!(
            is_removable_init(&Some(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 1.0,
                raw: "1".to_string(),
            }))),
            "literal init is removable"
        );
        assert!(
            is_removable_init(&Some(Expression::Identifier(ident("y")))),
            "bare identifier init is removable (pure read)"
        );
        assert!(
            !is_removable_init(&Some(Expression::CallExpression(CallExpression {
                cv: None,
                callee: Box::new(Expression::Identifier(ident("f"))),
                arguments: Vec::new(),
            }))),
            "call init is NOT removable (may have side effects)"
        );
    }
}
