- `agriculture/farm-animal-secondary-product.adj` (new) — a sibling to the already-shipped
  `farm-animals.adj` (`farm_animal_product(animal, product)`, ONE clear, source-stated product per
  animal: chicken eggs, duck eggs, sheep wool, rabbit wool, goat milk). That table's own per-row
  provenance already quotes, verbatim, a SECOND product for three of the five animals — that the
  one-product-per-animal schema had no room for: chicken's cited span also states "in eggs or
  meat," duck's cited span also states "produce meat and eggs," and sheep's cited span also states
  "produce wool, meat, and milk." New `farm_animal_secondary_product(animal, product)` table:
  chicken → meat, duck → meat, sheep → meat, sheep → milk. Honest abstention on rabbit and goat,
  whose own cited spans state only a USE ("used for fiber arts") or a PROCESSING note ("may be
  pasteurized") for their already-recorded product, not a genuinely different second product. New
  e2e test file `facts_farmanimalsecondaryproduct_e2e.rs` (3 tests: forward recall with citation,
  backward recall from a bound product, honest abstention on goat). No manifest objective, matching
  `farm-animals.adj`'s own precedent of not having one. First slice from the agriculture/ sweep
  tranche (1 strong candidate found; agriculture/ is now effectively closed after this ships).
