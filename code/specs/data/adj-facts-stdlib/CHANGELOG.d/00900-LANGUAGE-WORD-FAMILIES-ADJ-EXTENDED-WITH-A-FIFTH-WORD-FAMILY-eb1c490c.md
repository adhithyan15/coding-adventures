- `language/word-families.adj` — extended with a FIFTH word family, "-og" (dog, hog, fog, log,
  jog), added as five new rows in the existing `word_family` table alongside "-an", "-at", "-ig",
  and "-ug". The existing `rhymes_with` rule is reused UNCHANGED for the fifth time running.
  Quoted verbatim from a THIRD Super Teacher Worksheets page (same site, same `consensus` trust
  tier, its own real citation): "Here is a collection of printable activities for young readers
  to learn about the 'og' family of words... Words included: clog, jog, dog, hog, frog, fog, and
  log." — WebFetch-verified twice for consistency, mirroring "-ig"/"-ug"'s bar. Deliberately
  excludes "clog" and "frog" (four-letter consonant-blend words) to preserve the strict
  three-letter CVC scope, the same discipline every prior family has used. IMPORTANT: this slice
  also fixed the library's own abstention worked-example and test, which had used "dog" as the
  "unshipped word" case since slice 1 -- now that "dog" is itself a real `-og` member, both
  `word-families.query.adj` and `facts_wordfamilies_e2e.rs`'s abstention case were switched to
  "cup" (still genuinely untabled). No new manifest objective needed -- extends the same
  already-covered library `adj.literacy.k2.rhyming_word_families`. New e2e test
  `rhymes_with_isolates_a_fifth_family_and_abstains_on_excluded_blend_words` (7th test in
  `facts_wordfamilies_e2e.rs`), which asserts NO cross-contamination with any of the four prior
  families AND honest abstention on "frog".
