- `geometry/polygon-alt-name.adj` (new) — a sibling to the already-shipped `shapes.adj`
  (`polygon_sides(shape, sides)`, ONE side-count per polygon: triangle, quadrilateral, pentagon,
  hexagon, heptagon, octagon, nonagon, decagon). That table's own `source` field already quotes
  the MathWorld "Polygon" names table's triangle row verbatim — "3 | triangle (trigon)" — but the
  sides-only schema had no room for the parenthetical alternate name that row also states. New
  `polygon_alt_name(shape, alt_name)` table: triangle → trigon. Honest abstention on the other
  seven polygons, whose own MathWorld "Polygon" names-table rows carry no parenthetical alternate
  name. New e2e test file `facts_polygonaltname_e2e.rs` (3 tests: forward recall with citation,
  backward recall from a bound alternate name, honest abstention on quadrilateral). No manifest
  objective, matching `shapes.adj`'s own precedent of not having one. Fifth and LAST slice from
  the geometry/ sweep tranche — geometry/ (9 files) is now fully exhausted (5/5 candidates from
  the original sweep shipped: radius-definition, quadrilateral-secondary-property,
  quadrilateral-alt-name, triangle-alt-name, polygon-alt-name).
