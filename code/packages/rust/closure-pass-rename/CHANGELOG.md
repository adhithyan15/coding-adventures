# Changelog

All notable changes to the `coding-adventures-closure-pass-rename` crate will be documented in this file.

## [0.22.0] - 2026-07-14

### Changed — rename references inside default-parameter expressions — CLOC12.191 PR1

Picks up javascript-ast 0.42.0. A default parameter’s `right` (`function f(x, y = x){}`) is live code: the
apply step now rewrites its uses through the rename map so a reference tracks its renamed binding, and the
new `collect_param_idents` helper adds a default’s identifiers to the collision-avoidance set so a fresh
short name never shadows a name a default reads. Routed through every param-collection site (leaf function,
nested function/arrow, class method).

## [0.21.0] - 2026-07-14

### Changed — handle `FunctionParam::RestElement` — CLOC12.190 PR1

Picks up javascript-ast 0.41.0. Handles the new `FunctionParam::RestElement` variant via
`binding_identifier()`, so a rest parameter (`...name`) is walked as an ordinary single-name binding
(counted / looked up / renamed) rather than being unrepresentable. Additive; MINOR.

## [0.20.0] - 2026-07-12

### Changed — CLOC12.189 PR2: bail on module `export`

The local-renaming soundness gate now also declines when
`program_contains_export_declaration` is true, mirroring the existing `import`
and `with` gates — an exported binding is this module's public surface and must
not be renamed.

## [0.19.0] - 2026-07-12

### Added — CLOC12.189 PR1: export declaration the predicates report no function payload and the local-name walks skip exports

Exhaustive-match arms for the three new `Declaration::Export*` variants
(`ExportNamedDeclaration` / `ExportDefaultDeclaration` / `ExportAllDeclaration`).
PR1 keeps the nodes unreachable (no bridge yet), so the arms are conservative —
the predicates report no function payload and the local-name walks skip exports. Proper descent into an `export const x = 1`'s inner declaration and the
renaming-soundness gate land with the bridge PR.

## [0.18.0] - 2026-07-12

### Changed — CLOC12.188 PR2: bail on module `import`

The local-renaming soundness gate now also declines (returns the input unchanged)
when `program_contains_import_declaration` is true, mirroring the existing `with`
gate. Renaming an import binding — or colliding a renamed local with an import
name — would be unsound; a module with imports keeps its other optimizations but
skips renaming.

## [0.17.0] - 2026-07-11

### Added — CLOC12.188 PR1: `ImportDeclaration` arms

Exhaustive-match arms for the new `Declaration::ImportDeclaration` variant: the
process/`has_function` predicates report no function payload, and the local-name
walks skip the import (no descent). Renaming an import-introduced binding is
unsound — an imported local references a foreign export — so the full soundness
gate lands with the bridge PR; PR1 simply does not descend into imports.

## [0.16.0] - 2026-07-12

### Added — CLOC12.187 PR2a: decline to rename in the presence of `with`

`run` now bails at the top when `program_contains_with_statement` (new in
closure-scope-analyzer 0.14.0) is `true`, returning the input program unchanged
with no rename contributions. A `with (obj) …` splices `obj` onto the scope
chain, so a bare name in its body may resolve to an `obj` property rather than
the lexical binding the pass sees — renaming would then be unsound (the
"single declaration ⇒ single binding" safety argument does not hold). `with` is
a strict-mode syntax error and rare, so this program-wide bail costs little.
New `with_statement_disables_local_renaming` test. Sets up the `with` bridge
(PR2b): once the bridge produces the node, this gate keeps renaming sound.

## [0.15.0] - 2026-07-11

### Added — CLOC12.187 PR1: traverse `WithStatement`

New `TaggedStatement::WithStatement` arms in every local-rename walk (apply,
has-function probe, decl-occurrence collection, all-ident collection, and
use-rewrite) descend into the `with` object and body. Because the node is not
yet bridge-reachable, this simply keeps the exhaustive matches total; the
renaming-soundness handling that a live `with` demands lands with the bridge PR.
Picks up javascript-ast 0.38.0.

