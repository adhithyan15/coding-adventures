- `language/subordinating-conjunction-relationship-type.adj` (new) — a sibling to the
  already-shipped `conjunction-type.adj` (`conjunction_type(type, description)`,
  coordinating_conjunction/correlative_conjunction/subordinating_conjunction → their own defining
  sentences). That table's own header already reproduces, verbatim, the Grammarly sentence for the
  subordinating conjunction in full — "Subordinating conjunctions join dependent clauses to the
  independent clauses of sentences, signaling cause and effect, comparison, contrast, time, or some
  other kind of relationship between the clauses." — but the type/description schema had room only
  for the structural fact (joining clauses), not the four named relationship kinds the same
  sentence lists. New `subordinating_conjunction_relationship_type(relationship_type,
  conjunction_type)` table decodes those four kinds as their own rows: cause_and_effect/comparison/
  contrast/time → all subordinating_conjunction. The sentence's own vague trailing "or some other
  kind of relationship" clause is deliberately excluded — it names no additional category. Honest
  abstention on `concession` (a real relationship subordinating conjunctions can express in general
  grammar, but not one this specific quoted sentence names) and on `coordinating_conjunction` (a
  real, already-tabled conjunction category, but not the one this table covers). New e2e test file
  `facts_subordinatingconjunctionrelationshiptype_e2e.rs` (4 tests: forward recall with citation,
  4-answer backward recall, and two honest-abstention cases). No manifest objective, matching
  `conjunction-type.adj`'s own precedent. SEVENTH AND FINAL slice from the language/ domain cleanup
  sweep — this closes the sweep.
