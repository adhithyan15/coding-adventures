# Changelog

All notable changes to the `coding-adventures-closure-pass-treeshake` crate will be documented in this file.

## [0.3.2] - 2026-06-16

### Docs

De-staled the module header, struct doc, `run` body comment, and test-module
doc. They still claimed "v1 is identity" / "sweep deferred to CLOC13.C.1" /
"`changed` is hard-pinned to false" / "analyzer returns empty bindings" — none
of which has been true since the apply step and the scope-analyzer body landed.
`TreeshakePass::run` already deletes unreferenced top-level `function`/`class`
declarations (`changed = removed_count > 0`), as the existing
`apply_step_drops_unreferenced_function` test asserts. No code change — this
crate is now wired into closurec's SIMPLE pipeline, so its docs needed to stop
describing it as a no-op.

## [0.3.1] - 2026-06-02

### Added — retention test (paired with CLOC13.0.2 activation)

A single new test: `apply_step_keeps_function_when_called`. Fixture: `function f() {} f();`. Now that PR #4825 (CLOC13.0.2 nested scopes) is on main, the analyzer walks reference sites both at the top level AND inside function bodies under nested Function scopes. The `f()` callee Identifier emits a Reference resolving to the function binding, so `use_count[f] = 1` and the apply step's dead-shape scan correctly skips `f`.

