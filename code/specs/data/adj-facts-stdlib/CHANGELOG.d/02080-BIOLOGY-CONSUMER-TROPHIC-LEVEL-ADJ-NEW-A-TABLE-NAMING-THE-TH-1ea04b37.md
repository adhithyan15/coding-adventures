- `biology/consumer-trophic-level.adj` (new) — a `table` naming the three consumer trophic levels
  an ecosystem's food chain runs on (primary, secondary, tertiary consumer) and what each one eats,
  quoted verbatim from National Geographic Education's "Consumers" article (curl- and
  WebFetch-verified against the raw page HTML, a new source never before cited in this stdlib):
  `consumer_trophic_level(level, eats)`, primary_consumer → primary_producers, secondary_consumer →
  primary_consumers, tertiary_consumer → other_carnivores. This is the SECOND instance of the
  "ecosystems" Major Gap (ADJ-STDLIB-COVERAGE.md §5.1/§5.2), after `animal-habitat-definition.adj`,
  and the first to ground it with a fresh source rather than composing two already-shipped tables —
  a scoping pass across the three remaining single-instance K-8-science gaps (Earth-processes,
  observation/measurement, ecosystems) checked composition first (a full-tree literal-atom census
  restricted to each gap's relevant subject directories) and found every candidate pair either
  already-disqualified as a trivial column-split of siblings sharing ONE citation (e.g.
  `geology/fossil-preservation-subtype.adj` + `geology/fossil-preservation-type.adj`, and
  `geology/rock-type-formation-component.adj` + `earth-science/metamorphism-cause.adj`, both pairs
  decoding the SAME USGS/NPS sentence already used once), or too thin to generalize from (a single
  matching row). Grounds NGSS MS-LS2-3/5-LS2-1 ("matter and energy... among living... parts of an
  ecosystem") — the finer-grained "who eats whom" chain the already-shipped `food-chain-roles.adj`
  (NOAA, producer/consumer/decomposer) has no room for in its own flat `consumer` row. Honest
  abstention on `producer` and `decomposer` (the cited article's own structure keeps both OUTSIDE
  the three consumer trophic levels — its decomposer paragraph opens "In addition to consumers...");
  on `quaternary_consumer` (WebFetch-confirmed absent, two passes); and deliberately does NOT assert
  a numbered trophic-level rank, since the cited article's own body text ("the second trophic
  level") and its own embedded vocabulary glossary ("three" total positions, collapsing secondary
  and tertiary into one shared "third") disagree with each other on the count — encoding only the
  unambiguous "eats" relationship and abstaining on the level-number question entirely rather than
  picking a side. `trust consensus` (National Geographic Education, the same tier
  `animal-habitat-definition.adj`'s own `biome-type.adj` dependency already uses). New manifest
  objective `adj.science.6to8.consumer_trophic_level` (recall, NGSS, band 6-8). New e2e test
  `facts_consumertrophiclevel_e2e.rs` (3 tests: direct recall across all three levels, reverse
  binding, honest abstention on `decomposer`).
