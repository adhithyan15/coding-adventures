- `language/adverb-type.adj` (new) -- a new `adverb_type(type, description)` table names
  four adverb types and what each actually describes (manner->describes_how_an_action_is_
  performed, place->describes_where_an_action_happens, frequency->describes_how_often_an_
  action_occurs, duration->describes_how_long_an_action_lasts), quoted verbatim from
  Grammarly's "What Is an Adverb? Definition and Examples" article's "Types of adverbs"
  table -- `trust consensus`, the same tier already used by the sibling `verb-type.adj`/
  `sentence-type.adj`/`part-of-speech.adj`/`noun-type.adj`/`preposition-type.adj`. Closes
  out adverbs as the last major part-of-speech family this stdlib had not yet named on its
  own. Picked using the mandatory full-tree-grep-before-scoping discipline -- zero hits for
  `adverb_type` before writing. WebFetch-verified twice -- the second pass pulled every row
  of the source's "Types of adverbs" table, confirming manner/place/frequency/duration are
  each stated as their own clean, single-fact sentence. Honest abstention on `time`: the
  SAME table names a fifth adverb type, but its own defining sentence -- "Adverbs of time
  describe when, how long, or how often something happens" -- bundles three distinct facts
  into one sentence rather than stating a single clean fact, the same "reject bundled
  facts" discipline reinforced across recent slices (fossil-preservation-type, lunar-
  eclipse-type, comma-rule, sun-layer). New manifest objective `adj.literacy.k2.
  adverb_type` (band K-2, `recall` competency, `ccss.ela` coverage root, matching the
  sibling `*_type` part-of-speech objectives' band convention). New e2e test
  `facts_adverbtype_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  a real-but-bundled type).
