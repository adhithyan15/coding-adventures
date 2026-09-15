- `biology/animal-survival-adaptation.adj` (new) -- a new
  `animal_survival_adaptation(animal, adaptation)` table names three animals and the one
  survival adaptation each is known for (bats->echolocation, polar_bear->insulation,
  poison_dart_frog->warning_coloration), each row quoted verbatim from a DIFFERENT
  nationalgeographic.com animal-facts page -- `trust consensus`, the same tier this stdlib
  already reserves for other National Geographic sources (`animal-adaptation.adj`,
  `animal-habitat.adj`). Grounds NGSS 3-LS4-3. DELIBERATELY uses a different predicate name
  from the already-shipped `animal_adaptation` table (`animal-adaptation.adj`) -- that table
  already closed out arctic_fox/groundhog/canada_goose as its three rows, so this genuinely
  different set of animals/adaptations gets its own predicate rather than extending a closed
  table, mirroring how `monarch_life_stage`/`frog_life_stage` used distinct predicate names for
  the same shape applied to different organisms. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "\bmicrobat|animal_survival_adaptation|
  \becholocation\b|\baposematic\b" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming
  a completely fresh topic before this file was written. All three citations WebFetch-verified
  before writing (the bats quote needed a second, more targeted search for "microbats" in the
  page's "Classification" section, but is present verbatim). Since each row's animal comes from a
  DIFFERENT source page and an ADJ `table` carries only ONE table-level `source`/`locator`/
  `trust` block, the table's own citation is the bats row's (the primary/first-listed source),
  and the other two rows' own distinct citations are documented in header prose -- mirroring
  `animal-adaptation.adj`'s established multi-source discipline. Note the `polar_bear` atom also
  appears in `animal-habitat.adj` for a DIFFERENT fact (its habitat, not this adaptation) -- not
  a conflict, since that is a different predicate. Honest abstention on "chameleon" (a real,
  well-known animal, but not one of these three tabled here). New manifest objective
  `adj.science.3to5.animal_survival_adaptation` (band 3-5, `recall` competency, `ngss` coverage
  root). New e2e test `facts_animalsurvivaladaptation_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled animal).
