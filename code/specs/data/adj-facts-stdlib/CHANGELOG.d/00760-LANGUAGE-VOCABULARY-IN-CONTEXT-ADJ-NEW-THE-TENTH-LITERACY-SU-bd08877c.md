- `language/vocabulary-in-context.adj` (new) -- the TENTH literacy sub-skill library. A new
  `vocabulary_in_context(word, meaning)` table names three vocabulary words whose meaning a
  primary source teaches via a worked context-clue example sentence (ornithology ->
  scientific_study_of_birds, sentence: "People who study birds are experts in ornithology.";
  frugivorous -> eats_fruit_as_primary_food, sentence: "Frugivorous birds prefer eating fruit to
  any other kind of food."; inconspicuous -> hidden_or_not_easily_seen, sentence: "Some birds
  like to build their nests in inconspicuous spots -- high up in the tops of trees, well hidden
  by leaves."), quoted verbatim from Reading Rockets' "Using Context Clues to Understand Word
  Meanings" article, `trust consensus` -- the same tier as the other Reading Rockets citations
  already shipped in this directory. DESIGN NOTE: `meaning` is a short constant-style label
  rather than a full-sentence definition -- `fable-moral.adj`'s grammar discovery found that a
  quoted-string literal works as a `table` row VALUE but not as a query ARGUMENT, so using a
  short atom here (unlike `fable-moral.adj`'s sentence-valued `moral` column) keeps BOTH the
  direct and reverse queries usable as ordinary ground-argument binding queries. Honest
  abstention on "arboreal" (a real vocabulary word, but not one this source defines). New
  manifest objective `adj.literacy.k2.vocabulary_in_context` (band K-2, `recall` competency).
  New e2e test `facts_vocabularyincontext_e2e.rs` (3 tests: direct recall, reverse binding,
  honest abstention on an undefined word).
