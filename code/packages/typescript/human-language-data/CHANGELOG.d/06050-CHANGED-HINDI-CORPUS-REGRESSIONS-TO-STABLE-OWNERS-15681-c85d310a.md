### Changed — Hindi corpus regressions have stable owners (#15681)

- Replace the chapter-edited `tests/corpus/hindi.test.ts` aggregate with track,
  script-order, writing-ramp, exam, and ownership suites under `corpus/hindi/`.
- Let lesson-budget coverage derive the canonical schema-v2 lesson count and
  require every lesson to be measured with zero excess, instead of updating a
  shared lesson/content counter after each independent chapter.
- Derive A1 totals from point probes, prove every mapped probe is taught, and
  keep the ordinal tranche pinned directly without chapter-edited totals.
- Retain durable chapter and point findings in their existing changelog and
  inventory-note owners instead of duplicating cumulative diaries in tests.
