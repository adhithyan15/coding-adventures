# coding-adventures-closure-pass-dce

Dead-code elimination pass for the Closure Compiler clone. Walks the
program from entry / exported declarations, marks reachable nodes,
and deletes the rest. Per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
canonical pass set.

## Implemented cleanups

(The "v1 identity" notes below predate the real implementation — see the
module-level docs in `src/lib.rs` for the authoritative description.)

- **Dead-after-terminator**: in any block body, drop every statement after an
  unconditional terminator — a `return` **or a `throw`** (both end the
  statement list's execution in every block context). `break` / `continue` are
  handled only inside switch-case consequents (they terminate flow only
  relative to an enclosing loop/switch). **Hoisting guard:** the tail is dropped
  only when *every* statement in it is provably free of a hoisted binding
  (expression / empty / break / continue / return / throw / `let` / `const`).
  A `var` or `function` declaration — or any compound statement that could wrap
  a hoisted `var` (`if (c) var y;`, `for (var i …)`, blocks, switch) — preserves
  the tail, since those bindings hoist to function-top and remain observable
  before the terminator (a truly-unused one is still removed by
  `remove-unused-vars`).
- **Empty-statement removal**: drop `;` no-op nodes from block bodies.
- **Constant-discriminant switch collapse** and per-case dead-after-`break`
  dropping (gap-014).

## What's here (v1)

- `DcePass` implementing the `Pass` trait from
  [`closure-pass-pipeline`](../closure-pass-pipeline).
- Metadata pinned:
  - `name = "dce"`
  - `depends_on = ["constant-fold"]` — folds expose dead arms, so
    they run first per CLOC06 canonical order. Once
    `fold-control-flow` exists it joins this list.
  - `iteration_policy = FixedPoint` — deletion can free further
    nodes
  - `cost = 3`
- `Pass::run` is **identity** in v1: `javascript-ast` ships only
  `Program` / `SourceType` today, so there's nothing to delete.
  Real reachability walk + deletion lands once the AST grows
  `Statement` / `Expression` / declaration variants.

## What this PR locks down even though it's identity

1. The `depends_on("constant-fold")` edge is now in the scheduler's
   graph — so the moment both passes are in one pipeline, the
   correct order is enforced (test
   `pipeline_orders_constant_fold_before_dce` verifies this).
2. Pass metadata (name, policy, cost) is what the future `closurec`
   CLI reads to surface enable/disable flags.
3. The contribution-emission path is wired up for CLOC03's
   `"deleted"` tag, and every real removal now also **tombstones**
   the removed node's own CV entry via `cv.delete(...)` (see
   "Deletion provenance" below).

## Deletion provenance (correlation vector)

When DCE removes a node it does two things in the correlation-vector
log: it pushes a coarse summary `Contribution` against the enclosing
container (for stats/history), **and** it marks the removed node's own
CV entry with a `DeletionRecord` via `cv.delete(cv_id, "dce", reason,
meta)`. That second step is what makes the removal auditable: a
`--correlation_vector` consumer asking "what happened to this span?"
gets a definite answer — *dce removed it, because `<reason>`* — instead
of the span silently vanishing from the provenance graph.

Reason tags, one per removal site:

- `removed-dead-code` — after a block-level terminator;
- `removed-dead-code-in-case` — after a `switch`-case terminator;
- `removed-empty-statement` — a swept `;`;
- `removed-debugger` — a stripped `debugger;` (block body or top level).

`block-flattened` is intentionally NOT a deletion: flattening *moves* a
nested block's statements up one scope, so those nodes stay live in the
log. `delete` is a no-op when the log is disabled (the production
default), so this costs nothing off the `--correlation_vector` path.

## What's coming

Once the AST grows the needed variants:

- Reachability walk from `export` / entry declarations using sidecar
  `pure` / `no_side_effects` attributes to find dead expression
  statements.
- Deletion of unreferenced variable bindings whose initializers are
  pure (`closure-pass-remove-unused-vars` is a specialization of
  this).
- Extending `cv.delete()` provenance to the branch-elimination sites
  (constant-discriminant `switch` collapse, dead ternary arms), which
  currently emit only a summary contribution.

## Dependency whitelist

- `coding-adventures-closure-pass-pipeline` — `Pass` trait + types.
- `coding-adventures-javascript-ast` — `Program` input/output.
- `coding-adventures-type-sidecar` — `pure` / `no_side_effects`
  attributes inform deletion safety.
- `coding_adventures_correlation_vector` — `cv.delete()` +
  `"deleted"` `Contribution` per CLOC03.
- `serde_json` — `Contribution.meta` JSON values.

Dev-deps:
- `coding-adventures-javascript-tokens` for `EsVersion` in tests.
- `coding-adventures-closure-pass-constant-fold` for the integration
  test that verifies fold→DCE ordering.
