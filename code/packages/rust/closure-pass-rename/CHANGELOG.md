# Changelog

All notable changes to the `coding-adventures-closure-pass-rename` crate will be documented in this file.

## [0.8.0] - 2026-06-20

### Added — CLOC22: local renaming across `for`-`in` (loop-variable soundness)

Every phase of the pass recurses through `ForInStatement`. The crux is the loop
variable: for `for (var/let/const k in o)` the `left` binding is a rename target
treated exactly like a for-loop init binding — `collect_decl_occurrences_stmt`
records it (block-scoped to the loop) and `rewrite_uses` rewrites the declared
name via the rename map, so the binding and its uses inside the body rename
*consistently*. The expression-left form (`for (k in o)`) has its assignment
target rewritten as a use. Verified end-to-end: `for (var element in c)` →
`for (var b in c)` with `c[element]` → `c[b]`.

## [0.7.0] - 2026-06-20

### Added — CLOC21: handle `DebuggerStatement`

Every phase of the pass now covers `DebuggerStatement` (grouped with the other
childless leaf statements) as a no-op. A `debugger;` binds and references
nothing, so there is nothing to rename — added to keep the matches exhaustive
over the new AST variant.

## [0.6.0] - 2026-06-20

### Added — CLOC20: local renaming across `do`/`while`

The pass recurses through `DoWhileStatement` in every phase (`process_tagged`,
`stmt_has_function`, `collect_decl_occurrences_stmt`, `collect_all_idents_stmt`,
`rewrite_uses_tagged`), so local renames reach into the loop body and test,
mirroring the existing `while` handling. A do-while introduces no binding of its
own, so no special reservation is needed (unlike a catch parameter).

## [0.5.0] - 2026-06-20

### Added — CLOC19: local renaming across `try`/`catch`/`finally` (catch-param soundness)

The pass now recurses through `TryStatement` in every phase
(`process_tagged`, `stmt_has_function`, `collect_decl_occurrences_stmt`,
`collect_all_idents_stmt`, `rewrite_uses_tagged`), so local renames reach into the
protected block, catch handler, and finalizer. The catch `param` is treated as a
**reserved binding**:

* it is collected as an INELIGIBLE occurrence (never itself renamed), and
* it is added to the fresh-name avoid set, so no other local can be renamed to a
  name that collides with it.

A regression test pins the killer case: when the catch param is literally `a`,
the function's own param is renamed to `b` (not `a`) — proving a generated short
name never aliases the caught value. Without either guard the handler would
miscompile.

## [0.4.0] - 2026-06-16

### Added — local variables, not just parameters

`RenamePass` now renames not only leaf-function **parameters** but also their
function-body **`var`/`let`/`const` locals**:

```js
function f(input) { var doubled = input * 2; return doubled; }
//  ⇒  function f(a){var b=a * 2;return b}
```

A name is renamed only when it is **declared exactly once** in a leaf function
**and** its declaration's scope spans the whole function body — because the
renamer rewrites *every* in-body use of the name, so it is only sound when every
use provably resolves to that one binding:

- a **parameter** or a **`var`** is function-scoped (a `var` hoists, even from a
  nested block) → all in-body uses resolve to it → eligible;
- a **`let`/`const`** is block-scoped → eligible only when declared at the
  function-body **top level** (its block is then the whole body). A `let`/`const`
  nested inside an inner `{}`/`if`/loop/`switch`/`for`-init is **skipped**:
  the same identifier used *outside* that inner block resolves to an outer/global
  binding, and renaming "every use" would corrupt it. (This block-scope rule was
  added after a security review caught the count-alone rule as unsound.)
- a name declared **more than once** (a parameter also `var`'d, two block-scoped
  `let x`) is skipped — its uses could belong to distinct bindings.

- The collector that gated parameter renaming (`collect_decl_names`, a set) is
  now `collect_decl_occurrences` (an ordered `Vec` of `(name, eligible)` with
  duplicates) so the pass can count occurrences, track per-declaration scope
  eligibility, and assign fresh names in deterministic source order.
- Declaration sites are now rewrite targets: a `var`/`let`/`const` declarator id
  (including a `for (var i …)` init) is renamed alongside its uses.

- **8 new behavior tests**: local `var`, top-level `let`+`const`, parameter+local
  together, a `for`-loop var, the WHITESPACE_ONLY contrast, the load-bearing
  `skips_name_declared_twice` (two distinct `total` bindings not conflated), the
  soundness regression `skips_nested_block_scoped_let_used_outside_its_block`,
  and `renames_nested_var_because_function_scoped` (a `var` in a block IS
  eligible, unlike a `let`).

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
