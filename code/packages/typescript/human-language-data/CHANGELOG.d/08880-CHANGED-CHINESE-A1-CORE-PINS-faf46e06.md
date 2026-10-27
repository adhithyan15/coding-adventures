### Changed — Chinese A1 core pins

- `tests/filmstrip-target-counts/chinese.json`: 128 -> 141 (the thirteen writing
  lessons of chapters 85-97 each print a strip; the controlled-composition and
  timed lessons print none).
- `tests/corpus/chinese.test.ts`: the writing-stage pin now holds the two A1 stages
  the core proves, controlled composition before timed production, with A1
  complete, and the timed lesson's directed time stays inside five minutes.
- `tests/verbs.test.ts`: the mean core-verb coverage 40 -> 41. The core puts six
  canonical core-verb tags on their own nodes (VERB-GIVE, VERB-DO-MAKE, VERB-PUT,
  VERB-BRING, VERB-STAND, VERB-CAN), taking Chinese 2 -> 8 covered; measured with
  and without chapters 85-98.
- The Chinese letter-anchoring ceilings stay `cold 0, buildsToward 51,
  unreadInventory 0, unwritten 0`: all thirteen new letter lessons are anchored.
  No level pin moves; Chinese stays attained at pre-A1.
