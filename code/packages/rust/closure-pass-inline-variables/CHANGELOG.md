# Changelog

All notable changes to the `coding-adventures-closure-pass-inline-variables` crate will be documented in this file.

## [0.18.0] - 2026-09-25

### Added — optional chains resolve like plain ones (CLOC30)

Per [CLOC30](../../specs/CLOC30-optional-chain-resolution.md). The CLOC28
resolver walked `MemberExpression` spines only; it now walks optional ones
too, so every mixture of `.` and `?.` resolves the same way:

```js
var o = { a: { b: 1 } };
console.log(o?.a?.b);     // => console.log(1)
console.log(o.a?.b);      // => console.log(1)
console.log(o?.a.b);      // => console.log(1)

var a = [1, 2, 3];
console.log(a?.[1]);      // => console.log(2)
```

One ladder rung closes (60 → 59). The other four shapes had no rung at all —
they were found by probing the oracle *around* the rung rather than by the
rung itself, which is worth noting: a single rung marked one capability, and
the capability turned out to be five.

### Why `?.` needed no new soundness argument

`?.` short-circuits only when its object is `null` or `undefined`, and on a
path this pass accepts that cannot happen: the root is an object or array
literal, and `resolve` only continues through another one. So no `?.` on an
accepted path could have short-circuited, and it reads exactly as `.` would.
Every CLOC28 guard carries over untouched, because they live in `resolve` and
in candidate eligibility rather than in the spine walk.

Where a `?.` *would* short-circuit, the walk stops and nothing folds:

```js
var o = { a: null };
console.log(o?.a?.b);     // left alone; upstream folds it to void 0
```

Behind the oracle, never ahead of it.

### Internals

`chain_of` keeps its `MemberExpression` signature; a new `chain_of_expr`
accepts `MemberExpression`, `OptionalMemberExpression` and `ChainExpression`
at every step. `key_of` became `key_of_parts(computed, property)`, since both
member kinds carry the same pair.

7 tests added.

## [0.17.0] - 2026-09-25

### Added — propagate a scalar `let`/`var` to its use sites (CLOC29)

