# ladder_t4_while_advanced

Rung 4.while of the CCR-066 differential complexity ladder, at `ADVANCED`.

## What this rung is for

Exercises a while loop, which upstream rewrites into for form.

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
  --compilation_level ADVANCED_OPTIMIZATIONS --js tests/diff/ladder_t4_while_advanced/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
var i=0;for(;i<3;)i++;console.log(i);
```

closurec rewrites the while into a for but leaves the initializer as a separate statement rather than hoisting it into the for header (CCR-074). The ADVANCED rung additionally needs the loop-local rename of CCR-022.

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15855. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
