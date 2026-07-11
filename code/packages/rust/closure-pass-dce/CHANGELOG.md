# Changelog

All notable changes to the `coding-adventures-closure-pass-dce` crate will be documented in this file.

## [0.20.15] - 2026-07-11

### Added — CLOC12.175 PR1: `ClassMember::Field` arms

`javascript-ast` 0.34.0 added `ClassMember::Field`. Added `Field` arms (in both
the class-expression and class-declaration member loops) that run `dce_expression`
over the field's initializer and computed key, mirroring the `Method` arm.
Reachable once the CLOC12.175 PR2 bridge produces the node.

## [0.20.14] - 2026-07-10

### Added — CLOC12.174 PR1: `Declaration::ClassDeclaration` match arms

`javascript-ast` 0.33.0 added the `Declaration::ClassDeclaration` variant. Added
arms at each of this crate's exhaustive `Declaration` match sites: the transform
(`dce_class_declaration` runs DCE inside the heritage operand and method bodies,
mirroring `dce_class`), and two conservative predicates — `tail_is_safe_to_truncate`
and `block_is_scope_safe_to_flatten` both return `false` for a class declaration
(a name-binding, block-scoped node, like a function declaration; preserving it is
never a miscompile). Reachable once the CLOC12.174 PR2 bridge produces the node.

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

Added an `Expression::ImportExpression` case so the pass stays exhaustive over the new `javascript-ast` single-operand variant (part of the CLOC12.169 atomic node PR1). A dynamic `import(source)` carries one sub-expression (the module specifier), so the pass recurses into `source` exactly like the sibling `AwaitExpression` arm. No behaviour change to any existing node.


## [0.20.11] - 2026-07-07

### Changed — CLOC12.168: `ImportMeta` exhaustive-match arm

Added an `Expression::ImportMeta` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.168 atomic node PR1). `import.meta` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals. No behaviour change to any existing node.


## [0.20.10] - 2026-07-04

### Changed — CLOC12.167: `NewTarget` exhaustive-match arm

Added an `Expression::NewTarget` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.167 atomic node PR1). `new.target` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.20.9] - 2026-07-04

### Changed — CLOC12.166: `Super` exhaustive-match arm

Added an `Expression::Super` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.166 atomic node PR1). `super` is a leaf keyword with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.20.8] - 2026-07-04

### Changed — CLOC12.165: `ThisExpression` exhaustive-match arm

Added an `Expression::ThisExpression` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.165 atomic node PR1). `this` is a leaf keyword with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals. No behaviour change to any existing node.


## [0.20.7] - 2026-07-04

### Changed — CLOC12.164: `AwaitExpression` exhaustive-match arm

Added an `Expression::AwaitExpression` arm that rebuilds the node, recursing
into its `argument`, so the pass stays exhaustive over the new `javascript-ast`
variant (part of the CLOC12.164 atomic node PR1). No behaviour change to any
existing node; the await argument is now visited/rewritten exactly like any
other sub-expression the pass already handles.


## [0.20.6] - 2026-07-03

### Changed — CLOC12.163: `YieldExpression` exhaustive-match arm

Added an `Expression::YieldExpression` arm that rebuilds the node, preserving
`delegate` and recursing into the optional `argument` when present, so the pass
stays exhaustive over the new `javascript-ast` variant (part of the CLOC12.163
atomic node PR1). No behaviour change to any existing node; the yield argument
is now visited/rewritten exactly like any other sub-expression the pass already
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
variant added to `javascript-ast` (0.17.0): the pass treats `x++`/`--x` as a side effect: it is NOT dead code even in value-discarded position, and is preserved (recursing only into the operand). No behaviour
change for existing inputs — the bridge does not yet produce update
expressions (that lands in the CLOC12.158 PR2 bridge-enable), so these arms
are exercised only via hand-constructed AST today.

## [0.20.0] - 2026-07-02

### Added — CLOC12.154: `TemplateLiteral` traversal

Handle the new `Expression::TemplateLiteral` variant by recursing into its `${{…}}` sub-expressions (the `expressions` vector); the `quasis` are fixed leaf string segments with nothing to recurse. Part of the atomic `TemplateLiteral` enum-variant rollout (javascript-ast 0.16.0) — adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR. Template literals introduce no bindings or scopes, so the renaming/inlining arms need no map reduction.

## [0.19.0] - 2026-07-02

### Added — CLOC12.151: `ArrowFunctionExpression` traversal

Handle the new `Expression::ArrowFunctionExpression` variant by recursing into arrow bodies — both the block form (`x => { ... }`) and the concise/expression form (`x => expr`) — mirroring this pass's existing `FunctionExpression` handling. Part of the atomic `ArrowFunctionExpression` enum-variant rollout (javascript-ast 0.15.0); adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR.

## [0.18.0] - 2026-07-01

### Added — CLOC12.149: DCE inside `FunctionExpression` bodies

`dce_expression` recurses into a `FunctionExpression` body via
`dce_block_statement` (mirroring `dce_declaration` for a
`FunctionDeclaration`), so dead code after a `return`/`throw` inside a
function value is eliminated.

## [0.17.0] - 2026-07-01

