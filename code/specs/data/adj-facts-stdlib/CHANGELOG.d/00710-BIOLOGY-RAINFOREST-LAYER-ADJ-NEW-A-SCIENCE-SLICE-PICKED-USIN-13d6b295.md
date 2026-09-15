- `biology/rainforest-layer.adj` (new) -- a science slice picked using the new mandatory
  full-tree-grep-before-scoping discipline (see the entry above): `grep -ril "rainforest"
  code/specs/data/adj-facts-stdlib/` confirmed ZERO existing coverage before this file was
  written, unlike moon phases and food chain roles, both confirmed already covered elsewhere in
  the stdlib during the same research pass. A new `rainforest_layer(layer, description)` table
  names the four rainforest layers top to bottom and a one-fact description of each (emergent ->
  tallest_trees_dominate_skyline, canopy -> deep_treetop_vegetation_layer, understory ->
  dark_humid_layer_below_canopy, forest_floor -> darkest_layer_hard_for_plants_to_grow), quoted
  verbatim from National Geographic Education's "Rain Forest" entry, `trust consensus` -- a
  reputable education organization, not primary government, the same tier this stdlib already
  reserves for its other non-.gov sources. WebFetch-verified before writing (fetched twice, once
  for the overall page and once specifically to confirm the emergent layer's tree-height
  sentence verbatim). Honest abstention on "soil_layer" (not one of the four named layers). New
  manifest objective `adj.science.3to5.rainforest_layer` (band 3-5, `recall` competency, `ngss`
  coverage root). New e2e test `facts_rainforestlayer_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled layer).
