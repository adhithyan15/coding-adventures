# ladder_t7_generator_simple

Rung 7.generator of the CCR-066 differential complexity ladder, at `SIMPLE`.

## What this rung is for

Exercises a generator function and spread over its result.

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
  --compilation_level SIMPLE_OPTIMIZATIONS --js tests/diff/ladder_t7_generator_simple/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
function* g(){yield 1}console.log([...g()]);
```

At SIMPLE the only difference is a space: upstream emits `function*g`, closurec emits `function* g` (CCR-077). At ADVANCED upstream additionally inlines the single-use generator into its call site (CCR-068).

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15862. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
