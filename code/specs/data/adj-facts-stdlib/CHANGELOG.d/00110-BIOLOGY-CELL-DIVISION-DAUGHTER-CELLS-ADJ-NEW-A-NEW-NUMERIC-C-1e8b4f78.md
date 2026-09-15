- `biology/cell-division-daughter-cells.adj` (new) -- a new numeric-cell
  `cell_division_daughter_cells(process, count)` table names the two
  eukaryotic cell-division processes and how many daughter cells each one
  produces (mitosis->2, meiosis->4), each quoted from a fetched NIH National
  Human Genome Research Institute "Genetics Glossary" page -- `trust
  authoritative`, the same tier `dna-base-pairs.adj`/`anatomy/body-counts.adj`
  already establish for genome.gov. A genuinely NEW library, not an
  extension of the already-shipped `mitosis-phases.adj` family -- meiosis is
  a wholly different biological process, not another phase of mitosis.
  Picked after checking earth-science/geology/meteorology/astronomy tables
  this window (soil-horizons.adj, plate-boundaries.adj, mineral-hardness.adj,
  hurricane-categories.adj, wind-scale.adj, wave-properties.adj,
  spectral-classes.adj, digestive-organs.adj -- all exhaustive fixed
  classifications, non-extendable), then researching mitosis/meiosis as a
  fresh topic. Honest abstention on `binary_fission`: a real cell-division
  process, but the one prokaryotes/bacteria use, not one of the two
  eukaryotic processes this table names. New manifest objective
  `adj.science.6to8.cell_division_daughter_cells` (166 objectives total, up
  from 165). New e2e test `facts_celldivisiondaughtercells_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention).
