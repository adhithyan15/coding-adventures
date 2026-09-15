- `oceanography/ocean-zone-scientific-name.adj` (new) — a sibling to the already-shipped
  `ocean-zones.adj` (`ocean_zone(zone, order)`) and `ocean-zone-depth.adj` (`ocean_zone_depth(zone,
  max_depth_meters)`). The SAME already-quoted WHOI "Ocean Zones" midnight-zone sentence — "The midnight
  zone, or bathypelagic, extends to about 4,000 meters..." — also names an alternate scientific name for
  that zone, a fact neither sibling's schema had room for (one tables sequence position, the other tables
  depth). New `ocean_zone_scientific_name(zone, scientific_name)` table decodes that clause as its own row:
  midnight_zone → bathypelagic. Honest abstention on sunlight_zone and twilight_zone, whose cited spans
  name no alternate scientific name. New e2e test file `facts_oceanzonescientificname_e2e.rs` (3 tests:
  forward recall with citation, backward recall, honest abstention on sunlight_zone). No manifest
  objective, matching both siblings' own precedent. First slice from the oceanography/ domain sweep.
