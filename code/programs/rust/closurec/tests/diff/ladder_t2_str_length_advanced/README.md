# ladder_t2_str_length_advanced

Rung 2.str_length of the CCR-047 differential complexity ladder, at
`ADVANCED`.

## What this rung is for

Exercises a property read on a literal that upstream can fold.

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
  --compilation_level ADVANCED_OPTIMIZATIONS --js tests/diff/ladder_t2_str_length_advanced/input/a.js
```

## Status

Matches upstream byte for byte.
