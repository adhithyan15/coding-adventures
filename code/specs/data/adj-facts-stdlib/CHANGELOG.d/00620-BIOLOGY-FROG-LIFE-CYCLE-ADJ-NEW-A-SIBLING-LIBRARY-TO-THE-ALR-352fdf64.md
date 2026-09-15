- `biology/frog-life-cycle.adj` (new) -- a sibling library to the already-shipped
  `monarch-life-cycle.adj`, applying the SAME plain numbered life-cycle-stage recall shape
  (`frog_life_stage(stage, order)`) to a DIFFERENT organism. Three rows (egg->1, tadpole->2,
  frog->3), quoted verbatim from National Geographic Kids UK's "The Frog Life Cycle for Kids"
  page's three numbered stage headings ("Stage 1: Extraordinary eggs", "Stage 2: Teeny
  tadpoles!", "Stage 3: Fully-grown frog!") -- `trust consensus`, the same tier this stdlib
  already reserves for other National Geographic sources (`animal-habitat.adj`,
  `rainforest-layer.adj`). Grounds NGSS 3-LS1-1, the same standard `monarch-life-cycle.adj`
  grounds. Picked using the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE
  "\bfrog\b|\btadpole\b|\bfroglet\b" code/specs/data/adj-facts-stdlib/` found only incidental
  prose hits (`animal-classes.adj` lists "frog" as an amphibian example, `word-families.adj`
  lists "frog" in a rhyme list; neither is a life-cycle table), confirming zero prior coverage
  before this file was written. WebFetch-verified TWICE for consistency, both fetches returning
  the SAME three numbered headings. The source narrates leg growth occurring during the tadpole
  stage but gives that transition no separate numbered heading, so this table deliberately does
  NOT invent a fourth "froglet" row the source never numbers. Honest abstention on "adult" (the
  source's own heading says "frog," not "adult"). New manifest objective
  `adj.science.3to5.frog_life_cycle` (band 3-5, `recall` competency, `ngss` coverage root,
  mirroring `monarch-life-cycle.adj`'s exact band/competency). New e2e test
  `facts_froglifecycle_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  an untabled stage name).
