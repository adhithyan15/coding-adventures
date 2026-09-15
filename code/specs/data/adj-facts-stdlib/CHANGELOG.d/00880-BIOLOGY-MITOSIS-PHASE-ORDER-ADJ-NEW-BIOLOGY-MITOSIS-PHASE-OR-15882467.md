- `biology/mitosis-phase-order.adj` (new) + `biology/mitosis-phase-ordinal-position.adj` (new) --
  the FIRST biology-domain entry in the ordinal-bridge composition pattern
  `earth-science/season-start-month-number.adj`, `astronomy/planet-ordinal-position.adj`, and
  `astronomy/moon-phase-ordinal-position.adj` already established, and the FOURTH cross-directory
  `rule` composition overall. `mitosis-phases.adj` (already shipped) tables each phase's defining
  event but only encodes cycle ORDER as row order, not a queryable number, so
  `mitosis-phase-order.adj` makes that same source's ordering ("The four phases of mitosis are
  Prophase ... Metaphase ... Anaphase ... Telophase", the SAME NCI SEER sentence
  `mitosis-phases.adj` already cites -- zero new sourcing risk) a first-class
  `mitosis_phase_order(phase, order)` fact, mirroring `astronomy/moon-phases.adj`'s own
  `moon_phase_order` column. `mitosis-phase-ordinal-position.adj` then bridges that new fact to
  the already-shipped `mathematics/ordinal-numbers.adj` exactly as the astronomy ordinal bridges
  do, deriving `mitosis_phase_ordinal_position($Phase, $Ordinal)` (e.g. "anaphase" -> "third").
  Honest abstention on `interphase` (the resting phase BETWEEN divisions, deliberately excluded
  from both tables, mirroring `mitosis-phases.adj`'s own exclusion). New manifest objective
  `adj.science.6to8.mitosis_phase_ordinal_position` (band 6-8, matching where NGSS places cell
  division, vs. the K-2 band of the three prior astronomy/earth-science ordinal bridges). New
  e2e test `facts_mitosisphaseordinalposition_e2e.rs` (3 tests: direct derivation with dual
  citations, reverse binding, honest abstention on `interphase`).