Per [CLOC29](../../specs/CLOC29-scalar-let-var-propagation.md), the second
slice of CCR-068 (#15837). A top-level scalar `let` or `var` now propagates
into its reads under the closed-world gate:

```js
var x = 1;
if (x) { console.log(1); } else { console.log(2); }   // => console.log(1);

var a = null;
console.log(a ?? 2);                                   // => console.log(2);
```

No new folding was needed. Constant folding already reduced `1 ? A : B` and
`null ?? 2`; the pass simply never put the literal there, because the scalar
path admitted `const` and nothing else. Four ladder rungs close, and the
ledger goes 64 → 60.

### Why the gate was not simply lifted

The scalar path sizes its rewrite with `count_uses_*`, which deliberately does
not count a bare identifier in assignment position. For a `const` that is
correct — it cannot be assigned. Admit `var` on that counter and:

```js
var X = 1;
X = 2;
console.log(X);          // prints 2; the tally sees ONE use and folds to 1
```

That is the same miscompile CLOC28 hit from the other direction, where the
write was invisible because `AssignmentTarget` is `#[serde(untagged)]`.

So a scalar `let`/`var` candidate is marked `strong_proof` and routed through
CLOC28's whole-program proof instead: count every mention over the serialized
AST, refuse on any write or self-reference in the initializer, rewrite on a
clone, and commit only if the declaration's own binding target is the single
surviving mention. Over-counting only declines, so unmodelled AST shapes fail
closed. `const` keeps the cheap counter it has always used.

### Renamed: `with_structured_literals` → `closed_world`

The flag now gates two capabilities that share one justification — CLOC28's
chain resolution and CLOC29's scalar propagation are both unsound in an
open world, because a top-level binding is a property of the global object.
Measured at both levels:

```text
                            SIMPLE                     ADVANCED
var x=1; console.log(x)     var x=1;console.log(x);    console.log(1);
```

`does_not_propagate_let_or_var` became
`does_not_propagate_let_or_var_in_the_open_world`, with a new
`does_propagate_let_or_var_in_the_closed_world` pinning the other side. The
old test asserted the blanket claim "`let`/`var` are never propagated", which
is no longer true at ADVANCED; pinning both configurations keeps the gate from
being widened or dropped without a test moving.

### Known blind spot, shared with upstream

`eval` can write a binding from inside a string, where no AST scan reaches it:

```js
var x = 1;
eval("x = 9");
console.log(x);      // node prints 9; we emit eval("x=9");console.log(1)
```

`v20260915` emits byte-identical output, so this is exact parity, not a
divergence — Closure at ADVANCED documents that it does not support `eval`
modifying locals. Recorded so it is not rediscovered as a bug.

### Fixed — `with` could make propagation unsound, on any binding kind

Found in review. A `with` block resolves identifiers against a runtime object,
so `var x=1; with(JSON.parse('{"x":9}')) console.log(x)` prints `9`, and
folding `x` to `1` is wrong. The oracle rejects `with` outright, but closurec
compiles it, so the exposure was ours. The pass now declines every candidate in
a program containing a `WithStatement` — including on the `const` path, where
the hole predates this release.

### Fixed — the strong-proof path bypassed the multi-use size budget

Also found in review, and new in this release. The `strong_proof` branch
returned before the `literal_cost(..) > MAX_MULTIUSE_LITERAL_LEN` check, so a
long literal read at several sites was duplicated into all of them — meaning a
`var` optimized *worse* than the same program written with `const`, which is
backwards. The budget now applies to a scalar strong-proof candidate too. A
structured candidate stays exempt: what it plants is the scalar a chain
resolved to, never the literal.

### Fixed — propagation was quadratic in the number of candidates

New in this release and the reason it mattered: `propagate_structured` did two
whole-program `serde_json::to_value` calls plus a full `Program::clone()` for
every candidate. CLOC28 only sent rare object/array literals down that path;
CLOC29 sends every top-level scalar `let`/`var`, which is the common case.

Measured on N separate top-level bindings each read once, at ADVANCED:

| N | before | after | `const` baseline |
|---|---|---|---|
| 200 | 0.57s | 0.26s | 0.14s |
| 800 | 10.56s | 1.89s | 0.65s |

The program is now serialized once per sweep and shared across candidates, and
the post-rewrite count is derived arithmetically — `propagate_all` returns how
many sites it rewrote, and each rewrite removes exactly one mention, so
`remaining == total - replaced`. The residual growth is the per-candidate
clone, tracked separately.

10 tests added.

## [0.16.0] - 2026-09-24

### Added — resolve a member chain against an object/array literal (CLOC28)

Per [CLOC28](../../specs/CLOC28-literal-property-propagation.md), the first
slice of CCR-068 (#15837). Where the scalar path substitutes a `const`'s
literal at each bare use of its name, this resolves a member *chain* rooted at
a binding whose initializer is an object or array literal, and replaces the
whole chain with the scalar it reads:

```js
var o = { a: { b: { c: 1 } } };
console.log(o.a.b.c);            // => console.log(1)
```

`remove-unused-vars` then deletes the binding, as it already did for the
scalar case. No new pass and no new pipeline slot.

The unit of replacement is the chain rather than the identifier on purpose.
Substituting the literal itself would construct a fresh object at every use
site, so `o.a === o.a` would turn from `true` into `false`. What lands at the
use site here is always a scalar — number, string, boolean, `null`, bigint —
and scalars have no identity to preserve. A chain resolving to an object is
walked *through*, never substituted.

### The near-miss worth recording

The structured path admits `var` and `let`, where the scalar path is
`const`-only. That combination came within one test of shipping a miscompile:

```js
var o = { a: 1 };
o = { a: 2 };
console.log(o.a);     // prints 2; an early draft folded it to console.log(1)
```

Two independent blind spots lined up. `count_uses_*` does not count a bare
identifier assignment target — correctly, for a `const`, which cannot be
assigned, so nothing in the scalar path ever needed it. And the first
eligibility gate counted occurrences by looking for `"type":"Identifier"` in
the serialized AST, which does not appear for an assignment target because
`AssignmentTarget` is `#[serde(untagged)]`. The write was invisible twice over,
the single remaining read resolved, and the fold looked legitimate.

The gate now counts every object carrying `"name": "<name>"` regardless of
enum tagging, and accepts a rewrite only when the sole remaining mention is the
declaration itself. Over-counting merely declines a candidate, so unfamiliar
AST shapes fail closed. `refuses_a_binding_that_is_reassigned` pins it.

### Guards, each checked against the pinned oracle

An escaping object (`sink(o)`), a written property (`o.a = 2`, `o.a++`,
`delete o.a`), an absent key, and a chain resolving to a non-scalar all leave
the binding entirely alone — including its other, resolvable reads.
`v20260915` declines in the same places: it renames rather than folds an
escaped object, and it keeps `{}.b` and `typeof{}.toString` rather than
folding a missing key to `undefined`, because such a read can resolve up the
prototype chain.

Deliberately behind upstream, and recorded in CLOC28 rather than left to be
rediscovered: `a.length`, an out-of-range index, an array hole, a spread in
the literal, and a getter are all folded by upstream and declined here.

### Two miscompiles caught in review, before push

**An array spread before the index.** The resolver declined only when the
element *at* the index was a spread. A spread earlier in the literal
contributes an unknown number of elements, so every later position shifts:

```js
var a = [..."xy", 5];
console.log(a[1]);        // "y"; the fold answered 5
```

The object arm bails on any spread at all; the array arm quietly did not. The
guard is now positional — a spread at or before the index declines, one after
it does not, because `[1, 2, ...x][0]` is still `1` and upstream folds that
too. Note `[...[7,8],5]` does not reproduce it: an earlier pass pre-folds an
array-spread-of-an-array-literal, so the spread source has to be a string, an
iterator or a call.

**A self-reference inside the candidate's own initializer.** `propagate_all`
rewrites chains anywhere in the program, the declarator's `init` included:

```js
var a = [1, a[0], 5];
console.log(a[2]);        // TypeError, always; the fold printed 5
```

While the literal is evaluated the binding is still `undefined`, so `a[0]`
throws every time. Rewriting it erased the throw *and* removed the occurrence
that would otherwise have pushed the mention count above one and rejected the
candidate. `prefix_is_inert` cannot help — the declaration is the first item,
so there is no prefix. A candidate whose initializer mentions its own name is
now declined outright.

20 tests added.

## [0.15.1] - 2026-07-19

### Changed — test goldens updated for `closure-emitter` 0.55.0

`closure-emitter` 0.55.0 terminates a top-level function/class declaration with
`;` only when it is the last program item. This pass's emit-shape test goldens
were regenerated to the new byte-identical-to-Closure output. No behaviour change
in this crate — only the expected emitted strings moved.

## [0.15.0] - 2026-07-14

### Changed — handle `FunctionParam::RestElement` — CLOC12.190 PR1

Picks up javascript-ast 0.41.0. Handles the new `FunctionParam::RestElement` variant via
`binding_identifier()`, so a rest parameter (`...name`) is walked as an ordinary single-name binding
(counted / looked up / renamed) rather than being unrepresentable. Additive; MINOR.

## [0.14.0] - 2026-07-12

### Added — CLOC12.189 PR1: export declaration the inertness predicate reports not-inert and the count/propagate walks skip exports

Exhaustive-match arms for the three new `Declaration::Export*` variants
(`ExportNamedDeclaration` / `ExportDefaultDeclaration` / `ExportAllDeclaration`).
PR1 keeps the nodes unreachable (no bridge yet), so the arms are conservative —
the inertness predicate reports not-inert and the count/propagate walks skip exports. Proper descent into an `export const x = 1`'s inner declaration and the
renaming-soundness gate land with the bridge PR.

## [0.13.0] - 2026-07-11

### Added — CLOC12.188 PR1: `ImportDeclaration` arms

Exhaustive-match arms for the new `Declaration::ImportDeclaration` variant: the
inertness predicate reports an import is not inert (it has runtime effect), and
the count/propagate walks skip it as a no-op.

## [0.12.0] - 2026-07-11

### Added — CLOC12.187 PR1: traverse `WithStatement`

New `TaggedStatement::WithStatement` arms in the decl-name counter, use counter,
and const-propagation walk descend into the `with` object and body. Picks up
javascript-ast 0.38.0.

## [0.11.16] - 2026-07-11

### Added — CLOC12.176 PR1: `ClassMember::StaticBlock` arm

`javascript-ast` 0.35.0 added `ClassMember::StaticBlock(BlockStatement)`, the third class member (a `static { … }` initialization block). Added `StaticBlock` arms at all 5 sites: count/propagate recurse the block's statements (SOUNDNESS: a candidate use inside a static block runs at class-def time and must be counted before the const is propagated there); a static block declares no class-body name.

## [0.11.15] - 2026-07-11

### Added — CLOC12.175 PR1: `ClassMember::Field` arms

`javascript-ast` 0.34.0 added `ClassMember::Field`. Added `Field` handling at
every class member site: count/collect skip the field key (no statement-scope
binding) but recurse the initializer and computed key, and propagate substitutes
into the initializer in lockstep — so a candidate use inside a field initializer
is counted before it can be propagated. Reachable once the CLOC12.175 PR2 bridge
produces the node.

## [0.11.14] - 2026-07-10

### Added — CLOC12.174 PR1: `Declaration::ClassDeclaration` match arms

`javascript-ast` 0.33.0 added the `Declaration::ClassDeclaration` variant. Added
arms at each exhaustive `Declaration` match site: `decl_is_inert` returns `false`
(a class declaration runs code — its `extends` heritage is evaluated at the
declaration site, unlike a hoisted function declaration); `count_decl_names_decl`
counts the class name + method-body names; and `count_uses_decl` / `propagate_in_decl`
recurse the heritage operand + method bodies in lockstep (missing a use would let
a still-referenced const be inlined away — a miscompile). Reachable once the
CLOC12.174 PR2 bridge produces the node.

## [0.11.13] - 2026-07-08

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

## [0.11.12] - 2026-07-07

### Changed — CLOC12.169: `ImportExpression` exhaustive-match arm

Added an `Expression::ImportExpression` case so the pass stays exhaustive over the new `javascript-ast` single-operand variant (part of the CLOC12.169 atomic node PR1). A dynamic `import(source)` carries one sub-expression (the module specifier), so the pass recurses into `source` exactly like the sibling `AwaitExpression` arm. No behaviour change to any existing node.


## [0.11.11] - 2026-07-07

### Changed — CLOC12.168: `ImportMeta` exhaustive-match arm

Added an `Expression::ImportMeta` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.168 atomic node PR1). `import.meta` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals. No behaviour change to any existing node.


## [0.11.10] - 2026-07-04

### Changed — CLOC12.167: `NewTarget` exhaustive-match arm

Added an `Expression::NewTarget` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.167 atomic node PR1). `new.target` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.11.9] - 2026-07-04

