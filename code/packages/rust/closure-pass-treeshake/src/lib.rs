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
//! # Scope
//!
//! `TreeshakePass::run` is a real transform: it uses
//! `closure-scope-analyzer` to find top-level (`ScopeId::GLOBAL`)
//! `function`/`class` bindings that no [`Reference`] resolves to, and
//! deletes their declarations. This is the function/class-shaped
//! complement to `remove-unused-vars` (which handles `var`/`let`/`const`
//! and skips functions). Removing an unused function/class declaration is
//! unconditionally safe — declaring one has no side effect — so unlike
//! `remove-unused-vars` there is no initializer-purity gate.
//!
//! Current slice: top-level bindings only, removed by name against the
//! `Declaration::FunctionDeclaration` items in `program.body`. The full
//! cross-module import/export reachability walk lands once
//! `javascript-ast` grows `ImportDeclaration` / `ExportDeclaration`
//! variants and a host root-set; until then a function is "reachable" iff
//! some in-module reference resolves to it.
//!
//! What this pass locks down:
//!
//! 1. Pass metadata (`name`, `iteration_policy`, `cost`,
//!    `depends_on`) — what the scheduler keys on and what the
//!    `closurec` CLI surfaces as `--disable=treeshake`.
//! 2. The `depends_on("dce")` edge so the scheduler forces
//!    intra-module DCE before shaking.
//! 3. Removal of dead top-level function declarations, plus the
//!    DCE-ordering integration test.

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use std::collections::{HashMap, HashSet};

use coding_adventures_closure_scope_analyzer::{analyze, BindingId, BindingKind, ScopeId};
use coding_adventures_correlation_vector::Contribution;
use coding_adventures_javascript_ast::{Declaration, ProgramItem};
use serde_json::json;

/// `Pass::depends_on` value. CLOC06 canonical order pins DCE
/// before tree-shake. Kept as a `const` so future tests / sibling
/// crates can reference the dependency name without retyping.
const DEPS: &[&str] = &["dce"];

