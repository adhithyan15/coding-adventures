- `geography/map-type.adj` (new) -- a new `map_type(type, description)` table names three
  types of map and what each actually shows (political->shows_boundaries_between_countries_
  states_counties_and_other_political_units, physical->shows_the_natural_landscape_features_
  of_earth, topographic->shows_the_shape_of_earths_surface), quoted verbatim from
  Geology.com's "Types of Maps" article -- `trust consensus`, the same tier the sibling
  `rock-type.adj` uses for some of its non-USGS citations. Grounds NGSS/social-studies map-
  skills standards for grades 3-5. Picked using the mandatory full-tree-grep-before-scoping
  discipline -- `grep -rilE "map_type|physical_map|political_map|topographic_map|
  climate_map" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely
  fresh topic before this file was written. WebFetch-verified before writing (twice -- the
  second pass specifically confirmed each sentence's exact wording, the section heading it
  appears under, and that no additional clause is attached as part of the same defining
  thought). Honest abstention on "weather" (a real map category the same page also covers,
  but one whose own section never states a single complete defining sentence the way the
  three tabled here do). New manifest objective `adj.science.3to5.map_type` (band 3-5,
  `recall` competency, `ngss` coverage root -- no dedicated social-studies coverage root is
  declared in this manifest yet, so this follows the same convention already used for other
  geography-adjacent 3-5 recall content). New e2e test `facts_maptype_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an untabled map type).
