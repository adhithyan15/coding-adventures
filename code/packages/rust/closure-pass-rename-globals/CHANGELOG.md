# Changelog

All notable changes to the `coding-adventures-closure-pass-rename-globals` crate will be documented in this file.

## [0.10.15] - 2026-07-11

### Added — CLOC12.175 PR1: `ClassMember::Field` arms

`javascript-ast` 0.34.0 added `ClassMember::Field`. Added `Field` handling
(collect + rename) that leaves the field key alone (a property name, not a
variable) but collects and renames global identifiers referenced in the
initializer and computed key. Reachable once the CLOC12.175 PR2 bridge produces
the node.

## [0.10.14] - 2026-07-10

### Added — CLOC12.174 PR1: `Declaration::ClassDeclaration` match arms

`javascript-ast` 0.33.0 added the `Declaration::ClassDeclaration` variant. Added
arms at each exhaustive `Declaration` match site: `count_decl_names_decl` (class
name + method params + bodies — counting params upholds the rename invariant that
a global shadowed by a method param is disqualified), `collect_all_idents_decl`
(class name + heritage + member keys/params/bodies), and `rename_apply_decl`
(rename the class's own name as a global binding, the `extends` operand as a use,
and recurse each method body with the full map). Reachable once the CLOC12.174
PR2 bridge produces the node.

## [0.10.13] - 2026-07-08

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

## [0.10.12] - 2026-07-07

### Changed — CLOC12.169: `ImportExpression` exhaustive-match arm

Added an `Expression::ImportExpression` case so the pass stays exhaustive over the new `javascript-ast` single-operand variant (part of the CLOC12.169 atomic node PR1). A dynamic `import(source)` carries one sub-expression (the module specifier), so the pass recurses into `source` exactly like the sibling `AwaitExpression` arm. No behaviour change to any existing node.


## [0.10.11] - 2026-07-07

### Changed — CLOC12.168: `ImportMeta` exhaustive-match arm

Added an `Expression::ImportMeta` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.168 atomic node PR1). `import.meta` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals. No behaviour change to any existing node.


## [0.10.10] - 2026-07-04

### Changed — CLOC12.167: `NewTarget` exhaustive-match arm

Added an `Expression::NewTarget` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.167 atomic node PR1). `new.target` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.10.9] - 2026-07-04

### Changed — CLOC12.166: `Super` exhaustive-match arm

Added an `Expression::Super` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.166 atomic node PR1). `super` is a leaf keyword with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.10.8] - 2026-07-04

### Changed — CLOC12.165: `ThisExpression` exhaustive-match arm

Added an `Expression::ThisExpression` no-op arm so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.165 atomic node PR1). `this` binds and references no identifier and has no sub-expression, so the traversal does nothing for it. No behaviour change to any existing node.


## [0.10.7] - 2026-07-04

### Changed — CLOC12.164: `AwaitExpression` traversal arms

Added `Expression::AwaitExpression` arms that walk into the await's `argument`,
so the pass stays exhaustive over the new `javascript-ast` variant (part of the
CLOC12.164 atomic node PR1). No behaviour change to any existing node; the await
argument is now visited exactly like any other sub-expression the pass already
handles.


## [0.10.6] - 2026-07-03

### Changed — CLOC12.163: `YieldExpression` traversal arms

Added `Expression::YieldExpression` arms (2 across the pass's traversal
routines) that walk into the yield's optional `argument` when present, so the
pass stays exhaustive over the new `javascript-ast` variant (part of the
CLOC12.163 atomic node PR1). No behaviour change to any existing node; the
yield argument is now visited exactly like any other sub-expression the pass
already handles.


## [0.10.5] - 2026-07-03

### Changed — CLOC12.162: `SpreadElement` exhaustive-match arm

Added an `Expression::SpreadElement` arm that recurses into the spread's
`argument` so the pass stays exhaustive over the new `javascript-ast` variant
(part of the CLOC12.162 atomic node PR1). No behaviour change to any existing
node; the spread argument is now visited/rewritten exactly like any other
sub-expression the pass already handles.

## [0.10.4] - 2026-07-02

### Changed — CLOC12.161: handle `Expression::TaggedTemplateExpression`

Added a `TaggedTemplateExpression` match arm recursing into the `tag` callee
and each `${…}` insert of the applied template, so this pass keeps
compiling and traverses the new `javascript-ast` 0.20.0 node. No behaviour
change for any existing node.

## [0.10.3] - 2026-07-02

### Changed — CLOC12.160: handle `Expression::SequenceExpression`

Added a `SequenceExpression` match arm recursing into each operand so this
crate compiles and traverses the new `Expression::SequenceExpression`
variant. No behaviour change until the bridge produces sequence nodes
(CLOC12.160 PR2).


