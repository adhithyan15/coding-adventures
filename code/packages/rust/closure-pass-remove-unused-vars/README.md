# coding-adventures-closure-pass-remove-unused-vars

Unreferenced-variable cleanup pass for the Closure Compiler clone.
The final sweep: deletes variable bindings whose initializer is
pure and whose reference-count, after every earlier pass has run,
is zero. Per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
canonical pass set.

## What it does (once the AST grows the needed variants)

```js
// After dce + inline + treeshake have done their work, the
// program might still contain:

const TMP = pure_compute();   // 0 references
const X = 1;                  // 0 references — only used by code DCE deleted

// remove-unused-vars deletes both.
```

## Why a separate pass from DCE?

DCE catches *unreachable statements* — code paths nothing flows
through. It doesn't catch every *unreferenced binding* — a
`const X = pure_compute()` with no references is technically
reachable (the statement executes), but useless if
`pure_compute()` is pure.

Three reasons to split:

1. **Different safety question.** DCE: "reachable?" This pass:
   "pure initializer?". The second needs the sidecar's `pure` /
   `no_side_effects` attributes.
2. **Different ordering.** Must run after DCE *and* inline —
   both can leave newly-orphaned bindings.
3. **Different CLI knob.** Users may want
   `--disable=remove-unused-vars` separately from disabling all
   of DCE.

Closure Compiler itself ships a `removeUnusedVars` pass for
this reason.

## What's here

- `RemoveUnusedVarsPass` implementing the `Pass` trait from
  [`closure-pass-pipeline`](../closure-pass-pipeline).
- Metadata pinned:
  - `name = "remove-unused-vars"`
  - `depends_on = ["dce", "inline"]` — both shrink the
    reference graph; this pass catches what they leave behind.
  - `iteration_policy = FixedPoint` — removing one binding can
    unreference another.
  - `cost = 3` — per-scope binding-table build + delete; same
    shape as DCE.
- A real `Pass::run`: it runs `closure-scope-analyzer`, builds a
  per-binding use-count from the resolved references, and deletes
  top-level (`ScopeId::GLOBAL`) `var`/`let`/`const` bindings that
  nothing references — **as long as the initializer is
  side-effect-free**.

## What it removes (and what it deliberately keeps)

```js
var dead = 1;            // removed — no references, literal init
var a = 1, unused = 2;   // → var a = 1;   (only the dead declarator dropped)
a;                       //   (a is referenced, so it survives)

let x = sideEffect();    // KEPT — unreferenced, but the call may have a
                         //        side effect that must still run
```

A declarator is dropped only when its name has zero resolved references
**and** its initializer is removable — absent, a literal, or a bare
identifier (a pure read). Calls, `new`, member access, assignments, etc.
are treated as potentially side-effecting and keep the declarator. Both
the bare `ProgramItem::Declaration` and the
`ProgramItem::Statement(Statement::Declaration(...))` shapes (the latter
is what the `javascript-parser` bridge emits) are handled.

Current scope: top-level bindings only. Function-local removal and
sidecar-driven purity (to reach `const x = pureCall()`) are follow-ups.

## Scheduler integration

1. The `depends_on(["dce", "inline"])` edges are in the
   scheduler graph — both predecessors must finish first.
2. `pipeline_orders_dce_before_remove_unused_vars` registers
   RemoveUnusedVarsPass first and verifies the scheduler reorders.
3. `pipeline_orders_three_passes_canonically` registers all three
   out of order and verifies the canonical
   `dce / inline → remove-unused-vars` topo-sort.
4. Pass metadata drives `closurec --disable=remove-unused-vars`.

## Where this pass sits

CLOC06 §"Canonical pass set" pins this **last** before the
emitter. Everything else has had its turn to shrink references;
this pass cleans up the resulting orphans.

## Dependency whitelist

- `coding-adventures-closure-pass-pipeline` — `Pass` trait + types.
- `coding-adventures-javascript-ast` — `Program` input/output.
- `coding-adventures-type-sidecar` — `pure` / `no_side_effects`
  attributes inform delete safety.
- `coding_adventures_correlation_vector` — receives mutable
  `CVLog` for per-deletion `Contribution` emission.
- `serde_json` — `Contribution.meta` JSON values.

Dev-deps:
- `coding-adventures-javascript-tokens` for `EsVersion` in tests.
- `coding-adventures-closure-pass-dce` for the two-pass
  ordering integration test.
- `coding-adventures-closure-pass-inline` for the three-pass
  ordering integration test.
