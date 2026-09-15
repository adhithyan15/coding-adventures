- `geometry/quadrilateral-secondary-property.adj` (new) — a sibling to the already-shipped
  `quadrilateral-types.adj` (`quadrilateral_property(shape, property)`, ONE defining property
  per quadrilateral: square, rectangle, rhombus, parallelogram, trapezoid). That table's
  `property` column has room for only the PRIMARY defining property; the SAME already-quoted
  MathWorld sentences also state a SECOND, distinct property for two of the five quadrilaterals:
  rectangle → opposite_sides_equal_length ("opposite sides of equal lengths a and b"),
  parallelogram → opposite_angles_equal ("opposite sides parallel (and therefore opposite angles
  equal)"). Honest abstention on square, rhombus, and trapezoid, whose own cited spans state only
  the single primary property. New e2e test file `facts_quadrilateralsecondaryproperty_e2e.rs` (3
  tests: forward recall of both with citation, backward recall from a bound property, honest
  abstention on square). No manifest objective, matching `quadrilateral-types.adj`'s own
  precedent of not having one. Second slice from the geometry/ sweep tranche.
