# Changelog

All notable changes to the `coding-adventures-closure-pass-remove-unused-vars` crate will be documented in this file.

## [0.2.0] - 2026-06-01

### Added — CLOC13.E: wire pass to consume `closure-scope-analyzer`

The pass `run` body now calls `scope_analyzer::analyze(program)`,
builds a per-binding use-count by scanning `analysis.references`,
and identifies bindings with use-count zero whose `kind` is `Var`,
`Let`, or `Const` (skipping `Function`/`Param`/`Class` until a
follow-up). `nodes_touched` now counts the analyzer-visited
bindings + references so the scheduler sees real cost numbers.

**Why the program is still passthrough.** The current
`scope_analyzer 0.1.0` ships an identity `analyze` (returns one
global scope with empty bindings and references — the API surface
unblocker per CLOC13). So the use-count walk finds zero dead
bindings, `removed_count` is always 0, and the program comes out
unchanged. The wiring becomes *observable* in `stats.nodes_touched`
(now counts the analyzer-visited bindings + references) and
becomes *effective* the moment the analyzer's body lands as
CLOC13.0 — no churn here.

**Step 3 (apply removal) is deferred to CLOC13.E.1.** Cleanly
dropping a binding from the AST requires a binding → declarator
backreference that the analyzer doesn't yet ship. Once it does,
the eligibility list (`dead_bindings`) feeds straight into a
walk-and-drop pass over `Program.body`.

**`changed` is hard-pinned to `false` until step 3 lands.** Under
`IterationPolicy::FixedPoint`, reporting `changed = true` while
returning an unchanged program would cause the scheduler to
re-run this pass forever (each iteration finds the same
`dead_bindings`, reports change, returns the same program, repeats).
That bug would fire the moment the analyzer's body started
populating bindings — exactly the kind of cross-PR break that's
hard to bisect. So we compute `dead_bindings` for cost-accounting
observability via `nodes_touched`, but keep `changed = false`
until CLOC13.E.1 wires actual program mutation. Security review
caught this in CLOC13.E and the fix is in this commit.

### Changed

- Cargo dependency: adds
  `coding-adventures-closure-scope-analyzer = { path = ".." }`.
- Version bumped 0.1.0 → 0.2.0 (additive runtime behavior change;
  no API surface change on `RemoveUnusedVarsPass`).

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
