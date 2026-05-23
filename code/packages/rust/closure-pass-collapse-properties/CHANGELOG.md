# Changelog

All notable changes to the `coding-adventures-closure-pass-collapse-properties` crate will be documented in this file.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 canonical pass set — property-collapse. Collapses repeated nested property-access chains on stable namespace-style objects into shorter local bindings (`ns.utils.format.x`, `ns.utils.format.y`, `ns.utils.format.z` → `const $f = ns.utils.format; $f.x; $f.y; $f.z`).
- `CollapsePropertiesPass` zero-sized type implementing `Pass`:
  - `name = "collapse-properties"`
  - `depends_on = &["constant-fold"]` — folded constants resolve into recognisable property-access shapes the collapse pass can spot (e.g. `ns[KEY]` where `KEY = "utils"`).
  - `iteration_policy = IterationPolicy::FixedPoint` — collapsing one chain can expose new shared prefixes; bounded in practice by chain depth.
  - `cost = 3` pass-units — gather chain frequencies + emit binding + rewrite uses. Same shape as DCE.
  - `invalidates()` empty in v1 (informational only per CLOC06 Open Question 1).
- `CollapsePropertiesPass::new()` zero-arg constructor.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there are no `MemberExpression` / `Identifier` / `VariableDeclaration` nodes to collapse. Pass through unchanged, `changed = false`, `nodes_touched = 1`, no contributions emitted (per CLOC03 §"When a pass keeps a node unchanged").
- 9 tests covering: `name()` value, `iteration_policy == FixedPoint`, `cost == 3`, `depends_on == ["constant-fold"]`, `invalidates` empty, identity run, **two-pass pipeline orders constant-fold before collapse-properties** even when registered in reverse, solo pipeline run (with the v0.1.0 `pipeline.fixed-point-not-yet-iterated` diagnostic asserted), `Default` + `Clone` impls.

### Notes
- Safety prerequisite: collapse is only valid when the intermediate object is genuinely stable. The implementation will read the type sidecar's `stable` / `pure` / `frozen` attributes plus a local mutation analysis. CLOC04 left the exact attribute name open; the implementation will pin it once the sidecar grows the namespace-stability marker.
- Dependencies: `coding-adventures-closure-pass-pipeline`, `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar`, `coding_adventures_correlation_vector` (per-collapse `Contribution` per CLOC03), `serde_json`. Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`, `coding-adventures-closure-pass-constant-fold` for the two-pass ordering integration test.
- v1 is scaffolding. The full gather + rewrite lands once `javascript-ast` grows the needed variants. The public surface (name, policy, cost, depends_on) stays put — no churn upstream.
