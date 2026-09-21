# ladder_t4_for_loop_advanced

Rung 4.for_loop of the CCR-066 differential complexity ladder, at `ADVANCED`.

## What this rung is for

Exercises a counted for loop, exercising the three-clause header.

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
  --compilation_level ADVANCED_OPTIMIZATIONS --js tests/diff/ladder_t4_for_loop_advanced/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
for(var i=0;i<3;i++)console.log(i);
```

closurec does not rename loop-header locals. Upstream renames `i` to `a`. This is the CCR-022 renaming-breadth gap.

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15856. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
