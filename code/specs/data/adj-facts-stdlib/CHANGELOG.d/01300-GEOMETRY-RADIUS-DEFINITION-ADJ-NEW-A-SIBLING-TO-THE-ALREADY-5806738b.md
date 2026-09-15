- `geometry/radius-definition.adj` (new) — a sibling to the already-shipped `circle-parts.adj`
  (`circle_part(part, description)`, keyed by circle PART: radius, diameter, circumference,
  chord). This table recalls a DIFFERENT axis the SAME already-cited MathWorld "Radius" sentence
  states — what "radius" measures for TWO shapes, not just the circle `circle-parts.adj` already
  tables: `circle-parts.adj`'s own `source` field already quotes the full sentence "The distance
  from the center of a circle to its perimeter, or from the center of a sphere to its surface,"
  but that table's part-keyed schema had no row for the sphere half. New
  `radius_definition(shape, description)` table: circle → center_to_perimeter, sphere →
  center_to_surface. Honest abstention on any solid the cited sentence does not name (cube, cone,
  cylinder). New e2e test file `facts_radiusdefinition_e2e.rs` (3 tests: forward recall of both
  with citation, backward recall from a bound description, honest abstention on cube). No
  manifest objective, matching `circle-parts.adj`'s own precedent of not having one. First slice
  from a background Explore-agent sweep of `geometry/` (9 files) — found via a
  header/body-revisit of `circle-parts.adj`'s own `source` field (not even the header prose).
  Four more moderate-confidence candidates from the same sweep are queued
  (`quadrilateral-types.adj` second-clause properties, a rhombus/triangle alt-name pair, and a
  `shapes.adj` polygon alt-name — all single-to-few-row, deferred pending prioritization).
