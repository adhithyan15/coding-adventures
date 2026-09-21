# ladder_t7_static_block_ws

Rung 7.static_block of the CCR-066 differential complexity ladder, at `WHITESPACE_ONLY`.

## What this rung is for

Exercises a class static initialization block.

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
  --compilation_level WHITESPACE_ONLY --js tests/diff/ladder_t7_static_block_ws/input/a.js
```

## Status

Matches upstream byte for byte.
