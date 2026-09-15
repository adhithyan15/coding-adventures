- `earth-science/zone-water-table-position.adj` and
  `earth-science/karst-process-water-table-position.adj` (new, shipped together) — the THIRD and
  FOURTH cave/karst libraries, and the first pair in this series designed to COMPOSE.

  The table records where each groundwater zone sits relative to the water table:
  `zone_water_table_position(zone, position)`, `zone_of_saturation` → `below_the_water_table`,
  `zone_of_aeration` → `above_the_water_table`. `karst-process-zone.adj` deliberately declined to
  carry this as a third column, its header recording that a zone's position is a fact about the ZONE
  rather than the process and would repeat once per process row rather than adding an axis; this
  holds it once. A full-tree grep for `water_table_position`, `below_the_water_table` and
  `above_the_water_table` found only that sibling's own header prose, so the axis was uncovered.

  The rule DERIVES what neither table states: `karst_process_water_table_position(process,
  position)`, whose body chains `karst_process_zone(Process, Zone)` with
  `zone_water_table_position(Zone, Position)`. It has to be a rule rather than a two-goal query —
  a query in this language is a single term, and `? a(...), b(...)` is a parse error at the comma —
  so this follows the shape `civics/chamber-branch.adj` established. A derived answer carries the
  citations of BOTH premises and a proof trace (a `rule` step for the head, one `fact` step per body
  goal), which is what makes a composed answer auditable rather than indistinguishable from an
  asserted one. Both premises happen to be grounded in the same NPS sentence, so that sentence is
  honestly cited twice in one answer; the header says so, since it otherwise reads as duplication.

  *** THE RULE DERIVES ONLY ONE OF THE TWO PROCESSES, AND THAT IS THE POINT. *** The cave-formation
  row of `karst_process_zone` binds the HEDGED atom `zone_of_saturation_typically` — the source says
  caves TYPICALLY form below the water table, while saying speleothem deposition IS NOT POSSIBLE
  UNTIL they are above it — and that atom does not unify with the bare `zone_of_saturation` the
  position table keys on. So the identical chain that resolves for speleothem deposition ABSTAINS
  for cave formation. Had the hedge been dropped when that atom was authored, this rule would
  conclude "cave formation happens below the water table" as a flat, unqualified fact: exactly the
  claim the source declines to make. A qualifier that survives direct recall but evaporates the
  moment something reasons across two tables is not a qualifier, it is a comment. The two
  consequences are asserted as a MATCHED PAIR, and the negative assertion was FALSIFIED before being
  trusted — baring the atom in a scratch copy makes the rule immediately derive
  `"P":"below_the_water_table"`, so the guard genuinely can fail. The fix, if the cave-formation
  answer is ever wanted, is a source that states the unhedged claim, never a looser atom.

  Provenance: both rows and the rule's envelope cite the single sentence `karst-process-zone.adj`
  already cites, on the U.S. National Park Service's "Speleothems" article: "Although the formation
  of caves typically takes place below the water table in the zone of saturation, the deposition of
  speleothems is not possible until caves are above the water table in the zone of aeration."
  `trust authoritative`. HOW THE TABLE'S PAIRING IS READ is stated in its header so it can be
  audited: each column value appears verbatim ("below the water table", "the zone of saturation"),
  and the pairing is apposition — the sentence gives one location under two descriptions — rather
  than a standalone definitional sentence. Nothing is reworded and no value is supplied that the
  source does not state. Honest abstention on `vadose_zone` and `phreatic_zone`, the standard
  synonyms a reader may well ask by and which this source never uses, and on `capillary_fringe` and
  other hydrology subdivisions this sentence does not name; the rule additionally abstains on any
  process no premise places (`dissolution`, `speleogenesis`), since a derived relation cannot be
  better grounded than its premises.

  New `zone-water-table-position.query.adj` and `karst-process-water-table-position.query.adj`;
  new `facts_zonewatertableposition_e2e.rs` (3 tests: direct recall with its verbatim sentence and
  citation, backward recall, and the three-way synonym/subdivision abstention) and
  `facts_karstprocesswatertableposition_e2e.rs` (5 tests: the derivation, the proof trace naming
  both premise goals, backward recall through the rule, abstention on an unplaced process, and the
  hedge blocking the cave-formation join). New manifest objectives
  `adj.science.3to5.zone_water_table_position` (recall) and
  `adj.science.3to5.karst_process_water_table_position` (infer, with both premises as
  prerequisites).

