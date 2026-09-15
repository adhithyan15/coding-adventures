- `biology/seed-dispersal-mechanism.adj` (new) -- a new
  `seed_dispersal_mechanism(mechanism, description)` table names four ways
  a plant disperses its seeds and how each actually works
  (barochory->uses_gravity_as_a_simple_means_of_seed_dispersal,
  ballochory->seed_is_forcefully_ejected_by_explosive_dehiscence_of_the_fruit,
  anemochory->seeds_float_on_the_breeze_or_flutter_to_the_ground,
  epizoochory->transported_on_the_outside_of_vertebrate_animals), each
  quoted verbatim from its own subsection of Wikipedia's "Seed dispersal"
  article -- `trust consensus`, a MULTI-SOURCE-STYLE table (see
  `ocean-current-drivers.adj`). Picked after exhausting the
  "extend an existing table" pattern across 12 not-yet-checked science
  tables this window (animal-habitat.adj, plant-need.adj,
  ecosystem-factor-type.adj, fossil-formation-type.adj,
  fossil-preservation-type.adj, biome-type.adj, animal-adaptation.adj,
  animal-survival-adaptation.adj, plant-life-cycle.adj, frog-life-cycle.adj,
  ocean-current-drivers.adj, metamorphism-cause.adj -- none extendable),
  then researching seed-dispersal as a fresh topic and finding that three
  additional non-Wikipedia sources (NPS, which 404'd; USDA Forest Service
  research papers; a kids'-science page) all failed the clean-single-fact-
  sentence bar before Wikipedia's own per-mechanism subsections succeeded.
  Honest abstention on `hydrochory` (water dispersal -- every candidate
  sentence checked either conflates the mechanism with dispersal distance
  or is qualified by a following sentence) and `endozoochory` (ingestion
  dispersal -- its defining sentence bundles the definition together with
  a separate empirical claim about tree-species prevalence). New manifest
  objective `adj.science.6to8.seed_dispersal_mechanism` (band 6-8, `recall`
  competency, `ngss` coverage root; 159 objectives total, up from 158). New
  e2e test `facts_seeddispersalmechanism_e2e.rs` (3 tests: direct recall,
  reverse binding, honest abstention).