## [0.14.16] - 2026-07-11

### Added — CLOC12.176 PR1: `ClassMember::StaticBlock` arm

`javascript-ast` 0.35.0 added `ClassMember::StaticBlock(BlockStatement)`, the third class member (a `static { … }` initialization block). Added `StaticBlock` arms (collect + rewrite, decl + expression): over-collect identifiers in the block's statements so a fresh short name never collides, and rewrite renamed locals with the class-inner map.

## [0.14.15] - 2026-07-11

### Added — CLOC12.175 PR1: `ClassMember::Field` arms

`javascript-ast` 0.34.0 added `ClassMember::Field`. Added `Field` arms in the
class-declaration and class-expression collect/rewrite walks: the field key is a
property name (left untouched), while renamed locals in the initializer and
computed key are rewritten with the class-inner shadow map. Reachable once the
CLOC12.175 PR2 bridge produces the node.

## [0.14.14] - 2026-07-10

### Added — CLOC12.174 PR1: `Declaration::ClassDeclaration` match arms

`javascript-ast` 0.33.0 added the `Declaration::ClassDeclaration` variant. Added
arms at each exhaustive match site: `process_stmt` → `false` (a class is not a
leaf top-level function this pass renames); `stmt_has_function` → `true`
(a class carries method functions, so the leaf-binding rename is conservatively
disabled in its presence); `collect_decl_occurrences_stmt` marks the class name
ineligible (like a function name); and the soundness-critical
`collect_all_idents_stmt` / `rewrite_uses_stmt` recurse the class name, heritage,
and every method body (over-collect + shadow-aware rewrite, mirroring the
`Expression::ClassExpression` arms). Reachable once the CLOC12.174 PR2 bridge
produces the node.

## [0.14.13] - 2026-07-08

### Added — CLOC12.173 PR1: `ClassExpression` match arm (mirrors `FunctionExpression`)

`javascript-ast` 0.32.0 added the `Expression::ClassExpression` variant, which made
this crate’s exhaustive `Expression` match(es) non-exhaustive. Added a
`ClassExpression` arm at each site, mirroring the crate’s existing
`FunctionExpression` handling: recurse into the `extends` operand (a normal
expression) and each method’s `value` (a `FunctionExpression`, walked as its own
function scope). Variable-renaming passes leave method KEYS untouched (a method
key is a property name, not a variable); the property-renaming pass treats method
keys as renameable property names, mirroring object-literal keys. Rebuild/
transform arms delegate to an `#[inline(never)]` helper (frame-size DoS lesson).
Reachable once the CLOC12.173 PR2 bridge produces `ClassExpression` nodes.

## [0.14.12] - 2026-07-07

### Changed — CLOC12.169: `ImportExpression` exhaustive-match arm

Added an `Expression::ImportExpression` case so the pass stays exhaustive over the new `javascript-ast` single-operand variant (part of the CLOC12.169 atomic node PR1). A dynamic `import(source)` carries one sub-expression (the module specifier), so the pass recurses into `source` exactly like the sibling `AwaitExpression` arm. No behaviour change to any existing node.


## [0.14.11] - 2026-07-07

### Changed — CLOC12.168: `ImportMeta` exhaustive-match arm

Added an `Expression::ImportMeta` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.168 atomic node PR1). `import.meta` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals. No behaviour change to any existing node.


## [0.14.10] - 2026-07-04

### Changed — CLOC12.167: `NewTarget` exhaustive-match arm

Added an `Expression::NewTarget` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.167 atomic node PR1). `new.target` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.14.9] - 2026-07-04

### Changed — CLOC12.166: `Super` exhaustive-match arm

Added an `Expression::Super` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.166 atomic node PR1). `super` is a leaf keyword with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.14.8] - 2026-07-04

### Changed — CLOC12.165: `ThisExpression` exhaustive-match arm

