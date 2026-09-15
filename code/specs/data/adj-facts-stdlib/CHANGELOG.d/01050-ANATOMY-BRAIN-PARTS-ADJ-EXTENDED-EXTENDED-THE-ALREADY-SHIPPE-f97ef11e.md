- `anatomy/brain-parts.adj` (extended) — extended the already-shipped `brain_part_function(
  brain_part, function)` table from 6 to 15 rows. The `brainstem` row's own cited StatPearls
  source sentence always listed TEN autonomic functions ("breathing, temperature regulation,
  respiration, heart rate, wake-sleep cycles, coughing, sneezing, digestion, vomiting, and
  swallowing"), but only the first, `breathing`, had ever been turned into a row. Added the
  other nine as new rows sharing the same `brainstem` key -- the same many-valued-per-key
  shape `kingdoms.adj`/`opposites.adj`/`synonyms.adj` already established -- so the original
  `row (brainstem, breathing)` is untouched and every new row is a pure addition, not a
  restructure. This is distinct from the `insect-parts.adj` case checked earlier this window,
  where the unused header material lives inside an existing COMPOUND atom on a single row
  (`digestion_and_reproduction`) rather than as a candidate for new standalone rows -- that
  table was correctly left alone since splitting it would alter an already-shipped fact rather
  than purely add to the table. WebFetch re-verified the brainstem sentence live before
  writing -- byte-identical to what the header already quoted. New e2e test
  `anatomy_brain_parts_extension_recalls_newly_added_brainstem_functions`. No new manifest
  objective (same library, same objective, 168 total unchanged).
