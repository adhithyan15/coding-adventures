# Changelog

All notable changes to the `coding-adventures-closure-pass-fold-control-flow` crate will be documented in this file.

## [0.20.17] - 2026-07-11

### Added — CLOC12.177 PR1: `PropertyKey::PrivateName` arm

The object-literal key-rebuild match gains a `PropertyKey::PrivateName` arm that
passes the key through unchanged (a private name never occurs in an object
literal, but the match must stay exhaustive after `javascript-ast` 0.36.0 added
the variant). PATCH.

## [0.20.16] - 2026-07-11

### Added — CLOC12.176 PR1: `ClassMember::StaticBlock` arm

`javascript-ast` 0.35.0 added `ClassMember::StaticBlock(BlockStatement)`, the third class member (a `static { … }` initialization block). Added a `StaticBlock` arm that folds + `var`-hoists the block (a static block is its own hoisting scope), exactly like a method body.

## [0.20.15] - 2026-07-11

### Added — CLOC12.175 PR1: fold class-field initializers

`javascript-ast` 0.34.0 added `ClassMember::Field`. Added a `Field` arm to the
class-body member map folding the initializer (and computed key) with
`fold_expression`. Reachable once the CLOC12.175 PR2 bridge produces the node.

## [0.20.14] - 2026-07-10

### Added — CLOC12.174 PR1: `Declaration::ClassDeclaration` match arms

`javascript-ast` 0.33.0 added the `Declaration::ClassDeclaration` variant. Added
the transform arm to `fold_declaration` (folds control flow inside the heritage
operand and method bodies via a new shared `#[inline(never)] fold_class_body`
helper, also refactored out of `fold_class`), and a `false` arm to
`block_is_scope_safe_to_hoist` (a block-scoped class declaration must not be
hoisted out of an `else`, like a nested function declaration). Reachable once the
CLOC12.174 PR2 bridge produces the node.

## [0.20.13] - 2026-07-08

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

## [0.20.12] - 2026-07-07

### Changed — CLOC12.169: `ImportExpression` exhaustive-match arm

Added an `Expression::ImportExpression` case so the pass stays exhaustive over the new `javascript-ast` single-operand variant (part of the CLOC12.169 atomic node PR1). A dynamic `import(source)` carries one sub-expression (the module specifier), so the pass recurses into `source` exactly like the sibling `AwaitExpression` arm (and returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.20.11] - 2026-07-07

### Changed — CLOC12.168: `ImportMeta` exhaustive-match arm

Added an `Expression::ImportMeta` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.168 atomic node PR1). `import.meta` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.20.10] - 2026-07-04

### Changed — CLOC12.167: `NewTarget` exhaustive-match arm

Added an `Expression::NewTarget` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.167 atomic node PR1). `new.target` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.20.9] - 2026-07-04

### Changed — CLOC12.166: `Super` exhaustive-match arm

Added an `Expression::Super` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.166 atomic node PR1). `super` is a leaf keyword with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.20.8] - 2026-07-04

### Changed — CLOC12.165: `ThisExpression` exhaustive-match + `expression_cv` arm

Added `Expression::ThisExpression` arms to both the rebuild match (clones the leaf through, like the literals) and the `expression_cv` accessor (returns its `cv`), keeping the pass exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.165 atomic node PR1). No behaviour change to any existing node.


## [0.20.7] - 2026-07-04

### Changed — CLOC12.164: `AwaitExpression` exhaustive-match arm

Added an `Expression::AwaitExpression` arm that rebuilds the node, recursing
into its `argument`, plus a matching arm in the `expression_cv` accessor, so the
pass stays exhaustive over the new `javascript-ast` variant (part of the
CLOC12.164 atomic node PR1). No behaviour change to any existing node; the await
argument is now visited/rewritten exactly like any other sub-expression the pass
already handles.


## [0.20.6] - 2026-07-03

### Changed — CLOC12.163: `YieldExpression` exhaustive-match arm

Added an `Expression::YieldExpression` arm that rebuilds the node, preserving
`delegate` and recursing into the optional `argument` when present, plus a
matching arm in the `expression_cv` accessor, so the pass stays exhaustive over
the new `javascript-ast` variant (part of the CLOC12.163 atomic node PR1). No
behaviour change to any existing node; the yield argument is now
visited/rewritten exactly like any other sub-expression the pass already
handles.


## [0.20.5] - 2026-07-03

### Changed — CLOC12.162: `SpreadElement` exhaustive-match arm

Added an `Expression::SpreadElement` arm that recurses into the spread's
`argument` so the pass stays exhaustive over the new `javascript-ast` variant
(part of the CLOC12.162 atomic node PR1). No behaviour change to any existing
node; the spread argument is now visited/rewritten exactly like any other
sub-expression the pass already handles.

## [0.20.4] - 2026-07-02

