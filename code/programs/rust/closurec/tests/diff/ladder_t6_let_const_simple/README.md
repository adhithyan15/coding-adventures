# ladder_t6_let_const_simple

Rung 6.let_const of the CCR-066 differential complexity ladder, at `SIMPLE`.

## What this rung is for

Exercises block-scoped declarations.

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
  --compilation_level SIMPLE_OPTIMIZATIONS --js tests/diff/ladder_t6_let_const_simple/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
let a=1;const b=2;console.log(a+2);
```

At SIMPLE this runs the OPPOSITE way to every other rung: closurec propagates the const into its use site and upstream does not, so we optimize MORE than the oracle. That may be a win or an unsound propagation — tracked as CCR-078. At ADVANCED the ordinary CCR-068 gap applies.

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15863. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
