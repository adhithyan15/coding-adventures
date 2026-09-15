- `oceanography/ocean-zone-depth.adj` (new) — a sibling to the already-shipped `ocean-zones.adj`
  (`ocean_zone(zone, order)`, ONE ordinal position per depth zone): a new
  `ocean_zone_depth(zone, max_depth_meters)` table names the approximate depth in meters the SAME
  already-cited WHOI "Ocean Zones" page states for each zone, decoded from spans already sitting
  unused inside `ocean-zones.adj`'s own header truth table — no new WebFetch. Three rows:
  sunlight_zone → 200, twilight_zone → 1000, midnight_zone → 4000, all read off the SAME
  already-quoted WHOI spans that table's single-order schema had no room for. New e2e test file
  `facts_oceanzonedepth_e2e.rs` (3 tests: forward recall with citation, backward recall from a
  bound depth, honest abstention on the abyssal zone). New manifest objective
  `adj.science.3to5.ocean_zone_depth`, matching `ocean-zones.adj`'s own precedent of having one.
