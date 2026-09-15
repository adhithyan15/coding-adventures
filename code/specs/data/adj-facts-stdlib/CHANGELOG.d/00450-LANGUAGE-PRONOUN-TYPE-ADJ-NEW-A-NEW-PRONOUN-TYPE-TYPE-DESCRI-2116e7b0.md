- `language/pronoun-type.adj` (new) -- a new `pronoun_type(type, description)` table names
  three pronoun types and what each actually is
  (personal_pronoun->changes_form_based_on_grammatical_person,
  indefinite_pronoun->refers_generally_without_specific_identification,
  interrogative_pronoun->used_in_questions), quoted verbatim from Grammarly's "Pronouns:
  Definition and Examples" article -- `trust consensus`, the same source family already
  used by `noun-type.adj`/`verb-type.adj`. Grounds CCSS L.1.1.d. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "\bpronoun_type\b|
  \bpersonal_pronoun\b|\bindefinite_pronoun\b|\binterrogative_pronoun\b|
  \brelative_pronoun\b" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a
  completely fresh topic before this file was written. WebFetch-verified before writing
  (twice -- the second pass confirmed the FULL sentences, since a first, shorter-truncated
  fetch had clipped the personal-pronoun sentence and undercounted the relative-pronoun
  passage's sentence count). Honest abstention on "relative_pronoun" (a real pronoun type
  the same page also covers, but whose own explanation takes THREE sentences rather than
  one clean self-contained sentence like the three tabled here). New manifest objective
  `adj.literacy.k2.pronoun_type` (band K-2, `recall` competency, `ccss.ela` coverage root).
  New e2e test `facts_pronountype_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on an untabled pronoun type).
