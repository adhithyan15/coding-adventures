# coding-adventures-closure-pass-collapse-properties

Property-collapse pass for the Closure Compiler clone. Collapses
repeated nested property-access chains on stable namespace-style
objects into shorter local bindings. Per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
canonical pass set.

## What it does (once the AST grows the needed variants)

```js
// before
ns.utils.format.currency(1);
ns.utils.format.percent(0.5);
ns.utils.format.date(now);

// after collapse-properties
const $f = ns.utils.format;
$f.currency(1);
$f.percent(0.5);
$f.date(now);
```

Two wins: shorter source and fewer property lookups at runtime.

## Why "stable" matters

Collapsing is only safe when the intermediate is genuinely
stable — neither the chain nor any function in between can
mutate the namespace object. The pass reads the type sidecar's
`stable` / `pure` / `frozen` attributes plus a local mutation
analysis; without evidence, it bails.

## What's here (v1)

- `CollapsePropertiesPass` implementing the `Pass` trait from
  [`closure-pass-pipeline`](../closure-pass-pipeline).
- Metadata pinned:
  - `name = "collapse-properties"`
  - `depends_on = ["constant-fold"]` — folded constants resolve
    into recognisable property-access shapes that collapse can
    spot.
  - `iteration_policy = FixedPoint` — collapsing one chain can
    expose new shared prefixes (`$c.theme.x`, `$c.theme.y` → `$t.x`,
    `$t.y`).
  - `cost = 3` — gather chain frequencies + emit binding +
    rewrite uses. Same shape as DCE.
- `Pass::run` is **identity** in v1: `javascript-ast` ships
  only `Program` / `SourceType` today, so there are no
  `MemberExpression` / `Identifier` / `VariableDeclaration`
  nodes to collapse. The real gather + rewrite slots into
  `Pass::run` once the AST grows variants.

## What this PR locks down even as identity

1. The `depends_on("constant-fold")` edge is in the scheduler
   graph — folded constants before chain collapse.
2. The two-pass integration test
   (`pipeline_orders_constant_fold_before_collapse_properties`)
   registers `CollapsePropertiesPass` first and verifies the
   scheduler reorders.
3. Pass metadata drives the future
   `closurec --disable=collapse-properties`.

## Where this pass sits

CLOC06 §"Canonical pass set" pins this after the value-folding
group and before `rename` — so the pass sees folded names
(meaningful chain prefixes) but operates before renaming
flattens identifiers.

## Dependency whitelist

- `coding-adventures-closure-pass-pipeline` — `Pass` trait + types.
- `coding-adventures-javascript-ast` — `Program` input/output.
- `coding-adventures-type-sidecar` — `stable` / `pure` / `frozen`
  attributes inform collapse safety.
- `coding_adventures_correlation_vector` — receives mutable
  `CVLog` for per-collapse `Contribution` emission.
- `serde_json` — `Contribution.meta` JSON values.

Dev-deps:
- `coding-adventures-javascript-tokens` for `EsVersion` in tests.
- `coding-adventures-closure-pass-constant-fold` for the
  two-pass ordering integration test.
