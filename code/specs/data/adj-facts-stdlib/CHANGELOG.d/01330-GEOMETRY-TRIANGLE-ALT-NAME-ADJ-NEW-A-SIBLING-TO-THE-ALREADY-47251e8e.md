- `geometry/triangle-alt-name.adj` (new) — a sibling to the already-shipped `triangle-types.adj`
  (`triangle_sides(triangle, condition)`, ONE defining side-condition per triangle side-class:
  equilateral, isosceles, scalene). That table's equilateral row's own already-quoted MathWorld
  sentence — "An equilateral triangle is a triangle with all three sides of equal length a,
  corresponding to what could also be known as a 'regular' triangle." — also names an ALTERNATE
  word for equilateral that the condition-only schema had no room for. New
  `triangle_alt_name(triangle, alt_name)` table: equilateral → regular. Honest abstention on
  isosceles and scalene, whose own cited spans name no alternate word. New e2e test file
  `facts_trianglealtname_e2e.rs` (3 tests: forward recall with citation, backward recall from a
  bound alternate name, honest abstention on isosceles). No manifest objective, matching
  `triangle-types.adj`'s own precedent of not having one. Fourth slice from the geometry/ sweep
  tranche.
