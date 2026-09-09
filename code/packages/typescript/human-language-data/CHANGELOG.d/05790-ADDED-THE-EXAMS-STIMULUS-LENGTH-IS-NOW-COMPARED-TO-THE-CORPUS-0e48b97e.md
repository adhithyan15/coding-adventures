### Added — the exam's stimulus length is now compared to the corpus

- `reading-reach.ts` measures each track's longest `comprehension` passage
  against the `stimulusLength` its own `task-shapes/<level>.json` declares.
  Those numbers were transcribed carefully and then never compared to anything:
  `task-shapes.ts` validated that a length parses, and nothing asked whether the
  curriculum ever produces a text that long.
- Every inventory the registry enumerates gets exactly one status —
  `measurable`, `length-not-published`, or `no-reading-section` — so a row can
  never be silently dropped. Four inventories (arabic/A1, french/A1,
  french/pre-A1, german/A1) legitimately publish no word count, and are excluded
  **with the board's own wording** rather than skipped.
- `core/reading-reach-floor.json` is a ratchet: a track's parts-within-reach may
  rise but never fall. Absent entries floor at zero, so only a track that has
  actually gained reading is ever edited — deliberately not a corpus-wide total,
  which is the shape that made an earlier assertion move nine times in one day.
- Falsified rather than assumed: raising a floor above reality fails naming the
  track; a floor for a non-existent inventory fails naming the key; and
  relabelling Spanish's three passage headings — the only reading in the corpus —
  fails the non-vacuity check, which exists because every other assertion here
  holds vacuously over an all-zero table.
- A word is a run containing a letter **or a digit**. The digit half is not
  incidental: the boards count numerals in their published word counts, so
  counting letters only would measure our passages by a stricter rule than the
  target they are compared against.
- Read-only by design (`npm run report:reading-reach`). Three derived ledgers
  already regenerate on every lesson-prose edit; a fourth would tax every
  authoring branch to store something printable on demand.
