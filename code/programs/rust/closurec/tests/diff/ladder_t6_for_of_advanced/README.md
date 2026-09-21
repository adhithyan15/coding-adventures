# ladder_t6_for_of_advanced

Rung 6.for_of of the CCR-066 differential complexity ladder, at `ADVANCED`.

## What this rung is for

Exercises a for-of loop over an array literal.

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
  --compilation_level ADVANCED_OPTIMIZATIONS --js tests/diff/ladder_t6_for_of_advanced/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
for(const v of [1,2,3]){console.log(v)}
```

Three differences: upstream rewrites `const` to `let`, renames the loop binding (CCR-022), and drops the braces around the single-statement body (CCR-076).

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15861. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
