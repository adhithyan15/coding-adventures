- `geometry/quadrilateral-alt-name.adj` (new) — a sibling to the already-shipped
  `quadrilateral-types.adj` (`quadrilateral_property(shape, property)`, ONE defining property
  per quadrilateral: square, rectangle, rhombus, parallelogram, trapezoid). That table's rhombus
  row's own already-quoted MathWorld sentence — "A rhombus is a quadrilateral with both pairs of
  opposite sides parallel and all sides the same length, i.e., an equilateral parallelogram." —
  also names an ALTERNATE word for rhombus that the property-only schema had no room for. New
  `quadrilateral_alt_name(shape, alt_name)` table: rhombus → equilateral_parallelogram. Honest
  abstention on square, rectangle, parallelogram, and trapezoid, whose own cited spans name no
  alternate word. New e2e test file `facts_quadrilateralaltname_e2e.rs` (3 tests: forward recall
  with citation, backward recall from a bound alternate name, honest abstention on square). No
  manifest objective, matching `quadrilateral-types.adj`'s own precedent of not having one. Third
  slice from the geometry/ sweep tranche.