### Added — upstream `UnreachableCodeEliminationTest.java` conformance port (#88, CLOC12.148)

The **second** CLOC12 upstream-test port into this crate (alongside the
`PeepholeRemoveDeadCode` port). New file
`tests/upstream/unreachable_code_elimination_test.rs` (registered as the
`upstream_unreachable_code_elimination` test target) reshapes upstream
`UnreachableCodeEliminationTest.java` onto our typed-AST surface — built by
hand via the same helper style as the sibling port — asserting the surviving
function-body statements after running only `DcePass`.

- **8 active `#[test]`s pass on the first run** (no new DCE defect):
  drop-single/multiple statements after `return`, drop after `throw`, keep
  reachable code before the terminator, bare `return` unchanged, empty-statement
  removal, dead-code cleanup inside a nested (`if`-consequent) block, and the
  hoisting-soundness *decline* — a dead tail carrying a `var` is kept verbatim
  (declining to truncate is never a miscompile).
- **2 `#[ignore = "blocked on gap-NNN"]` placeholders** pin the CFG-based
  reachability upstream does that this pass does not — gap-151 (code after an
  `if` whose every branch terminates) and gap-152 (code after `break`/`continue`
  in a general loop block; ours only treats them as terminators in switch
  cases). Each is pinned to `code/specs/CLOC12-gaps.md`.

Notably the nested-block case surfaced (and now documents) the interaction with
the existing block-flattening step: a bare `{ return }` block flattens into its
parent, so the port wraps the nested block in an `if`-consequent to test
interior cleanup in isolation.

This is a **test-only** change: no `src/` file is touched, so there is no
ripple into downstream consumers. Bumps the crate 0.16.0 → 0.17.0.

## [0.16.0] - 2026-06-30

### Added — correlation-vector deletion provenance (#89, full CV tracing)

DCE now records *why code disappeared* in the correlation-vector log. Before
this change the pass received the shared `CVLog` but explicitly discarded it
(`let _ = self.cv`), pushing only a coarse summary `Contribution` against the
enclosing container. So a `--correlation_vector` consumer that asked "what
happened to the span at 42:3-42:19?" got no answer — the removed node simply
vanished from the provenance graph. For a minifier whose entire premise is
auditability, silently deleting code is the one thing it must never do.

Each removal site now **tombstones every removed node individually** via
`CVLog::delete(cv_id, "dce", <reason>, meta)`, populating that node's own
`DeletionRecord` so the span remains queryable forever with a definite answer:
*dce removed it, because `<reason>`*. Wired at all five statement-list removal
sites, each carrying its precise reason tag and the enclosing `container_cv`:

- `removed-dead-code` — statements after a block-level terminator;
- `removed-dead-code-in-case` — statements after a `switch`-case terminator;
- `removed-empty-statement` — `;` swept from a block body;
- `removed-debugger` — `debugger;` stripped from a block body **and** from the
  program top level (two distinct code paths, each covered).

`block-flattened` deliberately does **not** tombstone: flattening *moves* a
nested block's statements up one scope level rather than deleting them, so those
nodes must stay live in the log. A regression test pins this distinction.

New helpers: `DceState::record_deletion` (tombstone + keep the summary
contribution), and `statement_cv` / `tagged_statement_cv` / `program_item_cv`
to fetch a removed node's own CV id (exhaustive match — a new statement kind
fails to compile rather than silently losing provenance).

Zero cost on the production hot path: `delete` is a no-op when the CV log is
disabled (the default), so tombstones only materialise under
`--correlation_vector`. AST output is byte-for-byte unchanged — the same nodes
are removed and the same summary contributions emitted as before; only the CV
log gains the deletion records. Six new tests assert each removal tombstones
its node with `source == "dce"` and the right reason, that flattening does not
tombstone, and that a disabled log still strips code without panicking.

## [0.15.0] - 2026-06-20

### Added — CLOC24: strip `debugger` statements at SIMPLE/ADVANCED

The pass now removes `debugger;` statements from statement lists, matching the
upstream Closure Compiler. A `debugger` statement is a development-only
breakpoint: it pauses execution only when a debugger is attached and is a no-op
otherwise, so removing it from a shipped program is semantics-preserving. Two
sweeps cover the two list contexts:

- **block bodies** — `dce_block_statement` retains out `debugger` statements
  alongside the existing empty-statement sweep (`{ x; debugger; y; }` →
  `{ x; y; }`; a block of only `debugger`s collapses to `{}`);
- **program top level** — `dce_program` sweeps `debugger` from the program body
  (which is a list of `ProgramItem`s, not a `BlockStatement`).

Because the dce pass runs only inside the typed (SIMPLE/ADVANCED) pipeline,
`debugger` is preserved at WHITESPACE_ONLY — exactly the upstream behaviour. A
new `removed-debugger` contribution is recorded per sweep (and flips `changed`).

**Documented limitation:** a `debugger` reaching a *non-list* position (e.g. a
brace-less `if (c) debugger;` consequent) is left intact — the sweep is
list-scoped, consistent with how the empty-statement sweep already works.

- 4 new tests: block strip, top-level strip, all-`debugger` block → empty, and
  the preserved brace-less-consequent limitation.

