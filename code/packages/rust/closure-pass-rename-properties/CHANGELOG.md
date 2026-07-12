# Changelog

All notable changes to the `coding-adventures-closure-pass-rename-properties` crate will be documented in this file.

## [0.15.0] - 2026-07-11

### Added — CLOC12.188 PR1: `ImportDeclaration` arms

Exhaustive-match no-op arms for the new `Declaration::ImportDeclaration` variant
in the class-member classify and rewrite walks — an import has no properties to
classify or rename.

## [0.14.0] - 2026-07-12

### Added — CLOC12.187 PR2a: decline to rename properties in the presence of `with`

`run` now bails at the top when `program_contains_with_statement`
(closure-scope-analyzer 0.14.0, added as a new dependency) is `true`, returning
the program unchanged. Inside `with (obj) …` a bare `foo` may be the property
access `obj.foo` in disguise; the pass cannot see that, so renaming the property
`foo` elsewhere would desynchronize from the hidden access. `with` is rare (a
strict-mode syntax error), so the program-wide bail costs little. New
`with_statement_disables_property_renaming` test.

## [0.13.0] - 2026-07-11

### Added — CLOC12.187 PR1: traverse `WithStatement`

New `TaggedStatement::WithStatement` arms in `classify_stmt` and `rewrite_stmt`
descend into the `with` object and body. Picks up javascript-ast 0.38.0.

## [0.12.16] - 2026-07-11

### Added — CLOC12.176 PR1: `ClassMember::StaticBlock` arm

`javascript-ast` 0.35.0 added `ClassMember::StaticBlock(BlockStatement)`, the third class member (a `static { … }` initialization block). Added `StaticBlock` arms (classify + rewrite): a static block has no key to rename, but its statements may hold property accesses, so each statement is classified / rewritten.

## [0.12.15] - 2026-07-11

### Added — CLOC12.175 PR1: rename class-field property keys

`javascript-ast` 0.34.0 added `ClassMember::Field`. Added `Field` arms
(classify + rewrite) that treat the field's key as a renameable property name
(like a method key, with no constructor guard) and recurse into the computed key
and initializer. Reachable once the CLOC12.175 PR2 bridge produces the node.

## [0.12.14] - 2026-07-10

### Added — CLOC12.174 PR1: `Declaration::ClassDeclaration` match arms

`javascript-ast` 0.33.0 added the `Declaration::ClassDeclaration` variant. Added
arms to `classify_decl` and `rewrite_decl` that treat a class declaration's
members exactly like a class expression's — each non-computed method key is a
renameable property name (with `constructor` pinned as never-renameable). The
shared member logic was factored into `classify_class_members` /
`rewrite_class_members` helpers used by both the expression and declaration arms,
so classification and rewrite stay in lockstep. The class's own name is a
variable, not a property, so it is untouched. Reachable once the CLOC12.174 PR2
bridge produces the node.

## [0.12.13] - 2026-07-08

### Added — CLOC12.173 PR1: `ClassExpression` match arm (mirrors `FunctionExpression`)

`javascript-ast` 0.32.0 added the `Expression::ClassExpression` variant, which made
this crate’s exhaustive `Expression` match(es) non-exhaustive. Added a
`ClassExpression` arm at each site, mirroring the crate’s existing
`FunctionExpression` handling: recurse into the `extends` operand (a normal
expression) and each method’s `value` (a `FunctionExpression`, walked as its own
function scope). Variable-renaming passes leave method KEYS untouched (a method
key is a property name, not a variable); the property-renaming pass treats method
A `constructor` method key is NEVER renamed (`see_quoted`-pinned) — renaming it
would turn the constructor into a plain method and silently give the class an
implicit constructor (a construction-semantics miscompile). keys as renameable property names, mirroring object-literal keys. Rebuild/
transform arms delegate to an `#[inline(never)]` helper (frame-size DoS lesson).
Reachable once the CLOC12.173 PR2 bridge produces `ClassExpression` nodes.

