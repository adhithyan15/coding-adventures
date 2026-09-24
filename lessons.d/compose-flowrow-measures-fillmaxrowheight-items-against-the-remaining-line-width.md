---
category: Mosaic compiler pipeline
---

# Compose FlowRow measures fillMaxRowHeight items against the remaining line width, so it cannot stretch fraction-width items

**What happened.** CSS lays out a wrapping flex row in lines, and `align-items`
defaults to `stretch`, so every item on a line is as tall as the line's
tallest. On Compose, Calendar's week with an event was ragged. The event's
day ran about 55px below its neighbours, whose borders stopped at their 96px
`min-height`.

The obvious lowering is `FlowRowScope.fillMaxRowHeight()` on each item. It
compiles, and its emitter test passes. The render was catastrophic: all 42 day
cells shrank into a single line. `FlowRow` measures a `fillMaxRowHeight`
item against the **remaining** width of the current line, not the row's
width. Calendar's cells size themselves as a fraction of the incoming max
width (`_mosaicFillFraction(0.142857f)`), so each took a seventh of what was
left, and the line never broke.

**What to do instead.**
- Don't lower `align-items: stretch` in a wrapping row to `fillMaxRowHeight()`
  for items whose width depends on the incoming constraints (fractions,
  `fillMaxWidth`). It is only safe for fixed-width items, and those are rare
  here.
- A faithful lowering needs a custom wrapping `Layout`. It would:
  1. give each item its width from the row's width, not the line's remaining
     space;
  2. break lines;
  3. take each line's height from the items' `maxIntrinsicHeight` at that
     width;
  4. measure every item once, at that fixed height.
  This was left as backlog, not built.
- Render before pushing a layout change. The emitter's string assertions and a
  successful compile both said this change was fine.
