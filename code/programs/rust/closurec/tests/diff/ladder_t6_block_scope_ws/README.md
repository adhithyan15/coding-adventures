# ladder_t6_block_scope_ws

Rung 6.block_scope of the CCR-066 differential complexity ladder, at `WHITESPACE_ONLY`.

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
  --compilation_level WHITESPACE_ONLY --js tests/diff/ladder_t6_block_scope_ws/input/a.js
```

## Status

**Matches upstream.**