## [0.12.12] - 2026-07-07

### Changed — CLOC12.169: `ImportExpression` exhaustive-match arm

Added an `Expression::ImportExpression` case so the pass stays exhaustive over the new `javascript-ast` single-operand variant (part of the CLOC12.169 atomic node PR1). A dynamic `import(source)` carries one sub-expression (the module specifier), so the pass recurses into `source` exactly like the sibling `AwaitExpression` arm. No behaviour change to any existing node.


## [0.12.11] - 2026-07-07

### Changed — CLOC12.168: `ImportMeta` exhaustive-match arm

Added an `Expression::ImportMeta` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.168 atomic node PR1). `import.meta` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals. No behaviour change to any existing node.


## [0.12.10] - 2026-07-04

### Changed — CLOC12.167: `NewTarget` exhaustive-match arm

Added an `Expression::NewTarget` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.167 atomic node PR1). `new.target` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.12.9] - 2026-07-04

### Changed — CLOC12.166: `Super` exhaustive-match arm

Added an `Expression::Super` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.166 atomic node PR1). `super` is a leaf keyword with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.12.8] - 2026-07-04

### Changed — CLOC12.165: `ThisExpression` exhaustive-match arm

Added an `Expression::ThisExpression` no-op arm so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.165 atomic node PR1). `this` binds and references no identifier and has no sub-expression, so the traversal does nothing for it. No behaviour change to any existing node.


## [0.12.7] - 2026-07-04

### Changed — CLOC12.164: `AwaitExpression` traversal arms

Added `Expression::AwaitExpression` arms that walk into the await's `argument`,
so the pass stays exhaustive over the new `javascript-ast` variant (part of the
CLOC12.164 atomic node PR1). No behaviour change to any existing node; the await
argument is now visited exactly like any other sub-expression the pass already
handles.


## [0.12.6] - 2026-07-03

### Changed — CLOC12.163: `YieldExpression` traversal arms

Added `Expression::YieldExpression` arms (2 across the pass's traversal
routines) that walk into the yield's optional `argument` when present, so the
pass stays exhaustive over the new `javascript-ast` variant (part of the
CLOC12.163 atomic node PR1). No behaviour change to any existing node; the
yield argument is now visited exactly like any other sub-expression the pass
already handles.


## [0.12.5] - 2026-07-03

### Changed — CLOC12.162: `SpreadElement` exhaustive-match arm

Added an `Expression::SpreadElement` arm that recurses into the spread's
`argument` so the pass stays exhaustive over the new `javascript-ast` variant
(part of the CLOC12.162 atomic node PR1). No behaviour change to any existing
node; the spread argument is now visited/rewritten exactly like any other
sub-expression the pass already handles.

## [0.12.4] - 2026-07-02

### Changed — CLOC12.161: handle `Expression::TaggedTemplateExpression`

Added a `TaggedTemplateExpression` match arm recursing into the `tag` callee
and each `${…}` insert of the applied template, so this pass keeps
compiling and traverses the new `javascript-ast` 0.20.0 node. No behaviour
change for any existing node.

## [0.12.3] - 2026-07-02

### Changed — CLOC12.160: handle `Expression::SequenceExpression`

Added a `SequenceExpression` match arm recursing into each operand so this
crate compiles and traverses the new `Expression::SequenceExpression`
variant. No behaviour change until the bridge produces sequence nodes
(CLOC12.160 PR2).


## [0.12.2] - 2026-07-02

### Changed — CLOC12.159: handle `Expression::NewExpression`

Added a `NewExpression` match arm mirroring `CallExpression` (recurse into the
callee and each argument) so this crate compiles and traverses the new
`Expression::NewExpression` variant. No behaviour change until the bridge
produces `new` nodes (CLOC12.159 PR2).


## [0.12.1] - 2026-07-02

### Changed — CLOC12.158: exhaustiveness for new `Expression::UpdateExpression`