### Changed — CLOC12.161: handle `Expression::TaggedTemplateExpression`

Added a `TaggedTemplateExpression` match arm recursing into the `tag` callee
and each `${…}` insert of the applied template, so this pass keeps
compiling and traverses the new `javascript-ast` 0.20.0 node. No behaviour
change for any existing node.

## [0.20.3] - 2026-07-02

### Changed — CLOC12.160: handle `Expression::SequenceExpression`

Added a `SequenceExpression` match arm recursing into each operand so this
crate compiles and traverses the new `Expression::SequenceExpression`
variant. No behaviour change until the bridge produces sequence nodes
(CLOC12.160 PR2).


## [0.20.2] - 2026-07-02

### Changed — CLOC12.159: handle `Expression::NewExpression`

Added a `NewExpression` match arm mirroring `CallExpression` (recurse into the
callee and each argument) so this crate compiles and traverses the new
`Expression::NewExpression` variant. No behaviour change until the bridge
produces `new` nodes (CLOC12.159 PR2).


## [0.20.1] - 2026-07-02

### Changed — CLOC12.158: exhaustiveness for new `Expression::UpdateExpression`

Handle the new `Expression::UpdateExpression` (`++x` / `x++` / `--x` / `x--`)
variant added to `javascript-ast` (0.17.0): the pass recurses into the operand and preserves the update node; `cv()` accessor extended. No behaviour
change for existing inputs — the bridge does not yet produce update
expressions (that lands in the CLOC12.158 PR2 bridge-enable), so these arms
are exercised only via hand-constructed AST today.

## [0.20.0] - 2026-07-02

### Added — CLOC12.154: `TemplateLiteral` traversal

Handle the new `Expression::TemplateLiteral` variant by recursing into its `${{…}}` sub-expressions (the `expressions` vector); the `quasis` are fixed leaf string segments with nothing to recurse. Part of the atomic `TemplateLiteral` enum-variant rollout (javascript-ast 0.16.0) — adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR. Template literals introduce no bindings or scopes, so the renaming/inlining arms need no map reduction.

## [0.19.0] - 2026-07-02

### Added — CLOC12.151: `ArrowFunctionExpression` traversal

Handle the new `Expression::ArrowFunctionExpression` variant by recursing into arrow bodies — both the block form (`x => { ... }`) and the concise/expression form (`x => expr`) — mirroring this pass's existing `FunctionExpression` handling. Part of the atomic `ArrowFunctionExpression` enum-variant rollout (javascript-ast 0.15.0); adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR. Also covers the exhaustive `expression_cv` provenance map.

## [0.18.0] - 2026-07-01

### Added — CLOC12.149: fold control flow inside `FunctionExpression`

`fold_expression` recurses into a `FunctionExpression` body (fold +
`var` hoist), mirroring the `FunctionDeclaration` arm; `expression_cv`
gains a `FunctionExpression` arm so provenance is preserved.

## [0.17.0] - 2026-06-30

### Added — CV tombstones for constant-condition ternary collapse (#89 follow-up)

Closes the second and final follow-up noted in 0.15.0 (the first, block dead-code,
landed in 0.16.0). When `fold_conditional` collapses a `cond ? c : a` whose
`cond` is a literal, the unreachable arm — an **`Expression`**, not a statement —
is now tombstoned via the existing `record_fold_deleting`, matching what the
`if`/`while` branch eliminations already do:

- `(<truthy literal>) ? c : a` → `c` — the **`a`** (alternate) arm is tombstoned;
- `(<falsy literal>)  ? c : a` → `a` — the **`c`** (consequent) arm is tombstoned.

New helper `expression_cv` (the `Expression` analogue of `statement_cv`) — an
exhaustive match over all 16 expression variants, so a new expression kind fails
to compile rather than silently losing provenance.

The `de-morgan-swap-not-ternary` rewrite (`!x ? c : a` → `x ? a : c`) deliberately
does NOT tombstone — both arms are preserved, just swapped — and keeps plain
`record_fold`.

Byte-for-byte identical AST output; only the CV log gains the deletion records.
`delete` is a no-op when the log is disabled (production default). Two new tests
(true-arm and false-arm tombstoning). 38 unit + 13 upstream all green. Crate
0.16.0 → 0.17.0. With this, **every** elimination site in the pass records
deletion provenance.

## [0.16.0] - 2026-06-30

### Added — CV tombstones for block dead-code-after-terminator (#89 follow-up)

Closes the first of the two follow-ups noted in 0.15.0. The block-level
dead-code-after-terminator drop (`fold_block_statement`, tag `removed-dead-code`)
now tombstones each eliminated statement's own CV entry via the existing
`record_fold_deleting` + `statement_cv` machinery, instead of emitting only a
summary contribution. So `{ return; dead(); more(); }` → `{ return; }` records a
`DeletionRecord{source:"fold-control-flow", reason:"removed-dead-code"}` on each
of `dead()` / `more()`, exactly as the DCE pass records for the same
dead-after-terminator drop it performs. The dropped statements' CV ids are
captured in the block-fold loop before they are skipped.

