- `astronomy/comet-part.adj` (new) -- a new `comet_part(part, description)` table names
  three physical parts of a comet and what each actually is
  (nucleus->solid_frozen_core_at_the_heart_of_the_comet,
  coma->fuzzy_cloud_of_gas_and_dust_around_the_nucleus,
  tail->streams_away_from_the_nucleus_pushed_by_sunlight_and_solar_particles), quoted
  verbatim from NASA Space Place's "What Is a Comet?" page -- `trust authoritative`, the
  same tier the sibling `moon-phases.adj`/`solar-eclipse-type.adj` (the other libraries in
  this directory) already use for their NASA citations. Grounds NGSS 3-5 space-systems
  standards. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\bcomet_part\b|\bnucleus\b|\bcoma\b|comet.*tail|\bshort_period_comet\b|
  \blong_period_comet\b" code/specs/data/adj-facts-stdlib/` found only incidental
  unrelated matches (cell/atomic "nucleus" in biology/chemistry libraries), confirming a
  completely fresh comet-specific topic before this file was written. WebFetch-verified
  before writing (twice). Honest abstention on "short_period_comet" (a real comet-related
  term the same page also names, but one that classifies comets by ORBITAL PERIOD rather
  than by physical anatomy -- a different axis from nucleus/coma/tail, and not one of these
  three tabled here). New manifest objective `adj.science.3to5.comet_part` (band 3-5,
  `recall` competency, `ngss` coverage root). New e2e test `facts_cometpart_e2e.rs`
  (3 tests: direct recall, reverse binding, honest abstention on an untabled comet term).
