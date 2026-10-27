### Changed — Chinese A2 core pins

- `tests/filmstrip-target-counts/chinese.json`: 165 -> 175 (the ten writing lessons
  of chapters 147-156 each print a strip; the connected composition prints none).
- `tests/corpus/chinese.test.ts`: the writing-stage set gains
  `connected-composition`, and a new case pins the A2 level complete with
  ZH-W157-reply as the connected composition, after ZH-W97-my-own-line.
- The Chinese letter-anchoring ceilings stay `cold 0, buildsToward 51,
  unreadInventory 0, unwritten 0`: all ten new letter lessons are anchored. No
  level pin moves; Chinese stays attained at A1.
