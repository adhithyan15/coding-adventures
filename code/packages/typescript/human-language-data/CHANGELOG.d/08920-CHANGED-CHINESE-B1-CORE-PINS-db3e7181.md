### Changed — Chinese B1 core pins

- `tests/filmstrip-target-counts/chinese.json`: 223 -> 239 (the sixteen writing
  lessons of chapters 278-289 each print a strip; the reading and the three
  compositions print none).
- `tests/corpus/chinese.test.ts`: the composed-evidence list gains
  ZH-W281-one-reason (controlled), ZH-W285-a-note and ZH-W289-six-lines (connected),
  in sequence order, and a new case pins them after the A2 reply. The stage set is
  unchanged: no writing stage is first required at B1.
- `tests/levels.test.ts`: Chinese leaves the A2 reach list (now Marwadi and Punjabi)
  and joins the tracks whose reach is past A2 (twenty-one), with chapters 278-289.
- The Chinese letter-anchoring ceilings stay `cold 0, buildsToward 51,
  unreadInventory 0, unwritten 0`; letter lessons 209 -> 225, all anchored, four by a
  grammar-lesson headword. No attainment pin moves: Chinese stays attained at A2, and
  `tests/verbs.test.ts` does not move (no canonical VERB- tag).
