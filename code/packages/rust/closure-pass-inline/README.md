# coding-adventures-closure-pass-inline

Function-inlining pass for the Closure Compiler clone. Substitutes
a callee's body at the call site when doing so is cheaper than the
call. Per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
canonical pass set.

## What it does (once the AST grows the needed variants)

```js
// before
function double(x) { return x * 2; }
const a = double(7);

// after inline
const a = 7 * 2;

// after the next constant-fold iteration
const a = 14;
```

That cascade — inline exposes new constant-folding opportunities —
is exactly why `IterationPolicy::FixedPoint` is the right policy.

## The two questions every inliner answers

1. **Is it safe?** Substituting the body must not change
   semantics. `this`, `arguments`, captured variables, recursion,
   and side-effecting argument expressions used multiple times all
   need careful handling. The sidecar's `no_side_effects` / `pure`
   attributes plus a per-parameter use-count analysis answer most
   of it.
2. **Is it worth it?** Inlining a 1000-line function at 50 call
   sites bloats output. Inlining a 3-line single-use helper
   shrinks it. CLOC06 leaves the exact heuristic open.

## What's here (v1)

- `InlinePass` implementing the `Pass` trait from
  [`closure-pass-pipeline`](../closure-pass-pipeline).
- Metadata pinned:
  - `name = "inline"`
  - `depends_on = ["constant-fold"]` — folded arguments plug
    into parameters cleanly.
  - `iteration_policy = FixedPoint` — inlined bodies expose new
    inlineable calls.
  - `cost = 4` — call-graph build + heuristic eval +
    clone-and-rewrite per inlined call site.
- `Pass::run` is **identity** in v1: `javascript-ast` ships
  only `Program` / `SourceType` today, so there are no
  `FunctionDeclaration` / `CallExpression` / `Identifier` nodes
  to inline. The real call-graph walk slots into `Pass::run`
  once the AST grows variants.

## What this PR locks down even as identity

1. The `depends_on("constant-fold")` edge is in the scheduler
   graph — folded args before inlining.
2. The two-pass integration test
   (`pipeline_orders_constant_fold_before_inline`) registers
   `InlinePass` first and verifies the scheduler reorders.
3. Pass metadata drives the future `closurec` CLI's
   `--disable=inline` flag.

## Where this pass sits

CLOC06 §"Canonical pass set" pins:

```text
constant-fold → fold-control-flow → dce → inline → rename → ...
```

Inline runs **after DCE** so it doesn't bother inlining callees
that are about to be deleted, and **before rename** so the
inliner's heuristic sees meaningful names. Both relationships
are *preferences*, not *correctness*, so they're not in
`depends_on` — only the constant-fold dependency is.

## Dependency whitelist

- `coding-adventures-closure-pass-pipeline` — `Pass` trait + types.
- `coding-adventures-javascript-ast` — `Program` input/output.
- `coding-adventures-type-sidecar` — `no_side_effects` / `pure`
  attributes inform inline safety.
- `coding_adventures_correlation_vector` — receives mutable
  `CVLog` for per-inline `Contribution` emission.
- `serde_json` — `Contribution.meta` JSON values.

Dev-deps:
- `coding-adventures-javascript-tokens` for `EsVersion` in tests.
- `coding-adventures-closure-pass-constant-fold` for the
  two-pass ordering integration test.