Added an `Expression::ThisExpression` no-op arm so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.165 atomic node PR1). `this` binds and references no identifier and has no sub-expression, so the traversal does nothing for it. No behaviour change to any existing node.


## [0.14.7] - 2026-07-04

### Changed — CLOC12.164: `AwaitExpression` traversal arms

Added `Expression::AwaitExpression` arms that walk into the await's `argument`,
so the pass stays exhaustive over the new `javascript-ast` variant (part of the
CLOC12.164 atomic node PR1). No behaviour change to any existing node; the await
argument is now visited exactly like any other sub-expression the pass already
handles.


## [0.14.6] - 2026-07-03

### Changed — CLOC12.163: `YieldExpression` traversal arms

Added `Expression::YieldExpression` arms (2 across the pass's traversal
routines) that walk into the yield's optional `argument` when present, so the
pass stays exhaustive over the new `javascript-ast` variant (part of the
CLOC12.163 atomic node PR1). No behaviour change to any existing node; the
yield argument is now visited exactly like any other sub-expression the pass
already handles.


## [0.14.5] - 2026-07-03

### Changed — CLOC12.162: `SpreadElement` exhaustive-match arm

Added an `Expression::SpreadElement` arm that recurses into the spread's
`argument` so the pass stays exhaustive over the new `javascript-ast` variant
(part of the CLOC12.162 atomic node PR1). No behaviour change to any existing
node; the spread argument is now visited/rewritten exactly like any other
sub-expression the pass already handles.

## [0.14.4] - 2026-07-02

### Changed — CLOC12.161: handle `Expression::TaggedTemplateExpression`

Added a `TaggedTemplateExpression` match arm recursing into the `tag` callee
and each `${…}` insert of the applied template, so this pass keeps
compiling and traverses the new `javascript-ast` 0.20.0 node. No behaviour
change for any existing node.

## [0.14.3] - 2026-07-02

### Changed — CLOC12.160: handle `Expression::SequenceExpression`

Added a `SequenceExpression` match arm recursing into each operand so this
crate compiles and traverses the new `Expression::SequenceExpression`
variant. No behaviour change until the bridge produces sequence nodes
(CLOC12.160 PR2).


## [0.14.2] - 2026-07-02

### Changed — CLOC12.159: handle `Expression::NewExpression`

Added a `NewExpression` match arm mirroring `CallExpression` (recurse into the
callee and each argument) so this crate compiles and traverses the new
`Expression::NewExpression` variant. No behaviour change until the bridge
produces `new` nodes (CLOC12.159 PR2).


## [0.14.1] - 2026-07-02

### Changed — CLOC12.158: exhaustiveness for new `Expression::UpdateExpression`

Handle the new `Expression::UpdateExpression` (`++x` / `x++` / `--x` / `x--`)
variant added to `javascript-ast` (0.17.0): the pass recurses into the operand for ident collection and use rewriting. No behaviour
change for existing inputs — the bridge does not yet produce update
expressions (that lands in the CLOC12.158 PR2 bridge-enable), so these arms
are exercised only via hand-constructed AST today.

## [0.14.0] - 2026-07-02

### Added — CLOC12.154: `TemplateLiteral` traversal

Handle the new `Expression::TemplateLiteral` variant by recursing into its `${{…}}` sub-expressions (the `expressions` vector); the `quasis` are fixed leaf string segments with nothing to recurse. Part of the atomic `TemplateLiteral` enum-variant rollout (javascript-ast 0.16.0) — adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR. Template literals introduce no bindings or scopes, so the renaming/inlining arms need no map reduction.

## [0.13.0] - 2026-07-02

### Added — CLOC12.151: `ArrowFunctionExpression` traversal

Handle the new `Expression::ArrowFunctionExpression` variant by recursing into arrow bodies — both the block form (`x => { ... }`) and the concise/expression form (`x => expr`) — mirroring this pass's existing `FunctionExpression` handling. Part of the atomic `ArrowFunctionExpression` enum-variant rollout (javascript-ast 0.15.0); adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR. The arrow's params are removed from the active rename/substitute map before recursing into the body, so a param that shadows an outer binding of the same spelling is left untouched (arrows have no self-name to shadow).

