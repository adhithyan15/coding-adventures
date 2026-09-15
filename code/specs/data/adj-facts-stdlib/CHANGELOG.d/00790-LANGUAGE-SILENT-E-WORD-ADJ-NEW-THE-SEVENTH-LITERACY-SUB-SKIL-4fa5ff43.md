- `language/silent-e-word.adj` (new) -- the SEVENTH literacy sub-skill library, and the SECOND
  to move beyond CCSS RF.K.2, following `compound-word-spelling-example.adj`'s precedent into
  ANOTHER spelling pattern: the "silent e" / "magic e" (VCe) syllable type -- a long vowel
  spelled with a single letter, followed by a single consonant, followed by a silent "e". A new
  `silent_e_word(word, syllable_type)` table names the seven example words a primary source
  uses to teach this (wake, whale, while, yoke, yore, rude, hare), quoted verbatim from Reading
  Rockets' "Six Syllable Types" article, WebFetch-verified TWICE for consistency before writing.
  `trust consensus`, the same tier as the other Reading Rockets citations already shipped in
  this directory. Deliberately does NOT populate the manifest's `standards` field: CCSS RF.1.3.c
  ("Know final -e and common vowel team conventions for representing long vowel sounds") is a
  genuinely clean fit for this pattern (confirmed via two independent sources), but every
  objective in this stdlib so far describes its grounding standard in CHANGELOG/README prose
  rather than the manifest's `standards` array, so this library follows that same established
  convention rather than unilaterally breaking it. Honest abstention on "snake" (a real VCe
  word, but not one this source names). New manifest objective
  `adj.literacy.k2.silent_e_word` (band K-2, `recall` competency). New e2e test
  `facts_silentEword_e2e.rs` (3 tests: direct recall, reverse binding of all seven example
  words, honest abstention on an uncited VCe word).
