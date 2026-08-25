### Added - pass-ready assessment contracts and the gentle writing policy (#12207)

- Add HL16 and `core/assessment-policy.json`: instructional lessons remain capped
  at five minutes while reading, listening, writing, and speaking must each pass
  independently. The writing strand now has seven cumulative stages, from
  observe/trace through timed assessment production.
- Add a strict assessment-policy and per-track contract parser. A valid
  `<track>/assessment.json` must name an external exam or clearly labelled
  project-defined equivalent at every level, inventory all four skills, require
  explicit thresholds and writing stages, and provide at least two timed full
  mocks with rubrics and answer keys. Invalid contracts fail loudly.
- Add `assessment-contract` as the completion plan's highest-priority family.
  The current corpus now reports 22 visible contract deficits instead of letting
  content-coverage proxies imply exam readiness: 118 enumerable items and about
  10,190 projected tranches at the 2026-08-20 baseline.
- This tranche does not claim that the books are pass-ready. It makes the missing
  pass evidence machine-visible so the writing/task/mock rewrites cannot be
  forgotten or reported as zero.
