# ladder_t3_nested_fn_advanced

Rung 3.nested_fn of the CCR-047 differential complexity ladder, at
`ADVANCED`.

## What this rung is for

Exercises a function nested inside a function, single call site.

The ladder is ordered by deliberate complexity: every rung is simpler than the
rungs after it, so a failure is attributed to the simplest construct that
produces it. Work the ladder bottom-up — a tier 3 failure is a reason to stop
and fix tier 3, not to move on to tier 4.

## Provenance

Captured from Closure Compiler `v20260915` (commit `10ca677aff381d2c2e6e1b254ba32861e503173d`) using the
`closure-flags-file-v1` command in `tests/oracle/manifest.json`. `expected.stdout`
is **upstream's** output, not ours: this fixture measures parity, not
regression.

Reproduce:

```bash
java -Duser.language=en -Duser.country=US -Duser.timezone=UTC -Dfile.encoding=UTF-8 \
  -jar <closure-compiler-v20260915.jar> \
  --compilation_level ADVANCED_OPTIMIZATIONS --js tests/diff/ladder_t3_nested_fn_advanced/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
function o(){function i(){return 1}return i()}console.log(o());
```

closurec does not inline a single-use nested function. `inline` is registered only under ADVANCED, and even there handles a narrower case than upstream.

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15837. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
