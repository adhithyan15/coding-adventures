# Changelog

All notable changes to the `coding-adventures-closure-pass-inline` crate will be documented in this file.

## [0.6.0] - 2026-06-18

### Added (CLOC15 PR-2 — tail `return` with a discarded result)

The void statement-helper inliner (PR-1) now admits an **optional trailing
`return`** as the body's final statement. Because the call site discards
the result (the use is a statement call, not a value), the returned value
is never read, so the tail return is normalized for the splice:

```js
function init(n) { setup(n); return ready(); }
init(cfg);
// SIMPLE  ⇒  setup(cfg); ready();
//   (the tail `return ready()` is kept as `ready();` for its side effect;
//    the dead declaration is then removed by remove-unused-vars/treeshake)
```

`normalize_tail_return`:

- `return;` (no argument) → **dropped** (a no-op once the value is discarded);
- `return E;` where `E` is **provably inert** — a literal, or a bare read
  of a parameter / callee-local (a binding that always exists, so the read
  neither throws nor has a side effect) → **dropped**;
- `return E;` otherwise → rewritten to `E;` (an `ExpressionStatement`), so
  `E` is still evaluated for its side effects with the value discarded —
  exactly what the original function did before returning.

A bare *global* identifier (`return glob`) is deliberately **not** dropped:
reading an undeclared global throws `ReferenceError`, which must be
preserved as `glob;`.

This also unlocks a shape the expression inliner cannot reach — a body that
is a single `return g()` where `g` is a free global: the expression inliner
requires every identifier to be a parameter, but the discarded-statement
splice turns `f();` into `g();`.

Soundness rests on the **tail** restriction: a `return` anywhere but the
final position would change control flow when spliced (the caller's
following statements would still run), so an early `return` is a hard
reject — the body must be straight-line to the optional tail return.

- 6 new pass tests (literal/bare/param-identifier dropped, effectful call
  kept as `E;`, single-`return`-of-free-global splice, free-global
  identifier kept, early-`return` declined).
- Two closurec fixtures (`simple_dce_drops_dead_after_return`,
  `advanced-rename-globals`) updated: their helpers are now called twice so
  they survive the single-use statement-inliner, preserving each test's
  original intent (observing DCE-after-return / ADVANCED renaming a
  *surviving* top-level function) rather than collapsing away.

## [0.5.0] - 2026-06-18

### Added (CLOC15 PR-1 — void multi-statement statement-helper inlining)

The inliner is no longer limited to the `{ return EXPR; }` expression
shape. A new statement-level path splices a **single-use void
multi-statement helper** at its (statement-position) call site — the
1 → N statement splice the expression walker structurally could not do:

```js
function track(n, v) { const e = n + v; metrics.push(e); }
track(a, b);
// SIMPLE  ⇒  const c = a + b; metrics.push(c);
//           (helper inlined — local `e` alpha-renamed to a fresh `c`,
//            params substituted — then the dead declaration is removed
//            by remove-unused-vars / treeshake)
```

This implements the first staged slice of the
[CLOC15 spec](../../../specs/CLOC15-multi-statement-inlining.md). It is
**sound-by-construction** — every condition below is a hard reject, and
declining to inline is never a miscompile:

- **Single-use, single-declaration** — the helper's name is declared
  exactly once (no shadowing) and used exactly once.
- **The one use is a discarded statement call** (`track(…);`), not a
  value (`x = track(…)`, `log(track(…))`). A discarded result means
  there is nothing to capture — value capture is a later slice (PR-3).
- **Straight-line body, no `return`** — each body statement is an
  `ExpressionStatement` or a `let` / `const` `VariableDeclaration`;
  nothing else (no `return`, control flow, `var`, or nested blocks).
- **No `this` / `arguments`** — their meaning is frame-bound and would
  silently rebind on a splice; rejected explicitly.
- **Callee locals are alpha-renamed to program-fresh names** before
  splicing (a base-26 generator avoiding every identifier in the
  program), so a spliced `let e` can never collide with or shadow a
  binding live at the call site.
- **Free identifiers must be true globals** — a body name that is
  neither a parameter nor a callee-local must be declared *nowhere* in
  the program, so it is unshadowable at any splice site. (The
  conservative bootstrap the spec's Open Question 1 sanctions; a later
  slice can widen it via `closure-scope-analyzer`.)
- **Side-effect-free arguments** (the existing `is_simple_arg` gate) —
  substituting them for a parameter used any number of times never
  drops or duplicates a side effect.

When the call sits in an unbraced single-statement slot (`if (c) f();`),
the spliced statements are wrapped in a fresh block so control flow stays
correct; in a real statement list they are spliced in flat.

- Runs as a new Phase 4 after the expression inliner. The two operate on
  disjoint function shapes, so neither perturbs the other's candidate
  set, and the declaration-count map stays valid (inlining removes call
  sites, never declarations).
- A `let`/`const` local sharing a parameter's spelling (illegal JS, but
  cheap to guard) is declined — the name-based alpha-renamer is not
  scope-aware, so this is defense in depth against a non-conformant
  parser rather than a path reachable from valid input.
- 14 new tests: the signature local+global splice, the no-locals case,
  alpha-rename-avoids-argument-collision, empty-body call drop, the
  unbraced-`if` block wrap, and eight decline cases (value-position use,
  `var` local, tail `return`, `arguments`, free *declared* name,
  side-effecting argument, multi-use, recursion, param/local collision).

## [0.4.0] - 2026-06-17

### Added (CLOC13.G — multi-use inlining under a size budget)

The pass no longer inlines only single-use functions; it now inlines a callee
at **all** its call sites when doing so is a size win:

```js
function sq(x) { return x * x; }
a(sq(3));
b(sq(4));
// SIMPLE  ⇒  a(9); b(16);   (both sites inlined, then constant-fold)
```

- A function is inlined only when **every** use of its name is an inlinable
  call (matching arity, side-effect-free args) — `uses == inlinable_calls`. If
  even one use is a value (`g(f)`) or a non-inlinable call, the whole function
  is declined: partial inlining would duplicate the body *and* keep the
  declaration, usually a net loss.
- **Single-use** → always inlined (a strict win). **Multi-use (N>1)** → inlined
  only when the body fits the budget `expr_node_count(body) <= 2 + params.len()`
  (see `multiuse_budget_ok`), so the substituted body is never larger than the
  call it replaces — duplicating it across the sites can't grow the output, and
  removing the declaration is a pure saving. A body too large to duplicate is
  left alone.
- All the soundness guarantees of the single-use slice carry over unchanged
  (top-level plain function, body `{ return EXPR }`, every identifier a
  parameter → no capture/`this`/`arguments`/recursion, name declared once,
  side-effect-free args → safe to duplicate). Multi-use adds no new soundness
  obligation; the budget is purely a "worth it?" knob.

### Internals
- `count_name_uses_*` → `tally_*`: one walk now counts both total uses and
  inlinable calls (`Tally { uses, inlinable }`).
- `inline_single_call` → `inline_all_calls`: the substitution walk no longer
  short-circuits on the first match — it rewrites every call site.
- New `expr_node_count` / `multiuse_budget_ok` / `is_inlinable_call` helpers.

### Tests
- New roundtrip tests: multi-use small body inlined at both sites; multi-use
  large body declined (over budget); declined when one of several uses is a
  value; declined when one call has a side-effecting argument.

Crate bumped `0.3.0 → 0.4.0`.

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