### Changed — CLOC12.166: `Super` exhaustive-match arm

Added an `Expression::Super` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.166 atomic node PR1). `super` is a leaf keyword with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.11.8] - 2026-07-04

### Changed — CLOC12.165: `ThisExpression` exhaustive-match arm

Added an `Expression::ThisExpression` no-op arm so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.165 atomic node PR1). `this` binds and references no identifier and has no sub-expression, so the traversal does nothing for it. No behaviour change to any existing node.


## [0.11.7] - 2026-07-04

### Changed — CLOC12.164: `AwaitExpression` traversal arms

Added `Expression::AwaitExpression` arms that walk into the await's `argument`,
so the pass stays exhaustive over the new `javascript-ast` variant (part of the
CLOC12.164 atomic node PR1). No behaviour change to any existing node; the await
argument is now visited exactly like any other sub-expression the pass already
handles.


## [0.11.6] - 2026-07-03

### Changed — CLOC12.163: `YieldExpression` traversal arms

Added `Expression::YieldExpression` arms (2 across the pass's traversal
routines) that walk into the yield's optional `argument` when present, so the
pass stays exhaustive over the new `javascript-ast` variant (part of the
CLOC12.163 atomic node PR1). No behaviour change to any existing node; the
yield argument is now visited exactly like any other sub-expression the pass
already handles.


