- `geography/landform-extent.adj` (new) — a sibling to the already-shipped `landforms.adj`
  (`landform_description(landform, description)`, plateau → flat_elevated, plain →
  comparatively_level). That table's own already-quoted USGS Feature Type Thesaurus spans also
  state a distinct "extent" (size) descriptor for two landforms -- a fact the descriptor-only
  schema had no room for: plateau → great_extent (from "...areas of great extent and elevation..."),
  plain → considerable_extent (from "...and of considerable extent."). "Extent" is categorically
  distinct from each landform's own descriptor (flat_elevated is about being flat/elevated, not
  size; comparatively_level is about being level, not size), so decoding it is a genuinely separate
  fact. No new WebFetch -- reuses the same already-cited USGS spans. Honest abstention on mountain,
  valley, and canyon, whose own cited spans never use the word "extent" for their size. New e2e
  test file `facts_landformextent_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention). No manifest objective, matching `landforms.adj`'s own precedent. Fourth slice
  of the geography/ domain sweep. 112th content slice overall.
