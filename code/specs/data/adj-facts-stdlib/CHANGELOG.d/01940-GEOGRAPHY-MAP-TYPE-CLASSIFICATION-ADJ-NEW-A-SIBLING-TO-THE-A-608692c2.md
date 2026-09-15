- `geography/map-type-classification.adj` (new) — a sibling to the already-shipped `map-type.adj`
  (`map_type(type, description)`, topographic → shows_the_shape_of_earths_surface). That table's
  own already-quoted Geology.com span for the topographic row also classifies topographic maps as
  a KIND of map -- a fact the type/description schema had no room for: "Topographic maps are
  reference maps that show the shape of Earth's surface." New `map_type_classification(type,
  classification)` table decodes that span as its own row: topographic → reference_map. No new
  WebFetch -- reuses the same already-cited Geology.com sentence. Honest abstention on `political`
  and `physical`, whose own quotes never classify the map as a kind of map the way the topographic
  row's span does. New e2e test file `facts_maptypeclassification_e2e.rs` (3 tests: forward recall
  with citation, backward recall, honest abstention). No manifest objective, matching
  `map-type.adj`'s own precedent. Second slice of the geography/ domain sweep. 110th content slice
  overall.
