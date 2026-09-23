# ladder_t4_try_catch_simple

Rung 4.try_catch of the CCR-066 differential complexity ladder, at `SIMPLE`.

## What this rung is for

Exercises a try/catch, whose catch parameter is a binding in its own scope.

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
  --compilation_level SIMPLE_OPTIMIZATIONS --js tests/diff/ladder_t4_try_catch_simple/input/a.js
```

Two causes. Upstream renames the catch parameter `e` to `a`, which closurec skips (CCR-022), and terminates the try statement after its closing brace, which closurec omits (CCR-073). Both must be fixed before this rung matches.

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15856. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
