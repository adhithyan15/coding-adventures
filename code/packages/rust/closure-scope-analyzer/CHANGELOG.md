# Changelog

All notable changes to the `coding-adventures-closure-scope-analyzer` crate will be documented in this file.

## [0.12.17] - 2026-07-11

### Added — CLOC12.177 PR1: `PropertyKey::PrivateName` arm

The computed-key walk gains a `PropertyKey::PrivateName` arm (a no-op): a private
name is not an identifier reference — it names a slot in the class's private
brand, resolved lexically at parse time, never through scope — so it binds and
references nothing. Keeps the match exhaustive after `javascript-ast` 0.36.0
added the variant. PATCH.

## [0.12.16] - 2026-07-11

### Added — CLOC12.176 PR1: `ClassMember::StaticBlock` arm

`javascript-ast` 0.35.0 added `ClassMember::StaticBlock(BlockStatement)`, the third class member (a `static { … }` initialization block). Added `StaticBlock` arms (declaration + expression) walking the block as a free-standing `walk_block_statement` — a static block is its own block scope, so local `let`/`const`/`var` land in a block scope and references resolve.

## [0.12.15] - 2026-07-11

### Added — CLOC12.175 PR1: resolve references in field initializers

`javascript-ast` 0.34.0 added `ClassMember::Field`. Added `Field` arms (class
declaration + class expression) that walk the field's initializer and computed
key so references inside a field initializer (evaluated at construction in the
class scope) are resolved. Reachable once the CLOC12.175 PR2 bridge produces the
node.

## [0.12.14] - 2026-07-10

### Added — CLOC12.174 PR1: `Declaration::ClassDeclaration` walk arm

`javascript-ast` 0.33.0 added the `Declaration::ClassDeclaration` variant. Added
`walk_class_declaration` to the exhaustive `walk_declaration` match: it emits a
`BindingKind::Class` binding for the class name in the current scope (the lexical
analogue of the `Function`-kind binding a function declaration hoists — the
`Class` kind was already reserved for exactly this), then resolves references in
the `extends` heritage (enclosing scope) and each method value (its own function
scope). Reachable once the CLOC12.174 PR2 bridge produces the node.

## [0.12.13] - 2026-07-08

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

### Changed — CLOC12.164: `AwaitExpression` exhaustive-match arm

Added an `Expression::AwaitExpression` arm that walks into the await's
`argument`, so the analyzer stays exhaustive over the new `javascript-ast`
variant (part of the CLOC12.164 atomic node PR1). No behaviour change to any
existing node; the await argument is now visited exactly like any other
sub-expression the analyzer already handles.


## [0.12.6] - 2026-07-03

### Changed — CLOC12.163: `YieldExpression` exhaustive-match arm

Added an `Expression::YieldExpression` arm that walks into the yield's optional
`argument` when present, so the analyzer stays exhaustive over the new
`javascript-ast` variant (part of the CLOC12.163 atomic node PR1). No behaviour
change to any existing node; the yield argument is now visited exactly like any
other sub-expression the analyzer already handles.


## [0.12.5] - 2026-07-03

### Changed — CLOC12.162: `SpreadElement` exhaustive-match arm

Added an `Expression::SpreadElement` arm that recurses into the spread's
`argument` so the analyzer stays exhaustive over the new `javascript-ast`
variant (part of the CLOC12.162 atomic node PR1). No behaviour change to any
existing node; the spread argument is now visited exactly like any other
sub-expression the analyzer already handles.

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
variant added to `javascript-ast` (0.17.0): the pass walks the operand so its identifier reference is resolved in scope. No behaviour
change for existing inputs — the bridge does not yet produce update
expressions (that lands in the CLOC12.158 PR2 bridge-enable), so these arms
are exercised only via hand-constructed AST today.

## [0.12.0] - 2026-07-02

### Added — CLOC12.154: `TemplateLiteral` traversal

Handle the new `Expression::TemplateLiteral` variant by recursing into its `${{…}}` sub-expressions (the `expressions` vector); the `quasis` are fixed leaf string segments with nothing to recurse. Part of the atomic `TemplateLiteral` enum-variant rollout (javascript-ast 0.16.0) — adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR. Template literals introduce no bindings or scopes, so the renaming/inlining arms need no map reduction.