## [0.11.5] - 2026-07-03

### Changed — CLOC12.162: `SpreadElement` exhaustive-match arm

Added an `Expression::SpreadElement` arm that recurses into the spread's
`argument` so the pass stays exhaustive over the new `javascript-ast` variant
(part of the CLOC12.162 atomic node PR1). No behaviour change to any existing
node; the spread argument is now visited/rewritten exactly like any other
sub-expression the pass already handles.

## [0.11.4] - 2026-07-02

### Changed — CLOC12.161: handle `Expression::TaggedTemplateExpression`

Added a `TaggedTemplateExpression` match arm recursing into the `tag` callee
and each `${…}` insert of the applied template, so this pass keeps
compiling and traverses the new `javascript-ast` 0.20.0 node. No behaviour
change for any existing node.

## [0.11.3] - 2026-07-02

### Changed — CLOC12.160: handle `Expression::SequenceExpression`

Added a `SequenceExpression` match arm recursing into each operand so this
crate compiles and traverses the new `Expression::SequenceExpression`
variant. No behaviour change until the bridge produces sequence nodes
(CLOC12.160 PR2).


## [0.11.2] - 2026-07-02

### Changed — CLOC12.159: handle `Expression::NewExpression`

Added a `NewExpression` match arm mirroring `CallExpression` (recurse into the
callee and each argument) so this crate compiles and traverses the new
`Expression::NewExpression` variant. No behaviour change until the bridge
produces `new` nodes (CLOC12.159 PR2).


