# Changelog

All notable changes to the `coding-adventures-closure-pass-inline` crate will be documented in this file.

## [0.3.0] - 2026-06-17

### Added (CLOC13.B.1 — real call-site substitution)
- `InlinePass::run` is now a **real transform**: it inlines single-use top-level leaf functions whose body is exactly `{ return EXPR; }`, replacing the call with the substituted body. `log(double(7))` for `function double(x) { return x * 2; }` becomes `log(7 * 2)` (and the now-dead `double` declaration is removed by the later remove-unused-vars / treeshake passes — this pass deliberately leaves it in place).
- The implementation is **self-contained** (its own name-based shadow + use analysis over the Phase-1 AST), mirroring the `rename` pass's philosophy — it no longer relies on `closure-scope-analyzer`'s candidate scan (which keyed on optional per-node CvIds that the bridge does not populate).

### The provably-safe slice (every hard inlining hazard made structurally impossible)
A call `f(a₁, …, aₙ)` is inlined only when ALL hold:
1. **`f` is a top-level plain `function`** (not generator / not `async`) — no enclosing scope to capture, no resumable state.
2. **`f`'s body is exactly `{ return EXPR; }`** — substitution is a pure expression-for-expression swap; no locals/branches to splice.
3. **Every identifier in `EXPR` is one of `f`'s parameters** — the capture guard. No free identifiers ⇒ no global capture, no `this`/`arguments`, and recursion excluded for free (a self-call makes `f` a free identifier).
4. **`f`'s name is declared exactly once in the whole program** — no shadowing, so every use of the identifier resolves to this function and can be counted/located by name.
5. **`f` is used exactly once, and that use is the call**, with `arguments.len() == params.len()` — the unambiguous single-use size win.
6. **Every argument is side-effect-free** (a literal or bare identifier) — substituting for a parameter used zero/one/many times can neither drop nor duplicate a side effect.

Everything outside this subset is left untouched (`changed` stays `false`). The `changed = true` it now returns when it does inline is safe under `IterationPolicy::FixedPoint`: each round strictly removes a single-use callee's only reference, so the candidate set shrinks monotonically and the fixed point is reached in finitely many steps.

### Precedence
- Substitution operates on the typed AST, so the precedence-aware `closure-emitter` parenthesizes correctly — e.g. inlining a `BinaryExpression` body into a higher-precedence position emits the necessary parens from the tree structure.

### Tests
- 15 new source → bridge → inline → emit roundtrip tests covering the positive cases (single-use, identifier args, two params, computed members, nested-call arguments, property-name preservation) and every rejection (multi-use, recursive, free global, shadowed name, arity mismatch, side-effecting argument, non-call value use, multi-statement body).

### Dependencies
- Removed the (now unused) `closure-scope-analyzer`/`serde_json`/`type-sidecar`/`correlation-vector` reliance from the transform path; kept as deps for parity with sibling pass crates. Added dev-deps `coding-adventures-javascript-parser` and `coding-adventures-closure-emitter` for the roundtrip tests. Crate bumped `0.2.0 → 0.3.0`.

## [0.2.0] - 2026-06-01

