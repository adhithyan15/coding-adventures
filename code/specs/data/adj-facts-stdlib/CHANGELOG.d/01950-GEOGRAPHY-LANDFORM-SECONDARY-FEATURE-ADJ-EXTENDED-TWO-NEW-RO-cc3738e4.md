- `geography/landform-secondary-feature.adj` (extended) — two new rows added to the already-shipped
  sibling table (`landform_secondary_feature(landform, feature)`, off `landforms.adj`). The SAME
  already-cited USGS Feature Type Thesaurus spans this table already quotes name a further,
  previously-unused structural clause for two more landforms: `canyon` ("...narrow, deep
  depressions with steep sides...") → `steep_sides`, and `plain` ("Regions of general uniform
  slope, comparatively level and of considerable extent.") → `uniform_slope`. `canyon` now carries
  TWO rows (it already had `continuous_slope_at_bottom`), making its forward recall a genuine
  multi-answer query, the established pattern this stdlib already uses elsewhere (e.g.
  `geology/igneous-rock-type-eruption-location.adj`). No new WebFetch -- `steep_sides` reuses a
  clause already cited in this table's own header, and `uniform_slope`'s full span reuses the
  SAME quote already cited in `landforms.adj`'s header (no new source). Extended e2e test file
  `facts_landformsecondaryfeature_e2e.rs` (2 new tests: canyon multi-answer recall, plain forward
  recall with citation; 5 tests total). No manifest objective, matching this table's own
  precedent. Third slice of the geography/ domain sweep. 111th content slice overall.
