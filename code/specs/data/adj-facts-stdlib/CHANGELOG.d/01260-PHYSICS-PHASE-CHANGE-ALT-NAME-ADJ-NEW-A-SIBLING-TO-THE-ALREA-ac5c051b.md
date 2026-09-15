- `physics/phase-change-alt-name.adj` (new) — a sibling to the already-shipped
  `states-of-matter.adj` (`phase_change_name(change, name)`, ONE primary name per phase-change
  direction): a new `phase_change_alt_name(change, alt_name)` table names the older/alternate word
  the SAME already-cited LibreTexts pages state for a direction, decoded from spans already
  sitting unused inside `states-of-matter.adj`'s own header and provenance block — no new
  WebFetch. Three rows: solid_to_liquid → fusion, liquid_to_solid → solidification, liquid_to_gas
  → boiling, all read off the SAME already-quoted spans that table's single-name schema had no
  room for. Deliberately narrow: only these three of the table's six directions have an alternate
  name in the already-cited spans. New e2e test file `facts_phasechangealtname_e2e.rs` (3 tests:
  forward recall of all three with citation, backward recall from a bound alternate name, honest
  abstention on condensation). No manifest objective, matching `states-of-matter.adj`'s own
  precedent of not having one. Discovered via a background Explore-agent sweep of `physics/` (21
  files, a previously untouched large domain) — first slice from a newly-discovered tranche of
  unswept domains (agriculture/art/calendar/environment/geography/geometry/mathematics/
  metrology/money/music/nutrition/optics/physics/transportation).