Handle the new `Expression::UpdateExpression` (`++x` / `x++` / `--x` / `x--`)
variant added to `javascript-ast` (0.17.0): the pass recurses into the operand for classification and rewriting. No behaviour
change for existing inputs — the bridge does not yet produce update
expressions (that lands in the CLOC12.158 PR2 bridge-enable), so these arms
are exercised only via hand-constructed AST today.

## [0.12.0] - 2026-07-02

### Added — CLOC12.154: `TemplateLiteral` traversal

Handle the new `Expression::TemplateLiteral` variant by recursing into its `${{…}}` sub-expressions (the `expressions` vector); the `quasis` are fixed leaf string segments with nothing to recurse. Part of the atomic `TemplateLiteral` enum-variant rollout (javascript-ast 0.16.0) — adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR. Template literals introduce no bindings or scopes, so the renaming/inlining arms need no map reduction.

## [0.11.0] - 2026-07-02

### Added — CLOC12.151: `ArrowFunctionExpression` traversal

Handle the new `Expression::ArrowFunctionExpression` variant by recursing into arrow bodies — both the block form (`x => { ... }`) and the concise/expression form (`x => expr`) — mirroring this pass's existing `FunctionExpression` handling. Part of the atomic `ArrowFunctionExpression` enum-variant rollout (javascript-ast 0.15.0); adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR. (Params are variable names, never property names, so no map reduction is needed.)

## [0.10.0] - 2026-07-01

### Added — CLOC12.149: rename properties inside `FunctionExpression`

`classify_expr` and `rewrite_expr` recurse into a `FunctionExpression`
body so a quoted `o["foo"]` written there still disables renaming of
`foo`, and dotted accesses inside are rewritten. Variable bindings
(name/params) never touch the property namespace.

## [0.9.1] - 2026-07-01

### Added — CLOC12 upstream test port (`RenamePropertiesTest`)

Ported the applicable cases from Google Closure Compiler's
`RenamePropertiesTest.java` into `tests/upstream/rename_properties_test.rs`,
following the CLOC12.01 convention (header cites the Java source; `UPSTREAM_SHA`
pins the tracked commit; `ATTRIBUTION.md` records Apache-2.0 provenance; a
`[[test]]` entry wires the file in). Like the `rename-globals` port, the pass
exposes a source-string surface through public crate APIs, so each case drives
the real `source → bridge → rename → emit` chain and asserts on the emitted
string.

- **8 active `#[test]`s pass**: a private property renamed consistently across
  dotted reads, reads-and-object-literal-keys collapsing to one short name,
  distinct names assigned down a member chain (`a.a.b`), quoted-access poisoning
  the rename (`o["mode"]` leaves `.mode` alone), built-in / DOM names left
  untouched, a single-character property un-shortened, a computed-subscript
  index untouched, and an externs property preserved.
- **No new closurec bug** — every active expectation matched the pass on the
  first run.
- **3 `#[ignore = "blocked on gap-NNN"]` placeholders** for upstream behavior the
  name-based pass does not cover: type-/heap-aware disambiguation of same-named
  properties (gap-138), frequency-ordered short-name assignment (gap-139), and a
  cross-module shared rename map (gap-140). Pinned to
  `code/specs/CLOC12-gaps.md` §CLOC12.140; run with `--include-ignored` to track
  progress as they close.

No library code changed — this release is test coverage plus docs.

## [0.9.0] - 2026-06-30

### Added — correlation-vector rename provenance (#89)

Mirrors the rename-globals pass. Renaming is a transformation, not a deletion,
so this pass now records each property rename as a `renamed` **contribution**
carrying `{from, to}`. Before, `run()` returned `contributions: Vec::new()` and
never touched the CV log, so property renaming silently erased the link between
a minified property (`o.a`) and its original name (`o.longProp`): a
`--correlation_vector` consumer had no way to recover it.

`rename_properties` now returns its applied rename table (`(from, to)` pairs,
sorted by original name for deterministic output) alongside the `changed` flag,
and `run` maps each pair to a `Contribution{source:"rename-properties",
tag:"renamed", meta:{from, to}}`. The pipeline attaches these to the
program-root CV entry, so the rename table becomes queryable provenance.

