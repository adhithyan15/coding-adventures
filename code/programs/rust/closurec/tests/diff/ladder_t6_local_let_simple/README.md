# ladder_t6_local_let_simple

Rung 6.local_let of the CCR-066 differential complexity ladder, at `SIMPLE`.

## What this rung is for

Exercises a `let` bound to a literal inside a function body.

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
  --compilation_level SIMPLE_OPTIMIZATIONS --js tests/diff/ladder_t6_local_let_simple/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
function f(){let b=2;return b+1}console.log(f());
```

Same gap as `local_const`, and it shows the constraint is scope, not `const`-ness: a function-local binding is not observable from outside, so upstream folds `let` here as readily as `const`. `inline-variables` takes only top-level `const`, so neither is reached. CCR-078.

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15863. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
