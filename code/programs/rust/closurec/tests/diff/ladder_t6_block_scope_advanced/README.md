# ladder_t6_block_scope_advanced

Rung 6.block_scope of the CCR-066 differential complexity ladder, at `ADVANCED`.

## What this rung is for

Exercises a `const` bound to a literal inside a bare block.

Added by CCR-078. The `let_const` rung declares at top level, so it could only ever
measure the half of the gap where `closurec` optimizes MORE than the oracle.
These three measure the other half — local scope, where we optimize less.

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
  --compilation_level ADVANCED_OPTIMIZATIONS --js tests/diff/ladder_t6_block_scope_advanced/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
{const b=2;console.log(b)};
```

closurec keeps the block and the binding. Upstream propagates the value, drops the dead binding, and unwraps the now-single-statement block, leaving `console.log(2);`. Two causes: the local-scope propagation gap (CCR-078) and brace-dropping (CCR-076).

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15863. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