This test was intentionally deferred from CLOC13.C.1 (PR #4803) because at that time:
- The analyzer body (CLOC13.0 / PR #4787) populated bindings but not references.
- Reference activation (CLOC13.0.1 / PR #4800) was about to land but #4803 was forked from earlier main.
- Including the retention test in #4803 would have made it fail pre-#4800 and pass post-#4800 — fragile across PR sequencing.

Bundling it now as a tiny follow-up keeps the test surface complete without coupling to a mid-flight rebase.

No code changes to the pass body — just test coverage. No version bump beyond the patch level (`0.3.0` → `0.3.1`).

## [0.3.0] - 2026-06-02

### Added (CLOC13.C.1 — the apply step, `changed` unpinned)

Second apply step in the CLOC13 family (after CLOC13.E.1 / PR #4790). Walks `ctx.program.body` and actually drops dead `FunctionDeclaration` items.

Strategy:

1. **Use-count.** Per-binding scan over `analysis.references`, same shape as CLOC13.E.1. Unresolved references (free globals) don't increment any count.
2. **Dead-shape scan.** A binding is dead when ALL of: `kind ∈ { Function, Class }`, `uses == 0`, `scope == ScopeId::GLOBAL`. Restricting to GLOBAL keeps the apply step correct under future analyzer extensions that surface nested bindings — only top-level names get acted on here.
3. **Walk + drop.** For each `ProgramItem`:
   - `Declaration::FunctionDeclaration`: drop if name ∈ `dead_names`; passthrough otherwise.
   - `Declaration::VariableDeclaration`: passthrough unconditionally. Var/Let/Const aren't treeshake's responsibility — `remove-unused-vars` owns them.
   - `ProgramItem::Statement`: passthrough.
4. `changed = removed_count > 0`.

### Hard-pin lifted; safety preserved

`changed` is now derived from `removed_count`. Safe because we genuinely mutate when we report it: zero removals → `changed = false` (identical to v0.2.0 behavior), at-least-one removal → `changed = true` and the program is actually different.

**Why it stays safe under `IterationPolicy::FixedPoint`.** Each iteration strictly reduces the binding set. A removed `FunctionDeclaration` produces no new bindings; the dropped function body can't introduce new references either (those refs were inside the dead function and resolved either to other dead bindings — removed in the same iteration — or to live bindings whose use_count was incremented by those refs; removing the refs decrements the count, possibly making *those* newly-dead in the next iteration). The fixed point reaches when no Function/Class binding has zero refs.

### Cross-PR interaction with #4800 (CLOC13.0.1)

This PR is **forked from origin/main BEFORE #4800 merges.** That means the in-tree scope-analyzer has populated bindings (CLOC13.0) but empty references (CLOC13.0.1 not yet on main).

Under empty-references the apply step still works correctly — bindings with zero refs are dead. The tests in this PR are designed to work both pre- and post-#4800:

- *Drops unreferenced function* tests use fixtures with NO `f()` call site, so use_count is 0 either way → consistent result.
- *Passthrough Var/Let/Const* tests use Let/Const declarations, which are filtered out at step 2 regardless of references.
- *Passthrough Statement* tests use literal-only `ExpressionStatement` (no Identifier expressions to walk).
- *Empty program* test is the canonical identity case.

The "function with use" retention test (`function f() {} f();` should keep `f`) is intentionally NOT included here — it'd pass post-#4800 but fail pre-#4800 because the empty-refs state makes `f` look dead. That test belongs to the follow-up paired with the CLOC13.0.1 activation.

### Tests added (6 new; 15 total, was 9)

- `apply_step_drops_unreferenced_function`
- `apply_step_drops_multiple_unreferenced_functions`
- `apply_step_passes_var_declarations_through`
- `apply_step_passes_statements_through`
- `apply_step_mixed_program_drops_only_functions`
- `apply_step_empty_program_no_change`

All 9 v0.2.0 tests still pass unchanged.

### Bumped 0.2.0 → 0.3.0

`Pass::run` API unchanged. Behavior under empty analysis is unchanged. Version bump signals the pass now mutates the program when there's dead work to do.

## [0.2.0] - 2026-06-01

### Added (CLOC13.C — consume `closure-scope-analyzer`)
- Wired the pass to `coding-adventures-closure-scope-analyzer` (new `[dependencies]` entry). `run` now invokes `analyze(ctx.program)` and walks the returned `ScopeAnalysis` to identify **module-shape candidates** — bindings whose kind (`Function` / `Class`) is the shape that becomes a module export once `javascript-ast` grows `ImportDeclaration` / `ExportDeclaration` variants.
- Algorithm (mark phase; sweep deferred to CLOC13.C.1):
  1. Walk `analysis.bindings`. A binding is a *module-shape candidate* when its `kind` is `BindingKind::Function` or `BindingKind::Class`. These are the only shapes ESM allows as named top-level exports. `Var`/`Let`/`Const` *can* be exported but cross over to remove-unused-vars and collapse-properties; the cleanest split is kind-based.
  2. Track candidates in `Vec<BindingId>` for the observability path.
  3. *Sweep deferred*: removing a function/class binding cleanly requires the AST to grow `ImportDeclaration` / `ExportDeclaration` nodes (otherwise treeshake can't distinguish an exported function from an internal one).
- `PassStats::nodes_touched` now reports `1 + bindings.len() + references.len()` (root + every binding + every reference visited). Real cost surfacing for the scheduler instead of the v0.1.0 placeholder `1`.

### Critical safety pin (lesson from CLOC13.E security review)
- `changed` is **hard-pinned to `false`** until step 3 (the actual program mutation) lands. The pass identifies candidates and returns the program *unchanged*. Reporting `changed = true` while returning an unchanged program would cause the scheduler under `IterationPolicy::FixedPoint` to re-run forever — each iteration would find the same candidates, claim a change, return the same program, repeat. Documented in both the source (`fn run`) and here so the next contributor doesn't reintroduce the bug. Pinning lifts only when step 3 actually mutates the program.

### Why this is safe to merge ahead of the analyzer body
- The current `closure-scope-analyzer` v0.1.0 returns empty `bindings` + `references`. The candidate scan therefore produces zero shapes, the candidates vec stays empty, `nodes_touched` is small, and the program passes through unchanged — identical observable behavior to v0.1.0. The wiring becomes **effective** the moment CLOC13.0 lands the analyzer body — no churn here, no rebase needed.

### Dependencies
- Added `coding-adventures-closure-scope-analyzer = { path = "../closure-scope-analyzer" }` to `[dependencies]`. Crate bumped `0.1.0 → 0.2.0`.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 canonical pass set — tree-shaking. Removes `export` declarations and `import` bindings that aren't reachable from any entry point. The whole-program cousin of DCE.
- `TreeshakePass` zero-sized type implementing `Pass`:
  - `name = "treeshake"`
  - `depends_on = &["dce"]` — DCE shrinks intra-module use-sets first, simplifying the cross-module use-chain analysis tree-shake needs.
  - `iteration_policy = IterationPolicy::FixedPoint` — tree-shake removing an export can make its `import` dead, which DCE can delete, which can leave whole modules unreached, which tree-shake can then remove. Real cascade.
  - `cost = 3` pass-units — cross-module mark + sweep, same shape as DCE.
  - `invalidates()` empty in v1 (informational only per CLOC06 Open Question 1).
- `TreeshakePass::new()` zero-arg constructor.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there are no `ImportDeclaration` / `ExportDeclaration` nodes to shake. Pass through unchanged, `changed = false`, `nodes_touched = 1`, no contributions emitted (per CLOC03 §"When a pass keeps a node unchanged").
- 9 tests covering: `name()` value, `iteration_policy == FixedPoint`, `cost == 3`, `depends_on == ["dce"]`, `invalidates` empty, identity run, **two-pass pipeline orders dce before treeshake** even when registered in reverse, solo pipeline run (with the v0.1.0 `pipeline.fixed-point-not-yet-iterated` diagnostic asserted), `Default` + `Clone` impls.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline`, `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar` (future tree-shake reads module-level `external` attributes to seed root set), `coding_adventures_correlation_vector` (per-removal `Contribution` per CLOC03), `serde_json`. Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`, `coding-adventures-closure-pass-dce` for the two-pass ordering integration test.
- v1 is scaffolding. The full mark-and-sweep cross-module walk lands once `javascript-ast` grows module syntax. The public surface (name, policy, cost, depends_on) stays put — no churn upstream.