## [0.10.2] - 2026-07-02

### Changed — CLOC12.159: handle `Expression::NewExpression`

Added a `NewExpression` match arm mirroring `CallExpression` (recurse into the
callee and each argument) so this crate compiles and traverses the new
`Expression::NewExpression` variant. No behaviour change until the bridge
produces `new` nodes (CLOC12.159 PR2).


## [0.10.1] - 2026-07-02

### Changed — CLOC12.158: exhaustiveness for new `Expression::UpdateExpression`

Handle the new `Expression::UpdateExpression` (`++x` / `x++` / `--x` / `x--`)
variant added to `javascript-ast` (0.17.0): the pass recurses into the operand for ident collection and rename application. No behaviour
change for existing inputs — the bridge does not yet produce update
expressions (that lands in the CLOC12.158 PR2 bridge-enable), so these arms
are exercised only via hand-constructed AST today.

## [0.10.0] - 2026-07-02

### Added — CLOC12.154: `TemplateLiteral` traversal

Handle the new `Expression::TemplateLiteral` variant by recursing into its `${{…}}` sub-expressions (the `expressions` vector); the `quasis` are fixed leaf string segments with nothing to recurse. Part of the atomic `TemplateLiteral` enum-variant rollout (javascript-ast 0.16.0) — adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR. Template literals introduce no bindings or scopes, so the renaming/inlining arms need no map reduction.

## [0.9.0] - 2026-07-02

### Added — CLOC12.151: `ArrowFunctionExpression` traversal

Handle the new `Expression::ArrowFunctionExpression` variant by recursing into arrow bodies — both the block form (`x => { ... }`) and the concise/expression form (`x => expr`) — mirroring this pass's existing `FunctionExpression` handling. Part of the atomic `ArrowFunctionExpression` enum-variant rollout (javascript-ast 0.15.0); adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR. The arrow's params are removed from the active rename/substitute map before recursing into the body, so a param that shadows an outer binding of the same spelling is left untouched (arrows have no self-name to shadow).

## [0.8.0] - 2026-07-01

### Added — CLOC12.149: rename globals through `FunctionExpression`

`collect_all_idents_expr` records a function value's name + params +
body idents (avoid-set). `rename_apply_expr` recurses into the body with
the function's own name and params **removed from the active map**, so a
shadowed local is left untouched while genuine globals inside the body
are still renamed — a self-contained soundness guarantee.

## [0.7.1] - 2026-06-30

### Added — CLOC12 upstream test port (`RenameVarsTest`)

Ported the applicable cases from Google Closure Compiler's `RenameVarsTest.java`
into `tests/upstream/rename_vars_test.rs`, following the CLOC12.01 convention
(header cites the Java source; `UPSTREAM_SHA` pins the tracked commit;
`ATTRIBUTION.md` records Apache-2.0 provenance; a `[[test]]` entry wires the file
in). The pass exposes a source-string surface through public crate APIs, so —
unlike the AST-builder ports — each case drives the real
`source → bridge → rename → emit` chain and asserts on the emitted string, the
same surface upstream uses.

- **8 active `#[test]`s pass**: two globals → `a`/`b`, all-uses-rewritten, a
  global function-declaration rename, the reserved-extern case (keeps only the
  reserved `apiHandler`, renames the ordinary global `helper`→`a`), a
  free/undeclared global left untouched, a dotted property key left untouched, a
  global used as a computed-member index renamed, and a single-character global
  left un-lengthened.
- **No new closurec bug** — the port validated correct behavior; one draft
  expectation was corrected during authoring (the reserved-extern case).
- **4 `#[ignore = "blocked on gap-NNN"]` placeholders** for upstream behavior
  the global-only pass does not cover: function-local renaming (gap-134),
  parameter renaming (gap-135), short-name reuse across disjoint scopes
  (gap-136), and pseudo-name / stable-name mode (gap-137). Pinned to
  `code/specs/CLOC12-gaps.md` §CLOC12.139; run with `--include-ignored` to track
  progress as they close.

No library code changed — this release is test coverage plus docs.

## [0.7.0] - 2026-06-30

### Added — correlation-vector rename provenance (#89)

Renaming is a transformation, not a deletion, so — unlike DCE / fold-control-flow
/ treeshake, which *tombstone* removed nodes — this pass now records each global
rename as a `renamed` **contribution** carrying `{from, to}`. Before, the pass
returned `contributions: Vec::new()` and never touched the CV log, so renaming
silently erased the link between a minified global (`a`) and its original name
(`longName`): a `--correlation_vector` consumer had no way to recover it.

