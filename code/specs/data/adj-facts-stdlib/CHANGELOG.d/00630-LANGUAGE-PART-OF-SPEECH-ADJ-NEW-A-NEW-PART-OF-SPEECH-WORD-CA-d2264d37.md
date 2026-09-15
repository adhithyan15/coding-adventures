- `language/part-of-speech.adj` (new) -- a new `part_of_speech(word, category)` table names
  three example words and which grammatical part of speech each one is (noun, verb,
  adjective), in a sentence that shows it doing that job, quoted verbatim from Grammarly's
  "The 8 Parts of Speech" article -- `trust consensus`, the same source family already used by
  `sentence-type.adj`. Grounds CCSS L.K.1.b. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "\bnoun\b|\bverb\b|\badjective\b|part_of_speech"
  code/specs/data/adj-facts-stdlib/` found only incidental prose hits (grammar descriptions in
  `past-tense-ed-sound.adj`/`plural-s-sound.adj` and unrelated adjective-as-word-choice usages
  elsewhere), confirming zero prior coverage of a word-to-part-of-speech classification before
  this file was written. WebFetch-verified TWICE before writing. Uses SHORT ATOM-STYLE labels
  for the `word` column, mirroring `sentence-type.adj`'s established discipline. Honest
  abstention on "slowly" (a real word, an adverb, but not one of the three parts of speech this
  table covers). New manifest objective `adj.literacy.k2.part_of_speech` (band K-2, `recall`
  competency, `ccss.ela` coverage root). New e2e test `facts_partofspeech_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an untabled word).
