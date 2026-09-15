- `biology/vertebrate-thermoregulation.adj` (new) — a sibling to the already-shipped
  `vertebrate-groups.adj` (`vertebrate_trait(class, trait)`, ONE distinctive body-covering feature
  per class — fish → gills, bird → feathers, etc.). That table's own header already quotes,
  verbatim, the full NPS "Vertebrate Grab Bag" trait list for every class, and buried in every one
  of those five spans, alongside the body-covering feature the parent table already captures, is a
  second, cleanly binary fact the one-trait-per-class schema had no room for: whether the class is
  ectothermic ("cold-blooded") or endothermic ("warm-blooded"). New
  `vertebrate_thermoregulation(class, type)` table covers the full five-class domain with no
  abstention: fish → ectothermic, amphibian → ectothermic, reptile → ectothermic, bird →
  endothermic, mammal → endothermic. New e2e test file `facts_vertebratethermoregulation_e2e.rs` (3
  tests: forward recall with citation, backward recall of all ectothermic classes, full-domain
  coverage with no abstention). No manifest objective, matching `vertebrate-groups.adj`'s own
  precedent of not having one. First slice from a fresh biology/ sweep tranche (49 tables) —
  discovered after the 1-2-table-domain sweep initiative completed; two more strong candidates
  (`muscle-striation.adj`, `insulin-glucagon-trigger.adj`) are queued next.