- **Byte-for-byte identical program output** — the renames applied are exactly
  as before; only the returned `contributions` list is now populated. All 24
  existing output tests are unchanged.
- Two new tests: a renamed property emits one `renamed` contribution with the
  right `from`/shorter `to`; a program whose only property is a built-in
  (`o.length`) emits none.
- `correlation-vector` moved from dev- to regular dependency; `serde_json` added
  (for `Contribution.meta`). Crate version 0.8.0 → 0.9.0.

**Scope / follow-up.** This attaches the rename *table* at the program root.
Per-output-span provenance — contributing to each renamed property occurrence's
own CV id — needs the log threaded through the `rewrite_*` recursion and is a
documented follow-up.

## [0.8.0] - 2026-06-20

### Added — CLOC23: property renaming recurses through `for`-`of`

`classify_stmt` and `rewrite_stmt` recurse through `ForOfStatement` (left / right
/ body) so property accesses inside a for-of loop and in the iterable expression
are renamed consistently — identical to the `for`-`in` handling.

## [0.7.0] - 2026-06-20

### Added — CLOC22: property renaming recurses through `for`-`in`

`classify_stmt` and `rewrite_stmt` recurse through `ForInStatement` (left / right
/ body) so property accesses inside a for-in loop — including `obj[key]` member
reads in the body and the enumerated right-hand expression — are renamed
consistently.

## [0.6.0] - 2026-06-20

### Added — CLOC21: handle `DebuggerStatement`

`classify_stmt` and `rewrite_stmt` now cover `DebuggerStatement` (grouped with
the other childless leaf statements) as a no-op. A `debugger;` has no property
accesses — added to keep the matches exhaustive over the new AST variant.

## [0.5.0] - 2026-06-20

### Added — CLOC20: property renaming recurses through `do`/`while`

`classify_stmt` and `rewrite_stmt` recurse through `DoWhileStatement` (loop body
and test) so property accesses inside a do-while loop are renamed consistently.

## [0.4.0] - 2026-06-20

### Added — CLOC19: property renaming recurses through `try`/`catch`/`finally`

`classify_stmt` and `rewrite_stmt` recurse through `TryStatement` so property
accesses inside the protected block, catch handler, and finalizer are renamed
consistently. No catch-param handling is required here: property renaming
operates on member/key names, not variable bindings, so the catch `param` (a
variable binding) is irrelevant to this pass.

## [0.3.0] - 2026-06-18

### Added (CLOC13.L — bundled DOM/host property boundary, `DOM_PROPERTIES`)

A curated `DOM_PROPERTIES` list (~300 names) is now **always protected**
alongside the ECMAScript `BUILTIN_PROPERTIES`, closing the documented gap
that "the built-in list covers ECMAScript but NOT the DOM/host." Common
browser-surface property names — `innerHTML`, `textContent`, `classList`,
`addEventListener`, `onclick`/`onload`/… inline handlers, `querySelector`,
`getAttribute`, `style`, `dataset`, Window/Document/Location/History/
Storage/Navigator members, XHR/fetch/Response fields, drag-and-drop, and
event-object properties — are kept out of the box, so the pass no longer
renames a DOM property the author never listed in `--externs` (which would
silently break browser code).

- **Always-on, additive, sound.** The protected baseline is now
  `BUILTIN_PROPERTIES ∪ DOM_PROPERTIES`; `--externs` still unions on top.
  Over-protecting a program-private property that happens to share a DOM
  name merely forgoes a rename — never a miscompile (the same posture the
  ECMAScript list already had). The bundle is a safety net, not a
  replacement: vendor-/library-specific external properties still need a
  `--externs` file, which remains the authoritative boundary.
- Grouped by host area (EventTarget/events, inline `on*` handlers,
  Node/Element, classList, form/input, attributes, CSSOM, Document, Window,
  Location/History/Storage/Navigator, XHR/fetch/Response, drag-and-drop) for
  auditability.
