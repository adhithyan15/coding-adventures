- `geology/earth-layer-thickness.adj` (new) — a sibling to the already-shipped `earth-layers.adj`
  (`has_state(layer, state)`, ONE physical-state fact per layer): a new
  `earth_layer_thickness(layer, thickness_km)` table names the THICKNESS in kilometers the SAME
  already-cited USGS page states for a layer, decoded from spans already sitting unused inside
  `earth-layers.adj`'s own header and provenance block — no new WebFetch. Three rows: mantle →
  2900, outer_core → 2200, inner_core → 1250, all read off the SAME already-quoted USGS spans that
  table's single-state schema had no room for. Deliberately narrow: the crust's own span states
  only that it is "very thin," with no figure to decode, so it abstains honestly. New e2e test
  file `facts_earthlayerthickness_e2e.rs` (3 tests: forward recall of all three figures with
  citation, backward recall from a bound thickness, honest abstention on the crust). No manifest
  objective, matching `earth-layers.adj`'s own precedent of not having one.
