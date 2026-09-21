# ladder_t7_private_field_advanced

Rung 7.private_field of the CCR-066 differential complexity ladder, at `ADVANCED`.

## What this rung is for

Exercises a private class field.

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
  --compilation_level ADVANCED_OPTIMIZATIONS --js tests/diff/ladder_t7_private_field_advanced/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
class A{#x=1;get(){return this.#x}}console.log((new A).get());
```

Upstream DECLINES this input: Closure v20260915 does not implement private class elements at any level and exits non-zero with JSC_UNSUPPORTED_LANGUAGE_FEATURE. closurec compiles it successfully, so this is not a gap — it is closurec exceeding the oracle, and there is no upstream behaviour to converge on. Whether we should match the refusal is an open product decision, CCR-075.

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15860. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