## [0.11.1] - 2026-07-02

### Changed — CLOC12.158: exhaustiveness for new `Expression::UpdateExpression`

Handle the new `Expression::UpdateExpression` (`++x` / `x++` / `--x` / `x--`)
variant added to `javascript-ast` (0.17.0): the pass recurses into the operand for use-counting and propagation. No behaviour
change for existing inputs — the bridge does not yet produce update
expressions (that lands in the CLOC12.158 PR2 bridge-enable), so these arms
are exercised only via hand-constructed AST today.

## [0.11.0] - 2026-07-02

### Added — CLOC12.154: `TemplateLiteral` traversal

Handle the new `Expression::TemplateLiteral` variant by recursing into its `${{…}}` sub-expressions (the `expressions` vector); the `quasis` are fixed leaf string segments with nothing to recurse. Part of the atomic `TemplateLiteral` enum-variant rollout (javascript-ast 0.16.0) — adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR. Template literals introduce no bindings or scopes, so the renaming/inlining arms need no map reduction.

## [0.10.0] - 2026-07-02

### Added — CLOC12.151: `ArrowFunctionExpression` traversal

Handle the new `Expression::ArrowFunctionExpression` variant by recursing into arrow bodies — both the block form (`x => { ... }`) and the concise/expression form (`x => expr`) — mirroring this pass's existing `FunctionExpression` handling. Part of the atomic `ArrowFunctionExpression` enum-variant rollout (javascript-ast 0.15.0); adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR.

## [0.9.0] - 2026-07-01

### Added — CLOC12.149: propagate through `FunctionExpression` bodies

`count_uses_expr` and `propagate_in_expr` now recurse into a
`FunctionExpression` body (via the `_stmt` helpers), keeping the use
count and the substitution walk over the same positions. Over-counting
under param/self-name shadowing is conservative — it only declines an
inline, never performs a wrong one.

