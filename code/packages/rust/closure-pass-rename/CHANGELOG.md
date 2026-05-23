# Changelog

All notable changes to the `coding-adventures-closure-pass-rename` crate will be documented in this file.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 canonical pass set — variable renaming for output-size reduction. Replaces non-exported binding names (locals, internal function names, private class members) with short identifiers; preserves externally-visible names.
- `RenamePass` zero-sized type implementing `Pass`:
  - `name = "rename"`
  - `depends_on = &[]` — rename is correct standalone; it just produces less compression without earlier passes. A future `freeze-externals` pass would join this list.
  - `iteration_policy = IterationPolicy::OneShot` — one walk renames every renameable binding; rename doesn't open new opportunities for itself.
  - `cost = 3` pass-units — two-pass walk (collect bindings, then substitute) plus the name allocator.
  - `invalidates()` empty in v1 (informational only per CLOC06 Open Question 1).
- `RenamePass::new()` zero-arg constructor.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there are no `Identifier` / `VariableDeclarator` / `FunctionDeclaration` nodes to rename. Pass through unchanged, `changed = false`, `nodes_touched = 1`, no contributions emitted (per CLOC03 §"When a pass keeps a node unchanged").
- 8 tests covering: `name()` value, `iteration_policy == OneShot`, `cost == 3`, `depends_on` empty, `invalidates` empty, identity run, solo pipeline run (asserts no `pipeline.fixed-point-not-yet-iterated` diagnostic since OneShot ≠ FixedPoint), `Default` + `Clone` impls.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline`, `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar` (`external` attribute marks do-not-rename bindings), `coding_adventures_correlation_vector` (per-rename `Contribution` per CLOC03), `serde_json`. Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`.
- v1 is scaffolding. The full two-pass walk (collect → substitute) lands once `javascript-ast` grows the needed variants. The public surface (name, policy, cost, depends_on) stays put — no churn upstream.
