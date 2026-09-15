- `language/onset-rime.adj` (extended) — extended the already-shipped `onset_rime(word, onset,
  rime)` table from 2 to 4 rows. Discovered via the SAME fresh WebFetch pass that surfaced
  `syllable-deletion.adj`: Reading Rockets' "Phonological and Phonemic Awareness: In Practice"
  module -- a page already cited by `syllable-count.adj`/`phoneme-substitution.adj`/
  `phoneme-deletion.adj`/`syllable-substitution.adj`/`syllable-deletion.adj` -- has a "Blending
  Onset and Rime" section (map -> m/ap) and an "Onset-rime Completion" section (tape -> t/ape),
  each naming a word/onset/rime triple in the EXACT shape `onset-rime.adj` already carries (a
  header-revisit-style discovery, but from a DIFFERENT already-cited page than the one the
  original two rows came from, rather than the same table's own header). Both new rows are a
  pure addition; the original sleep/blast rows are untouched. WebFetch-verified THREE separate
  times for consistency before writing, all byte-identical (the third pass surfaced an even
  cleaner clean-prose sentence for `map`: "So in the word 'map,' /m/ is the onset and /ap/ is
  the rime."). New e2e test `onset_rime_extension_recalls_the_newly_added_map_and_tape_splits`.
  No new manifest objective (same library, same objective, 169 total unchanged).
