# coding-adventures-closure-pass-treeshake

Tree-shaking pass for the Closure Compiler clone. Removes
`export` declarations and `import` bindings that aren't reachable
from any entry point. Per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
canonical pass set.

## Why "tree-shake"?

The program is a tree of modules connected by `import` edges.
Roots = entry-point modules. Hold the tree by its roots, shake
it — anything not connected to a root falls off.

```text
entry.js  imports  { a, b }       from utils.js
utils.js  exports  a, b, c, d
                           ▲
                           └─ c and d are unreached;
                              treeshake removes them
```

## The difference from DCE

| | DCE | Tree-shake |
|---|---|---|
| **Scope** | Within a module/function | Across modules |
| **Finds** | Locally-unused bindings | Cross-module-unused exports/imports |
| **Sees `export`s?** | Can't — they're reachable by definition | Yes — that's the point |

That's why CLOC06 pins tree-shake to depend on `dce`: DCE
shrinks the reachable use-sets first, simplifying the
cross-module use-chain analysis tree-shake needs. And once
tree-shake decides an `export` is dead, the next DCE iteration
can finally delete the underlying definition — a real cascade.

## What's here (v1)

- `TreeshakePass` implementing the `Pass` trait from
  [`closure-pass-pipeline`](../closure-pass-pipeline).
- Metadata pinned:
  - `name = "treeshake"`
  - `depends_on = ["dce"]` — DCE first so intra-module dead
    code is gone before cross-module shaking.
  - `iteration_policy = FixedPoint` — tree-shake → DCE →
    tree-shake cascade.
  - `cost = 3` — cross-module mark+sweep, same shape as DCE.
- `Pass::run` is **identity** in v1: `javascript-ast` ships
  only `Program` / `SourceType` today, so there are no
  `ImportDeclaration` / `ExportDeclaration` nodes to shake.
  The real two-phase walk slots into `Pass::run` once the AST
  grows module syntax.

## What this PR locks down even as identity

1. The `depends_on("dce")` edge is in the scheduler graph —
   DCE before tree-shake the moment both passes are in one
   pipeline.
2. Two-pass integration test
   (`pipeline_orders_dce_before_treeshake`) registers
   `TreeshakePass` first and verifies the scheduler reorders.
3. Pass metadata drives the future `closurec --disable=treeshake`.

## Where this pass sits

CLOC06 §"Canonical pass set" pins:

```text
constant-fold → fold-control-flow → dce → inline → rename →
treeshake → ...
```

Tree-shake runs late — after DCE has trimmed every module
internally, after inline has fixed call-site references, after
rename has settled on the final names.

## Deletion provenance (correlation vector)

When treeshake removes an unreferenced top-level `function`, it
tombstones that function's own CV entry via `cv.delete(cv_id,
"treeshake", "removed-unreferenced-function", meta)` (meta carries the
function `name`), and emits one summary `Contribution` against the
program root. So a `--correlation_vector` consumer asking "what happened
to `function foo`?" gets a definite answer instead of the function
silently vanishing from the provenance graph — the same audit trail the
DCE and fold-control-flow passes record for what *they* delete. `delete`
is a no-op when the log is disabled (the production default), so this
costs nothing off the `--correlation_vector` path, and program output is
byte-for-byte unchanged.

## Dependency whitelist

- `coding-adventures-closure-pass-pipeline` — `Pass` trait + types.
- `coding-adventures-javascript-ast` — `Program` input/output.
- `coding-adventures-type-sidecar` — receives sidecar reference;
  future tree-shake will read module-level `external` attributes
  to seed the root set.
- `coding_adventures_correlation_vector` — receives mutable
  `CVLog`; removed functions are tombstoned via `cv.delete()` and a
  summary `Contribution` is emitted.
- `serde_json` — `Contribution.meta` JSON values.

Dev-deps:
- `coding-adventures-javascript-tokens` for `EsVersion` in tests.
- `coding-adventures-closure-pass-dce` for the two-pass
  ordering integration test.
