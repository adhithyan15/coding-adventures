### Changed — Punjabi attains A2 on the level gate

- `tests/level-gate-attainment/punjabi.json`: attained A1 -> A2, reach A2, with
  evidence for chapters 211-364. `tests/levels.test.ts` and
  `tests/level-gate.test.ts` read these owner files.
- `tests/level-gate.test.ts`: the atom-budget witness no longer fixes its level
  at A1. Its claim is that an over-budget lesson at level L blocks L and only L.
  It used the first track attained at A1, and Punjabi was the last one.
  - It now takes the lowest rung above pre-A1 that some track attains, marks
    one of that track's lessons at that rung over budget, and expects the
    track to fall exactly one rung.
  - The claim is unchanged. Only the fixture's level now comes from the corpus.
- `tests/corpus/punjabi/adds-punjabi-a2-vocabulary-5c1e8a37.case.ts`: chapters
  225-364 hold 700 distinct word lessons and 25 reviews. Every chapter opens by
  ear, and every lesson sits on an A2, A1 or pre-A1 node.
