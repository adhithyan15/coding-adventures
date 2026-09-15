- `agriculture/farm-animal-product-processing.adj` (new) — a sibling to the already-shipped
  `farm-animals.adj` (`farm_animal_product(animal, product)`) and `farm-animal-secondary-product.adj`
  (`farm_animal_secondary_product(animal, product)`). Goat's own per-row provenance in
  `farm-animals.adj` already quotes, VERBATIM, a PROCESSING method applied to its already-recorded
  milk -- a fact neither existing table's schema had room for: "Goat milk may be pasteurized for
  human consumption." New `farm_animal_product_processing(animal, product, processing)` table (a
  three-column shape already precedented in this stdlib, e.g. `biology/amino-acids.adj`) decodes
  that clause as its own row: goat, milk → pasteurized. This is explicitly the SAME clause
  `farm-animal-secondary-product.adj`'s own header already flagged as a processing note rather than
  a second product, left unshipped until now. No new WebFetch -- reuses the same already-cited
  CFSPH sentence. Honest abstention on every other animal/product pair, since none of the other
  four cited spans (chicken, duck, sheep, rabbit) states a processing method. New e2e test file
  `facts_farmanimalproductprocessing_e2e.rs` (3 tests: forward recall with citation, backward
  recall, honest abstention). No manifest objective, matching the parent tables' own precedent.
  114th content slice overall -- first slice of the agriculture/ domain sweep (mathematics/,
  metrology/, and calendar/ were also swept this cycle and found zero shippable candidates; see
  loop-state.json notes for the detailed rejection reasoning).
