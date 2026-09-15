- `geography/reference-line-degree.adj` (new) — a sibling to the already-shipped `reference-lines.adj`
  (`reference_line(line, marks)`, tropic_of_cancer → northernmost_sun_overhead,
  tropic_of_capricorn → southernmost_sun_overhead). That table's own already-quoted NOAA NESDIS
  span also states the precise signed numeric latitude of each tropic -- a fact the line/marks
  schema had no room for: "...Tropic of Cancer at +23.5° latitude and...Tropic of Capricorn at
  − 23.5° latitude." New `reference_line_degree(line, degrees)` table decodes that span as its own
  rows: tropic_of_cancer → 23.5, tropic_of_capricorn → -23.5 (negative/decimal number literals,
  confirmed supported by the ADJ grammar and already proven in other shipped tables e.g.
  `metrology/metric-prefixes.adj`, `physics/physical-constants.adj`). No new WebFetch -- reuses the
  same already-cited NOAA NESDIS sentence. Honest abstention on `equator`/`prime_meridian`, whose
  own 0-degree fact is already fully captured by their existing `marks` atoms
  (`zero_degrees_latitude`/`zero_degrees_longitude`) rather than left undecoded, and on
  `arctic_circle`/`antarctic_circle`, whose own quotes never state a numeric degree at all. New e2e
  test file `facts_referencelinedegree_e2e.rs` (3 tests: forward recall with citation, backward
  recall on a negative-number bind, honest abstention). No manifest objective, matching
  `reference-lines.adj`'s own precedent. First slice of the geography/ domain sweep (a fresh,
  never-before-swept domain this session — 8 existing tables, background Explore-agent sweep found
  4 STRONG + 3 MODERATE further candidates). 109th content slice overall.
