- `geography/reference-line-hemisphere-split.adj` (new) — a sibling to the already-shipped
  `reference-lines.adj` (`reference_line(line, marks)`, ONE degree-marking property per line:
  equator, prime_meridian, tropic_of_cancer, tropic_of_capricorn, arctic_circle,
  antarctic_circle). That table's own `source` and `cites` fields already quote a SECOND fact,
  verbatim, for two of the six lines — which pair of hemispheres each line divides the Earth
  into — that the marks-only schema had no room for: the equator's cited NOAA "What is
  latitude?" source states "it equally divides the Earth into the Northern and Southern
  hemispheres," and the prime meridian's cited NOAA "What is longitude?" corroboration states
  "It divides the Earth into the eastern and western hemispheres." New
  `reference_line_hemisphere_split(line, hemispheres)` table: equator → northern_southern,
  prime_meridian → eastern_western. Honest abstention on the two tropics and two polar circles,
  whose own cited spans state a latitude limit or polar ring, not a hemisphere split. New e2e
  test file `facts_referencelinehemispheresplit_e2e.rs` (3 tests: forward recall with citation,
  backward recall from a bound hemisphere pair, honest abstention on tropic_of_cancer). No
  manifest objective, matching `reference-lines.adj`'s own precedent of not having one. First
  slice from a fresh geography/ sweep (mathematics/ swept and closed empty first — see below).