## [0.8.0] - 2026-07-01

### Added — upstream `InlineVariablesTest.java` conformance port (#88, CLOC12.146)

The **first** CLOC12 upstream-test port into this crate. New file
`tests/upstream/inline_variables_test.rs` (registered as the
`upstream_inline_variables` test target) reshapes upstream
`InlineVariablesTest.java` onto our surface, driving the **real** source →
`grammar_to_program` bridge → `InlineVariablesPass` → `emit` roundtrip, so each
case is `assert_eq!(propagate(src), expected)` on emitted JS.

- **13 active `#[test]`s pass on the first run** (no new propagation defect):
  single-use const-literal propagation, propagation into a larger expression,
  a short literal duplicated across multiple sites, boolean/null literals, the
  multi-use size budget (a long literal is declined at multiple sites but
  propagated at a single site), `let`/`var` never propagated, non-literal
  initializers declined, the shadowed-name guard, property names never
  replaced while computed member indices are, and two TDZ soundness cases
  (inert-prefix propagates; code-before-declaration declines).
- **3 `#[ignore = "blocked on gap-NNN"]` placeholders** pin the whole-program
  `InlineVariables` behaviors closurec does not do in this pass —
  gap-148 (single-assignment `let`/`var` inlining), gap-149 (identifier-alias
  initializers), gap-150 (removing the dead `const` husk, which
  `remove-unused-vars` owns). Each is pinned to `code/specs/CLOC12-gaps.md`.

This is a **test-only** change: no `src/` file is touched, so there is no
ripple into downstream consumers. Scaffolding files
`tests/upstream/{UPSTREAM_SHA,ATTRIBUTION.md}` were added per the CLOC12 port
convention.

## [0.7.0] - 2026-07-01

### Added — CV provenance for constant propagation (#89)

The pass now records every constant it propagates as a `propagated`
correlation-vector contribution carrying `{name, value, sites}` — the original
`const` name, a compact rendering of its literal value, and how many use sites
the literal replaced. Propagation *dissolves* the binding: its declaration
becomes unreferenced (remove-unused-vars deletes it) and the literal is copied
to each reader, so without this record the minified output has no trace that a
named constant ever stood there. These contributions let a `--correlation_vector`
consumer map an inlined literal back to the `const` it came from.

- Records emit in program (source) order, one per propagated constant, so the
  contribution list is deterministic run to run.
- `value` renders numbers/bigints from their raw text, strings quoted, and
  `true`/`false`/`null`/`undefined` literally.
- Attached at the program root — a coarse name→value/site-count *table*. Tagging
  each substituted literal's own CV id is a documented follow-up, mirroring the
  inline / rename passes.
- Emitted JS is byte-identical: contributions are pure metadata. Verified by the
  full closurec end-to-end suite.

`coding_adventures_correlation_vector` moves from a dev-dependency to a runtime
dependency (the pass now names `Contribution`), and `serde_json` is added for
the `json!` meta values. Three new unit tests cover a single-use propagation
(`sites: 1`), a multi-use propagation (`sites: 2`), and the no-propagation
(`let`, empty table) case.

## [0.6.1] - 2026-06-30

### Changed — test sync for closure-emitter boolean shorthand

`closure-emitter` 0.18.9 now minifies `true`/`false` to `!0`/`!1`. The
`propagates_boolean_and_null_literals` golden-output test was updated to
expect the new rendering (`const ON=!0;const NONE=null;f(!0,null);`). No
behavior change in this crate — the propagation logic is unchanged.

## [0.6.0] - 2026-06-20

### Added — CLOC23: variable inlining inside `for`-`of`

`count_decl_names_stmt`, `count_uses_stmt`, and `propagate_in_stmt` now recurse
through `ForOfStatement`, counting the `left` declaration as the loop-variable
binding — identical to the `for`-`in` handling.

## [0.5.0] - 2026-06-20

### Added — CLOC22: variable inlining inside `for`-`in`

