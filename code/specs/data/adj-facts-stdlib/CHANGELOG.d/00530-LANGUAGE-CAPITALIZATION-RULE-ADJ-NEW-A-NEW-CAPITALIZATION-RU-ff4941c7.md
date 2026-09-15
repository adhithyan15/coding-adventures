- `language/capitalization-rule.adj` (new) -- a new `capitalization_rule(rule, description)`
  table names three common English capitalization rules and what each actually requires
  (first_word_of_sentence->capitalize_first_letter, pronoun_i->capitalized_anywhere_in_sentence,
  proper_noun->capitalized_regardless_of_position), quoted verbatim from Grammarly's
  "Capitalization Rules and Examples" article -- `trust consensus`, the same source family
  already used by `sentence-type.adj`/`part-of-speech.adj`/`contraction.adj`/
  `possessive-noun.adj`/`simile-meaning.adj`/`prefix-meaning.adj`. Grounds CCSS L.K.2.a. Picked
  using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\bcapitali[sz]" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a
  completely fresh topic before this file was written. WebFetch-verified before writing (twice,
  across two cycles of this loop). Honest abstention on "quotation" (a real capitalization rule
  the same article covers in its "Capitalization and quotes" section, but not one of these three
  tabled here). New manifest objective `adj.literacy.k2.capitalization_rule` (band K-2, matching
  `sentence-type.adj`'s band, `recall` competency, `ccss.ela` coverage root). New e2e test
  `facts_capitalizationrule_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention
  on an untabled rule).
