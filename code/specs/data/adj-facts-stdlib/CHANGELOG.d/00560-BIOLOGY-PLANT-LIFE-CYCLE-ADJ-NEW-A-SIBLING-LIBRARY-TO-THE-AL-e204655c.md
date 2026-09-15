- `biology/plant-life-cycle.adj` (new) -- a sibling library to the already-shipped
  `monarch-life-cycle.adj`/`frog-life-cycle.adj`, applying the SAME plain numbered
  life-cycle-stage recall shape (`plant_life_stage(stage, order)`) to a flowering plant's early
  life. Three rows (seed->1, germination->2, seedling->3), quoted verbatim from Ducksters'
  "Flowering Plants" (Biology for Kids) article's "Life-cycle of a Flowering Plant" section --
  `trust consensus`. This is the FIRST citation from Ducksters in this stdlib -- a reputable,
  long-running kids-science-education site, the same tier this stdlib already reserves for
  other non-.gov kids-education sources (National Geographic Kids, Grammarly), not a primary
  .gov source. Grounds NGSS 3-LS1-1. Picked using the mandatory full-tree-grep-before-scoping
  discipline -- `grep -rilE "\bgermination\b|plant_life_stage" code/specs/data/adj-facts-stdlib/`
  found ZERO hits, confirming a completely fresh topic before this file was written. Three
  candidate sources were ruled out before Ducksters: natgeokids.com's UK plant-life-cycle page
  (no clean numbered stages), coolkidfacts.com (no numbered list), and smartclass4kids.com
  (numbered but an unbranded low-trust site). WebFetch-verified before writing. Honest
  abstention on "flowering" (a real later stage the source's own narrative goes on to describe,
  but not one of these three tabled here, keeping this slice the same size as every sibling
  life-cycle library). New manifest objective `adj.science.3to5.plant_life_stage` (band 3-5,
  `recall` competency, `ngss` coverage root). New e2e test `facts_plantlifecycle_e2e.rs` (3
  tests: direct recall, reverse binding, honest abstention on an untabled stage name).
