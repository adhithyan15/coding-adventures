- `language/possessive-noun.adj` (new) -- a new `possessive_noun(word, category)` table names
  three example nouns and which of the three possessive-noun categories each one's possessive
  form falls into (dog->singular_possessive, bottles->plural_possessive,
  geese->irregular_possessive), in a sentence that shows the possessive form in use, quoted
  verbatim from Grammarly's "Possessive Nouns: How to Use Them, With Examples" article --
  `trust consensus`, the same source family already used by
  `sentence-type.adj`/`part-of-speech.adj`/`contraction.adj`. Grounds CCSS L.2.2.c. Picked
  using the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE "\bpossessive\b"
  code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely fresh topic
  before this file was written. WebFetch-verified TWICE before writing. Uses apostrophe-free
  atom labels for the `word` column (ADJ atom labels cannot contain punctuation), mirroring
  `contraction.adj`'s established discipline for punctuation-bearing content; the header prose
  quotes the ORIGINAL punctuated example sentences so each mapping stays independently
  checkable. Honest abstention on "cat" (a real noun whose possessive is "cat's," but not one
  of these three tabled here). New manifest objective `adj.literacy.k2.possessive_noun` (band
  K-2, `recall` competency, `ccss.ela` coverage root). New e2e test
  `facts_possessivenoun_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  an untabled noun).
