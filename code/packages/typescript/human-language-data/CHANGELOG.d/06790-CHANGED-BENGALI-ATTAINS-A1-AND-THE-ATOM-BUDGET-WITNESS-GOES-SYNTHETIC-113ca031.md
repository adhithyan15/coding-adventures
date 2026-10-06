### Changed — Bengali attains A1, and the atom-budget witness goes synthetic

- `tests/level-gate-attainment/bengali.json` moves from pre-A1 to A1.
- `tests/level-gate.test.ts`: "scopes the atom budget to the level" used to need a real track that attains pre-A1 while an A1 lesson is over budget. Bengali was the last one. The test now builds that case at import:
  - it takes the alphabetically first track that attains A1 on the real corpus;
  - it marks one of that track's A1 lessons over budget in the ramp report;
  - it asserts the gate drops the track to pre-A1, with exactly one atom-budget blocker at A1.
- `tests/ramp.test.ts`: the corpus's steepest lesson falls from six new atoms to four, because Bengali's *one to five* split its দুই history into a continuation. Three lessons remain over budget, all at four atoms: PA-C07-hona, PA-C07-khana and RU-C03-govorit.
