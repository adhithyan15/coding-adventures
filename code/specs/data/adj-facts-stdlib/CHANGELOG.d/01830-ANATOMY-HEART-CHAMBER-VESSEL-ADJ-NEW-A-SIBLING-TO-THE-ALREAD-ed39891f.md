- `anatomy/heart-chamber-vessel.adj` (new) — a sibling to the already-shipped `heart-chambers.adj`
  (`heart_chamber_function(chamber, function)`, right_atrium/right_ventricle/left_atrium/
  left_ventricle → receives_blood_from_body/pumps_blood_to_lungs/receives_blood_from_lungs/
  pumps_blood_to_body). That table's own header already quotes, verbatim, four StatPearls
  sentences that each name a chamber's function AND, in the same breath, the named vessel(s)/
  valve(s) blood passes through to get there — a fact the chamber/function schema had no room for.
  New `heart_chamber_vessel(chamber, vessel)` table decodes those named structures as their own
  rows (6 total): right_atrium → superior_vena_cava, inferior_vena_cava; right_ventricle →
  pulmonic_valve, pulmonary_artery; left_atrium → pulmonary_veins; left_ventricle → aortic_valve.
  Confirmed via discipline #30 (schema-duplication check) that this is genuinely distinct from the
  already-shipped `valve_separates(valve, boundary)` in `heart-valves.adj` — that table is keyed
  by VALVE and names the two CHAMBERS it separates; this one is keyed by CHAMBER and names the
  VESSEL(S)/VALVE(S) attached to it, and never mentions the vena cavae or pulmonary veins at all.
  Honest abstention on `septum`, a real cardiac structure but not one of the four chambers. New
  e2e test file `facts_heartchambervessel_e2e.rs` (3 tests: forward recall with citation,
  2-answer backward recall, honest abstention). No manifest objective, matching
  `heart-chambers.adj`'s own precedent. Fourth slice from the anatomy/ domain sweep — steady at
  176 objectives. 99th content slice overall.
