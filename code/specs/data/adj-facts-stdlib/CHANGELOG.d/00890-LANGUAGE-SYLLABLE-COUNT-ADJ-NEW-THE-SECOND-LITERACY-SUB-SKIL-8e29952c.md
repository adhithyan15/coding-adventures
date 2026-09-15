- `language/syllable-count.adj` (new) — the SECOND literacy sub-skill library in the ADJ stdlib,
  deliberately DIFFERENT in shape from `word-families.adj`'s rhyme-family derivation: a genuinely
  new phonological-awareness skill (syllable segmentation, CCSS RF.K.2.b) rather than another word
  family (RF.K.2.a). A new `syllable_count(word, count)` table names how many syllables each of
  four words has, quoted verbatim from Reading Rockets' "Phonological and Phonemic Awareness: In
  Practice" module, which demonstrates syllable segmentation as a classroom technique (one index
  card placed per syllable while the teacher says each part aloud): peanut, pencil, sunset, and
  laptop, all explicitly segmented on the page. WebFetch-verified TWICE for consistency before
  writing -- the first pass over-eagerly attributed a syllable count to "classroom" that the
  second, more careful pass found was NOT actually syllable-segmented on the page (just used in an
  unrelated sentence), so it was correctly dropped. All four confirmed words happen to be
  two-syllable in this cited source (the page's demonstration does not segment a one- or
  three-syllable word), so the table is honestly narrow rather than fabricating contrast; a future
  slice can add variety once a comparably clean citation for a different count is found. Grounds
  CCSS RF.K.2.b. New manifest objective `adj.literacy.k2.syllable_count` (band K-2, `recall`
  competency -- a pure lookup, not a `rule`-derived fact, since no composition was needed or
  available here). New e2e test `facts_syllablecount_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an unshipped word).