Byte-for-byte identical AST output; only the CV log gains the deletion records.
`delete` is a no-op when the log is disabled (production default). One new test
(`dead_code_after_terminator_is_tombstoned`); 36 unit + 13 upstream all green.
Crate 0.15.0 → 0.16.0.

The remaining follow-up — the constant-condition **ternary** collapse
(`cond ? c : a` with a literal `cond`), whose discarded arm is an `Expression`
and needs an `expression_cv` helper — is still open.

## [0.15.0] - 2026-06-30

### Added — correlation-vector deletion provenance for eliminated branches (#89)

Following the deletion-provenance work merged for the DCE pass, this pass now
records *why a branch disappeared* in the correlation-vector log. Before, the
pass received the shared `CVLog` but discarded it (`let _ = self.cv`), pushing
only a coarse summary `Contribution` against the enclosing `if`/`while`. So a
`--correlation_vector` consumer that asked "what happened to the code that used
to be in this branch?" got no answer — the eliminated branch simply vanished
from the provenance graph.

Each **constant-condition branch/loop elimination** now tombstones the
discarded branch's own CV entry via `CVLog::delete(cv_id, "fold-control-flow",
"folded-branch", meta)`:

- `if (true)  A else B` → `A` — **B** (the `else` branch) is tombstoned;
- `if (false) A else B` → `B` — **A** (the `then` branch) is tombstoned;
- `while (false) BODY`  → `;` — **BODY** is tombstoned.

`meta` carries the enclosing node's `container_cv`. New helper
`FoldState::record_fold_deleting` (tombstone + keep the summary contribution)
and `statement_cv`/`tagged_statement_cv` (exhaustive match — a new statement
kind fails to compile rather than silently losing provenance).

Rewrites that **preserve** both branches deliberately do NOT tombstone —
`if→ternary`, `if→&&`, and the De Morgan swaps keep their content (just
restructured), so they keep plain `record_fold`. A regression test pins that a
ternary rewrite tombstones neither arm.

Byte-for-byte identical AST output: the same branches are folded and the same
summary contributions emitted; only the CV log gains deletion records.
`delete` is a no-op when the log is disabled (production default), so zero cost
off the `--correlation_vector` path. Four new tests. Crate 0.14.0 → 0.15.0.

### Scope / follow-up

Two further elimination sites still emit only a summary contribution and are
left for a follow-up: the block-level dead-code-after-terminator drop
(`removed-dead-code` — the same behavior the DCE pass already tombstones), and
the constant-condition **ternary** collapse (`cond ? c : a` with a literal
`cond`), whose discarded arm is an `Expression` and needs an `expression_cv`
helper.

## [0.14.0] - 2026-06-20

### Added — CLOC25: drop a redundant `else` after a terminating consequent

`fold_block_statement` now removes the `else` from an `if` whose consequent
unconditionally terminates, hoisting the `else` body into the enclosing block —
upstream Closure's `MinimizeExitPoints`:

```text
if (c) { …; return x; } else { B }   →   if (c) { …; return x; } B
if (bad) throw e; else use(v);        →   if (bad) throw e; use(v);
```

When `c` is true the consequent exits (`return`/`throw`), so control reaches the
`else` body only when `c` was false — exactly the `else` semantics. Removing the
`else` deletes a keyword and (for a block) a pair of braces, and un-nests
`else if` chains.

- New helpers: `consequent_definitely_terminates` (accepts `return`/`throw`, and
  a block whose last statement does — it is broader than the return-only
  `is_terminator` that still gates the dead-code-after-terminator drop);
  `block_is_scope_safe_to_hoist` / `alternate_is_hoistable`.
- **Soundness — scope safety.** The `else` body is hoisted only when splicing it
  into the enclosing block changes no binding's scope: a block containing a
  block-scoped `let`/`const`/`function` declaration is **not** hoisted (it would
  leak the binding or cause a TDZ collision); plain `var` is function-scoped and
  hoists harmlessly. A bare (non-block) `else` body that is a `Declaration` is
  likewise declined. Mirrors `closure-pass-dce`'s `block_is_scope_safe_to_flatten`.
- When the hoisted tail itself ends in a terminator, the existing
  dead-code-after-terminator drop then removes any statements that followed the
  original `if`. A `removed-`-style `hoisted-else-after-terminator` contribution
  is recorded (and flips `changed`).
- 5 new tests: block-`else` hoist after `return`; bare-`else` hoist after
  `throw`; a `let` in the `else` block blocks the hoist; a non-terminating
  consequent is left alone; and the hoisted-tail dead-code drop.

## [0.13.0] - 2026-06-20

### Added — CLOC23: fold control flow inside `for`-`of`

New `ForOfStatement` arm recurses control-flow folding into the loop's left,
right expression, and body. Like the other loops, a for-of is not eliminated and
is not a terminator (the iterable may be empty, so the body may run zero times).

## [0.12.0] - 2026-06-20

### Added — CLOC22: fold control flow inside `for`-`in`

New `ForInStatement` arm recurses control-flow folding into the loop's left,
right expression, and body. Like the other loops, a for-in is not eliminated and
is not a terminator (the body may run zero times).

## [0.11.0] - 2026-06-20

### Added — CLOC21: handle `DebuggerStatement`

The statement dispatch now covers `DebuggerStatement` (grouped with the other
childless leaf statements), returned unchanged. Added to keep the match
exhaustive over the new AST variant.

## [0.10.0] - 2026-06-20

### Added — CLOC20: fold control flow inside `do`/`while` (no dead-loop elision)

New `DoWhileStatement` arm recurses control-flow folding into the loop body and
test. Crucially — unlike `while`, which can be eliminated when its test is
statically falsy — a `do`-`while` runs its body **at least once**, so it is NEVER
removed even when `test` folds to a falsy literal (the single body run is
observable). The arm therefore only recurses structurally.

## [0.9.0] - 2026-06-20

### Added — CLOC19: fold control flow inside `try`/`catch`/`finally`

New `TryStatement` arm recurses control-flow folding through the protected
block, the catch handler body, and the finalizer (each an ordinary block),
preserving the catch `param`. The `try` itself is never treated as a terminator —
it can catch and continue — so code after it stays reachable.

## [0.8.0] - 2026-06-04

### Added — CLOC12.37: var hoisting (gap-015)

Closes **gap-015**. After folding each `FunctionDeclaration`
body, lift `var x = expr;` declarations from inside nested
blocks up to the function-body top:

```js
function f() { if (cond) { var y = 1; } }
↓
function f() { var y; if (cond) y = 1; }
```

This matches upstream Closure's syntactically-visible hoist
and lets downstream rename / dce see all function-scoped
bindings at the body's top.

### Scope

- **Recurses into**: nested `BlockStatement`, `IfStatement`
  consequent/alternate, `WhileStatement` body,
  `ForStatement` body, `LabeledStatement` body,
  `SwitchStatement` case consequents.
- **Does NOT recurse into**: nested `FunctionDeclaration` /
  `FunctionExpression` bodies — they have their own
  function-scope and own hoisting (handled by their own
  dispatch through `fold_declaration`).
- **Does NOT touch**: `for(var x = 0; ...; ...)` init slots
  (already at hoistable position); `let` / `const`
  (block-scoped — hoisting doesn't apply).
- **Conservative bail**: pure passthrough when the function
  body contains no hoistable vars — no allocations,
  no `changed` signal.

### Rewrite shape

Each `var x = expr;` site collapses to:

- `[bare]` (no init): `EmptyStatement` at the original site;
  the name is hoisted to the prepended declaration.
- `[single init]`: `ExpressionStatement(x = expr)` at the
  original site.
- `[multiple inits]`: `BlockStatement` containing one
  assignment-statement per declarator-with-init. The
  block-flatten step of DCE may splice it back into the
  parent.

Names accumulate into a single `var x, y, z;` prepended to the
function body. One `var-hoisted` `Contribution` per function
body (tagged against the body's CV) — not per identifier.

### Helpers (private)

- `hoist_function_body_vars(body, st) -> BlockStatement` —
  the top-level entry called from
  `Declaration::FunctionDeclaration` arm.
- `hoist_visit_stmt(stmt, collected) -> Statement` —
  recursive walker over compound statements; does NOT enter
  inner function bodies.
- `hoist_visit_block(b, collected) -> BlockStatement` —
  block-body recursion helper.
- `hoist_rewrite_var_decl(v, collected) -> Statement` —
  collapses one `var ...;` site to its replacement statement.

### Tests (6 new, 20 → 26 inline)

| Test | Pins |
|------|------|
| `var_inside_if_consequent_block_hoists` | `if (cond) { var y = 1; }` → `var y; if (cond) y = 1;` |
| `var_at_top_of_function_body_is_split` | `var x = 1;` → `var x; x = 1;` (uniform split) |
| `let_declaration_is_not_hoisted` | `let` stays block-scoped (no `var-hoisted` contribution) |
| `nested_function_body_vars_are_isolated` | outer hoister doesn't touch inner function's vars |
| `empty_function_body_does_nothing` | `function f() {}` → no work, no contribution |
| `bare_var_no_init_collapses_to_empty_at_site` | `var y;` inside block → site becomes `EmptyStatement` |

closurec's e2e suites stay green — purely additive structural
rewrite that produces a more canonical form.

### Version bump

`0.7.0` → `0.8.0`.

## [0.7.0] - 2026-06-04

### Added — CLOC12.26: gap-019 return-then-return through if-else → ternary return

Closes `gap-019`. `fold_if_statement` now hoists terminal
`return` branches through an if-else into a single ternary-returning
return:

```
if (x) return E1; else return E2;       →   return x ? E1 : E2;
if (x) { return E1; } else { return E2; } →   return x ? E1 : E2;
```

Composes with the same-pass folds:

* gap-018 De Morgan: `if (!x) return E1; else return E2;` →
  (gap-018) `if (x) return E2; else return E1;` → (gap-019)
  `return x ? E2 : E1;`.
* literal_truthy: `if (true) return E1; else return E2;` →
  (literal_truthy) `return E1;` (still wins because literal_truthy
  fires before gap-019).

Both arguments must be `Some` — `if (x) return; else return E;`
stays unchanged. Synthesising an `undefined` expression for the
bare-return case requires `UndefinedLiteral` plumbing in this
pass and is tracked separately.

### Why this is safe

The if-else evaluates `test` once, takes exactly one branch, and
runs that branch's `return`. The ternary form also evaluates `test`
once, picks exactly one of `E1`/`E2`, then returns. The set of
values evaluated and the function's exit value match identically.

Control-flow preserved: in both forms the function returns
immediately after the chosen argument evaluates. No fall-through
possible because both branches were terminal returns.

### New helper

`single_return_with_arg(stmt: &Statement) -> Option<Expression>` —
mirror of `single_expr_stmt`. Returns the argument expression when
`stmt` is a single ReturnStatement with `argument: Some`; recurses
through single-statement BlockStatement layers; returns None on
multi-statement blocks, bare returns, or any other shape.

### Tests

* `tests/upstream/peephole_minimize_conditions_test.rs::test_fold_returns_into_ternary`
  un-ignored — exercises 3 shapes: bare return both sides, block-
  wrapped returns, and the conservative bail on `return;` (no arg).
* Upstream test count: 12 → 13.
* Inline tests: 20 → 20 (unchanged).
* closurec e2e (`diff_minify`): unaffected (no fixture uses returns).
* closure-pass-pipeline: 21 passed, 0 failed.

No public API change. No AST change.

## [0.6.0] - 2026-06-04

### Added — CLOC12.25: gap-018 De Morgan negation-swap

Closes `gap-018` from the CLOC12 gap tracker. Both `fold_if_statement`
and `fold_conditional` now strip a top-level `!` from the test and
swap consequent ↔ alternate when both branches exist:

```
if (!x) C; else A;     →   if (x) A; else C;
!x ? C : A             →   x ? A : C
```

Composes with previously-shipped folds in the same pass:

* gap-017 ternary: `if (!a) { foo() } else { bar() }` → (gap-018)
  `if (a) { bar() } else { foo() }` → (gap-017) `a ? bar() : foo();`.
* Literal-truthy: `if (!true) C; else A;` → (gap-018) `if (false) A; else C;`
  → (literal_truthy) `C;`. The swap runs BEFORE literal_truthy so
  the literal case is picked up in the same iteration.

### Why this is safe

`!x` and `x` make the same single `ToBoolean(x)` decision, just
flipped bit-wise. After the rewrite, the swapped branches produce
observationally identical control flow. The unary's argument is
*moved* (not cloned) into the new test position — no second runtime
evaluation of the operand is introduced. No side-effect reordering:
`x` evaluates before branch selection in both forms.

### Why the IfStatement case requires `alternate.is_some()`

Without an alternate the rewrite would have to synthesise an empty
branch (`if (x) ; else C;`), adding an EmptyStatement node — wrong
shape for minification. The gap-016 (CLOC12.24) `!x && C;` rewrite
already handles the no-alternate case better, and runs in the same
pass.

### Tests

* `tests/upstream/peephole_minimize_conditions_test.rs::test_fold_conditional_de_morgan`
  un-ignored — input `if (!a) { foo() } else { bar() }` folds through
  gap-018 + gap-017 to `a ? bar() : foo();`.
* Upstream test count: 11 → 12.
* Inline test count: 20 → 20 (unchanged).
* closurec e2e (`diff_minify`): unaffected (no fixture uses `!` in
  if/ternary tests).
* closure-pass-pipeline: 21 passed, 0 failed.

No public API change. No AST change.

## [0.5.0] - 2026-06-04

### Added — CLOC12.24: gap-016 `if (x) S` → `x && S` rewrite

Closes `gap-016` from the CLOC12 gap tracker. `fold_if_statement` now
has a third rewriting branch (after literal-truthy/falsy collapse and
gap-017 if-else→ternary): when the test is non-literal, the
consequent reduces to a single ExpressionStatement (directly or via
single-statement BlockStatement layers), and there is **no**
alternate, the IfStatement is rewritten to:

```
ExpressionStatement {
  LogicalExpression { left: test, op: And, right: consequent_expr }
}
```

Worked examples now folding:

```
if (x) a();              →  x && a();
if (x) { a(); }          →  x && a();   (single-expr block unwraps)
if (x) {{ a(); }}        →  x && a();   (nested single-stmt blocks)
if (x) y;                →  x && y;     (any expression statement)
```

Worked examples that stay as IfStatement (pre-conditions don't hold):

```
if (x) a(); else b();    →  x ? a() : b();   (gap-017 ternary fires
                                              first because alternate
                                              exists)
