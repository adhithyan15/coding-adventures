# Changelog

All notable changes to the `coding-adventures-closure-pass-inline` crate will be documented in this file.

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