- 2 new tests: a DOM property (`innerHTML`/`addEventListener`/`onclick`) is
  kept with no `--externs` while a program-private property is still
  renamed; a lone unlisted DOM property is kept (the safety net).

## [0.2.0] - 2026-06-18

### Added (CLOC13.K — `collect_property_names`, the externs property boundary)

A public function `collect_property_names(program) -> HashSet<String>` that
returns **every property name appearing anywhere** in a program — dotted member
accesses (`el.innerHTML`), quoted member accesses (`obj["data-id"]`), unquoted
object keys (`{ onload: f }`), and quoted object keys (`{ "aria-label": s }`).

This is the property-namespace analogue of collecting an externs file's
top-level variable/function names (the value-namespace boundary). A driver
(closurec) walks each `--externs` file through this function and unions the
results into the `do_not_rename` set it hands `RenamePropertiesPass::new`, so the
external host/library property surface is preserved while program-private
properties are still shortened.

- **Over-collects on purpose.** Both renameable (dotted) and off-limits (quoted)
  occurrences are returned: as an externs boundary, every named property is
  external and must be protected. Forgoing a rename is never a miscompile;
  renaming a genuinely external property is. Dynamic computed keys
  (`obj[runtimeExpr]`) contribute nothing — there is no static name to protect.
- Reuses the pass's existing whole-program `classify_item` walk (no second
  traversal implementation to keep in sync).
- 9 new unit tests + 1 doctest covering each occurrence shape, dynamic-key
  exclusion, function-body recursion, and an end-to-end "collected externs
  protect a property" round-trip.

## [0.1.0] - 2026-06-18

### Added (CLOC13.J — aggressive property renaming, algorithmic core)

New crate per CLOC06's canonical pass set — Closure Compiler's `RENAME_PROPERTIES`
in miniature. `RenamePropertiesPass::run` consistently shortens program-private
object **property names** across the whole program (every dotted `obj.x` member
access and every unquoted `{ x: … }` key of a renameable name → a fresh short
name). Property access is by name, so renaming a name at every occurrence is
semantics-preserving regardless of which objects carry it.

- **ADVANCED-only, sound under the externs contract.** A property name is
  renamed only when it: appears dotted/unquoted; is NOT quoted via a computed
  string member (`obj["x"]` — the bridge preserves this signal); is not a
  `BUILTIN_PROPERTIES` (a bundled ECMAScript default-externs substitute —
  `length`, `prototype`, `toString`, `push`, …); is not in the externs
  do-not-rename set; and is longer than one character. Each renameable property
  gets a distinct fresh name. Property names live in their own namespace, so the
  fresh name only avoids other property names + the built-ins + the externs set.
- **Honest limitations (documented in the crate):**
  - The built-in list covers ECMAScript but NOT the DOM/host — host property
    names (`innerHTML`, `addEventListener`, …) must be supplied via `--externs`.
  - The parser bridge currently collapses a *quoted object key* `{ "x": 1 }` to
    an identifier key, so object-key quoting is not a usable do-not-rename
    signal (only computed-member quoting `obj["x"]` is); protect such names via
    externs. (A separate bridge fix is tracked.)
  - Dynamic computed access `obj[runtimeString]` is the author's contract
    responsibility, exactly as in Closure.
- `name = "rename-properties"`, `depends_on = []`, `iteration_policy = OneShot`,
  `cost = 3`. `new(do_not_rename)` / `with_builtins_only()`.

This is the algorithmic core; wiring into ADVANCED (collecting externs property
names + deciding the safe-by-default policy — require externs / bundle DOM
externs) is a deliberate follow-up.

### Tests
- 13 tests: metadata contract + source → bridge → rename-properties → emit
  roundtrips covering consistent dotted+object-key renaming, computed-member
  quoting decline, built-in protection, externs protection, dynamic computed
  key, single-char skip, and a nested property chain.
