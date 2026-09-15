- `astronomy/planet-ordinal-position.adj` (new) — the SECOND cross-directory `rule` composition
  in the ADJ stdlib's science domain, following `earth-science/season-start-month-number.adj`'s
  precedent. Bridges the already-shipped `planet_order` table (`astronomy/planets.adj`) with the
  already-shipped `ordinal_number` table (`mathematics/ordinal-numbers.adj`) to DERIVE
  `planet_ordinal_position(planet, ordinal)` -- grounding the common early-elementary framing
  "Earth is the THIRD planet from the Sun." Reuses TWO already-verified citations (NASA planet
  order, standard English ordinal-number convention) with zero new sourcing work. Honest
  abstention on Pluto (reclassified a dwarf planet in 2006, deliberately not a row in
  `planets.adj`) -- the rule abstains rather than inventing a position. Same cross-directory
  pattern as `season-start-month-number.adj`: the library lives in `astronomy/` (its natural
  home) and imports its mathematics sibling via a relative `../mathematics/ordinal-numbers.adj`
  path; its `.query.adj` companion is placed at the package root so the CLI's import sandbox
  (rooted at the top-level program's own directory) resolves the `../` hop. New manifest
  objective `adj.science.k2.planet_ordinal_position` (band K-2, `infer` competency). New e2e test
  `facts_planetordinalposition_e2e.rs` (3 tests: direct derivation with dual citations, reverse
  binding, honest abstention on Pluto).
