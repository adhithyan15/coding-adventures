- `language/phoneme-deletion.adj` (new) -- a new `phoneme_deletion(original_word,
  removed_sound, new_word)` table names the one phoneme-deletion demonstration
  (bike->by, removing the last sound /k/) walked through on Reading Rockets'
  "Phonological and Phonemic Awareness: In Practice" module -- the SAME
  already-vetted page `syllable-count.adj` and `phoneme-substitution.adj`
  already cite (a different, "Deleting Sounds" section of it), so this slice
  carries zero new sourcing risk. `trust consensus`. The row composes two
  distinct sentences from that section ("I will change 'bike' to 'by'." +
  "The last sound in 'bike' is /k/.") the same way `phoneme-substitution.adj`'s
  own row already does. Also checked the same page's "suntan"->"sunset"
  demonstration before writing this table: that example substitutes a whole
  SYLLABLE, not a single phoneme, so it is a genuinely different skill and was
  deliberately left out (a future candidate for its own syllable-substitution
  table). New manifest objective `adj.literacy.k2.phoneme_deletion` (165
  objectives total, up from 164). New e2e test `facts_phonemedeletion_e2e.rs`
  (3 tests: direct recall, reverse binding, honest abstention).
