### Changed — Marwadi attains A2 on the level gate

- `tests/level-gate-attainment/marwadi.json`: A1 -> A2. The A2 core (chapters
  152-165) realized the four A2 spine nodes the track lacked, and the A2
  vocabulary (chapters 166-290, 625 headwords) clears the vocabulary and verb
  criteria with 1,265 headwords and 276 verbs at or below A2.
- `tests/level-gate.test.ts`: a comment records Marwadi moving from the A1 row
  to the A2 row.
- `tests/corpus/marwadi/pins-marwadi-lesson-content-budgets-*.case.ts`: lessons
  962 -> 1612 (625 word lessons and 25 reviews); idioms, senses and culture
  claims hold.
- `tests/corpus/marwadi/pins-marwadi-owned-chapters-and-objective-activities-*.case.ts`:
  1612 lessons, chapters 1..290, 1611 objective activities; the generated
  typed-recall pattern grows to chapters 166-290 and their 25 reviews, so it
  matches 546 -> 1196 activities and the hand-authored list is unchanged; the
  closure `lessonCount` is 1612.
- `tests/exam-inventory-marwadi.test.ts`: 962 / 165 / 887 -> 1612 / 290 / 1512
  lessons, chapters and introduced atoms; the A1 coverage holds.
- `tests/levels.test.ts`: comment only; Marwadi stays on the A2 reach list,
  since no chapter touches a B1 node.
- `tests/plan-cli.test.ts`: Marwadi's human-validation item moves up from the
  A2 to the B1 full mocks, the rung it now works on.
- `tests/drivable-instructions.test.ts`: three spoken questions join the
  hand-verb allowlist, each asking for a word's meaning, not for a hand action:
  'to write down' (MW-C199-madno) beside the existing 'to write'
  (MW-C57-likhno), 'to draw near' (MW-C245-dhukno), and the noun 'a clap'
  (MW-C276-tali).
- The filmstrip target count (49) and the letter-anchoring ceiling (unread 5)
  do not move: the vocabulary adds no letter lesson. The info-dump
  rule-statement ceiling (32) and glyph coverage hold: the one new
  "is used for" sentence was reworded, and CDIAL's Gujarati ɔ / ε were written
  o / e.
