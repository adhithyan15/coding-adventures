- `astronomy/moon-phase-ordinal-position.adj` (new) — the THIRD cross-directory `rule`
  composition in the ADJ stdlib's science domain, and the SECOND time the exact number-to-
  ordinal-word bridge pattern (first used in `astronomy/planet-ordinal-position.adj`) has been
  applied — this time to a DIFFERENT already-shipped table in the same `astronomy/` directory.
  Bridges the already-shipped `moon_phase_order` table (`astronomy/moon-phases.adj`) with the
  already-shipped `ordinal_number` table (`mathematics/ordinal-numbers.adj`) to DERIVE
  `moon_phase_ordinal_position(phase, ordinal)` -- grounding "the full Moon is the FIFTH phase in
  the cycle." Reuses TWO already-verified citations (NASA Moon phases, standard English ordinal-
  number convention) with zero new sourcing work. Honest abstention on "eclipse" (a different
  astronomical event, deliberately not a row in `moon-phases.adj`). Same cross-directory pattern
  as `planet-ordinal-position.adj`/`season-start-month-number.adj`: the library lives in
  `astronomy/` (its natural home) and imports its mathematics sibling via a relative
  `../mathematics/ordinal-numbers.adj` path; its `.query.adj` companion is placed at the package
  root. New manifest objective `adj.science.k2.moon_phase_ordinal_position` (band K-2, `infer`
  competency). New e2e test `facts_moonphaseordinalposition_e2e.rs` (3 tests: direct derivation
  with dual citations, reverse binding, honest abstention on "eclipse").
