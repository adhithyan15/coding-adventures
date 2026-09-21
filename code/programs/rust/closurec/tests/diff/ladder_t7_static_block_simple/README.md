# ladder_t7_static_block_simple

Rung 7.static_block of the CCR-066 differential complexity ladder, at `SIMPLE`.

## What this rung is for

Exercises a class static initialization block.

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
  --compilation_level SIMPLE_OPTIMIZATIONS --js tests/diff/ladder_t7_static_block_simple/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
class A{static x;static{A.x=1}}console.log(A.x);
```

At SIMPLE upstream lowers the class static block to an assignment after the class; closurec keeps the block. At ADVANCED upstream additionally propagates the result away entirely. Both are CCR-068 territory.

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15837. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