## [0.12.0] - 2026-07-01

### Added — CLOC12.149: rename locals through `FunctionExpression`

`collect_all_idents_expr` records a nested function value's name +
params + body idents (fresh-name avoidance). `rewrite_uses_expr` recurses
into the body with the function's own name/params removed from the map,
so a closure-over use of a renamed outer local is rewritten while a
shadowed use keeps its inner name.

## [0.11.0] - 2026-07-01

### Added — upstream `RenameVarsTest.java` conformance port (#88, CLOC12.145)

The **first** CLOC12 upstream-test port into this crate. New file
`tests/upstream/rename_vars_test.rs` (registered as the `upstream_rename_vars`
test target) reshapes upstream `RenameVarsTest.java` onto our surface, driving
the **real** source → `grammar_to_program` bridge → `RenamePass` → `emit`
roundtrip — the exact chain closurec's SIMPLE level uses — so each case is
`assert_eq!(rename(src), expected)` on emitted JS.

- **12 active `#[test]`s pass on the first run** (no new rename defect): leaf
  parameter and local `var`/`let`/`const` renaming at declaration + every use
  site; multiple params distinct; param+local together; reserved-name
  avoidance (a fresh short name never captures a referenced free global);
  property access and non-computed object keys never renamed; computed member
  index renamed; single-char names left alone; and two soundness guards (catch
  bindings reserved, and a name declared twice is skipped).
- **4 `#[ignore = "blocked on gap-NNN"]` placeholders** pin the whole-program
  `RenameVars` behaviors closurec deliberately splits out or defers —
  gap-144 (globals, owned by `rename-globals`), gap-145 (non-leaf function
  params), gap-146 (function declaration names), gap-147 (frequency-biased
  name allocation). Each is pinned to `code/specs/CLOC12-gaps.md`.

This is a **test-only** change: no `src/` file is touched, so there is no
emitter/pass ripple into downstream consumers. Scaffolding files
`tests/upstream/{UPSTREAM_SHA,ATTRIBUTION.md}` were added per the CLOC12 port
convention.

## [0.10.0] - 2026-06-30

### Added — correlation-vector rename provenance (#89)

The pass now records every local α-rename as a `renamed` correlation-vector
contribution carrying `{scope, from, to}` — the enclosing leaf function's name,
the original binding name, and its short form. Renaming is a transformation, not
a deletion, so (like the rename-globals / rename-properties passes) the pass
contributes a `renamed` record rather than a tombstone.

- The **`scope`** qualifier matters here in a way it does not for globals: local
  short names are allocated fresh *per function*, so the same `to` (`a`) recurs
  across functions. `scope` is what lets a `--correlation_vector` consumer map a
  minified local back to the right original binding.
- Records come out in `(function source order, then binding declaration order,
  ties broken by original name)`, so the emitted list is deterministic run to
  run.
- Program output is byte-for-byte unchanged: contributions are pure metadata.
  Verified by the full closurec end-to-end suite.
- This is the rename *table*; per-output-span provenance (contributing to each
  renamed identifier's own CV id) needs the log threaded through
  `rewrite_uses_block` — a documented follow-up mirroring the other rename
  passes.

Three new unit tests cover a single local rename, the `scope` qualifier
distinguishing the same original name across two functions, and the no-rename
(empty table) case.

## [0.9.0] - 2026-06-20

### Added — CLOC23: local renaming across `for`-`of` (loop-variable soundness)

Every phase of the pass recurses through `ForOfStatement`, treating the loop
variable identically to `for`-`in`: for `for (var/let/const v of it)` the `left`
binding is recorded as a rename occurrence and its declared name is rewritten via
the rename map, so the binding and its uses inside the body rename *consistently*.
Verified end-to-end: `for (var entry of values)` with `sum + entry` →
`for (var c of a)` with `b + c` under ADVANCED.

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
