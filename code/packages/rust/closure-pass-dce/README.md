# coding-adventures-closure-pass-dce

Dead-code elimination pass for the Closure Compiler clone. Walks the
program from entry / exported declarations, marks reachable nodes,
and deletes the rest. Per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
canonical pass set.

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
   `"deleted"` tag — when real deletion lands, the integration is
   in place.

## What's coming

Once the AST grows the needed variants:

- Reachability walk from `export` / entry declarations using sidecar
  `pure` / `no_side_effects` attributes to find dead expression
  statements.
- Deletion of unreferenced variable bindings whose initializers are
  pure (`closure-pass-remove-unused-vars` is a specialization of
  this).
- `cv.delete()` per CLOC03 §"When a pass deletes a node" with a
  `"deleted"` tag whose `meta` records why.

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
