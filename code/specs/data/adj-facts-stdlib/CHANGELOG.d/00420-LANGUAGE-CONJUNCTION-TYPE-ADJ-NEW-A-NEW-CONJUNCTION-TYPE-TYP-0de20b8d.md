- `language/conjunction-type.adj` (new) -- a new `conjunction_type(type, description)` table
  names three categories of conjunction and what each actually does
  (coordinating_conjunction->joins_words_phrases_and_clauses_of_equal_grammatical_rank,
  correlative_conjunction->are_pairs_of_conjunctions_that_work_together,
  subordinating_conjunction->joins_dependent_clauses_to_independent_clauses), quoted verbatim
  from Grammarly's "Conjunctions" article -- `trust consensus`, the same source family already
  used by `noun-type.adj`/`verb-type.adj`/`pronoun-type.adj`/`preposition-type.adj`. Grounds
  CCSS L.1.1.i. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "conjunction_type|coordinating_conjunction|correlative_conjunction|
  subordinating_conjunction|conjunctive_adverb" code/specs/data/adj-facts-stdlib/` found ZERO
  hits, confirming a completely fresh topic before this file was written. WebFetch-verified
  before writing (twice -- the second pass specifically re-confirmed the coordinating and
  correlative sentences were complete, with no truncation or missing clauses). Honest
  abstention on "conjunctive_adverb" (a real category the same page also names, but one that
  belongs to a DIFFERENT word class -- an adverb, not a conjunction -- rather than being a
  fourth conjunction type). New manifest objective `adj.literacy.k2.conjunction_type` (band
  K-2, `recall` competency, `ccss.ela` coverage root). New e2e test
  `facts_conjunctiontype_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention
  on an untabled conjunction type).
