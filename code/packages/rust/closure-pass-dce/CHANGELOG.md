# Changelog

All notable changes to the `coding-adventures-closure-pass-dce` crate will be documented in this file.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 canonical pass set — second concrete optimization pass after constant-fold.
- `DcePass` zero-sized type implementing `Pass`:
  - `name = "dce"`
  - `depends_on = &["constant-fold"]` — folds expose dead arms so they run first per CLOC06 canonical order. `fold-control-flow` will join this list once it exists.
  - `iteration_policy = IterationPolicy::FixedPoint` — deletion can free further nodes.
  - `cost = 3` pass-units (tree walk + reachability marking + post-walk deletion).
  - `invalidates()` empty in v1 (informational only per CLOC06 Open Question 1).
- `DcePass::new()` zero-arg constructor.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there's nothing to delete. Pass through unchanged, `changed = false`, `nodes_touched = 1`, no contributions emitted (per CLOC03 §"When a pass keeps a node unchanged").
- 8 tests covering: `name()` value, `iteration_policy` is FixedPoint, `cost` is 3, `depends_on == ["constant-fold"]`, `invalidates` empty, run on empty Program is identity, **pipeline correctly orders constant-fold before dce** even when DCE is registered first (this is the key value-add of the depends_on edge), DCE runs as a solo pass with unknown deps silently dropped per v0.1.0 scheduler, `Default` + `Clone` impls.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline`, `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar` (future `pure` / `no_side_effects` attributes inform deletion safety), `coding_adventures_correlation_vector` (`cv.delete()` + `"deleted"` `Contribution` per CLOC03), `serde_json`. Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`, `coding-adventures-closure-pass-constant-fold` for the ordering integration test.
- v1 is scaffolding. The full reachability walk + deletion lands once `javascript-ast` grows the needed variants. When that happens, the `Pass::run` body changes but the public surface stays put — no churn upstream.
