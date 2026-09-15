- `biology/animal-habitat-definition.adj` (new) — a `rule` composing the already-shipped
  `animal_habitat(animal, biome)` table (`biology/animal-habitat.adj`, National Geographic Kids)
  with the already-shipped `biome_type(biome, description)` table (`biology/biome-type.adj`,
  National Geographic Education) to DERIVE `animal_habitat_definition(animal, description)` — WHAT
  an animal's habitat actually IS, not just its name. Neither already-shipped table states outright
  "a bactrian camel's habitat is a dry area where rainfall is less than 50cm/20in per year", but
  animal-habitat's specific animal→biome assignment run through biome-type's general biome→
  definition fact answers it directly: bactrian_camel → dry_areas_where_rainfall_is_less_than_
  50_centimeters_20_inches_per_year, giraffe → open_regions_dominated_by_grass_with_a_warm_dry_
  climate. This is the FIFTH `rule`-based CAUSAL-COMPOSITION library in this loop's science
  curriculum sweep, following the same "compose two independently-citable facts into a derived,
  dual-cited conclusion" discipline `physics/heat-causes-phase-change.adj`,
  `physics/force-causes-acceleration.adj`, `geology/earth-layer-matter-behavior.adj`, and
  `chemistry/measuring-tool-si-unit.adj` already established — a SAME-DIRECTORY instance of that
  discipline (both tables already live in `biology/`), the shape `heat-causes-phase-change.adj`/
  `force-causes-acceleration.adj` first proved out, not the cross-directory shape the two most
  recent slices used — and the FIRST to ground the "ecosystems" Major Gap (ADJ-STDLIB-COVERAGE.md
  §5.1/§5.2) rather than an Earth-processes, physics-mechanics, or observation/measurement one.
  Zero new WebFetch: both composed tables were already shipped and already cited. This is also the
  cycle's answer to the standing question of whether heredity/ecosystems needed a fresh
  WebFetch-sourced table before a second composable table existed — verified against the corpus
  rather than accepted on faith: re-read all 58 then-shipped `biology/` table headers plus
  `environment/ecosystem-factor-type.adj`, found `animal-habitat.adj` + `biome-type.adj` share the
  literal biome atoms `desert`/`grassland` (2 of `animal_habitat`'s 3 rows), confirmed via
  `grep -rn "row (.*\b(desert|forest|grassland|tundra|arctic)\b" adj-facts-stdlib/` that this is the
  ONLY place those atoms collide across the whole stdlib (besides an unrelated `geography/
  oceans.adj` ranking row) — a genuine, previously-unexamined join, not a re-tried and rejected one:
  `biome-type.adj`'s own CHANGELOG entry had already checked `animal-habitat.adj` for overlap, but
  only to rule out a DUPLICATE TABLE ("animal-habitat.adj only maps individual animals to habitat
  names, never tables biome-level defining sentences"), never as a rule-composition join candidate.
  Honest abstention on `polar_bear` (`arctic`) — `animal-habitat.adj`'s own header quotes its
  source's word as the climate region "arctic", never the biome name "tundra" `biome-type.adj`
  tables; `biome-type.adj`'s own header independently confirms tundra is a real, differently-defined
  biome ("has extremely inhospitable conditions, with the lowest measured temperatures"), not merely
  a rename of "arctic" — a genuine cross-source terminology gap, so forcing the join would assert a
  definition neither cited source states, exactly the invented-mapping trap this stdlib's
  honest-abstention discipline forbids. Also checked and ruled out this cycle for heredity/
  ecosystems specifically: `biology/dna_complement` (DNA base pairs) has no shared-atom join
  partner among shipped tables; `biology/cell-division-genetic-outcome` and `biology/
  cell-division-daughter-cells` share their `process` key (mitosis/meiosis) but both decode the SAME
  underlying NHGRI sentences already jointly quoted in each other's own headers, so a rule joining
  them would be a trivial column-merge of siblings from one shared citation, not an independent
  cross-table derivation; `biology/animal-habitat` and `biology/animal-survival-adaptation` still
  share only ONE overlapping animal (`polar_bear`), too thin to generalize a rule from (reconfirming
  the prior cycle's finding); `biology/genetic-code` → `biology/amino-acids` remains a naming/decode
  BRIDGE, not a causal explanation (reconfirming the prior cycle's finding). Empirically verified in
  a scratch dir against the real built CLI binary before writing the shipped files, per the standing
  discipline. New manifest objective `adj.science.3to5.animal_habitat_definition` (band 3-5, `infer`
  competency, matching all four prior rule-derived facts' own precedent, and `biome-type.adj`'s own
  band since the derivation's ceiling is set by the more advanced source table). New e2e test
  `facts_animalhabitatdefinition_e2e.rs` (3 tests: derivation with dual citations, reverse binding,
  honest abstention).
