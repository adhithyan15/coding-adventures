# ladder_t3_unused_local_advanced

Rung 3.unused_local of the CCR-066 differential complexity ladder, at
`ADVANCED`.

## What this rung is for

Exercises a function-local binding that is never read.

The ladder is ordered by deliberate complexity: every rung is simpler than the
rungs after it, so a failure is attributed to the simplest construct that
produces it. Work the ladder bottom-up — a tier 3 failure is a reason to stop
and fix tier 3, not to move on to tier 4.

## Provenance

Captured from Closure Compiler `v20260915` (commit `10ca677aff381d2c2e6e1b254ba32861e503173d`) using the
`closure-flags-file-v1` command in `tests/oracle/manifest.json`. `expected.stdout`
is **upstream's** output, not ours: this fixture measures parity, not
regression.

Reproduce:

```bash
java -Duser.language=en -Duser.country=US -Duser.timezone=UTC -Dfile.encoding=UTF-8 \
  -jar <closure-compiler-v20260915.jar> \
  --compilation_level ADVANCED_OPTIMIZATIONS --js tests/diff/ladder_t3_unused_local_advanced/input/a.js
```

## Status

**Known divergence** — see `tests/ladder/divergences.json`.

## Divergence

`closurec` currently emits:

```js
function f(){var u=1;return 2}console.log(f());
```

closurec does not remove a function-local binding that is never read. `remove-unused-vars` is gated to ADVANCED, but upstream removes scope-local bindings at SIMPLE too, where the open-world contract does not apply.

Tracked in https://github.com/adhithyan15/coding-adventures/issues/15837. The divergence is pinned in `tests/ladder/divergences.json`, so closing the gap fails this fixture's harness assertion — that failure is the signal to delete the ledger entry.
