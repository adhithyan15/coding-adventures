- `meteorology/cloud-signal.adj` (new) — a sibling to the already-shipped `cloud-type.adj`
  (`cloud_type(cloud, weather_indication)`, each of three common cloud types and the single combined
  weather indication its presence signals). Two of that table's own three quoted NWS sentences each list
  MORE than one signal joined by an exhaustive "or" list — a fact the single `weather_indication` atom
  folded into one compound label. New `cloud_signal(cloud, signal)` table decodes each listed signal as
  its own row: cirrus → approaching_warm_front, cirrus → upper_level_jet_streak, stratus →
  precipitation_free, stratus → light_precipitation, stratus → drizzle. Honest abstention on
  cumulonimbus, whose cited span names only one signal with no listed alternative. New e2e test file
  `facts_cloudsignal_e2e.rs` (3 tests: forward multi-answer recall with citation, backward recall, honest
  abstention on cumulonimbus). No manifest objective, matching `cloud-type.adj`'s own precedent. First
  slice from a fresh domain sweep of meteorology/ — begins after the geology/ domain was fully exhausted
  this window.
