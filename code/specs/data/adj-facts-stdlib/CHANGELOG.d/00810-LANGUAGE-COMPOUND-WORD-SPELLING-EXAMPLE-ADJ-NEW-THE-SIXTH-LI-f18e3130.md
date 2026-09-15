- `language/compound-word-spelling-example.adj` (new) -- the SIXTH literacy sub-skill library,
  and the FIRST to move beyond CCSS RF.K.2 (all five named parts of which -- rhyming, syllable
  count, onset/rime, initial sound, phoneme substitution -- are now shipped). Grounds a SPELLING
  pattern instead of a phonological-awareness one: teaching a beginner to spell multisyllable
  words is easier when the word is a compound built from two words the learner can already
  spell. A new `compound_word_spelling_example(word, teaching_use)` table names the four
  compound words a primary source uses to teach this (catfish, hotdog, playground, yellowtail),
  quoted verbatim from Reading Rockets' "How Spelling Supports Reading" article, WebFetch-
  verified TWICE for consistency before writing. `trust consensus`, the same tier as the other
  Reading Rockets citations already shipped in this directory. Deliberately gives the table a
  genuine second column (`teaching_use`, a constant label for every row) rather than a bare
  `columns word` -- an earlier draft with a single-column table was empirically verified in a
  scratch dir to NOT produce ordinary `recall`/`abstained` query semantics on a fully-ground
  query (the engine instead falls back to its hypothesis-ranking/adjudication mode), so every
  table in this stdlib should keep at least two genuine columns even when the second is a
  constant. Deliberately does NOT cite a specific CCSS standard code: the closest candidates
  (RF.1.3.e general phonics, L.2.4.d compound-word MEANING prediction) both describe a different
  skill than what this source supports (spelling ease via compound decomposition, not decoding
  or meaning), so `standards` stays empty rather than force-citing a mismatched code. Honest
  abstention on "cupcake" (a real compound word, but not one this source names). New manifest
  objective `adj.literacy.k2.compound_word_spelling_example` (band K-2, `recall` competency).
  New e2e test `facts_compoundwordspellingexample_e2e.rs` (3 tests: direct recall, reverse
  binding of all four example words, honest abstention on an uncited compound).
