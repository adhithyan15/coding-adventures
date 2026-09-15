- `language/clause-type.adj` (new) -- a new `clause_type(type, description)` table names the
  two structural kinds of clause and what makes a clause one or the other
  (independent_clause->is_a_clause_that_alone_is_a_complete_sentence,
  dependent_clause->is_a_clause_that_alone_is_not_a_complete_sentence), quoted verbatim from
  Grammarly's "Independent and Dependent Clauses: Rules and Examples" article -- `trust
  consensus`, the same tier `determiner-type.adj`/`noun-type.adj`/`verb-type.adj`/
  `pronoun-type.adj`/`conjunction-type.adj` (the other libraries in this directory) already
  use for their Grammarly citations. Picked using the mandatory full-tree-grep-before-scoping
  discipline -- `grep -riE "clause_type|clause-type"` across `adj-facts-stdlib/` found no
  existing table for this predicate, only prose mentions of "clause" in sibling files.
  WebFetch-verified twice -- the second pass pulled the first three occurrences of
  "independent clause" and "dependent clause" from the top of the article, in order, to
  confirm the two clean, parallel, single-fact sentences tabled here (not the surrounding
  elaboration, which bundles in extra facts like "an independent clause ... is a simple
  sentence") are the article's own defining pair. Unlike most sibling tables in this
  directory, independent/dependent is a genuinely EXHAUSTIVE split, not an arbitrary subset
  of a longer list -- the article's own opening line states "every clause is either one or
  the other." Honest abstention on "noun_clause" instead: a real, well-documented clause
  category (Grammarly has its own dedicated guide to noun clauses), but one that names a
  FUNCTIONAL role a dependent clause can play, not a third structural type alongside these
  two. New manifest objective `adj.literacy.k2.clause_type` (band K-2, `recall` competency,
  `ccss.ela` coverage root). New e2e test `facts_clausetype_e2e.rs` (3 tests: direct recall,
  reverse binding, honest abstention on an untabled clause category).
