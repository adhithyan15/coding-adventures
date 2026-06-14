# Changelog

All notable changes to the `coding-adventures-closure-pass-inline` crate will be documented in this file.

## [0.2.0] - 2026-06-01

### Added (CLOC13.B — consume `closure-scope-analyzer`)
- Wired the pass to `coding-adventures-closure-scope-analyzer` (new `[dependencies]` entry). `run` now invokes `analyze(ctx.program)` and walks the returned `ScopeAnalysis` to identify **inline candidates** — function/class-shaped bindings that are called from exactly one site, the unambiguous-win case where substituting the body saves call overhead and exposes concrete arguments to downstream constant-fold.
- Algorithm (mark phase; substitute deferred to CLOC13.B.1):
  1. Per-binding use-count derived from `analysis.references`. The single-use property is the gate that makes inlining cheap (clone once vs. duplicating the body N times).
  2. Candidate scan: a binding qualifies when `kind == Function || kind == Class` AND `uses == 1`. `Param` is excluded (params aren't callable bodies). `Var`/`Let`/`Const` of function-expressions lower to `Function` once the analyzer grows expression tracking; until then those are handled by `collapse-properties` (CLOC13.D) for their alias form. `#[non_exhaustive]` future variants are conservatively skipped via the wildcard arm — same default as treeshake / collapse-properties.
  3. *Substitute deferred*: cleanly replacing a `CallExpression` with the callee's body requires both the AST to grow `CallExpression` / `FunctionDeclaration` variants AND the analyzer to surface a binding → defining-node backreference.
- Multi-use inlining is a budget decision (size threshold × call-site count); the single-use case is the cheapest substitution to land first.
- `PassStats::nodes_touched` now reports `1 + bindings.len() + references.len()` (root + every binding + every reference visited). Real cost surfacing instead of the v0.1.0 placeholder `1`.

### Critical safety pin (lesson from CLOC13.E security review)
- `changed` is **hard-pinned to `false`** until step 3 (the actual program mutation) lands. The pass identifies candidates and returns the program *unchanged*. Reporting `changed = true` while returning an unchanged program would cause the scheduler under `IterationPolicy::FixedPoint` to re-run forever — each iteration would find the same candidates, claim a change, return the same program, repeat. Documented in both the source and here.

### Why this is safe to merge ahead of the analyzer body
- The current `closure-scope-analyzer` v0.1.0 returns empty `bindings` + `references`. The candidate scan therefore produces zero call targets, the candidates vec stays empty, and the program passes through unchanged — identical observable behavior to v0.1.0. The wiring becomes **effective** the moment CLOC13.0 lands the analyzer body — no churn here, no rebase needed.

### Dependencies
- Added `coding-adventures-closure-scope-analyzer = { path = "../closure-scope-analyzer" }` to `[dependencies]`. Crate bumped `0.1.0 → 0.2.0`.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 canonical pass set — function inlining. Substitutes a callee's body at the call site when doing so is cheaper than the call; enables downstream constant-folding on now-concrete arguments.
- `InlinePass` zero-sized type implementing `Pass`:
  - `name = "inline"`
  - `depends_on = &["constant-fold"]` — folded arguments plug into parameters cleanly; unfolded would force the inliner to carry around expression trees as parameter bindings.
  - `iteration_policy = IterationPolicy::FixedPoint` — inlining `f(g(h(7)))` first inlines `f`, exposing the inner calls in the substituted body. Bounded in practice by the inlining-budget heuristic, not the policy.
  - `cost = 4` pass-units — heaviest of the v1 passes. Call-graph build + per-site heuristic eval + clone-and-rewrite of callee bodies.
  - `invalidates()` empty in v1 (informational only per CLOC06 Open Question 1).
- `InlinePass::new()` zero-arg constructor.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there are no `FunctionDeclaration` / `CallExpression` / `Identifier` nodes to inline. Pass through unchanged, `changed = false`, `nodes_touched = 1`, no contributions emitted (per CLOC03 §"When a pass keeps a node unchanged").
- 9 tests covering: `name()` value, `iteration_policy == FixedPoint`, `cost == 4`, `depends_on == ["constant-fold"]`, `invalidates` empty, identity run, **two-pass pipeline orders constant-fold before inline** even when registered in reverse, solo pipeline run (with the v0.1.0 `pipeline.fixed-point-not-yet-iterated` diagnostic asserted), `Default` + `Clone` impls.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline`, `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar` (`pure` / `no_side_effects` attributes inform inline safety), `coding_adventures_correlation_vector` (per-inline `Contribution` per CLOC03), `serde_json`. Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`, `coding-adventures-closure-pass-constant-fold` for the two-pass ordering integration test.
- DCE ordering (DCE-before-inline per canonical order) is a *preference*, not a *correctness* requirement, so it isn't in `depends_on`. Inlining is still correct on un-DCE'd input; it just wastes work on dead callees.
- v1 is scaffolding. The full call-graph walk + per-site substitution lands once `javascript-ast` grows the needed variants. The public surface (name, policy, cost, depends_on) stays put.
