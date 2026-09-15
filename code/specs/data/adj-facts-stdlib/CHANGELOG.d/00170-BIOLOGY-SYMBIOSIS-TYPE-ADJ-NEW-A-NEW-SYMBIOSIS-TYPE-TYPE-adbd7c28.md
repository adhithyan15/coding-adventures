- `biology/symbiosis-type.adj` (new) -- a new `symbiosis_type(type,
  description)` table names three types of symbiotic relationship and
  what actually defines each (mutualism->both_parties_benefit,
  commensalism->one_organism_benefits_and_the_other_is_not_significantly_harmed_or_helped,
  parasitism->the_parasite_benefits_while_the_host_is_harmed), each quoted
  verbatim from its own standalone sentence in Wikipedia's "Symbiosis"
  article -- `trust consensus`, a MULTI-SOURCE-STYLE table (see
  `ocean-current-drivers.adj`, `seed-dispersal-mechanism.adj`). Picked
  after checking 9 not-yet-reviewed science tables across
  chemistry/physics/geology/geography/anatomy/meteorology this window
  (mixture-types.adj, reaction-types.adj, element-categories.adj,
  friction-types.adj, precipitation-types.adj, joint-types.adj,
  acids-bases.adj, gas-laws.adj, forces.adj, separation-methods.adj --
  none extendable, all closed exhaustive classifications), then
  researching symbiosis as a fresh topic. Honest abstention on
  `amensalism`: a real interaction category the same article's opening
  paragraph also names, but its own defining sentence bundles it together
  with `competition` in one semicolon-joined compound sentence rather
  than stating one clean fact each the way mutualism/commensalism/
  parasitism do. Picked using the mandatory full-tree-grep-before-scoping
  discipline -- zero hits for
  "symbiosis_type"/"mutualism"/"commensalism"/"parasitism" before this
  file was written. New manifest objective
  `adj.science.6to8.symbiosis_type` (band 6-8, `recall` competency,
  `ngss` coverage root; 160 objectives total, up from 159). New e2e test
  `facts_symbiosistype_e2e.rs` (3 tests: direct recall, reverse binding,
  honest abstention).
