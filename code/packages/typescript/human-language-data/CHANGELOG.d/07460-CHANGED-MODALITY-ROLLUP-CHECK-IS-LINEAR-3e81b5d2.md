### Changed — the modality rollup check is linear

- `tests/modality-manifest.test.ts` "keeps every rollup internally
  consistent" looked each chapter's drivable lesson id up with
  `manifest.lessons.find`, scanning all ~1,400 entries per id. That made it
  quadratic in the corpus: about 11.5 s alone locally, and over the 30 s
  budget under full-suite parallel load on CI. It now indexes the manifest
  once in a `Map` (0.7 s locally) and asserts exactly the same things. The
  shared 30 s budget is unchanged.
