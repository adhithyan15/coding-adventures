- `language/phoneme-blending.adj` (new) — a new NINTH literacy sub-skill library:
  `phoneme_blending(sound_one, sound_two, sound_three, word)`, the OPPOSITE direction from
  `phoneme-deletion.adj`/`phoneme-substitution.adj` (which decompose or swap sounds in a word):
  this recalls the word formed by BLENDING three separate sounds together. One row: (s, o, p,
  soap). Discovered by re-visiting the SAME already-vetted Reading Rockets "In Practice" page
  (already cited by `syllable-count.adj`/`phoneme-substitution.adj`/`phoneme-deletion.adj`/
  `syllable-substitution.adj`/`syllable-deletion.adj`/`onset-rime.adj`) for its "Blending sounds"
  section, distinct from its "Adding sounds" section (which blends only TWO sounds, a different
  arity, left as a future `phoneme-addition.adj` candidate). WebFetch-verified THREE separate
  times for consistency before writing, all byte-identical. New e2e test file
  `facts_phonemeblending_e2e.rs` (3 tests: direct recall, reverse binding into all three sounds,
  honest abstention on an untabled blend). New manifest objective
  `adj.literacy.k2.phoneme_blending` (169 -> 170 total).
