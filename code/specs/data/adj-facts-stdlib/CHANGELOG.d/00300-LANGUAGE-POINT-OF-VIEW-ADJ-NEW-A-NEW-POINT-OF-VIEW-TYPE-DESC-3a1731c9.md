- `language/point-of-view.adj` (new) -- a new `point_of_view(type, description)` table names
  three narrative PERSPECTIVES a story can be told from
  (first_person->the_reader_accesses_the_story_through_one_person,
  second_person->uses_the_pronoun_you,
  third_person->the_narrator_has_the_ability_to_know_everything), quoted verbatim from
  Grammarly's "What Is Point of View in Writing, and How Does It Work?" article -- `trust
  consensus`, the same tier `pronoun-type.adj`/`sentence-type.adj`/`part-of-speech.adj` (the
  other libraries in this directory) already use for their Grammarly citations. Point of view
  is a LITERARY DEVICE, distinct from the sibling `pronoun-type.adj`, which tables
  grammatical pronoun CATEGORIES (personal/indefinite/interrogative) rather than narrative
  perspective -- the two libraries answer different questions and neither overlaps the
  other. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -riE "point_of_view|first_person|third_person|narrator"` across `adj-facts-stdlib/`
  found zero table-level hits, only an unrelated prose mention of "narrator" in
  `fable-moral.adj`. WebFetch-verified twice -- the second pass pulled the full opening
  paragraph for `first_person` and `third_person`, confirming each quoted sentence is the
  article's own first, complete, standalone defining sentence (the elaboration that follows
  each one is a separate sentence, not folded into the quoted definition). Honest abstention
  on `third_person_omniscient`: a real, well-documented term the same article defines with
  its own clean sentence, but the article itself frames it as a SUBTYPE of third person
  (alongside third_person_limited and third_person_objective), not a fourth peer point of
  view. New manifest objective `adj.literacy.k2.point_of_view` (band K-2, `recall`
  competency, `ccss.ela` coverage root). New e2e test `facts_pointofview_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an untabled subtype).
