- `astronomy/solar-eclipse-type.adj` (new) -- a new `solar_eclipse_type(type, description)`
  table names three solar eclipse types and what each actually is
  (total_solar_eclipse->completely_blocking_the_face_of_the_sun,
  annular_solar_eclipse->moon_at_or_near_its_farthest_point_from_earth,
  partial_solar_eclipse->sun_moon_and_earth_not_perfectly_lined_up), quoted verbatim from
  NASA's "Types of Solar Eclipses" page -- `trust authoritative`, the same tier the sibling
  `moon-phases.adj` (the other library in this directory) already uses for its NASA citation.
  Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\bsolar_eclipse\b|\btotal_solar_eclipse\b|\bannular\b|\bpartial_solar_eclipse\b|
  \bhybrid_solar_eclipse\b|eclipse_type" code/specs/data/adj-facts-stdlib/` found ZERO hits
  (the sibling `moon-phases.adj` only mentions "eclipse" once, as a deliberately-excluded
  non-phase example), confirming a completely fresh topic before this file was written.
  WebFetch-verified before writing (twice). Honest abstention on "hybrid_solar_eclipse" (a
  real eclipse type the same page also names, but whose own explanation takes TWO sentences
  -- how Earth's curved surface lets an eclipse shift between annular and total -- rather
  than one clean quotable sentence like the three tabled here). New manifest objective
  `adj.science.3to5.solar_eclipse_type` (band 3-5, `recall` competency, `ngss` coverage
  root). New e2e test `facts_solareclipsetype_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled eclipse type).
