# ladder_t6_top_const_cond_ws

Rung 6.top_const_cond of the CCR-066 differential complexity ladder, at `WHITESPACE_ONLY`.

## What this rung is for

Exercises a top-level `const` used as an `if` CONDITION.

Added by CCR-078 as a REGRESSION GUARD, and it is the more important half of that
issue's story.

At SIMPLE upstream treats a top-level `const` two different ways depending on
position. In a VALUE position it does not substitute (`const b=2;console.log(b);`
comes back unchanged). In a CONDITION position it uses the known value to fold
the control flow and keeps the declaration (`const DEBUG=false;if(DEBUG)…` becomes
`const DEBUG=!1;console.log(2);`).

An early attempt at CCR-078 read only the value-position half, concluded
`inline-variables` should not run at SIMPLE at all, and gated the whole pass.
That silently broke every rung in this family — and nothing caught it, because
no fixture in the 782-fixture inventory exercised a top-level `const` in a
condition. These rungs exist so that cannot happen twice.

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
  --compilation_level WHITESPACE_ONLY --js tests/diff/ladder_t6_top_const_cond_ws/input/a.js
```

## Status

**Matches upstream.**
