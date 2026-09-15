- `astronomy/lunar-eclipse-type.adj` (new) -- a new `lunar_eclipse_type(type, description)`
  table names three named lunar eclipse types and what each actually is
  (total_lunar_eclipse->the_moon_moves_into_the_inner_part_of_earths_shadow_the_umbra,
  partial_lunar_eclipse->an_imperfect_alignment_of_sun_earth_and_moon_results_in_partial_umbra_passage,
  penumbral_eclipse->the_moon_travels_through_earths_penumbra_the_faint_outer_part_of_its_shadow),
  quoted verbatim from NASA's "Eclipses and the Moon" page -- `trust authoritative`, the same
  tier the sibling `solar-eclipse-type.adj` already uses for its NASA citation. Picked using
  the mandatory full-tree-grep-before-scoping discipline -- zero hits for
  `lunar_eclipse|blood_moon` before writing. WebFetch-verified twice -- the second pass
  pulled the full paragraph under each heading, confirming each quoted sentence is the
  page's own first, complete, standalone defining sentence, with a separate elaboration
  sentence following each one (not folded into the quote). This cycle also ruled out two
  other candidates for bundling 3+ facts per sentence rather than one (spring/neap tides,
  nebula types) -- a discipline reinforcement: a source's sentence must state exactly ONE
  fact to earn a row here, no matter how authoritative the source. Honest abstention on
  `blood_moon`: a real term the SAME page discusses under its own heading, but as a
  nickname for the reddish color a total lunar eclipse produces, not a fourth peer eclipse
  type. New manifest objective `adj.science.3to5.lunar_eclipse_type` (band 3-5, `recall`
  competency, `ngss` coverage root). New e2e test `facts_lunareclipsetype_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an untabled nickname).