/// Tree-shaking pass — deletes top-level `function`/`class`
/// declarations nothing references. See crate-level docs.
///
/// Zero-sized type: no per-instance state. Pass-internal state (the
/// binding → reference-count map and the dead-name set) lives in
/// pass-local maps constructed inside [`Pass::run`] per CLOC06
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
        // Consume the shared `ScopeAnalysis` from
        // `closure-scope-analyzer` and delete top-level `Function`/
        // `Class` declarations that nothing references. The algorithm:
        //
        //   1. Use-count per binding (scan `analysis.references`).
        //   2. Dead-shape scan: a binding is removable when its
        //      use-count is 0, its scope is `ScopeId::GLOBAL`, and its
        //      kind is `Function` or `Class`. (`Var`/`Let`/`Const` are
        //      remove-unused-vars' / collapse-properties' job — the
        //      split is kind-based.)
        //   3. Apply: walk `program.body` and drop the matching
        //      `Declaration::FunctionDeclaration`s.
        //
        // `changed` is `true` iff at least one declaration was dropped.
        // Under `IterationPolicy::FixedPoint` this is sound because each
        // iteration strictly shrinks the binding set (a removed function
        // mints no new bindings), so it converges — see the apply-step
        // comment below.
        //
        // Note: a `Function`/`Class` declaration has no evaluation side
        // effect, so removing an unreferenced one is unconditionally
        // safe — no initializer-purity gate (unlike remove-unused-vars).

        let analysis = analyze(ctx.program);

        // Step 1: use-count per binding (same shape as
        // remove-unused-vars / collapse-properties / inline).
        // Unresolved references (binding = None) — free globals
        // like `console` — don't increment any count.
        let mut use_count: Vec<usize> = vec![0; analysis.bindings.len()];
        for reference in &analysis.references {
            if let Some(BindingId(idx)) = reference.binding {
                if let Some(slot) = use_count.get_mut(idx as usize) {
                    *slot += 1;
                }
            }
        }

        // Step 2: dead-shape scan. A binding is *dead* (removable
        // by treeshake) when ALL of:
        //   - kind ∈ { Function, Class } (the module-export-shaped
        //     kinds; Var/Let/Const cross over to remove-unused-vars
        //     and collapse-properties)
        //   - uses == 0 (no reference resolves to it)
        //   - scope == ScopeId::GLOBAL (the only scope CLOC13.0
        //     populates today; restricting here keeps the apply
        //     step correct when CLOC13.0.2 introduces nested
        //     scopes, since we only act on top-level names below)
        //
        // `#[non_exhaustive]` on BindingKind: future variants
        // conservatively passthrough via wildcard.
        let mut dead_names: HashSet<String> = HashSet::new();
        for (idx, binding) in analysis.bindings.iter().enumerate() {
            let uses = use_count.get(idx).copied().unwrap_or(0);
            if uses != 0 {
                continue;
            }
            if binding.scope != ScopeId::GLOBAL {
                continue;
            }
            match binding.kind {
                BindingKind::Function | BindingKind::Class => {
                    dead_names.insert(binding.name.clone());
                }
                _ => {}
            }
        }

        // Step 3 — APPLY. CLOC13.C.1 lifts the hard-pin on
        // `changed` and actually mutates the program.
        //
        // Walk program.body. For each item:
        //   - Declaration::FunctionDeclaration: drop if its name
        //     is in dead_names; passthrough otherwise.
        //   - Declaration::VariableDeclaration: passthrough
        //     unconditionally (Var/Let/Const are not treeshake's
        //     responsibility — remove-unused-vars handles them).
        //   - ProgramItem::Statement: passthrough. Statement
        //     walking lands as references arrive (CLOC13.0.1).
        //
        // **Pin lifted; safety preserved.** `changed = removed >
        // 0` is safe because we genuinely mutated when we say we
        // did. If `removed == 0` (no dead Function/Class bindings
        // in GLOBAL), the program passes through unchanged with
        // `changed = false`.
        //
        // **Why this stays safe under FixedPoint.** Each iteration
        // strictly reduces the binding set: a removed
        // FunctionDeclaration produces no new bindings, and the
        // dropped function body can't introduce *new* references
        // because those references were already inside the dead
        // function and resolved either (a) to other dead bindings
        // (removed in this same iteration) or (b) to live
        // bindings (whose use_count was incremented by those refs;
        // removing the refs decrements the count, possibly making
        // *those* bindings newly-dead in the next iteration). The
        // fixed point reaches when no Function/Class binding has
        // zero refs.

        let mut new_body: Vec<ProgramItem> = Vec::with_capacity(ctx.program.body.len());
        let mut removed_count: usize = 0;
        // Capture each removed function's own CV id (and name) BEFORE it
        // is dropped, so we can tombstone the exact span that vanished.
        let mut removed: Vec<(Option<String>, String)> = Vec::new();
        for item in &ctx.program.body {
            match item {
                ProgramItem::Declaration(Declaration::FunctionDeclaration(fd))
                    if dead_names.contains(&fd.id.name) =>
                {
                    removed_count += 1;
                    removed.push((fd.cv.clone(), fd.id.name.clone()));
                    // Drop.
                }
                _ => new_body.push(item.clone()),
            }
        }

        // Deletion provenance (#89). Treeshake is the whole-program
        // analogue of DCE: it deletes unreferenced top-level functions.
        // Like DCE, it must not delete code silently — each removed
        // function's own CV entry is tombstoned with a `DeletionRecord`
        // (via `CVLog::delete`), so a `--correlation_vector` consumer
        // asking "what happened to `function foo`?" gets a definite
        // answer: *treeshake removed it because it was unexported /
        // unreferenced.* `delete` is a no-op when the log is disabled
        // (production default), so this costs nothing off that path. We
        // also emit one summary `Contribution` against the program root.
        let mut contributions: Vec<Contribution> = Vec::new();
        if removed_count > 0 {
            for (cv_id, name) in &removed {
                if let Some(id) = cv_id {
                    let mut meta: HashMap<String, serde_json::Value> = HashMap::new();
                    meta.insert("name".to_string(), json!(name));
                    ctx.cv
                        .delete(id, "treeshake", "removed-unreferenced-function", meta);
                }
            }
            if let Some(prog_cv) = &ctx.program.cv {
                contributions.push(Contribution {
                    source: "treeshake".to_string(),
                    tag: "removed-unreferenced-function".to_string(),
                    meta: [
                        ("removed".to_string(), json!(removed_count)),
                        ("parent_cv".to_string(), json!(prog_cv)),
                    ]
                    .into_iter()
                    .collect(),
                });
            }
        }

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
    //! Tests pin the public contract: pass metadata, the DCE-ordering
    //! integration with `PassPipeline`, and the removal behavior itself
    //! — actual deletion of unreferenced top-level function
    //! declarations, with referenced functions and `var`/statements
    //! passed through.
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
        let _a: TreeshakePass = Default::default();
        let _b: TreeshakePass = TreeshakePass::new();
        let _c = _b;
        let _d = _c;
    }

    // -----------------------------------------------------------------
    // CLOC13.C.1 — apply-step tests.
    //
    // Treeshake drops top-level FunctionDeclarations whose names are
    // in the dead set (Function/Class kind, scope=GLOBAL, uses=0).
    // These tests exercise the body-walk machinery directly via the
    // Pass trait, not through the pipeline.
    // -----------------------------------------------------------------

    use coding_adventures_javascript_ast::{
        BindingTarget, BlockStatement, Expression, ExpressionStatement, FunctionDeclaration,
        Identifier, NumericLiteral, Statement, VarKind, VariableDeclaration,
        VariableDeclarator,
    };

    fn ident(name: &str) -> Identifier {
        Identifier {
            cv: None,
            name: name.to_string(),
        }
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

    fn var_decl_simple(name: &str) -> ProgramItem {
        ProgramItem::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind: VarKind::Let,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(ident(name)),
                init: None,
            }],
        }))
    }

    fn program_with(items: Vec<ProgramItem>) -> Program {
        let mut p = program();
        p.body = items;
        p
    }

    // -----------------------------------------------------------------
    // CV deletion provenance (#89).
    //
    // Mirror the pipeline: the lexer/parser `create` a CV entry per node
    // and stamp its id onto the AST. So we `create` the function's entry
    // FIRST, stamp its id, then run treeshake — otherwise `cv.delete`
    // has no entry to tombstone and the assertion would be vacuous.
    // Property: a treeshake-removed function's CV entry survives with a
    // `DeletionRecord{source:"treeshake"}`, so "what happened to
    // `function foo`?" stays answerable.
    // -----------------------------------------------------------------

    /// A `function <name>(){}` whose CV id is freshly created in `log`.
    fn traced_fn_decl(log: &mut CVLog, name: &str) -> (ProgramItem, String) {
        let id = log.create(None);
        let item = ProgramItem::Declaration(Declaration::FunctionDeclaration(FunctionDeclaration {
            cv: Some(id.clone()),
            id: ident(name),
            params: Vec::new(),
            body: BlockStatement {
                cv: None,
                body: Vec::new(),
            },
            generator: false,
            is_async: false,
        }));
        (item, id)
    }

    /// Like [`run_pass`] but threads the caller's CV log through so its
    /// `DeletionRecord`s can be inspected after the pass returns.
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
        TreeshakePass::new().run(ctx).expect("pass ran")
    }

    #[test]
    fn removed_function_is_tombstoned() {
        let mut log = CVLog::new(true);
        let (dead, dead_cv) = traced_fn_decl(&mut log, "dead");
        let prog = program_with(vec![dead]);

        let out = run_capturing_cv(&prog, &mut log);

        assert!(out.changed);
        assert!(out.program.body.is_empty());
        let del = log
            .get(&dead_cv)
            .unwrap()
            .deleted
            .as_ref()
            .expect("a removed function must be tombstoned");
        assert_eq!(del.source, "treeshake");
        assert_eq!(del.reason, "removed-unreferenced-function");
        assert_eq!(del.meta.get("name").and_then(|v| v.as_str()), Some("dead"));
    }

    #[test]
    fn each_removed_function_is_tombstoned() {
        let mut log = CVLog::new(true);
        let (f, f_cv) = traced_fn_decl(&mut log, "f");
        let (g, g_cv) = traced_fn_decl(&mut log, "g");
        let prog = program_with(vec![f, g]);

        let out = run_capturing_cv(&prog, &mut log);

        assert!(out.changed);
        assert!(log.get(&f_cv).unwrap().deleted.is_some());
        assert!(log.get(&g_cv).unwrap().deleted.is_some());
    }

    #[test]
    fn referenced_function_is_not_tombstoned() {
        // `function f(){} f();` — the call keeps f live, so it is
        // neither removed nor tombstoned.
        use coding_adventures_javascript_ast::{CallExpression, Expression};
        let mut log = CVLog::new(true);
        let (f, f_cv) = traced_fn_decl(&mut log, "f");
        let call_f = ProgramItem::Statement(Statement::expression_statement(ExpressionStatement {
            cv: None,
            expression: Expression::CallExpression(CallExpression {
                cv: None,
                callee: Box::new(Expression::Identifier(ident("f"))),
                arguments: Vec::new(),
            }),
        }));
        let prog = program_with(vec![f, call_f]);

        let out = run_capturing_cv(&prog, &mut log);

        assert!(!out.changed, "a referenced function must survive");
        assert!(
            log.get(&f_cv).unwrap().deleted.is_none(),
            "a surviving function must NOT be tombstoned"
        );
    }

    #[test]
    fn disabled_log_still_removes_without_panicking() {
        // With CV disabled, `delete` is a no-op; the pass must still
        // drop the dead function and never panic on the missing entry.
        let mut log = CVLog::new(false);
        let (dead, _cv) = traced_fn_decl(&mut log, "dead");
        let prog = program_with(vec![dead]);

        let out = run_capturing_cv(&prog, &mut log);

        assert!(out.changed);
        assert!(out.program.body.is_empty());
    }

    fn run_pass(prog: Program) -> coding_adventures_closure_pass_pipeline::PassOutput {
        let pass = TreeshakePass::new();
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
    fn apply_step_drops_unreferenced_function() {
        // `function f() {}` with no callers anywhere → f is dead
        // → apply step drops it. Works under both pre- and post-
        // CLOC13.0.1 analyzer (no `f()` call site means use_count=0
        // either way).
        let prog = program_with(vec![fn_decl("f")]);
        let out = run_pass(prog);
        assert!(out.changed, "unreferenced function should be removed");
        assert!(out.program.body.is_empty(), "no items survive");
    }

    #[test]
    fn apply_step_drops_multiple_unreferenced_functions() {
        // `function f() {} function g() {} function h() {}` — all
        // dropped. Pin that the loop processes every item.
        let prog = program_with(vec![fn_decl("f"), fn_decl("g"), fn_decl("h")]);
        let out = run_pass(prog);
        assert!(out.changed);
        assert!(out.program.body.is_empty());
    }

    #[test]
    fn apply_step_passes_var_declarations_through() {
        // `let x;` — treeshake does NOT remove Var/Let/Const,
        // even when use_count == 0. That's remove-unused-vars'
        // job. Passthrough should leave the item intact.
        let prog = program_with(vec![var_decl_simple("x")]);
        let out = run_pass(prog.clone());
        assert!(!out.changed, "Let-kind binding is not treeshake's target");
        assert_eq!(out.program.body, prog.body);
    }

    #[test]
    fn apply_step_passes_statements_through() {
        // Top-level ExpressionStatements are not declarations and
        // never enter the dead-set; passthrough unconditionally.
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
        let out = run_pass(prog.clone());
        assert!(!out.changed);
        assert_eq!(out.program.body, prog.body);
    }

    #[test]
    fn apply_step_mixed_program_drops_only_functions() {
        // `let x; function f() {} let y;` — only `f` is dropped;
        // the `let` declarations pass through. Pins that the apply
        // step's `match` arm correctly partitions Function decls
        // from other items.
        //
        // Under empty analyzer references (pre-CLOC13.0.1), use
        // counts are all 0; only `f` enters the dead set because of
        // the BindingKind::Function | Class filter at step 2.
        let prog = program_with(vec![
            var_decl_simple("x"),
            fn_decl("f"),
            var_decl_simple("y"),
        ]);
        let out = run_pass(prog);
        assert!(out.changed);
        assert_eq!(out.program.body.len(), 2);
        // Surviving items are the two var declarations, in order.
        match &out.program.body[0] {
            ProgramItem::Declaration(Declaration::VariableDeclaration(vd)) => {
                let BindingTarget::Identifier(id) = &vd.declarations[0].id;
                assert_eq!(id.name, "x");
            }
            _ => panic!("expected the let x declaration to survive at index 0"),
        }
        match &out.program.body[1] {
            ProgramItem::Declaration(Declaration::VariableDeclaration(vd)) => {
                let BindingTarget::Identifier(id) = &vd.declarations[0].id;
                assert_eq!(id.name, "y");
            }
            _ => panic!("expected the let y declaration to survive at index 1"),
        }
    }

    #[test]
    fn apply_step_empty_program_no_change() {
        // Pin the invariant: empty program → no work → changed=false.
        let prog = program();
        let out = run_pass(prog.clone());
        assert!(!out.changed);
        assert_eq!(out.program.body, prog.body);
    }

    #[test]
    fn apply_step_keeps_function_when_called() {
        // `function f() {} f();` — the function is referenced by
        // the top-level `f()` call site, so it must NOT be dropped.
        //
        // This is the retention test that was intentionally
        // deferred from CLOC13.C.1 (PR #4803). At the time, the
        // analyzer (CLOC13.0.1 / PR #4800) only collected
        // references inside top-level ExpressionStatements; the
        // `f()` call WAS in such a statement, so the reference
        // would have been emitted with `from_scope = GLOBAL` —
        // but the lookup would have correctly resolved `f` to
        // the function binding either way.
        //
        // With CLOC13.0.2 (PR #4825) on main, the analyzer now
        // walks function bodies under nested Function scopes too,
        // and reference resolution walks the parent chain. This
        // test exercises the simpler top-level-callee case, and
        // pins the retention contract: use_count[f] == 1 ⇒
        // treeshake skips ⇒ program unchanged ⇒ changed == false.
        use coding_adventures_javascript_ast::{
            CallExpression, Expression, ExpressionStatement, Identifier, Statement,
        };
        let call_f = ProgramItem::Statement(Statement::expression_statement(
            ExpressionStatement {
                cv: None,
                expression: Expression::CallExpression(CallExpression {
                    cv: None,
                    callee: Box::new(Expression::Identifier(Identifier {
                        cv: None,
                        name: "f".to_string(),
                    })),
                    arguments: Vec::new(),
                }),
            },
        ));
        let prog = program_with(vec![fn_decl("f"), call_f]);
        let out = run_pass(prog.clone());
        assert!(!out.changed, "f is referenced by the call site");
        assert_eq!(out.program.body.len(), 2);
        // The function declaration survives at index 0.
        match &out.program.body[0] {
            ProgramItem::Declaration(Declaration::FunctionDeclaration(fd)) => {
                assert_eq!(fd.id.name, "f");
            }
            _ => panic!("expected the function declaration to survive at index 0"),
        }
    }
}