### Added (CLOC13.B — consume `closure-scope-analyzer`)
- Wired the pass to `coding-adventures-closure-scope-analyzer` (new `[dependencies]` entry). `run` now invokes `analyze(ctx.program)` and walks the returned `ScopeAnalysis` to identify **inline candidates** — function/class-shaped bindings that are called from exactly one site, the unambiguous-win case where substituting the body saves call overhead and exposes concrete arguments to downstream constant-fold.
- Algorithm (mark phase; substitute deferred to CLOC13.B.1):
  1. Per-binding use-count derived from `analysis.references`. The single-use property is the gate that makes inlining cheap (clone once vs. duplicating the body N times).
  2. Candidate scan: a binding qualifies when `kind == Function || kind == Class` AND `uses == 1`. `Param` is excluded (params aren't callable bodies). `Var`/`Let`/`Const` of function-expressions lower to `Function` once the analyzer grows expression tracking; until then those are handled by `collapse-properties` (CLOC13.D) for their alias form. `#[non_exhaustive]` future variants are conservatively skipped via the wildcard arm — same default as treeshake / collapse-properties.
  3. *Substitute deferred*: cleanly replacing a `CallExpression` with the callee's body requires both the AST to grow `CallExpression` / `FunctionDeclaration` variants AND the analyzer to surface a binding → defining-node backreference.
- Multi-use inlining is a budget decision (size threshold × call-site count); the single-use case is the cheapest substitution to land first.
- `PassStats::nodes_touched` now reports `1 + bindings.len() + references.len()` (root + every binding + every reference visited). Real cost surfacing instead of the v0.1.0 placeholder `1`.

### Critical safety pin (lesson from CLOC13.E security review)
- `changed` is **hard-pinned to `false`** until step 3 (the actual program mutation) lands. The pass identifies candidates and returns the program *unchanged*. Reporting `changed = true` while returning an unchanged program would cause the scheduler under `IterationPolicy::FixedPoint` to re-run forever — each iteration would find the same candidates, claim a change, return the same program, repeat. Documented in both the source and here.

### Why this is safe to merge ahead of the analyzer body
- The current `closure-scope-analyzer` v0.1.0 returns empty `bindings` + `references`. The candidate scan therefore produces zero call targets, the candidates vec stays empty, and the program passes through unchanged — identical observable behavior to v0.1.0. The wiring becomes **effective** the moment CLOC13.0 lands the analyzer body — no churn here, no rebase needed.

### Dependencies
- Added `coding-adventures-closure-scope-analyzer = { path = "../closure-scope-analyzer" }` to `[dependencies]`. Crate bumped `0.1.0 → 0.2.0`.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 canonical pass set — function inlining. Substitutes a callee's body at the call site when doing so is cheaper than the call; enables downstream constant-folding on now-concrete arguments.
- `InlinePass` zero-sized type implementing `Pass`:
  - `name = "inline"`
  - `depends_on = &["constant-fold"]` — folded arguments plug into parameters cleanly; unfolded would force the inliner to carry around expression trees as parameter bindings.
  - `iteration_policy = IterationPolicy::FixedPoint` — inlining `f(g(h(7)))` first inlines `f`, exposing the inner calls in the substituted body. Bounded in practice by the inlining-budget heuristic, not the policy.
  - `cost = 4` pass-units — heaviest of the v1 passes. Call-graph build + per-site heuristic eval + clone-and-rewrite of callee bodies.
  - `invalidates()` empty in v1 (informational only per CLOC06 Open Question 1).
- `InlinePass::new()` zero-arg constructor.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there are no `FunctionDeclaration` / `CallExpression` / `Identifier` nodes to inline. Pass through unchanged, `changed = false`, `nodes_touched = 1`, no contributions emitted (per CLOC03 §"When a pass keeps a node unchanged").
- 9 tests covering: `name()` value, `iteration_policy == FixedPoint`, `cost == 4`, `depends_on == ["constant-fold"]`, `invalidates` empty, identity run, **two-pass pipeline orders constant-fold before inline** even when registered in reverse, solo pipeline run (with the v0.1.0 `pipeline.fixed-point-not-yet-iterated` diagnostic asserted), `Default` + `Clone` impls.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline`, `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar` (`pure` / `no_side_effects` attributes inform inline safety), `coding_adventures_correlation_vector` (per-inline `Contribution` per CLOC03), `serde_json`. Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`, `coding-adventures-closure-pass-constant-fold` for the two-pass ordering integration test.
- DCE ordering (DCE-before-inline per canonical order) is a *preference*, not a *correctness* requirement, so it isn't in `depends_on`. Inlining is still correct on un-DCE'd input; it just wastes work on dead callees.
- v1 is scaffolding. The full call-graph walk + per-site substitution lands once `javascript-ast` grows the needed variants. The public surface (name, policy, cost, depends_on) stays put.
