### Changed — the literal-markup corpus gate gets 180s

- `tests/literal-markup.test.ts`: "THE GATE" renders every generated book of
  every track, so it slows as the corpus grows. With Arabic past page 999 it
  took 36s locally and timed out at its 60s budget on a CI runner. It now has
  180s, the same budget as the other whole-corpus gates.
