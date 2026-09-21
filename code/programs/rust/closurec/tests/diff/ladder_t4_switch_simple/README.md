# ladder_t4_switch_simple

Rung 4.switch of the CCR-066 differential complexity ladder, at `SIMPLE`.

## What this rung is for

Exercises a switch with a default arm, exercising case and break punctuation.

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
  --compilation_level SIMPLE_OPTIMIZATIONS --js tests/diff/ladder_t4_switch_simple/input/a.js
```

Upstream terminates the switch statement after its closing brace; closurec places a semicolon inside the block instead. Emitter punctuation only — both forms parse identically. This is CCR-073.

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15854. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
