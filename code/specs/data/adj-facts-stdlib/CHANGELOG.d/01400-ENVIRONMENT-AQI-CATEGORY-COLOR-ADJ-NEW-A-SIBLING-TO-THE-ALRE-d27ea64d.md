- `environment/aqi-category-color.adj` (new) — a sibling to the already-shipped
  `air-quality-index.adj` (`air_quality_index(min_aqi, category)`, a RANGE/BRACKET lookup keyed by
  the numeric breakpoint of each of the six EPA AQI bands). That table's own per-row provenance
  already quotes, verbatim, a leading color word for every one of its six rows — the parent's own
  header even lays this out explicitly as a "source's colour" column in its truth table, shown
  "only to make the step visible" but never materialized as a row — that the `min_aqi, category`
  schema had no room for: good → green, moderate → yellow, unhealthy_for_sensitive_groups →
  orange, unhealthy → red, very_unhealthy → purple, hazardous → maroon. New
  `aqi_category_color(category, color)` table covers the full six-category domain with no
  abstention, since every category's cited span names a color. New e2e test file
  `facts_aqicategorycolor_e2e.rs` (3 tests: forward recall with citation, backward recall from a
  bound color, full-domain coverage with no abstention). No manifest objective, matching
  `air-quality-index.adj`'s own precedent of not having one. First and only strong candidate from
  the environment/ sweep tranche (environment/'s other table, `ecosystem-factor-type.adj`, was
  checked and confirmed to have nothing further viable); environment/ (2 tables) is now
  effectively closed after this ships.
