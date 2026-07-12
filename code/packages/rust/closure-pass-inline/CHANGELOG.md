# Changelog

All notable changes to the `coding-adventures-closure-pass-inline` crate will be documented in this file.

## [0.28.0] - 2026-07-12

### Added — CLOC12.189 PR1: export declaration walk arms are no-ops and the splice predicates report nothing spliced

Exhaustive-match arms for the three new `Declaration::Export*` variants
(`ExportNamedDeclaration` / `ExportDefaultDeclaration` / `ExportAllDeclaration`).
PR1 keeps the nodes unreachable (no bridge yet), so the arms are conservative —
walk arms are no-ops and the splice predicates report nothing spliced. Proper descent into an `export const x = 1`'s inner declaration and the
renaming-soundness gate land with the bridge PR.

## [0.27.0] - 2026-07-11

### Added — CLOC12.188 PR1: `ImportDeclaration` arms

Exhaustive-match arms for the new `Declaration::ImportDeclaration` variant across
the walk and splice paths: an import has no inlinable body, so the walk arms are
no-ops and the void/valued-call splice predicates report nothing spliced.

## [0.26.0] - 2026-07-11

### Added — CLOC12.187 PR1: traverse `WithStatement`

New `TaggedStatement::WithStatement` arms in every statement walk (decl-name
count, use tally, expression inlining, void/valued splice, and used-ident
collection) descend into the `with` object and body. Picks up javascript-ast
0.38.0.

## [0.25.16] - 2026-07-11

### Added — CLOC12.176 PR1: `ClassMember::StaticBlock` arm

`javascript-ast` 0.35.0 added `ClassMember::StaticBlock(BlockStatement)`, the third class member (a `static { … }` initialization block). Added `StaticBlock` arms at all 13 sites. SOUNDNESS-critical: tally/inline/mutated-params/used-idents recurse the block's statements (a candidate use runs at class-def time — counted before inlining); substitute/rename use the class-inner map; `expr_node_count` weighs one unit per statement; `splice_void`/`splice_valued` splice into the block (its body IS a `Vec<Statement>`, unlike a field value).

## [0.25.15] - 2026-07-11

### Added — CLOC12.175 PR1: `ClassMember::Field` arms

`javascript-ast` 0.34.0 added `ClassMember::Field`, making every exhaustive
`ClassMember` match and every `let ClassMember::Method(m) = member` binding
non-exhaustive / refutable. Added `Field` handling at all 13 sites:

- **Soundness-critical** — `tally_decl`/`tally_expr` count candidate uses inside a
  field initializer and computed key; `inline_in_decl`/`inline_in_expr` substitute
  there in lockstep; `expr_collect_mutated_params` and `collect_used_idents_decl`
  over-collect from the initializer. Missing any would let the pass inline a callee
  still used at class construction.
- **Scope-aware** — `substitute`/`rename_in_expr` recurse the initializer and
  computed key with the class-inner map (the class's own name in scope, no method
  params).
- **Correctly skipped** — `count_decl_names_decl` (a field binds no
  statement-scope name) and `splice_void_in_decl`/`splice_valued_in_decl` (a field
  initializer is an expression, not a `Vec<Statement>`).

Reachable once the CLOC12.175 PR2 bridge produces the node.

## [0.25.14] - 2026-07-10

### Added — CLOC12.174 PR1: `Declaration::ClassDeclaration` match arms

`javascript-ast` 0.33.0 added the `Declaration::ClassDeclaration` variant. Added
arms at every exhaustive `Declaration` match site: `count_decl_names_decl`
(class name + method params + bodies), `tally_decl` and `inline_in_decl` (recurse
the heritage operand + method bodies, kept in lockstep so a candidate use inside
a class is never missed then wrongly inlined), `collect_top_level_decl_names`
(a top-level `class C` binds `C`), `splice_void_in_decl` / `splice_valued_in_decl`
(splice into each method body), and `collect_used_idents_decl`. All mirror the
existing `Expression::ClassExpression` handling. Reachable once the CLOC12.174
PR2 bridge produces the node.

## [0.25.13] - 2026-07-08

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

## [0.25.12] - 2026-07-07

### Changed — CLOC12.169: `ImportExpression` exhaustive-match arm

Added an `Expression::ImportExpression` case so the pass stays exhaustive over the new `javascript-ast` single-operand variant (part of the CLOC12.169 atomic node PR1). A dynamic `import(source)` carries one sub-expression (the module specifier), so the pass recurses into `source` exactly like the sibling `AwaitExpression` arm. No behaviour change to any existing node.


## [0.25.11] - 2026-07-07

### Changed — CLOC12.168: `ImportMeta` exhaustive-match arm

