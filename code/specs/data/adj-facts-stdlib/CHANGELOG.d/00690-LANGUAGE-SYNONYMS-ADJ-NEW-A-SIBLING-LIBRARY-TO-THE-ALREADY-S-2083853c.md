- `language/synonyms.adj` (new) -- a sibling library to the already-shipped `opposites.adj`
  (antonyms): a new `synonym(word, synonym)` table names three common words and a synonym of
  each (happy -> cheerful, smart -> bright, quick -> fast), quoted verbatim from the English
  Wiktionary entry for each word's own "Synonyms" line -- the SAME source family and `trust
  consensus` tier `opposites.adj` already established for antonyms. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -ril "synonym"
  code/specs/data/adj-facts-stdlib/` confirmed ZERO existing coverage before this file was
  written (an "antonyms" candidate was considered first and DROPPED once `opposites.adj` was
  discovered to already cover that ground). WebFetch-verified before writing. Only one direction
  is shipped per pair, mirroring `opposites.adj`'s own established convention. Honest abstention
  on "purple" (a real word, but with no shipped synonym in this table -- the same abstention
  example `opposites.adj` already uses). New manifest objective `adj.literacy.k2.synonym_pair`
  (band K-2, `recall` competency, `ccss.ela` coverage root). New e2e test `facts_synonyms_e2e.rs`
  (3 tests: direct recall, reverse binding, honest abstention on an untabled word).
