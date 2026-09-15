- `biology/animal-habitat.adj` (new) -- a sibling library to the already-shipped
  `animal-homes.adj`, but a DIFFERENT axis: not the built STRUCTURE an animal lives in (a hive,
  a nest, a burrow), but the broad BIOME/environment type an animal calls home. A new
  `animal_habitat(animal, biome)` table names three animals and the biome each lives in
  (polar_bear -> arctic, bactrian_camel -> desert, giraffe -> grassland), quoted verbatim from
  National Geographic (`kids.nationalgeographic.com` for the polar bear and Bactrian camel fact
  pages, `education.nationalgeographic.org` for the giraffe/grassland sentence) -- the same
  source family and `trust consensus` tier this stdlib already reserves for
  `rainforest-layer.adj`'s National Geographic Education citation. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -ril "habitat\|biome"
  code/specs/data/adj-facts-stdlib/` found only two incidental prose mentions of the word
  "habitat" (in `fungus-parts.adj` and `rainforest-layer.adj`), neither tabling an animal-biome
  relation, and `animal-homes.adj` was re-read in full to confirm its five rows are all built
  structures (bee/bird/spider/rabbit/beaver), a genuinely disjoint column semantic and animal
  set from this table -- CONFIRMED distinct, not a duplicate. WebFetch-verified before writing.
  Honest abstention on "dog" (a real animal, but with no shipped habitat in this table). New
  manifest objective `adj.science.k2.animal_habitat` (band K-2, `recall` competency, `ngss`
  coverage root, mirroring `adj.science.k2.heat_causes_phase_change`'s band/coverage-root
  convention for K-2 science). New e2e test `facts_animalhabitat_e2e.rs` (3 tests: direct
  recall, reverse binding, honest abstention on an untabled animal).