Added an `Expression::ImportMeta` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.168 atomic node PR1). `import.meta` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals. No behaviour change to any existing node.


## [0.25.10] - 2026-07-04

### Changed — CLOC12.167: `NewTarget` exhaustive-match arm

Added an `Expression::NewTarget` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.167 atomic node PR1). `new.target` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.25.9] - 2026-07-04

### Changed — CLOC12.166: `Super` exhaustive-match arm

Added an `Expression::Super` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.166 atomic node PR1). `super` is a leaf keyword with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.25.8] - 2026-07-04

### Changed — CLOC12.165: `ThisExpression` exhaustive-match arms

Added `Expression::ThisExpression` arms to the pass's exhaustive expression matches (node-count → 0 sub-nodes; the collect/tally/substitute/rename traversals → no-op), keeping the pass exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.165 atomic node PR1). `this` is deliberately left OUT of the trivial-/pure-expression predicates: `this` is bound at the call site, so treating it as a freely-substitutable primary would be unsound. The inliner therefore handles it conservatively. No behaviour change to any existing node.


## [0.25.7] - 2026-07-04

### Changed — CLOC12.164: `AwaitExpression` traversal arms

Added `Expression::AwaitExpression` arms that walk into the await's `argument`,
so the pass stays exhaustive over the new `javascript-ast` variant (part of the
CLOC12.164 atomic node PR1). No behaviour change to any existing node; the await
argument is now visited exactly like any other sub-expression the pass already
handles.


## [0.25.6] - 2026-07-03

### Changed — CLOC12.163: `YieldExpression` traversal arms

Added `Expression::YieldExpression` arms (7 across the pass's traversal
routines) that walk into the yield's optional `argument` when present, so the
pass stays exhaustive over the new `javascript-ast` variant (part of the
CLOC12.163 atomic node PR1). No behaviour change to any existing node; the
yield argument is now visited exactly like any other sub-expression the pass
already handles.


## [0.25.5] - 2026-07-03

### Changed — CLOC12.162: `SpreadElement` exhaustive-match arm

Added an `Expression::SpreadElement` arm that recurses into the spread's
`argument` so the pass stays exhaustive over the new `javascript-ast` variant
(part of the CLOC12.162 atomic node PR1). No behaviour change to any existing
node; the spread argument is now visited/rewritten exactly like any other
sub-expression the pass already handles.

## [0.25.4] - 2026-07-02

### Changed — CLOC12.161: handle `Expression::TaggedTemplateExpression`

Added a `TaggedTemplateExpression` match arm recursing into the `tag` callee
and each `${…}` insert of the applied template, so this pass keeps
compiling and traverses the new `javascript-ast` 0.20.0 node. No behaviour
change for any existing node.

## [0.25.3] - 2026-07-02

### Changed — CLOC12.160: handle `Expression::SequenceExpression`

Added a `SequenceExpression` match arm recursing into each operand so this
crate compiles and traverses the new `Expression::SequenceExpression`
variant. No behaviour change until the bridge produces sequence nodes
(CLOC12.160 PR2).


## [0.25.2] - 2026-07-02

### Changed — CLOC12.159: handle `Expression::NewExpression`

Added a `NewExpression` match arm mirroring `CallExpression` (recurse into the
callee and each argument) so this crate compiles and traverses the new
`Expression::NewExpression` variant. No behaviour change until the bridge
produces `new` nodes (CLOC12.159 PR2).


## [0.25.1] - 2026-07-02

### Changed — CLOC12.158: exhaustiveness for new `Expression::UpdateExpression`

Handle the new `Expression::UpdateExpression` (`++x` / `x++` / `--x` / `x--`)
variant added to `javascript-ast` (0.17.0): the pass recurses into the operand for node-count / binding-collection / tally / inline / substitute / mutated-param / rename walks. No behaviour
change for existing inputs — the bridge does not yet produce update
expressions (that lands in the CLOC12.158 PR2 bridge-enable), so these arms
are exercised only via hand-constructed AST today.

## [0.25.0] - 2026-07-02

### Added — CLOC12.154: `TemplateLiteral` traversal

Handle the new `Expression::TemplateLiteral` variant by recursing into its `${{…}}` sub-expressions (the `expressions` vector); the `quasis` are fixed leaf string segments with nothing to recurse. Part of the atomic `TemplateLiteral` enum-variant rollout (javascript-ast 0.16.0) — adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR. Template literals introduce no bindings or scopes, so the renaming/inlining arms need no map reduction.

## [0.24.0] - 2026-07-02

### Added — CLOC12.151: `ArrowFunctionExpression` traversal

