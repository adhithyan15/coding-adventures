# ladder_t7_labeled_break_simple

Rung 7.labeled_break of the CCR-066 differential complexity ladder, at `SIMPLE`.

## What this rung is for

Exercises a labelled break out of a nested loop.

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
  --compilation_level SIMPLE_OPTIMIZATIONS --js tests/diff/ladder_t7_labeled_break_simple/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
outer:for(var i=0;i<2;i++){for(var j=0;j<2;j++){break outer}}console.log(i);
```

Upstream renames the label and the loop locals (CCR-022) and drops braces around single-statement bodies (CCR-076). At SIMPLE and ADVANCED it also hoists the initializer into the for header (CCR-074).

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15861. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
