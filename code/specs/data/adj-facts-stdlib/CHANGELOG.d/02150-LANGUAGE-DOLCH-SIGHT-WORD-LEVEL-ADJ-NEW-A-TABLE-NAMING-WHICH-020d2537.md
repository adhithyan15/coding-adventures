- `language/dolch-sight-word-level.adj` (new) — a `table` naming which of Edward W. Dolch's five
  grade-banded reading levels (Pre-Primer, Primer, First Grade, Second Grade, Third Grade) a
  common high-frequency "sight word" is first taught at: `dolch_sight_word_level(word, level)`,
  25 rows (the first five words of each level, in the source deck's own listed order). This is
  the THIRD fresh-WebFetch-sourced literacy instance shipped under `wave2-k8-literacy-foundations`,
  and genuinely distinct in KIND from `digraph-sound.adj`/`diphthong-sound.adj` (both phonics:
  a SPELLING → the SOUND it makes) — this is whole-word-recognition vocabulary instead (a WORD →
  the reading-level BAND it is first taught in), the opposite reading strategy from phonics
  decoding. Confirmed via full-tree grep that no shipped table anywhere already names `dolch` or
  `sight word`/`sight_word`. Sourced from the University of Florida Literacy Institute (UFLI)'s
  Virtual Teaching Resource Hub "Irregular and High Frequency Words" page (curl-fetched and
  confirmed byte-for-byte), which links UFLI's own "Dolch High Frequency Word List Slides"
  PowerPoint deck — downloaded directly from the same ufli.education.ufl.edu `trust authoritative`
  source family `digraph-sound.adj`/`diphthong-sound.adj` already established, and unzipped as raw
  OOXML to read each level-divider and word slide's own text run verbatim. Counting the word slides
  between each pair of level dividers gives exactly 40/52/41/46/41 (Pre-Primer/Primer/First
  Grade/Second Grade/Third Grade), summing to exactly 220 — an independent cross-check that this
  deck faithfully reproduces Dolch's own well-documented 220-word count. Ships 25 of the 220 words
  (the first five per level in the deck's own order), the same "representative subset" convention
  `food-groups.adj` already established (4-5 "example" items per category rather than the full
  source list). Two UFLI Foundations Toolbox blends-unit and Long-Vowel-Teams angles were also
  scoped this round before landing here — see below. Empirically verified reserved-keyword-shaped
  atoms (`to`, `and`, `if`) against the real built CLI binary in a scratch table BEFORE writing the
  shipped file, since the `adj-lang` lexer reserves `to`/`and`/`for` as grammar keywords elsewhere
  in the language; confirmed they parse fine as plain atoms in `row(...)` position (this stdlib's
  `homophones.adj` already had one precedent, `row (to, too)`). Honest abstention on `you` (a REAL
  Dolch Pre-Primer word, but outside this table's shipped first-five-per-level subset) and on
  `elephant` (not a Dolch word at all). New e2e test `facts_dolchsightwordlevel_e2e.rs` (5 tests);
  new manifest objective `adj.literacy.k2.dolch_sight_word_level` (`recall` competency).