## [0.11.0] - 2026-07-02

### Added — CLOC12.151: `ArrowFunctionExpression` traversal

Add `walk_arrow_function_expression`: an arrow introduces its own function scope binding its params (no body-local name — arrows are anonymous), then walks the block or concise-expression body. Part of the atomic `ArrowFunctionExpression` rollout (javascript-ast 0.15.0).

## [0.10.0] - 2026-07-01

### Added — CLOC12.149: scope-analyze `FunctionExpression`

Adds `walk_function_expression`, mirroring `walk_function_declaration`
except a named function expression's name is **body-local** (bound inside
the function's own scope for self-recursion, not the enclosing scope).
Params become Param bindings in the function scope; the body is walked in
that scope. Keeps renaming/inlining consumers sound over function values.

## [0.9.0] - 2026-06-20

### Added — CLOC23: scope analysis for `for`-`of`

`walk_tagged_statement` now handles `ForOfStatement`: it walks the `left` (a
loop-variable binding or assignment target), the iterable `right`, then the
body — identical to the `for`-`in` handling.

## [0.8.0] - 2026-06-20

### Added — CLOC22: scope analysis for `for`-`in`

`walk_tagged_statement` now handles `ForInStatement`: it walks the `left` (a
variable declaration binding the loop variable, or an assignment-target
expression), then the enumerated `right`, then the body.

## [0.7.0] - 2026-06-20

### Added — CLOC21: handle `DebuggerStatement`

`walk_tagged_statement` now has a `DebuggerStatement` arm. A `debugger;` has no
children and binds nothing, so the arm is a no-op — added only to keep the
statement match exhaustive over the new AST variant.

## [0.6.0] - 2026-06-20

### Added — CLOC20: scope analysis for `do`/`while`

`walk_tagged_statement` now handles `DoWhileStatement`: it walks the body and the
test in the current scope. Like `while`, a do-while introduces no new scope, so
this is a straight structural recurse.

## [0.5.0] - 2026-06-20

### Added — CLOC19: scope analysis for `try`/`catch`/`finally`

`walk_tagged_statement` now handles `TryStatement`: it walks the protected
block, opens a dedicated `ScopeKind::Block` scope for the catch handler and emits
the catch `param` as a `BindingKind::Let` binding scoped to the handler body
(catch params bind only within `catch { … }`, never the surrounding scope), then
walks the handler body and the finalizer. This keeps the analyzer's binding model
accurate for programs containing try/catch.

## [0.4.1] - 2026-06-16

### Docs

- Corrected a stale `analyze` doc comment that still claimed "Today we emit zero
  references" — reference collection + resolution has been live since
  CLOC13.0.1 (identifier uses, assignment targets, and computed member keys all
  produce resolved `Reference`s). No code change.

## [0.4.0] - 2026-06-02

### Added — CLOC13.0.2 nested scopes (Function-body + Block)

Replaces the v0.3.0 flat-global walker with a **recursive scope-tree** builder. Until this PR every binding and reference lived in `ScopeId::GLOBAL`. Now Function bodies create child Function scopes, BlockStatements create child Block scopes, `var` declarations correctly hoist to the enclosing function, and reference resolution walks the parent chain.

Architecture:

- **`WalkCtx`** — copy-state threaded through the walker. Holds `current` (where new `let`/`const` go + where refs emit `from_scope`) and `enclosing_function` (where `var` declarations hoist).
- **`PendingReference`** — references are collected during the walk but resolved AFTER the walk finishes via `analysis.resolve(name, from_scope)`. Two-phase deferral because `resolve()` reads from `analysis.scopes` / `analysis.bindings` which the walk mutates.

Scope rules:

| Construct                              | Binding scope                  |
|----------------------------------------|--------------------------------|
| Top-level `var`/`let`/`const`          | GLOBAL                         |
| Top-level `function f`                 | GLOBAL (kind=Function)         |
| `function f` params                    | f's Function scope (kind=Param)|
| Inside `function f`: `var` (any depth) | f's Function scope (hoisted)   |
| Inside any scope: `let`/`const`        | immediate enclosing scope      |
| Nested `function g`                    | name in enclosing; body new Function scope |
| Free-standing `{ … }` block            | new ScopeKind::Block           |

