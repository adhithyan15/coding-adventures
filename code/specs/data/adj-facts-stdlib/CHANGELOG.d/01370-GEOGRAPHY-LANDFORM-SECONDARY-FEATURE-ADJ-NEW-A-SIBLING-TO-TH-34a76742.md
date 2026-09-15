- `geography/landform-secondary-feature.adj` (new) — a sibling to the already-shipped
  `landforms.adj` (`landform_description(landform, description)`, ONE defining descriptor per
  landform: mountain, valley, plateau, plain, canyon). That table's own header already quotes the
  FULL USGS Feature Type Thesaurus span for every row, but the descriptor-only schema reduced each
  span to a single atom, leaving a second, structural clause unused for three of the five
  landforms: valley's span also states "containing a stream with an outlet," plateau's span also
  states "limited on at least one side by an abrupt descent," and canyon's span also states "the
  bottom of which generally has a continuous slope." New `landform_secondary_feature(landform,
  feature)` table: valley → contains_stream_with_outlet, plateau → bounded_by_abrupt_descent,
  canyon → continuous_slope_at_bottom. Honest abstention on mountain and plain, whose own cited
  spans state only the single descriptor already captured by the parent table. New e2e test file
  `facts_landformsecondaryfeature_e2e.rs` (3 tests: forward recall with citation, backward recall
  from a bound feature, honest abstention on mountain). No manifest objective, matching
  `landforms.adj`'s own precedent of not having one. Third and last slice from the geography/ sweep
  tranche — geography/ (5 tables) is now fully exhausted (3/3 candidates shipped:
  reference-line-hemisphere-split, ocean-deepest, landform-secondary-feature).
