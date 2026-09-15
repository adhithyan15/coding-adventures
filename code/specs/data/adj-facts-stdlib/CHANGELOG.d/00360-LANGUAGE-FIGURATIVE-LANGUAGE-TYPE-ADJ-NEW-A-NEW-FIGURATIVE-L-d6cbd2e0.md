- `language/figurative-language-type.adj` (new) -- a new `figurative_language_type(type,
  description)` table names three figures of speech and what each actually does
  (metaphor->describes_something_in_a_way_thats_not_literally_true_to_make_a_comparison,
  personification->gives_human_characteristics_to_nonhuman_or_abstract_things,
  hyperbole->a_great_exaggeration_used_to_add_emphasis), quoted verbatim from Grammarly's
  "Figurative Language Examples: 6 Common Types and Definitions" article -- `trust
  consensus`, the same tier the sibling `noun-type.adj`/`verb-type.adj`/`pronoun-type.adj`/
  `preposition-type.adj`/`conjunction-type.adj`/`determiner-type.adj`/
  `end-punctuation-mark.adj` (the other libraries in this directory) already use for their
  Grammarly citations. Grounds CCSS L.5.5a. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE
  "figurative_language|metaphor|personification|hyperbole|allusion"
  code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely fresh topic
  before this file was written. WebFetch-verified before writing (twice -- the second pass
  specifically confirmed each sentence's exact wording and that it stands alone as a
  complete definition before any follow-up example sentence). Honest abstention on
  "allusion" (a real device the same page also names with its own clean defining sentence,
  but one that works by referencing an external work, person, or event rather than by
  comparison, exaggeration, or personification -- a different rhetorical mechanism than the
  three tabled here). Simile and idiom, also named on the same page, are deliberately
  excluded too: both are already grounded as their own separately-shipped libraries in this
  directory (`simile-meaning.adj`, `idiom-meaning.adj`), so tabling them again here under a
  different predicate would duplicate coverage rather than add to it. New manifest objective
  `adj.literacy.k2.figurative_language_type` (band K-2, `recall` competency, `ccss.ela`
  coverage root). New e2e test `facts_figurativelanguagetype_e2e.rs` (3 tests: direct
  recall, reverse binding, honest abstention on an untabled figure of speech).
