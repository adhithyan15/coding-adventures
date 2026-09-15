- `language/homophones.adj` (new) -- a sibling library to the already-shipped `opposites.adj`
  (antonyms) and `synonyms.adj`: a new `homophone(word, sound_alike)` table names three common
  words and a word that sounds the same but is spelled/means differently (there -> their,
  flower -> flour, to -> too), quoted verbatim from the English Wiktionary entry for each word's
  own "Homophones" line -- the SAME source family and `trust consensus` tier `opposites.adj`/
  `synonyms.adj` already established. Picked using the mandatory full-tree-grep-before-scoping
  discipline -- `grep -ril "homophone\|homonym" code/specs/data/adj-facts-stdlib/` confirmed
  ZERO existing coverage before this file was written. WebFetch-verified before writing. Only
  one direction is shipped per pair, mirroring `opposites.adj`'s and `synonyms.adj`'s own
  established convention. Honest abstention on "here" (a real word with a real homophone "hear",
  but not one this table carries). New manifest objective `adj.literacy.k2.homophone_pair`
  (band K-2, `recall` competency, `ccss.ela` coverage root). New e2e test
  `facts_homophones_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on an
  untabled word).