if (x) return 1;         →  unchanged (return is not an expression
                                       statement; gap-019's territory)
if (x) { a; b; }         →  unchanged (multi-stmt consequent doesn't
                                       reduce to one expression)
if (x) ;                 →  unchanged (empty consequent would require
                                       synthesising undefined; deferred)
```

### Why this is safe

`x && consequent` and `if (x) S` have observably identical evaluation
order:

* `x` is evaluated first (the short-circuit gate).
* If `x` is falsy → `&&` returns `x` without evaluating the right
  operand; `if (x) S` likewise skips `S`. Behaviour match.
* If `x` is truthy → `&&` evaluates the right operand; `if (x) S`
  likewise executes `S` for its side effects. The wrapper
  ExpressionStatement discards the result of `&&`, so the *value*
  is irrelevant — only the side effects matter. Behaviour match.

No second evaluation of `x` is introduced. `consequent`'s side
effects fire when and only when `x` is truthy.

### What changed in tests

* `tests/upstream/peephole_minimize_conditions_test.rs::test_fold_one_child_blocks_if_to_logical_and`
  — un-ignored, now exercises 4 shapes: bare `if (x) a();`,
  single-stmt block, nested blocks, and `if (x) y;`.
* `tests/upstream/peephole_minimize_conditions_test.rs::test_fold_one_child_blocks_if_else_to_ternary`
  — the trailing "testSame: no alternate" assertion was removed
  (it's now covered by the new gap-016 test) with a comment
  marking the historical pre-gap-016 behaviour.
* `src/lib.rs::if_non_literal_test_with_no_alternate_passes_through`
  — renamed to `..._with_multi_statement_consequent_passes_through`
  and the test body switched from `if (flag) x;` (now folds) to
  a 2-statement consequent block (still doesn't fold).
* `src/lib.rs::if_with_unresolved_comparison_doesnt_fold_alone`
  — renamed to `..._folds_via_gap016` and updated to assert the
  new `(1<2) && A` fold shape, while still pinning that the inner
  `1 < 2` BinaryExpression is NOT folded (fold-control-flow alone
  doesn't do binary-comparison folding; that's constant-fold).

Inline tests: 20 → 20 (two pre-existing tests renamed + updated).
Upstream tests: 10 → 11 (un-ignored gap-016 placeholder).

No public API change. No AST change. CV plumbing unchanged.

## [0.4.2] - 2026-06-01

### Changed — CLOC12.16: handle new `UndefinedLiteral` Expression variant

The fold-control-flow pass gained an `Expression::UndefinedLiteral`
arm in its expression-walk leaf list so it compiles against the new
`javascript-ast 0.6.0` AST. Behaviour: passthrough.

## [0.4.1] - 2026-06-01

### Changed — CLOC12.15 rebase: handle new `BigIntLiteral` Expression variant

The fold-control-flow pass gained an `Expression::BigIntLiteral`
arm in its expression-walk leaf list so it compiles against the
new `javascript-ast 0.5.0` AST. `literal_truthy` falls through
the wildcard (returns `None`) for bigints — we don't yet model
the `0n is falsy, anything else truthy` rule, so the if-collapse
optimisation stays conservative around bigint tests.

Bumped to 0.4.1 (rather than 0.3.3 originally planned) because this
PR was rebased on top of CLOC12.18 (0.4.0, already on main).

## [0.4.0] - 2026-06-01

### Added — CLOC12.18: if-else→ternary fold (closes gap-017)

Adds a rewrite rule in `fold_if_statement`: when the test is not a
known literal AND both branches reduce to a single ExpressionStatement
(directly or via single-statement BlockStatement layers), the
IfStatement rewrites to an ExpressionStatement wrapping a
ConditionalExpression.

Truth table:

| Input                                       | Output                  |
|---------------------------------------------|-------------------------|
| `if (x) foo(); else bar();`                 | `x ? foo() : bar();`    |
| `if (x) { foo(); } else { bar(); }`         | `x ? foo() : bar();`    |
| `if (x) foo();`                             | unchanged (no alternate)|
| `if (x) { a; b; } else c;`                  | unchanged (multi-stmt)  |
| `if (x) return 1; else return 2;`           | unchanged (return ≠ expr; tracked as gap-019) |

Side-effect safety: a ConditionalExpression evaluates `test` first
then exactly one of the two branches — identical to the if-else.

Un-ignores `test_fold_one_child_blocks_if_else_to_ternary` in the
upstream port. Updates two existing tests
(`test_if_non_literal_test_left_alone`,
`if_non_literal_test_passes_through`) to reflect the new fold.

The helper `single_expr_stmt(stmt) -> Option<Expression>` recursively
unwraps single-statement BlockStatement layers.

## [0.3.2] - 2026-06-01

### Changed — CLOC12.14: handle new `ThrowStatement` variant

The fold-control-flow pass gained a `TaggedStatement::ThrowStatement`
match arm so it compiles against the new `javascript-ast 0.4.0` AST.
Behaviour: fold the argument expression. `throw` is a definite
terminator like `return` — the dead-after-throw rule and the
`if (x) foo(); else throw e;` early-throw rewrite will land here
in follow-up gaps.

## [0.3.1] - 2026-06-01

### Changed — CLOC12.13: handle new `LabeledStatement` variant

The fold-control-flow pass gained a
`TaggedStatement::LabeledStatement` match arm so it compiles against
the new `javascript-ast 0.3.0` AST. Behaviour: recurse into the
labelled body, preserve the label verbatim. The collapse-to-empty
optimisation for `a: break a;` lives elsewhere and is tracked under
the gap-009 follow-up.

## [0.3.0] - 2026-05-31

### Added — CLOC12.05: port subset of upstream `PeepholeMinimizeConditionsTest`

Third port under the CLOC12 byte-identical contract, after
`closure-pass-constant-fold` (CLOC12.02) and `closure-pass-dce`
(CLOC12.04). Establishes the `tests/upstream/` layout for
`closure-pass-fold-control-flow`.

- `tests/upstream/UPSTREAM_SHA` — pins
  `google/closure-compiler@5bb35ec1245dc1d3557481e5f8b4db344bcd1e6b`.
- `tests/upstream/ATTRIBUTION.md` — Apache-2.0 attribution per
  CLOC12.01 §5.
- `tests/upstream/peephole_minimize_conditions_test.rs` — 14 ported
  test methods.

### Test breakdown

|     | passing | ignored |
|-----|---------|---------|
| CLOC12.05 | **9** | **5** |

**Passing (9):** literal-test if-folds + non-literal `testSame`:

- `test_if_true_folds_to_consequent` — `if (true) x else y` → `x`.
- `test_if_false_folds_to_alternate` — `if (false) x else y` → `y`.
- `test_if_false_no_alternate_becomes_empty_statement` — `if (false) x` → `;`.
- `test_if_numeric_one_folds_to_consequent` — `if (1) x else y` → `x`.
- `test_if_numeric_zero_folds_to_alternate` — `if (0) x else y` → `y`.
- `test_if_nonempty_string_folds_to_consequent` — `if ("hi") x else y` → `x`.
- `test_if_empty_string_folds_to_alternate` — `if ("") x else y` → `y`.
- `test_if_null_folds_to_alternate` — `if (null) x else y` → `y`. (Also
  consumes the routing-gap behaviour earmarked as gap-011 in CLOC12.04
  — `if (null){x=1;}else{x=2;}` → `x=2;`.)
- `test_if_non_literal_test_left_alone` — `testSame("if (x) C else A")`.

**Ignored (5):** record upstream's broader compaction scope as new
`gap-NNN` entries:

| Test | Gap | What's needed |
|------|-----|---------------|
| `test_fold_one_child_blocks_if_to_logical_and` | gap-016 | `if (x) S` → `x && S` rewrite |
| `test_fold_one_child_blocks_if_else_to_ternary` | gap-017 | `if (x) C else A` → `x ? C : A` rewrite |
| `test_fold_conditional_de_morgan` | gap-018 | De Morgan / negation-swap rewrites |
| `test_fold_returns_into_ternary` | gap-019 | return-then-return through if-else into single ternary-return |
| `test_minimize_if_with_throw` | gap-020 | `ThrowStatement` not in Phase 1 AST |

### Cross-crate routing wins

The new `test_if_null_folds_to_alternate` passing here demonstrates
exactly what CLOC12.04's routing gaps (gap-011 / gap-012 / gap-013)
predicted: upstream's `PeepholeRemoveDeadCodeTest::testIf` line for
`null` doesn't really test DCE — it tests fold-control-flow. When
that upstream line gets re-ported into this crate (a future slice),
gap-011 can move to `RESOLVED via CLOC12.05` because the *behaviour*
is already covered here.

### Version bump

`0.1.0` → `0.3.0` (CHANGELOG already had a 0.2.0 entry from the
earlier real-body roll-out).

## [0.2.0] - 2026-05-24

### Added — real `Pass::run` body

Replaces the identity v0.1.0 body with a recursive bottom-up walker over `Program → ProgramItem → Statement → Expression`. Folds:

- **`IfStatement` with literal test** → consequent (truthy) / alternate (falsy) / `EmptyStatement` (falsy, no alternate). Truthy/falsy uses JS truthiness rules: any non-empty string / non-zero non-NaN number / true is truthy; null / 0 / "" / false is falsy.
- **`WhileStatement` with literal `false` test** → `EmptyStatement`. `while (true)` is intentionally left alone (semantics matter — infinite loops are observable).
- **Dead code after `ReturnStatement`** in `BlockStatement.body` → dropped. Recurses into nested blocks via `FunctionDeclaration.body`.
- **`ConditionalExpression` with literal test** (`true ? a : b → a`). Redundantly handled here for robustness when this pass runs solo; constant-fold also handles it.

Recurses through every Phase 1 node so deep trees are folded in one bottom-up walk.

### CV tracing — both modes work per CLOC09 amendment

- **Traced input** (`cv: Some(parent)`): the kept replacement keeps its own pre-existing `cv` (it's the same node, just promoted). A `Contribution { source: "fold-control-flow", tag: "folded-branch"|"removed-dead-code", meta: {before, after, parent_cv} }` is appended.
- **Untraced input** (`cv: None`): folds silently with no contributions. `changed: true` still set.

### Tests

19 tests (up from 8 in v0.1.0):
- pass metadata (unchanged)
- empty-program identity
- `if (true) {x} else {y} → x`
- `if (false) {x} else {y} → y`
- `if (false) {x}` no alternate → `EmptyStatement`
- truthiness across booleans, numbers, strings, null — every JS truthy/falsy case
- non-literal test (e.g. `if (flag) {…}`) passes through unchanged
- `if (1 < 2) {A}` alone does NOT fold (comparison is constant-fold's job) — documents the layering
- `while (false) {body}` → `EmptyStatement`
- `while (true)` is left alone
- dead code after `ReturnStatement` dropped (with `removed-dead-code` contribution)
- block without `return` is unchanged
- `ConditionalExpression` with truthy test folds
- **untraced mode** folds silently (no contributions)
- pipeline integration solo
- **pipeline with constant-fold registered**: `if (1 < 2) {A}` flows through both passes and ends as just `A`. Verifies the canonical CLOC06 ordering does what it's supposed to.

### Skipped (queued for v0.3.0+)
- `ThrowStatement` / labelled `BreakStatement` / `ContinueStatement` as terminators — wait for Phase 2 to add the variants.
- `while (true)` infinite-loop collapse when body is provably pure.
- `SwitchStatement` with literal discriminant — Phase 2.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 canonical pass set — slots between `constant-fold` and `dce` in the canonical order.
- `FoldControlFlowPass` zero-sized type implementing `Pass`:
  - `name = "fold-control-flow"`
  - `depends_on = &["constant-fold"]` — folds expose statically-known conditions (`if (1+1===2)` → `if (true)`) that this pass then collapses.
  - `iteration_policy = IterationPolicy::FixedPoint` — eliminating one branch can expose another that's also statically dead.
  - `cost = 2` pass-units — matches constant-fold's weight (single tree walk with per-node local decisions).
  - `invalidates()` empty in v1 (informational only per CLOC06 Open Question 1).
- `FoldControlFlowPass::new()` zero-arg constructor.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there are no `IfStatement` / `WhileStatement` / `SwitchStatement` / `ConditionalExpression` nodes to fold. Pass through unchanged, `changed = false`, `nodes_touched = 1`, no contributions emitted (per CLOC03 §"When a pass keeps a node unchanged").
- 9 tests covering: `name()` value, `iteration_policy == FixedPoint`, `cost == 2`, `depends_on == ["constant-fold"]`, `invalidates` empty, identity run, **two-pass pipeline orders constant-fold before fold-control-flow** even when registered in reverse, **three-pass pipeline produces the canonical order** (constant-fold → fold-control-flow → dce) when all three are registered out of order, solo run with unknown deps silently dropped per the v0.1.0 scheduler, `Default` + `Clone` impls.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline`, `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar` (future side-effect attributes inform fold safety), `coding_adventures_correlation_vector` (`cv.delete()` + `"folded-branch"` `Contribution` per CLOC03), `serde_json`. Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`, `coding-adventures-closure-pass-constant-fold` for the two-pass ordering integration test, `coding-adventures-closure-pass-dce` for the three-pass ordering integration test.
- v1 is scaffolding. The full reachability/fold logic lands once `javascript-ast` grows the needed variants. The public surface (name, policy, cost, depends_on) stays put — no churn upstream.
- Followup PR: tighten `dce`'s `depends_on` from `["constant-fold"]` to `["constant-fold", "fold-control-flow"]` so the canonical order is structurally required, not incidental. Kept separate per the small-PR principle.
