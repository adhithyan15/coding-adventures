# Changelog

All notable changes to the `coding-adventures-closure-pass-constant-fold` crate will be documented in this file.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 — first concrete optimization pass plugged into the `closure-pass-pipeline` harness.
- `ConstantFoldPass` zero-sized type implementing `Pass`:
  - `name = "constant-fold"`
  - `iteration_policy = IterationPolicy::FixedPoint` (folds expose further folds; full multi-iteration loop arrives when the pipeline grows past v0.1.0)
  - `cost = 2` pass-units (tree walk + small constant work per visit)
  - `depends_on()` / `invalidates()` empty in v1
- `ConstantFoldPass::new()` zero-arg constructor for ergonomic `PassPipeline::add(Box::new(ConstantFoldPass::new()))` registration.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there's nothing to fold. The pass clones the input `Program` unchanged, returns `changed = false`, `nodes_touched = 1`, no contributions (per CLOC03 §"When a pass keeps a node unchanged").
- 8 tests covering: `name()` value, `iteration_policy` is FixedPoint, `cost` is 2, `depends_on`/`invalidates` empty, run on empty Program is identity (program unchanged, no contributions, stats correct), full `PassPipeline` integration as solo pass (verifies FixedPoint note diagnostic flows through), pipeline integration alongside an unrelated upstream pass (registration order preserved), pass is `Default` + `Clone`.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline` (Pass trait + types), `coding-adventures-javascript-ast` (Program), `coding-adventures-type-sidecar` (future type-aware fold safety), `coding_adventures_correlation_vector` (Contribution plumbing), `serde_json` (meta JSON values). Dev-dep: `coding-adventures-javascript-tokens` for `EsVersion` in tests.
- v1 is scaffolding. Real folding (number/string/boolean/typeof/negation/comparison/conditional) lands once `javascript-ast` grows `Statement` / `Expression` variants — at that point this file becomes a real pass without any API churn upstream.
