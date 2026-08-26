# SIR30 — Switch statement

## Status

Defines `Stmt::Switch`/`SwitchCase` and `Feature::Switch` — a
C-family-style switch statement with real fall-through semantics
(task #51). SIR previously had no IR node for `switch`/`case` at all
(confirmed by a repo-wide grep, not assumed) — a gap discovered during
the Java frontend's own M2a milestone and tracked as its own standing
backlog item ever since, referenced repeatedly across
[JV02](JV02-java-to-semantic-ir.md)'s own milestone sections.

This slice lands the IR node, `Feature::Switch`, and full validator
enforcement — mirrors [SIR16](SIR16-ir-extensions-for-python-and-javascript.md)'s
own "Loop control (addendum)" rollout shape exactly: **no backend
accepts the feature yet**. Every backend crate gets a mechanical,
compile-forced rejection arm (the same "compile-exhaustiveness fix
only, not real support" panic convention SIR29's own nodes shipped
with). Real codegen is deferred to per-backend follow-up tasks, and no
frontend emits the node yet either — four frontends currently reject
`switch`/`case` source syntax outright with a "no SIR IR node" message
(`java-to-semantic-ir`, `javascript-to-semantic-ir`, `matlab-to-semantic-ir`,
`idl-to-semantic-ir`); this slice makes wiring each of them a
self-contained follow-up task rather than a blocked one.

## Motivation

Four frontends in this repo already parse `switch`/`case`-shaped source
constructs (Java, JavaScript, MATLAB's `switch`/`case`/`otherwise`, IDL's
`CASE`/`SWITCH`) and reject them purely because no IR primitive exists to
lower them to — not because of any deeper design obstacle. Landing the
primitive once, generally enough to cover every one of those four
languages' own real semantics, avoids each frontend inventing its own
incompatible answer later.

The two language families differ in one significant way: Java/JavaScript
have real **fall-through** (a `case` without an explicit `break` runs
into the next case's own body); MATLAB/IDL's `CASE`/`SWITCH` does
**not** (each case implicitly stops after its own body, no `break`
needed or meaningful). This spec's own IR primitive models the *harder*
semantic — true fall-through — since a frontend targeting a
non-fall-through source language can always emit an explicit
`Stmt::Break` at the end of each case body to simulate that behavior;
going the other direction (an IR with no fall-through concept, asked to
represent Java's very-much-real fall-through) would require either
duplicating case bodies' side-effecting code or inventing a goto-style
primitive SIR does not otherwise have — a correctness trap, not a
narrowing.

## New SIR node kinds

```text
Stmt::Switch {
    discriminant: Box<Expr>,
    cases: Vec<SwitchCase>,
    default: Option<Vec<Stmt>>,
    span: Span,
}

struct SwitchCase {
    value: Expr,
    body: Vec<Stmt>,
    span: Span,
}
```

`discriminant` is evaluated once. `cases` are tried in order; the first
`SwitchCase` whose `value` equals `discriminant` (equality — the same
notion of equality every other SIR construct that compares values
already uses; SIR30 introduces no new equality primitive) begins
execution, at that case's own `body`. Execution then **falls through**
into the next case's own `body` (and eventually into `default`, if
present) unless a `Stmt::Break` is reached first — see "Loop-control
interaction" below. Zero cases matching and no `default` present is
well-defined: the switch does nothing, matching every C-family
language's own behavior.

`discriminant`'s own `Kind`/static type (in a statically-typed source
language) and whether each case `value` is even equality-comparable
against it are entirely a **frontend-level** concern. This validator
does not kind-check either side, mirroring `Stmt::While`'s own condition
expression — SIR v0 is not a type-checker (see this repo's own
`Kind`-vs-`SirType` split precedent in the Java frontend); each frontend
that already targets a construct like this decides for itself what's
well-typed before ever emitting IR (Java, for instance, restricts a
`switch` discriminant to `int`/`char`/`String`/an enum before lowering).

**Multiple case labels, one body** (`case 1: case 2: foo(); break;`,
real Java/JS syntax) needs no dedicated IR shape: an empty-bodied
`SwitchCase` naturally falls through into the next one, so
`case 1: case 2: foo(); break;` is simply two `SwitchCase` entries —
the first with an empty `body`, the second with `foo()` and a trailing
`Break`. This is why `SwitchCase` carries exactly one `value`, not a
`Vec<Expr>` — fall-through already gives multi-value matching for free,
with no separate mechanism to keep in sync.

**`default` is a dedicated field, not a member of `cases`**, and — by
this node's own construction — is always the *last* case for both
match-order and fallthrough-order purposes. Real Java/JavaScript permit
a source-level `default:` label in any position (still matched last
regardless of where it's written, but falling through at its own
textual position if execution reaches it that way) — modeling that
precisely would require exposing two independent orderings (match-order
vs. fallthrough-order) to every consumer of this node, for a corner case
essentially nobody hits in practice. A frontend whose source declares
`default` in a non-last position rejects it as a narrow, disclosed scope
boundary rather than mis-lowering it — and structurally *cannot* emit
a non-last `default` through this node even by accident, since the
field layout makes the "always last" property true by construction, not
by a runtime check anywhere.

## Loop-control interaction

`Stmt::Break` is valid **directly inside a `Switch`** the same way it's
valid inside a loop — it exits the switch. This required extending
[`Feature::LoopControl`](SIR16-ir-extensions-for-python-and-javascript.md#loop-control-addendum)'s
own validator machinery (`loop_stack`/`LoopKind`) rather than inventing
a parallel mechanism: whether `break` inside `switch { while (...) {
break; } }` targets the switch or the loop is fundamentally a *nesting
order* question — the same question `loop_stack` already answers for
loop-inside-loop nesting — so a new `LoopKind::Switch` variant, pushed/
popped around a `Switch`'s own body walk exactly like `While`/`ForEach`
push `LoopKind::Safe`, reuses that existing, already-correct nesting
logic for free.

`Stmt::Continue`, by contrast, **never targets a switch** in any
C-family language — `continue` inside `switch { ... }` nested in a
`while` continues the *loop*, skipping the switch entirely, even though
the switch is more deeply nested. The validator's own `Continue` check
reflects this asymmetry directly: it walks `loop_stack` from the
innermost entry, **skipping past any `LoopKind::Switch` frames**, to
find the nearest *actual* loop. `Break`'s own check, by contrast, only
ever looks at the top of the stack (any breakable context, loop or
switch, satisfies it).

Both statements are gated by `Feature::LoopControl` exactly as before —
`Feature::Switch` gates the `Stmt::Switch` node's own existence, not
whether `break`/`continue` may appear inside one (a module could in
principle use `Switch` with no `Break`/`Continue` inside it at all, and
would then only need `Feature::Switch`).

## New `Feature` variant

```text
Feature::Switch
```

Kept as its own feature (not folded into `LoopControl`, despite the
interaction above) for the identical reason `LoopControl` itself was
split from `Loops`: this is a brand-new node no backend has ever needed
to emit before, so a backend must explicitly opt in rather than
`Loops`/`LoopControl` silently growing a promise their existing
implementations don't keep. A backend accepting `Feature::Switch`
promises real fall-through codegen, correct `default` handling, and
honoring `Stmt::Break` as a valid switch-exit (not just a loop-exit).

## Validation updates

- A module using `Stmt::Switch` must declare `Feature::Switch`.
- `discriminant` is checked against the *outer* scope (as it stood
  immediately before the switch), matching every other statement's own
  "condition/subject evaluated before the body" rule.
- The **entire switch body — every case plus `default` — shares one
  flat local-env scope**, matching real `javac`'s own (well-known-
  gotcha) scoping rule: a local declared in an earlier case is
  lexically in scope for every later case, regardless of whether
  execution actually falls through to reach it. Getting this "wrong"
  (i.e., giving each case its own independent scope, which would feel
  more intuitive) would make a validator-accepted module disagree with
  real `javac`'s own scope error for the identical source shape, which
  matters if a frontend ever re-derives its own diagnostics from this
  same rule. The scope opens once, before the first case, and closes
  once, after `default` (or the last case, if there is no `default`).
- `Stmt::Break` inside a `Switch` is accepted (see "Loop-control
  interaction" above); `Stmt::Continue` inside a `Switch` with no
  actual enclosing loop is rejected with the same `"continue" outside
  a loop` message a bare `continue` outside any loop already gets —
  from the validator's perspective, a switch that isn't itself inside a
  loop provides no valid `continue` target at all, identical to no
  loop existing.
- Depth-guarded the same way every other recursive `Stmt` is (`Switch`
  itself, and each case/default body walk, thread `depth + 1`).

## Backend status

This slice lands the IR/validator surface only (mirrors SIR29's own
Slice 0, and this exact addendum's own SIR16 precedent for
`Feature::LoopControl`) — **no backend accepts `Feature::Switch` at
first.** Every backend crate (`semantic-ir-to-{javascript,typescript,
python,ruby,go,c,rust}`) gained a mechanical, compile-forced rejection
arm at every `Stmt`-exhaustive match site it has (`emit_stmt` itself,
plus each backend's own internal traversal helpers — ancestry
collection, builtin-usage scanning, assigned-locals collection, however
many of those a given backend happens to have) — the identical
"capability check should have rejected it" panic wording every SIR29
node's own initial landing already established as this repo's standing
convention for "structurally unreachable once the feature gate is
correctly wired, but Rust's exhaustiveness check still needs a match
arm regardless."

Two frontends (`python-to-semantic-ir`, `ruby-to-semantic-ir`) also have
their own internal `Stmt`-exhaustive traversal passes (unrelated to
codegen — e.g. call-normalization, bound-name collection) that needed
the same mechanical fix, following whichever no-op/panic convention that
specific file's own existing SIR29-node arms already used.

**Suggested adoption order** (informed by, but not identical to,
`Feature::LoopControl`'s own four-backend rollout): `semantic-ir-to-
javascript`/`semantic-ir-to-typescript` first — both already have a
native `switch`/`case`/`default`/`break` construct with fall-through
semantics matching this node's own model almost exactly, making them
the lowest-risk first adopters (unlike `LoopControl`'s own rollout,
where JS/TS still had to solve the `Expr::If`-in-statement-position
hazard — a real risk here too, worth auditing for independently, since
nothing about this addendum's own design removes it). `semantic-ir-to-
go`/`semantic-ir-to-c`/`semantic-ir-to-rust` next — none of the three
has a fall-through switch natively (Go's `switch` doesn't fall through
by default; Rust's `match` doesn't either), so each needs a real
lowering strategy (e.g. a `loop { match ... }` wrapper with explicit
fallthrough via shared block labels, or literal case-body concatenation
guarded by a synthetic "have we started falling through yet" flag —
design work each backend's own adoption task should scope for itself,
not assumed here). `semantic-ir-to-ruby` last: Ruby's `case`/`when` has
no fall-through concept at all and no `break`-equivalent inside it,
making it the largest semantic gap to bridge of any backend surveyed.

**Frontend adoption**: four frontends can immediately start wiring their
own already-parsed `switch`/`case` source syntax to this node once their
own turn comes — `java-to-semantic-ir` (task #51's own original
motivation), `javascript-to-semantic-ir`, `matlab-to-semantic-ir`,
`idl-to-semantic-ir`. Each is its own follow-up task; none is blocked
on any other, and none is blocked on backend adoption either — a
frontend can emit `Stmt::Switch` into a `Module` that simply can't yet
be compiled by any backend, the same "IR ahead of both ends" state
`Feature::LoopControl` itself passed through between its own IR-landing
task and JS's first-adopter task.
