- `language/text-structure-type.adj` (new) -- a new `text_structure_type(type,
  description)` table names three ways a nonfiction text organizes its
  information (cause_and_effect->tells_why_something_happened_and_what_happened,
  compare_and_contrast->examines_the_similarities_and_differences_between_two_or_more_things,
  description->describes_a_topic_to_give_the_reader_a_mental_picture), each
  quoted verbatim from its own standalone sentence in Reading Rockets'
  "Teaching Text Structure" article -- `trust consensus`, the same tier
  this stdlib already reserves for other Reading Rockets citations (e.g.
  `word-families.adj`, `vocabulary-in-context.adj`). Picked after checking
  7 not-yet-reviewed literacy tables this window (opposites.adj,
  vowels.adj, word-families.adj, alphabet.adj, greek-alphabet.adj -- none
  extendable, each already exhaustive or deliberately CVC-scoped), then
  researching text structure as a fresh topic. Honest abstention on
  `sequence`: a real text structure the same article also names, but its
  defining sentence joins two distinct functions with "or" ("describes
  items or events in order, OR explains the steps to follow") rather than
  stating one clean fact; also honest abstention on `problem_and_solution`
  for the same reason (its sentence bundles three structural components in
  sequence). New manifest objective `adj.literacy.k2.text_structure_type`
  (163 objectives total, up from 162). New e2e test
  `facts_textstructuretype_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention).
