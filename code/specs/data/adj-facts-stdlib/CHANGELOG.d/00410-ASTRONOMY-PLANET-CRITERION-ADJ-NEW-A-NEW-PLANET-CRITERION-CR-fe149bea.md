- `astronomy/planet-criterion.adj` (new) -- a new `planet_criterion(criterion, requirement)`
  table names the three IAU requirements a body must meet to count as a full planet, not a
  dwarf planet (orbit->orbits_its_host_star, roundness->is_mostly_round,
  cleared_orbit->gravity_cleared_away_other_objects_of_similar_size_near_its_orbit), quoted
  verbatim from NASA Science's "Dwarf Planets" page -- `trust authoritative`, the same tier
  the sibling `comet-part.adj`/`space-rock-stage.adj` (the other libraries in this directory)
  already use for their NASA citations. Grounds NGSS 3-5 space-systems standards. Picked using
  the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE "planet_criterion|
  orbits_its_host_star|is_mostly_round|cleared_away|dwarf_planet"
  code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely fresh topic
  before this file was written. WebFetch-verified before writing (twice -- the second pass
  specifically confirmed the introductory sentence's exact wording and that it directly
  precedes the three-item list). Honest abstention on "dwarf_planet" (a real classification
  the same page also names, but one that is defined as satisfying the FIRST TWO criteria
  while FAILING the third -- a compound classification built FROM these criteria, not a
  fourth criterion itself, and not one of these three tabled here). New manifest objective
  `adj.science.3to5.planet_criterion` (band 3-5, `recall` competency, `ngss` coverage root).
  New e2e test `facts_planetcriterion_e2e.rs` (3 tests: direct recall, reverse binding,
  honest abstention on an untabled term).
