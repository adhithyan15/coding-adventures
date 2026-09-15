- `biology/pond-zone.adj` (new) -- a new `pond_zone(zone, description)` table
  names three zones of a freshwater lake or pond and what each actually is
  (littoral_zone->close_to_the_shore,
  limnetic_zone->open_and_well_lit_area_of_a_freestanding_body_of_fresh_water,
  profundal_zone->deep_zone_located_below_the_range_of_effective_light_penetration),
  quoted verbatim from three separate Wikipedia articles ("Littoral zone",
  "Limnetic zone", "Profundal zone"), each article's own opening sentence --
  `trust consensus`, the same tier this stdlib already reserves for other
  Wikipedia citations (e.g. `soil-texture-class.adj`). Distinct from the
  already-shipped `oceanography/ocean-zones.adj`, which names three OCEAN
  depth zones (sunlight/twilight/midnight, ordered by how far sunlight
  reaches through open ocean water) -- a completely different body of water
  and organizing question. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- zero hits for
  `littoral`/`limnetic`/`profundal`/`pond_zone` before writing. Honest
  abstention on `benthic_zone`: a real freshwater-zone term, but not one of
  the three tabled here. New manifest objective `adj.science.6to8.pond_zone`
  (band 6-8, `recall` competency, `ngss` coverage root). New e2e test
  `facts_pondzone_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention).
