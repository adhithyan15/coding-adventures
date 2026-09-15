- `geology/volcano-type.adj` (new) -- a new `volcano_type(type, description)` table names
  three types of volcano and what each actually is (cinder_cone->is_the_simplest_type_of_
  volcano, shield_volcano->built_almost_entirely_of_fluid_lava_flows, composite_volcano->
  also_called_a_stratovolcano), quoted verbatim from USGS's "About Volcanoes" page --
  `trust authoritative`, the same tier the sibling `rock-type.adj`/`mineral-hardness.adj`
  (the other libraries in this directory) already use for their USGS citations. Grounds
  NGSS 4-ESS1-1/MS-ESS2-1. Picked using the mandatory full-tree-grep-before-scoping
  discipline -- `grep -rilE "cinder_cone|shield_volcano|composite_volcano|lava_dome"
  code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely fresh topic
  before this file was written. WebFetch-verified before writing (twice -- the second pass
  specifically confirmed each sentence's exact wording and where it actually ends). Honest
  abstention on "lava_dome" (a real term the same page also names, but one the source
  ITSELF explicitly disclaims as not a type: "these are technically not a 'volcano type'
  but rather an eruption phenomenon"). New manifest objective `adj.science.3to5.volcano_type`
  (band 3-5, `recall` competency, `ngss` coverage root). New e2e test
  `facts_volcanotype_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on
  an untabled term).
