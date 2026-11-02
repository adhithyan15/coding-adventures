### Changed — Marwadi A2 core pins

- `tests/corpus/marwadi/pins-marwadi-lesson-content-budgets-*.case.ts`: lessons
  901 -> 962 (the A2 core, chapters 152-165); idioms, senses and culture claims
  hold.
- `tests/corpus/marwadi/pins-marwadi-owned-chapters-and-objective-activities-*.case.ts`:
  962 lessons, chapters 1..165, 961 objective activities, the 61 core activity
  ids added to the hand-authored list, and the closure `lessonCount`.
- `tests/exam-inventory-marwadi.test.ts`: 901 / 151 / 838 -> 962 / 165 / 887
  lessons, chapters and introduced atoms; the A1 coverage (103 / 197) holds.
- `tests/letter-anchoring-ceilings/marwadi.json`: unreadInventory 11 -> 5 (the
  core writes ऊ ध झ ओ ढ फ, each after its anchor word).
- `tests/filmstrip-target-counts/marwadi.json`: 43 -> 49 (one strip per new
  letter lesson).
- `tests/verbs.test.ts`: `tracksWithNoCoreVerb` 2 -> 1. Marwadi's verbs are all
  namespaced MW-VERB-* tags; the core's हूं · है lesson carries the canonical
  VERB-BE, so the track now covers one core verb. The mean holds at 41.
