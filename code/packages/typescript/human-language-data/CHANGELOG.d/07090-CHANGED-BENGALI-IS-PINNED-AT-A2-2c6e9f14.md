### Changed — Bengali is pinned at A2

- `tests/level-gate-attainment/bengali.json` moves from A1 to A2. Bengali's
  chapters 145-262 realize its last three A2 spine nodes (the past, the future
  and reading practical texts). They bring it to more than 1,200 headwords and
  120 verbs at or below A2, so no blocker remains below B1.
- `tests/level-gate.test.ts`: the climb history records Bengali's climb to A2.
- `tests/level-gate.test.ts`: the etymology-waiver test picks as its witness
  the first in-progress track with a waived etymology hook, then hosts a
  synthetic atom on a lesson at that track's in-progress level. Bengali became
  that witness once it reached A2, but it is in progress at B1 and has no B1
  lesson yet, so the atom had no host and the test failed. The witness must now
  also have a lesson at its in-progress level.
- `tests/gentle-ramp-retirement.test.ts`: the whole-corpus `--check` derivation
  test gets a 180s budget, matching `gentle-ramp.test.ts`, which builds the same
  report. It runs ~17s on an idle machine, and with these chapters it went past
  its old 30s under CI's full-suite load. It still runs in full and asserts the
  same result.
