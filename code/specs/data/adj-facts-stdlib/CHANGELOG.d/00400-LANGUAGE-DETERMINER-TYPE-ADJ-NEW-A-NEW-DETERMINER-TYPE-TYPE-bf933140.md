- `language/determiner-type.adj` (new) -- a new `determiner_type(type, description)` table
  names three determiner categories and what each actually does (article->precedes_a_noun_
  and_identifies_it_as_specific_or_nonspecific, demonstrative_determiner->communicates_the_
  placement_of_a_noun_in_space_or_time, distributive_determiner->refers_to_a_group_or_
  individual_parts_within_a_group), quoted verbatim from Grammarly's "What Are Determiners?"
  article -- `trust consensus`, the same tier the sibling `noun-type.adj`/`verb-type.adj`/
  `pronoun-type.adj`/`preposition-type.adj`/`conjunction-type.adj` (the other libraries in
  this directory) already use for their Grammarly citations. Grounds CCSS L.K.1.b/L.1.1.
  Picked using the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE
  "determiner_type|article|demonstrative_determiner|distributive_determiner|
  possessive_determiner" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a
  completely fresh topic before this file was written. WebFetch-verified before writing
  (twice -- the second pass specifically re-confirmed all three sentences were complete,
  with no truncation or additional clauses following them). Honest abstention on
  "possessive_determiner" (a real category the same page also names, but one whose own
  defining sentence bundles TWO separate facts -- that it is the possessive form of a
  personal pronoun, AND that it can appear before a noun -- plus a full inline list of
  examples, rather than one clean single-fact sentence like the three tabled here). New
  manifest objective `adj.literacy.k2.determiner_type` (band K-2, `recall` competency,
  `ccss.ela` coverage root). New e2e test `facts_determinertype_e2e.rs` (3 tests: direct
  recall, reverse binding, honest abstention on an untabled determiner type).
