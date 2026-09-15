- `language/simile-meaning.adj` (new) -- a new `simile_meaning(simile, meaning)` table names
  three common similes and what each actually means (as_brave_as_a_lion->extremely_courageous,
  like_a_needle_in_a_haystack->very_difficult_to_find, as_free_as_a_bird->free_or_unrestricted),
  quoted verbatim from Grammarly's "Simile: Definition and Examples" article's "Common simile
  examples" table -- `trust consensus`, the same source family already used by
  `sentence-type.adj`/`part-of-speech.adj`/`contraction.adj`/`possessive-noun.adj`. A sibling
  figurative-language library to `idiom-meaning.adj`, using the same band (3-5) and the same
  apostrophe/punctuation-free underscore-joined atom-label discipline for multi-word phrases.
  Grounds CCSS L.5.5.a. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\bsimile\b" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a
  completely fresh topic before this file was written (only an incidental "figurative-language"
  prose mention in `idiom-meaning.adj`'s own header, not an actual simile table). WebFetch-verified
  before writing. Honest abstention on "as_busy_as_a_bee" (a real, well-known simile, but not one
  of these three tabled here). New manifest objective `adj.literacy.3to5.simile_meaning` (band
  3-5, `recall` competency, `ccss.ela` coverage root). New e2e test
  `facts_simile_meaning_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  an untabled simile).
