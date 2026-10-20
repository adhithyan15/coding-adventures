# ladder_t6_array_spread_index_advanced

Rung 6.array_spread_index of the CCR-066 differential complexity ladder, at `ADVANCED`.

## What this rung is for

Exercises an array spread BEFORE an indexed read (#15942). A spread shifts every later element by an unknown amount, so `[..."xy",5][1]` must not fold to `5` — the CLOC28 miscompile no earlier rung could catch. The spread source is a string on purpose: an array-literal source is pre-flattened by another pass and hides the bug.

The ladder is ordered by deliberate complexity: every rung is simpler than the
rungs after it, so a failure is attributed to the simplest construct that
produces it. Work the ladder bottom-up.

## Provenance

Captured from Closure Compiler `v20260915` (commit `10ca677aff381d2c2e6e1b254ba32861e503173d`) using the
`closure-flags-file-v1` command in `tests/oracle/manifest.json`.
`expected.stdout` is **upstream's** output, not ours: this fixture measures
parity, not regression.

Reproduce:

```bash
java -Duser.language=en -Duser.country=US -Duser.timezone=UTC -Dfile.encoding=UTF-8 \
  -jar <closure-compiler-v20260915.jar> \
  --compilation_level ADVANCED_OPTIMIZATIONS --js tests/diff/ladder_t6_array_spread_index_advanced/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
var a=[..."xy",5];console.log(a[1]);
```

That output is correct, since it prints `y` like the source. It is not optimized: upstream inlines the single-use array literal into its use site and drops the binding (CCR-068). What matters for this rung is that closurec does **not** fold `a[1]` to `5`, which is the CLOC28 miscompile #15942 exists to catch.

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15837. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
