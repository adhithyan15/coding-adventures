# coding-adventures-closure-pass-fold-control-flow

Control-flow folding pass for the Closure Compiler clone. Sits
between `constant-fold` and `dce` per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
canonical pass set.

## What it does (once the AST grows the needed variants)

Eliminates statically-decidable control flow:

```text
if (false) { A } else { B }          →  B
if (true)  { A } else { B }          →  A
while (false) { ... }                →  (deleted)
function f() { return 1; A; B; }     →  function f() { return 1; }
switch (1) { case 1: A; break; ... } →  A
```

These rewrites typically open new opportunities for DCE — that's
why CLOC06 pins the order **constant-fold → fold-control-flow →
dce**.

## What's here (v1)

- `FoldControlFlowPass` implementing the `Pass` trait from
  [`closure-pass-pipeline`](../closure-pass-pipeline).
- Metadata pinned:
  - `name = "fold-control-flow"`
  - `depends_on = ["constant-fold"]` — folds turn `if (1+1===2)`
    into `if (true)`, exposing the dead arm.
  - `iteration_policy = FixedPoint` — eliminating one branch can
    expose another.
  - `cost = 2` (matches constant-fold; single tree walk).
- `Pass::run` is **identity** in v1: `javascript-ast` ships only
  `Program` / `SourceType` today, so there's nothing to fold. The
  real folding logic slots into `Pass::run` once the AST grows
  `IfStatement`, `WhileStatement`, `SwitchStatement`, and
  `ConditionalExpression`.

## What this PR locks down even though it's identity

1. The `depends_on("constant-fold")` edge is now in the
   scheduler's graph — the moment both passes are in one pipeline,
   constant-fold is forced first.
2. The three-pass integration test
   (`pipeline_orders_three_passes_canonically`) registers DCE
   first, fold-control-flow second, constant-fold third, and
   verifies the scheduler produces the canonical order.
3. Pass metadata is what the future `closurec` CLI reads to
   surface `--disable=fold-control-flow` flags.

## Why this isn't merged into constant-fold

- **Different node-kind focus.** Constant-fold works on
  `Expression`. Fold-control-flow works on `Statement` (and the
  ternary `?:` expression).
- **Different reasoning step.** Constant-fold asks "does
  evaluating this have side effects?". Fold-control-flow asks "is
  this branch reachable given what we statically know?". Once the
  AST grows nodes these are independent analyses.
- **Different CLI ergonomics.** `--disable=fold-control-flow` is
  a meaningful debug knob distinct from `--disable=constant-fold`.

## Followup PR

The `dce` crate currently has `depends_on = &["constant-fold"]`.
A one-line followup PR will tighten that to
`depends_on = &["constant-fold", "fold-control-flow"]` so the
canonical order survives even when DCE is registered without
constant-fold. Splitting it keeps each change reviewable in
isolation per the small-PR principle.

## Deletion provenance (correlation vector)

When the pass **eliminates** a branch under a constant condition it
tombstones that branch's own CV entry via `cv.delete(cv_id,
"fold-control-flow", "folded-branch", meta)`, alongside the summary
`Contribution` it records against the enclosing node. So a
`--correlation_vector` consumer asking "what happened to the code in
this branch?" gets a definite answer instead of the branch silently
vanishing from the provenance graph:

- `if (true)  A else B` → `A` — the `else` branch **B** is tombstoned;
- `if (false) A else B` → `B` — the `then` branch **A** is tombstoned;
- `while (false) BODY`  → `;` — **BODY** is tombstoned.
- `{ return; dead(); }` → `{ return; }` — the unreachable
  dead-after-terminator statements are tombstoned (`removed-dead-code`),
  matching what DCE records for the same drop.

Rewrites that *preserve* both branches (`if→ternary`, `if→&&`, De Morgan
swaps) do NOT tombstone — the content is restructured, not removed.
`delete` is a no-op when the log is disabled (the production default),
so this costs nothing off the `--correlation_vector` path. (Remaining
follow-up: the constant-condition ternary collapse, whose discarded arm
is an `Expression` and needs an `expression_cv` helper.)

## Dependency whitelist

- `coding-adventures-closure-pass-pipeline` — `Pass` trait +
  types.
- `coding-adventures-javascript-ast` — `Program` input/output.
- `coding-adventures-type-sidecar` — pass receives sidecar
  reference for future side-effect analysis.
- `coding_adventures_correlation_vector` — receives mutable
  `CVLog`; folded branches are tombstoned via `cv.delete()`.
- `serde_json` — `Contribution.meta` JSON values.

Dev-deps:
- `coding-adventures-javascript-tokens` for `EsVersion` in tests.
- `coding-adventures-closure-pass-constant-fold` for the two-pass
  ordering integration test.
- `coding-adventures-closure-pass-dce` for the three-pass
  ordering integration test.
