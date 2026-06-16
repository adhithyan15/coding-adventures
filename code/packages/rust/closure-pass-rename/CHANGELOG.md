# Changelog

All notable changes to the `coding-adventures-closure-pass-rename` crate will be documented in this file.

## [0.3.0] - 2026-06-16

### Added — real renaming (leaf-function parameters)

`RenamePass::run` is no longer identity. It now renames the **parameters of
leaf functions** (function declarations whose body declares no nested function)
to short names (`a`, `b`, …), rewriting the declaration and every use site:

```js
function f(longName) { return longName + 1; }   ⇒   function f(a){return a + 1}
```

It is a self-contained scope-aware α-rename over the Phase-1 AST. It
conservatively never renames:

- module/global top-level names (potentially externally visible);
- free globals (`console`, `window`, …);
- property names — the `.x` of a non-computed member access, a non-computed
  object-literal key;
- a parameter also declared `var`/`let`/`const` in the body (re-declared or
  block-shadowed) — skipped rather than mis-renamed;
- single-character parameters (already minimal).

Fresh names avoid every identifier that appears anywhere in the function, so a
rename can neither collide with another local nor capture a free global. Within
this subset the transform is provably sound; anything outside it is left
untouched (`changed` stays `false`).

This is the v1 slice. Broader renaming — non-leaf scopes, locals, module-private
top-level names — is future work on the same walker, and will consume
`closure-scope-analyzer` for cross-scope resolution (v1 does not yet use it).

- **11 new behavior tests** driving the real `source → bridge → rename → emit`
  roundtrip (added `javascript-parser` + `closure-emitter` dev-deps): renaming,
  property-name preservation, global avoidance, redeclared/non-leaf/single-char
  skips, computed-member rewriting, nested-block uses.
- De-staled the module/struct/test docs (they claimed "v1 is identity").

## [0.2.0] - 2026-06-01

### Added (CLOC13.A — consume `closure-scope-analyzer`)
- Wired the pass to `coding-adventures-closure-scope-analyzer` (new `[dependencies]` entry). `run` now invokes `analyze(ctx.program)` and walks the returned `ScopeAnalysis` to identify **rename candidates** — every binding the analyzer surfaces.
- Algorithm (collect phase; substitute deferred to CLOC13.A.1):
  1. Walk `analysis.bindings`. Every binding is a candidate. (`#[non_exhaustive]` BindingKind: future variants are admitted by default — they all need renaming. This is the *opposite* default from treeshake / collapse-properties, which conservatively *skip* unknown kinds. Rename is conservative-toward-compression rather than conservative-toward-skip — a missed rename is wasted bytes, not a correctness issue.)
  2. Track candidates in `Vec<BindingId>` for observability.
  3. *Substitute deferred*: cleanly rewriting Identifier nodes needs the AST to grow `Identifier` / `VariableDeclarator` variants AND the analyzer to surface a binding → uses backreference (currently absent).
- `PassStats::nodes_touched` now reports `1 + bindings.len() + references.len()` (root + every binding + every reference visited). Real cost surfacing for the scheduler instead of the v0.1.0 placeholder `1`.

### Critical safety pin (lesson from CLOC13.E security review — adapted for OneShot)
- `changed` is **hard-pinned to `false`** until step 3 (the actual program mutation) lands. Even though this pass's `iteration_policy` is `OneShot` — so the FixedPoint infinite-loop concern doesn't apply — the discipline of "don't lie to the scheduler about mutation" is the same. Pipeline consumers may key off `changed` for cache invalidation or to skip downstream serialization; reporting `true` without mutation would force unnecessary work. Documented in both the source (`fn run`) and here so the next contributor doesn't reintroduce the bug.

### Why this is safe to merge ahead of the analyzer body
- The current `closure-scope-analyzer` v0.1.0 returns empty `bindings` + `references`. The candidate scan therefore produces zero names, the candidates vec stays empty, `nodes_touched` is small, and the program passes through unchanged — identical observable behavior to v0.1.0. The wiring becomes **effective** the moment CLOC13.0 lands the analyzer body — no churn here, no rebase needed.

### Dependencies
- Added `coding-adventures-closure-scope-analyzer = { path = "../closure-scope-analyzer" }` to `[dependencies]`. Crate bumped `0.1.0 → 0.2.0`.

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
