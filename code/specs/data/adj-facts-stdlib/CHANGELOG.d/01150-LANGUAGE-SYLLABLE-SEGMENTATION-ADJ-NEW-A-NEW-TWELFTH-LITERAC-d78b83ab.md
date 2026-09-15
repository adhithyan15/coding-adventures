- `language/syllable-segmentation.adj` (new) — a new TWELFTH literacy sub-skill library, a sibling
  to the already-shipped `syllable-count.adj` (which recalls only HOW MANY syllables a word has):
  `syllable_segmentation(word, syllable_one, syllable_two)` names the actual two parts each word
  breaks into. Discovered via a header-revisit: `syllable-count.adj`'s own header already quotes,
  verbatim, the exact syllable split for all four of its words ("pea"/"nut", "pen"/"cil",
  "sun"/"set", "lap"/"top"), but that table's `syllable_count(word, count)` schema has no room for
  the actual syllable text -- the same "sibling table, different schema" shape already used for
  `comet-part.adj`/`comet-tail-type.adj`, `lung-lobes.adj`/`lung-lobe-names.adj`, and
  `chemical-bonds.adj`/`chemical-bond-family.adj`. Four rows: peanut/pea/nut, pencil/pen/cil,
  sunset/sun/set, laptop/lap/top. WebFetch-verified TWICE more this cycle for consistency, both
  byte-identical to `syllable-count.adj`'s existing quotes. New e2e test file
  `facts_syllablesegmentation_e2e.rs` (4 tests: direct recall, reverse binding, coverage of all
  four words, honest abstention on an untabled word). New manifest objective
  `adj.literacy.k2.syllable_segmentation` (171 -> 172 total, matching `syllable-count.adj`'s own
  precedent of having a manifest objective).