## [0.14.0] - 2026-06-20

### Added — CLOC23: DCE inside `for`-`of`

New `ForOfStatement` arm recurses dead-code elimination into the loop's left,
right expression, and body. Like the other loops, a for-of is NOT a terminator
(the iterable may be empty), so code after it stays reachable.

## [0.13.0] - 2026-06-20

### Added — CLOC22: DCE inside `for`-`in`

New `ForInStatement` arm recurses dead-code elimination into the loop's left,
right expression, and body. Like the other loops, a for-in is NOT a terminator
(the body may run zero times), so code after it stays reachable.

## [0.12.0] - 2026-06-20

### Added — CLOC21: handle `DebuggerStatement`

`dce_tagged_statement` now covers `DebuggerStatement` (grouped with the other
childless leaf statements), preserved as-is. A `debugger;` is a no-op leaf with
no bindings; stripping it is intentionally left as future work, so v1 keeps it.

## [0.11.0] - 2026-06-20

### Added — CLOC20: DCE inside `do`/`while`

New `DoWhileStatement` arm recurses dead-code elimination into the loop body and
test. Like `while`, a `do`-`while` is NOT a terminator — control can fall out of
the loop — so a statement following it stays reachable. (A do-while in a dead
tail is preserved by the existing `tail_is_safe_to_truncate` whitelist, which
defaults new compound statements to unsafe.)

## [0.10.0] - 2026-06-20

### Added — CLOC19: DCE inside `try`/`catch`/`finally`

New `TryStatement` arm recurses dead-code elimination into the protected block,
the catch handler body, and the finalizer. Dead-after-terminator and
empty-statement cleanup now apply within those blocks (e.g. a `dead()` call after
a `return` inside a `catch` handler is truncated). The catch `param` is preserved
verbatim, and the `try` statement is explicitly NOT treated as a terminator — a
statement following a `try`/`catch` remains reachable because the handler can
catch and continue. Added unit tests for both behaviours.

## [0.9.0] - 2026-06-19

### Added — dead-code-after-`throw` at block level

`throw E;` unconditionally ends the execution of its enclosing statement list
(it raises out of the function, propagating through any `try`/loop/block), so
every statement after it in the same `BlockStatement.body` is unreachable —
exactly like `return`. The block-level `is_terminator` now recognizes
`ThrowStatement` in addition to `ReturnStatement`, so:

```js
function f(x){ throw x; dead(); }
// before:  function f(x){ throw x; dead(); }
// after:   function f(x){ throw x; }

{ throw e; cleanup1(); cleanup2(); }
// after:   { throw e; }
```

This was a real gap: the switch-case terminator check (`is_case_terminator`)
already treated `throw` (and `break`) as flow-ending, but the general
block-level check dropped code only after `return`. Guard clauses and error
paths are common — and an `inline` that splices a helper body flat after a
guard makes them even more common on post-optimization code.

`break` / `continue` remain excluded at the general block level: they terminate
flow only relative to an enclosing loop/switch, which this context-free check
cannot prove (and a bare `break` in a function-body block is a SyntaxError a
faithful parser never produces). They are still handled inside switch-case
consequents via `is_case_terminator`.

### Fixed (soundness) — don't drop hoisted declarations from a dead tail

While extending dead-after-terminator to `throw`, found and fixed a latent
**miscompile** that also affected the pre-existing `return` path: `var` and
`function` declarations are **hoisted** to the top of the enclosing function
regardless of textual position, so a declaration in the otherwise-unreachable
tail still creates a binding that code *before* the terminator can observe. The
truncation was dropping it:

```js
function f(){ h(); throw e; function h(){} }
// h() is callable because `function h` hoists past the throw.
// Dropping `function h(){}` turned h() into a ReferenceError — a miscompile.
// (The same happened after `return`.)
```

The dead-after-terminator truncation now truncates **only** when the
unreachable tail is provably free of a hoisted binding, via a new
`tail_is_safe_to_truncate` **whitelist**: the tail is dropped only if every
statement is an `ExpressionStatement`, `EmptyStatement`, `break`, `continue`,
`return`, `throw`, or a `let`/`const` declaration. A `var` or `function`
declaration — OR any **compound** statement (`if` / `while` / `for` / block /
`switch` / labeled) that could transitively wrap a hoisted `var` (e.g.
`if (c) var y;`, `for (var i …)`) — makes the tail unsafe, so it is preserved.
(A top-level-`Declaration`-only check would miss the compound cases.) `let` /
`const` are block-scoped and not hoisted, so a tail of only those is still
dropped. Declining to drop dead statements is never a miscompile; a truly-unused
hoisted declaration is still removed downstream by `remove-unused-vars`. New
statement variants default to unsafe (preserved) until vetted.

Six new unit tests (drop-after-throw; the no-op last-statement case; the
hoisting guard for `throw` and `return`; compound-statement-tail preservation;
and a droppable `let`-only tail). No closurec fixture churn.
## [0.8.0] - 2026-06-04

### Added — CLOC12.36: constant-discriminant switch collapse (gap-014 step 4/N)

Third (and last simple-to-frame) peephole on top of the
SwitchStatement AST. **Closes gap-014's substantive
optimisation work.**

