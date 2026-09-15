- `agriculture/farm-animal-maintenance-level.adj` (new) — a sibling to the already-shipped
  `farm-animals.adj` and `farm-animal-secondary-product.adj`. Sheep's own span IS the sentence
  `farm-animals.adj` already carries as its provenance envelope's `source` field -- but only the
  trailing clause ("...they can produce wool, meat, and milk") was ever decoded into rows; the
  LEADING clause of that same sentence names a husbandry-difficulty fact about the animal itself,
  never decoded anywhere: "Sheep are low maintenance and versatile -- depending on the breed, they
  can produce wool, meat, and milk". New `farm_animal_maintenance_level(animal, level)` table
  decodes that leading clause as its own row: sheep -> low_maintenance. "Maintenance level" (how
  much care the ANIMAL needs) is categorically distinct from "product" (what the animal GIVES).
  The sentence's other unused word, "versatile," is deliberately NOT decoded into a second row: it
  is trivially re-derivable from sheep already carrying three rows across the two existing product
  tables (wool, meat, milk), so it would restate rather than add a fact. No new WebFetch -- reuses
  the same already-cited CFSPH sentence. Honest abstention on every other animal, since none of the
  other four cited spans states a husbandry-difficulty descriptor. New e2e test file
  `facts_farmanimalmaintenancelevel_e2e.rs` (3 tests: forward recall with citation, backward
  recall, honest abstention). No manifest objective, matching the parent tables' own precedent.
  115th content slice overall -- second slice of the agriculture/ domain sweep.
