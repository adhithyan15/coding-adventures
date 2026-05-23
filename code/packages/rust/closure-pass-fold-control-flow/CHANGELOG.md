# Changelog

All notable changes to the `coding-adventures-closure-pass-fold-control-flow` crate will be documented in this file.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 canonical pass set — slots between `constant-fold` and `dce` in the canonical order.
- `FoldControlFlowPass` zero-sized type implementing `Pass`:
  - `name = "fold-control-flow"`
  - `depends_on = &["constant-fold"]` — folds expose statically-known conditions (`if (1+1===2)` → `if (true)`) that this pass then collapses.
  - `iteration_policy = IterationPolicy::FixedPoint` — eliminating one branch can expose another that's also statically dead.
  - `cost = 2` pass-units — matches constant-fold's weight (single tree walk with per-node local decisions).
  - `invalidates()` empty in v1 (informational only per CLOC06 Open Question 1).
- `FoldControlFlowPass::new()` zero-arg constructor.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there are no `IfStatement` / `WhileStatement` / `SwitchStatement` / `ConditionalExpression` nodes to fold. Pass through unchanged, `changed = false`, `nodes_touched = 1`, no contributions emitted (per CLOC03 §"When a pass keeps a node unchanged").
- 9 tests covering: `name()` value, `iteration_policy == FixedPoint`, `cost == 2`, `depends_on == ["constant-fold"]`, `invalidates` empty, identity run, **two-pass pipeline orders constant-fold before fold-control-flow** even when registered in reverse, **three-pass pipeline produces the canonical order** (constant-fold → fold-control-flow → dce) when all three are registered out of order, solo run with unknown deps silently dropped per the v0.1.0 scheduler, `Default` + `Clone` impls.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline`, `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar` (future side-effect attributes inform fold safety), `coding_adventures_correlation_vector` (`cv.delete()` + `"folded-branch"` `Contribution` per CLOC03), `serde_json`. Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`, `coding-adventures-closure-pass-constant-fold` for the two-pass ordering integration test, `coding-adventures-closure-pass-dce` for the three-pass ordering integration test.
- v1 is scaffolding. The full reachability/fold logic lands once `javascript-ast` grows the needed variants. The public surface (name, policy, cost, depends_on) stays put — no churn upstream.
- Followup PR: tighten `dce`'s `depends_on` from `["constant-fold"]` to `["constant-fold", "fold-control-flow"]` so the canonical order is structurally required, not incidental. Kept separate per the small-PR principle.