When a `SwitchStatement`'s discriminant is a pure leaf literal
AND every case `test` is `None` (the default clause) or also a
pure leaf literal, the pass compile-time evaluates which case
runs and replaces the entire switch with a `BlockStatement`
holding the matched case's consequent.

### The rule

1. Find the first case whose `test` is strict-equal to the
   discriminant (per ECMAScript §IsStrictlyEqual, restricted to
   the literal subset).
2. If no case matches, fall back to the `default:` case if one
   exists.
3. Replace the switch with `BlockStatement(matched.consequent)`
   with any trailing `BreakStatement` stripped (it's spurious
   — there's no switch to exit). `ReturnStatement` /
   `ThrowStatement` at the end stay (they have observable
   behaviour beyond just exiting the switch).
4. No match AND no default → switch runs nothing; replace with
   `EmptyStatement`. Discriminant is pure, so dropping is safe.

### Conservative bail

- Matched case's consequent doesn't end with a case-terminator
  (Break / Return / Throw). Without one, control would
  fall through to the next case, and we don't model
  fall-through here. Future slice could concatenate
  consequents through the next terminator.
- Discriminant is `NaN`. Per §IsStrictlyEqual, `NaN !== NaN`,
  so NaN matches nothing — but rather than emit subtle no-
  match semantics on a surprising literal, `strict_equal_leaves`
  bails to `false` for NaN comparisons. The
  `pick_matching_case` walk returns `None`, default is used if
  present, otherwise `EmptyStatement` runs. Same observable
  behaviour as spec-correct NaN handling, just routed through
  the "no match" path.
- Anything non-leaf (Identifier discriminant or test, member
  access, call, binary, etc.) — bail.

### Cross-type strictness

`strict_equal_leaves` enforces strict-equality:
`NumericLiteral(1)` !== `StringLiteral("1")` even though they
`==` in JS. So `switch(1){case "1": ...; default: ...;}`
correctly skips the string case and uses default.

### New helpers

- `fn strict_equal_leaves(a: &Expression, b: &Expression) -> bool`
  — six match-arms for the literal subset that `is_pure_leaf`
  recognises plus the NaN guard.
- `fn pick_matching_case(disc, &[SwitchCase]) -> Option<&SwitchCase>`
  — walks cases looking for a strict-equal match, falls back
  to default.
- `fn strip_trailing_break(consequent) -> Vec<Statement>` —
  removes a trailing `BreakStatement`; leaves Return/Throw.

### Tests (11 new, 27 → 38 inline)

| Test | Pins |
|------|------|
| `switch_with_literal_disc_matching_case_collapses` | `switch(1){case 1:a;break;}` → `a;` |
| `switch_with_literal_disc_no_match_uses_default` | `switch(1){case 2:...; default:b;break;}` → `b;` |
| `switch_with_literal_disc_no_match_no_default_drops` | `switch(1){case 2:a;break;}` → `;` |
| `switch_collapse_with_return_terminator_preserves_return` | `switch(1){case 1:a;return;}` → `a; return;` (return stays) |
| `switch_collapse_string_discriminant` | `switch("b"){case "a":...; case "b":bx;break;}` → `bx;` |
| `switch_with_literal_disc_no_terminator_keeps_switch` | Conservative: no trailing break → keep switch |
| `switch_collapse_cross_type_test_does_not_match` | `switch(1){case "1":...; default:b;}` → `b;` (strict !==) |
| `switch_collapse_identifier_discriminant_keeps_switch` | Conservative: Identifier discriminant → keep |
| `switch_collapse_nan_discriminant_keeps_switch` | NaN routes through "no match" → empty (correct per spec) |
| `switch_collapse_empty_matched_case_keeps_switch_due_to_fallthrough` | `case 1: case 2: body; break;` with disc 1 → bail (fall-through) |
| `switch_collapse_preserves_trailing_labeled_break` | `case 1: a; break outer;` → labeled break stays |

closurec's e2e suites stay green — no existing fixture exercises
the shape, so behaviour is purely additive.

### Composition with steps 2 and 3

- Step 2 (empty-switch elimination) still catches the all-cases-
  empty branch with the same conditions; it runs first.
- Step 3 (drop-after-break) truncates case consequents BEFORE
  step 4 looks at them. So `case 1: body; break; dead;` is
  step-3'd to `case 1: body; break;` and then step-4'd to
  `body;`. The composition is clean: each step strictly
  reduces the AST size on success.
- The composition produces no surprises. **A matched case with
  an empty consequent does NOT collapse** — that case shape
  (`case 1: case 2: body; break;`) is the standard "share a
  case body" pattern, where execution falls through from the
  empty case to the next one. Step 4 conservatively bails on
  empty matched consequents to preserve fall-through behaviour.

### Security-review fixes folded in

The initial implementation had two correctness bugs the
security-review subagent caught before push:

1. **`strip_trailing_break` stripped labeled breaks too.**
   `case 1: a; break outer;` → original code dropped
   `break outer;` and silently kept an enclosing labeled
   loop running. Fixed: only strip when `b.label.is_none()`.
   Test `switch_collapse_preserves_trailing_labeled_break`
   pins this.
2. **Empty matched consequent collapsed to `{}`, dropping
   fall-through body.** The classic `case 1: case 2: body;
   break;` with discriminant `1` had `case 1` with empty
   consequent — the original `terminates || empty` condition
   collapsed it to `{}` and dropped `body`. Fixed: require
   `terminates && !empty` (no fall-through collapse). Test
   `switch_collapse_empty_matched_case_keeps_switch_due_to_fallthrough`
   pins this.

### Version bump

`0.7.0` → `0.8.0`.

## [0.7.0] - 2026-06-04

### Added — CLOC12.35: drop-after-break in case consequents (gap-014 step 3/N)

Second peephole on top of the SwitchStatement AST. Inside a
`SwitchCase.consequent` list, after the recursive walk, find the
first **case-terminator** (`BreakStatement`, `ReturnStatement`,
`ThrowStatement`) and truncate everything after it. Per-case,
independent across cases.

### Helper

`fn is_case_terminator(stmt: &Statement) -> bool` — three
match-arms: `BreakStatement`, `ReturnStatement`, `ThrowStatement`.
Documented inline.

### Why distinct from `is_terminator`

The existing `is_terminator` only matches `ReturnStatement`
because it's used by the block walker (`dce_block_statement`),
and at function-body block level a bare `break;` is a
SyntaxError. Broadening `is_terminator` would silently
mishandle that. Case consequents are the one statement
context where bare `break` is both legal and terminating.

`ContinueStatement` is intentionally NOT a case terminator —
it refers to an enclosing loop (`switch` is not a loop), not
the switch itself. Whether the enclosing loop body continues
or terminates depends on outer-context analysis we don't do
here, so we bail conservatively.

### Tests (7 new, 20 → 27 inline)

| Test | Pins |
|------|------|
| `drop_after_break_in_case_consequent` | `case 1: a; break; dead;` → drop `dead;` |
| `drop_after_return_in_case_consequent` | `case 1: return; dead;` → drop |
| `drop_after_throw_in_case_consequent` | `case 1: throw 1; dead;` → drop |
| `drop_after_break_in_default_consequent` | `default: a; break; dead;` → drop |
| `drop_after_break_applies_per_case` | Two cases, only the one with `break;dead;` truncates |
| `continue_in_case_consequent_keeps_following_statements` | `case 1: continue; y;` → unchanged (conservative) |
| `case_with_no_terminator_unchanged` | `case 1: a; b;` → unchanged |

closurec's e2e suites stay green — no existing fixture exercises
the shape, so behaviour is purely additive.

### Composition with step 2 (empty-switch elimination)

Step 2 (CLOC12.34) drops `switch(<pure>){}` when every
consequent is empty. Step 3 truncates consequents after a
terminator. The two compose naturally: a case body that's
ENTIRELY dead after a terminator at index 0 (e.g.
`case 1: break; …;` → `case 1: break;` is the limit of
step 3 — the `break;` itself stays because it's the
terminator, not after it). Further collapse to actually-
empty would require recognising that a `break;` alone in a
case body is equivalent to no body (since fallthrough
through an empty case is the same), which is a future
slice.

### Version bump

`0.6.0` → `0.7.0`.

## [0.6.0] - 2026-06-04

### Added — CLOC12.34: empty-switch elimination (gap-014 step 2/N)

First peephole on top of the SwitchStatement AST that CLOC12.33
landed. Drops `switch(<x>){}` (and equivalent shapes) entirely
when **all** of the following hold:

1. Every case's `consequent` is empty (or no cases at all).
2. The discriminant is a leaf literal — one of
   `NumericLiteral` / `StringLiteral` / `BooleanLiteral` /
   `NullLiteral` / `UndefinedLiteral` / `BigIntLiteral`.
3. Every case's `test` is either `None` (the `default:` clause)
   or also a leaf literal.

When the switch matches, the pass rewrites it to
`EmptyStatement`. The block walker drops that on its next sweep,
so the whole switch disappears from the output. One
`Contribution { tag: "switch_eliminated", before: ..., after:
"EmptyStatement" }` lands per elimination, tagged against the
switch's own `cv`.

### Why the conservative-bail design

The rule deliberately treats `Identifier` as not-pure. Reading
an `Identifier` can throw under TDZ for an uninitialised `let` /
`const`. Without scope analysis (which is `closure-scope-analyzer`'s
territory), we can't prove the read is safe to drop, so we leave
the switch intact. The same reasoning applies to member access,
calls, binary/unary, etc.

This means we DO eliminate `switch(1){}` / `switch("k"){}` /
`switch(true){}` / `switch(null){}` / `switch(void 0){}` /
`switch(2n){}` and the same shapes with all-empty case clauses,
but NOT `switch(x){}` or `switch(a.b){}` — those keep the switch.
A future "switch-with-pure-effect-analysis-cleared-discriminant"
slice can replace this when the broader effect-analysis pass
lands.

### Helper

`is_pure_leaf(expr: &Expression) -> bool` — private helper used
only by the switch rule. Six match-arms over the literal types
listed above. Documented inline.

### Tests (6 new inline, 14 → 20 total)

| Test | Pins |
|------|------|
| `empty_switch_with_literal_discriminant_drops_entirely` | `switch(1){}` → `;` (then dropped) |
| `empty_switch_with_pure_cases_drops_entirely` | `switch(1){case 2:;default:;}` → drop |
| `empty_switch_with_identifier_discriminant_keeps_switch` | `switch(x){}` → unchanged (TDZ conservative) |
| `switch_with_non_empty_consequent_keeps_switch` | `switch(1){case 1:y;}` → unchanged |
| `empty_switch_with_identifier_case_test_keeps_switch` | `switch(1){case k:;}` → unchanged |
| `empty_switch_with_boolean_discriminant_drops` | `switch(true){}` → drop |

closurec's `diff_minify` and other e2e suites stay green — no
existing fixture exercises empty-switch shapes, so behaviour is
purely additive.

### Version bump

`0.5.2` → `0.6.0` (new behaviour; not a strict semver minor
since the public `DcePass` impl is unchanged, but the
`Contribution` stream gains a new `tag`).

## [0.5.2] - 2026-06-01

### Changed — CLOC12.16: handle new `UndefinedLiteral` Expression variant

The DCE pass gained an `Expression::UndefinedLiteral` arm in its
expression-walk leaf list so it compiles against the new
`javascript-ast 0.6.0` AST. Behaviour: passthrough — undefined
literals are leaves with no children to recurse into.

## [0.5.1] - 2026-06-01

### Changed — CLOC12.15 rebase: handle new `BigIntLiteral` Expression variant

The DCE pass gained an `Expression::BigIntLiteral` arm in its
expression-walk leaf list so it compiles against the new
`javascript-ast 0.5.0` AST. Behaviour: passthrough — bigint
literals are leaves with no children to recurse into.

Bumped to 0.5.1 (rather than 0.4.3 originally planned) because this
PR was rebased on top of CLOC12.19 (0.5.0, already on main).

## [0.5.0] - 2026-06-01

### Added — CLOC12.19: block flattening (closes gap-010)

Adds a flatten step to `dce_block_statement` that splices any
direct-child `BlockStatement`'s body into the enclosing block —
after the recurse-into-children pass and before the existing
dead-after-terminator + empty-statement sweeps.

Truth table:

| Input                       | Output                  |
|-----------------------------|-------------------------|
| `{{foo();}}`                | `{foo();}`              |
| `{foo();{}}`                | `{foo();}`              |
| `{{};foo();}`               | `{foo();}`              |
| `{{a();b();};}`             | `{a();b();}`            |
| `{foo();{bar();baz();}}`    | `{foo();bar();baz();}`  |
| `{let x=1;{let x=2;}}`      | unchanged (scope-safe)  |
| `{var x=1;{var y=2;}}`      | `{var x=1;var y=2;}`    |

**Scope safety.** ECMAScript block scope means `let`, `const`,
`class`, and inner `function` declarations are bound to their
enclosing block. Hoisting them into the outer block would either
leak the binding to a wider scope or trigger a redeclaration TDZ
error against a same-named outer binding. The new helper
`block_is_scope_safe_to_flatten(b) -> bool` walks an inner
block's body and returns `false` if any statement is a block-bound
declaration — those inner blocks stay put. Plain `var` is fine
because `var` is function-scoped, not block-scoped, so hoisting
`{var x = 1;}` upward produces the same effective binding.

The flatten contributes a `block-flattened` record per outer
block where any splicing happened (not per child — that'd be
noisy). Dead-after-return and EmptyStatement drops still cascade
afterward, so input like `{x;{return;y;};z;}` collapses to
`{x;return;}` in a single pass.

Un-ignores upstream-port `test_fold_block_flattening` with four
assertions covering the `{{foo();}}`, `{foo();{}}`, `{{};foo();}`,
and `{foo();{bar();}}` cases. Updates the existing
`recurses_into_nested_blocks` inline test (renamed to
`recurses_into_nested_blocks_and_flattens`) to reflect the new
cascade behaviour where the inner block is now spliced into the
outer rather than preserved.

## [0.4.2] - 2026-06-01

### Changed — CLOC12.14: handle new `ThrowStatement` variant

The DCE pass gained a `TaggedStatement::ThrowStatement` match arm
so it compiles against the new `javascript-ast 0.4.0` AST.
Behaviour: recurse into the argument expression. `throw` is a
definite terminator — the dead-after-throw collapse inside
`BlockStatement` (analogous to dead-after-return) is a follow-up
gap; this PR only adds the structural walk.

## [0.4.1] - 2026-06-01

### Changed — CLOC12.13: handle new `LabeledStatement` variant + un-ignore the upstream test

The DCE pass gained a `TaggedStatement::LabeledStatement` match arm
so it compiles against the new `javascript-ast 0.3.0` AST.
Behaviour: recurse into the labelled body (so dead-after-return
inside `a: { ...return... ...dead... }` still gets stripped),
preserve the label verbatim. The collapse-to-empty optimisation
for `a: break a;` is a separate gap.

The previously-ignored upstream test
`test_remove_no_op_labelled_statement` is now un-ignored — it
builds the `a: break a;` AST by hand and asserts DCE leaves it
alone (the *current* behaviour). When the collapse optimisation
lands, the assertion flips from `assert_dce_same` →
`assert_dce_yields(..., vec![])`. The upstream-test passthrough
table in this CHANGELOG that previously listed
`test_remove_no_op_labelled_statement | gap-009` should now read
`gap-009 (AST modelled; collapse follow-up)` — see
`code/specs/CLOC12-gaps.md`.

## [0.4.0] - 2026-05-31

### Changed — CLOC12.06: gap-011 marked RESOLVED via cross-crate routing

Bookkeeping PR. The `test_if_with_constant_test_collapse` stub in
`tests/upstream/peephole_remove_dead_code_test.rs` was previously
`#[ignore]`-ed with `gap-011`, on the rationale that upstream
`PeepholeRemoveDeadCodeTest::testIf` lines like `if (1){…}` →
consequent really belong in `closure-pass-fold-control-flow`'s
territory in our setup.

CLOC12.05 (PR #4672) ported the matching upstream behaviour into
`closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs`
as `test_if_true_folds_to_consequent`, `test_if_false_folds_to_alternate`,
`test_if_null_folds_to_alternate`, etc. — all passing. The
behaviour is covered in the right crate.

This PR closes the loop:

- The DCE-side stub no longer carries `#[ignore]`. It now passes as
  a marker test whose body documents where the actual behavioural
  coverage lives. This keeps the cross-crate audit trail explicit
  for future readers searching upstream's `testIf` method name.
- `code/specs/CLOC12-gaps.md` marks gap-011 `RESOLVED in CLOC12.06`
  with the full list of fold-control-flow ports that cover the
  behaviour.

### Port score (this crate)

|             | passing | ignored |
|-------------|---------|---------|
| CLOC12.04   | 5       | 7       |
| **CLOC12.06** | **6** | **6**   |

### Version

`0.3.0` → `0.4.0`.

## [0.3.0] - 2026-05-31

### Added — CLOC12.04: port subset of upstream `PeepholeRemoveDeadCodeTest`

Second port under the CLOC12 byte-identical contract. Establishes the
`tests/upstream/` layout for `closure-pass-dce`, mirroring the
`closure-pass-constant-fold` layout from CLOC12.02.

- `tests/upstream/UPSTREAM_SHA` — pins
  `google/closure-compiler@5bb35ec1245dc1d3557481e5f8b4db344bcd1e6b`.
- `tests/upstream/ATTRIBUTION.md` — Apache-2.0 attribution per
  CLOC12.01 §5.
- `tests/upstream/peephole_remove_dead_code_test.rs` — 12 ported test
  methods.

### Test breakdown

|     | passing | ignored |
|-----|---------|---------|
| CLOC12.04 | **5** | **7** |

**Passing (5):** test the two narrow categories DCE actually
implements today:

- `test_function_with_bare_return_is_unchanged` — `foldSame(\"function f(){return;}\")`.
- `test_function_with_assignment_then_return_is_unchanged` — equivalent of
  `foldSame(\"function f(){x=3;return;}\")`, used to verify that
  not-unconditionally-terminating sequences stay put.
- `test_dead_statement_after_return_with_argument_is_dropped` —
  `function f(){return 3;foo();}` collapses to `function f(){return 3;}`.
- `test_multiple_dead_statements_after_return_are_dropped` —
  multi-statement tail after a bare return all get dropped.
- `test_empty_statements_dropped_from_function_body` — equivalent of
  `fold(\"{x=3;;;y=2;;;}\", \"x=3;y=2\")` applied at the function-body block.

**Ignored (7):** record upstream's broader scope as `gap-NNN`
entries that we expect to address in other passes or future AST work:

| Test | Gap | What's needed |
|------|-----|---------------|
| `test_remove_no_op_labelled_statement` | gap-009 | `LabeledStatement` / `BreakStatement` AST nodes |
| `test_fold_block_flattening` | gap-010 | nested-block flattening (single-child collapse) |
| `test_if_with_constant_test_collapse` | gap-011 | belongs in `fold-control-flow`, ported there |
| `test_hook_cleanup` | gap-012 | belongs in `constant-fold`, ported there |
| `test_fold_useless_loop_body` | gap-013 | belongs in `fold-control-flow` |
| `test_optimize_switch` | gap-014 | `SwitchStatement` AST not in Phase 1 |
| `test_var_lifting` | gap-015 | belongs in `remove-unused-vars` / hoisting pass |

### Why most tests are ignored

Upstream `PeepholeRemoveDeadCode` is much broader than our DCE pass.
It collapses dead `if` branches, simplifies useless loops, optimizes
switches, removes useless labelled statements, normalises `let`/
`const`/`var` lifting, etc. In our setup those responsibilities
mostly live in other pass crates:

- Dead-branch / loop-body simplification ⇒ `closure-pass-fold-control-flow`
- ConditionalExpression cleanup ⇒ `closure-pass-constant-fold`
- Unused variable / scope-based pruning ⇒ `closure-pass-remove-unused-vars`

So these ports stay marked `#[ignore]` in *this* file and will be
re-ported into the matching pass crates' `tests/upstream/` directories
in future CLOC12 slices. The gaps make the cross-crate routing
explicit.

### Version bump

`0.2.0` → `0.3.0`.

## [0.2.0] - 2026-05-24

### Added — real `Pass::run` body (final step of the autonomous chain)

Replaces v0.1.0's identity body with a recursive walker over `Program → ProgramItem → Statement → Expression`. Final step in the autonomous-chain real-body rollout (after constant-fold, fold-control-flow, and the closure-emitter).

Two cleanup categories per `BlockStatement.body`:

- **Dead-after-terminator**: drop everything after a `ReturnStatement`. Phase 1 doesn't have `ThrowStatement` yet; `BreakStatement` / `ContinueStatement` only qualify in their enclosing loop scope (Phase 2 work).
- **Empty-statement removal**: drop `EmptyStatement` nodes entirely. They're semantically a no-op (`;`) and clutter output.

Recurses through every Phase 1 node so nested blocks (function bodies, if-bodies, while-bodies, for-bodies) get cleaned too. Records one `Contribution` per drop *category* per block (not per-statement — that'd be too noisy).

### Why this overlaps with fold-control-flow's dead-after-return

Intentional overlap:

- `fold-control-flow` does the cleanup as part of its block rewrite when it observes the terminator while folding.
- DCE runs **after** fold-control-flow per CLOC06 canonical order, and catches:
  - Cases where fold-control-flow didn't enter the block (e.g. it was busy folding the surrounding `if`'s test);
  - `EmptyStatement` nodes that fold-control-flow *produced* when it collapsed `if (false) { … }` with no alternate;
  - Future cases where a Phase 2+ pass leaves dead code behind.

### CV tracing — both modes per CLOC09 amendment

- **Traced** (`cv: Some` on the block): `Contribution { source: "dce", tag: "removed-dead-code" | "removed-empty-statement", meta: {before, after, parent_cv} }` appended per category that triggered.
- **Untraced** (`cv: None`): drops silently, no contributions emitted. `changed: true` still set.

### Tests

14 tests (up from 8 in v0.1.0):
- pass metadata unchanged
- empty program identity
- `{x; return; y; z;}` → `{x; return;}` (drop y and z)
- `{x; y;}` (no return) unchanged
- `{x; ; y; ; ;}` → `{x; y;}` (drop empties)
- Both categories in one block: `{x; ; return; y; ;}` → `{x; return;}` with two contributions
- Nested blocks: outer kept, inner's dead-after-return cleaned
- Untraced mode drops silently
- Pipeline solo
- **Full canonical pipeline**: `constant-fold + fold-control-flow + dce` on `if (1 < 2) {z;}` → `z;` (the whole chain cooperates correctly)
- **End-to-end through the chain**: `function f() { if (false) {x;} return; y; }` → `function f() { return; }` after fold + fold-control-flow (drops if-false-branch and creates EmptyStatement, drops y after return) + dce (removes the leftover EmptyStatement)

### Dependencies
- Added `coding-adventures-closure-pass-fold-control-flow` as a dev-dep for the full-canonical-pipeline test.

### Skipped (Phase 1.x / 2+)
- Unreferenced `VariableDeclaration` removal — `closure-pass-remove-unused-vars`'s job.
- Empty `BlockStatement` collapse to `EmptyStatement` — preserves debugging-step shape for now.
- Phase 2: `ThrowStatement` as terminator, `BreakStatement` / `ContinueStatement` qualifying in their loop scope.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 canonical pass set — second concrete optimization pass after constant-fold.
- `DcePass` zero-sized type implementing `Pass`:
  - `name = "dce"`
  - `depends_on = &["constant-fold"]` — folds expose dead arms so they run first per CLOC06 canonical order. `fold-control-flow` will join this list once it exists.
  - `iteration_policy = IterationPolicy::FixedPoint` — deletion can free further nodes.
  - `cost = 3` pass-units (tree walk + reachability marking + post-walk deletion).
  - `invalidates()` empty in v1 (informational only per CLOC06 Open Question 1).
- `DcePass::new()` zero-arg constructor.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there's nothing to delete. Pass through unchanged, `changed = false`, `nodes_touched = 1`, no contributions emitted (per CLOC03 §"When a pass keeps a node unchanged").
- 8 tests covering: `name()` value, `iteration_policy` is FixedPoint, `cost` is 3, `depends_on == ["constant-fold"]`, `invalidates` empty, run on empty Program is identity, **pipeline correctly orders constant-fold before dce** even when DCE is registered first (this is the key value-add of the depends_on edge), DCE runs as a solo pass with unknown deps silently dropped per v0.1.0 scheduler, `Default` + `Clone` impls.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline`, `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar` (future `pure` / `no_side_effects` attributes inform deletion safety), `coding_adventures_correlation_vector` (`cv.delete()` + `"deleted"` `Contribution` per CLOC03), `serde_json`. Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`, `coding-adventures-closure-pass-constant-fold` for the ordering integration test.
- v1 is scaffolding. The full reachability walk + deletion lands once `javascript-ast` grows the needed variants. When that happens, the `Pass::run` body changes but the public surface stays put — no churn upstream.
