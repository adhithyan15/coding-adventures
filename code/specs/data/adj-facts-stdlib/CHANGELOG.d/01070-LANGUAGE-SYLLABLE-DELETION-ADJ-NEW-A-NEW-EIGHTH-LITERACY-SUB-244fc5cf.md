- `language/syllable-deletion.adj` (new) — a new EIGHTH literacy sub-skill library:
  `syllable_deletion(original_word, removed_syllable, new_word)`, the syllable-level analogue of
  `phoneme-deletion.adj` the same way `syllable-substitution.adj` is the syllable-level analogue
  of `phoneme-substitution.adj`. Discovered while scoping the next literacy slice: all five
  previously-scoped literacy candidates (syllable-count.adj, onset-rime.adj, initial-sound.adj,
  phoneme-substitution.adj, possessive-noun.adj) turned out to be honestly narrow-by-design with
  no unused header material -- a fresh WebFetch of the SAME already-vetted Reading Rockets "In
  Practice" page (`syllable-count.adj`/`phoneme-substitution.adj`/`phoneme-deletion.adj`/
  `syllable-substitution.adj` all already cite it) surfaced its "Deleting syllables" section,
  distinct from the "Deleting Sounds" section `phoneme-deletion.adj` draws from: `row(pencil,
  cil, pen)`. WebFetch-verified THREE separate times for consistency before writing, all
  byte-identical. New library, new e2e test file `facts_syllabledeletion_e2e.rs` (3 tests: direct
  recall, reverse binding, honest abstention on an untabled deletion). New manifest objective
  `adj.literacy.k2.syllable_deletion` (168 -> 169 total).