A `FunctionDeclaration.body` is a `BlockStatement`, but per ECMAScript spec it's the function's own scope — NOT a fresh Block child. The walker recognizes this and walks the body's statements directly under the new Function scope.

### Resolution walks parent chain

Reference resolution uses the existing `ScopeAnalysis::resolve()`. With nested scopes now populated, a reference inside `f` to `x` looks first in `f`'s scope, then `f`'s parent (GLOBAL), then returns `None` for free globals.

### Deferred to CLOC13.0.3+

- Catch-clause scope (AST variant not in Phase 1 yet).
- Strict-mode semantics (function-in-block scoping differs).
- TDZ enforcement (analyzer reports declarations + references in source order).
- `with (…)` statement.

### Tests (9 new; 33 total, was 24)

- `function_declaration_creates_child_function_scope`
- `function_params_become_param_bindings_in_function_scope`
- `param_reference_resolves_inside_function`
- `cross_scope_resolution_finds_outer_binding` (parent-chain walk through Function → GLOBAL)
- `block_statement_creates_block_scope`
- `var_in_block_hoists_to_enclosing_function` (the key hoisting test)
- `let_in_block_stays_in_block_scope` (opposite of var hoisting)
- `nested_function_creates_nested_function_scope`
- `empty_program_still_returns_global_scope_only` (regression)

