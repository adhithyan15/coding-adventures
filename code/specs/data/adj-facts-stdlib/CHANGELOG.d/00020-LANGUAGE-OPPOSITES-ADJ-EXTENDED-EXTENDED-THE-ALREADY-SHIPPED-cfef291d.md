- `language/opposites.adj` (extended) -- extended the already-shipped
  `opposite(word, opposite)` table from 7 to 21 rows. The header ABOVE
  already quoted the full Wiktionary "Antonyms" line for `hot`, `big`,
  `happy`, and `open` when this table originally shipped -- only the
  FIRST antonym in each of those four lines had ever been turned into a
  row, even though the same already-quoted span names one more (hot:
  chilled), five more (big: little, tiny, minuscule, miniature,
  minute), seven more (happy: blue, depressed, down, miserable, moody,
  morose, unhappy), and one more (open: shut). `fast`, `wet`, and `hard`
  were already fully captured (their sources name only one antonym
  each). This is the EIGHTH successful extend-pattern win in this
  loop's recent run. WebFetch re-verified all seven Wiktionary
  "Antonyms" lines live before writing (confirming each already-quoted
  span is accurate and unchanged, and specifically that `big`'s header
  ellipsis stopped at exactly six total antonyms). Mirrors
  `synonyms.adj`'s single-to-many-valued-per-key extension shape (a
  bound word now recalls multiple opposites) -- the sibling table this
  extension technique was first proven on. Extended the query file and
  e2e test `facts_opposites_e2e.rs` to 2 tests (original recall/abstain
  test + a new extension test covering three of the newly multi-valued
  words). No new manifest objective (same library, same objective, 167
  total unchanged). Also backfilled a pre-existing README.md
  documentation gap: `opposites.adj` had never been added to the
  per-directory documentation table (the THIRD such gap found this
  window, after `kingdoms.adj` and `animal-babies.adj`).
