# Changelog

All notable changes to the `coding-adventures-closure-pass-inline-variables` crate will be documented in this file.

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