Handle the new `Expression::ArrowFunctionExpression` variant by recursing into arrow bodies — both the block form (`x => { ... }`) and the concise/expression form (`x => expr`) — mirroring this pass's existing `FunctionExpression` handling. Part of the atomic `ArrowFunctionExpression` enum-variant rollout (javascript-ast 0.15.0); adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR. The arrow's params are removed from the active rename/substitute map before recursing into the body, so a param that shadows an outer binding of the same spelling is left untouched (arrows have no self-name to shadow).

## [0.23.0] - 2026-07-01

### Added — CLOC12.149: inline across `FunctionExpression` bodies

All seven expression walks gain a `FunctionExpression` arm: use tally,
call inlining, param mutation detection, node-count budget, binding-ident
collection, and `substitute`/`rename_in_expr` (the last two remove the
function's own name/params from the map before recursing, so a shadowed
reference is left untouched). Sound over function values.

## [0.22.0] - 2026-06-30

### Added — CV provenance for inlining (#89)

The pass now records an `inlined` correlation-vector contribution for every
function it dissolves, carrying `{name, sites}` — the original source name of
the helper and how many call sites its body was substituted into. Inlining
otherwise erases all trace of a helper: its declaration becomes unreferenced
(later removed) and its body is copied into each caller, so the minified output
has no record that a `helper(x)` call ever existed. These contributions let a
`--correlation_vector` consumer map inlined code back to the helper it came
from.

- All three inlining phases report: the expression inliner (`sites` = the call
  count it rewrote) and both statement-helper inliners (single-use, so
  `sites: 1`).
- Records are emitted in program (source) order, one per inlined function, so
  the contribution list is deterministic run to run.
