- `geology/igneous-rock-type-eruption-location.adj` (new) — a sibling to the already-shipped
  `igneous-rock-type.adj` (`igneous_rock_type(type, description)`, the two-way intrusive/extrusive split
  by cooling location). That table's own header already quotes, verbatim, the extrusive row's defining
  NPS sentence in full, listing TWO locations joined by "or" — a fact the single `description` atom
  folded into one compound label. New `igneous_rock_type_eruption_location(type, location)` table decodes
  each listed location as its own row: extrusive → surface, extrusive → atmosphere. Honest abstention on
  intrusive, whose cited span names only one location with no listed alternative. New e2e test file
  `facts_igneousrocktyperuptionlocation_e2e.rs` (3 tests: forward multi-answer recall with citation,
  backward recall, honest abstention on intrusive). No manifest objective, matching `igneous-rock-type.adj`'s
  own precedent. Second and final slice from the geology/ domain sweep — closes out that sweep entirely.
