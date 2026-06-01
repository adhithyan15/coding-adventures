# Changelog

All notable changes to the `coding-adventures-closure-pass-treeshake` crate will be documented in this file.

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
