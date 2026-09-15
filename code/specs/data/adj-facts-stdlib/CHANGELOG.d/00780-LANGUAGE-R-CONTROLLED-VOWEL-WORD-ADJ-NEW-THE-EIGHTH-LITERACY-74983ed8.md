- `language/r-controlled-vowel-word.adj` (new) -- the EIGHTH literacy sub-skill library, and the
  THIRD to move beyond CCSS RF.K.2, following `compound-word-spelling-example.adj`'s and
  `silent-e-word.adj`'s precedent into a phonics pattern: "r-controlled vowels" (aka "bossy r"),
  where a vowel followed by "r" no longer makes its expected sound. A new
  `r_controlled_vowel_word(word, pattern)` table names five example words and the r-controlled
  digraph in each (barn -> ar, corn -> or, fern -> er, bird -> ir, curl -> ur), quoted verbatim
  from the University of Florida Literacy Institute (UFLI)'s phonics foundations toolbox: "There
  are three main r-controlled vowel sounds: the /ar/ sound, as in barn; the /or/ sound, as in
  corn; and the /er/ sound, as in fern, bird, and curl." WebFetch-verified TWICE for consistency
  before writing (two independent fetches of the same page). `trust authoritative` -- UFLI is a
  university literacy research center (University of Florida, .edu), a primary academic source.
  DESIGN NOTE: the source groups fern/bird/curl under ONE phonetic label ("/er/ sound") despite
  three different spellings (er/ir/ur) -- `pattern` here is the LITERAL r-controlled digraph
  objectively present in each word's own spelling, NOT an assertion that the source itself
  distinguished er/ir/ur as separate categories (it did not), the same discipline
  `word-families.adj`'s `family` column already established for naming letters-in-the-word
  rather than a source-stated grouping. Honest abstention on "star" (a real ar-pattern word, but
  not one this source names). New manifest objective `adj.literacy.k2.r_controlled_vowel_word`
  (band K-2, `recall` competency). New e2e test `facts_rcontrolledvowelword_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an uncited word).
