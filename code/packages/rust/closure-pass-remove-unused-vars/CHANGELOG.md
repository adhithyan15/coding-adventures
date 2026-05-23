# Changelog

All notable changes to the `coding-adventures-closure-pass-remove-unused-vars` crate will be documented in this file.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 canonical pass set — the final cleanup pass. Deletes variable bindings whose initializer is pure and whose reference-count after DCE + inline have run is zero. Closure Compiler ships an analogous `removeUnusedVars` pass for the same reason.
- `RemoveUnusedVarsPass` zero-sized type implementing `Pass`:
  - `name = "remove-unused-vars"`
  - `depends_on = &["dce", "inline"]` — DCE prunes unreachable code that may have referenced bindings; inline replaces call sites and can leave function declarations unreferenced. Both must run first to catch the maximum orphan set.
  - `iteration_policy = IterationPolicy::FixedPoint` — removing one binding can unreference another (chain of pure initializers). Bounded in practice by chain length.
  - `cost = 3` pass-units — per-scope binding-table build + delete. Same shape as DCE.
  - `invalidates()` empty in v1 (informational only per CLOC06 Open Question 1).
- `RemoveUnusedVarsPass::new()` zero-arg constructor.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there are no `VariableDeclaration` / `Identifier` nodes to remove. Pass through unchanged, `changed = false`, `nodes_touched = 1`, no contributions emitted (per CLOC03 §"When a pass keeps a node unchanged").
- 10 tests covering: `name()` value, `iteration_policy == FixedPoint`, `cost == 3`, `depends_on == ["dce", "inline"]`, `invalidates` empty, identity run, **two-pass pipeline orders dce before remove-unused-vars** even when registered in reverse, **three-pass pipeline canonical-orders dce + inline before remove-unused-vars** when registered out of order, solo pipeline run (with the v0.1.0 `pipeline.fixed-point-not-yet-iterated` diagnostic asserted), `Default` + `Clone` impls.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline`, `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar` (`pure` / `no_side_effects` attributes inform delete safety), `coding_adventures_correlation_vector` (per-deletion `Contribution` per CLOC03), `serde_json`. Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`, `coding-adventures-closure-pass-dce` for the two-pass ordering integration test, `coding-adventures-closure-pass-inline` for the three-pass ordering integration test.
- v1 is scaffolding. The full per-scope walk + deletion lands once `javascript-ast` grows the needed variants. The public surface stays put — no churn upstream.
