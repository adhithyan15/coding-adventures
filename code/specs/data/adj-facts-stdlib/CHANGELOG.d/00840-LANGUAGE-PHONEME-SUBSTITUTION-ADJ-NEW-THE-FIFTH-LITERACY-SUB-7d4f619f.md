- `language/phoneme-substitution.adj` (new) -- the FIFTH literacy sub-skill library in the ADJ
  stdlib, completing coverage of all five named parts of CCSS RF.K.2. Deliberately different in
  shape from all four prior ones (rhyme derivation RF.K.2.a, syllable count RF.K.2.b, onset/rime
  RF.K.2.c, initial sound RF.K.2.d) -- this recalls what happens when you SUBSTITUTE one sound in
  a word for another, grounding RF.K.2.e ("Add or substitute individual sounds in simple,
  one-syllable words to make new words") as a pure lookup,
  `phoneme_substitution(original_word, original_sound, new_sound, new_word)`, a FOUR-column
  table. Quoted verbatim from Reading Rockets' "Phonological and Phonemic Awareness: In Practice"
  module (the SAME page `syllable-count.adj` already cites, a different section), WebFetch-
  verified TWICE for consistency before writing: "I can change one sound in a word to form a new
  word. Watch me. I will change 'make' to 'bake'." and "The first sound in make is /m/. The first
  sound in bake is /b/." Deliberately scoped to ONLY this ONE substitution the cited page walks
  through step by step -- honestly narrow (mirroring `onset-rime.adj`'s and `initial-sound.adj`'s
  precedent) rather than inventing a substitution the source does not demonstrate. New manifest
  objective `adj.literacy.k2.phoneme_substitution` (band K-2, `recall` competency). New e2e test
  `facts_phonemesubstitution_e2e.rs` (3 tests: direct recall of the new word, reverse binding of
  the original word/sound, honest abstention on an untabled substitution).