- Attached at the program root — a coarse name→site-count *table*. Per-output-
  span tagging (each substituted body's own CV id) needs the log threaded
  through the clone-and-substitute recursion and is a documented follow-up,
  mirroring the rename passes' coarse-table-first approach.
- Emitted JS is byte-identical: contributions are pure metadata. Verified by
  the full closurec end-to-end suite (685 tests) plus the pass-pipeline suite.

Three new unit tests cover a single-use inline (`sites: 1`), a multi-use inline
(`sites: 2`), and the no-inline case (empty table).

## [0.21.0] - 2026-06-30

### Added — CLOC12 upstream test port (`InlineFunctionsTest`)

Ported the function-inlining cases from Google Closure Compiler's
`InlineFunctionsTest.java` into `tests/upstream/inline_functions_test.rs`,
following the CLOC12.01 convention (header cites the Java source; `UPSTREAM_SHA`
pins the tracked commit; `ATTRIBUTION.md` records Apache-2.0 provenance; a
`[[test]]` entry wires the file in). Each case drives the real
`source → bridge → inline → emit` chain and asserts on the emitted string — the
same surface upstream uses.

- **7 active `#[test]`s pass**: zero-param constant and string-literal returns,
  a tiny body inlined at two call sites, argument substitution into a member
  object (property name preserved), a call nested in a binary expression, and
  the two decline cases (a non-call use of the callee; a multi-use body over the
  size budget). Because only the inline pass runs, the dead callee declaration
  is retained (removed downstream) and no folding happens (`d(2)` → `2*2`).
- **6 `#[ignore = "blocked on gap-NNN"]` placeholders** record upstream
  behaviors not yet covered, pinned to `code/specs/CLOC12-gaps.md` (CLOC12.137):
  gap-127 local-declaration bodies, gap-128 `this`-using methods, gap-129
  function-expression/arrow bindings, gap-130 void side-effect-only calls,
  gap-131 explicit recursion guard.
- **gap-132 — surfaced by this port.** A compound (non-leaf) argument
  expression is declined rather than inlined: `function d(x){return x*2}
  g(d(a+b));` is left as `g(d(a+b));` where upstream produces `g((a+b)*2);`. The
  slice substitutes only simple (identifier/literal) arguments. This is a
  conservative miss (not a miscompile); the fix needs single-use-parameter
  detection plus precedence-preserving parenthesization. Documented as a
  follow-up fix candidate.

Test-only change — no production code touched. Crate version 0.20.0 → 0.21.0.

## [0.20.0] - 2026-06-20

### Added — CLOC23: function inlining across `for`-`of`

Every phase of the pass recurses through `ForOfStatement` (left / right
expression / body), counting the `left` declaration as the loop-variable
binding — identical to the `for`-`in` handling.

## [0.19.0] - 2026-06-20

### Added — CLOC22: function inlining across `for`-`in`

Every phase of the pass recurses through `ForInStatement` (left / right
expression / body), mirroring the for-statement handling. The for-in `left`,
when a declaration, is counted as a binding (the loop variable). Calls inside a
for-in body are now inlined like anywhere else.

## [0.18.0] - 2026-06-20

### Added — CLOC21: handle `DebuggerStatement`

Every phase of the pass now covers `DebuggerStatement` (grouped with the other
childless leaf statements) as a no-op (`false` for the contains-function checks).
Added to keep the matches exhaustive over the new AST variant.

## [0.17.0] - 2026-06-20

### Added — CLOC20: function inlining across `do`/`while`

Every phase of the pass recurses through `DoWhileStatement`:
`count_decl_names_stmt`, `tally_stmt`, `inline_in_stmt`, `splice_void_in_slot`,
`splice_valued_in_stmt`, and `collect_used_idents_stmt` now descend into the loop
body and test, mirroring the existing `while` handling. Calls inside a do-while
body are now inlined like anywhere else.

## [0.16.0] - 2026-06-20

### Added — CLOC19: function inlining across `try`/`catch`/`finally`

Every phase of the pass recurses through `TryStatement`:
`count_decl_names_stmt`, `tally_stmt`, `inline_in_stmt`, `splice_void_in_stmt`,
`splice_valued_in_stmt`, and `collect_used_idents_stmt` now descend into the
protected block, catch handler body, and finalizer. The catch `param` is counted
as a declared binding (the CLOC16 shadow-guard linchpin — every binding must be
counted program-wide so shadowing is detected) and is added to the used-idents
avoid set so inlined temporaries never collide with it. Calls inside try/catch
blocks are now inlined like anywhere else.

## [0.15.0] - 2026-06-19

### Added (CLOC18 — parameter-mutation materialization)

Helpers that **reassign a parameter** are now inlinable. They were *declined*
by the 0.13.1 soundness guard (#6272) because the inliner substitutes each
parameter with its argument expression, so `function f(x){ x = x + 1; return
x; }` at `var g = f(7)` would have miscompiled to `g = 7` instead of `8`. This
supersedes that decline with a sound transform — and unblocks the *ubiquitous*
default-argument idiom `function f(x){ x = x || DEFAULT; … }`, accumulators,
and normalization helpers.

Each mutated parameter is **materialised** into a fresh *mutable* local seeded
from the argument and routed through the rename map:

```js
function f(x){ x = x + 1; return x; } var g = f(7); use(g);
// inline pass:  let b = 7; b = b + 1; const a = b; var g = a; …
// SIMPLE:       var g = 8; use(g);     ✓
```

**Why the rename path, not substitution:** `substitute` deliberately does not
rewrite a bare-identifier assignment *target* (substituting a literal there is
impossible), so a mutated parameter must flow through the target-aware `rename`
walk. `materialize_args` now returns `(prelude, substitute_map,
mutated_rename)`: a mutated parameter emits `let <fresh> = <arg>;` and goes in
`mutated_rename` (merged into the local-rename map at both splice sites,
`build_spliced_body` and `build_captured_body`); a pure parameter emits `const
<fresh> = <arg>;` and substitutes as before. The fast direct-substitution path
now requires all-simple arguments **and** no mutated parameters.

**Soundness.** A materialised parameter is exactly a real call's binding: the
argument is evaluated once into the `let`; reassigning the local never affects
the caller's argument (pass-by-value); and the fresh `let` name is program-fresh
so its block scope is inert (the same argument as CLOC15 `var` locals). A
member-target write through a parameter (`x.k = …`) mutates a *property* of the
argument, not the binding, so it stays on the substitution path.

Five tests: the three #6272 decline tests **flipped** to materialisation-positive
(simple / compound / nested), plus a mixed pure+mutated-parameter case and a
side-effecting-argument-evaluated-once case. No closurec fixture churn. Spec:
CLOC18 (PR #6279), now marked implemented.

## [0.14.0] - 2026-06-19

### Added (CLOC15 Open Q3 — `var` locals admitted)

Helper bodies with a `var` local are now inlinable (previously the candidate
filter declined any `var` declaration, admitting only `let`/`const`). Example
(SIMPLE):

```js
function f(x){ var t = x + 1; return t * 2; } var g = f(7); use(g);
// after:  var g = 16; use(g);   (var local hoisted-and-renamed, then folded)
```

**Soundness.** A `var` is function-scoped and hoists to the top of the
*caller's* function on a flat splice, whereas `let`/`const` stay block-scoped.
That difference is **observationally inert here** because every callee local —
`var` included — is alpha-renamed to a program-fresh name (one that appears in
no declaration or use anywhere in the program, per the existing condition-5
`avoid` set). Nothing reads or writes the fresh name except the spliced body,
in source order, so where the declaration hoists to cannot matter. The
collision case is the crux and is tested: a caller binding `var t = 9` is left
untouched while the helper's `var t` is renamed to a fresh `b`.

The bridge desugars `var t = E` into `var t; t = E`, so an admitted `var`-local
body contains an assignment to that local. That is sound under renaming (the
`rename` walk rewrites both the declaration id and the assignment target) and is
**not** flagged by the 0.13.1 parameter-mutation guard, which only declines
assignment to a *parameter* — a local is fine.

Four tests: one void-helper positive (flipped from the old decline test), one
value-capture positive, one local-reassignment positive, and the
caller-collision case. No closurec fixture churn.

## [0.13.1] - 2026-06-19

### Fixed (soundness) — decline helpers that reassign a parameter

A helper that **reassigns one of its parameters** (`function f(x){ x = x + 1;
return x; }`) was being inlined, producing a **miscompile**: inlining
substitutes each parameter occurrence with its *argument expression*, treating
the parameter as an immutable value, so `var g = f(7)` became `var g = 7`
(plus a stray global write `x = 8`) instead of the correct `var g = 8`. The
same broke in `return`/assignment capture positions.

This was a latent hazard that only became **reachable** once assignment-
expression statements parsed (the CLOC17 grammar fix, `javascript-parser`
0.9.0) — before that, any helper body containing `x = …` made the whole
program fall back to whitespace-only, so the inliner never saw it.

The candidate filter now declines any helper whose body assigns to a parameter
(`x = …`, `x += …`, or nested forms like `y = (x = 5)` / `f(x = 5)` — a new
recursive `body_assigns_to_param` walk covers every expression position). A
member-target whose *base* is a parameter (`x.k = 5`) is still admitted: it
mutates a property of the argument, not the parameter binding, and is sound
under substitution. (`++`/`--` parameter mutation is not reachable — the typed
AST has no `UpdateExpression` yet.)

Correctly inlining a mutated parameter would require materialising it into a
fresh local seeded from the argument; that is left as a future slice.
Declining is never a miscompile. Four new unit tests (three decline cases, one
positive control proving a *free*-variable assignment still inlines). No
closurec fixture churn.

## [0.13.0] - 2026-06-19

### Added (CLOC15 PR-6 — assignment-target value capture, `g = f(x)`)

Completes the value-capture family (after PR-3 const-init `const r = f(x)` and
PR-5 return `return f(x)`) with the assignment-target case — CLOC15 Open
Question 2's last form. A single-use multi-statement helper whose call is the
entire right-hand side of a **simple assignment to a bare identifier**
(`g = f(x)`) now has its body hoisted before the assignment, with the callee's
tail `return E` re-emitted as `g = E` — no temp, the value flows straight into
the target. Example (SIMPLE, post-fold):

```js
function f(x){ side(); return x * 2; } var g; g = f(7); use(g);
// before:  function f(x){side();return x*2};var g;g=f(7);use(g);
// after:   var g;side();g=14;use(g);
```

This was prototyped during PR-5 but could not fire — or even be unit-tested
through the bridge-based `inline_source` harness — until assignment-expression
statements parsed (the CLOC17 grammar fix, `javascript-parser` 0.9.0). It is
now reachable.

**Soundness (why the gate is narrow):**
- Only the simple `=` operator is admitted. **Compound** assignment
  (`g += f(x)`) reads the old `g` *before* the call runs; hoisting the body
  ahead (`body…; g += E`) would read `g` *after* the body's effects — if the
  body mutates `g` the two differ, so it is declined.
- Only a **bare identifier** target is admitted. **Member** targets
  (`obj.k = f(x)`) evaluate the reference to `obj` *before* the call; hoisting
  the body ahead could reorder observable effects (an `obj` getter, or the body
  mutating `obj`), so they are declined.
- The call must be the **entire** right-hand side (a `CallExpression`); a
  `g = f(x) + 1` RHS is a `BinaryExpression` and is declined.
- A void helper (no tail-return *value*) is not a valued candidate, so
  `g = f(1)` against such a helper is left intact.

Implemented as a third `CaptureTail::IntoAssignment` variant sharing
`build_captured_body` with the const-init and return paths, plus a
`capture_splice_for_assignment` recognizer wired into `try_capture_in_stmt`.
Six new unit tests (one positive, one composing local-rename + non-simple-arg
materialisation, four decline cases). No closurec fixture churn.

## [0.12.0] - 2026-06-19

### Added (CLOC16 Slice B1 — nested splice sites via a global-uniqueness gate)

Slice A admitted a free identifier resolving to a top-level declaration but
restricted such a candidate to **top-level** splice sites (where no scope can
shadow a top-level name). Slice B1 lifts that restriction for the common case
**without any scope walk**, using a global-uniqueness check:

```js
function dep() { return 1; } dep(); dep();
function f()   { log(0); use(dep); }
function main(){ f(); }            // f's only call — NESTED
main(); main();
// SIMPLE ⇒ function main(){ log(0); use(dep) }   (f now spliced into main)
```

**The gate.** `count_decl_names_*` counts **every** binding declaration at
every depth program-wide (its catch-all arm is deliberately exhaustive), so
`decl_counts[name]` is exact. If a top-level free ident has
`decl_counts[name] == 1`, it is declared **exactly once** in the entire program
(the top-level declaration) — **no other binding of that name exists anywhere**,
so it cannot be shadowed at *any* splice site. Such a name therefore behaves
like a true global for splice-location purposes: the candidate carries no
top-level-only obligation and inlines even at nested sites.

A top-level name that is **also** declared elsewhere (`decl_counts > 1`) keeps
the Slice A top-level-only obligation — a local of that name could shadow it at
a nested site, so the splice is declined there:

```js
function dep() { keep(); return 1; } dep();
function f()   { log(0); use(dep); }
function g()   { let dep = 99; f(); }   // dep declared twice ⇒ f stays top-level-only
g(); g();
// f is NOT inlined into g by the inline pass in isolation (decl_counts[dep] == 2).
```

This is sound on the pass's own terms: the gate reads `decl_counts` of the
program the pass actually receives, so a `== 1` count genuinely means the name
is unshadowable in that program. (An earlier pass that renamed away a shadowing
local merely makes the program the inliner sees have no shadow — still sound.)

**Implementation.** One classification branch in `void_candidate_from_function`:
a top-level free ident with `decl_counts == 1` is treated like a true global
(no obligation); `> 1` keeps the Slice A `free_top_level` (top-level-only)
behaviour. The splice-site guards are unchanged. 4 new/repurposed tests (76
total): the two former Slice A nested/block *decline* tests used a
singly-declared `const K`, so they are now positive Slice B1 cases
(`inlines_unique_top_level_ref_at_nested_site`, `inlines_unique_top_level_ref_in_block`);
new decline/admit tests cover the multiply-declared name at nested vs. top-level
sites. Resolves CLOC16 **Slice B1**; **Slice B2** (an in-scope-binding scope
walk to admit the multiply-declared, genuinely-unshadowed nested case) remains
future work.

## [0.11.0] - 2026-06-19

### Added (CLOC16 Slice A — free idents resolving to top-level declarations)

The statement inliner previously admitted a free identifier in a helper body
**only if it was declared nowhere in the program** (a true global like
`Math`/`console`). That rejected the common case where a helper references
another **top-level declaration** — a sibling `function`, a top-level
`const`/`let`/`var`. Slice A of [CLOC16](../../../specs/CLOC16-inline-free-identifier-widening.md)
admits those references, gated by a **sound, location-restricted** rule:

```js
function dep(x) { trace(x); return x * 2; } dep(0);
function f(p)   { log(p); use(dep(p)); }
f(5);
// SIMPLE ⇒ …; log(5); use(dep(5));   (f spliced at top level, declaration removed)
```

**The soundness rule.** A free ident that resolves to a top-level declaration
is recorded on the candidate (`free_top_level`). Such a candidate is spliced
**only when its single call is a direct `program.body` member** — at program
scope no intervening binding can shadow a top-level name, so the reference
resolves identically in the helper and at the splice site. At any **nested**
call site (inside a function, a block, an `if`/loop) the splice is **declined**
(the call is left intact — declining is never a miscompile), because a local of
the same name could capture the reference:

```js
function dep() { return 1; } dep(); dep();
function f()   { log(0); use(dep); }
function g()   { let dep = 99; f(); }   // f's only call — NESTED, under a local `dep`
g(); g();
// f is NOT inlined into g (would misread the local `dep`); it is left intact.
```

A free ident declared **only inside some other function** (never at program
scope) is still rejected outright — its resolution at an arbitrary splice site
can't be proven.

**Zero regression.** A candidate with no top-level free idents has an empty
`free_top_level` and splices everywhere exactly as before — the full existing
suite is unchanged and there is **no closurec fixture churn**. Both inline
paths (void and the valued PR-3/PR-5 capture) share the candidate gate, so the
top-level-only obligation is enforced at every program-level splice entry
point; a `return f(x)` capture is never at program scope, so it is simply
unreachable for `free_top_level` candidates (sound — no value there until
Slice B).

**Implementation.** New `collect_top_level_decl_names` (program-scope decl
names, direct `program.body` members only); `void_candidate_from_function`
classifies each free ident (param/local → splice; top-level decl →
`free_top_level`; true global → unchanged; otherwise reject);
`splice_void_call_program` and `splice_valued_call_program` skip their
nested-recursion calls when `free_top_level` is non-empty. 5 new/repurposed
tests (74 total) — including the nested-decline, top-level-block-decline, and
declared-only-in-other-function cases. Resolves CLOC16 Slice A; **Slice B**
(nested splice sites via an in-scope-binding walk) remains future work.

> The previously-named `does_not_inline_void_helper_with_free_declared_name`
> test asserted the *old* declined behaviour for `const K = 5; function f() {
> sink(K); } f();`. That is a Slice A **positive** case (top-level const,
> top-level call), so it has been repurposed to
> `inlines_top_level_helper_referencing_top_level_const` asserting the new
> inlined output — an intended behaviour change, flagged in CLOC16.

## [0.10.0] - 2026-06-19

### Added (CLOC15 PR-5 — value capture in `return`-argument position)

The value-capture path now admits the call appearing as the **entire argument
of a `return` statement** (`return f(x)`), the ubiquitous "tail-call a helper"
shape. PR-3 already handled `const r = f(x)` / `let r = f(x)`; this slice adds
the return position, where the helper's tail value becomes the **caller's own
return value** — with no temp, because the value flows straight out:

```js
function helper(p) { log(p); return p + 1; }
function main()    { return helper(3); }
main(); main();
// SIMPLE  ⇒  function main(){ log(3); return 4 }  (helper declaration removed)
```

Soundness: `return` is a terminator, so the single `return f(x)` is the last
reachable statement on its path. Replacing it with `body…; return E` runs the
body's effects exactly as they ran inside the callee before its own return,
then returns the same value; any statement textually after `return f(x)` was
dead before and remains dead after.

Declined positions (the call is not the *entire* return argument, so hoisting
the body would change evaluation order):

- `return cond && f(x)` — the call is the right operand of `&&`
  (a `LogicalExpression`, not a bare call);
- `return c ? f(x) : y` — the call is a branch of a `ConditionalExpression`;
- `return f(x)` where `f` has no tail-return *value* (a bare `return;`) — a
  void candidate, not a valued one, so nothing is synthesized.

Composes with the existing machinery: callee locals are alpha-renamed
program-fresh and non-simple arguments are materialised once into per-argument
temps (PR-4a) before the spliced body, exactly as in the `const`-init path.

**Implementation.** `build_captured_body` now takes a `CaptureTail` —
`IntoTemp(&temp)` (the PR-3 `const <temp> = E;` tail) or `AsReturn` (the new
`return E;` tail) — so the rename / substitute / arg-materialisation logic is
shared and only the final statement differs. `try_capture_in_stmt` matches a
`ReturnStatement` whose argument is exactly the target call via the new
`capture_splice_for_return`. 5 new tests (70 total).

This resolves CLOC15 spec **Open question 2**'s return-argument case.

## [0.9.0] - 2026-06-19

### Added (CLOC15 PR-4b — `if` without an early exit in the body)

The statement-inlining body shape now admits an `IfStatement`, so a helper
with conditional logic inlines:

```js
function guard(x) { if (x > 0) accept(x); else reject(x); }
guard(value);
// SIMPLE  ⇒  value > 0 ? accept(value) : reject(value);
//   (inlined, then fold-control-flow turns the if/else into a ternary and
//    treeshake removes the dead declaration)
```

Admitted **only** when the `if` is control-flow-inert and declaration-free —
each branch is an `ExpressionStatement` or a block of `ExpressionStatement`s
(`is_inlinable_if`). That excludes:

- `return` / `break` / `continue` inside a branch — an early exit a flat
  splice would mis-scope (the caller's following statements would still run);
- nested `let` / `const` / `var` declarations — block-scoped locals the
  name-based alpha-renamer cannot shadow-correctly;
- nested `if` / loops / other control constructs (kept for a later slice).

So an admitted `if` introduces no new local and no early exit: splicing it
into the straight-line body is observationally inert. The test expression is
unrestricted; its identifiers are vetted by the normal free-identifier walk
(param / callee-local / true-global), and the `rename`/`substitute` passes
now recurse into the `if` (test + branches) and its blocks. Composes with
PR-2 (an `if` may precede the tail return), PR-3 (value capture), and PR-4a
(non-simple arguments).

- 8 new tests: unbraced-branch `if`, block-branch `if`, an `if` whose test
  reads a renamed local, an `if` with a non-simple argument, three decline
  cases (early `return` in a branch, a nested declaration, a nested `if`),
  and a **dangling-else soundness guard** — a helper whose body ends in an
  else-less `if`, called from a braceless `if`-consequent that has a caller
  `else`: the single-statement-slot splice block-wraps the body
  (`if(c){if(v)a(v);}else other();`) so the `else` binds to the outer `if`,
  never the inner one. 65 pass-crate tests; full closurec suite + both
  downstream consumers green, no fixture churn.

## [0.8.0] - 2026-06-19

### Added (CLOC15 PR-4a — non-simple arguments via per-argument temps)

PR-1..PR-3 required every argument of an inlined statement-helper call to be
*simple* (a literal or bare identifier), so `f(obj.x)`, `f(a + 1)`, `f(g())`
were all declined. PR-4a lifts that for the statement-inlining paths (the
void/discard pass and the value-capture pass) by **materialising arguments
into temps**:

```js
function log2(x) { trace(x); record(x); }
log2(compute());
// SIMPLE  ⇒  const a = compute(); trace(a); record(a);
//   (compute() is evaluated ONCE, captured, and read twice — never
//    duplicated to trace(compute()); record(compute()))
```

`materialize_args`:

- **All arguments simple** → unchanged: direct substitution, no temps. This
  preserves the existing single-pass output byte-for-byte, so there is **no
  fixture churn**.
- **Any argument non-simple** → hoist EVERY argument into a fresh `const`
  temp, in source order, before the spliced body, and substitute each
  parameter with its temp. This evaluates all arguments left-to-right exactly
  once (JS call semantics) and captures their values, so a parameter used N
  times reads the captured value rather than re-evaluating the argument. The
  redundant temps on the simple arguments are removed downstream by
  `inline-variables` + `constant-fold`.

Composes with PR-3: a result-used helper called with a non-simple argument
hoists the arg temp before the captured body
(`var x = f(obj.y)` ⇒ `const b = obj.y; …; const a = …; var x = a;`).

**Soundness.** Any argument expression is admissible because the temp
captures its value once at the splice point — a throwing argument still
throws at the same point (before the body); a side-effecting argument runs
once. Arg temps are program-fresh, minted from the same `avoid` set as the
callee-local renames and **before** them, so the two name spaces are
disjoint. (Arguments the front-end cannot bridge — e.g. assignment
expressions — never reach the pass: the program falls back to whitespace-only
minification.)

**Plumbing.** The statement-path gate now counts name+arity matches
(`Tally::arity_calls`, via `name_use_and_arity_calls`) rather than the
expression inliner's simple-arg `inlinable` count, and `is_void_target_call`
drops its `is_simple_arg` requirement (name + arity only). The expression
inliner's `is_inlinable_call` is unchanged.

- 5 new / changed tests (side-effecting arg via temp — the former
  `does_not_inline_…_side_effecting_argument` now inlines; member arg used
  twice; left-to-right temping of mixed args; non-simple arg in value
  position; the all-simple no-temp no-churn guarantee). 57 pass-crate tests;
  full closurec suite + both downstream consumers green, no fixture churn.

## [0.7.0] - 2026-06-19

### Added (CLOC15 PR-3 — result-used helpers captured into a hoisted temp)

PR-1/PR-2 inlined a multi-statement helper only when its result was
**discarded** (the call was a statement). PR-3 handles the case where the
result is **used**: the body is hoisted to before the enclosing statement
and the tail-return value captured into a fresh temp.

```js
function compute(a) { const t = a + 1; return t * 2; }
var x = compute(5);
// SIMPLE  ⇒  var x = 12;
//   (body hoisted + return captured ⇒ `const u = 5+1; const v = u*2;
//    var x = v;`, then constant-fold + inline-variables + treeshake finish)
```

**The soundness crux is evaluation order:** hoisting the body to *before*
the enclosing statement runs it before anything else that statement
evaluates, which is sound only when nothing in the statement is evaluated
before the call. The airtight subset admitted — the call is the **entire
initializer of a single-declarator** `var`/`let`/`const`:

- `var x = compute(5);` → admitted (the call is the whole init, always
  evaluated, nothing before it);
- `var x = a + compute(5);` → declined (`a` is evaluated first);
- `var x = f(), y = compute(5);` → declined (multi-declarator ordering);
- `var x = h(compute(5));` → declined (call is not the init's top expr);
- a body with no tail-return value (`return;` / no return) → declined
  (nothing to capture).

All of PR-1/PR-2's guards still apply (single-use, no `this`/`arguments`,
callee locals alpha-renamed to program-fresh names, free identifiers are
true globals, side-effect-free args). The capture temp is itself a
program-fresh name minted before the locals so it cannot collide. Broader
value positions (assignment targets, `return` arguments, reordering-safe
operand positions) are later slices on this same machinery.

- New Phase 5 after the void inliner; runs second so the void pass consumes
  any discarded-statement use first, leaving only the value-position use.
- 7 new tests (the signature capture, free-global side effect hoisted, `let`
  binding, and four decline cases: call-not-the-whole-init, nested-call
  argument, multi-declarator, void-body-used-as-value). 53 pass-crate tests
  total; full closurec suite + both downstream consumers green (no churn).

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
