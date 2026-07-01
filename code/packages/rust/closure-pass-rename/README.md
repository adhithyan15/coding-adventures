# coding-adventures-closure-pass-rename

Variable renaming pass for the Closure Compiler clone. Replaces
non-exported binding names with short identifiers (`a`, `b`,
`c`, ...) to reduce output size while preserving externally-visible
names. Per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
canonical pass set.

## Why rename?

```js
// before
function calculateUserDiscount(user_account, current_promotions) {
  let discount_amount = 0;
  for (const promo of current_promotions) {
    if (promo.applies_to(user_account)) {
      discount_amount += promo.value;
    }
  }
  return discount_amount;
}

// after rename (export name preserved; locals shortened)
function calculateUserDiscount(a, b) {
  let c = 0;
  for (const d of b) {
    if (d.applies_to(a)) { c += d.value; }
  }
  return c;
}
```

Externally-visible names — `export`s, public class methods,
property keys reachable from outside code — **must not be
renamed**. The pass consults the type sidecar's `external`
attribute and AST export markers to build the do-not-rename set.

## What's here (v1)

- `RenamePass` implementing the `Pass` trait from
  [`closure-pass-pipeline`](../closure-pass-pipeline).
- Metadata pinned:
  - `name = "rename"`
  - `depends_on = []` — rename is correct with or without earlier
    passes; it just produces less compression on un-optimized
    input. A future `freeze-externals` pass would join this list.
  - `iteration_policy = OneShot` — one walk renames every
    renameable binding; rename doesn't open new opportunities for
    itself.
  - `cost = 3` — two-pass walk (collect bindings, then
    substitute) plus the name allocator.
- `Pass::run` renames the **uniquely-bound names of leaf functions** (parameters and
  function-body var/let/const, in functions with no nested function) to short
  names:

  ```js
  function f(longName) { return longName + 1; }
  // ⇒ function f(a) { return a + 1 }
  ```

  It is a self-contained, scope-aware α-rename. It conservatively never
  touches module/global top-level names, free globals (`console`),
  property names (`obj.x`, `{ x: … }`), parameters re-declared as a
  local, or single-character params. Fresh names avoid every identifier
  in the function, so a rename can't collide or capture a global.
  Broader renaming (locals, non-leaf scopes, module-private top-level
  names) is future work on the same walker.

## Rename provenance (correlation vector, #89)

Renaming is a transformation, not a deletion, so — like the `rename-globals`
and `rename-properties` passes — this pass records each local α-rename as a
`renamed` **contribution** carrying `{scope, from, to}`:

```text
function f(longParam) { return longParam; }
  → contribution { source: "rename", tag: "renamed",
                   meta: { scope: "f", from: "longParam", to: "a" } }
```

The `scope` qualifier is what makes local provenance usable. Unlike globals,
local short names are allocated *fresh per function*, so the same `to` (`a`)
recurs across functions — without `scope`, a minified `a` could not be traced
back to the right original binding. Records emit in `(function source order,
then binding order, ties by original name)`, so the list is deterministic run
to run, and the pipeline attaches them to the program-root CV entry.

Program output is byte-for-byte unchanged (contributions are pure metadata).
Per-output-span provenance — contributing to each renamed identifier's own CV
id — needs the log threaded through `rewrite_uses_block`, a documented
follow-up shared with the other rename passes.

## Where this pass sits

CLOC06 §"Canonical pass set" pins:

```text
constant-fold → fold-control-flow → dce → inline → rename → ...
```

Rename runs **late** — after dead code is gone, after inlining
has decided which functions stay. Renaming earlier wastes work on
bindings that'll get deleted and makes the inliner's heuristic
harder.

## Dependency whitelist

- `coding-adventures-closure-pass-pipeline` — `Pass` trait + types.
- `coding-adventures-javascript-ast` — `Program` input/output.
- `coding-adventures-type-sidecar` — `external` attribute marks
  bindings that must not be renamed.
- `coding_adventures_correlation_vector` — receives mutable
  `CVLog` for future per-rename `Contribution` emission.
- `serde_json` — `Contribution.meta` JSON values.

Dev-deps:
- `coding-adventures-javascript-tokens` for `EsVersion` in tests.
