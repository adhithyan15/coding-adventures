- `biology/plant-need.adj` (new) -- what a plant needs to grow and the specific role each input
  plays in photosynthesis. A new `plant_need(need, role)` table names three inputs and the role
  each plays (sunlight -> excites_chlorophyll_electrons, water -> split_for_oxygen_and_electrons,
  carbon_dioxide -> combined_to_make_glucose), quoted verbatim from Washington State University's
  "Ask Dr. Universe" science-outreach column -- "How do flowers use sunlight and water to grow?"
  -- `trust consensus` (a university outreach column, not a primary research paper). Picked using
  the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE "\bplant.need|\bgerminat"
  code/specs/data/adj-facts-stdlib/` found only one incidental prose mention of "germinate" (in
  `seed-parts.adj`), confirming zero prior coverage before this file was written. Also confirmed
  this cycle: "simple circuits" and "states of energy" are BOTH dead ends, already covered by
  `physics/circuit-parts.adj` and `physics/energy-forms.adj`+`physics/energy-sources.adj`.
  WebFetch-verified before writing. Deliberately scoped to ONLY the three inputs the source gives
  a distinct role sentence for -- soil/nutrients is mentioned only in passing, with no role
  sentence of its own, so it is NOT a row. Honest abstention on "soil" (a real plant-growth
  input, but with no shipped role in this table) and "moonlight" (not a real input). New manifest
  objective `adj.science.3to5.plant_need` (band 3-5 -- the photosynthesis/electron-excitation
  language is more technical than typical K-2 content, matching `rainforest-layer.adj`'s band
  3-5 precedent -- `recall` competency, `ngss` coverage root). New e2e test
  `facts_plantneed_e2e.rs` (3 tests: direct recall, reverse binding, honest abstention on an
  untabled input).