`count_decl_names_stmt`, `count_uses_stmt`, and `propagate_in_stmt` now recurse
through `ForInStatement`. The for-in `left`, when a declaration, is counted as a
binding (the loop variable), mirroring the for-statement init handling.

## [0.4.0] - 2026-06-20

### Added — CLOC21: handle `DebuggerStatement`

The statement walks (`count_decl_names_stmt`, `count_uses_stmt`,
`propagate_in_stmt`) now cover `DebuggerStatement` (grouped with the other
childless leaf statements) as a no-op. Added to keep the matches exhaustive over
the new AST variant.

## [0.3.0] - 2026-06-20

### Added — CLOC20: variable inlining inside `do`/`while`

`count_decl_names_stmt`, `count_uses_stmt`, and `propagate_in_stmt` now recurse
through `DoWhileStatement` (loop body and test), mirroring the existing `while`
handling so const-literal propagation reaches into do-while loops.

## [0.2.0] - 2026-06-20

### Added — CLOC19: variable inlining inside `try`/`catch`/`finally`

`count_decl_names_stmt`, `count_uses_stmt`, and `propagate_in_stmt` now recurse
through `TryStatement` (protected block, catch handler body, finalizer). The catch
`param` is counted as a declared binding in `count_decl_names_stmt` so a candidate
that shadows it is correctly excluded from propagation — preserving soundness when
a top-level name is also bound by a catch clause.

## [0.1.0] - 2026-06-17

### Added (CLOC13.H — constant propagation)

New crate per CLOC06's canonical pass set — Closure Compiler's `InlineVariables`
in miniature. `InlineVariablesPass::run` propagates a **top-level `const` bound
to a literal** to all of its use sites:

```js
const RATE = 2;
total = base * RATE;
// =>  const RATE = 2;   (now unreferenced — removed by remove-unused-vars)
//     total = base * 2;
```

- `InlinePass`-style metadata: `name = "inline-variables"`,
  `depends_on = ["constant-fold"]` (so a folded initializer `const X = 1 + 2`
  → `const X = 3` is a literal by the time we look), `iteration_policy =
  FixedPoint`, `cost = 3`.
- **Soundness** rests on three restrictions, plus the inline pass's
  self-contained shadow guard (the name must be declared exactly once in the
  whole program):
  - **`const` only** — a `let`/`var` can be reassigned between its declaration
    and a use, so its initializer is not a safe substitute. `const` cannot.
  - **literal values only** — a literal is immutable. `const X = y;` (an
    identifier whose value could later change) and `const X = o.p;` (a member
    read that could trigger a getter) are NOT propagated.
  - **temporal-dead-zone guard** — a `const` read before its declaration line
    runs throws `ReferenceError` (even from a function called early). We only
    propagate when every top-level item *before* the declaration is inert (a
    function declaration, or a variable declaration with only literal
    initializers), so nothing executes — and nothing can read the binding in
    its TDZ — before it initializes. Only single-declarator `const`s are taken.
- **Single-use** → always propagated (the whole `const` declaration becomes
  pure overhead once its one use is gone). **Multi-use** → propagated only when
  the literal's emitted form is short (`<= MAX_MULTIUSE_LITERAL_LEN`, 8 bytes),
  so duplicating it across the uses is outweighed by deleting the declaration.
- The pass only **propagates**; it leaves the emptied `const` declaration for
  `remove-unused-vars` to delete (mirrors how the inline pass leaves dead
  functions for treeshake). Property names (non-computed `.x` / object keys)
  and assignment targets are never substituted; computed `o[X]` is.
- Self-contained name-based analysis over the Phase-1 AST (same philosophy as
  the `inline` and `rename` passes); does not depend on `closure-scope-analyzer`.

### Tests
- 19 tests: metadata/pipeline-ordering contract + source → bridge →
  inline-variables → emit roundtrips covering single/multi-use propagation, the
  multi-use literal-size budget, and every rejection (let/var, non-literal
  value, shadowed name, property name, computed member).
