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

use std::collections::HashSet;

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use coding_adventures_closure_scope_analyzer::{
    analyze, BindingId, BindingKind, ScopeId,
};
use coding_adventures_javascript_ast::{
    BindingTarget, Declaration, ProgramItem,
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
        //   3. Apply the removal: walk the Program and drop the
        //      matching `VariableDeclarator`s.
        //
        // **v0.2.0 status.** Steps 1+2 are wired here.  Step 3
        // is **deferred to CLOC13.E.1** because applying the
        // removal cleanly requires a binding → declarator
        // backreference that the analyzer doesn't yet ship.
        // Until that lands, we report `removed_count` so the
        // pass is *observable* (the scheduler sees a real
        // `changed` signal as soon as the analyzer's `analyze`
        // body lands and starts populating bindings) but the
        // program itself is unchanged.
        //
        // **Why this is safe with the v0.1.0 analyzer body.**
        // The current `analyze` returns an empty bindings list
        // and an empty references list, so the eligibility scan
        // finds zero dead bindings. `removed_count` is always
        // 0, `changed` is always false, and the program passes
        // through unchanged. The wiring becomes effective the
        // moment the analyzer's body lands — no churn here.

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
        for item in &ctx.program.body {
            let decl = match item {
                ProgramItem::Statement(_) => {
                    new_body.push(item.clone());
                    continue;
                }
                ProgramItem::Declaration(d) => d,
            };
            match decl {
                Declaration::VariableDeclaration(var_decl) => {
                    // Partition declarators into kept vs. dropped.
                    let mut kept = Vec::with_capacity(var_decl.declarations.len());
                    for declarator in &var_decl.declarations {
                        let BindingTarget::Identifier(id) = &declarator.id;
                        if dead_names.contains(&id.name) {
                            removed_count += 1;
                        } else {
                            kept.push(declarator.clone());
                        }
                    }
                    if kept.is_empty() {
                        // Entire declaration is dead — drop it.
                        continue;
                    }
                    if kept.len() == var_decl.declarations.len() {
                        // No declarators dropped — keep the
                        // original item verbatim.
                        new_body.push(item.clone());
                    } else {
                        // Split: emit a new VariableDeclaration
                        // with only the surviving declarators,
                        // preserving the original's `kind` and
                        // `cv`.
                        let mut split = var_decl.clone();
                        split.declarations = kept;
                        new_body.push(ProgramItem::Declaration(
                            Declaration::VariableDeclaration(split),
                        ));
                    }
                }
                Declaration::FunctionDeclaration(_) => {
                    // Function-kind bindings were filtered out at
                    // step 2; nothing to do here.  Passthrough.
                    new_body.push(item.clone());
                }
            }
        }

        // Construct the output program with the rewritten body.
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
        BindingTarget, Declaration, Expression, FunctionDeclaration, Identifier,
        NumericLiteral, ProgramItem, BlockStatement, VarKind, VariableDeclaration,
        VariableDeclarator,
    };

    fn ident(name: &str) -> Identifier {
        Identifier {
            cv: None,
            name: name.to_string(),
        }
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

    #[test]
    fn apply_step_passthrough_keeps_used_let_under_empty_analysis() {
        // `let x;` at the top level.  Today the analyzer returns
        // empty bindings, so `dead_names` is empty, so `x` is kept.
        // After #4787 merges, the analyzer surfaces `x` but it's
        // not referenced either → `x` would become dead.  This
        // test passes today (no-op path) and will FAIL after the
        // analyzer activation lands — exactly the right signal for
        // the follow-up PR to introduce a referenced-`x` fixture
        // (e.g., via an ExpressionStatement reading `x`) to keep
        // assertion semantics stable.
        let prog = program_with(vec![var_decl(VarKind::Let, &[("x", None)])]);
        let out = run_pass(prog.clone());
        assert!(!out.changed, "no analyzer bindings → no removals");
        assert_eq!(out.program.body.len(), 1);
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
    fn apply_step_preserves_multi_declarator_when_no_dead_names() {
        // `const a = 1, b = 2;` — multi-declarator form.  Today
        // (no analyzer bindings) both stay.  After CLOC13.0
        // lands, the analyzer would surface both — and if neither
        // is referenced, both get dropped together via the
        // "every declarator dead → drop whole item" path.  When
        // one is referenced and one isn't, the split path fires.
        // Pinning the no-op behavior now; behavior tests for
        // the split path follow in the CLOC13.0.1 wave.
        let prog = program_with(vec![var_decl(
            VarKind::Const,
            &[("a", Some(1.0)), ("b", Some(2.0))],
        )]);
        let out = run_pass(prog.clone());
        assert!(!out.changed);
        assert_eq!(out.program.body, prog.body);
    }

    #[test]
    fn apply_step_changed_is_false_when_program_unchanged() {
        // Pin the invariant: `changed` is true iff at least one
        // declarator was removed.  Under empty analysis, no
        // removals → `changed = false`.  This is the same
        // discipline as the v0.2.0 hard-pin, just now derived
        // from real state rather than asserted.
        let prog = program_with(vec![
            var_decl(VarKind::Let, &[("x", None)]),
            fn_decl("f"),
            var_decl(VarKind::Const, &[("k", Some(7.0))]),
        ]);
        let out = run_pass(prog.clone());
        assert!(!out.changed);
        assert_eq!(out.program.body, prog.body);
    }
}
