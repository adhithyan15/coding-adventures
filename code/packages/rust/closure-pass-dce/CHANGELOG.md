# Changelog

All notable changes to the `coding-adventures-closure-pass-dce` crate will be documented in this file.

## [0.2.0] - 2026-05-24

### Added — real `Pass::run` body (final step of the autonomous chain)

Replaces v0.1.0's identity body with a recursive walker over `Program → ProgramItem → Statement → Expression`. Final step in the autonomous-chain real-body rollout (after constant-fold, fold-control-flow, and the closure-emitter).

Two cleanup categories per `BlockStatement.body`:

- **Dead-after-terminator**: drop everything after a `ReturnStatement`. Phase 1 doesn't have `ThrowStatement` yet; `BreakStatement` / `ContinueStatement` only qualify in their enclosing loop scope (Phase 2 work).
- **Empty-statement removal**: drop `EmptyStatement` nodes entirely. They're semantically a no-op (`;`) and clutter output.

Recurses through every Phase 1 node so nested blocks (function bodies, if-bodies, while-bodies, for-bodies) get cleaned too. Records one `Contribution` per drop *category* per block (not per-statement — that'd be too noisy).

### Why this overlaps with fold-control-flow's dead-after-return

Intentional overlap:

- `fold-control-flow` does the cleanup as part of its block rewrite when it observes the terminator while folding.
- DCE runs **after** fold-control-flow per CLOC06 canonical order, and catches:
  - Cases where fold-control-flow didn't enter the block (e.g. it was busy folding the surrounding `if`'s test);
  - `EmptyStatement` nodes that fold-control-flow *produced* when it collapsed `if (false) { … }` with no alternate;
  - Future cases where a Phase 2+ pass leaves dead code behind.

### CV tracing — both modes per CLOC09 amendment

- **Traced** (`cv: Some` on the block): `Contribution { source: "dce", tag: "removed-dead-code" | "removed-empty-statement", meta: {before, after, parent_cv} }` appended per category that triggered.
- **Untraced** (`cv: None`): drops silently, no contributions emitted. `changed: true` still set.

### Tests

14 tests (up from 8 in v0.1.0):
- pass metadata unchanged
- empty program identity
- `{x; return; y; z;}` → `{x; return;}` (drop y and z)
- `{x; y;}` (no return) unchanged
- `{x; ; y; ; ;}` → `{x; y;}` (drop empties)
- Both categories in one block: `{x; ; return; y; ;}` → `{x; return;}` with two contributions
- Nested blocks: outer kept, inner's dead-after-return cleaned
- Untraced mode drops silently
- Pipeline solo
- **Full canonical pipeline**: `constant-fold + fold-control-flow + dce` on `if (1 < 2) {z;}` → `z;` (the whole chain cooperates correctly)
- **End-to-end through the chain**: `function f() { if (false) {x;} return; y; }` → `function f() { return; }` after fold + fold-control-flow (drops if-false-branch and creates EmptyStatement, drops y after return) + dce (removes the leftover EmptyStatement)

### Dependencies
- Added `coding-adventures-closure-pass-fold-control-flow` as a dev-dep for the full-canonical-pipeline test.

### Skipped (Phase 1.x / 2+)
- Unreferenced `VariableDeclaration` removal — `closure-pass-remove-unused-vars`'s job.
- Empty `BlockStatement` collapse to `EmptyStatement` — preserves debugging-step shape for now.
- Phase 2: `ThrowStatement` as terminator, `BreakStatement` / `ContinueStatement` qualifying in their loop scope.

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
