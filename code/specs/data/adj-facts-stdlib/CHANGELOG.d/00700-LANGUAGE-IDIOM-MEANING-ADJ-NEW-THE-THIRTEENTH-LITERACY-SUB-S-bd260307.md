- `language/idiom-meaning.adj` (new) -- the THIRTEENTH literacy sub-skill library, and a
  genuinely new figurative-language skill beyond CCSS RF.K.2's five parts and the
  phonics/spelling/whole-text/vocabulary slices already shipped: an idiom's literal words do NOT
  give its meaning. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -ril "idiom\|proverb" code/specs/data/adj-facts-stdlib/` confirmed ZERO existing coverage
  before this file was written. A new `idiom_meaning(idiom, meaning)` table names three common
  idioms and their meanings (piece_of_cake -> very_easy_to_do, break_the_ice ->
  start_a_conversation, under_the_weather -> feeling_slightly_ill), quoted verbatim from Oxford
  International English's "30 Useful English Idiomatic Expressions & Their Meanings" article,
  `trust consensus` -- the same tier as this stdlib's other non-.gov language sources (7ESL,
  Speakspeak). WebFetch-verified before writing. Honest abstention on
  "raining_cats_and_dogs" (a real, well-known idiom, but not one of these three tabled example
  idioms). New manifest objective `adj.literacy.3to5.idiom_meaning` (band 3-5 -- idioms are
  typically a CCSS L.3.5.b, grade 3+ skill, unlike most of this stdlib's other K-2 literacy
  slices -- `recall` competency, `ccss.ela` coverage root). New e2e test
  `facts_idiommeaning_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on an
  untabled idiom).
