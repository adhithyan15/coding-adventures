- `geology/rock-type-formation-component.adj` (new) — a sibling to the already-shipped `rock-type.adj`
  (`rock_type(rock, formation_process)`, the process by which each of the three basic rock classes
  forms — igneous → crystallized_molten_rock, sedimentary → deposited_weathered_material, metamorphic
  → heat_and_pressure_transformation). Two of that table's own three per-row USGS quotes each list MORE
  than one material/agent joined by an exhaustive "or" or comma-list — a fact the single
  `formation_process` atom folded into one compound label. New
  `rock_type_formation_component(rock, component)` table decodes each listed item as its own row:
  sedimentary → pre_existing_rocks, sedimentary → pieces_of_once_living_organisms, metamorphic →
  high_heat, metamorphic → high_pressure, metamorphic → hot_mineral_rich_fluids. Honest abstention on
  igneous, whose cited span names only one material with no listed alternative. New e2e test file
  `facts_rocktypeformationcomponent_e2e.rs` (3 tests: forward multi-answer recall with citation,
  backward recall, honest abstention on igneous). No manifest objective, matching `rock-type.adj`'s own
  precedent. First slice from a fresh domain sweep of geology/ — begins after the astronomy/ domain was
  fully exhausted this window.
