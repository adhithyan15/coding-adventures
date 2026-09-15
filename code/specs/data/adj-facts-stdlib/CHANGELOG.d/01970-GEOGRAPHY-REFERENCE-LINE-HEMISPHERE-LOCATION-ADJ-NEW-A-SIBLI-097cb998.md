- `geography/reference-line-hemisphere-location.adj` (new) — a sibling to the already-shipped
  `reference-line-degree.adj` (`reference_line_degree(line, degrees)`, tropic_of_cancer → 23.5,
  tropic_of_capricorn → -23.5). That table's own already-quoted NOAA NESDIS span also names, in
  plain words, which single hemisphere each tropic sits within -- a fact the numeric-degree schema
  had no room for: "One in the Northern Hemisphere called the Tropic of Cancer at +23.5° latitude
  and one in the Southern Hemisphere called the Tropic of Capricorn at − 23.5° latitude." New
  `reference_line_hemisphere_location(line, hemisphere)` table decodes that span as its own rows:
  tropic_of_cancer → northern, tropic_of_capricorn → southern. NOT a duplicate of the
  already-shipped `reference_line_hemisphere_split(line, hemispheres)` table -- that table answers
  a different question ("which pair of hemispheres does this line DIVIDE", only equator/
  prime_meridian) while this table answers "which single hemisphere does this line SIT WITHIN"
  (only the two tropics) -- the two tables are disjoint over lines and disjoint in what they
  assert. No new WebFetch -- reuses the same already-cited NOAA NESDIS sentence. Honest abstention
  on equator and prime_meridian (their own cited spans state no single containing hemisphere) and
  on the polar circles (their own spans never name a hemisphere at all). New e2e test file
  `facts_referencelinehemispherelocation_e2e.rs` (3 tests: forward recall with citation, backward
  recall, honest abstention). No manifest objective, matching `reference-line-degree.adj`'s own
  precedent. Fifth and LAST slice of the geography/ domain sweep -- **GEOGRAPHY/ domain now FULLY
  EXHAUSTED**. 113th content slice overall.
