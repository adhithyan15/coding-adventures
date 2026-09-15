- `biology/biome-type.adj` (new) -- a new `biome_type(biome, description)` table names four
  major biomes and what defines each
  (desert->dry_areas_where_rainfall_is_less_than_50_centimeters_20_inches_per_year,
  forest->dominated_by_trees_and_cover_about_one_third_of_the_earth,
  grassland->open_regions_dominated_by_grass_with_a_warm_dry_climate,
  tundra->has_extremely_inhospitable_conditions_with_the_lowest_measured_temperatures),
  quoted verbatim from National Geographic Education's "The Five Major Types of Biomes"
  article -- `trust consensus`, the same tier already used for `map-type.adj`'s Geology.com
  citation and `figurative-language-type.adj`'s Grammarly citation. Picked using the
  mandatory full-tree-grep-before-scoping discipline -- `grep -rilE "biome_type|tundra"
  code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely fresh topic
  before this file was written (also confirmed `biology/animal-habitat.adj` maps individual
  animals to habitat NAMES, never tables biome-level defining sentences, so no overlap).
  WebFetch-verified before writing (twice -- the second pass pulled the full surrounding
  paragraph for each candidate sentence to confirm it stands alone grammatically and isn't
  qualified or contradicted by an immediately adjacent sentence). Honest abstention on
  "aquatic" (the source's fifth major biome, but one whose own section opens by deferring to
  its freshwater and marine sub-categories rather than stating a single, complete defining
  sentence the way the four tabled here do). New manifest objective
  `adj.science.3to5.biome_type` (band 3-5, `recall` competency, `ngss` coverage root). New
  e2e test `facts_biometype_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on an untabled biome).
