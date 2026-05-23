# Changelog

All notable changes to the `coding-adventures-closure-pass-treeshake` crate will be documented in this file.

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
