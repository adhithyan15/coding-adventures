### Added — HL-C03: the nine HL05 chapter gates, as measurement rather than judgement

- Add `src/chapters.ts` with all nine HL05 gates — `chapter-missing-capability`,
  `chapter-unknown-payoff-lesson`, `chapter-payoff-not-closed`,
  `chapter-payoff-not-representative`, `chapter-duplicate`, `chapter-title-drift`,
  `pattern-slot-not-closed`, `pattern-missing-production`, `pattern-multiple-atoms` —
  and publish them through the gap report's new `chapters` section.
- **Report-only, and that is the design, not caution.** 98 of the corpus's 377 book
  chapters carry no capability entry. Wiring these into `validateCurriculum()` as errors
  would have converted a measurement of pre-existing debt into 98 build failures on a
  corpus nobody had regressed. Per-track rollups carry a `clean` flag so a track flips to
  hard errors once its own debt is zero — the HL-V01 precedent, and the same reasoning
  that ships the LaTeX warning baselines unseeded.
- **The first published snapshot: 377 book chapters, 279 declared, 98 without a
  capability, 24 payoffs below the 0.5 representativeness floor, and zero unclosed
  payoffs, zero unknown payoff lessons, zero title drift, zero duplicates.** Three tracks
  — `chinese`, `japanese`, `latin` — are already clean and could flip to errors today.
- **`payoffsNotClosed` read 279 — every authored chapter — on the first run, and that was
  this module, not the corpus.** Introduced atoms live in a FLAT dotted frontmatter key
  (`introduces.knowledge`) plus block-level `hl-knowledge` directives; reading a nested
  `introduces: { knowledge }` object returns `undefined` for every lesson in the corpus,
  which silently empties the "taught so far" set instead of failing. The fix reads the
  union of both sources. A gate reporting total corpus failure is usually reporting on
  itself, and the pinned snapshot exists so that stays visible.
- The three `pattern` rules find nothing, because HL-C05 has not added the `pattern`
  lesson type yet. They are wired now so the first authored pattern is checked the moment
  it exists rather than being remembered later.
- Summary gains `chaptersWithoutCapability`, `chapterPayoffsNotRepresentative` and
  `chapterGateCleanTracks`, each `null` rather than `0` when a caller passes no ledgers —
  "not measured" and "measured, none found" are different facts.

