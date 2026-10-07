---
category: Testing & coverage
---

# Bresenham must seed its error term with dx - dy, and a seed bug copied across ports needs every port fixed together

The all-octant integer Bresenham loop (`e2 = 2*err; if e2 > -dy: err -= dy, x += sx; if e2 < dx: err += dx, y += sy`) only terminates when `err` starts at `dx - dy`. The Haskell, Java and Kotlin `paint-vm-ascii` ports all seeded it with `0`; for slopes like (dx=3, dy=1) or (dx=1, dy=3) the minor-axis cursor overshoots the endpoint and the `x == x2 && y == y2` exit is never seen again. That is a silent hang, not an exception, and the existing line tests (horizontal, vertical, a 45-degree clamp) never exercised an asymmetric slope, so it shipped in three ports (issue #12093; Dart and Swift were written later with the correct seed).

What to do: any port of a stepping algorithm whose exit is an equality test (`== endpoint`) needs a test sweep over asymmetric inputs in every octant, not just the axis-aligned and 45-degree cases, plus a path-shape property (starts at p0, ends at p1, `max(|dx|, |dy|) + 1` cells, unit steps). When one port has the bug, grep every sibling port for the same seed before closing the issue. To prove such a regression test actually guards the fix, temporarily revert the seed and run only the new tests under an external `timeout`: a hang (exit 124) is the expected red result.
