- `earth-science/metamorphism-cause.adj` (new) -- what causes a rock to become metamorphic, and
  what that process does to it. A new `metamorphism_cause(cause, effect)` table names three
  causes (heat, pressure, hot_mineral_rich_fluids) and their shared effect
  (denser_more_compact_rock), quoted verbatim from the U.S. Geological Survey's "What are
  metamorphic rocks?" FAQ page -- `trust authoritative`, a primary U.S. government geology
  source, the same tier `rock-types.adj` already established for its own NPS citation. A
  sibling library to the already-shipped `rock-types.adj`, but a genuinely different, FINER-
  grained axis: `rock-types.adj` gives ONE combined phrase for how metamorphic rock forms
  ("heat_and_pressure"), while this table decomposes the THREE distinct causes the USGS source
  names, each with its own row and the shared effect the source states. Picked using the
  mandatory full-tree-grep-before-scoping discipline -- `grep -rilE "\brock.cycle\b|\btransform"
  code/specs/data/adj-facts-stdlib/` found only incidental prose hits (mitosis-phases.adj's
  "chromatin is transformed," monarch-life-cycle.adj's "transforms inside," plate-boundaries.adj's
  unrelated "transform" plate-boundary type), confirming zero prior coverage of a
  metamorphism-cause relation before this file was written. WebFetch-verified before writing.
  Honest abstention on "sunlight" and "cold" (not cited causes of metamorphism). New manifest
  objective `adj.science.3to5.metamorphism_cause` (band 3-5, `recall` competency, `ngss`
  coverage root). New e2e test `facts_metamorphismcause_e2e.rs` (3 tests: direct recall,
  reverse binding enumerating all three causes, honest abstention on an untabled cause).
