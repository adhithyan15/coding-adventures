### Changed — the literal-markup corpus gate gets 360 s on slow Windows runners

`literal-markup.test.ts`'s "THE GATE: the committed corpus carries no literal
markup at either layer" timed out at 180 s on `build (windows-2025)` for #17972.
It finished in 210 s.

- **Measured.** Locally it takes 35.5 s: about 10 s to load the corpus, 22.5 s
  to render all 17,773 book outputs, and 1.5 s to measure them. The Windows
  runner, building ~490 packages at once, ran it about 6× slower. It was the
  second-slowest test in the suite there, after `plan-cli.test.ts`.
- **Change.** Its budget goes from 180 s to 360 s. It renders every book of
  every track, more than any other whole-corpus gate, so it gets twice their
  budget instead of raising theirs too. The other gates finished inside their
  limits on the same run (`gentle-ramp` took 144 s of 180 s).
