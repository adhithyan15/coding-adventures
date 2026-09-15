- `language/noun-type.adj` (new) -- a new `noun_type(type, definition)` table names three
  noun types and what each actually is (common_noun->generic_name_of_an_item_in_a_class_or_group,
  collective_noun->denotes_a_group_or_collection_of_people_or_things,
  abstract_noun->cannot_be_perceived_by_the_senses), quoted verbatim from Grammarly's "Nouns:
  Definition and Examples" article -- `trust consensus`, the same source family already used
  by `sentence-type.adj`/`part-of-speech.adj`/`possessive-noun.adj`/
  `superlative-adjective-rule.adj`. Grounds CCSS L.1.1.b. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "\bnoun_type\b|\bcommon_noun\b|
  \bcollective_noun\b|\babstract_noun\b|\bconcrete_noun\b" code/specs/data/adj-facts-stdlib/`
  found ZERO hits, confirming a completely fresh topic before this file was written.
  WebFetch-verified before writing (twice). Honest abstention on "possessive_noun" (a real
  noun type the same page mentions and describes functionally, but for which it gives no
  single formal one-sentence definition the way it does for the three tabled here -- a
  distinct concept from the already-shipped `possessive-noun.adj`, whose
  `possessive_noun(word, category)` table classifies possessive-FORM examples, not the
  general definition of "what is a possessive noun"). New manifest objective
  `adj.literacy.k2.noun_type` (band K-2, `recall` competency, `ccss.ela` coverage root). New
  e2e test `facts_nountype_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on an untabled noun type).
