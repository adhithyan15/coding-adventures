# Trestle — richer Gantt (v1)

Closes the "Richer Gantt" line of the design-fidelity gap in
`code/programs/mosaic/task-app/BACKLOG.md`: "day-grid columns,
weekend/today shading, milestone diamonds, dependency arrows, hover
tooltips, a legend. Current timeline is a simple proportional-bar-
per-row view." `code/specs/task-app-ui-design.md` §4.3 is the original
design thesis this gap measures against.

## What ships in this slice

- **A day-grid ruler**: one column per calendar day in the visible
  span, weekends shaded, today's column highlighted — placed once
  between the existing `tl-scale` caption and the row list, not
  composited behind each bar (see the kernel-limitation note under
  "Day-grid feasibility" below).
- **Percent-complete fill**: a darker overlay inside each bar, sized to
  `Task.percent_complete` — data the engine's `gantt()` projection
  already returns (`GanttBar.percent_complete`) but the host never
  drew. No engine change for this one.
- **Milestones as diamonds**: a zero-duration `TaskKind::Milestone` task
  renders as a small rotated square instead of the normal bar.
- **Hover tooltips**: name, dated window, day count, critical/percent
  status — via the kernel's existing `HostTooltip` primitive (UI29-4),
  wrapping each bar.
- **A legend**: static swatches for Normal / Critical / Milestone /
  Today, once, above the chart.

## What does NOT ship — dependency arrows

`task-app-ui-design.md` §4.3 describes them as "curved FS connectors,
red when both endpoints are critical" — genuinely 2D line-drawing
between two arbitrary bars' positions, not a styling variation on an
existing element the way everything else in this slice is. The UI29
kernel has no primitive for it: no SVG-path/canvas-overlay primitive,
and `HostDraggable`/`HostDropTarget` (the only cross-element
positioning kernel exists today) are drag-and-drop primitives, not a
drawing surface. `task-app-ui-design.md` §4.6 itself anticipated this —
it names a dedicated `mosaic-pkg-gantt` package as "the one real
net-new renderer... given the SVG dependency arrows and date-grid math"
that was never built; the shipped Timeline is a much simpler inline
view. Forcing arrows into the current text/box layout DSL would mean
either a fragile pixel-math hack outside the DSL's model or a new
kernel primitive — both bigger asks than this slice's remit. Left as
its own backlog item, explicitly needing either a new kernel primitive
(an SVG-overlay host component) or product guidance on visual
treatment before it's picked up.

Also not chasing (present in the original design mock, absent from the
backlog's actual wishlist, so treated as future polish rather than this
slice's job): the translucent dashed "free slack" tail past a
non-critical bar. It needs `total_slack`/`free_slack` on `GanttBar`,
which the projection doesn't return today — a small, additive engine
change if picked up later, just not bundled into this pass.

## Engine change

`GanttBar` gains `kind: TaskKind` (`Leaf`/`Summary`/`Milestone`) — the
projection already computes `depth` and `percent_complete` per bar from
the same `Task`, so this is the same shape of addition, not new
plumbing. `percent_complete` needed no change; it already existed on
`GanttBar` and simply wasn't drawn.

## Day-grid feasibility note

Calendar's own weekend-tinting item is still deferred (see
`BACKLOG.md`) because a per-cell background swap there would mean a
4-way branch duplicating the whole drop-target + event-loop render
tree — judged not worth it for a colour difference. Gantt's grid cells
carry no such weight: they're plain background boxes with no drag
target and no event overlay, so a 3-way branch (weekday / weekend /
today) per cell is cheap here in a way it wasn't for Calendar. This is
why the same "mosstyle can't vary one part's background per data value"
limitation that blocked Calendar doesn't block this.

One grid cell renders per calendar day in the visible span — for a
multi-year project that's a few hundred elements, not thousands; no
artificial cap is added. If a real multi-year project ever makes this
a measured problem, that's a virtualization concern for a later pass,
not a reason to withhold the grid now.

## Wiring summary

- `task-core`: `GanttBar.kind: TaskKind` added to the `gantt()`
  projection.
- `TaskApp.mil`: `slot timeline-grid : list<list<text>> ;` (one row per
  day: `[widthPct, weekendMarker, todayMarker]`). Existing
  `timeline-rows` cells extend from
  `[name, padWidth, barWidth, window, critical]` to add
  `[..., kind, percentComplete, tooltipText]` — appended, not inserted,
  so no existing `t[n]` reference shifts. The legend is static copy
  (its swatches and labels never vary per render), so it's written
  directly in `.mll`/`.msl` rather than threaded through a slot.
- `TaskApp.mll`: the day-grid renders as a `For` over `timeline-grid` —
  **a ruler strip above the bars, not a true per-row background
  overlay**. The kernel has no z-index/absolute-positioning primitive
  (`Row`/`Column`/`Box` compose by normal flow only), so compositing a
  grid literally *behind* every task row — the design mock's picture —
  isn't expressible without a new capability. A ruler strip is the
  honest, DSL-native version of the same information (day-by-day scale,
  weekend shading, a today marker), positioned once between `tl-scale`
  and the row list rather than repeated behind each bar. Each bar
  gains a `HostTooltip` wrapper and, for milestones, a diamond variant
  swapped in via `If (when: (t[5]))` (the kind cell) ahead of the
  existing critical/non-critical branch; a percent-complete overlay
  `Box` sits inside the bar sized from `(t[6])`. The legend is a static
  row of labelled swatches, not data-bound (nothing about it varies per
  render).
- `TaskApp.{light,dark}.msl`: day-grid cell parts (weekday / weekend /
  today), milestone diamond part, percent-complete overlay part, legend
  swatch parts.
- `main.tsx` / `timeline.ts`: `buildTimeline()` gains the grid-day
  computation (weekday-of-week + today comparison per calendar day in
  `[first, last]`) and extends each row with the kind/percent/tooltip
  cells. Pure arithmetic, same "no wasm engine, no DOM" testability
  `timeline.ts` already has — new grid-day logic gets its own unit
  tests there.
