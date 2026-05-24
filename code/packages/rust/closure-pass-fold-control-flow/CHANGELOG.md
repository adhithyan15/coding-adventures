# Changelog

All notable changes to the `coding-adventures-closure-pass-fold-control-flow` crate will be documented in this file.

## [0.2.0] - 2026-05-24

### Added — real `Pass::run` body

Replaces the identity v0.1.0 body with a recursive bottom-up walker over `Program → ProgramItem → Statement → Expression`. Folds:

- **`IfStatement` with literal test** → consequent (truthy) / alternate (falsy) / `EmptyStatement` (falsy, no alternate). Truthy/falsy uses JS truthiness rules: any non-empty string / non-zero non-NaN number / true is truthy; null / 0 / "" / false is falsy.
- **`WhileStatement` with literal `false` test** → `EmptyStatement`. `while (true)` is intentionally left alone (semantics matter — infinite loops are observable).
- **Dead code after `ReturnStatement`** in `BlockStatement.body` → dropped. Recurses into nested blocks via `FunctionDeclaration.body`.
- **`ConditionalExpression` with literal test** (`true ? a : b → a`). Redundantly handled here for robustness when this pass runs solo; constant-fold also handles it.

Recurses through every Phase 1 node so deep trees are folded in one bottom-up walk.

### CV tracing — both modes work per CLOC09 amendment

- **Traced input** (`cv: Some(parent)`): the kept replacement keeps its own pre-existing `cv` (it's the same node, just promoted). A `Contribution { source: "fold-control-flow", tag: "folded-branch"|"removed-dead-code", meta: {before, after, parent_cv} }` is appended.
- **Untraced input** (`cv: None`): folds silently with no contributions. `changed: true` still set.

### Tests

19 tests (up from 8 in v0.1.0):
- pass metadata (unchanged)
- empty-program identity
- `if (true) {x} else {y} → x`
- `if (false) {x} else {y} → y`
- `if (false) {x}` no alternate → `EmptyStatement`
- truthiness across booleans, numbers, strings, null — every JS truthy/falsy case
- non-literal test (e.g. `if (flag) {…}`) passes through unchanged
- `if (1 < 2) {A}` alone does NOT fold (comparison is constant-fold's job) — documents the layering
- `while (false) {body}` → `EmptyStatement`
- `while (true)` is left alone
- dead code after `ReturnStatement` dropped (with `removed-dead-code` contribution)
- block without `return` is unchanged
- `ConditionalExpression` with truthy test folds
- **untraced mode** folds silently (no contributions)
- pipeline integration solo
- **pipeline with constant-fold registered**: `if (1 < 2) {A}` flows through both passes and ends as just `A`. Verifies the canonical CLOC06 ordering does what it's supposed to.

### Skipped (queued for v0.3.0+)
- `ThrowStatement` / labelled `BreakStatement` / `ContinueStatement` as terminators — wait for Phase 2 to add the variants.
- `while (true)` infinite-loop collapse when body is provably pure.
- `SwitchStatement` with literal discriminant — Phase 2.

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