The existing `function_body_walks_for_references` test was updated: previously asserted `from_scope == GLOBAL` (correct under v0.3.0); now asserts `from_scope == ScopeId(1)` (the function's own scope).

### No consumer-pass regressions

All 5 consumer pass suites still green after this PR: rename 9, inline 9, treeshake 15, collapse-properties 8, remove-unused-vars 15 = 56 tests. Consumer apply steps still filter by `binding.scope == GLOBAL`, so non-GLOBAL bindings are correctly NOT removed/treeshaken/etc. by name accident.

### Bumped 0.3.0 → 0.4.0

`analyze` signature unchanged. Serde representation unchanged (`ScopeKind` and `BindingKind` were already `#[non_exhaustive]`).

## [0.3.0] - 2026-06-02

### Added — CLOC13.0.1 references collection

Adds **Phase 2** to `analyze`: a walker that emits a `Reference` for every `Identifier` use site in `program.body`. This closes the biggest gap from CLOC13.0 — until now `references` was always empty, which meant every binding read as "unused" to consumer passes that gate on `uses == 0` / `uses == 1` (`remove-unused-vars`, `inline`).

What's covered (v0.3.0):

- **Top-level `ExpressionStatement`** — recurses into the expression tree.
- **Top-level `IfStatement` / `WhileStatement` / `ForStatement` / `ReturnStatement` / `LabeledStatement` / `ThrowStatement`** — walks all subexpressions.
- **`VariableDeclaration` initializers** — `let y = x;` emits a reference for `x`.
- **`FunctionDeclaration.body`** — walks the function body recursively. (Today the body still lives in GLOBAL; nested function/block scopes land in CLOC13.0.2.)
- **Inside expressions:**
  - `Identifier` → emit Reference.
  - `BinaryExpression` / `LogicalExpression` → walk left + right.
  - `UnaryExpression` → walk argument.
  - `AssignmentExpression` → LHS Identifier emits a Reference (assignment is both read + write of the binding); MemberExpression target walks the object (and computed property). RHS walks.
  - `ConditionalExpression` → walk test, consequent, alternate.
  - `CallExpression` → walk callee + each argument.
  - `MemberExpression` → walk object always; walk property ONLY if `computed`.
  - `ArrayExpression` → walk each non-elided element.
  - `ObjectExpression` → for each `Property`, walk the value; walk the key only if `computed`.
  - Literal variants (Numeric/String/Boolean/Null/Undefined/BigInt) → no children, skipped.

Resolution: a single `HashMap<String, BindingId>` is built from the bindings table once, then every emitted Reference's `binding` field is filled via `names.get(&id.name).copied()`. `None` means the identifier is a free global (`console`, `window`, etc.) — consumers like `treeshake` and `remove-unused-vars` already treat unresolved references as "definitely used externally" per the existing contract.

### What's NOT walked (intentional)

- The `id` of a `VariableDeclarator` (binding declaration, not a reference).
- The `id` of a `FunctionDeclaration` (same).
- `FunctionParam` (declarations).
- Non-computed `MemberExpression.property` (static property name).
- Non-computed `Property.key` in `ObjectExpression` (static key name).
- The `label` of a `LabeledStatement` (label declaration).
- `BreakStatement.label` / `ContinueStatement.label` (label-reference, not a binding lookup — handled by labels, not the scope analyzer).

### Deferred to CLOC13.0.2

- **Nested scopes** — `FunctionDeclaration.body` should create a `ScopeKind::Function` child scope; `BlockStatement` (when it contains `let` / `const`) should create a `ScopeKind::Block` child scope. Today every reference's `from_scope` is `ScopeId::GLOBAL`. (Resolution still works because all bindings are in GLOBAL too.)
- **Var hoisting** — moot today since there are no nested scopes to hoist out of.
- **Function param bindings** — params should land as `BindingKind::Param` in the function scope.
- **Catch-clause scope** (not in Phase 1 AST yet).
- **Strict-mode binding semantics**.

### Activates `remove-unused-vars` + `inline`

`remove-unused-vars` (CLOC13.E body + CLOC13.E.1 apply step) gates removals on `uses == 0`. Before this PR, every binding had zero uses (because references was empty), so the apply step would have dropped ALL top-level `var`/`let`/`const` bindings — including ones that ARE referenced. The CLOC13.0.1 reference walker prevents that: bindings with actual reads will now be correctly retained.

Same activation for `inline` (CLOC13.B): it gates on `uses == 1`. Before this PR, single-use detection was impossible (every binding read as 0 uses). After, single-use `Function` / `Class` bindings become real inline candidates.

`collapse-properties` (CLOC13.D) and `treeshake` (CLOC13.C) also benefit but their apply steps haven't shipped yet (queued as CLOC13.D.1 / CLOC13.C.1).

### Test surface (11 new; 24 total, was 13)

Walker tests:
- `bare_identifier_in_expression_statement_emits_reference`
- `reference_resolves_to_top_level_binding`
- `unresolved_reference_to_free_global`
- `binary_expression_collects_both_sides`
- `call_expression_collects_callee_and_arguments`
- `member_expression_collects_object_only_when_not_computed`
- `member_expression_computed_form_collects_property`
- `variable_declaration_init_walks_for_references`
- `function_body_walks_for_references`
- `if_statement_walks_test_and_branches`
- `literals_in_expression_statements_emit_no_references`

The two CLOC13.0 "breadcrumb" tests (`statement_items_are_skipped_for_now`, `references_are_empty_in_cloc13_0`) still pass because their fixtures have no Identifier expressions — they now pin the *no-references-when-no-identifier-expressions* contract, which is the more durable form of the original promise. Comment in `references_are_empty_in_cloc13_0` was updated to reflect this.

### Bumped 0.2.0 → 0.3.0

`analyze` signature unchanged.

## [0.2.0] - 2026-06-02

### Added — CLOC13.0 minimal analyzer body

Replaces the v0.1.0 identity-style empty `analyze` with a real walk of `program.body` that surfaces **top-level declarations**:

- `VariableDeclaration` (`var` / `let` / `const`) — one `Binding` per `VariableDeclarator`, with `VarKind → BindingKind` mapping (`Var → Var`, `Let → Let`, `Const → Const`). Multi-declarator forms (`const a = 1, b = 2;`) emit one binding each.
- `FunctionDeclaration` — one `Binding` with `kind = Function` carrying the function name.
- All bindings land in `ScopeId::GLOBAL`. The global scope's `bindings` list mirrors the global table.
- `declared_at` is populated from the AST's `Identifier.cv` when CV tracing is on (otherwise stays `None`).
- `BindingTarget::Identifier` is the only Phase 1 variant; the match is total today. When Phase 3 adds destructuring patterns, they'll need their own arms.

### Activates 5 consumer passes simultaneously

This PR is the wire-then-activate completion. The five pass bodies (CLOC13.A..E, PRs #4766, #4773, #4775, #4777, #4778) all consume `bindings` via the analyzer's public surface. They went from "candidate scan finds zero" to "candidate scan finds real top-level decls" with **zero PR-side churn** — no rebases, no API changes.

`changed = false` is still hard-pinned in every consumer pass. Lighting up real bindings only makes the candidate scans non-empty; the apply step (CLOC13.{A,B,C,D,E}.1) is a per-pass follow-up.

### Deferred to CLOC13.0.1 (tracked inline in `fn analyze`)

1. **Function body scopes.** A `FunctionDeclaration` should create a `ScopeKind::Function` child scope holding params + nested decls. Today we only emit the function's name binding in `GLOBAL`.
2. **Block scopes.** `let`/`const` inside a `BlockStatement` should land in a `ScopeKind::Block` child scope. Today nested blocks are ignored.
3. **Var hoisting.** A `var x` inside a block must bind in the enclosing *function* scope. Pre-walk pattern documented inline.
4. **`References`.** Identifier use sites should produce `Reference`s. Today the references vec is empty. This is the biggest remaining gap — `remove-unused-vars` and `inline` both gate on `uses == 0` / `uses == 1`, so zero references reads as "every binding is unused".
5. **Catch-clause scope** (not in Phase 1 AST yet).
6. **Strict-mode binding semantics** (function-in-block scope).

### Tests added (7 new; 13 total, was 6)

- `top_level_let_surfaces_as_binding_in_global` — end-to-end pin of the binding shape.
- `top_level_var_let_const_map_to_three_kinds` — `VarKind → BindingKind` mapping.
- `top_level_function_declaration_surfaces` — function-name binding shape.
- `multi_declarator_emits_one_binding_per_declarator` — `const a = 1, b = 2;` form.
- `binding_ids_are_dense_and_monotonic` — `BindingId(0), (1), (2)` contract.
- `statement_items_are_skipped_for_now` — pin the deferred-Statement-walk contract; will fail when CLOC13.0.1 starts collecting references.
- `references_are_empty_in_cloc13_0` — pin the deferred-References contract.

All 6 v0.1.0 tests still pass unchanged.

### Bumped 0.1.0 → 0.2.0

Signature of `analyze` is unchanged; the v0.1.0 contract holds. Consumers don't need to recompile against a new API.

## [0.1.0] - 2026-06-01

### Added — CLOC13 unblocker: scaffold + stable API surface

First commit. Ships the **types and the public entry function** that
the five Phase-1 optimisation passes consume:

- `ScopeId`, `BindingId` — opaque newtype handles into the dense
  vectors on `ScopeAnalysis`.
- `Scope { kind, parent, bindings }`, `ScopeKind::{Global, Function,
  Block}`.
- `Binding { name, kind, scope, declared_at }`, `BindingKind::{Var,
  Let, Const, Function, Class, Param}`.
- `Reference { name, from_scope, binding, cv }` — one per identifier
  use site, with the resolved binding (`None` = free global).
- `ScopeAnalysis { scopes, bindings, references }` — the analysis
  output.  Has a `resolve(name, from_scope)` convenience that walks
  the parent chain.
- `analyze(program) -> ScopeAnalysis` — entry function.

**Identity body.** The v0.1.0 `analyze` returns a single global scope
with no bindings or references — i.e., it doesn't yet walk the AST.
The full traversal lands as a follow-up (tracked under CLOC13.0).
This split is deliberate: the **API surface** is the unblocker for the
five consumer passes, so freezing the contract here lets CLOC13.A
(rename), CLOC13.B (inline), CLOC13.C (treeshake), CLOC13.D
(collapse-properties), and CLOC13.E (remove-unused-vars) all proceed
as parallel work streams.

Tests cover: identity-body shape, `ScopeId::GLOBAL == 0`, `resolve`
on empty analysis, `resolve` walking the parent chain, innermost-shadow
wins, and full serde round-trip.

### Rationale

Why a separate crate (rather than putting the analysis in
`closure-pass-pipeline` or `javascript-ast`):

1. **AST stays backend-agnostic.** `javascript-ast` is shared with the
   future V8-on-LANG-VM clone; scope analysis is Closure-specific.
2. **Pipeline stays scheduling-only.** `closure-pass-pipeline` runs
   passes; it doesn't bake in any one pass's data structures.
3. **One build per pipeline run.** Five passes consuming a shared
   analysis beats five passes each rebuilding their own.
4. **Serialisable.** Newtype-wrapped IDs (not pointers) so the
   analysis can dump to a sidecar JSON for the CV pipeline.
