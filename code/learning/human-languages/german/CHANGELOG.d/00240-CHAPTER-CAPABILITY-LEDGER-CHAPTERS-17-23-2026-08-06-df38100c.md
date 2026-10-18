## Chapter capability ledger — Chapters 17–23 (2026-08-06)

- Added [`chapters.json`](./chapters.json), the track's HL05 capability ledger:
  a first-person `canDo`, the shared-spine nodes realised, and a validated
  payoff for each of the **seven** chapters that carry schema-v2 lessons.
- Deliberately authored **only Chapters 17–23**. Chapters 1–16 are still schema
  v1 and declare no `practises.knowledge`, so every payoff written for them
  would have to assess atoms that do not exist. They are omitted rather than
  stubbed: an absent entry is honest debt the gap report can count, a
  placeholder is destroyed signal.
- Every `payoff.assesses` list is a strict subset of the payoff lesson's own
  `practises.knowledge`; no atom is invented, and none is padded to clear a
  threshold.
- **Chapter 17 fails the 0.5 representativeness floor at 4/12 = 0.33.** It runs
  three word lessons deep — *Kopf*, *Kopf/Haupt*, *Hand* — with no terminal
  consolidation lesson, so the payoff can only be the last lesson by
  `sequence` and reaches just its own third of the chapter. The shortfall is
  recorded in the ledger rather than hidden; the fix is a real
  Kopf/Haupt/Hand practice lesson, not a longer `assesses` list.
- Chapter 18 also lacks a terminal practice lesson but still reaches 5/8 = 0.63
  because *nein* reassesses *ja*. Chapters 19–23 are single-lesson chapters and
  assess everything they introduce (1.00).
- Titles and labels are copied verbatim from `core/book-generation.json`, so the
  `chapter-title-drift` gate holds through the HL-C04 inversion.

