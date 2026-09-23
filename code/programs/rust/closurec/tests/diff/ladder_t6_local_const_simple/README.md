# ladder_t6_local_const_simple

Rung 6.local_const of the CCR-066 differential complexity ladder, at `SIMPLE`.

## What this rung is for

Exercises a `const` bound to a literal inside a function body.

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
  --compilation_level SIMPLE_OPTIMIZATIONS --js tests/diff/ladder_t6_local_const_simple/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
function f(){const b=2;return b+1}console.log(f());
```

closurec does not propagate a binding declared inside a function. `inline-variables` considers only TOP-LEVEL `const` declarations, so a function-local one is never substituted and the body cannot fold. Upstream folds `b+1` to `3` at SIMPLE and inlines the whole call at ADVANCED. CCR-078.

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15863. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
