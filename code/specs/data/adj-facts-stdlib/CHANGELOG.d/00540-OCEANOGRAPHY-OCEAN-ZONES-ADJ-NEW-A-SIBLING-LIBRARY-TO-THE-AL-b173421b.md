- `oceanography/ocean-zones.adj` (new) -- a sibling library to the already-shipped
  `plant-life-cycle.adj`/`frog-life-cycle.adj`, applying the SAME plain numbered
  ordered-sequence recall shape (`ocean_zone(zone, order)`) to the ocean's first three
  depth-based light zones. Three rows (sunlight_zone->1, twilight_zone->2, midnight_zone->3),
  quoted verbatim from the Woods Hole Oceanographic Institution (WHOI) "Ocean Zones" page's
  "What are the five ocean zones?" section, which lists all five zones in depth order in one
  summary sentence before giving each its own subsection in that same order -- `trust
  consensus` (WHOI is a reputable, long-running oceanographic research institution, but is NOT
  a .gov domain, distinct from the `authoritative` tier this stdlib reserves for primary .gov
  sources like NOAA, which the sibling `ocean-observing-instruments.adj` -- the only other
  library in this same directory -- uses). Grounds NGSS 3-5 ocean-systems standards. Picked
  using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\bepipelagic\b|\bmesopelagic\b|\bbathypelagic\b|ocean_zone|\bsunlight.zone\b|
  \btwilight.zone\b|\bocean.layer"  code/specs/data/adj-facts-stdlib/` found ZERO hits,
  confirming a completely fresh topic before this file was written. WebFetch-verified before
  writing (twice, across two cycles of this loop). Honest abstention on "abyssal_zone" (a real
  deeper zone the source names, but not one of these three tabled here, keeping this slice the
  same size as every sibling ordered-sequence library). New manifest objective
  `adj.science.3to5.ocean_zone` (band 3-5, `recall` competency, `ngss` coverage root). New e2e
  test `facts_oceanzones_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  an untabled zone name).
