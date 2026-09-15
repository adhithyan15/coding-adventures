- `earth-science/karst-process-zone.adj` (new) — a `table` recording which groundwater zone a
  named karst process happens in: `karst_process_zone(process, zone)`, `cave_formation` →
  `zone_of_saturation_typically`, `speleothem_deposition` → `zone_of_aeration`. The SECOND cave/karst
  library, sibling to `speleothem-growth-surface.adj` on a genuinely different axis: that one
  answers WHERE ON THE CAVE a speleothem grows, this one answers WHERE RELATIVE TO THE WATER TABLE
  the process can happen at all. A full-tree grep for `water_table`, `aeration`, `saturation`,
  `karst`, `vadose` and `phreatic` found no table on this axis — the shipped `pond_zone` and
  `ocean_zone` families are depth zones within a body of water, a different sense of the word.

  Both rows come from a SINGLE sentence of the U.S. National Park Service's "Speleothems" article:
  "Although the formation of caves typically takes place below the water table in the zone of
  saturation, the deposition of speleothems is not possible until caves are above the water table in
  the zone of aeration." `trust authoritative`, the same source `speleothem-growth-surface.adj` and
  `weathering-cause-type.adj` already cite. The page was CONTENT-verified rather than merely
  status-checked (200, no soft-404 markers, 118 substantive sentences, zero hub markers).

  THE HEDGE IS ASYMMETRIC, AND THAT ASYMMETRY IS FAITHFUL. The source hedges one clause and not the
  other: caves "TYPICALLY" form below the water table, whereas speleothem deposition "IS NOT POSSIBLE
  UNTIL" caves are above it — a typical tendency versus a stated precondition. Following the
  placement rule this stdlib already applies (a qualifier modifying ONE VALUE lives inside that
  value's atom, as in `veto-override.adj`'s `congress_can_override_in_most_cases`; a qualifier
  modifying the WHOLE FACT lives in the citation, as with `electoral-college-count.adj`'s
  "currently"), the tendency is carried in the atom itself: `zone_of_saturation_typically`. The
  deposition row carries no suffix because its own clause carries no hedge. The consequence is
  testable and is tested: a reverse query for the bare `zone_of_saturation` ABSTAINS, because asking
  for the unhedged zone is asking which process ALWAYS happens below the water table — a question
  this source does not answer. If that query ever starts binding, a tendency has been silently
  promoted to a rule.

  The sentence also states each zone's position relative to the water table (saturation below,
  aeration above), but that is a fact about the ZONE rather than the process, so it is deliberately
  NOT a third column — it would repeat once per process row rather than adding an axis. It remains
  available from the same sentence for a future `zone → water_table_position` sibling. Honest
  abstention also on `cave_decoration` as a process distinct from deposition (the page names
  decoration as a PHASE beginning when the chamber fills with air, without assigning it a zone of its
  own, so tabling it would double-count the deposition row) and on `dissolution`, `speleogenesis` and
  other real karst processes this source never places relative to the water table — inferring their
  zone from general karst knowledge is exactly what a grounded recall library must not do. New
  `karst-process-zone.query.adj` and `facts_karstprocesszone_e2e.rs` (5 tests: deposition placed with
  its verbatim sentence, the typicality hedge kept inside the atom with a negative assertion that no
  bare zone is stated, backward recall from the zone, the unhedged-zone abstention with a negative
  assertion that cave formation is never asserted as unconditional, and abstention on two unplaced
  karst processes). New manifest objective `adj.science.3to5.karst_process_zone`.

