- `environment/ecosystem-factor-type.adj` (new) -- a new `ecosystem_factor_type(factor,
  description)` table names the two kinds of ecosystem factor and what defines each
  (biotic->a_living_organism_that_shapes_its_environment,
  abiotic->a_non_living_part_of_an_ecosystem_that_shapes_its_environment), quoted verbatim
  from two sibling National Geographic Education resource pages, "Biotic Factors" and
  "Abiotic Factors" -- `trust consensus`, the same tier `biome-type.adj` (a sibling library,
  sourced from the same publisher's "The Five Major Types of Biomes" article) already uses.
  Picked using the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE
  "ecosystem_factor_type|biotic|abiotic" code/specs/data/adj-facts-stdlib/` found ZERO hits,
  confirming a completely fresh topic before this file was written. WebFetch-verified before
  writing (twice per page -- the second pass pulled the full surrounding paragraph for each
  candidate sentence, confirming it stands alone grammatically and that the sentence
  following it supplies examples rather than a qualification the definition depends on).
  Unlike most sibling tables in this directory, biotic and abiotic are not two items picked
  out of a longer enumerable list -- together they exhaust the two-way classification these
  sources describe, so there is no third "real but excluded" factor type to name. Honest
  abstention on "producer" instead: a real and commonly taught ecology term, but one that
  names a food-chain ROLE (already grounded under its own predicate in
  `biology/food-chain-roles.adj`), not a biotic/abiotic FACTOR TYPE. New manifest objective
  `adj.science.3to5.ecosystem_factor_type` (band 3-5, `recall` competency, `ngss` coverage
  root). New e2e test `facts_ecosystemfactortype_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled ecology term).
