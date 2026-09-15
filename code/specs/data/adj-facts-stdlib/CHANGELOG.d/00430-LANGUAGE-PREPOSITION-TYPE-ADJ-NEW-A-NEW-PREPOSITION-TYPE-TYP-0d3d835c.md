- `language/preposition-type.adj` (new) -- a new `preposition_type(type, description)` table
  names three categories of preposition and what each actually shows
  (preposition_of_place->shows_where_something_is_or_where_something_happened,
  preposition_of_time->shows_when_something_happened_or_will_happen,
  preposition_of_direction->shows_how_something_is_moving_or_which_way_its_going), quoted
  verbatim from Grammarly's "Prepositions: Definition, Types, and Examples" article --
  `trust consensus`, the same source family already used by `noun-type.adj`/`verb-type.adj`/
  `pronoun-type.adj`. Grounds CCSS L.1.1.i. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "preposition_type|
  preposition_of_place|preposition_of_time|preposition_of_direction|
  preposition_of_manner" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a
  completely fresh topic before this file was written. WebFetch-verified before writing
  (twice -- the second pass specifically re-checked whether the direction/movement sentence
  was complete on its own or ran on into a following worked example, since a first-pass fetch
  can misjudge where a source sentence actually ends). Honest abstention on
  "preposition_of_manner_cause_or_purpose" (a real category the same page also names, but one
  that bundles THREE distinct functions -- manner, cause, or purpose -- under a single label,
  rather than one clean single-concept category like the three tabled here). New manifest
  objective `adj.literacy.k2.preposition_type` (band K-2, `recall` competency, `ccss.ela`
  coverage root). New e2e test `facts_prepositiontype_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled preposition category).
