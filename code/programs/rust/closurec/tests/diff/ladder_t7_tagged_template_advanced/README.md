# ladder_t7_tagged_template_advanced

Rung 7.tagged_template of the CCR-066 differential complexity ladder, at `ADVANCED`.

## What this rung is for

Exercises a tagged template literal.

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
  --compilation_level ADVANCED_OPTIMIZATIONS --js tests/diff/ladder_t7_tagged_template_advanced/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
function t(s){return s[0]}console.log(t`hi`);
```

Upstream renames the tag function's parameter, which closurec skips (CCR-022). At ADVANCED it additionally inlines the single-use tag function into the call site (CCR-068).

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15856. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
