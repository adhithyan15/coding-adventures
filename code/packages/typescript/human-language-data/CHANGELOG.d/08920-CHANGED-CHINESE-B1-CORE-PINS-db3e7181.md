### Changed — Chinese B1 core pins

- `tests/filmstrip-target-counts/chinese.json`: 223 -> 239 (the sixteen writing
  lessons of chapters 278-289 each print a strip; the reading and the three
  compositions print none).
- `tests/corpus/chinese.test.ts`: the composed-evidence list gains
  ZH-W281-one-reason (controlled), ZH-W285-a-note and ZH-W289-six-lines (connected),
  in sequence order, and a new case pins them after the A2 reply. The stage set is
  unchanged: no writing stage is first required at B1.
- `tests/level-gate-attainment/chinese.json`: reach A2 -> B1, with evidence naming
  chapters 278-289, which touch every B1 spine node. `tests/levels.test.ts` and
  `tests/level-gate.test.ts` derive their counts from these owner files, so neither
  test changes.
- The Chinese letter-anchoring ceilings stay `cold 0, buildsToward 51,
  unreadInventory 0, unwritten 0`; letter lessons 209 -> 225, all anchored, four by a
  grammar-lesson headword. The attained level does not move: Chinese stays attained at A2, and
  `tests/verbs.test.ts` does not move (no canonical VERB- tag).