`rename_globals` now returns its applied rename table (`(from, to)` pairs, sorted
by original name for deterministic output) alongside the `changed` flag, and
`run` maps each pair to a `Contribution{source:"rename-globals", tag:"renamed",
meta:{from, to}}`. The pipeline attaches these to the program-root CV entry
(`cv.contribute(prog_cv, …)`), so the rename table becomes queryable provenance.

- **Byte-for-byte identical program output** — the renames applied are exactly
  as before; only the returned `contributions` list is now populated. All 16
  existing output tests are unchanged.
- Two new tests: a renamed global emits one `renamed` contribution with the
  right `from`/shorter `to`; a program with nothing to rename emits none.
- `correlation-vector` moved from dev- to regular dependency; `serde_json` added
  (for `Contribution.meta`). Crate version 0.6.0 → 0.7.0.

**Scope / follow-up.** This attaches the rename *table* at the program root.
Per-output-span provenance — contributing to each renamed identifier's *own* CV
id — needs the log threaded through the `rename_apply_*` recursion and is a
documented follow-up.

## [0.6.0] - 2026-06-20

### Added — CLOC23: global renaming across `for`-`of`

`count_decl_names_stmt`, `collect_all_idents_stmt`, and `rename_apply_tagged`
recurse through `ForOfStatement` (left / right / body), mirroring the `for`-`in`
handling so the loop variable and uses inside the body rename consistently.

## [0.5.0] - 2026-06-20

### Added — CLOC22: global renaming across `for`-`in`

`count_decl_names_stmt`, `collect_all_idents_stmt`, and `rename_apply_tagged`
recurse through `ForInStatement` (left / right / body), mirroring the
for-statement handling so the loop variable and uses inside the body rename
consistently.

## [0.4.0] - 2026-06-20

### Added — CLOC21: handle `DebuggerStatement`

Every phase of the pass now covers `DebuggerStatement` (grouped with the other
childless leaf statements) as a no-op. Added to keep the matches exhaustive over
the new AST variant.

## [0.3.0] - 2026-06-20

### Added — CLOC20: global renaming across `do`/`while`

`count_decl_names_stmt`, `collect_all_idents_stmt`, and `rename_apply_tagged`
recurse through `DoWhileStatement` (loop body and test), mirroring the existing
`while` handling.

## [0.2.0] - 2026-06-20

### Added — CLOC19: global renaming across `try`/`catch`/`finally` (catch-param soundness)

`count_decl_names_stmt`, `collect_all_idents_stmt`, and `rename_apply_tagged`
recurse through `TryStatement`. As in the local renamer, the catch `param` is
counted as a declared binding and added to the avoid set, so a global rename can
never produce a name that collides with a catch binding and the param itself is
never rewritten.

## [0.1.0] - 2026-06-18

### Added (CLOC13.I — aggressive top-level / global renaming)

New crate per CLOC06's canonical pass set — the **ADVANCED**-level complement to
`closure-pass-rename` (which only shortens the *locals* of leaf functions). It
renames program-private top-level names (`function` / `var` / `let` / `const`)
to short identifiers (`a`, `b`, …) at their declaration and every use site:

```js
function computeTotal(items) { return items; }
var result = computeTotal(list);
// => function a(items){return items} var b=a(list)
```

- **ADVANCED-only by construction.** In a script, a top-level name is part of
  the program's public surface, so renaming it is sound only under Closure's
  whole-program / `--externs` contract: everything externally visible is
  declared in the externs; anything else is private and may be shortened.
  `RenameGlobalsPass::new(do_not_rename: HashSet<String>)` takes that externs
  boundary; `with_no_externs()` is the pure whole-program form.
- **Soundness** (self-contained name-based analysis, same guard as `inline` /
  `inline-variables`): a top-level binding is renamed only when its name is
  **declared exactly once in the whole program** (so every use resolves to it —
  a sound α-conversion), is **not in the do-not-rename set**, and is longer than
  one character. Free globals (`console`, `window`, …) have no declaration here,
  so they are never candidates. The fresh name **avoids every identifier
  anywhere in the program** (declarations, uses, property names, free globals)
  and every externs name, so it can neither collide with another binding (incl.
  a function-local of the same letter) nor capture a free global. Property names
  (non-computed `.x` / object keys) are never rewritten; computed `o[x]` is.
- `name = "rename-globals"`, `depends_on = []`, `iteration_policy = OneShot`,
  `cost = 3`.

### Tests
- 16 tests: metadata contract + source → bridge → rename-globals → emit
  roundtrips covering top-level function/var renaming, use sites inside function
  bodies, the externs do-not-rename set, free-global and property-name
  preservation, computed-member use, single-char skip, shadowed-name skip, and
  fresh-name collision avoidance against a function-local.
