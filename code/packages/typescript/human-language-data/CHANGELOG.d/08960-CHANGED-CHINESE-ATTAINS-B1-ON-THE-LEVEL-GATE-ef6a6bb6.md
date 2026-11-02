### Changed — Chinese attains B1 on the level gate

- `tests/level-gate-attainment/chinese.json`: attained A2 -> B1, reach B1. The
  evidence names chapters 278-289, the B1 core, and 290-541, the B1 vocabulary.
  Together they give 2,589 headwords and 866 verbs at or below B1, and every B1
  spine node is realized. `tests/level-gate.test.ts` and `tests/levels.test.ts`
  read these owner files, so they now count 21 tracks at B1. Marwadi is the one
  track at A2 and Punjabi the one at A1.
- `tests/filmstrip-target-counts/chinese.json`: 239 -> 365. Each of the 126
  writing lessons of the B1 vocabulary prints one strip.
- The Chinese letter-anchoring ceilings stay at `cold 0, buildsToward 51,
  unreadInventory 0, unwritten 0`. Letter lessons go 225 -> 351, all anchored.
- The drivable-prefix opener ceiling does not move. Every opening lesson of
  chapters 290-541 can be driven by ear.
